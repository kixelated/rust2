//! What may hold the raw pointer made at the statement being lowered, so that rustc's
//! liveness of those locals bounds where the pointer may still be read.

use rustc_data_structures::either::Either;
use rustc_data_structures::fx::FxHashSet;
use rustc_index::bit_set::DenseBitSet;
use rustc_middle::mir::visit::{PlaceContext, Visitor};
use rustc_middle::mir::{AggregateKind, Local, Location, Operand, Place, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty::{self, Ty};
use rustc_span::Spanned;

use super::{Fx, own};

impl<'tcx> Fx<'_, '_, '_, 'tcx> {
	/// The locals that may hold the raw pointer the statement being lowered makes, or what
	/// is made of it: a copy, a cast, a reference or pointer made through it, a value read
	/// through it that could hold one, or what std returns given one. `None` if one may
	/// escape where liveness can't follow it: into memory, a closure, an async body's state
	/// (which is memory), or a call that may keep it.
	pub(super) fn holders(&self) -> Option<DenseBitSet<Local>> {
		let mir = self.mir;
		let made = match mir.stmt_at(self.at) {
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
		let mut holders = DenseBitSet::new_empty(mir.local_decls.len());
		holders.insert(made.local);
		let mut grew = true;
		while grew {
			grew = false;
			for data in mir.basic_blocks.iter() {
				for stmt in &data.statements {
					match &stmt.kind {
						StatementKind::Assign(assign) => {
							let (to, rvalue) = &**assign;
							let mut reads = Reads::new(&holders);
							reads.visit_rvalue(rvalue, Location::START);
							if !reads.found {
								continue;
							}
							if let Rvalue::Aggregate(kind, _) = rvalue
								&& matches!(
									**kind,
									AggregateKind::Closure(..)
										| AggregateKind::Coroutine(..)
										| AggregateKind::CoroutineClosure(..)
								) {
								return None;
							}
							grew |= self.held_in(*to, &mut holders)?;
						}
						StatementKind::Intrinsic(_) => {
							let mut reads = Reads::new(&holders);
							reads.visit_statement(stmt, Location::START);
							if reads.found {
								return None;
							}
						}
						_ => {}
					}
				}
				let term = data.terminator();
				match &term.kind {
					TerminatorKind::Call {
						func,
						args,
						destination,
						..
					} => {
						let given: Vec<Ty<'tcx>> = args
							.iter()
							.filter(|a| reads_operand(&a.node, &holders))
							.map(|a| a.node.ty(&mir.local_decls, self.tcx))
							.collect();
						if given.is_empty() && !reads_operand(func, &holders) {
							continue;
						}
						if !self.leaves(func, args, &given) {
							return None;
						}
						grew |= self.held_in(*destination, &mut holders)?;
					}
					TerminatorKind::Drop { place, .. } => {
						if holders.contains(place.local) && self.runs_own_code(place.ty(&mir.local_decls, self.tcx).ty)
						{
							return None;
						}
					}
					TerminatorKind::TailCall { .. }
					| TerminatorKind::InlineAsm { .. }
					| TerminatorKind::Yield { .. } => {
						let mut reads = Reads::new(&holders);
						reads.visit_terminator(term, Location::START);
						if reads.found {
							return None;
						}
					}
					_ => {}
				}
			}
		}
		Some(holders)
	}

	/// Whether `to`, given what may hold the pointer, newly holds it; `None` if that is
	/// memory, reached through a pointer.
	fn held_in(&self, to: Place<'tcx>, holders: &mut DenseBitSet<Local>) -> Option<bool> {
		if !self.carries(to.ty(&self.mir.local_decls, self.tcx).ty) {
			return Some(false);
		}
		own(to).then(|| holders.insert(to.local))
	}

	/// Whether a call of `func` given `args`, of which the holders are `given`, keeps none
	/// of them anywhere but its result: std's code, calling none of the crate's, with no
	/// argument reaching somewhere that could take one in, as `Vec::push` does.
	fn leaves(&self, func: &Operand<'tcx>, args: &[Spanned<Operand<'tcx>>], given: &[Ty<'tcx>]) -> bool {
		let ty::FnDef(_, gargs) = *func.ty(&self.mir.local_decls, self.tcx).kind() else {
			return false;
		};
		let tys: Vec<Ty<'tcx>> = args
			.iter()
			.map(|a| a.node.ty(&self.mir.local_decls, self.tcx))
			.collect();
		!self.translated(func)
			&& !gargs
				.skip_binder()
				.types()
				.chain(tys.iter().copied())
				.any(|t| self.runs_own_code(t))
			&& !tys.iter().any(|&t| given.iter().any(|&h| self.stores(t, h)))
	}

	/// Whether std's code handed a value of `ty` may call the crate's on it, which could
	/// keep what it is given: through the crate's own types, a trait object, a closure, or
	/// a function pointer.
	fn runs_own_code(&self, ty: Ty<'tcx>) -> bool {
		self.reaches(ty, |t| match t.kind() {
			ty::Adt(adt, _) => self.b.t.own(adt.did()),
			ty::FnDef(def_id, args) => self.b.t.translated(*def_id, args.skip_binder()),
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
				ty::Adt(adt, args) => stack.extend(adt.all_fields().map(|field| self.b.t.field_ty(field.did, args))),
				_ => return true,
			}
		}
		false
	}
}

/// Whether `op` reads one of `holders`.
fn reads_operand(op: &Operand<'_>, holders: &DenseBitSet<Local>) -> bool {
	op.place().is_some_and(|p| holders.contains(p.local))
}

/// Whether what it visits reads one of the holders.
struct Reads<'a> {
	holders: &'a DenseBitSet<Local>,
	found: bool,
}

impl<'a> Reads<'a> {
	fn new(holders: &'a DenseBitSet<Local>) -> Self {
		Self { holders, found: false }
	}
}

impl<'tcx> Visitor<'tcx> for Reads<'_> {
	fn visit_local(&mut self, local: Local, _: PlaceContext, _: Location) {
		self.found |= self.holders.contains(local);
	}
}
