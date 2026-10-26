# [M] Smaller output from MIR

## Goal

[rust2-ts](/ts/README.md)'s bundles on `just ts bench` are no larger than the THIR
lowering's were, without giving up the speed MIR gained: the codec's at most 27483
bytes minified and 8437 gzipped, and the async handoff's at most 10901 and 3446.

## Plan

At the move to MIR, the codec's bundle was 27750 bytes minified and 8876 gzipped (+1%,
+5%), and the async handoff's 18258 and 3993 (+67%, +16%), against THIR's numbers
above. Each candidate is measured on the bench's sizes and timings before it lands:

- A coroutine's class, about 520 bytes minified each against about 140 for a THIR
  generator: a base class in the runtime for the state and `poll` scaffolding, fields
  only for what lives across an `.await`, and a resume function structured where the
  graph allows (a dispatch at each loop header it enters) rather than a `switch` over
  every block. Generators stay out: they measured 1.4-5.5x slower on the handoff.
- Tuple temporaries a destructuring `const [a, b] = ..` would avoid, and the `let`
  list at the top of each function, where a local assigned once could be declared at
  its assignment.
- The codec's `fill`, `encodeFrames`, and `decodeSplit`, whose blocks nest deeper than
  THIR's loops did.

Public API: the translator's output, which changes shape but not behavior.
