# m2: TypeScript

## Goal

Research work with a named consumer that is not waiting on it yet.

## Required

- [Drop in the TypeScript translator](/quest/m2/ts-drop.md) - a `Drop` type's `drop()` runs where Rust drops it
- [Async in the TypeScript translator](/quest/m2/ts-async.md) - async code translates with Rust's lazy futures
- [Translate moq-net's model to TypeScript](/quest/m2/ts-model.md) - the driver grows from moq-net's codec to its model, for moq to compare against WASM
- [Namespaced external types](/quest/m2/uniffi-namespaces.md) - two crates linked into one library can each export a same-named type in every generator
