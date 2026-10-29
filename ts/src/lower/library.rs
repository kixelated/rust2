//! Rust's standard library: its types' representations, and its functions as runtime
//! helpers or JS. A function missing here fails translation with its path.
//!
//! Functions are keyed by their owner and name: an inherent method by its type
//! (`slice::split_first`), a trait method by its trait (`From::from`), and a free
//! function by its module (`mem::take`).

use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, GenericArgsRef, Instance, Ty, TyCtxt};
use rustc_span::Span;

use super::Translator;
use super::body::Body;
use super::fmt;
use super::num::{self, Int, Operand};
use crate::ts::{Expr, Stmt, StmtKind};

/// A type with a representation of its own: std's, or a well-known crate's that
/// [`super::stock`] maps.
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
	/// A JS string too, for `Cow<str>`, which keeps no variant: either is the text.
	Cow,
	/// Its contents.
	Box,
	FromUtf8Error,
	Utf8Error,
	TryFromIntError,
	TryFromFloatSecsError,
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
	/// `rt.Arc`, counted: an `Arc` or an `Rc`, which one thread can't tell apart.
	Arc,
	/// `rt.Weak`, of either.
	Weak,
	/// `rt.Mutex`, which is also its own guard.
	Mutex,
	MutexGuard,
	/// What it holds, a lock's guard or value.
	PoisonError,
	/// `rt.TryLockError<T>`, tagged as `Bound` is.
	TryLockError,
	/// `rt.Atomic<T>`, a place holding its `bool` or integer.
	Atomic,
	/// `rt.AtomicOrdering`: its discriminant.
	AtomicOrdering,
	/// `rt.OnceCell<T>` and `rt.OnceLock<T>`: the `Option<T>` each holds.
	OnceCell,
	OnceLock,
	/// `rt.Waker`.
	Waker,
	/// `rt.Context`: its waker.
	Context,
	/// `rt.Poll<T>`: `T` itself, or `rt.Pending`.
	Poll,
	/// Its pointer, or the value `pin!` pins in a `PinMacroHelper`.
	Pin,
	/// `rt.PollFn`: `poll_fn`'s future, which owns its function.
	PollFn,
	/// `future::Pending`: `rt.NEVER`, a future never ready.
	Pending,
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
	/// `..=end`: `{ end, inclusive: true }`.
	RangeToInclusive,
	/// `rt.RangeBound<T>`, tagged as the crate's own enums are: `{ tag: "Included", _0 }`.
	Bound,
	/// What it holds, which orders the other way.
	Reverse,
	/// Its integer, which is never 0.
	NonZero,
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
	/// `rt.Duration`: its seconds and nanoseconds, never changed once made.
	Duration,
	/// `bytes::Bytes`: an `rt.ByteSlice`, as `&[u8]` is.
	Bytes,
	/// `rand::rngs::ThreadRng`: `rt.ThreadRng`, which holds no state.
	ThreadRng,
}

impl Std {
	pub fn of(tcx: TyCtxt<'_>, did: DefId) -> Option<Self> {
		if !is_std(tcx, did) {
			return super::stock::of(tcx, did);
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
		if tcx.is_diagnostic_item(sym::Cow, did) {
			return Some(Self::Cow);
		}
		for (item, std) in [
			(sym::Arc, Self::Arc),
			(sym::ArcWeak, Self::Weak),
			(sym::Rc, Self::Arc),
			(sym::RcWeak, Self::Weak),
			(sym::Mutex, Self::Mutex),
			(sym::MutexGuard, Self::MutexGuard),
			(rustc_span::Symbol::intern("Atomic"), Self::Atomic),
			(rustc_span::Symbol::intern("Waker"), Self::Waker),
			(rustc_span::Symbol::intern("PinMacroHelper"), Self::Pin),
			(rustc_span::Symbol::intern("HashMap"), Self::HashMap),
			(rustc_span::Symbol::intern("HashSet"), Self::HashSet),
			(rustc_span::Symbol::intern("BTreeMap"), Self::BTreeMap),
			(rustc_span::Symbol::intern("BTreeSet"), Self::BTreeSet),
			(rustc_span::Symbol::intern("VecDeque"), Self::VecDeque),
			(rustc_span::Symbol::intern("Duration"), Self::Duration),
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
			(rustc_attr_ir::LangItem::RangeToInclusive, Self::RangeToInclusive),
		] {
			if tcx.is_lang_item(did, item) {
				return Some(std);
			}
		}
		match tcx.item_name(did).as_str() {
			"FromUtf8Error" => return Some(Self::FromUtf8Error),
			"TryFromIntError" => return Some(Self::TryFromIntError),
			"TryFromFloatSecsError" => return Some(Self::TryFromFloatSecsError),
			"PoisonError" => return Some(Self::PoisonError),
			"TryLockError" => return Some(Self::TryLockError),
			_ => {}
		}
		match std_path(tcx, did)
			.iter()
			.map(String::as_str)
			.collect::<Vec<_>>()
			.as_slice()
		{
			["str", "error", "Utf8Error"] => Some(Self::Utf8Error),
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
			["future", "poll_fn", "PollFn"] => Some(Self::PollFn),
			["future", "pending", "Pending"] => Some(Self::Pending),
			["convert", "Infallible"] => Some(Self::Infallible),
			["cmp", "Ordering"] => Some(Self::Ordering),
			["sync", "atomic", "Ordering"] => Some(Self::AtomicOrdering),
			["cell", "once", "OnceCell"] => Some(Self::OnceCell),
			["sync", "once_lock", "OnceLock"] => Some(Self::OnceLock),
			["cmp", "Reverse"] => Some(Self::Reverse),
			["ops", "range", "Bound"] => Some(Self::Bound),
			["num", "nonzero", "NonZero"] => Some(Self::NonZero),
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
	/// Breaking with an `Option<!>`: the `Option<C>` `?` was applied to.
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

	/// The test that `value`, a `ControlFlow` with `args`, continues, or breaks.
	pub fn test<'tcx>(t: &Translator<'_, 'tcx>, args: GenericArgsRef<'tcx>, value: Expr, continues: bool) -> Expr {
		match (Self::of(t.tcx, args), continues) {
			(Self::Result | Self::Other, true) => value.member("ok"),
			(Self::Result | Self::Other, false) => value.member("ok").not(),
			(Self::Option, true) => value.binary("!==", t.none(args.type_at(1))),
			(Self::Option, false) => value.binary("===", t.none(args.type_at(1))),
		}
	}
}

/// Whether a `ty`'s value is a JS string: a `str`, a `String`, or a `Cow<str>`.
pub fn string<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
	match ty.kind() {
		ty::Str => true,
		ty::Adt(adt, args) => match Std::of(tcx, adt.did()) {
			Some(Std::String) => true,
			// Its first argument is its lifetime.
			Some(Std::Cow) => args.types().next().is_some_and(|t| t.is_str()),
			_ => false,
		},
		_ => false,
	}
}

/// What a value of `ty` is in JS if `ty` wraps one value it is represented as: `Reverse<T>`'s
/// and `NonZero<T>`'s `T`, which differ only in their order and range, a pattern type's
/// base, and the field of each of core's `niche_types` structs that hold one.
pub fn wrapped<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
	match ty.kind() {
		ty::Pat(base, _) => Some(*base),
		ty::Adt(adt, args) => match Std::of(tcx, adt.did()) {
			Some(Std::Reverse | Std::NonZero) => Some(args.type_at(0)),
			Some(_) => None,
			None => {
				let path = std_path(tcx, adt.did());
				if !(path.len() == 3 && path[0] == "num" && path[1] == "niche_types" && adt.is_struct()) {
					return None;
				}
				let field = adt.non_enum_variant().fields.iter().next()?;
				Some(field.ty(tcx, args).skip_norm_wip())
			}
		},
		_ => None,
	}
}

/// A `Bound` or a `TryLockError` of type `ty`, tagged as the crate's own enums are: its
/// variant, by name, and the value it holds, if any.
pub fn tagged(variant: &str, value: Option<Expr>, ty: crate::ts::Type) -> Expr {
	let mut props = vec![(crate::ts::Key::Name("tag".into()), Expr::Str(variant.into()))];
	props.extend(value.map(|v| (crate::ts::Key::Name("_0".into()), v)));
	Expr::As(Box::new(Expr::Object(props)), ty)
}

/// `..end`, or `..=end` if `inclusive`: `{ end }`, or `{ end, inclusive: true }`.
pub fn range_to(end: Expr, inclusive: bool) -> Expr {
	let mut props = vec![(crate::ts::Key::Name("end".into()), end)];
	if inclusive {
		props.push((crate::ts::Key::Name("inclusive".into()), Expr::lit("true")));
	}
	Expr::Object(props)
}

/// The refusal of code that tells a `Cow<str>`'s variants apart.
pub fn cow_variant(ty: Ty<'_>) -> String {
	format!("a variant of `{ty}`, which is not supported: either is the same JS string")
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

pub fn is_std(tcx: TyCtxt<'_>, did: DefId) -> bool {
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
		// A binding's `Clone` is its own.
		ty::Adt(adt, _) if t.bound_drops(adt.did()).is_some() => false,
		ty::Adt(adt, args) if !t.crates.translates(adt.did()) => args.types().all(|arg| clone_copies(t, arg)),
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

/// Whether std's `instance` lowers from its own MIR, as the crate's functions do, rather
/// than mapping here: a method of `bool`, `Option`, `Result`, or `Poll`, or their `Clone`.
/// Not one taking a `&mut` or a raw pointer, whose writes the crate's analysis of what it
/// writes into would miss, nor an `unsafe` one, nor one mapped below as one expression or
/// as the runtime's panic.
pub fn from_mir<'tcx>(t: &Translator<'_, 'tcx>, instance: Instance<'tcx>) -> bool {
	let tcx = t.tcx;
	let ty::InstanceKind::Item(did) = instance.def else {
		return false;
	};
	if !is_std(tcx, did) || tcx.def_kind(did) != DefKind::AssocFn {
		return false;
	}
	let Some(imp) = tcx.impl_of_assoc(did) else {
		return false;
	};
	// `None` for `bool`'s.
	let std = match tcx.type_of(imp).instantiate_identity().skip_norm_wip().kind() {
		ty::Adt(adt, _) => match Std::of(tcx, adt.did()) {
			std @ Some(Std::Option | Std::Result | Std::Poll) => std,
			_ => return false,
		},
		ty::Bool => None,
		_ => return false,
	};
	let sig = tcx.fn_sig(did).instantiate_identity().skip_norm_wip().skip_binder();
	let writes = |ty: Ty<'tcx>| {
		ty.walk()
			.filter_map(|arg| arg.as_type())
			.any(|t| matches!(t.kind(), ty::Ref(_, _, ty::Mutability::Mut) | ty::RawPtr(..)))
	};
	if sig.safety().is_unsafe() || sig.inputs().iter().any(|&ty| writes(ty)) {
		return false;
	}
	let name = tcx.item_name(did);
	if let Some(tr) = tcx.trait_impl_of_assoc(did).map(|imp| tcx.impl_trait_id(imp)) {
		// A `Clone` that only copies is the copy.
		return tcx.is_lang_item(tr, rustc_attr_ir::LangItem::Clone)
			&& name == rustc_span::sym::clone
			&& t.self_ty(instance)
				.is_none_or(|ty| !(t.is_copy(ty) && clone_copies(t, ty)));
	}
	// A lock's `unwrap_or_else` maps, to unwrap its guard in place.
	let lock = std == Some(Std::Result)
		&& name.as_str() == "unwrap_or_else"
		&& matches!(instance.args.type_at(1).kind(), ty::Adt(adt, _) if Std::of(tcx, adt.did()) == Some(Std::PoisonError));
	!matches!(
		(std, name.as_str()),
		(
			Some(Std::Option),
			"unwrap" | "expect" | "is_some" | "is_none" | "as_ref" | "copied" | "cloned" | "iter"
		) | (
			Some(Std::Result),
			"unwrap" | "expect" | "unwrap_err" | "is_ok" | "is_err"
		)
	) && !lock
}

pub struct Call<'tcx> {
	/// The function as written, before resolving a trait method to its impl.
	pub def_id: DefId,
	/// Its generic arguments: the impl's or trait's, then the function's.
	pub args: GenericArgsRef<'tcx>,
	pub span: Span,
}

/// What the mapping of `owner::name` may discard of the operands it takes by value, rather
/// than move into its result or a closure, as it returns: the types, by their index among
/// the call's type arguments, whose droppers its runtime function takes after its own
/// arguments, as `unwrap_or` drops its fallback. `None` for any other mapping, which is
/// refused an operand that drops. Iterators refuse their own.
fn discards(owner: &str, name: &str) -> Option<&'static [usize]> {
	Some(match (owner, name) {
		("Option", "unwrap" | "expect")
		| ("Vec", "push")
		| ("Box", "new" | "pin")
		| ("mem", "replace" | "take" | "swap")
		| ("hint", "must_use")
		| ("From", "from")
		| ("Into", "into")
		// What a panic would drop, which aborts.
		| ("Result", "unwrap" | "expect" | "unwrap_err")
		| ("Vec", "insert") => &[],
		// `Vec<T>` drops each element `retain` rejects, `dedup` removes, and `truncate` and
		// `clear` cut.
		("Vec", "retain" | "retain_mut" | "dedup" | "dedup_by" | "dedup_by_key" | "truncate" | "clear") => &[0],
		// `get_or_init::<T, F>` drops its function uncalled if the cell is set.
		("OnceCell" | "OnceLock", "get_or_init") => &[1],
		_ => return None,
	})
}

/// Whether the mapping of `owner::name` tells an `Option`'s `None` from its `Some` where
/// the `None` is `rt.None`, as the runtime's functions that take `None` to be `undefined`
/// do not.
fn takes_none(owner: &str, name: &str) -> bool {
	none_last(owner, name)
		|| matches!(
			(owner, name),
			(
				"Option",
				"is_some" | "is_none" | "as_ref" | "copied" | "cloned" | "iter" | "iter_mut"
			) | ("Iterator", "next")
				| (
					"OnceCell" | "OnceLock",
					"get" | "set" | "get_or_init" | "take" | "into_inner"
				)
				| ("DoubleEndedIterator", "next_back" | "nth_back")
		)
}

/// Whether the runtime function `owner::name` maps to takes the `None` of the `Option` it
/// makes or reads, where that is `rt.None`, last: after what drops its operands.
pub(super) fn none_last(owner: &str, name: &str) -> bool {
	matches!(
		(owner, name),
		("Option", "unwrap" | "expect")
			| ("slice", "first" | "get")
			| ("HashMap" | "BTreeMap", "get" | "get_mut" | "insert" | "remove")
	)
}

/// The type of the first `Some` whose `Option`'s `None` is `rt.None`, of those in the
/// signature and bounds of the function `call` calls, over its type arguments: what the
/// mapping of a std function would make or read with `undefined` for `None`.
pub(super) fn sentinel_option<'tcx>(t: &Translator<'_, 'tcx>, call: &Call<'tcx>) -> Option<Ty<'tcx>> {
	let tcx = t.tcx;
	let sig = tcx.fn_sig(call.def_id).instantiate_identity().skip_norm_wip();
	let sig = tcx.instantiate_bound_regions_with_erased(sig);
	let bounds = tcx.param_env(call.def_id).caller_bounds();
	let bound_args = bounds.flat_map(|clause| {
		let trait_args = clause.as_trait_clause().map(|c| c.skip_binder().trait_ref.args);
		let projection = clause.as_projection_clause().map(|c| c.skip_binder());
		let projection_args = projection.map(|p| p.projection_term.args);
		let term = projection.and_then(|p| p.term.as_type());
		(trait_args.into_iter().chain(projection_args))
			.flat_map(|args| args.types().collect::<Vec<_>>())
			.chain(term)
	});
	let env = t.typing_env_pub();
	sig.inputs_and_output
		.iter()
		.chain(bound_args)
		.flat_map(|ty| ty.walk().filter_map(|arg| arg.as_type()).collect::<Vec<_>>())
		.filter_map(|ty| match ty.kind() {
			ty::Adt(adt, args) if Std::of(tcx, adt.did()) == Some(Std::Option) => Some(args.type_at(0)),
			_ => None,
		})
		.map(|inner| {
			let inner = ty::EarlyBinder::bind(tcx, inner)
				.instantiate(tcx, call.args)
				.skip_norm_wip();
			let inner = tcx.erase_and_anonymize_regions(inner);
			tcx.try_normalize_erasing_regions(env, ty::Unnormalized::new_wip(inner))
				.unwrap_or(inner)
		})
		.find(|&inner| t.erases_to(inner).undefined)
}

/// What a std function does with the closures it calls that own a value that drops.
#[derive(Default)]
struct Closures<'tcx> {
	/// The first it would neither drop where std's does nor keep: refused.
	leaked: Option<Ty<'tcx>>,
	/// Those it drops as it returns, having kept nothing of them, in the order
	/// their parameters are declared.
	spent: Vec<Expr>,
}

/// The closures `call` calls that own a value that drops, by what its mapping does with
/// each: kept by the iterator it returns, if `iter::holds_closure`; dropped as it returns, if
/// `Fn` or `FnMut` call it and nothing else it takes by value drops, as then nothing it
/// drops is ordered against it; dropped where `discarded` says, by the type's index among
/// the call's type arguments; or else leaked. A closure that `Fn` or `FnMut` call, given to
/// one that calls it through `FnOnce`, becomes `rt.once` of it, which drops it after the
/// call, as rustc's shim does.
fn called_closures<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	call: &Call<'tcx>,
	discarded: Option<&[usize]>,
	args: &mut [Expr],
) -> Closures<'tcx> {
	let tcx = b.t.tcx;
	let mut closures = Closures::default();
	let sig = tcx.fn_sig(call.def_id).instantiate_identity().skip_norm_wip();
	let sig = tcx.instantiate_bound_regions_with_erased(sig);
	let inputs = sig.inputs().to_vec();
	let output = sig.output();
	for (i, &input) in inputs.iter().enumerate() {
		let ty::Param(param) = *input.kind() else {
			continue;
		};
		let ty = call.args[param.index as usize].expect_ty();
		let ty::Closure(_, closure) = ty.kind() else {
			continue;
		};
		let Some(bound) = called_by(tcx, call.def_id, input) else {
			continue;
		};
		if !b.t.drops(ty) {
			continue;
		}
		let mentions = |t: Ty<'tcx>| t.walk().any(|a| a.as_type() == Some(input));
		if mentions(output) {
			// An `rt.Iter` or `rt.PollFn` drops the closure it holds.
			let held = matches!(output.kind(), ty::Adt(adt, args)
				if (super::iter::holds_closure(tcx, adt.did()) || Std::of(tcx, adt.did()) == Some(Std::PollFn))
					&& args.types().any(|a| a == input));
			if !held {
				closures.leaked = closures.leaked.or(Some(ty));
			}
			continue;
		}
		let kept = inputs.iter().enumerate().any(|(j, &t)| j != i && mentions(t));
		if bound != ty::ClosureKind::FnOnce && !kept && i < args.len() {
			match dropping_operand(b, call) {
				None => closures.spent.push(args[i].clone()),
				Some(_) => closures.leaked = closures.leaked.or(Some(ty)),
			}
			continue;
		}
		let index = call.args[..param.index as usize]
			.iter()
			.filter(|a| a.as_type().is_some())
			.count();
		if !discarded.is_some_and(|d| d.contains(&index)) {
			closures.leaked = closures.leaked.or(Some(ty));
			continue;
		}
		if bound == ty::ClosureKind::FnOnce && closure.as_closure().kind() != ty::ClosureKind::FnOnce && i < args.len()
		{
			let f = std::mem::replace(&mut args[i], Expr::undefined());
			args[i] = Expr::rt("once", vec![f]);
		}
	}
	closures
}

/// The trait `def_id` calls its parameter of type `input` by: `Fn`, then `FnMut`,
/// whichever it is bound by, or else `FnOnce`, or `None` if no function trait bounds it.
fn called_by<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, input: Ty<'tcx>) -> Option<ty::ClosureKind> {
	tcx.param_env(def_id)
		.caller_bounds()
		.filter_map(|p| p.as_trait_clause())
		.filter(|t| t.self_ty().skip_binder() == input)
		.filter_map(|t| tcx.fn_trait_kind_from_def_id(t.def_id()))
		.min_by_key(|kind| match kind {
			ty::ClosureKind::Fn => 0,
			ty::ClosureKind::FnMut => 1,
			ty::ClosureKind::FnOnce => 2,
		})
}

/// The first type `call` takes by value that drops, but for a closure a function trait
/// bounds, which `called_closures` decides.
fn dropping_operand<'tcx>(b: &mut Body<'_, '_, 'tcx>, call: &Call<'tcx>) -> Option<Ty<'tcx>> {
	let tcx = b.t.tcx;
	let generic = tcx.fn_sig(call.def_id).instantiate_identity().skip_norm_wip();
	let generic = tcx.instantiate_bound_regions_with_erased(generic).inputs().to_vec();
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
	let called = |i: usize, ty: Ty<'tcx>| {
		matches!(ty.kind(), ty::Closure(..)) && called_by(tcx, call.def_id, generic[i]).is_some()
	};
	inputs
		.into_iter()
		.enumerate()
		.find(|&(i, ty)| !called(i, ty) && b.t.drops(ty))
		.map(|(_, ty)| ty)
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
			ty::Int(_) | ty::Uint(_) | ty::Float(_) => self_ty.to_string(),
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
	let args = match super::stock::call(b, call, args, out) {
		Ok(e) => return e,
		Err(args) => args,
	};
	if !is_std(tcx, call.def_id) {
		return None;
	}
	let (owner, name) = key(tcx, call.def_id)?;
	if !takes_none(&owner, &name)
		&& let Some(inner) = sentinel_option(b.t, call)
	{
		return Some(b.err(
			call.span,
			format!("`{owner}::{name}` of `Option<{inner}>`, whose `None` is `rt.None`, which is not supported yet"),
		));
	}
	let discarded = discards(&owner, &name);
	let mut args = args;
	let closures = called_closures(b, call, discarded, &mut args);
	let mut e = mapping(b, call, owner.clone(), name.clone(), discarded, args, out)?;
	if let Some(ty) = closures.leaked {
		return Some(b.err(
			call.span,
			format!(
				"passing `{ty}`, a closure that owns a value that drops, to `{owner}::{name}`, which is not supported yet"
			),
		));
	}
	// The last parameter drops first, as a function's do.
	for f in closures.spent.into_iter().rev() {
		e = Expr::rt("dropAfter", vec![f, Expr::Lambda(Vec::new(), Box::new(e))]);
	}
	Some(e)
}

/// The mapping of a call of std's `owner::name`, which may discard what `discarded` says.
fn mapping<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	call: &Call<'tcx>,
	owner: String,
	name: String,
	discarded: Option<&'static [usize]>,
	args: Vec<Expr>,
	out: &mut Vec<Stmt>,
) -> Option<Expr> {
	let tcx = b.t.tcx;
	let span = call.span;
	let gargs = call.args;
	let args = match super::task::call(b, call, &owner, &name, args, out) {
		Ok(e) => return e,
		Err(args) => args,
	};
	let args = match super::time::call(b, call, &owner, &name, args) {
		Ok(e) => return e,
		Err(args) => args,
	};
	let mut args = match super::collections::call(b, call, &owner, &name, args, out) {
		Ok(e) => return e,
		Err(args) => args,
	};
	let dropping = match discarded {
		Some(_) => None,
		None => dropping_operand(b, call),
	};

	// Methods on integers.
	if let Some(int) = int_named(&owner) {
		let method = match name.as_str() {
			"from_be_bytes" => "fromBeBytes",
			"to_be_bytes" | "to_le_bytes" => "toBeBytes",
			"leading_ones" => "leadingOnes",
			"leading_zeros" => "leadingZeros",
			"checked_add" => "checkedAdd",
			"checked_sub" => "checkedSub",
			"checked_mul" => "checkedMul",
			"saturating_add" => "saturatingAdd",
			"saturating_sub" => "saturatingSub",
			"saturating_mul" => "saturatingMul",
			"wrapping_add" => "wrappingAdd",
			"wrapping_sub" => "wrappingSub",
			"wrapping_mul" => "wrappingMul",
			"div_ceil" => "divCeil",
			"is_multiple_of" => "isMultipleOf",
			_ => return None,
		};
		let e = int.method(method, args);
		// The big-endian bytes, the other way around.
		return Some(match name.as_str() {
			"to_le_bytes" => e.method("reverse", Vec::new()),
			_ => e,
		});
	}
	if owner == "f64" {
		return super::float::method(b, &name, args, span);
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
			// A `NonZero` converts as its integer.
			let (to, from) = (targ(gargs, 0), unwrapped(tcx, targ(gargs, 1)));
			let mut values = args;
			let value = values.pop()?;
			// One between strings, such as into a `Cow<str>` or out of one, keeps the text, and
			// `Some` of a value is the value.
			let some = matches!(to.kind(), ty::Adt(adt, args)
				if Std::of(tcx, adt.did()) == Some(Std::Option) && args.type_at(0) == targ(gargs, 1));
			if to == from || some || (string(tcx, to) && string(tcx, from.peel_refs())) {
				return Some(value);
			}
			// Each `From` between integers, `char`, and `f64`, or from `bool`, keeps the
			// value, as `as` then does.
			let scalar = |t: Ty<'tcx>| Int::of(t).is_some() || t.is_char() || super::float::is(t);
			if !scalar(to) || !(scalar(from) || from.is_bool()) {
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
		("TryFrom", "try_from") | ("TryInto", "try_into") => {
			let (to, from) = match owner.as_str() {
				"TryFrom" => (targ(gargs, 0), targ(gargs, 1)),
				_ => (targ(gargs, 1), targ(gargs, 0)),
			};
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
			if primitive(unwrapped(tcx, targ(gargs, 0))) =>
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
			match Int::of(unwrapped(tcx, targ(gargs, 0))) {
				Some(int) => int.compare(js, l, r),
				None => l.binary(js, r),
			}
		}
		("Ord", "max" | "min") if let Some(int) = Int::of(unwrapped(tcx, targ(gargs, 0))) => {
			let values = args;
			match int.big() {
				true => int.method(name.as_str(), values),
				false => Expr::rt(if name == "max" { "max" } else { "min" }, values),
			}
		}
		("Ord", "clamp") if let Some(int) = Int::of(targ(gargs, 0)) => match int.big() {
			true => int.method("clamp", args),
			false => Expr::rt("clamp", args),
		},
		("Clone", "clone") => {
			let ty = targ(gargs, 0);
			let values = args;
			let value = values.into_iter().next()?;
			match ty.kind() {
				ty::Adt(adt, args) if Std::of(tcx, adt.did()) == Some(Std::Vec) && is_u8(args.type_at(0)) => {
					Expr::rt("byteVecFrom", vec![Expr::rt("byteVecView", vec![value])])
				}
				ty::Adt(..) if string(tcx, ty) => value,
				ty::Adt(adt, args) if Std::of(tcx, adt.did()) == Some(Std::Vec) => {
					return vec_clone(b, value, args.type_at(0), span);
				}
				ty::Tuple(tys) if !(b.t.is_copy(ty) && clone_copies(b.t, ty)) => {
					let clones = tys.iter().map(|t| super::collections::clone_fn(b, t, span)).collect();
					Expr::rt("tupleClone", vec![value, Expr::Array(clones)])
				}
				// A box is what it holds, cloned.
				ty::Adt(adt, args) if Std::of(tcx, adt.did()) == Some(Std::Box) => {
					let inner = args.type_at(0);
					if !inner.is_sized(tcx, b.t.typing_env_pub()) {
						return None;
					}
					match super::collections::clone_fn(b, inner, span) {
						Expr::Lit(l) if l == "undefined" => value,
						clone => clone.call(vec![value]),
					}
				}
				ty::Adt(adt, _) if Std::of(tcx, adt.did()) == Some(Std::FromUtf8Error) => {
					Expr::rt("fromUtf8ErrorClone", vec![value])
				}
				// Through the reference, a slice of the bytes.
				_ if super::byte_array(ty) => Expr::rt("byteSliceToArray", vec![value]),
				ty::Adt(adt, _) if int_range(ty) => {
					Expr::rt_of(range_of(Std::of(tcx, adt.did())?)?, "clone").call(vec![value])
				}
				_ if b.t.is_copy(ty) && clone_copies(b.t, ty) => b.t.copied(value, ty, span),
				_ => return None,
			}
		}
		("Default", "default") => default_value(b, targ(gargs, 0), span, out)?,

		// Options, whose other combinators lower from core's MIR.
		("Option", "unwrap" | "expect") => rt2(args, "option", &name),
		("Option", "is_some") => args.pop()?.binary("!==", b.t.none(targ(gargs, 0))),
		("Option", "is_none") => args.pop()?.binary("===", b.t.none(targ(gargs, 0))),
		("Option", "as_ref" | "as_mut") => args.pop()?,
		("Option", "copied" | "cloned") => {
			let sig = tcx.fn_sig(call.def_id).instantiate(tcx, call.args).skip_norm_wip();
			let option = tcx.instantiate_bound_regions_with_erased(sig).output();
			let value = args.pop()?;
			b.t.copied(value, option, span)
		}

		// Results, whose other combinators lower from core's MIR.
		("Result", "is_ok") => args.pop()?.member("ok"),
		("Result", "is_err") => args.pop()?.member("ok").not(),
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

		("ControlFlow", "is_continue" | "is_break") => Flow::test(b.t, gargs, args.pop()?, name == "is_continue"),

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

		// A `NonZero` is its integer.
		("NonZero", "get") => args.pop()?,
		("NonZero", "new") => Expr::rt("nonZero", args),
		// `Bound<&T>::cloned::<T>`: what it holds, cloned.
		("Bound", "cloned") => {
			let value = args.pop()?;
			let sig = tcx.fn_sig(call.def_id).instantiate(tcx, gargs).skip_norm_wip();
			let bound = tcx.instantiate_bound_regions_with_erased(sig).output();
			match super::collections::clone_fn(b, targ(gargs, 0), span) {
				Expr::Lit(l) if l == "undefined" => b.t.copied(value, bound, span),
				clone => Expr::rt("boundMap", vec![value, clone]),
			}
		}
		("Bound", "as_ref") => args.pop()?,
		("RangeBounds", "start_bound" | "end_bound") => {
			let start = name == "start_bound";
			let range = targ(gargs, 0).peel_refs();
			let sig = tcx.fn_sig(call.def_id).instantiate(tcx, gargs).skip_norm_wip();
			let ts = b.t.ty(tcx.instantiate_bound_regions_with_erased(sig).output(), span);
			let r = args.pop()?;
			let (variant, field) = match (range.kind(), b.t.std_of(range)) {
				// `(Bound<T>, Bound<T>)`, whose bounds are its own.
				(ty::Tuple(_), _) => return Some(r.index(Expr::lit(if start { "0" } else { "1" }))),
				(_, Some(Std::Range)) if start => ("Included", Some("start")),
				(_, Some(Std::Range)) => ("Excluded", Some("end")),
				(_, Some(Std::RangeFrom)) if start => ("Included", Some("start")),
				(_, Some(Std::RangeTo)) if !start => ("Excluded", Some("end")),
				(_, Some(Std::RangeToInclusive)) if !start => ("Included", Some("end")),
				(_, Some(Std::RangeFrom | Std::RangeTo | Std::RangeToInclusive | Std::RangeFull)) => {
					("Unbounded", None)
				}
				// Exhausted, it ends before its end.
				(_, Some(Std::RangeInclusive)) => {
					let f = if start {
						"rangeInclusiveStartBound"
					} else {
						"rangeInclusiveEndBound"
					};
					return Some(Expr::As(Box::new(Expr::rt(f, vec![r])), ts));
				}
				_ => return None,
			};
			tagged(variant, field.map(|f| r.member(f)), ts)
		}

		// Slices, whose byte functions take an `rt.ByteSlice`.
		("slice", "len") => super::mir::slice_member(args.pop()?, "length"),
		("slice", "is_empty") => super::mir::slice_member(args.pop()?, "length").binary("===", Expr::lit("0")),
		("slice", "split_first" | "split_at" | "split_at_checked" | "first" | "get")
			if name != "get" || Int::of(targ(gargs, 1)).is_some() =>
		{
			let of = match is_u8(targ(gargs, 0)) {
				true => "byteSlice",
				false => "slice",
			};
			rt2(args, of, &super::camel(&name))
		}
		// An element JS copies is lent as its place, and an object as itself.
		("slice", "first_mut" | "last_mut" | "get_mut") if name != "get_mut" || Int::of(targ(gargs, 1)).is_some() => {
			let elem = targ(gargs, 0);
			let mut values = args;
			values[0] = elements(values[0].clone(), Ty::new_slice(tcx, elem));
			let name = match b.t.needs_place(elem) {
				true => name.as_str(),
				false => name.strip_suffix("_mut")?,
			};
			rt2(values, "slice", &super::camel(name))
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
		// A slice of the one value.
		("raw", "from_ref") if std_path(tcx, call.def_id) == ["slice", "raw", "from_ref"] => {
			let value = args.pop()?;
			match is_u8(targ(gargs, 0)) {
				true => Expr::rt(
					"byteSliceFrom",
					vec![Expr::ident("Uint8Array").member("of").call(vec![value])],
				),
				false => Expr::Array(vec![value]),
			}
		}
		("slice", "copy_from_slice") if is_u8(targ(gargs, 0)) => rt2(args, "byteSlice", "copyFromSlice"),
		("PartialEq", "eq") if byte_slice(targ(gargs, 0)) && byte_slice(targ(gargs, 1)) => {
			Expr::rt("byteSliceEq", args)
		}
		("slice", "to_vec") => {
			let elem = targ(gargs, 0);
			let slice = args.pop()?;
			match is_u8(elem) {
				true => Expr::rt("byteSliceToByteVec", vec![slice]),
				false => return vec_clone(b, slice, elem, span),
			}
		}
		("slice", "last") if !is_u8(targ(gargs, 0)) => Expr::rt("sliceLast", args),
		("slice", "reverse") if !is_u8(targ(gargs, 0)) => Expr::rt("sliceReverse", args),
		("slice", "swap") if !is_u8(targ(gargs, 0)) => Expr::rt("sliceSwap", args),
		("slice", "contains") => {
			let elem = targ(gargs, 0);
			let [s, x] = <[Expr; 2]>::try_from(args).ok()?;
			match is_u8(elem) {
				true => Expr::rt("byteSliceView", vec![s]).method("includes", vec![x]),
				false => {
					let eq = super::collections::eq_fn(b, elem, span);
					Expr::rt("sliceContains", vec![s, x, eq])
				}
			}
		}
		// Sorted by `PartialOrd::lt`, or by a comparator, or by `lt` of the keys a function makes.
		(
			"slice",
			"sort" | "sort_unstable" | "sort_by" | "sort_unstable_by" | "sort_by_key" | "sort_unstable_by_key",
		) if !is_u8(targ(gargs, 0)) => {
			let mut values = args;
			match name.as_str() {
				"sort" | "sort_unstable" => {
					values.push(super::collections::test_fn(b, targ(gargs, 0), "lt", span));
					Expr::rt("sliceSort", values)
				}
				"sort_by" | "sort_unstable_by" => Expr::rt("sliceSortBy", values),
				_ => {
					let key = targ(gargs, 1);
					if b.t.drops(key) {
						return Some(b.err(
							span,
							format!("`slice::{name}` by `{key}`, which drops, which is not supported yet"),
						));
					}
					values.push(super::collections::test_fn(b, key, "lt", span));
					Expr::rt("sliceSortByKey", values)
				}
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
		("Vec", "extend_from_slice") => {
			let mut values = args;
			let clone = super::collections::clone_fn(b, targ(gargs, 0), span);
			if !matches!(&clone, Expr::Lit(l) if l == "undefined") {
				values.push(clone);
			}
			Expr::rt("vecExtendFromSlice", values)
		}
		("Vec", "swap_remove") if !is_u8(targ(gargs, 0)) => Expr::rt("vecSwapRemove", args),
		("Vec", "insert" | "remove" | "truncate") if !is_u8(targ(gargs, 0)) => {
			Expr::rt_of("vec", &super::camel(&name)).call(args)
		}
		("Vec", "clear") => {
			let mut values = args;
			values.push(Expr::lit("0"));
			Expr::rt("vecTruncate", values)
		}
		// What each takes by value is a closure, which it drops once done.
		("Vec", "retain" | "retain_mut" | "dedup" | "dedup_by" | "dedup_by_key") if !is_u8(targ(gargs, 0)) => {
			if let Some(ty) = dropping_operand(b, call) {
				return Some(b.err(
					span,
					format!("passing `{ty}`, which drops, to `{owner}::{name}`, which is not supported yet"),
				));
			}
			let mut values = args;
			match name.as_str() {
				"retain" | "retain_mut" => Expr::rt("vecRetain", values),
				"dedup" => {
					values.push(super::collections::eq_fn(b, targ(gargs, 0), span));
					Expr::rt("vecDedupBy", values)
				}
				"dedup_by" => Expr::rt("vecDedupBy", values),
				_ => {
					// `Vec<T, A>::dedup_by_key::<F, K>`.
					let key = targ(gargs, 3);
					if b.t.drops(key) {
						return Some(b.err(
							span,
							format!("`Vec::{name}` by `{key}`, which drops, which is not supported yet"),
						));
					}
					values.push(super::collections::eq_fn(b, key, span));
					Expr::rt("vecDedupByKey", values)
				}
			}
		}
		("Vec", "as_slice" | "as_mut_slice") => {
			let v = args.pop()?;
			vec_view(v, targ(gargs, 0))
		}
		("Deref", "deref") | ("DerefMut", "deref_mut") => {
			let ty = targ(gargs, 0);
			let v = args.pop()?;
			match ty.kind() {
				ty::Adt(adt, vargs) => match Std::of(tcx, adt.did()) {
					Some(Std::Vec) => vec_view(v, vargs.type_at(0)),
					Some(Std::String | Std::Box) => v,
					Some(Std::Cow) if string(tcx, ty) => v,
					_ => return None,
				},
				_ => return None,
			}
		}

		// Strings.
		("String" | "str", "as_bytes") => Expr::rt("stringAsBytes", args),
		("String", "from_utf8") => Expr::rt("stringFromUtf8", args),
		// `str::from_utf8` is `core::str::converts::from_utf8`, or `str`'s own.
		("converts" | "str", "from_utf8") => Expr::rt("strFromUtf8", args),
		("Utf8Error", "valid_up_to") => args.pop()?.member("validUpTo"),
		("Utf8Error", "error_len") => args.pop()?.member("errorLen"),
		("String" | "str", "len") => Expr::rt("stringLen", args),
		("String", "as_str") => args.pop()?,
		("String", "new") => Expr::Str(String::new()),
		("String" | "str", "is_empty") => args.pop()?.member("length").binary("===", Expr::lit("0")),
		("str", "chars") => Expr::rt("strChars", args),
		("str", "bytes") => Expr::rt("strBytes", args),
		("str", "is_char_boundary") => Expr::rt("strIsCharBoundary", args),
		// Searches by a pattern.
		("str", "split" | "match_indices" | "contains" | "starts_with" | "ends_with" | "rfind" | "split_once")
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
		// A `Cow<str>` is its text, whichever variant holds it.
		("Cow", "into_owned") if targ(gargs, 0).is_str() => args.pop()?,
		("AsRef", "as_ref") if string(tcx, targ(gargs, 0)) && targ(gargs, 1).is_str() => args.pop()?,
		("Cow", "is_borrowed" | "is_owned") => {
			let sig = tcx.fn_sig(call.def_id).instantiate(tcx, gargs).skip_norm_wip();
			let cow = tcx.instantiate_bound_regions_with_erased(sig).inputs()[0].peel_refs();
			return Some(b.err(span, cow_variant(cow)));
		}
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
		// The size wasm32 lays the type out in, which translation targets.
		("mem", "size_of") => {
			let env = b.t.typing_env_pub();
			let layout = tcx.layout_of(env.as_query_input(targ(gargs, 0))).ok()?;
			Expr::lit(layout.size.bytes().to_string())
		}
		("array", "from_fn") => {
			let n = gargs.consts().next()?.try_to_target_usize(tcx)?;
			let f = match targ(gargs, 0).kind() {
				ty::Uint(ty::UintTy::U8) => "bytesFromFn",
				_ => "arrayFromFn",
			};
			Expr::rt(f, vec![Expr::lit(n.to_string()), args.pop()?])
		}
		("fmt", "format") => {
			let parts = args.pop()?;
			match literal(&parts) {
				Some(s) => Expr::Str(s),
				None => Expr::rt("fmtFormat", vec![parts]),
			}
		}
		("Display" | "Debug" | "LowerHex" | "UpperHex" | "Octal" | "Binary" | "LowerExp" | "UpperExp", "fmt") => {
			let tr = fmt::Trait::of(tcx, tcx.trait_of_assoc(call.def_id)?)?;
			let [value, f] = <[Expr; 2]>::try_from(args).ok()?;
			fmt::std_impl(b, tr, targ(gargs, 0), value, f, span, out)?
		}
		("ToString", "to_string") => {
			let ty = targ(gargs, 0);
			let value = args.pop()?;
			match ty.kind() {
				ty::Adt(..) if string(tcx, ty) => value,
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
	// What drops each operand the mapping may discard, then `rt.None` where it is one.
	let lost = discarded.unwrap_or_default().iter().map(|&n| targ(gargs, n));
	let Some(mut droppers) = b.t.droppers(lost, span) else {
		return Some(Expr::undefined());
	};
	let none = match none_last(&owner, &name) {
		true => sentinel_option(b.t, call).and_then(|inner| b.t.none_arg(inner)),
		false => None,
	};
	if let Some(none) = none {
		droppers.resize(discarded.unwrap_or_default().len(), Expr::undefined());
		droppers.push(none);
	}
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

/// A clone of `value`, a slice, array, or `Vec` of `elem` other than bytes, as a `Vec`: a
/// copy of the array if each element's clone is itself, or else each element cloned.
fn vec_clone<'tcx>(b: &mut Body<'_, '_, 'tcx>, value: Expr, elem: Ty<'tcx>, span: Span) -> Option<Expr> {
	let clone = super::collections::clone_fn(b, elem, span);
	if matches!(&clone, Expr::Lit(l) if l == "undefined") {
		return Some(value.method("slice", Vec::new()));
	}
	Some(value.method("map", vec![clone]))
}

/// A `str` method's pattern as the runtime takes it, an `rt.Pattern`: a `char`'s text, a
/// string or a predicate as it is, or a predicate for an array or slice of chars. `None`
/// for any other pattern.
fn pattern<'tcx>(t: &Translator<'_, 'tcx>, ty: Ty<'tcx>, p: Expr) -> Option<Expr> {
	match ty.kind() {
		ty::Char => Some(char_text(p)),
		ty::Closure(..) | ty::FnDef(..) => Some(p),
		ty::Ref(..) if is_text(t, ty) => Some(p),
		// Any of the chars of an array or slice.
		ty::Array(c, _) | ty::Slice(c) if c.is_char() => Some(Expr::rt("patternChars", vec![p])),
		ty::Ref(_, inner, _) if matches!(inner.kind(), ty::Array(c, _) | ty::Slice(c) if c.is_char()) => {
			Some(Expr::rt("patternChars", vec![p]))
		}
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

/// What a JS value of `ty` is through references and `NonZero`, which is its integer.
fn unwrapped<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Ty<'tcx> {
	let ty = ty.peel_refs();
	match ty.kind() {
		ty::Adt(adt, args) if Std::of(tcx, adt.did()) == Some(Std::NonZero) => args.type_at(0),
		_ => ty,
	}
}

fn primitive(ty: Ty<'_>) -> bool {
	matches!(ty.kind(), ty::Bool | ty::Int(_) | ty::Uint(_) | ty::Char) || super::float::is(ty)
}

/// A `Vec` of `elem`s as a slice: an `rt.ByteSlice` of a byte vector, or the array itself.
pub fn vec_view(v: Expr, elem: Ty<'_>) -> Expr {
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
		_ if super::bytes_ref(ty) => Expr::rt("byteSliceView", vec![v]),
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

/// `Default::default()` for a standard type, of its parts' defaults where it has parts.
pub fn default_value<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Option<Expr> {
	let tcx = b.t.tcx;
	let part = |b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, out: &mut Vec<Stmt>| {
		let f = super::collections::trait_fn(tcx, "Default", "default");
		b.call_def(f, tcx.mk_args(&[ty.into()]), Vec::new(), span, out)
	};
	Some(match ty.kind() {
		ty::Tuple(parts) if parts.is_empty() => Expr::undefined(),
		ty::Tuple(parts) => Expr::Array(parts.iter().map(|t| part(b, t, out)).collect()),
		ty::Array(elem, n) => {
			let n = n.try_to_target_usize(tcx)?;
			match elem.kind() {
				ty::Uint(ty::UintTy::U8) => return Some(Expr::rt("bytes", vec![Expr::lit(n.to_string())])),
				// Whatever its element, which needn't have a default.
				_ if n == 0 => return Some(Expr::Array(Vec::new())),
				_ => {}
			}
			// Each element its own, as an atomic must be.
			let mut body = Vec::new();
			let item = part(b, *elem, &mut body);
			body.push(StmtKind::Return(Some(item.typed())).at(None));
			let item_ty = b.t.ty(*elem, span);
			Expr::ident("Array").member("from").call(vec![
				Expr::Object(vec![(crate::ts::Key::Name("length".into()), Expr::lit(n.to_string()))]),
				Expr::Arrow(Vec::new(), item_ty, body),
			])
		}
		ty::Bool => Expr::lit("false"),
		ty::Int(_) | ty::Uint(_) => Int::of(ty)?.lit(0),
		_ if super::float::is(ty) => super::float::lit(0.0),
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
			Std::Option => b.t.none(args.type_at(0)),
			Std::Vec => match args.type_at(0).kind() {
				ty::Uint(ty::UintTy::U8) => Expr::New(Box::new(Expr::Rt("ByteVec".into())), Vec::new()),
				_ => Expr::Array(Vec::new()),
			},
			Std::String => Expr::Str(String::new()),
			Std::Cow if string(tcx, ty) => Expr::Str(String::new()),
			Std::Duration => super::time::zero(),
			Std::Atomic => Expr::New(
				Box::new(Expr::Rt("Atomic".into())),
				vec![default_value(b, args.type_at(0), span, out)?],
			),
			Std::OnceCell | Std::OnceLock => super::task::once_new(b, ty),
			Std::Box => part(b, args.type_at(0), out),
			Std::Arc => Expr::New(Box::new(Expr::Rt("Arc".into())), vec![part(b, args.type_at(0), out)]),
			Std::Mutex => Expr::New(Box::new(Expr::Rt("Mutex".into())), vec![part(b, args.type_at(0), out)]),
			Std::Weak => Expr::New(Box::new(Expr::Rt("Weak".into())), Vec::new()),
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
