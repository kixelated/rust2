//! The standard library's ownership and async types, as `rt/task.ts` gives them: `Arc`,
//! `Weak`, `Mutex`, `Waker`, `Context`, `Poll`, and `Pin`.

use rustc_middle::ty::{self, Ty};

use super::body::Body;
use super::library::{Args, Call, Std};
use crate::ts::{Expr, Stmt};

/// A call of a standard library function on these types, or its arguments back if it is
/// none of theirs. `Ok(None)` refuses it.
pub fn call<'tcx, 'x>(
	b: &mut Body<'_, '_, 'tcx>,
	call: &Call<'tcx>,
	owner: &str,
	name: &str,
	args: Args<'x>,
	out: &mut Vec<Stmt>,
) -> Result<Option<Expr>, Args<'x>> {
	let tcx = b.t.tcx;
	let std_of = |ty: Ty<'tcx>| match ty.peel_refs().kind() {
		ty::Adt(adt, _) => Std::of(tcx, adt.did()),
		_ => None,
	};
	let self_ty = call.args.types().next();
	let self_std = self_ty.and_then(std_of);
	let e = match (owner, name) {
		("Arc", "new") => Expr::New(Box::new(Expr::Rt("Arc".into())), b.args(args, out)),
		("Arc", "downgrade") => Expr::New(Box::new(Expr::Rt("Weak".into())), b.args(args, out)),
		("Arc", "strong_count") => first(b, args, out).member("strong"),
		("Arc", "ptr_eq") => {
			let [l, r] = two(b, args, out);
			l.binary("===", r)
		}
		("Weak", "upgrade") => Expr::rt("weakUpgrade", b.args(args, out)),
		("Weak", "strong_count") => first(b, args, out).member("arc").member("strong"),
		("Weak", "ptr_eq") => {
			let [l, r] = two(b, args, out);
			l.member("arc").binary("===", r.member("arc"))
		}
		("Clone", "clone") if self_std == Some(Std::Arc) => Expr::rt("arcClone", b.args(args, out)),
		("Clone", "clone") if self_std == Some(Std::Weak) => Expr::rt("weakClone", b.args(args, out)),
		// A waker is the executor's, whose `clone` is its own.
		("Clone", "clone") if self_std == Some(Std::Waker) => first(b, args, out).member("clone").call(vec![]),
		("Deref", "deref") | ("DerefMut", "deref_mut") => match self_std {
			Some(Std::Arc | Std::Mutex | Std::MutexGuard) => first(b, args, out).member("value"),
			Some(Std::Pin) => first(b, args, out),
			_ => return Err(args),
		},
		("Mutex", "new") => Expr::New(Box::new(Expr::Rt("Mutex".into())), b.args(args, out)),
		// The mutex is its own guard, and a lock never fails on one thread.
		("Mutex", "lock") => first(b, args, out),
		("Result", "unwrap" | "expect") if call.args.types().nth(1).and_then(std_of) == Some(Std::PoisonError) => {
			first(b, args, out)
		}
		("Waker", "wake_by_ref") => first(b, args, out).member("wakeByRef").call(vec![]),
		("Waker", "wake") => first(b, args, out).member("wake").call(vec![]),
		("Waker", "will_wake") => {
			let [l, r] = two(b, args, out);
			l.member("willWake").call(vec![r])
		}
		("Context", "waker" | "from_waker")
		| ("Pin", "new" | "new_unchecked" | "as_mut" | "get_mut" | "get_unchecked_mut")
		| ("future", "get_context")
		| ("IntoFuture", "into_future") => first(b, args, out),
		("mem", "drop") => {
			let ty = call.args.type_at(0);
			let value = first(b, args, out);
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

fn first<'tcx>(b: &mut Body<'_, '_, 'tcx>, args: Args<'_>, out: &mut Vec<Stmt>) -> Expr {
	b.args(args, out).into_iter().next().unwrap_or_else(Expr::undefined)
}

fn two<'tcx>(b: &mut Body<'_, '_, 'tcx>, args: Args<'_>, out: &mut Vec<Stmt>) -> [Expr; 2] {
	let mut values = b.args(args, out).into_iter();
	[
		values.next().unwrap_or_else(Expr::undefined),
		values.next().unwrap_or_else(Expr::undefined),
	]
}
