# Build-speed benchmark

`just bench` measures Rust build time on the pinned projects and build configurations in [corpus.toml](corpus.toml).
It runs nightly in CI ([bench.yml](../../.github/workflows/bench.yml)), and every build-speed change here states its effect with its numbers.

```bash
just bench                                    # everything; takes hours
just bench --corpus regex --runs 1            # a quick smoke test
just bench --corpus moq --variant baseline --variant mold
```

The script enters the `build-bench` dev shell itself, which adds hyperfine, mold, wild, measureme, and a pinned nightly to the default shell.
Results land in `target/bench/results/`, newest at `target/bench/results/latest/report.md`.
Checkouts and builds live in `/var/tmp/rust2-bench-<user>/` with their own `CARGO_HOME`, since cargo reads every `.cargo/config.toml` above the directory it runs in and a user-level one (such as `build.incremental = false`) would change what is measured.
The run also ignores the caller's `CARGO*`, `RUSTC*`, `RUSTDOC*`, `RUSTUP*`, and `RUSTFLAGS` variables, and `CI` and `TF_BUILD`, which make newer cargo skip incremental compilation.

## Corpus

- **moq** builds `moq-relay`, which pulls in moq-net, the QUIC stack, and C code through aws-lc-sys.
- **moq-pure-crypto** builds the same `moq-relay` with its crypto on graviola and RustCrypto, so it compiles no C.
- **regex** and **wasmparser** are pure Rust: the run fails if any crate in their build gains a build script or a `links` key.

Each corpus pins a git revision (and a lockfile when the project commits none).
Bump a pin in its own change, since numbers on either side of it do not compare.

## Scenarios

- `check`: a clean `cargo check`.
- `build`: a clean debug `cargo build`, dependencies included.
- `edit-leaf`: rebuild after a body edit in a crate nothing else in the build depends on.
- `edit-hub`: the same edit in a crate most of the build depends on (moq-net in moq).

An edit rewrites a marker function appended to the crate root, so every run changes code that reaches codegen.
Before the timed runs, each variant does one untimed clean build that warms the disk cache and logs any failure.

## Variants

A variant is a toolchain plus rustflags.
Each is compared against its `base`, run in the same job: the levers on stable against `baseline`, the nightly ones against `nightly`.
The `control` variants repeat their base, so their change is the run's noise floor: an A/A control.
Timed runs interleave the variants and rotate which goes first each round, so drift over a long job spreads across all of them.
To measure a new build-speed change, add a variant whose base is the configuration it improves on.

The report shows wall time (mean ± standard deviation, and min), CPU time, retired instructions, and peak RSS, all from hyperfine.
Instructions barely move with machine load, so they are the signal on a busy machine, where wall time is only a smoke test; they read `n/a` where the host exposes no performance counters.
No threshold is set: the numbers inform a decision, they do not gate one.

## Attribution

After the timed variants, one nightly build per corpus (clean, then each edit) records cargo's build analysis log with section timings and rustc's self-profile.
The report splits the unit-seconds of each step by kind of unit (build script compile and run, proc macro, dependency, workspace lib, binary) and by compiler section (frontend, codegen, link), and adds macro and proc-macro expansion time and the costliest queries.
Self-profiling slows the build a little, so read this section for shares, not absolute times.
