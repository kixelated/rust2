# [L] C++ bindings

## Goal

The exporter generates C++ over its one C ABI, per standard (17, 20, 23), compiled with
GCC, Clang, and MSVC, with and without exceptions and RTTI, including OBS's C17 and C++17
builds, meeting the [epic's](/quest/m1/ffi-export/README.md) rules for every generator,
native futures included.

## Plan

Public API: a new generator.
