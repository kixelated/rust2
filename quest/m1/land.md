# [XS] Land main through quest land

## Goal

PRs to `main` land only through `quest land` on a hosted runner: every queued
PR goes into one chain of squash commits, checked once with the repository's
Check steps, and `main` fast-forwards to the longest passing prefix. GitHub's
merge button cannot land around it.

## Plan

Decided 2026-10-26 (maintainer), planning moq-dev/moq's merge throughput
([moq-dev/moq#6854](https://github.com/moq-dev/moq/pull/6854)): the batched
gate is shared through kixelated/quest, and this repository adopts it.

[kixelated/rust2#137](https://github.com/kixelated/rust2/pull/137) added the
Land workflow, a test that only its land job names an environment, and
`CONTRIBUTING.md`'s landing rule. What remains is the maintainer's setup and
the live verification.

- **Push credential (maintainer).** A GitHub App installed on this repository
  is the `main` ruleset's bypass actor, with Contents and Workflows write
  (Workflows lets it land PRs that change `.github/workflows`) and no
  webhook. GitHub Actions cannot be: rulesets refused it as a bypass actor on
  a personal repository, kixelated/quest, on 2026-10-27. The `land`
  environment, deployable only from `main`, holds the App's client ID as the
  variable `LAND_APP_CLIENT_ID` and its key as the secret
  `LAND_APP_PRIVATE_KEY`.
- **Ruleset (maintainer).** A new `main` ruleset blocks deletion and
  non-fast-forward pushes and requires two checks: `Check` from GitHub
  Actions, and `quest land` pinned to the land App, which has no checks or
  statuses permission and so never reports it. Its bypass actors are the land
  App and the admin role, which keeps an emergency bypass. Ruleset writes are
  the maintainer's.
- The PR that finishes this quest lands through the gate, verifying it.
