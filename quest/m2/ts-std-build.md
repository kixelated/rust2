# [S] Translate against the prebuilt std

## Goal

Decide, by measurement, whether rust2-ts still needs `-Zbuild-std` now that it aborts on
panic ([#187](https://github.com/kixelated/rust2/pull/187)), and drop it if not, making the
translator's builds faster and simpler.

## Plan

Suggested by #187 on 2026-10-28.

- Translate the fixtures and moq's census against the prebuilt std and diff the census and
  every snapshot against today's. Equal output, with the differential tests passing, is
  the bar.
- The risk: the prebuilt std is built with optimizations that may inline or rewrite calls
  the translator expects to see by path (std's iterators, formatting, and the calls in
  `library.rs`), so its MIR may differ from what `-Zbuild-std` gives.
- Measure the build time saved.

Public API: none, unless the toolchain the driver needs changes.
