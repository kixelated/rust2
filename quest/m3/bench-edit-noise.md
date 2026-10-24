# [S] Steadier edit numbers in the build benchmark

## Goal

The [build-speed benchmark](/bench/build/README.md)'s moq edit scenarios
compare night to night within a few percent: an A/A control's change against its
base stays near the clean builds' 1%, and so does a lever's change from one
nightly to the next on the same CPU.

## Plan

The report compares trimmed means, which drop the occasional slow run. They do
not remove what remains: moq's edit times are bimodal at 5 runs (edit-leaf
clusters near 3.2 s and 3.6 s), and a whole variant can land on the slow mode.
Over three CI runs
([40040916279](https://github.com/kixelated/rust2/actions/runs/40040916279),
[40051484349](https://github.com/kixelated/rust2/actions/runs/40051484349),
[40068512970](https://github.com/kixelated/rust2/actions/runs/40068512970)),
the controls' edit changes have an RMS of 4.0% against 1.3% on clean builds,
and moq's nightly-control edit-leaf reads +15% in the third.

Options, to measure rather than assume:

- More edit runs. They are cheap next to clean builds; resampling those runs'
  samples puts the trimmed mean's A/A gap at 4.0% for 5 runs and 2.9% for 10.
- Compare minimums for edits. On the same data the min's A/A gap is 2.4% at 5
  runs, but it reports the best case, so a change that makes the slow mode
  more frequent goes unseen.
- Find and remove the slow mode's cause (page cache across the variants'
  target directories, CPU frequency, the runner's neighbors) if it is in the
  benchmark's control. The first timed edit run can be slow.

Public API: the benchmark's default run counts or report columns, if either
changes.
