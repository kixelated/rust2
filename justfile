set shell := ["bash", "-euo", "pipefail", "-c"]

# The TypeScript driver, its own workspace on its own nightly: `just ts <recipe>`.
mod ts

# The UniFFI fork, in the root workspace: `just uniffi <recipe>`.
mod uniffi

# List the available development commands.
default:
    @just --list

# Lint the repository tooling, the quest tree, the root workspace, and every project.
check:
    quest check
    actionlint
    shellcheck .claude/hooks/*.sh .codex/hooks/*.sh
    shfmt -d .claude/hooks/*.sh .codex/hooks/*.sh
    taplo format --check
    nixfmt --check flake.nix
    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    just ts check
    just uniffi check

# Run the tests.
test:
    bash .claude/hooks/direnv.test.sh
    just ts test
    just uniffi test

# Apply formatters without changing program behavior.
fix:
    shfmt -w .claude/hooks/*.sh .codex/hooks/*.sh
    taplo format
    nixfmt flake.nix
    cargo fmt --all
    just ts fix
