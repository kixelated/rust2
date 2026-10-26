# [S] The exporter refuses what it can't export

## Goal

Every API shape that rust2-ffi exports either works end to end in the generated
bindings, or the census refuses it with a span and a reason. No shape compiles to
broken glue or to an export nothing can use. Three cases from
[kixelated/rust2#52](https://github.com/kixelated/rust2/pull/52) don't meet that today:

- An `Option<&T>` value parameter lifts as `T`, so the generated glue doesn't compile.
- An impl with an unconstrained lifetime is instantiated as `'static`, without saying so.
- `BufferedLease` exports as an object no exported function can construct or return.

## Plan

Decided with the maintainer on 2026-10-25: one quest for the three, because they share
the rule "fail loud and early".

- Reproduce each case with a fixture first.
- Support `Option<&T>` if moq's census needs it; otherwise refuse it.
- Pick `'static` only where every use in the impl is sound for it, and refuse the rest
  with the lifetime's span.
- Refuse an object type no export reaches, naming it. This is a census refusal, not a
  generator warning.

Public API: the exporter's census and generated bindings.
