# [L] Expected-only C++ bindings

## Goal

[The fork](/uniffi/README.md)'s C++ generator supports callback interfaces and
foreign-implemented traits under `error_style = "expected"`. The exceptions style
is then deleted entirely, leaving one style, and the C++ coverall test comes back
under the expected style.

## Plan

Decided with the maintainer on 2026-10-24, after
[kixelated/rust2#34](https://github.com/kixelated/rust2/pull/34) removed
exception-style async. That left the exceptions style as sync code plus callback
interfaces, and the expected style as everything else, so coverall (async plus
callback interfaces) fits neither and its C++ test was dropped.

- Foreign implementations report failure without throwing, so the bindings still
  build with `-fno-exceptions`. Decide how a C++ implementation returns a typed
  error and how an unexpected failure reaches Rust.
- Decide whether async callback methods come back in the same change. The
  exception-style bridge #34 removed is a reference for cancellation and
  duplicate completions, not a design to restore.
- The C backend refuses callback interfaces today; say whether it follows.
- moq uses the expected style without callback interfaces, so its generated
  bindings must stay byte-identical.

Public API: generated C++ for callback interfaces, and `error_style` loses
`"exceptions"`, a breaking change to the generator's config.
