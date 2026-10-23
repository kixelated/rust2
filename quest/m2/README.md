# m2: TypeScript

## Goal

Research work with a named consumer that is not waiting on it yet.

## Required

- [Translate moq-net's codec to TypeScript](/quest/m2/ts.md) - an own rustc driver emits readable TypeScript for moq-net's codec, proven by differential tests
- [Namespaced external types](/quest/m2/uniffi-namespaces.md) - two crates linked into one library can each export a same-named type in every generator
- [Namespaced FFI contract](/quest/m2/uniffi-contract.md) - bindings and a cdylib from different UniFFI forks refuse to load together
