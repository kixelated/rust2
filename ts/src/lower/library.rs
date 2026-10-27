//! Rust's standard library: its types' representations, and its functions as runtime
//! helpers or JS. A function missing here fails translation with its path.
//!
//! Functions are keyed by their owner and name: an inherent method by its type
//! (`slice::split_first`), a trait method by its trait (`From::from`), and a free
//! function by its module (`mem::take`).

use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, GenericArgsRef, Instance, Ty, TyCtxt};
use rustc_span::Span;

use super::Translator;
use super::body::Body;
use super::fmt;
use super::num::{self, Int, Operand};
use crate::ts::{Expr, Stmt};

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
	/// What `?` branches on, as MIR spells it, and what else makes one: see [`Flow`].
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

/// How a `ControlFlow<B, C>` is represented. What `?` branches on is the value it was
/// applied to, which breaks with what can only be its `Err` or `None`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
	/// Breaking with a `Result<!, E>`: the `rt.Result<C, E>` `?` was applied to.
	Result,
	/// Breaking with an `Option<!>`: the `C | undefined` `?` was applied to.
	Option,
	/// Any other: an `rt.Result<C, B>`, `Ok` to continue and `Err` to break.
	Other,
}

impl Flow {
	/// The representation of a `ControlFlow` with `args`, its break's type first.
	pub fn of<'tcx>(tcx: TyCtxt<'tcx>, args: GenericArgsRef<'tcx>) -> Self {
		let ty::Adt(adt, residual) = args.type_at(0).kind() else {
			return Self::Other;
		};
		let uninhabited =
			|t: Ty<'tcx>| t.is_never() || matches!(t.kind(), ty::Adt(e, _) if e.is_enum() && e.variants().is_empty());
		match Std::of(tcx, adt.did()) {
			Some(Std::Result) if uninhabited(residual.type_at(0)) => Self::Result,
			Some(Std::Option) if uninhabited(residual.type_at(0)) => Self::Option,
			_ => Self::Other,
		}
	}

	/// The test that `value`, a `ControlFlow` represented so, continues, or breaks.
	pub fn test(self, value: Expr, continues: bool) -> Expr {
		match (self, continues) {
			(Self::Result | Self::Other, true) => value.member("ok"),
			(Self::Result | Self::Other, false) => value.member("ok").not(),
			(Self::Option, true) => value.binary("!==", Expr::undefined()),
			(Self::Option, false) => value.binary("===", Expr::undefined()),
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

/// If `instance` is a method of std's `PartialEq`, `PartialOrd`, `Ord`, or `Hash` impl
/// for references, which calls its referents' with the operands dereferenced, the
/// generic arguments of that call. A reference is its referent in JS, so the operands
/// pass as they are.
pub fn referent<'tcx>(
	tcx: TyCtxt<'tcx>,
	def_id: DefId,
	gargs: GenericArgsRef<'tcx>,
	instance: Instance<'tcx>,
) -> Option<GenericArgsRef<'tcx>> {
	let tr = tcx.trait_of_assoc(def_id)?;
	// A method the impl leaves to its trait's default, as `Ord::max` is, is not forwarded.
	let imp = tcx.impl_of_assoc(instance.def_id())?;
	if !is_std(tcx, imp) {
		return None;
	}
	let operands = match tcx.get_diagnostic_name(tr)?.as_str() {
		"PartialEq" | "PartialOrd" => 2,
		"Ord" | "Hash" => 1,
		_ => return None,
	};
	let mut args = gargs.to_vec();
	for arg in &mut args[..operands] {
		let ty::Ref(_, pointee, _) = arg.as_type()?.kind() else {
			return None;
		};
		*arg = (*pointee).into();
	}
	Some(tcx.mk_args(&args))
}

/// A comparison an impl of `PartialEq` or `PartialOrd` inherits from its trait, as a
/// derived one inherits all but `eq` and `partial_cmp`.
#[derive(Clone, Copy)]
pub enum Inherited {
	Ne,
	Lt,
	Le,
	Gt,
	Ge,
}

impl Inherited {
	/// The comparison `instance` is, if it is the trait's own default.
	pub fn of(tcx: TyCtxt<'_>, instance: Instance<'_>) -> Option<Self> {
		let ty::InstanceKind::Item(did) = instance.def else {
			return None;
		};
		let tr = tcx.trait_of_assoc(did)?;
		let inherited = match (tcx.get_diagnostic_name(tr)?.as_str(), tcx.item_name(did).as_str()) {
			("PartialEq", "ne") => Self::Ne,
			("PartialOrd", "lt") => Self::Lt,
			("PartialOrd", "le") => Self::Le,
			("PartialOrd", "gt") => Self::Gt,
			("PartialOrd", "ge") => Self::Ge,
			_ => return None,
		};
		Some(inherited)
	}

	/// What the default calls, with the same generic arguments and operands: `eq` or
	/// `partial_cmp`.
	pub fn via(self, tcx: TyCtxt<'_>) -> DefId {
		match self {
			Self::Ne => super::collections::trait_fn(tcx, "PartialEq", "eq"),
			_ => super::collections::trait_fn(tcx, "PartialOrd", "partial_cmp"),
		}
	}

	/// The comparison, from what `via` returned: `!eq`, or whether the `partial_cmp` is
	/// `Some` and of the order wanted, as std's defaults test it.
	pub fn test(self, b: &mut Body<'_, '_, '_>, value: Expr, out: &mut Vec<Stmt>) -> Expr {
		let (op, order) = match self {
			Self::Ne => return value.not(),
			Self::Lt => return value.binary("===", Expr::lit("-1")),
			Self::Gt => return value.binary("===", Expr::lit("1")),
			Self::Le => ("<=", "0"),
			Self::Ge => (">=", "0"),
		};
		let value = match value.is_path() {
			true => value,
			false => b.spill(value, "order", out),
		};
		let some = value.clone().binary("!==", Expr::undefined());
		some.binary("&&", value.binary(op, Expr::lit(order)))
	}
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

pub struct Call<'tcx> {
	/// The function as written, before resolving a trait method to its impl.
	pub def_id: DefId,
	/// Its generic arguments: the impl's or trait's, then the function's.
	pub args: GenericArgsRef<'tcx>,
	pub span: Span,
}

/// What the mapping of `owner::name` may discard of the operands it takes by value, rather
/// than move into its result or a closure, as it returns or as a panic unwinds it: the
/// types, by their index among the call's type arguments, whose droppers its runtime
/// function takes after its own arguments, as `unwrap_or` drops its fallback. `None` for
/// any other mapping, which is refused an operand that drops. Iterators refuse their own.
fn discards(owner: &str, name: &str) -> Option<&'static [usize]> {
	Some(match (owner, name) {
		("Option", "unwrap" | "expect")
		| ("Vec", "push")
		| ("Box", "new")
		| ("mem", "replace" | "take" | "swap")
		| ("hint", "must_use")
		| ("From", "from")
		| ("Into", "into") => &[],
		// `Option<T>::ok_or::<E>` drops its error, `unwrap_or` its fallback, and
		// `map_or::<U, F>` its fallback or its closure.
		("Option", "ok_or") => &[1],
		("Option", "unwrap_or") => &[0],
		("Option", "map_or") => &[1, 2],
		// `Option<T>::map::<U, F>` drops its closure on `None`, and `Result<T, E>`'s
		// `map::<U, F>` and `and_then::<U, F>` on an error, as `map_err::<F, O>` on a value.
		("Option", "map") => &[2],
		("Result", "map" | "map_err" | "and_then") => &[3],
		// `Result<T, E>::ok` drops an error, `err` a value, and `unwrap_or` its fallback or
		// the error.
		("Result", "ok") => &[1],
		("Result", "err") => &[0],
		("Result", "unwrap_or") => &[0, 1],
		// `unwrap` and `expect` drop the error they panic with, `unwrap_err` the value, and
		// `inspect_err` its error if its closure panics.
		("Result", "unwrap" | "expect" | "inspect_err") => &[1],
		("Result", "unwrap_err") => &[0],
		("bool", "then_some") => &[0],
		_ => return None,
	})
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
			ty::Char => "char".into(),
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

pub fn call<'tcx>(b: &mut Body<'_, '_, 'tcx>, call: &Call<'tcx>, args: Vec<Expr>, out: &mut Vec<Stmt>) -> Option<Expr> {
	let tcx = b.t.tcx;
	// thiserror's `Display` derive reads each field through `AsDisplay`, which is the
	// field itself for anything but a path.
	if tcx.crate_name(call.def_id.krate).as_str() == "thiserror"
		&& tcx.item_name(call.def_id).as_str() == "as_display"
		&& matches!(targ(call.args, 0).kind(), ty::Ref(..))
	{
		return args.into_iter().next_back();
	}
	if !is_std(tcx, call.def_id) {
		return None;
	}
	let (owner, name) = key(tcx, call.def_id)?;
	let span = call.span;
	let gargs = call.args;
	let args = match super::task::call(b, call, &owner, &name, args) {
		Ok(e) => return e,
		Err(args) => args,
	};
	let mut args = match super::collections::call(b, call, &owner, &name, args, out) {
		Ok(e) => return e,
		Err(args) => args,
	};
	let discarded = discards(&owner, &name);
	let dropping = match discarded {
		Some(_) => None,
		None => dropping_operand(b, call),
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
		return Some(int.method(method, args));
	}

	let self_ty = || gargs.types().next();
	let is_u8 = |t: Ty<'tcx>| matches!(t.kind(), ty::Uint(ty::UintTy::U8));
	let byte_slice = |t: Ty<'tcx>| matches!(t.kind(), ty::Slice(e) if is_u8(*e));
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
			let mut values = args;
			let value = values.pop()?;
			if to == from {
				return Some(value);
			}
			// Each `From` between integers and `char` keeps the value, as `as` then does.
			let scalar = |t: Ty<'tcx>| Int::of(t).is_some() || t.is_char();
			if !scalar(to) || !scalar(from) {
				return None;
			}
			let operand = Operand {
				expr: value,
				value: None,
			};
			num::cast(b, operand, from, to, span)
		}
		("Into", "into") => {
			let (from, to) = (targ(gargs, 0), targ(gargs, 1));
			let mut values = args;
			let value = values.pop()?;
			b.convert(value, from, to, span, out)
		}
		// A scalar value, but not a surrogate.
		("char", "from_u32") => Expr::rt("charFromU32", args),
		("TryFrom", "try_from") => {
			let (to, from) = (targ(gargs, 0), targ(gargs, 1));
			let (Some(int), Some(_)) = (Int::of(to), Int::of(from)) else {
				return None;
			};
			let values = args;
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
			let [l, r] = <[Expr; 2]>::try_from(args).ok()?;
			num::binary(
				b,
				op,
				Operand { expr: l, value: None },
				Operand { expr: r, value: None },
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
			let values = args;
			let [l, r] = <[Expr; 2]>::try_from(values).ok()?;
			match Int::of(targ(gargs, 0).peel_refs()) {
				Some(int) => int.compare(js, l, r),
				None => l.binary(js, r),
			}
		}
		("Ord", "max" | "min") if let Some(int) = Int::of(targ(gargs, 0)) => {
			let values = args;
			match int.big() {
				true => int.method(name.as_str(), values),
				false => Expr::rt(if name == "max" { "max" } else { "min" }, values),
			}
		}
		("Clone", "clone") => {
			let ty = targ(gargs, 0);
			let values = args;
			let value = values.into_iter().next()?;
			match ty.kind() {
				ty::Adt(adt, args) if Std::of(tcx, adt.did()) == Some(Std::Vec) && is_u8(args.type_at(0)) => {
					Expr::rt("byteVecFrom", vec![Expr::rt("byteVecView", vec![value])])
				}
				ty::Adt(adt, _) if Std::of(tcx, adt.did()) == Some(Std::String) => value,
				ty::Adt(adt, _) if int_range(ty) => {
					Expr::rt_of(range_of(Std::of(tcx, adt.did())?)?, "clone").call(vec![value])
				}
				_ if b.t.is_copy(ty) && clone_copies(b.t, ty) => b.t.copied(value, ty, span),
				_ => return None,
			}
		}
		("Default", "default") => default_value(b, targ(gargs, 0))?,

		// Options.
		("Option", "map") => rt2(args, "option", "map"),
		("Option", "map_or") => rt2(args, "option", "mapOr"),
		("Option", "unwrap_or") => rt2(args, "option", "unwrapOr"),
		("Option", "ok_or") => rt2(args, "option", "okOr"),
		("Option", "unwrap") => rt2(args, "option", "unwrap"),
		("Option", "expect") => rt2(args, "option", "expect"),
		("Option", "is_some") => args.pop()?.binary("!==", Expr::undefined()),
		("Option", "is_none") => args.pop()?.binary("===", Expr::undefined()),
		("Option", "as_ref" | "as_mut") => args.pop()?,
		("Option", "copied" | "cloned") => {
			let sig = tcx.fn_sig(call.def_id).instantiate(tcx, call.args).skip_norm_wip();
			let option = tcx.instantiate_bound_regions_with_erased(sig).output();
			let value = args.pop()?;
			b.t.copied(value, option, span)
		}

		// Results.
		("Result", "map") => rt2(args, "result", "map"),
		("Result", "map_err") => rt2(args, "result", "mapErr"),
		("Result", "and_then") => rt2(args, "result", "andThen"),
		("Result", "inspect_err") => rt2(args, "result", "inspectErr"),
		("Result", "is_ok") => args.pop()?.member("ok"),
		("Result", "is_err") => args.pop()?.member("ok").not(),
		("Result", "ok") => rt2(args, "result", "ok"),
		("Result", "err") => rt2(args, "result", "err"),
		// Their panics show the other side with its `Debug`.
		("Result", "unwrap" | "expect" | "unwrap_err") => {
			let (method, shown) = match name.as_str() {
				"unwrap" => ("unwrap", targ(gargs, 1)),
				"expect" => ("expect", targ(gargs, 1)),
				_ => ("unwrapErr", targ(gargs, 0)),
			};
			let mut values = args;
			values.push(b.fmt_fn(fmt::Trait::Debug, shown, span));
			Expr::rt_of("result", method).call(values)
		}
		("Result", "unwrap_or") => rt2(args, "result", "unwrapOr"),

		("ControlFlow", "is_continue" | "is_break") => Flow::of(tcx, gargs).test(args.pop()?, name == "is_continue"),

		// Ranges, which are their own iterators.
		// An integer range wraps within its type's bounds where std's `nth` does.
		("RangeInclusive", "new") if range => {
			let mut values = args;
			values.extend(self_ty().and_then(Int::of).map(|int| int.func("step")));
			Expr::New(Box::new(Expr::Rt("RangeInclusive".into())), values)
		}
		// JS compares integers as Rust does, and other endpoints by identity.
		("Range" | "RangeInclusive", "start" | "end" | "is_empty" | "contains")
			if range && (matches!(name.as_str(), "start" | "end") || self_ty().and_then(Int::of).is_some()) =>
		{
			let of = range_of(inherent_std(tcx, call.def_id)?)?;
			Expr::rt_of(of, &super::camel(&name)).call(args)
		}

		("bool", "then_some") => {
			let values = args;
			Expr::rt("thenSome", values)
		}

		// Slices, whose byte functions take an `rt.ByteSlice`.
		("slice", "len") => args.pop()?.member("length"),
		("slice", "is_empty") => args.pop()?.member("length").binary("===", Expr::lit("0")),
		("slice", "split_first" | "split_at" | "split_at_checked" | "first" | "get")
			if name != "get" || Int::of(targ(gargs, 1)).is_some() =>
		{
			let of = match is_u8(targ(gargs, 0)) {
				true => "byteSlice",
				false => "slice",
			};
			rt2(args, of, &super::camel(&name))
		}
		("slice", "split_first_chunk") => {
			let n = gargs.consts().next()?.try_to_target_usize(tcx)?;
			let mut values = args;
			values.push(Expr::lit(n.to_string()));
			match is_u8(targ(gargs, 0)) {
				true => Expr::rt("byteSliceSplitFirstChunk", values),
				false => Expr::rt("sliceSplitFirstChunk", values),
			}
		}
		("slice", "copy_from_slice") if is_u8(targ(gargs, 0)) => rt2(args, "byteSlice", "copyFromSlice"),
		("PartialEq", "eq") if byte_slice(targ(gargs, 0)) && byte_slice(targ(gargs, 1)) => {
			Expr::rt("byteSliceEq", args)
		}
		("slice", "to_vec") => {
			let elem = targ(gargs, 0);
			let values = args;
			match is_u8(elem) {
				true => Expr::rt("byteSliceToByteVec", values),
				false if b.t.is_copy(elem) => {
					let slice = values.into_iter().next()?;
					match b.t.copy_fn(elem, span) {
						Some(copy) => slice.member("map").call(vec![copy]),
						None => slice.member("slice").call(Vec::new()),
					}
				}
				false => return None,
			}
		}
		// A boxed slice's elements, and `vec![a, b]`'s array once `mir::vec` took the box
		// away. A `Copy` aggregate's operand may be shared with where it came from, which the
		// vector must not change.
		("slice", "into_vec") | ("boxed", "box_assume_init_into_vec_unsafe") => {
			let elem = targ(gargs, 0);
			let value = args.pop()?;
			if is_u8(elem) {
				// A boxed `[u8]` is an `rt.ByteSlice`; `vec![a, b]`'s `[u8; N]` is a `Uint8Array`.
				let f = if name == "into_vec" {
					"byteSliceToByteVec"
				} else {
					"byteVecFrom"
				};
				return Some(Expr::rt(f, vec![value]));
			}
			match (b.t.copy_fn(elem, span), value) {
				(None, value) => value,
				(Some(_), Expr::Array(items)) => {
					Expr::Array(items.into_iter().map(|item| owned(b, item, elem, span)).collect())
				}
				(Some(copy), value) => value.member("map").call(vec![copy]),
			}
		}
		("Box", "new") => args.pop()?,
		// `vec![x; n]`: `n - 1` clones of `x`, then `x` itself, which the caller gives as its
		// own, as it does whatever std keeps.
		("vec", "from_elem") => {
			let elem = targ(gargs, 0);
			let mut values = args;
			match is_u8(elem) {
				true => Expr::rt("byteVecOf", values),
				false => {
					values.push(super::collections::clone_fn(b, elem, span));
					Expr::rt("vecFromElem", values)
				}
			}
		}

		// Vectors.
		("Vec", "new") => match is_u8(targ(gargs, 0)) {
			true => Expr::New(Box::new(Expr::Rt("ByteVec".into())), Vec::new()),
			false => Expr::Array(Vec::new()),
		},
		("Vec", "with_capacity") => match is_u8(targ(gargs, 0)) {
			true => Expr::New(Box::new(Expr::Rt("ByteVec".into())), args),
			false => Expr::Array(Vec::new()),
		},
		("Vec", "push") => {
			let mut values = args;
			match is_u8(targ(gargs, 0)) {
				true => Expr::rt("byteVecPush", values),
				false => {
					let v = values.remove(0);
					v.method("push", values)
				}
			}
		}
		("Vec", "pop") => match is_u8(targ(gargs, 0)) {
			true => Expr::rt("byteVecPop", args),
			false => args.pop()?.method("pop", vec![]),
		},
		("Vec", "len") => args.pop()?.member("length"),
		("Vec", "is_empty") => args.pop()?.member("length").binary("===", Expr::lit("0")),
		("Vec", "truncate") if is_u8(targ(gargs, 0)) => Expr::rt("byteVecTruncate", args),
		("Vec", "clear") if is_u8(targ(gargs, 0)) => Expr::rt("byteVecClear", args),
		("Vec", "extend_from_slice") if is_u8(targ(gargs, 0)) => Expr::rt("byteVecExtendFromSlice", args),
		("Vec", "as_slice" | "as_mut_slice") => {
			let v = args.pop()?;
			vec_view(v, targ(gargs, 0))
		}
		("Deref", "deref") | ("DerefMut", "deref_mut") => {
			let ty = targ(gargs, 0);
			let v = args.pop()?;
			match ty.kind() {
				ty::Adt(adt, _) => match Std::of(tcx, adt.did()) {
					Some(Std::Vec) => vec_view(v, ty),
					Some(Std::String | Std::Box) => v,
					_ => return None,
				},
				_ => return None,
			}
		}

		// Strings.
		("String" | "str", "as_bytes") => Expr::rt("stringAsBytes", args),
		("String", "from_utf8") => Expr::rt("stringFromUtf8", args),
		("String" | "str", "len") => Expr::rt("stringLen", args),
		("String", "as_str") => args.pop()?,
		("String", "new") => Expr::Str(String::new()),
		("String" | "str", "is_empty") => args.pop()?.member("length").binary("===", Expr::lit("0")),
		("str", "chars") => Expr::rt("strChars", args),
		// Searches by a pattern.
		("str", "split" | "match_indices" | "contains" | "starts_with" | "rfind")
		| ("str", "trim_start_matches" | "trim_end_matches") => {
			let [s, p] = <[Expr; 2]>::try_from(args).ok()?;
			Expr::rt_of("str", &super::camel(&name)).call(vec![s, pattern(b.t, targ(gargs, 0), p)?])
		}
		// A slice of strings joined by a `&str`.
		("slice", "join") if is_text(b.t, targ(gargs, 0)) && targ(gargs, 1).peel_refs().is_str() => {
			let [parts, sep] = <[Expr; 2]>::try_from(args).ok()?;
			parts.method("join", vec![sep])
		}
		("char", "is_ascii_digit") => Expr::rt("charIsAsciiDigit", args),
		("ToString", "to_string") | ("ToOwned", "to_owned") if matches!(self_ty()?.peel_refs().kind(), ty::Str) => {
			args.pop()?
		}

		// Panics, which format their message as std does.
		("panicking", "panic_fmt") => {
			let message = args.pop()?;
			match literal(&message) {
				Some(s) => Expr::rt("panic", vec![Expr::Str(s)]),
				None => Expr::rt("panicFmt", vec![message]),
			}
		}
		("panicking", "panic") | ("rt", "begin_panic") => Expr::rt("panic", args),
		("panicking", "panic_explicit") => Expr::rt("panic", vec![Expr::Str("explicit panic".into())]),
		("panicking", "panic_display" | "unreachable_display") => {
			let value = args.pop()?;
			let value = b.fmt_value(fmt::Trait::Display, targ(gargs, 0), value, span, out);
			let mut message = vec![value];
			if name == "unreachable_display" {
				message.insert(0, Expr::Str("internal error: entered unreachable code: ".into()));
			}
			Expr::rt("panicFmt", vec![Expr::Array(message)])
		}
		// `assert_eq!` and `assert_ne!`: the operator, both sides, and any message.
		("panicking", "assert_failed") => {
			let [kind, left, right, message] = <[Expr; 4]>::try_from(args).ok()?;
			let left = b.fmt_value(fmt::Trait::Debug, targ(gargs, 0), left, span, out);
			let right = b.fmt_value(fmt::Trait::Debug, targ(gargs, 1), right, span, out);
			let mut values = vec![kind, left, right];
			if !matches!(&message, Expr::Lit(text) if text == "undefined") {
				values.push(message);
			}
			Expr::rt("assertFailed", values)
		}

		// Formatting.
		("hint", "must_use") => args.pop()?,
		("fmt", "format") => {
			let parts = args.pop()?;
			match literal(&parts) {
				Some(s) => Expr::Str(s),
				None => Expr::rt("fmtFormat", vec![parts]),
			}
		}
		("Display" | "Debug" | "LowerHex" | "UpperHex", "fmt") => {
			let tr = fmt::Trait::of(tcx, tcx.trait_of_assoc(call.def_id)?)?;
			let [value, f] = <[Expr; 2]>::try_from(args).ok()?;
			fmt::std_impl(b, tr, targ(gargs, 0), value, f, span, out)?
		}
		("ToString", "to_string") => {
			let ty = targ(gargs, 0);
			let value = args.pop()?;
			match ty.kind() {
				ty::Adt(adt, _) if Std::of(tcx, adt.did()) == Some(Std::String) => value,
				_ => {
					let value = b.fmt_value(fmt::Trait::Display, ty, value, span, out);
					Expr::rt("fmtToString", vec![value])
				}
			}
		}
		("Formatter", "write_fmt") => {
			let [f, parts] = <[Expr; 2]>::try_from(args).ok()?;
			match literal(&parts) {
				Some(s) => Expr::rt("fmtWriteStr", vec![f, Expr::Str(s)]),
				None => Expr::rt("fmtWriteFmt", vec![f, parts]),
			}
		}
		// Each takes its receiver first: `rt.fmtPad(f, s)`, `rt.debugStructField(b, name, value)`.
		("Formatter", "write_str" | "pad" | "alternate" | "width" | "debug_struct" | "debug_tuple" | "debug_list")
		| ("DebugStruct" | "DebugTuple", "field" | "finish" | "finish_non_exhaustive")
		| ("DebugList", "entry" | "finish" | "finish_non_exhaustive") => {
			let of = match owner.as_str() {
				"Formatter" => "fmt",
				"DebugStruct" => "debugStruct",
				"DebugTuple" => "debugTuple",
				_ => "debugList",
			};
			Expr::rt_of(of, &super::camel(&name)).call(args)
		}
		// Derived `Debug`: `debug_struct_field2_finish(f, name, "a", &a, "b", &b)`.
		("Formatter", "debug_struct_fields_finish") => Expr::rt("fmtDebugStructFields", args),
		("Formatter", "debug_tuple_fields_finish") => Expr::rt("fmtDebugTupleFields", args),
		("Formatter", n) if n.starts_with("debug_struct_field") || n.starts_with("debug_tuple_field") => {
			let mut values = args.into_iter();
			let (f, type_name) = (values.next()?, values.next()?);
			let structure = n.starts_with("debug_struct");
			let (of, start) = match structure {
				true => ("debugStruct", "fmtDebugStruct"),
				false => ("debugTuple", "fmtDebugTuple"),
			};
			let mut builder = Expr::rt(start, vec![f, type_name]);
			let values: Vec<Expr> = values.collect();
			let fields = match structure {
				true => values.chunks(2).map(<[Expr]>::to_vec).collect::<Vec<_>>(),
				false => values.into_iter().map(|v| vec![v]).collect(),
			};
			for field in fields {
				builder = Expr::rt_of(of, "field").call([vec![builder], field].concat());
			}
			Expr::rt_of(of, "finish").call(vec![builder])
		}
		_ => return super::iter::call(b, call, &owner, &name, args),
	};
	if let Some(ty) = dropping {
		return Some(b.err(
			span,
			format!("passing `{ty}`, which drops, to `{owner}::{name}`, which is not supported yet"),
		));
	}
	// What drops each operand the mapping may discard.
	let lost = discarded.unwrap_or_default().iter().map(|&n| targ(gargs, n));
	let Some(droppers) = b.t.droppers(lost, span) else {
		return Some(Expr::undefined());
	};
	Some(match e {
		Expr::Call(f, mut args) if !droppers.is_empty() => {
			args.extend(droppers);
			Expr::Call(f, args)
		}
		e => e,
	})
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

/// `value`, a `ty`, as a value of its own: copied if it is a `Copy` aggregate that is not
/// made right here.
fn owned<'tcx>(b: &mut Body<'_, '_, 'tcx>, value: Expr, ty: Ty<'tcx>, span: Span) -> Expr {
	fn made(e: &Expr) -> bool {
		match e {
			Expr::Array(_) | Expr::Object(_) | Expr::New(..) => true,
			Expr::As(e, _) | Expr::Located(_, e) => made(e),
			_ => false,
		}
	}
	match made(&value) {
		true => value,
		false => b.t.copied(value, ty, span),
	}
}

/// A `str` method's pattern as the runtime takes it, an `rt.Pattern`: a `char`'s text,
/// or a string or a predicate as it is. `None` for any other pattern.
fn pattern<'tcx>(t: &Translator<'_, 'tcx>, ty: Ty<'tcx>, p: Expr) -> Option<Expr> {
	match ty.kind() {
		ty::Char => Some(char_text(p)),
		ty::Closure(..) | ty::FnDef(..) => Some(p),
		ty::Ref(..) if is_text(t, ty) => Some(p),
		_ => None,
	}
}

/// Whether `ty` is a `str` or a `String`, behind any references: a JS string.
fn is_text<'tcx>(t: &Translator<'_, 'tcx>, ty: Ty<'tcx>) -> bool {
	let ty = ty.peel_refs();
	ty.is_str() || t.std_of(ty) == Some(Std::String)
}

/// A `char`'s text: a literal for a constant, or `String.fromCodePoint(c)`.
pub fn char_text(c: Expr) -> Expr {
	if let Expr::Lit(n) = &c
		&& let Some(c) = n.parse().ok().and_then(char::from_u32)
	{
		return Expr::Str(c.to_string());
	}
	Expr::ident("String").member("fromCodePoint").call(vec![c])
}

/// `rt.<of><Name>(args)`.
fn rt2(args: Vec<Expr>, of: &str, name: &str) -> Expr {
	Expr::rt_of(of, name).call(args)
}

/// What the runtime's functions of a std range type start with.
fn range_of(std: Std) -> Option<&'static str> {
	match std {
		Std::Range => Some("range"),
		Std::RangeInclusive => Some("rangeInclusive"),
		_ => None,
	}
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

/// A `Vec`'s elements as a slice: an `rt.ByteSlice` of a byte vector, or the array itself.
pub fn vec_view(v: Expr, ty: Ty<'_>) -> Expr {
	let elem = match ty.peel_refs().kind() {
		ty::Adt(_, args) => args.type_at(0),
		_ => ty,
	};
	match elem.kind() {
		ty::Uint(ty::UintTy::U8) => Expr::rt("byteVecSlice", vec![v]),
		_ => v,
	}
}

/// The elements of a slice, array, or `Vec`, a `ty`, as a JS array or `Uint8Array`: a
/// view of a byte slice's or byte vector's, which the runtime's iterators and formatting
/// index.
pub fn elements(v: Expr, ty: Ty<'_>) -> Expr {
	match ty.peel_refs().kind() {
		ty::Slice(elem) if matches!(elem.kind(), ty::Uint(ty::UintTy::U8)) => Expr::rt("byteSliceView", vec![v]),
		ty::Adt(_, args)
			if args
				.types()
				.next()
				.is_some_and(|t| matches!(t.kind(), ty::Uint(ty::UintTy::U8))) =>
		{
			Expr::rt("byteVecView", vec![v])
		}
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

/// `Default::default()` for a standard type.
pub fn default_value<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> Option<Expr> {
	let tcx = b.t.tcx;
	Some(match ty.kind() {
		ty::Bool => Expr::lit("false"),
		ty::Int(_) | ty::Uint(_) => Int::of(ty)?.lit(0),
		ty::Char => Expr::lit("0"),
		ty::Ref(_, inner, _) => match inner.kind() {
			ty::Slice(elem) if matches!(elem.kind(), ty::Uint(ty::UintTy::U8)) => Expr::rt(
				"byteSliceFrom",
				vec![Expr::New(Box::new(Expr::ident("Uint8Array")), vec![Expr::lit("0")])],
			),
			ty::Slice(_) => Expr::Array(Vec::new()),
			ty::Str => Expr::Str(String::new()),
			_ => return None,
		},
		ty::Adt(adt, args) => match Std::of(tcx, adt.did())? {
			Std::Option => Expr::undefined(),
			Std::Vec => match args.type_at(0).kind() {
				ty::Uint(ty::UintTy::U8) => Expr::New(Box::new(Expr::Rt("ByteVec".into())), Vec::new()),
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
