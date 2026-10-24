//! Rust, as rustc checked it, to the TypeScript AST.
//!
//! Translation starts from the configured roots and follows what they reach. Functions
//! are monomorphized: each instance becomes its own TypeScript function, so a trait
//! call is always a direct call. Types are translated once, generically: a generic
//! class's method is one instance's, and declares that instance as its `this`.

mod body;
mod consts;
mod drop;
mod fmt;
mod iter;
mod library;
mod num;
mod pat;
mod task;

use std::collections::VecDeque;

use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::thir::{ExprId, Thir};
use rustc_middle::ty::{
	self, AdtDef, EarlyBinder, GenericArgsRef, Instance, InstanceKind, Ty, TyCtxt, TypeVisitableExt, TypingEnv,
};
use rustc_span::Span;

use crate::census::{Census, Refusal};
use crate::config::Config;
use crate::program::{Program, Shape, Source, SymKind, Symbol};
use crate::ts::{self, Ctor, FieldId, Item, Loc, SymId, Type};

/// Each function's THIR, copied before borrow checking steals it.
pub struct Bodies<'tcx> {
	map: rustc_data_structures::fx::FxHashMap<LocalDefId, (Thir<'tcx>, ExprId)>,
}

impl<'tcx> Bodies<'tcx> {
	pub fn collect(tcx: TyCtxt<'tcx>) -> Self {
		let mut map = rustc_data_structures::fx::FxHashMap::default();
		for def_id in tcx.hir_body_owners() {
			if !matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn | DefKind::Closure) {
				continue;
			}
			let Ok((thir, expr)) = tcx.thir_body(def_id) else {
				continue;
			};
			// Constant evaluation during type checking may have built this body's MIR
			// already; translating it then fails with a span instead.
			if !thir.is_stolen() {
				map.insert(def_id, (thir.borrow().clone(), expr));
			}
		}
		Self { map }
	}
}

/// Translate what the config's roots reach, or `None` after reporting why not.
pub fn translate<'tcx>(tcx: TyCtxt<'tcx>, config: &Config, bodies: &Bodies<'tcx>) -> Option<Program> {
	let mut t = Translator::new(tcx, config, bodies, None)?;
	for root in &config.roots {
		t.root(root);
	}
	for path in &config.printable {
		t.printable(path);
	}
	t.drain();
	if t.failed {
		return None;
	}
	t.program.name();
	Some(t.program)
}

/// Every construct translation refuses in the crate, reached from each of its items
/// as if each were a root.
pub fn census<'tcx>(tcx: TyCtxt<'tcx>, config: &Config, bodies: &Bodies<'tcx>) -> Option<Census> {
	let mut t = Translator::new(tcx, config, bodies, Some(Vec::new()))?;
	for def_id in tcx.hir_crate_items(()).definitions() {
		if matches!(tcx.def_kind(def_id), DefKind::Struct | DefKind::Enum | DefKind::Union) {
			t.adt(tcx.adt_def(def_id), tcx.def_span(def_id));
		}
	}
	let functions = |tcx: TyCtxt<'tcx>| {
		tcx.hir_body_owners()
			.filter(move |&def_id| matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn))
	};
	// A derived impl comes along only when called or printable, as in translation.
	for path in &config.printable {
		t.printable(path);
	}
	let roots: Vec<DefId> = functions(tcx)
		.map(LocalDefId::to_def_id)
		.filter(|&def_id| {
			!tcx.impl_of_assoc(def_id)
				.is_some_and(|imp| tcx.is_automatically_derived(imp))
		})
		.collect();
	let generic = |def_id: DefId| tcx.generics_of(def_id).requires_monomorphization(tcx);
	for &def_id in roots.iter().filter(|&&def_id| !generic(def_id)) {
		t.instance(Instance::mono(tcx, def_id));
	}
	t.drain();
	// A generic function no instance reached is lowered over its own type parameters, so
	// what it refuses whatever they are is counted too.
	let mut polymorphic = 0;
	for &def_id in roots.iter().filter(|&&def_id| generic(def_id)) {
		let lowered = |instance: &Instance<'tcx>| instance.def_id() == def_id && !instance.args.has_param();
		if t.instances.keys().any(lowered) {
			continue;
		}
		polymorphic += 1;
		t.env = TypingEnv::post_analysis(tcx, def_id);
		let instance = Instance::new_raw(def_id, ty::GenericArgs::identity_for_item(tcx, def_id));
		let sym = t.instance(instance);
		t.queue.retain(|&(queued, _)| queued != instance);
		t.function(instance, sym);
		t.drain();
	}
	let sm = tcx.sess.source_map();
	let mut seen = rustc_data_structures::fx::FxHashSet::default();
	let mut refusals = Vec::new();
	for (span, message) in t.refusals.take().unwrap_or_default() {
		let (file, line, col, ..) = sm.span_to_location_info(span.source_callsite());
		let file = file.map_or_else(String::new, |file| {
			file.name
				.display(rustc_span::RemapPathScopeComponents::DIAGNOSTICS)
				.to_string()
		});
		let refusal = Refusal {
			file,
			line,
			col,
			message,
		};
		// Each instance of a generic function refuses the same spans.
		if seen.insert(refusal.clone()) {
			refusals.push(refusal);
		}
	}
	Some(Census {
		// Those over type parameters, other than the roots, were named but never lowered.
		functions: t.instances.keys().filter(|instance| !instance.args.has_param()).count() + polymorphic,
		polymorphic,
		refusals,
	})
}

pub struct Translator<'a, 'tcx> {
	pub tcx: TyCtxt<'tcx>,
	bodies: &'a Bodies<'tcx>,
	config: &'a Config,
	pub program: Program,
	instances: rustc_data_structures::fx::FxHashMap<Instance<'tcx>, SymId>,
	queue: VecDeque<(Instance<'tcx>, SymId)>,
	adts: rustc_data_structures::fx::FxHashMap<DefId, Adt>,
	/// Source files by name, as indexes into `program.sources`.
	sources: rustc_data_structures::fx::FxHashMap<String, Option<u32>>,
	failed: bool,
	/// Where trait bounds come from: none outside a census, which lowers some functions
	/// over their own type parameters.
	env: TypingEnv<'tcx>,
	/// Refusals collected for a census, rather than reported as errors.
	refusals: Option<Vec<(Span, String)>>,
	/// The root and printable types, which keep a class or companion in `Shape::Facade`.
	facade_types: rustc_data_structures::fx::FxHashSet<DefId>,
	/// Whether each type's drop does something JS can see, as [`Self::drops`] found.
	drops: rustc_data_structures::fx::FxHashMap<Ty<'tcx>, bool>,
	/// The types [`Self::drops`] is still deciding, and whether the query open now read one.
	drops_open: rustc_data_structures::fx::FxHashSet<Ty<'tcx>>,
	drops_cycle: bool,
	/// The classes given drop glue.
	glued: rustc_data_structures::fx::FxHashSet<DefId>,
}

/// A local struct or enum, and how it is represented.
#[derive(Clone)]
pub struct Adt {
	pub sym: SymId,
	pub repr: AdtRepr,
}

#[derive(Clone)]
pub enum AdtRepr {
	/// A struct: a class, whose fields are named at print time.
	Class { ctor: Ctor, fields: Vec<FieldId> },
	/// An enum whose variants carry no data: a string per variant.
	Tags,
	/// An enum: an object per variant, tagged by its name.
	Union,
}

/// A JS object that a `&mut` can change in place.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
	Class,
	Bytes,
	Array,
	Vec,
	Closure,
}

/// The JS values a type's own can be that `Option` and `Poll` erase to: `None` is
/// `undefined`, and `Pending` is `rt.Pending`.
#[derive(Clone, Copy, Default)]
pub struct Erased {
	pub undefined: bool,
	pub pending: bool,
}

/// Where a translated function lives.
#[derive(Clone, Copy)]
pub enum Owner {
	/// On a class: an instance method if it takes `self`, otherwise static.
	Class(SymId),
	/// On an enum's companion object, taking `self` as its first parameter.
	Companion(SymId),
	/// A module-level function.
	Module,
	/// A module-level function of a type, taking `self` as its first parameter, and
	/// called through the type's facade too if it has one.
	Free { ty: SymId, facade: bool },
}

impl<'a, 'tcx> Translator<'a, 'tcx> {
	fn new(
		tcx: TyCtxt<'tcx>,
		config: &'a Config,
		bodies: &'a Bodies<'tcx>,
		refusals: Option<Vec<(Span, String)>>,
	) -> Option<Self> {
		if tcx.data_layout.pointer_size().bits() != 32 {
			tcx.dcx().err(
				"rust2-ts: translation needs a 32-bit target, such as wasm32-unknown-unknown, so usize is a number",
			);
			return None;
		}
		let mut t = Self {
			tcx,
			bodies,
			config,
			program: Program {
				shape: Shape::from_env(),
				..Program::default()
			},
			instances: Default::default(),
			queue: VecDeque::new(),
			adts: Default::default(),
			sources: Default::default(),
			failed: false,
			env: TypingEnv::fully_monomorphized(),
			refusals,
			drops: Default::default(),
			drops_open: Default::default(),
			drops_cycle: false,
			glued: Default::default(),
			facade_types: Default::default(),
		};
		if t.program.shape == Shape::Facade {
			t.facade_types = (config.roots.iter().chain(&config.printable))
				.filter_map(|path| t.resolve_path(path))
				.filter(|&def_id| matches!(tcx.def_kind(def_id), DefKind::Struct | DefKind::Enum))
				.collect();
		}
		Some(t)
	}

	/// Whether a struct's values are plain objects rather than its class's.
	pub fn plain(&self, sym: SymId) -> bool {
		self.program.plain.contains(&sym)
	}

	/// A struct's value, from its fields' values in declaration order.
	pub fn construct(&self, sym: SymId, ctor: &Ctor, fields: &[FieldId], values: Vec<ts::Expr>) -> ts::Expr {
		let class = Box::new(ts::Expr::Sym(sym));
		let props = |values: Vec<ts::Expr>| fields.iter().zip(values).map(|(f, v)| (ts::Key::Field(*f), v)).collect();
		match ctor {
			_ if self.plain(sym) => ts::Expr::Object(props(values)),
			Ctor::Unit => ts::Expr::New(class, Vec::new()),
			Ctor::Positional => ts::Expr::New(class, values),
			Ctor::Named => ts::Expr::New(class, vec![ts::Expr::Object(props(values))]),
		}
	}

	/// Translate every queued function instance, and those they reach.
	///
	/// A census leaves an instance over type parameters to its own function's turn, since
	/// its bounds are that function's, not the caller's. Translation never reaches one.
	fn drain(&mut self) {
		while let Some((instance, sym)) = self.queue.pop_front() {
			if self.refusals.is_none() || !instance.args.has_param() {
				self.function(instance, sym);
			}
		}
	}

	/// Refuse a construct outside the subset.
	pub fn err(&mut self, span: Span, msg: impl Into<String>) {
		self.failed = true;
		match &mut self.refusals {
			Some(refusals) => refusals.push((span, msg.into())),
			None => _ = self.tcx.dcx().span_err(span, format!("rust2-ts: {}", msg.into())),
		}
	}

	fn typing_env(&self) -> TypingEnv<'tcx> {
		self.env
	}

	/// Whether a census is lowering a function over its own type parameters. Only an
	/// instance decides what a parameter is, so nothing that hinges on one is refused
	/// then: each instance translation reaches is lowered, and refused, on its own.
	pub fn polymorphic(&self) -> bool {
		self.env != TypingEnv::fully_monomorphized()
	}

	/// Whether `ty` is a type parameter, or a projection from one, that only an instance
	/// decides.
	pub fn deferred(&self, ty: Ty<'tcx>) -> bool {
		self.polymorphic() && matches!(ty.kind(), ty::Param(_) | ty::Alias(..))
	}

	pub fn typing_env_pub(&self) -> TypingEnv<'tcx> {
		self.typing_env()
	}

	/// A closure's THIR and body expression.
	pub fn body(&self, def_id: LocalDefId) -> Option<(&'a Thir<'tcx>, ExprId)> {
		self.bodies.map.get(&def_id).map(|(thir, expr)| (thir, *expr))
	}

	fn root(&mut self, path: &str) {
		let Some(def_id) = self.config_item(path) else {
			return;
		};
		match self.tcx.def_kind(def_id) {
			DefKind::Fn => {
				if self.tcx.generics_of(def_id).requires_monomorphization(self.tcx) {
					self.err(self.tcx.def_span(def_id), "a root must not be generic");
					return;
				}
				let instance = Instance::mono(self.tcx, def_id);
				self.instance(instance);
			}
			DefKind::Struct | DefKind::Enum => {
				if !self.config_type(def_id) {
					return;
				}
				let self_ty = self.tcx.type_of(def_id).instantiate_identity().skip_norm_wip();
				// Its inherent methods, and its impls of the crate's own traits: another
				// crate's trait, such as a derived `Debug`, comes along only when called or
				// printable.
				let mut impls: Vec<DefId> = self.tcx.inherent_impls(def_id).to_vec();
				for (trait_id, locals) in self.tcx.all_local_trait_impls(()) {
					if !trait_id.is_local() {
						continue;
					}
					for local in locals {
						let impl_ty = self
							.tcx
							.type_of(local.to_def_id())
							.instantiate_identity()
							.skip_norm_wip();
						if impl_ty == self_ty {
							impls.push(local.to_def_id());
						}
					}
				}
				for imp in impls {
					if self.tcx.generics_of(imp).requires_monomorphization(self.tcx) {
						continue;
					}
					for item in self.tcx.associated_items(imp).in_definition_order() {
						if item.is_fn() && !self.tcx.generics_of(item.def_id).requires_monomorphization(self.tcx) {
							self.instance(Instance::mono(self.tcx, item.def_id));
						}
					}
				}
				self.blanket_impls(self_ty);
			}
			kind => self.err(
				self.tcx.def_span(def_id),
				format!("a root must be a function or a type, not {kind:?}"),
			),
		}
	}

	/// A type whose `Display` and `Debug` impls come along, whichever it has, so callers
	/// can print it.
	fn printable(&mut self, path: &str) {
		let tcx = self.tcx;
		let Some(def_id) = self.config_item(path) else {
			return;
		};
		let kind = tcx.def_kind(def_id);
		if !matches!(kind, DefKind::Struct | DefKind::Enum) {
			self.err(
				tcx.def_span(def_id),
				format!("a printable item must be a type, not {kind:?}"),
			);
			return;
		}
		if !self.config_type(def_id) {
			return;
		}
		let args = ty::GenericArgs::for_item(tcx, def_id, |_, _| tcx.lifetimes.re_erased.into());
		let self_ty = Ty::new_adt(tcx, tcx.adt_def(def_id), args);
		let mut found = false;
		for name in [rustc_span::sym::Display, rustc_span::sym::Debug] {
			let Some(fmt) = tcx.get_diagnostic_item(name).and_then(|trait_id| {
				tcx.associated_items(trait_id)
					.in_definition_order()
					.find(|item| item.is_fn())
			}) else {
				continue;
			};
			if let Ok(Some(instance)) =
				Instance::try_resolve(tcx, self.typing_env(), fmt.def_id, tcx.mk_args(&[self_ty.into()]))
			{
				self.instance(instance);
				found = true;
			}
		}
		if !found {
			self.err(
				tcx.def_span(def_id),
				"a printable type must implement `Display` or `Debug`",
			);
		}
	}

	/// The item a config path names, or `None` after reporting that it names none.
	fn config_item(&mut self, path: &str) -> Option<DefId> {
		let def_id = self.resolve_path(path);
		if def_id.is_none() {
			self.tcx.dcx().err(format!("rust2-ts: no item `{path}` in the crate"));
			self.failed = true;
		}
		def_id
	}

	/// Translate a type the config names, which must not be generic.
	fn config_type(&mut self, def_id: DefId) -> bool {
		if self.tcx.generics_of(def_id).requires_monomorphization(self.tcx) {
			self.err(
				self.tcx.def_span(def_id),
				"a root or printable type must not be generic",
			);
			return false;
		}
		self.adt(self.tcx.adt_def(def_id), rustc_span::DUMMY_SP);
		true
	}

	/// The methods of blanket impls of the crate's traits that hold for `self_ty`, such
	/// as `impl<T: Message> Encode<Version> for T`.
	fn blanket_impls(&mut self, self_ty: Ty<'tcx>) {
		let tcx = self.tcx;
		for (trait_id, locals) in tcx.all_local_trait_impls(()) {
			if !trait_id.is_local() {
				continue;
			}
			for local in locals {
				let imp = local.to_def_id();
				let impl_ty = tcx.type_of(imp).instantiate_identity().skip_norm_wip();
				if !matches!(impl_ty.kind(), ty::Param(_)) {
					continue;
				}
				// Only the self parameter: any other would need choosing.
				let generics = tcx.generics_of(imp);
				let counts = generics.own_counts();
				if counts.types != 1 || counts.consts != 0 || generics.parent.is_some() {
					continue;
				}
				let args = tcx.mk_args_from_iter(generics.own_params.iter().map(|param| -> ty::GenericArg<'tcx> {
					match param.kind {
						ty::GenericParamDefKind::Lifetime => tcx.lifetimes.re_erased.into(),
						_ => self_ty.into(),
					}
				}));
				let trait_ref = tcx.impl_trait_ref(imp).instantiate(tcx, args).skip_norm_wip();
				for item in tcx.associated_items(imp).in_definition_order() {
					if !item.is_fn() || tcx.generics_of(item.def_id).own_requires_monomorphization() {
						continue;
					}
					let Some(trait_item) = item.trait_item_def_id() else {
						continue;
					};
					if let Ok(Some(instance)) =
						Instance::try_resolve(tcx, self.typing_env(), trait_item, trait_ref.args)
					{
						self.instance(instance);
					}
				}
			}
		}
	}

	fn resolve_path(&self, path: &str) -> Option<DefId> {
		let mut module = rustc_hir::def_id::CRATE_DEF_ID;
		let mut segments = path.split("::").peekable();
		while let Some(segment) = segments.next() {
			let child = self
				.tcx
				.module_children_local(module)
				.iter()
				.find(|child| child.ident.as_str() == segment)?;
			let Res::Def(kind, def_id) = child.res else {
				return None;
			};
			if segments.peek().is_none() {
				return Some(def_id);
			}
			if kind != DefKind::Mod {
				return None;
			}
			module = def_id.as_local()?;
		}
		None
	}

	/// The symbol of a function instance, queueing its translation the first time.
	pub fn instance(&mut self, instance: Instance<'tcx>) -> SymId {
		if let Some(&sym) = self.instances.get(&instance) {
			return sym;
		}
		let tcx = self.tcx;
		let def_id = instance.def_id();
		let owner = self.owner(instance);
		let (owner_sym, kind) = match owner {
			Owner::Class(sym) => (
				Some(sym),
				SymKind::Method {
					instance: self.has_self(def_id),
				},
			),
			Owner::Companion(sym) => (Some(sym), SymKind::Method { instance: false }),
			Owner::Module | Owner::Free { .. } => (None, SymKind::Function),
		};
		let base = camel(tcx.item_name(def_id).as_str());
		let suffix = self.suffix(instance);
		let order = self.order(tcx.def_span(def_id));
		// Without trimmed paths, which rustc computes only for a diagnostic.
		let printed = ty::print::with_no_trimmed_paths!(instance.to_string());
		let sym = self.program.sym(Symbol {
			module: match owner {
				Owner::Free { ty, .. } => self.program.syms[ty].module.clone(),
				_ => self.module_of(def_id),
			},
			base: match owner {
				Owner::Module => escape(&base),
				Owner::Free { ty, .. } => prefixed(&self.program.syms[ty].base, &base),
				_ => base.clone(),
			},
			suffix: suffix.clone(),
			owner: owner_sym,
			kind,
			order,
			instance: printed.clone(),
			name: String::new(),
		});
		if let Owner::Free { ty, facade: true } = owner {
			let instance = self.program.syms[ty].kind == SymKind::Class && self.has_self(def_id);
			let method = self.program.sym(Symbol {
				module: self.program.syms[ty].module.clone(),
				base,
				suffix,
				owner: Some(ty),
				kind: SymKind::Method { instance },
				order,
				instance: printed,
				name: String::new(),
			});
			self.program.facades.entry(ty).or_default().push((method, sym));
		}
		self.instances.insert(instance, sym);
		self.queue.push_back((instance, sym));
		sym
	}

	fn has_self(&self, def_id: DefId) -> bool {
		self.tcx
			.opt_associated_item(def_id)
			.is_some_and(|item| item.is_method())
	}

	/// The type an associated function's impl or trait is for.
	pub fn self_ty(&self, instance: Instance<'tcx>) -> Option<Ty<'tcx>> {
		let def_id = instance.def_id();
		if let Some(imp) = self.tcx.impl_of_assoc(def_id) {
			return Some(self.subst(instance.args, self.tcx.type_of(imp).skip_binder()));
		}
		self.tcx.trait_of_assoc(def_id).map(|_| instance.args.type_at(0))
	}

	fn owner(&mut self, instance: Instance<'tcx>) -> Owner {
		let Some(self_ty) = self.self_ty(instance) else {
			return Owner::Module;
		};
		let self_ty = self_ty.peel_refs();
		let ty::Adt(adt, _) = self_ty.kind() else {
			return Owner::Module;
		};
		if !adt.did().is_local() {
			return Owner::Module;
		}
		let span = self.tcx.def_span(instance.def_id());
		let info = self.adt(*adt, span);
		if self.program.shape != Shape::Classes {
			let facade = self.facade_types.contains(&adt.did());
			if facade && !matches!(info.repr, AdtRepr::Class { .. }) {
				self.program.syms[info.sym].kind = SymKind::Union { companion: true };
			}
			return Owner::Free { ty: info.sym, facade };
		}
		match info.repr {
			AdtRepr::Class { .. } => Owner::Class(info.sym),
			AdtRepr::Tags | AdtRepr::Union => {
				self.program.syms[info.sym].kind = SymKind::Union { companion: true };
				Owner::Companion(info.sym)
			}
		}
	}

	/// What tells this instance apart from others of the same name.
	fn suffix(&self, instance: Instance<'tcx>) -> String {
		let tcx = self.tcx;
		let mut parts = Vec::new();
		let def_id = instance.def_id();
		if let Some(imp) = tcx.trait_impl_of_assoc(def_id) {
			let trait_ref = tcx.impl_trait_ref(imp).skip_binder();
			let args = self.subst(instance.args, trait_ref.args);
			parts.extend(args.types().skip(1).map(|t| short_name(tcx, t)));
		}
		// The type whose class or companion this is on goes without saying, but not its
		// arguments, such as a default method's on `Wrap<Pair>`.
		let owner = self
			.self_ty(instance)
			.map(Ty::peel_refs)
			.filter(|t| matches!(t.kind(), ty::Adt(adt, _) if adt.did().is_local()));
		// A closure or function type follows from the others, and has no name.
		let named = |t: &Ty<'tcx>| !matches!(t.kind(), ty::Closure(..) | ty::FnDef(..) | ty::FnPtr(..));
		for t in instance.args.types().filter(named) {
			match (owner == Some(t), t.kind()) {
				(true, ty::Adt(_, args)) => parts.extend(args.types().map(|t| short_name(tcx, t))),
				_ => parts.push(short_name(tcx, t)),
			}
		}
		parts.dedup();
		// Impls of traits without arguments, such as `Display::fmt` and `Debug::fmt`, are
		// told apart by their trait.
		if parts.is_empty()
			&& let Some(imp) = tcx.trait_impl_of_assoc(def_id)
		{
			let trait_id = tcx.impl_trait_ref(imp).skip_binder().def_id;
			parts.push(tcx.item_name(trait_id).to_string());
		}
		parts.join("_")
	}

	/// The Rust module `def_id` is in.
	pub fn module_of(&self, def_id: DefId) -> Vec<String> {
		let Some(local) = def_id.as_local() else {
			return Vec::new();
		};
		let module = self.tcx.parent_module_from_def_id(local).to_def_id();
		self.tcx
			.def_path(module)
			.data
			.iter()
			.filter_map(|part| part.data.get_opt_name().map(|name| name.to_string()))
			.collect()
	}

	/// Where `span` is written, for sorting: where its macro is called, for an item a
	/// macro makes.
	pub fn order(&self, span: Span) -> u32 {
		span.source_callsite().lo().0
	}

	fn source(&mut self, file: &rustc_span::SourceFile) -> Option<u32> {
		let key = format!("{:?}", file.name);
		if let Some(&index) = self.sources.get(&key) {
			return index;
		}
		let index = match (file.name.clone().into_local_path(), file.src.as_deref()) {
			(Some(path), Some(text)) => {
				let path = std::path::absolute(&path).unwrap_or(path);
				self.program.sources.push(Source {
					path,
					text: text.to_string(),
				});
				Some(self.program.sources.len() as u32 - 1)
			}
			_ => None,
		};
		self.sources.insert(key, index);
		index
	}

	/// Where `span` starts, for the source map: the macro call for macro-made code.
	pub fn loc(&mut self, span: Span) -> Option<Loc> {
		if span.is_dummy() {
			return None;
		}
		let span = span.source_callsite();
		let sm = self.tcx.sess.source_map();
		let pos = sm.lookup_char_pos(span.lo());
		let file = self.source(&pos.file)?;
		let text = &self.program.sources[file as usize].text;
		let line_start = pos.file.lines()[pos.line - 1];
		let start = (line_start - rustc_span::RelativeBytePos(0)).0 as usize;
		let offset = (span.lo() - pos.file.start_pos).0 as usize;
		let col = text.get(start..offset).map_or(0, |s| s.encode_utf16().count()) as u32;
		Some(Loc {
			file,
			line: pos.line as u32 - 1,
			col,
		})
	}

	pub fn subst<T: ty::TypeFoldable<TyCtxt<'tcx>>>(&self, args: GenericArgsRef<'tcx>, value: T) -> T {
		self.tcx
			.instantiate_and_normalize_erasing_regions(args, self.typing_env(), EarlyBinder::bind(self.tcx, value))
	}

	/// The standard library type `ty` is, if it is one with its own representation.
	pub fn std_of(&self, ty: Ty<'tcx>) -> Option<library::Std> {
		match ty.kind() {
			ty::Adt(adt, _) => library::Std::of(self.tcx, adt.did()),
			_ => None,
		}
	}

	pub fn is_copy(&self, ty: Ty<'tcx>) -> bool {
		self.tcx.type_is_copy_modulo_regions(self.typing_env(), ty)
	}

	/// A local struct or enum, translating its definition the first time.
	pub fn adt(&mut self, adt: AdtDef<'tcx>, span: Span) -> Adt {
		if let Some(info) = self.adts.get(&adt.did()) {
			return info.clone();
		}
		let tcx = self.tcx;
		let did = adt.did();
		let repr = match () {
			_ if adt.is_struct() => {
				let variant = adt.non_enum_variant();
				// `struct E {}` and `struct E()` construct as `struct E;` does, since a
				// constructor taking an empty `fields` fails `noUnusedParameters`.
				let ctor = match variant.ctor_kind() {
					_ if variant.fields.is_empty() => Ctor::Unit,
					Some(rustc_hir::def::CtorKind::Fn) => Ctor::Positional,
					_ => Ctor::Named,
				};
				AdtRepr::Class {
					ctor,
					fields: Vec::new(),
				}
			}
			_ if adt.is_enum() && adt.variants().iter().all(|v| v.fields.is_empty()) => AdtRepr::Tags,
			_ if adt.is_enum() => AdtRepr::Union,
			_ => {
				self.err(span, format!("unions are not supported: `{}`", tcx.def_path_str(did)));
				AdtRepr::Union
			}
		};
		let kind = match repr {
			AdtRepr::Class { .. } => SymKind::Class,
			_ => SymKind::Union { companion: false },
		};
		let sym = self.program.sym(Symbol {
			module: self.module_of(did),
			base: tcx.item_name(did).to_string(),
			suffix: String::new(),
			owner: None,
			kind,
			order: self.order(tcx.def_span(did)),
			instance: String::new(),
			name: String::new(),
		});
		if matches!(repr, AdtRepr::Class { .. })
			&& self.program.shape != Shape::Classes
			&& !self.facade_types.contains(&did)
		{
			self.program.plain.insert(sym);
		}
		let mut info = Adt { sym, repr };
		if let AdtRepr::Class { fields, .. } = &mut info.repr {
			for (i, field) in adt.non_enum_variant().fields.iter().enumerate() {
				fields.push(self.program.field(sym, field_name(field.name.as_str(), i)));
			}
		}
		self.adts.insert(did, info.clone());

		let params: Vec<String> = tcx
			.generics_of(did)
			.own_params
			.iter()
			.filter_map(|param| match param.kind {
				ty::GenericParamDefKind::Type { .. } => Some(param.name.to_string()),
				ty::GenericParamDefKind::Lifetime => None,
				ty::GenericParamDefKind::Const { .. } => {
					self.err(
						span,
						format!("const generic types are not supported: `{}`", tcx.def_path_str(did)),
					);
					None
				}
			})
			.collect();
		let identity = ty::GenericArgs::identity_for_item(tcx, did);
		let item = match &info.repr {
			AdtRepr::Class { ctor, fields } => {
				let variant = adt.non_enum_variant();
				let fields = (variant.fields.iter().zip(fields))
					.map(|(field, &id)| {
						let span = tcx.def_span(field.did);
						ts::Field {
							id,
							ty: self.ty(field.ty(tcx, identity).skip_norm_wip(), span),
							doc: self.doc(field.did),
						}
					})
					.collect();
				Item::Class {
					sym,
					params,
					fields,
					ctor: ctor.clone(),
					doc: self.doc(did),
				}
			}
			AdtRepr::Tags => Item::Union {
				sym,
				params,
				// An enum without variants has no values, and a union of no strings would
				// print as no type at all.
				ty: match adt.variants().is_empty() {
					true => Type::Builtin("never"),
					false => Type::Union(
						adt.variants()
							.iter()
							.map(|v| Type::StrLit(v.name.to_string()))
							.collect(),
					),
				},
				doc: self.doc(did),
			},
			AdtRepr::Union => {
				let variants = adt
					.variants()
					.iter()
					.map(|v| {
						let mut fields = vec![("tag".to_string(), Type::StrLit(v.name.to_string()))];
						for (i, field) in v.fields.iter().enumerate() {
							let span = tcx.def_span(field.did);
							let ty = self.ty(field.ty(tcx, identity).skip_norm_wip(), span);
							fields.push((field_name(field.name.as_str(), i), ty));
						}
						Type::Object(fields)
					})
					.collect();
				Item::Union {
					sym,
					params,
					ty: Type::Union(variants),
					doc: self.doc(did),
				}
			}
		};
		self.program.items.push(item);
		self.drop_glue(adt, &info, span);
		info
	}

	/// What kind of JS object a `&mut` to `ty` can change in place, if any.
	pub fn object_kind(&self, ty: Ty<'tcx>) -> Option<ObjectKind> {
		match ty.kind() {
			ty::Array(elem, _) | ty::Slice(elem) => match elem.kind() {
				ty::Uint(ty::UintTy::U8) => Some(ObjectKind::Bytes),
				_ => Some(ObjectKind::Array),
			},
			ty::Adt(adt, _) => match library::Std::of(self.tcx, adt.did()) {
				Some(library::Std::Vec) => Some(ObjectKind::Vec),
				Some(
					library::Std::Formatter
					| library::Std::DebugStruct
					| library::Std::DebugTuple
					| library::Std::DebugList
					| library::Std::Arc
					| library::Std::Weak
					| library::Std::Mutex
					| library::Std::MutexGuard
					| library::Std::Waker
					| library::Std::Context
					| library::Std::Pin
					| library::Std::Range
					| library::Std::RangeInclusive
					| library::Std::Peekable,
				) => Some(ObjectKind::Class),
				Some(_) => None,
				None if adt.is_struct() => Some(ObjectKind::Class),
				None => None,
			},
			// Its captures change through it, as a JS function's do.
			ty::Closure(..) => Some(ObjectKind::Closure),
			ty::Coroutine(..) => Some(ObjectKind::Class),
			_ if self.deferred(ty) => Some(ObjectKind::Class),
			_ => None,
		}
	}

	/// The TypeScript type of a Rust type.
	pub fn ty(&mut self, ty: Ty<'tcx>, span: Span) -> Type {
		let tcx = self.tcx;
		match ty.kind() {
			ty::Bool => Type::Builtin("boolean"),
			ty::Int(_) | ty::Uint(_) => num::Int::of(ty).unwrap().ty(),
			ty::Str => Type::Builtin("string"),
			ty::Never => Type::Builtin("never"),
			ty::Tuple(tys) if tys.is_empty() => Type::Builtin("undefined"),
			ty::Tuple(tys) => Type::Tuple(tys.iter().map(|t| self.ty(t, span)).collect()),
			ty::Ref(_, inner, rustc_ast::Mutability::Mut) if self.object_kind(*inner).is_none() => {
				self.err(
					span,
					format!("`{ty}`: a JS value of `{inner}` is a copy, so a reference can't change it"),
				);
				Type::Builtin("unknown")
			}
			ty::Ref(_, inner, _) => self.ty(*inner, span),
			ty::Array(elem, _) | ty::Slice(elem) => match elem.kind() {
				ty::Uint(ty::UintTy::U8) => Type::Builtin("Uint8Array"),
				_ => Type::Array(Box::new(self.ty(*elem, span))),
			},
			ty::Param(param) => Type::Param(param.name.to_string()),
			ty::FnDef(..) | ty::FnPtr(..) => {
				let sig = tcx.instantiate_bound_regions_with_erased(ty.fn_sig(tcx));
				self.fn_ty(sig.inputs(), sig.output(), span)
			}
			ty::Closure(_, args) => {
				let sig = tcx.instantiate_bound_regions_with_erased(args.as_closure().sig());
				let inputs: Vec<Ty<'tcx>> = match sig.inputs() {
					[tupled] => tupled.tuple_fields().iter().collect(),
					other => other.to_vec(),
				};
				self.fn_ty(&inputs, sig.output(), span)
			}
			ty::Adt(adt, args) => self.adt_ty(*adt, args, span),
			ty::Coroutine(_, args) => Type::Rt("Future", vec![self.ty(args.as_coroutine().return_ty(), span)]),
			// `&dyn Debug` and `&dyn Display` are values ready to write.
			ty::Dynamic(preds, ..)
				if preds
					.principal_def_id()
					.and_then(|did| fmt::Trait::of(tcx, did))
					.is_some() =>
			{
				Type::Rt("Fmt", vec![])
			}
			_ if self.deferred(ty) => Type::Builtin("unknown"),
			_ => {
				self.err(span, format!("unsupported type `{ty}`"));
				Type::Builtin("unknown")
			}
		}
	}

	fn fn_ty(&mut self, inputs: &[Ty<'tcx>], output: Ty<'tcx>, span: Span) -> Type {
		let params = inputs.iter().map(|t| self.ty(*t, span)).collect();
		let ret = match output.is_unit() {
			true => Type::Builtin("void"),
			false => self.ty(output, span),
		};
		Type::Func(params, Box::new(ret))
	}

	/// The return type of a function returning `ty`.
	pub fn ret_ty(&mut self, ty: Ty<'tcx>, span: Span) -> Type {
		match ty.is_unit() {
			true => Type::Builtin("void"),
			false => self.ty(ty, span),
		}
	}

	fn adt_ty(&mut self, adt: AdtDef<'tcx>, args: GenericArgsRef<'tcx>, span: Span) -> Type {
		let tcx = self.tcx;
		if let Some(std) = library::Std::of(tcx, adt.did()) {
			return match std {
				library::Std::Option => {
					let inner = args.type_at(0);
					if self.erases_to(inner).undefined {
						self.err(
							span,
							format!(
								"`Option<{inner}>` is not supported: its `None` and `Some` would both be `undefined`"
							),
						);
					}
					Type::Union(vec![self.ty(inner, span), Type::Builtin("undefined")])
				}
				// A lock's result, which is never an error: its guard.
				library::Std::Result if self.std_of(args.type_at(1)) == Some(library::Std::PoisonError) => {
					self.ty(args.type_at(0), span)
				}
				library::Std::Result => Type::Rt(
					"Result",
					vec![self.ty(args.type_at(0), span), self.ty(args.type_at(1), span)],
				),
				library::Std::Vec => match args.type_at(0).kind() {
					ty::Uint(ty::UintTy::U8) => Type::Rt("ByteVec", vec![]),
					_ => Type::Array(Box::new(self.ty(args.type_at(0), span))),
				},
				library::Std::String => Type::Builtin("string"),
				library::Std::Box => self.ty(args.type_at(0), span),
				library::Std::FromUtf8Error => Type::Rt("FromUtf8Error", vec![]),
				library::Std::TryFromIntError => Type::Rt("TryFromIntError", vec![]),
				library::Std::Formatter => Type::Rt("Formatter", vec![]),
				library::Std::Arguments => Type::Rt("Arguments", vec![]),
				library::Std::FmtError => Type::Rt("FmtError", vec![]),
				library::Std::DebugStruct => Type::Rt("DebugStruct", vec![]),
				library::Std::DebugTuple => Type::Rt("DebugTuple", vec![]),
				library::Std::DebugList => Type::Rt("DebugList", vec![]),
				library::Std::AssertKind => Type::Builtin("string"),
				library::Std::Arc => Type::Rt("Arc", vec![self.ty(args.type_at(0), span)]),
				library::Std::Weak => Type::Rt("Weak", vec![self.ty(args.type_at(0), span)]),
				// A guard's first argument is its lifetime.
				library::Std::Mutex | library::Std::MutexGuard => {
					let inner = args.types().next().expect("a guarded type");
					Type::Rt("Mutex", vec![self.ty(inner, span)])
				}
				library::Std::PoisonError => Type::Builtin("never"),
				library::Std::Waker => Type::Rt("Waker", vec![]),
				library::Std::Context => Type::Rt("Context", vec![]),
				library::Std::Poll => {
					let inner = args.type_at(0);
					if self.erases_to(inner).pending {
						self.err(
							span,
							format!("`Poll<{inner}>` is not supported: it would be pending two ways with one value"),
						);
					}
					Type::Rt("Poll", vec![self.ty(inner, span)])
				}
				library::Std::Pin => self.ty(args.type_at(0), span),
				library::Std::Range => Type::Rt("Range", vec![self.ty(args.type_at(0), span)]),
				library::Std::RangeInclusive => Type::Rt("RangeInclusive", vec![self.ty(args.type_at(0), span)]),
				library::Std::Peekable => {
					let item = self.iter_item(Ty::new_adt(tcx, adt, args));
					Type::Rt("Peekable", vec![self.item_ty(item, span)])
				}
			};
		}
		// Every other iterator std has is an `rt.Iter` of its items.
		if !adt.did().is_local()
			&& let Some(item) = self.iter_item(Ty::new_adt(tcx, adt, args))
		{
			return Type::Rt("Iter", vec![self.ty(item, span)]);
		}
		if !adt.did().is_local() {
			self.err(
				span,
				format!("unsupported type `{}` from another crate", tcx.def_path_str(adt.did())),
			);
			return Type::Builtin("unknown");
		}
		let info = self.adt(adt, span);
		let args = args.types().map(|t| self.ty(t, span)).collect();
		Type::Sym(info.sym, args)
	}

	/// The type of an iterator's items, which every iterator has.
	fn item_ty(&mut self, item: Option<Ty<'tcx>>, span: Span) -> Type {
		match item {
			Some(item) => self.ty(item, span),
			None => {
				self.err(span, "an iterator without an item type");
				Type::Builtin("unknown")
			}
		}
	}

	/// Which of the values `Option` and `Poll` erase to a `ty`'s JS value can already be,
	/// seen through the wrappers that erase to their contents.
	pub fn erases_to(&self, ty: Ty<'tcx>) -> Erased {
		match ty.kind() {
			ty::Tuple(tys) if tys.is_empty() => Erased {
				undefined: true,
				pending: false,
			},
			ty::Ref(_, inner, _) => self.erases_to(*inner),
			ty::Adt(adt, args) => match library::Std::of(self.tcx, adt.did()) {
				Some(library::Std::Box | library::Std::Pin) => self.erases_to(args.type_at(0)),
				Some(library::Std::Option) => Erased {
					undefined: true,
					..self.erases_to(args.type_at(0))
				},
				Some(library::Std::Poll) => Erased {
					pending: true,
					..self.erases_to(args.type_at(0))
				},
				_ => Erased::default(),
			},
			_ => Erased::default(),
		}
	}

	/// Translate a queued function instance.
	fn function(&mut self, instance: Instance<'tcx>, sym: SymId) {
		let tcx = self.tcx;
		let def_id = instance.def_id();
		let span = tcx.def_span(def_id);
		let InstanceKind::Item(_) = instance.def else {
			self.err(span, format!("unsupported function `{}`", tcx.def_path_str(def_id)));
			return;
		};
		let Some(local) = def_id.as_local() else {
			self.err(span, format!("`{}` is from another crate", tcx.def_path_str(def_id)));
			return;
		};
		let Some((thir, expr)) = self.bodies.map.get(&local) else {
			self.err(
				span,
				format!(
					"no body for `{}`: constant evaluation consumed it",
					tcx.def_path_str(def_id)
				),
			);
			return;
		};
		let owner = self.owner(instance);
		let method = matches!(owner, Owner::Class(_)) && self.has_self(def_id);
		let function = body::function(self, instance, thir, *expr, method);
		let Some(mut function) = function else {
			return;
		};
		function.sym = sym;
		function.loc = self.loc(span);
		// A trait method's impl often leaves its docs to the trait.
		function.doc = self
			.doc(def_id)
			.or_else(|| tcx.trait_item_of(def_id).and_then(|item| self.doc(item)));
		match owner {
			Owner::Class(owner) | Owner::Companion(owner) => {
				self.program.methods.entry(owner).or_default().push(function)
			}
			Owner::Module | Owner::Free { .. } => self.program.items.push(Item::Function(function)),
		}
	}

	/// The doc comment on an item, as written.
	fn doc(&self, def_id: DefId) -> Option<String> {
		let local = def_id.as_local()?;
		let lines: Vec<String> = self
			.tcx
			.hir_attrs(self.tcx.local_def_id_to_hir_id(local))
			.iter()
			.filter_map(|attr| attr.doc_str())
			.map(|doc| doc.to_string())
			.collect();
		let doc = lines.join("\n");
		(!doc.trim().is_empty()).then_some(doc)
	}

	/// Whether invocations of the macro that made `span` are left out.
	pub fn ignored_macro(&self, mut span: Span) -> bool {
		// Walk out through the macros this one was expanded inside.
		while span.from_expansion() {
			let data = span.ctxt().outer_expn_data();
			if let Some(def_id) = data.macro_def_id {
				let path = self.tcx.def_path_str(def_id);
				if self.config.ignore_macros.contains(&path) {
					return true;
				}
			}
			span = data.call_site;
		}
		false
	}
}

/// A function of a type as a module-level name: `Decoder` and `slice` make `decoderSlice`.
fn prefixed(ty: &str, method: &str) -> String {
	let mut out = String::new();
	let mut chars = ty.chars();
	out.extend(chars.next().map(|c| c.to_ascii_lowercase()));
	out.extend(chars);
	let mut chars = method.chars();
	out.extend(chars.next().map(|c| c.to_ascii_uppercase()));
	out.extend(chars);
	out
}

/// A short name for a type, to tell instances apart.
fn short_name<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> String {
	match ty.kind() {
		// Its arguments, but not defaults such as a `Vec`'s allocator.
		ty::Adt(adt, args) => {
			let args = tcx.generics_of(adt.did()).own_args_no_defaults(tcx, args);
			std::iter::once(tcx.item_name(adt.did()).to_string())
				.chain(args.iter().filter_map(|arg| arg.as_type()).map(|t| short_name(tcx, t)))
				.collect::<Vec<_>>()
				.join("_")
		}
		ty::Ref(_, inner, _) => short_name(tcx, *inner),
		ty::Tuple(tys) => tys.iter().map(|t| short_name(tcx, t)).collect::<Vec<_>>().join("_"),
		ty::Array(elem, _) | ty::Slice(elem) => format!("{}s", short_name(tcx, *elem)),
		_ => ty.to_string().replace(|c: char| !c.is_alphanumeric(), ""),
	}
}

/// `snake_case` to `camelCase`, keeping leading underscores.
pub fn camel(name: &str) -> String {
	let name = name.strip_prefix("r#").unwrap_or(name);
	let trimmed = name.trim_start_matches('_');
	let mut out: String = name[..name.len() - trimmed.len()].to_string();
	let mut upper = false;
	for ch in trimmed.chars() {
		match ch {
			'_' => upper = true,
			c if upper => {
				out.extend(c.to_uppercase());
				upper = false;
			}
			c => out.push(c),
		}
	}
	if trimmed.ends_with('_') {
		out.push('_');
	}
	out
}

/// A field's property name: `camelCase`, or `_0` for a tuple field.
pub fn field_name(name: &str, index: usize) -> String {
	match name.parse::<usize>() {
		Ok(_) => format!("_{index}"),
		Err(_) => camel(name),
	}
}

/// A binding name that is not a JS reserved word.
pub fn escape(name: &str) -> String {
	const RESERVED: &[&str] = &[
		"arguments",
		"await",
		"break",
		"case",
		"catch",
		"class",
		"const",
		"continue",
		"debugger",
		"default",
		"delete",
		"do",
		"else",
		"enum",
		"eval",
		"export",
		"extends",
		"false",
		"finally",
		"for",
		"function",
		"if",
		"implements",
		"import",
		"in",
		"instanceof",
		"interface",
		"let",
		"new",
		"null",
		"package",
		"private",
		"protected",
		"public",
		"return",
		"static",
		"super",
		"switch",
		"this",
		"throw",
		"true",
		"try",
		"typeof",
		"undefined",
		"var",
		"void",
		"while",
		"with",
		"yield",
		"rt",
		// Globals the output calls.
		"Array",
		"Object",
		"Uint8Array",
	];
	match RESERVED.contains(&name) {
		true => format!("{name}_"),
		false => name.to_string(),
	}
}
