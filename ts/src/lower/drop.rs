//! Drops: which values JS can see drop, and where each one drops.
//!
//! A type drops if dropping it does something JS can see: runs a `Drop` impl, counts an
//! `Arc` down, or drops a waker. Such a value has `[Symbol.dispose]`, which a class gets
//! as drop glue: its `Drop` impl, then each field in order. Where each value drops is
//! MIR's: drop elaboration put a terminator at each drop, with a flag wherever a value is
//! moved on some paths only.

use rustc_middle::ty::{self, AdtDef, Instance, Ty, TypeVisitableExt};
use rustc_span::Span;

use super::library::Std;
use super::{Adt, AdtRepr, Translator};
use crate::program::{SymKind, Symbol};
use crate::ts::{self, Expr, Stmt, StmtKind, Type};

impl<'a, 'tcx> Translator<'a, 'tcx> {
	/// Whether dropping a `ty` does something JS can see.
	pub fn drops(&mut self, ty: Ty<'tcx>) -> bool {
		if let Some(&drops) = self.drops.get(&ty) {
			return drops;
		}
		// A type that holds itself drops through another field if at all.
		if !self.drops_open.insert(ty) {
			self.drops_cycle = true;
			return false;
		}
		let outer_cycle = std::mem::replace(&mut self.drops_cycle, false);
		let tcx = self.tcx;
		let drops = match ty.kind() {
			ty::Adt(adt, args) => match Std::of(tcx, adt.did()) {
				Some(Std::Arc | Std::Waker) => true,
				Some(
					Std::Option
					| Std::Result
					| Std::ControlFlow
					| Std::Box
					| Std::Vec
					| Std::Mutex
					| Std::Poll
					| Std::Pin
					| Std::HashMap
					| Std::HashSet
					| Std::BTreeMap
					| Std::BTreeSet
					| Std::VecDeque,
				) => args.types().any(|t| self.drops(t)),
				// An entry borrows its map, and holds only the key it was given.
				Some(Std::Entry) => args.types().next().is_some_and(|k| self.drops(k)),
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
		self.drops_open.remove(&ty);
		// A `false` that read a type still being decided holds only once the first of
		// them is decided: if it drops, so does every type it reached that reaches it.
		let cycle = std::mem::replace(&mut self.drops_cycle, outer_cycle);
		if drops || !cycle || self.drops_open.is_empty() {
			self.drops.insert(ty, drops);
		} else {
			self.drops_cycle = true;
		}
		drops
	}

	/// Whether any field of `adt` with `args` drops, counting one of a type parameter only
	/// if `params`.
	pub fn fields_drop(&mut self, adt: AdtDef<'tcx>, args: ty::GenericArgsRef<'tcx>, params: bool) -> bool {
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

	/// Whether whatever holds a `ty` can drop it without knowing its type, through the
	/// `[Symbol.dispose]` its JS value has, as an `Arc` and a generic class's glue do.
	fn disposable(&mut self, ty: Ty<'tcx>) -> bool {
		if !self.drops(ty) {
			return self.inert(ty);
		}
		match ty.kind() {
			ty::Adt(adt, args) => match Std::of(self.tcx, adt.did()) {
				Some(Std::Waker) => true,
				Some(Std::Arc | Std::Mutex | Std::Option | Std::Box | Std::Pin) => self.disposable(args.type_at(0)),
				Some(_) => false,
				None => adt.is_struct() && args.types().all(|t| self.disposable(t)),
			},
			ty::Coroutine(..) | ty::Param(_) => true,
			_ => false,
		}
	}

	/// Whether dropping a `ty` through its JS value's `[Symbol.dispose]` does nothing. A
	/// reference is its referent in JS, so dropping a borrow of a value that drops would
	/// drop the value.
	fn inert(&mut self, ty: Ty<'tcx>) -> bool {
		if self.drops(ty) {
			return false;
		}
		match ty.kind() {
			ty::Ref(_, inner, _) => self.inert(*inner),
			ty::Adt(adt, args) => match Std::of(self.tcx, adt.did()) {
				// Each is its contents in JS.
				Some(Std::Option | Std::Box | Std::Mutex | Std::MutexGuard | Std::Poll | Std::Pin) => {
					args.types().all(|t| self.inert(t))
				}
				// Its waker.
				Some(Std::Context) => false,
				Some(_) => true,
				// Its glue, if an instance that drops gave it one, drops its type parameters' fields.
				None if adt.is_struct() => args.types().all(|t| self.inert(t)),
				None => true,
			},
			_ => true,
		}
	}

	/// What drops the `ty` in `value`, or `None` if JS would see nothing drop.
	pub fn drop_value(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> Option<Expr> {
		if !self.drops(ty) {
			return None;
		}
		// An array or `Vec`, or one in an `Option` or a box, which is it or `undefined`.
		if let Some(elem) = self.elements(ty)
			&& self.disposable(elem)
		{
			return Some(Expr::rt("dropAll", vec![value]));
		}
		let supported = match ty.kind() {
			ty::Adt(adt, args) => match Std::of(self.tcx, adt.did()) {
				Some(Std::Arc | Std::Mutex | Std::Option | Std::Box | Std::Pin) => self.disposable(args.type_at(0)),
				Some(Std::Waker) => true,
				Some(_) => false,
				None if adt.is_struct() => {
					let info = self.adt(*adt, span);
					self.glue(*adt, &info, span);
					args.types().all(|t| self.disposable(t))
				}
				None => false,
			},
			ty::Coroutine(..) | ty::Param(_) => true,
			_ => false,
		};
		// An enum or tuple drops by the glue rustc builds for it.
		if !supported && let Some(sym) = super::mir::glue::shim(self, ty, span) {
			return Some(super::mir::glue::call(sym, value));
		}
		if !supported && let ty::Closure(..) = ty.kind() {
			// Its captures live in an arrow function's scope, where nothing outside can drop them.
			self.err(
				span,
				"dropping a closure that owns a value that drops, which is not supported yet",
			);
			return None;
		}
		if !supported {
			self.err(span, format!("dropping `{ty}`, which is not supported yet"));
			return None;
		}
		Some(Expr::rt("drop", vec![value]))
	}

	/// The elements of an array or `Vec` that `ty` is, or holds in an `Option` or a box,
	/// whose JS value is the array or `undefined`.
	fn elements(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
		match ty.kind() {
			ty::Array(elem, _) => Some(*elem),
			ty::Adt(adt, args) => match Std::of(self.tcx, adt.did())? {
				Std::Vec => Some(args.type_at(0)),
				Std::Option | Std::Box | Std::Pin => self.elements(args.type_at(0)),
				_ => None,
			},
			_ => None,
		}
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
		let AdtRepr::Struct { fields, .. } = &info.repr else {
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
					body.push(StmtKind::Expr(Expr::Sym(sym).call(vec![Expr::This])).at(None));
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
			kind: SymKind::Method,
			order: 0,
			instance: String::new(),
			name: String::new(),
			alias: None,
		});
		self.program.methods.entry(info.sym).or_default().push(ts::Function {
			sym,
			params: Vec::new(),
			ret: Type::Builtin("void"),
			body,
			loc: None,
			doc: None,
		});
	}
}
