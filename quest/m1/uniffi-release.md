# [XS] Waiting: release secrets

## Goal

kixelated/rust2 has the `APP_CLIENT_ID`, `APP_PRIVATE_KEY`, and
`CARGO_REGISTRY_TOKEN` secrets that `.github/workflows/release.yml` reads, so
release-plz can open release PRs and publish the `rust2-uniffi*` crates under
a `uniffi-v*` tag. Check: the Release workflow passes on `main`, and
`cargo search rust2-uniffi` lists the crates. Delete this quest once both hold.

## Plan

moq's [switch to the fork](https://github.com/moq-dev/moq/blob/main/quest/m2/uniffi-fork.md)
waits on that release: it pins the tag as a flake input and depends on the
crates from crates.io. The first publish of a new crate name needs a token;
crates.io trusted publishing can replace it afterwards.
