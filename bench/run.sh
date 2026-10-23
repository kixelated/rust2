#!/usr/bin/env bash
# Measures Rust build time on the pinned corpus and variants in corpus.toml.
# Usage and output are described in README.md.
set -euo pipefail

bench=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root=$(dirname "$bench")

# The bench shell carries hyperfine, the alternative linkers, and the pinned nightly.
if [[ -z ${RUST2_BENCH_NIGHTLY:-} ]]; then
    exec nix develop "$root#bench" --command "$bench/run.sh" "$@"
fi

die() {
    echo "bench: $*" >&2
    exit 1
}

usage() {
    cat <<'EOF'
Usage: just bench [OPTIONS]

Options:
  --corpus NAME    Measure only this corpus (repeatable; default: all)
  --variant NAME   Measure only this variant (repeatable; default: all)
  --runs N         Timed runs per scenario (default: 3 clean, 5 incremental)
  --no-attribute   Skip the nightly build that attributes where the time goes
  --out DIR        Write results here (default: target/bench/results/<UTC time>)
EOF
}

# The incremental scenarios rewrite one marker function in a crate root, so every
# run is a real body edit that reaches codegen.
if [[ ${1:-} == edit ]]; then
    file=$2
    [[ -f $file ]] || die "edit target $file does not exist"
    sed -i '/__rust2_bench_edit/d' "$file"
    printf '#[doc = "rust2 bench edit"] pub fn __rust2_bench_edit() -> u64 { %s }\n' "$(date +%s%N)" >>"$file"
    exit 0
fi

# Renders a results directory's report.md.
report() {
    local dir=$1 attribution='[]'
    local -a summaries=()
    [[ ! -d $dir/attribution ]] || mapfile -t summaries < <(find "$dir/attribution" -name summary.json | sort)
    ((${#summaries[@]} == 0)) || attribution=$(jq -s . "${summaries[@]}")
    jq -n -r -f "$bench/report.jq" --slurpfile host "$dir/host.json" \
        --slurpfile results "$dir/results.jsonl" --argjson attribution "$attribution" >"$dir/report.md"
}

if [[ ${1:-} == report ]]; then
    report "${2:?report needs a results directory}"
    exit 0
fi

corpora=()
variants=()
runs=
attribute=1
out=
while (($#)); do
    case $1 in
        --corpus) corpora+=("${2:?--corpus needs a name}") && shift 2 ;;
        --variant) variants+=("${2:?--variant needs a name}") && shift 2 ;;
        --runs) runs=${2:?--runs needs a count} && shift 2 ;;
        --no-attribute) attribute=0 && shift ;;
        --out) out=${2:?--out needs a directory} && shift 2 ;;
        -h | --help) usage && exit 0 ;;
        *) usage >&2 && die "unknown argument: $1" ;;
    esac
done
[[ -z $runs || $runs =~ ^[1-9][0-9]*$ ]] || die "--runs must be a positive integer, got $runs"

config=$(taplo get -f "$bench/corpus.toml" -o json)
mapfile -t all_corpora < <(jq -r '.corpus[].name' <<<"$config")
mapfile -t all_variants < <(jq -r '.variant[].name' <<<"$config")
((${#corpora[@]})) || corpora=("${all_corpora[@]}")
((${#variants[@]})) || variants=("${all_variants[@]}")
for name in "${corpora[@]}"; do
    jq -e --arg n "$name" 'any(.corpus[]; .name == $n)' <<<"$config" >/dev/null || die "unknown corpus: $name"
done
for name in "${variants[@]}"; do
    base=$(jq -r --arg n "$name" '.variant[] | select(.name == $n) | .base // .name' <<<"$config")
    [[ -n $base ]] || die "unknown variant: $name"
    [[ " ${variants[*]} " == *" $base "* ]] || die "variant $name compares against $base; select it too"
done

# Cargo reads every .cargo/config.toml above its working directory, so a user's
# ~/.cargo/config.toml (say, `build.incremental = false`) would reach checkouts
# under $HOME. Build outside it, with a private CARGO_HOME and none of the
# caller's cargo or rustc variables.
work=/var/tmp/rust2-bench-$(id -un)
results=$root/target/bench/results
out=${out:-$results/$(date -u +%Y%m%dT%H%M%SZ)}
mkdir -p "$work" "$out"
out=$(cd "$out" && pwd)
dir=$work
while :; do
    for f in "$dir/.cargo/config" "$dir/.cargo/config.toml"; do
        [[ ! -e $f ]] || die "$f would configure the corpus builds; move it"
    done
    [[ $dir != / ]] || break
    dir=$(dirname "$dir")
done
mapfile -t caller_vars < <(compgen -v | grep -E '^(CARGO|RUSTC|RUSTDOC|RUSTUP)|^RUSTFLAGS$' || true)
if ((${#caller_vars[@]})); then
    echo "bench: ignoring ${caller_vars[*]}"
    unset "${caller_vars[@]}"
fi
export CARGO_HOME=$work/cargo-home
stable_path=$PATH
nightly_path=$RUST2_BENCH_NIGHTLY:$PATH

field() { # corpus|variant, name, jq path
    jq -r --arg n "$2" ".$1[] | select(.name == \$n) | $3 // empty" <<<"$config"
}

# Enters a variant's toolchain and flags for the cargo calls that follow.
use_variant() {
    case $(field variant "$1" .toolchain) in
        stable) PATH=$stable_path ;;
        nightly) PATH=$nightly_path ;;
        *) die "variant $1 has no valid toolchain" ;;
    esac
    RUSTFLAGS=$(field variant "$1" .rustflags)
    export PATH RUSTFLAGS
}

record_host() {
    local cpu
    cpu=$(sed -n 's/^model name[[:space:]]*: //p' /proc/cpuinfo 2>/dev/null | head -1)
    [[ -n $cpu ]] || cpu=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo unknown)
    jq -n \
        --arg date "$(date -u +%FT%TZ)" \
        --arg os "$(uname -srm)" \
        --arg cpu "$cpu" \
        --arg cpus "$(getconf _NPROCESSORS_ONLN)" \
        --arg memory "$(awk '/MemTotal/ { printf "%.0f GiB", $2 / 1048576 }' /proc/meminfo 2>/dev/null)" \
        --arg runner "${RUNNER_NAME:-local}" \
        --arg rust2 "$(git -C "$root" rev-parse HEAD)" \
        --arg stable "$(PATH=$stable_path rustc -V)" \
        --arg nightly "$(PATH=$nightly_path rustc -V)" \
        --arg hyperfine "$(hyperfine --version)" \
        --arg mold "$(mold --version 2>/dev/null || echo missing)" \
        --arg wild "$(wild --version 2>/dev/null || echo missing)" \
        '$ARGS.named' >"$out/host.json"
}

# Clones the corpus at its pinned rev, undoing edits an interrupted run left behind.
checkout() {
    local name=$1 dir=$2 rev lock
    rev=$(field corpus "$name" .rev)
    lock=$(field corpus "$name" .lock)
    if [[ ! -d $dir/.git ]]; then
        git init -q "$dir"
        git -C "$dir" remote add origin "$(field corpus "$name" .git)"
    fi
    if [[ $(git -C "$dir" rev-parse -q --verify HEAD || true) != "$rev" ]]; then
        git -C "$dir" fetch -q --depth 1 origin "$rev"
        git -C "$dir" checkout -q --detach FETCH_HEAD
    fi
    git -C "$dir" reset -q --hard "$rev"
    git -C "$dir" clean -qfd
    [[ -z $lock ]] || cp "$bench/$lock" "$dir/Cargo.lock"
    (cd "$dir" && PATH=$stable_path cargo fetch --locked -q)
}

# Fails unless no crate in the build has a build script or a `links` key.
check_pure() {
    local name=$1 tree impure
    shift
    tree=$(cargo tree --locked --offline "$@" -e normal,build --prefix none -f '{p}' |
        awk '{ sub(/^v/, "", $2); print $1 " " $2 }' | sort -u | jq -R . | jq -s .)
    impure=$(cargo metadata --locked --offline --format-version 1 | jq -r --argjson tree "$tree" '
		.packages[]
		| select(("\(.name) \(.version)") as $k | $tree | index($k))
		| select(.links != null or any(.targets[]; .kind | index("custom-build")))
		| "\(.name) \(.version)"')
    [[ -z $impure ]] || die "corpus $name is marked pure but builds: $(tr '\n' ' ' <<<"$impure")"
}

# Times one scenario with hyperfine and appends a result record.
measure() { # corpus variant scenario runs setup prepare command
    local corpus=$1 variant=$2 scenario=$3 n=$4 setup=$5 prepare=$6 cmd=$7 json
    local -a flags=(--style basic --runs "$n" --command-name "$variant" --prepare "$prepare")
    [[ -z $setup ]] || flags+=(--setup "$setup")
    json=$out/raw/$corpus.$scenario.$variant.json
    echo "bench: $corpus $scenario $variant ($n runs)"
    hyperfine "${flags[@]}" --export-json "$json" "$cmd"
    jq -c --arg corpus "$corpus" --arg scenario "$scenario" --arg variant "$variant" \
        --arg base "$(field variant "$variant" '.base // .name')" \
        '.results[0] | .summary as $s | {
            $corpus, $scenario, $variant, $base,
            mean: $s.time_wall_clock.mean, stddev: $s.time_wall_clock.stddev, min: $s.time_wall_clock.min,
            cpu: $s.time_cpu.mean, instructions: $s.instructions.mean, rss: $s.memory_peak_resident.max,
            times: [.measurements[].time_wall_clock.value]
        }' \
        "$json" >>"$out/results.jsonl"
}

run_variant() { # corpus variant dir
    local corpus=$1 variant=$2 dir=$3 leaf hub clean_runs edit_runs build check
    local -a args
    mapfile -t args < <(jq -r --arg n "$corpus" '.corpus[] | select(.name == $n) | .args[]' <<<"$config")
    leaf=$(field corpus "$corpus" .leaf)
    hub=$(field corpus "$corpus" .hub)
    clean_runs=${runs:-3}
    edit_runs=${runs:-5}
    use_variant "$variant"
    export CARGO_TARGET_DIR=$work/target/$corpus/$variant
    rm -rf "$CARGO_TARGET_DIR"
    build=$(printf '%q ' cargo build --locked --offline -q "${args[@]}")
    check=$(printf '%q ' cargo check --locked --offline -q "${args[@]}")

    # An untimed clean build warms the disk cache and shows any failure in full.
    echo "bench: $corpus warm-up $variant"
    (cd "$dir" && cargo build --locked --offline "${args[@]}" >"$out/logs/$corpus.$variant.log" 2>&1) ||
        { tail -40 "$out/logs/$corpus.$variant.log" >&2 && die "$corpus failed to build with $variant"; }

    (
        cd "$dir"
        measure "$corpus" "$variant" check "$clean_runs" "" "cargo clean -q" "$check"
        measure "$corpus" "$variant" build "$clean_runs" "" "cargo clean -q" "$build"
        for scenario in edit-leaf edit-hub; do
            [[ $scenario == edit-leaf ]] && file=$leaf || file=$hub
            [[ -n $file ]] || continue
            edit=$(printf '%q ' "$bench/run.sh" edit "$file")
            measure "$corpus" "$variant" "$scenario" "$edit_runs" "$edit && $build" "$edit" "$build"
        done
    )

    git -C "$dir" checkout -q -- .
    rm -rf "$CARGO_TARGET_DIR"
}

# One nightly clean build and each edit, with cargo's build analysis log and
# rustc's self-profile, summarized into where the time goes.
attribute() { # corpus dir
    local corpus=$1 dir=$2 leaf hub prof stamp step log dest proc_macros
    local -a args logs steps=(build)
    mapfile -t args < <(jq -r --arg n "$corpus" '.corpus[] | select(.name == $n) | .args[]' <<<"$config")
    leaf=$(field corpus "$corpus" .leaf)
    hub=$(field corpus "$corpus" .hub)
    steps+=(edit-leaf)
    [[ -z $hub ]] || steps+=(edit-hub)
    echo "bench: $corpus attribution"
    # Build analysis logs proc macros as plain libs.
    proc_macros=$(cd "$dir" && cargo metadata --locked --offline --format-version 1 |
        jq -c '[.packages[] | select(any(.targets[]; .kind | index("proc-macro"))) | .id]')

    PATH=$nightly_path
    export CARGO_TARGET_DIR=$work/target/$corpus/attribution
    rm -rf "$CARGO_TARGET_DIR"
    prof=$CARGO_TARGET_DIR/self-profile
    stamp=$CARGO_TARGET_DIR/started
    mkdir -p "$prof"
    # One profile directory for every step: a changed RUSTFLAGS would rebuild everything.
    export RUSTFLAGS="-Z self-profile=$prof"
    # Add the markers up front so each edit step is a body edit, as in the timed runs.
    "$bench/run.sh" edit "$dir/$leaf"
    [[ -z $hub ]] || "$bench/run.sh" edit "$dir/$hub"
    for step in "${steps[@]}"; do
        case $step in
            edit-leaf) "$bench/run.sh" edit "$dir/$leaf" ;;
            edit-hub) "$bench/run.sh" edit "$dir/$hub" ;;
        esac
        touch "$stamp"
        (cd "$dir" && cargo -Z build-analysis -Z section-timings build --config build.analysis.enabled=true \
            --locked --offline "${args[@]}" >"$out/logs/$corpus.attribution.$step.log" 2>&1) ||
            { tail -40 "$out/logs/$corpus.attribution.$step.log" >&2 && die "$corpus failed the $step attribution build"; }
        # Cargo names the log by session, so find this build's among any written since.
        mapfile -t logs < <(find "$CARGO_HOME/log" -name '*.jsonl' -newer "$stamp")
        log=
        for candidate in "${logs[@]}"; do
            [[ $(head -1 "$candidate" | jq -r .target_dir) != "$CARGO_TARGET_DIR" ]] || log=$candidate
        done
        [[ -n $log ]] || die "found no cargo build log for $CARGO_TARGET_DIR"
        dest=$out/attribution/$corpus.$step
        mkdir -p "$dest"
        mv "$log" "$dest/cargo.jsonl"
        find "$prof" -name '*.mm_profdata' -print0 |
            xargs -0 -r -n 1 -P "$(getconf _NPROCESSORS_ONLN)" summarize summarize --json >/dev/null
        jq -s -f "$bench/self-profile.jq" "$prof"/*.json >"$dest/self-profile.json"
        rm -f "$prof"/*
        jq -s -f "$bench/attribute.jq" --arg corpus "$corpus" --arg step "$step" --argjson proc_macros "$proc_macros" \
            --slurpfile profile "$dest/self-profile.json" "$dest/cargo.jsonl" >"$dest/summary.json"
    done
    unset RUSTFLAGS

    git -C "$dir" checkout -q -- .
    rm -rf "$CARGO_TARGET_DIR"
}

mkdir -p "$out/raw" "$out/logs"
record_host
for corpus in "${corpora[@]}"; do
    dir=$work/src/$corpus
    checkout "$corpus" "$dir"
    mapfile -t args < <(jq -r --arg n "$corpus" '.corpus[] | select(.name == $n) | .args[]' <<<"$config")
    if [[ $(field corpus "$corpus" .pure) == true ]]; then
        (cd "$dir" && PATH=$stable_path check_pure "$corpus" "${args[@]}")
    fi
    # Subshells keep each variant's toolchain and flags from leaking into the next.
    for variant in "${variants[@]}"; do
        (run_variant "$corpus" "$variant" "$dir")
    done
    ((attribute == 0)) || (attribute "$corpus" "$dir")
done

report "$out"
mkdir -p "$results"
ln -sfn "$out" "$results/latest"
echo "bench: report at $out/report.md"
