# [S] Smaller output from MIR

## Goal

[rust2-ts](/ts/README.md)'s bundles on `just ts bench` are no larger than the THIR
lowering's were, without giving up the speed MIR gained: the codec's at most 30629
bytes minified and 9422 gzipped, and the async handoff's at most 3458 gzipped.

## Plan

[#95](https://github.com/kixelated/rust2/pull/95) brought the handoff to 3363 bytes
gzipped and the codec to 30094 minified. The codec's 9651 gzipped is what remains,
229 bytes over.

That gap is the runtime, not the lowering: [#89](https://github.com/kixelated/rust2/pull/89)
grew `rt/int.ts` after THIR's numbers were taken, and THIR's own output bundled with
today's `rt/int.ts` is 31112 bytes minified and 9666 gzipped, over MIR's. Closing it
means trimming what the codec pulls from `rt/int.ts`, without losing #89's speed.

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
