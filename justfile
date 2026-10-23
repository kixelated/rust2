set shell := ["bash", "-euo", "pipefail", "-c"]

# The UniFFI fork's fixture suites, which need `nix develop .#uniffi`.
mod uniffi

# List the available development commands.
default:
    @just --list

# Lint the repository tooling, the quest tree, and the Rust workspace.
check:
    quest check
    actionlint
    shellcheck .claude/hooks/*.sh .codex/hooks/*.sh
    shfmt -d .claude/hooks/*.sh .codex/hooks/*.sh
    taplo format --check
    nixfmt --check flake.nix
    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    # Only the published crates have docs anyone reads.
    RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --package 'rust2-uniffi*'

# Run the tests that need no foreign toolchain: the fork's crates, its generators, and its compile-fail suite.
test:
    bash .claude/hooks/direnv.test.sh
    cargo nextest run --locked --package 'rust2-uniffi*' --package uniffi-bindgen-go --package uniffi-dart --package uniffi-bindgen-cpp --package uniffi_uitests

# Apply formatters without changing program behavior.
fix:
    shfmt -w .claude/hooks/*.sh .codex/hooks/*.sh
    taplo format
    nixfmt flake.nix
    cargo fmt --all

# Open or update the release PR, then publish any crate whose version is new.
release:
    release-plz release-pr --git-token "$GITHUB_TOKEN"
    release-plz release --git-token "$GITHUB_TOKEN"
