//! Patterns: a test of whether a value matches, and the bindings it makes.
//!
//! Both read the scrutinee through a path, a name or member chain that is free to
//! evaluate more than once.

use rustc_middle::thir::{Pat, PatKind, PatRangeBoundary};
use rustc_middle::ty::{self, Ty};

use super::AdtRepr;
use super::body::Body;
use super::library::Std;
use super::num::Int;
use crate::ts::{Expr, Stmt, StmtKind};

impl<'b, 'a, 'tcx> Body<'b, 'a, 'tcx> {
	/// The test `scrut` must pass to match `pat`, or `None` if it always matches.
	pub fn pat_test(&mut self, pat: &Pat<'tcx>, scrut: &Expr) -> Option<Expr> {
		let ty = self.subst(pat.ty);
		match &pat.kind {
			PatKind::Wild | PatKind::Missing => None,
			PatKind::Binding { subpattern, .. } => subpattern.as_ref().and_then(|p| self.pat_test(p, scrut)),
			PatKind::Deref { subpattern, .. } => self.pat_test(subpattern, scrut),
			PatKind::Leaf { subpatterns } => {
				let tests: Vec<Expr> = subpatterns
					.iter()
					.filter_map(|f| {
						let field = self.project(scrut.clone(), ty, rustc_abi::FIRST_VARIANT, f.field, pat.span);
						self.pat_test(&f.pattern, &field)
					})
					.collect();
				all(tests)
			}
			PatKind::Variant {
				adt_def,
				variant_index,
				subpatterns,
				..
			} => {
				let mut tests = Vec::new();
				match Std::of(self.t.tcx, adt_def.did()) {
					Some(Std::Option) => tests.push(match variant_index.index() {
						0 => scrut.clone().binary("===", Expr::undefined()),
						_ => scrut.clone().binary("!==", Expr::undefined()),
					}),
					Some(Std::Result) => tests.push(match variant_index.index() {
						0 => scrut.clone().member("ok"),
						_ => scrut.clone().member("ok").not(),
					}),
					None if adt_def.did().is_local() => {
						let info = self.t.adt(*adt_def, pat.span);
						let name = Expr::Str(adt_def.variant(*variant_index).name.to_string());
						match info.repr {
							AdtRepr::Tags => tests.push(scrut.clone().binary("===", name)),
							AdtRepr::Union => tests.push(scrut.clone().member("tag").binary("===", name)),
							AdtRepr::Class { .. } => {}
						}
					}
					_ => {
						self.t.err(pat.span, format!("a pattern on `{ty}`"));
						return None;
					}
				}
				for f in subpatterns {
					let field = self.project(scrut.clone(), ty, *variant_index, f.field, pat.span);
					if let Some(test) = self.pat_test(&f.pattern, &field) {
						tests.push(test);
					}
				}
				all(tests)
			}
			PatKind::Constant { value } => {
				let constant = self.value(value.ty, value.valtree, pat.span);
				match (&constant, ty.peel_refs().kind()) {
					(Expr::Lit(t), ty::Bool) if t == "true" => Some(scrut.clone()),
					(Expr::Lit(t), ty::Bool) if t == "false" => Some(scrut.clone().not()),
					(_, ty::Int(_) | ty::Uint(_)) => {
						Some(Int::of(ty.peel_refs()).unwrap().compare("===", scrut.clone(), constant))
					}
					(_, ty::Bool | ty::Str) => Some(scrut.clone().binary("===", constant)),
					_ => {
						self.t.err(pat.span, format!("a constant pattern of type `{ty}`"));
						None
					}
				}
			}
			PatKind::Range(range) => {
				let mut tests = Vec::new();
				let compare = |op: &'static str, l: Expr, r: Expr| match Int::of(range.ty) {
					Some(int) => int.compare(op, l, r),
					None => l.binary(op, r),
				};
				if let PatRangeBoundary::Finite(lo) = range.lo {
					let lo = self.value(range.ty, lo, pat.span);
					tests.push(compare("<=", lo, scrut.clone()));
				}
				if let PatRangeBoundary::Finite(hi) = range.hi {
					let hi = self.value(range.ty, hi, pat.span);
					let op = match range.end {
						rustc_hir::RangeEnd::Included => "<=",
						rustc_hir::RangeEnd::Excluded => "<",
					};
					tests.push(compare(op, scrut.clone(), hi));
				}
				all(tests)
			}
			PatKind::Slice { prefix, slice, suffix } | PatKind::Array { prefix, slice, suffix } => {
				let mut tests = Vec::new();
				let fixed = matches!(pat.kind, PatKind::Array { .. });
				let n = prefix.len() + suffix.len();
				if !fixed {
					let op = if slice.is_some() { ">=" } else { "===" };
					tests.push(scrut.clone().member("length").binary(op, Expr::lit(n.to_string())));
				}
				for (i, p) in prefix.iter().enumerate() {
					let elem = scrut.clone().index(Expr::lit(i.to_string()));
					if let Some(test) = self.pat_test(p, &elem) {
						tests.push(test);
					}
				}
				for (i, p) in suffix.iter().enumerate() {
					let elem = self.suffix_elem(scrut, ty, suffix.len() - i);
					if let Some(test) = self.pat_test(p, &elem) {
						tests.push(test);
					}
				}
				if let Some(p) = slice
					&& let Some(_) = self.pat_test(p, scrut)
				{
					self.t.err(p.span, "a refutable pattern on a slice's rest");
				}
				all(tests)
			}
			PatKind::Or { pats } => {
				let mut tests = Vec::new();
				for p in pats.iter() {
					if super::body::has_bindings(p) {
						self.t.err(pat.span, "an or-pattern with bindings");
						return None;
					}
					tests.push(self.pat_test(p, scrut)?);
				}
				tests.into_iter().reduce(|a, b| a.binary("||", b))
			}
			_ => {
				self.t.err(pat.span, "unsupported pattern");
				None
			}
		}
	}

	/// The element `from_end` places before the end of the slice `scrut`.
	fn suffix_elem(&mut self, scrut: &Expr, ty: Ty<'tcx>, from_end: usize) -> Expr {
		match ty.peel_refs().kind() {
			ty::Array(_, len) if let Some(len) = len.try_to_target_usize(self.t.tcx) => {
				scrut.clone().index(Expr::lit((len as usize - from_end).to_string()))
			}
			_ => scrut.clone().index(
				scrut
					.clone()
					.member("length")
					.binary("-", Expr::lit(from_end.to_string())),
			),
		}
	}

	/// Declare the bindings `pat` makes from `scrut`. An `owned` scrutinee is a
	/// temporary, so a `Copy` value binds without a copy.
	pub fn pat_bind(&mut self, pat: &Pat<'tcx>, scrut: &Expr, owned: bool, out: &mut Vec<Stmt>) {
		let ty = self.subst(pat.ty);
		match &pat.kind {
			PatKind::Wild | PatKind::Missing | PatKind::Constant { .. } | PatKind::Range(_) => {}
			// Bindings in alternatives are refused by `pat_test`; binding the first
			// alternative's keeps that one error from cascading.
			PatKind::Or { pats } => self.pat_bind(&pats[0], scrut, owned, out),
			PatKind::Binding {
				name,
				var,
				mode,
				subpattern,
				..
			} => {
				let by_value = mode.0 == rustc_hir::ByRef::No;
				if !by_value {
					self.ref_binding(mode.0, ty, pat.span);
				}
				// Binding has no effect in JS, and TypeScript refuses an unread local.
				if subpattern.is_none() && !self.is_read(*var) {
					return;
				}
				let mutable = mode.1 == rustc_ast::Mutability::Mut;
				let value = match by_value && !owned {
					true => self.copied(scrut.clone(), ty),
					false => scrut.clone(),
				};
				let name = self.bind_var(*var, *name, mutable, ty, pat.span);
				let annotate = mutable.then(|| self.t.ty(ty, pat.span));
				out.push(StmtKind::decl(mutable, name, annotate, Some(value)).at(None));
				if let Some(sub) = subpattern {
					self.pat_bind(sub, scrut, owned, out);
				}
			}
			PatKind::Deref { subpattern, .. } => self.pat_bind(subpattern, scrut, false, out),
			PatKind::Leaf { subpatterns } => {
				for f in subpatterns {
					let field = self.project(scrut.clone(), ty, rustc_abi::FIRST_VARIANT, f.field, pat.span);
					self.pat_bind(&f.pattern, &field, owned, out);
				}
			}
			PatKind::Variant {
				variant_index,
				subpatterns,
				..
			} => {
				// A payload changed in place would change every copy of a `Copy` enum,
				// which JS shares.
				let copy_enum = self.t.is_copy(ty.peel_refs());
				for f in subpatterns {
					if copy_enum && binds_mut_ref(&f.pattern) {
						self.t.err(f.pattern.span, "a `ref mut` binding into a `Copy` enum");
					}
					let field = self.project(scrut.clone(), ty, *variant_index, f.field, pat.span);
					self.pat_bind(&f.pattern, &field, owned, out);
				}
			}
			PatKind::Slice { prefix, slice, suffix } | PatKind::Array { prefix, slice, suffix } => {
				for (i, p) in prefix.iter().enumerate() {
					let elem = scrut.clone().index(Expr::lit(i.to_string()));
					self.pat_bind(p, &elem, owned, out);
				}
				for (i, p) in suffix.iter().enumerate() {
					let elem = self.suffix_elem(scrut, ty, suffix.len() - i);
					self.pat_bind(p, &elem, owned, out);
				}
				if let Some(p) = slice {
					let start = Expr::lit(prefix.len().to_string());
					let end = match ty.peel_refs().kind() {
						ty::Array(_, len) if let Some(len) = len.try_to_target_usize(self.t.tcx) => {
							Expr::lit((len as usize - suffix.len()).to_string())
						}
						_ => scrut
							.clone()
							.member("length")
							.binary("-", Expr::lit(suffix.len().to_string())),
					};
					let elem = match ty.peel_refs().kind() {
						ty::Array(elem, _) | ty::Slice(elem) => *elem,
						_ => {
							self.t.err(pat.span, format!("a slice pattern on `{ty}`"));
							return;
						}
					};
					let method = match elem.kind() {
						ty::Uint(ty::UintTy::U8) => "subarray",
						_ => "slice",
					};
					let rest = scrut.clone().member(method).call(vec![start, end]);
					self.pat_bind(p, &rest, true, out);
				}
			}
			_ => {
				self.t.err(pat.span, "unsupported pattern");
			}
		}
	}
}

fn all(tests: Vec<Expr>) -> Option<Expr> {
	tests.into_iter().reduce(Expr::and)
}

fn binds_mut_ref(pat: &Pat<'_>) -> bool {
	let mut found = false;
	pat.walk_always(|p| {
		if let PatKind::Binding { mode, .. } = p.kind
			&& let rustc_hir::ByRef::Yes(_, rustc_ast::Mutability::Mut) = mode.0
		{
			found = true;
		}
	});
	found
}
