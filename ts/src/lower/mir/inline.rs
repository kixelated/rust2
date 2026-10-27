//! Each `const` read once, by the statement right after it, folded into that read: the
//! temporaries lowering leaves where MIR read a local twice, as a bounds check and an
//! index do, or where folding it as it lowered would have reordered reads.

use rustc_data_structures::fx::{FxHashMap, FxHashSet};

use super::{Read, scan, subst, visit_expr, visit_stmt};
use crate::ts::{Expr, Stmt, StmtKind};

/// Fold throughout `stmts`, a function's body, given the names of locals something
/// borrows, which a call can change.
pub(super) fn inline_temps(stmts: &mut Vec<Stmt>, shared: &FxHashSet<String>) {
	let mut names: FxHashMap<String, usize> = FxHashMap::default();
	for stmt in stmts.iter() {
		visit_stmt(stmt, &mut |e| {
			visit_expr(e, &mut |e| {
				if let Expr::Ident(name) = e {
					*names.entry(name.clone()).or_default() += 1;
				}
			})
		});
	}
	list(stmts, &names, shared);
}

fn list(stmts: &mut Vec<Stmt>, names: &FxHashMap<String, usize>, shared: &FxHashSet<String>) {
	for stmt in stmts.iter_mut() {
		match &mut stmt.kind {
			StmtKind::If { then, els, .. } => {
				list(then, names, shared);
				list(els, names, shared);
			}
			StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::Block { body, .. } => {
				list(body, names, shared)
			}
			StmtKind::Switch { cases, default, .. } => {
				cases.iter_mut().for_each(|(_, body)| list(body, names, shared));
				list(default, names, shared);
			}
			StmtKind::Catch { body, handler, .. } => {
				list(body, names, shared);
				list(handler, names, shared);
			}
			_ => {}
		}
	}
	// From the end, so a temporary folds into one that has folded into its reader.
	for i in (0..stmts.len().saturating_sub(1)).rev() {
		let StmtKind::Decl {
			mutable: false,
			name,
			ty,
			init: Some(value),
		} = &stmts[i].kind
		else {
			continue;
		};
		// A literal's type comes from its declaration: an array's may be a tuple's.
		let typed = ty.is_some() && matches!(super::step::peel(value), Expr::Array(_) | Expr::Object(_));
		if !typed
			&& names.get(name) == Some(&1)
			&& let Some(next) = eager(&stmts[i + 1])
			&& folds(value, name, next, shared)
		{
			let StmtKind::Decl {
				name,
				init: Some(value),
				..
			} = stmts.remove(i).kind
			else {
				unreachable!("matched above")
			};
			let next = eager_mut(&mut stmts[i]).expect("found above");
			subst(next, &name, &value);
		}
	}
}

/// What evaluating some code may do, as far as moving other code past it goes.
#[derive(Default)]
struct Does {
	/// Change something: a call, or an assignment.
	writes: bool,
	/// Panic without changing anything, as the runtime's arithmetic can.
	panics: bool,
	/// Read memory a call could change, or a local something borrows.
	reads: bool,
}

impl Does {
	fn of(events: &[Read], shared: &FxHashSet<String>) -> Self {
		let mut does = Self::default();
		for event in events {
			match event {
				Read::Effect => does.writes = true,
				Read::Panic => does.panics = true,
				Read::Memory | Read::Field(_) => does.reads = true,
				Read::Ident(name, _) => does.reads |= shared.contains(name.as_str()),
			}
		}
		does
	}
}

/// What `stmt` evaluates first, and always: its expression, an `if`'s condition, a loop's
/// array, or a block's first statement's.
fn eager(stmt: &Stmt) -> Option<&Expr> {
	match &stmt.kind {
		StmtKind::Expr(e) | StmtKind::Return(Some(e)) | StmtKind::Throw(e) => Some(e),
		StmtKind::Decl { init: Some(e), .. } | StmtKind::If { cond: e, .. } | StmtKind::ForOf { iter: e, .. } => {
			Some(e)
		}
		StmtKind::Block { body, .. } => body.first().and_then(eager),
		_ => None,
	}
}

fn eager_mut(stmt: &mut Stmt) -> Option<&mut Expr> {
	match &mut stmt.kind {
		StmtKind::Expr(e) | StmtKind::Return(Some(e)) | StmtKind::Throw(e) => Some(e),
		StmtKind::Decl { init: Some(e), .. } | StmtKind::If { cond: e, .. } | StmtKind::ForOf { iter: e, .. } => {
			Some(e)
		}
		StmtKind::Block { body, .. } => body.first_mut().and_then(eager_mut),
		_ => None,
	}
}

/// Whether `value`, the temporary `name`'s, can be evaluated where `next` reads it
/// instead of before `next`: the read always runs, and what `next` evaluates first
/// neither changes what `value` sees nor is changed or reordered by it.
fn folds(value: &Expr, name: &str, next: &Expr, shared: &FxHashSet<String>) -> bool {
	let mut events = Vec::new();
	scan(next, &mut events, false);
	let Some(at) = events.iter().position(|r| matches!(r, Read::Ident(n, _) if n == name)) else {
		return false;
	};
	if matches!(&events[at], Read::Ident(_, true)) {
		return false;
	}
	let before = Does::of(&events[..at], shared);
	let mut own = Vec::new();
	scan(value, &mut own, false);
	let it = Does::of(&own, shared);
	// A local one reads that the other assigns.
	let crossed = |events: &[Read], assigns: &dyn Fn(&str) -> bool| {
		events.iter().any(|r| matches!(r, Read::Ident(n, _) if assigns(n)))
	};
	let assigned = (before.writes && crossed(&own, &|n| expr_assigns(next, n)))
		|| (it.writes && crossed(&events[..at], &|n| expr_assigns(value, n)));
	!assigned
		&& !(before.writes && (it.writes || it.panics || it.reads))
		&& !(it.writes && (before.writes || before.panics || before.reads))
		&& !(before.panics && it.panics)
}

/// Whether `e` assigns the variable `name` whole.
fn expr_assigns(e: &Expr, name: &str) -> bool {
	let mut found = false;
	visit_expr(e, &mut |e| {
		if let Expr::Assign(target, _) | Expr::Postfix(target, _) = e {
			found |= matches!(&**target, Expr::Ident(n) if n == name);
		}
	});
	found
}
