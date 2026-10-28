# [L] Trait objects

## Goal

[rust2-ts](/ts/README.md) translates the trait objects moq uses that it still refuses
(`Arc<dyn Trait>`, `Box<dyn FnMut>` and `&dyn Fn`, and other crates' traits), with Rust's
dispatch and drop order, proven by differential tests against native Rust, so a consumer
neither rewrites nor binds them.

## Plan

Decided with the maintainer on 2026-10-27: a stock crate translates with no annotations or
config, so trait objects move from never to not yet. Until this lands, moq rewrites
`dyn Expiry` as an enum and binds its boxed closures ([bindings](/ts/README.md#bindings)).

- `&dyn Debug` and `&dyn Display` translate as functions writing their value, `dyn Future`
  as an `rt.Future`, and `&dyn Trait`, `&mut dyn Trait`, and `Box<dyn Trait>` of the
  crate's own traits as an object with the trait's methods, which a class per type wraps
  the value in. Build the rest on those.
- `Arc<dyn Trait>`: an `Arc` unsized from an `Arc<T>` shares its count, so the wrapper
  can't replace the `Arc`'s value; its clone, drop, and deref must still see one count.
  `Arc<dyn Future>` has the same problem.
- Another crate's traits: `Fn*` objects can be the closure itself, as `&dyn Debug` is the
  function writing it; `Iterator`, `Error`, `Any`, and `bytes::Buf` need their std
  methods mapped onto the object.
- Generic traits and associated types name their arguments in the object's type.

Public API: the translator's output.
