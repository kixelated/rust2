//! Rust, as rustc checked it, to the TypeScript AST.
//!
//! Translation starts from the configured roots and follows what they reach. Functions
//! are monomorphized: each instance becomes its own TypeScript function, so a trait
//! call is always a direct call. Types are translated once, generically, and a function of
//! a type is a module-level one taking `self` first, so a bundler drops what goes unused.

mod bind;
pub use bind::typecheck;
mod body;
mod collections;
mod consts;
mod copy;
mod drop;
mod float;
mod fmt;
mod iter;
mod library;
mod mir;
mod num;
mod task;
mod time;

use std::collections::VecDeque;

use rustc_hir::def::DefKind;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::ty::{
	self, AdtDef, EarlyBinder, GenericArgsRef, Instance, InstanceKind, Ty, TyCtxt, TypeVisitableExt, TypingEnv,
};
use rustc_span::Span;

use crate::census::{Census, Refusal};
use crate::config::Config;
use crate::program::{Program, Source, SymKind, Symbol};
use crate::ts::{self, Ctor, FieldId, Item, Loc, SymId, Type};

/// Translate what the config's roots reach, or `None` after reporting why not.
pub fn translate<'tcx>(tcx: TyCtxt<'tcx>, config: &Config) -> Option<Program> {
	let mut t = Translator::new(tcx, config, None)?;
	t.bind();
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
	t.program.merge();
	t.program.name();
	Some(t.program)
}

/// Every construct translation refuses in the crate: in what the config's roots reach, as
/// translation does, or where it names none, in what each item reaches as if each were a
/// root.
pub fn census<'tcx>(tcx: TyCtxt<'tcx>, config: &Config) -> Option<Census> {
	let mut t = Translator::new(tcx, config, Some(Vec::new()))?;
	t.bind();
	let polymorphic = match config.roots.is_empty() {
		true => t.every_item(),
		false => {
			for root in &config.roots {
				t.root(root);
			}
			for path in &config.printable {
				t.printable(path);
			}
			t.drain();
			0
		}
	};
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

impl<'tcx> Translator<'_, 'tcx> {
	/// Lower what each of the crate's items reaches as if each were a root, returning how
	/// many generic functions no instance reached, lowered over their type parameters.
	fn every_item(&mut self) -> usize {
		let tcx = self.tcx;
		let t = self;
		for def_id in tcx.hir_crate_items(()).definitions() {
			if matches!(tcx.def_kind(def_id), DefKind::Struct | DefKind::Enum | DefKind::Union)
				&& !t.bound(def_id.to_def_id())
			{
				t.adt(tcx.adt_def(def_id), tcx.def_span(def_id));
			}
		}
		let functions = |tcx: TyCtxt<'tcx>| {
			tcx.hir_body_owners()
				.filter(move |&def_id| matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn))
		};
		// A derived impl comes along only when called or printable, as in translation.
		for path in &t.config.printable {
			t.printable(path);
		}
		let roots: Vec<DefId> = functions(tcx)
			.map(LocalDefId::to_def_id)
			.filter(|&def_id| {
				!tcx.impl_of_assoc(def_id)
					.is_some_and(|imp| tcx.is_automatically_derived(imp))
					&& !t.bound(def_id)
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
		polymorphic
	}
}

pub struct Translator<'a, 'tcx> {
	pub tcx: TyCtxt<'tcx>,
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
	/// Whether each type's drop does something JS can see, as [`Self::drops`] found.
	drops: rustc_data_structures::fx::FxHashMap<Ty<'tcx>, bool>,
	/// The types [`Self::drops`] is still deciding, and whether the query open now read one.
	drops_open: rustc_data_structures::fx::FxHashSet<Ty<'tcx>>,
	drops_cycle: bool,
	/// The classes given drop glue.
	glued: rustc_data_structures::fx::FxHashSet<DefId>,
	/// The drop glue made from each type's drop shim.
	pub drop_shims: rustc_data_structures::fx::FxHashMap<Ty<'tcx>, SymId>,
	/// The function copying each `Copy` aggregate type.
	copy_shims: rustc_data_structures::fx::FxHashMap<Ty<'tcx>, SymId>,
	/// The types the crate writes into in place, which alone are copied.
	writes: copy::Writes<'tcx>,
	/// The class of each coroutine instance.
	pub coroutines: rustc_data_structures::fx::FxHashMap<Instance<'tcx>, mir::coroutine::Info<'tcx>>,
	/// Whether [`Self::ty`] is typing a variable: see [`Self::held_ty`].
	held: bool,
	/// Where the crate first reached each instance of std's that lowers from its MIR, which
	/// refuses there: its own spans are std's.
	sites: rustc_data_structures::fx::FxHashMap<Instance<'tcx>, Span>,
	/// While such an instance lowers, that span and its function.
	site: Option<(Span, DefId)>,
	/// The hand-written TypeScript the config binds items to.
	bindings: bind::Bindings<'tcx>,
}

/// A local struct or enum, and how it is represented.
#[derive(Clone)]
pub struct Adt {
	pub sym: SymId,
	pub repr: AdtRepr,
}

#[derive(Clone)]
pub enum AdtRepr {
	/// A struct: a plain object, or a class if it may drop, whose fields are named at print time.
	Struct { ctor: Ctor, fields: Vec<FieldId> },
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
/// `undefined`, or `rt.None` where `Some` can already be `undefined`, and `Pending` is
/// `rt.Pending`.
#[derive(Clone, Copy, Default)]
pub struct Erased {
	pub undefined: bool,
	pub none: bool,
	pub pending: bool,
	/// A type parameter's value, which each instance decides.
	pub param: bool,
}

impl Erased {
	/// What an `Option` of a type erased so erases to: its `None` is `undefined`, or
	/// `rt.None` where `Some` can already be `undefined`.
	fn option(self) -> Self {
		match self.undefined {
			true => Self { none: true, ..self },
			false => Self {
				undefined: true,
				..self
			},
		}
	}
}

impl<'a, 'tcx> Translator<'a, 'tcx> {
	fn new(tcx: TyCtxt<'tcx>, config: &'a Config, refusals: Option<Vec<(Span, String)>>) -> Option<Self> {
		if tcx.data_layout.pointer_size().bits() != 32 {
			tcx.dcx().err(
				"rust2-ts: translation needs a 32-bit target, such as wasm32-unknown-unknown, so usize is a number",
			);
			return None;
		}
		Some(Self {
			tcx,
			config,
			program: Program::default(),
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
			coroutines: Default::default(),
			drop_shims: Default::default(),
			copy_shims: Default::default(),
			// Once the bindings are known.
			writes: Default::default(),
			held: false,
			sites: Default::default(),
			site: None,
			bindings: Default::default(),
		})
	}

	/// Whether a struct's values are plain objects rather than its class's.
	pub fn plain(&self, sym: SymId) -> bool {
		self.program.plain.contains(&sym)
	}

	/// A struct's value, from its fields' values in declaration order.
	pub fn construct(&self, sym: SymId, ctor: &Ctor, fields: &[FieldId], values: Vec<ts::Expr>) -> ts::Expr {
		let class = Box::new(ts::Expr::Sym(sym));
		let props = |values: Vec<ts::Expr>| {
			fields
				.iter()
				.zip(values)
				.map(|(f, v)| (ts::Key::Field(*f), v))
				.collect()
		};
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

	/// Refuse a construct outside the subset: inside a body of std's, at the crate's call.
	pub fn err(&mut self, span: Span, msg: impl Into<String>) {
		self.failed = true;
		let (span, msg) = match self.site {
			Some((site, def_id)) => (site, format!("{}, in `{}`", msg.into(), self.tcx.def_path_str(def_id))),
			None => (span, msg.into()),
		};
		match &mut self.refusals {
			Some(refusals) => refusals.push((span, msg)),
			None => _ = self.tcx.dcx().span_err(span, format!("rust2-ts: {msg}")),
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
		let segments: Vec<&str> = path.split("::").collect();
		let def_id = bind::resolve_in(self.tcx, rustc_hir::def_id::CRATE_DEF_ID.to_def_id(), &segments)
			.filter(|def_id| def_id.is_local());
		let Some(def_id) = def_id else {
			self.tcx.dcx().err(format!("rust2-ts: no item `{path}` in the crate"));
			self.failed = true;
			return None;
		};
		if self.bound(def_id) {
			self.err(self.tcx.def_span(def_id), "a root or printable item can't be bound");
			return None;
		}
		Some(def_id)
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

	/// The symbol of a function instance, queueing its translation the first time, or the
	/// export of a binding that stands in for it.
	pub fn instance(&mut self, instance: Instance<'tcx>) -> SymId {
		if let Some(&sym) = self.instances.get(&instance) {
			return sym;
		}
		let tcx = self.tcx;
		let def_id = instance.def_id();
		if let Some(sym) = self.bound_fn(instance, tcx.def_span(def_id)) {
			return sym;
		}
		let base = camel(tcx.item_name(def_id).as_str());
		// A function of a local type sits in the type's own module, escaped only where it is
		// declared, so callers call it by its own name: `Decoder.new`.
		let (module, base) = match self.owner(instance) {
			Some(ty) => (self.program.syms[ty].module.clone(), base),
			None => (self.module_of(def_id), escape(&base)),
		};
		let sym = self.program.sym(Symbol {
			module,
			base,
			suffix: self.suffix(instance),
			owner: None,
			kind: SymKind::Function,
			order: self.order(tcx.def_span(def_id)),
			// Without trimmed paths, which rustc computes only for a diagnostic.
			instance: ty::print::with_no_trimmed_paths!(instance.to_string()),
			name: String::new(),
			alias: None,
		});
		self.instances.insert(instance, sym);
		self.queue.push_back((instance, sym));
		sym
	}

	/// The symbol of an instance a call at `span` reaches, noting where the crate first
	/// reached one of std's.
	pub fn callee(&mut self, instance: Instance<'tcx>, span: Span) -> SymId {
		if !instance.def_id().is_local() {
			let site = self.site.map_or(span, |(site, _)| site);
			self.sites.entry(instance).or_insert(site);
		}
		self.instance(instance)
	}

	/// The type an associated function's impl or trait is for.
	pub fn self_ty(&self, instance: Instance<'tcx>) -> Option<Ty<'tcx>> {
		let def_id = instance.def_id();
		if let Some(imp) = self.tcx.impl_of_assoc(def_id) {
			return Some(self.subst(instance.args, self.tcx.type_of(imp).skip_binder()));
		}
		self.tcx.trait_of_assoc(def_id).map(|_| instance.args.type_at(0))
	}

	/// The local type an associated function's impl or trait is for, if any.
	fn owner(&mut self, instance: Instance<'tcx>) -> Option<SymId> {
		let self_ty = self.self_ty(instance)?.peel_refs();
		let ty::Adt(adt, _) = self_ty.kind() else {
			return None;
		};
		if !adt.did().is_local() {
			return None;
		}
		let span = self.tcx.def_span(instance.def_id());
		Some(self.adt(*adt, span).sym)
	}

	/// What tells this instance apart from others of its item: the trait it implements, then
	/// the type arguments its type and itself take, as `Display_Pair` for `Wrap<Pair>`'s
	/// `Display::fmt`. It is printed whether or not another instance is reached, so reaching
	/// one renames nothing; empty for a function that is neither generic, of a generic type,
	/// nor of a trait.
	fn suffix(&self, instance: Instance<'tcx>) -> String {
		let tcx = self.tcx;
		let def_id = instance.def_id();
		let mut parts = Vec::new();
		let trait_ref = match tcx.trait_impl_of_assoc(def_id) {
			Some(imp) => Some(self.subst(instance.args, tcx.impl_trait_ref(imp).skip_binder())),
			// A trait's own method, which an impl did not override.
			None => tcx
				.trait_of_assoc(def_id)
				.map(|trait_id| ty::TraitRef::from_assoc(tcx, trait_id, instance.args)),
		};
		if let Some(trait_ref) = trait_ref {
			parts.push(tcx.item_name(trait_ref.def_id).to_string());
			// Without defaults, as `PartialEq`'s `Self`.
			let args = tcx
				.generics_of(trait_ref.def_id)
				.own_args_no_defaults(tcx, trait_ref.args);
			parts.extend(args.iter().filter_map(|arg| arg.as_type()).map(|t| short_name(tcx, t)));
		}
		// The type whose module this is in goes without saying, but not its arguments; any
		// other self type, such as `u64`'s in a trait impl, is spelled out.
		if let Some(self_ty) = self.self_ty(instance) {
			match self_ty.peel_refs().kind() {
				ty::Adt(adt, args) if adt.did().is_local() => parts.extend(args.types().map(|t| short_name(tcx, t))),
				_ => parts.push(short_name(tcx, self_ty)),
			}
		}
		let own = tcx.generics_of(def_id).parent_count;
		for arg in &instance.args[own..] {
			match arg.kind() {
				ty::GenericArgKind::Type(t) => parts.push(short_name(tcx, t)),
				ty::GenericArgKind::Const(c) => parts.push(c.to_string().replace(|c: char| !c.is_alphanumeric(), "")),
				ty::GenericArgKind::Lifetime(_) => {}
			}
		}
		parts.join("_")
	}

	/// The Rust module `def_id` is in, from the crate root; another crate's is under a
	/// directory of its name, as `core/option` for `Option::map`.
	pub fn module_of(&self, def_id: DefId) -> Vec<String> {
		let tcx = self.tcx;
		let mut module = tcx.parent(def_id);
		while tcx.def_kind(module) != DefKind::Mod {
			module = tcx.parent(module);
		}
		let krate = (!def_id.is_local()).then(|| tcx.crate_name(def_id.krate).to_string());
		let path = tcx.def_path(module).data;
		krate
			.into_iter()
			.chain(
				path.iter()
					.filter_map(|part| part.data.get_opt_name().map(|name| name.to_string())),
			)
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

	/// Whether a call of `def_id` with `args` runs a function whose body translates, the
	/// crate's own or std's from its MIR, rather than a mapping.
	pub fn translated(&self, def_id: DefId, args: GenericArgsRef<'tcx>) -> bool {
		match Instance::try_resolve(self.tcx, self.typing_env(), def_id, args) {
			Ok(Some(instance)) => self.lowers(instance),
			_ => false,
		}
	}

	/// Whether `instance`'s body lowers from its MIR: the crate's own function, unless a
	/// binding stands in for it, or one of std's that `library` leaves to its MIR.
	pub fn lowers(&self, instance: Instance<'tcx>) -> bool {
		match instance.def {
			InstanceKind::Item(did) => {
				self.bound_module(instance).is_none() && (did.is_local() || library::from_mir(self, instance))
			}
			_ => false,
		}
	}

	/// Whether a call of `def_id` with `args` runs std's mapping, which takes a `&mut`
	/// parameter as what it points at, rather than a body that translates or a binding's export.
	pub fn mapped(&self, def_id: DefId, args: GenericArgsRef<'tcx>) -> bool {
		match Instance::try_resolve(self.tcx, self.typing_env(), def_id, args) {
			Ok(Some(instance)) => !self.lowers(instance) && self.bound_module(instance).is_none(),
			_ => true,
		}
	}

	/// Whether a call of `def_id` with `args` calls a binding's export.
	pub fn binds(&self, def_id: DefId, args: GenericArgsRef<'tcx>) -> bool {
		match Instance::try_resolve(self.tcx, self.typing_env(), def_id, args) {
			Ok(Some(instance)) => self.bound_module(instance).is_some(),
			_ => false,
		}
	}

	/// A local struct or enum, translating its definition the first time.
	pub fn adt(&mut self, adt: AdtDef<'tcx>, span: Span) -> Adt {
		if let Some(info) = self.adts.get(&adt.did()) {
			return info.clone();
		}
		// Its fields' types are refused where they are written, whatever reached it.
		let held = std::mem::replace(&mut self.held, false);
		let info = self.define(adt, span);
		self.held = held;
		info
	}

	fn define(&mut self, adt: AdtDef<'tcx>, span: Span) -> Adt {
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
				AdtRepr::Struct {
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
			AdtRepr::Struct { .. } => SymKind::Struct,
			_ => SymKind::Union,
		};
		let name = tcx.item_name(did).to_string();
		let sym = self.program.sym(Symbol {
			module: crate::program::type_module(&self.module_of(did), &name),
			base: name,
			suffix: String::new(),
			owner: None,
			kind,
			order: self.order(tcx.def_span(did)),
			instance: String::new(),
			name: String::new(),
			alias: None,
		});
		// A struct is a plain object, unless it may drop: then it is a class, whose
		// `[Symbol.dispose]` whatever holds it calls without knowing its type. One holding a
		// type parameter may, through an instance.
		if matches!(repr, AdtRepr::Struct { .. })
			&& !self.drops(tcx.type_of(did).instantiate_identity().skip_norm_wip())
		{
			self.program.plain.insert(sym);
		}
		let mut info = Adt { sym, repr };
		if let AdtRepr::Struct { fields, .. } = &mut info.repr {
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
			AdtRepr::Struct { ctor, fields } => {
				let variant = adt.non_enum_variant();
				let fields = (variant.fields.iter().zip(fields))
					.map(|(field, &id)| {
						let span = tcx.def_span(field.did);
						ts::Field {
							id,
							ty: self.ty(field.ty(tcx, identity).skip_norm_wip(), span),
							doc: self.doc(field.did),
							init: ts::FieldInit::Ctor,
						}
					})
					.collect();
				Item::Struct {
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

	/// Whether a `ty`'s JS type is a union of its variants', which TypeScript narrows a
	/// variable to the member it was assigned.
	pub fn narrows(&self, ty: Ty<'tcx>) -> bool {
		match ty.kind() {
			ty::Adt(adt, _) => match library::Std::of(self.tcx, adt.did()) {
				Some(
					library::Std::Option
					| library::Std::Poll
					| library::Std::Result
					| library::Std::ControlFlow
					| library::Std::Ordering,
				) => true,
				Some(_) => false,
				None => adt.is_enum() && adt.variants().len() > 1,
			},
			_ => false,
		}
	}

	/// What kind of JS object a `&mut` to `ty` can change in place, if any.
	pub fn object_kind(&self, ty: Ty<'tcx>) -> Option<ObjectKind> {
		match ty.kind() {
			ty::Array(elem, _) | ty::Slice(elem) => match elem.kind() {
				ty::Uint(ty::UintTy::U8) => Some(ObjectKind::Bytes),
				_ => Some(ObjectKind::Array),
			},
			// A JS array, whose elements change in place as an array's do.
			ty::Tuple(tys) if !tys.is_empty() => Some(ObjectKind::Array),
			// A binding's value: an object, unless Rust copies it, which JS can't.
			ty::Adt(adt, _) if self.bound_drops(adt.did()).is_some() => {
				(!self.is_copy(ty)).then_some(ObjectKind::Class)
			}
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
					| library::Std::PollFn
					| library::Std::Pending
					| library::Std::Range
					| library::Std::RangeInclusive
					| library::Std::Peekable
					| library::Std::HashMap
					| library::Std::HashSet
					| library::Std::BTreeMap
					| library::Std::BTreeSet
					| library::Std::VecDeque
					| library::Std::Entry
					| library::Std::Hasher,
				) => Some(ObjectKind::Class),
				Some(_) => None,
				None if adt.is_struct() => Some(ObjectKind::Class),
				None => None,
			},
			// Its captures change through it, as a JS function's do.
			ty::Closure(..) => Some(ObjectKind::Closure),
			ty::Coroutine(..) => Some(ObjectKind::Class),
			ty::Dynamic(..) if self.dyn_future(ty).is_some() => Some(ObjectKind::Class),
			_ if self.deferred(ty) => Some(ObjectKind::Class),
			_ => None,
		}
	}

	/// Whether a JS value of `ty` is a copy that no JS value can point at, such as a number,
	/// so a `&mut` to it is an `rt.Place`: where the value is, read and written through `v`.
	/// A reference's referent is the object it points at, or a copy itself.
	pub fn needs_place(&self, ty: Ty<'tcx>) -> bool {
		self.object_kind(ty).is_none() && !ty.is_ref()
	}

	/// What `ty` points at, if it is a `&mut` that is an `rt.Place`.
	pub fn place_pointee(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
		match ty.kind() {
			ty::Ref(_, inner, rustc_ast::Mutability::Mut) if self.needs_place(*inner) => Some(*inner),
			_ => None,
		}
	}

	/// What `value`, a `ty` that is a reference, points at: an `rt.Place`'s `v`, and
	/// otherwise the reference's own JS value.
	pub fn pointee_value(&self, ty: Ty<'tcx>, value: ts::Expr) -> ts::Expr {
		match self.place_pointee(ty) {
			Some(_) => value.member("v"),
			None => value,
		}
	}

	/// Whether `ty` holds a `&mut` that is an `rt.Place`, other than in what a closure or
	/// coroutine captured.
	pub fn has_place(&self, ty: Ty<'tcx>) -> bool {
		let mut walker = ty.walk();
		while let Some(arg) = walker.next() {
			let Some(t) = arg.as_type() else { continue };
			if matches!(t.kind(), ty::Closure(..) | ty::Coroutine(..) | ty::CoroutineClosure(..)) {
				walker.skip_current_subtree();
			} else if self.place_pointee(t).is_some() {
				return true;
			}
		}
		false
	}

	/// The TypeScript type of a Rust type.
	pub fn ty(&mut self, ty: Ty<'tcx>, span: Span) -> Type {
		let tcx = self.tcx;
		match ty.kind() {
			ty::Bool => Type::Builtin("boolean"),
			ty::Int(_) | ty::Uint(_) => num::Int::of(ty).unwrap().ty(),
			_ if float::is(ty) => float::ty(),
			ty::Char => Type::Rt("char", Vec::new()),
			ty::Str => Type::Builtin("string"),
			ty::Never => Type::Builtin("never"),
			ty::Tuple(tys) if tys.is_empty() => Type::Builtin("undefined"),
			ty::Tuple(tys) => Type::Tuple(tys.iter().map(|t| self.ty(t, span)).collect()),
			ty::Ref(_, inner, rustc_ast::Mutability::Mut) if self.needs_place(*inner) => {
				Type::Rt("Place", vec![self.ty(*inner, span)])
			}
			ty::Ref(_, inner, rustc_ast::Mutability::Mut) if self.object_kind(*inner).is_none() => {
				self.err(
					span,
					format!("`{ty}`: a JS value of `{inner}` is a copy, so a reference can't change it"),
				);
				Type::Builtin("unknown")
			}
			ty::Ref(..) if bytes_ref(ty) => Type::Rt("ByteSlice", vec![]),
			ty::Ref(_, inner, _) => self.ty(*inner, span),
			ty::Array(elem, _) => match elem.kind() {
				ty::Uint(ty::UintTy::U8) => Type::Builtin("Uint8Array"),
				_ => Type::Array(Box::new(self.ty(*elem, span))),
			},
			ty::Slice(elem) => match elem.kind() {
				ty::Uint(ty::UintTy::U8) => Type::Rt("ByteSlice", vec![]),
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
			ty::Pat(base, _) => self.ty(*base, span),
			ty::Coroutine(_, args) => Type::Rt("Future", vec![self.ty(args.as_coroutine().return_ty(), span)]),
			ty::Dynamic(..) if let Some(output) = self.dyn_future(ty) => {
				Type::Rt("Future", vec![self.ty(output, span)])
			}
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
				self.unrepresented(span, format!("unsupported type `{ty}`"));
				Type::Builtin("unknown")
			}
		}
	}

	/// The TypeScript type of a variable that holds a `ty`. It refuses no type without a JS
	/// value, which is refused where a value of it is made or received: a parameter, a
	/// field, a call, or a constant.
	pub fn held_ty(&mut self, ty: Ty<'tcx>, span: Span) -> Type {
		let held = std::mem::replace(&mut self.held, true);
		let ts = self.ty(ty, span);
		self.held = held;
		ts
	}

	/// Refuse a type without a JS value, unless typing a variable.
	fn unrepresented(&mut self, span: Span, msg: String) {
		if !self.held {
			self.err(span, msg);
		}
	}

	/// Refuse each type in `ty` whose JS value can't tell two of its values apart, as an
	/// `Option` of what can already be `undefined`.
	pub fn check_erased(&mut self, ty: Ty<'tcx>, span: Span) {
		let mut msgs: Vec<String> = Vec::new();
		for part in ty.walk().filter_map(|arg| arg.as_type()) {
			if let Some(msg) = self.erasure_conflict(part)
				&& !msgs.contains(&msg)
			{
				msgs.push(msg);
			}
		}
		for msg in msgs {
			self.err(span, msg);
		}
	}

	/// Why a `ty`'s JS value, not looking inside its arguments, can't tell two of its
	/// values apart, if it can't.
	fn erasure_conflict(&self, ty: Ty<'tcx>) -> Option<String> {
		let ty::Adt(adt, args) = ty.kind() else {
			return None;
		};
		match library::Std::of(self.tcx, adt.did())? {
			library::Std::Option => {
				let inner = args.type_at(0);
				let erased = self.erases_to(inner);
				(erased.undefined && erased.none).then(|| {
					format!("`Option<{inner}>` is not supported: its `None` and `Some` would both be `rt.None`")
				})
			}
			library::Std::Poll => {
				let inner = args.type_at(0);
				self.erases_to(inner)
					.pending
					.then(|| format!("`Poll<{inner}>` is not supported: it would be pending two ways with one value"))
			}
			library::Std::ControlFlow if library::Flow::of(self.tcx, args) == library::Flow::Option => {
				let erased = self.erases_to(args.type_at(1));
				(erased.undefined && erased.none)
					.then(|| format!("`{ty}` is not supported: its break and its continue would both be `rt.None`"))
			}
			_ => None,
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
		if let Some(sym) = self.bound_type(adt, args, span) {
			let args = args.types().map(|t| self.ty(t, span)).collect();
			return Type::Sym(sym, args);
		}
		if let Some(std) = library::Std::of(tcx, adt.did()) {
			return match std {
				library::Std::Option => {
					if let Some(msg) = self.erasure_conflict(Ty::new_adt(tcx, adt, args)) {
						self.err(span, msg);
					}
					self.option_ty(args.type_at(0), span)
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
				library::Std::Cow if library::string(tcx, Ty::new_adt(tcx, adt, args)) => Type::Builtin("string"),
				library::Std::Cow => {
					self.unrepresented(span, format!("`{}` is not supported yet", Ty::new_adt(tcx, adt, args)));
					Type::Builtin("unknown")
				}
				library::Std::Box | library::Std::Reverse | library::Std::NonZero => self.ty(args.type_at(0), span),
				library::Std::Bound => Type::Rt("RangeBound", vec![self.ty(args.type_at(0), span)]),
				library::Std::FromUtf8Error => Type::Rt("FromUtf8Error", vec![]),
				library::Std::Utf8Error => Type::Rt("Utf8Error", vec![]),
				library::Std::TryFromIntError => Type::Rt("TryFromIntError", vec![]),
				library::Std::TryFromFloatSecsError => Type::Rt("TryFromFloatSecsError", vec![]),
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
					if let Some(msg) = self.erasure_conflict(Ty::new_adt(tcx, adt, args)) {
						self.err(span, msg);
					}
					Type::Rt("Poll", vec![self.ty(args.type_at(0), span)])
				}
				library::Std::Pin => self.ty(args.type_at(0), span),
				library::Std::PollFn | library::Std::Pending => {
					let output = self.future_output(Ty::new_adt(tcx, adt, args), span);
					Type::Rt("Future", vec![self.ty(output, span)])
				}
				library::Std::Range => Type::Rt("Range", vec![self.ty(args.type_at(0), span)]),
				library::Std::RangeInclusive => Type::Rt("RangeInclusive", vec![self.ty(args.type_at(0), span)]),
				library::Std::RangeTo => Type::Object(vec![("end".into(), self.ty(args.type_at(0), span))]),
				library::Std::RangeToInclusive => Type::Object(vec![
					("end".into(), self.ty(args.type_at(0), span)),
					("inclusive".into(), Type::Builtin("true")),
				]),
				library::Std::RangeFrom => Type::Object(vec![("start".into(), self.ty(args.type_at(0), span))]),
				library::Std::RangeFull => Type::Builtin("undefined"),
				library::Std::Argument => Type::Rt("Fmt", vec![]),
				library::Std::NonNull => self.ty(args.type_at(0), span),
				library::Std::Peekable => {
					let item = self.iter_item(Ty::new_adt(tcx, adt, args));
					Type::Rt("Peekable", vec![self.item_ty(item, span)])
				}
				library::Std::Infallible => Type::Builtin("never"),
				// The key and value types, or the item type, without the hasher or allocator.
				library::Std::HashMap
				| library::Std::BTreeMap
				| library::Std::Entry
				| library::Std::HashSet
				| library::Std::BTreeSet
				| library::Std::VecDeque => {
					let (name, n) = match std {
						library::Std::HashMap => ("HashMap", 2),
						library::Std::BTreeMap => ("BTreeMap", 2),
						library::Std::Entry => ("Entry", 2),
						library::Std::HashSet => ("HashSet", 1),
						library::Std::BTreeSet => ("BTreeSet", 1),
						_ => ("VecDeque", 1),
					};
					let tys: Vec<Ty<'tcx>> = args.types().take(n).collect();
					Type::Rt(name, tys.into_iter().map(|t| self.ty(t, span)).collect())
				}
				library::Std::Ordering => Type::Rt("Ordering", vec![]),
				library::Std::Hasher => Type::Rt("Hasher", vec![]),
				library::Std::Duration => Type::Rt("Duration", vec![]),
				library::Std::ControlFlow => {
					if let Some(msg) = self.erasure_conflict(Ty::new_adt(tcx, adt, args)) {
						self.err(span, msg);
					}
					let (residual, value) = (args.type_at(0), args.type_at(1));
					match library::Flow::of(tcx, args) {
						library::Flow::Result => {
							let ty::Adt(_, residual) = residual.kind() else {
								unreachable!("a `Result` residual")
							};
							Type::Rt("Result", vec![self.ty(value, span), self.ty(residual.type_at(1), span)])
						}
						library::Flow::Option => self.option_ty(value, span),
						library::Flow::Other => Type::Rt("Result", vec![self.ty(value, span), self.ty(residual, span)]),
					}
				}
			};
		}
		if let Some(inner) = library::wrapped(tcx, Ty::new_adt(tcx, adt, args)) {
			return self.ty(inner, span);
		}
		// Every other iterator std has is an `rt.Iter` of its items.
		if !adt.did().is_local()
			&& let Some(item) = self.iter_item(Ty::new_adt(tcx, adt, args))
		{
			return Type::Rt("Iter", vec![self.ty(item, span)]);
		}
		if !adt.did().is_local() {
			self.unrepresented(
				span,
				format!("unsupported type `{}` from another crate", tcx.def_path_str(adt.did())),
			);
			return Type::Builtin("unknown");
		}
		let info = self.adt(adt, span);
		if let Some(msg) = self.generic_none(adt, args) {
			self.err(span, msg);
		}
		let args = args.types().map(|t| self.ty(t, span)).collect();
		Type::Sym(info.sym, args)
	}

	/// Why an instance of the crate's generic `adt` with `args` can't hold its fields' values,
	/// if it can't: an `Option` over a type parameter that its definition types `T |
	/// undefined`, whose `None` this instance's `T` makes `rt.None`.
	fn generic_none(&self, adt: AdtDef<'tcx>, args: GenericArgsRef<'tcx>) -> Option<String> {
		let tcx = self.tcx;
		if !args.types().any(|t| self.erases_to(t).undefined) {
			return None;
		}
		adt.all_fields().find_map(|field| {
			let generic = tcx.type_of(field.did).instantiate_identity().skip_norm_wip();
			generic.walk().filter_map(|arg| arg.as_type()).find_map(|part| {
				let ty::Adt(option, option_args) = part.kind() else {
					return None;
				};
				if library::Std::of(tcx, option.did()) != Some(library::Std::Option) {
					return None;
				}
				let inner = option_args.type_at(0);
				let erased = self.erases_to(inner);
				if !erased.param || erased.undefined {
					return None;
				}
				let instance = ty::EarlyBinder::bind(tcx, inner).instantiate(tcx, args).skip_norm_wip();
				self.erases_to(instance).undefined.then(|| {
					let ty = Ty::new_adt(tcx, adt, args);
					format!(
						"`{ty}` is not supported: its field `{}` is `Option<{inner}>`, typed `{inner} | undefined`, whose `None` would be `rt.None` as `Option<{instance}>`",
						field.name
					)
				})
			})
		})
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

	/// The TypeScript type of an `Option<inner>`: `T | undefined`, or `T | rt.None` where
	/// `T` can already be `undefined`.
	fn option_ty(&mut self, inner: Ty<'tcx>, span: Span) -> Type {
		let none = match self.erases_to(inner).undefined {
			true => Type::Rt("None", vec![]),
			false => Type::Builtin("undefined"),
		};
		Type::Union(vec![self.ty(inner, span), none])
	}

	/// The JS value of `None` in an `Option<inner>`, and of `Break` in a `ControlFlow`
	/// that `?` on one makes: `undefined`, or `rt.None` where `inner` can already be
	/// `undefined`.
	pub fn none(&self, inner: Ty<'tcx>) -> ts::Expr {
		self.none_arg(inner).unwrap_or_else(ts::Expr::undefined)
	}

	/// `rt.None`, if it is an `Option<inner>`'s `None`, which a runtime function taking
	/// one takes after its own arguments.
	pub fn none_arg(&self, inner: Ty<'tcx>) -> Option<ts::Expr> {
		self.erases_to(inner).undefined.then(|| ts::Expr::Rt("None".into()))
	}

	/// Which of the values `Option` and `Poll` erase to a `ty`'s JS value can already be,
	/// seen through the wrappers that erase to their contents.
	pub fn erases_to(&self, ty: Ty<'tcx>) -> Erased {
		match ty.kind() {
			ty::Tuple(tys) if tys.is_empty() => Erased {
				undefined: true,
				..Erased::default()
			},
			ty::Param(_) => Erased {
				param: true,
				..Erased::default()
			},
			// An `rt.Place`, which is an object.
			ty::Ref(..) if self.place_pointee(ty).is_some() => Erased::default(),
			ty::Ref(_, inner, _) => self.erases_to(*inner),
			ty::Adt(adt, args) => match library::Std::of(self.tcx, adt.did()) {
				Some(library::Std::Box | library::Std::Pin | library::Std::Reverse | library::Std::NonZero) => {
					self.erases_to(args.type_at(0))
				}
				Some(library::Std::Option) => self.erases_to(args.type_at(0)).option(),
				Some(library::Std::Poll) => Erased {
					pending: true,
					..self.erases_to(args.type_at(0))
				},
				Some(library::Std::ControlFlow) if library::Flow::of(self.tcx, args) == library::Flow::Option => {
					self.erases_to(args.type_at(1)).option()
				}
				Some(library::Std::RangeFull) => Erased {
					undefined: true,
					..Erased::default()
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
		if !self.lowers(instance) {
			self.err(span, format!("`{}` is from another crate", tcx.def_path_str(def_id)));
			return;
		}
		let site = (!def_id.is_local()).then(|| (self.sites.get(&instance).copied().unwrap_or(span), def_id));
		let outer = std::mem::replace(&mut self.site, site);
		let function = self.lower_function(instance, sym, span);
		self.site = outer;
		if let Some(function) = function {
			if !def_id.is_local() {
				self.program.std.insert(sym);
			}
			self.program.items.push(Item::Function(function));
		}
	}

	fn lower_function(&mut self, instance: Instance<'tcx>, sym: SymId, span: Span) -> Option<ts::Function> {
		let tcx = self.tcx;
		let def_id = instance.def_id();
		// Without unwinding, std's MIR has none of the cleanups a panic runs.
		if !def_id.is_local()
			&& tcx.required_panic_strategy(def_id.krate) == Some(rustc_target::spec::PanicStrategy::Abort)
		{
			self.err(
				span,
				format!(
					"`{}` was built to abort on panic, so its MIR drops nothing a panic unwinds through: rust2-ts builds std from source to unwind",
					tcx.crate_name(def_id.krate)
				),
			);
			return None;
		}
		let mut function = mir::function(self, instance)?;
		function.sym = sym;
		function.loc = self.loc(span);
		// A trait method's impl often leaves its docs to the trait.
		function.doc = self
			.doc(def_id)
			.or_else(|| tcx.trait_item_of(def_id).and_then(|item| self.doc(item)));
		Some(function)
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

	/// Where the invocation of a left-out macro that made `span` is written, if one did.
	pub fn ignored_site(&self, mut span: Span) -> Option<Span> {
		while span.from_expansion() {
			let data = span.ctxt().outer_expn_data();
			if let Some(def_id) = data.macro_def_id
				&& self.config.ignore_macros.contains(&self.tcx.def_path_str(def_id))
			{
				return Some(data.call_site);
			}
			span = data.call_site;
		}
		None
	}
}

/// Whether `ty` is `[u8; N]`, which is a `Uint8Array`.
pub fn byte_array(ty: Ty<'_>) -> bool {
	matches!(ty.kind(), ty::Array(elem, _) if matches!(elem.kind(), ty::Uint(ty::UintTy::U8)))
}

/// Whether `ty` is a reference to a `[u8; N]`, which is an `rt.ByteSlice` of its bytes, as a
/// `&[u8]` is: a split's chunk is then no view.
pub fn bytes_ref(ty: Ty<'_>) -> bool {
	matches!(ty.kind(), ty::Ref(..)) && byte_array(ty.peel_refs())
}

/// A short name for a type, to tell instances apart.
pub fn short_name<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> String {
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
		// Named, since a suffix without its part would read as no instance.
		ty::Tuple(tys) if tys.is_empty() => "unit".into(),
		ty::Never => "never".into(),
		ty::Tuple(tys) => tys.iter().map(|t| short_name(tcx, t)).collect::<Vec<_>>().join("_"),
		ty::Array(elem, _) | ty::Slice(elem) => format!("{}s", short_name(tcx, *elem)),
		// By its trait and the types it binds, as `dyn_Future_u64`.
		ty::Dynamic(preds, ..) => {
			let principal = preds.principal().map(|p| p.skip_binder());
			let args = principal.iter().flat_map(|p| p.args.types());
			let bound = preds.projection_bounds().filter_map(|p| p.skip_binder().term.as_type());
			std::iter::once("dyn".to_string())
				.chain(principal.map(|p| tcx.item_name(p.def_id).to_string()))
				.chain(args.chain(bound).map(|t| short_name(tcx, t)))
				.collect::<Vec<_>>()
				.join("_")
		}
		// By where it is defined, not by its span, which code above it moves.
		ty::Closure(def_id, _) | ty::Coroutine(def_id, _) | ty::CoroutineClosure(def_id, _) | ty::FnDef(def_id, _) => {
			def_name(tcx, *def_id)
		}
		_ => ty.to_string().replace(|c: char| !c.is_alphanumeric(), ""),
	}
}

/// A function's name, or a closure's as Rust numbers it in what defines it: `run_closure0`
/// for `run::{closure#0}`. A tuple struct's constructor goes by the struct's name.
fn def_name(tcx: TyCtxt<'_>, def_id: DefId) -> String {
	use rustc_hir::definitions::{DefPathData, DefPathDataName};
	let path = tcx.def_path(def_id).data;
	let named = path.iter().rposition(|part| part.data.get_opt_name().is_some());
	path[named.unwrap_or(0)..]
		.iter()
		.filter(|part| part.data != DefPathData::Ctor)
		.map(|part| match part.data.name() {
			DefPathDataName::Named(name) => name.to_string(),
			DefPathDataName::Anon { namespace } => format!("{namespace}{}", part.disambiguator),
		})
		.collect::<Vec<_>>()
		.join("_")
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
		"Infinity",
		"Math",
		"NaN",
		"Number",
		"Object",
		"String",
		"Uint8Array",
	];
	match RESERVED.contains(&name) {
		true => format!("{name}_"),
		false => name.to_string(),
	}
}
