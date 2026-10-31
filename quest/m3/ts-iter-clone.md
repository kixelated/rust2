# [S] Cloning std's iterators

## Goal

[rust2-ts](/ts/README.md) translates `clone` of std's iterators and adaptors, such as a
`zip`, a `map`, a `chars`, and a set operation's iterator, the clone reading on from where
the original was, independently of it, proven by differential tests against native Rust.

## Plan

Found by [#314](https://github.com/moq-dev/rust2/pull/314) writing its zip fixture, and
by [#302](https://github.com/moq-dev/rust2/pull/302), where the set operations'
iterators refuse `clone` as other std iterators do. Planned without a moq consumer, under
the maintainer's 2026-10-27 decision that rust2 aims to support all of std.

- An `rt.Iter` that reads a collection by position clones its position; one that reads
  another iterator clones that one too, as std's derived `Clone` does, and a closure an
  adaptor holds is cloned as Rust clones the closure's captures.
- A clone must not share state the original advances: test both read after the clone,
  in turns, including partially read ones, a zip past its shorter side, and adaptors whose
  closures capture values that drop.

Public API: the translator's output.
