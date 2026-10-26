//! Rvalues: arithmetic, casts, aggregates, and closures.

use rustc_abi::{FieldIdx, VariantIdx};
use rustc_index::IndexVec;
use rustc_middle::mir::{self, AggregateKind, BinOp, CastKind, Operand, Place, Rvalue, UnOp};
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::{self, Instance, Ty};
use rustc_span::Span;

use super::{Env, Fx, Slot};
use crate::lower::library::Std;
use crate::lower::num::{self, Int};
use crate::lower::{AdtRepr, field_name};
use crate::ts::{Expr, Key, Stmt, StmtKind};

impl<'m, 'b, 'a, 'tcx> Fx<'m, 'b, 'a, 'tcx> {
	/// The value of `rvalue`, assigned to `dest`, or `None` if it has none to assign.
	pub(super) fn rvalue(
		&mut self,
		rvalue: &Rvalue<'tcx>,
		dest: Place<'tcx>,
		span: Span,
		out: &mut Vec<Stmt>,
	) -> Option<Expr> {
		let tcx = self.tcx;
		let ty = rvalue.ty(&self.mir.local_decls, tcx);
		Some(match rvalue {
			Rvalue::Use(op, _) => self.operand(op, span, out),
			Rvalue::CopyForDeref(place) => self.place_expr(*place, span, out),
			Rvalue::Repeat(op, count) => self.repeat(op, *count, span, out),
			Rvalue::Ref(_, kind, place) => {
				if let Some(e) = self.mut_subslice(*kind, *place, span) {
					return Some(e);
				}
				if let mir::BorrowKind::Mut { .. } = kind {
					let pointee = place.ty(&self.mir.local_decls, tcx).ty;
					if self.b.t.object_kind(pointee).is_none()
						&& !matches!(pointee.kind(), ty::Ref(_, _, rustc_ast::Mutability::Mut))
					{
						return Some(self.err(
							span,
							format!("a mutable reference to `{pointee}`, which is no object it can change in place"),
						));
					}
				}
				self.place_expr(*place, span, out)
			}
			Rvalue::RawPtr(..) => self.err(span, "raw pointers are not supported"),
			Rvalue::Reborrow(..) => self.err(span, "reborrowing a type by its `Reborrow` impl is not supported"),
			Rvalue::ThreadLocalRef(_) => self.err(span, "thread locals are not supported"),
			Rvalue::WrapUnsafeBinder(..) => self.err(span, "unsafe binders are not supported"),
			Rvalue::Cast(kind, op, to) => self.cast(*kind, op, *to, span, out),
			Rvalue::BinaryOp(BinOp::BitOr, ops)
				if let Operand::Move(p) = &ops.0
					&& let Some(&(hi, lo)) = p.as_local().and_then(|l| self.halves.get(&l)) =>
			{
				let hi = self.half(hi, span, out);
				let lo = self.half(lo, span, out);
				Expr::rt("u64FromHalves", vec![hi, lo])
			}
			Rvalue::BinaryOp(op, ops) => {
				let (l, r) = &**ops;
				let lty = l.ty(&self.mir.local_decls, tcx);
				let rty = r.ty(&self.mir.local_decls, tcx);
				let op = op.overflowing_to_wrapping().unwrap_or(*op);
				let op = match op {
					BinOp::AddUnchecked => BinOp::Add,
					BinOp::SubUnchecked => BinOp::Sub,
					BinOp::MulUnchecked => BinOp::Mul,
					BinOp::ShlUnchecked => BinOp::Shl,
					BinOp::ShrUnchecked => BinOp::Shr,
					op => op,
				};
				if matches!(op, BinOp::Cmp | BinOp::Offset) {
					return Some(self.err(span, format!("`{op:?}` is not supported")));
				}
				let lv = self.operand(l, span, out);
				let rv = self.operand(r, span, out);
				let l = num::Operand {
					expr: lv,
					value: self.const_int(l),
				};
				let r = num::Operand {
					expr: rv,
					value: self.const_int(r),
				};
				num::binary(self.b, op, l, r, lty, rty, span)
			}
			Rvalue::UnaryOp(op, operand) => {
				let aty = operand.ty(&self.mir.local_decls, tcx);
				let value = self.operand(operand, span, out);
				match op {
					UnOp::Not | UnOp::Neg => num::unary(self.b, *op, value, aty, span),
					UnOp::PtrMetadata => match aty.builtin_deref(true).map(|t| t.kind()) {
						Some(ty::Str) => Expr::rt("stringLen", vec![value]),
						Some(ty::Slice(_)) => value.member("length"),
						_ => self.err(span, format!("the metadata of `{aty}`")),
					},
				}
			}
			Rvalue::Discriminant(place) => {
				let pty = place.ty(&self.mir.local_decls, tcx).ty;
				let scrut = self.place_expr(*place, span, out);
				self.discriminant(scrut, pty, span, out)
			}
			Rvalue::Aggregate(kind, ops) => self.aggregate(kind, ops, ty, dest, span, out)?,
		})
	}

	/// An unsigned integer of at most 32 bits, as a `u32`.
	fn half(&mut self, op: &Operand<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Expr {
		let from = op.ty(&self.mir.local_decls, self.tcx);
		let expr = self.operand(op, span, out);
		let value = num::Operand {
			expr,
			value: self.const_int(op),
		};
		num::cast(self.b, value, from, self.tcx.types.u32, span)
	}

	/// The integer an operand has, if it is a constant one.
	pub(super) fn const_int(&self, op: &Operand<'tcx>) -> Option<i128> {
		let Operand::Constant(c) = op else {
			return None;
		};
		let mir::Const::Val(mir::ConstValue::Scalar(rustc_middle::mir::interpret::Scalar::Int(int)), ty) = c.const_
		else {
			return None;
		};
		let i = Int::of(ty)?;
		let size = rustc_abi::Size::from_bits(i.bits);
		Some(match i.signed {
			true => int.to_int(size),
			false => int.to_uint(size) as i128,
		})
	}

	fn repeat(&mut self, op: &Operand<'tcx>, count: ty::Const<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Expr {
		let tcx = self.tcx;
		let Some(n) = count.try_to_target_usize(tcx) else {
			return self.err(span, "an array length that is not a constant");
		};
		let elem_ty = op.ty(&self.mir.local_decls, tcx);
		if self.b.t.drops(elem_ty) {
			return self.err(
				span,
				"an array repeat of a value that drops, which is not supported yet",
			);
		}
		let v = self.operand(op, span, out);
		match elem_ty.kind() {
			ty::Uint(ty::UintTy::U8) => {
				let array = Expr::rt("bytes", vec![Expr::lit(n.to_string())]);
				match &v {
					Expr::Lit(text) if text == "0" => array,
					_ => array.member("fill").call(vec![v]),
				}
			}
			_ => {
				let v = self.path(v, "item", out);
				let item = self.b.copied(v, elem_ty);
				let item_ty = self.b.t.ty(elem_ty, span);
				Expr::ident("Array").member("from").call(vec![
					Expr::Object(vec![(Key::Name("length".into()), Expr::lit(n.to_string()))]),
					Expr::Arrow(Vec::new(), item_ty, vec![StmtKind::Return(Some(item.typed())).at(None)]),
				])
			}
		}
	}

	fn cast(&mut self, kind: CastKind, op: &Operand<'tcx>, to: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Expr {
		let tcx = self.tcx;
		let from = op.ty(&self.mir.local_decls, tcx);
		match kind {
			CastKind::IntToInt => {
				let value = self.operand(op, span, out);
				let operand = num::Operand {
					expr: value,
					value: self.const_int(op),
				};
				num::cast(self.b, operand, from, to, span)
			}
			CastKind::PointerCoercion(coercion, _) => match coercion {
				PointerCoercion::Unsize if let Some(tr) = self.b.fmt_object(to) => {
					let value = self.operand(op, span, out);
					let value = self.path(value, "arg", out);
					self.b.fmt_value(tr, from, value, span, out)
				}
				PointerCoercion::Unsize if self.b.trait_object(to) => self.err(span, format!("a coercion to `{to}`")),
				PointerCoercion::Unsize | PointerCoercion::ReifyFnPointer(_) | PointerCoercion::ClosureFnPointer(_) => {
					self.operand(op, span, out)
				}
				_ => self.err(span, format!("unsupported pointer coercion {coercion:?}")),
			},
			// A box's pointer is its contents in JS.
			CastKind::BoxDerefTransmute => self.operand(op, span, out),
			// An async body's context passes through `ResumeTy` and `NonNull`.
			CastKind::Transmute if context_like(self, from) && context_like(self, to) => self.operand(op, span, out),
			_ => self.err(span, format!("a `{kind:?}` cast from `{from}` to `{to}`")),
		}
	}

	/// A discriminant as its number.
	fn discriminant(&mut self, scrut: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Expr {
		match ty.kind() {
			ty::Coroutine(def_id, args) => {
				let info = super::coroutine::info(self.b.t, *def_id, args, span);
				scrut.field(info.state)
			}
			ty::Adt(adt, _) if adt.is_enum() => {
				let tcx = self.tcx;
				let discrs: Vec<(VariantIdx, u128)> = adt.discriminants(tcx).map(|(i, d)| (i, d.val)).collect();
				// Each discriminant in the enum's own discriminant type, as `as` reads it.
				let int = Int::of(ty.discriminant_ty(tcx)).expect("an enum's discriminant is an integer");
				let discrs: Vec<(VariantIdx, u128)> = discrs
					.into_iter()
					.map(|(i, d)| (i, int.fold(d as i128) as u128))
					.collect();
				match Std::of(tcx, adt.did()) {
					Some(Std::Option | Std::Poll | Std::Result) => {
						let scrut = self.path(scrut, "scrutinee", out);
						let first = self.variant_test(&scrut, ty, discrs[0].1, span);
						Expr::Cond(
							Box::new(first),
							Box::new(int.lit(discrs[0].1 as i128)),
							Box::new(int.lit(discrs[1].1 as i128)),
						)
					}
					None if adt.did().is_local() => {
						let info = self.b.t.adt(*adt, span);
						let table = discrs
							.iter()
							.map(|(i, d)| (Key::Name(adt.variant(*i).name.to_string()), int.lit(*d as i128)))
							.collect();
						let key = match info.repr {
							AdtRepr::Tags => scrut,
							_ => scrut.member("tag"),
						};
						Expr::Index(Box::new(Expr::Object(table)), Box::new(key))
					}
					_ => self.err(span, format!("the discriminant of `{ty}`")),
				}
			}
			_ => self.err(span, format!("the discriminant of `{ty}`")),
		}
	}

	fn aggregate(
		&mut self,
		kind: &AggregateKind<'tcx>,
		ops: &IndexVec<FieldIdx, Operand<'tcx>>,
		ty: Ty<'tcx>,
		dest: Place<'tcx>,
		span: Span,
		out: &mut Vec<Stmt>,
	) -> Option<Expr> {
		match kind {
			AggregateKind::Closure(def_id, args) => return Some(self.closure(*def_id, args, ops, span, out)),
			AggregateKind::Coroutine(def_id, args) => {
				return Some(super::coroutine::construct(self, *def_id, args, ops, span, out));
			}
			AggregateKind::CoroutineClosure(..) => return Some(self.err(span, "async closures are not supported")),
			AggregateKind::RawPtr(..) => return Some(self.err(span, "raw pointers are not supported")),
			_ => {}
		}
		let mut values = Vec::with_capacity(ops.len());
		for op in ops.iter() {
			let v = self.operand(op, span, out);
			values.push(v);
		}
		let _ = dest;
		self.parts = Some(values.clone());
		Some(match kind {
			AggregateKind::Tuple if values.is_empty() => Expr::undefined(),
			AggregateKind::Tuple => {
				let ts_ty = self.b.t.ty(ty, span);
				Expr::As(Box::new(Expr::Array(values)), ts_ty)
			}
			AggregateKind::Array(elem) => match elem.kind() {
				ty::Uint(ty::UintTy::U8) => Expr::ident("Uint8Array").member("of").call(values),
				_ => Expr::Array(values),
			},
			AggregateKind::Adt(def_id, variant, args, _, active) => {
				if active.is_some() {
					return Some(self.err(span, "unions are not supported"));
				}
				self.adt(*def_id, *variant, args, values, ty, span)
			}
			_ => unreachable!(),
		})
	}

	pub(super) fn adt(
		&mut self,
		def_id: rustc_hir::def_id::DefId,
		variant: VariantIdx,
		args: ty::GenericArgsRef<'tcx>,
		mut values: Vec<Expr>,
		ty: Ty<'tcx>,
		span: Span,
	) -> Expr {
		let tcx = self.tcx;
		let adt = tcx.adt_def(def_id);
		let def = adt.variant(variant);
		if let Some(std) = Std::of(tcx, def_id) {
			return match (std, variant.index()) {
				(Std::Option, 0) => Expr::undefined(),
				(Std::Poll, 1) => Expr::Rt("Pending".into()),
				(Std::Option | Std::Poll, _) => values.pop().unwrap(),
				(Std::Result, 0) => Expr::rt("Ok", values),
				(Std::Result, _) => Expr::rt("Err", values),
				(Std::FmtError, _) => Expr::New(Box::new(Expr::Rt("FmtError".into())), Vec::new()),
				// `ResumeTy`, an async body's context.
				(Std::Context, _) => values.pop().unwrap_or_else(Expr::undefined),
				(Std::RangeTo, _) => Expr::Object(vec![(Key::Name("end".into()), values.pop().unwrap())]),
				(Std::RangeFrom, _) => Expr::Object(vec![(Key::Name("start".into()), values.pop().unwrap())]),
				(Std::RangeFull, _) => Expr::undefined(),
				(Std::ControlFlow, _) => super::call::control_flow(self, ty, variant, values, span),
				(Std::Range, _) => {
					let ty::Adt(_, gargs) = ty.kind() else {
						unreachable!("a range is a struct")
					};
					if let Some(int) = Int::of(gargs.type_at(0))
						&& int.big()
					{
						values.push(int.func("step"));
					}
					Expr::New(Box::new(Expr::Rt("Range".into())), values)
				}
				(Std::AssertKind, _) => match def.name.as_str() {
					"Eq" => Expr::Str("==".into()),
					"Ne" => Expr::Str("!=".into()),
					_ => self.err(span, format!("constructing `{ty}`")),
				},
				_ => self.err(span, format!("constructing `{ty}`")),
			};
		}
		if super::place::transparent(self.b.t, ty) {
			return values.pop().unwrap_or_else(Expr::undefined);
		}
		match crate::lower::library::std_path(tcx, def_id)
			.iter()
			.map(String::as_str)
			.collect::<Vec<_>>()
			.as_slice()
		{
			["marker", "PhantomData"] => return Expr::undefined(),
			["iter", "sources", "empty", "Empty"] => return Expr::rt("iterEmpty", Vec::new()),
			_ => {}
		}
		if !def_id.is_local() {
			return self.err(span, format!("constructing `{ty}` from another crate"));
		}
		let info = self.b.t.adt(adt, span);
		match info.repr {
			AdtRepr::Struct { ctor, fields } => {
				let field_arg = |i: usize, value: Expr| {
					let generic = tcx
						.type_of(def.fields[FieldIdx::from_usize(i)].did)
						.instantiate_identity()
						.skip_norm_wip();
					match ty::TypeVisitableExt::has_param(&generic) {
						true => value,
						false => value.typed(),
					}
				};
				let values = values.into_iter().enumerate().map(|(i, v)| field_arg(i, v)).collect();
				self.b.t.construct(info.sym, &ctor, &fields, values)
			}
			AdtRepr::Tags => {
				let ts_ty = self.b.t.ty(ty, span);
				Expr::As(Box::new(Expr::Str(def.name.to_string())), ts_ty)
			}
			AdtRepr::Union => {
				let mut props = vec![(Key::Name("tag".into()), Expr::Str(def.name.to_string()))];
				for (i, value) in values.into_iter().enumerate() {
					let name = field_name(def.fields[FieldIdx::from_usize(i)].name.as_str(), i);
					props.push((Key::Name(name), value));
				}
				let _ = args;
				let ts_ty = self.b.t.ty(ty, span);
				Expr::As(Box::new(Expr::Object(props)), ts_ty)
			}
		}
	}

	/// A closure that captures nothing, as an arrow function.
	pub(super) fn bare_closure(
		&mut self,
		def_id: rustc_hir::def_id::DefId,
		args: ty::GenericArgsRef<'tcx>,
		span: Span,
	) -> Expr {
		self.closure(def_id, args, &IndexVec::new(), span, &mut Vec::new())
	}

	/// A closure, as an arrow function over what it captured.
	fn closure(
		&mut self,
		def_id: rustc_hir::def_id::DefId,
		args: ty::GenericArgsRef<'tcx>,
		ops: &IndexVec<FieldIdx, Operand<'tcx>>,
		span: Span,
		out: &mut Vec<Stmt>,
	) -> Expr {
		let tcx = self.tcx;
		// A capture by reference reads the place it borrows; one by value is the value as
		// it was when the closure was made, a reference held in a variable included.
		let mut captures = Vec::with_capacity(ops.len());
		for op in ops.iter() {
			let ty = op.ty(&self.mir.local_decls, tcx);
			let value = self.operand(op, span, out);
			let in_var = op
				.place()
				.and_then(|p| p.as_local())
				.is_some_and(|l| matches!(self.slots[l], Slot::Var(_)));
			let snapshot = match ty.kind() {
				ty::Ref(..) => in_var && !self.stable_name(&value),
				_ => true,
			};
			let value = match snapshot && !value.is_constant() {
				false => value,
				true => {
					let name = self.b.fresh("captured");
					let stmt = StmtKind::decl(false, name.clone(), None, Some(value)).at(None);
					self.emit(vec![stmt], None, false, out);
					Expr::ident(name)
				}
			};
			captures.push(value);
		}
		let instance = Instance::new_raw(def_id, args);
		let Some(mir) = super::instance_mir(self.b.t, instance, span) else {
			return Expr::undefined();
		};
		let sig = tcx.instantiate_bound_regions_with_erased(args.as_closure().sig());
		let ret = self.b.t.ret_ty(sig.output(), span);
		let mut inner = Fx::new(&mut *self.b, &mir, Env::Closure(captures), span);
		let params = inner.params();
		let body = inner.lower();
		Expr::Arrow(params, ret, body)
	}
}

/// Whether a `ty`'s JS value is the task's context: `&mut Context`, or the `ResumeTy` and
/// `NonNull` an async body passes it in.
fn context_like<'tcx>(fx: &Fx<'_, '_, '_, 'tcx>, ty: Ty<'tcx>) -> bool {
	let tcx = fx.tcx;
	match ty.kind() {
		ty::Ref(_, inner, _) => context_like(fx, *inner),
		ty::Adt(adt, args) => match Std::of(tcx, adt.did()) {
			Some(Std::Context) => true,
			_ if super::place::transparent(fx.b.t, ty) => args.types().next().is_none_or(|t| context_like(fx, t)),
			_ => false,
		},
		_ => false,
	}
}

/// Whether evaluating `rvalue` can do or throw anything.
pub fn effect<'tcx>(rvalue: &Rvalue<'tcx>, fx: &Fx<'_, '_, '_, 'tcx>) -> bool {
	match rvalue {
		Rvalue::BinaryOp(op, ops) => {
			let ty = ops.0.ty(&fx.mir.local_decls, fx.tcx);
			Int::of(ty.peel_refs()).is_some()
				&& !matches!(
					op,
					BinOp::Eq
						| BinOp::Ne
						| BinOp::Lt
						| BinOp::Le
						| BinOp::Gt
						| BinOp::Ge
						| BinOp::BitAnd
						| BinOp::BitOr
						| BinOp::BitXor
				)
		}
		Rvalue::UnaryOp(UnOp::Neg, _) => true,
		_ => false,
	}
}

/// Whether reading `p` reads memory something else could change.
fn place_memory<'tcx>(p: &Place<'tcx>, fx: &Fx<'_, '_, '_, 'tcx>) -> bool {
	!p.projection.is_empty()
		|| fx.borrowed[p.local]
		|| matches!(fx.slots[p.local], Slot::Alias(_))
		|| p.local.as_u32() == 1 && !matches!(fx.env, Env::Plain | Env::Shim)
}

/// Whether reading `op` reads memory something could change, a local changed in place
/// included: a copy of one held past a write to its field would see the write.
pub fn operand_memory<'tcx>(op: &Operand<'tcx>, fx: &Fx<'_, '_, '_, 'tcx>) -> bool {
	match op {
		Operand::Copy(p) | Operand::Move(p) => place_memory(p, fx) || fx.mutated[p.local],
		_ => false,
	}
}

/// Whether evaluating `rvalue` reads memory that something else could change.
pub fn memory<'tcx>(rvalue: &Rvalue<'tcx>, fx: &Fx<'_, '_, '_, 'tcx>) -> bool {
	let place = |p: &Place<'tcx>| place_memory(p, fx);
	let operand = |op: &Operand<'tcx>| operand_memory(op, fx);
	match rvalue {
		Rvalue::Use(op, _) | Rvalue::Repeat(op, _) | Rvalue::Cast(_, op, _) | Rvalue::UnaryOp(_, op) => operand(op),
		Rvalue::BinaryOp(_, ops) => operand(&ops.0) || operand(&ops.1),
		Rvalue::Ref(_, _, p) | Rvalue::Discriminant(p) | Rvalue::CopyForDeref(p) | Rvalue::RawPtr(_, p) => place(p),
		Rvalue::Aggregate(_, ops) => ops.iter().any(operand),
		Rvalue::ThreadLocalRef(_) | Rvalue::WrapUnsafeBinder(..) | Rvalue::Reborrow(..) => true,
	}
}
