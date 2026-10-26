#!/usr/bin/env bash
# Measures moq's Nix release builds in CPU time, built each way nix.nix offers.
# Usage and output are described in README.md.
set -euo pipefail

bench=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$bench/../.." && pwd)

# The build-bench shell carries hyperfine, taplo, and cargo.
if [[ -z ${RUST2_BENCH_NIGHTLY:-} ]]; then
    exec nix develop "$root#build-bench" --command "$bench/nix.sh" "$@"
fi

die() {
    echo "bench-nix: $*" >&2
    exit 1
}

usage() {
    cat <<'EOF'
Usage: just bench-nix [OPTIONS]

Options:
  --variant NAME   Measure only this variant (repeatable): crane, crane-mbx,
                   crate2nix, or cargo-mbx (default: all)
  --out DIR        Write results to this new or empty directory
                   (default: target/bench/nix/<UTC time>)
EOF
}

all_variants=(crane crane-mbx crate2nix cargo-mbx)
variants=()
out=
while (($#)); do
    case $1 in
        --variant) variants+=("${2:?--variant needs a name}") && shift 2 ;;
        --out) out=${2:?--out needs a directory} && shift 2 ;;
        -h | --help) usage && exit 0 ;;
        *) usage >&2 && die "unknown argument: $1" ;;
    esac
done
((${#variants[@]})) || variants=("${all_variants[@]}")
for name in "${variants[@]}"; do
    [[ " ${all_variants[*]} " == *" $name "* ]] || die "unknown variant: $name"
done

config=$(taplo get -f "$bench/nix.toml" -o json)
field() { jq -r "$1" <<<"$config"; }
mapfile -t packages < <(field '.packages[]')
mapfile -t bump < <(field '.bump[]')
hub=$(field .hub)

# Apart from run.sh's, so a `just bench` beside this run keeps its checkout.
work=/var/tmp/rust2-bench-$(id -un)/nix
results=$root/target/bench/nix
out=${out:-$results/$(date -u +%Y%m%dT%H%M%SZ)}
[[ -z $(find "$out" -mindepth 1 -print -quit 2>/dev/null) ]] || die "$out is not empty; pass a new --out"
mkdir -p "$work" "$out/raw" "$out/logs"
out=$(cd "$out" && pwd)
src=$work/src

# As in run.sh: none of the caller's cargo, rustc, or mbx settings reach the builds.
caller_vars=("${!CARGO@}" "${!RUSTC@}" "${!RUSTDOC@}" "${!RUSTUP@}" "${!MBX@}")
for name in RUSTFLAGS CI TF_BUILD; do
    [[ -z ${!name+set} ]] || caller_vars+=("$name")
done
if ((${#caller_vars[@]})); then
    echo "bench-nix: ignoring ${caller_vars[*]}"
    unset "${caller_vars[@]}"
fi
export CARGO_HOME=$work/cargo-home

tools=$(nix build --no-link --print-out-paths "$root#bench-nix")
PATH=$tools/bin:$PATH
# mbx's own store path, which the Nix builds take as an input.
mbx=$(dirname "$(dirname "$(readlink -f "$tools/bin/mbx")")")

# Each Nix variant builds in a store of its own, so it starts from nothing and
# substitutes nothing the host built. Builds run as this user, so their CPU time
# is this process's children's, in a build directory of the store's own, since
# the host's may belong to its daemon.
store_args() { # variant
    printf '%s\n' --store "$work/$1/store" --option build-dir "$work/$1/build" --option substituters https://cache.nixos.org \
        --option max-jobs "$(getconf _NPROCESSORS_ONLN)" --option cores 0 \
        --option extra-sandbox-paths "/mbx=$work/$1/mbx"
}

eval_args() {
    printf '%s\n' --impure -f "$bench/nix.nix" --argstr src "$src" --argstr mbx "$mbx" --argstr mbxStore /mbx
}

record_host() {
    jq -n \
        --arg date "$(date -u +%FT%TZ)" \
        --arg os "$(uname -srm)" \
        --arg cpu "$(sed -n 's/^model name[[:space:]]*: //p' /proc/cpuinfo | head -1)" \
        --arg cpus "$(getconf _NPROCESSORS_ONLN)" \
        --arg memory "$(awk '/MemTotal/ { printf "%.0f GiB", $2 / 1048576 }' /proc/meminfo)" \
        --arg runner "${RUNNER_NAME:-local}" \
        --arg rust2 "$(git -C "$root" rev-parse HEAD)" \
        --arg moq "$(field .rev)" \
        --arg nix "$(nix --version)" \
        --arg mbx "$("$mbx/bin/mbx" --version)" \
        --arg crate2nix "$(crate2nix --version)" \
        '$ARGS.named' >"$out/host.json"
}

# Resets the checkout to the pin, with no edit and the pinned lockfile.
checkout() {
    local rev
    rev=$(field .rev)
    if [[ ! -d $src/.git ]]; then
        git init -q "$src"
        git -C "$src" remote add origin "$(field .git)"
    fi
    if [[ $(git -C "$src" rev-parse -q --verify HEAD || true) != "$rev" ]]; then
        git -C "$src" fetch -q --depth 1 origin "$rev"
        git -C "$src" checkout -q --detach FETCH_HEAD
    fi
    git -C "$src" reset -q --hard "$rev"
    git -C "$src" clean -qfdx
}

# Times one command with hyperfine, its output in the log. hyperfine runs a
# command without a shell, so one redirects it.
time_cmd() { # json log command...
    local json=$1 log=$2 cmd
    shift 2
    cmd="$(printf '%q ' "$@") >$(printf '%q' "$log") 2>&1"
    hyperfine --style none --runs 1 --export-json "$json" "bash -c $(printf '%q' "$cmd")" >/dev/null ||
        { tail -60 "$log" >&2 && die "failed: $*"; }
}

# One hyperfine measurement, as the results record it.
measurement() { # hyperfine.json
    jq -c '.results[0].measurements[0]
        | {wall: .time_wall_clock.value, cpu: .time_cpu.value, instructions: .instructions.value, rss: .memory_peak_resident.value}' "$1"
}

# Appends one step's result. `compiled` counts the compilations that ran the
# compiler and `restored` those a cache served, each as the variant can tell.
record() { # variant scenario package derivations compiled restored eval build
    jq -n -c --arg variant "$1" --arg scenario "$2" --arg package "$3" \
        --argjson derivations "$4" --argjson compiled "$5" --argjson restored "$6" --argjson eval "$7" --argjson build "$8" \
        '{$variant, $scenario, $package, $eval, $build, $derivations, $compiled, $restored}' >>"$out/results.jsonl"
}

# The compilations an mbx build report says ran the compiler, and those it restored.
mbx_counts() { # report.json
    jq -r '"\(.misses + .unconsulted) \(.hits)"' "$1"
}

# The derivations that compile Rust: crane's, and crate2nix's one per crate.
measured() { # closure.json
    jq -r '.derivations | to_entries[] | select(.value.env.cargoVendorDir != null or .value.env.crateName != null) | .key' "$1"
}

# One step of a Nix variant: evaluate, realise what the build needs but does not
# measure (toolchain, sources, vendored crates), then time the build.
nix_step() { # variant scenario package attr
    local variant=$1 scenario=$2 package=$3 attr=$4 drv output compiled restored=null tag=$1.$2
    local -a nixo args measured prereqs built
    mapfile -t nixo < <(store_args "$variant")
    mapfile -t args < <(eval_args)
    time_cmd "$out/raw/$tag.eval.json" "$out/logs/$tag.eval.log" nix "${nixo[@]}" eval --raw "${args[@]}" "$attr.drvPath"
    drv=$(<"$out/logs/$tag.eval.log")
    [[ $drv == /nix/store/*.drv ]] || die "$variant $scenario evaluated to $drv"
    nix "${nixo[@]}" derivation show -r "$drv" >"$work/closure.json"
    mapfile -t measured < <(measured "$work/closure.json")
    mapfile -t prereqs < <(jq -r --args '
        .derivations as $d
        | [$ARGS.positional[] | $d[.].inputs.drvs | to_entries[] | select(.key | IN($ARGS.positional[]) | not)
            | "/nix/store/\(.key)^\(.value.outputs | join(","))"]
        | unique[]' "${measured[@]}" <"$work/closure.json")
    if ((${#prereqs[@]})); then
        nix "${nixo[@]}" build --no-link "${prereqs[@]}" >"$out/logs/$tag.prereqs.log" 2>&1 ||
            { tail -40 "$out/logs/$tag.prereqs.log" >&2 && die "$variant failed to realise what $scenario needs"; }
    fi
    # The derivations that compile Rust which the build will run.
    mapfile -t built < <(nix "${nixo[@]}" build --dry-run "$drv^out" 2>&1 |
        sed -n 's|^  /nix/store/\(.*\.drv\)$|\1|p' | grep -Fx -f <(printf '%s\n' "${measured[@]}") || true)
    time_cmd "$out/raw/$tag.build.json" "$out/logs/$tag.build.log" nix "${nixo[@]}" build --no-link -L "$drv^out"
    case $variant in
        crane) compiled=$(grep -cE '^[^>]*> +(Compiling|Checking) ' "$out/logs/$tag.build.log" || true) ;;
        crate2nix) compiled=${#built[@]} ;;
        crane-mbx)
            output=$(nix "${nixo[@]}" build --no-link --print-out-paths "$drv^out")
            read -r compiled restored < <(mbx_counts "$work/$variant/store$output/share/mbx/report.json")
            ;;
    esac
    record "$variant" "$scenario" "$package" "${#built[@]}" "$compiled" "$restored" \
        "$(measurement "$out/raw/$tag.eval.json")" "$(measurement "$out/raw/$tag.build.json")"
}

# One step of cargo with mbx: a fresh target directory, as a CI job starts with,
# on the store earlier steps filled.
cargo_step() { # scenario package
    local scenario=$1 package=$2 tag=cargo-mbx.$1 compiled restored
    rm -rf "$CARGO_TARGET_DIR" "$MBX_STATS_REPORT"
    (
        cd "$src"
        time_cmd "$out/raw/$tag.build.json" "$out/logs/$tag.build.log" \
            mbx build --release --locked --offline -p "$package" --features "$(jq -r --arg p "$package" '.[$p] | join(",")' <<<"$features")"
    )
    read -r compiled restored < <(mbx_counts "$MBX_STATS_REPORT")
    record cargo-mbx "$scenario" "$package" null "$compiled" "$restored" null "$(measurement "$out/raw/$tag.build.json")"
}

# crate2nix's build plan for the checkout. A project commits it, so its time is
# recorded apart from the builds.
generate() {
    local lock tag
    git -C "$src" diff --quiet -- Cargo.lock && lock=pinned || lock=bumped
    tag=crate2nix.generate-$lock
    (cd "$src" && time_cmd "$out/raw/$tag.json" "$out/logs/$tag.log" crate2nix generate)
    measurement "$out/raw/$tag.json" | jq -c --arg lock "$lock" '{$lock} + .' >>"$out/generate.jsonl"
}

# Runs every scenario for one variant, from an empty store.
run_variant() { # variant
    local variant=$1 first=${packages[0]} second=${packages[1]} attr
    echo "bench-nix: $variant"
    [[ ! -e $work/$variant ]] || chmod -R u+w "$work/$variant"
    rm -rf "${work:?}/$variant"
    mkdir -p "$work/$variant/mbx" "$work/$variant/build"
    checkout
    if [[ $variant == cargo-mbx ]]; then
        step() { cargo_step "$1" "$2"; }
        # The toolchain, features, and flags of the Nix builds, set as a CI job would.
        export MBX_CACHE_DIR=$work/$variant/mbx MBX_GC_AUTO=0 MBX_STATS_REPORT=$work/$variant/report.json \
            CARGO_TARGET_DIR=$work/$variant/target PATH=$toolchain/bin:$PATH RUSTFLAGS=$rustflags CI=true
        (cd "$src" && cargo fetch --locked -q)
    else
        nix copy --no-check-sigs --to "$work/$variant/store" "$mbx"
        attr=${variant/crane-mbx/craneMbx}
        step() { nix_step "$variant" "$1" "$2" "$attr.$2"; }
    fi
    [[ $variant != crate2nix ]] || generate
    step clean "$first"
    step second-package "$second"
    "$bench/run.sh" edit "$src/$hub"
    step edit-hub "$first"
    git -C "$src" checkout -q -- "$hub"
    (cd "$src" && cargo update -q "${bump[@]}")
    [[ $variant != crate2nix ]] || generate
    [[ $variant != cargo-mbx ]] || (cd "$src" && cargo fetch --locked -q)
    step lock-bump "$first"
    # A store holds gigabytes, more than a CI runner has free for four.
    chmod -R u+w "$work/$variant"
    rm -rf "${work:?}/$variant"
}

record_host
checkout
mapfile -t eval_args < <(eval_args)
toolchain=$(nix build --no-link --print-out-paths "${eval_args[@]}" toolchain)
features=$(nix eval --json "${eval_args[@]}" packages)
rustflags=$(nix eval --raw "${eval_args[@]}" rustflags --apply toString)
for variant in "${variants[@]}"; do
    # A subshell keeps one variant's environment out of the next.
    (run_variant "$variant")
done
jq -n -r -f "$bench/nix-report.jq" --slurpfile host "$out/host.json" --slurpfile results "$out/results.jsonl" \
    --slurpfile generate <(cat "$out/generate.jsonl" 2>/dev/null) >"$out/report.md"
mkdir -p "$results"
ln -sfn "$out" "$results/latest"
echo "bench-nix: report at $out/report.md"
