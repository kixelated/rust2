# [S] Bench only what changed since the last run

## Goal

A push to a PR reruns the build benchmark only when it changes a bench input since
the PR's last benched head. A README, quest, or docs-only push after a benched one
skips every bench job, so it costs no runner time.

## Plan

Decided with the maintainer on 2026-10-26, after
[#74](https://github.com/kixelated/rust2/pull/74)'s README-only push reran every
job, about 2 runner-hours. The `Changed` job in `bench.yml` diffs the whole PR
against its base, so once a PR touches `bench/build`, every push reruns everything.

- Compare against the last head whose bench run succeeded on this PR, when there is
  one, and against the base otherwise. Keep the flake-output comparison.
- Keep per-job filtering out: every job shares `run.sh`.
- A failed lookup runs the bench, so it never quietly skips.

Public API: none.
