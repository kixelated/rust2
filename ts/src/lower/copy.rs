//! Copies of `Copy` aggregates, by their type.
//!
//! JS shares an object where Rust copies its value, so a `Copy` value that is a JS object
//! Rust code can change in place is copied where Rust copies it. A function made for its
//! type copies what it holds by value and shares what it holds by reference, as Rust does.

use rustc_middle::ty::{self, EarlyBinder, Ty, TypeVisitableExt};
use rustc_span::Span;

use super::library::{Flow, Std};
use super::{AdtRepr, Translator, field_name, short_name};
use crate::program::{SymKind, Symbol};
use crate::ts::{self, Expr, Key, Param, Stmt, StmtKind};

impl<'a, 'tcx> Translator<'a, 'tcx> {
	/// Whether a `ty` is copied where Rust copies it: a `Copy` value that is, or holds by
	/// value, a JS object Rust code can change in place.
	pub fn needs_copy(&self, ty: Ty<'tcx>) -> bool {
		self.is_copy(ty) && self.mutable_aggregate(ty)
	}

	fn mutable_aggregate(&self, ty: Ty<'tcx>) -> bool {
		match ty.kind() {
			ty::Tuple(tys) => !tys.is_empty(),
			ty::Array(..) => true,
			ty::Adt(adt, args) => match Std::of(self.tcx, adt.did()) {
				// The runtime's methods hand out the payload itself.
				Some(Std::Option | Std::Result | Std::ControlFlow | Std::Poll) => {
					args.types().any(|t| self.mutable_aggregate(t))
				}
				Some(_) => false,
				None => adt.did().is_local() && adt.all_fields().next().is_some(),
			},
			_ => false,
		}
	}

	/// `value`, copied if a `ty` is.
	pub fn copied(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> Expr {
		match self.copy_fn(ty, span) {
			Some(copy) => copy.call(vec![value]),
			None => value,
		}
	}

	/// The function copying a `ty`, made the first time, or `None` if a `ty` is shared.
	pub fn copy_fn(&mut self, ty: Ty<'tcx>, span: Span) -> Option<Expr> {
		if !self.needs_copy(ty) {
			return None;
		}
		// An array's copy is the same at every length.
		let ty = match ty.kind() {
			ty::Array(elem, _) => Ty::new_array(self.tcx, *elem, 0),
			_ => ty,
		};
		if let Some(&sym) = self.copy_shims.get(&ty) {
			return Some(Expr::Sym(sym));
		}
		// Only an instance can say what a parameter holds; a census writes nothing.
		if self.polymorphic() && ty.has_param() {
			return None;
		}
		let sym = self.program.sym(Symbol {
			module: Vec::new(),
			base: "copy".into(),
			suffix: short_name(self.tcx, ty),
			owner: None,
			kind: SymKind::Function,
			order: 0,
			instance: ty::print::with_no_trimmed_paths!(ty.to_string()),
			name: String::new(),
			alias: None,
		});
		self.copy_shims.insert(ty, sym);
		let body = self.copy_body(ty, span);
		let ts_ty = self.ty(ty, span);
		self.program.items.push(ts::Item::Function(ts::Function {
			sym,
			params: vec![Param {
				name: "v".into(),
				ty: Some(ts_ty.clone()),
			}],
			ret: ts_ty,
			body,
			loc: None,
			doc: None,
		}));
		Some(Expr::Sym(sym))
	}

	/// The body of the function copying `v`, a `ty`.
	fn copy_body(&mut self, ty: Ty<'tcx>, span: Span) -> Vec<Stmt> {
		let v = || Expr::ident("v");
		let ret = |e: Expr| vec![StmtKind::Return(Some(e.typed())).at(None)];
		let tcx = self.tcx;
		match ty.kind() {
			ty::Tuple(tys) => {
				let parts = tys
					.iter()
					.enumerate()
					.map(|(i, t)| self.copied(v().index(Expr::lit(i.to_string())), t, span))
					.collect();
				ret(Expr::Array(parts))
			}
			ty::Array(elem, _) => ret(match self.copy_fn(*elem, span) {
				Some(copy) => v().member("map").call(vec![copy]),
				None => v().member("slice").call(Vec::new()),
			}),
			ty::Adt(adt, args) => match Std::of(tcx, adt.did()) {
				Some(Std::Option) => ret(self.copy_unless(Expr::undefined(), args.type_at(0), span)),
				Some(Std::Poll) => ret(self.copy_unless(Expr::Rt("Pending".into()), args.type_at(0), span)),
				Some(Std::Result) => ret(self.copy_result(args.type_at(0), args.type_at(1), span)),
				Some(Std::ControlFlow) => {
					let (residual, value) = (args.type_at(0), args.type_at(1));
					ret(match (Flow::of(tcx, args), residual.kind()) {
						(Flow::Result, ty::Adt(_, residual)) => self.copy_result(value, residual.type_at(1), span),
						(Flow::Option, _) => self.copy_unless(Expr::undefined(), value, span),
						_ => self.copy_result(value, residual, span),
					})
				}
				_ => {
					let info = self.adt(*adt, span);
					match info.repr {
						AdtRepr::Struct { ctor, fields } => {
							let variant = adt.non_enum_variant();
							let values = (variant.fields.iter().zip(&fields))
								.map(|(field, &id)| {
									let field_ty = self.field_ty(field.did, args);
									self.copied(v().field(id), field_ty, span)
								})
								.collect();
							ret(self.construct(info.sym, &ctor, &fields, values))
						}
						AdtRepr::Union => {
							let ts_ty = self.ty(ty, span);
							let mut body = Vec::new();
							for variant in adt.variants().iter().filter(|v| !v.fields.is_empty()) {
								let tag = variant.name.to_string();
								let mut props = vec![(Key::Name("tag".into()), Expr::Str(tag.clone()))];
								for (i, field) in variant.fields.iter().enumerate() {
									let name = field_name(field.name.as_str(), i);
									let field_ty = self.field_ty(field.did, args);
									props
										.push((Key::Name(name.clone()), self.copied(v().member(name), field_ty, span)));
								}
								body.push(
									StmtKind::If {
										cond: v().member("tag").binary("===", Expr::Str(tag)),
										then: ret(Expr::As(Box::new(Expr::Object(props)), ts_ty.clone())),
										els: Vec::new(),
									}
									.at(None),
								);
							}
							// A variant without fields has nothing to change in place.
							body.extend(ret(v()));
							body
						}
						AdtRepr::Tags => unreachable!("an enum without fields is shared"),
					}
				}
			},
			_ => unreachable!("`{ty}` is shared"),
		}
	}

	/// A copy of `v`, an `rt.Result` of `ok` or `err`.
	fn copy_result(&mut self, ok: Ty<'tcx>, err: Ty<'tcx>, span: Span) -> Expr {
		let v = || Expr::ident("v");
		Expr::Cond(
			Box::new(v().member("ok")),
			Box::new(Expr::rt("Ok", vec![self.copied(v().member("value"), ok, span)])),
			Box::new(Expr::rt("Err", vec![self.copied(v().member("error"), err, span)])),
		)
	}

	/// A copy of `v`, which is `none` or a `some`.
	fn copy_unless(&mut self, none: Expr, some: Ty<'tcx>, span: Span) -> Expr {
		let v = || Expr::ident("v");
		Expr::Cond(
			Box::new(v().binary("===", none)),
			Box::new(v()),
			Box::new(self.copied(v(), some, span)),
		)
	}

	/// The type of field `did` of an ADT with `args`.
	pub fn field_ty(&self, did: rustc_hir::def_id::DefId, args: ty::GenericArgsRef<'tcx>) -> Ty<'tcx> {
		let field = EarlyBinder::bind(self.tcx, self.tcx.type_of(did).instantiate_identity().skip_norm_wip());
		self.tcx
			.try_instantiate_and_normalize_erasing_regions(args, self.typing_env_pub(), field)
			.unwrap_or_else(|_| field.instantiate(self.tcx, args).skip_norm_wip())
	}
}
