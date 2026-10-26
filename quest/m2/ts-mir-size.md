# [M] Smaller output from MIR

## Goal

[rust2-ts](/ts/README.md)'s bundles on `just ts bench` are no larger than the THIR
lowering's were, without giving up the speed MIR gained: the codec's at most 30629
bytes minified and 9422 gzipped, and the async handoff's at most 10949 and 3458.

## Plan

The codec's bundle is 29852 bytes minified and 9503 gzipped (+0.9% gzipped), and the
async handoff's 15053 and 3712 (+37%, +7.3%), against THIR's numbers above, built from
the same fixtures. Each candidate is measured on the bench's sizes and timings before
it lands.

The handoff's two gaps have different sources:

- Minified, nearly all of it is the fixture's 16 one-await `levelN` classes, about 360
  bytes each against about 135 for a THIR generator and its wrapper. Gzip folds the
  copies: the 15 repeats cost 172 bytes gzipped, less than THIR's 201.
- Gzipped, the gap is the code outside the repeats: with one level each, MIR's bundle is
  284 bytes gzipped over THIR's. `drainFuture`'s `switch` dispatch, `recvFuture`, and
  `register` and `wake`, whose `for` loops are stepped `while` loops with temporaries,
  are where it is.

Tried and dropped:

- A runtime base class for the scaffolding (the resume-after-finish checks and the
  poisoning `catch`), with `resume` per class so its calls stay monomorphic: prototyped
  by hand on the 17 one-await classes, 792 bytes smaller minified but 38 larger gzipped.
  A base can't read a subclass's `#state`, so only those two pieces move.
- Collapsing an `async fn` whose body only awaits another that only moves its arguments
  in (`level2` into `level1` and on): exact, but it removes the depth the handoff
  measures, and moq-net's wrappers await `kio::wait`, not such an `async fn`.

Generators and closures stay out: they measured 1.4-5.5x and 1.3-1.8x slower on the
handoff. Coroutine fields stay private.

Public API: the translator's output, which changes shape but not behavior.
