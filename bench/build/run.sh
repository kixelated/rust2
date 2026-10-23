#!/usr/bin/env bash
# Measures Rust build time on the pinned corpus and variants in corpus.toml.
# Usage and output are described in README.md.
set -euo pipefail

bench=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$bench/../.." && pwd)

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
# caller's cargo or rustc variables. Newer cargo also turns incremental off when
# CI or TF_BUILD is set, and the edit scenarios measure a developer's rebuild.
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
caller_vars=("${!CARGO@}" "${!RUSTC@}" "${!RUSTDOC@}" "${!RUSTUP@}")
for name in RUSTFLAGS CI TF_BUILD; do
    [[ -z ${!name+set} ]] || caller_vars+=("$name")
done
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

# Enters a variant's toolchain, flags, and target directory for the cargo calls that follow.
use_variant() { # corpus variant
    case $(field variant "$2" .toolchain) in
        stable) PATH=$stable_path ;;
        nightly) PATH=$nightly_path ;;
        *) die "variant $2 has no valid toolchain" ;;
    esac
    RUSTFLAGS=$(field variant "$2" .rustflags)
    CARGO_TARGET_DIR=$work/target/$1/$2
    export PATH RUSTFLAGS CARGO_TARGET_DIR
}

# Times one run with hyperfine, appending its measurement to the scenario's raw log.
time_once() { # corpus dir scenario variant prepare command
    local corpus=$1 dir=$2 scenario=$3 variant=$4 prepare=$5 cmd=$6 json
    json=$out/raw/$corpus.$scenario.$variant.json
    (
        use_variant "$corpus" "$variant"
        cd "$dir"
        hyperfine --style none --runs 1 --prepare "$prepare" --export-json "$json" "$cmd"
    )
    jq -c '.results[0].measurements[0]' "$json" | tee -a "$out/raw/$corpus.$scenario.$variant.jsonl" |
        jq -r --arg label "$corpus $scenario $variant" '"bench: \($label) \(.time_wall_clock.value * 100 | round / 100) s"'
}

# Folds a scenario's runs of one variant into a result record.
summarize_runs() { # corpus scenario variant
    jq -s -c --arg corpus "$1" --arg scenario "$2" --arg variant "$3" \
        --arg base "$(field variant "$3" '.base // .name')" '
        def mean: add / length;
        [.[].time_wall_clock.value] as $t
        | ($t | mean) as $mean
        | {
            $corpus, $scenario, $variant, $base, $mean,
            stddev: (if ($t | length) > 1 then $t | map(. - $mean | . * .) | add / (length - 1) | sqrt else null end),
            min: ($t | min),
            cpu: (map(.time_cpu.value) | mean),
            instructions: (if all(.[]; .instructions.value != null) then map(.instructions.value) | mean else null end),
            rss: (map(.memory_peak_resident.value) | max),
            times: $t
        }' "$out/raw/$1.$2.$3.jsonl" >>"$out/results.jsonl"
}

# Times every scenario on one corpus. Runs interleave the variants, rotating
# which goes first each round, so drift over a long job spreads across all of
# them instead of favoring whichever ran last.
bench_corpus() { # corpus dir
    local corpus=$1 dir=$2 leaf hub build check scenario file prepare cmd n i j variant
    local -a args
    local count=${#variants[@]}
    mapfile -t args < <(jq -r --arg n "$corpus" '.corpus[] | select(.name == $n) | .args[]' <<<"$config")
    leaf=$(field corpus "$corpus" .leaf)
    hub=$(field corpus "$corpus" .hub)
    build=$(printf '%q ' cargo build --locked --offline -q "${args[@]}")
    check=$(printf '%q ' cargo check --locked --offline -q "${args[@]}")

    # An untimed clean build per variant warms the disk cache and shows any failure in full.
    for variant in "${variants[@]}"; do
        echo "bench: $corpus warm-up $variant"
        (
            use_variant "$corpus" "$variant"
            rm -rf "$CARGO_TARGET_DIR"
            cd "$dir"
            cargo build --locked --offline "${args[@]}" >"$out/logs/$corpus.$variant.log" 2>&1
        ) || { tail -40 "$out/logs/$corpus.$variant.log" >&2 && die "$corpus failed to build with $variant"; }
    done

    for scenario in check build edit-leaf edit-hub; do
        case $scenario in
            check | build)
                [[ $scenario == check ]] && cmd=$check || cmd=$build
                prepare="cargo clean -q"
                n=${runs:-3}
                ;;
            edit-*)
                [[ $scenario == edit-leaf ]] && file=$leaf || file=$hub
                [[ -n $file ]] || continue
                cmd=$build
                prepare=$(printf '%q ' "$bench/run.sh" edit "$file")
                n=${runs:-5}
                # Add the marker and build it, so each timed run is a body edit.
                "$bench/run.sh" edit "$dir/$file"
                for variant in "${variants[@]}"; do
                    (use_variant "$corpus" "$variant" && cd "$dir" && cargo build --locked --offline -q "${args[@]}")
                done
                ;;
        esac
        for ((i = 0; i < n; i++)); do
            for ((j = 0; j < count; j++)); do
                variant=${variants[(i + j) % count]}
                time_once "$corpus" "$dir" "$scenario" "$variant" "$prepare" "$cmd"
            done
        done
        for variant in "${variants[@]}"; do
            summarize_runs "$corpus" "$scenario" "$variant"
        done
    done

    git -C "$dir" checkout -q -- .
    rm -rf "$work/target/$corpus"
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
    bench_corpus "$corpus" "$dir"
    # A subshell keeps the nightly toolchain and flags from leaking into the next corpus.
    ((attribute == 0)) || (attribute "$corpus" "$dir")
done

report "$out"
mkdir -p "$results"
ln -sfn "$out" "$results/latest"
echo "bench: report at $out/report.md"
