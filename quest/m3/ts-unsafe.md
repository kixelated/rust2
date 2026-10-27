# [L] Unsafe code the translator can model

## Goal

[rust2-ts](/ts/README.md) translates the `unsafe` operations it can give exact semantics in
JS, such as `get_unchecked`, `ptr::read` and `ptr::write` of a local or element, and
`MaybeUninit` written before it is read, so a dependency using only those translates from
source instead of being bound by hand.

## Plan

Decided with the maintainer on 2026-10-27: a dependency containing `unsafe` is bound by
the consumer [at first](/ts/README.md#bindings); this follows once a consumer binds
enough of them to justify it. The census of moq-net's model found `bytes`, `hashlink`,
`slab`, and `smallvec` bound, and kio's lock-free `log`.

- An `unsafe` operation outside the modeled set is refused with its span, as today.
- Undefined behavior in Rust need not be reproduced, but a translated program never
  silently differs from native where Rust's behavior is defined.

Public API: the translator's output.
