//! Structured control flow from a control-flow graph, as JS needs it.
//!
//! A reducible graph, which every function without `.await` has, becomes nested blocks,
//! loops, and `if`s, left by labeled `break` and `continue`: Ramsey's "Beyond Relooper"
//! (ICFP 2022), which places each node under its immediate dominator. An irreducible
//! graph, such as a coroutine's resume function, which enters loops in the middle,
//! becomes a loop over a `switch` on the next node, with each chain of nodes that have
//! one predecessor in one case.

use crate::ts::{Expr, Stmt, StmtKind};

/// How a node leaves.
pub enum Exit {
	/// To one successor.
	Goto(usize),
	/// To the first target whose condition holds, else to the last.
	Branch(Vec<(Expr, usize)>, usize),
	/// Nowhere: the node returned, or can't finish.
	End,
}

/// What emits each node.
pub trait Lower {
	/// Emit `node`'s statements into `out`, and say how it leaves. A branch is decided
	/// from what `out` already holds.
	fn node(&mut self, node: usize, out: &mut Vec<Stmt>) -> Exit;
	/// Control is about to leave the statements emitted so far for code emitted
	/// elsewhere: emit whatever the node lowering held back.
	fn flush(&mut self, out: &mut Vec<Stmt>);
}

/// A control-flow graph, entered at node 0.
pub struct Graph {
	pub succs: Vec<Vec<usize>>,
}

/// The graph's nodes in order, its dominators, and its shape.
struct Shape {
	/// Each node's place in reverse postorder, or `usize::MAX` if unreachable.
	rpo: Vec<usize>,
	/// The reachable nodes in reverse postorder.
	order: Vec<usize>,
	idom: Vec<usize>,
	/// Each node's dominator-tree children, in reverse postorder.
	children: Vec<Vec<usize>>,
	/// How many reachable predecessors reach each node by a forward edge.
	forward: Vec<usize>,
	/// Whether a back edge targets each node.
	header: Vec<bool>,
	/// Whether every back edge targets a node that dominates its source.
	reducible: bool,
}

impl Shape {
	fn new(g: &Graph) -> Self {
		let n = g.succs.len();
		// Postorder, iteratively.
		let mut post = Vec::with_capacity(n);
		let mut seen = vec![false; n];
		let mut stack = vec![(0usize, 0usize)];
		seen[0] = true;
		while let Some((node, i)) = stack.pop() {
			match g.succs[node].get(i) {
				Some(&next) => {
					stack.push((node, i + 1));
					if !seen[next] {
						seen[next] = true;
						stack.push((next, 0));
					}
				}
				None => post.push(node),
			}
		}
		let order: Vec<usize> = post.into_iter().rev().collect();
		let mut rpo = vec![usize::MAX; n];
		for (i, &node) in order.iter().enumerate() {
			rpo[node] = i;
		}
		let mut preds = vec![Vec::new(); n];
		for &node in &order {
			for &next in &g.succs[node] {
				preds[next].push(node);
			}
		}
		// Cooper, Harvey, and Kennedy's iterative dominators.
		let mut idom = vec![usize::MAX; n];
		idom[0] = 0;
		let intersect = |idom: &[usize], mut a: usize, mut b: usize| {
			while a != b {
				while rpo[a] > rpo[b] {
					a = idom[a];
				}
				while rpo[b] > rpo[a] {
					b = idom[b];
				}
			}
			a
		};
		let mut changed = true;
		while changed {
			changed = false;
			for &node in order.iter().skip(1) {
				let mut new = usize::MAX;
				for &p in &preds[node] {
					if idom[p] == usize::MAX {
						continue;
					}
					new = match new {
						usize::MAX => p,
						cur => intersect(&idom, p, cur),
					};
				}
				if new != idom[node] {
					idom[node] = new;
					changed = true;
				}
			}
		}
		let mut children = vec![Vec::new(); n];
		for &node in order.iter().skip(1) {
			children[idom[node]].push(node);
		}
		let dominates = |a: usize, mut b: usize| loop {
			if a == b {
				return true;
			}
			if b == 0 {
				return false;
			}
			b = idom[b];
		};
		let mut forward = vec![0; n];
		let mut header = vec![false; n];
		let mut reducible = true;
		for &node in &order {
			for &next in &g.succs[node] {
				if rpo[next] > rpo[node] {
					forward[next] += 1;
				} else {
					header[next] = true;
					reducible &= dominates(next, node);
				}
			}
		}
		Self {
			rpo,
			order,
			idom,
			children,
			forward,
			header,
			reducible,
		}
	}

	fn merge(&self, node: usize) -> bool {
		self.forward[node] > 1
	}
}

/// The structured statements of `g`, each node emitted by `lower`.
pub fn structure(g: &Graph, lower: &mut impl Lower, labels: &mut u32) -> Vec<Stmt> {
	let shape = Shape::new(g);
	let mut out = Vec::new();
	match shape.reducible {
		true => {
			let mut s = Ramsey {
				shape: &shape,
				lower,
				labels: vec![None; g.succs.len()],
				next: labels,
			};
			s.tree(0, &mut out);
		}
		false => dispatch(g, &shape, lower, labels, &mut out),
	}
	// Dropping a jump can leave a branch empty, which the next pass inverts.
	simplify(&mut out);
	simplify(&mut out);
	out
}

struct Ramsey<'s, L> {
	shape: &'s Shape,
	lower: &'s mut L,
	/// The label of each node's loop or of the block it follows.
	labels: Vec<Option<String>>,
	next: &'s mut u32,
}

impl<L: Lower> Ramsey<'_, L> {
	fn label(&mut self, node: usize, prefix: &str) -> String {
		if let Some(label) = &self.labels[node] {
			return label.clone();
		}
		let label = format!("{prefix}{}", *self.next);
		*self.next += 1;
		self.labels[node] = Some(label.clone());
		label
	}

	/// `x` and what it dominates.
	fn tree(&mut self, x: usize, out: &mut Vec<Stmt>) {
		// The merge nodes it dominates, the last in reverse postorder outermost.
		let mut merges: Vec<usize> = self.shape.children[x]
			.iter()
			.copied()
			.filter(|&c| self.shape.merge(c))
			.collect();
		merges.sort_by_key(|&c| std::cmp::Reverse(self.shape.rpo[c]));
		match self.shape.header[x] {
			true => {
				let label = self.label(x, "l");
				let mut body = Vec::new();
				self.within(x, &merges, &mut body);
				out.push(
					StmtKind::While {
						label: Some(label),
						cond: Expr::lit("true"),
						body,
					}
					.at(None),
				);
			}
			false => self.within(x, &merges, out),
		}
	}

	fn within(&mut self, x: usize, merges: &[usize], out: &mut Vec<Stmt>) {
		if let [y, rest @ ..] = merges {
			let label = self.label(*y, "b");
			let mut body = Vec::new();
			self.within(x, rest, &mut body);
			out.push(
				StmtKind::Block {
					label: Some(label),
					body,
				}
				.at(None),
			);
			self.tree(*y, out);
			return;
		}
		match self.lower.node(x, out) {
			Exit::Goto(target) => self.branch(x, target, out),
			Exit::Branch(arms, default) => {
				let mut els = Vec::new();
				self.branch(x, default, &mut els);
				for (cond, target) in arms.into_iter().rev() {
					let mut then = Vec::new();
					self.branch(x, target, &mut then);
					els = vec![StmtKind::If { cond, then, els }.at(None)];
				}
				out.extend(els);
			}
			Exit::End => self.lower.flush(out),
		}
	}

	fn branch(&mut self, from: usize, to: usize, out: &mut Vec<Stmt>) {
		if self.shape.rpo[to] <= self.shape.rpo[from] {
			self.lower.flush(out);
			let label = self.label(to, "l");
			out.push(StmtKind::Continue(Some(label)).at(None));
		} else if self.shape.merge(to) {
			self.lower.flush(out);
			let label = self.label(to, "b");
			out.push(StmtKind::Break(Some(label)).at(None));
		} else {
			debug_assert_eq!(self.shape.idom[to], from);
			self.tree(to, out);
		}
	}
}

/// `let n = 0; for (;;) switch (n) { ... }`: each node with other than one predecessor is a
/// case, which the chain of nodes after it with one predecessor follows in place.
fn dispatch(g: &Graph, shape: &Shape, lower: &mut impl Lower, labels: &mut u32, out: &mut Vec<Stmt>) {
	let label = format!("l{labels}");
	let var = format!("bb{labels}");
	*labels += 1;
	let n = shape.rpo.len();
	let mut preds = vec![0usize; n];
	let mut case = vec![false; n];
	case[0] = true;
	for &node in &shape.order {
		for &next in &g.succs[node] {
			preds[next] += 1;
		}
	}
	for &node in &shape.order {
		if preds[node] != 1 {
			case[node] = true;
		}
	}
	let mut d = Dispatch {
		lower,
		case: &case,
		label: &label,
		var: &var,
	};
	let mut cases = Vec::new();
	for &node in &shape.order {
		if !case[node] {
			continue;
		}
		let mut body = Vec::new();
		d.chain(node, &mut body);
		cases.push((Expr::lit(node.to_string()), body));
	}
	out.push(
		StmtKind::decl(
			true,
			var.clone(),
			Some(crate::ts::Type::Builtin("number")),
			Some(Expr::lit("0")),
		)
		.at(None),
	);
	out.push(
		StmtKind::While {
			label: Some(label.clone()),
			cond: Expr::lit("true"),
			body: vec![
				StmtKind::Switch {
					label: None,
					disc: Expr::ident(&var),
					cases,
					default: Vec::new(),
				}
				.at(None),
			],
		}
		.at(None),
	);
}

struct Dispatch<'d, L> {
	lower: &'d mut L,
	case: &'d [bool],
	label: &'d str,
	var: &'d str,
}

impl<L: Lower> Dispatch<'_, L> {
	fn chain(&mut self, node: usize, out: &mut Vec<Stmt>) {
		match self.lower.node(node, out) {
			Exit::Goto(target) => self.jump(target, out),
			Exit::Branch(arms, default) => {
				let mut els = Vec::new();
				self.jump(default, &mut els);
				for (cond, target) in arms.into_iter().rev() {
					let mut then = Vec::new();
					self.jump(target, &mut then);
					els = vec![StmtKind::If { cond, then, els }.at(None)];
				}
				out.extend(els);
			}
			Exit::End => self.lower.flush(out),
		}
	}

	fn jump(&mut self, to: usize, out: &mut Vec<Stmt>) {
		if !self.case[to] {
			self.chain(to, out);
			return;
		}
		self.lower.flush(out);
		out.push(StmtKind::Expr(Expr::ident(self.var).assign(Expr::lit(to.to_string()))).at(None));
		out.push(StmtKind::Continue(Some(self.label.to_string())).at(None));
	}
}

/// Drop the jumps that only fall through, then the labels nothing targets.
pub fn simplify(stmts: &mut Vec<Stmt>) {
	for stmt in stmts.iter_mut() {
		match &mut stmt.kind {
			StmtKind::Block { label, body } => {
				simplify(body);
				if let Some(l) = label {
					strip_tail(body, &StmtKind::Break(Some(l.clone())));
				}
			}
			StmtKind::While { label, body, .. } => {
				simplify(body);
				if let Some(l) = label {
					strip_tail(body, &StmtKind::Continue(Some(l.clone())));
				}
			}
			StmtKind::If { then, els, .. } => {
				simplify(then);
				simplify(els);
			}
			StmtKind::Switch { cases, default, .. } => {
				cases.iter_mut().for_each(|(_, body)| simplify(body));
				simplify(default);
			}
			StmtKind::Try { body, finally } => {
				simplify(body);
				simplify(finally);
			}
			StmtKind::Catch { body, handler, .. } => {
				simplify(body);
				simplify(handler);
			}
			_ => {}
		}
	}
	// Unused labels, and blocks left without one.
	let mut i = 0;
	while i < stmts.len() {
		let unused = match &stmts[i].kind {
			StmtKind::Block { label: Some(l), body }
			| StmtKind::While {
				label: Some(l), body, ..
			} => !targets(body, l),
			_ => false,
		};
		if unused {
			match &mut stmts[i].kind {
				StmtKind::Block { label, .. } | StmtKind::While { label, .. } => *label = None,
				_ => unreachable!(),
			}
		}
		if let StmtKind::Block { label: None, .. } = &stmts[i].kind {
			let StmtKind::Block { body, .. } = std::mem::replace(&mut stmts[i].kind, StmtKind::Break(None)) else {
				unreachable!()
			};
			let len = body.len();
			stmts.splice(i..=i, body);
			i += len;
			continue;
		}
		// Nothing runs after a jump.
		if stmts[i].diverges() {
			stmts.truncate(i + 1);
			break;
		}
		i += 1;
	}
	// `if (c) {} else { x }` is `if (!c) { x }`.
	for stmt in stmts.iter_mut() {
		if let StmtKind::If { cond, then, els } = &mut stmt.kind
			&& then.is_empty()
			&& !els.is_empty()
		{
			*cond = std::mem::replace(cond, Expr::undefined()).not();
			std::mem::swap(then, els);
		}
	}
	stmts.retain(
		|s| !matches!(&s.kind, StmtKind::If { then, els, cond } if then.is_empty() && els.is_empty() && pure(cond)),
	);
}

/// Whether evaluating `e` does nothing but read.
pub fn pure(e: &Expr) -> bool {
	match e {
		Expr::Ident(_) | Expr::This | Expr::Sym(_) | Expr::Rt(_) | Expr::Lit(_) | Expr::Str(_) => true,
		Expr::Member(base, _) | Expr::Unary(_, base) | Expr::Located(_, base) | Expr::As(base, _) => pure(base),
		Expr::Binary(l, _, r) => pure(l) && pure(r),
		Expr::Cond(c, a, b) => pure(c) && pure(a) && pure(b),
		_ => false,
	}
}

/// Remove `jump` wherever it is the last thing `stmts` runs before falling off its end.
fn strip_tail(stmts: &mut Vec<Stmt>, jump: &StmtKind) {
	let Some(last) = stmts.last_mut() else {
		return;
	};
	match &mut last.kind {
		k if same_jump(k, jump) => {
			stmts.pop();
		}
		StmtKind::If { then, els, .. } => {
			strip_tail(then, jump);
			strip_tail(els, jump);
		}
		StmtKind::Block { body, .. } => strip_tail(body, jump),
		_ => {}
	}
}

fn same_jump(a: &StmtKind, b: &StmtKind) -> bool {
	match (a, b) {
		(StmtKind::Break(x), StmtKind::Break(y)) | (StmtKind::Continue(x), StmtKind::Continue(y)) => x == y,
		_ => false,
	}
}

/// Whether a `break` or `continue` in `stmts` names `label`.
fn targets(stmts: &[Stmt], label: &str) -> bool {
	stmts.iter().any(|stmt| match &stmt.kind {
		StmtKind::Break(Some(l)) | StmtKind::Continue(Some(l)) => l == label,
		StmtKind::If { then, els, .. } => targets(then, label) || targets(els, label),
		StmtKind::Block { body, .. }
		| StmtKind::While { body, .. }
		| StmtKind::ForRange { body, .. }
		| StmtKind::ForOf { body, .. } => targets(body, label),
		StmtKind::Try { body, finally } => targets(body, label) || targets(finally, label),
		StmtKind::Catch { body, handler, .. } => targets(body, label) || targets(handler, label),
		StmtKind::Switch { cases, default, .. } => {
			cases.iter().any(|(_, body)| targets(body, label)) || targets(default, label)
		}
		_ => false,
	})
}
