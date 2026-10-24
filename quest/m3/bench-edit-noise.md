# [S] Steadier edit numbers in the build benchmark

## Goal

The [build-speed benchmark](/bench/build/README.md)'s moq edit scenarios
compare night to night within a few percent: an A/A control's change against its
base stays near the clean builds' 1%, and so does a lever's change from one
nightly to the next on the same CPU.

## Plan

The report compares medians, which removes the occasional slow run. It does not
remove what remains: moq's edit times are bimodal at 5 runs (edit-leaf clusters
near 3.2 s and 3.6 s), so a median or mean lands on either mode. On the two
CI runs of [#2](https://github.com/kixelated/rust2/pull/2)
([40040916279](https://github.com/kixelated/rust2/actions/runs/40040916279),
[40051484349](https://github.com/kixelated/rust2/actions/runs/40051484349)),
the controls' edit changes have an RMS of 4.5% by median and 4.3% by mean
against 1.1% on clean builds, and a lever's change against its base moves 7%
RMS between the two runs.

Options, to measure rather than assume:

- More edit runs. They are cheap next to clean builds; resampling those runs'
  samples puts the median's A/A gap at 4.3% for 5 runs and 2.9% for 10.
- Compare minimums for edits. On the same data the min's A/A gap is 2.3% at 5
  runs, but it reports the best case, so a change that makes the slow mode
  more frequent goes unseen.
- Find and remove the slow mode's cause (page cache across the variants'
  target directories, CPU frequency, the runner's neighbors) if it is in the
  benchmark's control.

Public API: the benchmark's default run counts or report columns, if either
changes.
