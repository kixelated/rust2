# [XL] Exporter: moq's bindings from its crates' pub API

## Goal

[rust2-ffi](/ffi/README.md) exports what moq's bindings need from moq's own crates, with
no attributes and no hand-written wrapper beyond what the census shows is genuinely new
API, and generates the languages moq ships on its C ABI. moq-ffi shrinks to that remainder.

## Plan

From the spike ([kixelated/rust2#41](https://github.com/kixelated/rust2/pull/41)); split
into quests once the decision lands, ranked by what the census refuses:

- **Rules the census asks for**: tuples as out parameters (`origin::Producer::new`, so an
  origin can be made), `Path<'a>` exported as `Path<'static>`, a clone for a borrowed
  return of a `Clone` object (the 21 `info()` getters), `Range<u64>` and `RangeBounds`,
  and a policy for `poll_*` methods: 39 of 46 have an async twin, which may make them
  redundant, or the poll protocol may want them in place of the twin.
- **Several crates**: moq-native's sessions and the media crates are where moq-ffi's
  remaining API lives. Export each with its own census, naming another export's types
  across crates, as moq-pattern's re-exports already need.
- **Driving**: sessions and origins run a `Driver` polled with a time and a `kio::Waiter`.
  Decide between exporting that poll, a generated thread that drives it, or moq-native's
  runtime helpers.
- **A contract symbol** per build, so a header and a library from different builds refuse
  to load together.
- **Generators**: the IR holds what each language needs (paths, docs, receivers, exact
  borrow forms). Start with the language whose binding costs moq the most today, measured
  on the [binding benchmark](/quest/m1/ffi-bench.md).

## Required

- [Waiting: the exporter decision](/quest/m1/ffi-export-decision.md) - whether the exporter replaces the UniFFI fork
