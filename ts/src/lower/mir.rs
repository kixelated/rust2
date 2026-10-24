//! The async spike's state machine prototype: an `async` body lowered from MIR after
//! rustc's coroutine transform. Its future is a class whose fields are its state, its
//! upvars, and its saved locals; `poll` is the transform's resume function and the
//! dispose its drop shim, each a loop over a `switch` on the basic block. Types, calls,
//! drops, formatting, and closures go through the THIR lowering's own.

use rustc_abi::{FieldIdx, VariantIdx};
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::def_id::LocalDefId;
use rustc_index::IndexVec;
use rustc_middle::mir::interpret::Scalar;
use rustc_middle::mir::visit::{PlaceContext, Visitor};
use rustc_middle::mir::{
	self, AggregateKind, AssertKind, BasicBlock, BinOp, CastKind, ConstValue, CoroutineSavedLocal, Local, Operand,
	Place, ProjectionElem, Rvalue, StatementKind, TerminatorKind, UnOp,
};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

use super::body::Body;
use super::fmt::{self, Piece, Trait};
use super::library::{Args, Std};
use super::num::{self, Int};
use super::{AdtRepr, camel};
use crate::program::{SymKind, Symbol};
use crate::ts::{self, Ctor, Expr, FieldId, Key, Param, Stmt, StmtKind, SymId, Type};

/// The class of one `async` body's future, by the role of each field.
struct Class<'tcx> {
	sym: SymId,
	def_id: LocalDefId,
	state: FieldId,
	upvars: Vec<FieldId>,
	saved: IndexVec<CoroutineSavedLocal, FieldId>,
	layout: &'tcx mir::CoroutineLayout<'tcx>,
}

/// The future of the `async` body `def_id`, a `ty`, capturing `upvars`: its class is
/// translated here, and this is a new instance of it.
pub fn coroutine<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	def_id: LocalDefId,
	ty: Ty<'tcx>,
	upvars: Vec<Expr>,
	span: Span,
) -> Expr {
	let tcx = b.t.tcx;
	let ty::Coroutine(_, args) = *ty.kind() else {
		return b.err(span, format!("an async body of type `{ty}`"));
	};
	let Ok(layout) = tcx.coroutine_layout(def_id.to_def_id(), args) else {
		return b.err(span, "an async body without a layout");
	};
	let parent = tcx.typeck_root_def_id(def_id.to_def_id());
	let name = pascal(&camel(tcx.item_name(parent).as_str()));
	let order = b.t.order(tcx.def_span(def_id));
	let class = b.t.program.sym(Symbol {
		module: b.t.module_of(parent),
		base: format!("{name}Future"),
		suffix: String::new(),
		owner: None,
		kind: SymKind::Class,
		order,
		name: String::new(),
	});
	let mut fields = Vec::new();
	let state = b.t.program.field(class, "state".into());
	fields.push(ts::Field {
		id: state,
		ty: Type::Builtin("number"),
		doc: Some("Which await it is suspended at, as the coroutine transform numbers them.".into()),
	});
	let mut upvar_fields = Vec::new();
	let captures = tcx.closure_captures(def_id);
	for (i, upvar) in args.as_coroutine().upvar_tys().iter().enumerate() {
		let base = captures
			.get(i)
			.map_or(format!("upvar{i}"), |c| camel(c.to_symbol().as_str()));
		let id = b.t.program.field(class, base);
		let ty = local_type(b, upvar, span);
		fields.push(ts::Field { id, ty, doc: None });
		upvar_fields.push(id);
	}
	let mut saved = IndexVec::new();
	for (local, field) in layout.field_tys.iter_enumerated() {
		let id = b.t.program.field(class, format!("s{}", local.as_usize()));
		let ty = local_type(b, b.subst(field.ty), span);
		fields.push(ts::Field { id, ty, doc: None });
		saved.push(id);
	}
	let info = Class {
		sym: class,
		def_id,
		state,
		upvars: upvar_fields,
		saved,
		layout,
	};
	b.t.program.items.push(ts::Item::Class {
		sym: class,
		params: Vec::new(),
		fields,
		ctor: Ctor::Named,
		doc: Some(format!(
			"The future of `{}`, as a state machine.",
			tcx.def_path_str(parent)
		)),
	});

	let body = tcx.optimized_mir(def_id);
	let ret = b.subst(args.as_coroutine().return_ty());
	let poll_ret = Type::Rt("Poll", vec![local_type(b, ret, span)]);
	let cx = "cx".to_string();
	let poll_body = Lowering::new(b, body, &info, Some(cx.clone())).lower();
	let mut methods = vec![(
		"poll",
		vec![Param {
			name: cx,
			ty: Some(Type::Rt("Context", vec![])),
		}],
		poll_ret,
		poll_body,
	)];
	match body.coroutine_drop() {
		Some(drop) => {
			let dispose = Lowering::new(b, drop, &info, None).lower();
			methods.push(("[Symbol.dispose]", Vec::new(), Type::Builtin("void"), dispose));
		}
		None => b.t.err(span, "an async body without a drop shim"),
	}
	for (name, params, ret, body) in methods {
		let sym = b.t.program.sym(Symbol {
			module: b.t.module_of(parent),
			base: name.into(),
			suffix: String::new(),
			owner: Some(class),
			kind: SymKind::Method { instance: true },
			order,
			name: String::new(),
		});
		b.t.program.methods.entry(class).or_default().push(ts::Function {
			sym,
			method: true,
			params,
			ret,
			body,
			loc: None,
			doc: None,
			generator: false,
		});
	}

	let mut props = vec![(Key::Field(state), Expr::lit("0"))];
	for (&id, value) in info.upvars.iter().zip(upvars) {
		props.push((Key::Field(id), value));
	}
	for &id in &info.saved {
		props.push((Key::Field(id), Expr::lit("undefined!")));
	}
	Expr::New(Box::new(Expr::Sym(class)), vec![Expr::Object(props)])
}

/// The TypeScript type of a local, for the types only MIR has.
fn local_type<'tcx>(b: &mut Body<'_, '_, 'tcx>, ty: Ty<'tcx>, span: Span) -> Type {
	match ty.kind() {
		ty::Adt(adt, args) => match super::library::std_path(b.t.tcx, adt.did()).last().map(String::as_str) {
			Some("NonNull") => local_type(b, args.type_at(0), span),
			Some("Argument") => Type::Rt("Fmt", vec![]),
			_ => b.t.ty(ty, span),
		},
		ty::Array(elem, _) | ty::Slice(elem) if is_argument(b, *elem) => Type::Array(Box::new(Type::Rt("Fmt", vec![]))),
		ty::Ref(_, inner, _) | ty::RawPtr(inner, _) => match inner.kind() {
			ty::Array(elem, _) | ty::Slice(elem) if is_argument(b, *elem) => {
				Type::Array(Box::new(Type::Rt("Fmt", vec![])))
			}
			ty::Coroutine(..) => local_type(b, *inner, span),
			_ => b.t.ty(ty, span),
		},
		_ => b.t.ty(ty, span),
	}
}

/// Whether `ty` is `core::fmt::rt::Argument`.
fn is_argument<'tcx>(b: &Body<'_, '_, 'tcx>, ty: Ty<'tcx>) -> bool {
	match ty.kind() {
		ty::Adt(adt, _) => super::library::std_path(b.t.tcx, adt.did()) == ["fmt", "rt", "Argument"],
		_ => false,
	}
}

fn pascal(name: &str) -> String {
	let mut chars = name.chars();
	chars
		.next()
		.map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
}

/// One MIR body of a coroutine's class, lowered to a method's statements.
struct Lowering<'m, 'b, 'a, 'tcx> {
	b: &'m mut Body<'b, 'a, 'tcx>,
	mir: &'tcx mir::Body<'tcx>,
	class: &'m Class<'tcx>,
	/// The method's parameter for `_2`, the context a resume is given.
	cx: Option<String>,
	/// Locals some statement reads, which are the only ones declared.
	read: FxHashSet<Local>,
	/// Locals holding a byte string, a `format_args!` template.
	bytes: FxHashMap<Local, Vec<u8>>,
	/// How many edges reach each block.
	preds: IndexVec<BasicBlock, usize>,
	span: Span,
}

impl<'m, 'b, 'a, 'tcx> Lowering<'m, 'b, 'a, 'tcx> {
	fn new(
		b: &'m mut Body<'b, 'a, 'tcx>,
		mir: &'tcx mir::Body<'tcx>,
		class: &'m Class<'tcx>,
		cx: Option<String>,
	) -> Self {
		// Cleanup blocks run only as a panic unwinds, which a JS exception never stops
		// for: the target panics by aborting.
		let mut preds = IndexVec::from_elem_n(0, mir.basic_blocks.len());
		for data in mir.basic_blocks.iter().filter(|data| !data.is_cleanup) {
			for target in data.terminator().successors() {
				preds[target] += 1;
			}
		}
		let mut bytes = FxHashMap::default();
		for data in mir.basic_blocks.iter() {
			for stmt in &data.statements {
				let StatementKind::Assign(assign) = &stmt.kind else {
					continue;
				};
				let (place, rvalue) = &**assign;
				let Some(local) = place.as_local() else {
					continue;
				};
				match rvalue {
					Rvalue::Use(Operand::Constant(c), _) => {
						if let Some(found) = const_bytes(b.t.tcx, c) {
							bytes.insert(local, found);
						}
					}
					Rvalue::Ref(_, _, from) if from.projection.as_slice() == [ProjectionElem::Deref] => {
						if let Some(found) = bytes.get(&from.local).cloned() {
							bytes.insert(local, found);
						}
					}
					_ => {}
				}
			}
		}
		let mut reads = Reads(FxHashSet::default());
		reads.visit_body(mir);
		let read = reads.0.into_iter().filter(|local| !bytes.contains_key(local)).collect();
		Self {
			b,
			mir,
			class,
			cx,
			read,
			bytes,
			preds,
			span: mir.span,
		}
	}

	fn tcx(&self) -> ty::TyCtxt<'tcx> {
		self.b.t.tcx
	}

	fn err(&mut self, msg: impl Into<String>) -> Expr {
		let span = self.span;
		self.b.err(span, msg)
	}

	fn local_ty(&self, local: Local) -> Ty<'tcx> {
		self.b.subst(self.mir.local_decls[local].ty)
	}

	/// The method's statements: each local declared, then the dispatch loop.
	fn lower(mut self) -> Vec<Stmt> {
		let mut cases = Vec::new();
		for (block, data) in self.mir.basic_blocks.iter_enumerated() {
			if data.is_cleanup || self.inlined(block) {
				continue;
			}
			let mut body = Vec::new();
			self.block(block, &mut body);
			cases.push((Expr::lit(block.as_usize().to_string()), body));
		}
		let mut out = Vec::new();
		for (local, decl) in self.mir.local_decls.iter_enumerated() {
			let arg = (1..=self.mir.arg_count).contains(&local.as_usize());
			if arg || !self.read.contains(&local) {
				continue;
			}
			let ty = self.b.subst(decl.ty);
			if ty.is_unit() {
				continue;
			}
			let span = self.span;
			// The coroutine itself, which only its own class has fields for.
			let ts = match ty.builtin_deref(true).unwrap_or(ty).kind() {
				ty::Coroutine(def_id, _) if *def_id == self.class.def_id.to_def_id() => {
					Type::Sym(self.class.sym, Vec::new())
				}
				_ => local_type(self.b, ty, span),
			};
			out.push(StmtKind::decl(true, name(local), Some(ts), Some(Expr::lit("undefined!"))).at(None));
		}
		out.push(StmtKind::decl(true, "bb", Some(Type::Builtin("number")), Some(Expr::lit("0"))).at(None));
		out.push(
			StmtKind::While {
				label: None,
				cond: Expr::lit("true"),
				body: vec![
					StmtKind::Switch {
						value: Expr::ident("bb"),
						cases,
						default: Vec::new(),
					}
					.at(None),
				],
			}
			.at(None),
		);
		out
	}

	/// Whether `block` is lowered where its one predecessor jumps to it, not as a case.
	fn inlined(&self, block: BasicBlock) -> bool {
		block != mir::START_BLOCK && self.preds[block] == 1
	}

	/// Control to `target`: inline if only this edge reaches it.
	fn jump(&mut self, target: BasicBlock, out: &mut Vec<Stmt>) {
		if self.inlined(target) {
			self.block(target, out);
			return;
		}
		out.push(StmtKind::Expr(Expr::ident("bb").assign(Expr::lit(target.as_usize().to_string()))).at(None));
		out.push(StmtKind::Continue(None).at(None));
	}

	fn block(&mut self, block: BasicBlock, out: &mut Vec<Stmt>) {
		let data = &self.mir.basic_blocks[block];
		for stmt in &data.statements {
			self.span = stmt.source_info.span;
			self.statement(&stmt.kind, out);
		}
		let terminator = data.terminator();
		self.span = terminator.source_info.span;
		self.terminator(&terminator.kind, out);
	}

	fn statement(&mut self, kind: &StatementKind<'tcx>, out: &mut Vec<Stmt>) {
		match kind {
			StatementKind::Assign(assign) => {
				let (place, rvalue) = &**assign;
				if place
					.as_local()
					.is_some_and(|local| !self.read.contains(&local) || self.bytes.contains_key(&local))
				{
					return;
				}
				if self.b.subst(place.ty(self.mir, self.tcx()).ty).is_unit() {
					return;
				}
				let value = self.rvalue(rvalue, out);
				let target = self.place(place, out);
				out.push(StmtKind::Expr(target.assign(value)).at(None));
			}
			StatementKind::SetDiscriminant { place, variant_index } => {
				let ty = self.b.subst(place.ty(self.mir, self.tcx()).ty);
				match ty.kind() {
					ty::Coroutine(..) => {
						let target = self.place(place, out).field(self.class.state);
						out.push(
							StmtKind::Expr(target.assign(Expr::lit(variant_index.as_usize().to_string()))).at(None),
						);
					}
					_ => {
						self.err(format!("setting the discriminant of `{ty}`"));
					}
				}
			}
			StatementKind::StorageLive(_)
			| StatementKind::StorageDead(_)
			| StatementKind::Nop
			| StatementKind::FakeRead(_)
			| StatementKind::PlaceMention(_)
			| StatementKind::AscribeUserType(..)
			| StatementKind::Coverage(_)
			| StatementKind::ConstEvalCounter
			| StatementKind::BackwardIncompatibleDropHint { .. } => {}
			StatementKind::Intrinsic(_) => {
				self.err("an intrinsic");
			}
		}
	}

	fn terminator(&mut self, kind: &TerminatorKind<'tcx>, out: &mut Vec<Stmt>) {
		match kind {
			TerminatorKind::Goto { target, .. } => self.jump(*target, out),
			TerminatorKind::FalseEdge { real_target, .. } | TerminatorKind::FalseUnwind { real_target, .. } => {
				self.jump(*real_target, out)
			}
			TerminatorKind::SwitchInt { discr, targets } => {
				let ty = self.b.subst(discr.ty(self.mir, self.tcx()));
				let value = self.operand(discr, out);
				let value = match value.is_path() {
					true => value,
					false => self.b.spill(value, "value", out),
				};
				let mut els = Vec::new();
				self.jump(targets.otherwise(), &mut els);
				let arms: Vec<(u128, BasicBlock)> = targets.iter().collect();
				for (bits, target) in arms.into_iter().rev() {
					let test = match ty.kind() {
						ty::Bool if bits == 0 => value.clone().not(),
						ty::Bool => value.clone(),
						_ => match Int::of(ty) {
							Some(int) => value.clone().binary("===", int.lit(bits as i128)),
							None => self.err(format!("a switch on `{ty}`")),
						},
					};
					let mut then = Vec::new();
					self.jump(target, &mut then);
					els = vec![StmtKind::If { cond: test, then, els }.at(None)];
				}
				out.extend(els);
			}
			TerminatorKind::Return => {
				let ret = self.local_ty(mir::RETURN_PLACE);
				match ret.is_unit() {
					true => out.push(StmtKind::Return(None).at(None)),
					false => out.push(StmtKind::Return(Some(Expr::ident(name(mir::RETURN_PLACE)))).at(None)),
				}
			}
			TerminatorKind::Unreachable => {
				let panic = Expr::rt(
					"panic",
					vec![Expr::Str("internal error: entered unreachable code".into())],
				);
				out.push(StmtKind::Expr(panic).at(None));
			}
			TerminatorKind::Drop { place, target, .. } => {
				let ty = self.b.subst(place.ty(self.mir, self.tcx()).ty);
				let value = self.place(place, out);
				let span = self.span;
				if let Some(dropped) = self.b.t.drop_value(value, ty, span) {
					out.push(StmtKind::Expr(dropped).at(None));
				}
				self.jump(*target, out);
			}
			TerminatorKind::Call {
				func,
				args,
				destination,
				target,
				..
			} => {
				let value = self.call(func, args, out);
				let dest_ty = self.b.subst(destination.ty(self.mir, self.tcx()).ty);
				let unread = destination
					.as_local()
					.is_some_and(|local| !self.read.contains(&local) || self.bytes.contains_key(&local));
				match unread || dest_ty.is_unit() || dest_ty.is_never() {
					true => {
						if !value.is_path() {
							out.push(StmtKind::Expr(value).at(None));
						}
					}
					false => {
						let target = self.place(destination, out);
						out.push(StmtKind::Expr(target.assign(value)).at(None));
					}
				}
				if let Some(target) = target {
					self.jump(*target, out);
				}
			}
			TerminatorKind::Assert {
				cond,
				expected,
				msg,
				target,
				..
			} => {
				// A checked operation, an index, or a division panics on its own, with
				// the message its assert has.
				let message = match &**msg {
					AssertKind::ResumedAfterReturn(_) => Some("`async fn` resumed after completion"),
					AssertKind::ResumedAfterPanic(_) => Some("`async fn` resumed after panicking"),
					AssertKind::ResumedAfterDrop(_) => Some("`async fn` resumed after async drop"),
					_ => None,
				};
				if let Some(message) = message {
					let value = self.operand(cond, out);
					let test = match expected {
						true => value.not(),
						false => value,
					};
					out.push(
						StmtKind::If {
							cond: test,
							then: vec![StmtKind::Expr(Expr::rt("panic", vec![Expr::Str(message.into())])).at(None)],
							els: Vec::new(),
						}
						.at(None),
					);
				}
				self.jump(*target, out);
			}
			_ => {
				self.err(format!("unsupported terminator {kind:?}"));
			}
		}
	}

	fn call(&mut self, func: &Operand<'tcx>, args: &[rustc_span::Spanned<Operand<'tcx>>], out: &mut Vec<Stmt>) -> Expr {
		let tcx = self.tcx();
		let fn_ty = self.b.subst(func.ty(self.mir, tcx));
		let ty::FnDef(def_id, gargs) = *fn_ty.kind() else {
			return self.err(format!("a call through `{fn_ty}`"));
		};
		let gargs = gargs.skip_binder();
		let key = super::library::key(tcx, def_id);
		let key = key.as_ref().map(|(owner, name)| (owner.as_str(), name.as_str()));
		let span = self.span;
		// `format_args!`, as MIR has it: its arguments, then its template around them.
		if let Some(("Argument", name)) = key {
			let tr = match name {
				"new_display" => Trait::Display,
				"new_debug" => Trait::Debug,
				"new_lower_hex" => Trait::LowerHex,
				"new_upper_hex" => Trait::UpperHex,
				_ => return self.err(format!("`Argument::{name}`")),
			};
			let Some(ty) = gargs.types().next() else {
				return self.err("an `Argument` without a type");
			};
			let value = self.operand(&args[0].node, out);
			return self.b.fmt_value(tr, ty, value, span, out);
		}
		if let Some(("Arguments", "new")) = key {
			let template = match &args[0].node {
				Operand::Copy(place) | Operand::Move(place) => {
					place.as_local().and_then(|l| self.bytes.get(&l).cloned())
				}
				Operand::Constant(c) => const_bytes(tcx, c),
				_ => None,
			};
			let Some(template) = template else {
				return self.err("a `format_args!` template that is not a constant");
			};
			let arguments = self.operand(&args[1].node, out);
			let arguments = match arguments.is_path() {
				true => arguments,
				false => self.b.spill(arguments, "args", out),
			};
			let pieces = match fmt::decode(&template) {
				Ok(pieces) => pieces,
				Err(msg) => return self.err(format!("{msg} is not supported")),
			};
			let mut parts = Vec::new();
			for piece in pieces {
				parts.push(match piece {
					Piece::Lit(s) => Expr::Str(s),
					Piece::Arg { index, spec } => {
						let value = arguments.clone().index(Expr::lit(index.to_string()));
						match spec.object() {
							Some(spec) => Expr::Rt("fmt").member("spec").call(vec![spec, value]),
							None => value,
						}
					}
				});
			}
			return Expr::Array(parts);
		}
		let mut values = Vec::new();
		for arg in args {
			let value = self.operand(&arg.node, out);
			values.push(value);
		}
		// Lowered left to right: an earlier value a later one's statements could change is saved.
		self.b.call_def(def_id, gargs, Args::Values(values), span, out)
	}

	fn rvalue(&mut self, rvalue: &Rvalue<'tcx>, out: &mut Vec<Stmt>) -> Expr {
		let tcx = self.tcx();
		let span = self.span;
		match rvalue {
			Rvalue::Use(op, _) => self.operand(op, out),
			Rvalue::Ref(..) | Rvalue::Reborrow(..) => {
				let (mutability, place) = match rvalue {
					Rvalue::Ref(_, kind, place) => (kind.mutability(), place),
					Rvalue::Reborrow(_, mutability, place) => (*mutability, place),
					_ => unreachable!(),
				};
				let pointee = self.b.subst(place.ty(self.mir, tcx).ty);
				if mutability.is_mut() && self.b.t.object_kind(pointee).is_none() {
					return self.err(format!(
						"a mutable reference to `{pointee}`, which is no object it can change in place"
					));
				}
				self.place(place, out)
			}
			Rvalue::CopyForDeref(place) => self.place(place, out),
			Rvalue::Cast(kind, op, to) => {
				let to = self.b.subst(*to);
				let from = self.b.subst(op.ty(self.mir, tcx));
				let value = self.operand(op, out);
				match kind {
					CastKind::IntToInt => num::cast(
						self.b,
						num::Operand {
							expr: value,
							value: None,
						},
						from,
						to,
						span,
					),
					// A context's pointer, as `.await` passes it around.
					CastKind::Transmute | CastKind::PtrToPtr if context_like(tcx, from) && context_like(tcx, to) => {
						value
					}
					CastKind::PointerCoercion(..) | CastKind::Subtype => value,
					_ => self.err(format!("a {kind:?} cast from `{from}` to `{to}`")),
				}
			}
			Rvalue::BinaryOp(op, operands) => {
				let (l, r) = &**operands;
				let lty = self.b.subst(l.ty(self.mir, tcx));
				let rty = self.b.subst(r.ty(self.mir, tcx));
				let lv = self.operand(l, out);
				let rv = self.operand(r, out);
				// A checked operation panics itself, so its overflow flag is always false.
				let (op, checked) = match op {
					BinOp::AddWithOverflow => (BinOp::Add, true),
					BinOp::SubWithOverflow => (BinOp::Sub, true),
					BinOp::MulWithOverflow => (BinOp::Mul, true),
					op => (*op, false),
				};
				let value = match (lty.kind(), op) {
					(ty::Int(_) | ty::Uint(_), _) | (ty::Bool, BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor) => {
						num::binary(
							self.b,
							op,
							num::Operand { expr: lv, value: None },
							num::Operand { expr: rv, value: None },
							lty,
							rty,
							span,
						)
					}
					(_, BinOp::Eq) => lv.binary("===", rv),
					(_, BinOp::Ne) => lv.binary("!==", rv),
					_ => self.err(format!("`{op:?}` on `{lty}`")),
				};
				match checked {
					true => Expr::Array(vec![value, Expr::lit("false")]),
					false => value,
				}
			}
			Rvalue::UnaryOp(op, operand) => {
				let ty = self.b.subst(operand.ty(self.mir, tcx));
				let value = self.operand(operand, out);
				match op {
					UnOp::Not | UnOp::Neg => num::unary(self.b, *op, value, ty, span),
					UnOp::PtrMetadata => self.err("pointer metadata"),
				}
			}
			Rvalue::Discriminant(place) => {
				let ty = self.b.subst(place.ty(self.mir, tcx).ty);
				let value = self.place(place, out);
				self.discriminant(value, ty)
			}
			Rvalue::Aggregate(kind, operands) => self.aggregate(kind, operands, out),
			_ => self.err(format!("unsupported rvalue {rvalue:?}")),
		}
	}

	/// The variant index of `value`, a `ty`.
	fn discriminant(&mut self, value: Expr, ty: Ty<'tcx>) -> Expr {
		let tcx = self.tcx();
		match ty.kind() {
			ty::Coroutine(..) => value.field(self.class.state),
			ty::Adt(adt, _) => match Std::of(tcx, adt.did()) {
				Some(Std::Option) => Expr::Cond(
					Box::new(value.binary("===", Expr::undefined())),
					Box::new(Expr::lit("0")),
					Box::new(Expr::lit("1")),
				),
				Some(Std::Poll) => Expr::Cond(
					Box::new(value.binary("===", Expr::Rt("Pending"))),
					Box::new(Expr::lit("1")),
					Box::new(Expr::lit("0")),
				),
				Some(Std::Result) => Expr::Cond(
					Box::new(value.member("ok")),
					Box::new(Expr::lit("0")),
					Box::new(Expr::lit("1")),
				),
				_ => self.err(format!("the discriminant of `{ty}`")),
			},
			_ => self.err(format!("the discriminant of `{ty}`")),
		}
	}

	fn aggregate(
		&mut self,
		kind: &AggregateKind<'tcx>,
		operands: &IndexVec<FieldIdx, Operand<'tcx>>,
		out: &mut Vec<Stmt>,
	) -> Expr {
		let tcx = self.tcx();
		let span = self.span;
		match kind {
			AggregateKind::Tuple if operands.is_empty() => Expr::undefined(),
			AggregateKind::Tuple | AggregateKind::Array(_) => {
				let values = operands.iter().map(|op| self.operand(op, out)).collect();
				Expr::Array(values)
			}
			AggregateKind::Adt(did, variant, gargs, _, None) => {
				let gargs = self.b.subst(*gargs);
				let mut values: Vec<Expr> = operands.iter().map(|op| self.operand(op, out)).collect();
				let adt = tcx.adt_def(*did);
				if context_like(tcx, tcx.type_of(*did).instantiate(tcx, gargs).skip_norm_wip()) || is_resume(tcx, *did)
				{
					return values.pop().unwrap_or_else(Expr::undefined);
				}
				match (Std::of(tcx, *did), variant.as_usize()) {
					(Some(Std::Option), 0) => return Expr::undefined(),
					(Some(Std::Option), _) | (Some(Std::Poll), 0) => {
						return values.pop().unwrap_or_else(Expr::undefined);
					}
					(Some(Std::Poll), _) => return Expr::Rt("Pending"),
					(Some(Std::Result), 0) => return Expr::rt("Ok", values),
					(Some(Std::Result), _) => return Expr::rt("Err", values),
					(Some(_), _) => return self.err(format!("constructing `{}`", tcx.def_path_str(*did))),
					(None, _) => {}
				}
				let info = self.b.t.adt(adt, span);
				let ty = tcx.type_of(*did).instantiate(tcx, gargs).skip_norm_wip();
				let def = adt.variant(*variant);
				match info.repr {
					AdtRepr::Class { ctor, fields } => match ctor {
						Ctor::Unit => Expr::New(Box::new(Expr::Sym(info.sym)), Vec::new()),
						Ctor::Positional => Expr::New(
							Box::new(Expr::Sym(info.sym)),
							values.into_iter().map(Expr::typed).collect(),
						),
						Ctor::Named => {
							let props = fields
								.iter()
								.zip(values)
								.map(|(&id, value)| (Key::Field(id), value.typed()))
								.collect();
							Expr::New(Box::new(Expr::Sym(info.sym)), vec![Expr::Object(props)])
						}
					},
					AdtRepr::Tags => {
						let ts = self.b.t.ty(ty, span);
						Expr::As(Box::new(Expr::Str(def.name.to_string())), ts)
					}
					AdtRepr::Union => {
						let mut props = vec![(Key::Name("tag".into()), Expr::Str(def.name.to_string()))];
						for (i, value) in values.into_iter().enumerate() {
							let field = &def.fields[FieldIdx::from_usize(i)];
							props.push((Key::Name(super::field_name(field.name.as_str(), i)), value));
						}
						let ts = self.b.t.ty(ty, span);
						Expr::As(Box::new(Expr::Object(props)), ts)
					}
				}
			}
			AggregateKind::Closure(def_id, _) => {
				let Some(local) = def_id.as_local() else {
					return self.err("a closure from another crate");
				};
				let captures = tcx.closure_captures(local);
				for (capture, op) in captures.iter().zip(operands.iter()) {
					// A reference captured through a deref is still the object it points to.
					let whole = capture
						.place
						.projections
						.iter()
						.all(|p| matches!(p.kind, rustc_middle::hir::place::ProjectionKind::Deref));
					if !whole {
						return self.err("a closure that captures part of a variable");
					}
					let value = self.operand(op, out);
					let name = self.b.fresh(&camel(capture.to_symbol().as_str()));
					out.push(StmtKind::decl(false, name.clone(), None, Some(value)).at(None));
					self.b.bind_upvar(capture.get_root_variable(), name);
				}
				self.b.lower_closure(local, span)
			}
			_ => self.err(format!("unsupported aggregate {kind:?}")),
		}
	}

	fn operand(&mut self, op: &Operand<'tcx>, out: &mut Vec<Stmt>) -> Expr {
		match op {
			Operand::Copy(place) => {
				let ty = self.b.subst(place.ty(self.mir, self.tcx()).ty);
				let value = self.place(place, out);
				match context_like(self.tcx(), ty) {
					true => value,
					false => self.b.copied(value, ty),
				}
			}
			Operand::Move(place) => self.place(place, out),
			Operand::Constant(c) => self.constant(c),
			Operand::RuntimeChecks(_) => self.err("a runtime check"),
		}
	}

	fn constant(&mut self, c: &mir::ConstOperand<'tcx>) -> Expr {
		let tcx = self.tcx();
		let ty = self.b.subst(c.const_.ty());
		if ty.is_unit() || matches!(ty.kind(), ty::FnDef(..)) {
			return Expr::undefined();
		}
		let env = self.b.t.typing_env_pub();
		match ty.kind() {
			ty::Bool => match c.const_.try_eval_scalar_int(tcx, env) {
				Some(int) => Expr::lit(if int.is_null() { "false" } else { "true" }),
				None => self.err("a constant that does not evaluate"),
			},
			ty::Int(_) | ty::Uint(_) => {
				let int = Int::of(ty).expect("an integer");
				match c.const_.try_eval_bits(tcx, env) {
					Some(bits) => {
						let size = rustc_abi::Size::from_bits(int.bits);
						let value = match int.signed {
							true => size.sign_extend(bits),
							false => bits as i128,
						};
						int.lit(value)
					}
					None => self.err("a constant that does not evaluate"),
				}
			}
			ty::Ref(_, inner, _) if inner.is_str() => match c.const_.eval(tcx, env, self.span) {
				Ok(value) => match value.try_get_slice_bytes_for_diagnostics(tcx) {
					Some(bytes) => Expr::Str(String::from_utf8_lossy(bytes).into_owned()),
					None => self.err("a string constant"),
				},
				Err(_) => self.err("a constant that does not evaluate"),
			},
			_ => self.err(format!("a constant of type `{ty}`")),
		}
	}

	fn place(&mut self, place: &Place<'tcx>, out: &mut Vec<Stmt>) -> Expr {
		let tcx = self.tcx();
		let mut value = self.local(place.local);
		for (i, elem) in place.projection.iter().enumerate() {
			let base = Place::ty_from(place.local, &place.projection[..i], self.mir, tcx);
			let ty = self.b.subst(base.ty);
			value = match elem {
				ProjectionElem::Deref
				| ProjectionElem::Downcast(..)
				| ProjectionElem::OpaqueCast(_)
				| ProjectionElem::UnwrapUnsafeBinder(_) => value,
				ProjectionElem::Field(field, field_ty) => {
					let value = self.field(value, ty, base.variant_index, field);
					// TypeScript can't see which variant a switch on its discriminant took.
					match ty.kind() {
						ty::Adt(adt, _) if matches!(Std::of(tcx, adt.did()), Some(Std::Option | Std::Poll)) => {
							let field_ty = self.b.subst(field_ty);
							let span = self.span;
							Expr::As(Box::new(value), local_type(self.b, field_ty, span))
						}
						_ => value,
					}
				}
				ProjectionElem::Index(index) => {
					let base = match value.is_path() {
						true => value,
						false => self.b.spill(value, "slice", out),
					};
					let i = self.local(index);
					base.clone().index(Expr::rt("idx", vec![base, i]))
				}
				_ => self.err(format!("unsupported projection {elem:?}")),
			};
		}
		value
	}

	fn field(&mut self, base: Expr, ty: Ty<'tcx>, variant: Option<VariantIdx>, field: FieldIdx) -> Expr {
		let tcx = self.tcx();
		let span = self.span;
		match ty.kind() {
			ty::Coroutine(..) => match variant {
				None => match self.class.upvars.get(field.as_usize()) {
					Some(&id) => base.field(id),
					None => self.err("an upvar out of range"),
				},
				Some(variant) => {
					let saved = self.class.layout.variant_fields[variant][field];
					base.field(self.class.saved[saved])
				}
			},
			ty::Adt(adt, _) if context_like(tcx, ty) || Std::of(tcx, adt.did()) == Some(Std::Pin) => base,
			_ => self
				.b
				.project(base, ty, variant.unwrap_or(rustc_abi::FIRST_VARIANT), field, span),
		}
	}

	fn local(&mut self, local: Local) -> Expr {
		match local.as_usize() {
			1 => Expr::This,
			2 if let Some(cx) = &self.cx => Expr::ident(cx),
			_ => Expr::ident(name(local)),
		}
	}
}

/// A local's name: `_3`.
fn name(local: Local) -> String {
	format!("_{}", local.as_usize())
}

/// Whether `ty` is how `.await` carries the context: the context, a pointer to it, or
/// `ResumeTy`.
fn context_like<'tcx>(tcx: ty::TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
	match ty.kind() {
		ty::Ref(_, inner, _) => context_like(tcx, *inner),
		ty::Adt(adt, args) => {
			Std::of(tcx, adt.did()) == Some(Std::Context)
				|| (super::library::std_path(tcx, adt.did())
					.last()
					.is_some_and(|n| n == "NonNull")
					&& context_like(tcx, args.type_at(0)))
		}
		_ => false,
	}
}

/// Whether `did` is `ResumeTy`, the context an `async` body is resumed with.
fn is_resume(tcx: ty::TyCtxt<'_>, did: rustc_hir::def_id::DefId) -> bool {
	tcx.is_lang_item(did, rustc_attr_ir::LangItem::ResumeTy)
}

/// The bytes of a constant `&[u8; N]`.
fn const_bytes<'tcx>(tcx: ty::TyCtxt<'tcx>, c: &mir::ConstOperand<'tcx>) -> Option<Vec<u8>> {
	let ty::Ref(_, inner, _) = c.const_.ty().kind() else {
		return None;
	};
	let ty::Array(elem, len) = inner.kind() else {
		return None;
	};
	if !matches!(elem.kind(), ty::Uint(ty::UintTy::U8)) {
		return None;
	}
	let len = len.try_to_target_usize(tcx)? as usize;
	let value = c
		.const_
		.eval(tcx, ty::TypingEnv::fully_monomorphized(), rustc_span::DUMMY_SP)
		.ok()?;
	let ConstValue::Scalar(Scalar::Ptr(ptr, _)) = value else {
		return None;
	};
	let (prov, offset) = ptr.prov_and_relative_offset();
	let alloc = tcx.global_alloc(prov.alloc_id()).unwrap_memory();
	let start = offset.bytes_usize();
	Some(
		alloc
			.inner()
			.inspect_with_uninit_and_ptr_outside_interpreter(start..start + len)
			.to_vec(),
	)
}

/// The locals a body reads.
struct Reads(FxHashSet<Local>);

impl<'tcx> Visitor<'tcx> for Reads {
	fn visit_local(&mut self, local: Local, context: PlaceContext, _: mir::Location) {
		use rustc_middle::mir::visit::{MutatingUseContext, NonUseContext};
		match context {
			PlaceContext::NonUse(
				NonUseContext::StorageLive | NonUseContext::StorageDead | NonUseContext::VarDebugInfo,
			)
			| PlaceContext::MutatingUse(MutatingUseContext::Store | MutatingUseContext::Call) => {}
			_ => {
				self.0.insert(local);
			}
		}
	}
}
