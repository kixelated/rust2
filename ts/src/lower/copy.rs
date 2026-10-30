//! Copies of `Copy` aggregates, by their type.
//!
//! JS shares an object where Rust copies its value, so a `Copy` value that is a JS object
//! the crate's code writes into in place is copied where Rust copies it. A function made
//! for its type copies what it holds by value and shares what it holds by reference, as
//! Rust does. A value of a type nothing writes into never changes once made, so sharing
//! it is a copy no one can tell apart.

use std::cell::RefCell;

use rustc_data_structures::fx::{FxHashMap, FxHashSet, FxIndexSet};
use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_index::IndexVec;
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
				Some(Std::MaybeUninit) => return self.changes(args.type_at(0)),
				Some(_) => return false,
				None if self.own(adt.did()) && adt.all_fields().next().is_some() => {
					(true, adt.all_fields().map(|f| self.field_ty(f.did, args)).collect())
				}
				None => return false,
			},
			_ => return false,
		};
		object && self.writes.into.contains(self.tcx, ty) || parts.into_iter().any(|t| self.changes(t))
	}

	/// Whether the crate may give a part of a `ty` kept as a JS object another object: by
	/// a write of the part, or of the whole value one object deep.
	pub fn replaced(&self, ty: Ty<'tcx>) -> bool {
		self.writes.replaced.contains(self.tcx, ty)
	}

	/// `value`, a `ty` read out of where it is kept, as a value of its own, as Rust's read
	/// copies its bytes: the JS objects it holds by value copied where the crate's code
	/// writes into them in place.
	pub fn snapshot(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> Expr {
		match self.changes(ty).then(|| self.shim(ty, span)).flatten() {
			Some(copy) => copy.call(vec![value]),
			None => value,
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
		self.shim(ty, span)
	}

	/// The function copying a `ty`, made the first time.
	fn shim(&mut self, ty: Ty<'tcx>, span: Span) -> Option<Expr> {
		// An array's copy is the same at every length.
		let tcx = self.tcx;
		let ty = match ty.kind() {
			ty::Array(elem, _) => Ty::new_array(tcx, *elem, 0),
			ty::Adt(adt, args) if Std::of(tcx, adt.did()) == Some(Std::MaybeUninit) => match args.type_at(0).kind() {
				ty::Array(elem, _) => Ty::new_maybe_uninit(tcx, Ty::new_array(tcx, *elem, 0)),
				_ => ty,
			},
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
		self.copy_parts(ty, false, span)
	}

	/// The body of the function copying `v`, a `ty`, or a `MaybeUninit` of one if `uninit`:
	/// then any of its parts kept as objects may hold nothing yet.
	fn copy_parts(&mut self, ty: Ty<'tcx>, uninit: bool, span: Span) -> Vec<Stmt> {
		let v = || Expr::ident("v");
		let ret = |e: Expr| vec![StmtKind::Return(Some(e.typed())).at(None)];
		let tcx = self.tcx;
		let part = |t: Ty<'tcx>| match uninit {
			true => Ty::new_maybe_uninit(tcx, t),
			false => t,
		};
		match ty.kind() {
			ty::Tuple(tys) => {
				let parts = tys
					.iter()
					.enumerate()
					.map(|(i, t)| self.snapshot(v().index(Expr::lit(i.to_string())), part(t), span))
					.collect();
				ret(Expr::Array(parts))
			}
			ty::Array(elem, _) => ret(
				match self.changes(*elem).then(|| self.shim(part(*elem), span)).flatten() {
					Some(copy) => v().member("map").call(vec![copy]),
					None => v().member("slice").call(Vec::new()),
				},
			),
			ty::Adt(adt, args) => match Std::of(tcx, adt.did()) {
				Some(Std::MaybeUninit) => {
					let none = StmtKind::If {
						cond: v().binary("===", Expr::undefined()),
						then: ret(v()),
						els: Vec::new(),
					};
					let mut body = vec![none.at(None)];
					body.extend(self.copy_parts(args.type_at(0), true, span));
					body
				}
				Some(Std::Option) => ret(self.copy_unless(self.none(args.type_at(0)), args.type_at(0), span)),
				Some(Std::Poll) => ret(self.copy_unless(self.pending(args.type_at(0)), args.type_at(0), span)),
				Some(Std::Result) => ret(self.copy_result(args.type_at(0), args.type_at(1), span)),
				Some(Std::Reverse) => ret(self.snapshot(v(), args.type_at(0), span)),
				Some(Std::Bound) => {
					let ts_ty = self.ty(ty, span);
					let held = self.snapshot(v().member("_0"), args.type_at(0), span);
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
						(Flow::Option, _) => self.copy_unless(self.none(value), value, span),
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
									self.snapshot(v().field(id), part(field_ty), span)
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
									props.push((
										Key::Name(name.clone()),
										self.snapshot(v().member(name), field_ty, span),
									));
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
						// A slot no field was written into yet holds nothing.
						AdtRepr::Slots(_) => {
							let (_, slots) = self.overlay(*adt, span).expect("a union is laid out where it is made");
							let ts_ty = self.ty(ty, span);
							let slots = slots
								.into_iter()
								.map(|(name, slot_ty)| {
									let slot_ty = self.instantiate(slot_ty, args);
									let value = self.snapshot(
										v().member(name.clone()),
										Ty::new_maybe_uninit(tcx, slot_ty),
										span,
									);
									(name, value)
								})
								.collect();
							ret(super::unions::made(slots, ts_ty))
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
			Box::new(Expr::rt("Ok", vec![self.snapshot(v().member("value"), ok, span)])),
			Box::new(Expr::rt("Err", vec![self.snapshot(v().member("error"), err, span)])),
		)
	}

	/// A copy of `v`, which is `none` or a `some`.
	fn copy_unless(&mut self, none: Expr, some: Ty<'tcx>, span: Span) -> Expr {
		let v = || Expr::ident("v");
		Expr::Cond(
			Box::new(v().binary("===", none)),
			Box::new(v()),
			Box::new(self.snapshot(v(), some, span)),
		)
	}

	/// The type of field `did` of an ADT with `args`.
	pub fn field_ty(&self, did: rustc_hir::def_id::DefId, args: ty::GenericArgsRef<'tcx>) -> Ty<'tcx> {
		self.instantiate(self.tcx.type_of(did).instantiate_identity().skip_norm_wip(), args)
	}

	/// `ty`, over an item's own type parameters, with `args` for them.
	pub fn instantiate(&self, ty: Ty<'tcx>, args: ty::GenericArgsRef<'tcx>) -> Ty<'tcx> {
		let ty = EarlyBinder::bind(self.tcx, ty);
		self.tcx
			.try_instantiate_and_normalize_erasing_regions(args, self.typing_env_pub(), ty)
			.unwrap_or_else(|_| ty.instantiate(self.tcx, args).skip_norm_wip())
	}
}

/// The types the translated crates' code writes into in place: through a projection, as `p.x = 1`
/// writes into `p`; through a reference or a box, as `*r = v` does into what `r` points
/// at; or by anything code it doesn't read can do with a mutable reference or pointer to
/// one, which counts as a write: one a parameter of a std function the crate names takes,
/// or one kept anywhere but in a local, whose uses tell. Bodies are read as written, over
/// their own type parameters, so a write into a type parameter stands for every type its
/// bounds let it be.
#[derive(Default)]
pub struct Writes<'tcx> {
	pub into: Types<'tcx>,
	/// The types a part of which, kept as a JS object, the code may give another object:
	/// by a write of the part, as `p.x = Inner {..}`; by a write of the whole value, which
	/// goes one object deep, as `*r = v` does; or by a call of a std function, as a
	/// mutable reference to the type may do with any part of it.
	pub replaced: Types<'tcx>,
}

/// Types, each without type parameters or over those of the body naming it.
#[derive(Default)]
pub struct Types<'tcx> {
	/// Each type without type parameters, an array as a slice of its element, since the
	/// two share storage in JS.
	concrete: FxHashSet<Ty<'tcx>>,
	/// Each type over the type parameters of the body writing it, and that body.
	generic: FxIndexSet<(DefId, Ty<'tcx>)>,
	/// What [`Types::contains`] found of each type it was asked about.
	found: RefCell<FxHashMap<Ty<'tcx>, bool>>,
}

impl<'tcx> Writes<'tcx> {
	/// The writes of every function, closure, and coroutine body of `bodies` that `own`
	/// says translates, and of the closures and coroutines they hold: a binding stands in
	/// for any other, which counts as code it doesn't read.
	pub fn of_bodies(tcx: TyCtxt<'tcx>, bodies: impl IntoIterator<Item = DefId>, own: &dyn Fn(DefId) -> bool) -> Self {
		let mut writes = Writes::default();
		let mut queue: Vec<DefId> = bodies.into_iter().collect();
		let mut seen = FxHashSet::default();
		while let Some(def_id) = queue.pop() {
			if !seen.insert(def_id) || !own(def_id) {
				continue;
			}
			let body = match tcx.def_kind(def_id) {
				// A thread-local's `const`, which lowers from this MIR as a function.
				DefKind::Const if super::library::local_init(tcx, def_id).is_some() => tcx.mir_for_ctfe(def_id),
				kind if kind.is_fn_like() && tcx.is_mir_available(def_id) => tcx.optimized_mir(def_id),
				// One without MIR, such as an import, is refused where it is reached.
				_ => continue,
			};
			// Another crate lists none of its closures' bodies.
			for local in body.local_decls.iter() {
				queue.extend(local.ty.walk().filter_map(|arg| match *arg.as_type()?.kind() {
					ty::Closure(nested, _) | ty::Coroutine(nested, _) | ty::CoroutineClosure(nested, _) => Some(nested),
					_ => None,
				}));
			}
			BodyWrites {
				tcx,
				body,
				def_id,
				writes: &mut writes,
				own,
				borrows: borrows(body),
			}
			.visit_body(body);
		}
		writes
	}
}

impl<'tcx> Types<'tcx> {
	fn insert(&mut self, tcx: TyCtxt<'tcx>, def_id: DefId, ty: Ty<'tcx>) {
		let ty = storage(tcx, ty);
		match ty.has_param() {
			true => _ = self.generic.insert((def_id, ty)),
			false => _ = self.concrete.insert(ty),
		}
	}

	/// Whether `ty` is one of them. Over type parameters, as a census lowers some
	/// functions, it may be.
	pub fn contains(&self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
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

/// `ty` as [`Types`] keys it: without regions, a `MaybeUninit` as what it holds, and an
/// array as a slice of its element.
fn storage<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Ty<'tcx> {
	let mut ty = tcx.erase_and_anonymize_regions(ty);
	while let ty::Adt(adt, args) = ty.kind()
		&& Std::of(tcx, adt.did()) == Some(Std::MaybeUninit)
	{
		ty = args.type_at(0);
	}
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

/// The place each local of `body` borrows, where its one assignment is a borrow.
fn borrows<'tcx>(body: &mir::Body<'tcx>) -> IndexVec<mir::Local, Option<Place<'tcx>>> {
	let mut defs = IndexVec::from_elem(0u32, &body.local_decls);
	let mut borrows = IndexVec::from_elem(None, &body.local_decls);
	for data in body.basic_blocks.iter() {
		for stmt in &data.statements {
			if let mir::StatementKind::Assign(assign) = &stmt.kind
				&& let Some(local) = assign.0.as_local()
			{
				defs[local] += 1;
				if let Rvalue::Ref(_, _, place) = assign.1 {
					borrows[local] = Some(place);
				}
			}
		}
		if let mir::TerminatorKind::Call { destination, .. } = &data.terminator().kind
			&& let Some(local) = destination.as_local()
		{
			defs[local] += 1;
		}
	}
	for (local, borrow) in borrows.iter_enumerated_mut() {
		if defs[local] != 1 {
			*borrow = None;
		}
	}
	borrows
}

/// `place`, read through each reference that `borrow` says borrows a place, as that place:
/// `(*r).0` of `r = &mut pair` is `pair.0`.
pub fn through_borrows<'tcx>(
	tcx: TyCtxt<'tcx>,
	mut place: Place<'tcx>,
	borrow: impl Fn(mir::Local) -> Option<Place<'tcx>>,
) -> Place<'tcx> {
	while let [ProjectionElem::Deref, rest @ ..] = place.projection.as_slice()
		&& let Some(borrowed) = borrow(place.local)
	{
		place = borrowed.project_deeper(rest, tcx);
	}
	place
}

/// The writes of one body.
struct BodyWrites<'w, 'm, 'tcx> {
	tcx: TyCtxt<'tcx>,
	body: &'m mir::Body<'tcx>,
	def_id: DefId,
	writes: &'w mut Writes<'tcx>,
	own: &'w dyn Fn(DefId) -> bool,
	/// The place each local borrows, where its one assignment is a borrow.
	borrows: IndexVec<mir::Local, Option<Place<'tcx>>>,
}

impl<'tcx> BodyWrites<'_, '_, 'tcx> {
	fn insert(&mut self, ty: Ty<'tcx>) {
		self.writes.into.insert(self.tcx, self.def_id, ty);
	}

	/// `ty` as replaced, if a part of it may be kept as a JS object: a write of the whole
	/// goes one object deep, so it replaces only the parts it holds directly.
	fn replace(&mut self, ty: Ty<'tcx>) {
		if self.inline(ty) {
			self.writes.replaced.insert(self.tcx, self.def_id, ty);
		}
	}

	/// Whether a `ty` may be kept as a JS object that a pointer to it is: a tuple, an
	/// array, a struct or union of a crate that translates, or a `MaybeUninit` of one; or
	/// an `UnsafeCell`'s `rt.Cell`, which the pointer `get` gives is or holds.
	fn inline(&self, ty: Ty<'tcx>) -> bool {
		match ty.kind() {
			ty::Tuple(tys) => !tys.is_empty(),
			ty::Array(..) | ty::Param(_) | ty::Alias(..) => true,
			ty::Adt(adt, args) => match Std::of(self.tcx, adt.did()) {
				Some(Std::MaybeUninit) => self.inline(args.type_at(0)),
				Some(Std::Cell) => self.tcx.is_lang_item(adt.did(), rustc_attr_ir::LangItem::UnsafeCell),
				Some(_) => false,
				None => !adt.is_enum() && (self.own)(adt.did()),
			},
			_ => false,
		}
	}

	/// A write of a whole `place` through `(*r)`, which writes the place `r` borrows if `r`
	/// is an alias of it, as a store of that place.
	fn store_through(&mut self, place: Place<'tcx>) {
		if let [ProjectionElem::Deref] = place.projection.as_slice()
			&& let Some(borrowed) = self.borrows[place.local]
		{
			self.stored(borrowed);
		}
	}

	/// A whole `place` written: a whole part gives what holds it another object there, and a
	/// whole value written through a pointer each of its parts, as it goes one object deep.
	fn stored(&mut self, place: Place<'tcx>) {
		let ty = place.ty(self.body, self.tcx).ty;
		match place.iter_projections().last() {
			Some((_, ProjectionElem::Deref)) => self.replace(ty),
			Some((base, ProjectionElem::Field(..))) if self.inline(ty) => {
				let base = base.ty(self.body, self.tcx).ty;
				if !base.is_union() {
					self.replace(base);
				}
			}
			_ => {}
		}
	}

	/// What each mutable reference or pointer a value of `ty` holds points at, where code
	/// given the value can reach it, written into, and if `replace`, replaced. What a
	/// closure or a coroutine holds only its own body reads, a function holds nothing,
	/// and what a trait object holds counted where it became one.
	fn pointees(&mut self, ty: Ty<'tcx>, replace: bool) {
		let mut stack = vec![ty];
		while let Some(ty) = stack.pop() {
			match *ty.kind() {
				ty::Ref(_, inner, mutability) | ty::RawPtr(inner, mutability) => {
					if mutability.is_mut() {
						self.insert(inner);
						if replace {
							self.replace(inner);
						}
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

	/// Whether the function item `def_id` with `args` translates, whose body is
	/// read, rather than std's or a trait method only an instance resolves.
	fn crate_fn(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
		let env = ty::TypingEnv::post_analysis(self.tcx, self.def_id);
		matches!(
			ty::Instance::try_resolve(self.tcx, env, def_id, args),
			Ok(Some(instance)) if matches!(instance.def, ty::InstanceKind::Item(did) if (self.own)(did))
		)
	}
}

impl<'tcx> Visitor<'tcx> for BodyWrites<'_, '_, 'tcx> {
	fn visit_assign(&mut self, place: &Place<'tcx>, rvalue: &Rvalue<'tcx>, location: Location) {
		if place.as_local().is_none() {
			self.pointees(place.ty(self.body, self.tcx).ty, true);
		}
		self.store_through(*place);
		// A `dyn Any` has no method that writes: a write through its downcast counts where it is.
		let object = |t: Ty<'tcx>| t.is_trait() && !super::dynamic::is_any_object(self.tcx, t);
		if let Rvalue::Cast(mir::CastKind::PointerCoercion(ty::adjustment::PointerCoercion::Unsize, _), op, to) = rvalue
			&& to.walk().any(|arg| arg.as_type().is_some_and(object))
		{
			self.pointees(op.ty(self.body, self.tcx), true);
		}
		self.super_assign(place, rvalue, location);
	}

	fn visit_terminator(&mut self, terminator: &mir::Terminator<'tcx>, location: Location) {
		if let mir::TerminatorKind::Call {
			func,
			args,
			destination,
			..
		} = &terminator.kind
		{
			self.store_through(*destination);
			let rebound = func
				.const_fn_def()
				.map_or(0, |(def_id, _)| super::pointer::rebinds(self.tcx, def_id));
			for arg in args[..rebound].iter().filter_map(|a| a.node.place()) {
				self.store_through(self.tcx.mk_place_deref(arg));
			}
		}
		match &terminator.kind {
			mir::TerminatorKind::Call { destination, .. } if destination.as_local().is_none() => {
				self.pointees(destination.ty(self.body, self.tcx).ty, true)
			}
			mir::TerminatorKind::Yield { value, .. } => self.pointees(value.ty(self.body, self.tcx), true),
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
			// Those that point a pointer elsewhere or read through it replace nothing.
			let replace = !super::pointer::keeps(self.tcx, def_id);
			for &input in sig.skip_binder().inputs() {
				self.pointees(input, replace);
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
		if matches!(
			kind,
			MutatingUseContext::Store
				| MutatingUseContext::Call
				| MutatingUseContext::AsmOutput
				| MutatingUseContext::Yield
		) {
			self.stored(*place);
		}
	}
}
