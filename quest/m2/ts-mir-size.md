# [S] Smaller output from MIR

## Goal

[rust2-ts](/ts/README.md)'s bundles on `just ts bench` are no larger than the THIR
lowering's were, without giving up the speed MIR gained: the codec's at most 30629
bytes minified and 9422 gzipped, and the async handoff's at most 3458 gzipped.

## Plan

[#95](https://github.com/kixelated/rust2/pull/95) brought the handoff to 3423 bytes
gzipped. The codec is what remains: 30935 bytes minified and 9898 gzipped, 306 and 476
over.

That gap came after THIR's numbers were taken, not from the lowering. On the same
fixtures with #95, these PRs added to the codec:

- [#89](https://github.com/kixelated/rust2/pull/89) grew `rt/int.ts`. THIR's own output
  bundled with #95's `rt/int.ts`, before #87, is 31112 bytes minified and 9666 gzipped,
  over MIR's 30094 and 9651.
- [#87](https://github.com/kixelated/rust2/pull/87)'s drop glue with
  [#98](https://github.com/kixelated/rust2/pull/98) added 253 bytes minified and 93
  gzipped.
- [#82](https://github.com/kixelated/rust2/pull/82)'s `rt.ByteSlice` added 580 bytes
  minified and 156 gzipped.

Closing the gap means trimming what the codec pulls from the runtime or its drop glue,
without losing speed, or restating the goal against today's runtime.

Small cuts measured by hand on the handoff and left out, each under 1% gzipped:
hoisting a dispatch's entry case out of its `switch` (3-7 bytes), dropping `unwind`
stores before a statement that can't throw (4), and merging `if` arms whose cleanup is
the same (about 15).

Tried and dropped:

- A runtime base class for the scaffolding (the resume-after-finish checks and the
  poisoning `catch`), with `resume` per class so its calls stay monomorphic: prototyped
  by hand on the 17 one-await classes, 792 bytes smaller minified but 38 larger gzipped.
  A base can't read a subclass's `#state`, so only those two pieces move.
- Collapsing an `async fn` whose body only awaits another that only moves its arguments
  in (`level2` into `level1` and on): exact, but it removes the depth the handoff
  measures, and moq-net's wrappers await `kio::wait`, not such an `async fn`.

Decided with the maintainer on 2026-10-26, from
[#85](https://github.com/kixelated/rust2/pull/85): the handoff's target is gzipped
only. Its minified gap is the fixture's repeated one-await classes, which only a
shared driver (measured slower or larger) or collapsing the wrappers (which empties
the bench) would close.

Generators and closures stay out: they measured 1.4-5.5x and 1.3-1.8x slower on the
handoff. Coroutine fields stay private.

Public API: the translator's output, which changes shape but not behavior.
