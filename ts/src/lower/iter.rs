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
use crate::ts::{Expr, Param, StmtKind, Type};

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
	if !matches!(ty.kind(), ty::Adt(adt, _) if b.t.own(adt.did())) || ty.has_param() || ty.has_aliases() {
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
	// Only the type's crate can implement std's traits for it.
	for &other in tcx.trait_impls_in_crate(adt.did().krate) {
		let self_ty = tcx.type_of(other).instantiate_identity().skip_norm_wip();
		if !matches!(self_ty.kind(), ty::Adt(other, _) if other.did() == adt.did()) {
			continue;
		}
		let path = library::std_path(tcx, tcx.impl_trait_id(other));
		if let Some(name) = path.last().filter(|n| {
			matches!(
				n.as_str(),
				"DoubleEndedIterator" | "ExactSizeIterator" | "FusedIterator"
			)
		}) {
			t.err(
				tcx.def_span(other),
				format!("an iterator of the crate implementing `{name}`, which is not supported yet"),
			);
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
	let (ty, value) = peel_mut(ty, value);
	let owned = !ty.is_ref();
	if b.t.iter_item(ty).is_some() {
		not_bound(b, ty, span);
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
	// Elements a `&mut` lends as their places, which JS copies.
	let places = |b: &Body<'_, '_, 'tcx>, elem: Ty<'tcx>| {
		ty.ref_mutability() == Some(ty::Mutability::Mut) && b.t.needs_place(elem)
	};
	match ty.peel_refs().kind() {
		ty::Slice(elem) | ty::Array(elem, _) if places(b, *elem) => {
			Some(Expr::rt("iterSliceMut", vec![library::elements(value, ty)]))
		}
		ty::Slice(elem) | ty::Array(elem, _) => {
			let values = each(b, *elem, vec![library::elements(value, ty)]);
			Some(items(Expr::rt("iterSlice", values), *elem))
		}
		ty::Adt(adt, args) => match Std::of(tcx, adt.did())? {
			Std::Vec if places(b, args.type_at(0)) => {
				Some(Expr::rt("iterSliceMut", vec![library::elements(value, ty)]))
			}
			Std::Vec => {
				let values = each(b, args.type_at(0), vec![library::elements(value, ty)]);
				Some(items(Expr::rt("iterSlice", values), args.type_at(0)))
			}
			// A `&mut` lends the payload JS copies as the `Option`'s own place, while it has one.
			Std::Option if places(b, args.type_at(0)) => {
				let mut values = vec![value];
				values.extend(b.t.none_arg(args.type_at(0)));
				Some(Expr::rt("iterOption", vec![Expr::rt("optionAsMut", values)]))
			}
			// A `&mut` to one of another payload is its place, and yields the payload itself.
			Std::Option => {
				let inner = args.type_at(0);
				let mut values = each(b, inner, vec![b.t.pointee_value(ty, value)]);
				if let Some(none) = b.t.none_arg(inner) {
					values.resize(2, Expr::undefined());
					values.push(none);
				}
				Some(Expr::rt("iterOption", values))
			}
			_ => super::collections::into_iter(b, ty, value, span),
		},
		_ => None,
	}
}

/// `iter`, over the elements of a slice or `Vec` of `elem`s, yielding references to them:
/// a slice of each array of bytes.
pub(super) fn by_ref(iter: Expr, elem: Ty<'_>) -> Expr {
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

/// Whether `ty` is a `&mut` to a `&mut`, which is an `rt.Place` of it, or a box of one,
/// which is what it holds.
pub fn through_mut(ty: Ty<'_>) -> bool {
	match ty.boxed_ty() {
		Some(inner) => through_mut(inner),
		None => matches!(*ty.kind(), ty::Ref(_, inner, ty::Mutability::Mut) if inner.is_ref()),
	}
}

/// `value`, a `ty`, read through each box and `&mut` to a `&mut` that std's `Iterator` for
/// `Box<I>` and `&mut I` forward through, down to the last `&mut` and its JS value.
pub fn peel_mut<'tcx>(mut ty: Ty<'tcx>, mut value: Expr) -> (Ty<'tcx>, Expr) {
	while through_mut(ty) {
		ty = match ty.boxed_ty() {
			Some(inner) => inner,
			None => {
				value = value.member("v");
				ty.builtin_deref(true).expect("a reference")
			}
		};
	}
	(ty, value)
}

/// Refuse iterating `ty`, an iterator, if a binding stands in for it: it is no `rt.Iter`,
/// which std's adaptors and `for` take every iterator but the crate's own for.
fn not_bound<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span) {
	if let ty::Adt(adt, _) = ty.peel_refs().kind()
		&& b.t.bound_drops(adt.did()).is_some()
	{
		b.t.err(span, "iterating a bound type, which is not supported yet");
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
		("slice", "iter_mut") if b.t.needs_place(targ(0)?) => {
			let s = library::elements(args.pop()?, Ty::new_slice(tcx, targ(0)?));
			return Some(Expr::rt("iterSliceMut", vec![s]));
		}
		("slice", "iter" | "iter_mut") => {
			let s = args.pop()?;
			let s = library::elements(s, Ty::new_slice(tcx, targ(0)?));
			return Some(by_ref(Expr::rt("iterSlice", vec![s]), targ(0)?));
		}
		("Option", "iter" | "iter_mut") => {
			let mut args = args;
			if let Some(none) = b.t.none_arg(targ(0)?) {
				args.extend([Expr::undefined(), none]);
			}
			return Some(Expr::rt("iterOption", args));
		}
		// A `&mut` to a `&mut` iterator is itself, an `rt.Place` that each use reads through.
		("IntoIterator", "into_iter") if through_mut(targ(0)?) => return args.pop(),
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
		// A closure that owns a value that drops is refused as any std mapping refuses one
		// it calls but never drops.
		// Each takes its closure's `None` last, where that is `rt.None`.
		["iter", "sources", "from_fn", "from_fn"] => {
			let none = b.t.none_arg(targ(0)?);
			return Some(Expr::rt("iterFromFn", [args, none.into_iter().collect()].concat()));
		}
		// What drops the item it holds next.
		["iter", "sources", "successors", "successors"] => {
			let droppers = [b.t.dropper(targ(0)?, span)].into_iter().chain(b.t.none_arg(targ(0)?));
			return Some(Expr::rt("iterSuccessors", tail(args, droppers.collect())));
		}
		_ => {}
	}

	// What `collect` calls.
	if (owner, name) == ("FromIterator", "from_iter") {
		let source = targ(2)?;
		let iter = iter_of(b.t, source)?;
		let items = into_iter(b, source, args.pop()?, span)?;
		let trusted = implements(b.t, iter, "TrustedLen");
		return collect(b, targ(0)?, targ(1)?, items, iter, trusted, span);
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
		let mut items = into_iter(b, source, items, span)?;
		// `Extend<&T>` copies each element.
		if item != vec_args.type_at(0)
			&& let Some(copy) = b.t.copy_fn(vec_args.type_at(0), span)
		{
			items = Expr::rt("iterMap", vec![items, copy]);
		}
		// Of a `TrustedLen` iterator, by `for_each`, and reserving its upper bound first.
		let trusted = implements(b.t, iter, "TrustedLen");
		let take = iter_take(b, iter, trusted, span);
		let (extend, reserve) = match vec_args.type_at(0).kind() {
			ty::Uint(ty::UintTy::U8) => ("iterExtendBytes", vec![take, Expr::lit(trusted.to_string())]),
			_ => {
				let size = size_of(b, vec_args.type_at(0), span);
				("iterExtend", vec![take, size, Expr::lit(trusted.to_string())])
			}
		};
		return Some(Expr::rt(extend, tail(vec![v, items], reserve)));
	}

	let self_ty = match (owner, name) {
		("Iterator" | "DoubleEndedIterator" | "ExactSizeIterator", _) => targ(0)?,
		("Peekable", "peek" | "peek_mut" | "next_if" | "next_if_eq")
			if library::inherent_std(tcx, call.def_id) == Some(Std::Peekable) =>
		{
			let imp = tcx.inherent_impl_of_assoc(call.def_id)?;
			let generics = tcx.generics_of(imp).count();
			tcx.type_of(imp).instantiate(tcx, &gargs[..generics]).skip_norm_wip()
		}
		_ => return None,
	};
	let item = b.t.iter_item(self_ty.peel_refs())?;
	not_bound(b, self_ty, span);
	let mut values = args.into_iter();
	let (self_ty, recv) = peel_mut(self_ty, values.next()?);
	let rest: Vec<Expr> = values.collect();
	// What a consumer returns for `None`, after its other arguments, where that is `rt.None`.
	let none = b.t.none_arg(item);

	// `next` by the protocol, as an `Option`.
	if matches!(name, "next" | "next_back" | "nth_back") {
		let item = match name {
			"next" => next(b, self_ty, recv),
			"next_back" => recv.member("nextBack").call(Vec::new()),
			_ => {
				let droppers = [b.t.dropper(item, span)].into_iter().chain(none).collect();
				return Some(Expr::rt("iterNthBack", tail([vec![recv], rest].concat(), droppers)));
			}
		};
		return Some(Expr::rt("iterToOption", [item].into_iter().chain(none).collect()));
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
	let camel = super::camel(name);
	match name {
		"map" | "enumerate" | "rev" | "inspect" | "any" | "all" | "position" => method(&camel, rest),
		// Each takes its closure's `None` last, where that is `rt.None`.
		"filter_map" | "map_while" | "find_map" => {
			let none = b.t.none_arg(targ(1)?);
			method(&camel, [rest, none.into_iter().collect()].concat())
		}
		// What each drops: an item it skips or rejects, or one it peeked.
		"filter" | "skip_while" | "take_while" | "peekable" | "skip" | "take" | "step_by" => {
			let dropper = b.t.dropper(item, span);
			method(&camel, tail(rest, vec![dropper]))
		}
		"fuse" => {
			let fused = fused(b, self_ty, None);
			method(name, tail(rest, vec![fused]))
		}
		// `rt.owned<Name>`, which drops what `self_ty` holds and its items by `dropper` as
		// std's own does, where either drops: else `rt.iter<Name>`, so a bundle that never
		// drops doesn't carry it.
		"count" | "last" => {
			let dropper = b.t.dropper(item, span);
			let none: Vec<Expr> = none.into_iter().filter(|_| name == "last").collect();
			match drops_iter(b, self_ty, span) || !undefined(&dropper) {
				true => Some(Expr::rt_of("owned", name).call(tail(vec![recv.clone()], [vec![dropper], none].concat()))),
				false => method(name, none),
			}
		}
		"for_each" | "fold" => {
			let drops = iter_drops(b, self_ty, span);
			method(&camel, tail(rest, vec![drops]))
		}
		// std's `Sum` and `Product` are of the type or a reference to it: another is the crate's.
		"sum" | "product" if item.peel_refs() != targ(1)? => None,
		// `Duration`'s carries its nanoseconds and panics as std's does, not as `+` would.
		"sum" if b.t.std_of(targ(1)?) == Some(Std::Duration) => {
			let drops = iter_drops(b, self_ty, span);
			Some(Expr::rt("durationSum", tail(vec![recv.clone()], vec![drops])))
		}
		// std's float sum starts from -0.0, so a sum of none, or of -0.0s, is -0.0.
		"sum" | "product" if let Some(float) = super::float::Float::of(targ(1)?) => {
			let (op, unit) = match name {
				"sum" => ("add", -0.0),
				_ => ("mul", 1.0),
			};
			let drops = iter_drops(b, self_ty, span);
			method(
				name,
				tail(vec![Expr::rt_of(float.name(), op), float.lit(unit)], vec![drops]),
			)
		}
		"sum" | "product" => {
			let int = Int::of(targ(1)?)?;
			let (op, unit) = match name {
				"sum" => ("add", 0),
				_ => ("mul", 1),
			};
			let drops = iter_drops(b, self_ty, span);
			method(name, tail(vec![int.func(op), int.lit(unit)], vec![drops]))
		}
		// `ExactSizeIterator::len` and `size_hint` are each iterator's own.
		"len" => Some(recv.member("len").call(rest)),
		"size_hint" => Some(recv.member("sizeHint").call(rest)),
		"nth" | "find" | "rfind" => {
			let dropper = b.t.dropper(item, span);
			method(&camel, tail(rest, [dropper].into_iter().chain(none).collect()))
		}
		"peek" | "next_if" => method(&camel, [rest, none.into_iter().collect()].concat()),
		// An item JS copies is lent as its place, and an object as itself.
		"peek_mut" => method(&camel, vec![Expr::lit(b.t.needs_place(item).to_string())]),
		// By the item's `PartialEq<T>`: `eq_fn` of its own type, else the impl's `eq`, as a
		// call to it lowers.
		"next_if_eq" => {
			let expected = targ(1)?;
			let eq = match expected == item {
				true => super::collections::eq_fn(b, item, span),
				false => {
					let eq = super::collections::trait_fn(tcx, "PartialEq", "eq");
					let eq = b.fn_value(eq, tcx.mk_args(&[item.into(), expected.into()]), span);
					match super::byte_array(item) {
						// The `eq` takes a `&[u8; N]`, an `rt.ByteSlice` of the item's bytes.
						true => {
							let (a, c) = (b.fresh("a"), b.fresh("b"));
							let lent = Expr::rt("byteSliceFrom", vec![Expr::ident(&a)]);
							let call = eq.call(vec![lent, Expr::ident(&c)]);
							let params = [a, c].map(|name| Param { name, ty: None }).into();
							Expr::Arrow(
								params,
								Type::Builtin("boolean"),
								vec![StmtKind::Return(Some(call)).at(None)],
							)
						}
						false => eq,
					}
				}
			};
			method(&camel, [rest, vec![eq], none.into_iter().collect()].concat())
		}
		// By the comparator, which `best` calls with the best so far and then the next, as
		// std's `cmp::max_by` and `min_by` do.
		"max_by" | "min_by" => {
			let droppers = [iter_drops(b, self_ty, span), b.t.dropper(item, span)]
				.into_iter()
				.chain(none);
			method(&camel, tail(rest, droppers.collect()))
		}
		"reduce" => {
			let drops = iter_drops(b, self_ty, span);
			method(name, tail(rest, [drops].into_iter().chain(none).collect()))
		}
		// By the items' `Ord::max` or `min`, or the keys', which test `<` of the next and the
		// best. Not of places, which the runtime would compare as the objects they are.
		"max" | "min" | "max_by_key" | "min_by_key"
			if !b.t.has_place(if name.ends_with("_by_key") { targ(1)? } else { item }) =>
		{
			let by_key = name.ends_with("_by_key");
			let key = if by_key { targ(1)? } else { item };
			if by_key && b.t.drops(key) {
				return Some(b.err(
					span,
					format!("`{name}` by `{key}`, which drops, which is not supported yet"),
				));
			}
			// A `BTreeMap`'s `into_keys` reads the one end and drops the rest with it.
			if !by_key
				&& std_iter_path(tcx, self_ty).is_some_and(|p| p == ["collections", "btree", "map", "IntoKeys"])
				&& b.t.drops(self_ty)
			{
				return Some(b.err(
					span,
					format!("`{name}` of `{self_ty}`, which drops its values, which is not supported yet"),
				));
			}
			// A B-tree's iterators read an end, comparing nothing.
			if !by_key && btree_end(tcx, self_ty, name) {
				let end = match name {
					"max" => recv.member("nextBack").call(Vec::new()),
					_ => recv.member("next").call(Vec::new()),
				};
				return Some(Expr::rt("iterToOption", [end].into_iter().chain(none).collect()));
			}
			if !by_key && own_ord(b.t, key.peel_refs(), name) {
				return Some(b.err(
					span,
					format!("`{name}` of `{key}` through its own `Ord::{name}`, which is not supported yet"),
				));
			}
			let lt = super::collections::test_fn(b, key, "lt", span);
			let droppers = [iter_drops(b, self_ty, span), b.t.dropper(item, span)]
				.into_iter()
				.chain(none);
			method(&camel, tail([rest, vec![lt]].concat(), droppers.collect()))
		}
		// Whether each item and the next are in order: by `PartialOrd::le`, by a comparator,
		// or by `le` of the keys a function makes, as std's maps to them. Not of places, as
		// `max` is not.
		"is_sorted" | "is_sorted_by" | "is_sorted_by_key" => {
			let (items, compare, each) = match name {
				"is_sorted_by" => (recv.clone(), rest.into_iter().next()?, item),
				_ if b.t.has_place(item) => return None,
				"is_sorted" => (recv.clone(), super::collections::test_fn(b, item, "le", span), item),
				_ => {
					let key = targ(2)?;
					let le = super::collections::test_fn(b, key, "le", span);
					// The map drops its function as the iterator drops, so one through a reference
					// or box is lent, as one owned that drops is refused with such an iterator.
					let (mut f, f_ty) = (rest.into_iter().next()?, targ(1)?);
					let owned = matches!(f_ty.kind(), ty::Closure(..) | ty::FnDef(..) | ty::FnPtr(..));
					if !owned && drops_iter(b, self_ty, span) {
						f = Expr::rt("lent", vec![f]);
					}
					(Expr::rt("iterMap", vec![recv.clone(), f]), le, key)
				}
			};
			let mut droppers = vec![iter_drops(b, self_ty, span), b.t.dropper(each, span)];
			if name == "is_sorted_by" {
				droppers.push(b.t.dropper(targ(1)?, span));
			}
			Some(Expr::rt("iterIsSortedBy", tail(vec![items, compare], droppers)))
		}
		"rposition" => method(&camel, rest),
		"chain" => {
			let other = into_iter(b, targ(1)?, rest.into_iter().next()?, span)?;
			method(name, vec![other])
		}
		"zip" => {
			let other_ty = targ(1)?;
			let other = into_iter(b, other_ty, rest.into_iter().next()?, span)?;
			let zip = b.t.sig(call.def_id, gargs).output();
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
			let fused = fused(b, self_ty, Some(targ(2)?));
			let size = array_size(b, targ(1)?);
			let one_shot = Expr::lit(implements(b.t, inner, "OneShot").to_string());
			method("flatMap", tail(rest, vec![into, fused, size, one_shot]))
		}
		"flatten" => {
			let inner = iter_of(b.t, item)?;
			flattens(b, inner, span);
			let into = into_fn(b, item, span)?.unwrap_or_else(Expr::undefined);
			let fused = fused(b, self_ty, None);
			let size = array_size(b, item);
			let one_shot = Expr::lit(implements(b.t, inner, "OneShot").to_string());
			method("flatten", tail(Vec::new(), vec![into, fused, size, one_shot]))
		}
		"copied" | "cloned" => {
			let ty::Ref(_, inner, _) = item.kind() else {
				return None;
			};
			let clone = |b: &mut Body<'_, '_, 'tcx>| {
				let clone = tcx.lang_items().clone_fn()?;
				Some(b.fn_value(clone, tcx.mk_args(&[(*inner).into()]), span))
			};
			// A `Clone` written by hand may do other than copy, even for a `Copy` type. A JS
			// string is a value already.
			let copies = name == "copied" || library::clone_copies(b.t, *inner) || library::string(tcx, *inner);
			let copy = match () {
				// Each item through its `Clone`.
				_ if !copies => Some(clone(b)?),
				// Out of the slice each item is.
				_ if super::byte_array(*inner) => Some(Expr::Rt("byteSliceToArray".into())),
				_ => b.t.copy_fn(*inner, span),
			};
			// std's `copied` skips through its iterator's `nth`, and `cloned` by its own `next`.
			match (name, copy) {
				("copied", Some(copy)) => method("copied", vec![copy]),
				("copied", None) => Some(recv),
				(_, Some(copy)) => method("map", vec![copy]),
				(_, None) => method("cloned", Vec::new()),
			}
		}
		"collect" => {
			let trusted = implements(b.t, self_ty, "TrustedLen");
			collect(b, targ(1)?, item, recv, self_ty, trusted, span)
		}
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

/// The size and alignment of a `ty` in bytes on wasm32, by which std's collections of it
/// reserve, or else what stands in for a figure of them: `undefined` over parameters, which
/// only a census's instance, which never runs, has.
pub(super) fn layout<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span) -> Result<(u64, u64), Expr> {
	match b.t.tcx.layout_of(b.t.typing_env_pub().as_query_input(ty)) {
		Ok(layout) => Ok((layout.size.bytes(), layout.align.abi.bytes())),
		Err(_) if ty.has_param() => Err(Expr::undefined()),
		Err(_) => Err(b.err(span, format!("reserving for `{ty}`, whose layout is unknown"))),
	}
}

/// The size of a `ty` in bytes, as `layout` gives it, by which std's `Vec` of it reserves.
pub(super) fn size_of<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span) -> Expr {
	layout(b, ty, span).map_or_else(|e| e, |(size, _)| Expr::lit(size.to_string()))
}

/// Whether an iterator of type `ty`, with `held` beside it, drops and is a `FusedIterator`,
/// which std's `fuse` and `flatten` keep once it ends rather than drop: `true` or `false`.
fn fused<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, held: Option<Ty<'tcx>>) -> Expr {
	let drops = b.t.drops(ty) || held.is_some_and(|held| b.t.drops(held));
	Expr::lit(match drops && implements(b.t, ty, "FusedIterator") {
		true => "true",
		false => "false",
	})
}

/// How many items `ty` has, if it is an array or a reference to one, which std's `flatten`
/// counts toward its size hint: else `undefined`.
fn array_size<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> Expr {
	let ty = match *ty.kind() {
		ty::Ref(_, inner, _) => inner,
		_ => ty,
	};
	match ty.kind() {
		ty::Array(_, len) => len
			.try_to_target_usize(b.t.tcx)
			.map_or_else(Expr::undefined, |n| Expr::lit(n.to_string())),
		_ => Expr::undefined(),
	}
}

/// The path of the std type `ty` is, if it is one.
fn std_iter_path(tcx: ty::TyCtxt<'_>, ty: Ty<'_>) -> Option<Vec<String>> {
	let ty::Adt(adt, _) = ty.kind() else {
		return None;
	};
	Some(library::std_path(tcx, adt.did())).filter(|p| !p.is_empty())
}

/// Whether std's iterator `did` holds a closure that its `rt.Iter` drops after the iterator
/// it wraps, as std's fields order them.
pub(super) fn holds_closure(tcx: ty::TyCtxt<'_>, did: DefId) -> bool {
	let path = library::std_path(tcx, did);
	let path: Vec<&str> = path.iter().map(String::as_str).collect();
	matches!(
		path.as_slice(),
		[
			"iter",
			"adapters",
			_,
			"Map" | "Filter" | "FilterMap" | "SkipWhile" | "TakeWhile" | "MapWhile" | "Inspect"
		] | ["iter", "adapters", "flatten", "FlatMap"]
			| ["iter", "sources", "from_fn", "FromFn"]
			| ["iter", "sources", "successors", "Successors"]
	)
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

/// `iter.collect::<ty>()` of `item`s, from an iterator of type `from`, which it drops, and
/// which std's reserves for as it does a `TrustedLen` one if `trusted`.
fn collect<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	ty: Ty<'tcx>,
	item: Ty<'tcx>,
	iter: Expr,
	from: Ty<'tcx>,
	trusted: bool,
	span: Span,
) -> Option<Expr> {
	let tcx = b.t.tcx;
	let ty::Adt(adt, args) = ty.kind() else {
		return None;
	};
	let std = Std::of(tcx, adt.did())?;
	match std {
		Std::Vec => Some(match args.type_at(0).kind() {
			ty::Uint(ty::UintTy::U8) => {
				let take = iter_take(b, from, implements(b.t, from, "TrustedLen"), span);
				Expr::rt(
					"iterToByteVec",
					tail(vec![iter], vec![take, Expr::lit(trusted.to_string())]),
				)
			}
			_ => to_array(b, iter, from, args.type_at(0), trusted, span),
		}),
		// The values, collected as the `Ok` or `Some` type is, until the first `Err` or `None`.
		std @ (Std::Result | Std::Option) => {
			let ty::Adt(_, item_args) = item.kind() else {
				return None;
			};
			let value = item_args.type_at(0);
			// The runtime ends at the first `undefined` item, and returns `undefined` for `None`.
			if std == Std::Option
				&& let Some(some) = [value, args.type_at(0)]
					.into_iter()
					.find(|&t| b.t.erases_to(t).undefined)
			{
				let msg = format!("collecting `Option<{some}>`, whose `None` is `rt.None`, which is not supported yet");
				b.t.err(span, msg);
			}
			let value_ty = b.t.ty(value, span);
			// What it collects from is what `from` yields up to the first `Err` or `None`, which
			// std's collects in place of `from`'s source as it would `from`, but never trusts
			// the length of.
			let values = collect(b, args.type_at(0), value, Expr::ident("values"), from, false, span)?;
			let param = Param {
				name: "values".into(),
				ty: Some(Type::Rt("Iter", vec![value_ty])),
			};
			let collect = match std {
				Std::Result => "iterCollectResult",
				_ => "iterCollectOption",
			};
			let dropper = b.t.dropper(args.type_at(0), span);
			let args = vec![iter, Expr::Lambda(vec![param], Box::new(values))];
			Some(Expr::rt(collect, tail(args, vec![dropper])))
		}
		// From chars or text, as `extend` of an empty one, but for an owned string's, which
		// the rest extend once `next` gives it. A `Cow<str>` and a `Box<str>` own the `String`
		// std's collects.
		Std::String | Std::Cow | Std::Box if std == Std::String || args.types().next().is_some_and(|t| t.is_str()) => {
			match b.t.std_of(item) {
				Some(Std::String | Std::Cow) => {
					let drops = iter_drops(b, from, span);
					Some(Expr::rt("stringFromStrings", tail(vec![iter], vec![drops])))
				}
				_ => string_extend(b, Expr::Str(String::new()), iter, item, from, span),
			}
		}
		_ => super::collections::collect(b, ty, iter, from, trusted, span),
	}
}

/// `s` extended by `items`, an iterator of type `iter` of `item`s, as `String`'s `Extend`
/// reads it, by `for_each`: of chars, through any `&`, or of text, a `&str`, `String`,
/// `Box<str>`, or `Cow<str>`.
pub(super) fn string_extend<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	s: Expr,
	items: Expr,
	item: Ty<'tcx>,
	iter: Ty<'tcx>,
	span: Span,
) -> Option<Expr> {
	let text = match *item.peel_refs().kind() {
		ty::Char => false,
		ty::Str => true,
		ty::Adt(_, args) => match b.t.std_of(item)? {
			Std::String => true,
			Std::Box | Std::Cow => args.types().next().is_some_and(|t| t.is_str()),
			_ => return None,
		},
		_ => return None,
	};
	let extend = if text { "stringExtend" } else { "stringExtendChars" };
	let drops = iter_drops(b, iter, span);
	Some(Expr::rt(extend, tail(vec![s, items], vec![drops])))
}

/// Whether an iterator of type `ty` that a runtime function takes by value drops what it
/// holds, which its `rt.Iter` drops as std's does: `true`, or `undefined` for one that
/// drops nothing.
pub(super) fn iter_drops<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span) -> Expr {
	match drops_iter(b, ty, span) {
		true => Expr::lit("true"),
		false => Expr::undefined(),
	}
}

/// How a runtime function takes an iterator of type `ty` by value, as `rt.Take` says: by its
/// own `fold` where std's `folds` it, else by `next`, or `undefined` for one that drops
/// nothing.
pub(super) fn iter_take<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, folds: bool, span: Span) -> Expr {
	match drops_iter(b, ty, span) {
		true => Expr::Str(if folds { "fold" } else { "next" }.into()),
		false => Expr::undefined(),
	}
}

/// Whether an iterator of type `ty` drops, refusing one it can't drop: the crate's own
/// drops through `rt.iterFrom`.
fn drops_iter<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span) -> bool {
	if !b.t.drops(ty) {
		return false;
	}
	if own_next(b, ty).is_none() {
		b.t.dropper(ty, span);
	}
	true
}

/// `rt.iterToArray` of `iter`, an iterator of type `from`, collected into a `Vec<elem>` as
/// std's does: by `next` where it collects in place of the `Vec` its items come from, by
/// `try_fold`, which drops the closures the iterator holds only with it, where its own
/// `fold` drops them first. It reserves as std's does for a `TrustedLen` iterator if
/// `trusted`.
pub(super) fn to_array<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	iter: Expr,
	from: Ty<'tcx>,
	elem: Ty<'tcx>,
	trusted: bool,
	span: Span,
) -> Expr {
	let folds = implements(b.t, from, "TrustedLen") && !in_place(b.t, from, elem);
	let take = iter_take(b, from, folds, span);
	let size = size_of(b, elem, span);
	Expr::rt(
		"iterToArray",
		tail(vec![iter], vec![take, size, Expr::lit(trusted.to_string())]),
	)
}

/// Whether std's `collect` of an iterator of type `iter` into a `Vec<elem>` writes in place
/// of the `Vec` its items come from, as its adaptors let it where each item's layout leaves
/// room.
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

/// Whether `ty`, the crate's own type, has its own `Ord::max` or `min`, by `name`, which
/// `Iterator::max` or `min` calls.
fn own_ord<'tcx>(t: &Translator<'_, 'tcx>, ty: Ty<'tcx>, name: &str) -> bool {
	let tcx = t.tcx;
	if !matches!(ty.kind(), ty::Adt(adt, _) if t.own(adt.did())) {
		return false;
	}
	let f = super::collections::trait_fn(tcx, "Ord", name);
	Instance::try_resolve(tcx, t.typing_env_pub(), f, tcx.mk_args(&[ty.into()]))
		.ok()
		.flatten()
		.is_some_and(|instance| instance.def_id() != f)
}

/// Whether std's iterator `ty` has its own `min` or `max`, by `name`, which reads its first
/// or last item, as a B-tree's do.
fn btree_end(tcx: ty::TyCtxt<'_>, ty: Ty<'_>, name: &str) -> bool {
	let Some(path) = std_iter_path(tcx, ty) else {
		return false;
	};
	let path: Vec<&str> = path.iter().map(String::as_str).collect();
	match path[..] {
		[
			"collections",
			"btree",
			"map",
			"Iter" | "IterMut" | "Keys" | "Range" | "RangeMut" | "IntoKeys",
		]
		| ["collections", "btree", "set", "Iter" | "Range"] => true,
		[
			"collections",
			"btree",
			"set",
			"Difference" | "SymmetricDifference" | "Intersection" | "Union",
		] => name == "min",
		_ => false,
	}
}
