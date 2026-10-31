# [XS] The TS benchmark's A/B route as documented

## Goal

The A/B run [the TS README](/ts/README.md#bench) documents times another checkout's
translation as one more route, as written.

## Plan

Found by [#314](https://github.com/moq-dev/rust2/pull/314) on 2026-10-30:
`just ts bench --route main=<dir>` fails, since the recipe takes `checkout` and `route`
as positional parameters and no `--route` flag, and `just` reads a positional
`main=<dir>` as a variable override, failing with "justfile does not contain recipe".
Make the recipe take the route the way the README shows, or the README show a form that
works, and run the documented command once to prove it.

Public API: none.
