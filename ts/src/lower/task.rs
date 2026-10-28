//! The standard library's ownership and async types, as `rt/task.ts` gives them: `Arc`,
//! `Weak`, `Mutex`, atomics, `Waker`, `Context`, `Poll`, `Pin`, and std's futures.

use rustc_abi::VariantIdx;
use rustc_middle::mir::{BinOp, UnOp};
use rustc_middle::ty::{self, Instance, Ty};
use rustc_span::{Span, Symbol};

use super::Translator;
use super::body::Body;
use super::library::{self, Call, Std};
use super::num::{self, Int, Operand};
use crate::ts::{Expr, Param};

impl<'tcx> Translator<'_, 'tcx> {
	/// `Future::Output` of `ty`, a future.
	pub fn future_output(&mut self, ty: Ty<'tcx>, span: Span) -> Ty<'tcx> {
		let tcx = self.tcx;
		let output = tcx.lang_items().future_output().and_then(|item| {
			let projection = Ty::new_projection(tcx, ty::IsRigid::No, item, [ty]);
			tcx.try_normalize_erasing_regions(self.typing_env_pub(), ty::Unnormalized::new_wip(projection))
				.ok()
		});
		output.unwrap_or_else(|| {
			self.err(span, format!("`{ty}` is not a future"));
			tcx.types.unit
		})
	}
}

/// A call of a standard library function on these types, or its arguments back if it is
/// none of theirs. `Ok(None)` refuses it.
pub fn call<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	call: &Call<'tcx>,
	owner: &str,
	name: &str,
	args: Vec<Expr>,
) -> Result<Option<Expr>, Vec<Expr>> {
	let tcx = b.t.tcx;
	let std_of = |ty: Ty<'tcx>| match ty.peel_refs().kind() {
		ty::Adt(adt, _) => Std::of(tcx, adt.did()),
		_ => None,
	};
	let self_ty = call.args.types().next();
	let self_std = self_ty.and_then(std_of);
	let e = match (owner, name) {
		("Arc", "new") => Expr::New(Box::new(Expr::Rt("Arc".into())), args),
		// An `Arc<str>` of a string, which JS shares as it is.
		("From", "from")
			if let [arc, from] = call.args.as_slice()
				&& let ty::Adt(_, arc) = arc.expect_ty().kind()
				&& self_std == Some(Std::Arc)
				&& arc.type_at(0).is_str()
				&& library::string(tcx, from.expect_ty().peel_refs()) =>
		{
			Expr::New(Box::new(Expr::Rt("Arc".into())), args)
		}
		// A waker over the crate's `Wake` impl.
		("From", "from")
			if let [_, from] = call.args.as_slice()
				&& self_std == Some(Std::Waker)
				&& std_of(from.expect_ty()) == Some(Std::Arc) =>
		{
			arc_waker(b, from.expect_ty(), first(args), call.span)
		}
		("Waker", "noop") => Expr::Rt("NOOP_WAKER".into()),
		("AsRef", "as_ref") if self_std == Some(Std::Arc) => first(args).member("value"),
		("Arc", "downgrade") => Expr::New(Box::new(Expr::Rt("Weak".into())), args),
		("Arc", "strong_count") => first(args).member("strong"),
		("Arc", "ptr_eq") => {
			let [l, r] = two(args);
			l.binary("===", r)
		}
		("Weak", "upgrade") => Expr::rt("weakUpgrade", args),
		("Weak", "strong_count") => first(args).member("arc").member("strong"),
		("Weak", "ptr_eq") => {
			let [l, r] = two(args);
			l.member("arc").binary("===", r.member("arc"))
		}
		("Clone", "clone") if self_std == Some(Std::Arc) => Expr::rt("arcClone", args),
		("Clone", "clone") if self_std == Some(Std::Weak) => Expr::rt("weakClone", args),
		// A waker is the executor's, whose `clone` is its own.
		("Clone", "clone") if self_std == Some(Std::Waker) => first(args).member("clone").call(vec![]),
		("Deref", "deref") | ("DerefMut", "deref_mut") => match self_std {
			Some(Std::Arc | Std::Mutex | Std::MutexGuard) => first(args).member("value"),
			Some(Std::Pin) => first(args),
			_ => return Err(args),
		},
		("Mutex", "new") => Expr::New(Box::new(Expr::Rt("Mutex".into())), args),
		// The mutex is its own guard, and a lock never fails on one thread.
		("Mutex", "lock") => first(args),
		("Result", "unwrap" | "expect") if call.args.types().nth(1).and_then(std_of) == Some(Std::PoisonError) => {
			first(args)
		}
		("Atomic", _) => return atomic(b, call, name, args),
		("atomic", "fence" | "compiler_fence") => Expr::rt("fence", args),
		("Waker", "wake_by_ref") => first(args).member("wakeByRef").call(vec![]),
		("Waker", "wake") => first(args).member("wake").call(vec![]),
		("Waker", "will_wake") => {
			let [l, r] = two(args);
			l.member("willWake").call(vec![r])
		}
		// With what drops its function, unless that is borrowed.
		("poll_fn", "poll_fn") => {
			let dropper = b.t.drop_fn(call.args.type_at(1), call.span);
			Expr::New(
				Box::new(Expr::Rt("PollFn".into())),
				args.into_iter().chain(dropper).collect(),
			)
		}
		("pending", "pending") => Expr::Rt("NEVER".into()),
		// std's futures are the runtime's, which poll themselves.
		("Future", "poll") if matches!(self_std, Some(Std::PollFn | Std::Pending)) => {
			let [future, cx] = two(args);
			future.member("poll").call(vec![cx])
		}
		("Context", "waker" | "from_waker")
		| ("Pin", "new" | "new_unchecked" | "as_mut" | "get_mut" | "get_unchecked_mut")
		| ("pin", "pin_new_unchecked_in_helper")
		| ("future", "get_context")
		| ("IntoFuture", "into_future") => first(args),
		("mem", "drop") => {
			let ty = call.args.type_at(0);
			let value = first(args);
			match b.t.drop_value(value.clone(), ty, call.span) {
				Some(dropped) => dropped,
				None if value.is_path() => Expr::undefined(),
				None => value,
			}
		}
		_ => return Err(args),
	};
	Ok(Some(e))
}

/// An `Atomic<T>` method: an `rt.Atomic`'s, with its orderings, which the runtime checks.
/// `Ok(None)` refuses it.
fn atomic<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	call: &Call<'tcx>,
	name: &str,
	args: Vec<Expr>,
) -> Result<Option<Expr>, Vec<Expr>> {
	let tcx = b.t.tcx;
	let imp = tcx.inherent_impl_of_assoc(call.def_id).expect("an inherent method");
	let ty::Adt(_, impl_args) = tcx.type_of(imp).instantiate_identity().skip_norm_wip().kind() else {
		unreachable!("an atomic is a struct")
	};
	let ty = impl_args.type_at(0);
	let int = Int::of(ty);
	let fetch = |f: Expr, mut args: Vec<Expr>| {
		args.insert(1, f);
		Expr::rt("atomicFetch", args)
	};
	// A compare-exchange, given what compares a wide integer, whose limbs `===` can't.
	let exchange = |f: &str, mut args: Vec<Expr>| {
		args.extend(int.filter(|int| int.big()).map(|int| int.func("eq")));
		Expr::rt(f, args)
	};
	let e = match name {
		"new" => Expr::New(Box::new(Expr::Rt("Atomic".into())), args),
		"load" => Expr::rt("atomicLoad", args),
		"store" => Expr::rt("atomicStore", args),
		"swap" => Expr::rt("atomicSwap", args),
		"into_inner" => first(args).member("v"),
		// The atomic is the place that holds its value.
		"get_mut" => first(args),
		"compare_exchange" | "compare_exchange_weak" => exchange("atomicCompareExchange", args),
		"fetch_add" if let Some(int) = int => fetch(int.func("wrappingAdd"), args),
		"fetch_sub" if let Some(int) = int => fetch(int.func("wrappingSub"), args),
		"fetch_max" | "fetch_min" if let Some(int) = int => {
			let f = match int.big() {
				true => int.func(&name[6..]),
				false => Expr::Rt(name[6..].into()),
			};
			fetch(f, args)
		}
		"fetch_and" | "fetch_or" | "fetch_xor" | "fetch_nand" => {
			let op = match name {
				"fetch_and" | "fetch_nand" => BinOp::BitAnd,
				"fetch_or" => BinOp::BitOr,
				_ => BinOp::BitXor,
			};
			fetch(binary(b, op, name == "fetch_nand", ty, call.span), args)
		}
		// `fetch_xor(true)`, as std's is.
		"fetch_not" if ty.is_bool() => {
			let f = binary(b, BinOp::BitXor, false, ty, call.span);
			let mut args = args;
			args.insert(1, Expr::lit("true"));
			fetch(f, args)
		}
		// std's loop of compare-exchanges, which retries only if `f` changed the atomic.
		"try_update" | "fetch_update" => exchange("atomicTryUpdate", args),
		"update" => exchange("atomicUpdate", args),
		_ => return Ok(None),
	};
	Ok(Some(e))
}

/// `(old, value) => old op value` over `ty`, negated if `not`.
fn binary<'tcx>(b: &mut Body<'_, '_, 'tcx>, op: BinOp, not: bool, ty: Ty<'tcx>, span: Span) -> Expr {
	let operand = |name: &str| Operand {
		expr: Expr::ident(name),
		value: None,
	};
	let mut value = num::binary(b, op, operand("old"), operand("value"), ty, ty, span);
	if not {
		value = num::unary(b, UnOp::Not, value, ty, span);
	}
	let params = ["old", "value"].map(|name| Param {
		name: name.into(),
		ty: None,
	});
	Expr::Lambda(params.into(), Box::new(value))
}

/// The `atomic::Ordering` variant `index`: its discriminant.
pub fn atomic_ordering(index: VariantIdx) -> Expr {
	Expr::lit(index.as_u32().to_string())
}

/// `Waker::from(arc)`, an `Arc<W>`: an `rt.ArcWaker` over `W`'s `Wake::wake`, what drops
/// the `Arc`, and `W`'s own `wake_by_ref` if it overrides the default.
fn arc_waker<'tcx>(b: &mut Body<'_, '_, 'tcx>, arc: Ty<'tcx>, value: Expr, span: Span) -> Expr {
	let tcx = b.t.tcx;
	let ty::Adt(_, arc_args) = arc.kind() else {
		unreachable!("an Arc is a struct")
	};
	let wake_trait = tcx
		.get_diagnostic_item(Symbol::intern("Wake"))
		.expect("Waker::from bounds its Arc by Wake");
	let items = tcx.associated_items(wake_trait);
	let item = |name: &str| {
		items
			.filter_by_name_unhygienic(Symbol::intern(name))
			.next()
			.expect("a method of Wake")
			.def_id
	};
	let (wake, by_ref) = (item("wake"), item("wake_by_ref"));
	let w = tcx.mk_args(&[arc_args.type_at(0).into()]);
	let arc_param = b.fresh("arc");
	let dropped = b.t.drop_value(Expr::ident(arc_param.clone()), arc, span);
	let param = Param {
		name: arc_param,
		ty: Some(b.t.ty(arc, span)),
	};
	let drop = Expr::Lambda(vec![param], Box::new(dropped.unwrap_or_else(Expr::undefined)));
	let mut args = vec![value, b.fn_value(wake, w, span), drop];
	if let Ok(Some(instance)) = Instance::try_resolve(tcx, b.t.typing_env_pub(), by_ref, w)
		&& instance.def_id() != by_ref
	{
		args.push(b.fn_value(by_ref, w, span));
	}
	Expr::New(Box::new(Expr::Rt("ArcWaker".into())), args)
}

fn first(args: Vec<Expr>) -> Expr {
	args.into_iter().next().unwrap_or_else(Expr::undefined)
}

fn two(args: Vec<Expr>) -> [Expr; 2] {
	let mut values = args.into_iter();
	[
		values.next().unwrap_or_else(Expr::undefined),
		values.next().unwrap_or_else(Expr::undefined),
	]
}
