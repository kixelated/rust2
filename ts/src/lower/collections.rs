//! Collections, as `rt/collections.ts` gives them: `HashMap`, `HashSet`, `BTreeMap`,
//! `BTreeSet`, and `VecDeque`, with `Ordering` and `DefaultHasher`, which their keys need.
//!
//! A hash collection holds a key that is a JS primitive as its own JS map key, which JS
//! compares as Rust's `Eq` does for numbers, strings, and booleans, and finds any other
//! key through its translated or bound `Hash` and `Eq`, which `rt.hashing` makes it carry.
//! A B-tree orders every key by its translated or bound `Ord`. Each value a collection
//! stores is its own copy, as in Rust.

use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, GenericArgsRef, Instance, Ty};
use rustc_span::Span;

use super::Translator;
use super::body::Body;
use super::library::{self, Call, Std};
use super::num::Int;
use crate::ts::{Expr, Param, Stmt, StmtKind};

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
			| ("From", "from", Some(std), Some(t))
			| ("BitOr" | "BitAnd" | "Sub" | "BitXor", _, Some(std), Some(t)) => (std, t.peel_refs()),
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
				// The library maps the integers', `bool`'s, and byte slices'.
				let rhs = library::targ(call.args, 1);
				let slice = |t: Ty<'tcx>| matches!(t.kind(), ty::Slice(_)) && bytes(b, t);
				if slice(t) && slice(rhs) {
					return Err(args);
				}
				let t = peel(b, t);
				// Bytes of any form against any other, as the runtime reads each.
				if bytes(b, t) && bytes(b, peel(b, rhs)) {
					let eq = Expr::rt("eqBytes", args);
					return Ok(Some(if name == "eq" { eq } else { eq.not() }));
				}
				let kind = b.t.std_of(t) == Some(Std::ErrorKind);
				if matches!(t.kind(), ty::Bool | ty::Int(_) | ty::Uint(_))
					|| bytes(b, t)
					|| !(std_key(b, t) || kind)
					|| !alike(b, t, rhs)
				{
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
			("BuildHasher", _, Some(Std::RandomState), Some(t)) => (Std::RandomState, t.peel_refs()),
			// The discriminant derived `Hash` and `Ord` read of an enum.
			("intrinsics", "discriminant_value", ..) if let Some(t) = self_ty => {
				let value = args.pop();
				return Ok(value.map(|value| discriminant(b, t, value, span)));
			}
			_ => return Err(args),
		},
	};
	let by = lookup_ty(b, std, call, owner, name);
	// A `[u8; N]` key is a typed array, but a reference to an array or a slice, as a lookup
	// takes one, is an `rt.ByteSlice`, so a view of its bytes stands in. A range holds the
	// keys themselves.
	if let Some(by) = by
		&& !matches!(name, "range" | "range_mut")
		&& matches!(by.kind(), ty::Array(..) | ty::Slice(_))
		&& bytes(b, by)
	{
		// The key as held, through boxes and `Arc`s: a reference to an array is a slice already.
		let mut key = arg(coll, 0);
		while let Some(Std::Box | Std::Arc) = b.t.std_of(key) {
			key = arg(key, 0);
		}
		if let (true, Some(at)) = (super::byte_array(key), args.get_mut(1)) {
			*at = Expr::rt("byteSliceView", vec![std::mem::replace(at, Expr::undefined())]);
		}
	}
	// Bytes of any form hash and compare alike, so each finds the others.
	let alike = |key: Ty<'tcx>, by: Ty<'tcx>| same_key(b, key, by) || (bytes(b, peel(b, key)) && bytes(b, peel(b, by)));
	if let Some(by) = by
		&& !alike(arg(coll, 0), by)
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
	// What std hands a closure of the bytes the collection holds, lent as it returns them.
	if lending(std) {
		let inputs = b.t.sig(call.def_id, call.args).inputs().to_vec();
		for (value, input) in args.iter_mut().zip(inputs).skip(1) {
			if let Some((params, _)) = closure_sig(tcx, input) {
				let f = std::mem::replace(value, Expr::undefined());
				*value = lend_closure(b, f, &params, span, out);
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
		Std::RandomState => Some(random_state(b, call, coll, name, args)),
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
	// An `Option` of an element, key, or value that can be `undefined` tells `None` apart.
	if library::none_last(owner, name)
		&& let Some(none) = library::sentinel_option(b.t, call).and_then(|inner| b.t.none_arg(inner))
	{
		droppers.resize(lost.len(), Expr::undefined());
		droppers.push(none);
	}
	let e = match e {
		Expr::Call(f, mut args) if !droppers.is_empty() => {
			args.extend(droppers);
			Expr::Call(f, args)
		}
		e => e,
	};
	Ok(Some(match lending(std) {
		true => lend(b, b.t.sig(call.def_id, call.args).output(), e, span),
		false => e,
	}))
}

/// Whether the mappings of a collection of type `std` hand out the bytes it holds as
/// [`lend`] makes them: in what each returns and hands a closure.
fn lending(std: Std) -> bool {
	matches!(
		std,
		Std::HashMap | Std::HashSet | Std::BTreeMap | Std::BTreeSet | Std::Entry | Std::VecDeque | Std::BinaryHeap
	)
}

/// Whether `def_id`, a method of a collection [`lending`] the bytes it holds, takes and
/// returns them only where its mapping and [`lend`] make each what translated code reads:
/// a `[u8; N]` it holds is a `Uint8Array`, where a `&[u8; N]` is an `rt.ByteSlice`. `None`
/// if it is no such method.
pub fn lends_call<'tcx>(t: &Translator<'_, 'tcx>, def_id: DefId, args: GenericArgsRef<'tcx>) -> Option<bool> {
	let tcx = t.tcx;
	// An inherent method's type, or a trait method's `Self`.
	let coll = match tcx.inherent_impl_of_assoc(def_id) {
		Some(imp) => {
			let generics = tcx.generics_of(imp).count();
			tcx.type_of(imp).instantiate(tcx, &args[..generics]).skip_norm_wip()
		}
		None => args.types().next()?.peel_refs(),
	};
	let ty::Adt(_, held) = coll.kind() else {
		return None;
	};
	if !t.std_of(coll).is_some_and(lending) {
		return None;
	}
	// One it holds as a reference to bytes, which a reference to one it holds is too.
	if held.types().any(holds_bytes_ref) {
		return Some(false);
	}
	let name = tcx.item_name(def_id);
	let other_set =
		set_op("", name.as_str()).is_some() || matches!(name.as_str(), "bitor" | "bitand" | "sub" | "bitxor");
	let sig = t.sig(def_id, args);
	let taken = sig.inputs().iter().skip(1).all(|&input| match closure_sig(tcx, input) {
		Some((params, output)) => params.iter().all(|&p| lends(t, p).is_some()) && !holds_bytes_ref(output),
		// A key looked up or compared by a reference to its bytes, which the runtime reads in
		// either form, the other set of a set operation, which it reads as it holds, or what
		// holds none, nor yields any as `extend` takes what it stores.
		None if other_set && input.peel_refs() == coll => true,
		None => {
			let yielded = super::iter::iter_of(t, input).and_then(|iter| t.iter_item(iter));
			super::bytes_ref(input)
				|| (lends(t, input) == Some(false) && yielded.is_none_or(|item| lends(t, item) == Some(false)))
		}
	});
	Some(taken && lends(t, sig.output()).is_some())
}

/// Whether a `ty` a collection hands out holds a `&[u8; N]` of bytes it holds: `Some(true)`
/// if it does, by itself or in a tuple, an `Option`, or an iterator's items, which [`lend`]
/// reaches, `Some(false)` if it holds none, and `None` if it holds one [`lend`] cannot reach.
fn lends<'tcx>(t: &Translator<'_, 'tcx>, ty: Ty<'tcx>) -> Option<bool> {
	let parts = match ty.kind() {
		ty::Ref(_, inner, _) if super::byte_array(*inner) => return Some(true),
		// A reference to a `&[u8; N]` it holds, which is the `rt.ByteSlice` already.
		ty::Ref(_, inner, _) if super::bytes_ref(*inner) => return Some(false),
		ty::Tuple(parts) => parts.to_vec(),
		_ if t.std_of(ty) == Some(Std::Option) => vec![arg(ty, 0)],
		_ => match t.iter_item(ty) {
			Some(item) => vec![item],
			None => return (!holds_bytes_ref(ty)).then_some(false),
		},
	};
	parts
		.into_iter()
		.try_fold(false, |any, part| Some(lends(t, part)? || any))
}

/// Whether `ty` holds a `&[u8; N]`, as a closure's signature does if it takes one.
pub fn holds_bytes_ref(ty: Ty<'_>) -> bool {
	ty.walk().any(|arg| arg.as_type().is_some_and(super::bytes_ref))
}

/// The parameter types and output of a closure or function `ty`.
fn closure_sig<'tcx>(tcx: ty::TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<(Vec<Ty<'tcx>>, Ty<'tcx>)> {
	let (sig, tupled) = match ty.kind() {
		ty::Closure(_, args) => (args.as_closure().sig(), true),
		ty::FnDef(..) | ty::FnPtr(..) => (ty.fn_sig(tcx), false),
		_ => return None,
	};
	let sig = tcx.instantiate_bound_regions_with_erased(sig);
	let params = match (tupled, sig.inputs()) {
		(true, [params]) => params.tuple_fields().to_vec(),
		(_, params) => params.to_vec(),
	};
	Some((params, sig.output()))
}

/// `value`, a `ty` as a collection's runtime function hands it out, as translated code
/// reads it: each `&[u8; N]` in it, of bytes the collection holds as a `Uint8Array`, an
/// `rt.ByteSlice` of them, as `&[u8; N]` is elsewhere. It shares them, which nothing
/// writes through a `&`, and a `&mut` writes where the collection holds them.
fn lend<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, value: Expr, span: Span) -> Expr {
	if lends(b.t, ty) != Some(true) {
		return value;
	}
	if let Some(inner) = option_of(b, ty) {
		let f = lend_fn(b, inner, span);
		return Expr::rt("optionMap", vec![value, f]);
	}
	if !matches!(ty.kind(), ty::Ref(..) | ty::Tuple(_))
		&& let Some(item) = b.t.iter_item(ty)
	{
		let f = lend_fn(b, item, span);
		return Expr::rt("iterMap", vec![value, f]);
	}
	if let ty::Tuple(parts) = ty.kind()
		&& value.is_path()
	{
		let parts: Vec<Expr> = (parts.iter().enumerate())
			.map(|(i, part)| lend(b, part, value.clone().index(Expr::lit(i.to_string())), span))
			.collect();
		return Expr::Array(parts);
	}
	lend_fn(b, ty, span).call(vec![value])
}

/// A function that [`lend`]s a `ty`.
fn lend_fn<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span) -> Expr {
	if matches!(ty.kind(), ty::Ref(..)) && lends(b.t, ty) == Some(true) {
		return Expr::Rt("byteSliceFrom".into());
	}
	let name = b.fresh("held");
	let body = lend(b, ty, Expr::ident(&name), span);
	let param = Param { name, ty: None };
	Expr::Arrow(
		vec![param],
		b.t.ty(ty, span),
		vec![StmtKind::Return(Some(body)).at(None)],
	)
}

/// `f`, a closure std calls with `params`, given each as [`lend`] makes it.
fn lend_closure<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	f: Expr,
	params: &[Ty<'tcx>],
	span: Span,
	out: &mut Vec<Stmt>,
) -> Expr {
	if !params.iter().any(|&p| lends(b.t, p) == Some(true)) {
		return f;
	}
	// Made once, as std takes it.
	let f = match f.is_path() {
		true => f,
		false => b.spill(f, "f", out),
	};
	let names: Vec<String> = params.iter().map(|_| b.fresh("held")).collect();
	let values = (params.iter().zip(&names))
		.map(|(&p, name)| lend(b, p, Expr::ident(name), span))
		.collect();
	let params = names.into_iter().map(|name| Param { name, ty: None }).collect();
	Expr::Lambda(params, Box::new(f.call(values)))
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

/// `values` and then `flag`, which a runtime function defaults to `false` unless the
/// `rt.None` it takes last follows, where `none`.
fn flagged(mut values: Vec<Expr>, flag: bool, none: bool) -> Vec<Expr> {
	if flag || none {
		values.push(Expr::lit(flag.to_string()));
	}
	values
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
		("Index", "index") => match library::targ(call.args, 1).kind() {
			ty::Ref(_, by, _) => Some(*by),
			_ => None,
		},
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

/// Whether a `ty` compares to an `other` as to another `ty`: as keys do, as bytes do, or
/// by elements that do. Else the impl of `PartialEq<other>` is its own.
fn alike<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>, other: Ty<'tcx>) -> bool {
	let (ty, other) = (peel(b, ty), peel(b, other));
	same_key(b, ty, other)
		|| (bytes(b, ty) && bytes(b, other))
		|| matches!((elements_of(b, ty), elements_of(b, other)), (Some(t), Some(o)) if alike(b, t, o))
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
		Std::Vec => (&["retain_mut", "dedup_by", "dedup_by_key", "pop_if"], 0),
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
	let none = library::sentinel_option(b.t, call).is_some();
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
		(_, "get") => Expr::rt("hashKey", flagged(values, false, none)),
		(_, "take") => Expr::rt("hashKey", flagged(values, true, none)),
		(_, "replace") => Expr::rt("hashReplace", values),
		(_, "get_key_value") => Expr::rt("hashGetKeyValue", values),
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
		// What drops the half it yields, then the other, which the runtime drops as std's
		// does: before `next` returns, and after `fold` has the item.
		(_, "into_keys" | "into_values") => {
			let (kept, other) = if name == "into_keys" { (0, 1) } else { (1, 0) };
			let Some(droppers) = b.t.droppers([arg(coll, kept), arg(coll, other)], span) else {
				return Some(Expr::undefined());
			};
			let f = if name == "into_keys" { "hashKeys" } else { "hashValues" };
			Expr::rt(f, super::iter::tail(values, droppers))
		}
		(_, "drain") => {
			let droppers = entry_droppers(b, coll, span);
			Expr::rt(
				if set { "hashDrainKeys" } else { "hashDrain" },
				super::iter::tail(values, droppers),
			)
		}
		(_, "entry") => Expr::rt("entry", values),
		(_, name) if let Some(f) = set_op("hash", name) => Expr::rt(f, values),
		("BitOr" | "BitAnd" | "Sub" | "BitXor", _) => {
			let keys = set_op_keys(b, coll, "hash", owner, values, span)?;
			let target = hash_new(b, coll, span)?;
			let max = hash_max(b, coll, span);
			Expr::rt(
				"hashExtendKeys",
				vec![target, keys, Expr::undefined(), Expr::undefined(), max],
			)
		}
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

/// The runtime's function of a set's operation or test `name`, of the runtime's `prefix`
/// for its type: `hashUnion` of a `HashSet`'s `union`.
fn set_op(prefix: &str, name: &str) -> Option<String> {
	let op = match name {
		"union" => "Union",
		"intersection" => "Intersection",
		"difference" => "Difference",
		"symmetric_difference" => "SymmetricDifference",
		"is_subset" => "IsSubset",
		"is_superset" => "IsSuperset",
		"is_disjoint" => "IsDisjoint",
		_ => return None,
	};
	Some(format!("{prefix}{op}"))
}

/// What the set operator of trait `owner` builds a set of type `coll` from, as std's: its
/// operation of the two sets in `values`, each key cloned.
fn set_op_keys<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	coll: Ty<'tcx>,
	prefix: &str,
	owner: &str,
	values: Vec<Expr>,
	span: Span,
) -> Option<Expr> {
	let name = match owner {
		"BitOr" => "union",
		"BitAnd" => "intersection",
		"Sub" => "difference",
		_ => "symmetric_difference",
	};
	let keys = Expr::rt(set_op(prefix, name)?, values);
	let clone = clone_fn(b, arg(coll, 0), span);
	Some(match clone == Expr::undefined() {
		true => keys,
		false => Expr::rt("iterMap", vec![keys, clone]),
	})
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
	droppers.push(hash_max(b, coll, span));
	let f = match b.t.std_of(coll) {
		Some(Std::HashSet) => "hashExtendKeys",
		_ => "hashExtend",
	};
	Expr::rt(f, super::iter::tail(vec![target, source], droppers))
}

/// The most entries a hash collection of type `coll` holds on wasm32, past which hashbrown's
/// `reserve` panics: what its table of `(K, V)`, or `(K, ())`, takes as `capacity_to_buckets`
/// and `calculate_layout_for` allow, with 8-byte groups.
fn hash_max<'tcx>(b: &mut Body<'_, '_, 'tcx>, coll: Ty<'tcx>, span: Span) -> Expr {
	let (size, align) = match super::iter::layout(b, stored(b, coll), span) {
		Ok(layout) => layout,
		Err(e) => return e,
	};
	const GROUP: u64 = 8;
	let ctrl_align = align.max(GROUP);
	let fits = |cap: u64| -> bool {
		if cap == 0 {
			return true;
		}
		let buckets = if cap < 15 {
			let cap = cap.max(if size <= 1 { 7 } else { 3 });
			if cap < 4 {
				4
			} else if cap < 8 {
				8
			} else {
				16
			}
		} else {
			match cap.checked_mul(8).filter(|&n| n <= u64::from(u32::MAX)) {
				Some(n) => (n / 7).next_power_of_two(),
				None => return false,
			}
		};
		let len = (size * buckets).div_ceil(ctrl_align) * ctrl_align + buckets + GROUP;
		len <= u64::from(i32::MAX as u32) - (ctrl_align - 1)
	};
	// The greatest capacity that fits, of which every smaller one does.
	let (mut lo, mut hi) = (0, u64::from(u32::MAX));
	while lo < hi {
		let mid = lo + (hi - lo).div_ceil(2);
		if fits(mid) {
			lo = mid;
		} else {
			hi = mid - 1;
		}
	}
	Expr::lit(lo.to_string())
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
	// A fieldless enum is a JS string, and a bound value may be any primitive, which its
	// own `Eq` may tell apart where JS can't, as is the `Some` of an `Option` of one, and a
	// `Reverse` of one.
	let mut held = key;
	while let Some(inner) = option_of(b, held).or_else(|| reverse_of(b, held)) {
		held = peel(b, inner);
	}
	if matches!(held.kind(), ty::Adt(adt, _) if fieldless(*adt) || b.t.bound_drops(adt.did()).is_some()) {
		values.push(Expr::lit("true"));
	}
	Some(Some(Expr::rt("hashing", values)))
}

fn fieldless(adt: ty::AdtDef<'_>) -> bool {
	adt.is_enum() && adt.variants().iter().all(|v| v.fields.is_empty())
}

/// Whether `ty` compares, orders, and hashes by its own impls, which translate or which its
/// binding's exports stand in for, rather than as the runtime does std's.
fn local<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> bool {
	matches!(ty.kind(), ty::Adt(adt, _) if b.t.own(adt.did()) || b.t.bound_drops(adt.did()).is_some())
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
		_ if local(b, ty) => {
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
		_ if local(b, ty) => {
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
	let ty::Adt(_, args) = coll.kind() else {
		return None;
	};
	// `HashMap<K, V, S, A>` and `HashSet<T, S, A>`.
	let state = args
		.types()
		.nth(if b.t.std_of(coll) == Some(Std::HashSet) { 1 } else { 2 })?;
	built_hasher(b, state)
}

/// The `Hasher` a `BuildHasher` of type `state` builds, if it is `DefaultHasher`.
fn built_hasher<'tcx>(b: &Body<'_, '_, 'tcx>, state: Ty<'tcx>) -> Option<Ty<'tcx>> {
	let tcx = b.t.tcx;
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
	let mut ranged = None;
	if matches!(name, "range" | "range_mut") {
		let range = bounds(b, call, values.pop()?);
		values.push(range.bounds.clone());
		ranged = Some(range);
		// std checks the ends by `==` and `>`, which a key of std's alone does as its `Ord`
		// does, and one holding the crate's own or a bound type by that type's `PartialEq`
		// and `PartialOrd`.
		let target = range_input(b, call).1;
		if target.walk().filter_map(|arg| arg.as_type()).any(|t| local(b, t)) {
			values.push(eq_fn(b, target, span));
			values.push(test_fn(b, target, "gt", span));
		}
	}
	let none = library::sentinel_option(b.t, call).is_some();
	let last = |values: Vec<Expr>| flagged(values, true, none);
	let first = |values: Vec<Expr>| flagged(values, false, none);
	let e = match (owner, name) {
		(_, "new") | ("Default", "default") => btree_new(b, coll, span),
		(_, "len") => Expr::rt("btreeLen", values),
		(_, "is_empty") => Expr::rt("btreeLen", values).binary("===", Expr::lit("0")),
		(_, "clear") => Expr::rt("btreeClear", values),
		(_, "contains_key" | "contains") => Expr::rt("btreeContains", values),
		(_, "get_mut") if !set && b.t.needs_place(arg(coll, 1)) => Expr::rt("btreeGetMut", values),
		(_, "get" | "get_mut") if !set => Expr::rt("btreeGet", values),
		(_, "get") => Expr::rt("btreeKey", first(values)),
		(_, "take") => Expr::rt("btreeKey", last(values)),
		(_, "replace") => Expr::rt("btreeReplace", values),
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
		(_, "pop_first") if set => Expr::rt("btreePopKey", first(values)),
		(_, "pop_last") if set => Expr::rt("btreePopKey", last(values)),
		(_, "pop_first") => Expr::rt("btreePop", values),
		(_, "pop_last") => Expr::rt("btreePop", last(values)),
		(_, "first") => Expr::rt("btreeFirstKey", first(values)),
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
		(_, name) if let Some(f) = set_op("btree", name) => Expr::rt(f, values),
		("BitOr" | "BitAnd" | "Sub" | "BitXor", _) => {
			let keys = set_op_keys(b, coll, "btree", owner, values, span)?;
			let target = btree_new(b, coll, span);
			Expr::rt("btreeFromSorted", vec![target, keys])
		}
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
			let trusted = super::iter::implements(b.t, iter, "TrustedLen");
			btree_collect(b, coll, source, iter, trusted, span)
		}
		_ => return None,
	};
	Some(match ranged {
		Some(ranged) => ranged.after(e),
		None => e,
	})
}

/// `collect` and `from` into an empty B-tree of type `coll`, from `source`, an iterator of
/// type `iter`, which keep the last of equal keys, as std's sorted bulk build does. Of
/// entries that drop, std's collects into a `Vec` first, and then drops each but the last
/// of equal keys in the order a stable sort leaves them. The `Vec` reserves as for a
/// `TrustedLen` iterator if `trusted`.
fn btree_collect<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	coll: Ty<'tcx>,
	source: Expr,
	iter: Ty<'tcx>,
	trusted: bool,
	span: Span,
) -> Expr {
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
		droppers.push(super::iter::size_of(b, stored(b, coll), span));
		droppers.push(Expr::lit(trusted.to_string()));
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
	let entries = super::iter::to_array(b, source, iter, stored(b, coll), trusted, span);
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
		// A binding's, hand-written even where the Rust impl is derived.
		ty::Adt(adt, _) if b.t.bound_drops(adt.did()).is_some() => true,
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
	let none = library::sentinel_option(b.t, call).is_some();
	let back = |values: Vec<Expr>| flagged(values, true, none);
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
		(_, "front" | "front_mut") => Expr::rt("dequeEnd", flagged(values, false, none)),
		(_, "back" | "back_mut") => Expr::rt("dequeEnd", back(values)),
		(_, "get" | "get_mut") => Expr::rt("dequeGet", values),
		(_, "swap_remove_front") => Expr::rt("dequeSwapRemove", flagged(values, false, none)),
		(_, "swap_remove_back") => Expr::rt("dequeSwapRemove", back(values)),
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
			let ranged = bounds(b, call, values.pop()?);
			values.push(ranged.bounds.clone());
			let droppers = entry_droppers(b, coll, span);
			ranged.after(Expr::rt("dequeDrain", super::iter::tail(values, droppers)))
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
			// Of a `TrustedLen` iterator, by `for_each`, reserving its upper bound first.
			let trusted = super::iter::implements(b.t, iter, "TrustedLen");
			let take = super::iter::iter_take(b, iter, trusted, span);
			let reserve = vec![
				take,
				super::iter::size_of(b, item, span),
				Expr::lit(trusted.to_string()),
			];
			Expr::rt("dequeExtend", super::iter::tail(vec![target, source], reserve))
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
		// Indexing returns `undefined` for no element, where an `rt.None` must follow.
		(_, "peek") if library::sentinel_option(b.t, call).is_some() => Expr::rt("heapPeek", values),
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
		(_, "iter") => Expr::rt("heapIter", values),
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
			let trusted = super::iter::implements(b.t, iter, "TrustedLen");
			if owner == "From" {
				let d = super::iter::to_array(b, source, iter, item, trusted, span);
				return Some(Expr::rt(
					"heapFrom",
					[vec![d], heap_tests(b, item, true, span)].concat(),
				));
			}
			// Into its vector, of a `TrustedLen` iterator by `for_each`, reserving as it does.
			let take = super::iter::iter_take(b, iter, trusted, span);
			let mut args = vec![values.pop()?, source];
			args.extend(heap_tests(b, item, true, span));
			let reserve = vec![
				take,
				super::iter::size_of(b, item, span),
				Expr::lit(trusted.to_string()),
			];
			Expr::rt("heapExtend", super::iter::tail(args, reserve))
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

/// A function of `RandomState`, or of a trait it implements: making one, which `Default`
/// does too, its `Clone`, which is itself, and building its hasher, or hashing one value
/// with it as `BuildHasher::hash_one` does, dropping the value.
fn random_state<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	call: &Call<'tcx>,
	ty: Ty<'tcx>,
	name: &str,
	args: Vec<Expr>,
) -> Expr {
	let span = call.span;
	let mut values = args.into_iter();
	match (name, values.next()) {
		("new" | "default", _) => Expr::rt("randomState", vec![]),
		("clone", Some(state)) => state,
		("build_hasher", Some(state)) => Expr::rt("buildHasher", vec![state]),
		// It takes the value, which drops once hashed.
		("hash_one", Some(state)) if let Some(value) = values.next() => {
			let hasher = built_hasher(b, ty).expect("a RandomState builds a DefaultHasher");
			let value_ty = library::targ(call.args, 1);
			let mut args = vec![state, value, hash_fn(b, value_ty, hasher, span)];
			args.extend(b.t.drop_fn(value_ty, span));
			Expr::rt("hashOne", args)
		}
		_ => b.err(span, format!("`RandomState::{name}`, which is not supported yet")),
	}
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
	let bits = match name {
		"finish" => return Expr::rt("hashFinish", values),
		"write_str" => return Expr::rt("hashStr", swapped(values)),
		"write_u8" | "write_i8" => 8,
		"write_u16" | "write_i16" => 16,
		"write_u32" | "write_i32" | "write_usize" | "write_isize" | "write_length_prefix" => 32,
		"write_u64" | "write_i64" => 64,
		"write_u128" | "write_i128" => 128,
		_ => return b.err(span, format!("`Hasher::{name}`, which is not supported yet")),
	};
	hash_int(bits).call(swapped(values))
}

/// `Hash::hash` of an integer of `bits`, which writes its bytes, `usize`'s 32 as on wasm32.
fn hash_int(bits: u32) -> Expr {
	Expr::Rt(format!("hash{bits}"))
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
		std_key(b, t) || b.t.deferred(t) || local(b, t)
	};
	match ty.kind() {
		// A float is no key, but its `PartialEq` and `PartialOrd` are the runtime's.
		ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str => true,
		ty::Tuple(tys) => !tys.is_empty() && tys.iter().all(held),
		_ if let Some(elem) = elements_of(b, ty) => held(elem),
		_ if bytes(b, ty) => true,
		ty::Adt(..) if let Some(inner) = option_of(b, ty).or_else(|| arc_of(b, ty)).or_else(|| reverse_of(b, ty)) => {
			held(inner)
		}
		// Whose derived `PartialEq` lowers from its MIR.
		ty::Adt(_, args) if b.t.std_of(ty) == Some(Std::Result) => args.types().all(held),
		ty::Adt(..) if b.t.std_of(ty).is_some_and(Std::error) => true,
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

/// Whether `ty` is a `Vec`, a slice, or an array of bytes: a typed array, an `rt.ByteSlice`
/// behind a reference, or an `rt.ByteVec`.
fn bytes<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> bool {
	let elem = match ty.kind() {
		ty::Slice(elem) | ty::Array(elem, _) => *elem,
		ty::Adt(..) if b.t.std_of(ty) == Some(Std::Vec) => arg(ty, 0),
		_ => return false,
	};
	matches!(elem.kind(), ty::Uint(ty::UintTy::U8))
}

/// What `bounds` makes of the range a call takes: what an `rt.Bounds` reads, and, for a
/// range that drops, the range and what drops it.
pub struct Ranged {
	pub bounds: Expr,
	dropped: Option<(Expr, String, Expr)>,
}

impl Ranged {
	/// `e`, the call, which drops a range that drops once it returns: std drops it once it
	/// has read its bounds and searched by them, after which it does nothing JS can see
	/// before it returns.
	pub fn after(self, e: Expr) -> Expr {
		let Some((range, name, drop)) = self.dropped else {
			return e;
		};
		let call = Expr::Lambda(vec![Param { name, ty: None }], Box::new(e));
		Expr::rt("dropAfter", vec![range, call, drop])
	}
}

/// Makes `range`, the range `call` takes, what an `rt.Bounds` reads: std's range types
/// and a pair of `Bound`s are already, and any other `RangeBounds` becomes `rt.boundsOf` of
/// it and its `start_bound` and `end_bound`, which the runtime calls where std's does.
pub fn bounds<'tcx>(b: &mut Body<'_, '_, 'tcx>, call: &Call<'tcx>, range: Expr) -> Ranged {
	let tcx = b.t.tcx;
	let span = call.span;
	let (input, target) = range_input(b, call);
	let ty = b.t.sig(call.def_id, call.args).inputs()[input];
	// One that drops is the parameter of what `Ranged::after` calls, so it is read once.
	let (range, dropped) = match b.t.drop_fn(ty, span) {
		Some(drop) => {
			let name = b.fresh("range");
			(Expr::ident(&name), Some((range, name, drop)))
		}
		None => (range, None),
	};
	// Each instance decides for a range it is generic over.
	let bounds = if plain_bounds(b.t, ty) || b.t.deferred(ty) {
		range
	} else {
		let args = tcx.mk_args(&[ty.into(), target.into()]);
		let start = b.fn_value(trait_fn(tcx, "RangeBounds", "start_bound"), args, span);
		let end = b.fn_value(trait_fn(tcx, "RangeBounds", "end_bound"), args, span);
		Expr::rt("boundsOf", vec![range, start, end])
	};
	Ranged { bounds, dropped }
}

/// Whether an `rt.Bounds` reads `ty` as it is: std's range types and a pair of `Bound`s.
pub fn plain_bounds<'tcx>(t: &Translator<'_, 'tcx>, ty: Ty<'tcx>) -> bool {
	match (ty.kind(), t.std_of(ty)) {
		(
			_,
			Some(
				Std::Range
				| Std::RangeInclusive
				| Std::RangeFrom
				| Std::RangeTo
				| Std::RangeToInclusive
				| Std::RangeFull,
			),
		) => true,
		(ty::Tuple(tys), _) => tys.len() == 2 && tys.iter().all(|b| t.std_of(b) == Some(Std::Bound)),
		_ => false,
	}
}

/// Which of its inputs is the range `call` takes, and the `T` of the `RangeBounds<T>` it
/// implements, from the function's own bound on its range's type.
fn range_input<'tcx>(b: &Body<'_, '_, 'tcx>, call: &Call<'tcx>) -> (usize, Ty<'tcx>) {
	let tcx = b.t.tcx;
	let bounds = tcx.parent(trait_fn(tcx, "RangeBounds", "start_bound"));
	let sig = tcx
		.fn_sig(call.def_id)
		.instantiate_identity()
		.skip_norm_wip()
		.skip_binder();
	let (input, target) = tcx
		.param_env(call.def_id)
		.caller_bounds()
		.find_map(|clause| {
			let tr = clause.as_trait_clause()?.skip_binder().trait_ref;
			let input = sig.inputs().iter().position(|&ty| ty == tr.self_ty())?;
			(tr.def_id == bounds).then(|| (input, tr.args.type_at(1)))
		})
		.expect("a range's bound");
	(input, b.t.subst(call.args, target))
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
		_ if partial && let Some(float) = super::float::Float::of(ty) => Expr::rt_of(float.name(), "partialCmp"),
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
		// Whichever form it has, as the key it is or the operand a reference passes.
		_ if bytes(b, ty) => Expr::Rt("cmpBytes".into()),
		_ if local(b, ty) && partial => {
			let cmp = trait_fn(tcx, "PartialOrd", "partial_cmp");
			b.fn_value(cmp, tcx.mk_args(&[ty.into(), ty.into()]), span)
		}
		_ if local(b, ty) => {
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
		ty::Adt(..) if library::string(tcx, ty) || matches!(b.t.std_of(ty), Some(Std::Ordering | Std::ErrorKind)) => {
			Expr::Rt("same".into())
		}
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
		_ if bytes(b, ty) => Expr::Rt("eqBytes".into()),
		ty::Adt(..) if let Some(inner) = arc_of(b, ty) => {
			let mut values = vec![eq_fn(b, inner, span)];
			// std's `Arc` finds itself equal before comparing a value that is `Eq`.
			if super::iter::implements(b.t, inner, "Eq") {
				values.push(Expr::lit("true"));
			}
			Expr::rt("eqArc", values)
		}
		ty::Adt(..) if local(b, ty) || b.t.std_of(ty).is_some_and(|std| std == Std::Result || std.error()) => {
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
		// What std writes of each: a `bool` as a byte, a `char` as its `u32`.
		ty::Bool => Expr::Rt("hash8".into()),
		ty::Char => Expr::Rt("hash32".into()),
		ty::Int(_) | ty::Uint(_) => hash_int(Int::of(ty).expect("an integer").bits),
		ty::Str => Expr::Rt("hashStr".into()),
		ty::Adt(..) if library::string(tcx, ty) => Expr::Rt("hashStr".into()),
		// Its derived `Hash` writes its `i8` discriminant.
		ty::Adt(..) if b.t.std_of(ty) == Some(Std::Ordering) => Expr::Rt("hash8".into()),
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
		// Its length, then its bytes, whichever form it is: a `[u8; N]` hashes as a slice.
		_ if bytes(b, ty) => Expr::Rt("hashBytes".into()),
		_ if local(b, ty) => {
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
	let f = trait_fn(tcx, "Hash", "hash_slice");
	local(b, elem)
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
	if library::clone_copies(b.t, ty) || library::string(tcx, ty) {
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
		Std::BinaryHeap => "heapIter",
		Std::HashMap => "hashIter",
		Std::HashSet => "hashKeys",
		Std::BTreeMap => "btreeIter",
		Std::BTreeSet => "btreeKeys",
		Std::VecDeque => "dequeIter",
		_ => return None,
	};
	if !ty.is_ref() {
		let droppers = entry_droppers(b, coll, span);
		return Some(Expr::rt(name, super::iter::tail(vec![value], droppers)));
	}
	// What it holds, borrowed.
	let iter = Expr::rt(name, vec![value]);
	Some(match super::iter::iter_of(b.t, ty) {
		Some(into) => lend(b, into, iter, span),
		None => iter,
	})
}

/// `collect` into a collection of type `ty` from `iter`, an iterator of type `from`, which
/// std's reserves for as it does a `TrustedLen` one if `trusted`.
pub fn collect<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	ty: Ty<'tcx>,
	iter: Expr,
	from: Ty<'tcx>,
	trusted: bool,
	span: Span,
) -> Option<Expr> {
	let std = b.t.std_of(ty)?;
	Some(match std {
		Std::HashMap | Std::HashSet => {
			let target = hash_new(b, ty, span)?;
			hash_extend(b, ty, target, iter, from, span)
		}
		Std::BTreeMap | Std::BTreeSet => btree_collect(b, ty, iter, from, trusted, span),
		// std's collects into a `Vec`, and rebuilds it in place.
		Std::BinaryHeap => {
			let item = arg(ty, 0);
			let d = super::iter::to_array(b, iter, from, item, trusted, span);
			Expr::rt("heapFrom", [vec![d], heap_tests(b, item, true, span)].concat())
		}
		// std's collects into a `Vec` first.
		Std::VecDeque => {
			let target = deque_new(b, ty, span);
			let item = arg(ty, 0);
			if !b.t.drops(item) {
				let folds = super::iter::implements(b.t, from, "TrustedLen");
				let take = super::iter::iter_take(b, from, folds, span);
				let reserve = vec![
					take,
					super::iter::size_of(b, item, span),
					Expr::lit(trusted.to_string()),
				];
				Expr::rt("dequeExtend", super::iter::tail(vec![target, iter], reserve))
			} else {
				let items = super::iter::to_array(b, iter, from, item, trusted, span);
				Expr::rt("dequeExtend", vec![target, Expr::rt("iterSlice", vec![items])])
			}
		}
		_ => return None,
	})
}
