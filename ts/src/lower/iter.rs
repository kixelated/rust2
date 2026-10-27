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
/// type without a mapping. One that owns what it yields, by value, is given what drops
/// each, and the crate's own iterator what drops it.
pub fn into_iter<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, value: Expr, span: Span) -> Option<Expr> {
	let tcx = b.t.tcx;
	let owned = !ty.is_ref();
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
				let mut args = vec![value, Expr::Lambda(vec![param], Box::new(next))];
				if owned && b.t.drops(ty) {
					args.push(b.t.dropper(ty, span));
				}
				Expr::rt("iterFrom", args)
			}
			// One borrowed by `&mut`, which std's adaptors may drop once it ends, but which
			// is its owner's to drop, and which a consumer reads on through the borrow
			// rather than by its by-value overrides, as `Chain::count` ends both halves.
			None if !owned => Expr::rt("iterBorrow", vec![value]),
			None => value,
		});
	}
	// What drops each item of one that owns them.
	let each = |b: &mut Body<'_, '_, 'tcx>, elem: Ty<'tcx>, mut args: Vec<Expr>| {
		if owned && b.t.drops(elem) {
			args.push(b.t.dropper(elem, span));
		}
		args
	};
	// Items borrowed from what `ty` points at.
	let items = |iter: Expr, elem: Ty<'tcx>| match ty.is_ref() {
		true => by_ref(iter, elem),
		false => iter,
	};
	match ty.peel_refs().kind() {
		ty::Slice(elem) | ty::Array(elem, _) => {
			let values = each(b, *elem, vec![library::elements(value, ty)]);
			Some(items(Expr::rt("iterSlice", values), *elem))
		}
		ty::Adt(adt, args) => match Std::of(tcx, adt.did())? {
			Std::Vec => {
				let values = each(b, args.type_at(0), vec![library::elements(value, ty)]);
				Some(items(Expr::rt("iterSlice", values), args.type_at(0)))
			}
			Std::Option => {
				option_ok(b, args.type_at(0), span);
				Some(Expr::rt("iterOption", each(b, args.type_at(0), vec![value])))
			}
			_ => super::collections::into_iter(b, ty, value, span),
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
	let body = into_iter(b, ty, Expr::ident("value"), span)?;
	if matches!(&body, Expr::Ident(name) if name == "value") {
		return Some(None);
	}
	let param = Param {
		name: "value".into(),
		ty: Some(b.t.ty(ty, span)),
	};
	Some(Some(Expr::Lambda(vec![param], Box::new(body))))
}

/// `<ty as IntoIterator>::IntoIter`: the iterator `into_iter` makes of a `ty`.
pub(super) fn iter_of<'tcx>(t: &Translator<'_, 'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
	let tcx = t.tcx;
	let into = tcx.get_diagnostic_item(rustc_span::sym::IntoIterator)?;
	let item = tcx
		.associated_items(into)
		.filter_by_name_unhygienic(rustc_span::Symbol::intern("IntoIter"))
		.next()?
		.def_id;
	let projection = Ty::new_projection(tcx, ty::IsRigid::No, item, [ty]);
	tcx.try_normalize_erasing_regions(t.typing_env_pub(), ty::Unnormalized::new_wip(projection))
		.ok()
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

/// The mapping of a call on or to an iterator, if it is one. Where std drops an item it
/// skips, rejects, or holds, or an iterator it takes by value, the runtime function is
/// given what drops it after its own arguments.
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
			let dropper = b.t.dropper(targ(0)?, span);
			return Some(Expr::rt("iterOnce", tail(args, vec![dropper])));
		}
		["iter", "sources", "empty", "empty"] => return Some(Expr::rt("iterEmpty", Vec::new())),
		["iter", "sources", "from_fn", "from_fn"] => return Some(Expr::rt("iterFromFn", args)),
		_ => {}
	}

	// `Vec::extend` with an iterator, which it drops once done.
	if (owner, name) == ("Extend", "extend") {
		let (vec, item, source) = (targ(0)?, targ(1)?, targ(2)?);
		let ty::Adt(adt, vec_args) = vec.kind() else {
			return None;
		};
		if Std::of(tcx, adt.did()) != Some(Std::Vec) {
			return None;
		}
		let [v, items] = <[Expr; 2]>::try_from(args).ok()?;
		let iter = iter_of(b.t, source)?;
		// Of a `TrustedLen` iterator, by `for_each`.
		if implements(b.t, iter, "TrustedLen") {
			folds(b, iter, "extend", span);
		}
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
		let dropper = iter_dropper(b, iter, span);
		return Some(Expr::rt(extend, tail(vec![v, items], vec![dropper])));
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
	let mut values = args.into_iter();
	let recv = values.next()?;
	let rest: Vec<Expr> = values.collect();

	// `next` by the protocol, as an `Option`.
	if matches!(name, "next" | "next_back" | "nth_back") {
		option_ok(b, item, span);
		let item = match name {
			"next" => next(b, self_ty, recv),
			"next_back" => recv.member("nextBack").call(Vec::new()),
			_ => {
				let dropper = b.t.dropper(item, span);
				if !undefined(&dropper) {
					return Some(Expr::rt("ownedNthBack", [vec![recv], rest, vec![dropper]].concat()));
				}
				recv.member("nthBackItem").call(rest)
			}
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
	// `rt.owned<Name>`, which drops by `droppers` as std's own does, where any drops: else
	// `rt.iter<Name>`, so a bundle that never drops doesn't carry it.
	let owned = |name: &str, args: Vec<Expr>, droppers: Vec<Expr>| match droppers.iter().all(undefined) {
		true => method(name, args),
		false => Some(Expr::rt_of("owned", name).call([vec![recv.clone()], args, tail(Vec::new(), droppers)].concat())),
	};
	// Each consumer that takes the iterator by value drops it, by `fold` but for `nth`'s.
	if matches!(
		name,
		"count" | "last" | "fold" | "for_each" | "sum" | "product" | "max" | "min" | "max_by_key" | "min_by_key"
	) {
		folds(b, self_ty, name, span);
	}
	let camel = super::camel(name);
	match name {
		"map" | "filter_map" | "enumerate" | "rev" | "map_while" | "any" | "all" | "find_map" | "position" => {
			method(&camel, rest)
		}
		// What each drops: an item it skips or rejects, or one it peeked.
		"filter" | "skip_while" | "take_while" | "inspect" | "peekable" => {
			let dropper = b.t.dropper(item, span);
			method(&camel, tail(rest, vec![dropper]))
		}
		"skip" | "take" | "step_by" => {
			let dropper = b.t.dropper(item, span);
			owned(&camel, rest, vec![dropper])
		}
		"fuse" => {
			let fused = fused(b, self_ty);
			method(name, tail(rest, vec![fused]))
		}
		"count" => {
			let droppers = vec![iter_dropper(b, self_ty, span), b.t.dropper(item, span)];
			owned(name, rest, droppers)
		}
		"for_each" => {
			let dropper = iter_dropper(b, self_ty, span);
			method(&camel, tail(rest, vec![dropper]))
		}
		"sum" | "product" => {
			let int = Int::of(targ(1)?)?;
			let (op, unit) = match name {
				"sum" => ("add", 0),
				_ => ("mul", 1),
			};
			let dropper = iter_dropper(b, self_ty, span);
			method(name, tail(vec![int.func(op), int.lit(unit)], vec![dropper]))
		}
		// `fold::<B, F>`, whose accumulator drops if reading an item panics.
		"fold" => {
			let droppers = vec![iter_dropper(b, self_ty, span), b.t.dropper(targ(1)?, span)];
			method(name, tail(rest, droppers))
		}
		// `ExactSizeIterator::len` is each iterator's own.
		"len" => Some(recv.member("len").call(rest)),
		"last" | "nth" => {
			option_ok(b, item, span);
			let droppers = match name {
				"last" => vec![iter_dropper(b, self_ty, span), b.t.dropper(item, span)],
				_ => vec![b.t.dropper(item, span)],
			};
			owned(&camel, rest, droppers)
		}
		"find" | "peek" | "next_if" => {
			option_ok(b, item, span);
			let droppers = match name {
				"find" => vec![b.t.dropper(item, span)],
				_ => Vec::new(),
			};
			method(&camel, tail(rest, droppers))
		}
		"max" | "min" if primitive(item.peel_refs()) => {
			option_ok(b, item, span);
			// A `BTreeMap`'s `into_keys` reads the one end and drops the rest with it.
			if std_iter_path(tcx, self_ty).is_some_and(|p| p == ["collections", "btree", "map", "IntoKeys"])
				&& b.t.drops(self_ty)
			{
				return Some(b.err(
					span,
					format!("`{name}` of `{self_ty}`, which drops its values, which is not supported yet"),
				));
			}
			let dropper = iter_dropper(b, self_ty, span);
			method(name, tail(rest, vec![dropper]))
		}
		"max_by_key" | "min_by_key" if primitive(targ(1)?) => {
			option_ok(b, item, span);
			let droppers = vec![iter_dropper(b, self_ty, span), b.t.dropper(item, span)];
			method(&camel, tail(rest, droppers))
		}
		"chain" => {
			let other = into_iter(b, targ(1)?, rest.into_iter().next()?, span)?;
			method(name, vec![other])
		}
		"zip" => {
			let other_ty = targ(1)?;
			let other = into_iter(b, other_ty, rest.into_iter().next()?, span)?;
			let sig = tcx.fn_sig(call.def_id).instantiate(tcx, gargs).skip_norm_wip();
			let zip = b.t.subst(
				ty::List::empty(),
				tcx.instantiate_bound_regions_with_erased(sig).output(),
			);
			let ty::Adt(_, sides) = zip.kind() else {
				return None;
			};
			let (a, other_iter) = (sides.type_at(0), sides.type_at(1));
			let other_item = b.t.iter_item(other_iter)?;
			let trusted = zip_reads_by_index(b, a, other_iter, span);
			// std's reads one item past the end of the first, which it drops, where `next`
			// could see the difference.
			if trusted && (b.t.drops(item) || b.t.drops(other_item)) {
				return Some(b.err(
					span,
					format!(
						"a `zip` of `{a}` and `{other_iter}`, which std reads by index, of items that drop, which is not supported yet"
					),
				));
			}
			let droppers = vec![
				Expr::lit(if trusted { "true" } else { "false" }),
				b.t.dropper(item, span),
				b.t.dropper(other_item, span),
			];
			method(name, tail(vec![other], droppers))
		}
		// `flat_map::<U, F>`, of the source mapped by `F`, which drops as the source does.
		"flat_map" => {
			let inner = iter_of(b.t, targ(1)?)?;
			flattens(b, inner, span);
			let into = into_fn(b, targ(1)?, span)?.unwrap_or_else(Expr::undefined);
			let fused = fused(b, self_ty);
			method("flatMap", tail(rest, vec![into, fused]))
		}
		"flatten" => {
			let inner = iter_of(b.t, item)?;
			flattens(b, inner, span);
			let into = into_fn(b, item, span)?.unwrap_or_else(Expr::undefined);
			let fused = fused(b, self_ty);
			method("flatten", tail(Vec::new(), vec![into, fused]))
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
		"collect" => collect(b, targ(1)?, item, recv, self_ty, span),
		_ => None,
	}
}

/// `args`, then `tail` without the `undefined`s and `false`s it ends with, which a runtime
/// function's optional parameters default to.
pub(super) fn tail(mut args: Vec<Expr>, mut tail: Vec<Expr>) -> Vec<Expr> {
	while matches!(tail.last(), Some(Expr::Lit(l)) if l == "undefined" || l == "false") {
		tail.pop();
	}
	args.extend(tail);
	args
}

/// Whether `e` is `undefined`, as the dropper of what doesn't drop is.
fn undefined(e: &Expr) -> bool {
	matches!(e, Expr::Lit(l) if l == "undefined")
}

/// Whether an iterator of type `ty` that drops is a `FusedIterator`, which std's `fuse` and
/// `flatten` keep once it ends rather than drop: `true` or `false`.
fn fused<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> Expr {
	Expr::lit(match b.t.drops(ty) && implements(b.t, ty, "FusedIterator") {
		true => "true",
		false => "false",
	})
}

/// The path of the std type `ty` is, if it is one.
fn std_iter_path(tcx: ty::TyCtxt<'_>, ty: Ty<'_>) -> Option<Vec<String>> {
	let ty::Adt(adt, _) = ty.kind() else {
		return None;
	};
	Some(library::std_path(tcx, adt.did())).filter(|p| !p.is_empty())
}

/// Refuse a `flatten` or `flat_map` of `inner` iterators that drop and may hold what drops
/// once they end: std's `try_fold` drops each such one only as it reads the next, where
/// `next` drops it as it ends. What std's collections' and `Option`'s `into_iter` make
/// holds nothing then.
fn flattens<'tcx>(b: &mut Body<'_, '_, 'tcx>, inner: Ty<'tcx>, span: Span) {
	if !b.t.drops(inner) {
		return;
	}
	let path = std_iter_path(b.t.tcx, inner).unwrap_or_default();
	let path: Vec<&str> = path.iter().map(String::as_str).collect();
	let empties = matches!(
		path.as_slice(),
		["vec", "into_iter", "IntoIter"]
			| ["array", "iter", "IntoIter"]
			| ["option", "IntoIter"]
			| ["iter", "sources", "once", "Once"]
			| ["collections", "hash", "map", "IntoIter" | "IntoKeys" | "IntoValues"]
			| ["collections", "hash", "set", "IntoIter"]
			| ["collections", "btree", "map", "IntoIter" | "IntoKeys" | "IntoValues"]
			| ["collections", "btree", "set", "IntoIter"]
			| ["collections", "vec_deque", "into_iter", "IntoIter"]
	);
	if !empties {
		b.t.err(
			span,
			format!("a `flatten` of `{inner}`, which drops what it may hold once it ends, which is not supported yet"),
		);
	}
}

/// Refuse std's `name` folding `ty` through a `flatten` or `flat_map` whose source and
/// iterators both drop: a panic in `fold` drops the iterator it took before the source,
/// as a panic in `next` does not.
pub(super) fn folds<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, name: &str, span: Span) {
	let tcx = b.t.tcx;
	let flattens: Vec<Ty<'tcx>> = ty
		.walk()
		.filter_map(|arg| arg.as_type())
		.filter(|&t| {
			std_iter_path(tcx, t).is_some_and(|p| {
				p.len() == 4
					&& p[..3] == ["iter", "adapters", "flatten"]
					&& matches!(p[3].as_str(), "Flatten" | "FlatMap")
			})
		})
		.collect();
	for t in flattens {
		let ty::Adt(_, args) = t.kind() else {
			continue;
		};
		let outer = args.type_at(0);
		// `Flatten<I>` of `I::Item`s, or `FlatMap<I, U, F>` of `U`s by `F`.
		let (outer_drops, inner) = match args.types().count() {
			1 => (b.t.drops(outer), b.t.iter_item(outer).and_then(|i| iter_of(b.t, i))),
			_ => (
				b.t.drops(outer) || b.t.drops(args.type_at(2)),
				iter_of(b.t, args.type_at(1)),
			),
		};
		if outer_drops && inner.is_some_and(|i| b.t.drops(i)) {
			b.t.err(
				span,
				format!(
					"`{name}` of `{ty}`, which folds a `flatten` whose source and iterators both drop, which is not supported yet"
				),
			);
			return;
		}
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
	let Some(tr) = core_trait(tcx, name) else {
		return false;
	};
	let (infcx, env) = tcx.infer_ctxt().build_with_typing_env(t.typing_env_pub());
	infcx.type_implements_trait(tr, [ty], env).must_apply_modulo_regions()
}

/// `core`'s trait `name`.
fn core_trait(tcx: ty::TyCtxt<'_>, name: &str) -> Option<DefId> {
	let &core = tcx
		.crates(())
		.iter()
		.find(|&&c| tcx.crate_name(c) == rustc_span::sym::core)?;
	tcx.traits(core)
		.iter()
		.copied()
		.find(|&tr| tcx.item_name(tr).as_str() == name)
}

/// Whether `ty` compares in JS as Rust's `Ord` does: an integer, a `bool`, or a `char`.
fn primitive(ty: Ty<'_>) -> bool {
	matches!(ty.kind(), ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_))
}

/// `iter.collect::<ty>()` of `item`s, from an iterator of type `from`, which it drops.
fn collect<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	ty: Ty<'tcx>,
	item: Ty<'tcx>,
	iter: Expr,
	from: Ty<'tcx>,
	span: Span,
) -> Option<Expr> {
	let tcx = b.t.tcx;
	let ty::Adt(adt, args) = ty.kind() else {
		return None;
	};
	let std = Std::of(tcx, adt.did())?;
	// Into a `Vec` of a `TrustedLen` iterator, and into any other collection, by `for_each`.
	if !matches!(std, Std::Result | Std::Option) && (std != Std::Vec || implements(b.t, from, "TrustedLen")) {
		folds(b, from, "collect", span);
	}
	match std {
		Std::Vec => Some(match args.type_at(0).kind() {
			ty::Uint(ty::UintTy::U8) => {
				let dropper = iter_dropper(b, from, span);
				Expr::rt("iterToByteVec", tail(vec![iter], vec![dropper]))
			}
			_ => to_array(b, iter, from, args.type_at(0), span),
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
			// What it collects from is what `from` yields up to the first `Err` or `None`, which
			// std's collects in place of `from`'s source as it would `from`.
			let values = collect(b, args.type_at(0), value, Expr::ident("values"), from, span)?;
			let param = Param {
				name: "values".into(),
				ty: Some(Type::Rt("Iter", vec![value_ty])),
			};
			let (collect, mut droppers) = match std {
				Std::Result => ("iterCollectResult", vec![b.t.dropper(args.type_at(1), span)]),
				_ => ("iterCollectOption", Vec::new()),
			};
			droppers.insert(0, b.t.dropper(args.type_at(0), span));
			let args = vec![iter, Expr::Lambda(vec![param], Box::new(values))];
			Some(Expr::rt(collect, tail(args, droppers)))
		}
		_ => super::collections::collect(b, ty, iter, from, span),
	}
}

/// What drops an iterator of type `ty` that a runtime function takes by value: `rt.drop`,
/// since each is an `rt.Iter`, or `undefined` for one that drops nothing.
pub(super) fn iter_dropper<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span) -> Expr {
	if !b.t.drops(ty) {
		return Expr::undefined();
	}
	// The crate's own drops through `rt.iterFrom`; any other refuses one it can't drop.
	if own_next(b, ty).is_none() {
		b.t.dropper(ty, span);
	}
	Expr::Rt("drop".into())
}

/// `rt.iterToArray` of `iter`, an iterator of type `from`, collected into a `Vec<elem>` as
/// std's does: in place of the `Vec` its items come from, where it can.
pub(super) fn to_array<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	iter: Expr,
	from: Ty<'tcx>,
	elem: Ty<'tcx>,
	span: Span,
) -> Expr {
	let drop_it = iter_dropper(b, from, span);
	let drop_elem = b.t.dropper(elem, span);
	let drops = !matches!(&drop_it, Expr::Lit(l) if l == "undefined")
		|| !matches!(&drop_elem, Expr::Lit(l) if l == "undefined");
	let in_place = drops && in_place(b.t, from, elem);
	if in_place && let Some(zip) = zip_holding(b, from) {
		return b.err(
			span,
			format!("a `collect` in place of `{zip}`, whose second iterator drops, which is not supported yet"),
		);
	}
	let droppers = vec![Expr::lit(if in_place { "true" } else { "false" }), drop_it, drop_elem];
	Expr::rt("iterToArray", tail(vec![iter], droppers))
}

/// A `Zip` in `ty` whose second iterator drops, which std's `collect` in place drops after
/// what it collected, where `rt.iterToArray` drops the whole iterator before.
fn zip_holding<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
	let tcx = b.t.tcx;
	let zips: Vec<Ty<'tcx>> = ty
		.walk()
		.filter_map(|arg| arg.as_type())
		.filter(|&t| std_iter_path(tcx, t).is_some_and(|p| p == ["iter", "adapters", "zip", "Zip"]))
		.collect();
	zips.into_iter().find(|t| match t.kind() {
		ty::Adt(_, args) => b.t.drops(args.type_at(1)),
		_ => false,
	})
}

/// Whether std's `collect` of an iterator of type `iter` into a `Vec<elem>` writes in place
/// of the `Vec` its items come from, as its adaptors let it where each item's layout leaves
/// room: then a panic drops what it collected before the rest of the source.
fn in_place<'tcx>(t: &Translator<'_, 'tcx>, iter: Ty<'tcx>, elem: Ty<'tcx>) -> bool {
	let tcx = t.tcx;
	let env = t.typing_env_pub();
	let (Some(steps), Some(source)) = (core_trait(tcx, "InPlaceIterable"), core_trait(tcx, "SourceIter")) else {
		return false;
	};
	if !implements(t, iter, "InPlaceIterable") || !implements(t, iter, "SourceIter") {
		return false;
	}
	let assoc = |tr: DefId, name: &str| {
		tcx.associated_items(tr)
			.filter_by_name_unhygienic(rustc_span::Symbol::intern(name))
			.next()
			.map(|item| item.def_id)
	};
	// `<I as SourceIter>::Source`, a `vec::IntoIter` of what it collects in place of.
	let Some(src) = assoc(source, "Source").and_then(|item| {
		let projection = Ty::new_projection(tcx, ty::IsRigid::No, item, [iter]);
		tcx.try_normalize_erasing_regions(env, ty::Unnormalized::new_wip(projection))
			.ok()
	}) else {
		return false;
	};
	let ty::Adt(adt, src_args) = src.kind() else {
		return false;
	};
	if library::std_path(tcx, adt.did()) != ["vec", "into_iter", "IntoIter"] {
		return false;
	}
	// `MERGE_BY` and `EXPAND_BY`: how many source items make how many it collects, if known.
	let step = |name: &str| -> Option<u128> {
		let item = assoc(steps, name)?;
		let instance = Instance::try_resolve(tcx, env, item, tcx.mk_args(&[iter.into()])).ok()??;
		let value = tcx.const_eval_instance(env, instance, rustc_span::DUMMY_SP).ok()?;
		Some(value.try_to_scalar_int()?.to_bits_unchecked())
	};
	let (Some(merge), Some(expand)) = (step("MERGE_BY"), step("EXPAND_BY")) else {
		return false;
	};
	let layout = |ty: Ty<'tcx>| tcx.layout_of(env.as_query_input(ty)).ok();
	let (Some(from), Some(to)) = (layout(src_args.type_at(0)), layout(elem)) else {
		return false;
	};
	// As std's `in_place_collectible`, where `None`, an overflow, is 0.
	!from.is_zst()
		&& !to.is_zst()
		&& from.align.abi == to.align.abi
		&& merge > 0
		&& expand > 0
		&& u128::from(from.size.bytes()) * merge >= u128::from(to.size.bytes()) * expand
}
