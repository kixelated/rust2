//! The crate's `pub` API, as rustc checked it, to the [IR](crate::ir).
//!
//! Every `pub` item reachable from the crate root is exported by fixed rules, or refused
//! with its span. Nothing is opted in by attribute and nothing is skipped silently: the
//! config's `exclude` is the only way to leave an item out, and `#[doc(hidden)]` items,
//! which are no part of the documented API, are listed in the census.

use std::collections::VecDeque;

use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{CRATE_DEF_ID, DefId};
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
}

fn why(construct: &'static str, detail: impl Into<String>) -> Why {
	Why {
		construct,
		detail: detail.into(),
	}
}

/// An item the export refuses, with every reason found.
pub struct Refusal {
	/// Its path from the crate root.
	pub path: String,
	/// What kind of item it is: `fn`, `method`, `struct`, ...
	pub kind: &'static str,
	pub span: Span,
	pub why: Vec<Why>,
}

/// What collection found.
pub struct Collected {
	pub api: Api,
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
}

/// How a local type is exported, once decided.
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
	refusals: Vec<Refusal>,
	hidden: Vec<String>,
	excluded: Vec<(String, usize)>,
	aliases: Vec<String>,
	/// Local items the walk left out, and why: `excluded` or `doc(hidden)`.
	left_out: rustc_data_structures::fx::FxHashMap<DefId, &'static str>,
	/// The type parameters of the function being mapped that take a foreign
	/// implementation, with the callback each takes.
	impls: std::cell::RefCell<Vec<(Ty<'tcx>, usize)>>,
	opaque: Vec<(String, Why)>,
	twins: Vec<String>,
	bytes: Option<DefId>,
	api: Api,
}

/// Collect the crate's exported API, leaving out what `exclude` names.
pub fn collect<'tcx>(tcx: TyCtxt<'tcx>, exclude: &[String]) -> Collected {
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
	let mut c = Collector {
		tcx,
		items: Vec::new(),
		decls: Default::default(),
		refusals: Vec::new(),
		hidden: Vec::new(),
		excluded: exclude.iter().map(|path| (path.clone(), 0)).collect(),
		aliases: Vec::new(),
		left_out: Default::default(),
		impls: Default::default(),
		opaque: Vec::new(),
		twins: Vec::new(),
		bytes,
		api: Api {
			// The library's name, which may not be the package's.
			krate: tcx.crate_name(rustc_hir::def_id::LOCAL_CRATE).to_string(),
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
	Collected {
		api: c.api,
		refusals: c.refusals,
		opaque: c.opaque,
		twins: c.twins,
		hidden: c.hidden,
		excluded: c.excluded,
		aliases: c.aliases,
	}
}

impl<'tcx> Collector<'tcx> {
	/// Whether the config excludes `path`, counting the exclusion.
	fn exclude(&mut self, path: &[String]) -> bool {
		let joined = path.join("::");
		for (excluded, count) in &mut self.excluded {
			if joined == *excluded || joined.starts_with(&format!("{excluded}::")) {
				*count += 1;
				return true;
			}
		}
		false
	}

	fn refuse(&mut self, path: &[String], kind: &'static str, span: Span, why: Vec<Why>) {
		self.refusals.push(Refusal {
			path: path.join("::"),
			kind,
			span,
			why,
		});
	}

	/// Every `pub` item reachable from the crate root, breadth first so each keeps its
	/// shortest path.
	fn walk(&mut self) {
		let tcx = self.tcx;
		let mut queue = VecDeque::from([(CRATE_DEF_ID, Vec::new())]);
		let mut seen = rustc_data_structures::fx::FxHashSet::default();
		while let Some((module, path)) = queue.pop_front() {
			for child in tcx.module_children_local(module) {
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
				if !seen.insert(def_id) {
					continue;
				}
				if self.exclude(&path) {
					self.left_out.insert(def_id, "excluded type");
					continue;
				}
				if tcx.is_doc_hidden(def_id) {
					self.hidden.push(path.join("::"));
					self.left_out.insert(def_id, "doc(hidden) type");
					continue;
				}
				let span = child
					.reexport_chain
					.first()
					.and_then(|reexport| reexport.id())
					.map_or_else(|| tcx.def_span(def_id), |id| tcx.def_span(id));
				let Some(local) = def_id.as_local() else {
					let krate = tcx.crate_name(def_id.krate);
					self.refuse(
						&path,
						"use",
						span,
						vec![why("re-export of another crate's item", krate.to_string())],
					);
					continue;
				};
				match kind {
					DefKind::Mod => queue.push_back((local, path)),
					DefKind::TyAlias => self.aliases.push(path.join("::")),
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

	/// Decide how each type and trait is exported, before any signature names one: a
	/// struct whose fields are all `pub`, or an enum, is a value if its fields map, and
	/// any other type is an object.
	fn declare(&mut self) {
		let tcx = self.tcx;
		for (def_id, _) in self.items.clone() {
			let generic = || {
				tcx.generics_of(def_id)
					.own_params
					.iter()
					.any(|p| p.name != rustc_span::kw::SelfUpper)
			};
			let decl = match tcx.def_kind(def_id) {
				DefKind::Union => Decl::Refused(why("union", "")),
				DefKind::Struct | DefKind::Enum | DefKind::Trait if generic() => Decl::Refused(why("generic type", "")),
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
					api.objects.push(Object {
						path,
						span,
						docs,
						sync,
						copy,
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
		match implements(self.tcx, def_id, sym::Send) {
			true => Decl::Object(0),
			false => Decl::Refused(why("object not Send", "")),
		}
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
					self.opaque.push((path.join("::"), first));
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
		let identity = ty::GenericArgs::identity_for_item(tcx, def_id);
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
		let self_ty = tcx.type_of(def_id).instantiate_identity().skip_norm_wip();
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
					self.hidden.push(method.join("::"));
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
				true => self.twins.push(method.join("::")),
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
		// Whether what it returns borrows: any lifetime but `'static` in it.
		let borrows = sig.skip_binder().output().walk().any(|arg| match arg.kind() {
			GenericArgKind::Lifetime(region) => !region.is_static(),
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
		let mut params = Vec::new();
		for (i, ty) in inputs {
			let name = names
				.get(i)
				.copied()
				.flatten()
				.map_or_else(|| format!("arg{i}"), |ident| ident.to_string());
			match self.ty(ty, Pos::Param) {
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
			ty::Adt(adt, args) if tcx.is_diagnostic_item(sym::Result, adt.did()) => (
				self.ty(args.type_at(0), Pos::Return),
				Some(self.ty(args.type_at(1), Pos::Return)),
			),
			_ => (self.ty(output, Pos::Return), None),
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
	/// `Self`), its lifetimes erased, and each type parameter by the rules in
	/// [`Self::choose`].
	fn instantiate(&self, def_id: DefId, self_ty: Option<Ty<'tcx>>) -> Result<ty::GenericArgsRef<'tcx>, Vec<Why>> {
		let tcx = self.tcx;
		let generics = tcx.generics_of(def_id);
		let identity = ty::GenericArgs::identity_for_item(tcx, def_id);
		if !generics.requires_monomorphization(tcx) {
			return Ok(identity);
		}
		let mut errs = Vec::new();
		let mut args: Vec<ty::GenericArg<'tcx>> = identity.to_vec();
		let parent = generics.parent_count;
		// A trait method's `Self` stays a parameter: the trait is the callback.
		let _ = self_ty;
		for (i, arg) in identity.iter().enumerate().skip(parent) {
			match arg.kind() {
				GenericArgKind::Lifetime(_) => args[i] = tcx.lifetimes.re_erased.into(),
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
		let mut candidates = Vec::new();
		for bound in &bounds {
			let tr = bound.def_id();
			if tcx.is_diagnostic_item(sym::Into, tr) || tcx.is_diagnostic_item(sym::AsRef, tr) {
				let target = bound.skip_binder().trait_ref.args.type_at(1);
				candidates.push(match target.kind() {
					ty::Str => Ty::new_imm_ref(tcx, tcx.lifetimes.re_static, target),
					ty::Slice(_) => Ty::new_imm_ref(tcx, tcx.lifetimes.re_static, target),
					_ => target,
				});
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
					_ => self.ty(*inner, Pos::Behind)?,
				};
				let mutability = match m {
					ty::Mutability::Not => Mutability::Not,
					ty::Mutability::Mut => Mutability::Mut,
				};
				match (&inner_ty, pos, mutability) {
					// Returned as a copy: plain data reads the same either way, and so does a
					// `Copy` handle's value, but a clone of another handle is a new one.
					(_, Pos::Return, _) if self.holds_handle(*inner) => {
						return Err(why("borrowed object return", inner.to_string()));
					}
					(Type::Str | Type::Slice, Pos::Return, _) => {}
					(_, Pos::Return, _) if !self.is_clone(*inner) => {
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
				} else if !did.is_local() {
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
			ty::Tuple(_) => return Err(why("tuple", "")),
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
	/// Inside another type: an `Option`, a `Box`, or behind a reference.
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

/// Whether a callback can return `ty`: a value with no memory to own.
fn foreign_return(ty: &Type) -> bool {
	matches!(
		ty,
		Type::Unit | Type::Bool | Type::Int(_) | Type::F32 | Type::F64 | Type::Char
	)
}

impl Pos {
	/// The position of a type inside an `Option` or `Box` here: a field's stays a field's.
	fn inner(self) -> Pos {
		match self {
			Pos::Field => Pos::Field,
			_ => Pos::Behind,
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

/// A type's path without generic arguments.
fn strip_args(tcx: TyCtxt<'_>, did: DefId) -> String {
	tcx.def_path_str(did)
}
