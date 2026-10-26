//! Calls: to the crate's own functions, to closures, and to std through the library
//! mappings. Some std calls lower here instead, since MIR passes what THIR wrote as an
//! expression through a temporary: a place to replace (`mem::take`), a range to slice
//! by, an array of bytes to read, `format_args!`'s parts, and `?`'s `ControlFlow`.

use rustc_abi::VariantIdx;
use rustc_middle::mir::{Operand, Place};
use rustc_middle::ty::{self, GenericArgsRef, Instance, Ty, TyCtxt};
use rustc_span::{Span, Spanned};

use super::{Fx, Slot};
use crate::lower::fmt::{self, Piece};
use crate::lower::library::{self, Std};
use crate::lower::num::Int;
use crate::ts::{Expr, Stmt, StmtKind};

impl<'m, 'b, 'a, 'tcx> Fx<'m, 'b, 'a, 'tcx> {
	pub(super) fn call(
		&mut self,
		func: &Operand<'tcx>,
		args: &[Spanned<Operand<'tcx>>],
		dest: Place<'tcx>,
		span: Span,
		out: &mut Vec<Stmt>,
	) {
		let tcx = self.tcx;
		let fn_ty = func.ty(&self.mir.local_decls, tcx);
		let mut pre = Vec::new();
		let (value, effect) = match *fn_ty.kind() {
			// `next` into a local that holds the protocol's item or `rt.DONE`, unconverted.
			ty::FnDef(def_id, gargs)
				if crate::lower::iter::is_next(tcx, def_id)
					&& dest.as_local().is_some_and(|l| self.next_results.contains(&l)) =>
			{
				let iter_ty = gargs.skip_binder().type_at(0);
				let recv = self.operand_frozen(&args[0].node, span, &mut pre);
				if self.range_nexts.contains(&dest.local)
					&& let Some(name) = self.var(dest.local)
				{
					let name = name.to_string();
					self.assigned[dest.local] = true;
					let stepped = match &args[0].node {
						Operand::Copy(p) | Operand::Move(p) => {
							p.as_local().is_some_and(|b| self.stepped_borrows.contains_key(&b))
						}
						Operand::Constant(_) | Operand::RuntimeChecks(_) => false,
					};
					let read = self.uses[dest.local] > 1;
					let held = self.range_next(name, recv, stepped, read, &mut pre);
					self.emit(pre, None, false, out);
					if let Some((flag, more)) = held {
						self.hold_flag(flag, more);
					}
					return;
				}
				if let Some(name) = self.var(dest.local) {
					let name = name.to_string();
					self.next_names.insert(name);
				}
				if self.stepped_array(&args[0].node) {
					let it = self.path(recv, "iter", &mut pre);
					(self.stepped_next(it), true)
				} else {
					(crate::lower::iter::next(self.b, iter_ty, recv), true)
				}
			}
			ty::FnDef(def_id, gargs) => match self.intercept(def_id, gargs.skip_binder(), args, dest, span, &mut pre) {
				Some(Some(done)) => done,
				// Done: the destination is set.
				Some(None) => {
					self.emit(pre, None, false, out);
					return;
				}
				None => (self.call_def(def_id, gargs.skip_binder(), args, span, &mut pre), true),
			},
			ty::FnPtr(..) => {
				let f = self.operand(func, span, &mut pre);
				let values = self.operands(args, span, &mut pre);
				(f.call(values), true)
			}
			_ => (self.err(span, format!("a call through `{fn_ty}`")), true),
		};
		let value = match dest.as_local() {
			Some(local) if self.stepped.contains(&local) => self.stepped_make(value, span),
			_ => value,
		};
		let value = match self.b.t.loc(span) {
			Some(loc) if effect => Expr::Located(loc, Box::new(value)),
			_ => value,
		};
		if !pre.is_empty() {
			self.emit(pre, None, false, out);
		}
		// What an `Iterator::next` returns, by the protocol, unless a `next` made it already.
		let value = match dest.as_local() {
			Some(rustc_middle::mir::RETURN_PLACE)
				if self.next && !matches!(*fn_ty.kind(), ty::FnDef(d, _) if crate::lower::iter::is_next(tcx, d)) =>
			{
				super::done_of(value)
			}
			_ => value,
		};
		// A result into a local changed in place is its own copy: it may share the callee's.
		let value = match dest.as_local() {
			Some(local) if self.mutated[local] => {
				let ty = self.mir.local_decls[local].ty;
				self.b.copied(value, ty)
			}
			_ => value,
		};
		match dest.as_local() {
			Some(local) => {
				let memory = effect || args.iter().any(|a| super::rvalue::operand_memory(&a.node, self));
				self.set_local(local, value, effect, memory, None, out)
			}
			None => {
				let stmts = self.store(dest, value, span);
				self.emit(stmts, None, false, out);
			}
		}
	}

	/// Arguments, uncopied: a callee that changes a parameter in place copies it first.
	fn operands(&mut self, args: &[Spanned<Operand<'tcx>>], span: Span, out: &mut Vec<Stmt>) -> Vec<Expr> {
		let saved = std::mem::replace(&mut self.into_frozen, true);
		let values = args
			.iter()
			.map(|arg| self.operand_frozen(&arg.node, span, out))
			.collect();
		self.into_frozen = saved;
		values
	}

	/// An argument, which the callee copies if it changes it in place.
	fn operand_frozen(&mut self, op: &Operand<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Expr {
		match op {
			Operand::Copy(place) | Operand::Move(place) => self.place_expr(*place, span, out),
			_ => self.operand(op, span, out),
		}
	}

	/// A call of `def_id` with `gargs`: the crate's own function, a closure, or std's.
	fn call_def(
		&mut self,
		def_id: rustc_hir::def_id::DefId,
		gargs: GenericArgsRef<'tcx>,
		args: &[Spanned<Operand<'tcx>>],
		span: Span,
		out: &mut Vec<Stmt>,
	) -> Expr {
		let tcx = self.tcx;
		// Calling a closure or function value: `FnOnce::call_once(f, (args,))`.
		if let Some(trait_id) = tcx.trait_of_assoc(def_id)
			&& tcx.is_fn_trait(trait_id)
			&& let [callee, tupled] = args
		{
			let callee_ty = gargs.type_at(0);
			let values = match self.take_parts(&tupled.node) {
				Some(parts) => parts,
				None => {
					let tuple = self.operand_frozen(&tupled.node, span, out);
					let tuple = self.path(tuple, "args", out);
					let ty::Tuple(tys) = gargs.type_at(1).kind() else {
						return self.err(span, "a closure call whose arguments are not a tuple");
					};
					(0..tys.len())
						.map(|i| tuple.clone().index(Expr::lit(i.to_string())))
						.collect()
				}
			};
			if let ty::FnDef(inner, inner_args) = *callee_ty.peel_refs().kind() {
				return self.b.call_def(inner, inner_args.skip_binder(), values, span, out);
			}
			let f = self.operand(&callee.node, span, out);
			return f.call(values);
		}
		// A function of the crate gets what JS passes, so a `&mut` to a number can't reach it.
		let local = match Instance::try_resolve(tcx, self.b.t.typing_env_pub(), def_id, gargs) {
			Ok(Some(instance)) => matches!(instance.def, ty::InstanceKind::Item(did) if did.is_local()),
			_ => false,
		};
		if local {
			for arg in args {
				self.check_escape(&arg.node, span);
			}
		}
		let values = self.operands(args, span, out);
		self.b.call_def(def_id, gargs, values, span, out)
	}

	/// Refuse a `&mut` to a value JS copies, passed on whole to code that could write
	/// through it: it would change a copy.
	fn check_escape(&mut self, op: &Operand<'tcx>, span: Span) {
		let (Operand::Copy(place) | Operand::Move(place)) = op else {
			return;
		};
		if !place.projection.is_empty() {
			return;
		}
		let ty = self.mir.local_decls[place.local].ty;
		if let ty::Ref(_, inner, rustc_ast::Mutability::Mut) = ty.kind()
			&& matches!(self.slots[place.local], Slot::Alias(_))
			&& self.b.t.object_kind(*inner).is_none()
		{
			self.err(
				span,
				format!("a mutable reference to `{inner}` passed on, which is no object it can change in place"),
			);
		}
	}

	/// A std call MIR needs lowered here: `Some(Some((value, effect)))`, or `Some(None)`
	/// once it set the destination itself, or `None` for the library mappings.
	fn intercept(
		&mut self,
		def_id: rustc_hir::def_id::DefId,
		gargs: GenericArgsRef<'tcx>,
		args: &[Spanned<Operand<'tcx>>],
		dest: Place<'tcx>,
		span: Span,
		out: &mut Vec<Stmt>,
	) -> Option<Option<(Expr, bool)>> {
		let tcx = self.tcx;
		if library::std_path(tcx, def_id).is_empty() {
			return None;
		}
		let (owner, name) = library::key(tcx, def_id)?;
		let targ = |n: usize| gargs.types().nth(n);
		let done = |e: Expr, effect: bool| Some(Some((e, effect)));
		match (owner.as_str(), name.as_str()) {
			// Replacing what a `&mut` borrows.
			("mem", "take") => {
				let place = self.borrowed_place(&args[0].node, span)?;
				let default = library::default_value(self.b, targ(0)?)?;
				done(self.swap_out(place, default, "taken", out), true)
			}
			("mem", "replace") => {
				let place = self.borrowed_place(&args[0].node, span)?;
				let value = self.operand(&args[1].node, span, out);
				done(self.swap_out(place, value, "old", out), true)
			}
			("mem", "swap") => {
				let a = self.borrowed_place(&args[0].node, span)?;
				let c = self.borrowed_place(&args[1].node, span)?;
				let old = self.swap_out(a, c.clone(), "swapped", out);
				out.push(StmtKind::Expr(c.assign(old)).at(None));
				done(Expr::undefined(), true)
			}
			// A range of numbers steps in place, with nothing between it and the loop.
			("Iterator", "next") if super::small_range(tcx, targ(0)?) => {
				let range = self.alias_place(&args[0].node)?;
				let start = range.clone().member("start");
				let next = Expr::Cond(
					Box::new(start.clone().binary("<", range.member("end"))),
					Box::new(Expr::Postfix(Box::new(start), "++")),
					Box::new(Expr::undefined()),
				);
				done(next, true)
			}
			("Option", "take") => {
				let place = self.borrowed_place(&args[0].node, span)?;
				done(self.swap_out(place, Expr::undefined(), "taken", out), true)
			}
			// Indexing, whose result a `&mut` reads in place.
			("Index" | "IndexMut", "index" | "index_mut") => self.index(&name, gargs, args, dest, span, out),
			("slice", "copy_within") => {
				let s = self.operand(&args[0].node, span, out);
				let s = self.path(s, "slice", out);
				let (start, end) = self.range_bounds(&args[1].node, targ(1)?, span, out)?;
				let end = end.unwrap_or_else(|| s.clone().member("length"));
				let dest = self.operand(&args[2].node, span, out);
				done(Expr::rt("sliceCopyWithin", vec![s, start, end, dest]), true)
			}
			// An integer from an array of bytes made in place: its bytes shifted, no array made.
			(int, "from_be_bytes") if let Some(int) = int_named(int) => {
				let parts = match int.big() {
					true => None,
					false => self.take_parts(&args[0].node),
				};
				let value = match parts {
					Some(parts) => library::be_word(int, parts),
					None => {
						let bytes = self.operand_frozen(&args[0].node, span, out);
						int.method("fromBeBytes", vec![bytes])
					}
				};
				done(value, false)
			}
			// `format_args!`, as MIR builds it: each argument a function writing its value,
			// and a template of literal pieces and placeholders.
			("Argument", name) if name.starts_with("new_") || name == "from_usize" => {
				let tr = match fmt::argument(name) {
					Ok(tr) => tr,
					Err(construct) => {
						self.err(span, format!("{construct} is not supported"));
						return done(Expr::undefined(), false);
					}
				};
				let ty = targ(0)?;
				let value = self.operand(&args[0].node, span, out);
				// The function reads it later, in a closure, where TypeScript no longer
				// narrows a member of a union: a member is read now, into a name.
				let value = match value {
					Expr::Member(..) | Expr::Index(..) => {
						let name = self.b.fresh("arg");
						let stmt = StmtKind::decl(false, name.clone(), None, Some(value)).at(None);
						self.emit(vec![stmt], None, false, out);
						Expr::ident(name)
					}
					value => self.path(value, "arg", out),
				};
				done(self.b.fmt_value(tr, ty, value, span, out), false)
			}
			("Arguments", "from_str" | "from_str_nonconst") => {
				let s = self.operand(&args[0].node, span, out);
				done(Expr::Array(vec![s]), false)
			}
			("Arguments", "new") => {
				let Some(template) = self.const_bytes(&args[0].node) else {
					self.err(
						span,
						format!("an unexpected `format_args!` lowering: {:?}", args[0].node),
					);
					return done(Expr::undefined(), false);
				};
				let values = self.operand(&args[1].node, span, out);
				// The arguments array made once in place: each piece its function.
				let made: Option<Vec<Expr>> = match &values {
					Expr::Ident(name) => self.local_named(name).and_then(|l| self.var_parts.get(&l).cloned()),
					_ => None,
				};
				let values = self.path(values, "args", out);
				let pieces = match fmt::decode(&template) {
					Ok(pieces) => pieces,
					Err(msg) => {
						self.err(span, format!("{msg} is not supported"));
						return done(Expr::undefined(), false);
					}
				};
				let mut parts = Vec::new();
				for piece in pieces {
					match piece {
						Piece::Lit(s) => parts.push(Expr::Str(s)),
						Piece::Arg { index, spec } => {
							let value = match made.as_ref().and_then(|p| p.get(index)) {
								Some(part) => part.clone(),
								None => values.clone().index(Expr::lit(index.to_string())),
							};
							parts.push(match spec.object() {
								Some(spec) => Expr::rt("fmtSpec", vec![spec, value]),
								None => value,
							});
						}
					}
				}
				done(Expr::Array(parts), false)
			}
			// `?`: the value it was applied to is the `ControlFlow` MIR branches on.
			// The crate's own iterator is its own `IntoIterator`; std's adaptors convert it
			// where they take it.
			("IntoIterator", "into_iter")
				if matches!(targ(0)?.kind(), ty::Adt(adt, _) if adt.did().is_local())
					&& self.b.t.iter_item(targ(0)?).is_some() =>
			{
				let value = self.operand(&args[0].node, span, out);
				done(value, false)
			}
			("Try", "branch") => {
				let value = self.operand(&args[0].node, span, out);
				// A `next`'s item or `rt.DONE` branches as itself.
				if let Some(local) = dest.as_local()
					&& self.next_results.contains(&local)
					&& let Some(name) = self.var(local)
				{
					let name = name.to_string();
					self.next_names.insert(name);
				}
				done(value, false)
			}
			("FromResidual", "from_residual") => {
				let to = targ(0)?;
				let residual = targ(1)?;
				let value = self.operand(&args[0].node, span, out);
				match (self.b.t.std_of(to), to.kind(), residual.kind()) {
					(Some(Std::Option), ..) => done(Expr::undefined(), false),
					(Some(Std::Result), ty::Adt(_, to_args), ty::Adt(_, from_args)) => {
						let (from, to) = (from_args.type_at(1), to_args.type_at(1));
						if from == to {
							return done(value, false);
						}
						let value = self.path(value, "residual", out);
						// The residual is an `Err`.
						let err = crate::ts::Type::Object(vec![
							("ok".into(), crate::ts::Type::Builtin("false")),
							("error".into(), self.b.t.ty(from, span)),
						]);
						let error = Expr::As(Box::new(value), err).member("error");
						let converted = self.b.convert(error, from, to, span, out);
						done(Expr::rt("Err", vec![converted]), true)
					}
					_ => {
						self.err(span, format!("`?` from `{residual}` to `{to}`"));
						done(Expr::undefined(), false)
					}
				}
			}
			("DebugList", "entries") => {
				let list = self.operand(&args[0].node, span, out);
				let iter_ty = targ(1)?;
				let items = self.operand(&args[1].node, span, out);
				let items = self.items(items, iter_ty, span)?;
				let debug = self.b.fmt_fn(fmt::Trait::Debug, targ(0)?, span);
				done(Expr::rt("debugListEntries", vec![list, items, debug]), true)
			}
			_ => None,
		}
	}

	/// The place a `&mut` operand borrows, as an assignable path: an alias of a borrowed
	/// place, not a copy of a reference, which holds the object it points at.
	fn borrowed_place(&mut self, op: &Operand<'tcx>, span: Span) -> Option<Expr> {
		let (Operand::Copy(place) | Operand::Move(place)) = op else {
			return None;
		};
		if !place.projection.is_empty() {
			return None;
		}
		match &self.slots[place.local] {
			Slot::Alias(Some(target))
				if (target.is_path() || matches!(target, Expr::Index(..))) && self.alias_of_place(place.local) =>
			{
				Some(target.clone())
			}
			_ => {
				let pointee = self.mir.local_decls[place.local].ty.peel_refs();
				Some(self.err(span, format!("replacing the value behind a reference to `{pointee}`")))
			}
		}
	}

	/// The place a reference operand reads in place, if it is one.
	fn alias_place(&self, op: &Operand<'tcx>) -> Option<Expr> {
		let (Operand::Copy(place) | Operand::Move(place)) = op else {
			return None;
		};
		match (&self.slots[place.as_local()?], place.projection.is_empty()) {
			(Slot::Alias(Some(target)), true) if target.is_path() => Some(target.clone()),
			_ => None,
		}
	}

	/// `name = recv.next()` of a range of numbers, stepped in place rather than through
	/// `rt.Range`'s generic steps, into the item alone and a flag for whether there was
	/// one: a number that is sometimes `rt.DONE` is slower in V8. Past the end, `name` is
	/// left as it was, since only `Some` reads it. A range only `next` reads (`stepped`)
	/// steps past its end too, so the flag is one comparison, to hold back for the switch
	/// that reads it; it assigns `name` only if something reads it (`read`).
	fn range_next(
		&mut self,
		name: String,
		recv: Expr,
		stepped: bool,
		read: bool,
		out: &mut Vec<Stmt>,
	) -> Option<(String, Expr)> {
		let range = self.path(recv, "range", out);
		let start = range.clone().member("start");
		let flag = self.b.fresh(&format!("{name}Some"));
		self.range_flags.insert(name.clone(), flag.clone());
		if stepped {
			let step = Expr::Postfix(Box::new(start), "++");
			let item = match read {
				true => Expr::ident(&name).assign(step),
				false => step,
			};
			return Some((flag, item.binary("<", range.member("end"))));
		}
		let more = start.clone().binary("<", range.member("end"));
		out.push(StmtKind::Expr(Expr::ident(&flag).assign(more)).at(None));
		let step = Expr::ident(&name).assign(Expr::Postfix(Box::new(start), "++"));
		out.push(
			StmtKind::If {
				cond: Expr::ident(&flag),
				then: vec![StmtKind::Expr(step).at(None)],
				els: Vec::new(),
			}
			.at(None),
		);
		None
	}

	/// The value at `place`, after setting `place` to `value`.
	fn swap_out(&mut self, place: Expr, value: Expr, base: &str, out: &mut Vec<Stmt>) -> Expr {
		let name = self.b.fresh(base);
		out.push(StmtKind::decl(false, name.clone(), None, Some(place.clone())).at(None));
		out.push(StmtKind::Expr(place.assign(value)).at(None));
		Expr::ident(name)
	}

	/// `container[index]`, or a slice of it by a range, which a reference reads in place.
	fn index(
		&mut self,
		name: &str,
		gargs: GenericArgsRef<'tcx>,
		args: &[Spanned<Operand<'tcx>>],
		dest: Place<'tcx>,
		span: Span,
		out: &mut Vec<Stmt>,
	) -> Option<Option<(Expr, bool)>> {
		let container = gargs.type_at(0);
		let index_ty = gargs.type_at(1);
		let elem_ty = elem(self.tcx, container)?;
		let base = self.operand(&args[0].node, span, out);
		let base = match container.kind() {
			ty::Adt(..) => library::vec_view(base, container),
			_ => base,
		};
		match Int::of(index_ty) {
			Some(_) => {
				let base = self.path(base, "slice", out);
				let i = self.operand(&args[1].node, span, out);
				let i = self.path(i, "index", out);
				let checked = Expr::rt("idx", vec![base.clone(), i]);
				let checked = self.path(checked, "at", out);
				let place = base.index(checked);
				// The reference reads and writes the element where it is.
				if let Some(local) = dest.as_local()
					&& self.defs[local] == 1
					&& matches!(self.slots[local], Slot::Var(_))
				{
					self.slots[local] = Slot::Alias(Some(place));
					return Some(None);
				}
				Some(Some((place, true)))
			}
			None => {
				if name == "index_mut" && !matches!(elem_ty.kind(), ty::Uint(ty::UintTy::U8)) {
					// A JS array has no views, so a mutable subslice would be a copy.
					return None;
				}
				// The range's bounds are taken apart before the slice is read, while they are
				// still held back.
				let (start, end) = self.range_bounds(&args[1].node, index_ty, span, out)?;
				let base = self.path(base, "slice", out);
				let mut values = vec![base, start];
				values.extend(end);
				Some(Some((Expr::rt("sliceRange", values), true)))
			}
		}
	}

	/// The start and optional end of a range: `a..b`, `..b`, `a..`, or `..`.
	fn range_bounds(
		&mut self,
		op: &Operand<'tcx>,
		ty: Ty<'tcx>,
		span: Span,
		out: &mut Vec<Stmt>,
	) -> Option<(Expr, Option<Expr>)> {
		let tcx = self.tcx;
		let ty::Adt(adt, _) = ty.kind() else {
			return None;
		};
		let name = tcx.item_name(adt.did()).to_string();
		match name.as_str() {
			"RangeFull" => return Some((Expr::lit("0"), None)),
			"Range" | "RangeTo" | "RangeFrom" => {}
			_ => return None,
		}
		let fields: Vec<Expr> = match self.take_parts(op) {
			Some(parts) => parts,
			None => {
				let range = self.operand(op, span, out);
				let range = self.path(range, "range", out);
				adt.non_enum_variant()
					.fields
					.iter()
					.map(|f| range.clone().member(f.name.as_str()))
					.collect()
			}
		};
		let mut fields = fields.into_iter();
		match name.as_str() {
			"Range" => Some((fields.next()?, Some(fields.next()?))),
			"RangeTo" => Some((Expr::lit("0"), fields.next())),
			_ => Some((fields.next()?, None)),
		}
	}

	/// What a `DebugList` iterates, as an iterable JS value.
	fn items(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> Option<Expr> {
		let tcx = self.tcx;
		match ty.peel_refs().kind() {
			ty::Slice(_) | ty::Array(..) => Some(value),
			ty::Adt(adt, _) if Std::of(tcx, adt.did()) == Some(Std::Vec) => Some(library::vec_view(value, ty)),
			_ => crate::lower::iter::into_iter(self.b, ty, value, span),
		}
	}
}

/// The element type of a slice, array, or `Vec`.
fn elem<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
	match ty.peel_refs().kind() {
		ty::Slice(e) | ty::Array(e, _) => Some(*e),
		ty::Adt(adt, args) if Std::of(tcx, adt.did()) == Some(Std::Vec) => Some(args.type_at(0)),
		_ => None,
	}
}

fn int_named(name: &str) -> Option<Int> {
	let (signed, bits, pointer) = match name {
		"u8" => (false, 8, false),
		"u16" => (false, 16, false),
		"u32" => (false, 32, false),
		"u64" => (false, 64, false),
		"u128" => (false, 128, false),
		"usize" => (false, 32, true),
		"i8" => (true, 8, false),
		"i16" => (true, 16, false),
		"i32" => (true, 32, false),
		"i64" => (true, 64, false),
		"i128" => (true, 128, false),
		"isize" => (true, 32, true),
		_ => return None,
	};
	Some(Int { bits, signed, pointer })
}

/// The residual a `ControlFlow` breaks with: what `?` was applied to.
fn residual<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Std> {
	let ty::Adt(_, args) = ty.kind() else {
		return None;
	};
	match args.type_at(0).kind() {
		ty::Adt(adt, _) => Std::of(tcx, adt.did()),
		_ => None,
	}
}

/// The test that `scrut`, a `ControlFlow`, is `Continue` (variant 0) or `Break`.
pub fn control_flow_test<'tcx>(
	fx: &mut Fx<'_, '_, '_, 'tcx>,
	scrut: &Expr,
	ty: Ty<'tcx>,
	variant: VariantIdx,
	span: Span,
) -> Expr {
	let continues = variant.index() == 0;
	if matches!(scrut, Expr::Ident(n) if fx.next_names.contains(n)) {
		return match continues {
			true => scrut.clone().binary("!==", Expr::Rt("DONE".into())),
			false => scrut.clone().binary("===", Expr::Rt("DONE".into())),
		};
	}
	match residual(fx.tcx, ty) {
		Some(Std::Result) => match continues {
			true => scrut.clone().member("ok"),
			false => scrut.clone().member("ok").not(),
		},
		Some(Std::Option) => match continues {
			true => scrut.clone().binary("!==", Expr::undefined()),
			false => scrut.clone().binary("===", Expr::undefined()),
		},
		_ => fx.err(span, format!("`{ty}` other than from `?`")),
	}
}

/// A field of a `ControlFlow`: `Continue`'s value, or `Break`'s residual, which is the
/// value `?` was applied to.
pub fn control_flow_field<'tcx>(
	fx: &mut Fx<'_, '_, '_, 'tcx>,
	base: Expr,
	ty: Ty<'tcx>,
	variant: Option<VariantIdx>,
	span: Span,
) -> Expr {
	let continues = variant.is_none_or(|v| v.index() == 0);
	match (residual(fx.tcx, ty), continues) {
		(Some(Std::Result), true) => base.member("value"),
		(Some(Std::Result), false) => base,
		(Some(Std::Option), true) => base,
		(Some(Std::Option), false) => Expr::undefined(),
		_ => fx.err(span, format!("`{ty}` other than from `?`")),
	}
}

/// A `ControlFlow` made in place.
pub fn control_flow<'tcx>(
	fx: &mut Fx<'_, '_, '_, 'tcx>,
	ty: Ty<'tcx>,
	variant: VariantIdx,
	mut values: Vec<Expr>,
	span: Span,
) -> Expr {
	let value = values.pop().unwrap_or_else(Expr::undefined);
	match (residual(fx.tcx, ty), variant.index()) {
		(Some(Std::Result), 0) => Expr::rt("Ok", vec![value]),
		(Some(Std::Result), _) => value,
		(Some(Std::Option), 0) => value,
		(Some(Std::Option), _) => Expr::undefined(),
		_ => fx.err(span, format!("`{ty}` other than from `?`")),
	}
}
