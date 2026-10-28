# [S] A contract symbol

## Goal

Each build exports a contract symbol that bindings check when they load, so a header and a
library from different builds refuse to mix.

## Plan

It replaces the fork's namespaced contract and per-method checksums, which the export never
had.

Public API: one exported symbol per library.
