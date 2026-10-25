# m3: deferred

## Goal

Work that waits on evidence or has no consumer yet: the build-speed tracks,
each a measurement that decides whether its follow-up lands.

## Required

- [Narrower bench trigger](/quest/m3/bench-trigger.md) - PRs run the build benchmark only when they change what it measures
- [Steadier edit numbers](/quest/m3/bench-edit-noise.md) - moq's edit scenarios compare night to night within a few percent
- [ra-jit in the build benchmark](/quest/m3/bench-ra-jit.md) - a variant hot-patching changed functions on moq-net's edit, with what it refuses
- [Relink without rebuilding](/quest/m3/relink.md) - measure upstream's skip of dependency-only rebuilds on moq's edits once it reaches nightly
