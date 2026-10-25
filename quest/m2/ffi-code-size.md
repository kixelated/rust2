# [M] Smaller generated bindings

## Goal

The fork's generators emit measurably smaller bindings for moq-ffi in every
language, with unchanged behavior, reported raw and gzipped by the
[binding benchmark](/bench/ffi/README.md).

## Plan

Decided with the maintainer on 2026-10-23: measure first. moq-ffi's bindings
are 653 KiB in Python, 914 KiB in Kotlin, and 684 KiB in Swift, and BoltFFI's
were 2 to 3x smaller. Attribute the size per template (converters, checksums,
per-method boilerplate) before shrinking, and stop where the remaining gain is
not worth the template complexity.

Public API: generated code changes shape; wrapper APIs do not.

## Required

- [Exporter spike](/quest/m1/ffi-export-spike.md) - this quest is paused until the spike reports, and may be folded into the exporter
