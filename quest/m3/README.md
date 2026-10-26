# m3: deferred

## Goal

Work that waits on evidence or has no consumer yet: the build-speed tracks,
each a measurement that decides whether its follow-up lands.

## Required

- [Steadier edit numbers](/quest/m3/bench-edit-noise.md) - moq's edit scenarios compare night to night within a few percent
- [Body edits](/quest/m3/bench-body-edit.md) - moq edits that change a called sync and async body in moq-net, which ra-jit and relinking can skip work on
- [Relink without rebuilding](/quest/m3/relink.md) - measure upstream's skip of dependency-only rebuilds on moq's edits once it reaches nightly
- [Semantic API hash](/quest/m3/api-cutoff.md) - how often moq's edits keep a hash of what dependents can observe, before forking rustc and cargo for an early cutoff
- [Test selection from MIR](/quest/m3/test-select.md) - how many tests a function-level selector would skip beyond moq's crate-level selection
