# [XS] Relink without rebuilding

## Goal

Upstream's relinking without rebuilding reaches the pinned nightly, and the
[build-speed benchmark](/bench/build/README.md) measures it as a nightly
variant on moq's edits. Check with `rustc -Z help` (a public API hash, as in
[rust-lang/rust#156940](https://github.com/rust-lang/rust/pull/156940)) and
`cargo -Z help` (an early cutoff, as in
[rust-lang/cargo#17439](https://github.com/rust-lang/cargo/issues/17439)).

## Plan

It is the one upstream lever the moq-net edit leaves room for. The benchmark's
attribution ([run](https://github.com/kixelated/rust2/actions/runs/40093020939))
puts 4.5 s of that edit's 6.95 s critical path in crates rebuilt only because
moq-net changed, 0.6 s of it the final link that would remain. Cranelift and
the parallel frontend move the edit within noise; the backend is 6% of its
rustc time.

Status on 2026-10-24: neither flag is on nightly 2026-10-21. The 2025H2 project
goal will not complete; rust#156940 is a draft, and cargo's early cutoff is a
design discussion.

The [early cutoff spike](/quest/m3/edit-cutoff.md) may change this plan: it
decides whether rust2 carries rust#156940, helps upstream, or stops.

Public API: a benchmark variant.

## Related

- [Early cutoff for edit rebuilds](/quest/m3/edit-cutoff.md) - measures the same lever on real edits, without waiting on upstream
- [ra-jit in the build benchmark](/quest/m3/bench-ra-jit.md) - another way past dependent rebuilds and the relink
