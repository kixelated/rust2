//! Unwinding: what a panic drops on its way out, as `-C panic=unwind` MIR has it.
//!
//! Each call, assert, and drop that can panic names the cleanup block it unwinds to,
//! which drops what is live there and resumes unwinding. A function with any cleanup
//! that does something JS can see holds the cleanup of the code running now in a
//! variable, set before each terminator that can panic where it changes, and catches
//! what is thrown: the catch runs that cleanup, then throws it on.

use rustc_data_structures::fx::FxHashSet;
use rustc_middle::mir::{self, BasicBlock, StatementKind, TerminatorKind, UnwindAction};

use super::Fx;
use super::cfg::{self, Exit};
use crate::ts::{Expr, Stmt, StmtKind, Type};

/// A function's unwinding.
pub struct Unwind {
	/// The variable holding the cleanup to run: a block's index plus one, or 0 for none.
	pub var: String,
	/// What it holds now, where the code emitted knows.
	pub known: Option<usize>,
	/// The cleanup blocks that do something JS can see, from where unwinding enters them.
	visible: FxHashSet<BasicBlock>,
	/// The caught exception's name.
	pub caught: String,
}

impl<'tcx> Fx<'_, '_, '_, 'tcx> {
	/// Find the cleanups that do something JS can see, and lower with them if any do.
	pub(super) fn plan_unwind(&mut self) {
		let mir = self.mir;
		let mut visible = FxHashSet::default();
		for data in mir.basic_blocks.iter() {
			if data.is_cleanup {
				continue;
			}
			if let Some(UnwindAction::Cleanup(entry)) = data.terminator().unwind()
				&& self.cleanup_visible(*entry)
			{
				visible.insert(*entry);
			}
		}
		if visible.is_empty() {
			return;
		}
		let var = self.b.fresh("unwind");
		let caught = self.b.fresh("e");
		self.unwind = Some(Unwind {
			var,
			known: Some(0),
			visible,
			caught,
		});
	}

	/// Whether the cleanup entered at `entry` drops or changes anything JS can see.
	fn cleanup_visible(&mut self, entry: BasicBlock) -> bool {
		let mir = self.mir;
		let mut seen = FxHashSet::default();
		let mut stack = vec![entry];
		while let Some(bb) = stack.pop() {
			if !seen.insert(bb) {
				continue;
			}
			let data = &mir.basic_blocks[bb];
			if data
				.statements
				.iter()
				.any(|s| matches!(s.kind, StatementKind::SetDiscriminant { .. }))
			{
				return true;
			}
			let term = data.terminator();
			match &term.kind {
				TerminatorKind::Drop { place, .. } => {
					let ty = place.ty(&mir.local_decls, self.tcx).ty;
					if self.b.t.drops(ty) {
						return true;
					}
				}
				TerminatorKind::Call { .. } => return true,
				_ => {}
			}
			stack.extend(term.successors());
		}
		false
	}

	/// Before a block whose terminator can panic: point the cleanup variable at what it
	/// unwinds to, if that changed.
	pub(super) fn set_unwind(&mut self, term: &mir::Terminator<'tcx>, out: &mut Vec<Stmt>) {
		let Some(unwind) = &self.unwind else {
			return;
		};
		let can_panic = match &term.kind {
			TerminatorKind::Call { .. } | TerminatorKind::Assert { .. } => true,
			TerminatorKind::Drop { place, .. } => {
				let ty = place.ty(&self.mir.local_decls, self.tcx).ty;
				self.b.t.drops(ty)
			}
			_ => false,
		};
		if !can_panic {
			return;
		}
		let target = match term.unwind() {
			Some(UnwindAction::Cleanup(entry)) if unwind.visible.contains(entry) => entry.as_usize() + 1,
			_ => 0,
		};
		if unwind.known == Some(target) {
			return;
		}
		let var = unwind.var.clone();
		let stmt = StmtKind::Expr(Expr::ident(&var).assign(Expr::lit(target.to_string()))).at(None);
		// What is held back that can panic runs first, under the cleanup it had.
		self.emit_barrier(stmt, out);
		if let Some(unwind) = &mut self.unwind {
			unwind.known = Some(target);
		}
	}

	/// Wrap `body` in the catch that runs the cleanup the variable names, then throws on.
	pub(super) fn catch(&mut self, body: Vec<Stmt>) -> Vec<Stmt> {
		let Some(unwind) = self.unwind.take() else {
			return body;
		};
		let mir = self.mir;
		// The cleanup blocks, numbered after a first node that dispatches on the variable.
		let mut nodes: Vec<BasicBlock> = Vec::new();
		let mut index = rustc_index::IndexVec::<BasicBlock, Option<usize>>::from_elem_n(None, mir.basic_blocks.len());
		let mut entries: Vec<BasicBlock> = unwind.visible.iter().copied().collect();
		entries.sort();
		let mut stack = entries.clone();
		while let Some(bb) = stack.pop() {
			if index[bb].is_some() {
				continue;
			}
			index[bb] = Some(nodes.len() + 1);
			nodes.push(bb);
			stack.extend(mir.basic_blocks[bb].terminator().successors());
		}
		let mut succs = vec![entries.iter().map(|e| index[*e].unwrap()).collect::<Vec<_>>()];
		for &bb in &nodes {
			succs.push(
				mir.basic_blocks[bb]
					.terminator()
					.successors()
					.map(|s| index[s].unwrap())
					.collect(),
			);
		}
		let graph = cfg::Graph { succs };
		self.in_cleanup = Some(unwind.caught.clone());
		let mut lowering = Cleanup {
			fx: self,
			nodes: &nodes,
			index: &index,
			entries: &entries,
			var: &unwind.var,
		};
		let mut labels = lowering.fx.labels;
		let handler = cfg::structure(&graph, &mut lowering, &mut labels);
		self.labels = labels;
		self.in_cleanup = None;
		vec![
			StmtKind::decl(true, unwind.var.clone(), Some(Type::Builtin("number")), Some(Expr::lit("0"))).at(None),
			StmtKind::Catch {
				body,
				name: unwind.caught,
				handler,
			}
			.at(None),
		]
	}
}

/// The cleanup blocks, lowered as nodes after one that dispatches on the variable.
struct Cleanup<'c, 'm, 'b, 'a, 'tcx> {
	fx: &'c mut Fx<'m, 'b, 'a, 'tcx>,
	nodes: &'c [BasicBlock],
	index: &'c rustc_index::IndexVec<BasicBlock, Option<usize>>,
	entries: &'c [BasicBlock],
	var: &'c str,
}

impl cfg::Lower for Cleanup<'_, '_, '_, '_, '_> {
	fn node(&mut self, node: usize, out: &mut Vec<Stmt>) -> Exit {
		if node == 0 {
			// No cleanup: throw on at once.
			let caught = self.fx.in_cleanup.clone().unwrap_or_default();
			out.push(
				StmtKind::If {
					cond: Expr::ident(self.var).binary("===", Expr::lit("0")),
					then: vec![StmtKind::Throw(Expr::ident(&caught)).at(None)],
					els: Vec::new(),
				}
				.at(None),
			);
			let mut arms: Vec<(Expr, usize)> = self
				.entries
				.iter()
				.map(|e| {
					let test = Expr::ident(self.var).binary("===", Expr::lit((e.as_usize() + 1).to_string()));
					(test, self.index[*e].unwrap())
				})
				.collect();
			let (_, last) = arms.pop().expect("a cleanup JS can see");
			return match arms.is_empty() {
				true => Exit::Goto(last),
				false => Exit::Branch(arms, last),
			};
		}
		let bb = self.nodes[node - 1];
		match self.fx.block_mapped(bb, out) {
			Exit::Goto(target) => Exit::Goto(self.index[BasicBlock::from_usize(target)].unwrap()),
			Exit::Branch(arms, default) => Exit::Branch(
				arms.into_iter()
					.map(|(c, t)| (c, self.index[BasicBlock::from_usize(t)].unwrap()))
					.collect(),
				self.index[BasicBlock::from_usize(default)].unwrap(),
			),
			Exit::End => Exit::End,
		}
	}

	fn flush(&mut self, out: &mut Vec<Stmt>) {
		cfg::Lower::flush(self.fx, out);
	}
}
