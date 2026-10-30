# [S] fmt::Write

## Goal

[rust2-ts](/ts/README.md) translates `fmt::Write`: `String`'s impl, so `write!` and
`writeln!` into a `String` work, the impl forwarding through `&mut W`, and a crate's own
impl's provided `write_fmt` and `write_char`, proven by differential tests against native
Rust.

## Plan

Found by [#241](https://github.com/moq-dev/rust2/pull/241) on 2026-10-29:
`<&mut String as fmt::Write>` is refused, and so is `String`'s own. moq-net and kio call
neither, so this is m3 under the all-of-std decision.

- std's provided `write_fmt` reaches `fmt::write` through a `&mut dyn Write`, which
  [trait objects](/quest/m3/ts-dyn.md) refuse. Mapping it as `Formatter::write_fmt` is
  mapped avoids that, but keep std's semantics for a crate's own impl: one `write_str` per
  piece, in order, stopping at the first error, as `fmt::write` does.
- A `&mut String` is already an `rt.Place`, so `String`'s `write_str` is `push_str` on it.

Public API: the translator's output.

## Related

- [Trait objects](/quest/m3/ts-dyn.md) - `dyn fmt::Write` itself
