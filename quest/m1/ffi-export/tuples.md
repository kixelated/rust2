# [M] Tuples inside other types

## Goal

A tuple crosses wherever its elements map, not only as a function's whole return: as an
iterator's or `Vec`'s item, inside an `Option` or `Result`, and as a record's field or a
parameter.

hang's `catalog::Audio::ranked` and `catalog::Video::ranked` are the consumer. They return
`impl Iterator<Item = (&String, &AudioConfig)>` (and `&VideoConfig`), which
[#213](https://github.com/kixelated/rust2/pull/213) collects into an array but refuses for
the tuple. They export as an array of pairs, each a string and a clone of the config object.
The same refusal holds back moq-net's `stats::Snapshot::{control, traffic, sessions}` (`Vec`s
of 2- and 3-tuples) and moq-mux's `codec::annexb::find_start_code` (`Option<(usize, usize)>`),
which come with it. Their excludes leave [the config](/ffi/moq/ffi.toml).

C, C++ and Kotlin match native Rust's transcript, and the PR reports the census before and
after.

## Plan

`ir::Type::Tuple` is only a return today, lowered to one out pointer per element. As a value
it needs a layout in each language:

- C: a generated struct per distinct element list, with fields `_0`, `_1`, and so on, named
  by the rule from its elements.
- C++: `std::tuple`, as a return is already.
- Kotlin: `Pair`, `Triple`, and the generated data class beyond three, as a return is already.

Open: whether a whole return keeps its out pointers or becomes the struct. Keeping them avoids
a breaking change to every tuple-returning C function. One rule is simpler, but a change needs
a `MIGRATE.md` entry. Decide by which reads better at moq's call sites.

The `(start, end)` pair that maps to a bound keeps its own mapping. `Audio::enabled` and
`Video::enabled` stay refused, since they `filter` through a closure, which #213 refuses.
