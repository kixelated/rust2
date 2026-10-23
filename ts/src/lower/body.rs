//! Function bodies: THIR expressions to TypeScript statements and expressions.
//!
//! Rust is all expressions and JS is not, so each expression lowers either for its
//! value (`expr`), which may first push statements, or into a destination (`stmt`),
//! which lets control flow return or assign from each branch directly. Statements
//! pushed ahead of a value keep Rust's evaluation order: an earlier operand with an
//! effect is saved to a temporary first.

use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::{HirId, MatchSource};
use rustc_middle::middle::region;
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::{self, ArmId, BlockId, ExprId, ExprKind, LocalVarId, Pat, PatKind, StmtId, Thir};
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::{self, GenericArgsRef, Instance, Ty};
use rustc_span::{Span, Symbol};

use super::library::{self, Args};
use super::{AdtRepr, Translator, camel, escape};
use crate::program::SymKind;
use crate::ts::{self, Expr, Key, Param, Stmt, StmtKind, Target, Type};

/// Translate one function instance's body.
pub fn function<'a, 'tcx>(
	t: &mut Translator<'a, 'tcx>,
	instance: Instance<'tcx>,
	thir: &'a Thir<'tcx>,
	expr: ExprId,
	method: bool,
) -> Option<ts::Function> {
	let tcx = t.tcx;
	let def_id = instance.def_id();
	let sig = tcx.instantiate_bound_regions_with_erased(tcx.fn_sig(def_id).skip_binder());
	let sig = t.subst(instance.args, sig);
	let mut b = Body {
		t,
		thir,
		args: instance.args,
		locals: FxHashMap::default(),
		names: FxHashSet::default(),
		breakables: Vec::new(),
		ret: sig.output(),
	};
	let span = tcx.def_span(def_id);
	let mut body = Vec::new();
	let mut params = Vec::new();
	for (i, param) in thir.params.iter().enumerate() {
		let ty = b.subst(param.ty);
		let this = i == 0 && param.self_kind.is_some();
		match (&param.pat, this) {
			(Some(pat), true) => {
				let PatKind::Binding { var, .. } = pat.kind else {
					b.t.err(pat.span, "unsupported `self` pattern");
					continue;
				};
				let name = if method { "this" } else { "self" };
				b.names.insert(name.to_string());
				b.locals.insert(var.0, Local::Name {
					name: name.to_string(),
					mutable: false,
				});
				if !method {
					let ty = b.t.ty(ty, pat.span);
					params.push(Param {
						name: name.to_string(),
						ty: Some(ty),
					});
				}
			}
			(Some(pat), false) => {
				let name = b.param(pat, &mut body);
				let ty = b.t.ty(ty, pat.span);
				params.push(Param { name, ty: Some(ty) });
			}
			(None, _) => {
				let name = b.fresh("arg");
				let ty = b.t.ty(ty, span);
				params.push(Param { name, ty: Some(ty) });
			}
		}
	}
	let ret = b.ret;
	let dest = match ret.is_unit() || ret.is_never() {
		true => Dest::Discard,
		false => Dest::Return,
	};
	b.stmt(expr, dest, &mut body);
	let ret = b.t.ret_ty(ret, span);
	Some(ts::Function {
		sym: 0,
		method,
		params,
		ret,
		body,
		loc: None,
		doc: None,
	})
}

/// Where a lowered value goes.
#[derive(Clone, Debug)]
pub enum Dest {
	/// Nowhere: only its effects run.
	Discard,
	/// Returned from the function or closure.
	Return,
	/// Assigned to a declared variable.
	Assign(String),
}

#[derive(Clone)]
enum Local {
	Name { name: String, mutable: bool },
	/// A function item, which is a type and no value: calls name it statically.
	FnItem,
}

/// A loop or labeled block that `break` can target.
struct Breakable {
	scope: region::Scope,
	label: String,
	used: bool,
	dest: Dest,
	is_loop: bool,
}

pub struct Body<'b, 'a, 'tcx> {
	pub t: &'b mut Translator<'a, 'tcx>,
	pub thir: &'a Thir<'tcx>,
	pub args: GenericArgsRef<'tcx>,
	locals: FxHashMap<HirId, Local>,
	/// Every name in the function, closures included, so none shadows another.
	names: FxHashSet<String>,
	breakables: Vec<Breakable>,
	/// The return type of the function or closure being lowered.
	ret: Ty<'tcx>,
}

/// One `&&`-joined part of an `if` condition.
enum CondPart {
	Bool(ExprId),
	Let(ExprId, &'static Pat<'static>),
}

impl<'b, 'a, 'tcx> Body<'b, 'a, 'tcx> {
	pub fn subst<T: ty::TypeFoldable<ty::TyCtxt<'tcx>>>(&self, value: T) -> T {
		self.t.subst(self.args, value)
	}

	pub fn ty_of(&self, id: ExprId) -> Ty<'tcx> {
		self.subst(self.thir[id].ty)
	}

	pub fn span(&self, id: ExprId) -> Span {
		self.thir[id].span
	}

	pub fn err(&mut self, span: Span, msg: impl Into<String>) -> Expr {
		self.t.err(span, msg);
		Expr::undefined()
	}

	/// A name no other binding in the function has.
	pub fn fresh(&mut self, base: &str) -> String {
		let base = escape(base);
		let mut name = base.clone();
		let mut n = 2;
		while !self.names.insert(name.clone()) {
			name = format!("{base}{n}");
			n += 1;
		}
		name
	}

	/// A temporary named after what it holds.
	pub fn temp_for(&mut self, id: ExprId) -> String {
		let base = self.temp_base(id);
		self.fresh(&base)
	}

	fn temp_base(&self, id: ExprId) -> String {
		match &self.thir[id].kind {
			ExprKind::Scope { value, .. } | ExprKind::Borrow { arg: value, .. } => {
				self.temp_base(*value)
			}
			ExprKind::Call { ty, .. } => match self.subst(*ty).kind() {
				ty::FnDef(def_id, _) => camel(self.t.tcx.item_name(*def_id).as_str()),
				_ => "value".into(),
			},
			_ => match self.ty_of(id).kind() {
				ty::Adt(adt, _) => {
					let name = self.t.tcx.item_name(adt.did()).to_string();
					let mut chars = name.chars();
					chars.next().map_or(name.clone(), |c| c.to_lowercase().chain(chars).collect())
				}
				_ => "value".into(),
			},
		}
	}

	/// Declare a binding for a Rust variable.
	pub(super) fn bind_var(&mut self, var: LocalVarId, name: Symbol, mutable: bool) -> String {
		let name = self.fresh(&camel(name.as_str()));
		self.locals.insert(var.0, Local::Name {
			name: name.clone(),
			mutable,
		});
		name
	}

	/// A parameter's name, declaring its pattern's bindings at the top of the body.
	fn param(&mut self, pat: &Pat<'tcx>, out: &mut Vec<Stmt>) -> String {
		match &pat.kind {
			PatKind::Binding {
				name,
				var,
				mode,
				subpattern: None,
				..
			} if mode.0 == rustc_hir::ByRef::No => {
				let mutable = mode.1 == rustc_ast::Mutability::Mut;
				self.bind_var(*var, *name, mutable)
			}
			PatKind::Wild => self.fresh("_"),
			_ => {
				let name = self.fresh("arg");
				let scrut = Expr::ident(&name);
				if self.pat_test(pat, &scrut).is_some() {
					self.t.err(pat.span, "a refutable parameter pattern");
				}
				self.pat_bind(pat, &scrut, false, out);
				name
			}
		}
	}

	fn local(&mut self, var: HirId, span: Span) -> Expr {
		match self.locals.get(&var) {
			Some(Local::Name { name, .. }) if name == "this" => Expr::This,
			Some(Local::Name { name, .. }) => Expr::ident(name),
			Some(Local::FnItem) => self.err(span, "a function item read as a value"),
			None => self.err(span, "a variable from outside the function"),
		}
	}

	// Statements

	fn push(&mut self, out: &mut Vec<Stmt>, kind: StmtKind, span: Span) {
		let loc = self.t.loc(span);
		out.push(kind.at(loc));
	}

	/// Deliver an already-lowered value to `dest`.
	fn deliver(&mut self, value: Expr, dest: &Dest, span: Span, out: &mut Vec<Stmt>) {
		match dest {
			Dest::Discard => {
				if !value.is_path() {
					self.push(out, StmtKind::Expr(value), span);
				}
			}
			Dest::Return => self.push(out, StmtKind::Return(Some(value.typed())), span),
			Dest::Assign(name) => self.push(out, StmtKind::Expr(Expr::ident(name).assign(value.typed())), span),
		}
	}

	/// Lower `id` so its value goes to `dest`.
	pub fn stmt(&mut self, id: ExprId, dest: Dest, out: &mut Vec<Stmt>) {
		let e = &self.thir[id];
		let span = e.span;
		let ty = self.ty_of(id);
		if self.t.ignored_macro(span) {
			if !ty.is_unit() {
				self.t.err(span, "an ignored macro must evaluate to `()`");
			}
			return;
		}
		let dest = match ty.is_unit() || ty.is_never() {
			true => Dest::Discard,
			false => dest,
		};
		match &e.kind {
			ExprKind::Scope { value, region_scope, .. } => match self.thir[*value].kind {
				ExprKind::Loop { body } => self.lp(*region_scope, body, dest, out),
				_ => self.stmt(*value, dest, out),
			},
			ExprKind::ValueTypeAscription { source, .. } => self.stmt(*source, dest, out),
			ExprKind::NeverToAny { source } => self.stmt(*source, Dest::Discard, out),
			ExprKind::Block { block } => self.block(*block, dest, out),
			ExprKind::If { cond, then, else_opt, .. } => self.if_(*cond, *then, *else_opt, dest, span, out),
			ExprKind::Match {
				scrutinee,
				arms,
				match_source,
			} => self.match_(id, *scrutinee, arms, *match_source, dest, out),
			ExprKind::Loop { body } => {
				self.err(span, format!("a loop outside a scope: {body:?}"));
			}
			ExprKind::Break { label, value } => self.break_(*label, *value, span, out),
			ExprKind::Continue { label } => {
				let label = self.target(*label, true);
				self.push(out, StmtKind::Continue(label), span);
			}
			ExprKind::Return { value } => {
				match value {
					Some(value) if !self.ty_of(*value).is_unit() => self.stmt(*value, Dest::Return, out),
					Some(value) => {
						self.stmt(*value, Dest::Discard, out);
						self.push(out, StmtKind::Return(None), span);
					}
					None => self.push(out, StmtKind::Return(None), span),
				}
				// `stmt` returns from each branch of the value.
				if value.is_some_and(|v| !self.ty_of(v).is_unit()) && !ts::diverges(out) {
					self.push(out, StmtKind::Return(None), span);
				}
			}
			ExprKind::Assign { lhs, rhs } => self.assign(*lhs, *rhs, span, out),
			ExprKind::AssignOp { op, lhs, rhs } => self.assign_op(*op, *lhs, *rhs, span, out),
			_ => {
				let value = self.expr(id, out);
				self.deliver(value, &dest, span, out);
			}
		}
	}

	fn block(&mut self, id: BlockId, dest: Dest, out: &mut Vec<Stmt>) {
		let block = &self.thir[id];
		if block.targeted_by_break {
			let label = self.fresh("block");
			self.breakables.push(Breakable {
				scope: block.region_scope,
				label,
				used: false,
				dest: dest.clone(),
				is_loop: false,
			});
			let mut body = Vec::new();
			self.block_body(id, dest, &mut body);
			let breakable = self.breakables.pop().unwrap();
			let label = breakable.used.then_some(breakable.label);
			let span = block.span;
			self.push(out, StmtKind::Block { label, body }, span);
			return;
		}
		self.block_body(id, dest, out);
	}

	fn block_body(&mut self, id: BlockId, dest: Dest, out: &mut Vec<Stmt>) {
		let block = &self.thir[id];
		for &stmt in &block.stmts {
			self.stmt_item(stmt, out);
		}
		if let Some(expr) = block.expr {
			self.stmt(expr, dest, out);
		}
	}

	fn stmt_item(&mut self, id: StmtId, out: &mut Vec<Stmt>) {
		match &self.thir[id].kind {
			thir::StmtKind::Expr { expr, .. } => self.stmt(*expr, Dest::Discard, out),
			thir::StmtKind::Let {
				pattern,
				initializer,
				else_block,
				span,
				..
			} => self.let_(pattern, *initializer, *else_block, *span, out),
		}
	}

	fn let_(&mut self, pat: &'a Pat<'tcx>, init: Option<ExprId>, els: Option<BlockId>, span: Span, out: &mut Vec<Stmt>) {
		let pat_ty = self.subst(pat.ty);
		if let Some(init) = init
			&& self.t.ignored_macro(self.span(init))
		{
			self.t.err(span, "a binding of an ignored macro");
			return;
		}
		// A simple binding.
		if let PatKind::Binding {
			name,
			var,
			mode,
			subpattern: None,
			..
		} = &pat.kind
			&& els.is_none()
		{
			if matches!(pat_ty.kind(), ty::FnDef(..)) {
				if let Some(init) = init
					&& !self.simple(init)
				{
					self.stmt(init, Dest::Discard, out);
				}
				self.locals.insert(var.0, Local::FnItem);
				return;
			}
			if mode.0 != rustc_hir::ByRef::No {
				self.ref_binding(mode.0, pat_ty, pat.span);
			}
			let mutable = mode.1 == rustc_ast::Mutability::Mut;
			let ty = self.t.ty(pat_ty, pat.span);
			let Some(init) = init else {
				let name = self.bind_var(*var, *name, true);
				self.push(out, StmtKind::decl(true, name, Some(ty), None), span);
				return;
			};
			if self.needs_dest(init) {
				let name = self.bind_var(*var, *name, true);
				self.push(
					out,
					StmtKind::decl(true, name.clone(), Some(ty), None),
					span,
				);
				self.stmt(init, Dest::Assign(name), out);
				return;
			}
			let value = self.operand(init, out);
			let name = self.bind_var(*var, *name, mutable);
			let annotate = mutable || needs_annotation(&value);
			self.push(
				out,
				StmtKind::decl(mutable, name, annotate.then_some(ty), Some(value)),
				span,
			);
			return;
		}

		let Some(init) = init else {
			self.t.err(span, "a pattern binding without an initializer");
			return;
		};
		// `const [a, b] = ...` for a tuple or array of plain bindings.
		if els.is_none() && self.destructurable(pat, true) {
			let value = self.operand(init, out);
			let mut mutable = false;
			let target = self.destructure(pat, &mut mutable);
			self.push(
				out,
				StmtKind::Decl {
					mutable,
					target,
					ty: None,
					init: Some(value),
				},
				span,
			);
			return;
		}

		let owned = !self.is_place(init);
		let value = match owned {
			true => self.operand(init, out),
			false => self.place(init, out),
		};
		let scrut = self.path(value, init, out);
		if let Some(els) = els {
			match self.pat_test(pat, &scrut) {
				Some(test) => {
					let mut body = Vec::new();
					self.block(els, Dest::Discard, &mut body);
					self.push(out, StmtKind::If { cond: test.not(), then: body, els: Vec::new() }, span);
				}
				None => self.t.err(span, "`let ... else` with an irrefutable pattern"),
			}
		} else if self.pat_test(pat, &scrut).is_some() {
			self.t.err(span, "a refutable pattern in `let`");
		}
		self.pat_bind(pat, &scrut, owned, out);
	}

	/// Whether `pat` is a tuple or array of plain bindings, which JS destructures.
	fn destructurable(&self, pat: &Pat<'tcx>, top: bool) -> bool {
		let ty = self.subst(pat.ty);
		match &pat.kind {
			PatKind::Deref { subpattern, .. } => self.destructurable(subpattern, top),
			PatKind::Leaf { subpatterns } if matches!(ty.kind(), ty::Tuple(_)) => {
				// Every field, in order.
				let arity = match ty.kind() {
					ty::Tuple(tys) => tys.len(),
					_ => unreachable!(),
				};
				subpatterns.len() == arity
					&& subpatterns.iter().enumerate().all(|(i, f)| f.field.index() == i && self.destructurable(&f.pattern, false))
			}
			PatKind::Array {
				prefix,
				slice: None,
				suffix,
			} => suffix.is_empty() && prefix.iter().all(|p| self.destructurable(p, false)),
			PatKind::Binding {
				mode, subpattern: None, ..
			} if !top => match mode.0 {
				rustc_hir::ByRef::No => !self.needs_copy(ty),
				// A shared reference to a primitive reads as its value.
				rustc_hir::ByRef::Yes(_, rustc_ast::Mutability::Not) => {
					matches!(ty.peel_refs().kind(), ty::Bool | ty::Int(_) | ty::Uint(_))
				}
				_ => false,
			},
			PatKind::Wild => !top,
			_ => false,
		}
	}

	/// The destructuring target of a [`Self::destructurable`] pattern, binding its names.
	fn destructure(&mut self, pat: &Pat<'tcx>, mutable: &mut bool) -> Target {
		match &pat.kind {
			PatKind::Deref { subpattern, .. } => self.destructure(subpattern, mutable),
			PatKind::Leaf { subpatterns } => {
				Target::Array(subpatterns.iter().map(|f| self.destructure_elem(&f.pattern, mutable)).collect())
			}
			PatKind::Array { prefix, .. } => Target::Array(prefix.iter().map(|p| self.destructure_elem(p, mutable)).collect()),
			PatKind::Binding { name, var, mode, .. } => {
				let is_mut = mode.1 == rustc_ast::Mutability::Mut;
				*mutable |= is_mut;
				Target::Name(self.bind_var(*var, *name, is_mut))
			}
			_ => unreachable!("checked by destructurable"),
		}
	}

	fn destructure_elem(&mut self, pat: &Pat<'tcx>, mutable: &mut bool) -> Option<Target> {
		match pat.kind {
			PatKind::Wild => None,
			_ => Some(self.destructure(pat, mutable)),
		}
	}

	/// Whether `id` is control flow that reads best delivering into a variable.
	fn needs_dest(&self, id: ExprId) -> bool {
		match &self.thir[id].kind {
			ExprKind::Scope { value, .. } => self.needs_dest(*value),
			ExprKind::If { then, else_opt, cond, .. } => {
				!(self.simple(*then) && else_opt.is_some_and(|e| self.simple(e)) && self.simple(*cond))
			}
			ExprKind::Match { match_source, .. } => {
				!matches!(match_source, MatchSource::TryDesugar(_)) && !self.ternary_match(id)
			}
			ExprKind::Loop { .. } => true,
			_ => false,
		}
	}

	/// Whether `id` lowers to an expression with no statements before it.
	fn simple(&self, id: ExprId) -> bool {
		match &self.thir[id].kind {
			ExprKind::Scope { value, .. }
			| ExprKind::ValueTypeAscription { source: value, .. }
			| ExprKind::PlaceTypeAscription { source: value, .. }
			| ExprKind::Borrow { arg: value, .. }
			| ExprKind::Deref { arg: value }
			| ExprKind::Cast { source: value }
			| ExprKind::PointerCoercion { source: value, .. }
			| ExprKind::Field { lhs: value, .. }
			| ExprKind::Unary { arg: value, .. } => self.simple(*value),
			ExprKind::Block { block } => {
				let block = &self.thir[*block];
				block.stmts.is_empty() && !block.targeted_by_break && block.expr.is_some_and(|e| self.simple(e))
			}
			ExprKind::Binary { lhs, rhs, .. } | ExprKind::Index { lhs, index: rhs } => {
				self.simple(*lhs) && self.simple(*rhs)
			}
			ExprKind::LogicalOp { lhs, rhs, .. } => self.simple(*lhs) && self.simple(*rhs),
			ExprKind::Call { args, fun, .. } => self.simple(*fun) && args.iter().all(|a| self.simple(*a)),
			ExprKind::Tuple { fields } | ExprKind::Array { fields } => fields.iter().all(|f| self.simple(*f)),
			ExprKind::Adt(adt) => adt.fields.iter().all(|f| self.simple(f.expr)) && matches!(adt.base, thir::AdtExprBase::None),
			ExprKind::VarRef { .. }
			| ExprKind::UpvarRef { .. }
			| ExprKind::Literal { .. }
			| ExprKind::NonHirLiteral { .. }
			| ExprKind::ZstLiteral { .. }
			| ExprKind::NamedConst { .. }
			| ExprKind::ConstParam { .. } => true,
			_ => false,
		}
	}

	fn break_(&mut self, label: region::Scope, value: Option<ExprId>, span: Span, out: &mut Vec<Stmt>) {
		let Some(index) = self.breakables.iter().rposition(|b| b.scope == label) else {
			self.t.err(span, "a `break` out of the function");
			return;
		};
		let dest = self.breakables[index].dest.clone();
		if let Some(value) = value {
			self.stmt(value, dest.clone(), out);
			if matches!(dest, Dest::Return) && !ts::diverges(out) {
				// The value was unit, and the loop's value is returned.
				self.push(out, StmtKind::Return(None), span);
			}
			if matches!(dest, Dest::Return) {
				return;
			}
		}
		let label = self.target(label, false);
		self.push(out, StmtKind::Break(label), span);
	}

	/// The JS label a `break` or `continue` to `scope` needs, if any.
	fn target(&mut self, scope: region::Scope, continues: bool) -> Option<String> {
		let index = self.breakables.iter().rposition(|b| b.scope == scope)?;
		let innermost_loop = self.breakables.iter().rposition(|b| b.is_loop);
		let needs_label = !self.breakables[index].is_loop || innermost_loop != Some(index);
		debug_assert!(!continues || self.breakables[index].is_loop);
		if !needs_label {
			return None;
		}
		self.breakables[index].used = true;
		Some(self.breakables[index].label.clone())
	}

	fn lp(&mut self, scope: region::Scope, body: ExprId, dest: Dest, out: &mut Vec<Stmt>) {
		let span = self.span(body);
		let label = self.fresh("loop");
		self.breakables.push(Breakable {
			scope,
			label,
			used: false,
			dest,
			is_loop: true,
		});
		// `while c { .. }` arrives as `loop { if c { .. } else { break } }`.
		let mut stmts = Vec::new();
		let mut cond = None;
		if let Some((c, then)) = self.while_shape(scope, body) {
			let mut pre = Vec::new();
			let test = self.expr(c, &mut pre);
			if pre.is_empty() {
				cond = Some(test);
				self.stmt(then, Dest::Discard, &mut stmts);
			}
		}
		let cond = match cond {
			Some(cond) => cond,
			None => {
				stmts.clear();
				self.stmt(body, Dest::Discard, &mut stmts);
				Expr::lit("true")
			}
		};
		let breakable = self.breakables.pop().unwrap();
		let label = breakable.used.then_some(breakable.label);
		self.push(out, StmtKind::While { label, cond, body: stmts }, span);
	}

	/// The condition and body of a `while` loop's desugaring.
	fn while_shape(&self, scope: region::Scope, body: ExprId) -> Option<(ExprId, ExprId)> {
		let block = match self.thir[self.strip(body)].kind {
			ExprKind::Block { block } => &self.thir[block],
			_ => return None,
		};
		if !block.stmts.is_empty() || block.targeted_by_break {
			return None;
		}
		let ExprKind::If { cond, then, else_opt: Some(els), .. } = self.thir[self.strip(block.expr?)].kind else {
			return None;
		};
		if self.has_let(cond) {
			return None;
		}
		let ExprKind::Block { block: els } = self.thir[self.strip(els)].kind else {
			return None;
		};
		let els = &self.thir[els];
		let only = match (els.stmts.as_ref(), els.expr) {
			([], Some(e)) => e,
			([s], None) => match self.thir[*s].kind {
				thir::StmtKind::Expr { expr, .. } => expr,
				_ => return None,
			},
			_ => return None,
		};
		match self.thir[self.strip(only)].kind {
			ExprKind::Break { label, value: None } if label == scope => Some((cond, then)),
			_ => None,
		}
	}

	/// `id` without the wrappers that change nothing.
	pub fn strip(&self, id: ExprId) -> ExprId {
		match self.thir[id].kind {
			ExprKind::Scope { value, .. } | ExprKind::NeverToAny { source: value } => {
				self.strip(value)
			}
			_ => id,
		}
	}

	fn has_let(&self, id: ExprId) -> bool {
		match &self.thir[self.strip(id)].kind {
			ExprKind::Let { .. } => true,
			ExprKind::LogicalOp {
				op: thir::LogicalOp::And,
				lhs,
				rhs,
			} => self.has_let(*lhs) || self.has_let(*rhs),
			_ => false,
		}
	}

	fn cond_parts(&self, id: ExprId, parts: &mut Vec<CondPart>) {
		let id = self.strip(id);
		match &self.thir[id].kind {
			ExprKind::Let { expr, pat } => {
				// SAFETY of the lifetime: the pattern lives in `self.thir`, which outlives
				// the parts, which are used within the call that made them.
				let pat: &'static Pat<'static> = unsafe { std::mem::transmute::<&Pat<'tcx>, &Pat<'static>>(&**pat) };
				parts.push(CondPart::Let(*expr, pat));
			}
			ExprKind::LogicalOp {
				op: thir::LogicalOp::And,
				lhs,
				rhs,
			} if self.has_let(id) => {
				self.cond_parts(*lhs, parts);
				self.cond_parts(*rhs, parts);
			}
			_ => parts.push(CondPart::Bool(id)),
		}
	}

	fn if_(&mut self, cond: ExprId, then: ExprId, els: Option<ExprId>, dest: Dest, span: Span, out: &mut Vec<Stmt>) {
		let mut parts = Vec::new();
		self.cond_parts(cond, &mut parts);
		let mut else_stmts = Vec::new();
		if let Some(els) = els {
			self.stmt(els, dest.clone(), &mut else_stmts);
		}

		// One condition, as JS writes it.
		if let [part] = parts.as_slice() {
			let mut then_stmts = Vec::new();
			let test = match part {
				CondPart::Bool(id) => Some(self.expr(*id, out)),
				CondPart::Let(expr, pat) => {
					let pat: &Pat<'tcx> = unsafe { std::mem::transmute::<&Pat<'static>, &Pat<'tcx>>(*pat) };
					let owned = !self.is_place(*expr);
					let value = match owned {
						true => self.operand(*expr, out),
						false => self.place(*expr, out),
					};
					let scrut = self.path(value, *expr, out);
					let test = self.pat_test(pat, &scrut);
					self.pat_bind(pat, &scrut, owned, &mut then_stmts);
					test
				}
			};
			self.stmt(then, dest, &mut then_stmts);
			match test {
				Some(test) => self.push(
					out,
					StmtKind::If {
						cond: test,
						then: then_stmts,
						els: else_stmts,
					},
					span,
				),
				None => out.extend(then_stmts),
			}
			return;
		}

		// A let chain: each part nests in the last, and any failing runs the else.
		let has_else = !else_stmts.is_empty();
		let label = self.fresh("chain");
		let mut then_stmts = Vec::new();
		self.stmt(then, dest, &mut then_stmts);
		if has_else && !ts::diverges(&then_stmts) {
			then_stmts.push(StmtKind::Break(Some(label.clone())).at(None));
		}
		let mut levels: Vec<(Vec<Stmt>, Option<Expr>)> = Vec::new();
		let mut pre = Vec::new();
		for part in &parts {
			let mut binds = Vec::new();
			let test = match part {
				CondPart::Bool(id) => Some(self.expr(*id, &mut pre)),
				CondPart::Let(expr, pat) => {
					let pat: &Pat<'tcx> = unsafe { std::mem::transmute::<&Pat<'static>, &Pat<'tcx>>(*pat) };
					let owned = !self.is_place(*expr);
					let value = match owned {
						true => self.operand(*expr, &mut pre),
						false => self.place(*expr, &mut pre),
					};
					let scrut = self.path(value, *expr, &mut pre);
					let test = self.pat_test(pat, &scrut);
					self.pat_bind(pat, &scrut, owned, &mut binds);
					test
				}
			};
			levels.push((std::mem::take(&mut pre), test));
			levels.last_mut().unwrap().0.extend(Vec::<Stmt>::new());
			if !binds.is_empty() {
				// Bindings run once the test passed, before the next part.
				levels.push((binds, None));
			}
		}
		let mut body = then_stmts;
		for (pre, test) in levels.into_iter().rev() {
			let mut wrapped = pre;
			match test {
				Some(test) => wrapped.push(
					StmtKind::If {
						cond: test,
						then: body,
						els: Vec::new(),
					}
					.at(None),
				),
				None => wrapped.extend(body),
			}
			body = wrapped;
		}
		match has_else {
			true => {
				body.extend(else_stmts);
				self.push(out, StmtKind::Block { label: Some(label), body }, span);
			}
			false => out.extend(body),
		}
	}

	fn match_(
		&mut self,
		id: ExprId,
		scrutinee: ExprId,
		arms: &[ArmId],
		source: MatchSource,
		dest: Dest,
		out: &mut Vec<Stmt>,
	) {
		let span = self.span(id);
		match source {
			MatchSource::TryDesugar(_) => {
				let value = self.try_(scrutinee, arms, span, out);
				self.deliver(value, &dest, span, out);
				return;
			}
			MatchSource::ForLoopDesugar => {
				self.for_(scrutinee, arms, span, out);
				return;
			}
			_ => {}
		}

		let owned = !self.is_place(scrutinee);
		let value = match owned {
			true => self.operand(scrutinee, out),
			false => self.place(scrutinee, out),
		};
		let scrut = self.path(value, scrutinee, out);

		struct Lowered {
			test: Option<Expr>,
			binds: Vec<Stmt>,
			guard: Option<(Vec<Stmt>, Expr)>,
			body: Vec<Stmt>,
		}
		let mut lowered = Vec::new();
		for &arm in arms {
			let arm = &self.thir[arm];
			let test = self.pat_test(&arm.pattern, &scrut);
			let mut binds = Vec::new();
			self.pat_bind(&arm.pattern, &scrut, owned, &mut binds);
			let guard = arm.guard.map(|g| {
				let mut pre = Vec::new();
				let e = self.expr(g, &mut pre);
				(pre, e)
			});
			let mut body = Vec::new();
			self.stmt(arm.body, dest.clone(), &mut body);
			lowered.push(Lowered { test, binds, guard, body });
		}

		// Arms that all do nothing, such as logging left out.
		if lowered.iter().all(|arm| arm.body.is_empty() && arm.guard.is_none()) {
			return;
		}

		let chain = lowered.iter().all(|arm| match &arm.guard {
			None => true,
			Some((pre, _)) => pre.is_empty() && arm.binds.is_empty(),
		});
		if chain {
			// An if-else chain; the last arm is what Rust proved is left.
			let mut els: Vec<Stmt> = Vec::new();
			let count = lowered.len();
			for (i, arm) in lowered.into_iter().enumerate().rev() {
				let last = i + 1 == count;
				let cond = match (arm.test, arm.guard) {
					(t, Some((_, g))) => Some(match t {
						Some(t) => t.and(g),
						None => g,
					}),
					(Some(_), None) if last => None,
					(t, None) => t,
				};
				let mut then = arm.binds;
				then.extend(arm.body);
				els = match cond {
					Some(cond) => vec![StmtKind::If { cond, then, els }.at(None)],
					None => then,
				};
			}
			let loc = self.t.loc(span);
			if let Some(first) = els.first_mut()
				&& first.loc.is_none()
			{
				first.loc = loc;
			}
			out.extend(els);
			return;
		}

		// Guards that read the arm's bindings: each arm tests, binds, then guards.
		let label = self.fresh("arms");
		let mut body = Vec::new();
		let count = lowered.len();
		for (i, arm) in lowered.into_iter().enumerate() {
			let mut inner = arm.binds;
			let mut taken = arm.body;
			if i + 1 < count && !ts::diverges(&taken) {
				taken.push(StmtKind::Break(Some(label.clone())).at(None));
			}
			match arm.guard {
				Some((pre, g)) => {
					inner.extend(pre);
					inner.push(
						StmtKind::If {
							cond: g,
							then: taken,
							els: Vec::new(),
						}
						.at(None),
					);
				}
				None => inner.extend(taken),
			}
			match arm.test {
				Some(test) => body.push(
					StmtKind::If {
						cond: test,
						then: inner,
						els: Vec::new(),
					}
					.at(None),
				),
				None => body.extend(inner),
			}
		}
		self.push(out, StmtKind::Block { label: Some(label), body }, span);
	}

	/// Whether a `match` reads as a ternary chain: no bindings or guards, and arms that
	/// are each a simple value.
	fn ternary_match(&self, id: ExprId) -> bool {
		let ExprKind::Match { arms, .. } = &self.thir[self.strip(id)].kind else {
			return false;
		};
		arms.iter().all(|&arm| {
			let arm = &self.thir[arm];
			arm.guard.is_none() && !has_bindings(&arm.pattern) && self.simple(arm.body)
		})
	}

	/// `expr?`: the value, returning early on `Err` or `None`.
	fn try_(&mut self, scrutinee: ExprId, arms: &[ArmId], span: Span, out: &mut Vec<Stmt>) -> Expr {
		// The scrutinee is `Try::branch(expr)`.
		let ExprKind::Call { args, .. } = &self.thir[self.strip(scrutinee)].kind else {
			return self.err(span, "an unexpected `?` desugaring");
		};
		let inner = args[0];
		let inner_ty = self.ty_of(inner);
		let value = self.operand(inner, out);
		let name = match value {
			Expr::Ident(ref name) => name.clone(),
			value => {
				let name = self.temp_for(inner);
				self.push(
					out,
					StmtKind::decl(false, name.clone(), None, Some(value)),
					span,
				);
				name
			}
		};
		let t = Expr::ident(&name);
		let ty::Adt(adt, adt_args) = inner_ty.kind() else {
			return self.err(span, format!("`?` on `{inner_ty}`"));
		};
		match library::Std::of(self.t.tcx, adt.did()) {
			Some(library::Std::Option) => {
				let ret = self.ret;
				let ty::Adt(ret_adt, _) = ret.kind() else {
					return self.err(span, "`?` on an `Option` in a function that returns no `Option`");
				};
				if library::Std::of(self.t.tcx, ret_adt.did()) != Some(library::Std::Option) {
					return self.err(span, "`?` on an `Option` in a function that returns no `Option`");
				}
				self.push(
					out,
					StmtKind::If {
						cond: t.clone().binary("===", Expr::undefined()),
						then: vec![StmtKind::Return(Some(Expr::undefined())).at(None)],
						els: Vec::new(),
					},
					span,
				);
				t
			}
			Some(library::Std::Result) => {
				let from = adt_args.type_at(1);
				let ret = self.ret;
				let to = match ret.kind() {
					ty::Adt(ret_adt, ret_args) if library::Std::of(self.t.tcx, ret_adt.did()) == Some(library::Std::Result) => {
						ret_args.type_at(1)
					}
					_ => return self.err(span, "`?` on a `Result` in a function that returns no `Result`"),
				};
				let error = t.clone().member("error");
				let converted = self.convert(error, from, to, span, out);
				self.push(
					out,
					StmtKind::If {
						cond: t.clone().member("ok").not(),
						then: vec![StmtKind::Return(Some(Expr::rt("Err", vec![converted]))).at(None)],
						els: Vec::new(),
					},
					span,
				);
				let _ = arms;
				t.member("value")
			}
			_ => self.err(span, format!("`?` on `{inner_ty}`")),
		}
	}

	/// `From::from(value)`, converting a `from` into a `to`.
	pub fn convert(&mut self, value: Expr, from: Ty<'tcx>, to: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Expr {
		if from == to {
			return value;
		}
		let tcx = self.t.tcx;
		let Some(from_trait) = tcx.get_diagnostic_item(rustc_span::sym::From) else {
			return self.err(span, "no `From` trait");
		};
		let Some(from_fn) = tcx.associated_items(from_trait).in_definition_order().find(|i| i.is_fn()) else {
			return self.err(span, "no `From::from`");
		};
		let args = tcx.mk_args(&[to.into(), from.into()]);
		self.call_def(from_fn.def_id, args, Args::Values(vec![value]), span, out)
	}

	/// `for pat in iter { body }`, from its desugaring.
	fn for_(&mut self, scrutinee: ExprId, arms: &[ArmId], span: Span, out: &mut Vec<Stmt>) {
		// match IntoIterator::into_iter(iter) { mut it => loop { match next(&mut it) {
		//     None => break, Some(pat) => body } } }
		let bad = |b: &mut Self| {
			b.t.err(span, "an unexpected `for` desugaring");
		};
		let ExprKind::Call { args, .. } = &self.thir[self.strip(scrutinee)].kind else {
			return bad(self);
		};
		let iterable = args[0];
		let [arm] = arms else {
			return bad(self);
		};
		let arm = &self.thir[*arm];
		let ExprKind::Scope { region_scope, value, .. } = self.thir[arm.body].kind else {
			return bad(self);
		};
		let ExprKind::Loop { body } = self.thir[value].kind else {
			return bad(self);
		};
		let ExprKind::Block { block } = self.thir[self.strip(body)].kind else {
			return bad(self);
		};
		let next = match (self.thir[block].stmts.as_ref(), self.thir[block].expr) {
			([s], None) => match self.thir[*s].kind {
				thir::StmtKind::Expr { expr, .. } => expr,
				_ => return bad(self),
			},
			([], Some(e)) => e,
			_ => return bad(self),
		};
		let ExprKind::Match { arms: next_arms, .. } = &self.thir[self.strip(next)].kind else {
			return bad(self);
		};
		let Some(some_arm) = next_arms.iter().map(|a| &self.thir[*a]).find(|a| matches!(a.pattern.kind, PatKind::Variant { variant_index, .. } if variant_index.index() == 1)) else {
			return bad(self);
		};
		let PatKind::Variant { subpatterns, .. } = &some_arm.pattern.kind else {
			return bad(self);
		};
		let pat = &subpatterns[0].pattern;
		let loop_body = some_arm.body;

		let iter_ty = self.ty_of(iterable);
		let source = self.iterable(iterable, iter_ty, span, out);
		let Some(source) = source else {
			return;
		};

		let label = self.fresh("loop");
		self.breakables.push(Breakable {
			scope: region_scope,
			label,
			used: false,
			dest: Dest::Discard,
			is_loop: true,
		});
		let mut stmts = Vec::new();
		let simple_name = match &pat.kind {
			PatKind::Binding {
				name,
				var,
				mode,
				subpattern: None,
				..
			} if mode.1 == rustc_ast::Mutability::Not && !self.needs_copy(self.subst(pat.ty)) => {
				Some(self.bind_var(*var, *name, false))
			}
			PatKind::Wild => Some(self.fresh("_")),
			_ => None,
		};
		let name = match simple_name {
			Some(name) => name,
			None => {
				let name = self.fresh("item");
				let scrut = Expr::ident(&name);
				if self.pat_test(pat, &scrut).is_some() {
					self.t.err(pat.span, "a refutable `for` pattern");
				}
				self.pat_bind(pat, &scrut, true, &mut stmts);
				name
			}
		};
		self.stmt(loop_body, Dest::Discard, &mut stmts);
		let breakable = self.breakables.pop().unwrap();
		let label = breakable.used.then_some(breakable.label);
		let kind = match source {
			Iterable::Range { start, end } => StmtKind::ForRange {
				label,
				name,
				start,
				end,
				body: stmts,
			},
			Iterable::Of(iter) => StmtKind::ForOf {
				label,
				name,
				iter,
				body: stmts,
			},
		};
		self.push(out, kind, span);
	}

	/// What a `for` loop iterates.
	fn iterable(&mut self, id: ExprId, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Option<Iterable> {
		let tcx = self.t.tcx;
		match ty.kind() {
			ty::Adt(adt, _) if tcx.def_path_str(adt.did()) == "std::ops::Range" || tcx.is_lang_item(adt.did(), rustc_attr_ir::LangItem::Range) => {
				let ExprKind::Adt(range) = &self.thir[self.strip(id)].kind else {
					self.t.err(span, "a `for` loop over a range that is not written in place");
					return None;
				};
				let mut fields: Vec<ExprId> = vec![range.fields[0].expr, range.fields[1].expr];
				if range.fields[0].name.index() == 1 {
					fields.swap(0, 1);
				}
				let mut values = self.operands(&fields, out);
				let end = values.pop().unwrap();
				let mut start = values.pop().unwrap();
				let end = match end.is_constant() {
					true => end,
					false => {
						if !start.is_constant() {
							start = self.spill(start, "start", out);
						}
						self.spill(end, "end", out)
					}
				};
				Some(Iterable::Range { start, end })
			}
			ty::Ref(_, inner, _) => match inner.kind() {
				ty::Slice(_) | ty::Array(..) => Some(Iterable::Of(self.operand(id, out))),
				ty::Adt(adt, _) if library::Std::of(tcx, adt.did()) == Some(library::Std::Vec) => {
					let v = self.operand(id, out);
					Some(Iterable::Of(library::vec_view(self, v, *inner)))
				}
				_ => {
					self.t.err(span, format!("a `for` loop over `{ty}`"));
					None
				}
			},
			ty::Adt(adt, _) if library::Std::of(tcx, adt.did()) == Some(library::Std::Vec) => {
				let v = self.operand(id, out);
				Some(Iterable::Of(library::vec_view(self, v, ty)))
			}
			ty::Adt(adt, args) if tcx.def_path_str(adt.did()) == "std::slice::Iter" => {
				// `slice.iter()`: iterate the slice itself.
				let ExprKind::Call { args: call_args, .. } = &self.thir[self.strip(id)].kind else {
					self.t.err(span, format!("a `for` loop over `{ty}`"));
					return None;
				};
				let _ = args;
				Some(Iterable::Of(self.operand(call_args[0], out)))
			}
			_ => {
				self.t.err(span, format!("a `for` loop over `{ty}`"));
				None
			}
		}
	}

	fn spill(&mut self, value: Expr, base: &str, out: &mut Vec<Stmt>) -> Expr {
		let name = self.fresh(base);
		out.push(
			StmtKind::decl(false, name.clone(), None, Some(value))
			.at(None),
		);
		Expr::ident(name)
	}

	/// `value`, from `id`, as something patterns can read more than once.
	fn path(&mut self, value: Expr, id: ExprId, out: &mut Vec<Stmt>) -> Expr {
		if value.is_path() {
			return value;
		}
		let name = self.temp_for(id);
		out.push(
			StmtKind::decl(false, name.clone(), None, Some(value))
			.at(None),
		);
		Expr::ident(name)
	}

	fn assign(&mut self, lhs: ExprId, rhs: ExprId, span: Span, out: &mut Vec<Stmt>) {
		let mut value = self.operand(rhs, out);
		let mut pre = Vec::new();
		let target = self.place(lhs, &mut pre);
		if (!pre.is_empty() || !target.is_path()) && !value.is_constant() {
			value = self.spill(value, "value", out);
		}
		out.extend(pre);
		// `*r = v` through a reference replaces what it points to.
		if let ExprKind::Deref { arg } = self.thir[self.strip(lhs)].kind
			&& matches!(self.ty_of(arg).kind(), ty::Ref(..))
			&& self.is_place(arg)
		{
			let pointee = self.ty_of(lhs);
			match self.object_kind(pointee) {
				Some(ObjectKind::Class) => {
					let e = Expr::ident("Object").member("assign").call(vec![target, value]);
					self.push(out, StmtKind::Expr(e), span);
				}
				Some(ObjectKind::Bytes) => {
					let e = target.member("set").call(vec![value]);
					self.push(out, StmtKind::Expr(e), span);
				}
				_ => {
					self.t.err(span, format!("an assignment through a reference to `{pointee}`"));
				}
			}
			return;
		}
		self.push(out, StmtKind::Expr(target.assign(value)), span);
	}

	fn assign_op(&mut self, op: rustc_middle::mir::AssignOp, lhs: ExprId, rhs: ExprId, span: Span, out: &mut Vec<Stmt>) {
		let mut value = self.operand(rhs, out);
		if !value.is_path() {
			value = self.spill(value, "value", out);
		}
		let target = self.place(lhs, out);
		if !target.is_path() && !matches!(target, Expr::Index(..)) {
			self.t.err(span, "a compound assignment to a computed place");
			return;
		}
		let lty = self.ty_of(lhs);
		let rty = self.ty_of(rhs);
		let op = rustc_middle::mir::BinOp::from(op);
		let lhs_value = super::num::Operand {
			expr: target.clone(),
			value: None,
		};
		let rhs_value = super::num::Operand { expr: value, value: None };
		let result = super::num::binary(self, op, lhs_value, rhs_value, lty, rty, span);
		self.push(out, StmtKind::Expr(target.assign(result)), span);
	}

	// Expressions

	/// Lower `id` as an operand: its value, copied if it is a `Copy` aggregate read
	/// from a place, so a later write to either side does not reach the other.
	pub fn operand(&mut self, id: ExprId, out: &mut Vec<Stmt>) -> Expr {
		self.expr(id, out)
	}

	/// Lower operands left to right, saving earlier ones that a later one's statements
	/// could change.
	pub fn operands(&mut self, ids: &[ExprId], out: &mut Vec<Stmt>) -> Vec<Expr> {
		let mut values: Vec<Expr> = Vec::with_capacity(ids.len());
		for &id in ids {
			let mut pre = Vec::new();
			let value = self.operand(id, &mut pre);
			if !pre.is_empty() {
				for earlier in values.iter_mut() {
					if !earlier.is_constant() {
						let saved = std::mem::replace(earlier, Expr::undefined());
						*earlier = self.spill(saved, "arg", out);
					}
				}
			}
			out.extend(pre);
			values.push(value);
		}
		values
	}

	/// Whether `id` names a place, rather than making a temporary value.
	pub fn is_place(&self, id: ExprId) -> bool {
		match &self.thir[id].kind {
			ExprKind::Scope { value, .. } | ExprKind::PlaceTypeAscription { source: value, .. } => self.is_place(*value),
			ExprKind::VarRef { .. } | ExprKind::UpvarRef { .. } | ExprKind::Field { .. } | ExprKind::Index { .. } => {
				true
			}
			ExprKind::Deref { .. } => true,
			_ => false,
		}
	}

	pub fn needs_copy(&self, ty: Ty<'tcx>) -> bool {
		if !self.t.is_copy(ty) {
			return false;
		}
		self.mutable_aggregate(ty)
	}

	/// Whether values of `ty` are JS objects that Rust code can change in place.
	fn mutable_aggregate(&self, ty: Ty<'tcx>) -> bool {
		match ty.kind() {
			ty::Tuple(tys) => !tys.is_empty(),
			ty::Array(..) => true,
			ty::Adt(adt, args) => match library::Std::of(self.t.tcx, adt.did()) {
				Some(library::Std::Option) => self.mutable_aggregate(args.type_at(0)),
				Some(_) => false,
				None => adt.is_struct() && !adt.non_enum_variant().fields.is_empty(),
			},
			_ => false,
		}
	}

	/// What kind of JS object a `&mut` to `ty` can change in place.
	fn object_kind(&self, ty: Ty<'tcx>) -> Option<ObjectKind> {
		match ty.kind() {
			ty::Array(elem, _) | ty::Slice(elem) => match elem.kind() {
				ty::Uint(ty::UintTy::U8) => Some(ObjectKind::Bytes),
				_ => Some(ObjectKind::Array),
			},
			ty::Adt(adt, _) => match library::Std::of(self.t.tcx, adt.did()) {
				Some(library::Std::Vec) => Some(ObjectKind::Vec),
				Some(library::Std::Box) => None,
				Some(_) => None,
				None if adt.is_struct() => Some(ObjectKind::Class),
				None => None,
			},
			_ => None,
		}
	}

	pub(super) fn ref_binding(&mut self, by_ref: rustc_hir::ByRef, ty: Ty<'tcx>, span: Span) {
		if let rustc_hir::ByRef::Yes(_, rustc_ast::Mutability::Mut) = by_ref {
			let pointee = match ty.kind() {
				ty::Ref(_, inner, _) => *inner,
				_ => ty,
			};
			if self.object_kind(pointee).is_none() {
				self.t.err(span, format!("a `ref mut` binding to `{pointee}`, which is no object a reference can change"));
			}
		}
	}

	/// Lower `id` for its value.
	pub fn expr(&mut self, id: ExprId, out: &mut Vec<Stmt>) -> Expr {
		let e = &self.thir[id];
		let span = e.span;
		let ty = self.ty_of(id);
		if self.t.ignored_macro(span) {
			if !ty.is_unit() {
				return self.err(span, "an ignored macro must evaluate to `()`");
			}
			return Expr::undefined();
		}
		match &e.kind {
			ExprKind::Scope { value, .. } => match self.thir[*value].kind {
				ExprKind::Loop { .. } => self.via_temp(id, ty, span, out),
				_ => self.expr(*value, out),
			},
			ExprKind::ValueTypeAscription { source, .. }
			| ExprKind::ValueExpr { source } => self.expr(*source, out),
			ExprKind::NeverToAny { source } => {
				self.stmt(*source, Dest::Discard, out);
				Expr::undefined()
			}
			ExprKind::Block { block } if !self.thir[*block].targeted_by_break => {
				let block = &self.thir[*block];
				for &stmt in &block.stmts {
					self.stmt_item(stmt, out);
				}
				match block.expr {
					Some(expr) => self.expr(expr, out),
					None => Expr::undefined(),
				}
			}
			ExprKind::If { cond, then, else_opt, .. }
				if self.simple(*then) && else_opt.is_some_and(|e| self.simple(e)) && !self.has_let(*cond) =>
			{
				let c = self.expr(*cond, out);
				let mut pre = Vec::new();
				let a = self.expr(*then, &mut pre);
				let b = self.expr(else_opt.unwrap(), &mut pre);
				if !pre.is_empty() {
					out.extend(pre);
					return self.err(span, "an `if` that was simple and became not");
				}
				Expr::Cond(Box::new(c), Box::new(a), Box::new(b))
			}
			ExprKind::Match {
				scrutinee,
				arms,
				match_source: MatchSource::TryDesugar(_),
			} => self.try_(*scrutinee, arms, span, out),
			ExprKind::Match { .. } if self.ternary_match(id) => self.ternary(id, out),
			ExprKind::Block { .. } | ExprKind::If { .. } | ExprKind::Match { .. } | ExprKind::Loop { .. } => {
				self.via_temp(id, ty, span, out)
			}
			ExprKind::Call { ty: fn_ty, fun, args, fn_span, .. } => {
				let call = self.call(*fn_ty, *fun, args, *fn_span, out);
				match self.t.loc(span) {
					Some(loc) => Expr::Located(loc, Box::new(call)),
					None => call,
				}
			}
			ExprKind::VarRef { .. }
			| ExprKind::UpvarRef { .. }
			| ExprKind::Field { .. }
			| ExprKind::Index { .. }
			| ExprKind::Deref { .. }
			| ExprKind::PlaceTypeAscription { .. } => {
				if let ExprKind::Index { lhs, index } = &e.kind {
					let base = self.place(*lhs, out);
					let base = self.path(base, *lhs, out);
					let index_ty = self.ty_of(*lhs);
					let i = self.operand(*index, out);
					let value = match self.const_index(index_ty, *index) {
						true => base.index(i),
						false => Expr::rt("at", vec![base, i]),
					};
					return self.copied(value, ty);
				}
				if matches!(self.ty_of(id).kind(), ty::FnDef(..))
					&& let ExprKind::VarRef { .. } | ExprKind::UpvarRef { .. } = e.kind
				{
					let ty::FnDef(def_id, args) = *ty.kind() else { unreachable!() };
					return self.fn_value(def_id, args.skip_binder(), span, out);
				}
				let value = self.place(id, out);
				self.copied(value, ty)
			}
			ExprKind::Borrow { borrow_kind, arg } => {
				if let BorrowKind::Mut { .. } = borrow_kind {
					let pointee = self.ty_of(*arg);
					if self.object_kind(pointee).is_none() && !matches!(pointee.kind(), ty::Ref(_, _, rustc_ast::Mutability::Mut)) {
						return self.err(span, format!("a mutable reference to `{pointee}`, which is no object it can change in place"));
					}
				}
				self.place(*arg, out)
			}
			ExprKind::RawBorrow { .. } => self.err(span, "raw pointers are not supported"),
			ExprKind::Binary { op, lhs, rhs } => {
				let (lty, rty) = (self.ty_of(*lhs), self.ty_of(*rhs));
				let values = self.operands(&[*lhs, *rhs], out);
				let [l, r] = <[Expr; 2]>::try_from(values).unwrap();
				let l = super::num::Operand {
					expr: l,
					value: self.const_int(*lhs),
				};
				let r = super::num::Operand {
					expr: r,
					value: self.const_int(*rhs),
				};
				super::num::binary(self, *op, l, r, lty, rty, span)
			}
			ExprKind::LogicalOp { op, lhs, rhs } => {
				let l = self.expr(*lhs, out);
				let mut pre = Vec::new();
				let r = self.expr(*rhs, &mut pre);
				let js = match op {
					thir::LogicalOp::And => "&&",
					thir::LogicalOp::Or => "||",
				};
				if pre.is_empty() {
					return l.binary(js, r);
				}
				// The right side needs statements: run them only when it is evaluated.
				let name = self.fresh("cond");
				out.push(
					StmtKind::decl(true, name.clone(), Some(Type::Builtin("boolean")), Some(l))
					.at(None),
				);
				pre.push(StmtKind::Expr(Expr::ident(&name).assign(r)).at(None));
				let cond = match op {
					thir::LogicalOp::And => Expr::ident(&name),
					thir::LogicalOp::Or => Expr::ident(&name).not(),
				};
				out.push(StmtKind::If { cond, then: pre, els: Vec::new() }.at(None));
				Expr::ident(name)
			}
			ExprKind::Unary { op, arg } => {
				let aty = self.ty_of(*arg);
				let value = self.operand(*arg, out);
				super::num::unary(self, *op, value, aty, span)
			}
			ExprKind::Cast { source } => {
				let from = self.ty_of(*source);
				let value = self.operand(*source, out);
				let operand = super::num::Operand {
					expr: value,
					value: self.const_int(*source),
				};
				super::num::cast(self, operand, from, ty, span)
			}
			ExprKind::PointerCoercion { cast, source, .. } => match cast {
				PointerCoercion::Unsize | PointerCoercion::ReifyFnPointer(_) | PointerCoercion::ClosureFnPointer(_) => {
					self.expr(*source, out)
				}
				_ => self.err(span, format!("unsupported pointer coercion {cast:?}")),
			},
			ExprKind::Literal { .. }
			| ExprKind::NonHirLiteral { .. }
			| ExprKind::ZstLiteral { .. }
			| ExprKind::NamedConst { .. }
			| ExprKind::ConstParam { .. }
			| ExprKind::ConstBlock { .. } => self.constant(id, out),
			ExprKind::Tuple { fields } if fields.is_empty() => Expr::undefined(),
			ExprKind::Tuple { fields } => Expr::Array(self.operands(fields, out)),
			ExprKind::Array { fields } => {
				let values = self.operands(fields, out);
				match ty.kind() {
					ty::Array(elem, _) if matches!(elem.kind(), ty::Uint(ty::UintTy::U8)) => {
						Expr::ident("Uint8Array").member("of").call(values)
					}
					_ => Expr::Array(values),
				}
			}
			ExprKind::Repeat { value, count } => {
				let n = match count.try_to_target_usize(self.t.tcx) {
					Some(n) => n,
					None => return self.err(span, "an array length that is not a constant"),
				};
				let elem_ty = self.ty_of(*value);
				let v = self.operand(*value, out);
				match elem_ty.kind() {
					ty::Uint(ty::UintTy::U8) => {
						let array = Expr::New(Box::new(Expr::ident("Uint8Array")), vec![Expr::lit(n.to_string())]);
						match &v {
							Expr::Lit(text) if text == "0" => array,
							_ => array.member("fill").call(vec![v]),
						}
					}
					_ => {
						let v = self.path(v, *value, out);
						let item = self.copied(v, elem_ty);
						let item_ty = self.t.ty(elem_ty, span);
						Expr::ident("Array").member("from").call(vec![
							Expr::Object(vec![(Key::Name("length".into()), Expr::lit(n.to_string()))]),
							Expr::Arrow(Vec::new(), item_ty, vec![StmtKind::Return(Some(item.typed())).at(None)]),
						])
					}
				}
			}
			ExprKind::Adt(adt) => self.adt_expr(adt, ty, span, out),
			ExprKind::Closure(closure) => self.closure(closure, span),
			ExprKind::Assign { .. }
			| ExprKind::AssignOp { .. }
			| ExprKind::Break { .. }
			| ExprKind::Continue { .. }
			| ExprKind::Return { .. } => {
				self.stmt(id, Dest::Discard, out);
				Expr::undefined()
			}
			ExprKind::Let { .. } => self.err(span, "a `let` outside an `if` condition"),
			kind => self.err(span, format!("unsupported expression: {}", kind_name(kind))),
		}
	}

	/// `value`, copied if `ty` is a `Copy` aggregate.
	pub fn copied(&mut self, value: Expr, ty: Ty<'tcx>) -> Expr {
		match self.needs_copy(ty) {
			true => Expr::rt("copy", vec![value]),
			false => value,
		}
	}

	/// Whether indexing a `ty` at `index` is in bounds by construction: a constant index
	/// into an array long enough.
	fn const_index(&self, ty: Ty<'tcx>, index: ExprId) -> bool {
		let ty::Array(_, len) = ty.peel_refs().kind() else {
			return false;
		};
		let (Some(len), Some(i)) = (len.try_to_target_usize(self.t.tcx), self.const_int(index)) else {
			return false;
		};
		i >= 0 && (i as u64) < len
	}

	fn via_temp(&mut self, id: ExprId, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Expr {
		if ty.is_unit() || ty.is_never() {
			self.stmt(id, Dest::Discard, out);
			return Expr::undefined();
		}
		let name = self.temp_for(id);
		let ts_ty = self.t.ty(ty, span);
		out.push(
			StmtKind::decl(true, name.clone(), Some(ts_ty), None)
			.at(None),
		);
		self.stmt(id, Dest::Assign(name.clone()), out);
		Expr::ident(name)
	}

	/// A `match` of simple arms as a chain of `?:`.
	fn ternary(&mut self, id: ExprId, out: &mut Vec<Stmt>) -> Expr {
		let ExprKind::Match { scrutinee, arms, .. } = &self.thir[self.strip(id)].kind else {
			unreachable!()
		};
		let owned = !self.is_place(*scrutinee);
		let value = match owned {
			true => self.operand(*scrutinee, out),
			false => self.place(*scrutinee, out),
		};
		let scrut = self.path(value, *scrutinee, out);
		let mut chain: Vec<(Option<Expr>, Expr)> = Vec::new();
		for &arm in arms.iter() {
			let arm = &self.thir[arm];
			let test = self.pat_test(&arm.pattern, &scrut);
			let mut pre = Vec::new();
			let value = self.expr(arm.body, &mut pre);
			out.extend(pre);
			chain.push((test, value));
		}
		let (_, mut result) = chain.pop().unwrap();
		while let Some((test, value)) = chain.pop() {
			result = match (test, value, result) {
				(None, value, _) => value,
				(Some(test), Expr::Lit(a), Expr::Lit(b)) if a == "true" && b == "false" => test,
				(Some(test), Expr::Lit(a), Expr::Lit(b)) if a == "false" && b == "true" => test.not(),
				(Some(test), value, result) => Expr::Cond(Box::new(test), Box::new(value), Box::new(result)),
			};
		}
		result
	}

	/// Lower `id` as a place: no copy is made of what it names.
	pub fn place(&mut self, id: ExprId, out: &mut Vec<Stmt>) -> Expr {
		let e = &self.thir[id];
		let span = e.span;
		match &e.kind {
			ExprKind::Scope { value, .. } | ExprKind::PlaceTypeAscription { source: value, .. } => self.place(*value, out),
			ExprKind::VarRef { id: var } => self.local(var.0, span),
			ExprKind::UpvarRef { var_hir_id, .. } => self.local(var_hir_id.0, span),
			ExprKind::Deref { arg } => self.expr(*arg, out),
			ExprKind::Field { lhs, variant_index, name } => {
				let base = self.place(*lhs, out);
				let lty = self.ty_of(*lhs);
				self.project(base, lty, *variant_index, *name, span)
			}
			ExprKind::Index { lhs, index } => {
				let base = self.place(*lhs, out);
				let base = self.path(base, *lhs, out);
				let lty = self.ty_of(*lhs);
				let i = self.operand(*index, out);
				match self.const_index(lty, *index) {
					true => base.index(i),
					false => {
						let i = match i.is_path() {
							true => i,
							false => self.spill(i, "index", out),
						};
						base.clone().index(Expr::rt("idx", vec![base, i]))
					}
				}
			}
			_ => self.expr(id, out),
		}
	}

	/// A field of `base`, a `ty`.
	pub fn project(
		&mut self,
		base: Expr,
		ty: Ty<'tcx>,
		variant: rustc_abi::VariantIdx,
		field: rustc_abi::FieldIdx,
		span: Span,
	) -> Expr {
		let ty = ty.peel_refs();
		match ty.kind() {
			ty::Tuple(_) => base.index(Expr::lit(field.index().to_string())),
			ty::Adt(adt, _) => match library::Std::of(self.t.tcx, adt.did()) {
				Some(library::Std::Option) => base,
				Some(library::Std::Result) => match variant.index() {
					0 => base.member("value"),
					_ => base.member("error"),
				},
				Some(library::Std::Box) => base,
				Some(_) => self.err(span, format!("a field of `{ty}`")),
				None => {
					let info = self.t.adt(*adt, span);
					match info.repr {
						AdtRepr::Class { fields, .. } => base.field(fields[field.index()]),
						AdtRepr::Union | AdtRepr::Tags => {
							let def = &adt.variant(variant).fields[field];
							base.member(super::field_name(def.name.as_str(), field.index()))
						}
					}
				}
			},
			_ => self.err(span, format!("a field of `{ty}`")),
		}
	}

	fn adt_expr(&mut self, adt: &thir::AdtExpr<'tcx>, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Expr {
		let tcx = self.t.tcx;
		let def = adt.adt_def;
		let variant = def.variant(adt.variant_index);
		let ids: Vec<ExprId> = adt.fields.iter().map(|f| f.expr).collect();
		let mut values = self.operands(&ids, out);
		if let Some(std) = library::Std::of(tcx, def.did()) {
			return match (std, adt.variant_index.index()) {
				(library::Std::Option, 0) => Expr::undefined(),
				(library::Std::Option, _) => values.pop().unwrap(),
				(library::Std::Result, 0) => Expr::rt("Ok", values),
				(library::Std::Result, _) => Expr::rt("Err", values),
				_ => self.err(span, format!("constructing `{ty}`")),
			};
		}
		if !def.did().is_local() {
			return self.err(span, format!("constructing `{ty}` from another crate"));
		}
		let info = self.t.adt(def, span);
		let base = match &adt.base {
			thir::AdtExprBase::None => None,
			thir::AdtExprBase::Base(fru) => {
				// The base is read after the fields, so earlier fields are saved first.
				let mut pre = Vec::new();
				let base = self.operand(fru.base, &mut pre);
				if !pre.is_empty() || !base.is_path() {
					for value in values.iter_mut() {
						if !value.is_constant() {
							let saved = std::mem::replace(value, Expr::undefined());
							*value = self.spill(saved, "field", out);
						}
					}
				}
				out.extend(pre);
				Some(self.path(base, fru.base, out))
			}
			thir::AdtExprBase::DefaultFields(_) => return self.err(span, "default field values are not supported"),
		};
		match info.repr {
			AdtRepr::Class { ctor, fields } => {
				let class = Box::new(Expr::Sym(info.sym));
				match ctor {
					ts::Ctor::Unit => Expr::New(class, Vec::new()),
					ts::Ctor::Positional => {
						let mut ordered: Vec<Option<Expr>> = vec![None; variant.fields.len()];
						for (f, value) in adt.fields.iter().zip(values) {
							ordered[f.name.index()] = Some(value.typed());
						}
						let args = ordered
							.into_iter()
							.enumerate()
							.map(|(i, v)| match (v, &base) {
								(Some(v), _) => v,
								(None, Some(base)) => {
									let field_ty = self.subst(variant.fields[rustc_abi::FieldIdx::from_usize(i)].ty(tcx, adt.args).skip_norm_wip());
									self.copied(base.clone().field(fields[i]), field_ty)
								}
								(None, None) => Expr::undefined(),
							})
							.collect();
						Expr::New(class, args)
					}
					ts::Ctor::Named => {
						let mut props: Vec<(Key, Expr)> = adt
							.fields
							.iter()
							.zip(values)
							.map(|(f, value)| (Key::Field(fields[f.name.index()]), value.typed()))
							.collect();
						if let Some(base) = &base {
							for (i, field) in variant.fields.iter().enumerate() {
								if adt.fields.iter().any(|f| f.name.index() == i) {
									continue;
								}
								let field_ty = self.subst(field.ty(tcx, adt.args).skip_norm_wip());
								let value = self.copied(base.clone().field(fields[i]), field_ty);
								props.push((Key::Field(fields[i]), value));
							}
						}
						Expr::New(class, vec![Expr::Object(props)])
					}
				}
			}
			AdtRepr::Tags => {
				let ts_ty = self.t.ty(ty, span);
				Expr::As(Box::new(Expr::Str(variant.name.to_string())), ts_ty)
			}
			AdtRepr::Union => {
				let mut props = vec![(Key::Name("tag".into()), Expr::Str(variant.name.to_string()))];
				let mut ordered: Vec<(usize, Expr)> =
					adt.fields.iter().zip(values).map(|(f, value)| (f.name.index(), value)).collect();
				ordered.sort_by_key(|(i, _)| *i);
				for (i, value) in ordered {
					let name = super::field_name(variant.fields[rustc_abi::FieldIdx::from_usize(i)].name.as_str(), i);
					props.push((Key::Name(name), value));
				}
				let ts_ty = self.t.ty(ty, span);
				Expr::As(Box::new(Expr::Object(props)), ts_ty)
			}
		}
	}

	fn closure(&mut self, closure: &thir::ClosureExpr<'tcx>, span: Span) -> Expr {
		let tcx = self.t.tcx;
		let local = closure.closure_id;
		let Some((thir, body)) = self.t.body(local) else {
			return self.err(span, "no body for a closure");
		};
		// A copy captured by `move` would be shared in JS: refuse one that can change.
		for capture in tcx.closure_captures(local) {
			if let ty::UpvarCapture::ByValue = capture.info.capture_kind {
				let var = capture.get_root_variable();
				let mutable = matches!(self.locals.get(&var), Some(Local::Name { mutable: true, .. }));
				if mutable && self.t.is_copy(self.subst(capture.place.ty())) {
					self.t.err(capture.get_path_span(tcx), "a closure that copies a mutable variable");
				}
			}
		}
		let saved_thir = std::mem::replace(&mut self.thir, thir);
		let saved_breakables = std::mem::take(&mut self.breakables);
		let closure_ty = self.subst(tcx.type_of(local).instantiate_identity().skip_norm_wip());
		let sig = match closure_ty.kind() {
			ty::Closure(_, args) => tcx.instantiate_bound_regions_with_erased(args.as_closure().sig()),
			_ => {
				self.thir = saved_thir;
				self.breakables = saved_breakables;
				return self.err(span, format!("a closure of type `{closure_ty}`"));
			}
		};
		let saved_ret = std::mem::replace(&mut self.ret, sig.output());
		let mut body_stmts = Vec::new();
		let mut params = Vec::new();
		// The first parameter is the closure's environment.
		for param in thir.params.iter().skip(1) {
			let name = match &param.pat {
				Some(pat) => self.param(pat, &mut body_stmts),
				None => self.fresh("arg"),
			};
			let ty = self.subst(param.ty);
			let ty = self.t.ty(ty, span);
			params.push(Param { name, ty: Some(ty) });
		}
		let ret = self.ret;
		let dest = match ret.is_unit() || ret.is_never() {
			true => Dest::Discard,
			false => Dest::Return,
		};
		self.stmt(body, dest, &mut body_stmts);
		let ret_ty = self.t.ret_ty(ret, span);
		self.thir = saved_thir;
		self.breakables = saved_breakables;
		self.ret = saved_ret;
		Expr::Arrow(params, ret_ty, body_stmts)
	}

	// Calls

	fn call(&mut self, fn_ty: Ty<'tcx>, fun: ExprId, args: &[ExprId], span: Span, out: &mut Vec<Stmt>) -> Expr {
		let fn_ty = self.subst(fn_ty);
		match *fn_ty.kind() {
			ty::FnDef(def_id, gargs) => {
				// A function item is a type: evaluating `fun` only runs its effects.
				if !self.simple(fun) {
					self.stmt(fun, Dest::Discard, out);
				}
				self.call_def(def_id, gargs.skip_binder(), Args::Thir(args), span, out)
			}
			ty::FnPtr(..) => {
				let f = self.operand(fun, out);
				let values = self.operands(args, out);
				f.call(values)
			}
			_ => self.err(span, format!("a call through `{fn_ty}`")),
		}
	}

	/// A call of `def_id` with `gargs`, from wherever it resolves.
	pub fn call_def(
		&mut self,
		def_id: rustc_hir::def_id::DefId,
		gargs: GenericArgsRef<'tcx>,
		args: Args<'_>,
		span: Span,
		out: &mut Vec<Stmt>,
	) -> Expr {
		let tcx = self.t.tcx;
		let gargs = self.subst(gargs);

		// A tuple struct or variant's constructor, called or passed as a value.
		if let rustc_hir::def::DefKind::Ctor(..) = tcx.def_kind(def_id) {
			return self.ctor(def_id, gargs, args, span, out);
		}

		// Calling a closure or function value: `FnOnce::call_once(f, (args,))`.
		if let Some(trait_id) = tcx.trait_of_assoc(def_id)
			&& tcx.is_fn_trait(trait_id)
		{
			let callee_ty = gargs.type_at(0);
			let Args::Thir(ids) = args else {
				return self.err(span, "a closure call with lowered arguments");
			};
			let tupled = self.strip(ids[1]);
			let ExprKind::Tuple { fields } = &self.thir[tupled].kind else {
				return self.err(span, "a closure call whose arguments are not a tuple");
			};
			let fields = fields.clone();
			if let ty::FnDef(inner, inner_args) = *callee_ty.peel_refs().kind() {
				return self.call_def(inner, inner_args.skip_binder(), Args::Thir(&fields), span, out);
			}
			let mut all = vec![ids[0]];
			all.extend(fields.iter());
			let mut values = self.operands(&all, out);
			let f = values.remove(0);
			return f.call(values);
		}

		let instance = match Instance::try_resolve(tcx, self.t.typing_env_pub(), def_id, gargs) {
			Ok(Some(instance)) => instance,
			_ => return self.err(span, format!("`{}` does not resolve", tcx.def_path_str(def_id))),
		};
		if let ty::InstanceKind::Item(did) = instance.def
			&& did.is_local()
		{
			let sym = self.t.instance(instance);
			let values: Vec<Expr> = self.args(args, out).into_iter().map(Expr::typed).collect();
			return match self.t.program.syms[sym].kind {
				SymKind::Method { instance: true } => {
					let mut values = values;
					let recv = values.remove(0);
					Expr::Method(Box::new(recv), sym, values)
				}
				_ => Expr::Sym(sym).call(values),
			};
		}
		let call = library::Call { def_id, args: gargs, span };
		match library::call(self, &call, args, out) {
			Some(e) => e,
			None => self.err(span, format!("unsupported call to `{}`", tcx.def_path_str_with_args(def_id, gargs))),
		}
	}

	pub fn args(&mut self, args: Args<'_>, out: &mut Vec<Stmt>) -> Vec<Expr> {
		match args {
			Args::Thir(ids) => self.operands(ids, out),
			Args::Values(values) => values,
		}
	}

	fn ctor(
		&mut self,
		def_id: rustc_hir::def_id::DefId,
		gargs: GenericArgsRef<'tcx>,
		args: Args<'_>,
		span: Span,
		out: &mut Vec<Stmt>,
	) -> Expr {
		let tcx = self.t.tcx;
		let values = self.args(args, out);
		let parent = tcx.parent(def_id);
		let (adt_id, variant) = match tcx.def_kind(parent) {
			rustc_hir::def::DefKind::Variant => (tcx.parent(parent), tcx.adt_def(tcx.parent(parent)).variant_index_with_id(parent)),
			_ => (parent, rustc_abi::FIRST_VARIANT),
		};
		let adt = tcx.adt_def(adt_id);
		if let Some(std) = library::Std::of(tcx, adt_id) {
			let mut values = values;
			return match (std, variant.index()) {
				(library::Std::Option, 1) => values.pop().unwrap(),
				(library::Std::Result, 0) => Expr::rt("Ok", values),
				(library::Std::Result, _) => Expr::rt("Err", values),
				_ => self.err(span, format!("constructing `{}`", tcx.def_path_str(adt_id))),
			};
		}
		let info = self.t.adt(adt, span);
		let ty = tcx.type_of(adt_id).instantiate(tcx, gargs).skip_norm_wip();
		match info.repr {
			AdtRepr::Class { .. } => Expr::New(Box::new(Expr::Sym(info.sym)), values.into_iter().map(Expr::typed).collect()),
			AdtRepr::Tags => {
				let ts_ty = self.t.ty(ty, span);
				Expr::As(Box::new(Expr::Str(adt.variant(variant).name.to_string())), ts_ty)
			}
			AdtRepr::Union => {
				let mut props = vec![(Key::Name("tag".into()), Expr::Str(adt.variant(variant).name.to_string()))];
				for (i, value) in values.into_iter().enumerate() {
					props.push((Key::Name(format!("_{i}")), value));
				}
				let ts_ty = self.t.ty(ty, span);
				Expr::As(Box::new(Expr::Object(props)), ts_ty)
			}
		}
	}

	/// A function item as a value.
	pub fn fn_value(&mut self, def_id: rustc_hir::def_id::DefId, gargs: GenericArgsRef<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Expr {
		let tcx = self.t.tcx;
		let gargs = self.subst(gargs);
		let sig = tcx.instantiate_bound_regions_with_erased(tcx.fn_sig(def_id).instantiate(tcx, gargs).skip_norm_wip());
		let names: Vec<String> = (0..sig.inputs().len()).map(|i| self.fresh(&format!("a{i}"))).collect();
		let values: Vec<Expr> = names.iter().map(Expr::ident).collect();
		// A local function that is not a method is named directly.
		if !matches!(tcx.def_kind(def_id), rustc_hir::def::DefKind::Ctor(..))
			&& let Ok(Some(instance)) = Instance::try_resolve(tcx, self.t.typing_env_pub(), def_id, gargs)
			&& let ty::InstanceKind::Item(did) = instance.def
			&& did.is_local()
		{
			let sym = self.t.instance(instance);
			if !matches!(self.t.program.syms[sym].kind, SymKind::Method { instance: true }) {
				return Expr::Sym(sym);
			}
		}
		let mut body = Vec::new();
		let call = self.call_def(def_id, gargs, Args::Values(values), span, &mut body);
		body.push(StmtKind::Return(Some(call.typed())).at(None));
		let params = names
			.into_iter()
			.zip(sig.inputs())
			.map(|(name, ty)| {
				let ty = self.t.ty(*ty, span);
				Param { name, ty: Some(ty) }
			})
			.collect();
		let _ = out;
		let ret = self.t.ret_ty(sig.output(), span);
		Expr::Arrow(params, ret, body)
	}

	/// The integer a literal operand has, folding constant arithmetic.
	pub fn const_int(&self, id: ExprId) -> Option<i128> {
		match &self.thir[id].kind {
			ExprKind::Scope { value, .. } => self.const_int(*value),
			ExprKind::Literal { lit, neg } => match lit.node {
				rustc_ast::LitKind::Int(n, _) => {
					let n = n.get() as i128;
					Some(if *neg { -n } else { n })
				}
				_ => None,
			},
			ExprKind::NonHirLiteral { lit, .. } => {
				let ty = self.ty_of(id);
				let int = super::num::Int::of(ty)?;
				let size = rustc_abi::Size::from_bits(int.bits);
				Some(match int.signed {
					true => lit.to_int(size),
					false => lit.to_uint(size) as i128,
				})
			}
			ExprKind::Binary { op, lhs, rhs } => {
				let ty = self.ty_of(id);
				let int = super::num::Int::of(ty)?;
				super::num::fold(*op, self.const_int(*lhs)?, self.const_int(*rhs)?, int)
			}
			_ => None,
		}
	}
}

/// A JS object kind that a `&mut` can change in place.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ObjectKind {
	Class,
	Bytes,
	Array,
	Vec,
}

enum Iterable {
	Range { start: Expr, end: Expr },
	Of(Expr),
}

/// Whether a declaration's initializer is one TypeScript can't type on its own.
fn needs_annotation(value: &Expr) -> bool {
	match value {
		Expr::Array(items) => items.is_empty(),
		Expr::Lit(text) => text == "undefined",
		Expr::Arrow(..) => false,
		Expr::Located(_, e) => needs_annotation(e),
		_ => false,
	}
}

pub fn has_bindings(pat: &Pat<'_>) -> bool {
	let mut found = false;
	pat.walk_always(|p| {
		if let PatKind::Binding { .. } = p.kind {
			found = true;
		}
	});
	found
}

fn kind_name(kind: &ExprKind<'_>) -> String {
	let debug = format!("{kind:?}");
	debug.split([' ', '(', '{']).next().unwrap_or("?").to_string()
}
