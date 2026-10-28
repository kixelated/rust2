# [L] Trait objects

## Goal

[rust2-ts](/ts/README.md) translates the trait objects moq uses that it still refuses
(`Box<dyn FnMut>` and `&dyn Fn`, `Arc<dyn Any>`, and other crates' traits), with Rust's
dispatch and drop order, proven by differential tests against native Rust, so a consumer
neither rewrites nor binds them.

## Plan

Decided with the maintainer on 2026-10-27: a stock crate translates with no annotations or
config, so trait objects move from never to not yet. Until this lands, moq binds its boxed
closures ([bindings](/ts/README.md#bindings)).

- `&dyn Debug` and `&dyn Display` translate as functions writing their value, `dyn Future`
  as an `rt.Future`, and `&dyn Trait`, `&mut dyn Trait`, `Box<dyn Trait>`, and
  `Arc<dyn Trait>` of the crate's own traits as an object with the trait's methods, which
  a class per type wraps the value in; an `Arc`'s is an `rt.ArcDyn` sharing the count of
  the `Arc` it was made from. Build the rest on those.
- Another crate's traits: `Fn*` objects can be the closure itself, as `&dyn Debug` is the
  function writing it; `Iterator`, `Error`, `Any`, and `bytes::Buf` need their std
  methods mapped onto the object.
- Generic traits and associated types name their arguments in the object's type.

Public API: the translator's output.
