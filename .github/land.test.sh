#!/usr/bin/env bash
# The `land` environment holds the key of the App that main's ruleset lets bypass it,
# so only the Land workflow's land job may name it.
set -euo pipefail

# Print `workflow:job` for every job in the given workflows that names the `land` environment.
users() {
    awk '
        FNR == 1 { job = "workflow"; mapping = 0 }
        # Jobs sit two spaces under the top-level `jobs:`.
        /^  [A-Za-z0-9_-]+:[ \t]*$/ { job = $1; sub(/:$/, "", job) }
        # The mapping form puts `name:` on the line after `environment:`.
        (mapping && /^[ \t]*name:[ \t]*["\047]?land["\047]?[ \t]*$/) || /^[ \t]*environment:.*\<land\>/ {
            n = split(FILENAME, path, "/")
            print path[n] ":" job
        }
        { mapping = /^[ \t]*environment:[ \t]*$/ }
    ' "$@"
}

fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
printf 'jobs:\n  build:\n    environment: land\n' >"$fixture/scalar.yml"
printf 'jobs:\n  build:\n    environment:\n      name: land\n' >"$fixture/mapping.yml"
printf 'jobs:\n  build:\n    environment: release\n' >"$fixture/other.yml"
[[ $(users "$fixture/scalar.yml" "$fixture/mapping.yml" "$fixture/other.yml") == $'scalar.yml:build\nmapping.yml:build' ]]

workflows=$(cd "$(dirname "$0")" && pwd)/workflows
found=$(users "$workflows"/*.yml)
if [[ $found != land.yml:land ]]; then
    printf 'only land.yml:land may use the land environment, found:\n%s\n' "$found" >&2
    exit 1
fi
