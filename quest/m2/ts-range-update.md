# [XS] Struct update syntax on a std range

## Goal

[rust2-ts](/ts/README.md) translates `Range { start, ..r }` with the base's fields, or
refuses it with an error at its span, under both lowerings. Today the THIR lowering
drops the base when it builds the range, and panics the driver on an index when a
`for` loop iterates it.

## Plan

Found in [#49](https://github.com/kixelated/rust2/pull/49). The std arm of the THIR
aggregate lowering returns before the base is read, and the `for` loop and slicing
paths assume every field is written. MIR's aggregate carries every field, so it may
already be right: prove it with a differential case before deciding whether THIR
supports it or refuses it. If [Lower bodies from MIR](/quest/m2/ts-mir.md) lands
first, only the MIR check remains.

Public API: none.
