# [XS] Empty braced structs in the TypeScript translator

## Goal

[rust2-ts](/ts/README.md) translates `struct E {}` to TypeScript that passes
`tsc` with `noUnusedParameters`.

## Plan

Decided with the maintainer on 2026-10-23, from the formatting PR (#8): today it
emits `constructor(fields: {})`, which `tsc` rejects as an unused parameter.

Public API: the translator's output for empty braced structs.
