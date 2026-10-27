//! Iterators stepped in place: a slice's or a `Vec`'s iterator, or a range of numbers,
//! that only `next` reads, as a `for` loop's is, is two variables, of the array and an
//! index or of the bounds, rather than an `rt.Iter`, which a bundler can't shake its
//! adaptors from. Variables, not an object of both: V8 took up to 1.25x as long over a
//! `BTreeMap` stepping an object's fields.

use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_middle::mir::{self, Local, Operand, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty::{self, Ty, TyCtxt};

use super::Fx;
use crate::lower::library::{self, Std};
use crate::ts::{Expr, Key, Stmt, StmtKind, Type};

/// The item type of a slice's or a `Vec`'s iterator, if `ty` is one.
fn array_iter<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
	let ty::Adt(adt, args) = ty.kind() else {
		return None;
	};
	let path = library::std_path(tcx, adt.did());
	let path: Vec<&str> = path.iter().map(String::as_str).collect();
	match path.as_slice() {
		["slice", "iter", "Iter" | "IterMut"] | ["vec", "into_iter", "IntoIter"] => args.types().next(),
		_ => None,
	}
}

/// Whether `ty` is what such an iterator is made from: a slice, an array, or a `Vec`.
fn array<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
	match ty.peel_refs().kind() {
		ty::Slice(_) | ty::Array(..) => true,
		ty::Adt(adt, _) => Std::of(tcx, adt.did()) == Some(Std::Vec),
		_ => false,
	}
}

impl<'tcx> Fx<'_, '_, '_, 'tcx> {
	/// Find the iterators to step in place: each made by a call from an array, or moved
	/// from another, and otherwise only borrowed for `next` into a protocol local and
	/// dropped. Each borrow is read only as `next`'s receiver.
	pub(super) fn find_stepped(&mut self) {
		let mir = self.mir;
		let tcx = self.tcx;
		let mut iters: Vec<Local> = mir
			.local_decls
			.indices()
			.filter(|&l| !self.is_arg(l) && l != mir::RETURN_PLACE && self.defs[l] == 1)
			.filter(|&l| {
				let ty = mir.local_decls[l].ty;
				array_iter(tcx, ty).is_some() || super::small_range(tcx, ty)
			})
			.collect();
		if iters.is_empty() {
			return;
		}
		// What each reads that is allowed: borrows for `next`, drops, moves into another.
		let mut reads: FxHashMap<Local, u32> = FxHashMap::default();
		let mut made = FxHashSet::default();
		let mut moves: Vec<(Local, Local)> = Vec::new();
		let mut borrows: FxHashMap<Local, Local> = FxHashMap::default();
		let mut next_reads: FxHashMap<Local, u32> = FxHashMap::default();
		for data in mir.basic_blocks.iter() {
			for stmt in &data.statements {
				let StatementKind::Assign(assign) = &stmt.kind else {
					continue;
				};
				let (place, rvalue) = &**assign;
				match rvalue {
					Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, from) if from.as_local().is_some() => {
						let from = from.as_local().unwrap();
						*reads.entry(from).or_default() += 1;
						if let Some(to) = place.as_local() {
							borrows.insert(to, from);
						}
					}
					Rvalue::Use(Operand::Move(from), _) if from.as_local().is_some() => {
						let from = from.as_local().unwrap();
						*reads.entry(from).or_default() += 1;
						if let Some(to) = place.as_local() {
							moves.push((from, to));
						}
					}
					Rvalue::Aggregate(..) if let Some(to) = place.as_local() => {
						made.insert(to);
					}
					_ => {}
				}
			}
			match &data.terminator().kind {
				// Counted as `uses` counts it: only a drop JS sees.
				TerminatorKind::Drop { place, .. }
					if let Some(local) = place.as_local()
						&& self.b.t.drops(mir.local_decls[local].ty) =>
				{
					*reads.entry(local).or_default() += 1;
				}
				TerminatorKind::Call {
					func,
					args,
					destination,
					..
				} => {
					let ty::FnDef(def_id, _) = *func.ty(&mir.local_decls, tcx).kind() else {
						continue;
					};
					let maker = library::key(tcx, def_id).is_some_and(|(o, n)| {
						matches!(
							(o.as_str(), n.as_str()),
							("IntoIterator", "into_iter") | ("slice", "iter" | "iter_mut")
						)
					});
					if maker
						&& let Some(to) = destination.as_local()
						&& let [arg] = &**args
					{
						match &arg.node {
							// A range is its own iterator.
							Operand::Move(from)
								if let Some(from) = from.as_local()
									&& super::small_range(tcx, mir.local_decls[from].ty) =>
							{
								*reads.entry(from).or_default() += 1;
								moves.push((from, to));
							}
							op if array(tcx, op.ty(&mir.local_decls, tcx)) => {
								made.insert(to);
							}
							_ => {}
						}
					}
					if crate::lower::iter::is_next(tcx, def_id)
						&& destination.as_local().is_some_and(|l| self.next_results.contains(&l))
						&& let [arg] = &**args
						&& let Operand::Copy(p) | Operand::Move(p) = &arg.node
						&& let Some(recv) = p.as_local()
					{
						*next_reads.entry(recv).or_default() += 1;
					}
				}
				_ => {}
			}
		}
		// A borrow only `next` reads, assigned once and borrowed by nothing.
		let stepping = |fx: &Self, borrow: Local| {
			fx.defs[borrow] == 1
				&& !fx.borrowed[borrow]
				&& next_reads.get(&borrow).copied().unwrap_or(0) == fx.uses[borrow]
		};
		let mut ok: FxHashSet<Local> = iters.drain(..).collect();
		ok.retain(|&l| reads.get(&l).copied().unwrap_or(0) == self.uses[l]);
		ok.retain(|&l| made.contains(&l) || moves.iter().any(|&(_, to)| to == l));
		ok.retain(|&l| borrows.iter().all(|(&b, &from)| from != l || stepping(self, b)));
		// A move joins two iterators that must both step in place, or neither.
		loop {
			let before = ok.len();
			for &(from, to) in &moves {
				if ok.contains(&from) != ok.contains(&to) {
					ok.remove(&from);
					ok.remove(&to);
				}
			}
			if ok.len() == before {
				break;
			}
		}
		for (b, from) in borrows {
			if ok.contains(&from) {
				self.stepped_borrows.insert(b, from);
			}
		}
		// One moved into another and read nowhere else shares its variables: the other is
		// assigned only by the move, so neither is live while the other is.
		for &(from, to) in &moves {
			if ok.contains(&from) && self.uses[from] == 1 {
				self.stepped_into.insert(from, to);
			}
		}
		self.stepped = ok;
	}

	/// The local whose variables `local` holds its fields in: the last it is moved into.
	pub(super) fn stepped_root(&self, mut local: Local) -> Local {
		while let Some(&to) = self.stepped_into.get(&local) {
			local = to;
		}
		local
	}

	/// The array iterator a `next` receiver `op` borrows, if it steps in place. A range's
	/// `next` steps one in place anyway.
	pub(super) fn stepped_array(&self, op: &Operand<'tcx>) -> bool {
		let (Operand::Copy(p) | Operand::Move(p)) = op else {
			return false;
		};
		p.as_local()
			.and_then(|b| self.stepped_borrows.get(&b))
			.is_some_and(|&l| array_iter(self.tcx, self.mir.local_decls[l].ty).is_some())
	}

	/// The keys of `local`'s fields: its array and index, or its bounds.
	fn stepped_keys(&self, local: Local) -> [&'static str; 2] {
		match array_iter(self.tcx, self.mir.local_decls[local].ty) {
			Some(_) => ["s", "i"],
			None => ["start", "end"],
		}
	}

	/// Name the two variables `local` holds its fields in, unless it shares another's. A
	/// read of either counts as one of memory, since `next` changes them in place.
	pub(super) fn name_stepped(&mut self, local: Local, base: &str) {
		if self.stepped_root(local) != local {
			return;
		}
		let vars = self.stepped_keys(local).map(|key| {
			let mut suffix = key.to_string();
			suffix[..1].make_ascii_uppercase();
			let name = self.b.fresh(&format!("{base}{suffix}"));
			self.shared.insert(name.clone());
			name
		});
		self.stepped_vars.insert(local, vars);
	}

	/// Give each stepped local that shares its variables those of the local it shares.
	pub(super) fn share_stepped(&mut self) {
		for &local in &self.stepped {
			let root = self.stepped_root(local);
			if root != local {
				let vars = self.stepped_vars[&root].clone();
				self.stepped_vars.insert(local, vars);
			}
		}
	}

	/// `local` read whole, if it is stepped in place: an object of its variables, which
	/// [`Self::set_stepped`] and [`stepped_field`] take apart again.
	pub(super) fn stepped_value(&self, local: Local) -> Option<Expr> {
		let vars = self.stepped_vars.get(&local)?;
		let keys = self.stepped_keys(local);
		let fields = keys.into_iter().zip(vars);
		Some(Expr::Object(
			fields.map(|(k, v)| (Key::Name(k.into()), Expr::ident(v))).collect(),
		))
	}

	/// Assign `local`'s variables the fields of `value`, as [`Self::stepped_make`] made it.
	pub(super) fn set_stepped(&mut self, local: Local, value: Expr, out: &mut Vec<Stmt>) {
		let value = match value {
			Expr::Located(_, value) => *value,
			value => value,
		};
		// Anything else is refused already.
		let Expr::Object(fields) = value else {
			return;
		};
		self.assigned[local] = true;
		let vars = self.stepped_vars[&local].clone();
		for ((_, value), var) in fields.into_iter().zip(vars) {
			// A move from a local that shares the variables.
			if matches!(&value, Expr::Ident(name) if *name == var) {
				continue;
			}
			let stmt = StmtKind::Expr(Expr::ident(&var).assign(value)).at(None);
			self.emit(vec![stmt], Some(&var), false, out);
		}
	}

	/// An iterator stepped in place, from the `rt.Iter` its mapping makes: `{ s, i: 0 }`
	/// from `rt.iterSlice(s)`, and `{ start, end }` from `new rt.Range(start, end)`. A
	/// moved one is already made.
	pub(super) fn stepped_make(&mut self, value: Expr, span: rustc_span::Span) -> Expr {
		let rt = |f: &Expr, name: &str| matches!(f, Expr::Rt(n) if n == name);
		let object = |fields: [(&str, Expr); 2]| {
			Expr::Object(fields.into_iter().map(|(k, v)| (Key::Name(k.into()), v)).collect())
		};
		match value {
			Expr::Call(f, mut args) if rt(&f, "iterSlice") && args.len() == 1 => {
				object([("s", args.pop().unwrap()), ("i", Expr::lit("0"))])
			}
			Expr::New(f, args) if rt(&f, "Range") && args.len() == 2 => {
				let [start, end] = <[Expr; 2]>::try_from(args).expect("checked above");
				object([("start", start), ("end", end)])
			}
			Expr::Object(_) => value,
			// Refused already.
			Expr::Lit(lit) if lit == "undefined" => Expr::undefined(),
			_ => self.err(span, "an iterator stepped in place that its mapping didn't make"),
		}
	}

	/// The next item of `it`, stepped in place, or `rt.DONE`.
	pub(super) fn stepped_next(&self, it: &Expr) -> Expr {
		let s = stepped_field(it, "s");
		let i = stepped_field(it, "i");
		Expr::Cond(
			Box::new(i.clone().binary("<", s.clone().member("length"))),
			Box::new(s.index(Expr::Postfix(Box::new(i), "++"))),
			Box::new(Expr::Rt("DONE".into())),
		)
	}

	/// The types of a stepped local's variables: its array and index, or its bounds.
	pub(super) fn stepped_tys(&mut self, local: Local) -> [Type; 2] {
		let ty = self.mir.local_decls[local].ty;
		let span = self.local_span(local);
		match array_iter(self.tcx, ty) {
			// What `library::elements` makes of the slice: a view of bytes.
			Some(item) => [
				match item.kind() {
					ty::Uint(ty::UintTy::U8) => Type::Builtin("Uint8Array"),
					_ => self.b.t.held_ty(Ty::new_slice(self.tcx, item), span),
				},
				Type::Builtin("number"),
			],
			None => {
				let ty::Adt(_, args) = ty.kind() else {
					unreachable!("a range is a struct")
				};
				let bound = self.b.t.held_ty(args.type_at(0), span);
				[bound.clone(), bound]
			}
		}
	}
}

/// The field `key` of an iterator stepped in place: its variable, from the object
/// [`Fx::stepped_value`] reads it as.
pub(super) fn stepped_field(it: &Expr, key: &str) -> Expr {
	match it {
		Expr::Object(fields) => fields
			.iter()
			.find(|(k, _)| matches!(k, Key::Name(n) if n == key))
			.map(|(_, v)| v.clone())
			.expect("a stepped iterator's field"),
		it => it.clone().member(key),
	}
}
