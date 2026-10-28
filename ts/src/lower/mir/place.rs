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

use super::{Env, Fx, Pending, Slot};
use crate::lower::library::{Flow, Std};
use crate::lower::num::Int;
use crate::lower::{AdtRepr, ObjectKind, byte_array, bytes_ref};
use crate::ts::{Expr, Stmt, StmtKind};

impl<'m, 'b, 'a, 'tcx> Fx<'m, 'b, 'a, 'tcx> {
	/// A local read whole.
	pub(super) fn local_expr(&mut self, local: Local, span: Span) -> Expr {
		if local.as_u32() == 1 {
			match &self.env {
				Env::Coroutine => return Expr::This,
				Env::Closure(_) => return self.err(span, "a closure's environment read whole"),
				Env::Plain | Env::Shim => {}
			}
		}
		if let Some(value) = self.stepped_value(local) {
			return value;
		}
		match &self.slots[local] {
			Slot::Var(name) => Expr::ident(name),
			// A value of type `!`, as a generic instantiated at `!` reads: native code never
			// gets here, and `tsc` sees that through `rt.unreachable`.
			Slot::Unit if self.mir.local_decls[local].ty.is_never() => Expr::rt("unreachable", vec![Expr::undefined()]),
			Slot::Unit => Expr::undefined(),
			// Passed on or kept, a `&mut` to a borrowed place JS copies is its `rt.Place`.
			Slot::Alias(Some(target)) if self.points_at_place(local) => {
				let target = target.clone();
				let payload = match self.def_rvalue(local) {
					Some(mir::Rvalue::Ref(_, _, p)) => self.payload_ty(*p, span),
					_ => None,
				};
				self.place_of(target, payload)
			}
			Slot::Alias(Some(target)) => target.clone(),
			Slot::Alias(None) => self.err(span, "a reference read before its borrow"),
			Slot::Discr(_) | Slot::Flag(_) => self.err(span, "a discriminant read other than by a switch"),
		}
	}

	/// The JS expression of `place`, read without a copy.
	pub fn place_expr(&mut self, place: Place<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Expr {
		let tcx = self.tcx;
		// A local held as its parts reads them, as far as they go.
		if self.scalars.contains_key(&place.local) {
			let (e, n) = self.scalar_place(place, span);
			let mut pty = PlaceTy::from_ty(self.mir.local_decls[place.local].ty);
			for elem in &place.projection[..n] {
				pty = pty.projection_ty(tcx, *elem);
			}
			return self.project(e, pty, place, n, span, out);
		}
		let decls = &self.mir.local_decls;
		let mut pty = PlaceTy::from_ty(decls[place.local].ty);
		let mut projection = place.projection.iter().peekable();
		let held = match projection.peek() {
			Some(&ProjectionElem::Field(field, _)) if !self.mutated[place.local] => {
				self.held_part(place.local, field.as_usize())
			}
			_ => None,
		};
		// A closure's captures, and a coroutine's state, are `_1`'s fields.
		let e = match (&self.env, place.local.as_u32()) {
			(Env::Closure(captures), 1) => {
				let captures = captures.clone();
				if matches!(projection.peek(), Some(ProjectionElem::Deref)) {
					pty = pty.projection_ty(tcx, ProjectionElem::<Local, Ty<'tcx>>::Deref);
					projection.next();
				}
				match projection.next() {
					Some(elem @ ProjectionElem::Field(field, _)) => {
						pty = pty.projection_ty(tcx, elem);
						let Some(capture) = captures.get(field.as_usize()) else {
							return self.err(span, "a capture the closure doesn't have");
						};
						let (value, pointee) = (capture.value.clone(), capture.pointee);
						match projection.peek() {
							Some(ProjectionElem::Deref) if pointee => {
								pty = pty.projection_ty(tcx, ProjectionElem::<Local, Ty<'tcx>>::Deref);
								projection.next();
								value
							}
							// Passed on or kept whole, it is the place's `rt.Place`.
							None if pointee => self.place_of(value, None),
							_ => value,
						}
					}
					_ => return self.err(span, "a closure's environment read whole"),
				}
			}
			// A field of an aggregate held back for its one read.
			_ if let Some(part) = held => {
				let elem = *projection.peek().unwrap();
				pty = pty.projection_ty(tcx, elem);
				projection.next();
				part
			}
			// A field of an aggregate made once and never changed is its operand: a
			// reference, held in a path when the aggregate is made, which borrow checking
			// keeps as it was, or a name that stays put.
			(_, _)
				if let Some(parts) = self.var_parts.get(&place.local)
					&& let Some(ProjectionElem::Field(field, field_ty)) = projection.peek()
					&& let Some(part) = parts.get(field.as_usize())
					&& (matches!(field_ty.kind(), ty::Ref(..)) || part.is_constant() || self.stable_name(part)) =>
			{
				let part = part.clone();
				let elem = *projection.peek().unwrap();
				pty = pty.projection_ty(tcx, elem);
				projection.next();
				part
			}
			// An alias of a borrowed place is that place, which the dereference reads.
			_ if matches!(projection.peek(), Some(ProjectionElem::Deref)) && self.points_at_place(place.local) => {
				let Slot::Alias(Some(target)) = &self.slots[place.local] else {
					return self.err(span, "a reference read before its borrow");
				};
				let target = target.clone();
				pty = pty.projection_ty(tcx, ProjectionElem::<Local, Ty<'tcx>>::Deref);
				projection.next();
				target
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
		let from = place.projection.len() - projection.len();
		self.project(e, pty, place, from, span, out)
	}

	/// `e`, the value of `place` projected as far as `from`, a `pty`, projected the rest of
	/// the way.
	fn project(
		&mut self,
		mut e: Expr,
		mut pty: PlaceTy<'tcx>,
		place: Place<'tcx>,
		from: usize,
		span: Span,
		out: &mut Vec<Stmt>,
	) -> Expr {
		let tcx = self.tcx;
		let decls = &self.mir.local_decls;
		// Whether `e` is an `rt.ByteSlice` of the place's bytes: a `[u8]`, or a `[u8; N]`
		// behind a reference.
		let mut in_bytes = false;
		for (read, elem) in place.projection.iter().enumerate().skip(from) {
			let ty = pty.ty;
			in_bytes = in_bytes || byte_slice(ty);
			match elem {
				ProjectionElem::Deref => {
					// Drop glue's pointer is the value it drops.
					let glue =
						matches!(self.env, Env::Shim) && place.local.as_u32() == 1 && pty.ty == decls[place.local].ty;
					if self.b.t.place_pointee(ty).is_some() && !glue {
						e = e.member("v");
					}
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
				ProjectionElem::Downcast(_, variant) => {
					let base = Place {
						local: place.local,
						projection: tcx.mk_place_elems(&place.projection[..read]),
					};
					if self.narrowing.stale(self.at, base) {
						e = self.as_stale_variant(e, base, ty, variant, span);
					} else if !e.path_key().is_some_and(|key| self.switched.contains(&key)) {
						e = self.as_variant(e, ty, variant, span);
					}
				}
				ProjectionElem::Index(index) => {
					let i = self.local_expr(index, span);
					e = match in_bytes {
						true => {
							let base = self.slice_path(e, out);
							byte_at(base, i)
						}
						false => e.index(i),
					};
				}
				ProjectionElem::ConstantIndex {
					offset,
					min_length: _,
					from_end,
				} => {
					let base = match from_end || in_bytes {
						true => self.slice_path(e, out),
						false => e,
					};
					let i = match from_end {
						false => Expr::lit(offset.to_string()),
						true => super::scalar::sub(slice_member(base.clone(), "length"), Expr::lit(offset.to_string())),
					};
					e = match in_bytes {
						true => byte_at(base, i),
						false => base.index(i),
					};
				}
				// A slice pattern's rest: `[a, rest @ ..]`.
				ProjectionElem::Subslice {
					from,
					to,
					from_end: true,
				} if byte_slice(ty) => {
					let base = self.slice_path(e, out);
					// A range of parts, which the pattern that took it checked the length for.
					if let Some([data, start, length]) = made_slice(&base) {
						let len = super::scalar::sub(length.clone(), Expr::lit((from + to).to_string()));
						let start = super::scalar::add(start.clone(), Expr::lit(from.to_string()));
						e = Expr::New(Box::new(Expr::Rt("ByteSlice".into())), vec![data.clone(), start, len]);
					} else {
						let mut values = vec![base.clone(), Expr::lit(from.to_string())];
						if to != 0 {
							values.push(base.member("length").binary("-", Expr::lit(to.to_string())));
						}
						e = Expr::rt("byteSliceRange", values);
					}
				}
				// Part of a `[u8; N]` behind a reference, as `[a, rest @ ..]` of a `&[u8; N]`.
				ProjectionElem::Subslice { from, to, from_end } if in_bytes => {
					let end = match (from_end, ty.kind()) {
						(false, _) => to,
						(true, ty::Array(_, len)) if let Some(len) = len.try_to_target_usize(tcx) => len - to,
						_ => return self.err(span, format!("a subslice of `{ty}`")),
					};
					let base = self.slice_path(e, out);
					e = match made_slice(&base) {
						// In bounds of the array's.
						Some([data, start, _]) => Expr::New(
							Box::new(Expr::Rt("ByteSlice".into())),
							vec![
								data.clone(),
								super::scalar::add(start.clone(), Expr::lit(from.to_string())),
								Expr::lit((end - from).to_string()),
							],
						),
						None => {
							let values = vec![base, Expr::lit(from.to_string()), Expr::lit(end.to_string())];
							Expr::rt("byteSliceRange", values)
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
			in_bytes = match elem {
				ProjectionElem::Deref => bytes_ref(ty),
				ProjectionElem::Subslice { .. } => in_bytes,
				_ => false,
			};
			pty = pty.projection_ty(tcx, elem);
		}
		e
	}

	/// Whether `place` is a `[u8; N]` read through a reference, whose JS value is an
	/// `rt.ByteSlice` of its bytes rather than a `Uint8Array`, as [`Self::place_expr`]
	/// reads it.
	pub(super) fn in_bytes(&self, place: Place<'tcx>) -> bool {
		let mut in_bytes = false;
		for (base, elem) in place.iter_projections() {
			let ty = base.ty(&self.mir.local_decls, self.tcx).ty;
			in_bytes = match elem {
				ProjectionElem::Deref => bytes_ref(ty),
				ProjectionElem::Subslice { .. } => in_bytes || byte_slice(ty),
				_ => false,
			};
		}
		in_bytes && byte_array(place.ty(&self.mir.local_decls, self.tcx).ty)
	}

	/// An error if `kind` borrows a subslice of other than bytes mutably: a JS array has
	/// no views, so it would change a copy.
	pub(super) fn mut_subslice(&mut self, kind: mir::BorrowKind, place: Place<'tcx>, span: Span) -> Option<Expr> {
		if !matches!(kind, mir::BorrowKind::Mut { .. }) {
			return None;
		}
		let elem = place.iter_projections().find_map(|(base, elem)| match elem {
			ProjectionElem::Subslice { .. } => match base.ty(&self.mir.local_decls, self.tcx).ty.kind() {
				ty::Array(elem, _) | ty::Slice(elem) if !matches!(elem.kind(), ty::Uint(ty::UintTy::U8)) => Some(*elem),
				_ => None,
			},
			_ => None,
		})?;
		Some(self.err(
			span,
			format!("a mutable borrow of a subslice of `{elem}`, which JS would copy, having no views"),
		))
	}

	/// `base`, a `ty`, cast to its `variant`, which MIR reads without a switch where the
	/// others can't be, as a derived `==` reads the other operand after comparing
	/// discriminants. `tsc` narrows only what a switch tests.
	fn as_variant(&mut self, base: Expr, ty: Ty<'tcx>, variant: VariantIdx, span: Span) -> Expr {
		match ty.kind() {
			ty::Adt(adt, _) if adt.did().is_local() => match self.variant_ty(ty, variant, span) {
				Some(variant) => Expr::As(Box::new(base), variant),
				None => base,
			},
			_ => base,
		}
	}

	/// `base`, the `place` of a `ty`, cast to its `variant` through its whole type:
	/// TypeScript may have narrowed it to another before a change it doesn't see.
	fn as_stale_variant(
		&mut self,
		base: Expr,
		place: Place<'tcx>,
		ty: Ty<'tcx>,
		variant: VariantIdx,
		span: Span,
	) -> Expr {
		// `rt.Entry` is one class for either variant, which TypeScript narrows nothing by.
		if matches!(ty.kind(), ty::Adt(adt, _) if matches!(Std::of(self.tcx, adt.did()), Some(Std::Entry))) {
			return base;
		}
		let Some(variant) = self.variant_ty(ty, variant, span) else {
			return self.err(
				span,
				format!("a payload of `{ty}` read where TypeScript narrowed it to another variant"),
			);
		};
		let whole = match place.as_local() {
			Some(local) => self.decl_ty(local),
			None => self.b.t.ty(ty, span),
		};
		Expr::As(Box::new(Expr::As(Box::new(base), whole)), variant)
	}

	/// The JS type of `ty`'s `variant`, for a type whose variants hold their payloads:
	/// a union's member, `Result`'s and `ControlFlow`'s, or `Option`'s and `Poll`'s value.
	fn variant_ty(&mut self, ty: Ty<'tcx>, variant: VariantIdx, span: Span) -> Option<crate::ts::Type> {
		let tcx = self.tcx;
		let ty::Adt(adt, args) = ty.kind() else {
			return None;
		};
		// `rt.Result`'s variants.
		let result = |ok: bool, payload| {
			let (flag, name) = match ok {
				true => ("true", "value"),
				false => ("false", "error"),
			};
			crate::ts::Type::Object(vec![
				("ok".into(), crate::ts::Type::Builtin(flag)),
				(name.into(), payload),
			])
		};
		match Std::of(tcx, adt.did()) {
			Some(Std::Option | Std::Poll) => {
				let payload = adt.variant(variant).fields.iter().next()?;
				Some(self.b.t.ty(payload.ty(tcx, args).skip_norm_wip(), span))
			}
			Some(Std::Result) => {
				let payload = self.b.t.ty(args.type_at(variant.index()), span);
				Some(result(variant.index() == 0, payload))
			}
			// What `control_flow_field` reads of each variant.
			Some(Std::ControlFlow) => {
				let (breaks, continues) = (args.type_at(0), args.type_at(1));
				Some(match (Flow::of(tcx, args), variant.index()) {
					(Flow::Option, 0) => self.b.t.ty(continues, span),
					(Flow::Option, _) => crate::ts::Type::Builtin("undefined"),
					(Flow::Result | Flow::Other, 0) => result(true, self.b.t.ty(continues, span)),
					(Flow::Result, _) => self.b.t.ty(breaks, span),
					(Flow::Other, _) => result(false, self.b.t.ty(breaks, span)),
				})
			}
			None if self.b.t.own(adt.did()) && matches!(self.b.t.adt(*adt, span).repr, AdtRepr::Union) => {
				let def = adt.variant(variant);
				let mut fields = vec![("tag".to_string(), crate::ts::Type::StrLit(def.name.to_string()))];
				for (i, field) in def.fields.iter().enumerate() {
					let ty = self.b.t.ty(field.ty(tcx, args).skip_norm_wip(), span);
					fields.push((crate::lower::field_name(field.name.as_str(), i), ty));
				}
				Some(crate::ts::Type::Object(fields))
			}
			_ => None,
		}
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
				Some(Std::Option | Std::Poll | Std::Box | Std::Pin | Std::Reverse | Std::NonZero) => base,
				// Cast to its variant, which a test narrows only where it reads the field.
				Some(Std::Bound) => {
					let ty::Adt(_, args) = ty.kind() else { unreachable!() };
					let variant = variant.unwrap_or(rustc_abi::FIRST_VARIANT);
					let fields = vec![
						(
							"tag".to_string(),
							crate::ts::Type::StrLit(adt.variant(variant).name.to_string()),
						),
						("_0".to_string(), self.b.t.ty(args.type_at(0), span)),
					];
					Expr::As(Box::new(base), crate::ts::Type::Object(fields)).member("_0")
				}
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
				Some(Std::Range | Std::RangeInclusive | Std::RangeTo | Std::RangeFrom | Std::RangeToInclusive) => {
					base.member(adt.non_enum_variant().fields[field].name.as_str())
				}
				Some(Std::Context | Std::NonNull | Std::Entry) => base,
				Some(Std::Cow) => self.err(span, crate::lower::library::cow_variant(ty)),
				Some(Std::ControlFlow) => super::call::control_flow_field(self, base, ty, variant),
				_ if transparent(self.b.t, ty) || crate::lower::library::wrapped(tcx, ty).is_some() => base,
				None if self.b.t.own(adt.did()) => {
					let info = self.b.t.adt(*adt, span);
					match info.repr {
						AdtRepr::Struct { fields, .. } => base.field(fields[field.index()]),
						AdtRepr::Union | AdtRepr::Tags => {
							let variant = variant.unwrap_or(rustc_abi::FIRST_VARIANT);
							let def = &adt.variant(variant).fields[field];
							let name = crate::lower::field_name(def.name.as_str(), field.index());
							// A cleanup block runs in a `catch`, outside the test that narrowed it.
							let base = match self.in_cleanup.is_some() {
								true => {
									let ty::Adt(_, args) = ty.kind() else { unreachable!() };
									let v = adt.variant(variant);
									let mut fields =
										vec![("tag".to_string(), crate::ts::Type::StrLit(v.name.to_string()))];
									for (i, f) in v.fields.iter().enumerate() {
										let field_ty = self.b.t.ty(f.ty(tcx, args).skip_norm_wip(), span);
										fields.push((crate::lower::field_name(f.name.as_str(), i), field_ty));
									}
									Expr::As(Box::new(base), crate::ts::Type::Object(fields))
								}
								false => base,
							};
							base.member(name)
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
			// An array of bytes read through a reference, copied out of its slice.
			Operand::Copy(place) | Operand::Move(place) if self.in_bytes(*place) => {
				let value = self.place_expr(*place, span, out);
				Expr::rt("byteSliceToArray", vec![value])
			}
			// JS shares objects where Rust copies, so a `Copy` aggregate is copied where
			// either side could change in place: into a local changed in place, or out of a
			// place that could change.
			Operand::Copy(place) => {
				let ty = place.ty(&self.mir.local_decls, self.tcx).ty;
				if changes_captures(ty) {
					return self.err(span, "a copy of a closure that changes what it captured");
				}
				let value = self.place_expr(*place, span, out);
				match self.force_share || self.into_frozen && self.frozen(*place) {
					true => value,
					false => self.b.t.copied(value, ty, span),
				}
			}
			// Moved out of a local's own value changed in place, which is its own.
			Operand::Move(place) => {
				let ty = place.ty(&self.mir.local_decls, self.tcx).ty;
				let value = self.place_expr(*place, span, out);
				match self.into_frozen || super::own(*place) && self.mutated[place.local] {
					true => value,
					false => self.b.t.copied(value, ty, span),
				}
			}
			Operand::Constant(c) => self.constant(c, span),
			Operand::RuntimeChecks(_) => Expr::lit("false"),
		}
	}

	/// Whether `e` names a local nothing assigns again or changes in place, so reading it
	/// later reads what it held earlier.
	pub(super) fn stable_name(&self, e: &Expr) -> bool {
		let Expr::Ident(name) = e else {
			return false;
		};
		self.local_named(name).is_some_and(|local| {
			let assigned = self.defs[local] + u32::from(self.is_arg(local));
			assigned <= 1 && !self.mutated[local]
		})
	}

	/// Whether `place` is part of a local's own value that nothing changes in place.
	pub(super) fn frozen(&self, place: Place<'tcx>) -> bool {
		!self.mutated[place.local] && place.projection.iter().all(|e| !matches!(e, ProjectionElem::Deref))
	}

	/// The statements that assign `value` to `place`, which has projections.
	pub(super) fn store(&mut self, place: Place<'tcx>, value: Expr, span: Span) -> Vec<Stmt> {
		let (place, value) = self.rebuilt(place, value, span);
		let mut out = Vec::new();
		// Bytes through a reference are copied into the slice of them it holds.
		if self.in_bytes(place) {
			let target = self.place_expr(place, span, &mut out);
			let target = self.slice_path(target, &mut out);
			let set = target.clone().member("data").member("set");
			out.push(StmtKind::Expr(set.call(vec![value, target.member("start")])).at(self.b.t.loc(span)));
			return out;
		}
		// `*r = v` replaces the object `r` points at in place, which other references and
		// captures share. An alias of a borrowed place, or of a reference to no object,
		// assigns the place, and an `rt.Place` its `v`.
		if let [ProjectionElem::Deref] = place.projection.as_slice()
			&& !self.alias_of_place(place.local)
			&& self.b.t.place_pointee(self.mir.local_decls[place.local].ty).is_none()
			&& !(place.local.as_u32() == 1 && matches!(self.env, Env::Closure(_)))
		{
			let pointee = place.ty(&self.mir.local_decls, self.tcx).ty;
			let alias = matches!(self.slots[place.local], Slot::Alias(_));
			let target = self.local_expr(place.local, span);
			// A binding's value may keep what it holds where `Object.assign` can't reach.
			let bound = matches!(pointee.kind(), ty::Adt(adt, _) if self.b.t.bound_drops(adt.did()).is_some());
			let e = match self.b.t.object_kind(pointee) {
				Some(ObjectKind::Class) if !shared(self, pointee) && !bound => {
					Some(Expr::ident("Object").member("assign").call(vec![target, value.clone()]))
				}
				Some(ObjectKind::Bytes) => Some(target.member("set").call(vec![value.clone()])),
				_ => None,
			};
			match (e, alias) {
				(Some(e), _) => {
					out.push(StmtKind::Expr(e).at(None));
					return out;
				}
				(None, true) => {}
				(None, false) => {
					return vec![
						StmtKind::Expr(self.err(span, format!("an assignment through a reference to `{pointee}`")))
							.at(None),
					];
				}
			}
		}
		let target = self.place_expr(place, span, &mut out);
		out.push(StmtKind::Expr(target.assign(value)).at(self.b.t.loc(span)));
		out
	}

	/// A store of `value` to `place` as a store of the whole value to what holds it, where
	/// `place`, or what a reference to it reads, is the payload of an `Option`, `Poll`,
	/// `Result`, or `ControlFlow`: JS shares their values as immutable, so a payload
	/// changes by making its enum again.
	fn rebuilt(&mut self, place: Place<'tcx>, value: Expr, span: Span) -> (Place<'tcx>, Expr) {
		let tcx = self.tcx;
		let (mut at, mut value, mut rebuilt) = (place, value, false);
		loop {
			if let [ProjectionElem::Deref, rest @ ..] = at.projection.as_slice()
				&& self.alias_of_place(at.local)
				&& let Some(mir::Rvalue::Ref(_, _, borrowed)) = self.def_rvalue(at.local)
			{
				at = borrowed.project_deeper(rest, tcx);
				continue;
			}
			if let [ProjectionElem::Deref, rest @ ..] = at.projection.as_slice()
				&& let Some(&borrowed) = self.reborrows.get(&at.local)
			{
				at = borrowed.project_deeper(rest, tcx);
				continue;
			}
			let [.., ProjectionElem::Downcast(_, variant), ProjectionElem::Field(..)] = at.projection.as_slice() else {
				break;
			};
			let base = Place {
				local: at.local,
				projection: tcx.mk_place_elems(&at.projection[..at.projection.len() - 2]),
			};
			let ty = base.ty(&self.mir.local_decls, tcx).ty;
			let ty::Adt(adt, args) = ty.kind() else {
				break;
			};
			if !matches!(
				Std::of(tcx, adt.did()),
				Some(Std::Option | Std::Poll | Std::Result | Std::ControlFlow)
			) {
				break;
			}
			value = self.adt(adt.did(), *variant, args, vec![value], ty, span);
			(at, rebuilt) = (base, true);
		}
		match rebuilt {
			true => (at, value),
			false => (place, value),
		}
	}

	/// Whether `local` is an alias of a borrowed place, as `&mut x` or `&mut v[i]`, rather
	/// than a copy of a reference or a reborrow of what one points at.
	pub(super) fn alias_of_place(&self, local: Local) -> bool {
		matches!(self.slots[local], Slot::Alias(_))
			&& match self.def_rvalue(local) {
				// A reborrow of what an alias of a borrowed place points at borrows that place.
				Some(mir::Rvalue::Ref(_, _, p)) => match p.projection.as_slice() {
					[ProjectionElem::Deref] => self.points_at_place(p.local),
					[.., ProjectionElem::Deref] => false,
					_ => true,
				},
				// A copy of a `&mut` the closure captured by reference.
				Some(mir::Rvalue::CopyForDeref(p) | mir::Rvalue::Use(Operand::Copy(p) | Operand::Move(p), _)) => {
					self.pointee_capture(*p).is_some()
				}
				Some(_) => false,
				// An element `index_mut` borrows.
				None => true,
			}
	}

	/// The place `place` borrows, if it is a `&mut` the closure captured by reference.
	pub(super) fn pointee_capture(&self, place: Place<'tcx>) -> Option<Expr> {
		let Env::Closure(captures) = &self.env else {
			return None;
		};
		let ([ProjectionElem::Field(field, _)] | [ProjectionElem::Deref, ProjectionElem::Field(field, _)]) =
			place.projection.as_slice()
		else {
			return None;
		};
		let capture = captures
			.get(field.as_usize())
			.filter(|c| c.pointee && place.local.as_u32() == 1)?;
		Some(capture.value.clone())
	}

	/// Whether `local` is an `rt.Place` reference that aliases the place it borrows, so its
	/// target is that place rather than the `rt.Place`.
	pub(super) fn points_at_place(&self, local: Local) -> bool {
		self.b.t.place_pointee(self.mir.local_decls[local].ty).is_some() && self.alias_of_place(local)
	}

	/// The `rt.Place` of `target`, the JS path of the borrowed `place`, fixed where it is now.
	pub(super) fn make_place(&mut self, place: Place<'tcx>, target: Expr, span: Span, out: &mut Vec<Stmt>) -> Expr {
		let target = self.stable_target(place, target, span, out);
		let payload = self.payload_ty(place, span);
		self.place_of(target, payload)
	}

	/// The type of `place` if it is a variant's payload, whose JS path is its whole enum,
	/// as `Some`'s is the `Option`.
	pub(super) fn payload_ty(&mut self, place: Place<'tcx>, span: Span) -> Option<crate::ts::Type> {
		let downcast = place.iter_projections().any(|(base, e)| {
			matches!(e, ProjectionElem::Downcast(..)) && base.ty(&self.mir.local_decls, self.tcx).ty.is_enum()
		});
		downcast.then(|| self.b.t.ty(place.ty(&self.mir.local_decls, self.tcx).ty, span))
	}

	/// `target`, the JS path of the borrowed `place`, with the objects and index it goes
	/// through held where they are now, as the borrow fixes them, for an `rt.Place` that
	/// reads and writes it later.
	pub(super) fn stable_target(&mut self, place: Place<'tcx>, target: Expr, span: Span, out: &mut Vec<Stmt>) -> Expr {
		// A store into a `Result`'s value makes the `Result` again, which a path can't.
		for (base, elem) in place.iter_projections() {
			if let ProjectionElem::Downcast(..) = elem
				&& let Some(std @ (Std::Result | Std::ControlFlow)) =
					self.b.t.std_of(base.ty(&self.mir.local_decls, self.tcx).ty)
			{
				return self.err(
					span,
					format!("a mutable reference into a `{std:?}`'s payload, passed on or kept"),
				);
			}
		}
		self.stable_path(target, super::own(place), span, out)
	}

	/// `target`, a path, with the index it goes through held where it is now, and the
	/// object too unless it is a local's `own` value, which can't change while borrowed.
	pub(super) fn stable_path(&mut self, target: Expr, own: bool, span: Span, out: &mut Vec<Stmt>) -> Expr {
		self.materialize(&target, out);
		let mut base = |fx: &mut Self, e: Expr| match own {
			true => e,
			false => fx.held_now(e, out),
		};
		match target {
			Expr::Located(_, e) => self.stable_path(*e, own, span, out),
			Expr::Ident(_) | Expr::This => target,
			Expr::Member(object, key) => Expr::Member(Box::new(base(self, *object)), key),
			Expr::Index(object, i) => {
				let object = base(self, *object);
				Expr::Index(Box::new(object), Box::new(self.held_now(*i, out)))
			}
			_ => self.err(span, "a mutable reference passed on or kept, to a value with no place"),
		}
	}

	/// `e`, evaluated now into a constant unless reading it later reads the same.
	fn held_now(&mut self, e: Expr, out: &mut Vec<Stmt>) -> Expr {
		if e.is_constant() || matches!(e, Expr::This) || self.stable_name(&e) {
			return e;
		}
		let name = self.b.fresh("borrowed");
		let stmt = StmtKind::decl(false, name.clone(), None, Some(e)).at(None);
		self.emit(vec![stmt], None, false, out);
		Expr::ident(name)
	}

	/// The `rt.Place` that reads and writes `target`, a path that stays put, read as
	/// `payload` if it is a variant's.
	pub(super) fn place_of(&mut self, target: Expr, payload: Option<crate::ts::Type>) -> Expr {
		let value = self.b.fresh("value");
		let param = crate::ts::Param {
			name: value.clone(),
			ty: None,
		};
		let read = match payload {
			Some(ty) => Expr::As(Box::new(target.clone()), ty),
			None => target.clone(),
		};
		let get = Expr::Lambda(Vec::new(), Box::new(read));
		let set = Expr::Lambda(vec![param], Box::new(target.assign(Expr::ident(value))));
		Expr::rt("place", vec![get, set])
	}

	/// The `rt.Place` of `place`, a reborrow `&mut *r` of a reference `r` that is one, which is `r`'s.
	/// A shared reborrow `&*r` is what `r` points at instead.
	pub(super) fn reborrowed(
		&mut self,
		kind: mir::BorrowKind,
		place: Place<'tcx>,
		span: Span,
		out: &mut Vec<Stmt>,
	) -> Option<Expr> {
		if !matches!(kind, mir::BorrowKind::Mut { .. }) {
			return None;
		}
		let [rest @ .., ProjectionElem::Deref] = place.projection.as_slice() else {
			return None;
		};
		let base = Place {
			local: place.local,
			projection: self.tcx.mk_place_elems(rest),
		};
		let base_ty = base.ty(&self.mir.local_decls, self.tcx).ty;
		if self.b.t.place_pointee(base_ty).is_none() || base.as_local().is_some_and(|l| self.alias_of_place(l)) {
			return None;
		}
		Some(self.place_expr(base, span, out))
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
					Some(Std::Option)
						if let Expr::Ident(n) = scrut
							&& let Some(flag) = self.range_flags.get(n) =>
					{
						let flag = Expr::ident(flag);
						match index.index() {
							0 => flag.not(),
							_ => flag,
						}
					}
					Some(Std::Option) if matches!(scrut, Expr::Ident(n) if self.next_names.contains(n)) => {
						match index.index() {
							0 => scrut.clone().binary("===", Expr::Rt("DONE".into())),
							_ => scrut.clone().binary("!==", Expr::Rt("DONE".into())),
						}
					}
					Some(Std::Option) => match index.index() {
						0 => scrut.clone().binary("===", Expr::undefined()),
						_ => scrut.clone().binary("!==", Expr::undefined()),
					},
					Some(Std::Poll) => match index.index() {
						0 => scrut.clone().binary("!==", Expr::Rt("Pending".into())),
						_ => scrut.clone().binary("===", Expr::Rt("Pending".into())),
					},
					Some(Std::Result) => match index.index() {
						0 => scrut.clone().member("ok"),
						_ => scrut.clone().member("ok").not(),
					},
					None if self.b.t.own(adt.did()) => {
						let info = self.b.t.adt(*adt, span);
						let name = Expr::Str(adt.variant(index).name.to_string());
						match info.repr {
							AdtRepr::Tags => scrut.clone().binary("===", name),
							AdtRepr::Union => scrut.clone().member("tag").binary("===", name),
							AdtRepr::Struct { .. } => self.err(span, format!("a switch on `{ty}`")),
						}
					}
					Some(Std::ControlFlow) => super::call::control_flow_test(self, scrut, ty, index),
					Some(Std::Entry) => crate::lower::collections::entry_test(*adt, index, scrut.clone()),
					Some(Std::Cow) => self.err(span, crate::lower::library::cow_variant(ty)),
					Some(Std::Ordering) => scrut
						.clone()
						.binary("===", crate::lower::collections::ordering_variant(index)),
					Some(Std::Bound) => scrut
						.clone()
						.member("tag")
						.binary("===", Expr::Str(adt.variant(index).name.to_string())),
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
			&& !matches!(ty.kind(), ty::Adt(adt, _) if adt.is_struct() && self.b.t.own(adt.did()) && adt.all_fields().next().is_none())
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
				ty::Adt(adt, _) if adt.is_struct() && self.b.t.own(adt.did()) => {
					let info = self.b.t.adt(adt, span);
					self.b.t.construct(info.sym, &crate::ts::Ctor::Unit, &[], Vec::new())
				}
				ty::Array(..) => Expr::Array(Vec::new()),
				ty::Closure(def_id, args) => self.bare_closure(def_id, args, span),
				ty::Adt(adt, _) if Std::of(tcx, adt.did()) == Some(Std::RangeFull) => Expr::undefined(),
				_ => self.err(span, format!("a zero-sized constant of type `{ty}`")),
			},
			ConstValue::Scalar(Scalar::Int(int)) => match ty.kind() {
				ty::Bool => Expr::lit((int.to_bits_unchecked() != 0).to_string()),
				ty::Char => Expr::lit(int.to_u32().to_string()),
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
				Expr::rt(
					"byteSliceFrom",
					vec![Expr::ident("Uint8Array").member("of").call(items)],
				)
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
						Expr::rt(
							"byteSliceFrom",
							vec![Expr::ident("Uint8Array").member("of").call(items)],
						)
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

	/// The slice of the bytes an operand copies from behind a reference, directly or through
	/// a temporary still held back to fold into its one read, which is then no longer
	/// pending: read where they are, they need no copy.
	pub(super) fn bytes_source(&mut self, op: &Operand<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Option<Expr> {
		let (Operand::Copy(place) | Operand::Move(place)) = op else {
			return None;
		};
		if self.in_bytes(*place) {
			return Some(self.place_expr(*place, span, out));
		}
		let local = place.as_local()?;
		if self.defs[local] != 1 || self.uses[local] != 1 {
			return None;
		}
		let name = self.var(local)?.to_string();
		let i = self.pending.iter().position(|p| p.name == name)?;
		let Expr::Call(f, args) = &self.pending[i].value else {
			return None;
		};
		let (Expr::Rt(rt), [slice]) = (&**f, args.as_slice()) else {
			return None;
		};
		if rt != "byteSliceToArray" {
			return None;
		}
		let slice = slice.clone();
		let p = self.pending.remove(i);
		// A path, a subslice of one in bounds, or a local's parts, reads only bytes nothing
		// changed since.
		let path = |e: &Expr| {
			e.is_path()
				|| made_slice(e).is_some()
				|| matches!(e, Expr::Index(base, i) if base.is_path() && i.is_constant())
		};
		let stable = match &slice {
			Expr::Call(f, args) if matches!(&**f, Expr::Rt(name) if name == "byteSliceRange") => {
				matches!(args.as_slice(), [base, Expr::Lit(_), ..] if path(base))
			}
			e => path(e),
		};
		if stable {
			return Some(slice);
		}
		// What the copy read, held where the copy was.
		let name = self.b.fresh("slice");
		self.pending.push(Pending {
			name: name.clone(),
			value: slice,
			parts: None,
			copy: false,
			decl: true,
			..p
		});
		Some(Expr::ident(name))
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

/// Whether `ty` is `[u8]`, which is an `rt.ByteSlice`.
pub fn byte_slice(ty: Ty<'_>) -> bool {
	matches!(ty.kind(), ty::Slice(elem) if matches!(elem.kind(), ty::Uint(ty::UintTy::U8)))
}

/// Byte `i` of `slice`, an `rt.ByteSlice` that reading twice reads the same.
pub fn byte_at(slice: Expr, i: Expr) -> Expr {
	let at = super::scalar::add(slice_member(slice.clone(), "start"), i);
	slice_member(slice, "data").index(at)
}

/// The field `key` of `slice`, an `rt.ByteSlice`: its part, if it is made in place of parts
/// that do nothing, as a local held as its parts reads.
pub fn slice_member(slice: Expr, key: &str) -> Expr {
	if let Some([data, start, length]) = made_slice(&slice) {
		return match key {
			"data" => data.clone(),
			"start" => start.clone(),
			"length" => length.clone(),
			_ => unreachable!("a field of a byte slice"),
		};
	}
	slice.member(key)
}

/// The parts of `slice`, if it is an `rt.ByteSlice` made in place.
pub fn made_slice(slice: &Expr) -> Option<&[Expr; 3]> {
	match slice {
		Expr::New(f, args) if matches!(&**f, Expr::Rt(name) if name == "ByteSlice") => args.as_slice().try_into().ok(),
		_ => None,
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

/// Whether `ty` holds an `FnMut` closure by value: a copy of its arrow function shares
/// the variables it captured, which in Rust each copy has its own of.
fn changes_captures(ty: Ty<'_>) -> bool {
	match ty.kind() {
		ty::Closure(_, args) => {
			let closure = args.as_closure();
			closure.kind() == ty::ClosureKind::FnMut || closure.upvar_tys().iter().any(changes_captures)
		}
		ty::Tuple(tys) => tys.iter().any(changes_captures),
		ty::Array(t, _) => changes_captures(*t),
		ty::Adt(_, args) => args.types().any(changes_captures),
		_ => false,
	}
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
