# [L] Namespaced external types in the UniFFI fork

## Goal

Two crates linked into one library can each export a type with the same
name, and a type in one can take the other's: `media.GroupConsumer(group:
GroupConsumer)` generates and runs in Kotlin, Swift, Python, Go, Dart, C++,
and C from [the fork](/uniffi/README.md).

## Plan

Decided with the maintainer on 2026-10-13 in moq, and moved here on
2026-10-23 since the fix lands entirely in the fork: fix
[mozilla/uniffi-rs#3026](https://github.com/mozilla/uniffi-rs/issues/3026)
in the fork instead of waiting for upstream. That issue shows closed only
because [moq-dev/moq#5498](https://github.com/moq-dev/moq/pull/5498)
auto-closed it; it remains unfixed upstream.

- `TypeUniverse` keys types by `(module_path, name)` instead of bare name,
  and each generator qualifies cross-crate references through the other
  crate's module. moq's probes
  ([moq-dev/moq#5460](https://github.com/moq-dev/moq/pull/5460) and
  [moq-dev/moq#5473](https://github.com/moq-dev/moq/pull/5473)) found Python
  and Go already reference external types that way. Kotlin and Dart shadow
  them, Swift needs per-crate modules plus an import, C++ sorts by bare name,
  and generated C refuses external types.
- Reuse the two-crate spike harness from moq-dev/moq#5460 as the regression:
  both layers export `GroupConsumer` and `Frame`, and every generator compiles
  and runs the consumer.
- Re-running moq's per-layer crate split decision (rejected in
  moq-dev/moq#5460) is a moq follow-up once this lands.

Public API: the fork's generated code for cross-crate types.

## Related

- [Namespaced FFI contract](/quest/m2/uniffi-contract.md) - lands first if this changes the FFI contract
