#!/usr/bin/env bash
# Measures what each of moq's binding wrappers adds over Rust, with moq-ffi at
# the revision in pins.toml built against this tree's UniFFI fork.
# Usage and output are described in README.md.
set -euo pipefail

bench=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$bench/../.." && pwd)

die() {
    echo "bench-ffi: $*" >&2
    exit 1
}

usage() {
    cat <<'EOF'
Usage: just bench-ffi [OPTIONS]

Options:
  --runner NAME   Measure only this wrapper (repeatable): go, py, kt, cpp, dart,
                  browser, or swift (macOS). Default: every one this host runs.
                  The Rust baselines always run.
  --rounds N      Timed rounds per variant (default: 3)
  --seconds S     Time budget per case and round (default: 1)
  --base REV      Also measure the fork at this rust2 revision, as `base`
  --out DIR       Write results to this new or empty directory
                  (default: target/bench/ffi/<UTC time>)
EOF
}

# The result files each runner writes into MOQ_BENCH_DIR.
outputs() {
    case $1 in
        rust) echo rust-ffi rust-net ;;
        py) echo python ;;
        kt) echo kotlin ;;
        *) echo "$1" ;;
    esac
}

# Summarizes a results directory's raw samples and renders its report.md. One
# sample per runner, case, variant, and round, then one record per runner, case,
# and variant. A wrapper's ratio to rust-ffi is taken within each round and
# variant, so drift between rounds cancels. Medians, so one slow round on a busy
# host does not move a comparison.
report() {
    local dir=$1 file rel
    for file in "$dir"/raw/*/[0-9]*/*.json; do
        rel=${file#"$dir/raw/"}
        jq -c --arg variant "${rel%%/*}" --argjson round "$(cut -d/ -f2 <<<"$rel")" --arg runner "$(basename "$file" .json)" \
            'to_entries[] | .value + {$variant, $round, $runner, order: .key} | {variant, round, runner, name, unit, value, order}' "$file"
    done >"$dir/samples.jsonl"
    jq -s -c --slurpfile host "$dir/host.json" '
        def mean: add / length;
        def median: sort | if length % 2 == 1 then .[length / 2 | floor] else (.[length / 2 - 1] + .[length / 2]) / 2 end;
        def worse($b): if .unit == "MB/s" then $b / .value else .value / $b end;
        (map(select(.runner == "rust-ffi") | {key: "\(.variant)/\(.round)/\(.name)", value: .value}) | from_entries) as $rust
        | map(. + {ratio: (if .runner == "rust-ffi" then null else worse($rust["\(.variant)/\(.round)/\(.name)"]) end)})
        | group_by([.variant, .runner, .name])[]
        | map(.value) as $v
        | ($v | mean) as $mean
        | {
            variant: .[0].variant,
            base: ({control: "fork", fork: (if $host[0].base == "" then null else "base" end)}[.[0].variant]),
            runner: .[0].runner,
            name: .[0].name,
            unit: .[0].unit,
            median: ($v | median),
            $mean,
            stddev: (if ($v | length) > 1 then $v | map(. - $mean | . * .) | add / (length - 1) | sqrt else null end),
            min: ($v | min),
            max: ($v | max),
            ratio: (if .[0].ratio == null then null else map(.ratio) | median end),
            values: $v,
            order: (map(.order) | min)
        }' "$dir/samples.jsonl" >"$dir/results.jsonl"
    jq -n -r -f "$bench/report.jq" --slurpfile host "$dir/host.json" \
        --slurpfile results "$dir/results.jsonl" >"$dir/report.md"
}

# Inside moq's dev shell: one untimed round per checkout, then the timed rounds,
# interleaving the variants and rotating which goes first each round.
measure() {
    local out=$1 plan seconds rounds ubrn count i j variant checkout
    local -a runners variants
    plan=$out/plan.json
    seconds=$(jq -r .seconds "$plan")
    rounds=$(jq -r .rounds "$plan")
    ubrn=$(jq -r .ubrn "$plan")
    mapfile -t runners < <(jq -r '.runners[]' "$plan")
    mapfile -t variants < <(jq -r '.variants[].name' "$plan")
    count=${#variants[@]}

    jq -n \
        --arg rustc "$(rustc -V)" \
        --arg python "$(python3 --version)" \
        --arg go "$(go version 2>/dev/null || echo missing)" \
        --arg java "$(java -version 2>&1 | head -1 || echo missing)" \
        --arg dart "$(dart --version 2>&1 || echo missing)" \
        --arg cmake "$(cmake --version 2>/dev/null | head -1 || echo missing)" \
        --arg cxx "$(c++ --version 2>/dev/null | head -1 || echo missing)" \
        --arg swift "$(swift --version 2>/dev/null | head -1 || echo missing)" \
        --arg playwright "Playwright ${PLAYWRIGHT_VERSION:-missing}" \
        '$ARGS.named' >"$out/tools.json"

    # One round, timed or not, of every runner on one variant.
    run_round() { # variant round
        local variant=$1 round=$2 checkout generators browser dest runner log file
        checkout=$(jq -r --arg n "$variant" '.variants[] | select(.name == $n) | .checkout' "$plan")
        generators=$(jq -r --arg n "$variant" '.variants[] | select(.name == $n) | .generators' "$plan")
        browser=$(jq -r --arg n "$variant" '.variants[] | select(.name == $n) | .browser' "$plan")
        dest=$out/raw/$variant/$round
        mkdir -p "$dest"
        for runner in rust "${runners[@]}"; do
            log=$out/logs/$variant.$round.$runner.log
            echo "bench-ffi: $variant round $round $runner"
            (
                cd "$checkout"
                export PATH=$generators:$PATH MOQ_BENCH_DIR=$dest MOQ_BENCH_SECONDS=$seconds
                case $runner in
                    rust) just rs test -p moq-ffi --cargo-profile release -E 'test(bench::benchmark_ffi)' --run-ignored only ;;
                    go) just go bench -benchtime="${seconds}s" ;;
                    browser) "$bench/browser.sh" "$checkout" "$browser" "$ubrn" ;;
                    *) just "$runner" bench ;;
                esac
            ) >"$log" 2>&1 || { tail -40 "$log" >&2 && die "$runner failed on $variant; see $log"; }
            for file in $(outputs "$runner"); do
                [[ -s $dest/$file.json ]] || die "$runner wrote no $file.json on $variant; see $log"
            done
        done
    }

    # The warm-up builds every runner once per checkout, so no build lands in a timed round.
    local -A warmed=()
    for variant in "${variants[@]}"; do
        checkout=$(jq -r --arg n "$variant" '.variants[] | select(.name == $n) | .checkout' "$plan")
        [[ -z ${warmed[$checkout]:-} ]] || continue
        warmed[$checkout]=1
        run_round "$variant" warmup
    done
    for ((i = 0; i < rounds; i++)); do
        for ((j = 0; j < count; j++)); do
            run_round "${variants[(i + j) % count]}" "$i"
        done
    done
}

case ${1:-} in
    measure)
        measure "${2:?measure needs a results directory}"
        exit 0
        ;;
    report)
        report "${2:?report needs a results directory}"
        exit 0
        ;;
esac

all_runners=(go py kt cpp dart browser)
[[ $(uname -s) != Darwin ]] || all_runners+=(swift)
runners=()
rounds=3
seconds=1
base=
out=
while (($#)); do
    case $1 in
        --runner) runners+=("${2:?--runner needs a name}") && shift 2 ;;
        --rounds) rounds=${2:?--rounds needs a count} && shift 2 ;;
        --seconds) seconds=${2:?--seconds needs a duration} && shift 2 ;;
        --base) base=${2:?--base needs a revision} && shift 2 ;;
        --out) out=${2:?--out needs a directory} && shift 2 ;;
        -h | --help) usage && exit 0 ;;
        *) usage >&2 && die "unknown argument: $1" ;;
    esac
done
[[ $rounds =~ ^[1-9][0-9]*$ ]] || die "--rounds must be a positive integer, got $rounds"
[[ $seconds =~ ^[0-9]*\.?[0-9]+$ && $seconds != 0 ]] || die "--seconds must be a positive number, got $seconds"
((${#runners[@]})) || runners=("${all_runners[@]}")
for runner in "${runners[@]}"; do
    [[ " ${all_runners[*]} " == *" $runner "* ]] || die "unknown runner on this host: $runner"
done

# Cargo reads every .cargo/config.toml above its working directory, and a user's
# (say, a target-cpu rustflag) would change the code measured. Build outside
# $HOME, with a private CARGO_HOME and none of the caller's cargo or rustc variables.
work=/var/tmp/rust2-bench-$(id -un)/ffi
results=$root/target/bench/ffi
out=${out:-$results/$(date -u +%Y%m%dT%H%M%SZ)}
[[ -z $(find "$out" -mindepth 1 -print -quit 2>/dev/null) ]] || die "$out is not empty; pass a new --out"
mkdir -p "$work" "$out/logs"
out=$(cd "$out" && pwd)
dir=$work
while :; do
    for f in "$dir/.cargo/config" "$dir/.cargo/config.toml"; do
        [[ ! -e $f ]] || die "$f would configure the builds; move it"
    done
    [[ $dir != / ]] || break
    dir=$(dirname "$dir")
done
caller_vars=("${!CARGO@}" "${!RUSTC@}" "${!RUSTDOC@}" "${!RUSTUP@}")
[[ -z ${RUSTFLAGS+set} ]] || caller_vars+=(RUSTFLAGS)
if ((${#caller_vars[@]})); then
    echo "bench-ffi: ignoring ${caller_vars[*]}"
    unset "${caller_vars[@]}"
fi
export CARGO_HOME=$work/cargo-home

moq_git=$(taplo get -f "$bench/pins.toml" moq.git)
moq_rev=$(taplo get -f "$bench/pins.toml" moq.rev)
ubrn_git=$(taplo get -f "$bench/pins.toml" ubrn.git)
ubrn_rev=$(taplo get -f "$bench/pins.toml" ubrn.rev)
fork_rev=$(git -C "$root" rev-parse HEAD)
[[ -z $(git -C "$root" status --porcelain -- uniffi Cargo.toml Cargo.lock) ]] || fork_rev+="-dirty"
base_rev=
if [[ -n $base ]]; then
    base_rev=$(git -C "$root" rev-parse --verify "$base^{commit}") || die "unknown revision: $base"
fi

# Builds the fork's Go, Dart, and C++ generators from a rust2 tree. moq's scripts
# find them on PATH; its own Kotlin, Python, and Swift generator builds from the
# redirected `uniffi` crate.
generators() { # tree dest
    local tree=$1 dest=$2 bin
    CARGO_TARGET_DIR=$dest/target cargo build --locked -q --manifest-path "$tree/Cargo.toml" \
        -p uniffi-bindgen-go -p uniffi-bindgen-cpp -p uniffi-dart --features uniffi-dart/binary --bins
    mkdir -p "$dest/bin"
    for bin in uniffi-bindgen-go uniffi-bindgen-cpp uniffi_bindgen_dart; do
        ln -sfn "$dest/target/debug/$bin" "$dest/bin/$bin"
    done
}

# Clones a repository at a pinned revision, undoing anything an earlier run left
# in its tracked files.
clone() { # git rev dir
    local url=$1 rev=$2 dir=$3
    if [[ ! -d $dir/.git ]]; then
        git init -q "$dir"
        git -C "$dir" remote add origin "$url"
    fi
    if [[ $(git -C "$dir" rev-parse -q --verify HEAD || true) != "$rev" ]]; then
        git -C "$dir" fetch -q --depth 1 origin "$rev"
        git -C "$dir" checkout -q --detach FETCH_HEAD
    fi
    git -C "$dir" reset -q --hard "$rev"
    git -C "$dir" clean -qfd
}

# Builds ubrn's CLI and packs its two TypeScript runtimes, @ubjs/core and @ubjs/wasm.
ubrn() { # dir
    local dir=$1
    clone "$ubrn_git" "$ubrn_rev" "$dir/src"
    [[ $(cat "$dir/stamp" 2>/dev/null) == "$ubrn_rev" ]] && return
    rm -rf "${dir:?}/bin" "${dir:?}/pkgs"
    cargo install --locked -q --root "$dir" --path "$dir/src/crates/ubrn_cli"
    mkdir -p "$dir/pkgs"
    (cd "$dir/src/typescript" && npm ci -s --no-audit --no-fund && npm run -s build &&
        npm pack -s --pack-destination "$dir/pkgs" >/dev/null)
    (cd "$dir/src/runtimes/wasm" && npm ci -s --no-audit --no-fund && npm pack -s --pack-destination "$dir/pkgs" >/dev/null)
    mv "$dir"/pkgs/ubjs-core-*.tgz "$dir/pkgs/ubjs-core.tgz"
    mv "$dir"/pkgs/ubjs-wasm-*.tgz "$dir/pkgs/ubjs-wasm.tgz"
    echo "$ubrn_rev" >"$dir/stamp"
}

# Clones moq at the pin and points every `uniffi` dependency at the fork in `tree`.
checkout() { # tree dir
    local tree=$1 dir=$2 strays
    clone "$moq_git" "$moq_rev" "$dir"
    # A crate keeps its upstream name but not its version, so `[patch]` cannot
    # redirect it: rewrite each `uniffi` dependency's source to the fork's path.
    git -C "$dir" ls-files -z '*Cargo.toml' | (cd "$dir" && xargs -0 sed -i -E \
        -e "s#^(uniffi = )\"[^\"]*\"\$#\\1{ path = \"$tree/uniffi/uniffi\" }#" \
        -e "s#^(uniffi = \\{ )(version = \"[^\"]*\"|git = \"[^\"]*\", (rev|tag|branch) = \"[^\"]*\")#\\1path = \"$tree/uniffi/uniffi\"#")
    # Resolving without --locked updates the lockfile to the fork, so moq's own
    # --locked builds accept it. Every uniffi crate must then be the fork's.
    strays=$(cargo metadata --format-version 1 --manifest-path "$dir/Cargo.toml" | jq -r --arg fork "$tree/uniffi/" '
        .packages[]
        | select(.name | test("^uniffi(_|$)"))
        | select(.manifest_path | startswith($fork) | not)
        | "\(.name) \(.version) from \(.source // .manifest_path)"')
    [[ -z $strays ]] || die "moq still builds upstream UniFFI: $(tr '\n' ' ' <<<"$strays")"
    # ubrn's player calls an allocator and panic hook its helper crate exports on wasm32.
    if [[ -n $browser ]]; then
        sed -i "/^uniffi = { workspace = true, features = \[\"wasm-unstable-single-threaded\"\] }\$/a uniffi-runtime-wasm = { path = \"$work/ubrn/src/runtimes/wasm/helper-crate\" }" \
            "$dir/rs/moq-ffi/Cargo.toml"
        sed -i 's/^uniffi::setup_scaffolding!/#[cfg(target_arch = "wasm32")]\nextern crate uniffi_runtime_wasm as _;\n\n&/' "$dir/rs/moq-ffi/src/lib.rs"
        if ! grep -q '^uniffi-runtime-wasm = ' "$dir/rs/moq-ffi/Cargo.toml" ||
            ! grep -q '^extern crate uniffi_runtime_wasm' "$dir/rs/moq-ffi/src/lib.rs"; then
            die "could not link ubrn's runtime into moq-ffi; its manifest or crate root moved"
        fi
        cargo metadata --format-version 1 --manifest-path "$dir/Cargo.toml" >/dev/null
    fi
}

browser=
if [[ " ${runners[*]} " == *" browser "* ]]; then
    browser=1
    echo "bench-ffi: preparing ubrn"
    ubrn "$work/ubrn"
fi

variants=()
add_variant() { # name tree
    variants+=("$(jq -n -c --arg name "$1" --arg checkout "$work/moq/$1" --arg generators "$work/generators/$1/bin" \
        --arg browser "$work/browser/$1" '$ARGS.named')")
    echo "bench-ffi: preparing $1"
    generators "$2" "$work/generators/$1"
    checkout "$2" "$work/moq/$1"
}
add_variant fork "$root"
# The A/A control reruns fork's builds, so its change is the run's noise floor.
variants+=("$(jq -c '.name = "control"' <<<"${variants[0]}")")
if [[ -n $base_rev ]]; then
    tree=$work/rust2-base
    git -C "$root" worktree remove --force "$tree" 2>/dev/null || rm -rf "$tree"
    git -C "$root" worktree add -q --detach "$tree" "$base_rev"
    trap 'git -C "$root" worktree remove --force "$tree"' EXIT
    add_variant base "$tree"
fi

jq -n \
    --argjson rounds "$rounds" \
    --arg seconds "$seconds" \
    --arg ubrn "$work/ubrn" \
    --args '{$rounds, $seconds, $ubrn, runners: $ARGS.positional}' "${runners[@]}" |
    jq -c --slurpfile variants <(printf '%s\n' "${variants[@]}") '. + {$variants}' >"$out/plan.json"

# Linux reads /proc; macOS has none and answers through sysctl.
if [[ -r /proc/cpuinfo ]]; then
    cpu=$(sed -n 's/^model name[[:space:]]*: //p' /proc/cpuinfo | head -1)
    memory=$(awk '/MemTotal/ { printf "%.0f GiB", $2 / 1048576 }' /proc/meminfo)
else
    cpu=$(sysctl -n machdep.cpu.brand_string)
    memory="$(($(sysctl -n hw.memsize) / 1073741824)) GiB"
fi
load() {
    if [[ -r /proc/loadavg ]]; then cut -d' ' -f1-3 /proc/loadavg; else sysctl -n vm.loadavg | tr -d '{}' | xargs; fi
}
load_start=$(load)

# The dev shell of the pinned moq carries every wrapper's toolchain.
nix develop "github:moq-dev/moq/$moq_rev" --command "$bench/run.sh" measure "$out"

jq -n \
    --arg date "$(date -u +%FT%TZ)" \
    --arg os "$(uname -srm)" \
    --arg cpu "$cpu" \
    --arg cpus "$(getconf _NPROCESSORS_ONLN)" \
    --arg memory "$memory" \
    --arg runner "${RUNNER_NAME:-local}" \
    --arg load_start "$load_start" \
    --arg load_end "$(load)" \
    --arg rust2 "$fork_rev" \
    --arg base "$base_rev" \
    --arg moq "$moq_rev" \
    --arg ubrn "${browser:+$ubrn_rev}" \
    --argjson rounds "$rounds" \
    --arg seconds "$seconds" \
    --slurpfile tools "$out/tools.json" \
    '$ARGS.named + {tools: $tools[0]}' >"$out/host.json"

report "$out"
mkdir -p "$results"
ln -sfn "$out" "$results/latest"
echo "bench-ffi: report at $out/report.md"
