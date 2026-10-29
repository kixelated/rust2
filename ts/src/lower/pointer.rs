//! Raw pointers, `NonNull`, and `MaybeUninit`.
//!
//! A raw pointer is what a `&mut` to its pointee is: the object it points at, where JS
//! keeps the pointee where Rust does, as a tuple, an array, or the crate's own struct; or
//! else an `rt.Place` of the value, which JS copies. Null is `rt.NULL`, and a dangling
//! pointer `rt.DANGLING`. A pointer to any other object, such as a `Vec` or an `Arc`, is
//! refused: its JS object is the heap behind it, not the place Rust keeps it.
//!
//! A whole value written through a pointer to an object is copied into that object, so
//! every pointer to it sees the write, and one read out of it is a copy of it, so a later
//! write leaves it be. Both go one object deep, as `*r = v` does through a `&mut`.
//!
//! A `MaybeUninit<T>` is a `T`: `undefined` where JS copies a `T` and it holds none yet,
//! or an object with none of its parts yet where a pointer to it is the object.

use rustc_middle::ty::{self, Ty, TypeVisitableExt};
use rustc_span::Span;

use super::body::Body;
use super::library::{Call, Std, targ};
use super::{AdtRepr, ObjectKind, Translator};
use crate::ts::{Expr, Stmt, StmtKind, Type};

/// What a raw pointer to a type is in JS.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pointee {
	/// An `rt.Place` of the value, which JS copies.
	Place,
	/// The object it points at.
	Object,
}

impl<'a, 'tcx> Translator<'a, 'tcx> {
	/// What a raw pointer to `ty` is, or why there is none.
	pub fn pointee(&self, ty: Ty<'tcx>) -> Result<Pointee, String> {
		// Only an instance decides what a parameter is.
		if self.polymorphic() && ty.has_param() {
			return Ok(Pointee::Object);
		}
		let ty = self.uninit_inner(ty);
		let Some(kind) = self.object_kind(ty) else {
			return Ok(Pointee::Place);
		};
		let object = match (kind, ty.kind()) {
			(ObjectKind::Array | ObjectKind::Bytes, ty::Array(..) | ty::Tuple(_)) => true,
			(ObjectKind::Class, ty::Adt(adt, _)) => {
				(Std::of(self.tcx, adt.did()).is_none() && self.bound_drops(adt.did()).is_none())
					|| Std::of(self.tcx, adt.did()) == Some(Std::Context)
			}
			_ => false,
		};
		match object {
			true => Ok(Pointee::Object),
			false => Err(format!(
				"a raw pointer to `{ty}`, whose JS value is not where Rust keeps it"
			)),
		}
	}

	/// What a `MaybeUninit` holds, through any number of them, or `ty` itself.
	pub fn uninit_inner(&self, ty: Ty<'tcx>) -> Ty<'tcx> {
		let mut ty = ty;
		while let ty::Adt(adt, args) = ty.kind()
			&& Std::of(self.tcx, adt.did()) == Some(Std::MaybeUninit)
		{
			ty = args.type_at(0);
		}
		ty
	}

	/// The TypeScript type of a raw pointer or `NonNull` to `pointee`.
	pub fn pointer_ty(&mut self, pointee: Ty<'tcx>, span: Span) -> Type {
		match self.pointee(pointee) {
			Ok(Pointee::Place) => Type::Rt("Place", vec![self.ty(pointee, span)]),
			Ok(Pointee::Object) => self.ty(pointee, span),
			Err(msg) => {
				self.unrepresented(span, msg);
				Type::Builtin("unknown")
			}
		}
	}

	/// Whether `ty` is a raw pointer that is an `rt.Place`.
	pub fn raw_place(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
		match ty.kind() {
			ty::RawPtr(inner, _) if self.pointee(*inner) == Ok(Pointee::Place) => Some(*inner),
			_ => None,
		}
	}

	/// A `MaybeUninit<ty>` that holds nothing yet.
	fn uninit(&mut self, ty: Ty<'tcx>, span: Span) -> Expr {
		let ts_ty = self.ty(ty, span);
		let empty = match (self.pointee(ty), ty.kind()) {
			(Ok(Pointee::Object), ty::Tuple(tys)) => Expr::Array(tys.iter().map(|_| Expr::undefined()).collect()),
			(Ok(Pointee::Object), ty::Array(elem, len)) => {
				let len = len.try_to_target_usize(self.tcx).unwrap_or(0);
				match elem.kind() {
					ty::Uint(ty::UintTy::U8) => {
						Expr::New(Box::new(Expr::ident("Uint8Array")), vec![Expr::lit(len.to_string())])
					}
					_ => Expr::New(Box::new(Expr::ident("Array")), vec![Expr::lit(len.to_string())]),
				}
			}
			(Ok(Pointee::Object), ty::Adt(adt, _)) => {
				let info = self.adt(*adt, span);
				match info.repr {
					AdtRepr::Struct { ctor, fields } => {
						let values = fields.iter().map(|_| Expr::undefined()).collect();
						self.construct(info.sym, &ctor, &fields, values)
					}
					AdtRepr::Slots(_) => Expr::Object(Vec::new()),
					_ => unreachable!("only a struct or union is an object a pointer points at"),
				}
			}
			_ => Expr::undefined(),
		};
		Expr::As(Box::new(Expr::As(Box::new(empty), Type::Builtin("unknown"))), ts_ty)
	}
}

/// A call of std's raw pointer, `NonNull`, `MaybeUninit`, or `Box` functions that make or
/// take a raw pointer: its mapping, `None` if it has none, or its arguments back if it is
/// no such call.
pub fn call<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	call: &Call<'tcx>,
	owner: &str,
	name: &str,
	args: Vec<Expr>,
	out: &mut Vec<Stmt>,
) -> Result<Option<Expr>, Vec<Expr>> {
	let span = call.span;
	let gargs = call.args;
	let pointee = || targ(gargs, 0);
	let kind = |b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>| match b.t.pointee(ty) {
		Ok(kind) => Some(kind),
		Err(msg) => {
			b.err(span, msg);
			None
		}
	};
	let mut args = args.into_iter();
	let mut arg = || args.next().expect("an argument");
	let e = match (owner, name) {
		("ptr", "null" | "null_mut") => Expr::Rt("NULL".into()),
		("NonNull", "dangling") | ("ptr", "dangling" | "dangling_mut") => Expr::Rt("DANGLING".into()),
		("ptr", "is_null") => arg().binary("===", Expr::Rt("NULL".into())),
		// The same pointer, as another type of the same pointee.
		("ptr", "cast_const" | "cast_mut")
		| ("NonNull", "as_ptr" | "new_unchecked" | "cast_const" | "cast_mut")
		| ("MaybeUninit", "new" | "assume_init" | "assume_init_mut")
		| ("NonNull", "as_mut")
		| ("Clone", "clone")
			if owner != "Clone" || pointer_self(b, call).is_some() =>
		{
			arg()
		}
		// A shared reference to a value JS copies is the value, which no write changes.
		("From", "from") if let Some(ty) = pointer_self(b, call) => {
			let from = targ(gargs, 1);
			match (kind(b, ty), from.kind()) {
				(Some(Pointee::Place), ty::Ref(_, _, ty::Mutability::Not)) => {
					Expr::Object(vec![(crate::ts::Key::Name("v".into()), arg())])
				}
				_ => arg(),
			}
		}
		("NonNull", "new") => {
			let p = b.spill(arg(), "pointer", out);
			Expr::Cond(
				Box::new(p.clone().binary("===", Expr::Rt("NULL".into()))),
				Box::new(Expr::undefined()),
				Box::new(p),
			)
		}
		// `Option<&T>`: a shared reference to a value JS copies is the value.
		("ptr", "as_ref" | "as_mut") => {
			let p = b.spill(arg(), "pointer", out);
			let shared = name == "as_ref" && kind(b, pointee()) == Some(super::pointer::Pointee::Place);
			let some = match shared {
				true => p.clone().member("v"),
				false => p.clone(),
			};
			Expr::Cond(
				Box::new(p.binary("===", Expr::Rt("NULL".into()))),
				Box::new(Expr::undefined()),
				Box::new(some),
			)
		}
		("NonNull", "as_ref") | ("MaybeUninit", "assume_init_ref") => match kind(b, pointee()) {
			Some(Pointee::Place) if owner == "NonNull" => arg().member("v"),
			_ => arg(),
		},
		// A pointer to what a box held, which no box holds any longer.
		("Box", "into_raw" | "leak") => match kind(b, pointee()) {
			Some(Pointee::Place) => Expr::Object(vec![(crate::ts::Key::Name("v".into()), arg())]),
			_ => arg(),
		},
		("Box", "from_raw") => match kind(b, pointee()) {
			Some(Pointee::Place) => arg().member("v"),
			_ => arg(),
		},
		("ptr", "read" | "read_unaligned" | "read_volatile") => {
			let p = arg();
			read(b, p, pointee(), span)
		}
		("ptr", "write" | "write_unaligned" | "write_volatile") => {
			let (p, value) = (arg(), arg());
			write(b, p, value, pointee(), span)
		}
		("ptr", "replace") => {
			let p = b.spill(arg(), "pointer", out);
			let value = arg();
			let old = read(b, p.clone(), pointee(), span);
			let old = b.spill(old, "old", out);
			let write = write(b, p, value, pointee(), span);
			out.push(StmtKind::Expr(write).at(None));
			old
		}
		("ptr", "swap") => {
			let (x, y) = (b.spill(arg(), "x", out), b.spill(arg(), "y", out));
			let old = read(b, x.clone(), pointee(), span);
			let old = b.spill(old, "old", out);
			let from_y = read(b, y.clone(), pointee(), span);
			let into_x = write(b, x, from_y, pointee(), span);
			out.push(StmtKind::Expr(into_x).at(None));
			write(b, y, old, pointee(), span)
		}
		("ptr", "drop_in_place") => {
			let p = arg();
			let value = match kind(b, pointee()) {
				Some(Pointee::Place) => p.member("v"),
				_ => p,
			};
			b.t.drop_value(value, pointee(), span).unwrap_or_else(Expr::undefined)
		}
		("ptr", "eq") => {
			let (x, y) = (arg(), arg());
			compare(b, x, y, pointee(), span)
		}
		("PartialEq", "eq" | "ne") if let Some(ty) = pointer_self(b, call) => {
			let (x, y) = (arg(), arg());
			let same = compare(b, x, y, ty, span);
			match name {
				"eq" => same,
				_ => same.not(),
			}
		}
		("MaybeUninit", "uninit") => b.t.uninit(pointee(), span),
		// A shared reference to a value JS copies is the value, which no write changes.
		("MaybeUninit", "as_ptr") if kind(b, pointee()) == Some(Pointee::Place) => {
			Expr::Object(vec![(crate::ts::Key::Name("v".into()), arg())])
		}
		("MaybeUninit", "as_ptr" | "as_mut_ptr") => arg(),
		("MaybeUninit", "write") => {
			let p = b.spill(arg(), "uninit", out);
			let value = arg();
			let write = write(b, p.clone(), value, pointee(), span);
			out.push(StmtKind::Expr(write).at(None));
			p
		}
		// A shared reference to a value JS copies is the value.
		("MaybeUninit", "assume_init_read") => match kind(b, pointee()) {
			Some(Pointee::Place) => {
				let value = arg();
				b.t.copied(value, pointee(), span)
			}
			_ => Expr::rt("ptrRead", vec![arg()]),
		},
		("MaybeUninit", "assume_init_drop") => {
			let p = arg();
			let value = match kind(b, pointee()) {
				Some(Pointee::Place) => p.member("v"),
				_ => p,
			};
			b.t.drop_value(value, pointee(), span).unwrap_or_else(Expr::undefined)
		}
		("mem", "forget") => Expr::undefined(),
		_ => return Err(args.collect()),
	};
	Ok(Some(e))
}

/// What a `PartialEq` or `Clone` call's `Self` points at, if it is a raw pointer or a
/// `NonNull`.
fn pointer_self<'tcx>(b: &Body<'_, '_, 'tcx>, call: &Call<'tcx>) -> Option<Ty<'tcx>> {
	let this = call.args.types().next()?;
	match this.kind() {
		ty::RawPtr(pointee, _) => Some(*pointee),
		ty::Adt(adt, args) if Std::of(b.t.tcx, adt.did()) == Some(Std::NonNull) => Some(args.type_at(0)),
		_ => None,
	}
}

/// The value `p`, a pointer to `ty`, points at, as a value of its own.
fn read<'tcx>(b: &mut Body<'_, '_, 'tcx>, p: Expr, ty: Ty<'tcx>, span: Span) -> Expr {
	match b.t.pointee(ty) {
		Ok(Pointee::Place) => p.member("v"),
		Ok(Pointee::Object) => Expr::rt("ptrRead", vec![p]),
		Err(msg) => b.err(span, msg),
	}
}

/// The write of `value`, a `ty`, where `p` points.
fn write<'tcx>(b: &mut Body<'_, '_, 'tcx>, p: Expr, value: Expr, ty: Ty<'tcx>, span: Span) -> Expr {
	match b.t.pointee(ty) {
		Ok(Pointee::Place) => p.member("v").assign(value),
		Ok(Pointee::Object) => Expr::rt("ptrWrite", vec![p, value]),
		Err(msg) => b.err(span, msg),
	}
}

/// Whether pointers `x` and `y` to `ty` point at the same place: the same object, or
/// null both. Two `rt.Place`s of one place are two objects, so they are refused.
pub fn compare<'tcx>(b: &mut Body<'_, '_, 'tcx>, x: Expr, y: Expr, ty: Ty<'tcx>, span: Span) -> Expr {
	let null = |e: &Expr| matches!(e, Expr::Rt(n) if n == "NULL");
	match b.t.pointee(ty) {
		Ok(Pointee::Object) => x.binary("===", y),
		Ok(Pointee::Place) if null(&x) || null(&y) => x.binary("===", y),
		Ok(Pointee::Place) => b.err(
			span,
			format!("comparing pointers to `{ty}`, whose `rt.Place`s of one place are two objects"),
		),
		Err(msg) => b.err(span, msg),
	}
}
