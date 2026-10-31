//! What may hold a raw pointer, so that rustc's liveness of those locals bounds where it
//! may still be read: the pointer made at the statement being lowered, or every pointer
//! into memory a write fills, which nothing points into yet if none may be read after.

use std::cell::RefCell;

use rustc_data_structures::either::Either;
use rustc_data_structures::fx::{FxHashMap, FxHashSet, FxIndexSet};
use rustc_hir::def_id::DefId;
use rustc_index::IndexVec;
use rustc_index::bit_set::DenseBitSet;
use rustc_middle::mir::visit::{PlaceContext, Visitor};
use rustc_middle::mir::{
	self, AggregateKind, Local, Location, Operand, Place, ProjectionElem, Rvalue, StatementKind, TerminatorKind,
};
use rustc_middle::ty::{self, Instance, Ty, TypingEnv};
use rustc_mir_dataflow::impls::MaybeLiveLocals;
use rustc_mir_dataflow::{Analysis, ResultsCursor};
use rustc_span::Spanned;

use super::{Fx, own};
use crate::lower::Translator;
use crate::lower::pointer::{self, Derives};

impl<'tcx> Fx<'_, '_, '_, 'tcx> {
	/// The locals that may hold the raw pointer the statement being lowered makes, or what
	/// is made of it, as [`Trace::holders`] finds them. `None` if one may escape where
	/// liveness can't follow it.
	pub(super) fn holders(&self) -> Option<DenseBitSet<Local>> {
		let made = match self.mir.stmt_at(self.at) {
			Either::Left(stmt) => match &stmt.kind {
				StatementKind::Assign(assign) => assign.0,
				_ => return None,
			},
			Either::Right(term) => match &term.kind {
				TerminatorKind::Call { destination, .. } => *destination,
				_ => return None,
			},
		};
		if !own(made) {
			return None;
		}
		let memo = RefCell::default();
		let held = Trace::new(self.b.t, self.mir, self.b.t.typing_env_pub(), &memo).holders(made.local, false);
		held.escapes.is_empty().then_some(held.all)
	}
}

/// A body's raw pointers, followed from where they are made.
pub(in crate::lower) struct Trace<'x, 'a, 'tcx> {
	t: &'x Translator<'a, 'tcx>,
	mir: &'x mir::Body<'tcx>,
	/// Where the body's trait bounds come from.
	env: TypingEnv<'tcx>,
	/// What each of the crate's functions does with the pointers it is given, as found.
	memo: &'x RefCell<Memo>,
	/// Each local's whole assignments, and whether anything else may change it.
	defs: RefCell<Option<IndexVec<Local, Defs<'x, 'tcx>>>>,
}

/// What each of the crate's functions does with the pointers it is given.
#[derive(Default)]
pub(in crate::lower) struct Memo {
	/// Whether a function may keep any of the given parameters anywhere but in its result.
	kept: FxHashMap<(DefId, Vec<Local>), bool>,
	/// The parameters each function's result may point where they do, or `None` if it may
	/// point anywhere else.
	returned: FxHashMap<DefId, Option<Vec<Local>>>,
}

/// The locals that may hold a pointer, and where one may escape them.
pub(in crate::lower) struct Held {
	pub all: DenseBitSet<Local>,
	/// Those that may hold a pointer to a part of what one points at, or to anything else
	/// made of it, rather than only to its whole.
	pub parts: DenseBitSet<Local>,
	/// The local the pointers point to, if they are made of it rather than held in it.
	value: Option<Local>,
	/// Where one may escape where liveness can't follow it: into memory, a closure, an
	/// async body's state (which is memory), or a call that may keep it.
	pub escapes: Vec<Location>,
}

/// A local's whole assignments, and whether anything else may change it.
#[derive(Default)]
struct Defs<'x, 'tcx> {
	whole: Vec<Def<'x, 'tcx>>,
	other: bool,
}

#[derive(Clone, Copy)]
enum Def<'x, 'tcx> {
	Assign(&'x Rvalue<'tcx>),
	Call(&'x Operand<'tcx>, &'x [Spanned<Operand<'tcx>>]),
}

/// Where a pointer may have come from.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Source {
	/// Memory the call assigning this local allocated.
	Fresh(Local),
	/// A borrow of this local or a part of it.
	Local(Local),
	/// This parameter.
	Param(Local),
}

/// The memory a write fills.
#[derive(Clone, Copy)]
pub(in crate::lower) enum Filled {
	/// Where this local points.
	Through(Local),
	/// This local or a part of it.
	Local(Local),
}

impl<'x, 'a, 'tcx> Trace<'x, 'a, 'tcx> {
	pub fn new(
		t: &'x Translator<'a, 'tcx>,
		mir: &'x mir::Body<'tcx>,
		env: TypingEnv<'tcx>,
		memo: &'x RefCell<Memo>,
	) -> Self {
		Self {
			t,
			mir,
			env,
			memo,
			defs: RefCell::new(None),
		}
	}

	/// The locals that may hold the pointer `root` holds, or with `value`, one to `root`
	/// itself, or what is made of it: a copy, a cast, a reference or pointer made through
	/// it, a value read through it that could hold one, or what std or the crate returns
	/// given one.
	pub fn holders(&self, root: Local, value: bool) -> Held {
		let mir = self.mir;
		let mut held = Held {
			all: DenseBitSet::new_empty(mir.local_decls.len()),
			parts: DenseBitSet::new_empty(mir.local_decls.len()),
			value: value.then_some(root),
			escapes: Vec::new(),
		};
		held.all.insert(root);
		let mut grew = true;
		while grew {
			grew = false;
			held.escapes.clear();
			for (block, data) in mir.basic_blocks.iter_enumerated() {
				for (statement_index, stmt) in data.statements.iter().enumerate() {
					let at = Location { block, statement_index };
					match &stmt.kind {
						StatementKind::Assign(assign) => {
							let (to, rvalue) = &**assign;
							if !reads(&held.all, |v| v.visit_rvalue(rvalue, at)) {
								continue;
							}
							if let Rvalue::Aggregate(kind, _) = rvalue
								&& matches!(
									**kind,
									AggregateKind::Closure(..)
										| AggregateKind::Coroutine(..)
										| AggregateKind::CoroutineClosure(..)
								) {
								held.escapes.push(at);
								continue;
							}
							let part = !whole(rvalue, &held);
							match self.held_in(*to, part, &mut held) {
								Some(new) => grew |= new,
								None => held.escapes.push(at),
							}
						}
						StatementKind::Intrinsic(_) if reads(&held.all, |v| v.visit_statement(stmt, at)) => {
							held.escapes.push(at);
						}
						_ => {}
					}
				}
				let at = mir.terminator_loc(block);
				let term = data.terminator();
				match &term.kind {
					TerminatorKind::Call {
						func,
						args,
						destination,
						..
					} => {
						let given: Vec<usize> = (0..args.len())
							.filter(|&i| reads_operand(&args[i].node, &held.all))
							.collect();
						if given.is_empty() && !reads_operand(func, &held.all) {
							continue;
						}
						match self.call(func, args, &given, &held) {
							Some(part) => match self.held_in(*destination, part, &mut held) {
								Some(new) => grew |= new,
								None => held.escapes.push(at),
							},
							None => held.escapes.push(at),
						}
					}
					TerminatorKind::Drop { place, .. }
						if held.all.contains(place.local)
							&& self.runs_own_code(place.ty(&mir.local_decls, self.t.tcx).ty) =>
					{
						held.escapes.push(at);
					}
					TerminatorKind::TailCall { .. } | TerminatorKind::InlineAsm { .. } | TerminatorKind::Yield { .. }
						if reads(&held.all, |v| v.visit_terminator(term, at)) =>
					{
						held.escapes.push(at);
					}
					_ => {}
				}
			}
		}
		held
	}

	/// Whether `to`, given a pointer, or one to a part of what it points at if `part`, newly
	/// holds it; `None` if that is memory, reached through a pointer.
	fn held_in(&self, to: Place<'tcx>, part: bool, held: &mut Held) -> Option<bool> {
		if !self.carries(to.ty(&self.mir.local_decls, self.t.tcx).ty) {
			return Some(false);
		}
		if !own(to) {
			return None;
		}
		let mut new = held.all.insert(to.local);
		// A local assigned in part holds whatever else it held, too.
		if part || !to.projection.is_empty() {
			new |= held.parts.insert(to.local);
		}
		Some(new)
	}

	/// What a call of `func` given `args`, of which those at `given` hold pointers, returns:
	/// whether a pointer to a part of what one points at; `None` if it may keep one
	/// anywhere else.
	fn call(&self, func: &Operand<'tcx>, args: &[Spanned<Operand<'tcx>>], given: &[usize], held: &Held) -> Option<bool> {
		if reads_operand(func, &held.all) {
			return None;
		}
		let ty::FnDef(def_id, gargs) = *func.ty(&self.mir.local_decls, self.t.tcx).kind() else {
			return None;
		};
		if let Some(derives) = pointer::derives(self.t.tcx, def_id) {
			let same = derives == Derives::Same
				&& given == [0]
				&& args[0].node.place().is_some_and(|p| held.points(p.local));
			return Some(!same);
		}
		if let Some(did) = self.read_fn(def_id, gargs.skip_binder()) {
			let params = given.iter().map(|&i| Local::from_usize(i + 1)).collect();
			return (!self.keeps(did, params)).then_some(true);
		}
		let given: Vec<Ty<'tcx>> = given
			.iter()
			.map(|&i| args[i].node.ty(&self.mir.local_decls, self.t.tcx))
			.collect();
		self.leaves(def_id, gargs.skip_binder(), args, &given).then_some(true)
	}

	/// The crate's function a call of `def_id` with `args` runs, if its body translates and
	/// takes its arguments as it declares them, which a closure's doesn't.
	fn read_fn(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<DefId> {
		let tcx = self.t.tcx;
		let instance = Instance::try_resolve(tcx, self.env, def_id, args).ok()??;
		let ty::InstanceKind::Item(did) = instance.def else {
			return None;
		};
		(self.t.lowers(instance)
			&& !crate::lower::library::is_std(tcx, did)
			&& !tcx.is_closure_like(did)
			&& tcx.def_kind(did).is_fn_like()
			&& tcx.is_mir_available(did))
		.then_some(did)
	}

	/// Whether the crate's function `did` may keep what its `params` hold anywhere but in
	/// its result.
	fn keeps(&self, did: DefId, params: Vec<Local>) -> bool {
		let key = (did, params);
		if let Some(&kept) = self.memo.borrow().kept.get(&key) {
			return kept;
		}
		// Until found, as for a function that calls itself, it may.
		self.memo.borrow_mut().kept.insert(key.clone(), true);
		let callee = self.callee(did);
		let kept = key.1.iter().any(|&param| !callee.holders(param, false).escapes.is_empty());
		self.memo.borrow_mut().kept.insert(key, kept);
		kept
	}

	/// The parameters of the crate's function `did` whose pointers its result may point
	/// where they do, or `None` if it may point anywhere else.
	fn returned(&self, did: DefId) -> Option<Vec<Local>> {
		if let Some(returned) = self.memo.borrow().returned.get(&did) {
			return returned.clone();
		}
		self.memo.borrow_mut().returned.insert(did, None);
		let returned = self.callee(did).origins(mir::RETURN_PLACE).and_then(|sources| {
			sources
				.into_iter()
				.map(|source| match source {
					Source::Param(param) => Some(param),
					Source::Fresh(_) | Source::Local(_) => None,
				})
				.collect()
		});
		self.memo.borrow_mut().returned.insert(did, returned.clone());
		returned
	}

	/// The trace of the crate's function `did`'s body.
	fn callee(&self, did: DefId) -> Trace<'x, 'a, 'tcx> {
		let tcx = self.t.tcx;
		Trace::new(self.t, tcx.optimized_mir(did), TypingEnv::post_analysis(tcx, did), self.memo)
	}

	/// Whether a call of std's `def_id` with `args`, of which the holders are `given`, keeps
	/// none of them anywhere but its result: std's code, calling none of the crate's, with
	/// no argument reaching somewhere that could take one in, as `Vec::push` does.
	fn leaves(
		&self,
		def_id: DefId,
		gargs: ty::GenericArgsRef<'tcx>,
		args: &[Spanned<Operand<'tcx>>],
		given: &[Ty<'tcx>],
	) -> bool {
		let tys: Vec<Ty<'tcx>> = args
			.iter()
			.map(|a| a.node.ty(&self.mir.local_decls, self.t.tcx))
			.collect();
		!self.translated(def_id, gargs)
			&& !gargs
				.types()
				.chain(tys.iter().copied())
				.any(|t| self.runs_own_code(t))
			&& !tys.iter().any(|&t| given.iter().any(|&h| self.t.stores(t, h)))
	}

	/// Whether a call of `def_id` with `args` runs a function whose body translates.
	fn translated(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
		match Instance::try_resolve(self.t.tcx, self.env, def_id, args) {
			Ok(Some(instance)) => self.t.lowers(instance) || self.t.dispatches(instance),
			_ => false,
		}
	}

	/// Whether std's code handed a value of `ty` may call the crate's on it, which could
	/// keep what it is given: through the crate's own types, a trait object, a closure, or
	/// a function pointer.
	fn runs_own_code(&self, ty: Ty<'tcx>) -> bool {
		self.t.reaches(ty, |t| match t.kind() {
			ty::Adt(adt, _) => self.t.own(adt.did()),
			ty::FnDef(def_id, args) => self.translated(*def_id, args.skip_binder()),
			ty::Dynamic(..)
			| ty::Closure(..)
			| ty::CoroutineClosure(..)
			| ty::Coroutine(..)
			| ty::FnPtr(..)
			| ty::Param(_)
			| ty::Alias(..) => true,
			_ => false,
		})
	}

	/// Whether a value of `ty` could hold a pointer or a reference, in any field of any type.
	fn carries(&self, ty: Ty<'tcx>) -> bool {
		let mut seen = FxHashSet::default();
		let mut stack = vec![ty];
		while let Some(ty) = stack.pop() {
			if !seen.insert(ty) {
				continue;
			}
			match ty.kind() {
				ty::Bool
				| ty::Char
				| ty::Int(_)
				| ty::Uint(_)
				| ty::Float(_)
				| ty::Str
				| ty::Never
				| ty::FnDef(..)
				| ty::FnPtr(..) => {}
				ty::Array(elem, _) | ty::Slice(elem) | ty::Pat(elem, _) => stack.push(*elem),
				ty::Tuple(tys) => stack.extend(tys.iter()),
				ty::Adt(adt, args) => stack.extend(adt.all_fields().map(|field| self.t.field_ty(field.did, args))),
				_ => return true,
			}
		}
		false
	}

	/// Whether a write at `at` fills memory nothing points into yet: where it may run, no
	/// pointer into the memory has escaped, and every local that may hold one and be read
	/// after, but the call's own result `made`, holds only pointers to the whole of what it
	/// was made for, which the write fills in place and gives only its parts other objects.
	pub fn fresh(
		&self,
		filled: Filled,
		at: Location,
		made: Option<Local>,
		live: &mut ResultsCursor<'x, 'tcx, MaybeLiveLocals>,
	) -> bool {
		let sources = match filled {
			Filled::Through(pointer) => match self.origins(pointer) {
				Some(sources) => sources,
				None => return false,
			},
			Filled::Local(local) => FxIndexSet::from_iter([Source::Local(local)]),
		};
		sources.into_iter().all(|source| {
			let held = match source {
				Source::Fresh(root) => self.holders(root, false),
				Source::Local(root) => self.holders(root, true),
				Source::Param(_) => return false,
			};
			if held.escapes.iter().any(|&escape| self.reaches(escape, at)) {
				return false;
			}
			live.seek_before_primary_effect(at);
			!held
				.parts
				.iter()
				.any(|local| Some(local) != made && live.get().contains(local))
		})
	}

	/// Rustc's liveness of the body's locals.
	pub fn liveness(&self) -> ResultsCursor<'x, 'tcx, MaybeLiveLocals> {
		MaybeLiveLocals
			.iterate_to_fixpoint(self.t.tcx, self.mir, None)
			.into_results_cursor(self.mir)
	}

	/// Whether code at `to` may run after code at `from`, or is it.
	fn reaches(&self, from: Location, to: Location) -> bool {
		if from.block == to.block && from.statement_index <= to.statement_index {
			return true;
		}
		let blocks = &self.mir.basic_blocks;
		let mut seen = DenseBitSet::new_empty(blocks.len());
		let mut stack: Vec<_> = blocks[from.block].terminator().successors().collect();
		while let Some(block) = stack.pop() {
			if block == to.block {
				return true;
			}
			if seen.insert(block) {
				stack.extend(blocks[block].terminator().successors());
			}
		}
		false
	}

	/// Where the pointers `local` may hold may have come from, following copies, casts,
	/// pointers made through them, and calls of std or the crate that return where they
	/// point; `None` if from anywhere else, such as memory.
	fn origins(&self, local: Local) -> Option<FxIndexSet<Source>> {
		let defs = self.defs();
		let defs = defs.as_ref().expect("defs are found");
		let tcx = self.t.tcx;
		let mut sources = FxIndexSet::default();
		let mut seen = FxHashSet::default();
		let mut stack = vec![local];
		while let Some(local) = stack.pop() {
			if !seen.insert(local) {
				continue;
			}
			if defs[local].other {
				return None;
			}
			if local != mir::RETURN_PLACE && local.as_usize() <= self.mir.arg_count {
				sources.insert(Source::Param(local));
			}
			for def in &defs[local].whole {
				match *def {
					Def::Assign(Rvalue::Use(op, _) | Rvalue::Cast(_, op, _)) => stack.push(direct(op.place()?)?),
					Def::Assign(Rvalue::CopyForDeref(place)) => stack.push(direct(*place)?),
					Def::Assign(Rvalue::Ref(_, _, place) | Rvalue::RawPtr(_, place)) => {
						match place.projection.split_first() {
							Some((ProjectionElem::Deref, rest)) if !rest.contains(&ProjectionElem::Deref) => {
								stack.push(place.local)
							}
							_ if !place.is_indirect() => _ = sources.insert(Source::Local(place.local)),
							_ => return None,
						}
					}
					Def::Assign(_) => return None,
					Def::Call(func, args) => {
						let ty::FnDef(def_id, gargs) = *func.ty(&self.mir.local_decls, tcx).kind() else {
							return None;
						};
						if pointer::allocates(tcx, def_id) {
							sources.insert(Source::Fresh(local));
						} else if pointer::derives(tcx, def_id).is_some() {
							for arg in args {
								if self.carries(arg.node.ty(&self.mir.local_decls, tcx)) {
									stack.push(direct(arg.node.place()?)?);
								}
							}
						} else {
							let did = self.read_fn(def_id, gargs.skip_binder())?;
							for param in self.returned(did)? {
								stack.push(direct(args[param.as_usize() - 1].node.place()?)?);
							}
						}
					}
				}
			}
		}
		Some(sources)
	}

	/// The body's [`Defs`], found the first time.
	fn defs(&self) -> std::cell::Ref<'_, Option<IndexVec<Local, Defs<'x, 'tcx>>>> {
		if self.defs.borrow().is_none() {
			let mir = self.mir;
			let mut defs: IndexVec<Local, Defs<'x, 'tcx>> = IndexVec::from_fn_n(|_| Defs::default(), mir.local_decls.len());
			let assigned = |to: &Place<'tcx>, def: Def<'x, 'tcx>, defs: &mut IndexVec<Local, Defs<'x, 'tcx>>| {
				match to.as_local() {
					Some(local) => defs[local].whole.push(def),
					None if !to.is_indirect() => defs[to.local].other = true,
					None => {}
				}
			};
			for data in mir.basic_blocks.iter() {
				for stmt in &data.statements {
					match &stmt.kind {
						StatementKind::Assign(assign) => {
							let (to, rvalue) = &**assign;
							assigned(to, Def::Assign(rvalue), &mut defs);
							// A local something borrows may change through the borrow.
							if let Rvalue::Ref(_, _, place) | Rvalue::RawPtr(_, place) = rvalue
								&& !place.is_indirect()
							{
								defs[place.local].other = true;
							}
						}
						StatementKind::SetDiscriminant { place, .. } if !place.is_indirect() => {
							defs[place.local].other = true
						}
						_ => {}
					}
				}
				match &data.terminator().kind {
					TerminatorKind::Call {
						func,
						args,
						destination,
						..
					} => assigned(destination, Def::Call(func, args), &mut defs),
					TerminatorKind::InlineAsm { .. } | TerminatorKind::Yield { .. } | TerminatorKind::TailCall { .. } => {
						defs.iter_mut().for_each(|d| d.other = true)
					}
					_ => {}
				}
			}
			*self.defs.borrow_mut() = Some(defs);
		}
		self.defs.borrow()
	}
}

/// Whether a local of `rvalue`, which reads one of the holders, holds only pointers to the
/// whole of what they point at: a copy or cast of a holder that does, a field of it such
/// as an `Option`'s payload, or a reference or pointer made of one or through it.
fn whole(rvalue: &Rvalue<'_>, held: &Held) -> bool {
	match rvalue {
		Rvalue::Use(op, _) | Rvalue::Cast(_, op, _) => op.place().is_some_and(|p| !p.is_indirect() && held.points(p.local)),
		Rvalue::CopyForDeref(place) => !place.is_indirect() && held.points(place.local),
		Rvalue::Ref(_, _, place) | Rvalue::RawPtr(_, place) => match place.projection.as_slice() {
			[] => !held.parts.contains(place.local),
			[ProjectionElem::Deref] => held.points(place.local),
			_ => false,
		},
		_ => false,
	}
}

impl Held {
	/// Whether `local` holds only pointers to the whole of what they point at.
	fn points(&self, local: Local) -> bool {
		!self.parts.contains(local) && self.value != Some(local)
	}
}

/// The local `place` is, read as a whole or through fields of it, with no pointer on its
/// path.
fn direct(place: Place<'_>) -> Option<Local> {
	(!place.is_indirect()).then_some(place.local)
}

/// Whether `op` reads one of `holders`.
fn reads_operand(op: &Operand<'_>, holders: &DenseBitSet<Local>) -> bool {
	op.place().is_some_and(|p| holders.contains(p.local))
}

/// Whether what `visit` visits reads one of `holders`.
fn reads<'tcx>(holders: &DenseBitSet<Local>, visit: impl FnOnce(&mut Reads<'_>)) -> bool {
	let mut reads = Reads { holders, found: false };
	visit(&mut reads);
	reads.found
}

/// Whether what it visits reads one of the holders.
struct Reads<'a> {
	holders: &'a DenseBitSet<Local>,
	found: bool,
}

impl<'tcx> Visitor<'tcx> for Reads<'_> {
	fn visit_local(&mut self, local: Local, _: PlaceContext, _: Location) {
		self.found |= self.holders.contains(local);
	}
}
