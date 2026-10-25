# [XS] Steadier edit numbers in the build benchmark

## Goal

The [build-speed benchmark](/bench/build/README.md)'s moq edit scenarios
compare night to night within a few percent: an A/A control's change against its
base stays near the clean builds' 1%, and so does a lever's change from one
nightly to the next on the same CPU.

## Plan

The bimodal edit times came from writeback: earlier runs left over a GiB of
dirty pages, and the runs it overlapped were about 10% slower. Each timed run now
starts after a `sync`, which removed that stall in a diagnostic CI run.

What remains is to check a week of nightlies since then against the 15 runs
before it, where the controls' trimmed-mean edit change had an RMS of 5.4%
(edit-leaf) and 2.9% (edit-hub) against 1.0% on clean builds. Some runners
also vary in CPU time itself, which a sync cannot remove; if the controls
still miss, measure more edit runs (cheap next to clean builds) before
changing the estimator. Minimums hide a change that makes slow runs more
frequent.

Public API: the benchmark's default run counts, if they change.
