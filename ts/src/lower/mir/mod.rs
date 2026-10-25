//! Function bodies from MIR: the spike's prototype, beside the THIR lowering in `body`.
//!
//! MIR is each body's control-flow graph after borrow checking. Every drop is a
//! terminator where drop elaboration put it, with a flag wherever a value is moved on
//! some paths only, and an async body is already a state machine, its resume function
//! and its drop. Blocks become structured JS (`cfg`). Statements become JS statements,
//! with each temporary that is assigned once and read once folded back into the
//! expression that reads it, unless that would reorder an effect.
//!
//! Library calls go through the THIR lowering's mappings, which take their arguments
//! lowered: a detached `Body` holds the names, and its translator everything else.

mod call;
mod cfg;
pub mod coroutine;
pub mod glue;
mod place;
mod rvalue;
mod unwind;

use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_index::IndexVec;
use rustc_middle::mir::{self, BasicBlock, Local, Operand, Place, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty::{self, EarlyBinder, Instance, Ty, TyCtxt};
use rustc_span::Span;

use self::cfg::Exit;
use super::Translator;
use super::body::Body;

use crate::ts::{self, Expr, Param, Stmt, StmtKind, Type};

/// Translate one function instance's body from its MIR.
pub fn function<'a, 'tcx>(t: &mut Translator<'a, 'tcx>, instance: Instance<'tcx>) -> Option<ts::Function> {
	let tcx = t.tcx;
	let def_id = instance.def_id();
	let span = tcx.def_span(def_id);
	let mir = instance_mir(t, instance, span)?;
	let next = super::iter::is_next(tcx, def_id);
	if next {
		super::iter::check_impl(t, def_id);
	}
	let ret_ty = mir.return_ty();
	let mut b = Body::detached(t, instance.args, ret_ty);
	let mut fx = Fx::new(&mut b, &mir, Env::Plain, span);
	let params = fx.params();
	let body = fx.lower();
	let ret = match (next, ret_ty.kind()) {
		(true, ty::Adt(_, args)) => Type::Union(vec![fx.b.t.ty(args.type_at(0), span), Type::Rt("Done", Vec::new())]),
		_ => fx.b.t.ret_ty(ret_ty, span),
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

/// The MIR of `instance`, with its generic arguments in place.
fn instance_mir<'tcx>(t: &mut Translator<'_, 'tcx>, instance: Instance<'tcx>, span: Span) -> Option<mir::Body<'tcx>> {
	let tcx = t.tcx;
	let def_id = instance.def_id();
	if !tcx.is_mir_available(def_id) {
		t.err(span, format!("no MIR for `{}`", tcx.def_path_str(def_id)));
		return None;
	}
	let body = tcx.optimized_mir(def_id).clone();
	Some(instance.instantiate_mir_and_normalize_erasing_regions(tcx, t.typing_env_pub(), EarlyBinder::bind(tcx, body)))
}

/// What `_1` is.
#[derive(Clone)]
pub enum Env {
	/// A parameter like any other.
	Plain,
	/// A closure's environment, whose fields are the expressions it captured.
	Closure(Vec<Expr>),
	/// A coroutine's state, behind `Pin<&mut Self>` or `&mut Self`: `this` of its class.
	Coroutine,
	/// Drop glue, whose `_1` is a pointer to the value: the value in JS.
	Shim,
}

/// How a MIR local lowers.
#[derive(Clone, Debug)]
enum Slot<'tcx> {
	/// No JS value: `()`, a function item, or another zero-sized value.
	Unit,
	/// A JS variable.
	Var(String),
	/// A reference assigned once, read as the place it borrows, which is set once its
	/// assignment is lowered.
	Alias(Option<Expr>),
	/// A discriminant only a `switchInt` reads: tested as the place's variant.
	Discr(Option<(Expr, Ty<'tcx>)>),
}

/// A temporary's value, held back to fold into the one expression that reads it.
struct Pending {
	name: String,
	value: Expr,
	/// In the order assignments ran.
	order: u32,
	/// Whether evaluating it can do or throw anything.
	effect: bool,
	/// Whether it reads memory something else could change.
	memory: bool,
	/// The aggregate's operands, for a call that takes them apart.
	parts: Option<Vec<Expr>>,
	/// The names it reads, which an assignment to must not pass it.
	reads: Vec<String>,
	/// Whether it is a `Copy` aggregate read from where it could change, which only an
	/// argument may share: emitted on its own, it is copied.
	copy: bool,
	/// Whether it is an aggregate's part, with no variable of its own until emitted.
	decl: bool,
}

pub struct Fx<'m, 'b, 'a, 'tcx> {
	pub b: &'m mut Body<'b, 'a, 'tcx>,
	pub mir: &'m mir::Body<'tcx>,
	pub tcx: TyCtxt<'tcx>,
	env: Env,
	slots: IndexVec<Local, Slot<'tcx>>,
	/// Where each local assigned once is assigned, by a statement.
	def_at: IndexVec<Local, Option<(BasicBlock, usize)>>,
	/// How many times each local is read, and assigned whole.
	uses: IndexVec<Local, u32>,
	defs: IndexVec<Local, u32>,
	/// Locals something borrows, which a call or closure can change behind a read.
	borrowed: IndexVec<Local, bool>,
	/// Locals changed in place: assigned through a projection, or borrowed mutably.
	mutated: IndexVec<Local, bool>,
	/// The names of borrowed locals.
	shared: FxHashSet<String>,
	/// `(value, overflowed)` pairs of checked arithmetic, whose `.1` is false once the value
	/// is made, which panics where the assert would.
	checked: FxHashSet<Local>,
	/// The temporaries held back, in order.
	pending: Vec<Pending>,
	order: u32,
	/// Locals some statement assigns, which need a declaration.
	assigned: IndexVec<Local, bool>,
	span: Span,
	pub labels: u32,
	/// Whether this is an `Iterator::next`, which returns an item or `rt.DONE`.
	next: bool,
	/// Blocks whose only statement is `unreachable`.
	unreachable: FxHashSet<BasicBlock>,
	/// The operands of the aggregate lowered last.
	parts: Option<Vec<Expr>>,
	/// Blocks that start an ignored macro's code, and the block after it.
	redirect: FxHashMap<BasicBlock, BasicBlock>,
	/// Whether the operands lowered now go to a local nothing changes in place.
	into_frozen: bool,
	/// Whether the value held back next is copied unless it folds into an argument.
	copy_on_flush: bool,
	/// Whether a copy lowered now shares what it copies, whatever it reads from.
	force_share: bool,
	/// Locals whose one read is as a call's argument, which may share what it copies.
	arg_use: IndexVec<Local, bool>,
	/// Locals whose one read is as an argument to a call given nothing that changes memory.
	arg_alone: IndexVec<Local, bool>,
	/// Locals whose one read is as an argument to a call whose result could hold it.
	arg_kept: IndexVec<Local, bool>,
	/// The cleanups a panic runs, if any does something JS can see.
	unwind: Option<unwind::Unwind>,
	/// While the cleanups are lowered, the caught exception's name.
	in_cleanup: Option<String>,
	/// Where each left-out macro invocation in the body is written.
	ignored_sites: Vec<Span>,
	/// The operands of aggregates assigned once to locals nothing changes.
	var_parts: FxHashMap<Local, Vec<Expr>>,
	/// Locals holding what `Iterator::next` returns by the protocol: an item or `rt.DONE`.
	next_results: FxHashSet<Local>,
	/// Their names, which a switch on their variant tests against `rt.DONE`.
	next_names: FxHashSet<String>,
	/// Next results of a range of numbers stepped in place: each holds only the item, so
	/// JS keeps it a number, and a flag holds whether there was one.
	range_nexts: FxHashSet<Local>,
	/// Their names, and their flags' names, which a switch on their variant tests.
	range_flags: FxHashMap<String, String>,
}

impl<'m, 'b, 'a, 'tcx> Fx<'m, 'b, 'a, 'tcx> {
	pub fn new(b: &'m mut Body<'b, 'a, 'tcx>, mir: &'m mir::Body<'tcx>, env: Env, span: Span) -> Self {
		let tcx = b.t.tcx;
		let n = mir.local_decls.len();
		let mut fx = Fx {
			b,
			mir,
			tcx,
			env,
			slots: IndexVec::from_elem_n(Slot::Unit, n),
			def_at: IndexVec::from_elem_n(None, n),
			uses: IndexVec::from_elem_n(0, n),
			defs: IndexVec::from_elem_n(0, n),
			borrowed: IndexVec::from_elem_n(false, n),
			mutated: IndexVec::from_elem_n(false, n),
			shared: FxHashSet::default(),
			checked: FxHashSet::default(),
			pending: Vec::new(),
			order: 0,
			assigned: IndexVec::from_elem_n(false, n),
			span,
			labels: 0,
			next: crate::lower::iter::is_next(tcx, mir.source.def_id()),
			unreachable: FxHashSet::default(),
			parts: None,
			redirect: FxHashMap::default(),
			into_frozen: false,
			copy_on_flush: false,
			force_share: false,
			arg_use: IndexVec::from_elem_n(false, n),
			arg_alone: IndexVec::from_elem_n(false, n),
			arg_kept: IndexVec::from_elem_n(false, n),
			unwind: None,
			in_cleanup: None,
			ignored_sites: Vec::new(),
			var_parts: FxHashMap::default(),
			next_results: FxHashSet::default(),
			next_names: FxHashSet::default(),
			range_nexts: FxHashSet::default(),
			range_flags: FxHashMap::default(),
		};
		fx.analyze();
		fx
	}

	pub fn err(&mut self, span: Span, msg: impl Into<String>) -> Expr {
		self.b.t.err(span, msg);
		Expr::undefined()
	}

	/// Count each local's reads and assignments, and decide how each lowers.
	fn analyze(&mut self) {
		let mir = self.mir;
		for (bb, data) in mir.basic_blocks.iter_enumerated() {
			if data.statements.iter().all(|s| {
				matches!(
					s.kind,
					StatementKind::StorageDead(_) | StatementKind::StorageLive(_) | StatementKind::Nop
				)
			}) && matches!(data.terminator().kind, TerminatorKind::Unreachable)
			{
				self.unreachable.insert(bb);
			}
			for (i, stmt) in data.statements.iter().enumerate() {
				match &stmt.kind {
					StatementKind::Assign(assign) => {
						let (place, rvalue) = &**assign;
						if let Some(local) = place.as_local() {
							self.def_at[local] = Some((bb, i));
						}
						self.def(*place);
						self.rvalue_uses(rvalue);
						if let Rvalue::BinaryOp(op, _) = rvalue
							&& op.overflowing_to_wrapping().is_some()
							&& let Some(local) = place.as_local()
						{
							self.checked.insert(local);
						}
					}
					StatementKind::SetDiscriminant { place, .. } => {
						self.place_use(**place);
						self.mutated[place.local] = true;
					}
					_ => {}
				}
			}
			let term = data.terminator();
			match &term.kind {
				TerminatorKind::SwitchInt { discr, .. } => self.operand_use(discr),
				TerminatorKind::Return => self.uses[mir::RETURN_PLACE] += 1,
				TerminatorKind::Drop { place, .. } => self.place_use(*place),
				TerminatorKind::Call {
					func,
					args,
					destination,
					..
				} => {
					self.operand_use(func);
					// A callee given nothing that changes memory can't change an argument's
					// source while it runs.
					let alone = !std::iter::once(func)
						.chain(args.iter().map(|a| &a.node))
						.any(|op| self.reaches_mutable(op.ty(&mir.local_decls, self.tcx)));
					let result = destination.ty(&mir.local_decls, self.tcx).ty;
					for arg in args.iter() {
						self.operand_use(&arg.node);
						if let Operand::Copy(p) | Operand::Move(p) = &arg.node
							&& let Some(local) = p.as_local()
						{
							self.arg_use[local] = true;
							self.arg_alone[local] = alone;
							self.arg_kept[local] = holds(result, mir.local_decls[local].ty);
						}
					}
					self.def(*destination);
				}
				TerminatorKind::Assert { cond, msg, .. } => {
					// The assert after checked arithmetic reads `.1`, which lowering folds away.
					let folded = matches!(cond, Operand::Move(p) | Operand::Copy(p)
						if p.projection.len() == 1 && self.checked.contains(&p.local));
					if !folded && !redundant(msg) {
						self.operand_use(cond);
						// Only a bounds check's message shows its operands.
						if let mir::AssertKind::BoundsCheck { len, index } = &**msg {
							self.operand_use(len);
							self.operand_use(index);
						}
					}
				}
				_ => {}
			}
		}
		self.find_next_results();
		for local in mir.local_decls.indices() {
			let ty = mir.local_decls[local].ty;
			let slot = if self.zero_sized(ty) {
				Slot::Unit
			} else if let Some(value) = self.unit_struct(local, ty) {
				Slot::Alias(Some(value))
			} else if self.alias(local) {
				Slot::Alias(None)
			} else if self.discr_only(local) {
				Slot::Discr(None)
			} else {
				Slot::Var(String::new())
			};
			self.slots[local] = slot;
		}
		// Names: what debug info calls a local, else its number.
		let mut named: FxHashMap<Local, rustc_span::Symbol> = FxHashMap::default();
		for info in &mir.var_debug_info {
			if let mir::VarDebugInfoContents::Place(place) = &info.value
				&& let Some(local) = place.as_local()
				&& info.composite.is_none()
			{
				named.entry(local).or_insert(info.name);
			}
		}
		for local in mir.local_decls.indices() {
			if let Slot::Var(_) = self.slots[local] {
				let base = match named.get(&local) {
					Some(name) => super::camel(name.as_str()),
					None => format!("_{}", local.as_u32()),
				};
				let base = match self.uses[local] == 0 && local.as_usize() <= mir.arg_count {
					// A parameter nothing reads, which `noUnusedParameters` wants marked.
					true if !base.starts_with('_') => format!("_{base}"),
					_ => base,
				};
				let name = self.b.fresh(&base);
				if self.borrowed[local] {
					self.shared.insert(name.clone());
				}
				self.slots[local] = Slot::Var(name);
			}
		}
	}

	fn def(&mut self, place: Place<'tcx>) {
		match place.as_local() {
			Some(local) => self.defs[local] += 1,
			// Assigning a projection reads the local it starts from, and changes it in place
			// unless it goes through a reference.
			None => {
				self.place_use(place);
				if own(place) {
					self.mutated[place.local] = true;
				}
			}
		}
	}

	fn place_use(&mut self, place: Place<'tcx>) {
		self.uses[place.local] += 1;
		for elem in place.projection {
			if let mir::ProjectionElem::Index(index) = elem {
				self.uses[index] += 1;
			}
		}
	}

	fn operand_use(&mut self, op: &Operand<'tcx>) {
		if let Operand::Copy(place) | Operand::Move(place) = op {
			self.place_use(*place);
		}
	}

	fn rvalue_uses(&mut self, rvalue: &Rvalue<'tcx>) {
		match rvalue {
			Rvalue::Use(op, _) | Rvalue::Repeat(op, _) | Rvalue::Cast(_, op, _) | Rvalue::UnaryOp(_, op) => {
				self.operand_use(op)
			}
			Rvalue::BinaryOp(_, ops) => {
				self.operand_use(&ops.0);
				self.operand_use(&ops.1);
			}
			// A borrow through a reference borrows what it points at, not the local.
			Rvalue::Ref(_, kind, place) => {
				self.place_use(*place);
				if own(*place) {
					self.borrowed[place.local] = true;
					if let mir::BorrowKind::Mut { .. } = kind {
						self.mutated[place.local] = true;
					}
				}
			}
			Rvalue::RawPtr(_, place) => {
				self.place_use(*place);
				if own(*place) {
					self.borrowed[place.local] = true;
					self.mutated[place.local] = true;
				}
			}
			Rvalue::Discriminant(place) | Rvalue::CopyForDeref(place) => self.place_use(*place),
			Rvalue::Reborrow(_, _, place) => {
				self.place_use(*place);
				self.borrowed[place.local] = true;
			}
			Rvalue::Aggregate(_, ops) => ops.iter().for_each(|op| self.operand_use(op)),
			Rvalue::ThreadLocalRef(_) | Rvalue::WrapUnsafeBinder(..) => {}
		}
	}

	/// Whether a `ty` has no JS value: `()`, a function item, or `..`.
	fn zero_sized(&self, ty: Ty<'tcx>) -> bool {
		match ty.kind() {
			ty::Tuple(tys) => tys.is_empty(),
			ty::FnDef(..) | ty::Never => true,
			ty::Adt(adt, _) => {
				crate::lower::library::Std::of(self.tcx, adt.did()) == Some(crate::lower::library::Std::RangeFull)
			}
			_ => false,
		}
	}

	/// A struct of no fields that nothing assigns, which MIR leaves unassigned since it
	/// holds nothing: JS needs it made.
	fn unit_struct(&mut self, local: Local, ty: Ty<'tcx>) -> Option<Expr> {
		if self.defs[local] != 0 || self.is_arg(local) || local == mir::RETURN_PLACE {
			return None;
		}
		let ty::Adt(adt, _) = ty.kind() else {
			return None;
		};
		if !adt.is_struct() || !adt.did().is_local() || adt.all_fields().next().is_some() {
			return None;
		}
		let info = self.b.t.adt(*adt, self.span);
		Some(self.b.t.construct(info.sym, &ts::Ctor::Unit, &[], Vec::new()))
	}

	/// Whether `local` is a box's pointer, as a box's dereference reads it.
	fn box_pointer(&self, local: Local) -> bool {
		matches!(
			self.def_rvalue(local),
			Some(Rvalue::Cast(mir::CastKind::BoxDerefTransmute, ..))
		)
	}

	/// Whether `local` is a parameter.
	fn is_arg(&self, local: Local) -> bool {
		(1..=self.mir.arg_count).contains(&local.as_usize())
	}

	/// Whether `local` is a reference assigned once from a place that stays put, which
	/// every read can read in its place, or a box's pointer, which is the box in JS.
	fn alias(&self, local: Local) -> bool {
		let mir = self.mir;
		if self.is_arg(local) || self.defs[local] != 1 || self.borrowed[local] {
			return false;
		}
		let Some(rvalue) = self.def_rvalue(local) else {
			return false;
		};
		match (mir.local_decls[local].ty.kind(), rvalue) {
			// A borrow: borrow checking keeps the place as it is but through the borrow,
			// unless the path to it goes through a reference held in a field or element,
			// which can be replaced while the borrow lives.
			(ty::Ref(..), Rvalue::Ref(_, _, place)) => {
				self.stable(*place)
					&& place
						.projection
						.iter()
						.skip(1)
						.all(|e| !matches!(e, mir::ProjectionElem::Deref))
			}
			// A copy of a reference, or a box's pointer: the place it is read from must not
			// change at all.
			(
				ty::Ref(..),
				Rvalue::Use(Operand::Copy(place) | Operand::Move(place), _) | Rvalue::CopyForDeref(place),
			)
			| (
				ty::RawPtr(..),
				Rvalue::Cast(mir::CastKind::BoxDerefTransmute, Operand::Copy(place) | Operand::Move(place), _),
			) if self.env_field(*place) => true,
			(
				ty::Ref(..),
				Rvalue::Use(Operand::Copy(place) | Operand::Move(place), _) | Rvalue::CopyForDeref(place),
			)
			| (
				ty::RawPtr(..),
				Rvalue::Cast(mir::CastKind::BoxDerefTransmute, Operand::Copy(place) | Operand::Move(place), _),
			) => {
				self.stable(*place)
					&& !self.mutated[place.local]
					&& place
						.projection
						.iter()
						.all(|e| !matches!(e, mir::ProjectionElem::Deref))
			}
			_ => false,
		}
	}

	/// Whether `place` is a field of a closure's environment, which stays put for the call.
	fn env_field(&self, place: Place<'tcx>) -> bool {
		matches!(self.env, Env::Closure(_))
			&& place.local.as_u32() == 1
			&& matches!(
				place.projection.as_slice(),
				[mir::ProjectionElem::Field(..)] | [mir::ProjectionElem::Deref, mir::ProjectionElem::Field(..)]
			)
	}

	/// Locals an `Iterator::next` assigns that only a switch on their variant and a read
	/// of `Some`'s value read: each holds the item or `rt.DONE`, as the protocol returns
	/// it, so an iterator of `Option` keeps each `None` apart from its end.
	fn find_next_results(&mut self) {
		use rustc_middle::mir::visit::{MutatingUseContext, NonMutatingUseContext, PlaceContext, Visitor};
		struct Uses<'tcx> {
			local: Local,
			/// The variants whose value may be read.
			variants: &'static [u32],
			/// Where a move of the whole into `?`'s branch may be.
			branch: Option<mir::Location>,
			ok: bool,
			_tcx: std::marker::PhantomData<Ty<'tcx>>,
		}
		impl<'tcx> Visitor<'tcx> for Uses<'tcx> {
			fn visit_place(&mut self, place: &Place<'tcx>, cx: PlaceContext, at: mir::Location) {
				if place.local != self.local {
					return;
				}
				self.ok &= match place.projection.as_slice() {
					[] => {
						matches!(
							cx,
							PlaceContext::NonMutatingUse(NonMutatingUseContext::Inspect)
								| PlaceContext::MutatingUse(MutatingUseContext::Call)
						) || (matches!(cx, PlaceContext::NonMutatingUse(NonMutatingUseContext::Move))
							&& Some(at) == self.branch)
					}
					[
						mir::ProjectionElem::Downcast(_, v),
						mir::ProjectionElem::Field(f, _),
						..,
					] => self.variants.contains(&v.as_u32()) && f.as_u32() == 0,
					_ => false,
				};
			}
		}
		let mir = self.mir;
		let tcx = self.tcx;
		// `?` on a local: where, and what it branches to.
		let mut branches: FxHashMap<Local, (mir::Location, Local)> = FxHashMap::default();
		let mut nexts = Vec::new();
		for (bb, data) in mir.basic_blocks.iter_enumerated() {
			let TerminatorKind::Call {
				func,
				args,
				destination,
				..
			} = &data.terminator().kind
			else {
				continue;
			};
			let ty::FnDef(def_id, gargs) = *func.ty(&mir.local_decls, tcx).kind() else {
				continue;
			};
			let Some(local) = destination.as_local() else {
				continue;
			};
			if crate::lower::iter::is_next(tcx, def_id) && self.defs[local] == 1 {
				nexts.push((local, gargs.skip_binder().type_at(0)));
			}
			if crate::lower::library::key(tcx, def_id).is_some_and(|(o, n)| o == "Try" && n == "branch")
				&& let [arg] = &**args
				&& let Operand::Move(p) = &arg.node
				&& let Some(from) = p.as_local()
			{
				let at = mir::Location {
					block: bb,
					statement_index: data.statements.len(),
				};
				branches.insert(from, (at, local));
			}
		}
		for (local, iter_ty) in nexts {
			let branch = branches.get(&local).copied();
			let mut uses = Uses {
				local,
				variants: &[1],
				branch: branch.map(|(at, _)| at),
				ok: true,
				_tcx: std::marker::PhantomData,
			};
			uses.visit_body(mir);
			if !uses.ok {
				continue;
			}
			// What `?` branches to holds the same: the item, or `rt.DONE` to return early.
			if let Some((_, to)) = branch {
				let mut uses = Uses {
					local: to,
					variants: &[0, 1],
					branch: None,
					ok: self.defs[to] == 1,
					_tcx: std::marker::PhantomData,
				};
				uses.visit_body(mir);
				if !uses.ok {
					continue;
				}
				self.next_results.insert(to);
			} else if local != mir::RETURN_PLACE && small_range(tcx, iter_ty) {
				self.range_nexts.insert(local);
			}
			self.next_results.insert(local);
		}
		// An `Iterator::next` returns the item or `rt.DONE` too.
		if self.next {
			self.next_results.insert(mir::RETURN_PLACE);
		}
	}

	/// The rvalue a local's one assignment by a statement assigns.
	fn def_rvalue(&self, local: Local) -> Option<&'m Rvalue<'tcx>> {
		let (bb, i) = self.def_at[local]?;
		let mir: &'m mir::Body<'tcx> = self.mir;
		match &mir.basic_blocks[bb].statements[i].kind {
			StatementKind::Assign(assign) => Some(&assign.1),
			_ => None,
		}
	}

	/// Whether `place` names the same memory wherever it is read: every local in it is
	/// assigned at most once.
	fn stable(&self, place: Place<'tcx>) -> bool {
		let fixed = |local: Local| self.defs[local] <= 1 && (!self.is_arg(local) || self.defs[local] == 0);
		fixed(place.local)
			&& place.projection.iter().all(|elem| match elem {
				mir::ProjectionElem::Index(index) => fixed(index),
				_ => true,
			})
	}

	/// Whether every read of `local` is a `switchInt` on it, after its one assignment of a
	/// discriminant.
	fn discr_only(&self, local: Local) -> bool {
		if self.defs[local] != 1 || self.uses[local] != 1 {
			return false;
		}
		let assigned = matches!(self.def_rvalue(local), Some(Rvalue::Discriminant(_)));
		let mut switched = false;
		for data in self.mir.basic_blocks.iter() {
			if let TerminatorKind::SwitchInt {
				discr: Operand::Move(p) | Operand::Copy(p),
				..
			} = &data.terminator().kind
				&& p.as_local() == Some(local)
			{
				switched = true;
			}
		}
		assigned && switched
	}

	/// The parameters: `_1` through the argument count, but a closure's environment or a
	/// coroutine's `this`.
	pub fn params(&mut self) -> Vec<Param> {
		let mut params = Vec::new();
		let skip = match self.env {
			Env::Plain | Env::Shim => 0,
			_ => 1,
		};
		for local in self.mir.args_iter().skip(skip) {
			let ty = match (&self.env, self.mir.local_decls[local].ty.kind()) {
				(Env::Shim, ty::RawPtr(pointee, _)) => *pointee,
				(_, _) => self.mir.local_decls[local].ty,
			};
			let name = match &self.slots[local] {
				Slot::Var(name) => name.clone(),
				_ => self.b.fresh("_"),
			};
			let ty = self.b.t.ty(ty, self.span);
			params.push(Param { name, ty: Some(ty) });
		}
		params
	}

	/// The body: its locals' declarations, then its blocks as structured statements.
	pub fn lower(&mut self) -> Vec<Stmt> {
		// A parameter changed in place gets its own copy, since the caller's may be shared.
		let mut entry = Vec::new();
		for local in self.mir.args_iter() {
			let ty = self.mir.local_decls[local].ty;
			if self.mutated[local]
				&& self.b.needs_copy(ty)
				&& let Slot::Var(name) = &self.slots[local]
			{
				let name = name.clone();
				entry.push(
					StmtKind::Expr(Expr::ident(&name).assign(Expr::rt("copy", vec![Expr::ident(&name)]))).at(None),
				);
			}
		}
		// A closure that captures nothing holds nothing, so MIR never assigns it: JS needs
		// its function.
		for local in self.mir.local_decls.indices() {
			if let ty::Closure(def_id, args) = *self.mir.local_decls[local].ty.kind()
				&& self.defs[local] == 0
				&& !self.is_arg(local)
				&& self.uses[local] > 0
				&& let Slot::Var(name) = self.slots[local].clone()
			{
				let f = self.bare_closure(def_id, args, self.span);
				self.assigned[local] = true;
				entry.push(StmtKind::Expr(Expr::ident(&name).assign(f)).at(None));
			}
		}
		let n = self.mir.basic_blocks.len();
		self.skip_ignored_macros();
		self.plan_unwind();
		let mut succs = vec![Vec::new(); n];
		for (bb, data) in self.mir.basic_blocks.iter_enumerated() {
			if data.is_cleanup {
				continue;
			}
			succs[bb.as_usize()] = match self.redirect.get(&bb) {
				Some(&to) => vec![to.as_usize()],
				None => self
					.successors(data.terminator())
					.into_iter()
					.map(BasicBlock::as_usize)
					.collect(),
			};
		}
		let graph = cfg::Graph { succs };
		let mut labels = self.labels;
		let body = cfg::structure(&graph, self, &mut labels);
		self.labels = labels;
		let body = self.catch(body);
		let mut out = Vec::new();
		for local in self.mir.local_decls.indices() {
			// Parameters are declared by the signature.
			if self.is_arg(local) || !self.assigned[local] {
				continue;
			}
			if let Slot::Var(name) = self.slots[local].clone() {
				let ty = self.decl_ty(local);
				out.push(StmtKind::Declare { name, ty }.at(None));
			}
		}
		let mut flags: Vec<String> = self.range_flags.values().cloned().collect();
		flags.sort();
		for name in flags {
			out.push(
				StmtKind::Declare {
					name,
					ty: Type::Builtin("boolean"),
				}
				.at(None),
			);
		}
		out.extend(entry);
		out.extend(body);
		drop_self_assignments(&mut out);
		prune_unread(&mut out);
		out
	}

	/// Send each block whose terminator an ignored macro made to the one block its code
	/// leaves to. Such a macro evaluates to `()`, so its code has one way out.
	fn skip_ignored_macros(&mut self) {
		let mir = self.mir;
		// Every left-out invocation's span, which its arguments' code is within too.
		let mut sites = Vec::new();
		for data in mir.basic_blocks.iter() {
			let spans = data.statements.iter().map(|s| s.source_info.span);
			for span in spans.chain([data.terminator().source_info.span]) {
				if let Some(site) = self.b.t.ignored_site(span)
					&& !sites.contains(&site)
				{
					sites.push(site);
				}
			}
		}
		self.ignored_sites = sites;
		let flags: IndexVec<BasicBlock, bool> = mir
			.basic_blocks
			.iter()
			.map(|data| !data.is_cleanup && self.in_ignored(data.terminator().source_info.span))
			.collect();
		let ignored = |bb: BasicBlock| flags[bb];
		for bb in mir.basic_blocks.indices() {
			if !ignored(bb) {
				continue;
			}
			let mut seen = FxHashSet::default();
			let mut stack = vec![bb];
			let mut exits = FxHashSet::default();
			while let Some(next) = stack.pop() {
				if !seen.insert(next) {
					continue;
				}
				for succ in self.successors(mir.basic_blocks[next].terminator()) {
					match ignored(succ) {
						true => stack.push(succ),
						false => {
							exits.insert(succ);
						}
					}
				}
			}
			match exits.len() {
				1 => {
					self.redirect.insert(bb, *exits.iter().next().unwrap());
				}
				_ => {
					let span = mir.basic_blocks[bb].terminator().source_info.span;
					self.err(span, "an ignored macro whose code has other than one way out");
				}
			}
		}
	}

	/// Whether `span` is within a left-out macro's invocation.
	fn in_ignored(&self, span: Span) -> bool {
		let span = span.source_callsite();
		self.ignored_sites.iter().any(|site| site.contains(span))
	}

	/// A local's declared type: the value's, for a checked pair.
	fn decl_ty(&mut self, local: Local) -> Type {
		let ty = self.mir.local_decls[local].ty;
		if self.next_results.contains(&local)
			&& let ty::Adt(adt, args) = ty.kind()
		{
			// `Option<Item>`, or `?`'s `ControlFlow<_, Item>`.
			let item = match crate::lower::library::Std::of(self.tcx, adt.did()) {
				Some(crate::lower::library::Std::ControlFlow) => args.type_at(1),
				_ => args.type_at(0),
			};
			let item = self.b.t.ty(item, self.span);
			if self.range_nexts.contains(&local) {
				return item;
			}
			return Type::Union(vec![item, Type::Rt("Done", Vec::new())]);
		}
		let ty = match (self.checked.contains(&local), ty.kind()) {
			(true, ty::Tuple(tys)) => tys[0],
			_ => ty,
		};
		self.b.t.ty(ty, self.span)
	}

	/// Where a terminator can go, without its unwinding.
	fn successors(&self, term: &mir::Terminator<'tcx>) -> Vec<BasicBlock> {
		match &term.kind {
			TerminatorKind::Goto { target, .. } => vec![*target],
			// An unreachable default is no edge: the last value takes its place.
			TerminatorKind::SwitchInt { targets, .. } => {
				let mut all = targets.all_targets().to_vec();
				if self.unreachable.contains(&targets.otherwise()) && all.len() > 1 {
					all.pop();
				}
				all
			}
			TerminatorKind::Assert { .. } if fails(term) => Vec::new(),
			TerminatorKind::Drop { target, .. } | TerminatorKind::Assert { target, .. } => vec![*target],
			TerminatorKind::Call { target, .. } => target.iter().copied().collect(),
			TerminatorKind::FalseEdge { real_target, .. } | TerminatorKind::FalseUnwind { real_target, .. } => {
				vec![*real_target]
			}
			TerminatorKind::Yield { resume, .. } => vec![*resume],
			TerminatorKind::InlineAsm { targets, .. } => targets.to_vec(),
			_ => Vec::new(),
		}
	}

	// Locals

	/// The JS name of a local held in a variable.
	fn var(&self, local: Local) -> Option<&str> {
		match &self.slots[local] {
			Slot::Var(name) => Some(name),
			_ => None,
		}
	}

	/// Whether `local` can be held back as a pending temporary: one assignment, one read.
	fn foldable(&self, local: Local) -> bool {
		matches!(self.slots[local], Slot::Var(_))
			&& self.defs[local] == 1
			&& self.uses[local] == 1
			&& !self.borrowed[local]
			&& !self.is_arg(local)
	}

	// Emission

	/// Emit the statements `stmts`, which together are one MIR statement, folding in the
	/// pending temporaries they read once where that keeps every effect in order.
	/// `writes` is the local the statement assigns whole. If `hold`, nothing runs now:
	/// the statements are a value being held back itself.
	fn emit(&mut self, stmts: Vec<Stmt>, writes: Option<&str>, hold: bool, out: &mut Vec<Stmt>) {
		if self.pending.is_empty() {
			out.extend(stmts);
			return;
		}
		// Where each pending temporary is read, in evaluation order, and what else is read
		// before it.
		let mut reads = Vec::new();
		for stmt in &stmts {
			scan_stmt(stmt, &mut reads, false);
		}
		let names: FxHashMap<&str, usize> = self
			.pending
			.iter()
			.enumerate()
			.map(|(i, p)| (p.name.as_str(), i))
			.collect();
		let mut count = vec![0u32; self.pending.len()];
		let mut deferred = vec![false; self.pending.len()];
		for read in &reads {
			if let Read::Ident(name, lazy) = read
				&& let Some(&i) = names.get(name.as_str())
			{
				count[i] += 1;
				deferred[i] |= *lazy;
			}
		}
		let fold: Vec<bool> = (0..self.pending.len()).map(|i| count[i] == 1 && !deferred[i]).collect();
		// Check the folded ones run in their order, after nothing they could change or be
		// changed by.
		let mut ok = true;
		let mut last: Option<u32> = None;
		let mut seen_memory = false;
		let mut seen_effect = false;
		for read in &reads {
			match read {
				Read::Ident(name, _) => match names.get(name.as_str()) {
					Some(&i) if fold[i] => {
						let p = &self.pending[i];
						if last.is_some_and(|l| l >= p.order)
							|| (p.effect && (seen_memory || seen_effect))
							|| (p.memory && seen_effect)
						{
							ok = false;
						}
						last = Some(p.order);
					}
					Some(_) => {}
					None => seen_memory |= self.shared.contains(name.as_str()),
				},
				Read::Memory => seen_memory = true,
				Read::Effect => seen_effect = true,
			}
		}
		// What stays pending past this statement, and what must run before it.
		let first_folded = (0..self.pending.len())
			.filter(|&i| fold[i])
			.map(|i| self.pending[i].order)
			.min();
		let mut flush = Vec::new();
		for (i, p) in self.pending.iter().enumerate() {
			if fold[i] {
				continue;
			}
			let read = count[i] > 0;
			let keep = match hold {
				true => !read,
				false => {
					let overwritten = writes.is_some_and(|w| w == p.name || p.reads.iter().any(|r| r == w));
					!read && !p.effect && !p.memory && !overwritten
				}
			};
			let after = first_folded.is_some_and(|f| p.order > f);
			if !keep {
				ok &= !after;
				flush.push(i);
			} else if hold && after && (p.effect || p.memory) {
				// It ran after something folded into a value held back past it.
				ok = false;
			}
		}
		if !ok {
			self.flush(out);
			out.extend(stmts);
			return;
		}
		let pending = std::mem::take(&mut self.pending);
		let mut stmts = stmts;
		let mut kept = Vec::new();
		for (i, p) in pending.into_iter().enumerate() {
			if fold[i] {
				for stmt in &mut stmts {
					if subst_stmt(stmt, &p.name, &p.value) {
						break;
					}
				}
			} else if flush.contains(&i) {
				self.flush_one(p, out);
			} else {
				kept.push(p);
			}
		}
		self.pending = kept;
		out.extend(stmts);
	}

	fn flush_one(&mut self, p: Pending, out: &mut Vec<Stmt>) {
		if let Some(local) = self.local_named(&p.name) {
			self.assigned[local] = true;
		}
		let value = match p.copy {
			true => Expr::rt("copy", vec![p.value]),
			false => p.value,
		};
		out.push(match p.decl {
			true => StmtKind::decl(false, p.name, None, Some(value)).at(None),
			false => StmtKind::Expr(Expr::ident(&p.name).assign(value)).at(None),
		});
	}

	/// Emit the pending temporaries `e` reads, and those held back before them, so that
	/// `e` can be read more than once, or later.
	pub(super) fn materialize(&mut self, e: &Expr, out: &mut Vec<Stmt>) {
		let mut names = Vec::new();
		reads_of(e, &mut names);
		let Some(last) = self
			.pending
			.iter()
			.filter(|p| names.contains(&p.name))
			.map(|p| p.order)
			.max()
		else {
			return;
		};
		let pending = std::mem::take(&mut self.pending);
		let (flush, keep): (Vec<Pending>, Vec<Pending>) = pending.into_iter().partition(|p| p.order <= last);
		self.pending = keep;
		for p in flush {
			self.flush_one(p, out);
		}
	}

	fn local_named(&self, name: &str) -> Option<Local> {
		self.slots
			.iter_enumerated()
			.find(|(_, slot)| matches!(slot, Slot::Var(n) if n == name))
			.map(|(local, _)| local)
	}

	/// `exprs`, after the statements `pre`, with what they read that is held back folded
	/// in where that keeps every effect in order; what can't fold is emitted to `out`.
	fn fold_exprs(&mut self, pre: Vec<Stmt>, exprs: Vec<Expr>, hold: bool, out: &mut Vec<Stmt>) -> Vec<Expr> {
		let n = exprs.len();
		let mut stmts = pre;
		stmts.extend(exprs.into_iter().map(|e| StmtKind::Expr(e).at(None)));
		let mut emitted = Vec::new();
		self.emit(stmts, None, hold, &mut emitted);
		let folded = emitted
			.drain(emitted.len() - n..)
			.map(|s| match s.kind {
				StmtKind::Expr(e) => e,
				_ => unreachable!("an expression statement"),
			})
			.collect();
		out.extend(emitted);
		folded
	}

	/// Hold back `value`, the assignment of `local`, to fold into what reads it.
	fn hold(&mut self, local: Local, value: Expr, effect: bool, memory: bool, parts: Option<Vec<Expr>>) {
		let name = self.var(local).expect("a pending local is a variable").to_string();
		self.order += 1;
		let mut reads = Vec::new();
		reads_of(&value, &mut reads);
		let copy = std::mem::take(&mut self.copy_on_flush);
		self.pending.push(Pending {
			name,
			value,
			order: self.order,
			effect,
			memory,
			parts,
			reads,
			copy,
			decl: false,
		});
	}

	/// The parts of the pending aggregate `local`, each held back in its place, so the
	/// expression that reads them keeps what they do and read in order.
	fn take_parts(&mut self, op: &Operand<'tcx>) -> Option<Vec<Expr>> {
		let Operand::Move(place) = op else {
			return None;
		};
		let local = place.as_local()?;
		let name = self.var(local)?;
		let i = self.pending.iter().position(|p| p.name == name && p.parts.is_some())?;
		let p = self.pending.remove(i);
		let parts = p.parts.expect("checked above");
		if !p.effect && !p.memory {
			return Some(parts);
		}
		let n = parts.len() as u32;
		for q in &mut self.pending {
			if q.order > p.order {
				q.order += n;
			}
		}
		self.order += n;
		let mut names = Vec::new();
		let mut held = 0;
		for value in parts {
			let mut events = Vec::new();
			scan(&value, &mut events, false);
			let effect = events.iter().any(|r| matches!(r, Read::Effect));
			let memory = events.iter().any(|r| match r {
				Read::Memory => true,
				Read::Ident(name, _) => self.shared.contains(name.as_str()),
				Read::Effect => false,
			});
			if !effect && !memory {
				names.push(value);
				continue;
			}
			let mut reads = Vec::new();
			reads_of(&value, &mut reads);
			let name = self.b.fresh("part");
			names.push(Expr::ident(&name));
			self.pending.insert(
				i + held,
				Pending {
					name,
					value,
					order: p.order + 1 + held as u32,
					effect,
					memory,
					parts: None,
					reads,
					copy: false,
					decl: true,
				},
			);
			held += 1;
		}
		Some(names)
	}

	/// Assign `value` to the local `local` now, or hold it back.
	fn set_local(
		&mut self,
		local: Local,
		value: Expr,
		effect: bool,
		memory: bool,
		parts: Option<Vec<Expr>>,
		out: &mut Vec<Stmt>,
	) {
		match self.slots[local].clone() {
			Slot::Var(name) => {
				if self.uses[local] == 0 {
					// Nothing reads it: only its effects run.
					if effect {
						self.emit(vec![StmtKind::Expr(value).at(None)], None, false, out);
					}
					return;
				}
				if self.foldable(local) {
					// What it reads that is held back folds into it first, so nothing held
					// back reads another, and its effects and reads go with it.
					let mut reads = Vec::new();
					reads_of(&value, &mut reads);
					let (effect, memory) = self
						.pending
						.iter()
						.filter(|p| reads.contains(&p.name))
						.fold((effect, memory), |(e, m), p| (e || p.effect, m || p.memory));
					let mut folded = self.fold_exprs(Vec::new(), vec![value], true, out);
					let value = folded.pop().unwrap();
					let parts = parts.and_then(|_| parts_of(&value));
					self.hold(local, value, effect, memory, parts);
					return;
				}
				self.assigned[local] = true;
				let stmt = StmtKind::Expr(Expr::ident(&name).assign(value)).at(None);
				self.emit(vec![stmt], Some(&name), false, out);
				// An aggregate assigned once and never changed: its operands, as emitted,
				// read in place of its fields.
				if parts.is_some()
					&& self.defs[local] == 1
					&& !self.mutated[local]
					&& let Some(Stmt {
						kind: StmtKind::Expr(Expr::Assign(_, value)),
						..
					}) = out.last()
					&& let Some(parts) = parts_of(value)
				{
					self.var_parts.insert(local, parts);
				}
			}
			Slot::Unit => {
				if effect {
					self.emit(vec![StmtKind::Expr(value).at(None)], None, false, out);
				}
			}
			Slot::Alias(_) | Slot::Discr(_) => unreachable!("set by their own assignment"),
		}
	}

	// Blocks

	fn block(&mut self, bb: BasicBlock, out: &mut Vec<Stmt>) -> Exit {
		let mir = self.mir;
		let data = &mir.basic_blocks[bb];
		self.set_unwind(data.terminator(), out);
		for stmt in &data.statements {
			if !self.in_ignored(stmt.source_info.span) {
				self.statement(stmt, out);
			}
		}
		if let Some(&to) = self.redirect.get(&bb) {
			return Exit::Goto(to.as_usize());
		}
		self.terminator(data.terminator(), out)
	}

	fn statement(&mut self, stmt: &mir::Statement<'tcx>, out: &mut Vec<Stmt>) {
		let span = stmt.source_info.span;
		match &stmt.kind {
			StatementKind::Assign(assign) => {
				let (place, rvalue) = &**assign;
				self.assign(*place, rvalue, span, out);
			}
			StatementKind::SetDiscriminant { place, variant_index } => {
				let target = self.set_discriminant(**place, *variant_index, span);
				if let Some(stmt) = target {
					self.emit(vec![stmt], None, false, out);
				}
			}
			StatementKind::Intrinsic(intrinsic) => match &**intrinsic {
				mir::NonDivergingIntrinsic::Assume(_) => {}
				mir::NonDivergingIntrinsic::CopyNonOverlapping(_) => {
					self.err(span, "`copy_nonoverlapping` is not supported");
				}
			},
			StatementKind::FakeRead(..)
			| StatementKind::StorageLive(_)
			| StatementKind::StorageDead(_)
			| StatementKind::PlaceMention(_)
			| StatementKind::AscribeUserType(..)
			| StatementKind::Coverage(_)
			| StatementKind::ConstEvalCounter
			| StatementKind::Nop
			| StatementKind::BackwardIncompatibleDropHint { .. } => {}
		}
	}

	fn assign(&mut self, place: Place<'tcx>, rvalue: &Rvalue<'tcx>, span: Span, out: &mut Vec<Stmt>) {
		if let Some(local) = place.as_local() {
			match self.slots[local] {
				Slot::Alias(_) => {
					let target = match rvalue {
						Rvalue::Ref(_, kind, p) => match self.mut_subslice(*kind, *p, span) {
							Some(e) => e,
							None => self.place_expr(*p, span, out),
						},
						Rvalue::CopyForDeref(p) => self.place_expr(*p, span, out),
						Rvalue::Use(Operand::Copy(p) | Operand::Move(p), _) => self.place_expr(*p, span, out),
						Rvalue::Cast(_, Operand::Copy(p) | Operand::Move(p), _) => self.place_expr(*p, span, out),
						_ => unreachable!("an alias is a borrow or a copy of one"),
					};
					self.materialize(&target, out);
					self.slots[local] = Slot::Alias(Some(target));
					return;
				}
				Slot::Discr(_) => {
					let Rvalue::Discriminant(p) = rvalue else {
						unreachable!("checked by discr_only")
					};
					let ty = p.ty(&self.mir.local_decls, self.tcx).ty;
					let target = self.place_expr(*p, span, out);
					let target = self.path(target, "scrutinee", out);
					self.slots[local] = Slot::Discr(Some((target, ty)));
					return;
				}
				_ => {}
			}
		}
		let mut pre = Vec::new();
		// A copy into a local nothing changes in place can share what it copies, if that
		// doesn't change either. One into a temporary only an argument reads shares it as
		// long as it folds into the argument, before anything could change it, if the callee
		// can't change it either (a local's own value nothing borrows, or a callee given
		// nothing that changes memory) nor return it.
		self.into_frozen = place.as_local().is_some_and(|l| !self.mutated[l]);
		let shares = place.as_local().is_some_and(|l| {
			self.arg_use[l]
				&& !self.arg_kept[l]
				&& self.foldable(l)
				&& matches!(rvalue, Rvalue::Use(Operand::Copy(p), _)
					if !self.frozen(*p)
						&& (own(*p) && !self.borrowed[p.local] || self.arg_alone[l])
						&& self.b.needs_copy(p.ty(&self.mir.local_decls, self.tcx).ty))
		});
		if shares {
			self.into_frozen = true;
			self.force_share = true;
		}
		let value = self.rvalue(rvalue, place, span, &mut pre);
		self.into_frozen = false;
		self.force_share = false;
		self.copy_on_flush = shares;
		let Some(value) = value else {
			out.extend(pre);
			return;
		};
		// What an `Iterator::next` returns, by the protocol.
		let value = match (
			self.next && place.local == mir::RETURN_PLACE && place.projection.is_empty(),
			rvalue,
		) {
			(false, _) => value,
			(true, Rvalue::Aggregate(kind, _)) if matches!(**kind, mir::AggregateKind::Adt(_, v, ..) if v.as_u32() == 0) => {
				Expr::Rt("DONE")
			}
			(true, Rvalue::Aggregate(..)) => value,
			(true, Rvalue::Use(Operand::Move(p) | Operand::Copy(p), _))
				if p.as_local().is_some_and(|l| self.next_results.contains(&l)) =>
			{
				value
			}
			(true, _) => done_of(value),
		};
		let effect = rvalue::effect(rvalue, self);
		// A copy shared until it folds into its argument reads its source there.
		let memory = rvalue::memory(rvalue, self) || shares;
		let parts = self.parts.take();
		if let Some(local) = place.as_local() {
			let parts = match rvalue {
				Rvalue::Aggregate(..) => parts,
				_ => None,
			};
			if !pre.is_empty() {
				self.emit(pre, None, false, out);
			}
			self.set_local(local, value, effect, memory, parts, out);
			return;
		}
		if !pre.is_empty() {
			self.emit(pre, None, false, out);
		}
		let stmts = self.store(place, value, span);
		self.emit(stmts, None, false, out);
	}

	/// Hold `value` in a path that reads it again for free.
	pub fn path(&mut self, value: Expr, base: &str, out: &mut Vec<Stmt>) -> Expr {
		if value.is_path() {
			self.materialize(&value, out);
			return value;
		}
		let name = self.b.fresh(base);
		let stmt = StmtKind::decl(false, name.clone(), None, Some(value)).at(None);
		self.emit(vec![stmt], None, false, out);
		Expr::ident(name)
	}

	fn terminator(&mut self, term: &mir::Terminator<'tcx>, out: &mut Vec<Stmt>) -> Exit {
		let span = term.source_info.span;
		match &term.kind {
			TerminatorKind::Goto { target, .. } => Exit::Goto(target.as_usize()),
			TerminatorKind::FalseEdge { real_target, .. } | TerminatorKind::FalseUnwind { real_target, .. } => {
				Exit::Goto(real_target.as_usize())
			}
			TerminatorKind::Return if !self.mir.local_decls[mir::RETURN_PLACE].ty.is_never() => {
				let ret = self.mir.local_decls[mir::RETURN_PLACE].ty;
				let value = match self.zero_sized(ret) {
					true => None,
					false => {
						let mut pre = Vec::new();
						// Uncopied: a caller that changes the result in place copies it.
						let value = self.place_expr(Place::return_place(), span, &mut pre);
						out.extend(pre);
						Some(value)
					}
				};
				let loc = self.b.t.loc(span);
				self.emit(vec![StmtKind::Return(value).at(loc)], None, false, out);
				self.flush(out);
				Exit::End
			}
			// A `Return` from a function of type `!`, as a generic instantiated at `!` is, is
			// unreachable too: it has no value to return, and `tsc` must see that.
			TerminatorKind::Unreachable | TerminatorKind::Return => {
				self.emit(
					vec![StmtKind::Expr(Expr::rt("unreachable", vec![Expr::undefined()])).at(None)],
					None,
					false,
					out,
				);
				Exit::End
			}
			TerminatorKind::SwitchInt { discr, targets } => self.switch(discr, targets, span, out),
			TerminatorKind::Drop { place, target, .. } => {
				let ty = place.ty(&self.mir.local_decls, self.tcx).ty;
				if self.b.t.drops(ty) {
					let mut pre = Vec::new();
					let value = self.place_expr(*place, span, &mut pre);
					if let Some(dropped) = self.b.t.drop_value(value, ty, span) {
						pre.push(StmtKind::Expr(dropped).at(None));
					}
					self.emit(pre, None, false, out);
				}
				Exit::Goto(target.as_usize())
			}
			TerminatorKind::Call {
				func,
				args,
				destination,
				target,
				fn_span,
				..
			} => {
				self.call(func, args, *destination, *fn_span, out);
				match target {
					Some(target) => Exit::Goto(target.as_usize()),
					None => {
						self.flush(out);
						Exit::End
					}
				}
			}
			TerminatorKind::Assert {
				cond,
				expected,
				msg,
				target,
				..
			} => {
				self.assert(cond, *expected, msg, span, out);
				match fails(term) {
					true => {
						self.flush(out);
						Exit::End
					}
					false => Exit::Goto(target.as_usize()),
				}
			}
			TerminatorKind::TailCall { .. } => {
				self.err(span, "`become` is not supported");
				Exit::End
			}
			TerminatorKind::InlineAsm { .. } => {
				self.err(span, "inline assembly is not supported");
				Exit::End
			}
			TerminatorKind::Yield { .. } | TerminatorKind::CoroutineDrop => {
				self.err(span, "a coroutine before its state transform");
				Exit::End
			}
			TerminatorKind::UnwindResume | TerminatorKind::UnwindTerminate(_) => match self.in_cleanup.clone() {
				Some(caught) => {
					self.flush(out);
					out.push(StmtKind::Throw(Expr::ident(caught)).at(None));
					Exit::End
				}
				None => {
					self.err(span, "unwinding outside cleanup");
					Exit::End
				}
			},
		}
	}

	fn switch(&mut self, discr: &Operand<'tcx>, targets: &mir::SwitchTargets, span: Span, out: &mut Vec<Stmt>) -> Exit {
		let mut arms: Vec<(u128, BasicBlock)> = targets.iter().collect();
		let mut default = targets.otherwise();
		// An unreachable default leaves the last value as the default.
		if self.unreachable.contains(&default)
			&& let Some((_, last)) = arms.pop()
		{
			default = last;
		}
		let discr_local = match discr {
			Operand::Copy(p) | Operand::Move(p) => p.as_local(),
			_ => None,
		};
		let tests: Vec<(Expr, BasicBlock)> = match discr_local.map(|l| self.slots[l].clone()) {
			Some(Slot::Discr(Some((scrut, ty)))) => arms
				.iter()
				.map(|&(value, target)| (self.variant_test(&scrut, ty, value, span), target))
				.collect(),
			_ => {
				let ty = discr.ty(&self.mir.local_decls, self.tcx);
				let mut pre = Vec::new();
				let value = self.operand(discr, span, &mut pre);
				let value = match arms.len() {
					0 | 1 => value,
					_ => match value.is_path() {
						true => value,
						false => {
							let name = self.b.fresh("value");
							pre.push(StmtKind::decl(false, name.clone(), None, Some(value)).at(None));
							Expr::ident(name)
						}
					},
				};
				let mut tests = Vec::new();
				for &(v, target) in &arms {
					let test = match ty.kind() {
						ty::Bool => match v {
							0 => value.clone().not(),
							_ => value.clone(),
						},
						_ => match super::num::Int::of(ty) {
							Some(int) => {
								let lit = int.lit(int.fold(v as i128));
								int.compare("===", value.clone(), lit)
							}
							None => self.err(span, format!("a switch on `{ty}`")),
						},
					};
					tests.push((test, target));
				}
				// What the tests read that is held back folds into them.
				let targets: Vec<BasicBlock> = tests.iter().map(|(_, t)| *t).collect();
				let conds = self.fold_exprs(pre, tests.into_iter().map(|(c, _)| c).collect(), false, out);
				conds.into_iter().zip(targets).collect()
			}
		};
		self.flush(out);
		// Arms to one target are one test.
		let mut grouped: Vec<(Expr, BasicBlock)> = Vec::new();
		for (cond, target) in tests {
			match grouped.iter_mut().find(|(_, t)| *t == target) {
				Some((c, _)) => *c = std::mem::replace(c, Expr::undefined()).binary("||", cond),
				None => grouped.push((cond, target)),
			}
		}
		let tests: Vec<(Expr, usize)> = grouped.into_iter().map(|(c, t)| (fold(c), t.as_usize())).collect();
		match tests.is_empty() {
			true => Exit::Goto(default.as_usize()),
			false => Exit::Branch(tests, default.as_usize()),
		}
	}

	fn assert(
		&mut self,
		cond: &Operand<'tcx>,
		expected: bool,
		msg: &mir::AssertMessage<'tcx>,
		span: Span,
		out: &mut Vec<Stmt>,
	) {
		// Checked arithmetic already panicked with this message as it made its value.
		if let Operand::Move(p) | Operand::Copy(p) = cond
			&& p.projection.len() == 1
			&& self.checked.contains(&p.local)
		{
			return;
		}
		if redundant(msg) {
			return;
		}
		let mut pre = Vec::new();
		let value = self.operand(cond, span, &mut pre);
		let mut exprs = vec![value];
		if let mir::AssertKind::BoundsCheck { len, index } = msg {
			exprs.push(self.operand(len, span, &mut pre));
			exprs.push(self.operand(index, span, &mut pre));
		}
		let mut folded = self.fold_exprs(pre, exprs, false, out).into_iter();
		let value = fold(folded.next().unwrap());
		let operands: Vec<Expr> = folded.collect();
		let mut pre = Vec::new();
		let failed = fold(match expected {
			true => value.not(),
			false => value,
		});
		// An assert of a constant that holds, such as of a shift by a literal.
		if matches!(&failed, Expr::Lit(t) if t == "false") {
			return;
		}
		let message = match self.assert_message(msg, operands, span) {
			Some(message) => message,
			None => return,
		};
		// An assert that always fails, such as a coroutine's after it returned.
		if matches!(&failed, Expr::Lit(t) if t == "true") {
			pre.push(StmtKind::Expr(Expr::rt("panic", vec![message])).at(self.b.t.loc(span)));
			self.emit(pre, None, false, out);
			return;
		}
		pre.push(
			StmtKind::If {
				cond: failed,
				then: vec![StmtKind::Expr(Expr::rt("panic", vec![message])).at(None)],
				els: Vec::new(),
			}
			.at(self.b.t.loc(span)),
		);
		self.emit(pre, None, false, out);
	}

	/// The message an assert panics with, as the runtime spells it.
	fn assert_message(&mut self, msg: &mir::AssertMessage<'tcx>, operands: Vec<Expr>, span: Span) -> Option<Expr> {
		use mir::AssertKind;
		use rustc_middle::mir::BinOp;
		let text = |s: &str| Some(Expr::Str(s.to_string()));
		match msg {
			AssertKind::BoundsCheck { .. } => {
				let [len, index] = <[Expr; 2]>::try_from(operands).ok()?;
				Some(
					Expr::Str("index out of bounds: the len is ".into())
						.binary("+", len)
						.binary("+", Expr::Str(" but the index is ".into()))
						.binary("+", index),
				)
			}
			AssertKind::Overflow(op, ..) => match op {
				BinOp::Add => text("attempt to add with overflow"),
				BinOp::Sub => text("attempt to subtract with overflow"),
				BinOp::Mul => text("attempt to multiply with overflow"),
				BinOp::Div => text("attempt to divide with overflow"),
				BinOp::Rem => text("attempt to calculate the remainder with overflow"),
				BinOp::Shl => text("attempt to shift left with overflow"),
				BinOp::Shr => text("attempt to shift right with overflow"),
				_ => {
					self.err(span, format!("an overflow check of `{op:?}`"));
					None
				}
			},
			AssertKind::OverflowNeg(_) => text("attempt to negate with overflow"),
			AssertKind::DivisionByZero(_) => text("attempt to divide by zero"),
			AssertKind::RemainderByZero(_) => text("attempt to calculate the remainder with a divisor of zero"),
			AssertKind::ResumedAfterReturn(_) => text("`async fn` resumed after completion"),
			AssertKind::ResumedAfterPanic(_) => text("`async fn` resumed after panicking"),
			_ => {
				self.err(span, format!("an assert of `{msg:?}`"));
				None
			}
		}
	}

	/// Emit every pending temporary, in order.
	fn flush_pending(&mut self, out: &mut Vec<Stmt>) {
		for p in std::mem::take(&mut self.pending) {
			self.flush_one(p, out);
		}
	}
}

impl cfg::Lower for Fx<'_, '_, '_, '_> {
	fn node(&mut self, node: usize, out: &mut Vec<Stmt>) -> Exit {
		self.block(BasicBlock::from_usize(node), out)
	}

	fn flush(&mut self, out: &mut Vec<Stmt>) {
		self.flush_pending(out);
		// Control goes elsewhere, which other code may reach with another cleanup.
		if let Some(unwind) = &mut self.unwind {
			unwind.known = None;
		}
	}
}

impl Fx<'_, '_, '_, '_> {
	fn flush(&mut self, out: &mut Vec<Stmt>) {
		cfg::Lower::flush(self, out);
	}

	/// A block's statements and how it leaves, for the cleanups' lowering.
	fn block_mapped(&mut self, bb: BasicBlock, out: &mut Vec<Stmt>) -> Exit {
		self.block(bb, out)
	}

	/// Emit a statement every held-back value that can do anything runs before.
	fn emit_barrier(&mut self, stmt: Stmt, out: &mut Vec<Stmt>) {
		self.emit(vec![stmt], None, false, out);
	}
}

impl<'tcx> Fx<'_, '_, '_, 'tcx> {
	/// Whether a value of `ty` can change memory: it holds a mutable reference, a raw
	/// pointer, or something with interior mutability.
	fn reaches_mutable(&self, ty: Ty<'tcx>) -> bool {
		use rustc_middle::ty::TypeVisitableExt;
		let env = self.b.t.typing_env_pub();
		ty.walk().any(|arg| {
			arg.as_type().is_some_and(|t| {
				matches!(t.kind(), ty::Ref(_, _, rustc_ast::Mutability::Mut) | ty::RawPtr(..))
					// Inside a binder, as a function pointer's signature, only its kind tells.
					|| !t.has_escaping_bound_vars() && !t.is_freeze(self.tcx, env)
			})
		})
	}
}

/// Whether a value of `ty` could hold a `part`: contains its type, or hides what it holds.
fn holds<'tcx>(ty: Ty<'tcx>, part: Ty<'tcx>) -> bool {
	ty.walk().any(|arg| {
		arg.as_type().is_some_and(|t| {
			t == part
				|| matches!(
					t.kind(),
					ty::Dynamic(..) | ty::Alias(..) | ty::Param(_) | ty::Closure(..) | ty::Coroutine(..)
				)
		})
	})
}

/// Whether `place` is part of its local's own value, rather than reached through a
/// reference or pointer.
fn own(place: Place<'_>) -> bool {
	place
		.projection
		.iter()
		.all(|e| !matches!(e, mir::ProjectionElem::Deref))
}

/// What an expression reads, in evaluation order.
enum Read {
	/// A name, and whether it is read later or maybe not at all: in a function, or in a
	/// branch that may not run.
	Ident(String, bool),
	/// Memory: a member or an element.
	Memory,
	/// A call, which may do anything.
	Effect,
}

fn scan_stmt(stmt: &Stmt, reads: &mut Vec<Read>, lazy: bool) {
	match &stmt.kind {
		StmtKind::Expr(e) | StmtKind::Return(Some(e)) | StmtKind::Throw(e) => scan(e, reads, lazy),
		StmtKind::Decl { init: Some(e), .. } => scan(e, reads, lazy),
		StmtKind::If { cond, then, els } => {
			scan(cond, reads, lazy);
			then.iter().chain(els).for_each(|s| scan_stmt(s, reads, true));
		}
		StmtKind::While { cond, body, .. } => {
			scan(cond, reads, true);
			body.iter().for_each(|s| scan_stmt(s, reads, true));
		}
		StmtKind::Block { body, .. } => body.iter().for_each(|s| scan_stmt(s, reads, lazy)),
		_ => {}
	}
}

fn scan(e: &Expr, reads: &mut Vec<Read>, lazy: bool) {
	match e {
		Expr::Ident(name) => reads.push(Read::Ident(name.clone(), lazy)),
		Expr::This | Expr::Sym(_) | Expr::Rt(_) | Expr::Lit(_) | Expr::Str(_) | Expr::Yield => {}
		Expr::Array(items) => items.iter().for_each(|e| scan(e, reads, lazy)),
		Expr::Object(props) => props.iter().for_each(|(_, e)| scan(e, reads, lazy)),
		// The runtime's and the program's own items never change.
		Expr::Member(base, _) if matches!(&**base, Expr::Rt(_) | Expr::Sym(_)) => {}
		Expr::Member(base, _) => {
			scan(base, reads, lazy);
			reads.push(Read::Memory);
		}
		Expr::Index(base, index) => {
			scan(base, reads, lazy);
			scan(index, reads, lazy);
			reads.push(Read::Memory);
		}
		Expr::Call(f, args) | Expr::New(f, args) => {
			// A method's lookup reads no data the arguments could change: only its receiver.
			match &**f {
				Expr::Member(recv, _) => scan(recv, reads, lazy),
				f => scan(f, reads, lazy),
			}
			args.iter().for_each(|e| scan(e, reads, lazy));
			reads.push(Read::Effect);
		}
		Expr::Unary(_, e) | Expr::Located(_, e) | Expr::As(e, _) => scan(e, reads, lazy),
		Expr::Postfix(e, _) => {
			scan(e, reads, lazy);
			reads.push(Read::Effect);
		}
		Expr::Binary(l, op, r) => {
			scan(l, reads, lazy);
			let lazy_r = lazy || matches!(*op, "&&" | "||" | "??");
			scan(r, reads, lazy_r);
		}
		Expr::Cond(c, a, b) => {
			scan(c, reads, lazy);
			scan(a, reads, true);
			scan(b, reads, true);
		}
		Expr::Assign(target, value) => {
			// A variable assigned is not read; a place's base and index are, before the value.
			if !matches!(**target, Expr::Ident(_)) {
				scan(target, reads, lazy);
			}
			scan(value, reads, lazy);
			reads.push(Read::Effect);
		}
		Expr::Arrow(_, _, body) => body.iter().for_each(|s| scan_stmt(s, reads, true)),
		Expr::Lambda(_, body) => scan(body, reads, true),
	}
}

/// Replace the one read of `name` in `stmt` with `value`; whether it was there.
fn subst_stmt(stmt: &mut Stmt, name: &str, value: &Expr) -> bool {
	match &mut stmt.kind {
		StmtKind::Expr(e) | StmtKind::Return(Some(e)) | StmtKind::Throw(e) => subst(e, name, value),
		StmtKind::Decl { init: Some(e), .. } => subst(e, name, value),
		StmtKind::If { cond, .. } => subst(cond, name, value),
		_ => false,
	}
}

fn subst(e: &mut Expr, name: &str, value: &Expr) -> bool {
	match e {
		Expr::Ident(n) if n == name => {
			*e = value.clone();
			true
		}
		Expr::Ident(_) | Expr::This | Expr::Sym(_) | Expr::Rt(_) | Expr::Lit(_) | Expr::Str(_) | Expr::Yield => false,
		Expr::Array(items) => items.iter_mut().any(|e| subst(e, name, value)),
		Expr::Object(props) => props.iter_mut().any(|(_, e)| subst(e, name, value)),
		Expr::Member(base, _) => subst(base, name, value),
		Expr::Index(base, index) => subst(base, name, value) || subst(index, name, value),
		Expr::Call(f, args) | Expr::New(f, args) => {
			subst(f, name, value) || args.iter_mut().any(|e| subst(e, name, value))
		}
		Expr::Unary(_, e) | Expr::Located(_, e) | Expr::As(e, _) | Expr::Postfix(e, _) => subst(e, name, value),
		Expr::Binary(l, _, r) | Expr::Assign(l, r) => subst(l, name, value) || subst(r, name, value),
		Expr::Cond(c, a, b) => subst(c, name, value) || subst(a, name, value) || subst(b, name, value),
		Expr::Arrow(..) | Expr::Lambda(..) => false,
	}
}

/// Drop the declarations of variables nothing reads, keeping the effects of what was
/// assigned to them.
fn prune_unread(stmts: &mut Vec<Stmt>) {
	let mut declared = Vec::new();
	for stmt in stmts.iter() {
		if let StmtKind::Declare { name, .. } = &stmt.kind {
			declared.push(name.clone());
		}
	}
	let mut reads = Vec::new();
	for stmt in stmts.iter() {
		scan_reads(stmt, &mut reads);
	}
	let read: FxHashSet<&str> = reads.iter().map(String::as_str).collect();
	let unread: FxHashSet<String> = declared.into_iter().filter(|n| !read.contains(n.as_str())).collect();
	if unread.is_empty() {
		return;
	}
	strip_assignments(stmts, &unread);
}

/// Every name `stmt` reads, but not as the target of an assignment to it.
fn scan_reads(stmt: &Stmt, out: &mut Vec<String>) {
	let mut visit = |e: &Expr| reads_of(e, out);
	match &stmt.kind {
		StmtKind::Expr(e) | StmtKind::Return(Some(e)) | StmtKind::Throw(e) => visit(e),
		StmtKind::Decl { init: Some(e), .. } => visit(e),
		StmtKind::If { cond, then, els } => {
			visit(cond);
			then.iter().chain(els).for_each(|s| scan_reads(s, out));
		}
		StmtKind::While { cond, body, .. } => {
			reads_of(cond, out);
			body.iter().for_each(|s| scan_reads(s, out));
		}
		StmtKind::Block { body, .. } => body.iter().for_each(|s| scan_reads(s, out)),
		StmtKind::Switch {
			disc, cases, default, ..
		} => {
			reads_of(disc, out);
			for (v, body) in cases {
				reads_of(v, out);
				body.iter().for_each(|s| scan_reads(s, out));
			}
			default.iter().for_each(|s| scan_reads(s, out));
		}
		StmtKind::Try { body, finally } => body.iter().chain(finally).for_each(|s| scan_reads(s, out)),
		StmtKind::Catch { body, handler, .. } => body.iter().chain(handler).for_each(|s| scan_reads(s, out)),
		StmtKind::ForRange { start, end, body, .. } => {
			reads_of(start, out);
			reads_of(end, out);
			body.iter().for_each(|s| scan_reads(s, out));
		}
		StmtKind::ForOf { iter, body, .. } => {
			reads_of(iter, out);
			body.iter().for_each(|s| scan_reads(s, out));
		}
		_ => {}
	}
}

fn reads_of(e: &Expr, out: &mut Vec<String>) {
	match e {
		Expr::Ident(name) => out.push(name.clone()),
		Expr::This | Expr::Sym(_) | Expr::Rt(_) | Expr::Lit(_) | Expr::Str(_) | Expr::Yield => {}
		Expr::Array(items) => items.iter().for_each(|e| reads_of(e, out)),
		Expr::Object(props) => props.iter().for_each(|(_, e)| reads_of(e, out)),
		Expr::Member(base, _) => reads_of(base, out),
		Expr::Index(base, index) => {
			reads_of(base, out);
			reads_of(index, out);
		}
		Expr::Call(f, args) | Expr::New(f, args) => {
			reads_of(f, out);
			args.iter().for_each(|e| reads_of(e, out));
		}
		Expr::Unary(_, e) | Expr::Located(_, e) | Expr::As(e, _) | Expr::Lambda(_, e) | Expr::Postfix(e, _) => {
			reads_of(e, out)
		}
		Expr::Binary(l, _, r) => {
			reads_of(l, out);
			reads_of(r, out);
		}
		Expr::Cond(c, a, b) => {
			reads_of(c, out);
			reads_of(a, out);
			reads_of(b, out);
		}
		// Assigning a name whole doesn't read it.
		Expr::Assign(target, value) => {
			if !matches!(&**target, Expr::Ident(_)) {
				reads_of(target, out);
			}
			reads_of(value, out);
		}
		Expr::Arrow(_, _, body) => body.iter().for_each(|s| scan_reads(s, out)),
	}
}

fn strip_assignments(stmts: &mut Vec<Stmt>, unread: &FxHashSet<String>) {
	stmts.retain(|s| !matches!(&s.kind, StmtKind::Declare { name, .. } if unread.contains(name)));
	for stmt in stmts.iter_mut() {
		match &mut stmt.kind {
			StmtKind::Expr(e) => {
				if let Expr::Assign(target, value) = e
					&& let Expr::Ident(name) = &**target
					&& unread.contains(name)
				{
					*e = std::mem::replace(&mut **value, Expr::undefined());
				}
			}
			StmtKind::If { then, els, .. } => {
				strip_assignments(then, unread);
				strip_assignments(els, unread);
			}
			StmtKind::While { body, .. } | StmtKind::Block { body, .. } => strip_assignments(body, unread),
			StmtKind::Switch { cases, default, .. } => {
				cases.iter_mut().for_each(|(_, body)| strip_assignments(body, unread));
				strip_assignments(default, unread);
			}
			StmtKind::Try { body, finally } => {
				strip_assignments(body, unread);
				strip_assignments(finally, unread);
			}
			StmtKind::Catch { body, handler, .. } => {
				strip_assignments(body, unread);
				strip_assignments(handler, unread);
			}
			_ => {}
		}
	}
	// What is left of an assignment and does nothing.
	stmts.retain(|s| !matches!(&s.kind, StmtKind::Expr(e) if inert(e)));
}

/// The operands of an aggregate, from its expression.
fn parts_of(value: &Expr) -> Option<Vec<Expr>> {
	match value {
		Expr::Array(items) => Some(items.clone()),
		Expr::As(inner, _) => parts_of(inner),
		Expr::Call(f, items) if matches!(&**f, Expr::Member(base, crate::ts::Key::Name(of)) if of == "of" && matches!(&**base, Expr::Ident(u) if u == "Uint8Array")) => {
			Some(items.clone())
		}
		Expr::Object(props) => Some(props.iter().map(|(_, v)| v.clone()).collect()),
		Expr::New(f, items) if matches!(&**f, Expr::Rt("Range")) => Some(items.clone()),
		_ => None,
	}
}

/// `e` with comparisons of two number literals, and their negations, worked out.
pub fn fold(e: Expr) -> Expr {
	match e {
		Expr::Binary(l, op, r) => {
			let (l, r) = (fold(*l), fold(*r));
			let num = |e: &Expr| match e {
				Expr::Lit(t) => t.parse::<f64>().ok(),
				_ => None,
			};
			if let (Some(a), Some(b)) = (num(&l), num(&r)) {
				// Integer arithmetic JS does exactly, in 32 bits for the bitwise operators.
				let small = |v: f64| v.fract() == 0.0 && v.abs() < 2f64.powi(31);
				if small(a) && small(b) {
					let (x, y) = (a as i64, b as i64);
					let int = match op {
						"<<" if (0..31).contains(&y) => Some((x as i32).wrapping_shl(y as u32) as i64),
						">>" if (0..32).contains(&y) => Some(((x as i32) >> y) as i64),
						"|" => Some((x as i32 | y as i32) as i64),
						"&" => Some((x as i32 & y as i32) as i64),
						"^" => Some((x as i32 ^ y as i32) as i64),
						"+" => Some(x + y),
						"-" => Some(x - y),
						_ => None,
					};
					if let Some(v) = int
						&& (x >= 0 && y >= 0 || !matches!(op, "<<" | ">>" | "|" | "&" | "^"))
					{
						return Expr::lit(v.to_string());
					}
				}
				let value = match op {
					"<" => Some(a < b),
					"<=" => Some(a <= b),
					">" => Some(a > b),
					">=" => Some(a >= b),
					"===" => Some(a == b),
					"!==" => Some(a != b),
					_ => None,
				};
				if let Some(value) = value {
					return Expr::lit(value.to_string());
				}
			}
			Expr::Binary(Box::new(l), op, Box::new(r))
		}
		// An element of an array made in place, as a format argument's tuple is read.
		Expr::Index(base, index) => {
			let (base, index) = (fold(*base), fold(*index));
			let items = match &base {
				Expr::Array(items) => Some(items),
				Expr::As(inner, _) => match &**inner {
					Expr::Array(items) => Some(items),
					_ => None,
				},
				_ => None,
			};
			if let (Some(items), Expr::Lit(i)) = (items, &index)
				&& let Ok(i) = i.parse::<usize>()
				&& i < items.len()
				&& items.iter().all(Expr::is_path)
			{
				return items[i].clone();
			}
			Expr::Index(Box::new(base), Box::new(index))
		}
		Expr::Unary("!", inner) => match fold(*inner) {
			Expr::Lit(t) if t == "true" => Expr::lit("false"),
			Expr::Lit(t) if t == "false" => Expr::lit("true"),
			inner => Expr::Unary("!", Box::new(inner)),
		},
		e => e,
	}
}

/// Whether an assert checks what the runtime's division, remainder, negation, or shift
/// checks as the operation it guards runs next, which panics with the same message.
fn redundant(msg: &mir::AssertMessage<'_>) -> bool {
	use mir::{AssertKind, BinOp};
	matches!(
		msg,
		AssertKind::OverflowNeg(_)
			| AssertKind::DivisionByZero(_)
			| AssertKind::RemainderByZero(_)
			| AssertKind::Overflow(BinOp::Div | BinOp::Rem | BinOp::Shl | BinOp::Shr, ..)
	)
}

/// An `Option` as `Iterator::next` returns it: the item, or `rt.DONE` for `None`.
pub fn done_of(option: Expr) -> Expr {
	match option {
		Expr::Lit(t) if t == "undefined" => Expr::Rt("DONE"),
		option => Expr::Rt("iter").member("fromOption").call(vec![option]),
	}
}

/// Whether `term` is an assert of a constant that fails.
fn fails(term: &mir::Terminator<'_>) -> bool {
	let TerminatorKind::Assert {
		cond: Operand::Constant(c),
		expected,
		..
	} = &term.kind
	else {
		return false;
	};
	c.const_.try_to_bool().is_some_and(|b| b != *expected)
}

/// Drop the assignments of a variable to itself, which ferrying a value through
/// temporaries leaves.
fn drop_self_assignments(stmts: &mut Vec<Stmt>) {
	stmts.retain(|s| !matches!(&s.kind, StmtKind::Expr(Expr::Assign(t, v)) if matches!((&**t, &**v), (Expr::Ident(a), Expr::Ident(b)) if a == b)));
	for stmt in stmts.iter_mut() {
		match &mut stmt.kind {
			StmtKind::If { then, els, .. } => {
				drop_self_assignments(then);
				drop_self_assignments(els);
			}
			StmtKind::While { body, .. } | StmtKind::Block { body, .. } => drop_self_assignments(body),
			StmtKind::Switch { cases, default, .. } => {
				cases.iter_mut().for_each(|(_, body)| drop_self_assignments(body));
				drop_self_assignments(default);
			}
			StmtKind::Try { body, finally } => {
				drop_self_assignments(body);
				drop_self_assignments(finally);
			}
			StmtKind::Catch { body, handler, .. } => {
				drop_self_assignments(body);
				drop_self_assignments(handler);
			}
			_ => {}
		}
	}
}

/// Whether evaluating `e` does nothing at all: a path, or a literal made of them.
fn inert(e: &Expr) -> bool {
	match e {
		Expr::Array(items) => items.iter().all(inert),
		Expr::Object(props) => props.iter().all(|(_, v)| inert(v)),
		Expr::As(e, _) | Expr::Located(_, e) => inert(e),
		Expr::Lambda(..) | Expr::Arrow(..) => true,
		e => e.is_path(),
	}
}

/// Whether `ty` is a `Range` of numbers that fit a JS number, which steps in place.
pub(super) fn small_range<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
	matches!(ty.kind(), ty::Adt(adt, args)
		if crate::lower::library::Std::of(tcx, adt.did()) == Some(crate::lower::library::Std::Range)
			&& crate::lower::num::Int::of(args.type_at(0)).is_some_and(|i| !i.big()))
}
