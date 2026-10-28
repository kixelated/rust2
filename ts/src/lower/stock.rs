//! Well-known crates that translation maps itself, as it maps std, so a consumer binds
//! none of them: `bytes::Bytes`, which is the `rt.ByteSlice` a `&[u8]` is, and `rand`'s
//! thread-local generator, which draws from the platform's cryptographic source.
//! `tracing`'s events are left out where they are written, by
//! [`super::Translator::ignored_site`].

use rustc_hir::def::DefKind;
use rustc_hir::def_id::{CrateNum, DefId};
use rustc_middle::ty::{self, Ty, TyCtxt};

use super::body::Body;
use super::library::{Call, Std, key, string, targ};
use super::num::Int;
use crate::ts::{Expr, Stmt};

/// The type `did` is, if it is one of these crates' that has a representation of its own.
pub fn of(tcx: TyCtxt<'_>, did: DefId) -> Option<Std> {
	if !is_stock(tcx, did) {
		return None;
	}
	match (tcx.crate_name(did.krate).as_str(), tcx.opt_item_name(did)?.as_str()) {
		("bytes", "Bytes") => Some(Std::Bytes),
		("rand", "ThreadRng") => Some(Std::ThreadRng),
		_ => None,
	}
}

/// Whether `did` is an item of one of these crates, rather than of the crate translated,
/// whatever its name.
pub fn is_stock(tcx: TyCtxt<'_>, did: DefId) -> bool {
	!did.is_local() && is_stock_crate(tcx, did.krate)
}

/// Whether `krate`, a dependency, is one of these crates, which maps rather than translates.
pub fn is_stock_crate(tcx: TyCtxt<'_>, krate: CrateNum) -> bool {
	matches!(tcx.crate_name(krate).as_str(), "bytes" | "rand")
}

/// A call of one of these crates' functions, or of a std trait's on one of their types:
/// its mapping, `None` if it has none, or its arguments back if it is no such call.
pub fn call<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	call: &Call<'tcx>,
	args: Vec<Expr>,
	out: &mut Vec<Stmt>,
) -> Result<Option<Expr>, Vec<Expr>> {
	let tcx = b.t.tcx;
	let Some((owner, name)) = key(tcx, call.def_id) else {
		return Err(args);
	};
	let self_std = call.args.types().next().and_then(|t| b.t.std_of(t.peel_refs()));
	if is_stock(tcx, call.def_id) {
		return Ok(match tcx.crate_name(call.def_id.krate).as_str() {
			"bytes" => bytes(b, call, &name, args, out),
			_ => rand(b, call, &owner, &name, args, out),
		});
	}
	match (owner.as_str(), name.as_str(), self_std) {
		// A clone of either shares what it holds, and a `Bytes` is the slice it derefs to.
		("Clone", "clone", Some(Std::Bytes | Std::ThreadRng))
		| ("Deref", "deref", Some(Std::Bytes))
		| ("AsRef", "as_ref", Some(Std::Bytes))
		| ("Borrow", "borrow", Some(Std::Bytes)) => Ok(args.into_iter().next()),
		("Default", "default", Some(Std::Bytes)) => Ok(Some(Expr::rt("bytesNew", vec![]))),
		("Default", "default", Some(Std::ThreadRng)) => Ok(Some(Expr::rt("randRng", vec![]))),
		("PartialEq", "eq", Some(Std::Bytes)) => {
			let other = targ(call.args, 1).peel_refs();
			let bytes = b.t.std_of(other) == Some(Std::Bytes)
				|| matches!(other.kind(), ty::Slice(e) if matches!(e.kind(), ty::Uint(ty::UintTy::U8)));
			Ok(bytes.then(|| Expr::rt("byteSliceEq", args)))
		}
		("From", "from", _) => from(b, call, args),
		_ => Err(args),
	}
}

/// `From::from` into or out of a `Bytes`.
fn from<'tcx>(b: &mut Body<'_, '_, 'tcx>, call: &Call<'tcx>, args: Vec<Expr>) -> Result<Option<Expr>, Vec<Expr>> {
	let tcx = b.t.tcx;
	let (to, from) = (targ(call.args, 0), targ(call.args, 1));
	let byte_vec = |t: Ty<'tcx>| b.t.std_of(t) == Some(Std::Vec) && targ_is_u8(t);
	let f = match (b.t.std_of(to), b.t.std_of(from)) {
		(Some(Std::Bytes), _) if byte_vec(from) => "byteVecSlice",
		(Some(Std::Bytes), _) if string(tcx, from.peel_refs()) => "stringAsBytes",
		(Some(Std::Bytes), _) if is_bytes_ref(from) => return Ok(args.into_iter().next()),
		(Some(Std::Bytes), _) => return Ok(None),
		(_, Some(Std::Bytes)) if byte_vec(to) => "byteSliceToByteVec",
		(_, Some(Std::Bytes)) => return Ok(None),
		_ => return Err(args),
	};
	Ok(Some(Expr::rt(f, args)))
}

/// A function of `bytes`: `Bytes`'s own.
fn bytes<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	call: &Call<'tcx>,
	name: &str,
	args: Vec<Expr>,
	out: &mut Vec<Stmt>,
) -> Option<Expr> {
	let tcx = b.t.tcx;
	if super::library::inherent_std(tcx, call.def_id) != Some(Std::Bytes) {
		return None;
	}
	let mut args = args.into_iter();
	Some(match name {
		"new" => Expr::rt("bytesNew", vec![]),
		// A `&'static [u8]` never changes, so the slice is the `Bytes`.
		"from_static" => args.next()?,
		"copy_from_slice" => Expr::rt("bytesCopyFromSlice", vec![args.next()?]),
		"len" => args.next()?.member("length"),
		"is_empty" => args.next()?.member("length").binary("===", Expr::lit("0")),
		// The other ranges' bounds are read where the range is made, by the body's lowering.
		"slice" if b.t.std_of(targ(call.args, 0)) == Some(Std::RangeInclusive) => {
			Expr::rt("bytesSliceInclusive", vec![args.next()?, args.next()?])
		}
		// The owner is read once, as `bytes` reads it, and would drop with the last `Bytes`
		// that shares it, which no value here tracks.
		"from_owner" => {
			let owner = targ(call.args, 0);
			if b.t.drops(owner) {
				let msg = format!(
					"`Bytes::from_owner` of `{owner}`, which would drop with the last clone of what it makes, which is not supported"
				);
				return Some(b.err(call.span, msg));
			}
			let as_ref = trait_fn(tcx, rustc_span::sym::AsRef)?;
			let slice = Ty::new_slice(tcx, tcx.types.u8);
			b.call_def(
				as_ref,
				tcx.mk_args(&[owner.into(), slice.into()]),
				vec![args.next()?],
				call.span,
				out,
			)
		}
		_ => return None,
	})
}

/// A function of `rand`: `rng`, or a method of its `Rng` or `RngExt` on a `ThreadRng`.
fn rand<'tcx>(
	b: &mut Body<'_, '_, 'tcx>,
	call: &Call<'tcx>,
	owner: &str,
	name: &str,
	args: Vec<Expr>,
	out: &mut Vec<Stmt>,
) -> Option<Expr> {
	let tcx = b.t.tcx;
	if tcx.def_kind(call.def_id) == DefKind::Fn && matches!(name, "rng" | "thread_rng") {
		return Some(Expr::rt("randRng", vec![]));
	}
	if !matches!(owner, "Rng" | "RngExt") || b.t.std_of(targ(call.args, 0).peel_refs()) != Some(Std::ThreadRng) {
		return None;
	}
	// The generator holds no state, so only the arguments after it are read.
	let mut args = args.into_iter().skip(1);
	let ty = targ(call.args, 1);
	let width = |int: Int| (Expr::lit(int.bits.to_string()), Expr::lit(int.signed.to_string()));
	Some(match name {
		"random" if ty.is_bool() => Expr::rt("randBool", vec![]),
		"random" => {
			let int = Int::of(ty)?;
			let (bits, signed) = width(int);
			Expr::As(Box::new(Expr::rt("randInt", vec![bits, signed])), int.ty())
		}
		"random_range" => {
			let int = Int::of(ty)?;
			let (bits, signed) = width(int);
			let range = args.next()?;
			let value = match b.t.std_of(targ(call.args, 2))? {
				Std::Range => {
					let range = b.spill(range, "range", out);
					let (start, end) = (range.clone().member("start"), range.member("end"));
					Expr::rt("randRange", vec![start, end, bits, signed])
				}
				Std::RangeTo => Expr::rt("randRange", vec![int.lit(0), range.member("end"), bits, signed]),
				Std::RangeInclusive => Expr::rt("randRangeInclusive", vec![range, bits, signed]),
				_ => return None,
			};
			Expr::As(Box::new(value), int.ty())
		}
		_ => return None,
	})
}

/// The one function of the trait `item` names, such as `AsRef::as_ref`.
fn trait_fn(tcx: TyCtxt<'_>, item: rustc_span::Symbol) -> Option<DefId> {
	let tr = tcx.get_diagnostic_item(item)?;
	tcx.associated_items(tr)
		.in_definition_order()
		.find(|i| i.is_fn())
		.map(|i| i.def_id)
}

fn targ_is_u8(ty: Ty<'_>) -> bool {
	matches!(ty.kind(), ty::Adt(_, args) if args.types().next().is_some_and(|t| matches!(t.kind(), ty::Uint(ty::UintTy::U8))))
}

/// Whether `ty` is `&[u8]`.
fn is_bytes_ref(ty: Ty<'_>) -> bool {
	matches!(ty.kind(), ty::Ref(_, inner, _) if matches!(inner.kind(), ty::Slice(e) if matches!(e.kind(), ty::Uint(ty::UintTy::U8))))
}
