//! Places, operands, and constants.
//!
//! A reference is its referent in JS, so `Deref` reads nothing and a field of a borrowed
//! struct is a member of the same object. A reference assigned once from a place that
//! stays put reads as that place, which is how a `&mut` to a number changes the number.

use rustc_abi::{FieldIdx, VariantIdx};
use rustc_middle::mir::interpret::{GlobalId, Scalar};
use rustc_middle::mir::{self, ConstValue, Local, Operand, Place, PlaceTy, ProjectionElem};
use rustc_middle::ty::{self, Instance, Ty};
use rustc_span::Span;

use super::{Env, Fx, Slot};
use crate::lower::library::Std;
use crate::lower::num::Int;
use crate::lower::{AdtRepr, ObjectKind};
use crate::ts::{Expr, Stmt, StmtKind};

impl<'m, 'b, 'a, 'tcx> Fx<'m, 'b, 'a, 'tcx> {
	/// A local read whole.
	pub(super) fn local_expr(&mut self, local: Local, span: Span) -> Expr {
		if local.as_u32() == 1 {
			match &self.env {
				Env::Method if self.self_var => {}
				Env::Method | Env::Coroutine => return Expr::This,
				Env::Closure(_) => return self.err(span, "a closure's environment read whole"),
				Env::Plain | Env::Shim => {}
			}
		}
		match &self.slots[local] {
			Slot::Var(name) => Expr::ident(name),
			Slot::Unit => Expr::undefined(),
			Slot::Alias(Some(target)) => target.clone(),
			Slot::Alias(None) => self.err(span, "a reference read before its borrow"),
			Slot::Discr(_) => self.err(span, "a discriminant read other than by a switch"),
		}
	}

	/// The JS expression of `place`, read without a copy.
	pub fn place_expr(&mut self, place: Place<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Expr {
		let tcx = self.tcx;
		let decls = &self.mir.local_decls;
		let mut pty = PlaceTy::from_ty(decls[place.local].ty);
		let mut projection = place.projection.iter().peekable();
		// A closure's captures, and a coroutine's state, are `_1`'s fields.
		let mut e = match (&self.env, place.local.as_u32()) {
			(Env::Closure(captures), 1) => {
				let captures = captures.clone();
				if matches!(projection.peek(), Some(ProjectionElem::Deref)) {
					pty = pty.projection_ty(tcx, ProjectionElem::<Local, Ty<'tcx>>::Deref);
					projection.next();
				}
				match projection.next() {
					Some(elem @ ProjectionElem::Field(field, _)) => {
						pty = pty.projection_ty(tcx, elem);
						match captures.get(field.as_usize()) {
							Some(capture) => capture.clone(),
							None => return self.err(span, "a capture the closure doesn't have"),
						}
					}
					_ => return self.err(span, "a closure's environment read whole"),
				}
			}
			// A field of an aggregate made once and never changed is its operand: a
			// reference, which borrow checking keeps as it was, or a path that stays put.
			(_, _)
				if let Some(parts) = self.var_parts.get(&place.local)
					&& let Some(ProjectionElem::Field(field, field_ty)) = projection.peek()
					&& let Some(part) = parts.get(field.as_usize())
					&& (matches!(field_ty.kind(), ty::Ref(..))
						|| part.is_constant()
						|| matches!(part, Expr::Ident(_))) =>
			{
				let part = part.clone();
				let elem = *projection.peek().unwrap();
				pty = pty.projection_ty(tcx, elem);
				projection.next();
				part
			}
			_ => self.local_expr(place.local, span),
		};
		// Checked arithmetic's pair is its value, and `.1` is whether it overflowed, which
		// it didn't: it would have panicked.
		if self.checked.contains(&place.local)
			&& let Some(ProjectionElem::Field(field, _)) = projection.peek()
		{
			let field = *field;
			projection.next();
			if field.as_u32() == 1 {
				return Expr::lit("false");
			}
			pty = PlaceTy::from_ty(match decls[place.local].ty.kind() {
				ty::Tuple(tys) => tys[0],
				_ => unreachable!("a checked pair is a tuple"),
			});
		}
		for elem in projection {
			let ty = pty.ty;
			match elem {
				ProjectionElem::Deref => {
					// A box's pointer is the box in JS: one to an object shares it.
					if let ty::RawPtr(pointee, _) = ty.kind()
						&& !matches!(self.slots[place.local], Slot::Alias(_))
						&& !(matches!(self.env, Env::Shim) && place.local.as_u32() == 1)
						&& !(self.box_pointer(place.local) && self.b.t.object_kind(*pointee).is_some())
					{
						return self.err(span, "a raw pointer dereferenced");
					}
				}
				ProjectionElem::Field(field, _) => {
					e = self.field(e, ty, pty.variant_index, field, span);
				}
				ProjectionElem::Downcast(..) => {}
				ProjectionElem::Index(index) => {
					let i = self.local_expr(index, span);
					e = e.index(i);
				}
				ProjectionElem::ConstantIndex {
					offset,
					min_length: _,
					from_end,
				} => {
					e = match from_end {
						false => e.index(Expr::lit(offset.to_string())),
						true => {
							let base = self.path(e, "slice", out);
							let len = base.clone().member("length");
							base.index(len.binary("-", Expr::lit(offset.to_string())))
						}
					};
				}
				ProjectionElem::Subslice { from, to, from_end } => {
					let elem_ty = match ty.kind() {
						ty::Array(elem, _) | ty::Slice(elem) => *elem,
						_ => return self.err(span, format!("a subslice of `{ty}`")),
					};
					let method = match elem_ty.kind() {
						ty::Uint(ty::UintTy::U8) => "subarray",
						_ => "slice",
					};
					let base = self.path(e, "slice", out);
					let end = match (from_end, ty.kind()) {
						(false, _) => Expr::lit(to.to_string()),
						(true, ty::Array(_, len)) if let Some(len) = len.try_to_target_usize(tcx) => {
							Expr::lit((len - to).to_string())
						}
						(true, _) => base.clone().member("length").binary("-", Expr::lit(to.to_string())),
					};
					e = base.member(method).call(vec![Expr::lit(from.to_string()), end]);
				}
				ProjectionElem::OpaqueCast(_)
				| ProjectionElem::UnwrapUnsafeBinder(_)
				| ProjectionElem::PhantomDeref => {}
			}
			pty = pty.projection_ty(tcx, elem);
		}
		e
	}

	/// A field of `base`, a `ty`, in `variant` if it was downcast to one.
	fn field(&mut self, base: Expr, ty: Ty<'tcx>, variant: Option<VariantIdx>, field: FieldIdx, span: Span) -> Expr {
		let tcx = self.tcx;
		match ty.kind() {
			ty::Tuple(_) => base.index(Expr::lit(field.index().to_string())),
			ty::Coroutine(def_id, args) => {
				let info = super::coroutine::info(self.b.t, *def_id, args, span);
				match variant {
					None => base.field(info.upvars[field.index()]),
					Some(variant) => {
						let saved = info.layout_variant_field(variant, field);
						base.field(info.saved[saved])
					}
				}
			}
			ty::Adt(adt, _) => match Std::of(tcx, adt.did()) {
				Some(Std::Option | Std::Poll | Std::Box | Std::Pin) => base,
				// A variant MIR reads without a test, since the other can't be, is cast to it.
				Some(Std::Result) => {
					let ok = variant.map_or(0, VariantIdx::index) == 0;
					let ty::Adt(_, args) = ty.kind() else { unreachable!() };
					let (this, other) = match ok {
						true => (args.type_at(0), args.type_at(1)),
						false => (args.type_at(1), args.type_at(0)),
					};
					let uninhabited = other.is_never()
						|| matches!(other.kind(), ty::Adt(adt, _) if adt.is_enum() && adt.variants().is_empty());
					let (flag, name) = match ok {
						true => ("true", "value"),
						false => ("false", "error"),
					};
					let base = match uninhabited {
						true => {
							let variant = crate::ts::Type::Object(vec![
								("ok".into(), crate::ts::Type::Builtin(flag)),
								(name.into(), self.b.t.ty(this, span)),
							]);
							Expr::As(Box::new(base), variant)
						}
						false => base,
					};
					base.member(name)
				}
				Some(Std::Range | Std::RangeInclusive) => {
					base.member(adt.non_enum_variant().fields[field].name.as_str())
				}
				Some(Std::Context | Std::NonNull) => base,
				Some(Std::ControlFlow) => super::call::control_flow_field(self, base, ty, variant, span),
				_ if transparent(self.b.t, ty) => base,
				None if adt.did().is_local() => {
					let info = self.b.t.adt(*adt, span);
					match info.repr {
						AdtRepr::Class { fields, .. } => base.field(fields[field.index()]),
						AdtRepr::Union | AdtRepr::Tags => {
							let variant = variant.unwrap_or(rustc_abi::FIRST_VARIANT);
							let def = &adt.variant(variant).fields[field];
							base.member(crate::lower::field_name(def.name.as_str(), field.index()))
						}
					}
				}
				_ => self.err(span, format!("a field of `{ty}`")),
			},
			_ => self.err(span, format!("a field of `{ty}`")),
		}
	}

	/// An operand's value, copied if it is a `Copy` aggregate read from a place that keeps it.
	pub fn operand(&mut self, op: &Operand<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Expr {
		match op {
			// JS shares objects where Rust copies, so a `Copy` aggregate is copied where
			// either side could change in place: into a local changed in place, or out of a
			// place that could change.
			Operand::Copy(place) => {
				let ty = place.ty(&self.mir.local_decls, self.tcx).ty;
				let value = self.place_expr(*place, span, out);
				match self.force_share || self.into_frozen && self.frozen(*place) {
					true => value,
					false => self.b.copied(value, ty),
				}
			}
			Operand::Move(place) => {
				let ty = place.ty(&self.mir.local_decls, self.tcx).ty;
				let value = self.place_expr(*place, span, out);
				match self.into_frozen {
					true => value,
					false => self.b.copied(value, ty),
				}
			}
			Operand::Constant(c) => self.constant(c, span),
			Operand::RuntimeChecks(_) => Expr::lit("false"),
		}
	}

	/// Whether `place` is part of a local's own value that nothing changes in place.
	pub(super) fn frozen(&self, place: Place<'tcx>) -> bool {
		!self.mutated[place.local] && place.projection.iter().all(|e| !matches!(e, ProjectionElem::Deref))
	}

	/// The statements that assign `value` to `place`, which has projections.
	pub(super) fn store(&mut self, place: Place<'tcx>, value: Expr, span: Span) -> Vec<Stmt> {
		let mut out = Vec::new();
		// `*r = v` through a reference that is no alias replaces what it points at.
		if let [ProjectionElem::Deref] = place.projection.as_slice()
			&& !matches!(self.slots[place.local], Slot::Alias(_))
			&& !(place.local.as_u32() == 1 && matches!(self.env, Env::Closure(_)))
		{
			let pointee = place.ty(&self.mir.local_decls, self.tcx).ty;
			let target = self.local_expr(place.local, span);
			let e = match self.b.t.object_kind(pointee) {
				Some(ObjectKind::Class) if !shared(self, pointee) => {
					Expr::ident("Object").member("assign").call(vec![target, value])
				}
				Some(ObjectKind::Bytes) => target.member("set").call(vec![value]),
				_ => {
					return vec![
						StmtKind::Expr(self.err(span, format!("an assignment through a reference to `{pointee}`")))
							.at(None),
					];
				}
			};
			out.push(StmtKind::Expr(e).at(None));
			return out;
		}
		let target = self.place_expr(place, span, &mut out);
		out.push(StmtKind::Expr(target.assign(value)).at(self.b.t.loc(span)));
		out
	}

	/// `SetDiscriminant`: a coroutine's state.
	pub(super) fn set_discriminant(&mut self, place: Place<'tcx>, variant: VariantIdx, span: Span) -> Option<Stmt> {
		let ty = place.ty(&self.mir.local_decls, self.tcx).ty;
		let ty::Coroutine(def_id, args) = ty.kind() else {
			self.err(span, format!("setting the discriminant of `{ty}`"));
			return None;
		};
		let info = super::coroutine::info(self.b.t, *def_id, args, span);
		let mut out = Vec::new();
		let target = self.place_expr(place, span, &mut out);
		Some(StmtKind::Expr(target.field(info.state).assign(Expr::lit(variant.as_u32().to_string()))).at(None))
	}

	/// The test that `scrut`, a `ty`, has the variant whose discriminant is `value`.
	pub(super) fn variant_test(&mut self, scrut: &Expr, ty: Ty<'tcx>, value: u128, span: Span) -> Expr {
		let tcx = self.tcx;
		match ty.kind() {
			ty::Coroutine(def_id, args) => {
				let info = super::coroutine::info(self.b.t, *def_id, args, span);
				scrut
					.clone()
					.field(info.state)
					.binary("===", Expr::lit(value.to_string()))
			}
			ty::Adt(adt, _) if adt.is_enum() => {
				let Some((index, _)) = adt.discriminants(tcx).find(|(_, d)| d.val == value) else {
					return self.err(span, format!("no variant of `{ty}` has discriminant {value}"));
				};
				match Std::of(tcx, adt.did()) {
					Some(Std::Option) if matches!(scrut, Expr::Ident(n) if self.next_names.contains(n)) => {
						match index.index() {
							0 => scrut.clone().binary("===", Expr::Rt("DONE")),
							_ => scrut.clone().binary("!==", Expr::Rt("DONE")),
						}
					}
					Some(Std::Option) => match index.index() {
						0 => scrut.clone().binary("===", Expr::undefined()),
						_ => scrut.clone().binary("!==", Expr::undefined()),
					},
					Some(Std::Poll) => match index.index() {
						0 => scrut.clone().binary("!==", Expr::Rt("Pending")),
						_ => scrut.clone().binary("===", Expr::Rt("Pending")),
					},
					Some(Std::Result) => match index.index() {
						0 => scrut.clone().member("ok"),
						_ => scrut.clone().member("ok").not(),
					},
					None if adt.did().is_local() => {
						let info = self.b.t.adt(*adt, span);
						let name = Expr::Str(adt.variant(index).name.to_string());
						match info.repr {
							AdtRepr::Tags => scrut.clone().binary("===", name),
							AdtRepr::Union => scrut.clone().member("tag").binary("===", name),
							AdtRepr::Class { .. } => self.err(span, format!("a switch on `{ty}`")),
						}
					}
					Some(Std::ControlFlow) => super::call::control_flow_test(self, scrut, ty, index, span),
					_ => self.err(span, format!("a switch on `{ty}`")),
				}
			}
			_ => self.err(span, format!("a switch on the discriminant of `{ty}`")),
		}
	}

	/// A constant operand's value.
	pub fn constant(&mut self, c: &mir::ConstOperand<'tcx>, span: Span) -> Expr {
		let tcx = self.tcx;
		let env = self.b.t.typing_env_pub();
		match c.const_ {
			mir::Const::Ty(ty, ct) => match ct.kind() {
				ty::ConstKind::Value(v) => self.b.value(v.ty, v.valtree, span),
				_ => self.err(span, format!("a constant of type `{ty}` that is no value")),
			},
			mir::Const::Unevaluated(uv, ty) => {
				let instance = match Instance::try_resolve(tcx, env, uv.def, uv.args) {
					Ok(Some(instance)) => instance,
					_ => return self.err(span, "a constant that does not resolve"),
				};
				let cid = GlobalId {
					instance,
					promoted: uv.promoted,
				};
				match tcx.const_eval_global_id_for_typeck(env, cid, span) {
					Ok(Ok(valtree)) => self.b.value(ty, valtree, span),
					Ok(Err(_)) => self.err(span, format!("a constant of type `{ty}`, which has no value tree")),
					Err(_) => self.err(span, "a constant that failed to evaluate"),
				}
			}
			mir::Const::Val(val, ty) => self.const_value(val, ty, span),
		}
	}

	fn const_value(&mut self, val: ConstValue, ty: Ty<'tcx>, span: Span) -> Expr {
		let tcx = self.tcx;
		// An aggregate, field by field.
		if let ty::Adt(..) | ty::Tuple(_) | ty::Array(..) = ty.kind()
			&& !matches!(ty.kind(), ty::Tuple(tys) if tys.is_empty())
			&& !matches!(ty.kind(), ty::Adt(adt, _) if adt.is_struct() && adt.did().is_local() && adt.all_fields().next().is_none())
		{
			let Some(d) = tcx.try_destructure_mir_constant_for_user_output(val, ty) else {
				return self.err(span, format!("a constant of type `{ty}` that can't be taken apart"));
			};
			let values: Vec<Expr> = d.fields.iter().map(|(v, t)| self.const_value(*v, *t, span)).collect();
			return match ty.kind() {
				ty::Tuple(_) => {
					let ts_ty = self.b.t.ty(ty, span);
					Expr::As(Box::new(Expr::Array(values)), ts_ty)
				}
				ty::Array(elem, _) => match elem.kind() {
					ty::Uint(ty::UintTy::U8) => Expr::ident("Uint8Array").member("of").call(values),
					_ => Expr::Array(values),
				},
				ty::Adt(adt, args) => self.adt(
					adt.did(),
					d.variant.unwrap_or(rustc_abi::FIRST_VARIANT),
					args,
					values,
					ty,
					span,
				),
				_ => unreachable!(),
			};
		}
		match val {
			ConstValue::ZeroSized => match *ty.kind() {
				ty::FnDef(def_id, args) => self.b.fn_value(def_id, args.skip_binder(), span),
				ty::Tuple(tys) if tys.is_empty() => Expr::undefined(),
				ty::Adt(adt, _) if adt.is_struct() && adt.did().is_local() => {
					let info = self.b.t.adt(adt, span);
					Expr::New(Box::new(Expr::Sym(info.sym)), Vec::new())
				}
				ty::Array(..) => Expr::Array(Vec::new()),
				ty::Closure(def_id, args) => self.bare_closure(def_id, args, span),
				ty::Adt(adt, _) if Std::of(tcx, adt.did()) == Some(Std::RangeFull) => Expr::undefined(),
				_ => self.err(span, format!("a zero-sized constant of type `{ty}`")),
			},
			ConstValue::Scalar(Scalar::Int(int)) => match ty.kind() {
				ty::Bool => Expr::lit((int.to_bits_unchecked() != 0).to_string()),
				_ => match Int::of(ty) {
					Some(i) => {
						let size = rustc_abi::Size::from_bits(i.bits);
						let value = match i.signed {
							true => int.to_int(size),
							false => int.to_uint(size) as i128,
						};
						i.lit(value)
					}
					None => self.err(span, format!("a scalar constant of type `{ty}`")),
				},
			},
			ConstValue::Scalar(Scalar::Ptr(ptr, _)) => {
				// `&[u8; N]`, such as a byte string.
				let ty::Ref(_, inner, _) = ty.kind() else {
					return self.err(span, format!("a pointer constant of type `{ty}`"));
				};
				let ty::Array(elem, len) = inner.kind() else {
					return self.err(span, format!("a constant of type `{ty}`"));
				};
				let (Some(len), ty::Uint(ty::UintTy::U8)) = (len.try_to_target_usize(tcx), elem.kind()) else {
					return self.err(span, format!("a constant of type `{ty}`"));
				};
				let (prov, offset) = ptr.prov_and_relative_offset();
				let alloc = tcx.global_alloc(prov.alloc_id()).unwrap_memory();
				let start = offset.bytes() as usize;
				let bytes = alloc
					.inner()
					.inspect_with_uninit_and_ptr_outside_interpreter(start..start + len as usize);
				let items = bytes.iter().map(|b| Expr::lit(b.to_string())).collect();
				Expr::ident("Uint8Array").member("of").call(items)
			}
			ConstValue::Slice { .. } => {
				let Some(bytes) = val.try_get_slice_bytes_for_diagnostics(tcx) else {
					return self.err(span, "a slice constant out of bounds");
				};
				match ty.peel_refs().kind() {
					ty::Str => match std::str::from_utf8(bytes) {
						Ok(s) => Expr::Str(s.to_string()),
						Err(_) => self.err(span, "a string constant that is not UTF-8"),
					},
					ty::Slice(elem) if matches!(elem.kind(), ty::Uint(ty::UintTy::U8)) => {
						let items = bytes.iter().map(|b| Expr::lit(b.to_string())).collect();
						Expr::ident("Uint8Array").member("of").call(items)
					}
					_ => self.err(span, format!("a slice constant of type `{ty}`")),
				}
			}
			ConstValue::Indirect { .. } => self.err(span, format!("a constant of type `{ty}` held in memory")),
		}
	}

	/// The constant an operand is, through a temporary that holds only it, which is
	/// then no longer pending.
	pub(super) fn const_operand(&mut self, op: &Operand<'tcx>) -> Option<Box<mir::ConstOperand<'tcx>>> {
		match op {
			Operand::Constant(c) => Some(c.clone()),
			Operand::Copy(place) | Operand::Move(place) => {
				let local = place.as_local()?;
				if self.defs[local] != 1 {
					return None;
				}
				let Some(mir::Rvalue::Use(Operand::Constant(c), _)) = self.def_rvalue(local) else {
					return None;
				};
				let c = c.clone();
				if let Some(name) = self.var(local) {
					let name = name.to_string();
					self.pending.retain(|p| p.name != name);
				}
				Some(c)
			}
			_ => None,
		}
	}

	/// The bytes of a byte string constant, `&[u8; N]`.
	pub(super) fn const_bytes(&mut self, op: &Operand<'tcx>) -> Option<Vec<u8>> {
		let tcx = self.tcx;
		let c = self.const_operand(op)?;
		let mir::Const::Val(ConstValue::Scalar(Scalar::Ptr(ptr, _)), ty) = c.const_ else {
			return None;
		};
		let ty::Ref(_, inner, _) = ty.kind() else {
			return None;
		};
		let ty::Array(_, len) = inner.kind() else {
			return None;
		};
		let len = len.try_to_target_usize(tcx)? as usize;
		let (prov, offset) = ptr.prov_and_relative_offset();
		let alloc = tcx.global_alloc(prov.alloc_id()).unwrap_memory();
		let start = offset.bytes() as usize;
		Some(
			alloc
				.inner()
				.inspect_with_uninit_and_ptr_outside_interpreter(start..start + len)
				.to_vec(),
		)
	}
}

/// Whether `ty` is a struct of one field that JS represents as that field: a wrapper
/// around a pointer, such as the `ResumeTy` and `NonNull` an async body passes its
/// context in.
pub fn transparent<'tcx>(t: &crate::lower::Translator<'_, 'tcx>, ty: Ty<'tcx>) -> bool {
	let ty::Adt(adt, _) = ty.kind() else {
		return false;
	};
	let path = crate::lower::library::std_path(t.tcx, adt.did());
	let path: Vec<&str> = path.iter().map(String::as_str).collect();
	matches!(
		path.as_slice(),
		["future", "ResumeTy"] | ["ptr", "non_null", "NonNull"] | ["future", _, "ResumeTy"]
	)
}

/// Whether the JS object of a `ty` is shared or private, so that copying fields into it
/// would change other handles, or nothing.
fn shared<'tcx>(fx: &Fx<'_, '_, '_, 'tcx>, ty: Ty<'tcx>) -> bool {
	let tcx = fx.tcx;
	let mut erased = ty;
	while let ty::Adt(adt, args) = erased.kind()
		&& matches!(Std::of(tcx, adt.did()), Some(Std::Pin | Std::Box))
	{
		erased = args.type_at(0);
	}
	match erased.kind() {
		ty::Adt(adt, _) => matches!(
			Std::of(tcx, adt.did()),
			Some(Std::Arc | Std::Waker | Std::Context | Std::MutexGuard)
		),
		ty::Ref(..) | ty::Coroutine(..) => true,
		_ => false,
	}
}
