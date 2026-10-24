//! Literals and constants. A named constant is evaluated by rustc and written as its
//! value at each use, as Rust inlines it.

use rustc_middle::mir::interpret::GlobalId;
use rustc_middle::thir::{ExprId, ExprKind};
use rustc_middle::ty::{self, Instance, Ty, ValTree};
use rustc_span::Span;

use super::AdtRepr;
use super::body::Body;
use super::library::Std;
use super::num::Int;
use crate::ts::{Expr, Key};

impl<'b, 'a, 'tcx> Body<'b, 'a, 'tcx> {
	/// A literal or constant expression.
	pub fn constant(&mut self, id: ExprId) -> Expr {
		let tcx = self.t.tcx;
		let e = &self.thir[id];
		let span = e.span;
		let ty = self.ty_of(id);
		match &e.kind {
			ExprKind::Literal { lit, neg } => match lit.node {
				rustc_ast::LitKind::Int(n, _) => {
					let Some(int) = Int::of(ty) else {
						return self.err(span, format!("an integer literal of type `{ty}`"));
					};
					let value = n.get() as i128;
					let value = if *neg { -value } else { value };
					match self.literal_text(span, *neg) {
						Some(text) => int.spelled(text, value),
						None => int.lit(value),
					}
				}
				rustc_ast::LitKind::Bool(b) => Expr::lit(b.to_string()),
				rustc_ast::LitKind::Str(s, _) => Expr::Str(s.to_string()),
				rustc_ast::LitKind::Byte(b) => Expr::lit(b.to_string()),
				rustc_ast::LitKind::ByteStr(ref bytes, _) => {
					let items = bytes.as_byte_str().iter().map(|b| Expr::lit(b.to_string())).collect();
					Expr::ident("Uint8Array").member("of").call(items)
				}
				_ => self.err(span, format!("unsupported literal of type `{ty}`")),
			},
			ExprKind::NonHirLiteral { lit, .. } => match (ty.kind(), Int::of(ty)) {
				(ty::Bool, _) => Expr::lit((lit.to_bits_unchecked() != 0).to_string()),
				(_, Some(int)) => {
					let size = rustc_abi::Size::from_bits(int.bits);
					let value = match int.signed {
						true => lit.to_int(size),
						false => lit.to_uint(size) as i128,
					};
					int.lit(value)
				}
				_ => self.err(span, format!("a literal of type `{ty}`")),
			},
			ExprKind::ZstLiteral { .. } => match *ty.kind() {
				ty::FnDef(def_id, args) => self.fn_value(def_id, args.skip_binder(), span),
				ty::Tuple(tys) if tys.is_empty() => Expr::undefined(),
				ty::Adt(adt, _) if adt.is_struct() && adt.did().is_local() => {
					let info = self.t.adt(adt, span);
					self.t.construct(info.sym, &crate::ts::Ctor::Unit, &[], Vec::new())
				}
				_ => self.err(span, format!("a value of type `{ty}`")),
			},
			ExprKind::NamedConst { def_id, args, .. } => {
				let args = self.subst(*args);
				let env = self.t.typing_env_pub();
				let instance = match Instance::try_resolve(tcx, env, *def_id, args) {
					Ok(Some(instance)) => instance,
					_ => return self.err(span, "a constant that does not resolve"),
				};
				let cid = GlobalId {
					instance,
					promoted: None,
				};
				match tcx.const_eval_global_id_for_typeck(env, cid, span) {
					Ok(Ok(valtree)) => self.value(ty, valtree, span),
					Ok(Err(_)) => self.err(span, format!("a constant of type `{ty}`, which has no value tree")),
					Err(_) => self.err(span, "a constant that failed to evaluate"),
				}
			}
			ExprKind::ConstParam { .. } | ExprKind::ConstBlock { .. } => {
				self.err(span, "const generics and const blocks in expressions are not supported")
			}
			_ => unreachable!(),
		}
	}

	/// A literal as the source spells it, when that is also valid JS.
	fn literal_text(&self, span: Span, neg: bool) -> Option<String> {
		if span.from_expansion() {
			return None;
		}
		let text = self.t.tcx.sess.source_map().span_to_snippet(span).ok()?;
		let text = text.trim_start_matches('-').trim();
		let (prefix, digits) = match text.get(..2) {
			Some("0x" | "0o" | "0b") => text.split_at(2),
			_ => ("", text),
		};
		let radix = match prefix {
			"0x" => 16,
			"0o" => 8,
			"0b" => 2,
			_ => 10,
		};
		// Drop a type suffix, which hex digits can't be confused with.
		let end = digits
			.find(|c: char| c != '_' && !c.is_digit(radix))
			.unwrap_or(digits.len());
		let digits = &digits[..end];
		// JS allows `_` only between two digits.
		let chars: Vec<char> = digits.chars().collect();
		let mut cleaned = String::new();
		for (i, &c) in chars.iter().enumerate() {
			if c == '_' {
				let between = i > 0 && chars[i - 1] != '_' && chars.get(i + 1).is_some_and(|n| *n != '_');
				if !between {
					continue;
				}
			}
			cleaned.push(c);
		}
		let cleaned = cleaned.trim_matches('_');
		if cleaned.is_empty() || cleaned.contains("__") {
			return None;
		}
		let sign = if neg { "-" } else { "" };
		Some(format!("{sign}{prefix}{cleaned}"))
	}

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
					AdtRepr::Class { ctor, fields } => self.t.construct(info.sym, &ctor, &fields, values),
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
