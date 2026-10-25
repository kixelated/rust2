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
use rustc_middle::ty::{self, GenericArgsRef, Instance, Ty, TypeVisitableExt};
use rustc_span::{Span, Symbol};

use super::drop::Owned;
use super::library::{self, Args};
use super::{AdtRepr, ObjectKind, Translator, camel, escape};
use crate::program::SymKind;
use crate::ts::{self, Expr, Key, Param, Stmt, StmtKind, Target, Type};

/// Translate one function instance's body.
pub fn function<'a, 'tcx>(
	t: &mut Translator<'a, 'tcx>,
	instance: Instance<'tcx>,
	thir: &'a Thir<'tcx>,
	expr: ExprId,
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
		read: reads(tcx, thir),
		breakables: Vec::new(),
		ret: sig.output(),
		visited: FxHashSet::default(),
		scopes: Vec::new(),
		moved: FxHashSet::default(),
		borrowed: FxHashSet::default(),
		yields: 0,
		done: super::iter::is_next(tcx, def_id),
	};
	let span = tcx.def_span(def_id);
	if b.done {
		super::iter::check_impl(b.t, def_id);
	}
	let mut body = Vec::new();
	let mut params = Vec::new();
	b.open_scope();
	for param in thir.params.iter() {
		let ty = b.subst(param.ty);
		match &param.pat {
			Some(pat) => {
				let name = b.param(pat, &mut body);
				let ty = b.t.ty(ty, pat.span);
				params.push(Param { name, ty: Some(ty) });
			}
			None => {
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
	b.close_scope(&mut body, 0, span);
	// A `self` the body never names, such as a unit struct's derived `Display`, takes a `_`
	// so it passes `noUnusedParameters`.
	if let Some(param) = params.first_mut()
		&& param.name == "self"
		&& !ts::mentions(&body, "self")
	{
		param.name = "_self".into();
	}
	let ret = match (b.done, ret.kind()) {
		(true, ty::Adt(_, args)) => Type::Union(vec![b.t.ty(args.type_at(0), span), Type::Rt("Done", Vec::new())]),
		_ => b.t.ret_ty(ret, span),
	};
	Some(ts::Function {
		sym: 0,
		params,
		ret,
		body,
		loc: None,
		doc: None,
		generator: false,
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
	Name {
		name: String,
		mutable: bool,
	},
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
	/// Every variable the function and its closures read.
	read: FxHashSet<HirId>,
	breakables: Vec<Breakable>,
	/// The return type of the function or closure being lowered.
	ret: Ty<'tcx>,
	/// Expressions lowered so far, by THIR and id, so a refused call lowers only the
	/// arguments its mapping did not.
	visited: FxHashSet<(*const Thir<'tcx>, ExprId)>,
	/// The variables each open scope drops when it closes, innermost last.
	pub(super) scopes: Vec<Vec<Owned<'tcx>>>,
	/// The variables a move left `undefined`, which are declared `let` for it.
	pub(super) moved: FxHashSet<String>,
	/// The variables a generator lowered so far borrows mutably: each is a parameter
	/// holding the caller's object, so replacing it whole must change that object.
	borrowed: FxHashSet<HirId>,
	/// How many `yield`s are lowered so far, so an operand can tell whether it awaits.
	yields: usize,
	/// Whether this is an `Iterator::next`, which returns an item or `rt.DONE` rather
	/// than an `Option`: its closures are not.
	done: bool,
}

/// One `&&`-joined part of an `if` condition.
enum CondPart<'a, 'tcx> {
	Bool(ExprId),
	Let(ExprId, &'a Pat<'tcx>),
}

impl<'b, 'a, 'tcx> Body<'b, 'a, 'tcx> {
	/// A body with no THIR of its own: what a MIR lowering calls the library mappings
	/// through, passing their arguments already lowered. It names the locals.
	pub fn detached(t: &'b mut Translator<'a, 'tcx>, args: GenericArgsRef<'tcx>, ret: Ty<'tcx>) -> Self {
		let bodies: &'a super::Bodies<'tcx> = t.bodies;
		let thir = &bodies.empty;
		Body {
			t,
			thir,
			args,
			locals: FxHashMap::default(),
			names: FxHashSet::default(),
			read: FxHashSet::default(),
			breakables: Vec::new(),
			ret,
			visited: FxHashSet::default(),
			scopes: Vec::new(),
			moved: FxHashSet::default(),
			borrowed: FxHashSet::default(),
			yields: 0,
			done: false,
		}
	}

	pub fn subst<T: ty::TypeFoldable<ty::TyCtxt<'tcx>>>(&self, value: T) -> T {
		self.t.subst(self.args, value)
	}

	pub fn ty_of(&self, id: ExprId) -> Ty<'tcx> {
		self.subst(self.thir[id].ty)
	}

	/// Record that `id` is lowered, or is being.
	pub fn visit(&mut self, id: ExprId) {
		self.visited.insert((std::ptr::from_ref(self.thir), id));
	}

	fn visited(&self, id: ExprId) -> bool {
		self.visited.contains(&(std::ptr::from_ref(self.thir), id))
	}

	pub fn span(&self, id: ExprId) -> Span {
		self.thir[id].span
	}

	pub fn err(&mut self, span: Span, msg: impl Into<String>) -> Expr {
		self.t.err(span, msg);
		Expr::undefined()
	}

	/// Whether anything reads `var`, so its binding is worth declaring.
	pub fn is_read(&self, var: LocalVarId) -> bool {
		self.read.contains(&var.0)
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
			ExprKind::Scope { value, .. } | ExprKind::Borrow { arg: value, .. } => self.temp_base(*value),
			ExprKind::Call { ty, .. } => match self.subst(*ty).kind() {
				ty::FnDef(def_id, _) => camel(self.t.tcx.item_name(*def_id).as_str()),
				_ => "value".into(),
			},
			_ => match self.ty_of(id).kind() {
				ty::Adt(adt, _) => {
					let name = self.t.tcx.item_name(adt.did()).to_string();
					let mut chars = name.chars();
					chars
						.next()
						.map_or(name.clone(), |c| c.to_lowercase().chain(chars).collect())
				}
				_ => "value".into(),
			},
		}
	}

	/// Declare a binding for a Rust variable of type `ty`, which its scope drops if it
	/// `owns` the value rather than borrowing it.
	pub(super) fn bind_var(
		&mut self,
		var: LocalVarId,
		name: Symbol,
		mutable: bool,
		ty: Ty<'tcx>,
		span: Span,
		owns: bool,
	) -> String {
		// A JS variable holding a number is a copy: writing through it reaches nothing.
		if let ty::Ref(_, inner, rustc_ast::Mutability::Mut) = self.subst(ty).kind()
			&& self.t.object_kind(*inner).is_none()
		{
			self.t.err(
				span,
				format!("a `&mut {inner}` binding: a JS value of `{inner}` is a copy, so it can't change"),
			);
		}
		let name = self.fresh(&camel(name.as_str()));
		self.locals.insert(
			var.0,
			Local::Name {
				name: name.clone(),
				mutable,
			},
		);
		if owns {
			let ty = self.subst(ty);
			self.own(&name, ty);
		}
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
				// A parameter nothing reads, such as `other` in a derived `eq` of two
				// `struct E {}`, takes a `_` so it passes `noUnusedParameters`.
				let name = match self.is_read(*var) || name.as_str().starts_with('_') {
					true => *name,
					false => Symbol::intern(&format!("_{name}")),
				};
				self.bind_var(*var, name, mutable, pat.ty, pat.span, true)
			}
			_ if self.t.drops(self.subst(pat.ty)) => {
				self.t.err(
					pat.span,
					"a parameter that drops bound by a pattern, which is not supported yet",
				);
				self.fresh("arg")
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
			Dest::Return if self.done => {
				if let ty::Adt(_, args) = self.ret.kind() {
					super::iter::option_ok(self, args.type_at(0), span);
				}
				let value = Expr::Rt("iter").member("fromOption").call(vec![value]);
				self.push(out, StmtKind::Return(Some(value)), span);
			}
			Dest::Return => self.push(out, StmtKind::Return(Some(value.typed())), span),
			Dest::Assign(name) => self.push(out, StmtKind::Expr(Expr::ident(name).assign(value.typed())), span),
		}
	}

	/// Lower `id` so its value goes to `dest`.
	pub fn stmt(&mut self, id: ExprId, dest: Dest, out: &mut Vec<Stmt>) {
		self.visit(id);
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
			ExprKind::Scope {
				value, region_scope, ..
			} => match self.scoped_loop(*value) {
				Some(body) => self.lp(*region_scope, body, dest, out),
				None => self.stmt(*value, dest, out),
			},
			ExprKind::ValueTypeAscription { source, .. } | ExprKind::ValueExpr { source } => {
				self.stmt(*source, dest, out)
			}
			ExprKind::NeverToAny { source } => self.stmt(*source, Dest::Discard, out),
			ExprKind::Block { block } => self.block(*block, dest, out),
			ExprKind::If {
				cond, then, else_opt, ..
			} => self.if_(*cond, *then, *else_opt, dest, span, out),
			ExprKind::Match {
				scrutinee,
				arms,
				match_source,
			} if !arms.is_empty() => self.match_(id, *scrutinee, arms, *match_source, dest, out),
			ExprKind::Loop { .. } => {
				self.err(span, "a loop outside a scope");
			}
			ExprKind::Break { label, value } => self.break_(*label, *value, span, out),
			ExprKind::Continue { label } => {
				let label = self.target(*label, true);
				self.push(out, StmtKind::Continue(label), span);
			}
			ExprKind::Return { value } => match value {
				// `stmt` returns from every branch that makes a value; a `!` value makes none.
				Some(value) if !self.ty_of(*value).is_unit() => self.stmt(*value, Dest::Return, out),
				Some(value) => {
					self.stmt(*value, Dest::Discard, out);
					self.push(out, StmtKind::Return(None), span);
				}
				None => self.push(out, StmtKind::Return(None), span),
			},
			ExprKind::Assign { lhs, rhs } => self.assign(*lhs, *rhs, span, out),
			ExprKind::AssignOp { op, lhs, rhs } => self.assign_op(*op, *lhs, *rhs, span, out),
			_ => {
				if self.done
					&& let Dest::Return = dest
					&& let Some(value) = self.done_value(id, out)
				{
					self.push(out, StmtKind::Return(Some(value)), span);
					return;
				}
				let value = self.expr(id, out);
				// A value that goes nowhere drops here.
				if let Dest::Discard = dest
					&& let Some(dropped) = self.t.drop_value(value.clone(), ty, span)
				{
					self.push(out, StmtKind::Expr(dropped), span);
					return;
				}
				self.deliver(value, &dest, span, out);
			}
		}
	}

	/// What an `Iterator::next` returns for `id` without an `Option` between: `rt.DONE`
	/// for `None`, the item for `Some(item)`, and another iterator's `next` as is.
	fn done_value(&mut self, id: ExprId, out: &mut Vec<Stmt>) -> Option<Expr> {
		match &self.thir[id].kind {
			ExprKind::Adt(adt) if library::Std::of(self.t.tcx, adt.adt_def.did()) == Some(library::Std::Option) => {
				self.visit(id);
				Some(match adt.fields.first() {
					None => Expr::Rt("DONE"),
					Some(field) => self.operand(field.expr, out).typed(),
				})
			}
			_ => self.raw_next(id, out),
		}
	}

	/// `id` lowered by the iterator protocol if it is a call to `Iterator::next`: an item
	/// or `rt.DONE`, rather than an `Option`.
	fn raw_next(&mut self, id: ExprId, out: &mut Vec<Stmt>) -> Option<Expr> {
		let ExprKind::Call { ty, args, .. } = &self.thir[self.strip(id)].kind else {
			return None;
		};
		let ty::FnDef(def_id, _) = *self.subst(*ty).kind() else {
			return None;
		};
		if !super::iter::is_next(self.t.tcx, def_id) {
			return None;
		}
		self.visit(id);
		let recv_ty = self.ty_of(args[0]);
		let recv = self.operand(args[0], out);
		Some(super::iter::next(self, recv_ty, recv))
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
		let start = out.len();
		self.open_scope();
		for &stmt in &block.stmts {
			self.stmt_item(stmt, out);
		}
		if let Some(expr) = block.expr {
			self.stmt(expr, dest, out);
		}
		self.close_scope(out, start, block.span);
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

	fn let_(
		&mut self,
		pat: &'a Pat<'tcx>,
		init: Option<ExprId>,
		els: Option<BlockId>,
		span: Span,
		out: &mut Vec<Stmt>,
	) {
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
			let by_value = mode.0 == rustc_hir::ByRef::No;
			let ty = self.t.ty(pat_ty, pat.span);
			let Some(init) = init else {
				let name = self.bind_var(*var, *name, true, pat_ty, pat.span, by_value);
				self.push(out, StmtKind::decl(true, name, Some(ty), None), span);
				return;
			};
			// A `ref` binding borrows a place where it is, and keeps a temporary alive to the
			// end of the block, which then drops it as it would a variable.
			let borrows = !by_value && self.is_place(init);
			let owns = !borrows;
			if self.needs_dest(init) {
				let name = self.bind_var(*var, *name, true, pat_ty, pat.span, owns);
				self.push(out, StmtKind::decl(true, name.clone(), Some(ty), None), span);
				self.stmt(init, Dest::Assign(name), out);
				return;
			}
			// A move from another variable or field: it is left `undefined`.
			if by_value && self.moves(init) {
				let place = self.place(init, out);
				let name = self.bind_var(*var, *name, mutable, pat_ty, pat.span, owns);
				let annotate = mutable || diverges_when_called(pat_ty);
				self.push(
					out,
					StmtKind::decl(mutable, name, annotate.then_some(ty), Some(place.clone())),
					span,
				);
				self.clear(place, out);
				return;
			}
			let value = match borrows {
				true => self.place(init, out),
				false => self.operand(init, out),
			};
			let name = self.bind_var(*var, *name, mutable, pat_ty, pat.span, owns);
			let annotate = mutable || needs_annotation(&value) || diverges_when_called(pat_ty);
			self.push(
				out,
				StmtKind::decl(mutable, name, annotate.then_some(ty), Some(value)),
				span,
			);
			return;
		}

		let Some(init) = init else {
			// `let _: T;`, as derives write to assert a bound, binds nothing.
			if !matches!(pat.kind, PatKind::Wild) {
				self.t.err(span, "a pattern binding without an initializer");
			}
			return;
		};
		// `let _ = value;` drops a value at once, and evaluates a place without moving it:
		// an index still runs, and checks its bounds.
		if let PatKind::Wild = pat.kind
			&& els.is_none()
			&& self.t.drops(pat_ty)
		{
			match self.is_place(init) {
				true => {
					let place = self.place(init, out);
					if !place.is_path() {
						self.push(out, StmtKind::Expr(place), span);
					}
				}
				false => {
					let value = self.operand(init, out);
					if let Some(dropped) = self.t.drop_value(value, pat_ty, span) {
						self.push(out, StmtKind::Expr(dropped), span);
					}
				}
			}
			return;
		}
		// What the pattern leaves unbound drops with its place, or at the end of the
		// statement from a temporary, which nothing marks. The shortcut below would move a
		// place out whole, so a place binds part by part instead.
		let leaves = self.leaves_drop(pat);
		if leaves && !self.is_place(init) {
			self.t.err(
				span,
				"a `let` on a temporary that drops, which it doesn't move out of whole",
			);
		}
		// `const [a, b] = ...` for a tuple or array of plain bindings.
		if els.is_none() && !leaves && self.destructurable(pat, true) {
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
					self.push(
						out,
						StmtKind::If {
							cond: test.not(),
							then: body,
							els: Vec::new(),
						},
						span,
					);
				}
				None => self.t.err(span, "`let ... else` with an irrefutable pattern"),
			}
		} else if self.pat_test(pat, &scrut).is_some() {
			self.t.err(span, "a refutable pattern in `let`");
		}
		self.pat_bind(pat, &scrut, owned, out);
	}

	/// Whether binding `pat` by value leaves part of a value that drops unbound: under a
	/// `_` or a `..`, in a field it doesn't name, or in another variant than the one it
	/// matches.
	fn leaves_drop(&mut self, pat: &Pat<'tcx>) -> bool {
		let ty = self.subst(pat.ty);
		if !self.t.drops(ty) {
			return false;
		}
		let tcx = self.t.tcx;
		match &pat.kind {
			PatKind::Binding {
				mode, subpattern: None, ..
			} => mode.0 != rustc_hir::ByRef::No,
			PatKind::Leaf { subpatterns } | PatKind::Variant { subpatterns, .. } => {
				let (variant, others) = match (ty.kind(), &pat.kind) {
					(ty::Tuple(tys), _) => (tys.to_vec(), Vec::new()),
					(ty::Adt(adt, args), PatKind::Variant { variant_index, .. }) => {
						let fields = |v: &ty::VariantDef| -> Vec<Ty<'tcx>> {
							v.fields.iter().map(|f| f.ty(tcx, args).skip_norm_wip()).collect()
						};
						let others = adt
							.variants()
							.iter_enumerated()
							.filter(|(i, _)| i != variant_index)
							.flat_map(|(_, v)| fields(v))
							.collect();
						(fields(adt.variant(*variant_index)), others)
					}
					(ty::Adt(adt, args), _) => (
						adt.non_enum_variant()
							.fields
							.iter()
							.map(|f| f.ty(tcx, args).skip_norm_wip())
							.collect(),
						Vec::new(),
					),
					_ => return true,
				};
				others.into_iter().any(|t| self.t.drops(t))
					|| variant.into_iter().enumerate().any(|(i, field)| {
						match subpatterns.iter().find(|f| f.field.index() == i) {
							Some(f) => self.leaves_drop(&f.pattern),
							None => self.t.drops(field),
						}
					})
			}
			PatKind::Array {
				prefix,
				slice: None,
				suffix,
			} => prefix.iter().chain(suffix.iter()).any(|p| self.leaves_drop(p)),
			_ => true,
		}
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
					&& subpatterns
						.iter()
						.enumerate()
						.all(|(i, f)| f.field.index() == i && self.destructurable(&f.pattern, false))
			}
			// Bytes are a `Uint8Array`, which JS destructures through its iterator: indexing
			// reads them faster.
			PatKind::Array {
				prefix,
				slice: None,
				suffix,
			} => {
				!matches!(ty.kind(), ty::Array(elem, _) if matches!(elem.kind(), ty::Uint(ty::UintTy::U8)))
					&& suffix.is_empty()
					&& prefix.iter().all(|p| self.destructurable(p, false))
			}
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
			PatKind::Leaf { subpatterns } => Target::Array(
				subpatterns
					.iter()
					.map(|f| self.destructure_elem(&f.pattern, mutable))
					.collect(),
			),
			PatKind::Array { prefix, .. } => {
				Target::Array(prefix.iter().map(|p| self.destructure_elem(p, mutable)).collect())
			}
			PatKind::Binding { name, var, mode, .. } => {
				let is_mut = mode.1 == rustc_ast::Mutability::Mut;
				*mutable |= is_mut;
				Target::Name(self.bind_var(*var, *name, is_mut, pat.ty, pat.span, mode.0 == rustc_hir::ByRef::No))
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

	/// Whether `id` is control flow that reads best delivering into a variable. One that
	/// never makes a value leaves the variable unassigned.
	fn needs_dest(&mut self, id: ExprId) -> bool {
		match &self.thir[id].kind {
			ExprKind::Scope { value, .. } => self.needs_dest(*value),
			ExprKind::If {
				then, else_opt, cond, ..
			} => !(self.simple(*then) && else_opt.is_some_and(|e| self.simple(e)) && self.simple(*cond)),
			ExprKind::Match { match_source, .. } => {
				!matches!(match_source, MatchSource::TryDesugar(_)) && !self.ternary_match(id)
			}
			ExprKind::Loop { .. } | ExprKind::NeverToAny { .. } => true,
			_ => false,
		}
	}

	/// Whether `id` lowers to an expression with no statements before it.
	fn simple(&mut self, id: ExprId) -> bool {
		self.simple_at(id, false)
	}

	/// [`Self::simple`], for `id` read as a place, which a move out of would clear.
	fn simple_at(&mut self, id: ExprId, place: bool) -> bool {
		// A move leaves `undefined` behind with a statement of its own.
		if !place && self.moves(id) {
			return false;
		}
		match &self.thir[id].kind {
			ExprKind::Scope { value, .. }
			| ExprKind::ValueTypeAscription { source: value, .. }
			| ExprKind::PlaceTypeAscription { source: value, .. } => self.simple_at(*value, place),
			ExprKind::Borrow { arg: value, .. }
			| ExprKind::Deref { arg: value }
			| ExprKind::Field { lhs: value, .. } => self.simple_at(*value, true),
			ExprKind::Cast { source: value }
			| ExprKind::PointerCoercion { source: value, .. }
			| ExprKind::Unary { arg: value, .. } => self.simple(*value),
			ExprKind::Block { block } => {
				let block = &self.thir[*block];
				block.stmts.is_empty() && !block.targeted_by_break && block.expr.is_some_and(|e| self.simple(e))
			}
			ExprKind::Binary { lhs, rhs, .. } => self.simple(*lhs) && self.simple(*rhs),
			ExprKind::Index { lhs, index } => self.simple_at(*lhs, true) && self.simple(*index),
			ExprKind::LogicalOp { lhs, rhs, .. } => self.simple(*lhs) && self.simple(*rhs),
			ExprKind::Call { args, fun, .. } => self.simple(*fun) && args.iter().all(|a| self.simple(*a)),
			ExprKind::Tuple { fields } | ExprKind::Array { fields } => fields.iter().all(|f| self.simple(*f)),
			// Fields written out of declaration order may be spilled to keep their order.
			ExprKind::Adt(adt) => {
				adt.fields.iter().all(|f| self.simple(f.expr))
					&& adt.fields.is_sorted_by_key(|f| f.name)
					&& matches!(adt.base, thir::AdtExprBase::None)
			}
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
			// `stmt` returned from every branch that makes a value; a `!` value makes none.
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
		self.push(
			out,
			StmtKind::While {
				label,
				cond,
				body: stmts,
			},
			span,
		);
	}

	/// The body of the loop a scope holds, if it holds one. A loop that only `return`
	/// leaves has type `!`, coerced to whatever its place needs.
	fn scoped_loop(&self, value: ExprId) -> Option<ExprId> {
		let value = match self.thir[value].kind {
			ExprKind::NeverToAny { source } => source,
			_ => value,
		};
		match self.thir[value].kind {
			ExprKind::Loop { body } => Some(body),
			_ => None,
		}
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
		let ExprKind::If {
			cond,
			then,
			else_opt: Some(els),
			..
		} = self.thir[self.strip(block.expr?)].kind
		else {
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
			ExprKind::Scope { value, .. } | ExprKind::NeverToAny { source: value } => self.strip(value),
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

	fn cond_parts(&self, id: ExprId, parts: &mut Vec<CondPart<'a, 'tcx>>) {
		let id = self.strip(id);
		let thir: &'a Thir<'tcx> = self.thir;
		match &thir[id].kind {
			ExprKind::Let { expr, pat } => parts.push(CondPart::Let(*expr, pat)),
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

	/// Lower one part of a condition: its statements to `pre`, the bindings a `let`
	/// makes once it passed to `binds`, and the test, if it can fail.
	fn cond_part(&mut self, part: &CondPart<'a, 'tcx>, pre: &mut Vec<Stmt>, binds: &mut Vec<Stmt>) -> Option<Expr> {
		match part {
			CondPart::Bool(id) => Some(self.expr(*id, pre)),
			CondPart::Let(expr, pat) => {
				// A failed test leaves the value whole, so the pattern must take all of it
				// from every variant: a `let` with no other arm to take the rest.
				if !self.is_place(*expr) && self.leaves_drop(pat) {
					self.t.err(
						pat.span,
						"an `if let` on a temporary that drops, which it doesn't move out of whole",
					);
				}
				let reads = self.test_reads(pat) + bindings(pat);
				let (scrut, owned) = self.scrutinee(*expr, reads, pre);
				let test = self.pat_test(pat, &scrut);
				self.pat_bind(pat, &scrut, owned, binds);
				test
			}
		}
	}

	/// A match's scrutinee, which its patterns read `reads` times: as a path they can
	/// read repeatedly, or as is when they read it at most once, first. Also whether it
	/// is a temporary.
	fn scrutinee(&mut self, id: ExprId, reads: usize, out: &mut Vec<Stmt>) -> (Expr, bool) {
		let owned = !self.is_place(id);
		let value = match owned {
			true => self.operand(id, out),
			false => self.place(id, out),
		};
		match reads {
			// Unread, it still runs.
			0 if !value.is_path() => {
				out.push(StmtKind::Expr(value).at(None));
				(Expr::undefined(), owned)
			}
			0 | 1 => (value, owned),
			_ => (self.path(value, id, out), owned),
		}
	}

	/// How many times the test of `pat` reads its scrutinee.
	fn test_reads(&self, pat: &Pat<'tcx>) -> usize {
		match &pat.kind {
			PatKind::Wild | PatKind::Missing => 0,
			PatKind::Binding { subpattern, .. } => subpattern.as_ref().map_or(0, |p| self.test_reads(p)),
			PatKind::Deref { subpattern, .. } => self.test_reads(subpattern),
			PatKind::Leaf { subpatterns } => subpatterns.iter().map(|f| self.test_reads(&f.pattern)).sum(),
			PatKind::Variant { subpatterns, .. } => {
				1 + subpatterns.iter().map(|f| self.test_reads(&f.pattern)).sum::<usize>()
			}
			PatKind::Or { pats } => pats.iter().map(|p| self.test_reads(p)).sum(),
			PatKind::Array { prefix, suffix, .. } => {
				prefix.iter().chain(suffix.iter()).map(|p| self.test_reads(p)).sum()
			}
			PatKind::Constant { .. } => 1,
			PatKind::Range(range) => usize::from(range.lo.is_finite()) + usize::from(range.hi.is_finite()),
			// Ranges, slices, and anything else: assume more than once.
			_ => 2,
		}
	}

	fn if_(&mut self, cond: ExprId, then: ExprId, els: Option<ExprId>, dest: Dest, span: Span, out: &mut Vec<Stmt>) {
		let mut parts = Vec::new();
		self.cond_parts(cond, &mut parts);

		// One condition, as JS writes it.
		if let [part] = parts.as_slice() {
			let mut then_stmts = Vec::new();
			self.open_scope();
			let test = self.cond_part(part, out, &mut then_stmts);
			self.stmt(then, dest.clone(), &mut then_stmts);
			self.close_scope(&mut then_stmts, 0, span);
			let mut else_stmts = Vec::new();
			if let Some(els) = els {
				self.stmt(els, dest, &mut else_stmts);
			}
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
				// An irrefutable `let`: the else never runs.
				None => out.extend(then_stmts),
			}
			return;
		}

		// A let chain: each part nests in the last, and any failing runs the else.
		let label = self.fresh("chain");
		let mut levels = Vec::new();
		self.open_scope();
		for part in &parts {
			let mut pre = Vec::new();
			let mut binds = Vec::new();
			let test = self.cond_part(part, &mut pre, &mut binds);
			levels.push((pre, test, binds));
		}
		if self.scope_owns() {
			self.t.err(span, "a let chain that binds a value that drops");
		}
		self.scopes.pop();
		let mut then_stmts = Vec::new();
		self.stmt(then, dest.clone(), &mut then_stmts);
		let mut else_stmts = Vec::new();
		if let Some(els) = els {
			self.stmt(els, dest, &mut else_stmts);
		}
		let has_else = !else_stmts.is_empty();
		if has_else && !ts::diverges(&then_stmts) {
			then_stmts.push(StmtKind::Break(Some(label.clone())).at(None));
		}
		let mut body = then_stmts;
		for (pre, test, binds) in levels.into_iter().rev() {
			let mut inner = binds;
			inner.extend(body);
			let mut wrapped = pre;
			match test {
				Some(test) => wrapped.push(
					StmtKind::If {
						cond: test,
						then: inner,
						els: Vec::new(),
					}
					.at(None),
				),
				None => wrapped.extend(inner),
			}
			body = wrapped;
		}
		match has_else {
			true => {
				body.extend(else_stmts);
				self.push(
					out,
					StmtKind::Block {
						label: Some(label),
						body,
					},
					span,
				);
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
				let value = self.try_(scrutinee, span, out);
				self.deliver(value, &dest, span, out);
				return;
			}
			MatchSource::ForLoopDesugar => {
				self.for_(scrutinee, arms, span, out);
				return;
			}
			_ => {}
		}

		let thir: &'a Thir<'tcx> = self.thir;
		// One arm binding the whole value, as `.await` desugars to, is a `let` scoped to it.
		if let [arm] = arms
			&& thir[*arm].guard.is_none()
			&& let PatKind::Binding {
				mode, subpattern: None, ..
			} = thir[*arm].pattern.kind
			&& mode.0 == rustc_hir::ByRef::No
		{
			let start = out.len();
			self.open_scope();
			self.let_(&thir[*arm].pattern, Some(scrutinee), None, span, out);
			self.stmt(thir[*arm].body, dest, out);
			self.close_scope(out, start, span);
			return;
		}
		if !self.is_place(scrutinee)
			&& self.t.drops(self.ty_of(scrutinee))
			&& !arms
				.iter()
				.all(|&arm| thir[arm].guard.is_none() && takes_all(&thir[arm].pattern))
		{
			self.t.err(
				span,
				"a `match` on a temporary that drops, which it doesn't move out of whole",
			);
		}
		let pats: Vec<&Pat<'tcx>> = arms.iter().map(|&arm| &*thir[arm].pattern).collect();
		// The last arm is the else, whose test never runs, unless it is guarded.
		let tested = match arms.last().is_some_and(|&arm| thir[arm].guard.is_none()) {
			true => pats.len() - 1,
			false => pats.len(),
		};
		let first_reads = pats.first().map_or(0, |p| self.test_reads(p));
		let later_reads: usize = pats.iter().take(tested).skip(1).map(|p| self.test_reads(p)).sum();
		let binds: usize = pats.iter().map(|p| bindings(p)).sum();
		let reads = match first_reads <= 1 && later_reads == 0 && binds == 0 {
			true => first_reads,
			false => 2,
		};
		let (scrut, owned) = self.scrutinee(scrutinee, reads, out);

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
			self.open_scope();
			self.pat_bind(&arm.pattern, &scrut, owned, &mut binds);
			let guard = arm.guard.map(|g| {
				let mut pre = Vec::new();
				let e = self.expr(g, &mut pre);
				(pre, e)
			});
			let mut body = Vec::new();
			self.stmt(arm.body, dest.clone(), &mut body);
			// What the arm binds that drops is dropped around its bindings and body.
			if self.scope_owns() {
				if guard.is_some() {
					self.t.err(arm.span, "a guarded arm that binds a value that drops");
				}
				binds.append(&mut body);
				body = std::mem::take(&mut binds);
			}
			self.close_scope(&mut body, 0, arm.span);
			lowered.push(Lowered {
				test,
				binds,
				guard,
				body,
			});
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
			let guard_free = arm.guard.is_none();
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
			// The last arm without a guard is what Rust proved is left.
			let test = match i + 1 == count && guard_free {
				true => None,
				false => arm.test,
			};
			match test {
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
		self.push(
			out,
			StmtKind::Block {
				label: Some(label),
				body,
			},
			span,
		);
	}

	/// Whether a `match` reads as a ternary chain: no bindings or guards, and arms that
	/// are each a simple value.
	fn ternary_match(&mut self, id: ExprId) -> bool {
		let ExprKind::Match { arms, .. } = &self.thir[self.strip(id)].kind else {
			return false;
		};
		arms.iter().all(|&arm| {
			let arm = &self.thir[arm];
			arm.guard.is_none() && !has_bindings(&arm.pattern) && self.simple(arm.body)
		})
	}

	/// `expr?`: the value, returning early on `Err` or `None`.
	fn try_(&mut self, scrutinee: ExprId, span: Span, out: &mut Vec<Stmt>) -> Expr {
		// The scrutinee is `Try::branch(expr)`.
		let ExprKind::Call { args, .. } = &self.thir[self.strip(scrutinee)].kind else {
			return self.err(span, "an unexpected `?` desugaring");
		};
		let inner = args[0];
		let inner_ty = self.ty_of(inner);
		// `it.next()?` tests for the end itself, so its item may be an `Option`.
		let raw = self.raw_next(inner, out);
		let none = match raw {
			Some(_) => Expr::Rt("DONE"),
			None => Expr::undefined(),
		};
		let value = match raw {
			Some(value) => value,
			None => self.operand(inner, out),
		};
		let name = match value {
			Expr::Ident(ref name) => name.clone(),
			value => {
				let name = self.temp_for(inner);
				self.push(out, StmtKind::decl(false, name.clone(), None, Some(value)), span);
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
						cond: t.clone().binary("===", none),
						then: vec![
							StmtKind::Return(Some(match self.done {
								true => Expr::Rt("DONE"),
								false => Expr::undefined(),
							}))
							.at(None),
						],
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
					ty::Adt(ret_adt, ret_args)
						if library::Std::of(self.t.tcx, ret_adt.did()) == Some(library::Std::Result) =>
					{
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
		let Some(from_fn) = tcx
			.associated_items(from_trait)
			.in_definition_order()
			.find(|i| i.is_fn())
		else {
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
		let ExprKind::Scope {
			region_scope, value, ..
		} = self.thir[arm.body].kind
		else {
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
		let Some(some_arm) = next_arms
			.iter()
			.map(|a| &self.thir[*a])
			.find(|a| matches!(a.pattern.kind, PatKind::Variant { variant_index, .. } if variant_index.index() == 1))
		else {
			return bad(self);
		};
		let PatKind::Variant { subpatterns, .. } = &some_arm.pattern.kind else {
			return bad(self);
		};
		let pat = &subpatterns[0].pattern;
		let loop_body = some_arm.body;

		let iter_ty = self.ty_of(iterable);
		let source = match self.iterable(iterable, iter_ty, span, out) {
			Some(source) => source,
			// Refused, but what the iterable and the body refuse is reported too.
			None => Iterable::Of(self.operand(iterable, out)),
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
		// The iterator drops after the loop, with what it did not yield.
		if self.t.drops(self.subst(pat.ty)) || self.t.drops(iter_ty) {
			self.t.err(
				pat.span,
				"a `for` loop over values that drop, which is not supported yet",
			);
		}
		self.open_scope();
		let simple_name = match &pat.kind {
			PatKind::Binding {
				name,
				var,
				mode,
				subpattern: None,
				..
			} if mode.1 == rustc_ast::Mutability::Not && !self.needs_copy(self.subst(pat.ty)) => {
				Some(self.bind_var(*var, *name, false, pat.ty, pat.span, mode.0 == rustc_hir::ByRef::No))
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
		self.close_scope(&mut stmts, 0, span);
		let breakable = self.breakables.pop().unwrap();
		let label = breakable.used.then_some(breakable.label);
		let kind = match source {
			Iterable::Range { start, end, inclusive } => StmtKind::ForRange {
				label,
				name,
				start,
				end,
				inclusive,
				body: stmts,
			},
			Iterable::Of(iter) => StmtKind::ForOf {
				label,
				name,
				iter,
				body: stmts,
			},
			// `while (true) { const x = iter.next(); if (x === rt.DONE) break; ... }`
			Iterable::Next(next) => {
				let mut body = vec![
					StmtKind::decl(false, name.clone(), None, Some(next)).at(None),
					StmtKind::If {
						cond: Expr::ident(&name).binary("===", Expr::Rt("DONE")),
						then: vec![StmtKind::Break(None).at(None)],
						els: Vec::new(),
					}
					.at(None),
				];
				body.extend(stmts);
				StmtKind::While {
					label,
					cond: Expr::lit("true"),
					body,
				}
			}
		};
		self.push(out, kind, span);
	}

	/// What a `for` loop iterates.
	fn iterable(&mut self, id: ExprId, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Option<Iterable> {
		let tcx = self.t.tcx;
		match ty.kind() {
			ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), rustc_attr_ir::LangItem::Range) => {
				let ExprKind::Adt(range) = &self.thir[self.strip(id)].kind else {
					self.t
						.err(span, "a `for` loop over a range that is not written in place");
					return None;
				};
				let ids: Vec<ExprId> = range.fields.iter().map(|f| f.expr).collect();
				let mut values = self.operands(&ids, out);
				// `end` is read on every turn, so unless constant it is spilled, after what is
				// written before it.
				let swapped = range.fields[0].name.index() == 1;
				let end_at = if swapped { 0 } else { 1 };
				if !values[end_at].is_constant() {
					for (i, value) in values.iter_mut().enumerate().take(end_at + 1) {
						if !value.is_constant() {
							let saved = std::mem::replace(value, Expr::undefined());
							*value = self.spill(saved, if i == end_at { "end" } else { "start" }, out);
						}
					}
				}
				if swapped {
					values.swap(0, 1);
				}
				let end = values.pop().unwrap();
				let start = values.pop().unwrap();
				Some(range_of(ty, start, end, false))
			}
			ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), rustc_attr_ir::LangItem::RangeInclusiveStruct) => {
				// `a..=b` is `RangeInclusive::new(a, b)`.
				let ExprKind::Call { args, .. } = &self.thir[self.strip(id)].kind else {
					self.t
						.err(span, "a `for` loop over a range that is not written in place");
					return None;
				};
				let args = args.clone();
				let mut values = self.operands(&args, out);
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
				Some(range_of(ty, start, end, true))
			}
			// An array by value iterates its elements, already copied.
			ty::Array(..) => Some(Iterable::Of(self.operand(id, out))),
			ty::Ref(_, inner, _) if matches!(inner.kind(), ty::Slice(_) | ty::Array(..)) => {
				Some(Iterable::Of(self.operand(id, out)))
			}
			ty::Ref(_, inner, _) if self.is_vec(*inner) => {
				let v = self.operand(id, out);
				Some(Iterable::Of(library::vec_view(v, *inner)))
			}
			ty::Adt(..) if self.is_vec(ty) => {
				let v = self.operand(id, out);
				Some(Iterable::Of(library::vec_view(v, ty)))
			}
			// `slice.iter()` iterates as the slice itself does.
			_ if let Some(slice) = library::iterated_slice(self, id) => {
				self.visit(id);
				Some(Iterable::Of(self.operand(slice, out)))
			}
			// Any other iterator, by the protocol.
			_ => {
				let value = self.operand(id, out);
				let iterator = self.t.iter_item(ty).is_some();
				let value = match iterator {
					true => value,
					false => match super::iter::into_iter(self, ty, value, span) {
						Some(value) => value,
						None => {
							self.t.err(span, format!("a `for` loop over `{ty}`"));
							return None;
						}
					},
				};
				let iter = match value.is_path() {
					true => value,
					false => self.spill(value, "iter", out),
				};
				Some(Iterable::Next(match iterator {
					true => super::iter::next(self, ty, iter),
					false => iter.member("next").call(Vec::new()),
				}))
			}
		}
	}

	fn is_vec(&self, ty: Ty<'tcx>) -> bool {
		matches!(ty.kind(), ty::Adt(adt, _) if library::Std::of(self.t.tcx, adt.did()) == Some(library::Std::Vec))
	}

	pub fn spill(&mut self, value: Expr, base: &str, out: &mut Vec<Stmt>) -> Expr {
		let name = self.fresh(base);
		out.push(StmtKind::decl(false, name.clone(), None, Some(value)).at(None));
		Expr::ident(name)
	}

	/// Fields' values by index, from the order they are written to declaration order.
	/// Each that would then evaluate after one written later is spilled first, unless
	/// constant, so they still evaluate as written.
	pub fn declared(&mut self, mut written: Vec<(usize, Expr)>, out: &mut Vec<Stmt>) -> Vec<(usize, Expr)> {
		// The longest tail already in declaration order evaluates in place, after the rest.
		let mut tail = written.len().saturating_sub(1);
		while tail > 0 && written[tail - 1].0 < written[tail].0 {
			tail -= 1;
		}
		for (_, value) in &mut written[..tail] {
			if !value.is_constant() {
				let saved = std::mem::replace(value, Expr::undefined());
				*value = self.spill(saved, "field", out);
			}
		}
		written.sort_by_key(|(i, _)| *i);
		written
	}

	/// `value`, from `id`, as something patterns can read more than once.
	fn path(&mut self, value: Expr, id: ExprId, out: &mut Vec<Stmt>) -> Expr {
		if value.is_path() {
			return value;
		}
		let name = self.temp_for(id);
		out.push(StmtKind::decl(false, name.clone(), None, Some(value)).at(None));
		Expr::ident(name)
	}

	fn assign(&mut self, lhs: ExprId, rhs: ExprId, span: Span, out: &mut Vec<Stmt>) {
		let mut value = self.operand(rhs, out);
		let mut pre = Vec::new();
		let yields = self.yields;
		let target = self.place(lhs, &mut pre);
		// The value waits out the `.await` where no scope drops it if the future is cancelled.
		if self.yields != yields && self.t.drops(self.ty_of(rhs)) {
			self.t.err(
				self.span(lhs),
				"an `.await` in the place an assignment of a value that drops writes to, which is not supported yet: bind the awaited value first",
			);
		}
		if (!pre.is_empty() || !target.is_path()) && !value.is_constant() {
			value = self.spill(value, "value", out);
		}
		out.extend(pre);
		// The old value drops once the new one is made.
		let lty = self.ty_of(lhs);
		if self.t.drops(lty) {
			if !target.is_path() && !matches!(target, Expr::Index(..)) {
				self.t
					.err(span, "an assignment that drops the old value of a computed place");
				return;
			}
			if !value.is_path() && !value.is_constant() {
				value = self.spill(value, "value", out);
			}
			if let Some(dropped) = self.t.drop_value(target.clone(), lty, span) {
				self.push(out, StmtKind::Expr(dropped), span);
			}
		}
		// `*r = v` through a reference replaces what it points to, as does replacing a
		// variable a generator borrows mutably.
		let through_ref = match self.thir[self.strip(lhs)].kind {
			ExprKind::Deref { arg } => matches!(self.ty_of(arg).kind(), ty::Ref(..)) && self.is_place(arg),
			ExprKind::UpvarRef { var_hir_id, .. } => self.borrowed.contains(&var_hir_id.0),
			_ => false,
		};
		if through_ref {
			let pointee = self.ty_of(lhs);
			// Every clone of an `Arc` or waker is one JS object, a guard is its mutex, a
			// reference is its referent, and a future keeps its state private: copying fields
			// in would change the other handles, or nothing. A `Pin` or `Box` is its pointee
			// in JS.
			let mut erased = pointee;
			while let ty::Adt(adt, args) = erased.kind()
				&& matches!(
					library::Std::of(self.t.tcx, adt.did()),
					Some(library::Std::Pin | library::Std::Box)
				) {
				erased = args.type_at(0);
			}
			let shared = match erased.kind() {
				ty::Adt(adt, _) => matches!(
					library::Std::of(self.t.tcx, adt.did()),
					Some(library::Std::Arc | library::Std::Waker | library::Std::Context | library::Std::MutexGuard)
				),
				ty::Ref(..) | ty::Coroutine(..) => true,
				_ => false,
			};
			match self.t.object_kind(pointee) {
				Some(ObjectKind::Class) if shared => {
					self.t.err(
						span,
						format!("replacing the `{pointee}` behind a reference, whose JS object is shared or private"),
					);
				}
				Some(ObjectKind::Class) => {
					let e = Expr::ident("Object").member("assign").call(vec![target, value]);
					self.push(out, StmtKind::Expr(e), span);
				}
				Some(ObjectKind::Bytes) => {
					let e = target.member("set").call(vec![value]);
					self.push(out, StmtKind::Expr(e), span);
				}
				_ => {
					self.t
						.err(span, format!("an assignment through a reference to `{pointee}`"));
				}
			}
			return;
		}
		self.push(out, StmtKind::Expr(target.assign(value)), span);
	}

	fn assign_op(
		&mut self,
		op: rustc_middle::mir::AssignOp,
		lhs: ExprId,
		rhs: ExprId,
		span: Span,
		out: &mut Vec<Stmt>,
	) {
		// The value evaluates first, so it is saved before the place reads it again or
		// runs statements of its own.
		let mut value = self.operand(rhs, out);
		let mut pre = Vec::new();
		let target = self.place(lhs, &mut pre);
		if (!pre.is_empty() || !value.is_path()) && !value.is_constant() {
			value = self.spill(value, "value", out);
		}
		out.extend(pre);
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
		let rhs_value = super::num::Operand {
			expr: value,
			value: None,
		};
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
		// Whether an earlier operand is a value that drops, which a later `.await` would
		// hold where no scope drops it if the future is cancelled.
		let mut holds = false;
		for &id in ids {
			let mut pre = Vec::new();
			let yields = self.yields;
			let value = self.operand(id, &mut pre);
			if holds && self.yields != yields {
				self.t.err(
					self.span(id),
					"an `.await` in an operand after one that drops, which is not supported yet: bind the awaited value first",
				);
			}
			holds |= self.t.drops(self.ty_of(id));
			self.after(&mut values, pre, out);
			values.push(value);
		}
		values
	}

	/// Emit `pre`, the statements of an operand that evaluates after `earlier`, saving
	/// `earlier` first if there are any, so they cannot change it.
	pub fn after(&mut self, earlier: &mut [Expr], pre: Vec<Stmt>, out: &mut Vec<Stmt>) {
		if !pre.is_empty() {
			for value in earlier.iter_mut().filter(|v| !v.is_constant()) {
				let saved = std::mem::replace(value, Expr::undefined());
				*value = self.spill(saved, "arg", out);
			}
		}
		out.extend(pre);
	}

	/// Whether `id` names a place, rather than making a temporary value.
	pub fn is_place(&self, id: ExprId) -> bool {
		match &self.thir[id].kind {
			ExprKind::Scope { value, .. } | ExprKind::PlaceTypeAscription { source: value, .. } => {
				self.is_place(*value)
			}
			ExprKind::VarRef { .. } | ExprKind::UpvarRef { .. } | ExprKind::Field { .. } | ExprKind::Index { .. } => {
				true
			}
			// What a `Box` made in place points at is part of that temporary.
			ExprKind::Deref { arg } => matches!(self.ty_of(*arg).kind(), ty::Ref(..)) || self.is_place(*arg),
			_ => false,
		}
	}

	/// Whether `id` makes a temporary that drops, which only a statement could drop.
	fn temporary_drops(&mut self, id: ExprId) -> bool {
		!self.is_place(id) && self.t.drops(self.ty_of(id))
	}

	/// Whether lowering `id` for its value moves it out of a place that drops it.
	pub fn moves(&mut self, id: ExprId) -> bool {
		let ty = self.ty_of(id);
		self.is_place(id) && !self.t.is_copy(ty) && self.t.drops(ty)
	}

	/// Whether a block binds a variable that drops, so its value can't outlive it.
	fn block_owns(&mut self, id: BlockId) -> bool {
		let thir: &'a Thir<'tcx> = self.thir;
		let mut owns = false;
		for &stmt in &thir[id].stmts {
			if let thir::StmtKind::Let {
				pattern, initializer, ..
			} = &thir[stmt].kind
			{
				// A `ref` binding keeps a temporary alive to the end of the block.
				if let PatKind::Binding {
					mode, subpattern: None, ..
				} = pattern.kind
					&& mode.0 != rustc_hir::ByRef::No
					&& let Some(init) = *initializer
					&& !self.is_place(init)
				{
					owns |= self.t.drops(self.ty_of(init));
				}
				pattern.walk_always(|p| {
					if let PatKind::Binding { mode, ty, .. } = p.kind
						&& mode.0 == rustc_hir::ByRef::No
					{
						owns |= self.t.drops(self.t.subst(self.args, ty));
					}
				});
			}
		}
		owns
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
				// The runtime's methods hand out the payload itself.
				Some(library::Std::Option | library::Std::Result) => args.types().any(|t| self.mutable_aggregate(t)),
				Some(_) => false,
				None => adt.is_struct() && !adt.non_enum_variant().fields.is_empty(),
			},
			_ => false,
		}
	}

	pub(super) fn ref_binding(&mut self, by_ref: rustc_hir::ByRef, ty: Ty<'tcx>, span: Span) {
		if let rustc_hir::ByRef::Yes(_, rustc_ast::Mutability::Mut) = by_ref {
			let pointee = match ty.kind() {
				ty::Ref(_, inner, _) => *inner,
				_ => ty,
			};
			if self.t.object_kind(pointee).is_none() {
				self.t.err(
					span,
					format!("a `ref mut` binding to `{pointee}`, which is no object a reference can change"),
				);
			}
		}
	}

	/// Lower `id` for its value.
	pub fn expr(&mut self, id: ExprId, out: &mut Vec<Stmt>) -> Expr {
		self.visit(id);
		let e = &self.thir[id];
		let span = e.span;
		let ty = self.ty_of(id);
		if self.t.ignored_macro(span) {
			if !ty.is_unit() {
				return self.err(span, "an ignored macro must evaluate to `()`");
			}
			return Expr::undefined();
		}
		if self.is_format_args(id) {
			return self.format_args(id, out);
		}
		match &e.kind {
			ExprKind::Scope { value, .. } => match self.scoped_loop(*value) {
				Some(_) => self.via_temp(id, ty, span, out),
				None => self.expr(*value, out),
			},
			ExprKind::ValueTypeAscription { source, .. } | ExprKind::ValueExpr { source } => self.expr(*source, out),
			// The temp is never assigned: control never gets past the source.
			ExprKind::NeverToAny { .. } => self.via_temp(id, ty, span, out),
			// A block of type `!` without a value, such as `panic!(..)`, goes through a temp.
			ExprKind::Block { block }
				if !self.thir[*block].targeted_by_break
					&& !self.block_owns(*block)
					&& (self.thir[*block].expr.is_some() || !ty.is_never()) =>
			{
				let block = &self.thir[*block];
				for &stmt in &block.stmts {
					self.stmt_item(stmt, out);
				}
				match block.expr {
					Some(expr) => self.expr(expr, out),
					None => Expr::undefined(),
				}
			}
			ExprKind::If {
				cond, then, else_opt, ..
			} if self.simple(*then) && else_opt.is_some_and(|e| self.simple(e)) && !self.has_let(*cond) => {
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
				match_source: MatchSource::TryDesugar(_),
				..
			} => self.try_(*scrutinee, span, out),
			// A scrutinee that diverges, such as `match return x {}`, has no value to pass.
			ExprKind::Match { scrutinee, arms, .. } if arms.is_empty() && self.ty_of(*scrutinee).is_never() => {
				self.stmt(*scrutinee, Dest::Discard, out);
				Expr::undefined()
			}
			// A value of a type without values: native code never gets here.
			ExprKind::Match { scrutinee, arms, .. } if arms.is_empty() => {
				let value = self.operand(*scrutinee, out);
				Expr::rt("unreachable", vec![value])
			}
			ExprKind::Match { .. } if self.ternary_match(id) => self.ternary(id, out),
			ExprKind::Block { .. } | ExprKind::If { .. } | ExprKind::Match { .. } | ExprKind::Loop { .. } => {
				self.via_temp(id, ty, span, out)
			}
			ExprKind::Call {
				ty: fn_ty,
				fun,
				args,
				fn_span,
				..
			} => {
				let call = self.call(*fn_ty, *fun, args, *fn_span, out);
				match self.t.loc(span) {
					Some(loc) => Expr::Located(loc, Box::new(call)),
					None => call,
				}
			}
			// A place of type `!`, as in a generic instantiated at `!`, which native code never
			// reads: `tsc` sees that through `rt.unreachable`.
			ExprKind::VarRef { .. }
			| ExprKind::UpvarRef { .. }
			| ExprKind::Field { .. }
			| ExprKind::Index { .. }
			| ExprKind::Deref { .. }
			| ExprKind::PlaceTypeAscription { .. }
				if ty.is_never() =>
			{
				let place = self.place(id, out);
				Expr::rt("unreachable", vec![place])
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
					let ty::FnDef(def_id, args) = *ty.kind() else {
						unreachable!()
					};
					return self.fn_value(def_id, args.skip_binder(), span);
				}
				if self.moves(id) {
					let place = self.place(id, out);
					return self.move_out(place, span, out);
				}
				let value = self.place(id, out);
				self.copied(value, ty)
			}
			ExprKind::Borrow { borrow_kind, arg } => {
				if self.temporary_drops(*arg) {
					return self.err(span, "a borrow of a temporary that drops: bind it to a variable first");
				}
				if let BorrowKind::Mut { .. } = borrow_kind {
					let pointee = self.ty_of(*arg);
					if self.t.object_kind(pointee).is_none()
						&& !matches!(pointee.kind(), ty::Ref(_, _, rustc_ast::Mutability::Mut))
					{
						return self.err(
							span,
							format!("a mutable reference to `{pointee}`, which is no object it can change in place"),
						);
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
				out.push(StmtKind::decl(true, name.clone(), Some(Type::Builtin("boolean")), Some(l)).at(None));
				pre.push(StmtKind::Expr(Expr::ident(&name).assign(r)).at(None));
				let cond = match op {
					thir::LogicalOp::And => Expr::ident(&name),
					thir::LogicalOp::Or => Expr::ident(&name).not(),
				};
				out.push(
					StmtKind::If {
						cond,
						then: pre,
						els: Vec::new(),
					}
					.at(None),
				);
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
				// `&T` to `&dyn Debug`: a function writing the value.
				PointerCoercion::Unsize if let Some(tr) = self.fmt_object(ty) => {
					let source_ty = self.ty_of(*source);
					let value = self.operand(*source, out);
					self.fmt_value(tr, source_ty, value, span, out)
				}
				PointerCoercion::Unsize if self.trait_object(ty) => self.err(span, format!("a coercion to `{ty}`")),
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
			| ExprKind::ConstBlock { .. } => self.constant(id),
			ExprKind::Tuple { fields } if fields.is_empty() => Expr::undefined(),
			ExprKind::Tuple { fields } => {
				let values = self.operands(fields, out);
				// TypeScript widens an array literal to an array of the union.
				let ts_ty = self.t.ty(ty, span);
				Expr::As(Box::new(Expr::Array(values)), ts_ty)
			}
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
				// `[v; 0]` drops `v` at once, which no element of the array would.
				if self.t.drops(elem_ty) {
					return self.err(
						span,
						"an array repeat of a value that drops, which is not supported yet",
					);
				}
				let v = self.operand(*value, out);
				match elem_ty.kind() {
					ty::Uint(ty::UintTy::U8) => {
						let array = Expr::rt("bytes", vec![Expr::lit(n.to_string())]);
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
			ExprKind::Closure(closure) => self.closure(closure.closure_id, span, out),
			ExprKind::Assign { .. }
			| ExprKind::AssignOp { .. }
			| ExprKind::Break { .. }
			| ExprKind::Continue { .. }
			| ExprKind::Return { .. } => {
				self.stmt(id, Dest::Discard, out);
				Expr::undefined()
			}
			ExprKind::Let { .. } => self.err(span, "a `let` outside an `if` condition"),
			ExprKind::Yield { value } => {
				self.stmt(*value, Dest::Discard, out);
				self.yields += 1;
				Expr::Yield
			}
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

	/// `id` assigned to a new variable, which a value of type `!` leaves unassigned: control
	/// never gets past it.
	fn via_temp(&mut self, id: ExprId, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Expr {
		if ty.is_unit() {
			self.stmt(id, Dest::Discard, out);
			return Expr::undefined();
		}
		let name = self.temp_for(id);
		let ts_ty = self.t.ty(ty, span);
		out.push(StmtKind::decl(true, name.clone(), Some(ts_ty), None).at(None));
		self.stmt(id, Dest::Assign(name.clone()), out);
		Expr::ident(name)
	}

	/// A `match` of simple arms as a chain of `?:`.
	fn ternary(&mut self, id: ExprId, out: &mut Vec<Stmt>) -> Expr {
		let ExprKind::Match { scrutinee, arms, .. } = &self.thir[self.strip(id)].kind else {
			unreachable!()
		};
		let thir: &'a Thir<'tcx> = self.thir;
		// Its arms bind nothing, so a temporary that drops is left for nothing to drop.
		if self.temporary_drops(*scrutinee) {
			return self.err(
				self.span(id),
				"a `match` on a temporary that drops, which it doesn't move out of whole",
			);
		}
		let reads: usize = arms
			.iter()
			.take(arms.len().saturating_sub(1))
			.map(|&arm| self.test_reads(&thir[arm].pattern))
			.sum();
		let (scrut, _) = self.scrutinee(*scrutinee, reads.min(2), out);
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
		self.visit(id);
		let e = &self.thir[id];
		let span = e.span;
		match &e.kind {
			ExprKind::Scope { value, .. } | ExprKind::PlaceTypeAscription { source: value, .. } => {
				self.place(*value, out)
			}
			ExprKind::VarRef { id: var } => self.local(var.0, span),
			ExprKind::UpvarRef { var_hir_id, .. } => self.local(var_hir_id.0, span),
			// Reading through a `Box` that drops reads its pointee: the box stays where it is.
			ExprKind::Deref { arg } if self.is_place(*arg) && self.t.drops(self.ty_of(*arg)) => self.place(*arg, out),
			ExprKind::Deref { arg } => self.expr(*arg, out),
			ExprKind::Field {
				lhs,
				variant_index,
				name,
			} => {
				if self.temporary_drops(*lhs) {
					return self.err(span, "a field of a temporary that drops: bind it to a variable first");
				}
				// A field of a tuple made in place, `(a, b).0`: its other fields still run.
				if let ExprKind::Tuple { fields } = &self.thir[self.strip(*lhs)].kind {
					let fields = fields.clone();
					let mut values = self.operands(&fields, out);
					let index = name.index();
					let later_effects = values[index + 1..].iter().any(|v| !v.is_path());
					let mut field = std::mem::replace(&mut values[index], Expr::undefined());
					if later_effects && !field.is_constant() {
						field = self.spill(field, "field", out);
					}
					for (i, value) in values.into_iter().enumerate() {
						if i != index && !value.is_path() {
							out.push(StmtKind::Expr(value).at(None));
						}
					}
					return field;
				}
				let base = self.place(*lhs, out);
				let lty = self.ty_of(*lhs);
				self.project(base, lty, *variant_index, *name, span)
			}
			ExprKind::Index { lhs, index } => {
				if self.temporary_drops(*lhs) {
					return self.err(
						span,
						"an index into a temporary that drops: bind it to a variable first",
					);
				}
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
				Some(library::Std::Option | library::Std::Poll) => base,
				Some(library::Std::Result) => match variant.index() {
					0 => base.member("value"),
					_ => base.member("error"),
				},
				Some(library::Std::Box) => base,
				Some(library::Std::Range) => base.member(adt.non_enum_variant().fields[field].name.as_str()),
				None if adt.did().is_local() => {
					let info = self.t.adt(*adt, span);
					match info.repr {
						AdtRepr::Struct { fields, .. } => base.field(fields[field.index()]),
						AdtRepr::Union | AdtRepr::Tags => {
							let def = &adt.variant(variant).fields[field];
							base.member(super::field_name(def.name.as_str(), field.index()))
						}
					}
				}
				_ => self.err(span, format!("a field of `{ty}`")),
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
				(library::Std::Option, 0) | (library::Std::Poll, 1) => match std {
					library::Std::Poll => Expr::Rt("Pending"),
					_ => Expr::undefined(),
				},
				(library::Std::Option | library::Std::Poll, _) => values.pop().unwrap(),
				(library::Std::Result, 0) => Expr::rt("Ok", values),
				(library::Std::Result, _) => Expr::rt("Err", values),
				(library::Std::FmtError, _) => Expr::New(Box::new(Expr::Rt("FmtError")), Vec::new()),
				(library::Std::Range, _) => {
					let written = adt.fields.iter().map(|f| f.name.index()).zip(values).collect();
					let mut args: Vec<Expr> = self.declared(written, out).into_iter().map(|(_, v)| v).collect();
					// A wide range steps past 2^53 through its type.
					let ty::Adt(_, gargs) = ty.kind() else {
						unreachable!("a range is a struct")
					};
					if let Some(int) = super::num::Int::of(gargs.type_at(0))
						&& int.big()
					{
						args.push(Expr::Rt(int.rt()));
					}
					Expr::New(Box::new(Expr::Rt("Range")), args)
				}
				(library::Std::AssertKind, _) => match variant.name.as_str() {
					"Eq" => Expr::Str("==".into()),
					"Ne" => Expr::Str("!=".into()),
					_ => self.err(span, format!("constructing `{ty}`")),
				},
				_ => self.err(span, format!("constructing `{ty}`")),
			};
		}
		if !def.did().is_local() {
			return self.err(span, format!("constructing `{ty}` from another crate"));
		}
		let info = self.t.adt(def, span);
		let base = match &adt.base {
			thir::AdtExprBase::None => None,
			thir::AdtExprBase::Base(fru) if self.t.drops(self.ty_of(fru.base)) => {
				return self.err(span, "struct update syntax from a value that drops");
			}
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
			AdtRepr::Struct { ctor, fields } => {
				// The fields in the order they evaluate: as written, then those the base fills.
				let mut written: Vec<(usize, Expr)> = adt
					.fields
					.iter()
					.zip(values)
					.map(|(f, value)| (f.name.index(), field_arg(tcx, &variant.fields[f.name], value)))
					.collect();
				if let Some(base) = &base {
					for (i, field) in variant.fields.iter().enumerate() {
						if adt.fields.iter().any(|f| f.name.index() == i) {
							continue;
						}
						let field_ty = self.subst(field.ty(tcx, adt.args).skip_norm_wip());
						written.push((i, self.copied(base.clone().field(fields[i]), field_ty)));
					}
				}
				match ctor {
					ts::Ctor::Unit => self.t.construct(info.sym, &ts::Ctor::Unit, &fields, Vec::new()),
					// A class takes its named fields as an object, which evaluates them as written.
					ts::Ctor::Named if !self.t.plain(info.sym) => {
						let props = written.into_iter().map(|(i, v)| (Key::Field(fields[i]), v)).collect();
						Expr::New(Box::new(Expr::Sym(info.sym)), vec![Expr::Object(props)])
					}
					// A plain object's properties go in declaration order, so every value of the
					// struct shares one shape.
					_ => {
						let args = self.declared(written, out).into_iter().map(|(_, v)| v).collect();
						self.t.construct(info.sym, &ctor, &fields, args)
					}
				}
			}
			AdtRepr::Tags => {
				let ts_ty = self.t.ty(ty, span);
				Expr::As(Box::new(Expr::Str(variant.name.to_string())), ts_ty)
			}
			AdtRepr::Union => {
				let mut props = vec![(Key::Name("tag".into()), Expr::Str(variant.name.to_string()))];
				let written = adt.fields.iter().map(|f| f.name.index()).zip(values).collect();
				for (i, value) in self.declared(written, out) {
					let name = super::field_name(variant.fields[rustc_abi::FieldIdx::from_usize(i)].name.as_str(), i);
					props.push((Key::Name(name), value));
				}
				let ts_ty = self.t.ty(ty, span);
				Expr::As(Box::new(Expr::Object(props)), ts_ty)
			}
		}
	}

	/// A closure as an arrow function, or an `async` body as a future around a generator.
	///
	/// The generator is a function of its own, taking its upvars as parameters: a
	/// generator function made per call costs V8 a prototype object and a new shape for
	/// each generator, about 25 times what one made once does.
	fn closure(&mut self, local: rustc_hir::def_id::LocalDefId, span: Span, out: &mut Vec<Stmt>) -> Expr {
		let tcx = self.t.tcx;
		let Some((thir, body)) = self.t.body(local) else {
			return self.err(span, "no body for a closure");
		};
		let closure_ty = self.subst(tcx.type_of(local).instantiate_identity().skip_norm_wip());
		let (ret, coroutine) = match closure_ty.kind() {
			ty::Closure(_, args) => (
				tcx.instantiate_bound_regions_with_erased(args.as_closure().sig())
					.output(),
				false,
			),
			ty::Coroutine(_, args) if tcx.coroutine_is_async(local.to_def_id()) => {
				(args.as_coroutine().return_ty(), true)
			}
			_ => return self.err(span, format!("a closure of type `{closure_ty}`")),
		};
		if coroutine && self.t.erases_to(ret).pending {
			self.t.err(
				span,
				format!("an async body returning `{ret}` is not supported: its `Ready` could be pending"),
			);
		}
		// An async fn's body is this block alone, which rustc starts by moving each argument
		// into a local: the function gives its arguments up whole, and those locals drop
		// them. A block moves what it owns out of its place, as any move does, and drops it.
		let fn_body = matches!(
			tcx.coroutine_kind(local),
			Some(rustc_hir::CoroutineKind::Desugared(_, rustc_hir::CoroutineSource::Fn))
		);
		// A generator's upvars, each a parameter: its name, its value here, its type, and
		// whether the future owns it and so drops it.
		let mut upvars: Vec<(String, Expr, Ty<'tcx>, bool)> = Vec::new();
		for capture in tcx.closure_captures(local) {
			let var = capture.get_root_variable();
			let ty = self.subst(capture.place.ty());
			let by_value = matches!(capture.info.capture_kind, ty::UpvarCapture::ByValue);
			let mutable = matches!(self.locals.get(&var), Some(Local::Name { mutable: true, .. }));
			// A copy captured by `move` would be shared in JS: refuse one that can change.
			if !coroutine && by_value && mutable && self.t.is_copy(ty) {
				self.t
					.err(capture.get_path_span(tcx), "a closure that copies a mutable variable");
			}
			if !coroutine {
				if by_value && self.t.drops(ty) {
					self.t.err(
						capture.get_path_span(tcx),
						"a closure that captures a value that drops, which is not supported yet",
					);
				}
				continue;
			}
			if !capture.place.projections.is_empty() {
				self.t.err(
					capture.get_path_span(tcx),
					"an async body that captures part of a variable",
				);
				continue;
			}
			// Its generator takes the value: a change through the borrow would change a copy.
			if let ty::UpvarCapture::ByRef(kind) = capture.info.capture_kind
				&& kind != ty::BorrowKind::Immutable
			{
				if self.t.object_kind(ty).is_none() {
					self.t.err(
						capture.get_path_span(tcx),
						format!("an async body that borrows `{ty}` mutably, which is no object it can change in place"),
					);
					continue;
				}
				self.borrowed.insert(var);
			}
			let Some(Local::Name { name, .. }) = self.locals.get(&var).cloned() else {
				self.t
					.err(capture.get_path_span(tcx), "an async body capturing a function item");
				continue;
			};
			let owns = by_value && self.t.drops(ty);
			let place = Expr::ident(&name);
			let value = match (owns, fn_body) {
				(true, true) => {
					self.disown(&name);
					place
				}
				(true, false) => self.move_out(place, capture.get_path_span(tcx), out),
				// A `Copy` aggregate taken by value is the future's own, as JS would share it.
				(false, _) if by_value => self.copied(place, ty),
				(false, _) => place,
			};
			upvars.push((name, value, ty, owns));
		}
		let saved_thir = std::mem::replace(&mut self.thir, thir);
		self.read.extend(reads(tcx, thir));
		let saved_breakables = std::mem::take(&mut self.breakables);
		let saved_ret = std::mem::replace(&mut self.ret, ret);
		let saved_done = std::mem::replace(&mut self.done, false);
		let mut body_stmts = Vec::new();
		let mut params = Vec::new();
		self.open_scope();
		// Upvars drop in capture order, after the locals: the first is the innermost.
		for (name, _, ty, owns) in upvars.iter().rev() {
			if *owns && !fn_body {
				self.own(name, *ty);
			}
		}
		// The first parameter is the closure's environment; a coroutine's second is the
		// context each poll resumes it with.
		for param in thir.params.iter().skip(1) {
			let name = match &param.pat {
				Some(pat) => self.param(pat, &mut body_stmts),
				None => self.fresh("arg"),
			};
			let ty = self.subst(param.ty);
			let ty = self.t.ty(ty, span);
			params.push(Param { name, ty: Some(ty) });
		}
		let dest = match ret.is_unit() || ret.is_never() {
			true => Dest::Discard,
			false => Dest::Return,
		};
		self.stmt(body, dest, &mut body_stmts);
		self.close_scope(&mut body_stmts, 0, span);
		let ret_ty = self.t.ret_ty(ret, span);
		self.thir = saved_thir;
		self.breakables = saved_breakables;
		self.ret = saved_ret;
		self.done = saved_done;
		if !coroutine {
			return Expr::Arrow(params, ret_ty, body_stmts);
		}

		// `rt.future(body, drop, ...upvars)`: the body and what drops the upvars if it
		// never runs, each a function of the upvars.
		let parent = tcx.typeck_root_def_id(local.to_def_id());
		let base = camel(tcx.item_name(parent).as_str());
		let mut drops = Vec::new();
		for (name, _, ty, owns) in &upvars {
			if *owns && let Some(dropped) = self.t.drop_value(Expr::ident(name), *ty, span) {
				drops.push(StmtKind::Expr(dropped).at(None));
			}
		}
		let upvar_params: Vec<Param> = upvars
			.iter()
			.map(|(name, _, ty, _)| Param {
				name: name.clone(),
				ty: Some(self.t.ty(*ty, span)),
			})
			.collect();
		params.extend(upvar_params.iter().cloned());
		let body_fn = self.hoist(parent, format!("{base}Body"), params, ret_ty, body_stmts, true, span);
		// The drop reads only the upvars it drops: TypeScript lets the rest go unread as `_`s.
		let drop_params = upvars
			.iter()
			.zip(upvar_params)
			.map(|((_, _, _, owns), param)| match owns {
				true => param,
				false => Param {
					name: format!("_{}", param.name),
					..param
				},
			})
			.collect();
		let drop_fn = match drops.is_empty() {
			true => Expr::undefined(),
			false => self.hoist(
				parent,
				format!("{base}Drop"),
				drop_params,
				Type::Builtin("void"),
				drops,
				false,
				span,
			),
		};
		let mut args = vec![body_fn, drop_fn];
		args.extend(upvars.into_iter().map(|(_, value, _, _)| value));
		Expr::rt("future", args)
	}

	/// A module-level function in `parent`'s module, by the name `base` and the suffix of
	/// the instance of `parent` it is part of.
	#[expect(clippy::too_many_arguments, reason = "a function's parts")]
	fn hoist(
		&mut self,
		parent: rustc_hir::def_id::DefId,
		base: String,
		params: Vec<Param>,
		ret: Type,
		body: Vec<Stmt>,
		generator: bool,
		span: Span,
	) -> Expr {
		let order = self.t.order(span);
		let sym = self.t.program.sym(crate::program::Symbol {
			module: self.t.module_of(parent),
			base,
			suffix: self.t.suffix(Instance::new_raw(parent, self.args)),
			owner: None,
			kind: SymKind::Function,
			order,
			instance: String::new(),
			name: String::new(),
			alias: None,
		});
		let loc = self.t.loc(span);
		self.t.program.items.push(ts::Item::Function(ts::Function {
			sym,
			params,
			ret,
			body,
			loc,
			doc: None,
			generator,
		}));
		Expr::Sym(sym)
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
				let ids: Vec<ExprId> = std::iter::once(fun).chain(args.iter().copied()).collect();
				let mut values = self.operands(&ids, out);
				let f = values.remove(0);
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
			// A trait method of a type parameter: each instance resolves it.
			Ok(None) if self.t.polymorphic() && gargs.has_param() => {
				self.args(args, out);
				return Expr::undefined();
			}
			_ => return self.err(span, format!("`{}` does not resolve", tcx.def_path_str(def_id))),
		};
		// An `async` body's `Future::poll`, which its future has itself.
		if let ty::InstanceKind::Item(did) = instance.def
			&& tcx.is_coroutine(did)
		{
			let mut values = self.args(args, out);
			let future = values.remove(0);
			return future.member("poll").call(values);
		}
		if let ty::InstanceKind::Item(did) = instance.def
			&& did.is_local()
		{
			let sym = self.t.instance(instance);
			let values: Vec<Expr> = self.args(args, out).into_iter().map(Expr::typed).collect();
			let call = Expr::Sym(sym).call(values);
			// The crate's own `next` returns an item or `rt.DONE`, and its caller an `Option`.
			if super::iter::is_next(tcx, did) {
				let sig = tcx.fn_sig(did).instantiate(tcx, instance.args).skip_norm_wip();
				self.t.ty(tcx.instantiate_bound_regions_with_erased(sig).output(), span);
				return Expr::Rt("iter").member("toOption").call(vec![call]);
			}
			return call;
		}
		let call = library::Call {
			def_id,
			args: gargs,
			span,
		};
		let ids = match &args {
			Args::Thir(ids) => *ids,
			Args::Values(_) => &[],
		};
		match library::call(self, &call, args, out) {
			Some(e) => e,
			None => {
				// Refused, but what the arguments refuse is reported too. The mapping may
				// have lowered some already: lowering them twice would double the work at
				// each level of a nested call.
				let unvisited: Vec<ExprId> = ids.iter().copied().filter(|&id| !self.visited(id)).collect();
				self.operands(&unvisited, out);
				self.err(
					span,
					format!("unsupported call to `{}`", tcx.def_path_str_with_args(def_id, gargs)),
				)
			}
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
			rustc_hir::def::DefKind::Variant => (
				tcx.parent(parent),
				tcx.adt_def(tcx.parent(parent)).variant_index_with_id(parent),
			),
			_ => (parent, rustc_abi::FIRST_VARIANT),
		};
		let adt = tcx.adt_def(adt_id);
		if let Some(std) = library::Std::of(tcx, adt_id) {
			let mut values = values;
			return match (std, variant.index()) {
				(library::Std::Option, 1) | (library::Std::Poll, 0) => values.pop().unwrap(),
				(library::Std::Result, 0) => Expr::rt("Ok", values),
				(library::Std::Result, _) => Expr::rt("Err", values),
				_ => self.err(span, format!("constructing `{}`", tcx.def_path_str(adt_id))),
			};
		}
		if !adt_id.is_local() {
			return self.err(
				span,
				format!("constructing `{}` from another crate", tcx.def_path_str(adt_id)),
			);
		}
		let info = self.t.adt(adt, span);
		let ty = tcx.type_of(adt_id).instantiate(tcx, gargs).skip_norm_wip();
		match info.repr {
			AdtRepr::Struct { ctor, fields } => {
				let args = (adt.variant(variant).fields.iter().zip(values))
					.map(|(field, value)| field_arg(tcx, field, value))
					.collect();
				self.t.construct(info.sym, &ctor, &fields, args)
			}
			AdtRepr::Tags => {
				let ts_ty = self.t.ty(ty, span);
				Expr::As(Box::new(Expr::Str(adt.variant(variant).name.to_string())), ts_ty)
			}
			AdtRepr::Union => {
				let mut props = vec![(
					Key::Name("tag".into()),
					Expr::Str(adt.variant(variant).name.to_string()),
				)];
				for (i, value) in values.into_iter().enumerate() {
					props.push((Key::Name(format!("_{i}")), value));
				}
				let ts_ty = self.t.ty(ty, span);
				Expr::As(Box::new(Expr::Object(props)), ts_ty)
			}
		}
	}

	/// A function item as a value.
	pub fn fn_value(&mut self, def_id: rustc_hir::def_id::DefId, gargs: GenericArgsRef<'tcx>, span: Span) -> Expr {
		let tcx = self.t.tcx;
		let gargs = self.subst(gargs);
		let sig = tcx.instantiate_bound_regions_with_erased(tcx.fn_sig(def_id).instantiate(tcx, gargs).skip_norm_wip());
		let names: Vec<String> = (0..sig.inputs().len()).map(|i| self.fresh(&format!("a{i}"))).collect();
		let values: Vec<Expr> = names.iter().map(Expr::ident).collect();
		// A local function is named directly.
		if !matches!(tcx.def_kind(def_id), rustc_hir::def::DefKind::Ctor(..))
			&& let Ok(Some(instance)) = Instance::try_resolve(tcx, self.t.typing_env_pub(), def_id, gargs)
			&& let ty::InstanceKind::Item(did) = instance.def
			&& did.is_local()
		{
			return Expr::Sym(self.t.instance(instance));
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

/// A range of `ty` as a `for` loop iterates it: counting in a number, or as the runtime's
/// range of a wide integer, which steps past 2^53 into limbs.
fn range_of(ty: Ty<'_>, start: Expr, end: Expr, inclusive: bool) -> Iterable {
	let ty::Adt(_, args) = ty.kind() else {
		unreachable!("a range is a struct")
	};
	match super::num::Int::of(args.type_at(0)) {
		Some(int) if int.big() => {
			let class = if inclusive { "RangeInclusive" } else { "Range" };
			Iterable::Of(Expr::New(
				Box::new(Expr::Rt(class)),
				vec![start, end, Expr::Rt(int.rt())],
			))
		}
		_ => Iterable::Range { start, end, inclusive },
	}
}

enum Iterable {
	Range {
		start: Expr,
		end: Expr,
		inclusive: bool,
	},
	/// A JS array or `Uint8Array`, which `for ... of` iterates.
	Of(Expr),
	/// An iterator, by its `next`: an item or `rt.DONE`.
	Next(Expr),
}

/// The variables `thir` reads, those its closures capture included.
fn reads<'tcx>(tcx: ty::TyCtxt<'tcx>, thir: &Thir<'tcx>) -> FxHashSet<HirId> {
	let mut read = FxHashSet::default();
	for expr in thir.exprs.iter() {
		match &expr.kind {
			ExprKind::VarRef { id } => {
				read.insert(id.0);
			}
			ExprKind::UpvarRef { var_hir_id, .. } => {
				read.insert(var_hir_id.0);
			}
			ExprKind::Closure(closure) => {
				read.extend(
					tcx.closure_captures(closure.closure_id)
						.iter()
						.map(|capture| capture.get_root_variable()),
				);
			}
			_ => {}
		}
	}
	read
}

/// Whether `ty` is a closure or function pointer that never returns, whose binding `tsc`
/// sees a call to diverge only through a declared type.
fn diverges_when_called(ty: Ty<'_>) -> bool {
	let output = match ty.peel_refs().kind() {
		ty::Closure(_, args) => args.as_closure().sig().output(),
		ty::FnPtr(sig, header) => sig.with(*header).output(),
		_ => return false,
	};
	output.skip_binder().is_never()
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

/// Whether matching `pat` moves its whole value into a binding, or matches only a variant
/// with nothing in it: either way nothing is left to drop.
fn takes_all(pat: &Pat<'_>) -> bool {
	match &pat.kind {
		PatKind::Binding {
			mode, subpattern: None, ..
		} => mode.0 == rustc_hir::ByRef::No,
		// `Some(..)` names no field, which then stays in the temporary.
		PatKind::Variant {
			adt_def,
			variant_index,
			subpatterns,
			..
		} => {
			subpatterns.len() == adt_def.variant(*variant_index).fields.len()
				&& subpatterns.iter().all(|f| takes_all(&f.pattern))
		}
		_ => false,
	}
}

pub fn has_bindings(pat: &Pat<'_>) -> bool {
	bindings(pat) > 0
}

/// How many bindings `pat` makes, each reading the scrutinee once.
fn bindings(pat: &Pat<'_>) -> usize {
	let mut count = 0;
	pat.walk_always(|p| {
		if let PatKind::Binding { .. } = p.kind {
			count += 1;
		}
	});
	count
}

fn kind_name(kind: &ExprKind<'_>) -> String {
	let debug = format!("{kind:?}");
	debug.split([' ', '(', '{']).next().unwrap_or("?").to_string()
}

/// A field's value as a class's constructor takes it: without an outer `as` where the
/// field's type gives the value its type, but not where TypeScript infers a class
/// parameter from it, which would widen a literal.
fn field_arg<'tcx>(tcx: ty::TyCtxt<'tcx>, field: &ty::FieldDef, value: Expr) -> Expr {
	match tcx
		.type_of(field.did)
		.instantiate_identity()
		.skip_norm_wip()
		.has_param()
	{
		true => value,
		false => value.typed(),
	}
}
