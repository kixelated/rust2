# [L] Kotlin bindings

## Goal

The exporter generates Kotlin over its C ABI through JNI or Panama, meeting the
[epic's](/quest/m1/ffi-export/README.md) rules for every generator, native futures
included.

## Plan

Not JNA: the fork measured 3.2 us per call through it against 7 ns for upstream's JNI
bindgen.

Public API: a new generator.
