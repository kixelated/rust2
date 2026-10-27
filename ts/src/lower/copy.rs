//! Copies of `Copy` aggregates, by their type.
//!
//! JS shares an object where Rust copies its value, so a `Copy` value that is a JS object
//! the crate's code writes into in place is copied where Rust copies it. A function made
//! for its type copies what it holds by value and shares what it holds by reference, as
//! Rust does. A value of a type nothing writes into never changes once made, so sharing
//! it is a copy no one can tell apart.

use std::cell::RefCell;

use rustc_data_structures::fx::{FxHashMap, FxHashSet, FxIndexSet};
use rustc_hir::def_id::DefId;
use rustc_middle::mir::visit::{MutatingUseContext, PlaceContext, Visitor};
use rustc_middle::mir::{self, Location, Place, ProjectionElem, Rvalue};
use rustc_middle::ty::fast_reject::DeepRejectCtxt;
use rustc_middle::ty::{self, EarlyBinder, Ty, TyCtxt, TypeVisitableExt};
use rustc_span::{DUMMY_SP, Span};

use super::library::{Flow, Std};
use super::{AdtRepr, Translator, field_name, short_name};
use crate::program::{SymKind, Symbol};
use crate::ts::{self, Expr, Key, Param, Stmt, StmtKind};

impl<'a, 'tcx> Translator<'a, 'tcx> {
	/// Whether a `ty` is copied where Rust copies it: a `Copy` value that is, or holds by
	/// value, a JS object the crate's code writes into in place.
	pub fn needs_copy(&self, ty: Ty<'tcx>) -> bool {
		self.is_copy(ty) && self.changes(ty)
	}

	/// Whether a value of `ty` is, or holds by value, a JS object the crate writes into.
	fn changes(&self, ty: Ty<'tcx>) -> bool {
		let (object, parts): (bool, Vec<Ty<'tcx>>) = match ty.kind() {
			ty::Tuple(tys) if !tys.is_empty() => (true, tys.to_vec()),
			ty::Array(elem, _) => (true, vec![*elem]),
			ty::Adt(adt, args) => match Std::of(self.tcx, adt.did()) {
				// Their payload, or no object at all; a write into an `rt.Result`'s payload
				// replaces the result.
				Some(Std::Option | Std::Poll | Std::Result | Std::ControlFlow | Std::Reverse) => {
					(false, args.types().collect())
				}
				Some(Std::Bound) => (true, args.types().collect()),
				Some(_) => return false,
				None if adt.did().is_local() && adt.all_fields().next().is_some() => {
					(true, adt.all_fields().map(|f| self.field_ty(f.did, args)).collect())
				}
				None => return false,
			},
			_ => return false,
		};
		object && self.writes.written(self.tcx, ty) || parts.into_iter().any(|t| self.changes(t))
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
				Some(Std::Reverse) => ret(self.copied(v(), args.type_at(0), span)),
				Some(Std::Bound) => {
					let ts_ty = self.ty(ty, span);
					let held = self.copied(v().member("_0"), args.type_at(0), span);
					let copy = Expr::Object(vec![
						(Key::Name("tag".into()), v().member("tag")),
						(Key::Name("_0".into()), held),
					]);
					ret(Expr::Cond(
						Box::new(v().member("tag").binary("===", Expr::Str("Unbounded".into()))),
						Box::new(v()),
						Box::new(Expr::As(Box::new(copy), ts_ty)),
					))
				}
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

/// The types the crate's code writes into in place: through a projection, as `p.x = 1`
/// writes into `p`; through a reference or a box, as `*r = v` does into what `r` points
/// at; or by anything code it doesn't read can do with a mutable reference or pointer to
/// one, which counts as a write: one a parameter of a std function the crate names takes,
/// or one kept anywhere but in a local, whose uses tell. Bodies are read as written, over
/// their own type parameters, so a write into a type parameter stands for every type its
/// bounds let it be.
pub struct Writes<'tcx> {
	/// Each type written without type parameters, an array as a slice of its element,
	/// since the two share storage in JS.
	concrete: FxHashSet<Ty<'tcx>>,
	/// Each type over the type parameters of the body writing it, and that body.
	generic: FxIndexSet<(DefId, Ty<'tcx>)>,
	/// What [`Writes::written`] found of each type it was asked about.
	found: RefCell<FxHashMap<Ty<'tcx>, bool>>,
}

impl<'tcx> Writes<'tcx> {
	/// The writes of every function, closure, and coroutine body in the crate.
	pub fn of_crate(tcx: TyCtxt<'tcx>) -> Self {
		let mut writes = Writes {
			concrete: FxHashSet::default(),
			generic: FxIndexSet::default(),
			found: RefCell::default(),
		};
		for &def_id in tcx.mir_keys(()) {
			if !tcx.def_kind(def_id).is_fn_like() {
				continue;
			}
			let def_id = def_id.to_def_id();
			let body = tcx.optimized_mir(def_id);
			BodyWrites {
				tcx,
				body,
				def_id,
				writes: &mut writes,
			}
			.visit_body(body);
		}
		writes
	}

	fn insert(&mut self, tcx: TyCtxt<'tcx>, def_id: DefId, ty: Ty<'tcx>) {
		let ty = storage(tcx, ty);
		match ty.has_param() {
			true => _ = self.generic.insert((def_id, ty)),
			false => _ = self.concrete.insert(ty),
		}
	}

	/// Whether the crate writes into a value of `ty` in place. Over type parameters, as a
	/// census lowers some functions, it may.
	fn written(&self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
		let ty = tcx.erase_and_anonymize_regions(ty);
		let stored = storage(tcx, ty);
		if ty.has_param() || self.concrete.contains(&stored) {
			return true;
		}
		if let Some(&found) = self.found.borrow().get(&ty) {
			return found;
		}
		// A parameter can be the array itself, as in `*dst = value` with `dst: &mut T`, and
		// its slice pattern can be any array of that element.
		let found = self.generic.iter().any(|&(def_id, written)| {
			instantiates(tcx, def_id, written, ty) || stored != ty && instantiates(tcx, def_id, written, stored)
		});
		self.found.borrow_mut().insert(ty, found);
		found
	}
}

/// `ty` as [`Writes`] keys it: without regions, and an array as a slice of its element.
fn storage<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Ty<'tcx> {
	let ty = tcx.erase_and_anonymize_regions(ty);
	match ty.kind() {
		ty::Array(elem, _) => Ty::new_slice(tcx, *elem),
		_ => ty,
	}
}

/// Whether `written`, a type over the type parameters of `def_id`, is `ty` for some
/// arguments that satisfy its bounds. A projection on them, which only those arguments
/// decide, is any type that satisfies its associated type's bounds.
fn instantiates<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, written: Ty<'tcx>, ty: Ty<'tcx>) -> bool {
	use rustc_infer::infer::TyCtxtInferExt;
	use rustc_infer::traits::{Obligation, ObligationCause};
	use rustc_middle::ty::{BottomUpFolder, TypeFoldable, Upcast};
	use rustc_trait_selection::traits::ObligationCtxt;
	if !DeepRejectCtxt::relate_rigid_infer(tcx).types_may_unify(ty, written) {
		return false;
	}
	let (infcx, env) = tcx
		.infer_ctxt()
		.build_with_typing_env(ty::TypingEnv::fully_monomorphized());
	let ocx = ObligationCtxt::new(&infcx);
	let cause = ObligationCause::dummy();
	let register = |clause: ty::Clause<'tcx>| ocx.register_obligation(Obligation::new(tcx, cause.clone(), env, clause));
	let args = infcx.fresh_args_for_item(DUMMY_SP, def_id);
	let written = EarlyBinder::bind(tcx, written)
		.instantiate(tcx, args)
		.skip_normalization();
	let mut projections = Vec::new();
	let written = written.fold_with(&mut BottomUpFolder {
		tcx,
		ty_op: |t: Ty<'tcx>| match *t.kind() {
			ty::Alias(
				_,
				alias @ ty::AliasTy {
					kind: ty::Projection { .. },
					..
				},
			) => {
				let var = infcx.next_ty_var(DUMMY_SP);
				projections.push((alias, var));
				var
			}
			_ => t,
		},
		lt_op: |r| r,
		ct_op: |c| c,
	});
	for (alias, var) in projections {
		let ty::Projection { def_id: item } = alias.kind else {
			unreachable!("only projections are replaced")
		};
		for clause in tcx.item_bounds(item).iter_instantiated(tcx, alias.args) {
			let clause = clause.skip_normalization().fold_with(&mut BottomUpFolder {
				tcx,
				ty_op: |t: Ty<'tcx>| match *t.kind() {
					ty::Alias(_, a) if a == alias => var,
					_ => t,
				},
				lt_op: |r| r,
				ct_op: |c| c,
			});
			register(ocx.normalize(&cause, env, ty::Unnormalized::new(clause)));
		}
		let projection = ty::ProjectionClause {
			projection_term: alias.into(),
			term: var.into(),
		};
		register(projection.upcast(tcx));
	}
	let written = ocx.normalize(&cause, env, ty::Unnormalized::new(written));
	if ocx.eq(&cause, env, written, ty).is_err() {
		return false;
	}
	for (clause, _) in tcx.clauses_of(def_id).instantiate(tcx, args) {
		register(ocx.normalize(&cause, env, clause));
	}
	!ocx.try_evaluate_obligations().has_errors()
}

/// The writes of one body.
struct BodyWrites<'w, 'm, 'tcx> {
	tcx: TyCtxt<'tcx>,
	body: &'m mir::Body<'tcx>,
	def_id: DefId,
	writes: &'w mut Writes<'tcx>,
}

impl<'tcx> BodyWrites<'_, '_, 'tcx> {
	fn insert(&mut self, ty: Ty<'tcx>) {
		self.writes.insert(self.tcx, self.def_id, ty);
	}

	/// What each mutable reference or pointer a value of `ty` holds points at, where code
	/// given the value can reach it. What a closure or a coroutine holds only its own body
	/// reads, a function holds nothing, and what a trait object holds counted where it
	/// became one.
	fn pointees(&mut self, ty: Ty<'tcx>) {
		let mut stack = vec![ty];
		while let Some(ty) = stack.pop() {
			match *ty.kind() {
				ty::Ref(_, inner, mutability) | ty::RawPtr(inner, mutability) => {
					if mutability.is_mut() {
						self.insert(inner);
					}
					stack.push(inner);
				}
				ty::Adt(_, args) => stack.extend(args.types()),
				ty::Tuple(tys) => stack.extend(tys),
				ty::Array(elem, _) | ty::Slice(elem) => stack.push(elem),
				_ => {}
			}
		}
	}

	/// Whether the function item `def_id` with `args` is the crate's own, whose body is
	/// read, rather than std's or a trait method only an instance resolves.
	fn crate_fn(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
		let env = ty::TypingEnv::post_analysis(self.tcx, self.def_id);
		matches!(
			ty::Instance::try_resolve(self.tcx, env, def_id, args),
			Ok(Some(instance)) if matches!(instance.def, ty::InstanceKind::Item(did) if did.is_local())
		)
	}
}

impl<'tcx> Visitor<'tcx> for BodyWrites<'_, '_, 'tcx> {
	fn visit_assign(&mut self, place: &Place<'tcx>, rvalue: &Rvalue<'tcx>, location: Location) {
		if place.as_local().is_none() {
			self.pointees(place.ty(self.body, self.tcx).ty);
		}
		if let Rvalue::Cast(mir::CastKind::PointerCoercion(ty::adjustment::PointerCoercion::Unsize, _), op, to) = rvalue
			&& to.walk().any(|arg| arg.as_type().is_some_and(|t| t.is_trait()))
		{
			self.pointees(op.ty(self.body, self.tcx));
		}
		self.super_assign(place, rvalue, location);
	}

	fn visit_terminator(&mut self, terminator: &mir::Terminator<'tcx>, location: Location) {
		match &terminator.kind {
			mir::TerminatorKind::Call { destination, .. } if destination.as_local().is_none() => {
				self.pointees(destination.ty(self.body, self.tcx).ty)
			}
			mir::TerminatorKind::Yield { value, .. } => self.pointees(value.ty(self.body, self.tcx)),
			_ => {}
		}
		self.super_terminator(terminator, location);
	}

	/// A function not of the crate, called here or taken as a value that a pointer or a
	/// type parameter calls where no call shows: what its parameters take escapes to it.
	/// This holds while the runtime writes in place only through such a parameter, as
	/// `copy_from_slice` does, never in a std trait impl that a type parameter reaches.
	fn visit_operand(&mut self, operand: &mir::Operand<'tcx>, location: Location) {
		if let mir::Operand::Constant(constant) = operand
			&& let ty::FnDef(def_id, args) = *constant.ty().kind()
			&& !self.crate_fn(def_id, args.skip_binder())
		{
			let sig = self
				.tcx
				.fn_sig(def_id)
				.instantiate(self.tcx, args.skip_binder())
				.skip_norm_wip();
			for &input in sig.skip_binder().inputs() {
				self.pointees(input);
			}
		}
		self.super_operand(operand, location);
	}

	fn visit_place(&mut self, place: &Place<'tcx>, context: PlaceContext, _: Location) {
		let PlaceContext::MutatingUse(kind) = context else {
			return;
		};
		// A write to a part of a value writes into the value, unless the part is behind a
		// reference, whose referent it writes into instead.
		for (base, elem) in place.iter_projections() {
			if !matches!(elem, ProjectionElem::Deref) {
				self.insert(base.ty(self.body, self.tcx).ty);
			}
		}
		// A write to a whole local gives it another value, but one through a reference or a
		// box writes into the value there.
		if matches!(place.projection.last(), Some(ProjectionElem::Deref))
			|| matches!(kind, MutatingUseContext::SetDiscriminant)
		{
			self.insert(place.ty(self.body, self.tcx).ty);
		}
	}
}
