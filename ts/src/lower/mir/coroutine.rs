//! Async bodies, after rustc's state transform: each coroutine instance is a class of
//! its state, its captures, and the locals it keeps across an `.await`. Its `poll` is
//! the resume function the transform made, and its `[Symbol.dispose]` the drop the
//! transform made, which drops what the state it is in holds.

use rustc_abi::{FieldIdx, VariantIdx};
use rustc_hir::def_id::DefId;
use rustc_index::IndexVec;
use rustc_middle::mir::{CoroutineSavedLocal, Operand};
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
	let state = t.program.field(sym, "state".into());
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
		let name = names.get(i).cloned().unwrap_or_else(|| format!("u{i}"));
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
	let mut saved = IndexVec::new();
	for field in layout.field_tys.iter() {
		// rustc's own locals, such as `__awaitee`, start with `_`.
		let name = match field
			.debuginfo_name
			.map(|n| n.as_str().trim_start_matches('_').to_string())
		{
			Some(name) if !name.is_empty() => crate::lower::camel(&name),
			_ => "saved".into(),
		};
		let id = t.program.field(sym, name);
		saved.push(id);
		let ty = t.subst(args, field.ty);
		// A local held across an `.await`, refused as any variable is.
		let ty = t.held_ty(ty, field.source_info.span);
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
	if let Some(mir) = super::instance_mir(t, instance, span) {
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
