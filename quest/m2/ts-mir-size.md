# [M] Smaller output from MIR

## Goal

[rust2-ts](/ts/README.md)'s bundles on `just ts bench` are no larger than the THIR
lowering's were, without giving up the speed MIR gained: the codec's at most 30629
bytes minified and 9422 gzipped, and the async handoff's at most 10949 and 3458.

## Plan

The codec's bundle is 29996 bytes minified and 9545 gzipped (+1.3% gzipped), and the
async handoff's 15119 and 3756 (+38%, +8.6%), against THIR's numbers above, built from
the same fixtures. Each candidate is measured on the bench's sizes and timings before
it lands:

- A coroutine's class, about 330 bytes minified each against about 140 for a THIR
  generator, for the drop, the checks for a resume after it finished, and the
  `try`/`catch` that poisons it and drops what it holds on a panic, where a THIR
  generator had a `finally` and the runtime's one `Async` class: a base class in the
  runtime for that scaffolding, or one field for saved locals that are never live at
  once (the layout's `storage_conflicts`), each timed on V8 and JSC, since both change
  what a field holds. Generators and closures stay out: they measured 1.4-5.5x and
  1.3-1.8x slower on the handoff.
- The handoff's code outside the classes, about 140 bytes gzipped over THIR's.
- The codec's `fill`, `encodeFrames`, and `decodeSplit`, whose blocks nest deeper than
  THIR's loops did.

Public API: the translator's output, which changes shape but not behavior.
