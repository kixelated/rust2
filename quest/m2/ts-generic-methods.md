# [M] Methods on generic types in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md)'s output for methods on a generic struct or enum
type-checks under `tsc`, derived `Debug` included, proven by differential tests
on generic fixtures.

## Plan

Decided with the maintainer on 2026-10-23, from the formatting PR (#8): each
monomorphized method lands on the one generic class, so a body calls an
instance's method on a field typed `T` (such as `this.value.fmt$Pair(f)`), which
`tsc` rejects. moq-net's model has generic types, so the model quest needs this.

Public API: the translator's output for generic types.
