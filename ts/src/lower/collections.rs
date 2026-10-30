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
use super::library::{self, Call, Std};
use super::num::Int;
use crate::ts::{Expr, Stmt};

/// A call of a standard library function on these types, or its arguments back if it is
/// none of theirs.
pub fn call<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	call: &Call<'tcx>,
	owner: &str,
	name: &str,
	mut args: Vec<Expr>,
	out: &mut Vec<Stmt>,
) -> Result<Option<Expr>, Vec<Expr>> {
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
				let f = cmp_fn(b, t, name == "partial_cmp", span);
				return Ok(Some(f.call(args)));
			}
			("Hash", "hash", ..) if let Some(t) = self_ty => {
				let t = peel(b, t);
				// Only into `DefaultHasher`, whose state the runtime's functions write.
				let hasher = call.args.types().nth(1).filter(|&h| b.t.std_of(h) == Some(Std::Hasher));
				let (true, Some(hasher)) = (std_key(b, t), hasher) else {
					return Err(args);
				};
				let f = hash_fn(b, t, hasher, span);
				return Ok(Some(f.call(args)));
			}
			// `Ord`'s defaults, of a std type or the crate's, which compare by `PartialOrd`'s `<`,
			// `<=`, and `>`.
			("Ord", "max" | "min" | "clamp", ..) if let Some(t) = self_ty => {
				let t = peel(b, t);
				let local = matches!(t.kind(), ty::Adt(adt, _) if b.t.own(adt.did()));
				if !(std_key(b, t) || local) || Int::of(t).is_some() {
					return Err(args);
				}
				let tests: &[&str] = if name == "clamp" { &["le", "lt", "gt"] } else { &["lt"] };
				let mut values = args;
				values.extend(tests.iter().map(|test| test_fn(b, t, test, span)));
				// What drops the operands it does not return.
				let Some(droppers) = b.t.droppers(self_ty, span) else {
					return Ok(Some(Expr::undefined()));
				};
				values.extend(droppers);
				let f = match name {
					"max" => "ordMax",
					"min" => "ordMin",
					_ => "ordClamp",
				};
				return Ok(Some(Expr::rt(f, values)));
			}
			// A std type's own comparisons, as a tuple's are, by its `partial_cmp`. The library
			// maps the integers' and `bool`'s.
			("PartialOrd", "lt" | "le" | "gt" | "ge", ..) if let Some(t) = self_ty => {
				let t = peel(b, t);
				if matches!(t.kind(), ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_)) || !std_key(b, t) {
					return Err(args);
				}
				if own_test(b, t, name) {
					return Ok(Some(test_fn(b, t, name, span).call(args)));
				}
				let order = cmp_fn(b, t, true, span).call(args);
				let test = match name {
					"lt" => library::Inherited::Lt,
					"le" => library::Inherited::Le,
					"gt" => library::Inherited::Gt,
					_ => library::Inherited::Ge,
				};
				return Ok(Some(test.test(b, order, out)));
			}
			("PartialEq", "eq" | "ne", ..) if let Some(t) = self_ty => {
				// The library maps the integers' and `bool`'s.
				let t = peel(b, t);
				if matches!(t.kind(), ty::Bool | ty::Int(_) | ty::Uint(_)) || !std_key(b, t) {
					return Err(args);
				}
				// A tuple's and an `Arc`'s `ne` call what they hold's, which `!eq` stands for
				// only while that is the default.
				if name == "ne" && own_ne(b, t) {
					return Ok(Some(b.err(
						span,
						format!("`<{t} as PartialEq>::ne` through a held type's own, which is not supported yet"),
					)));
				}
				let eq = match (eq_fn(b, t, span), <[Expr; 2]>::try_from(args)) {
					(Expr::Rt(same), Ok([l, r])) if same == "same" => l.binary("===", r),
					(eq, Ok(args)) => eq.call(args.into()),
					(_, Err(args)) => return Err(args),
				};
				return Ok(Some(if name == "eq" { eq } else { eq.not() }));
			}
			("Borrow", "borrow", ..) | ("BorrowMut", "borrow_mut", ..) if let Some(t) = self_ty => {
				let by = library::targ(call.args, 1);
				let value = match (same_key(b, t, by), arc_of(b, peel(b, t))) {
					(true, _) => args.pop(),
					(false, Some(inner)) if same_key(b, inner, by) => args.pop().map(|v| v.member("value")),
					_ => return Err(args),
				};
				return Ok(value);
			}
			("Hasher", _, Some(Std::Hasher), _) => return Ok(Some(hasher_call(b, name, args, span, out))),
			// The discriminant derived `Hash` and `Ord` read of an enum.
			("intrinsics", "discriminant_value", ..) if let Some(t) = self_ty => {
				let value = args.pop();
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
	if let Some(by) = lookup_ty(b, std, call, owner, name)
		&& !same_key(b, arg(coll, 0), by)
	{
		// An `Arc`'s `Hash`, `Eq`, and `Ord` are its value's, so the value it is looked up by
		// stands in as an `Arc` of it, which the collection only compares.
		let arc = arc_of(b, peel(b, arg(coll, 0))).is_some_and(|inner| same_key(b, inner, by));
		match args.get_mut(1) {
			Some(key) if arc && !matches!(name, "range" | "range_mut") => {
				let value = std::mem::replace(key, Expr::undefined());
				*key = Expr::New(Box::new(Expr::Rt("Arc".into())), vec![value]);
			}
			_ => {
				return Ok(Some(b.err(
					span,
					format!("`{owner}::{name}` of `{coll}` by `{by}`, which is not supported yet"),
				)));
			}
		}
	}
	let e = match std {
		Std::HashMap | Std::HashSet => hash(b, coll, call, owner, name, args, out),
		Std::BTreeMap | Std::BTreeSet => btree(b, coll, call, owner, name, args),
		Std::VecDeque => deque(b, coll, call, owner, name, args, out),
		Std::BinaryHeap => heap(b, coll, call, owner, name, args, out),
		Std::PeekMut if name == "pop" => Some(Expr::rt("peekMutPop", args)),
		Std::Entry => entry(b, coll, name, args, span),
		Std::Ordering => ordering(name, args),
		Std::Hasher if matches!(name, "new" | "default") => Some(Expr::rt("hasher", vec![])),
		_ => return Err(args),
	};
	let Some(e) = e else {
		return Ok(None);
	};
	// The runtime drops what std's would rather than return or keep, given what drops it.
	let lost = discarded(std, owner, name).unwrap_or_default();
	let Some(mut droppers) = b.t.droppers(lost.iter().map(|&n| arg(coll, n)), span) else {
		return Ok(Some(Expr::undefined()));
	};
	// A map's value that can be `undefined`, whose `Option` `get` returns tells `None` apart.
	if library::none_last(owner, name)
		&& let Some(none) = library::sentinel_option(b.t, call).and_then(|inner| b.t.none_arg(inner))
	{
		droppers.resize(lost.len(), Expr::undefined());
		droppers.push(none);
	}
	if droppers.is_empty() {
		return Ok(Some(e));
	}
	Ok(Some(match e {
		Expr::Call(f, mut args) => {
			args.extend(droppers);
			Expr::Call(f, args)
		}
		e => e,
	}))
}

/// What a method of a collection of type `std` may drop rather than return or keep, as the
/// indices of the collection's type arguments whose runtime function takes what drops each,
/// after its own arguments: the key `insert` and `entry` are given if an equal one is there
/// already, the key `remove` takes out, the value `or_insert` is given if one is, and what
/// `clear`, `retain`, and `truncate` discard. An iterator, and `extend` and `from`, take
/// theirs where they map.
fn discarded(std: Std, owner: &str, name: &str) -> Option<&'static [usize]> {
	let map = matches!(std, Std::HashMap | Std::BTreeMap);
	let set = matches!(std, Std::HashSet | Std::BTreeSet);
	Some(match (owner, name) {
		(_, "insert" | "entry" | "remove") if map || set => &[0],
		(_, "or_insert_with" | "or_insert_with_key" | "or_default") if std == Std::Entry => &[],
		(_, "remove") if std == Std::Entry => &[0],
		(_, "or_insert") if std == Std::Entry => &[1],
		(_, "clear" | "retain" | "retain_mut") if map => &[0, 1],
		(_, "clear" | "retain" | "retain_mut") if set => &[0],
		(_, "clear" | "retain" | "retain_mut" | "truncate") if std == Std::VecDeque => &[0],
		(_, "clear" | "retain") if std == Std::BinaryHeap => &[0],
		_ => return None,
	})
}

/// What drops each key, and each value of a map, of a collection of type `coll`, of those
/// that drop: `undefined` for one that drops nothing.
fn entry_droppers<'tcx>(b: &mut Body<'_, '_, 'tcx>, coll: Ty<'tcx>, span: Span) -> Vec<Expr> {
	let n = match b.t.std_of(coll) {
		Some(Std::HashMap | Std::BTreeMap) => 2,
		_ => 1,
	};
	(0..n).map(|i| b.t.dropper(arg(coll, i), span)).collect()
}

/// What a collection of type `coll` stores each of: a key, a `(key, value)`, or an item.
fn stored<'tcx>(b: &Body<'_, '_, 'tcx>, coll: Ty<'tcx>) -> Ty<'tcx> {
	match b.t.std_of(coll) {
		Some(Std::HashMap | Std::BTreeMap) => Ty::new_tup(b.t.tcx, &[arg(coll, 0), arg(coll, 1)]),
		_ => arg(coll, 0),
	}
}

/// The type a lookup of a map or set takes its key as: the `Q` of `get<Q>` where
/// `K: Borrow<Q>`, or of `Index<&Q>`.
fn lookup_ty<'tcx>(b: &Body<'_, '_, 'tcx>, std: Std, call: &Call<'tcx>, owner: &str, name: &str) -> Option<Ty<'tcx>> {
	if !matches!(std, Std::HashMap | Std::HashSet | Std::BTreeMap | Std::BTreeSet) {
		return None;
	}
	match (owner, name) {
		("Index", "index") => Some(library::targ(call.args, 1)),
		(
			_,
			"get" | "get_mut" | "get_key_value" | "contains_key" | "contains" | "remove" | "remove_entry" | "take"
			| "range" | "range_mut" | "split_off",
		) => {
			let tcx = b.t.tcx;
			let imp = tcx.inherent_impl_of_assoc(call.def_id)?;
			call.args[tcx.generics_of(imp).count()..]
				.iter()
				.find_map(|a| a.as_type())
		}
		_ => None,
	}
}

/// Whether a key of type `by` is a JS value that hashes, compares, and orders as one of
/// type `key` does: the same type through references and boxes, or a string either way.
fn same_key<'tcx>(b: &Body<'_, '_, 'tcx>, key: Ty<'tcx>, by: Ty<'tcx>) -> bool {
	let tcx = b.t.tcx;
	let (key, by) = (peel(b, key), peel(b, by));
	tcx.erase_and_anonymize_regions(key) == tcx.erase_and_anonymize_regions(by)
		|| (library::string(tcx, key) && library::string(tcx, by))
}

/// A key or item type of `coll` that can be `undefined` in JS, so that an `Option` of it,
/// as `first` returns, could not tell `None` from `Some`. A map's value can be: its
/// `get`, `insert`, and `remove` take the `Option`'s `rt.None`.
fn undefined_arg<'tcx>(b: &Body<'_, '_, 'tcx>, std: Std, coll: Ty<'tcx>) -> Option<Ty<'tcx>> {
	if !matches!(
		std,
		Std::HashMap | Std::BTreeMap | Std::Entry | Std::HashSet | Std::BTreeSet | Std::VecDeque | Std::BinaryHeap
	) {
		return None;
	}
	Some(arg(coll, 0)).filter(|&key| b.t.erases_to(key).undefined)
}

/// Whether `call` lends a value a collection or an atomic holds that JS copies, which its
/// mapping makes an `rt.Place` of: returned as `get_mut` and an entry's do, yielded as
/// `iter_mut`'s, or handed to a closure as `retain`'s. An atomic is that place itself.
pub fn place_call<'tcx>(b: &Body<'_, '_, 'tcx>, call: &Call<'tcx>) -> bool {
	let tcx = b.t.tcx;
	let Some(std) = library::inherent_std(tcx, call.def_id) else {
		return false;
	};
	let name = tcx.item_name(call.def_id);
	let coll = impl_self_ty(b, call);
	let (methods, value): (&[&str], usize) = match std {
		Std::Vec => (&["retain_mut", "dedup_by", "dedup_by_key"], 0),
		Std::VecDeque => (&["get_mut", "front_mut", "back_mut", "iter_mut", "retain_mut"], 0),
		Std::HashMap => (&["get_mut", "iter_mut", "values_mut", "retain"], 1),
		Std::BTreeMap => (&["get_mut", "iter_mut", "values_mut", "range_mut", "retain"], 1),
		Std::Entry if is_vacant(b, coll) => (&["insert"], 1),
		Std::Entry => (
			&[
				"or_insert",
				"or_insert_with",
				"or_insert_with_key",
				"or_default",
				"get_mut",
				"into_mut",
				"and_modify",
			],
			1,
		),
		Std::Atomic => (&["get_mut"], 0),
		_ => return false,
	};
	methods.contains(&name.as_str()) && b.t.needs_place(arg(coll, value))
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

/// What a JS value of `ty` is: through references and boxes, which are their pointee,
/// up to a `&mut` that is an `rt.Place`, and a `NonZero`, which is its integer.
fn peel<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> Ty<'tcx> {
	match ty.kind() {
		ty::Ref(_, inner, _) if b.t.place_pointee(ty).is_none() => peel(b, *inner),
		ty::Adt(adt, args) if matches!(Std::of(b.t.tcx, adt.did()), Some(Std::Box | Std::NonZero)) => {
			peel(b, args.type_at(0))
		}
		_ => ty,
	}
}

/// Evaluate `values` for their effects: a receiver or a capacity, which nothing here keeps.
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
	args: Vec<Expr>,
	out: &mut Vec<Stmt>,
) -> Option<Expr> {
	let span = call.span;
	let set = b.t.std_of(coll) == Some(Std::HashSet);
	let key = arg(coll, 0);
	let value = if set { None } else { Some(arg(coll, 1)) };
	let mut values = args;
	let e = match (owner, name) {
		(_, "new" | "with_capacity") | ("Default", "default") => {
			discard(values, out);
			hash_new(b, coll, span)?
		}
		(_, "len") => Expr::rt("hashLen", values),
		(_, "is_empty") => Expr::rt("hashLen", values).binary("===", Expr::lit("0")),
		(_, "clear") => Expr::rt("hashClear", values),
		(_, "contains_key" | "contains") => Expr::rt("hashContains", values),
		(_, "get_mut") if !set && b.t.needs_place(arg(coll, 1)) => Expr::rt("hashGetMut", values),
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
		(_, "retain") if !set && b.t.needs_place(arg(coll, 1)) => Expr::rt("hashRetainMut", values),
		(_, "retain") => Expr::rt("hashRetain", values),
		(_, "iter_mut") if b.t.needs_place(arg(coll, 1)) => Expr::rt("hashIterMut", values),
		(_, "values_mut") if b.t.needs_place(arg(coll, 1)) => Expr::rt("hashValuesMut", values),
		(_, "iter" | "iter_mut") if !set => Expr::rt("hashIter", values),
		(_, "iter" | "keys") => Expr::rt("hashKeys", values),
		(_, "values" | "values_mut") => Expr::rt("hashValues", values),
		// std's `fold` of either drops the half it does not yield after the one it does, where
		// `next` drops it before.
		(_, "into_keys" | "into_values") => {
			let (kept, other) = if name == "into_keys" { (0, 1) } else { (1, 0) };
			if b.t.drops(arg(coll, other)) {
				let other = arg(coll, other);
				return Some(b.err(
					span,
					format!("`{name}` of `{coll}`, which drops `{other}`, which is not supported yet"),
				));
			}
			let dropper = b.t.dropper(arg(coll, kept), span);
			let f = if name == "into_keys" { "hashKeys" } else { "hashValues" };
			Expr::rt(f, super::iter::tail(values, vec![dropper]))
		}
		(_, "drain") => {
			let droppers = entry_droppers(b, coll, span);
			Expr::rt(
				if set { "hashDrainKeys" } else { "hashDrain" },
				super::iter::tail(values, droppers),
			)
		}
		(_, "entry") => Expr::rt("entry", values),
		(_, "reserve" | "shrink_to_fit") => {
			discard(values, out);
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
			let (source, iter) = extend_source(b, coll, call, owner, values.pop()?)?;
			let target = match owner {
				"From" => hash_new(b, coll, span)?,
				_ => values.pop()?,
			};
			hash_extend(b, coll, target, source, iter, span)
		}
		_ => return None,
	};
	Some(e)
}

/// `extend` of `target`, a hash collection of type `coll`, from `source`, an iterator of
/// type `iter`, or `collect` into an empty one.
fn hash_extend<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	coll: Ty<'tcx>,
	target: Expr,
	source: Expr,
	iter: Ty<'tcx>,
	span: Span,
) -> Expr {
	let mut droppers = entry_droppers(b, coll, span);
	// By `for_each`.
	droppers.push(super::iter::iter_take(b, iter, true, span));
	let f = match b.t.std_of(coll) {
		Some(Std::HashSet) => "hashExtendKeys",
		_ => "hashExtend",
	};
	Expr::rt(f, super::iter::tail(vec![target, source], droppers))
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
		ty::Bool | ty::Char | ty::Str => true,
		ty::Int(_) | ty::Uint(_) => !Int::of(key)?.big(),
		ty::Adt(..) if library::string(b.t.tcx, key) => true,
		ty::Adt(adt, _) if b.t.own(adt.did()) && fieldless(*adt) => derived_eq(b, key),
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

/// Whether `ty`'s `ne` reaches the crate's own `ne` of a type it holds: through tuples,
/// `Arc`s, `Reverse`s, and the elements of slices, but not an `Option`, whose `ne` is
/// `!eq`.
fn own_ne<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> bool {
	let tcx = b.t.tcx;
	let ty = peel(b, ty);
	match ty.kind() {
		ty::Tuple(tys) => tys.iter().any(|t| own_ne(b, t)),
		_ if let Some(elem) = elements_of(b, ty) => own_ne(b, elem),
		ty::Adt(..) if let Some(inner) = arc_of(b, ty).or_else(|| reverse_of(b, ty)) => own_ne(b, inner),
		ty::Adt(adt, _) if b.t.own(adt.did()) => {
			let ne = trait_fn(tcx, "PartialEq", "ne");
			Instance::try_resolve(tcx, b.t.typing_env_pub(), ne, tcx.mk_args(&[ty.into(), ty.into()]))
				.ok()
				.flatten()
				.is_some_and(|instance| instance.def_id() != ne)
		}
		_ => false,
	}
}

/// Whether `ty`'s `PartialOrd::{test}`, `lt` or another of its tests, reaches the crate's
/// own `test` of a type it holds, which `partial_cmp` stands for only while that is the
/// default: through a tuple's last element, `Arc`s, and `Reverse`s.
fn own_test<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>, test: &str) -> bool {
	let tcx = b.t.tcx;
	let ty = peel(b, ty);
	match ty.kind() {
		ty::Tuple(tys) => tys.last().is_some_and(|&t| own_test(b, t, test)),
		ty::Adt(..) if let Some(inner) = arc_of(b, ty).or_else(|| reverse_of(b, ty)) => own_test(b, inner, test),
		ty::Adt(adt, _) if b.t.own(adt.did()) => {
			let f = trait_fn(tcx, "PartialOrd", test);
			Instance::try_resolve(tcx, b.t.typing_env_pub(), f, tcx.mk_args(&[ty.into(), ty.into()]))
				.ok()
				.flatten()
				.is_some_and(|instance| instance.def_id() != f)
		}
		_ => false,
	}
}

/// `PartialOrd::{test}` of `ty` as a function, `lt` or another of its tests: the crate's own
/// where `ty` reaches it, or else by `partial_cmp`, as the trait's defaults test it.
pub fn test_fn<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, test: &str, span: Span) -> Expr {
	let tcx = b.t.tcx;
	let ty = peel(b, ty);
	// An integer's, which the runtime tests directly rather than through `partial_cmp`.
	if matches!(ty.kind(), ty::Char | ty::Int(_) | ty::Uint(_)) {
		return Expr::Rt(test.into());
	}
	if !own_test(b, ty, test) {
		let by = match test {
			"lt" => "cmpLt",
			"le" => "cmpLe",
			"gt" => "cmpGt",
			_ => "cmpGe",
		};
		return Expr::rt(by, vec![cmp_fn(b, ty, true, span)]);
	}
	match ty.kind() {
		// The fields before the last chain by `partial_cmp`, which no crate's type overrides.
		ty::Tuple(tys) if let Some((&last, init)) = tys.split_last() => {
			let cmps = init.iter().map(|&t| cmp_fn(b, t, true, span)).collect();
			let order = if matches!(test, "lt" | "le") { "-1" } else { "1" };
			let last = test_fn(b, last, test, span);
			Expr::rt("testTuple", vec![Expr::Array(cmps), last, Expr::lit(order)])
		}
		ty::Adt(..) if let Some(inner) = arc_of(b, ty) => Expr::rt("cmpArc", vec![test_fn(b, inner, test, span)]),
		ty::Adt(..) if let Some(inner) = reverse_of(b, ty) => {
			Expr::rt("cmpReverse", vec![test_fn(b, inner, test, span)])
		}
		_ => {
			let f = trait_fn(tcx, "PartialOrd", test);
			b.fn_value(f, tcx.mk_args(&[ty.into(), ty.into()]), span)
		}
	}
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
	args: Vec<Expr>,
) -> Option<Expr> {
	let span = call.span;
	let set = b.t.std_of(coll) == Some(Std::BTreeSet);
	let key = arg(coll, 0);
	let value = if set { None } else { Some(arg(coll, 1)) };
	let mut values = args;
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
		(_, "get_mut") if !set && b.t.needs_place(arg(coll, 1)) => Expr::rt("btreeGetMut", values),
		(_, "get" | "get_mut") if !set => Expr::rt("btreeGet", values),
		(_, "get_key_value") => Expr::rt("btreeGetKeyValue", values),
		(_, "insert") if set => Expr::rt("btreeAdd", values),
		(_, "insert") => Expr::rt("btreeInsert", values),
		(_, "remove") if set => Expr::rt("btreeDelete", values),
		(_, "remove") => Expr::rt("btreeRemove", values),
		(_, "remove_entry") => Expr::rt("btreeRemoveEntry", values),
		(_, "retain") if !set && b.t.needs_place(arg(coll, 1)) => Expr::rt("btreeRetainMut", values),
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
		(_, "iter_mut") if b.t.needs_place(arg(coll, 1)) => Expr::rt("btreeIterMut", values),
		(_, "values_mut") if b.t.needs_place(arg(coll, 1)) => Expr::rt("btreeValuesMut", values),
		(_, "iter" | "iter_mut") if !set => Expr::rt("btreeIter", values),
		(_, "iter" | "keys") => Expr::rt("btreeKeys", values),
		(_, "values" | "values_mut") => Expr::rt("btreeValues", values),
		// Each of which drops the half it does not yield as it reads, and whose `last` reads
		// the back.
		(_, "into_keys" | "into_values") => {
			let mut droppers = entry_droppers(b, coll, span);
			if name == "into_keys" && b.t.drops(coll) {
				droppers.push(Expr::lit("true"));
			}
			let f = if name == "into_keys" {
				"btreeKeys"
			} else {
				"btreeValues"
			};
			Expr::rt(f, super::iter::tail(values, droppers))
		}
		(_, "range") if set => Expr::rt("btreeRangeKeys", values),
		(_, "range_mut") if b.t.needs_place(arg(coll, 1)) => Expr::rt("btreeRangeMut", values),
		(_, "range" | "range_mut") => Expr::rt("btreeRange", values),
		(_, "entry") => Expr::rt("entry", values),
		(_, "split_off") => Expr::rt("btreeSplitOff", values),
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
		("Extend", "extend") => {
			let (source, iter) = extend_source(b, coll, call, owner, values.pop()?)?;
			let mut droppers = vec![Expr::lit("false")];
			droppers.extend(entry_droppers(b, coll, span));
			// By `for_each`.
			droppers.push(super::iter::iter_take(b, iter, true, span));
			let target = values.pop()?;
			let f = if set { "btreeExtendKeys" } else { "btreeExtend" };
			Expr::rt(f, super::iter::tail(vec![target, source], droppers))
		}
		("From", "from") => {
			let (source, iter) = extend_source(b, coll, call, owner, values.pop()?)?;
			btree_collect(b, coll, source, iter, span)
		}
		_ => return None,
	};
	Some(e)
}

/// `collect` and `from` into an empty B-tree of type `coll`, from `source`, an iterator of
/// type `iter`, which keep the last of equal keys, as std's sorted bulk build does. Of
/// entries that drop, std's collects into a `Vec` first, and then drops each but the last
/// of equal keys in the order a stable sort leaves them.
fn btree_collect<'tcx>(b: &mut Body<'_, '_, 'tcx>, coll: Ty<'tcx>, source: Expr, iter: Ty<'tcx>, span: Span) -> Expr {
	let set = b.t.std_of(coll) == Some(Std::BTreeSet);
	let target = btree_new(b, coll, span);
	let key = arg(coll, 0);
	if !b.t.drops(coll) {
		let mut droppers = vec![Expr::lit("true"), Expr::undefined()];
		if !set {
			droppers.push(Expr::undefined());
		}
		// Into a `Vec` first.
		let folds = super::iter::implements(b.t, iter, "TrustedLen");
		droppers.push(super::iter::iter_take(b, iter, folds, span));
		let f = if set { "btreeExtendKeys" } else { "btreeExtend" };
		return Expr::rt(f, super::iter::tail(vec![target, source], droppers));
	}
	// A sort by an `Ord` of the crate's may panic, or leave equal keys in another order.
	if own_ord(b, key, &mut Default::default()) {
		return b.err(
			span,
			format!(
				"`collect` of `{coll}`, which sorts by `{key}`'s own `Ord`, of entries that drop, which is not supported yet"
			),
		);
	}
	let entries = super::iter::to_array(b, source, iter, stored(b, coll), span);
	let droppers = entry_droppers(b, coll, span);
	let f = if set { "btreeCollectKeys" } else { "btreeCollect" };
	Expr::rt(f, super::iter::tail(vec![target, entries], droppers))
}

/// Whether ordering or comparing a `ty` may run the crate's own `Ord` or `PartialEq`,
/// rather than one derived, which may panic or disagree with itself.
fn own_ord<'tcx>(
	b: &Body<'_, '_, 'tcx>,
	ty: Ty<'tcx>,
	seen: &mut rustc_data_structures::fx::FxHashSet<Ty<'tcx>>,
) -> bool {
	let tcx = b.t.tcx;
	let ty = peel(b, ty);
	if !seen.insert(ty) {
		return false;
	}
	match ty.kind() {
		ty::Tuple(tys) => tys.iter().any(|t| own_ord(b, t, seen)),
		ty::Adt(..) if let Some(inner) = option_of(b, ty).or_else(|| arc_of(b, ty)) => own_ord(b, inner, seen),
		ty::Adt(adt, args) if b.t.own(adt.did()) => {
			let derived = |tr: &str, item: &str, args: &[ty::GenericArg<'tcx>]| {
				let f = trait_fn(tcx, tr, item);
				Instance::try_resolve(tcx, b.t.typing_env_pub(), f, tcx.mk_args(args))
					.ok()
					.flatten()
					.and_then(|instance| tcx.impl_of_assoc(instance.def_id()))
					.is_some_and(|imp| tcx.is_automatically_derived(imp))
			};
			!derived("Ord", "cmp", &[ty.into()])
				|| !derived("PartialEq", "eq", &[ty.into(), ty.into()])
				|| adt
					.all_fields()
					.any(|f| own_ord(b, f.ty(tcx, args).skip_norm_wip(), seen))
		}
		_ => false,
	}
}

/// An empty B-tree of type `coll`, ordered by its keys' `Ord`.
fn btree_new<'tcx>(b: &mut Body<'_, '_, 'tcx>, coll: Ty<'tcx>, span: Span) -> Expr {
	let cmp = cmp_fn(b, arg(coll, 0), false, span);
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
	args: Vec<Expr>,
	out: &mut Vec<Stmt>,
) -> Option<Expr> {
	let span = call.span;
	let item = arg(coll, 0);
	let mut values = args;
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
		(_, "front_mut") if b.t.needs_place(item) => Expr::rt("dequeEndMut", values),
		(_, "back_mut") if b.t.needs_place(item) => Expr::rt("dequeEndMut", back(values)),
		(_, "get_mut") if b.t.needs_place(item) => Expr::rt("dequeGetMut", values),
		(_, "front" | "front_mut") => Expr::rt("dequeEnd", values),
		(_, "back" | "back_mut") => Expr::rt("dequeEnd", back(values)),
		(_, "get" | "get_mut") => Expr::rt("dequeGet", values),
		(_, "truncate") => Expr::rt("dequeTruncate", values),
		(_, "retain_mut") if b.t.needs_place(item) => Expr::rt("dequeRetainMut", values),
		(_, "retain" | "retain_mut") => Expr::rt("dequeRetain", values),
		(_, "remove") => Expr::rt("dequeRemove", values),
		(_, "insert") => Expr::rt("dequeInsert", values),
		(_, "contains") => {
			values.push(eq_fn(b, peel(b, item), span));
			Expr::rt("dequeContains", values)
		}
		(_, "iter_mut") if b.t.needs_place(item) => Expr::rt("dequeIterMut", values),
		(_, "iter" | "iter_mut") => Expr::rt("dequeIter", values),
		(_, "drain") => {
			let droppers = entry_droppers(b, coll, span);
			Expr::rt("dequeDrain", super::iter::tail(values, droppers))
		}
		(_, "reserve" | "reserve_exact" | "shrink_to_fit") => {
			discard(values, out);
			Expr::undefined()
		}
		("Index" | "IndexMut", "index" | "index_mut") => Expr::rt("dequeIndex", values),
		("Clone", "clone") => {
			values.push(clone_fn(b, item, span));
			Expr::rt("dequeClone", values)
		}
		("Extend", "extend") | ("From", "from") => {
			let (source, iter) = extend_source(b, coll, call, owner, values.pop()?)?;
			let target = match owner {
				"From" => deque_new(b, coll, span),
				_ => values.pop()?,
			};
			// Of a `TrustedLen` iterator, by `for_each`.
			let folds = super::iter::implements(b.t, iter, "TrustedLen");
			let take = super::iter::iter_take(b, iter, folds, span);
			Expr::rt("dequeExtend", super::iter::tail(vec![target, source], vec![take]))
		}
		_ => return None,
	};
	Some(e)
}

/// `PartialOrd`'s tests of `item` that std's heap sifts make: `le` up, and `le`, `ge`, and
/// `lt` down.
fn heap_tests<'tcx>(b: &mut Body<'_, '_, 'tcx>, item: Ty<'tcx>, down: bool, span: Span) -> Vec<Expr> {
	let tests: &[&str] = if down { &["le", "ge", "lt"] } else { &["le"] };
	tests.iter().map(|test| test_fn(b, item, test, span)).collect()
}

/// A method of `BinaryHeap`, or of a trait it implements, each that sifts given the tests
/// its sifts make.
fn heap<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	coll: Ty<'tcx>,
	call: &Call<'tcx>,
	owner: &str,
	name: &str,
	args: Vec<Expr>,
	out: &mut Vec<Stmt>,
) -> Option<Expr> {
	let span = call.span;
	let item = arg(coll, 0);
	let bytes = matches!(item.kind(), ty::Uint(ty::UintTy::U8));
	let mut values = args;
	let e = match (owner, name) {
		(_, "new" | "with_capacity") | ("Default", "default") => {
			discard(values, out);
			empty(b, Expr::rt("heapNew", vec![]), coll, span)
		}
		(_, "len") => values.pop()?.member("d").member("length"),
		(_, "is_empty") => values.pop()?.member("d").member("length").binary("===", Expr::lit("0")),
		(_, "peek") => values.pop()?.member("d").index(Expr::lit("0")),
		(_, "push" | "pop" | "peek_mut" | "append" | "retain" | "into_sorted_vec") => {
			values.extend(heap_tests(b, item, !matches!(name, "push" | "pop"), span));
			let f = match name {
				"push" => "heapPush",
				"pop" => "heapPop",
				"peek_mut" => "heapPeekMut",
				"append" => "heapAppend",
				"retain" => "heapRetain",
				_ => return Some(heap_vec(Expr::rt("heapIntoSortedVec", values), bytes)),
			};
			Expr::rt(f, values)
		}
		(_, "into_vec") => heap_vec(values.pop()?.member("d"), bytes),
		(_, "as_slice") if bytes => Expr::rt(
			"byteSliceFrom",
			vec![
				Expr::ident("Uint8Array")
					.member("from")
					.call(vec![values.pop()?.member("d")]),
			],
		),
		(_, "as_slice") => values.pop()?.member("d"),
		(_, "clear") => Expr::rt("heapClear", values),
		(_, "drain") => {
			let droppers = entry_droppers(b, coll, span);
			Expr::rt("heapDrain", super::iter::tail(values, droppers))
		}
		(_, "iter") => {
			let by_ref = Ty::new_imm_ref(b.t.tcx, b.t.tcx.lifetimes.re_erased, coll);
			into_iter(b, by_ref, values.pop()?, span)?
		}
		(_, "reserve" | "reserve_exact" | "shrink_to_fit" | "shrink_to") => {
			discard(values, out);
			Expr::undefined()
		}
		("Clone", "clone") => {
			let clone = clone_fn(b, item, span);
			Expr::rt("heapClone", super::iter::tail(values, vec![clone]))
		}
		// `From<Vec<T>>` rebuilds the vector in place, and `From<[T; N]>` collects the array
		// into one first.
		("From", "from") if b.t.std_of(library::targ(call.args, 1)) == Some(Std::Vec) => {
			let v = values.pop()?;
			let d = match bytes {
				true => Expr::ident("Array")
					.member("from")
					.call(vec![Expr::rt("byteVecView", vec![v])]),
				false => v,
			};
			Expr::rt("heapFrom", [vec![d], heap_tests(b, item, true, span)].concat())
		}
		("Extend", "extend") | ("From", "from") => {
			let (source, iter) = extend_source(b, coll, call, owner, values.pop()?)?;
			if owner == "From" {
				let d = super::iter::to_array(b, source, iter, item, span);
				return Some(Expr::rt(
					"heapFrom",
					[vec![d], heap_tests(b, item, true, span)].concat(),
				));
			}
			// Into its vector, of a `TrustedLen` iterator by `for_each`.
			let folds = super::iter::implements(b.t, iter, "TrustedLen");
			let take = super::iter::iter_take(b, iter, folds, span);
			let mut args = vec![values.pop()?, source];
			args.extend(heap_tests(b, item, true, span));
			Expr::rt("heapExtend", super::iter::tail(args, vec![take]))
		}
		_ => return None,
	};
	Some(e)
}

/// A `Vec` of a heap's elements `d`, an `rt.ByteVec` of bytes.
pub fn heap_vec(d: Expr, bytes: bool) -> Expr {
	match bytes {
		true => Expr::rt(
			"byteVecFrom",
			vec![Expr::ident("Uint8Array").member("from").call(vec![d])],
		),
		false => d,
	}
}

/// What `extend` or `from` adds to a collection: an `rt.Iter` of what `source` holds, and
/// the type of the iterator std's makes of it.
fn extend_source<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	coll: Ty<'tcx>,
	call: &Call<'tcx>,
	owner: &str,
	source: Expr,
) -> Option<(Expr, Ty<'tcx>)> {
	// `Extend<A>::extend::<T>` and `From<T>::from`.
	let ty = library::targ(call.args, if owner == "Extend" { 2 } else { 1 });
	let iter = super::iter::iter_of(b.t, ty)?;
	let Some(items) = super::iter::into_iter(b, ty, source, call.span) else {
		return Some((
			b.err(call.span, format!("`{owner}` from `{ty}`, which is not supported yet")),
			iter,
		));
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
	if !borrowed || !elems.iter().any(|&t| b.t.needs_copy(t)) {
		return Some((items, iter));
	}
	let items = match b.t.copy_fn(stored, call.span) {
		Some(copy) => Expr::rt("iterMap", vec![items, copy]),
		None => items,
	};
	Some((items, iter))
}

/// A method of an `Entry`, `OccupiedEntry`, or `VacantEntry`.
fn entry<'tcx>(b: &mut Body<'_, '_, 'tcx>, coll: Ty<'tcx>, name: &str, args: Vec<Expr>, span: Span) -> Option<Expr> {
	let value = arg(coll, 1);
	let mut values = args;
	// What these return a `&mut` to is a place where JS would copy it.
	let place = b.t.needs_place(value);
	let mutable = |f: &str| match place {
		true => format!("{f}Mut"),
		false => f.to_string(),
	};
	let e = match name {
		"or_insert" => Expr::rt(mutable("entryOrInsert"), values),
		// A vacant entry's.
		"insert" if is_vacant(b, coll) => Expr::rt(mutable("entryOrInsert"), values),
		"insert" => Expr::rt("entryInsert", values),
		"or_insert_with" | "or_insert_with_key" => Expr::rt(mutable("entryOrInsertWith"), values),
		"or_default" => {
			let tcx = b.t.tcx;
			let default = trait_fn(tcx, "Default", "default");
			values.push(b.fn_value(default, tcx.mk_args(&[value.into()]), span));
			Expr::rt(mutable("entryOrInsertWith"), values)
		}
		"and_modify" => Expr::rt(mutable("entryAndModify"), values),
		"key" => Expr::rt("entryKey", values),
		"into_key" => values.pop()?.member("k"),
		"get" => Expr::rt("entryGet", values),
		"get_mut" | "into_mut" => Expr::rt(mutable("entryGet"), values),
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
fn ordering(name: &str, mut values: Vec<Expr>) -> Option<Expr> {
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
fn hasher_call(b: &mut Body<'_, '_, '_>, name: &str, args: Vec<Expr>, span: Span, out: &mut Vec<Stmt>) -> Expr {
	let mut values = args;
	// The runtime takes the value first, so a hasher that is not a path evaluates first.
	if let [state, value] = values.as_mut_slice()
		&& !state.is_path()
		&& !value.is_constant()
	{
		let saved = std::mem::replace(state, Expr::undefined());
		*state = b.spill(saved, "state", out);
	}
	let swapped = |values: Vec<Expr>| values.into_iter().rev().collect::<Vec<_>>();
	match name {
		"finish" => Expr::rt("hashFinish", values),
		"write_str" => Expr::rt("hashStr", swapped(values)),
		n if n.starts_with("write_") => Expr::rt("hashInt", swapped(values)),
		_ => b.err(span, format!("`Hasher::{name}`, which is not supported yet")),
	}
}

/// The discriminant of `value`, an enum of the crate, as `discriminant_value` reads it.
fn discriminant<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, value: Expr, span: Span) -> Expr {
	let tcx = b.t.tcx;
	let ty = ty.peel_refs();
	let ty::Adt(adt, _) = ty.kind() else {
		return b.err(span, format!("the discriminant of `{ty}`"));
	};
	if !adt.is_enum() || !b.t.own(adt.did()) {
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
/// `Hash` do, through what it holds.
fn std_key<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> bool {
	// What a std type holds may be the crate's, which compares and hashes by its own impls.
	let held = |t: Ty<'tcx>| {
		let t = peel(b, t);
		std_key(b, t) || b.t.deferred(t) || matches!(t.kind(), ty::Adt(adt, _) if b.t.own(adt.did()))
	};
	match ty.kind() {
		// A float is no key, but its `PartialEq` and `PartialOrd` are the runtime's.
		ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str => true,
		ty::Tuple(tys) => !tys.is_empty() && tys.iter().all(held),
		_ if let Some(elem) = elements_of(b, ty) => held(elem),
		ty::Adt(..) if let Some(inner) = option_of(b, ty).or_else(|| arc_of(b, ty)).or_else(|| reverse_of(b, ty)) => {
			held(inner)
		}
		ty::Adt(..) => library::string(b.t.tcx, ty) || matches!(b.t.std_of(ty), Some(Std::Ordering | Std::Duration)),
		_ => false,
	}
}

/// What `ty` holds if it is an `Option`.
fn option_of<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
	(b.t.std_of(ty) == Some(Std::Option)).then(|| arg(ty, 0))
}

/// What `ty` holds if it is a `Reverse`.
fn reverse_of<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
	(b.t.std_of(ty) == Some(Std::Reverse)).then(|| arg(ty, 0))
}

/// The elements of `ty` if it is a `Vec`, a slice, or an array that is a JS array: of any
/// type but `u8`, whose bytes are typed arrays.
fn elements_of<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
	let elem = match ty.kind() {
		ty::Slice(elem) | ty::Array(elem, _) => *elem,
		ty::Adt(..) if b.t.std_of(ty) == Some(Std::Vec) => arg(ty, 0),
		_ => return None,
	};
	(!matches!(elem.kind(), ty::Uint(ty::UintTy::U8))).then_some(elem)
}

/// What `ty` holds if it is an `Arc`.
fn arc_of<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
	(b.t.std_of(ty) == Some(Std::Arc)).then(|| arg(ty, 0))
}

/// `Ord::cmp` of `ty` as a function, or `PartialOrd::partial_cmp` if `partial`: the
/// runtime's for a std type, which calls the same of what it holds, or the crate's.
pub fn cmp_fn<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, partial: bool, span: Span) -> Expr {
	let tcx = b.t.tcx;
	let ty = peel(b, ty);
	match ty.kind() {
		ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) => Expr::Rt("cmp".into()),
		// `None` for a NaN: only `PartialOrd`, since a float is no `Ord`.
		ty::Float(_) if partial => Expr::Rt("f64PartialCmp".into()),
		ty::Str => Expr::Rt("cmpStr".into()),
		ty::Adt(..) if library::string(tcx, ty) => Expr::Rt("cmpStr".into()),
		ty::Adt(..) if b.t.std_of(ty) == Some(Std::Ordering) => Expr::Rt("cmp".into()),
		ty::Adt(..) if b.t.std_of(ty) == Some(Std::Duration) => Expr::Rt("durationCmp".into()),
		ty::Tuple(tys) if !tys.is_empty() => {
			let fns = tys.iter().map(|t| cmp_fn(b, t, partial, span)).collect();
			Expr::rt("cmpTuple", vec![Expr::Array(fns)])
		}
		ty::Adt(..) if let Some(inner) = option_of(b, ty) => {
			let cmp = cmp_fn(b, inner, partial, span);
			Expr::rt("cmpOption", [cmp].into_iter().chain(b.t.none_arg(inner)).collect())
		}
		ty::Adt(..) if let Some(inner) = arc_of(b, ty) => Expr::rt("cmpArc", vec![cmp_fn(b, inner, partial, span)]),
		ty::Adt(..) if let Some(inner) = reverse_of(b, ty) => {
			Expr::rt("cmpReverse", vec![cmp_fn(b, inner, partial, span)])
		}
		_ if let Some(elem) = elements_of(b, ty) => Expr::rt("cmpSlice", vec![cmp_fn(b, elem, partial, span)]),
		ty::Adt(adt, _) if b.t.own(adt.did()) && partial => {
			let cmp = trait_fn(tcx, "PartialOrd", "partial_cmp");
			b.fn_value(cmp, tcx.mk_args(&[ty.into(), ty.into()]), span)
		}
		ty::Adt(adt, _) if b.t.own(adt.did()) => {
			let cmp = trait_fn(tcx, "Ord", "cmp");
			b.fn_value(cmp, tcx.mk_args(&[ty.into()]), span)
		}
		// Each instance decides what its type parameters are.
		_ if b.t.deferred(ty) => Expr::undefined(),
		_ => b.err(span, format!("ordering `{ty}`, which is not supported yet")),
	}
}

/// `PartialEq::eq` of `ty`, as a function: the runtime's for a std type, or the crate's.
pub fn eq_fn<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span) -> Expr {
	let tcx = b.t.tcx;
	let ty = peel(b, ty);
	match ty.kind() {
		ty::Int(_) | ty::Uint(_)
			if let Some(int) = Int::of(ty)
				&& int.big() =>
		{
			int.func("eq")
		}
		ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str => Expr::Rt("same".into()),
		ty::Adt(..) if library::string(tcx, ty) || b.t.std_of(ty) == Some(Std::Ordering) => Expr::Rt("same".into()),
		ty::Adt(..) if b.t.std_of(ty) == Some(Std::Duration) => Expr::Rt("durationEq".into()),
		ty::Tuple(tys) if !tys.is_empty() => {
			let fns = tys.iter().map(|t| eq_fn(b, t, span)).collect();
			Expr::rt("eqTuple", vec![Expr::Array(fns)])
		}
		ty::Adt(..) if let Some(inner) = option_of(b, ty) => match eq_fn(b, inner, span) {
			// `None` is no value of `inner`, so `===` tells it from each.
			Expr::Rt(same) if same == "same" => Expr::Rt(same),
			eq => Expr::rt("eqOption", [eq].into_iter().chain(b.t.none_arg(inner)).collect()),
		},
		ty::Adt(..) if let Some(inner) = reverse_of(b, ty) => eq_fn(b, inner, span),
		// A slice compares its elements by `!=`, which `!eq` stands for only while that is
		// the default.
		_ if let Some(elem) = elements_of(b, ty)
			&& own_ne(b, elem) =>
		{
			b.err(
				span,
				format!(
					"comparing `{ty}` by `<{elem} as PartialEq>::ne` through a held type's own, which is not supported yet"
				),
			)
		}
		_ if let Some(elem) = elements_of(b, ty) => Expr::rt("eqSlice", vec![eq_fn(b, elem, span)]),
		ty::Adt(..) if let Some(inner) = arc_of(b, ty) => {
			let mut values = vec![eq_fn(b, inner, span)];
			// std's `Arc` finds itself equal before comparing a value that is `Eq`.
			if super::iter::implements(b.t, inner, "Eq") {
				values.push(Expr::lit("true"));
			}
			Expr::rt("eqArc", values)
		}
		ty::Adt(adt, _) if b.t.own(adt.did()) => {
			let eq = trait_fn(tcx, "PartialEq", "eq");
			b.fn_value(eq, tcx.mk_args(&[ty.into(), ty.into()]), span)
		}
		_ if b.t.deferred(ty) => Expr::undefined(),
		_ => b.err(span, format!("comparing `{ty}` as a key, which is not supported yet")),
	}
}

/// `Hash::hash` of `ty` into a `hasher`, as a function: the runtime's for a std type, or
/// the crate's.
fn hash_fn<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, hasher: Ty<'tcx>, span: Span) -> Expr {
	let tcx = b.t.tcx;
	let ty = peel(b, ty);
	match ty.kind() {
		ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) => Expr::Rt("hashInt".into()),
		ty::Str => Expr::Rt("hashStr".into()),
		ty::Adt(..) if library::string(tcx, ty) => Expr::Rt("hashStr".into()),
		ty::Adt(..) if b.t.std_of(ty) == Some(Std::Ordering) => Expr::Rt("hashInt".into()),
		ty::Adt(..) if b.t.std_of(ty) == Some(Std::Duration) => Expr::Rt("durationHash".into()),
		ty::Tuple(tys) if !tys.is_empty() => {
			let fns = tys.iter().map(|t| hash_fn(b, t, hasher, span)).collect();
			Expr::rt("hashTuple", vec![Expr::Array(fns)])
		}
		ty::Adt(..) if let Some(inner) = option_of(b, ty) => {
			let hash = hash_fn(b, inner, hasher, span);
			Expr::rt("hashOption", [hash].into_iter().chain(b.t.none_arg(inner)).collect())
		}
		ty::Adt(..) if let Some(inner) = arc_of(b, ty) => Expr::rt("hashArc", vec![hash_fn(b, inner, hasher, span)]),
		ty::Adt(..) if let Some(inner) = reverse_of(b, ty) => hash_fn(b, inner, hasher, span),
		_ if let Some(elem) = elements_of(b, ty) => {
			if own_hash_slice(b, elem, hasher) {
				let msg =
					format!("hashing `{ty}` through `{elem}`'s own `Hash::hash_slice`, which is not supported yet");
				return b.err(span, msg);
			}
			Expr::rt("hashSlice", vec![hash_fn(b, elem, hasher, span)])
		}
		ty::Adt(adt, _) if b.t.own(adt.did()) => {
			let hash = trait_fn(tcx, "Hash", "hash");
			b.fn_value(hash, tcx.mk_args(&[ty.into(), hasher.into()]), span)
		}
		_ if b.t.deferred(ty) => Expr::undefined(),
		_ => b.err(span, format!("hashing `{ty}` as a key, which is not supported yet")),
	}
}

/// Whether the crate's own `Hash` of `elem` overrides `hash_slice`, which a slice of it
/// hashes through in place of each element's `hash`.
fn own_hash_slice<'tcx>(b: &Body<'_, '_, 'tcx>, elem: Ty<'tcx>, hasher: Ty<'tcx>) -> bool {
	let tcx = b.t.tcx;
	let elem = peel(b, elem);
	let ty::Adt(adt, _) = elem.kind() else {
		return false;
	};
	let f = trait_fn(tcx, "Hash", "hash_slice");
	b.t.own(adt.did())
		&& Instance::try_resolve(tcx, b.t.typing_env_pub(), f, tcx.mk_args(&[elem.into(), hasher.into()]))
			.ok()
			.flatten()
			.is_some_and(|instance| instance.def_id() != f)
}

/// `Clone::clone` of `ty`, as a function, or `undefined` where it is the value itself.
pub fn clone_fn<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span) -> Expr {
	let tcx = b.t.tcx;
	if library::clone_copies(b.t, ty)
		&& let Some(copy) = b.t.copy_fn(ty, span)
	{
		return copy;
	}
	if (b.t.is_copy(ty) && library::clone_copies(b.t, ty)) || library::string(tcx, ty) {
		return Expr::undefined();
	}
	let clone = trait_fn(tcx, "Clone", "clone");
	b.fn_value(clone, tcx.mk_args(&[ty.into()]), span)
}

/// An item of one of std's traits, by the trait's diagnostic name: `Ord`'s `cmp`.
pub fn trait_fn(tcx: ty::TyCtxt<'_>, tr: &str, item: &str) -> rustc_hir::def_id::DefId {
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
/// `rt.Iter` of its entries, keys, or items, which by value drops those it still holds.
pub fn into_iter<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, value: Expr, span: Span) -> Option<Expr> {
	let coll = ty.peel_refs();
	// A `&mut` lends each value JS copies as its place.
	let places = |n: usize| ty.ref_mutability() == Some(ty::Mutability::Mut) && b.t.needs_place(arg(coll, n));
	let name = match b.t.std_of(coll)? {
		Std::HashMap if places(1) => "hashIterMut",
		Std::BTreeMap if places(1) => "btreeIterMut",
		Std::VecDeque if places(0) => "dequeIterMut",
		// A heap's elements in order, as its `Vec`'s, which by reference are borrowed.
		Std::BinaryHeap if ty.is_ref() => {
			return Some(super::iter::by_ref(Expr::rt("heapIter", vec![value]), arg(coll, 0)));
		}
		Std::BinaryHeap => "heapIter",
		Std::HashMap => "hashIter",
		Std::HashSet => "hashKeys",
		Std::BTreeMap => "btreeIter",
		Std::BTreeSet => "btreeKeys",
		Std::VecDeque => "dequeIter",
		_ => return None,
	};
	let droppers = match ty.is_ref() {
		true => Vec::new(),
		false => entry_droppers(b, coll, span),
	};
	Some(Expr::rt(name, super::iter::tail(vec![value], droppers)))
}

/// `collect` into a collection of type `ty` from `iter`, an iterator of type `from`.
pub fn collect<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, iter: Expr, from: Ty<'tcx>, span: Span) -> Option<Expr> {
	let std = b.t.std_of(ty)?;
	if let Some(elem) = undefined_arg(b, std, ty) {
		return Some(b.err(
			span,
			format!("`{ty}` holds `{elem}`, which is `undefined` as `None` is, which is not supported"),
		));
	}
	Some(match std {
		Std::HashMap | Std::HashSet => {
			let target = hash_new(b, ty, span)?;
			hash_extend(b, ty, target, iter, from, span)
		}
		Std::BTreeMap | Std::BTreeSet => btree_collect(b, ty, iter, from, span),
		// std's collects into a `Vec`, and rebuilds it in place.
		Std::BinaryHeap => {
			let item = arg(ty, 0);
			let d = super::iter::to_array(b, iter, from, item, span);
			Expr::rt("heapFrom", [vec![d], heap_tests(b, item, true, span)].concat())
		}
		// std's collects into a `Vec` first.
		Std::VecDeque => {
			let target = deque_new(b, ty, span);
			let item = arg(ty, 0);
			if !b.t.drops(item) {
				let folds = super::iter::implements(b.t, from, "TrustedLen");
				let take = super::iter::iter_take(b, from, folds, span);
				Expr::rt("dequeExtend", super::iter::tail(vec![target, iter], vec![take]))
			} else {
				let items = super::iter::to_array(b, iter, from, item, span);
				Expr::rt("dequeExtend", vec![target, Expr::rt("iterSlice", vec![items])])
			}
		}
		_ => return None,
	})
}
