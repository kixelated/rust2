//! Drop glue rustc builds, for a value whose JS form carries no `[Symbol.dispose]` of its
//! own: a `Result`, an enum, or a tuple holding something that drops. The glue switches on
//! the variant and drops each field, as rustc's `drop_in_place` shim does.

use rustc_middle::ty::{self, EarlyBinder, Instance, Ty, TypeVisitableExt};
use rustc_span::Span;

use super::{Env, Fx};
use crate::lower::Translator;
use crate::lower::body::Body;
use crate::program::{SymKind, Symbol};
use crate::ts::{self, Expr, Param, SymId, Type};

/// The function dropping a `ty`, made from its drop shim the first time.
pub fn shim<'tcx>(t: &mut Translator<'_, 'tcx>, ty: Ty<'tcx>, span: Span) -> Option<SymId> {
	if let Some(&sym) = t.drop_shims.get(&ty) {
		return Some(sym);
	}
	// Only an instance can say what a parameter's glue is.
	if ty.has_param() || ty.has_aliases() {
		return None;
	}
	let structural = match ty.kind() {
		ty::Tuple(_) | ty::Closure(..) => true,
		ty::Adt(adt, _) => adt.is_enum(),
		_ => false,
	};
	if !structural {
		return None;
	}
	let tcx = t.tcx;
	let instance = Instance::resolve_drop_glue(tcx, ty);
	let sym = t.program.sym(Symbol {
		module: Vec::new(),
		base: "drop".into(),
		suffix: crate::lower::short_name(tcx, ty),
		owner: None,
		kind: SymKind::Function,
		order: 0,
		instance: ty::print::with_no_trimmed_paths!(ty.to_string()),
		name: String::new(),
	});
	t.drop_shims.insert(ty, sym);
	let body = tcx.instance_mir(instance.def).clone();
	let mir = instance.instantiate_mir_and_normalize_erasing_regions(tcx, t.typing_env_pub(), EarlyBinder::bind(tcx, body));
	let mut b = Body::detached(t, instance.args, tcx.types.unit);
	let mut fx = Fx::new(&mut b, &mir, Env::Shim, span);
	let params = fx.params();
	let body = fx.lower();
	// The shim takes a pointer, which is the value in JS.
	let params = params
		.into_iter()
		.map(|p| Param {
			ty: Some(b.t.ty(ty, span)),
			..p
		})
		.collect();
	t.program.items.push(ts::Item::Function(ts::Function {
		sym,
		method: false,
		params,
		ret: Type::Builtin("void"),
		body,
		loc: None,
		doc: None,
		generator: false,
	}));
	Some(sym)
}

/// `value` dropped by its glue.
pub fn call(sym: SymId, value: Expr) -> Expr {
	Expr::Sym(sym).call(vec![value])
}
