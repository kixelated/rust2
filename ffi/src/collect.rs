//! The `pub` API of each exported crate, as rustc checked it, to the [IR](crate::ir).
//!
//! Every `pub` item reachable from an exported crate's root is exported by fixed rules, or
//! refused with its span. An export names another's types across crates; another crate's
//! type is refused. Nothing is opted in by attribute and nothing is skipped silently: the
//! config's `exclude` is the only way to leave an item out, and `#[doc(hidden)]` items,
//! which are no part of the documented API, are listed in the census.

use std::collections::VecDeque;

use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{CrateNum, DefId};
use rustc_infer::infer::TyCtxtInferExt;
use rustc_infer::traits::{Obligation, ObligationCause};
use rustc_middle::ty::{self, GenericArgKind, Ty, TyCtxt, TypingEnv};
use rustc_span::{Span, sym};
use rustc_trait_selection::infer::InferCtxtExt;
use rustc_trait_selection::traits::query::evaluate_obligation::InferCtxtExt as _;

use crate::ir::{
	Access, Api, Callback, Constant, Dyn, Enum, Field, Function, Int, Literal, Mutability, Object, Param, Path,
	Receiver, Record, Traits, Type, Variant,
};

/// Why something is refused: a construct, ranked by the census, and what it names.
#[derive(Clone, Debug)]
pub struct Why {
	pub construct: &'static str,
	pub detail: String,
	/// Where else the reason lies, and what is there.
	pub note: Option<(Span, String)>,
}

fn why(construct: &'static str, detail: impl Into<String>) -> Why {
	Why {
		construct,
		detail: detail.into(),
		note: None,
	}
}

/// An item the export refuses, with every reason found.
pub struct Refusal {
	/// Its path from its crate's root.
	pub path: String,
	/// What kind of item it is: `fn`, `method`, `struct`, ...
	pub kind: &'static str,
	pub span: Span,
	pub why: Vec<Why>,
}

/// What collection found.
pub struct Collected {
	pub api: Api,
	/// Each export's, in the order given.
	pub reports: Vec<Report>,
}

/// An exported crate.
pub struct Crate<'a> {
	pub krate: CrateNum,
	pub package: &'a str,
	/// Paths from its root the config leaves out.
	pub exclude: &'a [String],
}

/// What collection found in one exported crate, by paths from its root.
pub struct Report {
	pub package: String,
	/// Its crate's name, the first segment of its items' paths in the IR.
	pub krate: String,
	pub refusals: Vec<Refusal>,
	/// Records and enums exported as objects, since a field does not map: the first.
	pub opaque: Vec<(String, Why)>,
	/// `poll_x` methods that do not map, left out since `x` exports as a future.
	pub twins: Vec<String>,
	/// `#[doc(hidden)]` items, by path.
	pub hidden: Vec<String>,
	/// Excluded paths, and how many items each left out.
	pub excluded: Vec<(String, usize)>,
	/// Type aliases, which export nothing of their own.
	pub aliases: Vec<String>,
	/// Re-exports of another export's items, which name that export's.
	pub reexports: Vec<String>,
}

/// How an exported crate's type is exported, once decided.
#[derive(Clone)]
enum Decl {
	Record(usize),
	Enum(usize),
	Object(usize),
	Callback(usize),
	Refused(Why),
}

struct Collector<'tcx> {
	tcx: TyCtxt<'tcx>,
	/// Each `pub` item and its shortest path, in the order the walk met them.
	items: Vec<(DefId, Path)>,
	decls: rustc_data_structures::fx::FxHashMap<DefId, Decl>,
	/// The exported crates, the configured one first, and what collection found in each.
	crates: Vec<CrateNum>,
	reports: Vec<Report>,
	/// Items the walk left out, and why: `excluded` or `doc(hidden)`.
	left_out: rustc_data_structures::fx::FxHashMap<DefId, &'static str>,
	/// The type parameters of the function being mapped that take a foreign
	/// implementation, with the callback each takes.
	impls: std::cell::RefCell<Vec<(Ty<'tcx>, usize)>>,
	bytes: Option<DefId>,
	/// `std::ops::Bound`, and `std::ops::RangeBounds`, which names it.
	bound: Option<DefId>,
	range_bounds: Option<DefId>,
	/// Objects no export returns, refused.
	orphans: rustc_data_structures::fx::FxHashSet<DefId>,
	api: Api,
}

/// Collect the exported crates' API, leaving out what each one's `exclude` names.
pub fn collect<'tcx>(tcx: TyCtxt<'tcx>, crates: &[Crate<'_>]) -> Collected {
	let bytes = tcx
		.crates(())
		.iter()
		.find(|&&krate| tcx.crate_name(krate).as_str() == "bytes")
		.and_then(|&krate| {
			tcx.module_children(krate.as_def_id())
				.iter()
				.find(|child| child.ident.as_str() == "Bytes")
				.and_then(|child| child.res.opt_def_id())
		});
	let range_bounds = tcx.get_diagnostic_item(rustc_span::Symbol::intern("RangeBounds"));
	// `RangeBounds::start_bound` returns a `Bound`, which has no diagnostic item of its own.
	let bound = range_bounds.and_then(|tr| {
		let start = tcx
			.associated_items(tr)
			.in_definition_order()
			.find(|item| item.name().as_str() == "start_bound")?;
		match tcx.fn_sig(start.def_id).skip_binder().skip_binder().output().kind() {
			ty::Adt(adt, _) => Some(adt.did()),
			_ => None,
		}
	});
	// Refusing an object no export returns refuses what takes it, which may leave another
	// object no export returns, so collection repeats until it refuses no more.
	let mut orphans = rustc_data_structures::fx::FxHashSet::default();
	loop {
		let mut c = Collector {
			tcx,
			items: Vec::new(),
			decls: Default::default(),
			crates: crates.iter().map(|c| c.krate).collect(),
			reports: crates
				.iter()
				.map(|c| Report {
					package: c.package.to_string(),
					krate: tcx.crate_name(c.krate).to_string(),
					refusals: Vec::new(),
					opaque: Vec::new(),
					twins: Vec::new(),
					hidden: Vec::new(),
					excluded: c.exclude.iter().map(|path| (path.clone(), 0)).collect(),
					aliases: Vec::new(),
					reexports: Vec::new(),
				})
				.collect(),
			left_out: Default::default(),
			impls: Default::default(),
			bytes,
			bound,
			range_bounds,
			orphans: orphans.clone(),
			api: Api {
				records: Vec::new(),
				enums: Vec::new(),
				objects: Vec::new(),
				callbacks: Vec::new(),
				functions: Vec::new(),
				constants: Vec::new(),
			},
		};
		c.walk();
		c.declare();
		c.define();
		let more = c.orphans();
		if more.is_empty() {
			return Collected {
				api: c.api,
				reports: c.reports,
			};
		}
		orphans.extend(more);
	}
}

impl<'tcx> Collector<'tcx> {
	/// What collection found in the crate `path` starts with, and the rest of `path`.
	fn report(&mut self, path: &[String]) -> (&mut Report, String) {
		let report = (self.reports.iter_mut())
			.find(|r| r.krate == path[0])
			.expect("a path starts with its exported crate");
		(report, path[1..].join("::"))
	}

	/// Whether `krate` is exported.
	fn exported(&self, krate: CrateNum) -> bool {
		self.crates.contains(&krate)
	}

	/// Whether the config excludes `path`, counting the exclusion.
	fn exclude(&mut self, path: &[String]) -> bool {
		let (report, joined) = self.report(path);
		for (excluded, count) in &mut report.excluded {
			if joined == *excluded || joined.starts_with(&format!("{excluded}::")) {
				*count += 1;
				return true;
			}
		}
		false
	}

	fn refuse(&mut self, path: &[String], kind: &'static str, span: Span, why: Vec<Why>) {
		let (report, path) = self.report(path);
		report.refusals.push(Refusal { path, kind, span, why });
	}

	/// Every `pub` item reachable from each exported crate's root, breadth first so each
	/// keeps its shortest path, which starts with the crate's name.
	fn walk(&mut self) {
		let tcx = self.tcx;
		for krate in self.crates.clone() {
			// Per export: two may re-export one item, and each refuses or excludes it.
			let mut seen = rustc_data_structures::fx::FxHashSet::default();
			let mut queue = VecDeque::from([(krate.as_def_id(), vec![tcx.crate_name(krate).to_string()])]);
			while let Some((module, path)) = queue.pop_front() {
				let children = match module.as_local() {
					Some(local) => tcx.module_children_local(local),
					None => tcx.module_children(module),
				};
				for child in children {
					if !child.vis.is_public() || child.ident.name == rustc_span::kw::Underscore {
						continue;
					}
					let Res::Def(kind, def_id) = child.res else {
						continue;
					};
					// Variants and constructors come along with their type.
					if matches!(kind, DefKind::Ctor(..) | DefKind::Variant) {
						continue;
					}
					let mut path = path.clone();
					path.push(child.ident.to_string());
					// Another export's item, which that export walks from its own root.
					if def_id.krate != krate && self.exported(def_id.krate) {
						let (report, rest) = self.report(&path);
						report.reexports.push(rest);
						continue;
					}
					if !seen.insert(def_id) {
						continue;
					}
					if self.exclude(&path) {
						self.left_out.insert(def_id, "excluded type");
						continue;
					}
					if tcx.is_doc_hidden(def_id) {
						let (report, rest) = self.report(&path);
						report.hidden.push(rest);
						self.left_out.insert(def_id, "doc(hidden) type");
						continue;
					}
					let span = child
						.reexport_chain
						.first()
						.and_then(|reexport| reexport.id())
						.map_or_else(|| tcx.def_span(def_id), |id| tcx.def_span(id));
					if def_id.krate != krate {
						let other = tcx.crate_name(def_id.krate);
						self.refuse(
							&path,
							"use",
							span,
							vec![why("re-export of another crate's item", other.to_string())],
						);
						continue;
					}
					match kind {
						DefKind::Mod => queue.push_back((def_id, path)),
						DefKind::TyAlias => {
							let (report, rest) = self.report(&path);
							report.aliases.push(rest);
						}
						DefKind::Macro(_) => self.refuse(&path, "macro", span, vec![why("macro", "")]),
						DefKind::Static { .. } => self.refuse(&path, "static", span, vec![why("static", "")]),
						DefKind::Struct
						| DefKind::Enum
						| DefKind::Union
						| DefKind::Trait
						| DefKind::Fn
						| DefKind::Const => self.items.push((def_id, path)),
						_ => self.refuse(&path, "item", span, vec![why("item kind", format!("{kind:?}"))]),
					}
				}
			}
		}
	}

	/// Decide how each type and trait is exported, before any signature names one: a
	/// struct whose fields are all `pub`, or an enum, is a value if its fields map, and
	/// any other type is an object.
	fn declare(&mut self) {
		let tcx = self.tcx;
		for (def_id, _) in self.items.clone() {
			let params = &tcx.generics_of(def_id).own_params;
			let generic = params.iter().any(|p| p.name != rustc_span::kw::SelfUpper);
			let lifetimes = params
				.iter()
				.all(|p| matches!(p.kind, ty::GenericParamDefKind::Lifetime));
			let decl = match tcx.def_kind(def_id) {
				DefKind::Union => Decl::Refused(why("union", "")),
				// Held by a handle that outlives any call, it is the form that borrows nothing.
				DefKind::Struct | DefKind::Enum if generic && lifetimes => self.object(def_id),
				DefKind::Struct | DefKind::Enum | DefKind::Trait if generic => Decl::Refused(why("generic type", "")),
				// A value is copied across and lowered into memory C frees, so its own `Drop`
				// would run when Rust lets go of it, not when C does.
				DefKind::Struct | DefKind::Enum if tcx.adt_destructor(def_id).is_some() => self.object(def_id),
				DefKind::Struct => {
					let variant = tcx.adt_def(def_id).non_enum_variant();
					let record = !variant.fields.is_empty()
						&& variant.fields.iter().all(|f| f.vis.is_public())
						&& !variant.is_field_list_non_exhaustive();
					match record {
						true => Decl::Record(0),
						false => self.object(def_id),
					}
				}
				DefKind::Enum if tcx.adt_def(def_id).variants().is_empty() => {
					Decl::Refused(why("enum without variants", ""))
				}
				DefKind::Enum => Decl::Enum(0),
				DefKind::Trait => Decl::Callback(0),
				_ => continue,
			};
			self.decls.insert(def_id, decl);
		}
		self.settle();
		// A trait whose methods do not map is refused before any signature takes it.
		for (def_id, path) in self.items.clone() {
			if matches!(self.decls.get(&def_id), Some(Decl::Callback(_)))
				&& let Err(why) = self.callback(def_id, &path)
			{
				let first = why.into_iter().next().expect("a refusal has a reason");
				self.decls.insert(def_id, Decl::Refused(first));
			}
		}
		// Number each kind in item order, with placeholders filled in by `define`.
		for (def_id, path) in self.items.clone() {
			let span = tcx.def_span(def_id);
			let docs = docs(tcx, def_id);
			let Some(decl) = self.decls.get_mut(&def_id) else {
				continue;
			};
			let api = &mut self.api;
			match decl {
				Decl::Record(i) => {
					*i = api.records.len();
					api.records.push(Record {
						path,
						span,
						docs,
						fields: Vec::new(),
						tuple: false,
						methods: Vec::new(),
						traits: Traits::default(),
					});
				}
				Decl::Enum(i) => {
					*i = api.enums.len();
					api.enums.push(Enum {
						path,
						span,
						docs,
						variants: Vec::new(),
						exhaustive: true,
						methods: Vec::new(),
						traits: Traits::default(),
					});
				}
				Decl::Object(i) => {
					*i = api.objects.len();
					let sync = implements(tcx, def_id, sym::Sync);
					let copy = implements(tcx, def_id, sym::Copy);
					let lifetimes = tcx.generics_of(def_id).own_counts().lifetimes;
					api.objects.push(Object {
						path,
						span,
						docs,
						sync,
						copy,
						lifetimes,
						methods: Vec::new(),
						traits: Traits::default(),
					});
				}
				Decl::Callback(i) => {
					*i = api.callbacks.len();
					api.callbacks.push(Callback {
						path,
						span,
						docs,
						methods: Vec::new(),
					});
				}
				Decl::Refused(_) => {}
			}
		}
	}

	/// An object, if the type can move across threads. Shared borrows of one that is not
	/// `Sync` are exclusive.
	fn object(&self, def_id: DefId) -> Decl {
		if self.orphans.contains(&def_id) {
			return Decl::Refused(why("object no export returns", ""));
		}
		match implements(self.tcx, def_id, sym::Send) {
			true => Decl::Object(0),
			false => Decl::Refused(why("object not Send", "")),
		}
	}

	/// The objects C can never hold a handle to: no export it can call returns one.
	fn orphans(&self) -> Vec<DefId> {
		let api = &self.api;
		let types = (api.objects.iter().enumerate()).map(|(i, o)| (Type::Object(i), &o.methods, o.traits));
		let types = types.chain((api.records.iter().enumerate()).map(|(i, r)| (Type::Record(i), &r.methods, r.traits)));
		let types = types.chain((api.enums.iter().enumerate()).map(|(i, e)| (Type::Enum(i), &e.methods, e.traits)));
		// Each function and the type its receiver is, and each type `Default` makes.
		let mut functions: Vec<(&Function, Option<Type>)> = api.functions.iter().map(|f| (f, None)).collect();
		let mut defaults = Vec::new();
		for (ty, methods, traits) in types {
			functions.extend(methods.iter().map(|f| (f, f.receiver.map(|_| ty.clone()))));
			if traits.default {
				defaults.push(ty);
			}
		}
		let mut held = vec![false; api.objects.len()];
		loop {
			let mut reached = Vec::new();
			for ty in &defaults {
				returns(api, ty, &mut reached);
			}
			for (function, receiver) in &functions {
				let callable = (receiver.iter())
					.chain(function.params.iter().map(|p| &p.ty))
					.all(|ty| buildable(api, ty, &held));
				if callable {
					returns(api, &function.ok, &mut reached);
					if let Some(err) = &function.err {
						returns(api, err, &mut reached);
					}
				}
			}
			reached.retain(|&i| !held[i]);
			if reached.is_empty() {
				break;
			}
			for i in reached {
				held[i] = true;
			}
		}
		self.decls
			.iter()
			.filter_map(|(&def_id, decl)| match decl {
				Decl::Object(i) if !held[*i] => Some(def_id),
				_ => None,
			})
			.collect()
	}

	/// Make each record or enum whose fields do not map an object, until none changes.
	fn settle(&mut self) {
		loop {
			let mut changed = false;
			for (def_id, path) in self.items.clone() {
				if !matches!(self.decls.get(&def_id), Some(Decl::Record(_) | Decl::Enum(_))) {
					continue;
				}
				let fields = match self.recursive(def_id) {
					true => Err(vec![why("recursive value", "")]),
					false => self.fields(def_id).map(|_| ()),
				};
				if let Err(why) = fields {
					let first = why.into_iter().next().expect("a refusal has a reason");
					let (report, rest) = self.report(&path);
					report.opaque.push((rest, first));
					let decl = self.object(def_id);
					self.decls.insert(def_id, decl);
					changed = true;
				}
			}
			if !changed {
				break;
			}
		}
	}

	/// Whether a value holds itself, through other values, an `Option`, or a `Box`: C
	/// holds each value in place, so it cannot.
	fn recursive(&self, start: DefId) -> bool {
		let tcx = self.tcx;
		let mut seen = rustc_data_structures::fx::FxHashSet::default();
		let mut stack = vec![start];
		while let Some(def_id) = stack.pop() {
			let adt = tcx.adt_def(def_id);
			let identity = ty::GenericArgs::identity_for_item(tcx, def_id);
			for field in adt.all_fields() {
				for arg in field.ty(tcx, identity).skip_norm_wip().walk() {
					let Some(ty) = arg.as_type() else {
						continue;
					};
					let ty::Adt(inner, _) = ty.kind() else {
						continue;
					};
					let did = inner.did();
					if !matches!(self.decls.get(&did), Some(Decl::Record(_) | Decl::Enum(_))) {
						continue;
					}
					if did == start {
						return true;
					}
					if seen.insert(did) {
						stack.push(did);
					}
				}
			}
		}
		false
	}

	/// A struct's or enum's variants, with their fields mapped.
	fn fields(&self, def_id: DefId) -> Result<Vec<Variant>, Vec<Why>> {
		let tcx = self.tcx;
		let adt = tcx.adt_def(def_id);
		let identity = ty::GenericArgs::identity_for_item(tcx, def_id);
		let mut errs = Vec::new();
		let mut variants = Vec::new();
		let discriminants: Vec<i128> = match adt.is_enum() {
			true => adt.discriminants(tcx).map(|(_, d)| signed(tcx, d.ty, d.val)).collect(),
			false => vec![0],
		};
		for (variant, discriminant) in adt.variants().iter().zip(discriminants) {
			if adt.is_enum() && variant.is_field_list_non_exhaustive() {
				errs.push(why("non-exhaustive variant", variant.name.to_string()));
			}
			let mut fields = Vec::new();
			for field in &variant.fields {
				let ty = field.ty(tcx, identity).skip_norm_wip();
				match self.ty(ty, Pos::Field) {
					Ok(ty) => fields.push(Field {
						name: field.name.to_string(),
						docs: docs(tcx, field.did),
						ty,
					}),
					Err(why) => errs.push(why),
				}
			}
			let tuple = variant.ctor_kind() == Some(rustc_hir::def::CtorKind::Fn);
			variants.push(Variant {
				name: variant.name.to_string(),
				discriminant,
				fields,
				tuple,
			});
		}
		match errs.is_empty() {
			true => Ok(variants),
			false => Err(errs),
		}
	}

	/// Fill in every declaration, then the functions and constants.
	fn define(&mut self) {
		let tcx = self.tcx;
		for (def_id, path) in self.items.clone() {
			let span = tcx.def_span(def_id);
			let kind = tcx.def_kind(def_id);
			match self.decls.get(&def_id).cloned() {
				Some(Decl::Refused(why)) => {
					let kind = match kind {
						DefKind::Trait => "trait",
						DefKind::Enum => "enum",
						DefKind::Union => "union",
						_ => "struct",
					};
					self.refuse(&path, kind, span, vec![why]);
				}
				Some(Decl::Record(index)) => {
					let variants = self.fields(def_id).expect("settled");
					let variant = variants.into_iter().next().expect("a struct has a variant");
					let (methods, traits) = self.methods(def_id, &path);
					let record = &mut self.api.records[index];
					record.fields = variant.fields;
					record.tuple = variant.tuple;
					record.methods = methods;
					record.traits = traits;
				}
				Some(Decl::Enum(index)) => {
					let variants = self.fields(def_id).expect("settled");
					let exhaustive = !tcx.adt_def(def_id).is_variant_list_non_exhaustive();
					let (methods, traits) = self.methods(def_id, &path);
					let e = &mut self.api.enums[index];
					e.variants = variants;
					e.exhaustive = exhaustive;
					e.methods = methods;
					e.traits = traits;
				}
				Some(Decl::Object(index)) => {
					let (mut methods, traits) = self.methods(def_id, &path);
					if kind == DefKind::Struct {
						methods.splice(0..0, self.getters(def_id, &path));
					}
					let object = &mut self.api.objects[index];
					object.methods = methods;
					object.traits = traits;
				}
				Some(Decl::Callback(index)) => {
					self.api.callbacks[index].methods = self.callback(def_id, &path).expect("settled");
				}
				None => match kind {
					DefKind::Fn => match self.function(def_id, path.clone(), None) {
						Ok(function) => self.api.functions.push(function),
						Err(why) => self.refuse(&path, "fn", span, why),
					},
					DefKind::Const => {
						if let Some(getter) = self.constant_item(def_id, path) {
							self.api.functions.push(getter);
						}
					}
					_ => {}
				},
			}
		}
	}

	/// A getter for each `pub` field of a struct exported as an object, returning a copy.
	fn getters(&mut self, def_id: DefId, path: &[String]) -> Vec<Function> {
		let tcx = self.tcx;
		let identity = statics(tcx, def_id);
		let mut getters = Vec::new();
		for field in &tcx.adt_def(def_id).non_enum_variant().fields {
			if !field.vis.is_public() {
				continue;
			}
			let mut getter = path.to_vec();
			getter.push(field.name.to_string());
			if self.exclude(&getter) {
				continue;
			}
			let span = tcx.def_span(field.did);
			let ty = field.ty(tcx, identity).skip_norm_wip();
			// Read through `&self`, so returned as a copy, as a borrowed return is.
			match self.ty(Ty::new_imm_ref(tcx, tcx.lifetimes.re_erased, ty), Pos::Return) {
				Ok(ty) => getters.push(Function {
					path: getter,
					span,
					docs: docs(tcx, field.did),
					receiver: Some(Receiver::Ref),
					params: Vec::new(),
					ok: ty,
					err: None,
					future: false,
					eager: false,
					access: Access::Field,
				}),
				Err(why) => self.refuse(&getter, "field", span, vec![why]),
			}
		}
		getters
	}

	/// A type's inherent `pub` methods, and the std traits it implements.
	fn methods(&mut self, def_id: DefId, path: &[String]) -> (Vec<Function>, Traits) {
		let tcx = self.tcx;
		let mut methods = Vec::new();
		let mut refused = Vec::new();
		let self_ty = tcx
			.type_of(def_id)
			.instantiate(tcx, statics(tcx, def_id))
			.skip_norm_wip();
		for &imp in tcx.inherent_impls(def_id) {
			for item in tcx.associated_items(imp).in_definition_order() {
				let (ty::AssocKind::Fn { name, .. } | ty::AssocKind::Const { name, .. }) = item.kind else {
					continue;
				};
				if !tcx.visibility(item.def_id).is_public() {
					continue;
				}
				let mut method = path.to_vec();
				method.push(name.to_string());
				if self.exclude(&method) {
					continue;
				}
				if tcx.is_doc_hidden(item.def_id) {
					let (report, rest) = self.report(&method);
					report.hidden.push(rest);
					continue;
				}
				let span = tcx.def_span(item.def_id);
				if let ty::AssocKind::Const { .. } = item.kind {
					methods.extend(self.constant_item(item.def_id, method));
					continue;
				}
				match self.function(item.def_id, method.clone(), Some(self_ty)) {
					Ok(function) => methods.push(function),
					Err(why) => refused.push((method, span, why)),
				}
			}
		}
		// `poll_x` is left out when `x` exports as a future: the future is its poll.
		for (method, span, why) in refused {
			let name = method.last().expect("a method has a name");
			let twin = name
				.strip_prefix("poll_")
				.is_some_and(|x| methods.iter().any(|m| m.future && m.name() == x));
			match twin {
				true => {
					let (report, rest) = self.report(&method);
					report.twins.push(rest);
				}
				false => self.refuse(&method, "method", span, why),
			}
		}
		let env = TypingEnv::post_analysis(tcx, def_id);
		let implements = |item| {
			tcx.get_diagnostic_item(item).is_some_and(|tr| {
				let (infcx, env) = tcx.infer_ctxt().build_with_typing_env(env);
				infcx
					.type_implements_trait(tr, [self_ty], env)
					.must_apply_modulo_regions()
			})
		};
		let traits = Traits {
			clone: implements(sym::Clone),
			eq: tcx.lang_items().eq_trait().is_some_and(|tr| {
				let (infcx, env) = tcx.infer_ctxt().build_with_typing_env(env);
				infcx
					.type_implements_trait(tr, [self_ty, self_ty], env)
					.must_apply_modulo_regions()
			}),
			display: implements(sym::Display),
			debug: implements(sym::Debug),
			default: implements(sym::Default),
		};
		(methods, traits)
	}

	/// A trait's required methods, as a callback interface.
	fn callback(&mut self, def_id: DefId, path: &[String]) -> Result<Vec<Function>, Vec<Why>> {
		let tcx = self.tcx;
		let mut errs = Vec::new();
		// It may already cover the foreign implementation, which could not then implement it.
		if !tcx.trait_impls_of(def_id).blanket_impls().is_empty() {
			errs.push(why("trait with a blanket impl", ""));
		}
		for clause in tcx.explicit_super_clauses_of(def_id).iter_identity_copied() {
			let (clause, _) = clause.skip_norm_wip();
			if let Some(trait_pred) = clause.as_trait_clause() {
				let tr = trait_pred.def_id();
				let auto = [sym::Send, sym::Sync]
					.iter()
					.any(|&item| tcx.is_diagnostic_item(item, tr))
					|| [tcx.lang_items().sized_trait(), tcx.lang_items().meta_sized_trait()].contains(&Some(tr));
				if !auto {
					errs.push(why("supertrait", tcx.def_path_str(tr)));
				}
			}
		}
		let mut methods = Vec::new();
		let self_ty = Ty::new_param(tcx, 0, rustc_span::kw::SelfUpper);
		for item in tcx.associated_items(def_id).in_definition_order() {
			match item.kind {
				ty::AssocKind::Fn { name, .. } => {
					// A provided method keeps its body: only required ones are foreign.
					if item.defaultness(tcx).has_value() {
						continue;
					}
					let mut method = path.to_vec();
					method.push(name.to_string());
					match self.function(item.def_id, method, Some(self_ty)) {
						Ok(function) if function.future => errs.push(why("async trait method", name.to_string())),
						Ok(function)
							if !function.params.iter().all(|p| foreign_arg(&p.ty))
								|| !foreign_return(&function.ok)
								|| function.err.is_some() =>
						{
							errs.push(why("callback signature", name.to_string()))
						}
						Ok(function) if function.receiver == Some(Receiver::Owned) || function.receiver.is_none() => {
							errs.push(why("trait method without &self", name.to_string()))
						}
						Ok(function) => methods.push(function),
						Err(why) => errs.extend(why),
					}
				}
				// An `impl Trait` return's own type: the method refuses it.
				ty::AssocKind::Type {
					data: ty::AssocTypeData::Rpitit(_),
				} => {}
				ty::AssocKind::Type { .. } => errs.push(why("associated type", item.name().to_string())),
				ty::AssocKind::Const { .. } => errs.push(why("associated const", item.name().to_string())),
			}
		}
		match errs.is_empty() {
			true => Ok(methods),
			false => Err(errs),
		}
	}

	/// A function or method's signature, with its type parameters chosen.
	fn function(&self, def_id: DefId, path: Path, self_ty: Option<Ty<'tcx>>) -> Result<Function, Vec<Why>> {
		let tcx = self.tcx;
		let mut errs = Vec::new();
		self.impls.borrow_mut().clear();
		let args = self.instantiate(def_id, self_ty)?;
		let env = TypingEnv::post_analysis(tcx, def_id);
		let sig = tcx.fn_sig(def_id).instantiate(tcx, args).skip_norm_wip();
		let (statics, lifetimes) = self.borrows(def_id, sig.skip_binder());
		errs.extend(lifetimes);
		// Whether what it returns borrows: any lifetime in it that is not `'static`.
		let borrows = sig.skip_binder().output().walk().any(|arg| match arg.kind() {
			GenericArgKind::Lifetime(region) => !statics.contains(&region),
			_ => false,
		});
		let sig = tcx.instantiate_bound_regions_with_erased(sig);
		let normalize = |ty: Ty<'tcx>| {
			tcx.try_normalize_erasing_regions(env, ty::Unnormalized::new_wip(ty))
				.unwrap_or(ty)
		};
		let names = tcx.fn_arg_idents(def_id);
		let has_self = matches!(tcx.def_kind(def_id), DefKind::AssocFn)
			&& matches!(
				tcx.associated_item(def_id).kind,
				ty::AssocKind::Fn { has_self: true, .. }
			);
		let mut inputs = sig.inputs().iter().copied().map(normalize).enumerate();
		let receiver = match has_self {
			false => None,
			true => {
				let (_, ty) = inputs.next().expect("a method has self");
				let self_ty = self_ty.map(normalize);
				match ty.kind() {
					_ if Some(ty) == self_ty => Some(Receiver::Owned),
					ty::Ref(_, inner, m) if Some(*inner) == self_ty => Some(match m {
						ty::Mutability::Not => Receiver::Ref,
						ty::Mutability::Mut => Receiver::Mut,
					}),
					_ => {
						errs.push(why("receiver", ty.to_string()));
						None
					}
				}
			}
		};
		let callback = tcx.def_kind(tcx.parent(def_id)) == DefKind::Trait;
		let mut params = Vec::new();
		for (i, ty) in inputs {
			let name = names
				.get(i)
				.copied()
				.flatten()
				.map_or_else(|| format!("arg{i}"), |ident| ident.to_string());
			match self.ty(ty, Pos::Param) {
				// C's value is lifted into an owned Rust one, lent only as the parameter
				// itself. A callback's parameters are lent the other way, to C.
				Ok(lifted) if !callback && lends_inside(&lifted) => {
					errs.push(why("reference inside a parameter", ty.to_string()));
				}
				Ok(ty) => params.push(Param { name, ty }),
				Err(why) => errs.push(why),
			}
		}
		let mut output = normalize(sig.output());
		let mut future = false;
		let eager = !tcx.asyncness(def_id).is_async();
		if let Some(inner) = self.future_output(output, env) {
			if !self.is_send(output, env) {
				errs.push(why("future not Send", ""));
			}
			// Called at once, the future must outlive the arguments the call borrowed.
			if eager && borrows {
				errs.push(why("borrowing future from a function that is not async", ""));
			}
			output = normalize(inner);
			future = true;
		}
		let (ok, err) = match output.kind() {
			ty::Adt(adt, args) if tcx.is_diagnostic_item(sym::Result, adt.did()) => {
				(self.ret(args.type_at(0)), Some(self.ty(args.type_at(1), Pos::Return)))
			}
			_ => (self.ret(output), None),
		};
		let ok = ok.map_err(|why| errs.push(why)).ok();
		let err = match err {
			Some(Ok(err)) => Some(err),
			Some(Err(why)) => {
				errs.push(why);
				None
			}
			None => None,
		};
		match (errs.is_empty(), ok) {
			(true, Some(ok)) => Ok(Function {
				path,
				span: tcx.def_span(def_id),
				docs: docs(tcx, def_id),
				receiver,
				params,
				ok,
				err,
				future,
				eager: future && eager,
				access: Access::Call,
			}),
			_ => Err(errs),
		}
	}

	/// The generic arguments a function is exported with: its parent's own (a trait's
	/// `Self`), lifetimes as they are, which [`Self::borrows`] decides, and each type
	/// parameter by the rules in [`Self::choose`].
	fn instantiate(&self, def_id: DefId, self_ty: Option<Ty<'tcx>>) -> Result<ty::GenericArgsRef<'tcx>, Vec<Why>> {
		let tcx = self.tcx;
		let generics = tcx.generics_of(def_id);
		let identity = ty::GenericArgs::identity_for_item(tcx, def_id);
		let mut errs = Vec::new();
		let mut args: Vec<ty::GenericArg<'tcx>> = identity.to_vec();
		let parent = generics.parent_count;
		// A trait method's `Self` stays a parameter: the trait is the callback.
		let _ = self_ty;
		for (i, arg) in identity.iter().enumerate() {
			match arg.kind() {
				_ if i < parent => {}
				GenericArgKind::Lifetime(_) => {}
				GenericArgKind::Const(_) => errs.push(why("const generic", "")),
				GenericArgKind::Type(param) => match self.choose(def_id, param) {
					Ok(ty) => args[i] = ty.into(),
					// The parameter stays, and `ty` maps it to the foreign implementation.
					Err(Why {
						construct: "callback bound",
						..
					}) if let Some(callback) = self.foreign(def_id, param) => {
						self.impls.borrow_mut().push((param, callback));
					}
					Err(why) => errs.push(why),
				},
			}
		}
		match errs.is_empty() {
			true => Ok(tcx.mk_args(&args)),
			false => Err(errs),
		}
	}

	/// The type a type parameter is exported as: `X` for `impl Into<X>` or
	/// `impl AsRef<X>`, else the first of `String`, `Bytes`, and `Vec<u8>` meeting its
	/// bounds.
	fn choose(&self, def_id: DefId, param: Ty<'tcx>) -> Result<Ty<'tcx>, Why> {
		let tcx = self.tcx;
		let clauses = tcx.clauses_of(def_id);
		let identity = clauses.instantiate_identity(tcx);
		let bounds: Vec<_> = identity
			.clauses
			.iter()
			.map(|clause| clause.skip_norm_wip())
			.filter_map(|clause| clause.as_trait_clause())
			.filter(|pred| pred.self_ty().skip_binder() == param)
			.collect();
		// A parameter that must outlive the call cannot borrow what C lends for it.
		let outlives = identity.clauses.iter().any(|clause| {
			clause
				.skip_norm_wip()
				.as_type_outlives_clause()
				.is_some_and(|pred| pred.skip_binder().0 == param)
		});
		let mut candidates = Vec::new();
		for bound in &bounds {
			let tr = bound.def_id();
			let target = || bound.skip_binder().trait_ref.args.type_at(1);
			if tcx.is_diagnostic_item(sym::Into, tr) || tcx.is_diagnostic_item(sym::AsRef, tr) {
				let target = target();
				match target.kind() {
					ty::Str | ty::Slice(_) if outlives => {}
					ty::Str | ty::Slice(_) => candidates.push(Ty::new_imm_ref(tcx, tcx.lifetimes.re_erased, target)),
					_ => candidates.push(target),
				}
			} else if Some(tr) == self.range_bounds {
				candidates.extend(self.bounds(target()));
			}
		}
		if let Some(string) = tcx.lang_items().string() {
			candidates.push(Ty::new_adt(tcx, tcx.adt_def(string), ty::List::empty()));
		}
		if let Some(bytes) = self.bytes {
			candidates.push(Ty::new_adt(tcx, tcx.adt_def(bytes), ty::List::empty()));
		}
		if let Some(vec) = tcx.get_diagnostic_item(sym::Vec) {
			let alloc = tcx.lang_items().global_alloc_ty();
			if let Some(alloc) = alloc {
				let args = tcx.mk_args(&[
					tcx.types.u8.into(),
					Ty::new_adt(tcx, tcx.adt_def(alloc), ty::List::empty()).into(),
				]);
				candidates.push(Ty::new_adt(tcx, tcx.adt_def(vec), args));
			}
		}
		let env = TypingEnv::post_analysis(tcx, def_id);
		for candidate in candidates {
			let (infcx, param_env) = tcx.infer_ctxt().build_with_typing_env(env);
			let mut args: Vec<ty::GenericArg<'tcx>> = ty::GenericArgs::identity_for_item(tcx, def_id).to_vec();
			let ty::Param(p) = param.kind() else {
				unreachable!("a generic type argument is a parameter");
			};
			args[p.index as usize] = candidate.into();
			let instantiated = clauses.instantiate(tcx, tcx.mk_args(&args));
			let holds = instantiated.clauses.iter().all(|clause| {
				let clause = clause.skip_norm_wip();
				let obligation = Obligation::new(tcx, ObligationCause::dummy(), param_env, clause);
				infcx.predicate_must_hold_modulo_regions(&obligation)
			});
			// `impl Into<Arc<str>>` takes a `String` too, which maps where `Arc<str>` does not.
			if holds && self.ty(candidate, Pos::Param).is_ok() {
				return Ok(candidate);
			}
		}
		// A bound on one of the crate's traits takes a foreign implementation.
		let local: Vec<_> = bounds
			.iter()
			.map(|b| b.def_id())
			.filter(|tr| matches!(self.decls.get(tr), Some(Decl::Callback(_))))
			.collect();
		let sized = [tcx.lang_items().sized_trait(), tcx.lang_items().meta_sized_trait()];
		let names: Vec<String> = bounds
			.iter()
			.filter(|b| !sized.contains(&Some(b.def_id())))
			.map(|b| tcx.def_path_str(b.def_id()))
			.collect();
		match local.as_slice() {
			[_] => Err(why("callback bound", names.join(" + "))),
			_ => Err(why("generic parameter", names.join(" + "))),
		}
	}

	/// The callback a type parameter takes a foreign implementation of: one bounded by a
	/// single exported trait, and otherwise only by what that implementation meets.
	fn foreign(&self, def_id: DefId, param: Ty<'tcx>) -> Option<usize> {
		let tcx = self.tcx;
		let mut callback = None;
		for clause in tcx.clauses_of(def_id).instantiate_identity(tcx).clauses {
			let Some(bound) = clause.skip_norm_wip().as_trait_clause() else {
				continue;
			};
			if bound.self_ty().skip_binder() != param {
				continue;
			}
			let tr = bound.def_id();
			let met = [sym::Send, sym::Sync]
				.iter()
				.any(|&item| tcx.is_diagnostic_item(item, tr))
				|| [tcx.lang_items().sized_trait(), tcx.lang_items().meta_sized_trait()].contains(&Some(tr));
			match self.decls.get(&tr) {
				Some(Decl::Callback(i)) if callback.is_none() => callback = Some(*i),
				_ if met => {}
				_ => return None,
			}
		}
		callback
	}

	/// What `ty` resolves to when awaited, if it is a future.
	fn future_output(&self, ty: Ty<'tcx>, env: TypingEnv<'tcx>) -> Option<Ty<'tcx>> {
		let tcx = self.tcx;
		let future = tcx.lang_items().future_trait()?;
		let output = tcx.lang_items().future_output()?;
		let (infcx, param_env) = tcx.infer_ctxt().build_with_typing_env(env);
		if !infcx
			.type_implements_trait(future, [ty], param_env)
			.must_apply_modulo_regions()
		{
			return None;
		}
		let projection = Ty::new_projection(tcx, ty::IsRigid::No, output, [ty]);
		tcx.try_normalize_erasing_regions(env, ty::Unnormalized::new_wip(projection))
			.ok()
	}

	/// The lifetimes the export makes `'static`, and why a signature's borrows cannot
	/// cross. A handle outlives every call, so it holds the `'static` form of its type:
	/// each lifetime an object takes, in the impl's type, an argument, or the return, is
	/// `'static`, as is each lifetime that must outlive one. A borrow C lends lives only
	/// for the call, so it may take none of them; every other lifetime is the call's.
	fn borrows(&self, def_id: DefId, sig: ty::FnSig<'tcx>) -> (Vec<ty::Region<'tcx>>, Vec<Why>) {
		let tcx = self.tcx;
		// Each `'long: 'short` the signature's bounds name.
		let mut outlives = Vec::new();
		for clause in tcx.clauses_of(def_id).instantiate_identity(tcx).clauses {
			let clause = clause.skip_norm_wip();
			if let Some(pred) = clause.as_region_outlives_clause() {
				let pred = pred.skip_binder();
				outlives.push((pred.0, pred.1));
			} else if let Some(pred) = clause.as_type_outlives_clause() {
				let pred = pred.skip_binder();
				outlives.extend(
					pred.0
						.walk()
						.filter_map(|arg| arg.as_region())
						.map(|long| (long, pred.1)),
				);
			}
		}
		let close = |statics: &mut Vec<ty::Region<'tcx>>| {
			while let Some(&(long, _)) = outlives
				.iter()
				.find(|(long, short)| statics.contains(short) && !statics.contains(long))
			{
				statics.push(long);
			}
		};
		let mut statics = vec![tcx.lifetimes.re_static];
		let parent = tcx.parent(def_id);
		if matches!(tcx.def_kind(parent), DefKind::Impl { .. }) {
			let self_ty = tcx.type_of(parent).instantiate_identity().skip_norm_wip();
			statics.extend(self_ty.walk().filter_map(|arg| arg.as_region()));
		}
		let mut lent = Vec::new();
		for &input in sig.inputs() {
			let (held, borrows) = self.lifetimes(input);
			statics.extend(held);
			lent.extend(borrows.into_iter().map(|region| (region, input)));
		}
		close(&mut statics);
		let mut errs = Vec::new();
		let mut refused = Vec::new();
		for &(region, input) in &lent {
			if statics.contains(&region) && !refused.contains(&input) {
				refused.push(input);
				let mut why = why("'static borrow", input.to_string());
				why.note = self.region_span(def_id, region).map(|span| {
					let note = format!("`{region}` is `'static`, as a handle outlives every call");
					(span, note)
				});
				errs.push(why);
			}
		}
		// A returned object's lifetimes are `'static` too, which no lent borrow may be.
		let returned = statics.len();
		for region in self.lifetimes(sig.output()).0 {
			if !statics.contains(&region) {
				statics.push(region);
			}
		}
		close(&mut statics);
		if lent.iter().any(|(region, _)| statics[returned..].contains(region)) {
			errs.push(why("returned object that borrows", sig.output().to_string()));
		}
		(statics, errs)
	}

	/// Where a named lifetime of `def_id`'s signature is declared.
	fn region_span(&self, def_id: DefId, region: ty::Region<'tcx>) -> Option<Span> {
		let tcx = self.tcx;
		match region.kind() {
			ty::ReEarlyParam(param) => Some(tcx.def_span(tcx.generics_of(def_id).region_param(param, tcx).def_id)),
			ty::ReBound(_, bound) => match bound.kind {
				ty::BoundRegionKind::Named(id) => Some(tcx.def_span(id)),
				_ => None,
			},
			_ => None,
		}
	}

	/// The lifetimes in `ty`: those its objects take, then those it borrows with.
	fn lifetimes(&self, ty: Ty<'tcx>) -> (Vec<ty::Region<'tcx>>, Vec<ty::Region<'tcx>>) {
		let mut held = Vec::new();
		let mut lent = Vec::new();
		for ty in ty.walk().filter_map(|arg| arg.as_type()) {
			match ty.kind() {
				ty::Ref(region, ..) => lent.push(*region),
				ty::Adt(adt, args) if matches!(self.decls.get(&adt.did()), Some(Decl::Object(_))) => {
					held.extend(args.regions())
				}
				// An `impl Future` holds what its bounds name, as an `async fn` its output.
				ty::Alias(
					_,
					ty::AliasTy {
						kind: ty::AliasTyKind::Opaque { def_id },
						args,
						..
					},
				) => {
					let tcx = self.tcx;
					for &(clause, _) in tcx.explicit_item_bounds(*def_id).skip_binder() {
						let term = clause
							.as_projection_clause()
							.and_then(|p| p.skip_binder().term.as_type());
						// Instantiated alone, outside the clause's binder, so the call's own
						// lifetimes stay as the signature binds them.
						if let Some(term) = term {
							let term = ty::EarlyBinder::bind(tcx, term).instantiate(tcx, args).skip_norm_wip();
							let (h, l) = self.lifetimes(term);
							held.extend(h);
							lent.extend(l);
						}
					}
				}
				_ => {}
			}
		}
		(held, lent)
	}

	/// What a function returns: a tuple's elements each, or else one value.
	fn ret(&self, ty: Ty<'tcx>) -> Result<Type, Why> {
		let ty::Tuple(list) = ty.kind() else {
			return self.ty(ty, Pos::Return);
		};
		if list.is_empty() {
			return Ok(Type::Unit);
		}
		let mut elements = Vec::new();
		for element in list.iter() {
			match self.ty(element, Pos::Return)? {
				Type::Unit => return Err(why("unit in a tuple", ty.to_string())),
				element => elements.push(element),
			}
		}
		Ok(Type::Tuple(elements))
	}

	/// `(Bound<T>, Bound<T>)`, which a `RangeBounds<T>` parameter takes.
	fn bounds(&self, int: Ty<'tcx>) -> Option<Ty<'tcx>> {
		let tcx = self.tcx;
		let bound = Ty::new_adt(tcx, tcx.adt_def(self.bound?), tcx.mk_args(&[int.into()]));
		Some(Ty::new_tup(tcx, &[bound, bound]))
	}

	/// The integer a `Bound` holds.
	fn bound_of(&self, ty: Ty<'tcx>) -> Option<Int> {
		match ty.kind() {
			ty::Adt(adt, args) if Some(adt.did()) == self.bound => int(args.type_at(0)),
			_ => None,
		}
	}

	/// Whether `ty` holds an object that is not `Copy`, which a copy would duplicate.
	fn holds_handle(&self, ty: Ty<'tcx>) -> bool {
		let tcx = self.tcx;
		ty.walk().filter_map(|arg| arg.as_type()).any(|ty| match ty.kind() {
			ty::Adt(adt, _) => {
				matches!(self.decls.get(&adt.did()), Some(Decl::Object(_))) && !implements(tcx, adt.did(), sym::Copy)
			}
			_ => false,
		})
	}

	fn is_clone(&self, ty: Ty<'tcx>) -> bool {
		let tcx = self.tcx;
		let Some(clone) = tcx.lang_items().clone_trait() else {
			return false;
		};
		let (infcx, env) = tcx.infer_ctxt().build_with_typing_env(TypingEnv::fully_monomorphized());
		infcx
			.type_implements_trait(clone, [ty], env)
			.must_apply_modulo_regions()
	}

	fn is_send(&self, ty: Ty<'tcx>, env: TypingEnv<'tcx>) -> bool {
		let tcx = self.tcx;
		let Some(send) = tcx.get_diagnostic_item(sym::Send) else {
			return true;
		};
		let (infcx, param_env) = tcx.infer_ctxt().build_with_typing_env(env);
		infcx
			.type_implements_trait(send, [ty], param_env)
			.must_apply_modulo_regions()
	}

	/// A constant: a C constant if a `bool` or integer, or else a function returning it,
	/// which the caller adds.
	fn constant_item(&mut self, def_id: DefId, path: Path) -> Option<Function> {
		let tcx = self.tcx;
		let span = tcx.def_span(def_id);
		let why = match self.constant(def_id) {
			Ok(value) => {
				self.api.constants.push(Constant {
					path,
					span,
					docs: docs(tcx, def_id),
					value,
				});
				return None;
			}
			Err(why) => why,
		};
		let ty = tcx.type_of(def_id).instantiate_identity().skip_norm_wip();
		if why.construct != "constant type" {
			self.refuse(&path, "const", span, vec![why]);
			return None;
		}
		match self.ty(ty, Pos::Return) {
			Ok(ok) => Some(Function {
				path,
				span,
				docs: docs(tcx, def_id),
				receiver: None,
				params: Vec::new(),
				ok,
				err: None,
				future: false,
				eager: false,
				access: Access::Const,
			}),
			Err(why) => {
				self.refuse(&path, "const", span, vec![why]);
				None
			}
		}
	}

	/// A constant's value, if it is a `bool` or integer.
	fn constant(&self, def_id: DefId) -> Result<Literal, Why> {
		let tcx = self.tcx;
		let ty = tcx.type_of(def_id).instantiate_identity().skip_norm_wip();
		let int = match ty.kind() {
			ty::Bool => None,
			ty::Int(_) | ty::Uint(_) => match int(ty) {
				Some(int) => Some(int),
				None => return Err(why("constant type", ty.to_string())),
			},
			_ => return Err(why("constant type", ty.to_string())),
		};
		let Ok(value) = tcx.const_eval_poly(def_id) else {
			return Err(why("constant value", ""));
		};
		let Some(scalar) = value.try_to_scalar_int() else {
			return Err(why("constant value", ""));
		};
		let bits = scalar.to_bits(scalar.size());
		Ok(match int {
			None => Literal::Bool(bits != 0),
			Some(int) => Literal::Int(int, signed(tcx, ty, bits)),
		})
	}

	/// The exported form of `ty` where it appears.
	fn ty(&self, ty: Ty<'tcx>, pos: Pos) -> Result<Type, Why> {
		let tcx = self.tcx;
		Ok(match ty.kind() {
			ty::Tuple(list) if list.is_empty() => match pos {
				Pos::Return => Type::Unit,
				_ => return Err(why("unit outside a return", "")),
			},
			ty::Bool => Type::Bool,
			ty::Char => Type::Char,
			ty::Float(ty::FloatTy::F32) => Type::F32,
			ty::Float(ty::FloatTy::F64) => Type::F64,
			ty::Int(_) | ty::Uint(_) => Type::Int(int(ty).ok_or_else(|| why("128-bit integer", ty.to_string()))?),
			ty::Ref(_, inner, m) => {
				let inner_ty = match inner.kind() {
					ty::Str => Type::Str,
					ty::Slice(elem) if *elem == tcx.types.u8 => Type::Slice,
					ty::Slice(elem) => return Err(why("slice", elem.to_string())),
					ty::Dynamic(..) => return Err(why("trait object", inner.to_string())),
					_ => self.ty(
						*inner,
						match pos {
							Pos::Return | Pos::Returned => Pos::Returned,
							_ => Pos::Behind,
						},
					)?,
				};
				let mutability = match m {
					ty::Mutability::Not => Mutability::Not,
					ty::Mutability::Mut => Mutability::Mut,
				};
				match (&inner_ty, pos, mutability) {
					// Returned as a clone: a handle in it is a new handle to its clone.
					(Type::Str | Type::Slice, Pos::Return | Pos::Returned, _) => {}
					(_, Pos::Return | Pos::Returned, _) if !self.is_clone(*inner) => {
						return Err(why("borrowed return that is not Clone", inner.to_string()));
					}
					(_, Pos::Field, _) => return Err(why("reference field", inner.to_string())),
					(Type::Object(_), Pos::Param, _) => {}
					// Lent only through a handle: one inside a `Box` or `Option` crosses by
					// value, which would move it out of the caller's handle.
					(_, Pos::Param, _) if self.holds_handle(*inner) => {
						return Err(why("reference to a handle inside a value", inner.to_string()));
					}
					(_, Pos::Param, Mutability::Mut) => return Err(why("&mut of a value", inner.to_string())),
					_ => {}
				}
				Type::Ref(mutability, Box::new(inner_ty))
			}
			ty::Adt(adt, args) => {
				let did = adt.did();
				if tcx.lang_items().string() == Some(did) {
					Type::String
				} else if tcx.is_diagnostic_item(sym::Vec, did) {
					match args.type_at(0) == tcx.types.u8 {
						true => Type::Vec,
						false => return Err(why("Vec", args.type_at(0).to_string())),
					}
				} else if Some(did) == self.bytes {
					Type::Bytes
				} else if tcx.is_diagnostic_item(rustc_span::Symbol::intern("Duration"), did) {
					Type::Duration
				} else if tcx.lang_items().range_struct() == Some(did) {
					Type::Range(int(args.type_at(0)).ok_or_else(|| why("Range", args.type_at(0).to_string()))?)
				} else if Some(did) == self.bound {
					Type::Bound(int(args.type_at(0)).ok_or_else(|| why("Bound", args.type_at(0).to_string()))?)
				} else if tcx.is_diagnostic_item(sym::Option, did) {
					let inner = self.ty(args.type_at(0), pos.inner())?;
					// C has one `None` per option, so it cannot hold one inside another, even
					// through a `Box`.
					if matches!(inner.base(), Type::Option(_)) {
						return Err(why("nested Option", ty.to_string()));
					}
					Type::Option(Box::new(inner))
				} else if tcx.is_diagnostic_item(sym::Result, did) {
					return Err(why("Result outside a return", ""));
				} else if adt.is_box() {
					let inner = args.type_at(0);
					match inner.kind() {
						ty::Dynamic(preds, ..) => {
							return self.dynamic(preds.principal_def_id(), Dyn::Box, inner, pos);
						}
						_ => Type::Box(Box::new(self.ty(inner, pos.inner())?)),
					}
				} else if tcx.is_diagnostic_item(sym::Arc, did) {
					let inner = args.type_at(0);
					match inner.kind() {
						ty::Dynamic(preds, ..) => {
							return self.dynamic(preds.principal_def_id(), Dyn::Arc, inner, pos);
						}
						_ => return Err(why("Arc", strip_args(tcx, did))),
					}
				} else if !self.exported(did.krate) {
					return Err(why("foreign type", strip_args(tcx, did)));
				} else {
					match self.decls.get(&did) {
						Some(Decl::Record(i)) => Type::Record(*i),
						Some(Decl::Enum(i)) => Type::Enum(*i),
						// A value holds only values, and handles it can copy.
						Some(Decl::Object(_)) if pos == Pos::Field && !implements(tcx, did, sym::Copy) => {
							return Err(why("object in a value", tcx.def_path_str(did)));
						}
						Some(Decl::Object(i)) => Type::Object(*i),
						Some(Decl::Refused(_)) => return Err(why("refused type", tcx.def_path_str(did))),
						Some(Decl::Callback(_)) | None => {
							let construct = self
								.left_out
								.get(&did)
								.copied()
								.unwrap_or("type not reachable from the crate root");
							return Err(why(construct, tcx.def_path_str(did)));
						}
					}
				}
			}
			ty::Tuple(list) => match list.as_slice() {
				[start, end]
					if start == end
						&& let Some(int) = self.bound_of(*start) =>
				{
					Type::Bounds(int)
				}
				_ => return Err(why("tuple", "")),
			},
			ty::Array(..) => return Err(why("array", ty.to_string())),
			ty::Slice(_) | ty::Str => return Err(why("unsized type", ty.to_string())),
			ty::Dynamic(..) => return Err(why("trait object", ty.to_string())),
			ty::Param(_) => match self.impls.borrow().iter().find(|(param, _)| *param == ty) {
				Some((_, callback)) if pos == Pos::Param => Type::Callback(*callback, Dyn::Impl),
				_ => return Err(why("type parameter", ty.to_string())),
			},
			ty::Closure(..) | ty::FnDef(..) | ty::FnPtr(..) => return Err(why("function value", "")),
			ty::RawPtr(..) => return Err(why("raw pointer", "")),
			ty::Never => return Err(why("never type", "")),
			ty::Alias(..) => return Err(why("opaque type", ty.to_string())),
			_ => return Err(why("type", ty.to_string())),
		})
	}

	/// A trait object of one of the crate's traits, as a foreign implementation.
	/// A foreign implementation crosses only into Rust, as a parameter.
	fn dynamic(&self, principal: Option<DefId>, form: Dyn, ty: Ty<'tcx>, pos: Pos) -> Result<Type, Why> {
		match principal.and_then(|tr| self.decls.get(&tr)) {
			Some(Decl::Callback(i)) if pos == Pos::Param => Ok(Type::Callback(*i, form)),
			_ => Err(why("trait object", ty.to_string())),
		}
	}
}

/// Where a type appears, which decides the forms it may take.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pos {
	Param,
	Return,
	Field,
	/// Inside what a function returns: an `Option`, a `Box`, or behind a reference.
	Returned,
	/// Inside another type.
	Behind,
}

/// Whether a callback can take `ty`: lent to the foreign side for the call, and
/// copied by it if kept.
fn foreign_arg(ty: &Type) -> bool {
	match ty {
		Type::Bool | Type::Int(_) | Type::F32 | Type::F64 | Type::Char | Type::Duration => true,
		Type::String | Type::Vec | Type::Bytes => true,
		Type::Ref(Mutability::Not, inner) => matches!(**inner, Type::Str | Type::Slice),
		Type::Option(inner) => foreign_arg(inner),
		_ => false,
	}
}

/// Whether C can make a `ty` to pass, holding handles only to the objects in `held`.
fn buildable(api: &Api, ty: &Type, held: &[bool]) -> bool {
	match ty {
		Type::Object(i) => held[*i],
		Type::Ref(_, inner) | Type::Box(inner) => buildable(api, inner, held),
		Type::Record(i) => api.records[*i].fields.iter().all(|f| buildable(api, &f.ty, held)),
		Type::Enum(i) => api.enums[*i]
			.variants
			.iter()
			.any(|v| v.fields.iter().all(|f| buildable(api, &f.ty, held))),
		_ => true,
	}
}

/// The objects a returned `ty` may hand C a handle to.
fn returns(api: &Api, ty: &Type, objects: &mut Vec<usize>) {
	match ty {
		Type::Object(i) => objects.push(*i),
		Type::Ref(_, inner) | Type::Box(inner) | Type::Option(inner) => returns(api, inner, objects),
		Type::Tuple(elements) => elements.iter().for_each(|ty| returns(api, ty, objects)),
		Type::Record(i) => api.records[*i].fields.iter().for_each(|f| returns(api, &f.ty, objects)),
		Type::Enum(i) => api.enums[*i]
			.variants
			.iter()
			.flat_map(|v| &v.fields)
			.for_each(|f| returns(api, &f.ty, objects)),
		_ => {}
	}
}

/// Whether `ty` holds a reference inside it, such as `Option<&T>`.
fn lends_inside(ty: &Type) -> bool {
	match ty {
		Type::Ref(_, inner) | Type::Option(inner) | Type::Box(inner) => {
			matches!(**inner, Type::Ref(..)) || lends_inside(inner)
		}
		_ => false,
	}
}

/// Whether a callback can return `ty`: a value with no memory to own.
fn foreign_return(ty: &Type) -> bool {
	matches!(
		ty,
		Type::Unit | Type::Bool | Type::Int(_) | Type::F32 | Type::F64 | Type::Char
	)
}

impl Pos {
	/// The position of a type inside an `Option` or `Box` here: a field's stays a field's,
	/// and a return's is still returned.
	fn inner(self) -> Pos {
		match self {
			Pos::Field => Pos::Field,
			Pos::Return | Pos::Returned => Pos::Returned,
			Pos::Param | Pos::Behind => Pos::Behind,
		}
	}
}

fn int(ty: Ty<'_>) -> Option<Int> {
	Some(match ty.kind() {
		ty::Uint(ty::UintTy::U8) => Int::U8,
		ty::Uint(ty::UintTy::U16) => Int::U16,
		ty::Uint(ty::UintTy::U32) => Int::U32,
		ty::Uint(ty::UintTy::U64) => Int::U64,
		ty::Uint(ty::UintTy::Usize) => Int::Usize,
		ty::Int(ty::IntTy::I8) => Int::I8,
		ty::Int(ty::IntTy::I16) => Int::I16,
		ty::Int(ty::IntTy::I32) => Int::I32,
		ty::Int(ty::IntTy::I64) => Int::I64,
		ty::Int(ty::IntTy::Isize) => Int::Isize,
		_ => return None,
	})
}

/// An integer of type `ty` from its bits: sign-extended if `ty` is signed.
fn signed<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, bits: u128) -> i128 {
	let width = match int(ty) {
		Some(Int::U8 | Int::I8) => 8,
		Some(Int::U16 | Int::I16) => 16,
		Some(Int::U32 | Int::I32) => 32,
		Some(Int::U64 | Int::I64) => 64,
		_ => tcx.data_layout.pointer_size().bits() as u32,
	};
	match ty.is_signed() {
		true => {
			let shift = 128 - width;
			((bits << shift) as i128) >> shift
		}
		false => bits as i128,
	}
}

/// An item's doc comment, each line as written after `///`.
fn docs(tcx: TyCtxt<'_>, def_id: DefId) -> String {
	#[expect(deprecated, reason = "doc comments are no parsed attribute to find")]
	let attrs = tcx.get_all_attrs(def_id);
	let lines: Vec<String> = attrs
		.iter()
		.filter_map(|attr| attr.doc_str())
		.flat_map(|doc| {
			doc.as_str()
				.lines()
				.map(|line| line.strip_prefix(' ').unwrap_or(line).to_string())
				.collect::<Vec<_>>()
		})
		.collect();
	lines.join("\n").trim().to_string()
}

/// Whether the local type `def_id` implements the std trait `item`.
fn implements(tcx: TyCtxt<'_>, def_id: DefId, item: rustc_span::Symbol) -> bool {
	let ty = tcx.type_of(def_id).instantiate_identity().skip_norm_wip();
	let Some(tr) = tcx.get_diagnostic_item(item) else {
		return false;
	};
	let (infcx, env) = tcx
		.infer_ctxt()
		.build_with_typing_env(TypingEnv::post_analysis(tcx, def_id));
	infcx.type_implements_trait(tr, [ty], env).must_apply_modulo_regions()
}

/// A type's own generic arguments as it is exported: its lifetimes `'static`.
fn statics<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId) -> ty::GenericArgsRef<'tcx> {
	ty::GenericArgs::for_item(tcx, def_id, |param, _| match param.kind {
		ty::GenericParamDefKind::Lifetime => tcx.lifetimes.re_static.into(),
		_ => tcx.mk_param_from_def(param),
	})
}

/// A type's path without generic arguments.
fn strip_args(tcx: TyCtxt<'_>, did: DefId) -> String {
	tcx.def_path_str(did)
}
