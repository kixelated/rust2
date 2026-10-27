//! Iterators, as `rt/iter.ts` gives them: std's iterators are `rt.Iter` objects whose
//! `next` returns an item or `rt.DONE`, and whose adaptors and consumers are runtime
//! functions taking the iterator first: `rt.iterMap(it, f)`. The crate's own `impl
//! Iterator` keeps that protocol in its `next`, and joins std's adaptors through
//! `rt.iterFrom`.

use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, Instance, Ty, TypeVisitableExt};
use rustc_span::Span;

use super::Translator;
use super::body::Body;
use super::library::{self, Call, Std};
use super::num::Int;
use crate::ts::{Expr, Param, Type};

/// `Iterator::next`, and the trait's `Item`.
fn trait_items(tcx: ty::TyCtxt<'_>) -> Option<(DefId, DefId)> {
	let iterator = tcx.get_diagnostic_item(rustc_span::sym::Iterator)?;
	let items = tcx.associated_items(iterator);
	let next = items.filter_by_name_unhygienic(rustc_span::sym::next).next()?.def_id;
	let item = items.filter_by_name_unhygienic(rustc_span::sym::Item).next()?.def_id;
	Some((next, item))
}

/// Whether `def_id` is `Iterator::next`, or an impl's.
pub fn is_next(tcx: ty::TyCtxt<'_>, def_id: DefId) -> bool {
	let Some((next, _)) = trait_items(tcx) else {
		return false;
	};
	def_id == next || tcx.trait_item_of(def_id) == Some(next)
}

impl<'tcx> Translator<'_, 'tcx> {
	/// `Iterator::Item` of `ty`, if `ty` is an iterator. A type over parameters, which a
	/// generic type's fields and a census have, may leave it a projection.
	pub fn iter_item(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
		let tcx = self.tcx;
		let (_, item) = trait_items(tcx)?;
		let projection = Ty::new_projection(tcx, ty::IsRigid::No, item, [ty]);
		tcx.try_normalize_erasing_regions(self.typing_env_pub(), ty::Unnormalized::new_wip(projection))
			.ok()
	}
}

/// The crate's own iterator a `ty` is, through any references: its `next`, whose
/// instance the translation names.
fn own_next<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> Option<Instance<'tcx>> {
	let tcx = b.t.tcx;
	let ty = ty.peel_refs();
	// Over parameters, only a census's instance, which never runs, would call it.
	if !matches!(ty.kind(), ty::Adt(adt, _) if adt.did().is_local()) || ty.has_param() || ty.has_aliases() {
		return None;
	}
	let (next, _) = trait_items(tcx)?;
	Instance::try_resolve(tcx, b.t.typing_env_pub(), next, tcx.mk_args(&[ty.into()])).ok()?
}

/// Refuse an `impl Iterator` of the crate, whose `next` is `next_fn`, that std's adaptors
/// would read other than through `next`: one that overrides `fold`, `nth`, or `size_hint`,
/// which they call in its place or beside it, that has a `next_back` or `len`, which this
/// protocol lacks, or that is a `FusedIterator`, which `fuse` reads on past its end.
pub fn check_impl(t: &mut Translator<'_, '_>, next_fn: DefId) {
	let tcx = t.tcx;
	let Some(imp) = tcx.impl_of_assoc(next_fn) else {
		return;
	};
	for item in tcx.associated_items(imp).in_definition_order() {
		let name = item.name();
		if item.is_fn() && name != rustc_span::sym::next {
			t.err(
				tcx.def_span(item.def_id),
				format!("overriding `Iterator::{name}`, which std's adaptors call in place of `next` or beside it"),
			);
		}
	}
	let ty::Adt(adt, _) = tcx.type_of(imp).instantiate_identity().skip_norm_wip().kind() else {
		return;
	};
	for (&tr, impls) in tcx.all_local_trait_impls(()) {
		let path = library::std_path(tcx, tr);
		let Some(name) = path.last().filter(|n| {
			matches!(
				n.as_str(),
				"DoubleEndedIterator" | "ExactSizeIterator" | "FusedIterator"
			)
		}) else {
			continue;
		};
		for other in impls {
			let self_ty = tcx.type_of(other.to_def_id()).instantiate_identity().skip_norm_wip();
			if matches!(self_ty.kind(), ty::Adt(other, _) if other.did() == adt.did()) {
				t.err(
					tcx.def_span(other.to_def_id()),
					format!("an iterator of the crate implementing `{name}`, which is not supported yet"),
				);
			}
		}
	}
}

/// `iter.next()` by the protocol, for an iterator of type `ty`: an item or `rt.DONE`.
pub fn next<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, iter: Expr) -> Expr {
	match own_next(b, ty) {
		Some(instance) => {
			let sym = b.t.instance(instance);
			Expr::Sym(sym).call(vec![iter])
		}
		None => iter.member("next").call(Vec::new()),
	}
}

/// What `IntoIterator::into_iter` makes of `value`, a `ty`: an `rt.Iter`, or `None` for a
/// type without a mapping.
pub fn into_iter<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, value: Expr, span: Span) -> Option<Expr> {
	let tcx = b.t.tcx;
	no_drops(b, ty, span);
	if b.t.iter_item(ty).is_some() {
		return Some(match own_next(b, ty) {
			Some(instance) => {
				let sym = b.t.instance(instance);
				let param_ty = b.t.ty(ty.peel_refs(), span);
				let it = "it".to_string();
				let next = Expr::Sym(sym).call(vec![Expr::ident(&it)]);
				let param = Param {
					name: it,
					ty: Some(param_ty),
				};
				Expr::rt("iterFrom", vec![value, Expr::Lambda(vec![param], Box::new(next))])
			}
			None => value,
		});
	}
	// Items borrowed from what `ty` points at.
	let items = |iter: Expr, elem: Ty<'tcx>| match ty.is_ref() {
		true => by_ref(iter, elem),
		false => iter,
	};
	match ty.peel_refs().kind() {
		ty::Slice(elem) | ty::Array(elem, _) => {
			Some(items(Expr::rt("iterSlice", vec![library::elements(value, ty)]), *elem))
		}
		ty::Adt(adt, args) => match Std::of(tcx, adt.did())? {
			Std::Vec => Some(items(
				Expr::rt("iterSlice", vec![library::elements(value, ty)]),
				args.type_at(0),
			)),
			Std::Option => {
				option_ok(b, args.type_at(0), span);
				Some(Expr::rt("iterOption", vec![value]))
			}
			_ => super::collections::into_iter(b, ty, value),
		},
		_ => None,
	}
}

/// `iter`, over the elements of a slice or `Vec` of `elem`s, yielding references to them:
/// a slice of each array of bytes.
fn by_ref(iter: Expr, elem: Ty<'_>) -> Expr {
	match super::byte_array(elem) {
		true => Expr::rt("iterMap", vec![iter, Expr::Rt("byteSliceFrom".into())]),
		false => iter,
	}
}

/// A function doing `into_iter` on each `ty`, or `None` where it is the value itself.
fn into_fn<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span) -> Option<Option<Expr>> {
	if b.t.iter_item(ty).is_some() && own_next(b, ty).is_none() {
		return Some(None);
	}
	let param = Param {
		name: "value".into(),
		ty: Some(b.t.ty(ty, span)),
	};
	let body = into_iter(b, ty, Expr::ident("value"), span)?;
	Some(Some(Expr::Lambda(vec![param], Box::new(body))))
}

/// Refuse an iterator that owns a `ty` that drops: an `rt.Iter` drops nothing it skips,
/// consumes, or holds when it is dropped.
fn no_drops<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span) {
	if b.t.drops(ty) {
		b.t.err(
			span,
			format!("an iterator that owns `{ty}`, which drops, which is not supported yet"),
		);
	}
}

/// Refuse an `Option` of `ty` that `T | undefined` can't hold, as the type would be.
pub fn option_ok<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span) {
	if b.t.erases_to(ty).undefined {
		b.t.err(
			span,
			format!("`Option<{ty}>` is not supported: its `None` and `Some` would both be `undefined`"),
		);
	}
}

/// The mapping of a call on or to an iterator, if it is one.
pub fn call<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	call: &Call<'tcx>,
	owner: &str,
	name: &str,
	mut args: Vec<Expr>,
) -> Option<Expr> {
	let tcx = b.t.tcx;
	let span = call.span;
	let gargs = call.args;
	let targ = |n: usize| gargs.types().nth(n);
	let path = library::std_path(tcx, call.def_id);
	let path: Vec<&str> = path.iter().map(String::as_str).collect();

	// Where iterators come from.
	match (owner, name) {
		("slice", "iter" | "iter_mut") => {
			let s = args.pop()?;
			let s = library::elements(s, Ty::new_slice(tcx, targ(0)?));
			return Some(by_ref(Expr::rt("iterSlice", vec![s]), targ(0)?));
		}
		("Option", "iter" | "iter_mut") => {
			option_ok(b, targ(0)?, span);
			return Some(Expr::rt("iterOption", args));
		}
		("IntoIterator", "into_iter") => {
			let value = args.pop()?;
			return into_iter(b, targ(0)?, value, span);
		}
		_ => {}
	}
	match path.as_slice() {
		["iter", "sources", "once", "once"] => {
			no_drops(b, targ(0)?, span);
			return Some(Expr::rt("iterOnce", args));
		}
		["iter", "sources", "empty", "empty"] => return Some(Expr::rt("iterEmpty", Vec::new())),
		["iter", "sources", "from_fn", "from_fn"] => return Some(Expr::rt("iterFromFn", args)),
		_ => {}
	}

	// `Vec::extend` with an iterator.
	if (owner, name) == ("Extend", "extend") {
		let (vec, item, source) = (targ(0)?, targ(1)?, targ(2)?);
		let ty::Adt(adt, vec_args) = vec.kind() else {
			return None;
		};
		if Std::of(tcx, adt.did()) != Some(Std::Vec) {
			return None;
		}
		let [v, items] = <[Expr; 2]>::try_from(args).ok()?;
		let mut items = into_iter(b, source, items, span)?;
		// `Extend<&T>` copies each element.
		if item != vec_args.type_at(0)
			&& let Some(copy) = b.t.copy_fn(vec_args.type_at(0), span)
		{
			items = Expr::rt("iterMap", vec![items, copy]);
		}
		let extend = match vec_args.type_at(0).kind() {
			ty::Uint(ty::UintTy::U8) => "iterExtendBytes",
			_ => "iterExtend",
		};
		return Some(Expr::rt(extend, vec![v, items]));
	}

	let self_ty = match (owner, name) {
		("Iterator" | "DoubleEndedIterator" | "ExactSizeIterator", _) => targ(0)?,
		("Peekable", "peek" | "next_if") if library::inherent_std(tcx, call.def_id) == Some(Std::Peekable) => {
			let imp = tcx.inherent_impl_of_assoc(call.def_id)?;
			let generics = tcx.generics_of(imp).count();
			tcx.type_of(imp).instantiate(tcx, &gargs[..generics]).skip_norm_wip()
		}
		_ => return None,
	};
	let item = b.t.iter_item(self_ty.peel_refs())?;
	// The crate's own `next` hands its item over; every other call may leave items behind.
	if name != "by_ref" && !(name == "next" && own_next(b, self_ty).is_some()) {
		no_drops(b, item, span);
	}
	let mut values = args.into_iter();
	let recv = values.next()?;
	let rest: Vec<Expr> = values.collect();

	// `next` by the protocol, as an `Option`.
	if matches!(name, "next" | "next_back" | "nth_back") {
		option_ok(b, item, span);
		let item = match name {
			"next" => next(b, self_ty, recv),
			"next_back" => recv.member("nextBack").call(Vec::new()),
			_ => recv.member("nthBackItem").call(rest),
		};
		return Some(Expr::rt("iterToOption", vec![item]));
	}
	// The same object, borrowed.
	if name == "by_ref" {
		return Some(recv);
	}

	let recv = into_iter(b, self_ty, recv, span)?;
	// `rt.iter<Name>(recv, ...args)`, or `peekable<Name>` of a `Peekable`'s own.
	let method = |name: &str, args: Vec<Expr>| {
		let of = match owner {
			"Peekable" => "peekable",
			_ => "iter",
		};
		Some(Expr::rt_of(of, name).call([vec![recv.clone()], args].concat()))
	};
	match name {
		"map" | "filter" | "filter_map" | "enumerate" | "rev" | "skip" | "take" | "skip_while" | "take_while"
		| "map_while" | "step_by" | "inspect" | "fuse" | "peekable" | "count" | "any" | "all" | "find_map"
		| "position" | "fold" | "for_each" => method(&super::camel(name), rest),
		// `ExactSizeIterator::len` is each iterator's own.
		"len" => Some(recv.member("len").call(rest)),
		"last" | "nth" | "find" | "peek" | "next_if" => {
			option_ok(b, item, span);
			method(&super::camel(name), rest)
		}
		"max" | "min" if primitive(item.peel_refs()) => {
			option_ok(b, item, span);
			method(name, rest)
		}
		"max_by_key" | "min_by_key" if primitive(targ(1)?) => {
			option_ok(b, item, span);
			method(&super::camel(name), rest)
		}
		"chain" => {
			let other = into_iter(b, targ(1)?, rest.into_iter().next()?, span)?;
			method(name, vec![other])
		}
		"zip" => {
			let other = into_iter(b, targ(1)?, rest.into_iter().next()?, span)?;
			let sig = tcx.fn_sig(call.def_id).instantiate(tcx, gargs).skip_norm_wip();
			let zip = b.t.subst(
				ty::List::empty(),
				tcx.instantiate_bound_regions_with_erased(sig).output(),
			);
			let ty::Adt(_, sides) = zip.kind() else {
				return None;
			};
			let mut args = vec![other];
			if zip_reads_by_index(b, sides.type_at(0), sides.type_at(1), span) {
				args.push(Expr::lit("true"));
			}
			method(name, args)
		}
		"flat_map" => {
			let mut rest = rest;
			rest.extend(into_fn(b, targ(1)?, span)?);
			method("flatMap", rest)
		}
		"flatten" => {
			let into = into_fn(b, item, span)?;
			method("flatten", into.into_iter().collect())
		}
		"copied" | "cloned" => {
			let ty::Ref(_, inner, _) = item.kind() else {
				return None;
			};
			// A `Clone` written by hand may do other than copy, even for a `Copy` type.
			let clones = name == "cloned" && !library::clone_copies(b.t, *inner);
			match () {
				_ if clones => {
					let clone = tcx.lang_items().clone_fn()?;
					let clone = b.fn_value(clone, tcx.mk_args(&[(*inner).into()]), span);
					method("map", vec![clone])
				}
				// Out of the slice each item is.
				_ if super::byte_array(*inner) => method("map", vec![Expr::Rt("byteSliceToArray".into())]),
				_ if b.t.needs_copy(*inner) => method("map", b.t.copy_fn(*inner, span).into_iter().collect()),
				// A JS string is a value already.
				_ if b.t.is_copy(*inner) || library::string(tcx, *inner) => Some(recv),
				// Each item through its `Clone`.
				_ => {
					let clone = tcx.lang_items().clone_fn()?;
					let clone = b.fn_value(clone, tcx.mk_args(&[(*inner).into()]), span);
					method("map", vec![clone])
				}
			}
		}
		"sum" | "product" => {
			let int = Int::of(targ(1)?)?;
			let (op, unit) = match name {
				"sum" => ("add", 0),
				_ => ("mul", 1),
			};
			method(name, vec![int.func(op), int.lit(unit)])
		}
		"collect" => collect(b, targ(1)?, item, recv, span),
		_ => None,
	}
}

/// Whether std's `Zip` of `a` and `b` reads both by index, never past the shorter: when
/// both are `TrustedRandomAccess`. Refuse one that `fold` reads that way but `next` does
/// not, where the first calls a closure that would see the difference.
fn zip_reads_by_index<'tcx>(b: &mut Body<'_, '_, 'tcx>, a: Ty<'tcx>, other: Ty<'tcx>, span: Span) -> bool {
	let both = |b: &mut Body<'_, '_, 'tcx>, name: &str| implements(b.t, a, name) && implements(b.t, other, name);
	if both(b, "TrustedRandomAccess") {
		return true;
	}
	if (both(b, "TrustedRandomAccessNoCoerce") || both(b, "TrustedLen")) && calls_closures(b.t.tcx, a) {
		b.t.err(
			span,
			format!("a `zip` of `{a}`, whose closures std calls one item further through `next` than through `fold`"),
		);
	}
	false
}

/// Whether `ty`, an iterator, has std's closure-calling adaptors in it, which std counts as
/// side effects.
fn calls_closures(tcx: ty::TyCtxt<'_>, ty: Ty<'_>) -> bool {
	ty.walk().filter_map(|arg| arg.as_type()).any(|t| match t.kind() {
		ty::Adt(adt, _) => matches!(
			library::std_path(tcx, adt.did()).last().map(String::as_str),
			Some("Map" | "Cloned" | "Inspect")
		),
		_ => false,
	})
}

/// Whether `ty` implements `core`'s trait `name`, one std specializes on: an iterator
/// marker it names nowhere else, or `Eq`.
pub(super) fn implements<'tcx>(t: &Translator<'_, 'tcx>, ty: Ty<'tcx>, name: &str) -> bool {
	use rustc_infer::infer::TyCtxtInferExt;
	use rustc_trait_selection::infer::InferCtxtExt;
	let tcx = t.tcx;
	let Some(&core) = tcx
		.crates(())
		.iter()
		.find(|&&c| tcx.crate_name(c) == rustc_span::sym::core)
	else {
		return false;
	};
	let Some(&tr) = tcx.traits(core).iter().find(|&&tr| tcx.item_name(tr).as_str() == name) else {
		return false;
	};
	let (infcx, env) = tcx.infer_ctxt().build_with_typing_env(t.typing_env_pub());
	infcx.type_implements_trait(tr, [ty], env).must_apply_modulo_regions()
}

/// Whether `ty` compares in JS as Rust's `Ord` does: an integer, a `bool`, or a `char`.
fn primitive(ty: Ty<'_>) -> bool {
	matches!(ty.kind(), ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_))
}

/// `iter.collect::<ty>()` of `item`s.
fn collect<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, item: Ty<'tcx>, iter: Expr, span: Span) -> Option<Expr> {
	let tcx = b.t.tcx;
	let ty::Adt(adt, args) = ty.kind() else {
		return None;
	};
	match Std::of(tcx, adt.did())? {
		Std::Vec => Some(match args.type_at(0).kind() {
			ty::Uint(ty::UintTy::U8) => Expr::rt("iterToByteVec", vec![iter]),
			_ => Expr::rt("iterToArray", vec![iter]),
		}),
		// The values, collected as the `Ok` or `Some` type is, until the first `Err` or `None`.
		std @ (Std::Result | Std::Option) => {
			let ty::Adt(_, item_args) = item.kind() else {
				return None;
			};
			let value = item_args.type_at(0);
			if std == Std::Option {
				option_ok(b, args.type_at(0), span);
			}
			let value_ty = b.t.ty(value, span);
			let values = collect(b, args.type_at(0), value, Expr::ident("values"), span)?;
			let param = Param {
				name: "values".into(),
				ty: Some(Type::Rt("Iter", vec![value_ty])),
			};
			let collect = match std {
				Std::Result => "iterCollectResult",
				_ => "iterCollectOption",
			};
			Some(Expr::rt(
				collect,
				vec![iter, Expr::Lambda(vec![param], Box::new(values))],
			))
		}
		_ => super::collections::collect(b, ty, iter, span),
	}
}
