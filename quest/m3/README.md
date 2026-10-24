# m3: deferred

## Goal

Work that waits on evidence or has no consumer yet: the build-speed tracks,
each a measurement that decides whether its follow-up lands.

## Required

- [No-C check](/quest/m3/bench-no-c.md) - the benchmark fails if the `moq-pure-crypto` corpus gains C through `links`, `cc`, or `cmake`
- [Steadier edit numbers](/quest/m3/bench-edit-noise.md) - moq's edit scenarios compare night to night within a few percent
- [Early cutoff for edit rebuilds](/quest/m3/edit-cutoff.md) - real moq-net edits decide how rust2 skips rebuilding dependents without waiting on cargo
- [Relink without rebuilding](/quest/m3/relink.md) - measure upstream's skip of dependency-only rebuilds on moq's edits once it reaches nightly
