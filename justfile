set shell := ["bash", "-euo", "pipefail", "-c"]

# List the available development commands.
default:
    @just --list

# Lint the repository tooling and the quest tree.
check:
    quest check
    actionlint
    shellcheck .claude/hooks/*.sh .codex/hooks/*.sh
    shfmt -d .claude/hooks/*.sh .codex/hooks/*.sh
    taplo format --check
    nixfmt --check flake.nix

# Run the tests.
test:
    bash .claude/hooks/direnv.test.sh

# Apply formatters without changing program behavior.
fix:
    shfmt -w .claude/hooks/*.sh .codex/hooks/*.sh
    taplo format
    nixfmt flake.nix
