//! Switches an enclosing switch already decided: `match x { Dot => match x { .. } }`
//! tests `x` again where only `Dot` reaches. TypeScript narrows `x` in the arm and
//! refuses a test for another variant, so each such switch goes straight to the arms
//! that can still match.
//!
//! A forward dataflow over the body finds, at each switch on a place's variant, the
//! variants the place can hold: a switch's edge narrows them, and anything that can
//! change the place forgets them.

use rustc_data_structures::fx::FxHashSet;
use rustc_index::IndexVec;
use rustc_middle::mir::visit::Visitor;
use rustc_middle::mir::{
	self, BasicBlock, BorrowKind, Local, Location, MutBorrowKind, Operand, Place, ProjectionElem, Rvalue,
	StatementKind, SwitchTargets, TerminatorKind,
};
use rustc_middle::ty::{self, Ty, TyCtxt};

/// Fold each switch in `body` that an enclosing switch decided.
pub fn fold<'tcx>(tcx: TyCtxt<'tcx>, body: &mut mir::Body<'tcx>) {
	let flow = Flow::new(tcx, body);
	let mut entry: IndexVec<BasicBlock, Option<Facts<'tcx>>> = IndexVec::from_elem(None, &body.basic_blocks);
	entry[mir::START_BLOCK] = Some(Facts::default());
	let mut work = vec![mir::START_BLOCK];
	while let Some(bb) = work.pop() {
		let Some(facts) = entry[bb].clone() else {
			continue;
		};
		for (to, out) in flow.block(body, bb, facts).1 {
			let joined = match &entry[to] {
				None => out,
				Some(old) => old.join(&out),
			};
			if entry[to].as_ref() != Some(&joined) {
				entry[to] = Some(joined);
				work.push(to);
			}
		}
	}
	let mut folded = false;
	for bb in body.basic_blocks.indices() {
		let Some(facts) = entry[bb].clone() else {
			continue;
		};
		let (Some((def, kept, otherwise)), _) = flow.block(body, bb, facts) else {
			continue;
		};
		let data = &mut body.basic_blocks_mut()[bb];
		let TerminatorKind::SwitchInt { discr, targets } = &mut data.terminator_mut().kind else {
			unreachable!("decided only for a switch")
		};
		if kept.len() == targets.iter().count() {
			continue;
		}
		folded = true;
		if kept.iter().any(|&(_, t)| t != otherwise) {
			*targets = SwitchTargets::new(kept.into_iter(), otherwise);
			continue;
		}
		// A discriminant the switch moved has no other reader.
		let moved = matches!(discr, Operand::Move(_));
		data.terminator_mut().kind = TerminatorKind::Goto {
			target: otherwise,
			loop_hint_attrs: Default::default(),
		};
		if moved {
			data.statements[def].kind = StatementKind::Nop;
		}
	}
	// What only a folded edge reached is gone, so nothing it does counts as a use.
	if folded {
		let reachable = mir::traversal::reachable_as_bitset(body);
		for (bb, data) in body.basic_blocks_mut().iter_enumerated_mut() {
			if !reachable.contains(bb) {
				data.statements.clear();
				data.terminator_mut().kind = TerminatorKind::Unreachable;
			}
		}
	}
}

/// The variants, by discriminant, that each place known so far can hold.
#[derive(Clone, Default, PartialEq)]
struct Facts<'tcx>(Vec<(Place<'tcx>, Vec<u128>)>);

impl<'tcx> Facts<'tcx> {
	/// What both `self` and `other` know.
	fn join(&self, other: &Self) -> Self {
		Self(
			self.0
				.iter()
				.filter_map(|(p, vs)| {
					let (_, theirs) = other.0.iter().find(|(q, _)| q == p)?;
					let mut all: Vec<u128> = vs.iter().chain(theirs).copied().collect();
					all.sort_unstable();
					all.dedup();
					Some((*p, all))
				})
				.collect(),
		)
	}

	/// Forget what `gone` says of a place, and of the discriminants read in this block.
	fn forget(&mut self, discrs: &mut Vec<(Local, usize, Place<'tcx>)>, gone: impl Fn(Place<'tcx>) -> bool) {
		self.0.retain(|(p, _)| !gone(*p));
		discrs.retain(|(_, _, p)| !gone(*p));
	}
}

/// A switch's statement that reads its discriminant, and the arms and default that can
/// still match.
type Decided = (usize, Vec<(u128, BasicBlock)>, BasicBlock);

/// What a body's places are, for the dataflow.
struct Flow<'tcx> {
	tcx: TyCtxt<'tcx>,
	/// Each reference local's one assignment, as the place it borrows or the place behind
	/// the reference it copies.
	borrows: IndexVec<Local, Option<Place<'tcx>>>,
	/// Parameters assigned nowhere, which keep their value.
	kept: IndexVec<Local, bool>,
	/// Locals something can change behind the dataflow's back, by a raw pointer or a
	/// two-phase borrow.
	escaped: IndexVec<Local, bool>,
}

impl<'tcx> Flow<'tcx> {
	fn new(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>) -> Self {
		let n = body.local_decls.len();
		let mut defs: IndexVec<Local, usize> = IndexVec::from_elem_n(0, n);
		let mut rvalues: IndexVec<Local, Option<&Rvalue<'tcx>>> = IndexVec::from_elem_n(None, n);
		for data in body.basic_blocks.iter() {
			for stmt in &data.statements {
				if let StatementKind::Assign(assign) = &stmt.kind
					&& let Some(local) = assign.0.as_local()
				{
					defs[local] += 1;
					rvalues[local] = Some(&assign.1);
				}
			}
			if let TerminatorKind::Call { destination, .. } = &data.terminator().kind
				&& let Some(local) = destination.as_local()
			{
				defs[local] += 1;
			}
		}
		let arg = |local: Local| (1..=body.arg_count).contains(&local.as_usize());
		let mut borrows = IndexVec::from_elem_n(None, n);
		for (local, rvalue) in rvalues.iter_enumerated() {
			if defs[local] != 1 || arg(local) || !body.local_decls[local].ty.is_ref() {
				continue;
			}
			borrows[local] = match rvalue {
				Some(Rvalue::Ref(_, _, place)) => Some(*place),
				Some(Rvalue::Use(Operand::Copy(p) | Operand::Move(p), _) | Rvalue::CopyForDeref(p)) => {
					Some(tcx.mk_place_deref(*p))
				}
				_ => None,
			};
		}
		let kept = defs.iter_enumerated().map(|(local, &d)| d == 0 && arg(local)).collect();
		let mut flow = Self {
			tcx,
			borrows,
			kept,
			escaped: IndexVec::from_elem_n(false, n),
		};
		for data in body.basic_blocks.iter() {
			for stmt in &data.statements {
				let StatementKind::Assign(assign) = &stmt.kind else {
					continue;
				};
				let escapes = match &assign.1 {
					Rvalue::RawPtr(_, p) => Some(*p),
					// Reads go past a two-phase borrow before it writes, which then forgets
					// only what the reference is known to borrow.
					Rvalue::Ref(
						_,
						BorrowKind::Mut {
							kind: MutBorrowKind::TwoPhaseBorrow,
						},
						p,
					) if flow.canon(tcx.mk_place_deref(assign.0)).local == assign.0.local => Some(*p),
					_ => None,
				};
				if let Some(p) = escapes {
					let local = flow.canon(p).local;
					flow.escaped[local] = true;
				}
			}
		}
		flow
	}

	/// `place`, through each reference it reads that borrows a place that stays put.
	fn canon(&self, mut place: Place<'tcx>) -> Place<'tcx> {
		while let [ProjectionElem::Deref, rest @ ..] = place.projection.as_slice()
			&& let Some(borrowed) = self.borrows[place.local]
		{
			let borrowed = self.canon(borrowed);
			let fixed = borrowed.projection.iter().enumerate().all(|(i, elem)| match elem {
				ProjectionElem::Deref => i == 0 && self.kept[borrowed.local],
				_ => !indexed(elem),
			});
			if !fixed {
				break;
			}
			place = borrowed.project_deeper(rest, self.tcx);
		}
		place
	}

	/// Whether the dataflow can follow the variant `place` holds.
	fn tracks(&self, body: &mir::Body<'tcx>, place: Place<'tcx>) -> bool {
		!self.escaped[place.local] && !place.projection.iter().any(indexed) && variants(self.tcx, body, place).is_some()
	}

	/// What a write to `place` makes unknown: `place` and what holds it, but not its own
	/// parts, which a variant's payload is.
	fn written(&self, place: Place<'tcx>) -> impl Fn(Place<'tcx>) -> bool + use<'tcx> {
		let place = self.canon(place);
		move |p| prefix(place, p)
	}

	/// What a copy of a pointer `place` holds can change: what it points at, and whatever
	/// is reached through a pointer it holds.
	fn lent(&self, body: &mir::Body<'tcx>, place: Place<'tcx>) -> Option<impl Fn(Place<'tcx>) -> bool + use<'tcx>> {
		let ty = place.ty(&body.local_decls, self.tcx).ty;
		if !lends(self.tcx, ty) {
			return None;
		}
		let base = self.canon(place);
		let pointee = (ty.is_ref() || ty.is_raw_ptr()).then(|| self.canon(self.tcx.mk_place_deref(place)));
		Some(move |p: Place<'tcx>| {
			pointee.is_some_and(|t| prefix(t, p))
				|| p.projection.iter().enumerate().any(|(i, e)| {
					matches!(e, ProjectionElem::Deref)
						&& base.local == p.local
						&& p.projection[..i].starts_with(base.projection)
				})
		})
	}

	/// Run `bb` from `facts`: the switch it decided, if any, and the facts on each edge
	/// that can be taken.
	fn block(
		&self,
		body: &mir::Body<'tcx>,
		bb: BasicBlock,
		mut facts: Facts<'tcx>,
	) -> (Option<Decided>, Vec<(BasicBlock, Facts<'tcx>)>) {
		let data = &body.basic_blocks[bb];
		// The discriminants read in this block so far: their locals, statements, and places.
		let mut discrs: Vec<(Local, usize, Place<'tcx>)> = Vec::new();
		for (i, stmt) in data.statements.iter().enumerate() {
			let location = Location {
				block: bb,
				statement_index: i,
			};
			for lent in lent_places(|v| v.visit_statement(stmt, location)) {
				if let Some(gone) = self.lent(body, lent) {
					facts.forget(&mut discrs, gone);
				}
			}
			match &stmt.kind {
				StatementKind::Assign(assign) => {
					let (place, rvalue) = &**assign;
					// A copy holds the variant its original does.
					let copied: Vec<(Place<'tcx>, Vec<u128>)> = match rvalue {
						Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _) | Rvalue::CopyForDeref(from) => {
							let (from, to) = (self.canon(*from), self.canon(*place));
							facts
								.0
								.iter()
								.filter(|(p, _)| prefix(from, *p))
								.map(|(p, vs)| {
									let rest = &p.projection[from.projection.len()..];
									(to.project_deeper(rest, self.tcx), vs.clone())
								})
								.filter(|(p, _)| self.tracks(body, *p))
								.collect()
						}
						_ => Vec::new(),
					};
					facts.forget(&mut discrs, self.written(*place));
					discrs.retain(|(d, _, _)| place.local != *d);
					// A `&mut` can change what it borrows from then on.
					if let Rvalue::Ref(_, BorrowKind::Mut { .. }, borrowed) = rvalue {
						facts.forget(&mut discrs, self.written(*borrowed));
					}
					facts.0.extend(copied);
					if let Rvalue::Discriminant(of) = rvalue
						&& let Some(local) = place.as_local()
						&& self.tracks(body, self.canon(*of))
					{
						discrs.push((local, i, self.canon(*of)));
					}
				}
				StatementKind::SetDiscriminant { place, .. } => facts.forget(&mut discrs, self.written(**place)),
				StatementKind::StorageLive(local) | StatementKind::StorageDead(local) => {
					facts.forget(&mut discrs, |p| p.local == *local);
				}
				_ => {}
			}
		}
		let term = data.terminator();
		let location = Location {
			block: bb,
			statement_index: data.statements.len(),
		};
		for lent in lent_places(|v| v.visit_terminator(term, location)) {
			if let Some(gone) = self.lent(body, lent) {
				facts.forget(&mut discrs, gone);
			}
		}
		let mut out: Vec<(BasicBlock, Facts<'tcx>)> = Vec::new();
		let mut decided = None;
		match &term.kind {
			TerminatorKind::SwitchInt {
				discr: Operand::Copy(d) | Operand::Move(d),
				targets,
			} if let Some(&(_, def, of)) = discrs.iter().find(|(l, _, _)| d.as_local() == Some(*l)) => {
				let all = variants(self.tcx, body, of).expect("checked where read");
				let known = facts
					.0
					.iter()
					.find(|(p, _)| *p == of)
					.map_or(all.clone(), |(_, vs)| vs.clone());
				let target = |v: u128| {
					targets
						.iter()
						.find(|&(w, _)| w == v)
						.map_or(targets.otherwise(), |(_, t)| t)
				};
				// Each edge that can be taken, with the variants that take it.
				for &v in &known {
					match out.iter_mut().find(|(t, _)| *t == target(v)) {
						Some((_, facts)) => facts.0.last_mut().expect("pushed below").1.push(v),
						None => {
							let mut facts = facts.clone();
							facts.0.retain(|(p, _)| *p != of);
							facts.0.push((of, vec![v]));
							out.push((target(v), facts));
						}
					}
				}
				for (_, facts) in &mut out {
					facts.0.last_mut().expect("pushed above").1.sort_unstable();
				}
				let mut kept: Vec<(u128, BasicBlock)> = targets.iter().filter(|(v, _)| known.contains(v)).collect();
				let listed = |v: &u128| targets.iter().any(|(w, _)| w == *v);
				let mut otherwise = targets.otherwise();
				// Where nothing reaches the default now, the last arm left takes its place.
				if all.iter().any(|v| !listed(v))
					&& known.iter().all(listed)
					&& let Some((_, last)) = kept.pop()
				{
					otherwise = last;
				}
				decided = Some((def, kept, otherwise));
			}
			TerminatorKind::Goto { target, .. }
			| TerminatorKind::Assert { target, .. }
			| TerminatorKind::FalseEdge {
				real_target: target, ..
			}
			| TerminatorKind::FalseUnwind {
				real_target: target, ..
			} => out.push((*target, facts.clone())),
			TerminatorKind::Call {
				destination,
				target: Some(target),
				..
			} => {
				let mut facts = facts.clone();
				facts.forget(&mut discrs, self.written(*destination));
				out.push((*target, facts));
			}
			TerminatorKind::Drop { place, target, .. } => {
				let mut facts = facts.clone();
				facts.forget(&mut discrs, self.written(*place));
				// Drop glue gets a `&mut` to the value.
				if let Some(gone) = self.lent(body, *place) {
					facts.forget(&mut discrs, gone);
				}
				out.push((*target, facts));
			}
			_ => {}
		}
		// Unwinding, and whatever else leaves, knows nothing.
		if decided.is_none() {
			for to in term.successors() {
				if !out.iter().any(|(t, _)| *t == to) {
					out.push((to, Facts::default()));
				}
			}
		}
		(decided, out)
	}
}

/// The places a statement or terminator copies or moves, and that a `CopyForDeref`
/// reads, as `visit` walks it.
fn lent_places<'tcx>(visit: impl FnOnce(&mut Operands<'tcx>)) -> Vec<Place<'tcx>> {
	let mut ops = Operands(Vec::new());
	visit(&mut ops);
	ops.0
}

struct Operands<'tcx>(Vec<Place<'tcx>>);

impl<'tcx> Visitor<'tcx> for Operands<'tcx> {
	fn visit_operand(&mut self, op: &Operand<'tcx>, _: Location) {
		if let Operand::Copy(p) | Operand::Move(p) = op {
			self.0.push(*p);
		}
	}

	fn visit_rvalue(&mut self, rvalue: &Rvalue<'tcx>, location: Location) {
		if let Rvalue::CopyForDeref(p) = rvalue {
			self.0.push(*p);
		}
		self.super_rvalue(rvalue, location);
	}
}

/// Whether a value of `ty` can change what it points at: it is or holds a `&mut` or a
/// `*mut`, or hides what it holds.
fn lends<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
	let mut seen = FxHashSet::default();
	let mut stack = vec![ty];
	while let Some(ty) = stack.pop() {
		for t in ty.walk().filter_map(|arg| arg.as_type()) {
			match t.kind() {
				ty::Ref(_, _, ty::Mutability::Mut)
				| ty::RawPtr(_, ty::Mutability::Mut)
				| ty::Dynamic(..)
				| ty::Alias(..)
				| ty::Param(_) => return true,
				ty::Adt(adt, args) if adt.did().is_local() && seen.insert(t) => {
					stack.extend(adt.all_fields().map(|field| field.ty(tcx, args).skip_norm_wip()));
				}
				_ => {}
			}
		}
	}
	false
}

/// Whether `elem` picks an element, which a write to another element can be.
fn indexed(elem: ProjectionElem<Local, Ty<'_>>) -> bool {
	matches!(
		elem,
		ProjectionElem::Index(_) | ProjectionElem::ConstantIndex { .. } | ProjectionElem::Subslice { .. }
	)
}

/// Whether `a` is `b` or holds it.
fn prefix<'tcx>(a: Place<'tcx>, b: Place<'tcx>) -> bool {
	a.local == b.local && b.projection.starts_with(a.projection)
}

/// The discriminants of the enum at `place`, if it is one.
fn variants<'tcx>(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>, place: Place<'tcx>) -> Option<Vec<u128>> {
	match place.ty(&body.local_decls, tcx).ty.kind() {
		ty::Adt(adt, _) if adt.is_enum() => Some(adt.discriminants(tcx).map(|(_, d)| d.val).collect()),
		_ => None,
	}
}
