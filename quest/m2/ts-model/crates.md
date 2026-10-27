# [L] Translate safe dependencies with the package

## Goal

[rust2-ts](/ts/README.md) translates the dependencies `ts.toml` names together with the
package, as one program: their types and calls translate as the package's own do, proven
by differential tests against native Rust. moq-net translates with kio, moq-clock,
moq-pattern, and rangemap.

## Plan

Decided with the maintainer on 2026-10-27, from the
[model's](/quest/m2/ts-model/README.md) census: its roots refuse calls into kio,
moq-clock, and moq-pattern 2,588 times (`kio::Waiter`, `Ref::deref`, `Shared::lock`,
`moq_clock::Instant`), and 424 of the 1,112 items that refuse anything refuse nothing
else. rangemap adds 266.

- One program: the driver lowers the named crates' MIR with the package's. A
  `cargo check` encodes no MIR for a dependency's functions, and the driver wraps every
  rustc, so it gives the named crates `-Zalways-encode-mir`.
- The driver passes `--cfg rust2_ts` to the package and the named crates, so moq's safe
  twins of its internal `unsafe` are what translates.
- Each crate's output goes in its own module directory, and the instances a crate's
  generics get from the package go with the crate that defines them.
- The rule is general: a dependency without `unsafe` translates from source; one with
  `unsafe` is refused with a span unless the consumer [binds](/quest/m2/ts-model/bind.md)
  it.
- A named crate's own refusals belong to other quests: kio's atomics and `thread_local!`
  to [shared ownership](/quest/m2/ts-shared.md), and its `RawWaker` vtable to bindings.
- Report the model's census before and after.

Public API: `ts.toml` names the dependencies translated with the package; the output gains
their modules.
