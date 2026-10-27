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
use crate::lower::library::{self, Flow, Std};
use crate::lower::num::Int;
use crate::ts::{Expr, Stmt, StmtKind};

/// What a `&mut` borrows: its path, and the place a store through the reference writes,
/// unless the reference is refused.
type Borrowed<'tcx> = (Expr, Option<Place<'tcx>>);

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
		let result = dest.ty(&self.mir.local_decls, tcx).ty;
		let mut pre = Vec::new();
		let (value, effect) = match *fn_ty.kind() {
			// A binding's export, which takes what JS passes, as std's mappings do not.
			ty::FnDef(def_id, gargs) if self.b.t.binds(def_id, gargs.skip_binder()) => (
				self.call_def(def_id, gargs.skip_binder(), args, result, span, &mut pre),
				true,
			),
			ty::FnDef(def_id, gargs) if self.bytes_across(def_id, gargs.skip_binder(), fn_ty) => {
				let path = tcx.def_path_str(def_id);
				let msg = format!(
					"`{path}` of a reference to an array of bytes it is generic over, which its mapping would take as the array, not as a slice of it"
				);
				(self.err(span, msg), true)
			}
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
					(self.stepped_next(&recv), true)
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
				None => (
					self.call_def(def_id, gargs.skip_binder(), args, result, span, &mut pre),
					true,
				),
			},
			ty::FnPtr(..) => {
				let f = self.operand(func, span, &mut pre);
				let values = self.operands(false, args, result, span, &mut pre);
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
		let value = match *fn_ty.kind() {
			ty::FnDef(def_id, gargs) if self.place_result(def_id, gargs.skip_binder(), args, span) => {
				match self.hold_place(dest, value, span, out) {
					Some(value) => value,
					None => return,
				}
			}
			_ => value,
		};
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
				self.b.t.copied(value, ty, span)
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

	/// Whether std's `def_id` makes a reference to a `[u8; N]` it holds or is handed, as
	/// `Option::as_ref` of an `Option<[u8; 4]>` or `filter` over `[u8; 4]` items do: its
	/// mapping has it as the `Uint8Array` itself, where translated code has a slice of it.
	fn bytes_across(&self, def_id: rustc_hir::def_id::DefId, gargs: GenericArgsRef<'tcx>, fn_ty: Ty<'tcx>) -> bool {
		let tcx = self.tcx;
		let t = &*self.b.t;
		if t.translated(def_id, gargs) || !gargs.types().any(owns_bytes) {
			return false;
		}
		let self_ty = gargs.types().next();
		// Mappings that make the slice, or take it.
		if let Some((owner, name)) = library::key(tcx, def_id) {
			let handled = match (owner.as_str(), name.as_str()) {
				("Index" | "IndexMut", "index" | "index_mut") | ("Clone", "clone") | ("Argument", _) => true,
				("slice", "iter" | "iter_mut") => true,
				("IntoIterator", "into_iter") => self_ty.is_some_and(|s| elem(tcx, s).is_some()),
				_ => false,
			};
			if handled {
				return false;
			}
		}
		// A closure is the crate's own.
		if tcx.trait_of_assoc(def_id).is_some_and(|tr| tcx.is_fn_trait(tr)) {
			return false;
		}
		let env = t.typing_env_pub();
		let normal = |ty: Ty<'tcx>| {
			tcx.try_normalize_erasing_regions(env, ty::Unnormalized::new_wip(ty))
				.unwrap_or(ty)
		};
		// An iterator's references were made where its items came from, unless it makes
		// them of items it holds.
		if tcx.trait_of_assoc(def_id).is_some()
			&& let Some(item) = self_ty.and_then(|s| t.iter_item(s))
			&& !owns_bytes(normal(item))
		{
			return false;
		}
		let sig = tcx.instantiate_bound_regions_with_erased(fn_ty.fn_sig(tcx));
		// An iterator of items without such references was checked where each adaptor was
		// made: a closure it holds takes them from there.
		let refs = |ty: Ty<'tcx>| {
			let ty = normal(ty);
			match t.iter_item(ty).map(normal) {
				Some(item) if !owns_bytes(item) && !holds_bytes_ref(item) => false,
				item => std::iter::once(ty).chain(item).any(holds_bytes_ref),
			}
		};
		sig.inputs_and_output.iter().any(refs) || gargs.types().any(refs)
	}

	/// Arguments, uncopied, since a callee that changes a parameter in place copies it
	/// first, but for a `Copy` aggregate std `keeps`, copied unless moved out of a local's
	/// own value, which the analysis then made the local's own.
	fn operands(
		&mut self,
		translated: bool,
		args: &[Spanned<Operand<'tcx>>],
		result: Ty<'tcx>,
		span: Span,
		out: &mut Vec<Stmt>,
	) -> Vec<Expr> {
		let saved = std::mem::replace(&mut self.into_frozen, true);
		let mut values = Vec::with_capacity(args.len());
		for arg in args {
			let value = self.operand_frozen(&arg.node, span, out);
			values.push(match &arg.node {
				// Its type, rather than its variant's, for std's runtime function to infer
				// its type parameters from, as `widen` gives a local's.
				Operand::Constant(c) if !translated && self.b.t.narrows(c.ty()) => {
					let ty = self.b.t.ty(c.ty(), span);
					super::whole(value, ty).unwrap_or_else(|value| value)
				}
				Operand::Copy(p) | Operand::Move(p)
					if self.moved_from(&arg.node).is_none()
						&& self.keeps(translated, args, p.ty(&self.mir.local_decls, self.tcx).ty, result) =>
				{
					self.b.t.copied(value, p.ty(&self.mir.local_decls, self.tcx).ty, span)
				}
				_ => value,
			});
		}
		self.into_frozen = saved;
		values
	}

	/// An argument, which the callee copies if it changes it in place.
	fn operand_frozen(&mut self, op: &Operand<'tcx>, span: Span, out: &mut Vec<Stmt>) -> Expr {
		match op {
			Operand::Copy(place) | Operand::Move(place) if !self.in_bytes(*place) => self.place_expr(*place, span, out),
			_ => self.operand(op, span, out),
		}
	}

	/// A call of `def_id` with `gargs`: the crate's own function, a closure, or std's.
	fn call_def(
		&mut self,
		def_id: rustc_hir::def_id::DefId,
		gargs: GenericArgsRef<'tcx>,
		args: &[Spanned<Operand<'tcx>>],
		result: Ty<'tcx>,
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
			// `FnOnce::call_once` of a closure that `Fn` or `FnMut` call drops it after,
			// as rustc's shim does, where an `FnOnce` closure's body drops what it owns.
			if tcx.fn_trait_kind_from_def_id(trait_id) == Some(ty::ClosureKind::FnOnce)
				&& let ty::Closure(_, closure) = callee_ty.kind()
				&& closure.as_closure().kind() != ty::ClosureKind::FnOnce
				&& self.b.t.drops(callee_ty)
			{
				return Expr::rt("callOnce", [vec![f], values].concat());
			}
			return f.call(values);
		}
		// A function of the crate gets what JS passes, so a `&mut` to a number can't reach it.
		let local = self.b.t.translated(def_id, gargs);
		if local {
			for arg in args {
				self.check_escape(&arg.node, span);
			}
		}
		let values = self.operands(local, args, result, span, out);
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
		// A box's own `Drop` frees its memory, once drop elaboration dropped or moved out
		// what it holds: nothing JS sees.
		let drop_trait = tcx
			.trait_of_assoc(def_id)
			.or_else(|| tcx.trait_impl_of_assoc(def_id).map(|imp| tcx.impl_trait_id(imp)));
		if drop_trait.is_some_and(|tr| tcx.is_lang_item(tr, rustc_attr_ir::LangItem::Drop))
			&& self
				.b
				.t
				.self_ty(Instance::new_raw(def_id, gargs))
				.is_some_and(|ty| self.b.t.std_of(ty) == Some(Std::Box))
		{
			return Some(Some((Expr::undefined(), false)));
		}
		let (owner, name) = library::key(tcx, def_id)?;
		let targ = |n: usize| gargs.types().nth(n);
		let done = |e: Expr, effect: bool| Some(Some((e, effect)));
		match (owner.as_str(), name.as_str()) {
			// Replacing what a `&mut` borrows.
			("mem", "take") => {
				let place = self.borrowed_place(&args[0].node, span)?;
				let default = library::default_value(self.b, targ(0)?)?;
				done(self.swap_out(place, default, "taken", span, out), true)
			}
			("mem", "replace") => {
				let place = self.borrowed_place(&args[0].node, span)?;
				let value = self.operand(&args[1].node, span, out);
				done(self.swap_out(place, value, "old", span, out), true)
			}
			("mem", "swap") => {
				let a = self.borrowed_place(&args[0].node, span)?;
				let c = self.borrowed_place(&args[1].node, span)?;
				let old = self.swap_out(a, c.0.clone(), "swapped", span, out);
				self.set_borrowed(&c, old, span, out);
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
			// A JS string can't change in place, so what the `&mut` borrows gets a longer one.
			("String", "push") => {
				let place = self.borrowed_place(&args[0].node, span)?;
				let c = self.operand(&args[1].node, span, out);
				let pushed = place.0.clone().binary("+", library::char_text(c));
				self.set_borrowed(&place, pushed, span, out);
				done(Expr::undefined(), false)
			}
			// A `Cow<str>` is the `String` it would hold, so the `&mut` borrows the same place.
			("Cow", "to_mut") if targ(0).is_some_and(|t| t.is_str()) => {
				let (place, borrowed) = self.borrowed_place(&args[0].node, span)?;
				if let Some(local) = dest.as_local()
					&& self.defs[local] == 1
					&& matches!(self.slots[local], Slot::Var(_))
				{
					self.slots[local] = Slot::Alias(Some(place));
					self.reborrows.extend(borrowed.map(|b| (local, b)));
					return Some(None);
				}
				done(place, true)
			}
			("Option", "take") => {
				let place = self.borrowed_place(&args[0].node, span)?;
				done(self.swap_out(place, Expr::undefined(), "taken", span, out), true)
			}
			// Indexing, whose result a `&mut` reads in place.
			("Index" | "IndexMut", "index" | "index_mut") => self.index(&name, gargs, args, dest, span, out),
			// A `&mut` to a value JS copies, such as through a `MutexGuard`, reads and writes
			// the place it borrows, as an index does.
			("DerefMut", "deref_mut")
				if let ty::Ref(_, pointee, _) = dest.ty(&self.mir.local_decls, tcx).ty.kind()
					&& self.b.t.object_kind(*pointee).is_none()
					&& let Some(local) = dest.as_local()
					&& self.defs[local] == 1
					&& matches!(self.slots[local], Slot::Var(_)) =>
			{
				let result = dest.ty(&self.mir.local_decls, tcx).ty;
				let place = self.call_def(def_id, gargs, args, result, span, out);
				if !place.is_path() {
					return done(place, true);
				}
				self.materialize(&place, out);
				self.slots[local] = Slot::Alias(Some(place));
				Some(None)
			}
			// Of bytes only: JS's `copyWithin` would share a `Copy` aggregate Rust copies.
			("slice", "copy_within") if targ(0).is_some_and(|t| matches!(t.kind(), ty::Uint(ty::UintTy::U8))) => {
				let s = self.operand(&args[0].node, span, out);
				let s = self.path(s, "slice", out);
				let (start, end) = self.range_bounds(&args[1].node, targ(1)?, span, out)?;
				let end = end.unwrap_or_else(|| s.clone().member("length"));
				let dest = self.operand(&args[2].node, span, out);
				done(Expr::rt("byteSliceCopyWithin", vec![s, start, end, dest]), true)
			}
			// An integer from an array of bytes made in place: its bytes shifted, no array made.
			(int, "from_be_bytes") if let Some(int) = int_named(int) => {
				let parts = match int.big() {
					true => None,
					// Bytes through a reference are read where they are.
					false if let Some(slice) = self.bytes_source(&args[0].node, span, out) => {
						let n = u64::from(int.bits / 8);
						Some(self.bytes_at(slice, n, out))
					}
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
				// An array of bytes comes by reference, which is a slice of them.
				let ty = match targ(0)? {
					ty if crate::lower::byte_array(ty) => Ty::new_slice(tcx, tcx.types.u8),
					ty => ty,
				};
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
				if matches!(targ(0)?.kind(), ty::Adt(adt, _) if self.b.t.own(adt.did()))
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
				// What a `Result` residual goes into: a `Result`, or one that is ready, which
				// is the same value.
				let result = match (self.b.t.std_of(to), to.kind()) {
					(Some(Std::Result), _) => Some(to),
					(Some(Std::Poll), ty::Adt(_, poll_args)) => {
						let inner = poll_args.type_at(0);
						(self.b.t.std_of(inner) == Some(Std::Result)).then_some(inner)
					}
					_ => None,
				};
				match (self.b.t.std_of(to), result.map(|r| r.kind()), residual.kind()) {
					(Some(Std::Option), ..) => done(Expr::undefined(), false),
					(_, Some(ty::Adt(_, to_args)), ty::Adt(_, from_args))
						if self.b.t.std_of(residual) == Some(Std::Result) =>
					{
						let (from, to) = (from_args.type_at(1), to_args.type_at(1));
						if from == to {
							return done(value, false);
						}
						let error = match value {
							// Made in place: its error.
							Expr::Call(f, mut items)
								if matches!(&*f, Expr::Rt(n) if n == "Err") && items.len() == 1 =>
							{
								items.pop().expect("checked above")
							}
							value => {
								let value = self.path(value, "residual", out);
								// The residual is an `Err`.
								let err = crate::ts::Type::Object(vec![
									("ok".into(), crate::ts::Type::Builtin("false")),
									("error".into(), self.b.t.ty(from, span)),
								]);
								Expr::As(Box::new(value), err).member("error")
							}
						};
						let converted = self.b.convert(error, from, to, span, out);
						done(Expr::result(false, converted), true)
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

	/// Whether a call returns the `rt.Place` of a value a map holds that JS copies, or an
	/// `Option` of one: a map's mapping, or `expect` or `unwrap` of such an `Option`.
	fn place_result(
		&self,
		def_id: rustc_hir::def_id::DefId,
		gargs: GenericArgsRef<'tcx>,
		args: &[Spanned<Operand<'tcx>>],
		span: Span,
	) -> bool {
		let call = library::Call {
			def_id,
			args: self.b.subst(gargs),
			span,
		};
		if crate::lower::collections::place_call(self.b, &call) {
			return true;
		}
		let unwrap = library::key(self.tcx, def_id)
			.is_some_and(|(o, n)| o == "Option" && matches!(n.as_str(), "expect" | "unwrap"));
		unwrap
			&& matches!(args.first().map(|a| &a.node), Some(Operand::Move(p) | Operand::Copy(p))
				if p.as_local().is_some_and(|l| self.place_opts.contains(&l)))
	}

	/// The value of a call that returns a place, for its destination: a reference, which
	/// reads and writes through the place held in a constant, and `None` once it is one; an
	/// `Option` only tested and taken apart; or else a refusal, since no other code takes a
	/// place for a reference.
	fn hold_place(&mut self, dest: Place<'tcx>, value: Expr, span: Span, out: &mut Vec<Stmt>) -> Option<Expr> {
		let ty = dest.ty(&self.mir.local_decls, self.tcx).ty;
		match dest.as_local() {
			Some(local) if self.uses[local] == 0 => Some(value),
			Some(local)
				if matches!(ty.kind(), ty::Ref(..))
					&& self.defs[local] == 1
					&& matches!(self.slots[local], Slot::Var(_)) =>
			{
				let place = self.path(value, "place", out);
				self.slots[local] = Slot::Alias(Some(place.member("v")));
				None
			}
			Some(local) if self.place_opts.contains(&local) => Some(value),
			_ => Some(self.err(
				span,
				format!("`{ty}`, a mutable reference to a value a map holds, kept other than to read and write it"),
			)),
		}
	}

	/// The place a `&mut` operand borrows, as an assignable path, and through the
	/// reference, which a store resolves: an alias of a borrowed place, not a copy of a
	/// reference, which holds the object it points at.
	fn borrowed_place(&mut self, op: &Operand<'tcx>, span: Span) -> Option<Borrowed<'tcx>> {
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
				Some((target.clone(), Some(self.tcx.mk_place_deref(*place))))
			}
			_ => {
				let pointee = self.mir.local_decls[place.local].ty.peel_refs();
				let err = self.err(span, format!("replacing the value behind a reference to `{pointee}`"));
				Some((err, None))
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
		let range = match recv {
			Expr::Object(_) => recv,
			recv => self.path(recv, "range", out),
		};
		let start = super::step::stepped_field(&range, "start");
		let end = super::step::stepped_field(&range, "end");
		let flag = self.b.fresh(&format!("{name}Some"));
		self.range_flags.insert(name.clone(), flag.clone());
		if stepped {
			let step = Expr::Postfix(Box::new(start), "++");
			let item = match read {
				true => Expr::ident(&name).assign(step),
				false => step,
			};
			return Some((flag, item.binary("<", end)));
		}
		let more = start.clone().binary("<", end);
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
	fn swap_out(&mut self, place: Borrowed<'tcx>, value: Expr, base: &str, span: Span, out: &mut Vec<Stmt>) -> Expr {
		let name = self.b.fresh(base);
		out.push(StmtKind::decl(false, name.clone(), None, Some(place.0.clone())).at(None));
		self.set_borrowed(&place, value, span, out);
		Expr::ident(name)
	}

	/// Set what a `&mut` borrows to `value`.
	fn set_borrowed(&mut self, (read, place): &Borrowed<'tcx>, value: Expr, span: Span, out: &mut Vec<Stmt>) {
		match place {
			Some(place) => out.extend(self.store(*place, value, span)),
			None => out.push(StmtKind::Expr(read.clone().assign(value)).at(None)),
		}
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
		// A string slices by UTF-8 byte offsets, which a JS string doesn't index by.
		if container.is_str() || self.b.t.std_of(container) == Some(Std::String) {
			if name == "index_mut" {
				return None;
			}
			let base = self.operand(&args[0].node, span, out);
			let (start, end) = self.range_bounds(&args[1].node, index_ty, span, out)?;
			let mut values = vec![self.path(base, "s", out), start];
			values.extend(end);
			return Some(Some((Expr::rt("strSlice", values), true)));
		}
		let elem_ty = elem(self.tcx, container)?;
		let bytes = matches!(elem_ty.kind(), ty::Uint(ty::UintTy::U8));
		let base = self.operand(&args[0].node, span, out);
		match Int::of(index_ty) {
			Some(_) => {
				// A `Vec`'s element is its storage's, a byte vector's in `data`.
				let base = self.path(base, "slice", out);
				let i = self.operand(&args[1].node, span, out);
				let i = self.path(i, "index", out);
				let checked = Expr::rt("idx", vec![base.clone(), i]);
				let checked = self.path(checked, "at", out);
				let place = match container.kind() {
					ty::Adt(..) if bytes => base.member("data").index(checked),
					// An array's bytes come through a reference, which is a slice of them.
					_ if super::place::byte_slice(container) || crate::lower::byte_array(container) => {
						super::place::byte_at(base, checked)
					}
					_ => base.index(checked),
				};
				// A reference to an array of bytes is a slice of them.
				if crate::lower::byte_array(elem_ty) {
					return Some(Some((Expr::rt("byteSliceFrom", vec![place]), true)));
				}
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
				let base = match container.kind() {
					ty::Adt(..) => library::vec_view(base, container),
					_ => base,
				};
				let base = self.path(base, "slice", out);
				let mut values = vec![base, start];
				values.extend(end);
				let range = match bytes {
					true => "byteSliceRange",
					false => "sliceRange",
				};
				Some(Some((Expr::rt(range, values), true)))
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

	/// What a `DebugList` iterates, as an iterable JS value, which drops nothing it reads.
	fn items(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> Option<Expr> {
		let tcx = self.tcx;
		if self.b.t.drops(ty) {
			return Some(self.err(
				span,
				format!("`DebugList::entries` of `{ty}`, which drops, which is not supported yet"),
			));
		}
		match ty.peel_refs().kind() {
			ty::Slice(_) | ty::Array(..) => Some(library::elements(value, ty)),
			ty::Adt(adt, _) if Std::of(tcx, adt.did()) == Some(Std::Vec) => Some(library::elements(value, ty)),
			_ => crate::lower::iter::into_iter(self.b, ty, value, span),
		}
	}
}

/// Whether `ty` holds a `[u8; N]` other than behind a reference or in a closure, which std
/// only calls.
fn owns_bytes(ty: Ty<'_>) -> bool {
	match ty.kind() {
		_ if crate::lower::byte_array(ty) => true,
		ty::Array(elem, _) | ty::Slice(elem) => owns_bytes(*elem),
		ty::Tuple(tys) => tys.iter().any(owns_bytes),
		ty::Adt(_, args) | ty::Alias(_, ty::AliasTy { args, .. }) => args.types().any(owns_bytes),
		_ => false,
	}
}

/// Whether `ty` holds a `&[u8; N]`, as a closure's signature does if it takes one.
fn holds_bytes_ref(ty: Ty<'_>) -> bool {
	ty.walk().any(|arg| arg.as_type().is_some_and(crate::lower::bytes_ref))
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

/// How a `ControlFlow` type is represented.
fn flow<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Flow {
	match ty.kind() {
		ty::Adt(_, args) => Flow::of(tcx, args),
		_ => unreachable!("checked by the caller"),
	}
}

/// The test that `scrut`, a `ControlFlow`, is `Continue` (variant 0) or `Break`.
pub fn control_flow_test<'tcx>(fx: &mut Fx<'_, '_, '_, 'tcx>, scrut: &Expr, ty: Ty<'tcx>, variant: VariantIdx) -> Expr {
	let continues = variant.index() == 0;
	if matches!(scrut, Expr::Ident(n) if fx.next_names.contains(n)) {
		return match continues {
			true => scrut.clone().binary("!==", Expr::Rt("DONE".into())),
			false => scrut.clone().binary("===", Expr::Rt("DONE".into())),
		};
	}
	flow(fx.tcx, ty).test(scrut.clone(), continues)
}

/// A field of a `ControlFlow`: `Continue`'s value, or `Break`'s, which for `?` is the
/// value it was applied to.
pub fn control_flow_field<'tcx>(
	fx: &mut Fx<'_, '_, '_, 'tcx>,
	base: Expr,
	ty: Ty<'tcx>,
	variant: Option<VariantIdx>,
) -> Expr {
	let continues = variant.is_none_or(|v| v.index() == 0);
	match (flow(fx.tcx, ty), continues) {
		(Flow::Result | Flow::Other, true) => base.member("value"),
		(Flow::Result, false) => base,
		(Flow::Other, false) => base.member("error"),
		(Flow::Option, true) => base,
		(Flow::Option, false) => Expr::undefined(),
	}
}

/// A `ControlFlow` made in place.
pub fn control_flow<'tcx>(
	fx: &mut Fx<'_, '_, '_, 'tcx>,
	ty: Ty<'tcx>,
	variant: VariantIdx,
	mut values: Vec<Expr>,
) -> Expr {
	let value = values.pop().unwrap_or_else(Expr::undefined);
	match (flow(fx.tcx, ty), variant.index()) {
		(Flow::Result | Flow::Other, 0) => Expr::result(true, value),
		(Flow::Result, _) => value,
		(Flow::Other, _) => Expr::result(false, value),
		(Flow::Option, 0) => value,
		(Flow::Option, _) => Expr::undefined(),
	}
}
