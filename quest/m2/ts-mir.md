# [L] Lower bodies from MIR

## Goal

[rust2-ts](/ts/README.md) lowers every body from MIR, as the
[MIR spike](https://github.com/kixelated/rust2/pull/36) decided: `--lower` goes,
the THIR lowering (`src/lower/body.rs`, `pat.rs`, the drop scopes in `drop.rs`, and
`rt.future`'s generators) is deleted, and every fixture passes its snapshot,
differential, unwinding, and census tests from MIR.

## Plan

The spike's prototype (`src/lower/mir/`) already passes the codec, lang, and task
fixtures' differential tests and the task fixture's unwinding test. What the move
still needs:

- The reject fixture and its census, rewritten to what MIR refuses (`reject_mir`
  already checks MIR's errors), and snapshots of MIR's output. Constructs THIR refuses that MIR lowers move into the differential
  fixtures, such as a `move` closure taking a reference from a variable assigned again
  after (`a closure that copies a mutable variable`), which the prototype snapshots.
- The census to at least THIR's coverage of moq-net and kio. MIR types each
  intermediate it holds, so a gap in the library or model shows up once more per
  temporary; read the census by construct, not by total. MIR-only refusals to close:
  `?` on `Poll<Result<..>>` (284, which THIR refused as 235 too), `ControlFlow` other
  than from `?`, switches on and fields of types without a mapping, and a reference
  copied out of a field and then passed on.
- `?` on an `Option` returned through a `next` whose item can be `undefined`, and the
  other THIR special cases the prototype mirrors (`for` loops by the iterator protocol,
  `from_be_bytes` of bytes made in place, ranges stepped in place): keep each only
  where `just ts bench` measures it.
- Size: the task fixture's bundle is 35% larger gzipped than THIR's, the codec's 9%.
  Likely wins, each to measure: a coroutine resume function structured where the
  graph allows (a dispatch at each loop header the resume enters, rather than a
  `switch` over every block), temporaries folded through reads of borrowed places, and
  `_0 = v; return _0` returned in place.
- Source maps: MIR spans point at calls and asserts; check stack traces read well.
- Panics unwind (`-C panic=unwind`), as decided in
  [#36](https://github.com/kixelated/rust2/pull/36) at a measured 4-6% on V8's async
  handoff; `RUST2_TS_ABORT`, kept only to measure aborting, goes with `--lower`.

Watch what the prototype's comments say it relies on: MIR's single-assignment
temporaries, `Box` derefs elaborated into `BoxDerefTransmute`, and drop shims that
take a raw pointer. A nightly bump that changes them should fail loudly, as each does
now.

Public API: the translator's whole output, and the `--lower` option, which goes.

## Required

- [MIR's codec as fast as THIR's](/quest/m2/ts-mir-codec.md) - the codec from MIR no slower than from THIR in any engine
