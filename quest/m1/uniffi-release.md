# [S] Waiting: publish the UniFFI fork

## Goal

[The fork](/uniffi/README.md) publishes to crates.io once the maintainer
decides it should. Until then every crate keeps upstream's name with
`publish = false`, and consumers depend on a revision of this repository.
Check: ask the maintainer. crates.io moq-ffi cannot have a git dependency, so
moq's [switch to the fork](https://github.com/moq-dev/moq/blob/main/quest/m2/uniffi-fork.md)
needs this before it releases.

## Plan

- **Names**: rename the published crates to `rust2-*` (`rust2-uniffi`,
  `rust2-uniffi-core`, ...) in their manifests only, keeping upstream's
  library names, so code and cherry-picks stay unchanged. Consumers write
  `uniffi = { package = "rust2-uniffi", ... }`.
- **Versions and tags**: decide then. The crates sit at 0.1.0 with no tag.
- **Release**: [the import PR](https://github.com/moq-dev/rust2/pull/3)
  drafted release-plz with one version group, one `uniffi-v{version}` tag and
  changelog, and a Release workflow reading `APP_CLIENT_ID`,
  `APP_PRIVATE_KEY`, and `CARGO_REGISTRY_TOKEN`. The first publish of a new
  crate name needs a token; trusted publishing can replace it afterwards.
- **Scope**: publish the crates a consumer needs from crates.io. The
  generators can stay in the flake and `cargo install --git` until one needs
  them there.
