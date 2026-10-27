# [S] Duration

## Goal

rust2-ts translates `std::time::Duration` with std's arithmetic, overflow panics, and
ordering, so moq-net's model and moq-clock make, compare, and convert durations, proven by
differential tests against native Rust.

## Plan

Found by the [model's](/quest/m2/ts-model/README.md) census on 2026-10-27: 214 refusals in
the model and 67 in moq-clock, including its constants (`Duration::ZERO`, `MAX`).

Decided with the maintainer on 2026-10-27: std's seconds (`u64`) and nanoseconds (`u32`),
so overflow, `checked_*`, and `Ord` match std, and core's MIR can lower its methods later.

- The clocks that read the time are [bindings](/ts/README.md#bindings).

Public API: the translator's output.
