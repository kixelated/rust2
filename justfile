set shell := ["bash", "-euo", "pipefail", "-c"]

# The TypeScript driver, on its own nightly: `just ts <recipe>`.
mod ts

# List the available development commands.
default:
    @just --list

# Lint the repository tooling, the quest tree, and every project.
check:
    quest check
    actionlint
    shellcheck .claude/hooks/*.sh .codex/hooks/*.sh bench/build/*.sh
    shfmt -d .claude/hooks/*.sh .codex/hooks/*.sh bench/build/*.sh
    taplo format --check
    nixfmt --check flake.nix
    just ts check

# Run the tests.
test:
    bash .claude/hooks/direnv.test.sh
    just ts test

# Measure Rust build time on the pinned corpus; see bench/build/README.md.
bench *args:
    bench/build/run.sh {{ args }}

# Apply formatters without changing program behavior.
fix:
    shfmt -w .claude/hooks/*.sh .codex/hooks/*.sh bench/build/*.sh
    taplo format
    nixfmt flake.nix
    just ts fix
