//! Formatting: `format_args!`, the formatting traits, and std's impls of them.
//!
//! The runtime ports `core::fmt`, so translated output matches native Rust's byte for
//! byte. A placeholder's value becomes `(f) => ...`, a function writing it to a
//! formatter, and `fmt::Arguments` an array of literal pieces and such functions. Each
//! call goes to the impl of the value's type directly, as monomorphization allows.

use rustc_hir::def_id::DefId;
use rustc_middle::thir::{ExprId, ExprKind, StmtKind};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Span;

use super::body::Body;
use super::library::{self, Args, Std};
use super::num::Int;
use crate::ts::{Expr, Key, Param, Stmt, Type};

/// A formatting trait.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trait {
	Display,
	Debug,
	LowerHex,
	UpperHex,
}

impl Trait {
	/// The trait `did` is, if it is one of these.
	pub fn of(tcx: TyCtxt<'_>, did: DefId) -> Option<Self> {
		match library::std_path(tcx, did)
			.iter()
			.map(String::as_str)
			.collect::<Vec<_>>()
			.as_slice()
		{
			["fmt", "Display"] => Some(Self::Display),
			["fmt", "Debug"] => Some(Self::Debug),
			["fmt", "LowerHex"] => Some(Self::LowerHex),
			["fmt", "UpperHex"] => Some(Self::UpperHex),
			_ => None,
		}
	}

	fn name(self) -> &'static str {
		match self {
			Self::Display => "Display",
			Self::Debug => "Debug",
			Self::LowerHex => "LowerHex",
			Self::UpperHex => "UpperHex",
		}
	}

	/// The trait's `fmt` method.
	fn method(self, tcx: TyCtxt<'_>) -> Option<DefId> {
		let module = tcx.parent(tcx.get_diagnostic_item(rustc_span::sym::Debug)?);
		let child = tcx
			.module_children(module)
			.iter()
			.find(|child| child.ident.as_str() == self.name())?;
		let trait_id = child.res.opt_def_id()?;
		let method = tcx
			.associated_items(trait_id)
			.in_definition_order()
			.find(|item| item.is_fn())?;
		Some(method.def_id)
	}
}

const UNEXPECTED: &str = "an unexpected `format_args!` lowering";
pub const FROM_ARGUMENT: &str = "a width or precision from an argument, as in `{:1$}`,";

/// A formatter's type, for the parameters of the functions written here.
fn formatter() -> Type {
	Type::Rt("Formatter", Vec::new())
}

impl<'b, 'a, 'tcx> Body<'b, 'a, 'tcx> {
	/// Whether `id` is what `format_args!` lowers to: `Arguments::from_str(s)`, or a block
	/// of two `super let`s ending in `unsafe { Arguments::new(template, &args) }`.
	pub fn is_format_args(&self, id: ExprId) -> bool {
		let calls = |id: ExprId, names: &[&str]| match &self.thir[self.strip(id)].kind {
			ExprKind::Call { ty, .. } => self
				.callee(*ty)
				.is_some_and(|(owner, name)| owner == "Arguments" && names.contains(&name.as_str())),
			_ => false,
		};
		let tail = |id: ExprId| match &self.thir[self.strip(id)].kind {
			ExprKind::Block { block } => self.thir[*block].expr,
			_ => None,
		};
		let block = match &self.thir[self.strip(id)].kind {
			ExprKind::Block { block } => &self.thir[*block],
			_ => return calls(id, &["from_str", "from_str_nonconst"]),
		};
		block.stmts.len() == 2 && block.expr.and_then(tail).is_some_and(|call| calls(call, &["new"]))
	}

	/// The formatting trait of a `&dyn Debug` or the like.
	pub fn fmt_object(&self, ty: Ty<'tcx>) -> Option<Trait> {
		let ty::Ref(_, inner, rustc_ast::Mutability::Not) = ty.kind() else {
			return None;
		};
		let ty::Dynamic(preds, ..) = inner.kind() else {
			return None;
		};
		Trait::of(self.t.tcx, preds.principal_def_id()?)
	}

	/// Whether `ty` points to a trait object.
	pub fn trait_object(&self, ty: Ty<'tcx>) -> bool {
		ty.builtin_deref(true)
			.is_some_and(|pointee| matches!(pointee.kind(), ty::Dynamic(..)))
	}

	/// The `owner::name` key of a call's callee in std.
	fn callee(&self, fn_ty: Ty<'tcx>) -> Option<(String, String)> {
		match *self.subst(fn_ty).kind() {
			ty::FnDef(def_id, _) => library::key(self.t.tcx, def_id),
			_ => None,
		}
	}

	/// A `fmt::Arguments`, from what `format_args!` lowers to: an array of literal pieces
	/// and functions writing each placeholder's value.
	pub fn format_args(&mut self, id: ExprId, out: &mut Vec<Stmt>) -> Expr {
		let span = self.span(id);
		match self.arguments(id, out) {
			Ok(parts) => Expr::Array(parts),
			Err(msg) if msg == UNEXPECTED => self.err(span, msg),
			Err(msg) => self.err(span, format!("{msg} is not supported")),
		}
	}

	/// The parts of a `format_args!`, or the construct that is outside the subset.
	fn arguments(&mut self, id: ExprId, out: &mut Vec<Stmt>) -> Result<Vec<Expr>, String> {
		let id = self.strip(id);
		let thir = self.thir;
		let block = match &thir[id].kind {
			// A template without placeholders.
			ExprKind::Call { args, .. } if args.len() == 1 => return Ok(vec![self.operand(args[0], out)]),
			ExprKind::Block { block } => &thir[*block],
			_ => return Err(UNEXPECTED.into()),
		};
		// `super let args = (&a, &b); super let args = [Argument::new_display(args.0), ..];`
		let [values, arguments] = block.stmts[..] else {
			return Err(UNEXPECTED.into());
		};
		let binding = |stmt: rustc_middle::thir::StmtId| match &thir[stmt].kind {
			StmtKind::Let {
				pattern,
				initializer: Some(init),
				..
			} => match pattern.kind {
				rustc_middle::thir::PatKind::Binding { var, .. } => Some((var, *init)),
				_ => None,
			},
			_ => None,
		};
		let ((values_var, values), (_, arguments)) = binding(values).zip(binding(arguments)).ok_or(UNEXPECTED)?;
		let ExprKind::Tuple { fields } = &thir[self.strip(values)].kind else {
			return Err(UNEXPECTED.into());
		};
		// Each value is evaluated once, in order, before anything is written.
		let mut values = self.operands(fields, out);
		for value in &mut values {
			if !value.is_path() {
				let saved = std::mem::replace(value, Expr::undefined());
				*value = self.spill(saved, "arg", out);
			}
		}
		let ExprKind::Array { fields: arguments } = &thir[self.strip(arguments)].kind else {
			return Err(UNEXPECTED.into());
		};
		let mut placeholders = Vec::new();
		for &argument in arguments.iter() {
			let ExprKind::Call { ty, args, .. } = &thir[self.strip(argument)].kind else {
				return Err(UNEXPECTED.into());
			};
			let ty::FnDef(def_id, gargs) = *self.subst(*ty).kind() else {
				return Err(UNEXPECTED.into());
			};
			let tr = match self.t.tcx.item_name(def_id).as_str() {
				"new_display" => Trait::Display,
				"new_debug" => Trait::Debug,
				"new_lower_hex" => Trait::LowerHex,
				"new_upper_hex" => Trait::UpperHex,
				"from_usize" => return Err(FROM_ARGUMENT.into()),
				"new_octal" => return Err("`Octal` formatting, as in `{:o}`,".into()),
				"new_binary" => return Err("`Binary` formatting, as in `{:b}`,".into()),
				"new_lower_exp" => return Err("`LowerExp` formatting, as in `{:e}`,".into()),
				"new_upper_exp" => return Err("`UpperExp` formatting, as in `{:E}`,".into()),
				"new_pointer" => return Err("`Pointer` formatting, as in `{:p}`,".into()),
				_ => return Err(UNEXPECTED.into()),
			};
			let field = self.tuple_field(args[0], values_var).ok_or(UNEXPECTED)?;
			let ty = self.subst(gargs.skip_binder().types().next().ok_or(UNEXPECTED)?);
			placeholders.push((tr, field, ty));
		}
		// `unsafe { Arguments::new(template, &args) }`
		let call = match &thir[self.strip(block.expr.ok_or(UNEXPECTED)?)].kind {
			ExprKind::Block { block } => thir[*block].expr.ok_or(UNEXPECTED)?,
			_ => return Err(UNEXPECTED.into()),
		};
		let ExprKind::Call { args, .. } = &thir[self.strip(call)].kind else {
			return Err(UNEXPECTED.into());
		};
		let template = self.byte_str(args[0]).ok_or(UNEXPECTED)?;
		let span = self.span(id);
		let mut parts = Vec::new();
		for piece in decode(&template)? {
			match piece {
				Piece::Lit(s) => parts.push(Expr::Str(s)),
				Piece::Arg { index, spec } => {
					let &(tr, field, ty) = placeholders.get(index).ok_or(UNEXPECTED)?;
					let value = values.get(field).ok_or(UNEXPECTED)?.clone();
					let value = self.fmt_value(tr, ty, value, span, out);
					parts.push(match spec.object() {
						Some(spec) => Expr::Rt("fmt").member("spec").call(vec![spec, value]),
						None => value,
					});
				}
			}
		}
		Ok(parts)
	}

	/// The index of `args.N`, a field of the tuple `var`, behind borrows and derefs.
	fn tuple_field(&self, id: ExprId, var: rustc_middle::thir::LocalVarId) -> Option<usize> {
		match &self.thir[self.strip(id)].kind {
			ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => self.tuple_field(*arg, var),
			ExprKind::Field { lhs, name, .. } => match self.thir[self.strip(*lhs)].kind {
				ExprKind::VarRef { id } if id == var => Some(name.index()),
				_ => None,
			},
			_ => None,
		}
	}

	/// The bytes of a byte string literal, behind borrows and derefs.
	fn byte_str(&self, id: ExprId) -> Option<Vec<u8>> {
		match &self.thir[self.strip(id)].kind {
			ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => self.byte_str(*arg),
			ExprKind::Literal { lit, .. } => match &lit.node {
				rustc_ast::LitKind::ByteStr(bytes, _) => Some(bytes.as_byte_str().to_vec()),
				_ => None,
			},
			_ => None,
		}
	}

	/// `value`, a `ty`, written by `tr` to `f`, through the impl for `ty`.
	pub fn fmt_call(&mut self, tr: Trait, ty: Ty<'tcx>, value: Expr, f: Expr, span: Span) -> Expr {
		let tcx = self.t.tcx;
		let Some(method) = tr.method(tcx) else {
			return self.err(span, format!("no `{}::fmt`", tr.name()));
		};
		let args = tcx.mk_args(&[ty.into()]);
		let mut pre = Vec::new();
		let call = self.call_def(method, args, Args::Values(vec![value, f]), span, &mut pre);
		match pre.is_empty() {
			true => call,
			false => self.err(span, format!("`{}` of `{ty}` needs statements", tr.name())),
		}
	}

	/// A function writing a `ty` by `tr`: `(value, f) => ...`, or the runtime's own.
	pub fn fmt_fn(&mut self, tr: Trait, ty: Ty<'tcx>, span: Span) -> Expr {
		let body = self.fmt_call(tr, ty, Expr::ident("value"), Expr::ident("f"), span);
		if let Expr::Call(callee, args) = &body
			&& let [Expr::Ident(value), Expr::Ident(f)] = args.as_slice()
			&& value == "value"
			&& f == "f"
			&& runtime(callee)
		{
			return (**callee).clone();
		}
		let params = vec![
			Param {
				name: "value".into(),
				ty: Some(self.t.ty(ty, span)),
			},
			Param {
				name: "f".into(),
				ty: Some(formatter()),
			},
		];
		Expr::Lambda(params, Box::new(body))
	}

	/// `value`, a `ty`, ready to be written by `tr`: `(f) => ...`. It is read now, once,
	/// as a placeholder's or a `&dyn Debug`'s value is.
	pub fn fmt_value(&mut self, tr: Trait, ty: Ty<'tcx>, value: Expr, span: Span, out: &mut Vec<Stmt>) -> Expr {
		let value = match value.is_path() {
			true => value,
			false => self.spill(value, "arg", out),
		};
		self.thunk(tr, ty, value, span)
	}

	/// `(f) => ...` writing `value`, which must be [`pure`].
	fn thunk(&mut self, tr: Trait, ty: Ty<'tcx>, value: Expr, span: Span) -> Expr {
		let f = match mentions(&value, "f") {
			true => self.fresh("f"),
			false => "f".to_string(),
		};
		let body = self.fmt_call(tr, ty, value, Expr::ident(&f), span);
		Expr::Lambda(
			vec![Param {
				name: f,
				ty: Some(formatter()),
			}],
			Box::new(body),
		)
	}
}

/// Whether `e` is a runtime function, `rt.a.b`, which reads no `this`.
fn runtime(e: &Expr) -> bool {
	match e {
		Expr::Rt(_) => true,
		Expr::Member(base, Key::Name(_)) => runtime(base),
		_ => false,
	}
}

/// Whether the path `e` reads the variable `name`.
fn mentions(e: &Expr, name: &str) -> bool {
	match e {
		Expr::Ident(ident) => ident == name,
		Expr::Member(base, _) | Expr::Located(_, base) => mentions(base, name),
		Expr::Index(base, index) => mentions(base, name) || mentions(index, name),
		_ => false,
	}
}

/// Whether reading `e` again gives the same value and does nothing else: a path, or an
/// element of one.
fn pure(e: &Expr) -> bool {
	match e {
		Expr::Index(base, index) => pure(base) && index.is_constant(),
		e => e.is_path(),
	}
}

/// std's impl of `tr` for `ty`, writing `value` to `f`; `None` if it has none here.
pub fn std_impl<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	tr: Trait,
	ty: Ty<'tcx>,
	value: Expr,
	f: Expr,
	span: Span,
	out: &mut Vec<Stmt>,
) -> Option<Expr> {
	let tcx = b.t.tcx;
	let rt = |name: &str, args: Vec<Expr>| Expr::Rt("fmt").member(name).call(args);
	let text = matches!(tr, Trait::Display | Trait::Debug);
	Some(match ty.kind() {
		ty::Ref(_, inner, _) => b.fmt_call(tr, *inner, value, f, span),
		ty::Int(_) | ty::Uint(_) => {
			let method = match tr {
				Trait::Display | Trait::Debug => "display",
				Trait::LowerHex => "lowerHex",
				Trait::UpperHex => "upperHex",
			};
			Int::of(ty)?.method(method, vec![value, f])
		}
		ty::Bool if text => rt("bool", vec![value, f]),
		ty::Str if tr == Trait::Display => f.member("pad").call(vec![value]),
		ty::Str if tr == Trait::Debug => rt("str", vec![value, f]),
		ty::Tuple(tys) if tys.is_empty() && tr == Trait::Debug => rt("unit", vec![value, f]),
		ty::Tuple(tys) if tr == Trait::Debug => {
			// Each element's function reads the tuple.
			let value = match pure(&value) {
				true => value,
				false => b.spill(value, "arg", out),
			};
			let fields = tys
				.iter()
				.enumerate()
				.map(|(i, t)| b.thunk(tr, t, value.clone().index(Expr::lit(i.to_string())), span))
				.collect();
			rt("tuple", vec![Expr::Array(fields), f])
		}
		ty::Array(elem, _) | ty::Slice(elem) if tr == Trait::Debug => {
			let item = b.fmt_fn(tr, *elem, span);
			rt("list", vec![value, f, item])
		}
		// A `&dyn Debug` is already a function writing its value.
		ty::Dynamic(preds, ..) if preds.principal_def_id().and_then(|did| Trait::of(tcx, did)) == Some(tr) => {
			value.call(vec![f])
		}
		ty::Adt(adt, args) => match (Std::of(tcx, adt.did())?, tr) {
			(Std::String, Trait::Display) => f.member("pad").call(vec![value]),
			(Std::String, Trait::Debug) => rt("str", vec![value, f]),
			(Std::Box, _) => b.fmt_call(tr, args.type_at(0), value, f, span),
			(Std::Vec, Trait::Debug) => {
				let item = b.fmt_fn(tr, args.type_at(0), span);
				rt("list", vec![library::vec_view(value, ty), f, item])
			}
			(Std::Option, Trait::Debug) => {
				let some = b.fmt_fn(tr, args.type_at(0), span);
				rt("option", vec![value, f, some])
			}
			(Std::Result, Trait::Debug) => {
				let ok = b.fmt_fn(tr, args.type_at(0), span);
				let err = b.fmt_fn(tr, args.type_at(1), span);
				rt("result", vec![value, f, ok, err])
			}
			(Std::Arguments, Trait::Display | Trait::Debug) => rt("arguments", vec![value, f]),
			(Std::FmtError, Trait::Debug) => rt("errorDebug", vec![value, f]),
			(Std::FmtError, Trait::Display) => rt("errorDisplay", vec![value, f]),
			(Std::TryFromIntError, Trait::Debug) => rt("tryFromIntErrorDebug", vec![value, f]),
			(Std::TryFromIntError, Trait::Display) => rt("tryFromIntErrorDisplay", vec![value, f]),
			(Std::FromUtf8Error, Trait::Debug) => rt("fromUtf8ErrorDebug", vec![value, f]),
			(Std::FromUtf8Error, Trait::Display) => rt("fromUtf8ErrorDisplay", vec![value, f]),
			_ => return None,
		},
		_ => return None,
	})
}

/// One part of a `format_args!` template.
pub enum Piece {
	Lit(String),
	/// A placeholder writing the `index`th argument.
	Arg {
		index: usize,
		spec: Spec,
	},
}

/// A placeholder's options, from `FormattingOptions`' flags.
pub struct Spec {
	fill: char,
	align: Option<char>,
	alternate: bool,
	zero: bool,
	width: Option<u16>,
}

impl Spec {
	/// The runtime's `Spec` object, or `None` for the defaults.
	pub fn object(&self) -> Option<Expr> {
		let mut props = Vec::new();
		if self.fill != ' ' {
			props.push(("fill", Expr::Str(self.fill.to_string())));
		}
		if let Some(align) = self.align {
			props.push(("align", Expr::Str(align.to_string())));
		}
		if self.alternate {
			props.push(("alternate", Expr::lit("true")));
		}
		if self.zero {
			props.push(("zero", Expr::lit("true")));
		}
		if let Some(width) = self.width {
			props.push(("width", Expr::lit(width.to_string())));
		}
		let props: Vec<(Key, Expr)> = props.into_iter().map(|(k, v)| (Key::Name(k.into()), v)).collect();
		(!props.is_empty()).then_some(Expr::Object(props))
	}
}

// `FormattingOptions`' flags, as `core::fmt` lays them out.
const SIGN_PLUS: u32 = 1 << 21;
const SIGN_MINUS: u32 = 1 << 22;
const ALTERNATE: u32 = 1 << 23;
const ZERO_PAD: u32 = 1 << 24;
const DEBUG_HEX: u32 = 0b11 << 25;
const WIDTH: u32 = 1 << 27;
const PRECISION: u32 = 1 << 28;
const FILL: u32 = (1 << 21) - 1;
const DEFAULT_FLAGS: u32 = ' ' as u32 | 3 << 29;

/// Decode a template as `core::fmt::write` reads it: literal pieces, and placeholders
/// with their argument and options. An option outside the subset is an error naming it.
pub fn decode(template: &[u8]) -> Result<Vec<Piece>, &'static str> {
	const TRUNCATED: &str = "a truncated template";
	let byte = |i: usize| template.get(i).copied().ok_or(TRUNCATED);
	let u16_at = |i: usize| Ok::<_, &str>(u16::from_le_bytes([byte(i)?, byte(i + 1)?]));
	let lit = |start: usize, len: usize| {
		let bytes = template.get(start..start + len).ok_or(TRUNCATED)?;
		String::from_utf8(bytes.to_vec()).map_err(|_| "a template piece that is not UTF-8")
	};
	let mut pieces = Vec::new();
	let mut i = 0;
	let mut next = 0;
	loop {
		let n = byte(i)?;
		i += 1;
		match n {
			0 => return Ok(pieces),
			1..=0x7f => {
				pieces.push(Piece::Lit(lit(i, n.into())?));
				i += usize::from(n);
			}
			0x80 => {
				let len = usize::from(u16_at(i)?);
				pieces.push(Piece::Lit(lit(i + 2, len)?));
				i += 2 + len;
			}
			0xc0..=0xff => {
				let mut flags = DEFAULT_FLAGS;
				let mut width = 0;
				if n & 1 != 0 {
					flags = u32::from_le_bytes([byte(i)?, byte(i + 1)?, byte(i + 2)?, byte(i + 3)?]);
					i += 4;
				}
				if n & 2 != 0 {
					width = u16_at(i)?;
					i += 2;
				}
				if n & 4 != 0 {
					i += 2;
				}
				if n & 8 != 0 {
					next = usize::from(u16_at(i)?);
					i += 2;
				}
				if n & (16 | 32) != 0 {
					return Err(FROM_ARGUMENT);
				}
				if flags & PRECISION != 0 {
					return Err("a precision, as in `{:.2}`,");
				}
				if flags & SIGN_PLUS != 0 {
					return Err("the `+` flag of a format spec");
				}
				if flags & SIGN_MINUS != 0 {
					return Err("the `-` flag of a format spec");
				}
				if flags & DEBUG_HEX != 0 {
					return Err("hex `Debug`, as in `{:x?}`,");
				}
				let spec = Spec {
					fill: char::from_u32(flags & FILL).ok_or("a fill that is not a char")?,
					align: match flags >> 29 & 3 {
						0 => Some('<'),
						1 => Some('>'),
						2 => Some('^'),
						_ => None,
					},
					alternate: flags & ALTERNATE != 0,
					zero: flags & ZERO_PAD != 0,
					width: (flags & WIDTH != 0).then_some(width),
				};
				pieces.push(Piece::Arg { index: next, spec });
				next += 1;
			}
			_ => return Err("an unknown template byte"),
		}
	}
}
