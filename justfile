set shell := ["bash", "-euo", "pipefail", "-c"]

# List the available development commands.
default:
    @just --list

# Lint the repository tooling and the quest tree.
check:
    quest check
    actionlint
    shellcheck .claude/hooks/*.sh .codex/hooks/*.sh bench/*.sh
    shfmt -d .claude/hooks/*.sh .codex/hooks/*.sh bench/*.sh
    taplo format --check
    nixfmt --check flake.nix

# Run the tests.
test:
    bash .claude/hooks/direnv.test.sh

# Measure Rust build time on the pinned corpus; see bench/README.md.
bench *args:
    bench/run.sh {{ args }}

# Apply formatters without changing program behavior.
fix:
    shfmt -w .claude/hooks/*.sh .codex/hooks/*.sh bench/*.sh
    taplo format
    nixfmt flake.nix
