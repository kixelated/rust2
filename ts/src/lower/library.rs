//! Rust's standard library: its types' representations, and its functions as runtime
//! helpers or JS. A function missing here fails translation with its path.
//!
//! Functions are keyed by their owner and name: an inherent method by its type
//! (`slice::split_first`), a trait method by its trait (`From::from`), and a free
//! function by its module (`mem::take`).

use rustc_hir::def_id::DefId;
use rustc_middle::thir::{ExprId, ExprKind};
use rustc_middle::ty::{self, GenericArgsRef, Instance, Ty, TyCtxt};
use rustc_span::Span;

use super::body::Body;
use super::fmt;
use super::num::{self, Int, Operand};
use crate::ts::{Expr, Stmt, StmtKind};

/// A standard library type with its own representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Std {
	/// `T | undefined`.
	Option,
	/// `rt.Result<T, E>`.
	Result,
	/// `rt.ByteVec` for bytes, an array otherwise.
	Vec,
	/// A JS string.
	String,
	/// Its contents.
	Box,
	FromUtf8Error,
	TryFromIntError,
	/// `rt.Formatter`.
	Formatter,
	/// `rt.Arguments`: what `format_args!` makes.
	Arguments,
	/// `rt.FmtError`.
	FmtError,
	/// The `Debug` builders: `rt.DebugStruct` and the like.
	DebugStruct,
	DebugTuple,
	DebugList,
	/// What `assert_eq!` and `assert_ne!` compare by: its operator, as a string.
	AssertKind,
	/// `rt.Arc`, counted.
	Arc,
	/// `rt.Weak`.
	Weak,
	/// `rt.Mutex`, which is also its own guard.
	Mutex,
	MutexGuard,
	/// What a `Mutex` can't be locked with on one thread: never.
	PoisonError,
	/// `rt.Waker`.
	Waker,
	/// `rt.Context`: its waker.
	Context,
	/// `rt.Poll<T>`: `T` itself, or `rt.Pending`.
	Poll,
	/// Its pointer.
	Pin,
	/// `rt.Range`, an iterator with `start` and `end` fields.
	Range,
	/// `rt.RangeInclusive`.
	RangeInclusive,
	/// `rt.Peekable`, an iterator with `peek`.
	Peekable,
	/// What `?` branches on, as MIR spells it: the value it was applied to.
	ControlFlow,
	/// `..end`, `start..`, and `..`, as MIR makes them to slice by: `{ end }`, `{ start }`,
	/// and `undefined`.
	RangeTo,
	RangeFrom,
	RangeFull,
	/// A `format_args!` argument: a function writing its value, `rt.Fmt`.
	Argument,
	/// Its pointee: what an async body passes its context in.
	NonNull,
	/// No value: `never`.
	Infallible,
	/// `rt.HashMap`, and `rt.HashSet`, a map to `undefined`.
	HashMap,
	HashSet,
	/// `rt.BTreeMap`, and `rt.BTreeSet`, a map to `undefined`.
	BTreeMap,
	BTreeSet,
	/// `rt.VecDeque`, a ring buffer.
	VecDeque,
	/// `rt.Entry`: a map's `Entry`, `OccupiedEntry`, or `VacantEntry`.
	Entry,
	/// `rt.Ordering`: its discriminant, -1, 0, or 1.
	Ordering,
	/// `rt.Hasher`: `DefaultHasher`, which hash collections hash their keys with.
	Hasher,
}

impl Std {
	pub fn of(tcx: TyCtxt<'_>, did: DefId) -> Option<Self> {
		if !is_std(tcx, did) {
			return None;
		}
		use rustc_span::sym;
		if tcx.is_diagnostic_item(sym::Option, did) {
			return Some(Self::Option);
		}
		if tcx.is_diagnostic_item(sym::Result, did) {
			return Some(Self::Result);
		}
		if tcx.is_diagnostic_item(sym::Vec, did) {
			return Some(Self::Vec);
		}
		if tcx.is_lang_item(did, rustc_attr_ir::LangItem::String) {
			return Some(Self::String);
		}
		if tcx.is_lang_item(did, rustc_attr_ir::LangItem::OwnedBox) {
			return Some(Self::Box);
		}
		for (item, std) in [
			(sym::Arc, Self::Arc),
			(sym::ArcWeak, Self::Weak),
			(sym::Mutex, Self::Mutex),
			(sym::MutexGuard, Self::MutexGuard),
			(rustc_span::Symbol::intern("Waker"), Self::Waker),
			(rustc_span::Symbol::intern("HashMap"), Self::HashMap),
			(rustc_span::Symbol::intern("HashSet"), Self::HashSet),
			(rustc_span::Symbol::intern("BTreeMap"), Self::BTreeMap),
			(rustc_span::Symbol::intern("BTreeSet"), Self::BTreeSet),
			(rustc_span::Symbol::intern("VecDeque"), Self::VecDeque),
		] {
			if tcx.is_diagnostic_item(item, did) {
				return Some(std);
			}
		}
		for (item, std) in [
			(rustc_attr_ir::LangItem::Context, Self::Context),
			(rustc_attr_ir::LangItem::ResumeTy, Self::Context),
			(rustc_attr_ir::LangItem::Poll, Self::Poll),
			(rustc_attr_ir::LangItem::Pin, Self::Pin),
			(rustc_attr_ir::LangItem::Range, Self::Range),
			(rustc_attr_ir::LangItem::RangeInclusiveStruct, Self::RangeInclusive),
			(rustc_attr_ir::LangItem::RangeTo, Self::RangeTo),
			(rustc_attr_ir::LangItem::RangeFrom, Self::RangeFrom),
			(rustc_attr_ir::LangItem::RangeFull, Self::RangeFull),
		] {
			if tcx.is_lang_item(did, item) {
				return Some(std);
			}
		}
		match tcx.item_name(did).as_str() {
			"FromUtf8Error" => return Some(Self::FromUtf8Error),
			"TryFromIntError" => return Some(Self::TryFromIntError),
			"PoisonError" => return Some(Self::PoisonError),
			_ => {}
		}
		match std_path(tcx, did)
			.iter()
			.map(String::as_str)
			.collect::<Vec<_>>()
			.as_slice()
		{
			["fmt", "Formatter"] => Some(Self::Formatter),
			["fmt", "Arguments"] => Some(Self::Arguments),
			["fmt", "Error"] => Some(Self::FmtError),
			["fmt", "builders", "DebugStruct"] => Some(Self::DebugStruct),
			["fmt", "builders", "DebugTuple"] => Some(Self::DebugTuple),
			["fmt", "builders", "DebugList"] => Some(Self::DebugList),
			["panicking", "AssertKind"] => Some(Self::AssertKind),
			["iter", "adapters", "peekable", "Peekable"] => Some(Self::Peekable),
			["ops", "control_flow", "ControlFlow"] => Some(Self::ControlFlow),
			["fmt", "rt", "Argument"] => Some(Self::Argument),
			["ptr", "non_null", "NonNull"] => Some(Self::NonNull),
			["convert", "Infallible"] => Some(Self::Infallible),
			["cmp", "Ordering"] => Some(Self::Ordering),
			["hash", "random", "DefaultHasher"] => Some(Self::Hasher),
			["collections", "hash", "map", "Entry" | "OccupiedEntry" | "VacantEntry"]
			| [
				"collections",
				"btree",
				"map",
				"entry",
				"Entry" | "OccupiedEntry" | "VacantEntry",
			] => Some(Self::Entry),
			_ => None,
		}
	}
}

/// The path of a std item within its crate, as `["slice", "iter", "Iter"]`; empty for
/// any other crate's.
pub fn std_path(tcx: TyCtxt<'_>, did: DefId) -> Vec<String> {
	if !is_std(tcx, did) {
		return Vec::new();
	}
	tcx.def_path(did)
		.data
		.iter()
		.filter_map(|d| d.data.get_opt_name())
		.map(|n| n.to_string())
		.collect()
}

fn is_std(tcx: TyCtxt<'_>, did: DefId) -> bool {
	matches!(tcx.crate_name(did.krate).as_str(), "core" | "alloc" | "std")
}

/// Whether `Clone` of `ty`, a `Copy` type, only copies: no `Clone` written by hand runs
/// for it or anything in it, as a derived or std `Clone` clones each field.
pub fn clone_copies<'tcx>(t: &super::Translator<'_, 'tcx>, ty: Ty<'tcx>) -> bool {
	let tcx = t.tcx;
	match ty.kind() {
		ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Ref(..) | ty::FnPtr(..) | ty::Never => true,
		ty::Tuple(fields) => fields.iter().all(|field| clone_copies(t, field)),
		ty::Array(elem, _) => clone_copies(t, *elem),
		ty::Adt(adt, args) if !adt.did().is_local() => args.types().all(|arg| clone_copies(t, arg)),
		ty::Adt(adt, args) => {
			let Some(clone) = tcx.lang_items().clone_fn() else {
				return false;
			};
			let derived = Instance::try_resolve(tcx, t.typing_env_pub(), clone, tcx.mk_args(&[ty.into()]))
				.ok()
				.flatten()
				.and_then(|instance| tcx.impl_of_assoc(instance.def_id()))
				.is_some_and(|imp| tcx.is_automatically_derived(imp));
			derived
				&& adt
					.all_fields()
					.all(|field| clone_copies(t, t.subst(args, field.ty(tcx, args).skip_norm_wip())))
		}
		_ => false,
	}
}

/// The standard type an inherent method is of, such as `Range` for `Range::contains`.
pub fn inherent_std(tcx: TyCtxt<'_>, def_id: DefId) -> Option<Std> {
	let imp = tcx.inherent_impl_of_assoc(def_id)?;
	match tcx.type_of(imp).instantiate_identity().skip_norm_wip().kind() {
		ty::Adt(adt, _) => Std::of(tcx, adt.did()),
		_ => None,
	}
}

/// A call's arguments: still THIR, or already lowered.
pub enum Args<'x> {
	Thir(&'x [ExprId]),
	Values(Vec<Expr>),
}

pub struct Call<'tcx> {
	/// The function as written, before resolving a trait method to its impl.
	pub def_id: DefId,
	/// Its generic arguments: the impl's or trait's, then the function's.
	pub args: GenericArgsRef<'tcx>,
	pub span: Span,
}

/// Whether the mapping of `owner::name` moves each operand it takes by value into its
/// result or a closure, and so drops nothing JS would leave undropped. Any other mapping
/// may discard one, as `unwrap_or` does its fallback. Iterators refuse their own.
fn keeps_operands(owner: &str, name: &str) -> bool {
	matches!(
		(owner, name),
		("Option", "map" | "unwrap" | "expect")
			| (
				"Result",
				"map" | "map_err" | "and_then" | "inspect_err" | "unwrap" | "expect" | "unwrap_err"
			)
			| ("Vec", "push")
			| ("Box", "new")
			| ("boxed", "box_new")
			| ("mem", "replace" | "take" | "swap")
			| ("hint", "must_use")
			| ("From", "from")
			| ("Into", "into")
	)
}

/// The first type `call` takes by value that drops.
fn dropping_operand<'tcx>(b: &mut Body<'_, '_, 'tcx>, call: &Call<'tcx>) -> Option<Ty<'tcx>> {
	let tcx = b.t.tcx;
	let sig = tcx.fn_sig(call.def_id).instantiate(tcx, call.args).skip_norm_wip();
	let env = b.t.typing_env_pub();
	// A census's call over type parameters may leave a projection unnormalized.
	let inputs: Vec<Ty<'tcx>> = tcx
		.instantiate_bound_regions_with_erased(sig)
		.inputs()
		.iter()
		.map(|&ty| {
			tcx.try_normalize_erasing_regions(env, ty::Unnormalized::new_wip(ty))
				.unwrap_or(ty)
		})
		.collect();
	inputs.into_iter().find(|&ty| b.t.drops(ty))
}

/// `owner::name` for a standard library function.
pub fn key(tcx: TyCtxt<'_>, def_id: DefId) -> Option<(String, String)> {
	let name = tcx.item_name(def_id).to_string();
	if let Some(trait_id) = tcx.trait_of_assoc(def_id) {
		return Some((tcx.item_name(trait_id).to_string(), name));
	}
	if let Some(imp) = tcx.inherent_impl_of_assoc(def_id) {
		let self_ty = tcx.type_of(imp).instantiate_identity().skip_norm_wip();
		let owner = match self_ty.kind() {
			ty::Adt(adt, _) => tcx.item_name(adt.did()).to_string(),
			ty::Slice(_) => "slice".into(),
			ty::Array(..) => "array".into(),
			ty::Str => "str".into(),
			ty::Bool => "bool".into(),
			ty::Int(_) | ty::Uint(_) => self_ty.to_string(),
			_ => return None,
		};
		return Some((owner, name));
	}
	let parent = tcx.parent(def_id);
	let owner = tcx
		.opt_item_name(parent)
		.map_or_else(|| tcx.crate_name(def_id.krate).to_string(), |n| n.to_string());
	Some((owner, name))
}

pub fn call<'tcx>(b: &mut Body<'_, '_, 'tcx>, call: &Call<'tcx>, args: Args<'_>, out: &mut Vec<Stmt>) -> Option<Expr> {
	let tcx = b.t.tcx;
	// thiserror's `Display` derive reads each field through `AsDisplay`, which is the
	// field itself for anything but a path.
	if tcx.crate_name(call.def_id.krate).as_str() == "thiserror"
		&& tcx.item_name(call.def_id).as_str() == "as_display"
		&& matches!(targ(call.args, 0).kind(), ty::Ref(..))
	{
		return b.args(args, out).pop();
	}
	if !is_std(tcx, call.def_id) {
		return None;
	}
	let (owner, name) = key(tcx, call.def_id)?;
	let span = call.span;
	let gargs = call.args;
	let args = match super::task::call(b, call, &owner, &name, args, out) {
		Ok(e) => return e,
		Err(args) => args,
	};
	let args = match super::collections::call(b, call, &owner, &name, args, out) {
		Ok(e) => return e,
		Err(args) => args,
	};
	let dropping = match keeps_operands(&owner, &name) {
		true => None,
		false => dropping_operand(b, call),
	};

	// Methods on integers.
	if let Some(int) = int_named(&owner) {
		let method = match name.as_str() {
			"from_be_bytes" => "fromBeBytes",
			"to_be_bytes" => "toBeBytes",
			"leading_ones" => "leadingOnes",
			"leading_zeros" => "leadingZeros",
			"checked_add" => "checkedAdd",
			"checked_sub" => "checkedSub",
			"checked_mul" => "checkedMul",
			"saturating_add" => "saturatingAdd",
			"saturating_sub" => "saturatingSub",
			"wrapping_add" => "wrappingAdd",
			"wrapping_sub" => "wrappingSub",
			_ => return None,
		};
		if let ("from_be_bytes", Args::Thir([bytes])) = (name.as_str(), &args) {
			return Some(from_be_bytes(b, int, *bytes, out));
		}
		let values = b.args(args, out);
		return Some(int.method(method, values));
	}

	let self_ty = || gargs.types().next();
	let elem = |t: Ty<'tcx>| -> Option<Ty<'tcx>> {
		match t.peel_refs().kind() {
			ty::Slice(e) | ty::Array(e, _) => Some(*e),
			ty::Adt(adt, args) if Std::of(tcx, adt.did()) == Some(Std::Vec) => Some(args.type_at(0)),
			_ => None,
		}
	};
	let is_u8 = |t: Ty<'tcx>| matches!(t.kind(), ty::Uint(ty::UintTy::U8));
	let range = matches!(inherent_std(tcx, call.def_id), Some(Std::Range | Std::RangeInclusive));
	// A range of integers, whose comparisons and clones JS makes as Rust does.
	let int_range = |t: Ty<'tcx>| match t.peel_refs().kind() {
		ty::Adt(adt, args) => {
			matches!(Std::of(tcx, adt.did()), Some(Std::Range | Std::RangeInclusive))
				&& Int::of(args.type_at(0)).is_some()
		}
		_ => false,
	};

	let e = match (owner.as_str(), name.as_str()) {
		// Conversions.
		("From", "from") => {
			let (to, from) = (targ(gargs, 0), targ(gargs, 1));
			let mut values = b.args(args, out);
			let value = values.pop()?;
			if to == from {
				return Some(value);
			}
			match (Int::of(to), Int::of(from)) {
				(Some(_), Some(_)) => {
					let operand = Operand {
						expr: value,
						value: None,
					};
					num::cast(b, operand, from, to, span)
				}
				_ => return None,
			}
		}
		("Into", "into") => {
			let (from, to) = (targ(gargs, 0), targ(gargs, 1));
			let mut values = b.args(args, out);
			let value = values.pop()?;
			b.convert(value, from, to, span, out)
		}
		("TryFrom", "try_from") => {
			let (to, from) = (targ(gargs, 0), targ(gargs, 1));
			let (Some(int), Some(_)) = (Int::of(to), Int::of(from)) else {
				return None;
			};
			let values = b.args(args, out);
			int.method("tryFrom", values)
		}

		// Operators on references to primitives.
		("Add" | "Sub" | "Mul" | "Div" | "Rem" | "BitAnd" | "BitOr" | "BitXor" | "Shl" | "Shr", _)
			if primitive(targ(gargs, 0).peel_refs()) =>
		{
			let op = match owner.as_str() {
				"Add" => rustc_middle::mir::BinOp::Add,
				"Sub" => rustc_middle::mir::BinOp::Sub,
				"Mul" => rustc_middle::mir::BinOp::Mul,
				"Div" => rustc_middle::mir::BinOp::Div,
				"Rem" => rustc_middle::mir::BinOp::Rem,
				"BitAnd" => rustc_middle::mir::BinOp::BitAnd,
				"BitOr" => rustc_middle::mir::BinOp::BitOr,
				"BitXor" => rustc_middle::mir::BinOp::BitXor,
				"Shl" => rustc_middle::mir::BinOp::Shl,
				_ => rustc_middle::mir::BinOp::Shr,
			};
			let (lty, rty) = (targ(gargs, 0), targ(gargs, 1));
			let consts = match &args {
				Args::Thir(ids) => (b.const_int(ids[0]), b.const_int(ids[1])),
				Args::Values(_) => (None, None),
			};
			let values = b.args(args, out);
			let [l, r] = <[Expr; 2]>::try_from(values).ok()?;
			num::binary(
				b,
				op,
				Operand {
					expr: l,
					value: consts.0,
				},
				Operand {
					expr: r,
					value: consts.1,
				},
				lty,
				rty,
				span,
			)
		}
		("PartialEq", "eq" | "ne") | ("PartialOrd", "lt" | "le" | "gt" | "ge")
			if primitive(targ(gargs, 0).peel_refs()) =>
		{
			let js = match name.as_str() {
				"eq" => "===",
				"ne" => "!==",
				"lt" => "<",
				"le" => "<=",
				"gt" => ">",
				_ => ">=",
			};
			let values = b.args(args, out);
			let [l, r] = <[Expr; 2]>::try_from(values).ok()?;
			match Int::of(targ(gargs, 0).peel_refs()) {
				Some(int) => int.compare(js, l, r),
				None => l.binary(js, r),
			}
		}
		("Ord", "max" | "min") if let Some(int) = Int::of(targ(gargs, 0)) => {
			let values = b.args(args, out);
			match int.big() {
				true => int.method(name.as_str(), values),
				false => Expr::rt(if name == "max" { "max" } else { "min" }, values),
			}
		}
		("Clone", "clone") => {
			let ty = targ(gargs, 0);
			let values = b.args(args, out);
			let value = values.into_iter().next()?;
			match ty.kind() {
				ty::Adt(adt, args) if Std::of(tcx, adt.did()) == Some(Std::Vec) && is_u8(args.type_at(0)) => {
					Expr::Rt("ByteVec")
						.member("from")
						.call(vec![value.method("view", vec![])])
				}
				ty::Adt(adt, _) if Std::of(tcx, adt.did()) == Some(Std::String) => value,
				ty::Adt(..) if int_range(ty) => value.method("clone", vec![]),
				_ if b.t.is_copy(ty) && clone_copies(b.t, ty) => b.copied(value, ty),
				_ => return None,
			}
		}
		("Default", "default") => default_value(b, targ(gargs, 0))?,

		// Options.
		("Option", "map") => rt2(b, args, out, "option", "map"),
		("Option", "map_or") => rt2(b, args, out, "option", "mapOr"),
		("Option", "unwrap_or") => rt2(b, args, out, "option", "unwrapOr"),
		("Option", "ok_or") => rt2(b, args, out, "option", "okOr"),
		("Option", "unwrap") => rt2(b, args, out, "option", "unwrap"),
		("Option", "expect") => rt2(b, args, out, "option", "expect"),
		("Option", "is_some") => b.args(args, out).pop()?.binary("!==", Expr::undefined()),
		("Option", "is_none") => b.args(args, out).pop()?.binary("===", Expr::undefined()),
		("Option", "as_ref" | "as_mut") => b.args(args, out).pop()?,
		("Option", "copied" | "cloned") => {
			let inner = targ(gargs, 0);
			let value = b.args(args, out).pop()?;
			b.copied(value, inner)
		}
		("Option", "take") => {
			let Args::Thir(ids) = args else {
				return None;
			};
			let place = borrowed_place(b, ids[0], out)?;
			swap_out(b, place, Expr::undefined(), "taken", out)
		}

		// Results.
		("Result", "map") => rt2(b, args, out, "result", "map"),
		("Result", "map_err") => rt2(b, args, out, "result", "mapErr"),
		("Result", "and_then") => rt2(b, args, out, "result", "andThen"),
		("Result", "inspect_err") => rt2(b, args, out, "result", "inspectErr"),
		("Result", "is_ok") => b.args(args, out).pop()?.member("ok"),
		("Result", "is_err") => b.args(args, out).pop()?.member("ok").not(),
		("Result", "ok") => rt2(b, args, out, "result", "ok"),
		("Result", "err") => rt2(b, args, out, "result", "err"),
		// Their panics show the other side with its `Debug`.
		("Result", "unwrap" | "expect" | "unwrap_err") => {
			let (method, shown) = match name.as_str() {
				"unwrap" => ("unwrap", targ(gargs, 1)),
				"expect" => ("expect", targ(gargs, 1)),
				_ => ("unwrapErr", targ(gargs, 0)),
			};
			let mut values = b.args(args, out);
			values.push(b.fmt_fn(fmt::Trait::Debug, shown, span));
			Expr::Rt("result").member(method).call(values)
		}
		("Result", "unwrap_or") => rt2(b, args, out, "result", "unwrapOr"),

		// Ranges, which are their own iterators.
		// An integer range wraps within its type's bounds where std's `nth` does.
		("RangeInclusive", "new") if range => {
			let mut values = b.args(args, out);
			values.extend(self_ty().and_then(Int::of).map(|int| Expr::Rt(int.rt())));
			Expr::New(Box::new(Expr::Rt("RangeInclusive")), values)
		}
		// JS compares integers as Rust does, and other endpoints by identity.
		("Range" | "RangeInclusive", "start" | "end" | "is_empty" | "contains")
			if range && (matches!(name.as_str(), "start" | "end") || self_ty().and_then(Int::of).is_some()) =>
		{
			let mut values = b.args(args, out);
			let recv = values.remove(0);
			recv.method(&super::camel(&name), values)
		}

		("bool", "then_some") => {
			let values = b.args(args, out);
			Expr::rt("thenSome", values)
		}

		// Slices.
		("slice", "len") => b.args(args, out).pop()?.member("length"),
		("slice", "is_empty") => b.args(args, out).pop()?.member("length").binary("===", Expr::lit("0")),
		("slice", "split_first") => rt2(b, args, out, "slice", "splitFirst"),
		("slice", "split_at") => rt2(b, args, out, "slice", "splitAt"),
		("slice", "split_at_checked") => rt2(b, args, out, "slice", "splitAtChecked"),
		("slice", "first") => rt2(b, args, out, "slice", "first"),
		("slice", "get") if Int::of(targ(gargs, 1)).is_some() => rt2(b, args, out, "slice", "get"),
		("slice", "split_first_chunk") => {
			let n = gargs.consts().next()?.try_to_target_usize(tcx)?;
			let mut values = b.args(args, out);
			values.push(Expr::lit(n.to_string()));
			Expr::Rt("slice").member("splitFirstChunk").call(values)
		}
		("slice", "copy_from_slice") if is_u8(targ(gargs, 0)) => rt2(b, args, out, "slice", "copyFromSlice"),
		("slice", "copy_within") if is_u8(targ(gargs, 0)) => {
			let Args::Thir(ids) = args else {
				return None;
			};
			let mut s = [b.operand(ids[0], out)];
			let (start, end) = range_bounds(b, &mut s, ids[1], out)?;
			let [s] = s;
			let end = end.unwrap_or_else(|| s.clone().member("length"));
			let dest = b.operand(ids[2], out);
			Expr::Rt("slice").member("copyWithin").call(vec![s, start, end, dest])
		}
		("slice", "to_vec") => {
			let elem = targ(gargs, 0);
			let values = b.args(args, out);
			match is_u8(elem) {
				true => Expr::Rt("slice").member("toByteVec").call(values),
				false if b.t.is_copy(elem) => Expr::Rt("slice").member("toVec").call(values),
				false => return None,
			}
		}
		("slice", "into_vec") => {
			// `vec![..]`: `<[T]>::into_vec(box_new([..]))`.
			let elem = targ(gargs, 0);
			let value = b.args(args, out).pop()?;
			match is_u8(elem) {
				true => Expr::Rt("ByteVec").member("from").call(vec![value]),
				false => value,
			}
		}
		("boxed", "box_new") | ("Box", "new") => b.args(args, out).pop()?,
		("vec", "from_elem") => {
			let elem = targ(gargs, 0);
			let values = b.args(args, out);
			match is_u8(elem) {
				true => Expr::rt("byteVecOf", values),
				false if b.t.is_copy(elem) => {
					let [value, n] = <[Expr; 2]>::try_from(values).ok()?;
					let value = match value.is_path() {
						true => value,
						false => {
							let name = b.fresh("item");
							out.push(StmtKind::decl(false, name.clone(), None, Some(value)).at(None));
							Expr::ident(name)
						}
					};
					let item = b.copied(value, elem);
					let item_ty = b.t.ty(elem, span);
					Expr::ident("Array").member("from").call(vec![
						Expr::Object(vec![(crate::ts::Key::Name("length".into()), n)]),
						Expr::Arrow(Vec::new(), item_ty, vec![StmtKind::Return(Some(item.typed())).at(None)]),
					])
				}
				false => return None,
			}
		}

		// Vectors.
		("Vec", "new") => match is_u8(targ(gargs, 0)) {
			true => Expr::New(Box::new(Expr::Rt("ByteVec")), Vec::new()),
			false => Expr::Array(Vec::new()),
		},
		("Vec", "with_capacity") => match is_u8(targ(gargs, 0)) {
			true => Expr::New(Box::new(Expr::Rt("ByteVec")), b.args(args, out)),
			false => {
				b.args(args, out);
				Expr::Array(Vec::new())
			}
		},
		("Vec", "push") => {
			let mut values = b.args(args, out);
			let v = values.remove(0);
			v.method("push", values)
		}
		("Vec", "pop") => b.args(args, out).pop()?.method("pop", vec![]),
		("Vec", "len") => b.args(args, out).pop()?.member("length"),
		("Vec", "is_empty") => b.args(args, out).pop()?.member("length").binary("===", Expr::lit("0")),
		("Vec", "truncate") if is_u8(targ(gargs, 0)) => {
			let mut values = b.args(args, out);
			let v = values.remove(0);
			v.method("truncate", values)
		}
		("Vec", "clear") if is_u8(targ(gargs, 0)) => b.args(args, out).pop()?.method("clear", vec![]),
		("Vec", "extend_from_slice") if is_u8(targ(gargs, 0)) => {
			let mut values = b.args(args, out);
			let v = values.remove(0);
			v.method("extendFromSlice", values)
		}
		("Vec", "as_slice" | "as_mut_slice") => {
			let v = b.args(args, out).pop()?;
			vec_view(v, targ(gargs, 0))
		}
		("Deref", "deref") | ("DerefMut", "deref_mut") => {
			let ty = targ(gargs, 0);
			let v = b.args(args, out).pop()?;
			match ty.kind() {
				ty::Adt(adt, _) => match Std::of(tcx, adt.did()) {
					Some(Std::Vec) => vec_view(v, ty),
					Some(Std::String | Std::Box) => v,
					_ => return None,
				},
				_ => return None,
			}
		}
		("Index", "index") | ("IndexMut", "index_mut") => {
			let Args::Thir(ids) = args else {
				return None;
			};
			let container = targ(gargs, 0);
			let index_ty = targ(gargs, 1);
			let elem_ty = elem(container)?;
			let base = b.operand(ids[0], out);
			let base = match container.kind() {
				ty::Adt(..) => vec_view(base, container),
				_ => base,
			};
			match Int::of(index_ty) {
				Some(_) if name == "index" => {
					let i = b.operand(ids[1], out);
					Expr::rt("at", vec![base, i])
				}
				Some(_) => {
					let base = path(b, base, "slice", out);
					let i = b.operand(ids[1], out);
					let i = path(b, i, "index", out);
					base.clone().index(Expr::rt("idx", vec![base, i]))
				}
				None => {
					if name == "index_mut" && !is_u8(elem_ty) {
						// A JS array has no views, so a mutable subslice would be a copy.
						return None;
					}
					let mut base = [base];
					let (start, end) = range_bounds(b, &mut base, ids[1], out)?;
					let [base] = base;
					let mut values = vec![base, start];
					values.extend(end);
					Expr::Rt("slice").member("range").call(values)
				}
			}
		}

		// Strings.
		("String", "as_bytes") => Expr::Rt("string").member("asBytes").call(b.args(args, out)),
		("String", "from_utf8") => Expr::Rt("string").member("fromUtf8").call(b.args(args, out)),
		("str", "as_bytes") => Expr::Rt("string").member("asBytes").call(b.args(args, out)),
		("str", "len") => Expr::Rt("string").member("len").call(b.args(args, out)),
		("String", "as_str") => b.args(args, out).pop()?,
		("String", "len") => Expr::Rt("string").member("len").call(b.args(args, out)),
		("String", "new") => Expr::Str(String::new()),
		("ToString", "to_string") | ("ToOwned", "to_owned") if matches!(self_ty()?.peel_refs().kind(), ty::Str) => {
			b.args(args, out).pop()?
		}

		// Moving out of a place.
		("mem", "take") => {
			let Args::Thir(ids) = args else {
				return None;
			};
			let ty = targ(gargs, 0);
			let place = borrowed_place(b, ids[0], out)?;
			let default = default_value(b, ty)?;
			swap_out(b, place, default, "taken", out)
		}
		("mem", "replace") => {
			let Args::Thir(ids) = args else {
				return None;
			};
			let place = borrowed_place(b, ids[0], out)?;
			let value = b.operand(ids[1], out);
			swap_out(b, place, value, "old", out)
		}
		("mem", "swap") => {
			let Args::Thir(ids) = args else {
				return None;
			};
			let a = borrowed_place(b, ids[0], out)?;
			let c = borrowed_place(b, ids[1], out)?;
			let old = swap_out(b, a.clone(), c.clone(), "swapped", out);
			out.push(StmtKind::Expr(c.assign(old)).at(None));
			Expr::undefined()
		}

		// Panics, which format their message as std does.
		("panicking", "panic_fmt") => {
			let message = b.args(args, out).pop()?;
			Expr::rt("panic", vec![literal(&message).map_or(message, Expr::Str)])
		}
		("panicking", "panic") | ("rt", "begin_panic") => Expr::rt("panic", b.args(args, out)),
		("panicking", "panic_explicit") => Expr::rt("panic", vec![Expr::Str("explicit panic".into())]),
		("panicking", "panic_display" | "unreachable_display") => {
			let value = b.args(args, out).pop()?;
			let value = b.fmt_value(fmt::Trait::Display, targ(gargs, 0), value, span, out);
			let mut message = vec![value];
			if name == "unreachable_display" {
				message.insert(0, Expr::Str("internal error: entered unreachable code: ".into()));
			}
			Expr::rt("panic", vec![Expr::Array(message)])
		}
		// `assert_eq!` and `assert_ne!`: the operator, both sides, and any message.
		("panicking", "assert_failed") => {
			let [kind, left, right, message] = <[Expr; 4]>::try_from(b.args(args, out)).ok()?;
			let left = b.fmt_value(fmt::Trait::Debug, targ(gargs, 0), left, span, out);
			let right = b.fmt_value(fmt::Trait::Debug, targ(gargs, 1), right, span, out);
			let mut values = vec![kind, left, right];
			if !matches!(&message, Expr::Lit(text) if text == "undefined") {
				values.push(message);
			}
			Expr::rt("assertFailed", values)
		}

		// Formatting.
		("hint", "must_use") => b.args(args, out).pop()?,
		("fmt", "format") => {
			let parts = b.args(args, out).pop()?;
			match literal(&parts) {
				Some(s) => Expr::Str(s),
				None => Expr::Rt("fmt").member("format").call(vec![parts]),
			}
		}
		("Display" | "Debug" | "LowerHex" | "UpperHex", "fmt") => {
			let tr = fmt::Trait::of(tcx, tcx.trait_of_assoc(call.def_id)?)?;
			let [value, f] = <[Expr; 2]>::try_from(b.args(args, out)).ok()?;
			fmt::std_impl(b, tr, targ(gargs, 0), value, f, span, out)?
		}
		("ToString", "to_string") => {
			let ty = targ(gargs, 0);
			let value = b.args(args, out).pop()?;
			match ty.kind() {
				ty::Adt(adt, _) if Std::of(tcx, adt.did()) == Some(Std::String) => value,
				_ => {
					let value = b.fmt_value(fmt::Trait::Display, ty, value, span, out);
					Expr::Rt("fmt").member("toString").call(vec![value])
				}
			}
		}
		("Formatter", "write_fmt") => {
			let [f, parts] = <[Expr; 2]>::try_from(b.args(args, out)).ok()?;
			match literal(&parts) {
				Some(s) => f.method("writeStr", vec![Expr::Str(s)]),
				None => f.method("writeFmt", vec![parts]),
			}
		}
		("Formatter", "write_str" | "pad" | "alternate" | "width" | "debug_struct" | "debug_tuple" | "debug_list")
		| ("DebugStruct" | "DebugTuple", "field" | "finish" | "finish_non_exhaustive")
		| ("DebugList", "entry" | "finish" | "finish_non_exhaustive") => {
			let mut values = b.args(args, out);
			let recv = values.remove(0);
			recv.method(&super::camel(&name), values)
		}
		// Derived `Debug`: `debug_struct_field2_finish(f, name, "a", &a, "b", &b)`.
		("Formatter", "debug_struct_fields_finish") => {
			let mut values = b.args(args, out);
			let f = values.remove(0);
			f.method("debugStructFields", values)
		}
		("Formatter", "debug_tuple_fields_finish") => {
			let mut values = b.args(args, out);
			let f = values.remove(0);
			f.method("debugTupleFields", values)
		}
		("Formatter", n) if n.starts_with("debug_struct_field") || n.starts_with("debug_tuple_field") => {
			let mut values = b.args(args, out).into_iter();
			let (f, type_name) = (values.next()?, values.next()?);
			let structure = n.starts_with("debug_struct");
			let mut builder = f.method(if structure { "debugStruct" } else { "debugTuple" }, vec![type_name]);
			let values: Vec<Expr> = values.collect();
			let fields = match structure {
				true => values.chunks(2).map(<[Expr]>::to_vec).collect::<Vec<_>>(),
				false => values.into_iter().map(|v| vec![v]).collect(),
			};
			for field in fields {
				builder = builder.method("field", field);
			}
			builder.method("finish", vec![])
		}
		("DebugList", "entries") => {
			let Args::Thir(ids) = args else {
				return None;
			};
			let list = b.operand(ids[0], out);
			let items = items(b, ids[1], out)?;
			let debug = b.fmt_fn(fmt::Trait::Debug, targ(gargs, 0), span);
			list.method("entries", vec![items, debug])
		}
		_ => return super::iter::call(b, call, &owner, &name, args, out),
	};
	if let Some(ty) = dropping {
		return Some(b.err(
			span,
			format!("passing `{ty}`, which drops, to `{owner}::{name}`, which is not supported yet"),
		));
	}
	Some(e)
}

/// `int::from_be_bytes(bytes)`. A number from an array written in place is its bytes
/// shifted and or-ed, with no array made. Otherwise the runtime reads the array the
/// argument names, uncopied, since the read neither keeps nor changes it.
fn from_be_bytes<'tcx>(b: &mut Body<'_, '_, 'tcx>, int: Int, bytes: ExprId, out: &mut Vec<Stmt>) -> Expr {
	let thir = b.thir;
	let fields = match &thir[b.strip(bytes)].kind {
		ExprKind::Array { fields } if !int.big() => fields,
		_ => {
			let value = b.place(bytes, out);
			return int.method("fromBeBytes", vec![value]);
		}
	};
	let values = b.operands(fields, out);
	be_word(int, values)
}

/// A number from its big-endian bytes, each a value: shifted and or-ed.
pub fn be_word(int: Int, values: Vec<Expr>) -> Expr {
	let last = values.len() - 1;
	let mut word: Option<Expr> = None;
	let mut top = false;
	for (i, value) in values.into_iter().enumerate() {
		if matches!(&value, Expr::Lit(l) if l == "0") {
			continue;
		}
		top |= i == 0;
		let shift = 8 * (last - i);
		let term = match shift {
			0 => value,
			_ => value.binary("<<", Expr::lit(shift.to_string())),
		};
		word = Some(match word {
			Some(word) => word.binary("|", term),
			None => term,
		});
	}
	let Some(word) = word else {
		return Expr::lit("0");
	};
	// JS's `|` and `<<` make an `i32`, so only a `u32` with its top byte set and the
	// narrower signed types need their bits read again.
	match (int.signed, int.bits) {
		(false, 32) if top => int.wrap(word),
		(true, bits) if bits < 32 => int.wrap(word),
		_ => word,
	}
}

/// The `n`th type among a call's generic arguments, skipping lifetimes.
pub fn targ<'tcx>(gargs: GenericArgsRef<'tcx>, n: usize) -> Ty<'tcx> {
	gargs.types().nth(n).expect("a type argument")
}

/// `rt.<namespace>.<name>(args)`.
fn rt2<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	args: Args<'_>,
	out: &mut Vec<Stmt>,
	namespace: &'static str,
	name: &str,
) -> Expr {
	let values = b.args(args, out);
	Expr::Rt(namespace).member(name).call(values)
}

fn int_named(name: &str) -> Option<Int> {
	let (signed, bits, pointer) = match name {
		"u8" => (false, 8, false),
		"u16" => (false, 16, false),
		"u32" => (false, 32, false),
		"u64" => (false, 64, false),
		"u128" => (false, 128, false),
		"usize" => (false, 32, true),
		"i8" => (true, 8, false),
		"i16" => (true, 16, false),
		"i32" => (true, 32, false),
		"i64" => (true, 64, false),
		"i128" => (true, 128, false),
		"isize" => (true, 32, true),
		_ => return None,
	};
	Some(Int { bits, signed, pointer })
}

fn primitive(ty: Ty<'_>) -> bool {
	matches!(ty.kind(), ty::Bool | ty::Int(_) | ty::Uint(_) | ty::Char)
}

/// A `Vec`'s elements as a slice: a view of a byte vector, or the array itself.
pub fn vec_view(v: Expr, ty: Ty<'_>) -> Expr {
	let elem = match ty.peel_refs().kind() {
		ty::Adt(_, args) => args.type_at(0),
		_ => ty,
	};
	match elem.kind() {
		ty::Uint(ty::UintTy::U8) => v.method("view", vec![]),
		_ => v,
	}
}

trait MethodCall {
	fn method(self, name: &str, args: Vec<Expr>) -> Expr;
}

impl MethodCall for Expr {
	fn method(self, name: &str, args: Vec<Expr>) -> Expr {
		self.member(name).call(args)
	}
}

fn path<'tcx>(b: &mut Body<'_, '_, 'tcx>, value: Expr, base: &str, out: &mut Vec<Stmt>) -> Expr {
	if value.is_path() {
		return value;
	}
	let name = b.fresh(base);
	out.push(StmtKind::decl(false, name.clone(), None, Some(value)).at(None));
	Expr::ident(name)
}

/// The start and optional end of a range written in place: `a..b`, `..b`, `a..`, `..`.
/// `earlier` are the operands evaluated before it, spilled if the range needs statements.
fn range_bounds<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	earlier: &mut [Expr],
	id: ExprId,
	out: &mut Vec<Stmt>,
) -> Option<(Expr, Option<Expr>)> {
	let tcx = b.t.tcx;
	let written = id;
	let id = b.strip(id);
	let ty = b.ty_of(id);
	let ty::Adt(adt, _) = ty.kind() else {
		return None;
	};
	let name = tcx.item_name(adt.did()).to_string();
	if name == "RangeFull" {
		return Some((Expr::lit("0"), None));
	}
	let ExprKind::Adt(range) = &b.thir[id].kind else {
		return None;
	};
	let ids: Vec<ExprId> = range.fields.iter().map(|f| f.expr).collect();
	b.visit(written);
	let mut pre = Vec::new();
	let values = b.operands(&ids, &mut pre);
	let values = range.fields.iter().map(|f| f.name.index()).zip(values).collect();
	let mut values: Vec<Expr> = b.declared(values, &mut pre).into_iter().map(|(_, v)| v).collect();
	if !pre.is_empty() {
		for value in earlier.iter_mut().filter(|v| !v.is_constant()) {
			let saved = std::mem::replace(value, Expr::undefined());
			*value = b.spill(saved, "arg", out);
		}
	}
	out.extend(pre);
	match name.as_str() {
		"Range" => {
			let end = values.pop()?;
			let start = values.pop()?;
			Some((start, Some(end)))
		}
		"RangeTo" => Some((Expr::lit("0"), values.pop())),
		"RangeFrom" => Some((values.pop()?, None)),
		_ => None,
	}
}

/// The place behind `&mut place`, as an assignable path.
fn borrowed_place<'tcx>(b: &mut Body<'_, '_, 'tcx>, id: ExprId, out: &mut Vec<Stmt>) -> Option<Expr> {
	let written = id;
	let id = b.strip(id);
	let ExprKind::Borrow { mut arg, .. } = b.thir[id].kind else {
		return None;
	};
	// A reborrow, `&mut *&mut place`.
	while let ExprKind::Deref { arg: inner } = b.thir[b.strip(arg)].kind
		&& let ExprKind::Borrow { arg: place, .. } = b.thir[b.strip(inner)].kind
	{
		arg = place;
	}
	// The path to `*r` is `r` itself: assigning it would rebind the reference.
	if let ExprKind::Deref { arg: pointer } = b.thir[b.strip(arg)].kind
		&& matches!(b.ty_of(pointer).kind(), ty::Ref(..))
	{
		let pointee = b.ty_of(arg);
		return Some(b.err(
			b.thir[arg].span,
			format!("replacing the value behind a reference to `{pointee}`"),
		));
	}
	b.visit(written);
	let place = b.place(arg, out);
	place.is_path().then_some(place)
}

/// The value at `place`, after setting `place` to `value`.
pub fn swap_out<'tcx>(b: &mut Body<'_, '_, 'tcx>, place: Expr, value: Expr, base: &str, out: &mut Vec<Stmt>) -> Expr {
	let name = b.fresh(base);
	out.push(StmtKind::decl(false, name.clone(), None, Some(place.clone())).at(None));
	out.push(StmtKind::Expr(place.assign(value)).at(None));
	Expr::ident(name)
}

/// `Default::default()` for a standard type.
pub fn default_value<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> Option<Expr> {
	let tcx = b.t.tcx;
	Some(match ty.kind() {
		ty::Bool => Expr::lit("false"),
		ty::Int(_) | ty::Uint(_) => Int::of(ty)?.lit(0),
		ty::Ref(_, inner, _) => match inner.kind() {
			ty::Slice(elem) if matches!(elem.kind(), ty::Uint(ty::UintTy::U8)) => {
				Expr::New(Box::new(Expr::ident("Uint8Array")), vec![Expr::lit("0")])
			}
			ty::Slice(_) => Expr::Array(Vec::new()),
			ty::Str => Expr::Str(String::new()),
			_ => return None,
		},
		ty::Adt(adt, args) => match Std::of(tcx, adt.did())? {
			Std::Option => Expr::undefined(),
			Std::Vec => match args.type_at(0).kind() {
				ty::Uint(ty::UintTy::U8) => Expr::New(Box::new(Expr::Rt("ByteVec")), Vec::new()),
				_ => Expr::Array(Vec::new()),
			},
			Std::String => Expr::Str(String::new()),
			_ => return None,
		},
		_ => return None,
	})
}

/// The text of a `fmt::Arguments` with only literal pieces, which writes it as is.
pub fn literal(parts: &Expr) -> Option<String> {
	let Expr::Array(parts) = parts else {
		return None;
	};
	parts
		.iter()
		.map(|part| match part {
			Expr::Str(s) => Some(s.as_str()),
			_ => None,
		})
		.collect()
}

/// The slice `id` iterates if it is `<[T]>::iter(slice)`, which iterates as the JS array
/// or `Uint8Array` itself does.
pub fn iterated_slice(b: &Body<'_, '_, '_>, id: ExprId) -> Option<ExprId> {
	let ExprKind::Call { ty: fn_ty, args, .. } = &b.thir[b.strip(id)].kind else {
		return None;
	};
	let ty::FnDef(iter, _) = *b.subst(*fn_ty).kind() else {
		return None;
	};
	(key(b.t.tcx, iter) == Some(("slice".into(), "iter".into()))).then(|| args[0])
}

/// What an `IntoIterator` iterates, as an iterable JS value: a slice, array, or `Vec`
/// itself, by value or by reference, or else an `rt.Iter`.
fn items<'tcx>(b: &mut Body<'_, '_, 'tcx>, id: ExprId, out: &mut Vec<Stmt>) -> Option<Expr> {
	let tcx = b.t.tcx;
	if let Some(slice) = iterated_slice(b, id) {
		b.visit(id);
		return Some(b.operand(slice, out));
	}
	let id = b.strip(id);
	let ty = b.ty_of(id);
	match ty.peel_refs().kind() {
		ty::Slice(_) | ty::Array(..) => Some(b.operand(id, out)),
		ty::Adt(adt, _) if Std::of(tcx, adt.did()) == Some(Std::Vec) => {
			let v = b.operand(id, out);
			Some(vec_view(v, ty))
		}
		_ => {
			let value = b.operand(id, out);
			super::iter::into_iter(b, ty, value, b.span(id))
		}
	}
}
