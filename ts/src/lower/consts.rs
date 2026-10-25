//! Constants: rustc evaluates each, and its value is written at each use, as Rust
//! inlines it.

use rustc_middle::ty::{self, Ty, ValTree};
use rustc_span::Span;

use super::AdtRepr;
use super::body::Body;
use super::library::Std;
use super::num::Int;
use crate::ts::{Expr, Key};

impl<'b, 'a, 'tcx> Body<'b, 'a, 'tcx> {
	/// A constant's value tree as a JS value of type `ty`.
	pub fn value(&mut self, ty: Ty<'tcx>, valtree: ValTree<'tcx>, span: Span) -> Expr {
		let tcx = self.t.tcx;
		let leaf = |v: ValTree<'tcx>| v.try_to_leaf();
		let children = |v: ValTree<'tcx>| -> Vec<(Ty<'tcx>, ValTree<'tcx>)> {
			match &**v {
				ty::ValTreeKind::Branch(consts) => consts
					.iter()
					.map(|c| match c.kind() {
						ty::ConstKind::Value(value) => (value.ty, value.valtree),
						_ => unreachable!("a value tree holds values"),
					})
					.collect(),
				ty::ValTreeKind::Leaf(_) => Vec::new(),
			}
		};
		match ty.kind() {
			ty::Bool => Expr::lit((leaf(valtree).is_some_and(|l| l.to_bits_unchecked() != 0)).to_string()),
			ty::Int(_) | ty::Uint(_) => {
				let int = Int::of(ty).unwrap();
				let Some(scalar) = leaf(valtree) else {
					return self.err(span, "an integer constant without a value");
				};
				let size = rustc_abi::Size::from_bits(int.bits);
				let value = match int.signed {
					true => scalar.to_int(size),
					false => scalar.to_uint(size) as i128,
				};
				int.lit(value)
			}
			ty::Ref(_, inner, _) => match inner.kind() {
				ty::Str => {
					let bytes: Vec<u8> = children(valtree)
						.into_iter()
						.map(|(_, v)| v.try_to_leaf().map_or(0, |l| l.to_u8()))
						.collect();
					match String::from_utf8(bytes) {
						Ok(s) => Expr::Str(s),
						Err(_) => self.err(span, "a string constant that is not UTF-8"),
					}
				}
				ty::Slice(elem) => {
					let items: Vec<Expr> = children(valtree)
						.into_iter()
						.map(|(t, v)| self.value(t, v, span))
						.collect();
					match elem.kind() {
						ty::Uint(ty::UintTy::U8) => Expr::ident("Uint8Array").member("of").call(items),
						_ => Expr::Array(items),
					}
				}
				_ => self.value(*inner, valtree, span),
			},
			ty::Array(elem, _) => {
				let items: Vec<Expr> = children(valtree)
					.into_iter()
					.map(|(t, v)| self.value(t, v, span))
					.collect();
				match elem.kind() {
					ty::Uint(ty::UintTy::U8) => Expr::ident("Uint8Array").member("of").call(items),
					_ => Expr::Array(items),
				}
			}
			ty::Tuple(tys) if tys.is_empty() => Expr::undefined(),
			ty::Tuple(_) => Expr::Array(
				children(valtree)
					.into_iter()
					.map(|(t, v)| self.value(t, v, span))
					.collect(),
			),
			ty::Adt(adt, _) => {
				let fields = children(valtree);
				let (variant, fields) = match adt.is_enum() {
					true => {
						let Some(((_, discr), rest)) = fields.split_first() else {
							return self.err(span, "an enum constant without a variant");
						};
						let index = discr.try_to_leaf().map_or(0, |l| l.to_u32());
						(rustc_abi::VariantIdx::from_u32(index), rest.to_vec())
					}
					false => (rustc_abi::FIRST_VARIANT, fields),
				};
				let mut values: Vec<Expr> = fields.into_iter().map(|(t, v)| self.value(t, v, span)).collect();
				if let Some(std) = Std::of(tcx, adt.did()) {
					return match (std, variant.index()) {
						(Std::Option, 0) => Expr::undefined(),
						(Std::Option, _) => values.pop().unwrap(),
						(Std::Result, 0) => Expr::rt("Ok", values),
						(Std::Result, _) => Expr::rt("Err", values),
						_ => self.err(span, format!("a constant of type `{ty}`")),
					};
				}
				if !adt.did().is_local() {
					return self.err(span, format!("a constant of type `{ty}` from another crate"));
				}
				let info = self.t.adt(*adt, span);
				let def = adt.variant(variant);
				match info.repr {
					AdtRepr::Struct { ctor, fields } => self.t.construct(info.sym, &ctor, &fields, values),
					AdtRepr::Tags => {
						let ts_ty = self.t.ty(ty, span);
						Expr::As(Box::new(Expr::Str(def.name.to_string())), ts_ty)
					}
					AdtRepr::Union => {
						let mut props = vec![(Key::Name("tag".into()), Expr::Str(def.name.to_string()))];
						for (i, (field, value)) in def.fields.iter().zip(values).enumerate() {
							props.push((Key::Name(super::field_name(field.name.as_str(), i)), value));
						}
						let ts_ty = self.t.ty(ty, span);
						Expr::As(Box::new(Expr::Object(props)), ts_ty)
					}
				}
			}
			_ => self.err(span, format!("a constant of type `{ty}`")),
		}
	}
}
