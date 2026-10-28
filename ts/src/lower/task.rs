//! The standard library's ownership and async types, as `rt/task.ts` gives them: `Arc`,
//! `Weak`, `Mutex`, `Waker`, `Context`, `Poll`, `Pin`, and std's futures.

use rustc_middle::ty::{self, Instance, Ty};
use rustc_span::{Span, Symbol};

use super::Translator;
use super::body::Body;
use super::library::{self, Call, Std};
use crate::ts::{Expr, Param, Stmt};

impl<'tcx> Translator<'_, 'tcx> {
	/// `Future::Output` of `ty`, if it is a `dyn Future`, whose JS value is an `rt.Future`.
	pub fn dyn_future(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
		let ty::Dynamic(preds, ..) = ty.kind() else {
			return None;
		};
		if preds.principal_def_id()? != self.tcx.lang_items().future_trait()? {
			return None;
		}
		let output = preds.projection_bounds().next()?;
		self.tcx.instantiate_bound_regions_with_erased(output).term.as_type()
	}

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

impl<'tcx> Body<'_, '_, 'tcx> {
	/// `value`, a `from` that is a future, as the `rt.Future` a `dyn Future` is: itself if
	/// it is one already, and else an `rt.DynFuture` over its `Future` impl, which drops it
	/// with itself unless `borrowed`.
	pub fn dyn_future(&mut self, from: Ty<'tcx>, value: Expr, borrowed: bool, span: Span) -> Expr {
		let tcx = self.t.tcx;
		match from.kind() {
			ty::Dynamic(..) | ty::Coroutine(..) => return value,
			// What a reference or a box points at, which its JS value is.
			ty::Ref(_, inner, _) => return self.dyn_future(*inner, value, true, span),
			ty::Adt(adt, args) => match Std::of(tcx, adt.did()) {
				Some(Std::PollFn | Std::Pending) => return value,
				Some(Std::Box | Std::Pin) => return self.dyn_future(args.type_at(0), value, borrowed, span),
				_ => {}
			},
			_ => {}
		}
		// What `Future::poll` takes would be an `rt.Place`, which the box doesn't have.
		if self.t.needs_place(from) {
			return self.err(span, format!("a `dyn Future` of `{from}`, which JS copies"));
		}
		let future = tcx.lang_items().future_trait().expect("a dyn Future names the trait");
		let poll = tcx
			.associated_items(future)
			.filter_by_name_unhygienic(Symbol::intern("poll"))
			.next()
			.expect("Future::poll")
			.def_id;
		let mut args = vec![value, self.fn_value(poll, tcx.mk_args(&[from.into()]), span)];
		if !borrowed {
			args.extend(self.t.drop_fn(from, span));
		}
		Expr::New(Box::new(Expr::Rt("DynFuture".into())), args)
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
	out: &mut Vec<Stmt>,
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
		// A box's JS value is what it holds.
		("Box", "pin") => first(args),
		("Future", "poll") => match self_ty.map(Ty::kind) {
			// std's futures and a `dyn Future` are `rt.Future`s, which poll themselves.
			_ if matches!(self_std, Some(Std::PollFn | Std::Pending))
				|| self_ty.is_some_and(|ty| b.t.dyn_future(ty).is_some()) =>
			{
				let [future, cx] = two(args);
				future.member("poll").call(vec![cx])
			}
			// A pinned pointer, a box, or a `&mut` polls what it points at, which its JS
			// value is.
			Some(ty::Adt(adt, adt_args)) if matches!(Std::of(tcx, adt.did()), Some(Std::Pin | Std::Box)) => {
				let inner = match Std::of(tcx, adt.did()) {
					Some(Std::Pin) => adt_args.type_at(0).builtin_deref(true),
					_ => Some(adt_args.type_at(0)),
				};
				let Some(inner) = inner else { return Err(args) };
				b.call_def(call.def_id, tcx.mk_args(&[inner.into()]), args, call.span, out)
			}
			Some(ty::Ref(_, inner, _)) => {
				b.call_def(call.def_id, tcx.mk_args(&[(*inner).into()]), args, call.span, out)
			}
			_ => return Err(args),
		},
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
