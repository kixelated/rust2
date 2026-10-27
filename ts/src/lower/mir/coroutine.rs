//! Async bodies, after rustc's state transform: each coroutine instance is a class of
//! its state, its captures, and the locals it keeps across an `.await`. Its `poll` is
//! the resume function the transform made, and its `[Symbol.dispose]` the drop the
//! transform made, which drops what the state it is in holds.

use rustc_abi::{FieldIdx, VariantIdx};
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def_id::DefId;
use rustc_index::IndexVec;
use rustc_middle::mir::{self, CoroutineSavedLocal, Operand};
use rustc_middle::ty::{self, EarlyBinder, GenericArgsRef, Instance};
use rustc_span::Span;

use super::{Env, Fx};
use crate::lower::Translator;
use crate::lower::body::Body;
use crate::program::{SymKind, Symbol};
use crate::ts::{self, Expr, FieldId, FieldInit, Item, Param, Stmt, SymId, Type};

/// A coroutine instance's class.
#[derive(Clone)]
pub struct Info<'tcx> {
	pub sym: SymId,
	/// Which state it is in: rustc's discriminant.
	pub state: FieldId,
	pub upvars: Vec<FieldId>,
	pub saved: IndexVec<CoroutineSavedLocal, FieldId>,
	variant_fields: IndexVec<VariantIdx, IndexVec<FieldIdx, CoroutineSavedLocal>>,
	_tcx: std::marker::PhantomData<ty::Ty<'tcx>>,
}

impl Info<'_> {
	/// The saved local a variant's field is.
	pub fn layout_variant_field(&self, variant: VariantIdx, field: FieldIdx) -> CoroutineSavedLocal {
		self.variant_fields[variant][field]
	}
}

/// The class of the coroutine `def_id` with `args`, made the first time.
pub fn info<'tcx>(t: &mut Translator<'_, 'tcx>, def_id: DefId, args: GenericArgsRef<'tcx>, span: Span) -> Info<'tcx> {
	let instance = Instance::new_raw(def_id, args);
	if let Some(info) = t.coroutines.get(&instance) {
		return info.clone();
	}
	let tcx = t.tcx;
	let parent = tcx.typeck_root_def_id(def_id);
	let base = format!("{}Future", crate::lower::camel(tcx.item_name(parent).as_str()));
	// The suffix of the instance of `parent` it is part of.
	let suffix = t.suffix(Instance::new_raw(
		parent,
		args.truncate_to(tcx, tcx.generics_of(parent)),
	));
	let sym = t.program.sym(Symbol {
		module: t.module_of(parent),
		base,
		suffix,
		owner: None,
		kind: SymKind::Struct,
		order: t.order(tcx.def_span(def_id)),
		instance: String::new(),
		name: String::new(),
		alias: None,
	});
	let mut fields = Vec::new();
	// Private, as only the class's own methods read its fields, which a minifier can then
	// rename.
	let state = t.program.field(sym, "#state".into());
	fields.push(ts::Field {
		id: state,
		ty: Type::Builtin("number"),
		doc: None,
		init: FieldInit::Value(Expr::lit("0")),
	});
	let coroutine = args.as_coroutine();
	let names: Vec<String> = match def_id.as_local() {
		Some(local) => tcx
			.closure_captures(local)
			.iter()
			.map(|c| crate::lower::camel(c.to_symbol().as_str()))
			.collect(),
		None => Vec::new(),
	};
	let mut upvars = Vec::new();
	for (i, ty) in coroutine.upvar_tys().iter().enumerate() {
		let name = format!("#{}", names.get(i).cloned().unwrap_or_else(|| format!("u{i}")));
		let id = t.program.field(sym, name);
		upvars.push(id);
		let ty = t.ty(ty, span);
		fields.push(ts::Field {
			id,
			ty,
			doc: None,
			init: FieldInit::Ctor,
		});
	}
	let layout = match tcx.coroutine_layout(def_id, args) {
		Ok(layout) => layout,
		Err(_) => {
			t.err(span, "a coroutine without a layout");
			return Info {
				sym,
				state,
				upvars,
				saved: IndexVec::new(),
				variant_fields: IndexVec::new(),
				_tcx: std::marker::PhantomData,
			};
		}
	};
	let resume = super::instance_mir(t, instance, span);
	let moved = resume
		.as_ref()
		.map(|mir| moved_upvars(mir, def_id, layout))
		.unwrap_or_default();
	let mut saved = IndexVec::new();
	// Locals of one type that are never live at once share a field, as rustc overlaps
	// them in memory: each `.await` of a future of one type keeps it in the same one.
	let mut shared: Vec<(FieldId, Type, Vec<CoroutineSavedLocal>)> = Vec::new();
	for (local, field) in layout.field_tys.iter_enumerated() {
		// An argument the body moves into its own variable keeps its field.
		if let Some(&upvar) = moved.get(&local) {
			saved.push(upvars[upvar]);
			continue;
		}
		let ty = t.subst(args, field.ty);
		// A local held across an `.await`, refused as any variable is.
		let ty = t.held_ty(ty, field.source_info.span);
		let free = shared
			.iter_mut()
			.find(|(_, of, locals)| *of == ty && locals.iter().all(|&l| !layout.storage_conflicts.contains(l, local)));
		if let Some((id, _, locals)) = free {
			locals.push(local);
			saved.push(*id);
			continue;
		}
		// rustc's own locals, such as `__awaitee`, start with `_`.
		let name = match field
			.debuginfo_name
			.map(|n| n.as_str().trim_start_matches('_').to_string())
		{
			Some(name) if !name.is_empty() => format!("#{}", crate::lower::camel(&name)),
			_ => "#saved".into(),
		};
		let id = t.program.field(sym, name);
		saved.push(id);
		shared.push((id, ty.clone(), vec![local]));
		fields.push(ts::Field {
			id,
			ty,
			doc: None,
			init: FieldInit::Later,
		});
	}
	let info = Info {
		sym,
		state,
		upvars,
		saved,
		variant_fields: layout.variant_fields.clone(),
		_tcx: std::marker::PhantomData,
	};
	t.coroutines.insert(instance, info.clone());
	t.program.items.push(Item::Struct {
		sym,
		params: Vec::new(),
		fields,
		ctor: match info.upvars.is_empty() {
			true => ts::Ctor::Unit,
			false => ts::Ctor::Positional,
		},
		doc: None,
	});
	// `poll`: the resume function.
	let poll = t.program.sym(Symbol {
		module: t.module_of(parent),
		base: "poll".into(),
		suffix: String::new(),
		owner: Some(sym),
		kind: SymKind::Method,
		order: 0,
		instance: String::new(),
		name: String::new(),
		alias: None,
	});
	if let Some(mir) = resume {
		let ret = mir.return_ty();
		let mut b = Body::new(t, args);
		let mut fx = Fx::new(&mut b, &mir, Env::Coroutine, span);
		let params = fx.params();
		let body = fx.lower();
		let ret = b.t.ret_ty(ret, span);
		let loc = t.loc(span);
		t.program.methods.entry(sym).or_default().push(ts::Function {
			sym: poll,
			params,
			ret,
			body,
			loc,
			doc: None,
		});
	}
	// `[Symbol.dispose]`: the drop, by the state it is in.
	let dispose = t.program.sym(Symbol {
		module: t.module_of(parent),
		base: "[Symbol.dispose]".into(),
		suffix: String::new(),
		owner: Some(sym),
		kind: SymKind::Method,
		order: 0,
		instance: String::new(),
		name: String::new(),
		alias: None,
	});
	let drop = tcx.optimized_mir(def_id).coroutine_drop().cloned();
	let body: Vec<Stmt> = match drop {
		Some(drop) => {
			let mir = instance.instantiate_mir_and_normalize_erasing_regions(
				tcx,
				t.typing_env_pub(),
				EarlyBinder::bind(tcx, drop),
			);
			let mut b = Body::new(t, args);
			let mut fx = Fx::new(&mut b, &mir, Env::Coroutine, span);
			fx.lower()
		}
		None => Vec::new(),
	};
	t.program.methods.entry(sym).or_default().push(ts::Function {
		sym: dispose,
		params: Vec::<Param>::new(),
		ret: Type::Builtin("void"),
		body,
		loc: None,
		doc: None,
	});
	info
}

/// The class of the future the `async fn` `instance` returns, if its body only moves its
/// arguments, in order, into it: a call can then make the future itself.
pub fn of_async_fn<'tcx>(t: &mut Translator<'_, 'tcx>, instance: Instance<'tcx>, span: Span) -> Option<SymId> {
	use rustc_middle::mir::{AggregateKind, Rvalue, StatementKind, TerminatorKind};
	if !t.tcx.asyncness(instance.def_id()).is_async() {
		return None;
	}
	let mir = super::instance_mir(t, instance, span)?;
	if mir.basic_blocks.len() != 1 {
		return None;
	}
	let block = &mir.basic_blocks[rustc_middle::mir::START_BLOCK];
	if !matches!(block.terminator().kind, TerminatorKind::Return) {
		return None;
	}
	let mut made = None;
	for stmt in &block.statements {
		match &stmt.kind {
			StatementKind::StorageLive(_) | StatementKind::StorageDead(_) | StatementKind::Nop => {}
			StatementKind::Assign(assign) if made.is_none() => made = Some(&**assign),
			_ => return None,
		}
	}
	let (place, Rvalue::Aggregate(kind, ops)) = made? else {
		return None;
	};
	let AggregateKind::Coroutine(def_id, args) = **kind else {
		return None;
	};
	let moved_args = ops.len() == mir.arg_count
		&& ops.iter().zip(mir.args_iter()).all(|(op, arg)| match op {
			Operand::Move(p) | Operand::Copy(p) => p.as_local() == Some(arg),
			Operand::Constant(_) | Operand::RuntimeChecks(_) => false,
		});
	if place.as_local() != Some(rustc_middle::mir::RETURN_PLACE) || !moved_args {
		return None;
	}
	Some(info(t, def_id, args, span).sym)
}

/// A coroutine made in place: `new Class(...captures)`.
pub fn construct<'tcx>(
	fx: &mut Fx<'_, '_, '_, 'tcx>,
	def_id: DefId,
	args: GenericArgsRef<'tcx>,
	ops: &IndexVec<FieldIdx, Operand<'tcx>>,
	span: Span,
	out: &mut Vec<Stmt>,
) -> Expr {
	let info = info(fx.b.t, def_id, args, span);
	let values = ops.iter().map(|op| fx.operand(op, span, out)).collect();
	Expr::New(Box::new(Expr::Sym(info.sym)), values)
}

/// The saved locals the resume function `mir` of the coroutine `def_id` moves or copies
/// an upvar into, with the upvar's index, where that is the upvar's one read: nothing
/// reads the upvar's field after, so the local can keep its value there.
fn moved_upvars<'tcx>(
	mir: &mir::Body<'tcx>,
	def_id: DefId,
	layout: &mir::CoroutineLayout<'tcx>,
) -> FxHashMap<CoroutineSavedLocal, usize> {
	use mir::visit::{PlaceContext, Visitor};
	use mir::{ProjectionElem, Rvalue, StatementKind};

	struct Reads<'m, 'tcx> {
		mir: &'m mir::Body<'tcx>,
		def_id: DefId,
		reads: FxHashMap<FieldIdx, u32>,
	}
	impl<'tcx> Reads<'_, 'tcx> {
		/// The upvar `place` is, or the field of a variant, of this coroutine.
		fn split(&self, place: mir::Place<'tcx>) -> Option<(Option<VariantIdx>, FieldIdx)> {
			let ty = self.mir.local_decls[place.local].ty.builtin_deref(true)?;
			if !matches!(ty.kind(), ty::Coroutine(d, _) if *d == self.def_id) {
				return None;
			}
			match place.projection.as_slice() {
				[ProjectionElem::Deref, ProjectionElem::Field(f, _), ..] => Some((None, *f)),
				[
					ProjectionElem::Deref,
					ProjectionElem::Downcast(_, v),
					ProjectionElem::Field(f, _),
					..,
				] => Some((Some(*v), *f)),
				_ => None,
			}
		}
	}
	impl<'tcx> Visitor<'tcx> for Reads<'_, 'tcx> {
		fn visit_place(&mut self, place: &mir::Place<'tcx>, cx: PlaceContext, at: mir::Location) {
			// Debug info names a place without reading it.
			if !matches!(cx, PlaceContext::NonUse(_))
				&& let Some((None, f)) = self.split(*place)
			{
				*self.reads.entry(f).or_default() += 1;
			}
			self.super_place(place, cx, at);
		}
	}

	let mut reads = Reads {
		mir,
		def_id,
		reads: FxHashMap::default(),
	};
	reads.visit_body(mir);
	let mut moved = FxHashMap::default();
	for data in mir.basic_blocks.iter() {
		for stmt in &data.statements {
			let StatementKind::Assign(assign) = &stmt.kind else {
				continue;
			};
			let (dest, Rvalue::Use(Operand::Move(src) | Operand::Copy(src), _)) = &**assign else {
				continue;
			};
			if src.projection.len() == 2
				&& dest.projection.len() == 3
				&& let Some((None, upvar)) = reads.split(*src)
				&& let Some((Some(variant), field)) = reads.split(*dest)
				&& reads.reads.get(&upvar) == Some(&1)
			{
				moved.insert(layout.variant_fields[variant][field], upvar.index());
			}
		}
	}
	moved
}
