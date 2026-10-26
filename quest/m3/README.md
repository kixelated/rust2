# m3: deferred

## Goal

Work that waits on evidence or has no consumer yet: the build-speed tracks,
each a measurement that decides whether its follow-up lands.

## Required

- [Steadier edit numbers](/quest/m3/bench-edit-noise.md) - moq's edit scenarios compare night to night within a few percent
- [Retrial Cranelift on moq's tests](/quest/m3/cranelift-unwind.md) - waits on upstream: moq's tests pass under Cranelift, so its CI can weigh it again
- [Relink without rebuilding](/quest/m3/relink.md) - measure upstream's skip of dependency-only rebuilds on moq's edits once it reaches nightly
- [Bench only what changed](/quest/m3/bench-rerun.md) - a push reruns the build benchmark only when it changes a bench input since the last benched head
