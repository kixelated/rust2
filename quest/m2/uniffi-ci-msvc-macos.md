# [S] Nightly MSVC and macOS for the C and C++ fixtures

## Goal

A nightly CI job compiles [the fork](/uniffi/README.md)'s C and C++ fixtures with
MSVC and Apple Clang at C++17, once at C++20, and once as a consumer built
without exceptions or RTTI.

## Plan

Decided with the maintainer on 2026-10-24: nightly, not the merge gate. Today
all MSVC evidence for the generated bindings comes from moq's release workflow,
so a break surfaces only when moq releases.
