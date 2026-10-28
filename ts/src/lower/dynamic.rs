//! Trait objects. A `dyn Trait` is a JS object with a method for each of the trait's
//! dispatchable functions, its supertraits' included, named as an impl's function is,
//! for its trait (`area$Shape`), and taking the arguments after `self`. That is the
//! trait's type, which a TS caller can implement. A coercion to one wraps the value in a
//! class of its type's own, whose methods call that type's impl and whose
//! `[Symbol.dispose]`, which a `Box<dyn Trait>` drops it by, drops the value. An
//! `Arc<dyn Trait>` is an `rt.ArcDyn` around the one a box would be.

use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, GenericArgsRef, Instance, Interner as _, Ty, TyCtxt};
use rustc_span::Span;

use super::Translator;
use super::body::Body;
use crate::program::{SymKind, Symbol};
use crate::ts::{self, Expr, FieldId, FieldInit, Item, Param, Stmt, StmtKind, SymId, Type};

/// A trait that objects of it translate for: its type, and its methods in vtable order.
#[derive(Clone)]
pub struct Object {
	pub sym: SymId,
	pub methods: Vec<Method>,
}

#[derive(Clone)]
pub struct Method {
	pub def_id: DefId,
	pub name: String,
}

/// The class a coercion wraps a value of one type in, and its field holding the value.
#[derive(Clone, Copy)]
pub struct Class {
	pub sym: SymId,
	pub v: FieldId,
}

/// How a wrapper class holds the value a pointer to it was unsized from.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Held {
	/// Its value, from `&T` or `Box<T>`: the class is an `rt.Place` of it, as a `v`.
	Value,
	/// The `rt.Place` a `&mut T` to a value JS copies is.
	Place,
}

impl<'tcx> Translator<'_, 'tcx> {
	/// The trait of the trait object `ty`, a `dyn Trait`, made the first time.
	pub fn object(&mut self, ty: Ty<'tcx>, span: Span) -> Option<Object> {
		let tcx = self.tcx;
		let ty::Dynamic(preds, ..) = ty.kind() else {
			return None;
		};
		let Some(principal) = preds.principal_def_id() else {
			self.unrepresented(
				span,
				format!("`{ty}`, a trait object of no trait with methods, which is not supported yet"),
			);
			return None;
		};
		if let Some(object) = self.objects.get(&principal) {
			return Some(object.clone());
		}
		// A trait of a crate translated from source is as the crate's own.
		if !self.own(principal) && !is_any(tcx, principal) {
			self.unrepresented(
				span,
				format!("`{ty}`, a trait object of another crate's trait, which is not supported yet"),
			);
			return None;
		}
		if preds.projection_bounds().next().is_some() {
			self.unrepresented(
				span,
				format!("`{ty}`, a trait object with an associated type, which is not supported yet"),
			);
			return None;
		}
		let traits: Vec<DefId> = tcx.supertrait_def_ids(principal).collect();
		if let Some(&generic) = traits.iter().find(|&&tr| tcx.generics_of(tr).count() > 1) {
			let msg = format!(
				"`{ty}`, a trait object of the generic trait `{}`, which is not supported yet",
				tcx.def_path_str(generic)
			);
			self.unrepresented(span, msg);
			return None;
		}
		let name = tcx.item_name(principal).to_string();
		let sym = self.program.sym(Symbol {
			module: crate::program::type_module(&self.module_of(principal), &name),
			base: name,
			suffix: String::new(),
			owner: None,
			kind: SymKind::Union,
			order: self.order(tcx.def_span(principal)),
			instance: String::new(),
			name: String::new(),
			alias: None,
		});
		let mut methods = Vec::new();
		// `Any`'s object is its class, which a downcast checks for, so it needs no method.
		for &tr in traits.iter().filter(|&&tr| !is_any(tcx, tr)) {
			for &def_id in tcx.own_existential_vtable_entries(tr) {
				let name = format!("{}${}", super::camel(tcx.item_name(def_id).as_str()), tcx.item_name(tr));
				if methods.iter().any(|m: &Method| m.name == name) {
					self.err(span, format!("`{ty}`, whose traits have two methods named `{name}`"));
					return None;
				}
				methods.push(Method { def_id, name });
			}
		}
		let object = Object { sym, methods };
		// Before its methods' types, which may name it.
		self.objects.insert(principal, object.clone());
		let mut members = Vec::new();
		for method in &object.methods {
			let args = method_args(tcx, method.def_id, ty);
			let sig = tcx.fn_sig(method.def_id).instantiate(tcx, args).skip_norm_wip();
			let sig = tcx.instantiate_bound_regions_with_erased(sig);
			let func = self.fn_ty(&sig.inputs()[1..], sig.output(), span);
			members.push((method.name.clone(), func));
		}
		self.program.items.push(Item::Union {
			sym,
			params: Vec::new(),
			// Any object, as a downcast checks for the class of the value's own type.
			ty: match is_any(tcx, principal) {
				true => Type::Builtin("object"),
				false => Type::Object(members),
			},
			doc: self.doc(principal),
		});
		Some(object)
	}

	/// Whether `instance` is a call through a trait object's method, whose wrapper takes
	/// its arguments as the crate's functions do. A `&dyn Debug` or `&dyn Display` is a
	/// function writing its value instead, and a `dyn Future` an `rt.Future`.
	pub fn dispatches(&self, instance: Instance<'tcx>) -> bool {
		let ty::InstanceKind::Virtual(..) = instance.def else {
			return false;
		};
		let object = instance.args.type_at(0);
		let ty::Dynamic(preds, ..) = object.kind() else {
			return false;
		};
		self.dyn_future(object).is_none()
			&& preds
				.principal_def_id()
				.is_none_or(|principal| super::fmt::Trait::of(self.tcx, principal).is_none())
	}

	/// The class wrapping a `from` as the trait object `to`, held as `held`, made the first
	/// time.
	pub fn object_class(&mut self, from: Ty<'tcx>, to: Ty<'tcx>, held: Held, span: Span) -> Option<Class> {
		let tcx = self.tcx;
		let from = tcx.erase_and_anonymize_regions(from);
		let to = tcx.erase_and_anonymize_regions(to);
		let object = self.object(to, span)?;
		let key = (from, object.sym, held);
		if let Some(&class) = self.object_classes.get(&key) {
			return Some(class);
		}
		let trait_sym = &self.program.syms[object.sym];
		let base = super::short_name(tcx, from);
		let sym = self.program.sym(Symbol {
			module: trait_sym.module.clone(),
			base,
			suffix: match held {
				Held::Value => String::new(),
				Held::Place => "Place".into(),
			},
			owner: None,
			kind: SymKind::Struct,
			order: trait_sym.order,
			instance: String::new(),
			name: String::new(),
			alias: None,
		});
		let v = self.program.field(sym, "v".into());
		self.object_classes.insert(key, Class { sym, v });
		let ty = self.ty(from, span);
		let field_ty = match held {
			Held::Value => ty,
			Held::Place => Type::Rt("Place", vec![ty]),
		};
		self.program.items.push(Item::Struct {
			sym,
			params: Vec::new(),
			fields: vec![ts::Field {
				id: v,
				ty: field_ty,
				doc: None,
				init: FieldInit::Ctor,
			}],
			ctor: ts::Ctor::Positional,
			doc: None,
		});
		let value = Expr::This.field(v);
		for method in &object.methods {
			let args = method_args(tcx, method.def_id, from);
			let sig = tcx.fn_sig(method.def_id).instantiate(tcx, args).skip_norm_wip();
			let sig = tcx.instantiate_bound_regions_with_erased(sig);
			let receiver = match self.receiver(sig.inputs()[0], from, held, value.clone()) {
				Some(receiver) => receiver,
				None => {
					let msg = format!(
						"`{to}` with the receiver `{}`, which is not supported yet",
						sig.inputs()[0]
					);
					self.err(span, msg);
					continue;
				}
			};
			let mut b = Body::new(self, args);
			let params: Vec<Param> = sig.inputs()[1..]
				.iter()
				.map(|input| Param {
					name: b.fresh("a"),
					ty: Some(b.t.ty(*input, span)),
				})
				.collect();
			let values = params.iter().map(|p| Expr::ident(&p.name));
			let mut body = Vec::new();
			let call = b.call_def(
				method.def_id,
				args,
				[receiver].into_iter().chain(values).collect(),
				span,
				&mut body,
			);
			let ret = self.ret_ty(sig.output(), span);
			body.push(StmtKind::Return(Some(call.typed())).at(None));
			let method = self.method(sym, method.name.clone());
			self.program.methods.entry(sym).or_default().push(ts::Function {
				sym: method,
				params,
				ret,
				body,
				loc: None,
				doc: None,
			});
		}
		if held == Held::Value
			&& let Some(dropped) = self.drop_value(value, from, span)
		{
			let dispose = self.method(sym, "[Symbol.dispose]".into());
			self.program.methods.entry(sym).or_default().push(ts::Function {
				sym: dispose,
				params: Vec::new(),
				ret: Type::Builtin("void"),
				body: vec![StmtKind::Expr(dropped).at(None)],
				loc: None,
				doc: None,
			});
		}
		Some(Class { sym, v })
	}

	/// A method `base` of the class `owner`.
	fn method(&mut self, owner: SymId, base: String) -> SymId {
		let module = self.program.syms[owner].module.clone();
		self.program.sym(Symbol {
			module,
			base,
			suffix: String::new(),
			owner: Some(owner),
			kind: SymKind::Method,
			order: 0,
			instance: String::new(),
			name: String::new(),
			alias: None,
		})
	}

	/// What a wrapper of a `from`, holding `value` as `held`, passes as `self` of type
	/// `receiver`, if it can. A `&mut` wrapper is never a `Box<Self>`'s, which it reads anyway.
	fn receiver(&self, receiver: Ty<'tcx>, from: Ty<'tcx>, held: Held, value: Expr) -> Option<Expr> {
		let read = match held {
			Held::Place => value.clone().member("v"),
			Held::Value => value.clone(),
		};
		match receiver.kind() {
			ty::Ref(_, _, ty::Mutability::Not) => Some(read),
			ty::Ref(_, _, ty::Mutability::Mut) => Some(match held {
				// The class is a place of its value.
				Held::Value if self.needs_place(from) => Expr::This,
				_ => value,
			}),
			_ if receiver.is_box() => Some(read),
			_ => None,
		}
	}
}

/// Whether `def_id` is std's `Any`.
fn is_any(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
	tcx.is_diagnostic_item(rustc_span::Symbol::intern("Any"), def_id)
}

/// Whether `ty` is a `dyn Any`.
pub fn is_any_object(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
	principal(ty).is_some_and(|p| is_any(tcx, p))
}

/// The trait of the trait object `ty`.
fn principal(ty: Ty<'_>) -> Option<DefId> {
	match ty.kind() {
		ty::Dynamic(preds, ..) => preds.principal_def_id(),
		_ => None,
	}
}

/// The arguments of a trait object's method `def_id` for `self_ty`: the trait's `Self`,
/// then its own early-bound lifetimes, erased, which are all a dispatchable method takes.
fn method_args<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, self_ty: Ty<'tcx>) -> GenericArgsRef<'tcx> {
	ty::GenericArgs::for_item(tcx, def_id, |param, _| match param.index {
		0 => self_ty.into(),
		_ => tcx.lifetimes.re_erased.into(),
	})
}

impl<'tcx> Body<'_, '_, 'tcx> {
	/// A call of the trait method `def_id` on a trait object, `args[0]`, through the method
	/// its object has.
	pub fn virtual_call(&mut self, def_id: DefId, object: Ty<'tcx>, mut args: Vec<Expr>, span: Span) -> Expr {
		let Some(found) = self.t.object(object, span) else {
			return Expr::undefined();
		};
		let Some(method) = found.methods.iter().find(|m| m.def_id == def_id) else {
			if self.t.tcx.trait_of_assoc(def_id).is_some_and(|tr| is_any(self.t.tcx, tr)) {
				return self.err(
					span,
					format!("`{}` of `{object}`, which is not supported yet", self.t.tcx.def_path_str(def_id)),
				);
			}
			return self.err(
				span,
				format!("`{}` is not a method of `{object}`", self.t.tcx.def_path_str(def_id)),
			);
		};
		let receiver = args.remove(0);
		receiver.member(method.name.clone()).call(args)
	}

	/// `value`, a `&T`, `&mut T`, or `Box<T>`, unsized to `to`, the same pointer to a trait
	/// object.
	pub fn object_of(&mut self, value: Expr, from: Ty<'tcx>, to: Ty<'tcx>, span: Span) -> Expr {
		let unsupported = |b: &mut Self| b.err(span, format!("a coercion to `{to}`, which is not supported yet"));
		let (Some(pointee), Some(object)) = (from.builtin_deref(true), to.builtin_deref(true)) else {
			return unsupported(self);
		};
		// The pointer itself, not a struct whose last field unsizes.
		if !object.is_trait() {
			return unsupported(self);
		}
		// A box holds one as a `Uint8Array` and a reference as an `rt.ByteSlice`, which one
		// class's methods can't both take.
		if super::byte_array(pointee) {
			return self.err(
				span,
				format!("a trait object of `{pointee}`, which is not supported yet"),
			);
		}
		if self.t.object(object, span).is_none() {
			return Expr::undefined();
		}
		// An upcast, from a trait object, which has its supertraits' methods already. One
		// to `dyn Any` would need the class of the value's own type, which a downcast checks.
		if pointee.is_trait() {
			if is_any_object(self.t.tcx, object) && principal(pointee) != principal(object) {
				return self.err(span, format!("an upcast from `{pointee}` to `{object}`, which is not supported yet"));
			}
			return value;
		}
		let held = match from.kind() {
			ty::Ref(_, _, ty::Mutability::Mut) if self.t.needs_place(pointee) => Held::Place,
			ty::Ref(..) => Held::Value,
			_ if from.is_box() => Held::Value,
			_ => return unsupported(self),
		};
		match self.t.object_class(pointee, object, held, span) {
			Some(class) => Expr::New(Box::new(Expr::Sym(class.sym)), vec![value]),
			None => Expr::undefined(),
		}
	}
}

/// A call of `dyn Any`'s `is`, `downcast_ref`, or `downcast_mut`, or of `Box`'s or `Arc`'s
/// `downcast`, each of which checks for the class a coercion wrapped a value of its type
/// in, or its arguments back if it is none of these.
pub fn call<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	call: &super::library::Call<'tcx>,
	owner: &str,
	name: &str,
	args: Vec<Expr>,
	out: &mut Vec<Stmt>,
) -> Result<Option<Expr>, Vec<Expr>> {
	let tcx = b.t.tcx;
	if !matches!((owner, name), ("dyn Any", "is" | "downcast_ref" | "downcast_mut") | ("Box" | "Arc", "downcast")) {
		return Err(args);
	}
	let sig = tcx.fn_sig(call.def_id).instantiate(tcx, call.args).skip_norm_wip();
	let receiver = tcx.instantiate_bound_regions_with_erased(sig).inputs()[0];
	let object = match receiver.kind() {
		ty::Adt(_, args) => args.type_at(0),
		_ => receiver.builtin_deref(true).unwrap_or(receiver),
	};
	if !is_any_object(tcx, object) {
		return Err(args);
	}
	let span = call.span;
	let to = call.args.types().last().expect("a downcast's type");
	let Some(value) = b.t.object_class(to, object, Held::Value, span) else {
		return Ok(None);
	};
	// A `&mut` to a value JS copies is the place it is in, which a class of its own holds.
	let place = match b.t.needs_place(to) && owner == "dyn Any" {
		true => b.t.object_class(to, object, Held::Place, span),
		false => None,
	};
	let o = first(args);
	let o = if o.is_path() { o } else { b.spill(o, "object", out) };
	let is = |class: Class| o.clone().binary("instanceof", Expr::Sym(class.sym));
	let none = b.t.none(to);
	let e = match name {
		"is" => place.into_iter().fold(is(value), |e, place| e.binary("||", is(place))),
		"downcast_ref" | "downcast_mut" => {
			// A `&mut` to a value JS copies is the class itself, whose `v` is the value.
			let get = |class: Class, held: Held| match (name, held) {
				("downcast_mut", Held::Value) if place.is_some() => o.clone(),
				("downcast_mut", Held::Place) => o.clone().field(class.v),
				(_, Held::Value) => o.clone().field(class.v),
				(_, Held::Place) => o.clone().field(class.v).member("v"),
			};
			let rest = match place {
				Some(place) => Expr::Cond(Box::new(is(place)), Box::new(get(place, Held::Place)), Box::new(none)),
				None => none,
			};
			Expr::Cond(Box::new(is(value)), Box::new(get(value, Held::Value)), Box::new(rest))
		}
		_ if owner == "Box" => Expr::Cond(
			Box::new(is(value)),
			Box::new(Expr::result(true, o.clone().field(value.v))),
			Box::new(Expr::result(false, o.clone())),
		),
		// The `Arc` the object was made from, whose count the object shares.
		_ => Expr::rt("arcDowncast", vec![o, Expr::Sym(value.sym)]),
	};
	Ok(Some(e))
}

fn first(args: Vec<Expr>) -> Expr {
	args.into_iter().next().expect("a receiver")
}
