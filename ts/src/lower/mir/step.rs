//! Iterators over an array stepped in place: a slice's or a `Vec`'s iterator that only
//! `next` reads, as a `for` loop's is, is the array and an index, `{ s, i }`, rather than
//! an `rt.Iter`, which a bundler can't shake its adaptors from.

use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_middle::mir::{self, Local, Operand, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty::{self, Ty, TyCtxt};

use super::Fx;
use crate::lower::library::{self, Std};
use crate::ts::{Expr, Key, Type};

/// The item type of a slice's or a `Vec`'s iterator, if `ty` is one.
fn array_iter<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
	let ty::Adt(adt, args) = ty.kind() else {
		return None;
	};
	let path = library::std_path(tcx, adt.did());
	let path: Vec<&str> = path.iter().map(String::as_str).collect();
	match path.as_slice() {
		["slice", "iter", "Iter" | "IterMut"] | ["vec", "into_iter", "IntoIter"] => args.types().next(),
		_ => None,
	}
}

/// Whether `ty` is what such an iterator is made from: a slice, an array, or a `Vec`.
fn array<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
	match ty.peel_refs().kind() {
		ty::Slice(_) | ty::Array(..) => true,
		ty::Adt(adt, _) => Std::of(tcx, adt.did()) == Some(Std::Vec),
		_ => false,
	}
}

impl<'tcx> Fx<'_, '_, '_, 'tcx> {
	/// Find the iterators to step in place: each made by a call from an array, or moved
	/// from another, and otherwise only borrowed for `next` into a protocol local and
	/// dropped. Each borrow is read only as `next`'s receiver.
	pub(super) fn find_stepped(&mut self) {
		let mir = self.mir;
		let tcx = self.tcx;
		let mut iters: Vec<Local> = mir
			.local_decls
			.indices()
			.filter(|&l| !self.is_arg(l) && l != mir::RETURN_PLACE && self.defs[l] == 1)
			.filter(|&l| array_iter(tcx, mir.local_decls[l].ty).is_some())
			.collect();
		if iters.is_empty() {
			return;
		}
		// What each reads that is allowed: borrows for `next`, drops, moves into another.
		let mut reads: FxHashMap<Local, u32> = FxHashMap::default();
		let mut made = FxHashSet::default();
		let mut moves: Vec<(Local, Local)> = Vec::new();
		let mut borrows: FxHashMap<Local, Local> = FxHashMap::default();
		let mut next_reads: FxHashMap<Local, u32> = FxHashMap::default();
		for data in mir.basic_blocks.iter() {
			for stmt in &data.statements {
				let StatementKind::Assign(assign) = &stmt.kind else {
					continue;
				};
				let (place, rvalue) = &**assign;
				match rvalue {
					Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, from) if from.as_local().is_some() => {
						let from = from.as_local().unwrap();
						*reads.entry(from).or_default() += 1;
						if let Some(to) = place.as_local() {
							borrows.insert(to, from);
						}
					}
					Rvalue::Use(Operand::Move(from), _) if from.as_local().is_some() => {
						let from = from.as_local().unwrap();
						*reads.entry(from).or_default() += 1;
						if let Some(to) = place.as_local() {
							moves.push((from, to));
						}
					}
					_ => {}
				}
			}
			match &data.terminator().kind {
				TerminatorKind::Drop { place, .. } if let Some(local) = place.as_local() => {
					*reads.entry(local).or_default() += 1;
				}
				TerminatorKind::Call {
					func,
					args,
					destination,
					..
				} => {
					let ty::FnDef(def_id, _) = *func.ty(&mir.local_decls, tcx).kind() else {
						continue;
					};
					let maker = library::key(tcx, def_id).is_some_and(|(o, n)| {
						matches!(
							(o.as_str(), n.as_str()),
							("IntoIterator", "into_iter") | ("slice", "iter" | "iter_mut")
						)
					});
					if maker
						&& let Some(to) = destination.as_local()
						&& let [arg] = &**args
						&& array(tcx, arg.node.ty(&mir.local_decls, tcx))
					{
						made.insert(to);
					}
					if crate::lower::iter::is_next(tcx, def_id)
						&& destination.as_local().is_some_and(|l| self.next_results.contains(&l))
						&& let [arg] = &**args
						&& let Operand::Copy(p) | Operand::Move(p) = &arg.node
						&& let Some(recv) = p.as_local()
					{
						*next_reads.entry(recv).or_default() += 1;
					}
				}
				_ => {}
			}
		}
		// A borrow only `next` reads, assigned once and borrowed by nothing.
		let stepping = |fx: &Self, borrow: Local| {
			fx.defs[borrow] == 1
				&& !fx.borrowed[borrow]
				&& next_reads.get(&borrow).copied().unwrap_or(0) == fx.uses[borrow]
		};
		let mut ok: FxHashSet<Local> = iters.drain(..).collect();
		ok.retain(|&l| reads.get(&l).copied().unwrap_or(0) == self.uses[l]);
		ok.retain(|&l| made.contains(&l) || moves.iter().any(|&(_, to)| to == l));
		ok.retain(|&l| borrows.iter().all(|(&b, &from)| from != l || stepping(self, b)));
		// A move joins two iterators that must both step in place, or neither.
		loop {
			let before = ok.len();
			for &(from, to) in &moves {
				if ok.contains(&from) != ok.contains(&to) {
					ok.remove(&from);
					ok.remove(&to);
				}
			}
			if ok.len() == before {
				break;
			}
		}
		for (b, from) in borrows {
			if ok.contains(&from) {
				self.stepped_borrows.insert(b, from);
			}
		}
		self.stepped = ok;
	}

	/// The iterator a `next` receiver `op` borrows, if it steps in place.
	pub(super) fn stepped_recv(&self, op: &Operand<'tcx>) -> Option<Local> {
		let (Operand::Copy(p) | Operand::Move(p)) = op else {
			return None;
		};
		self.stepped_borrows.get(&p.as_local()?).copied()
	}

	/// `{ s, i: 0 }`, from what `into_iter` or `iter` made: `rt.iterSlice(s)`.
	pub(super) fn stepped_make(&mut self, value: Expr, span: rustc_span::Span) -> Expr {
		match value {
			Expr::Call(f, mut args) if matches!(&*f, Expr::Rt(name) if name == "iterSlice") && args.len() == 1 => {
				Expr::Object(vec![
					(Key::Name("s".into()), args.pop().unwrap()),
					(Key::Name("i".into()), Expr::lit("0")),
				])
			}
			// Refused already.
			Expr::Lit(lit) if lit == "undefined" => Expr::undefined(),
			_ => self.err(
				span,
				"an iterator stepped in place that its mapping didn't make from an array",
			),
		}
	}

	/// The next item of `it`, stepped in place, or `rt.DONE`.
	pub(super) fn stepped_next(&self, it: Expr) -> Expr {
		let s = it.clone().member("s");
		let i = it.member("i");
		Expr::Cond(
			Box::new(i.clone().binary("<", s.clone().member("length"))),
			Box::new(s.index(Expr::Postfix(Box::new(i), "++"))),
			Box::new(Expr::Rt("DONE".into())),
		)
	}

	/// The type of a local stepped in place: its array and index.
	pub(super) fn stepped_ty(&mut self, local: Local) -> Option<Type> {
		if !self.stepped.contains(&local) {
			return None;
		}
		let item = array_iter(self.tcx, self.mir.local_decls[local].ty)?;
		let array = self.b.t.ty(Ty::new_slice(self.tcx, item), self.span);
		Some(Type::Object(vec![
			("s".into(), array),
			("i".into(), Type::Builtin("number")),
		]))
	}
}
