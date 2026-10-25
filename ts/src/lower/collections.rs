//! Collections, as `rt/collections.ts` gives them: `HashMap`, `HashSet`, `BTreeMap`,
//! `BTreeSet`, and `VecDeque`, with `Ordering` and `DefaultHasher`, which their keys need.
//!
//! A hash collection holds a key that is a JS primitive as its own JS map key, which JS
//! compares as Rust's `Eq` does for numbers, strings, and booleans, and finds any other
//! key through its translated `Hash` and `Eq`, which `rt.hashing` makes it carry. A
//! B-tree orders every key by its translated `Ord`. Each value a collection stores is its
//! own copy, as in Rust.

use rustc_middle::ty::{self, Instance, Ty};
use rustc_span::Span;

use super::body::Body;
use super::library::{self, Args, Call, Std};
use super::num::Int;
use crate::ts::{Expr, Stmt};

/// A call of a standard library function on these types, or its arguments back if it is
/// none of theirs.
pub fn call<'tcx, 'x>(
	b: &mut Body<'_, '_, 'tcx>,
	call: &Call<'tcx>,
	owner: &str,
	name: &str,
	args: Args<'x>,
	out: &mut Vec<Stmt>,
) -> Result<Option<Expr>, Args<'x>> {
	let tcx = b.t.tcx;
	let span = call.span;
	let self_ty = call.args.types().next();
	let self_std = self_ty.and_then(|t| b.t.std_of(t.peel_refs()));
	// An inherent method's type, or a trait method's `Self`.
	let (std, coll) = match library::inherent_std(tcx, call.def_id) {
		Some(std) => (std, impl_self_ty(b, call)),
		None => match (owner, name, self_std, self_ty) {
			("Default", "default", Some(std), Some(t))
			| ("Clone", "clone", Some(std), Some(t))
			| ("Index" | "IndexMut", "index" | "index_mut", Some(std), Some(t))
			| ("Extend", "extend", Some(std), Some(t))
			| ("From", "from", Some(std), Some(t)) => (std, t.peel_refs()),
			// Comparing and hashing a key of a std type.
			("Ord", "cmp", ..) | ("PartialOrd", "partial_cmp", ..) if let Some(t) = self_ty => {
				let t = peel(b, t);
				if !std_key(b, t) {
					return Err(args);
				}
				let f = cmp_fn(b, t, span);
				return Ok(Some(f.call(b.args(args, out))));
			}
			("Hash", "hash", ..) if let Some(t) = self_ty => {
				let t = peel(b, t);
				// Only into `DefaultHasher`, whose state the runtime's functions write.
				let hasher = call.args.types().nth(1).filter(|&h| b.t.std_of(h) == Some(Std::Hasher));
				let (true, Some(hasher)) = (std_key(b, t), hasher) else {
					return Err(args);
				};
				let f = hash_fn(b, t, hasher, span);
				return Ok(Some(f.call(b.args(args, out))));
			}
			("PartialEq", "eq" | "ne", ..) if let Some(t) = self_ty => {
				let t = peel(b, t);
				if !matches!(t.kind(), ty::Tuple(tys) if !tys.is_empty()) && b.t.std_of(t) != Some(Std::Ordering) {
					return Err(args);
				}
				let eq = eq_fn(b, t, span).call(b.args(args, out));
				return Ok(Some(if name == "eq" { eq } else { eq.not() }));
			}
			("Hasher", _, Some(Std::Hasher), _) => return Ok(Some(hasher_call(b, name, args, out))),
			// The discriminant derived `Hash` and `Ord` read of an enum.
			("intrinsics", "discriminant_value", ..) if let Some(t) = self_ty => {
				let value = b.args(args, out).pop();
				return Ok(value.map(|value| discriminant(b, t, value, span)));
			}
			_ => return Err(args),
		},
	};
	if let Some(elem) = undefined_arg(b, std, coll) {
		return Ok(Some(b.err(
			span,
			format!("`{coll}` holds `{elem}`, which is `undefined` as `None` is, which is not supported"),
		)));
	}
	let e = match std {
		Std::HashMap | Std::HashSet => hash(b, coll, call, owner, name, args, out),
		Std::BTreeMap | Std::BTreeSet => btree(b, coll, call, owner, name, args, out),
		Std::VecDeque => deque(b, coll, call, owner, name, args, out),
		Std::Entry => entry(b, coll, name, args, span, out),
		Std::Ordering => ordering(b, name, args, span, out),
		Std::Hasher if matches!(name, "new" | "default") => Some(Expr::rt("hasher", vec![])),
		_ => return Err(args),
	};
	if e.is_some()
		&& let Some(lost) = discarded(std, coll, owner, name)
		&& b.t.drops(lost)
	{
		let what = match lost == coll {
			true => "what it holds".to_string(),
			false => format!("the `{lost}` it is given"),
		};
		return Ok(Some(b.err(
			span,
			format!("`{owner}::{name}` of `{coll}`, which may drop {what}, which is not supported yet"),
		)));
	}
	Ok(e)
}

/// What a method of `coll` may drop rather than return or keep, which the runtime would
/// leave to JS's collector: the key `insert` and `entry` are given if an equal one is
/// there already, the value `or_insert` is given if one is, or anything `coll` holds.
fn discarded<'tcx>(std: Std, coll: Ty<'tcx>, owner: &str, name: &str) -> Option<Ty<'tcx>> {
	let map = matches!(std, Std::HashMap | Std::HashSet | Std::BTreeMap | Std::BTreeSet);
	match (owner, name) {
		(_, "insert" | "entry") if map => Some(arg(coll, 0)),
		(_, "or_insert") if std == Std::Entry => Some(arg(coll, 1)),
		("Extend", _) | ("From", _) if map => Some(coll),
		(_, "clear" | "retain" | "retain_mut" | "truncate" | "drain" | "into_keys" | "into_values") => Some(coll),
		_ => None,
	}
}

/// A key, value, or item type of `coll` that can be `undefined` in JS, so that an `Option`
/// of it, as `get` returns, could not tell `None` from `Some`.
fn undefined_arg<'tcx>(b: &Body<'_, '_, 'tcx>, std: Std, coll: Ty<'tcx>) -> Option<Ty<'tcx>> {
	let n = match std {
		Std::HashMap | Std::BTreeMap | Std::Entry => 2,
		Std::HashSet | Std::BTreeSet | Std::VecDeque => 1,
		_ => 0,
	};
	(0..n).map(|i| arg(coll, i)).find(|&t| b.t.erases_to(t).undefined)
}

/// The type an inherent method's impl is of, such as `HashMap<K, V, RandomState>`.
fn impl_self_ty<'tcx>(b: &Body<'_, '_, 'tcx>, call: &Call<'tcx>) -> Ty<'tcx> {
	let tcx = b.t.tcx;
	let imp = tcx.inherent_impl_of_assoc(call.def_id).expect("an inherent method");
	let generics = tcx.generics_of(imp).count();
	let ty = tcx
		.type_of(imp)
		.instantiate(tcx, &call.args[..generics])
		.skip_norm_wip();
	tcx.instantiate_bound_regions_with_erased(ty::Binder::dummy(ty))
}

/// A collection's `n`th type argument: a key, value, or item type.
fn arg<'tcx>(coll: Ty<'tcx>, n: usize) -> Ty<'tcx> {
	let ty::Adt(_, args) = coll.kind() else {
		unreachable!("a collection is a struct")
	};
	args.types().nth(n).expect("a collection's type argument")
}

/// What a JS value of `ty` is: through references and boxes, which are their pointee.
fn peel<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> Ty<'tcx> {
	let ty = ty.peel_refs();
	match ty.kind() {
		ty::Adt(adt, args) if Std::of(b.t.tcx, adt.did()) == Some(Std::Box) => peel(b, args.type_at(0)),
		_ => ty,
	}
}

/// Evaluate `values` for their effects: a capacity, which nothing here keeps.
fn discard(values: Vec<Expr>, out: &mut Vec<Stmt>) {
	for value in values {
		if !value.is_path() {
			out.push(crate::ts::StmtKind::Expr(value).at(None));
		}
	}
}

/// A method of `HashMap` or `HashSet`, or of a trait they implement.
fn hash<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	coll: Ty<'tcx>,
	call: &Call<'tcx>,
	owner: &str,
	name: &str,
	args: Args<'_>,
	out: &mut Vec<Stmt>,
) -> Option<Expr> {
	let span = call.span;
	let set = b.t.std_of(coll) == Some(Std::HashSet);
	let key = arg(coll, 0);
	let value = if set { None } else { Some(arg(coll, 1)) };
	let mut values = b.args(args, out);
	let e = match (owner, name) {
		(_, "new" | "with_capacity") | ("Default", "default") => {
			discard(values, out);
			hash_new(b, coll, span)?
		}
		(_, "len") => Expr::rt("hashLen", values),
		(_, "is_empty") => Expr::rt("hashLen", values).binary("===", Expr::lit("0")),
		(_, "clear") => Expr::rt("hashClear", values),
		(_, "contains_key" | "contains") => Expr::rt("hashContains", values),
		(_, "get" | "get_mut") if !set => Expr::rt("hashGet", values),
		(_, "get") => Expr::rt("hashKey", values),
		(_, "take") => {
			values.push(Expr::lit("true"));
			Expr::rt("hashKey", values)
		}
		(_, "insert") if set => Expr::rt("hashAdd", values),
		(_, "insert") => Expr::rt("hashInsert", values),
		(_, "remove") if set => Expr::rt("hashDelete", values),
		(_, "remove") => Expr::rt("hashRemove", values),
		(_, "remove_entry") => Expr::rt("hashRemoveEntry", values),
		(_, "retain") => Expr::rt("hashRetain", values),
		(_, "iter" | "iter_mut") if !set => Expr::rt("hashIter", values),
		(_, "iter" | "keys" | "into_keys") => Expr::rt("hashKeys", values),
		(_, "values" | "values_mut" | "into_values") => Expr::rt("hashValues", values),
		(_, "drain") if set => Expr::rt("hashDrainKeys", values),
		(_, "drain") => Expr::rt("hashDrain", values),
		(_, "entry") => Expr::rt("entry", values),
		(_, "reserve" | "shrink_to_fit") => {
			let m = values.remove(0);
			discard(values, out);
			let _ = m;
			Expr::undefined()
		}
		("Index", "index") => Expr::rt("hashIndex", values),
		("Clone", "clone") => {
			let k = clone_fn(b, key, span);
			let v = match value {
				Some(v) => clone_fn(b, v, span),
				None => Expr::undefined(),
			};
			values.extend([k, v]);
			Expr::rt("hashClone", values)
		}
		("Extend", "extend") | ("From", "from") => {
			let source = extend_source(b, coll, call, owner, values.pop()?)?;
			let target = match owner {
				"From" => hash_new(b, coll, span)?,
				_ => values.pop()?,
			};
			Expr::rt(if set { "hashExtendKeys" } else { "hashExtend" }, vec![target, source])
		}
		_ => return None,
	};
	Some(e)
}

/// An empty hash collection of type `coll`, which finds its keys as their type needs.
fn hash_new<'tcx>(b: &mut Body<'_, '_, 'tcx>, coll: Ty<'tcx>, span: Span) -> Option<Expr> {
	let key = arg(coll, 0);
	let values = hashing(b, coll, key, span)?.into_iter().collect();
	Some(empty(b, Expr::rt("hashNew", values), coll, span))
}

/// `new`, as an empty collection of type `coll`: TS infers a generic key's type, as of
/// `rt.cmp`, from no argument, only from the type it is asserted to be.
fn empty<'tcx>(b: &mut Body<'_, '_, 'tcx>, new: Expr, coll: Ty<'tcx>, span: Span) -> Expr {
	Expr::As(Box::new(new), b.t.ty(coll, span))
}

/// The `rt.Hashing` a hash collection of type `coll` finds a `key` that is a JS object
/// by, or `Some(None)` if every key is a JS primitive that compares as Rust's `Eq` does.
fn hashing<'tcx>(b: &mut Body<'_, '_, 'tcx>, coll: Ty<'tcx>, key: Ty<'tcx>, span: Span) -> Option<Option<Expr>> {
	let key = peel(b, key);
	let primitive = match key.kind() {
		ty::Bool | ty::Str => true,
		ty::Int(_) | ty::Uint(_) => !Int::of(key)?.big(),
		ty::Adt(..) if b.t.std_of(key) == Some(Std::String) => true,
		ty::Adt(adt, _) if adt.did().is_local() && fieldless(*adt) => derived_eq(b, key),
		_ => false,
	};
	if primitive {
		return Some(None);
	}
	let Some(hasher) = hasher_ty(b, coll) else {
		b.t.err(span, format!("hashing by the hasher of `{coll}`"));
		return None;
	};
	let mut values = vec![hash_fn(b, key, hasher, span), eq_fn(b, key, span)];
	// A fieldless enum is a JS string, which its own `Eq` may tell apart where JS can't.
	if matches!(key.kind(), ty::Adt(adt, _) if fieldless(*adt)) {
		values.push(Expr::lit("true"));
	}
	Some(Some(Expr::rt("hashing", values)))
}

fn fieldless(adt: ty::AdtDef<'_>) -> bool {
	adt.is_enum() && adt.variants().iter().all(|v| v.fields.is_empty())
}

/// Whether `ty`'s `PartialEq` is derived, so it compares as JS compares its tags.
fn derived_eq<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> bool {
	let tcx = b.t.tcx;
	let eq = trait_fn(tcx, "PartialEq", "eq");
	Instance::try_resolve(tcx, b.t.typing_env_pub(), eq, tcx.mk_args(&[ty.into(), ty.into()]))
		.ok()
		.flatten()
		.and_then(|instance| tcx.impl_of_assoc(instance.def_id()))
		.is_some_and(|imp| tcx.is_automatically_derived(imp))
}

/// The `Hasher` a hash collection of type `coll` hashes with: its `BuildHasher`'s.
fn hasher_ty<'tcx>(b: &Body<'_, '_, 'tcx>, coll: Ty<'tcx>) -> Option<Ty<'tcx>> {
	let tcx = b.t.tcx;
	let ty::Adt(_, args) = coll.kind() else {
		return None;
	};
	// `HashMap<K, V, S, A>` and `HashSet<T, S, A>`.
	let state = args
		.types()
		.nth(if b.t.std_of(coll) == Some(Std::HashSet) { 1 } else { 2 })?;
	let hasher = trait_fn(tcx, "BuildHasher", "Hasher");
	let projection = Ty::new_projection(tcx, ty::IsRigid::No, hasher, [state]);
	let hasher = tcx
		.try_normalize_erasing_regions(b.t.typing_env_pub(), ty::Unnormalized::new_wip(projection))
		.ok()?;
	(b.t.std_of(hasher) == Some(Std::Hasher)).then_some(hasher)
}

/// A method of `BTreeMap` or `BTreeSet`, or of a trait they implement.
fn btree<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	coll: Ty<'tcx>,
	call: &Call<'tcx>,
	owner: &str,
	name: &str,
	args: Args<'_>,
	out: &mut Vec<Stmt>,
) -> Option<Expr> {
	let span = call.span;
	let set = b.t.std_of(coll) == Some(Std::BTreeSet);
	let key = arg(coll, 0);
	let value = if set { None } else { Some(arg(coll, 1)) };
	let mut values = b.args(args, out);
	let last = |mut values: Vec<Expr>| {
		values.push(Expr::lit("true"));
		values
	};
	let e = match (owner, name) {
		(_, "new") | ("Default", "default") => btree_new(b, coll, span),
		(_, "len") => Expr::rt("btreeLen", values),
		(_, "is_empty") => Expr::rt("btreeLen", values).binary("===", Expr::lit("0")),
		(_, "clear") => Expr::rt("btreeClear", values),
		(_, "contains_key" | "contains") => Expr::rt("btreeContains", values),
		(_, "get" | "get_mut") if !set => Expr::rt("btreeGet", values),
		(_, "get_key_value") => Expr::rt("btreeGetKeyValue", values),
		(_, "insert") if set => Expr::rt("btreeAdd", values),
		(_, "insert") => Expr::rt("btreeInsert", values),
		(_, "remove") if set => Expr::rt("btreeDelete", values),
		(_, "remove") => Expr::rt("btreeRemove", values),
		(_, "remove_entry") => Expr::rt("btreeRemoveEntry", values),
		(_, "retain") => Expr::rt("btreeRetain", values),
		(_, "first_key_value") => Expr::rt("btreeFirst", values),
		(_, "last_key_value") => Expr::rt("btreeFirst", last(values)),
		(_, "pop_first") if set => Expr::rt("btreePopKey", values),
		(_, "pop_last") if set => Expr::rt("btreePopKey", last(values)),
		(_, "pop_first") => Expr::rt("btreePop", values),
		(_, "pop_last") => Expr::rt("btreePop", last(values)),
		(_, "first") => Expr::rt("btreeFirstKey", values),
		(_, "last") => Expr::rt("btreeFirstKey", last(values)),
		(_, "first_entry") => Expr::rt("btreeFirstEntry", values),
		(_, "last_entry") => Expr::rt("btreeFirstEntry", last(values)),
		(_, "iter" | "iter_mut") if !set => Expr::rt("btreeIter", values),
		(_, "iter" | "keys" | "into_keys") => Expr::rt("btreeKeys", values),
		(_, "values" | "values_mut" | "into_values") => Expr::rt("btreeValues", values),
		(_, "range") if set => Expr::rt("btreeRangeKeys", values),
		(_, "range" | "range_mut") => Expr::rt("btreeRange", values),
		(_, "entry") => Expr::rt("entry", values),
		("Index", "index") => Expr::rt("btreeIndex", values),
		("Clone", "clone") => {
			let k = clone_fn(b, key, span);
			let v = match value {
				Some(v) => clone_fn(b, v, span),
				None => Expr::undefined(),
			};
			values.extend([k, v]);
			Expr::rt("btreeClone", values)
		}
		("Extend", "extend") | ("From", "from") => {
			let source = extend_source(b, coll, call, owner, values.pop()?)?;
			let target = match owner {
				"From" => btree_new(b, coll, span),
				_ => values.pop()?,
			};
			let mut values = vec![target, source];
			// `from` keeps the last of equal keys, as std's sorted bulk build does.
			if owner == "From" {
				values.push(Expr::lit("true"));
			}
			Expr::rt(if set { "btreeExtendKeys" } else { "btreeExtend" }, values)
		}
		_ => return None,
	};
	Some(e)
}

/// An empty B-tree of type `coll`, ordered by its keys' `Ord`.
fn btree_new<'tcx>(b: &mut Body<'_, '_, 'tcx>, coll: Ty<'tcx>, span: Span) -> Expr {
	let cmp = cmp_fn(b, arg(coll, 0), span);
	empty(b, Expr::rt("btreeNew", vec![cmp]), coll, span)
}

/// An empty `VecDeque` of type `coll`.
fn deque_new<'tcx>(b: &mut Body<'_, '_, 'tcx>, coll: Ty<'tcx>, span: Span) -> Expr {
	empty(b, Expr::rt("dequeNew", vec![]), coll, span)
}

/// A method of `VecDeque`, or of a trait it implements.
fn deque<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	coll: Ty<'tcx>,
	call: &Call<'tcx>,
	owner: &str,
	name: &str,
	args: Args<'_>,
	out: &mut Vec<Stmt>,
) -> Option<Expr> {
	let span = call.span;
	let item = arg(coll, 0);
	let mut values = b.args(args, out);
	let back = |mut values: Vec<Expr>| {
		values.push(Expr::lit("true"));
		values
	};
	let e = match (owner, name) {
		(_, "new" | "with_capacity") | ("Default", "default") => {
			discard(values, out);
			deque_new(b, coll, span)
		}
		(_, "len") => values.pop()?.member("n"),
		(_, "is_empty") => values.pop()?.member("n").binary("===", Expr::lit("0")),
		(_, "clear") => Expr::rt("dequeClear", values),
		(_, "push_back") => Expr::rt("dequePushBack", values),
		(_, "push_front") => Expr::rt("dequePushFront", values),
		(_, "pop_back") => Expr::rt("dequePopBack", values),
		(_, "pop_front") => Expr::rt("dequePopFront", values),
		(_, "front" | "front_mut") => Expr::rt("dequeEnd", values),
		(_, "back" | "back_mut") => Expr::rt("dequeEnd", back(values)),
		(_, "get" | "get_mut") => Expr::rt("dequeGet", values),
		(_, "truncate") => Expr::rt("dequeTruncate", values),
		(_, "retain" | "retain_mut") => Expr::rt("dequeRetain", values),
		(_, "remove") => Expr::rt("dequeRemove", values),
		(_, "insert") => Expr::rt("dequeInsert", values),
		(_, "contains") => {
			values.push(eq_fn(b, peel(b, item), span));
			Expr::rt("dequeContains", values)
		}
		(_, "iter" | "iter_mut") => Expr::rt("dequeIter", values),
		(_, "drain") => Expr::rt("dequeDrain", values),
		(_, "reserve" | "reserve_exact" | "shrink_to_fit") => {
			values.remove(0);
			discard(values, out);
			Expr::undefined()
		}
		("Index" | "IndexMut", "index" | "index_mut") => Expr::rt("dequeIndex", values),
		("Clone", "clone") => {
			values.push(clone_fn(b, item, span));
			Expr::rt("dequeClone", values)
		}
		("Extend", "extend") | ("From", "from") => {
			let source = extend_source(b, coll, call, owner, values.pop()?)?;
			let target = match owner {
				"From" => deque_new(b, coll, span),
				_ => values.pop()?,
			};
			Expr::rt("dequeExtend", vec![target, source])
		}
		_ => return None,
	};
	Some(e)
}

/// What `extend` or `from` adds to a collection: an `rt.Iter` of what `source` holds.
fn extend_source<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	coll: Ty<'tcx>,
	call: &Call<'tcx>,
	owner: &str,
	source: Expr,
) -> Option<Expr> {
	// `Extend<A>::extend::<T>` and `From<T>::from`.
	let ty = library::targ(call.args, if owner == "Extend" { 2 } else { 1 });
	let Some(items) = super::iter::into_iter(b, ty, source, call.span) else {
		return Some(b.err(call.span, format!("`{owner}` from `{ty}`, which is not supported yet")));
	};
	// `Extend<&T>` and `Extend<(&K, &V)>`, whose items are `Copy`, copy each one.
	let elems: Vec<Ty<'tcx>> = match b.t.std_of(coll) {
		Some(Std::HashMap | Std::BTreeMap) => vec![arg(coll, 0), arg(coll, 1)],
		_ => vec![arg(coll, 0)],
	};
	let stored = match elems.as_slice() {
		[k, v] => Ty::new_tup(b.t.tcx, &[*k, *v]),
		_ => elems[0],
	};
	let borrowed = owner == "Extend" && library::targ(call.args, 1) != stored;
	Some(match borrowed && elems.iter().any(|&t| b.needs_copy(t)) {
		true => items.member("map").call(vec![Expr::Rt("copy")]),
		false => items,
	})
}

/// A method of an `Entry`, `OccupiedEntry`, or `VacantEntry`.
fn entry<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	coll: Ty<'tcx>,
	name: &str,
	args: Args<'_>,
	span: Span,
	out: &mut Vec<Stmt>,
) -> Option<Expr> {
	let value = arg(coll, 1);
	let mut values = b.args(args, out);
	let e = match name {
		"or_insert" => Expr::rt("entryOrInsert", values),
		// A vacant entry's.
		"insert" if is_vacant(b, coll) => Expr::rt("entryOrInsert", values),
		"insert" => Expr::rt("entryInsert", values),
		"or_insert_with" | "or_insert_with_key" => Expr::rt("entryOrInsertWith", values),
		"or_default" => {
			let tcx = b.t.tcx;
			let default = trait_fn(tcx, "Default", "default");
			values.push(b.fn_value(default, tcx.mk_args(&[value.into()]), span));
			Expr::rt("entryOrInsertWith", values)
		}
		"and_modify" => Expr::rt("entryAndModify", values),
		"key" => Expr::rt("entryKey", values),
		"into_key" => values.pop()?.member("k"),
		"get" | "get_mut" | "into_mut" => Expr::rt("entryGet", values),
		"remove" => Expr::rt("entryRemove", values),
		"remove_entry" => Expr::rt("entryRemoveEntry", values),
		_ => return None,
	};
	Some(e)
}

/// Whether `entry`, an `Entry`, is its variant `index`: `Occupied` while the map holds
/// its key. Each variant's entry is the `Entry` itself.
pub fn entry_test(adt: ty::AdtDef<'_>, index: rustc_abi::VariantIdx, entry: Expr) -> Expr {
	let occupied = Expr::rt("entryOccupied", vec![entry]);
	match adt.variant(index).name.as_str() {
		"Occupied" => occupied,
		_ => occupied.not(),
	}
}

/// Whether `entry` is a `VacantEntry`, whose `insert` inserts and returns the value.
fn is_vacant<'tcx>(b: &Body<'_, '_, 'tcx>, entry: Ty<'tcx>) -> bool {
	matches!(entry.kind(), ty::Adt(adt, _) if b.t.tcx.item_name(adt.did()).as_str() == "VacantEntry")
}

/// An `Ordering` variant, which is its discriminant: `Less` is `-1`, `Equal` `0`, and
/// `Greater` `1`.
pub fn ordering_variant(index: rustc_abi::VariantIdx) -> Expr {
	Expr::lit((index.as_u32() as i32 - 1).to_string())
}

/// `Ordering`'s methods, on its discriminant.
fn ordering(b: &mut Body<'_, '_, '_>, name: &str, args: Args<'_>, span: Span, out: &mut Vec<Stmt>) -> Option<Expr> {
	let _ = span;
	let mut values = b.args(args, out);
	let zero = || Expr::lit("0");
	let e = match name {
		"is_eq" => values.pop()?.binary("===", zero()),
		"is_ne" => values.pop()?.binary("!==", zero()),
		"is_lt" => values.pop()?.binary("<", zero()),
		"is_gt" => values.pop()?.binary(">", zero()),
		"is_le" => values.pop()?.binary("<=", zero()),
		"is_ge" => values.pop()?.binary(">=", zero()),
		"reverse" => Expr::rt("orderingReverse", values),
		"then" => Expr::rt("orderingThen", values),
		"then_with" => {
			let [o, f] = <[Expr; 2]>::try_from(values).ok()?;
			Expr::rt("orderingThen", vec![o, f, Expr::lit("true")])
		}
		_ => return None,
	};
	Some(e)
}

/// `Hasher`'s methods: writing an integer or a string, and `finish`.
fn hasher_call(b: &mut Body<'_, '_, '_>, name: &str, args: Args<'_>, out: &mut Vec<Stmt>) -> Expr {
	let values = b.args(args, out);
	let swapped = |values: Vec<Expr>| values.into_iter().rev().collect::<Vec<_>>();
	match name {
		"finish" => Expr::rt("hashFinish", values),
		"write_str" => Expr::rt("hashStr", swapped(values)),
		n if n.starts_with("write_") => Expr::rt("hashInt", swapped(values)),
		_ => Expr::undefined(),
	}
}

/// The discriminant of `value`, an enum of the crate, as `discriminant_value` reads it.
fn discriminant<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, value: Expr, span: Span) -> Expr {
	let tcx = b.t.tcx;
	let ty = ty.peel_refs();
	let ty::Adt(adt, _) = ty.kind() else {
		return b.err(span, format!("the discriminant of `{ty}`"));
	};
	if !adt.is_enum() || !adt.did().is_local() {
		return b.err(span, format!("the discriminant of `{ty}`"));
	}
	let info = b.t.adt(*adt, span);
	let Some(int) = Int::of(ty.discriminant_ty(tcx)) else {
		return b.err(span, format!("the discriminant of `{ty}`"));
	};
	let table = adt
		.discriminants(tcx)
		.map(|(i, d)| {
			(
				crate::ts::Key::Name(adt.variant(i).name.to_string()),
				int.lit(int.fold(d.val as i128)),
			)
		})
		.collect();
	let tag = match info.repr {
		super::AdtRepr::Tags => value,
		_ => value.member("tag"),
	};
	Expr::Index(Box::new(Expr::Object(table)), Box::new(tag))
}

/// Whether the runtime compares and hashes values of `ty`, a std type, as its `Ord` and
/// `Hash` do.
fn std_key<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> bool {
	match ty.kind() {
		ty::Bool | ty::Int(_) | ty::Uint(_) | ty::Str => true,
		ty::Tuple(tys) => !tys.is_empty(),
		ty::Adt(..) => matches!(b.t.std_of(ty), Some(Std::String | Std::Ordering)),
		_ => false,
	}
}

/// `Ord::cmp` of `ty`, as a function: the runtime's for a std type, or the crate's.
fn cmp_fn<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span) -> Expr {
	let tcx = b.t.tcx;
	let ty = peel(b, ty);
	match ty.kind() {
		ty::Bool | ty::Int(_) | ty::Uint(_) => Expr::Rt("cmp"),
		ty::Str => Expr::Rt("cmpStr"),
		ty::Adt(..) if b.t.std_of(ty) == Some(Std::String) => Expr::Rt("cmpStr"),
		ty::Adt(..) if b.t.std_of(ty) == Some(Std::Ordering) => Expr::Rt("cmp"),
		ty::Tuple(tys) if !tys.is_empty() => {
			let fns = tys.iter().map(|t| cmp_fn(b, t, span)).collect();
			Expr::rt("cmpTuple", vec![Expr::Array(fns)])
		}
		ty::Adt(adt, _) if adt.did().is_local() => {
			let cmp = trait_fn(tcx, "Ord", "cmp");
			b.fn_value(cmp, tcx.mk_args(&[ty.into()]), span)
		}
		_ => b.err(span, format!("ordering `{ty}`, which is not supported yet")),
	}
}

/// `PartialEq::eq` of `ty`, as a function: the runtime's for a std type, or the crate's.
fn eq_fn<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span) -> Expr {
	let tcx = b.t.tcx;
	let ty = peel(b, ty);
	match ty.kind() {
		ty::Int(_) | ty::Uint(_)
			if let Some(int) = Int::of(ty)
				&& int.big() =>
		{
			Expr::Rt(int.rt()).member("eq")
		}
		ty::Bool | ty::Int(_) | ty::Uint(_) | ty::Str => Expr::Rt("same"),
		ty::Adt(..) if matches!(b.t.std_of(ty), Some(Std::String | Std::Ordering)) => Expr::Rt("same"),
		ty::Tuple(tys) if !tys.is_empty() => {
			let fns = tys.iter().map(|t| eq_fn(b, t, span)).collect();
			Expr::rt("eqTuple", vec![Expr::Array(fns)])
		}
		ty::Adt(adt, _) if adt.did().is_local() => {
			let eq = trait_fn(tcx, "PartialEq", "eq");
			b.fn_value(eq, tcx.mk_args(&[ty.into(), ty.into()]), span)
		}
		_ => b.err(span, format!("comparing `{ty}` as a key, which is not supported yet")),
	}
}

/// `Hash::hash` of `ty` into a `hasher`, as a function: the runtime's for a std type, or
/// the crate's.
fn hash_fn<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, hasher: Ty<'tcx>, span: Span) -> Expr {
	let tcx = b.t.tcx;
	let ty = peel(b, ty);
	match ty.kind() {
		ty::Bool | ty::Int(_) | ty::Uint(_) => Expr::Rt("hashInt"),
		ty::Str => Expr::Rt("hashStr"),
		ty::Adt(..) if b.t.std_of(ty) == Some(Std::String) => Expr::Rt("hashStr"),
		ty::Adt(..) if b.t.std_of(ty) == Some(Std::Ordering) => Expr::Rt("hashInt"),
		ty::Tuple(tys) if !tys.is_empty() => {
			let fns = tys.iter().map(|t| hash_fn(b, t, hasher, span)).collect();
			Expr::rt("hashTuple", vec![Expr::Array(fns)])
		}
		ty::Adt(adt, _) if adt.did().is_local() => {
			let hash = trait_fn(tcx, "Hash", "hash");
			b.fn_value(hash, tcx.mk_args(&[ty.into(), hasher.into()]), span)
		}
		_ => b.err(span, format!("hashing `{ty}` as a key, which is not supported yet")),
	}
}

/// `Clone::clone` of `ty`, as a function, or `undefined` where it is the value itself.
fn clone_fn<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span) -> Expr {
	let tcx = b.t.tcx;
	if b.needs_copy(ty) && library::clone_copies(b.t, ty) {
		return Expr::Rt("copy");
	}
	if (b.t.is_copy(ty) && library::clone_copies(b.t, ty)) || b.t.std_of(ty) == Some(Std::String) {
		return Expr::undefined();
	}
	let clone = trait_fn(tcx, "Clone", "clone");
	b.fn_value(clone, tcx.mk_args(&[ty.into()]), span)
}

/// An item of one of std's traits, by the trait's diagnostic name: `Ord`'s `cmp`.
fn trait_fn(tcx: ty::TyCtxt<'_>, tr: &str, item: &str) -> rustc_hir::def_id::DefId {
	let tr = tcx
		.get_diagnostic_item(rustc_span::Symbol::intern(tr))
		.expect("a trait of std");
	tcx.associated_items(tr)
		.filter_by_name_unhygienic(rustc_span::Symbol::intern(item))
		.next()
		.expect("an item of a trait of std")
		.def_id
}

/// What `into_iter` makes of a collection of type `ty`, by value or by reference: an
/// `rt.Iter` of its entries, keys, or items.
pub fn into_iter<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>, value: Expr) -> Option<Expr> {
	let name = match b.t.std_of(ty.peel_refs())? {
		Std::HashMap => "hashIter",
		Std::HashSet => "hashKeys",
		Std::BTreeMap => "btreeIter",
		Std::BTreeSet => "btreeKeys",
		Std::VecDeque => "dequeIter",
		_ => return None,
	};
	Some(Expr::rt(name, vec![value]))
}

/// `collect` into a collection of type `ty`.
pub fn collect<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, iter: Expr, span: Span) -> Option<Expr> {
	let std = b.t.std_of(ty)?;
	if let Some(elem) = undefined_arg(b, std, ty) {
		return Some(b.err(
			span,
			format!("`{ty}` holds `{elem}`, which is `undefined` as `None` is, which is not supported"),
		));
	}
	let (target, extend) = match std {
		Std::HashMap => (hash_new(b, ty, span)?, "hashExtend"),
		Std::HashSet => (hash_new(b, ty, span)?, "hashExtendKeys"),
		Std::BTreeMap => (btree_new(b, ty, span), "btreeExtend"),
		Std::BTreeSet => (btree_new(b, ty, span), "btreeExtendKeys"),
		Std::VecDeque => (deque_new(b, ty, span), "dequeExtend"),
		_ => return None,
	};
	// A map or set drops the duplicates it is given.
	if std != Std::VecDeque && b.t.drops(ty) {
		return Some(b.err(
			span,
			format!("collecting into `{ty}`, which may drop a duplicate, which is not supported yet"),
		));
	}
	let mut values = vec![target, iter];
	// A B-tree's `collect` keeps the last of equal keys, as std's sorted bulk build does.
	if matches!(b.t.std_of(ty), Some(Std::BTreeMap | Std::BTreeSet)) {
		values.push(Expr::lit("true"));
	}
	Some(Expr::rt(extend, values))
}
