//! Drops: which values JS can see drop, and where each one drops.
//!
//! A type drops if dropping it does something JS can see: runs a `Drop` impl, counts an
//! `Arc` down, or drops a waker. Such a value has `[Symbol.dispose]`, which a class gets
//! as drop glue: its `Drop` impl, then each field in order. A variable drops at the end
//! of its scope through a `finally`, so an early exit, a panic, or a cancelled future
//! drops it too. A move leaves `undefined` in the place it left, which drops as nothing,
//! so a variable moved on only some paths still drops on the others.

use rustc_middle::ty::{self, AdtDef, Instance, Ty, TypeVisitableExt};
use rustc_span::Span;

use super::body::Body;
use super::library::Std;
use super::{Adt, AdtRepr, Translator};
use crate::program::{SymKind, Symbol};
use crate::ts::{self, Expr, Stmt, StmtKind, Target, Type};

/// A variable its scope drops when it ends.
pub struct Owned<'tcx> {
	pub name: String,
	pub ty: Ty<'tcx>,
}

impl<'a, 'tcx> Translator<'a, 'tcx> {
	/// Whether dropping a `ty` does something JS can see.
	pub fn drops(&mut self, ty: Ty<'tcx>) -> bool {
		if let Some(&drops) = self.drops.get(&ty) {
			return drops;
		}
		// A type that holds itself drops through another field if at all.
		self.drops.insert(ty, false);
		let tcx = self.tcx;
		let drops = match ty.kind() {
			ty::Adt(adt, args) => match Std::of(tcx, adt.did()) {
				Some(Std::Arc | Std::Waker) => true,
				Some(Std::Option | Std::Result | Std::Box | Std::Vec | Std::Mutex | Std::Poll) => {
					args.types().any(|t| self.drops(t))
				}
				Some(_) => false,
				None if adt.did().is_local() => adt.has_dtor(tcx) || self.fields_drop(*adt, args, true),
				None => false,
			},
			ty::Tuple(tys) => tys.iter().any(|t| self.drops(t)),
			ty::Array(elem, _) => self.drops(*elem),
			ty::Closure(_, args) => args.as_closure().upvar_tys().iter().any(|t| self.drops(t)),
			ty::Coroutine(..) | ty::Param(_) => true,
			_ => false,
		};
		self.drops.insert(ty, drops);
		drops
	}

	/// Whether any field of `adt` with `args` drops, counting one of a type parameter only
	/// if `params`.
	fn fields_drop(&mut self, adt: AdtDef<'tcx>, args: ty::GenericArgsRef<'tcx>, params: bool) -> bool {
		let tcx = self.tcx;
		let env = self.typing_env_pub();
		adt.all_fields().any(|field| {
			// A projection from a type parameter normalizes only where its bounds are known.
			let field = ty::EarlyBinder::bind(tcx, tcx.type_of(field.did).instantiate_identity().skip_norm_wip());
			let ty = tcx
				.try_instantiate_and_normalize_erasing_regions(args, env, field)
				.unwrap_or_else(|_| field.instantiate(tcx, args).skip_norm_wip());
			(params || !ty.has_param()) && self.drops(ty)
		})
	}

	/// Whether a `ty` that drops does so through its own `[Symbol.dispose]`, so whatever
	/// holds one can drop it without knowing its type.
	fn disposable(&mut self, ty: Ty<'tcx>) -> bool {
		if !self.drops(ty) {
			return true;
		}
		match ty.kind() {
			ty::Adt(adt, args) => match Std::of(self.tcx, adt.did()) {
				Some(Std::Arc | Std::Waker | Std::Mutex) => true,
				Some(Std::Option | Std::Box) => self.disposable(args.type_at(0)),
				Some(_) => false,
				None => adt.is_struct(),
			},
			ty::Coroutine(..) | ty::Param(_) => true,
			_ => false,
		}
	}

	/// What drops the `ty` in `value`, or `None` if JS would see nothing drop.
	pub fn drop_value(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> Option<Expr> {
		if !self.drops(ty) {
			return None;
		}
		let supported = match ty.kind() {
			ty::Adt(adt, args) => match Std::of(self.tcx, adt.did()) {
				Some(Std::Vec) if self.disposable(args.type_at(0)) => {
					return Some(Expr::rt("dropAll", vec![value]));
				}
				Some(Std::Arc | Std::Mutex | Std::Option | Std::Box) => self.disposable(args.type_at(0)),
				Some(Std::Waker) => true,
				Some(_) => false,
				None if adt.is_struct() => {
					let info = self.adt(*adt, span);
					self.glue(*adt, &info, span);
					args.types().all(|t| self.disposable(t))
				}
				None => false,
			},
			ty::Array(elem, _) if self.disposable(*elem) => return Some(Expr::rt("dropAll", vec![value])),
			ty::Coroutine(..) | ty::Param(_) => true,
			_ => false,
		};
		if !supported {
			self.err(span, format!("dropping `{ty}`, which is not supported yet"));
			return None;
		}
		Some(Expr::rt("drop", vec![value]))
	}

	/// Give a class whose own fields or `Drop` impl drop its drop glue up front, so JS
	/// that holds one can drop it. One that drops only through a type parameter gets
	/// it once an instance that drops does.
	pub fn drop_glue(&mut self, adt: AdtDef<'tcx>, info: &Adt, span: Span) {
		let identity = ty::GenericArgs::identity_for_item(self.tcx, adt.did());
		if adt.has_dtor(self.tcx) || self.fields_drop(adt, identity, false) {
			self.glue(adt, info, span);
		}
	}

	/// The `[Symbol.dispose]` of a class: its `Drop` impl, then each field that drops.
	fn glue(&mut self, adt: AdtDef<'tcx>, info: &Adt, span: Span) {
		let tcx = self.tcx;
		if !self.glued.insert(adt.did()) {
			return;
		}
		// An enum that drops is refused where it drops: its glue would switch on its variant.
		let AdtRepr::Class { fields, .. } = &info.repr else {
			if adt.has_dtor(tcx) {
				self.err(
					span,
					format!(
						"`{}` is an enum that implements `Drop`, which is not supported yet",
						tcx.def_path_str(adt.did())
					),
				);
			}
			return;
		};
		let mut body: Vec<Stmt> = Vec::new();
		if let Some(dtor) = adt.destructor(tcx) {
			match tcx.generics_of(adt.did()).requires_monomorphization(tcx) {
				true => self.err(
					span,
					format!(
						"`{}` is generic and implements `Drop`, which is not supported yet",
						tcx.def_path_str(adt.did())
					),
				),
				false => {
					let sym = self.instance(Instance::mono(tcx, dtor.did));
					body.push(StmtKind::Expr(Expr::Method(Box::new(Expr::This), sym, Vec::new())).at(None));
				}
			}
		}
		for (field, &id) in adt.non_enum_variant().fields.iter().zip(fields) {
			let ty = tcx.type_of(field.did).instantiate_identity().skip_norm_wip();
			if let Some(dropped) = self.drop_value(Expr::This.field(id), ty, span) {
				body.push(StmtKind::Expr(dropped).at(None));
			}
		}
		let sym = self.program.sym(Symbol {
			module: self.module_of(adt.did()),
			base: "[Symbol.dispose]".into(),
			suffix: String::new(),
			owner: Some(info.sym),
			kind: SymKind::Method { instance: true },
			order: (0, 0),
			name: String::new(),
		});
		self.program.methods.entry(info.sym).or_default().push(ts::Function {
			sym,
			method: true,
			params: Vec::new(),
			ret: Type::Builtin("void"),
			body,
			loc: None,
			doc: None,
			generator: false,
		});
	}
}

impl<'b, 'a, 'tcx> Body<'b, 'a, 'tcx> {
	/// Start a scope: the variables bound until it closes drop when it does.
	pub fn open_scope(&mut self) {
		self.scopes.push(Vec::new());
	}

	/// Drop `name`, a `ty`, when the innermost scope closes, if JS would see it drop.
	pub fn own(&mut self, name: &str, ty: Ty<'tcx>) {
		if !self.t.drops(ty) {
			return;
		}
		if let Some(scope) = self.scopes.last_mut() {
			scope.push(Owned {
				name: name.to_string(),
				ty,
			});
		}
	}

	/// Stop dropping `name` here: it moved somewhere that drops it itself.
	pub fn disown(&mut self, name: &str) {
		for scope in &mut self.scopes {
			scope.retain(|owned| owned.name != name);
		}
	}

	/// Whether the innermost scope drops anything yet.
	pub fn scope_owns(&self) -> bool {
		self.scopes.last().is_some_and(|scope| !scope.is_empty())
	}

	/// Close the innermost scope, whose statements are `out[start..]`: each variable it
	/// drops is dropped in a `finally` around the statements after its declaration, the
	/// last declared innermost, or around them all for a parameter.
	pub fn close_scope(&mut self, out: &mut Vec<Stmt>, start: usize, span: Span) {
		let owned = self.scopes.pop().unwrap_or_default();
		for owned in owned.into_iter().rev() {
			let Some(dropped) = self.t.drop_value(Expr::ident(&owned.name), owned.ty, span) else {
				continue;
			};
			let moved = self.moved.contains(&owned.name);
			let mut at = start;
			for (i, stmt) in out.iter_mut().enumerate().skip(start) {
				if let StmtKind::Decl {
					mutable, target, init, ..
				} = &mut stmt.kind
					&& declares(target, &owned.name)
				{
					*mutable |= moved;
					// The `finally` may run before anything assigns it.
					init.get_or_insert_with(|| Expr::lit("undefined!"));
					at = i + 1;
					break;
				}
			}
			// What the declaration moved from is cleared before anything can throw.
			while out.get(at).is_some_and(clears) {
				at += 1;
			}
			let body = out.split_off(at);
			let dropped = StmtKind::Expr(dropped).at(None);
			match body.is_empty() {
				true => out.push(dropped),
				false => out.push(
					StmtKind::Try {
						body,
						finally: vec![dropped],
					}
					.at(None),
				),
			}
		}
	}

	/// The value of the place `place`, a `ty` that drops, moved out: the place is left
	/// `undefined`, so whatever drops it later drops nothing.
	pub fn move_out(&mut self, place: Expr, span: Span, out: &mut Vec<Stmt>) -> Expr {
		let base = match &place {
			Expr::Ident(name) => name.clone(),
			_ => "moved".into(),
		};
		match &place {
			Expr::Ident(name) => {
				self.moved.insert(name.clone());
			}
			Expr::Member(..) if place.is_path() => {}
			_ => return self.err(span, "a move out of a place that is no variable or field"),
		}
		let name = self.fresh(&base);
		out.push(StmtKind::decl(false, name.clone(), None, Some(place.clone())).at(None));
		self.clear(place, out);
		Expr::ident(name)
	}

	/// Leave `undefined` in `place`, whose value moved.
	pub fn clear(&mut self, place: Expr, out: &mut Vec<Stmt>) {
		if let Expr::Ident(name) = &place {
			self.moved.insert(name.clone());
		}
		out.push(StmtKind::Expr(place.assign(Expr::lit("undefined!"))).at(None));
	}
}

/// Whether `stmt` leaves a moved-out place `undefined`.
fn clears(stmt: &Stmt) -> bool {
	matches!(&stmt.kind, StmtKind::Expr(Expr::Assign(_, value)) if matches!(&**value, Expr::Lit(text) if text == "undefined!"))
}

/// Whether `target` declares `name`.
fn declares(target: &Target, name: &str) -> bool {
	match target {
		Target::Name(n) => n == name,
		Target::Array(items) => items.iter().flatten().any(|t| declares(t, name)),
	}
}
