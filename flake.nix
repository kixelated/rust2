{
  description = "rust2: Rust to other languages and faster builds";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # The quest CLI, which also serves the quest guide and skills the stubs in
    # .claude/skills call. Bump the rev to upgrade them.
    quest = {
      url = "github:kixelated/quest/ed657ba9362a941b3be9bbea55120a3c5a90374c";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    # The daemon the benchmark's ra-jit variant measures. Bumping the rev is a
    # benchmark corpus change: numbers before and after it do not compare.
    ra-jit = {
      url = "github:indexable-inc/ra-jit/d4f6027e82d312bf805ebfe6252369f6048a1e18";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-overlay.follows = "rust-overlay";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      quest,
      ra-jit,
      ...
    }:
    flake-utils.lib.eachSystem
      [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ]
      (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
          toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          # The benchmark's nightly levers (--jobs-frontend, cranelift, self-profile).
          # Bumping the date is a benchmark corpus change: numbers before and
          # after it do not compare.
          nightly = pkgs.rust-bin.nightly."2026-10-22".minimal.override {
            extensions = [ "rustc-codegen-cranelift-preview" ];
          };
          # ra-jit works with one rustc only. This is the toolchain its flake builds
          # with, so the daemon and the variant's plain cargo share it.
          raJitToolchain = pkgs.rust-bin.stable."1.99.0".default.override {
            extensions = [ "rust-src" ];
          };
          # The TypeScript driver's own nightly, kept off PATH so the other projects keep
          # the repo toolchain. ts/justfile puts it first for its recipes.
          tsToolchain = pkgs.rust-bin.fromRustupToolchainFile ./ts/rust-toolchain.toml;
          quest-cli = quest.packages.${system}.default;
          # The driver links its nightly's rustc libraries, and runs cargo from it too.
          rust2-ts =
            (pkgs.makeRustPlatform {
              cargo = tsToolchain;
              rustc = tsToolchain;
            }).buildRustPackage
              {
                pname = "rust2-ts";
                version = "0.0.0";
                # With the driver front end it shares, which it depends on by path.
                src = pkgs.lib.fileset.toSource {
                  root = ./.;
                  fileset = pkgs.lib.fileset.unions [
                    ./ts
                    ./driver
                  ];
                };
                buildAndTestSubdir = "ts";
                cargoRoot = "ts";
                cargoLock.lockFile = ./ts/Cargo.lock;
                cargoBuildFlags = [
                  "--package"
                  "rust2-ts"
                ];
                RUSTFLAGS = "-C link-args=-Wl,-rpath,${tsToolchain}/lib";
                nativeBuildInputs = [ pkgs.makeWrapper ];
                postInstall = ''
                  wrapProgram $out/bin/rust2-ts --set-default CARGO ${tsToolchain}/bin/cargo
                '';
                # The tests need node, tsc, and wasm builds: `just ts test` runs them.
                doCheck = false;
              };

          # Every generator binary of the UniFFI fork, built once from its lockfile.
          uniffi =
            (pkgs.makeRustPlatform {
              cargo = toolchain;
              rustc = toolchain;
            }).buildRustPackage
              {
                pname = "rust2-uniffi";
                version = (builtins.fromTOML (builtins.readFile ./uniffi/uniffi/Cargo.toml)).package.version;
                # The root workspace, without the other projects.
                src = pkgs.lib.fileset.toSource {
                  root = ./.;
                  fileset = pkgs.lib.fileset.unions [
                    ./Cargo.toml
                    ./Cargo.lock
                    ./uniffi
                  ];
                };
                cargoLock.lockFile = ./Cargo.lock;
                cargoBuildFlags = [
                  "--package=uniffi"
                  "--package=uniffi-bindgen-go"
                  "--package=uniffi-dart"
                  "--package=uniffi-bindgen-cpp"
                  "--bins"
                ];
                buildFeatures = [
                  "uniffi/cli"
                  "uniffi-dart/binary"
                ];
                # The fixture suite needs every foreign toolchain; `just uniffi fixtures` runs it.
                doCheck = false;
              };

          # One generator's binaries, so a consumer can put only that one on PATH.
          generator =
            name: bins:
            pkgs.runCommand name { } ''
              mkdir -p $out/bin
              ${pkgs.lib.concatMapStringsSep "\n" (bin: "ln -s ${uniffi}/bin/${bin} $out/bin/") bins}
            '';

          # Kotlin's generated async code needs coroutines on the classpath; nixpkgs has no jar.
          kotlinx-coroutines = pkgs.fetchurl {
            url = "https://repo1.maven.org/maven2/org/jetbrains/kotlinx/kotlinx-coroutines-core-jvm/1.10.2/kotlinx-coroutines-core-jvm-1.10.2.jar";
            hash = "sha256-XKF1s43zMf1kFVs1zYyuElH6nuNpcJs21C4KKIzM4/0=";
          };
        in
        {
          packages = {
            quest = quest-cli;
            ts = rust2-ts;
            inherit uniffi;
            uniffi-bindgen = generator "uniffi-bindgen" [
              "uniffi-bindgen"
              "uniffi-bindgen-swift"
            ];
            uniffi-bindgen-go = generator "uniffi-bindgen-go" [ "uniffi-bindgen-go" ];
            uniffi-bindgen-dart = generator "uniffi-bindgen-dart" [ "uniffi_bindgen_dart" ];
            uniffi-bindgen-cpp = generator "uniffi-bindgen-cpp" [ "uniffi-bindgen-cpp" ];
          }
          # ra-jit and its toolchain for the benchmark's ra-jit variants, which build it
          # only when they run: it compiles from source. ra-jit skips Intel macOS.
          // pkgs.lib.optionalAttrs (ra-jit.packages ? ${system}) {
            bench-ra-jit = pkgs.symlinkJoin {
              name = "bench-ra-jit";
              paths = [
                ra-jit.packages.${system}.ra-jit
                raJitToolchain
              ];
            };
          };
          # `just bench` enters this shell; the nightly toolchain stays off PATH
          # so stable remains the default. It holds only what the benchmark runs:
          # a PR runs bench.yml only when this shell, or another output it lists
          # (bench-ra-jit), changes derivation.
          devShells.build-bench = pkgs.mkShell {
            packages =
              with pkgs;
              [
                toolchain
                just
                jq
                git
                taplo
                hyperfine
                measureme
              ]
              # The alternative linkers the stable variants measure are ELF-only.
              ++ lib.optionals stdenv.hostPlatform.isLinux [
                mold
                wild
              ];
            RUST2_BENCH_NIGHTLY = "${nightly}/bin";
          };
          devShells.default = pkgs.mkShell {
            packages = with pkgs; [
              toolchain
              quest-cli
              just
              jq
              git
              gh
              cargo-nextest
              cargo-deny
              cargo-shear
              cargo-sort
              actionlint
              shellcheck
              shfmt
              taplo
              # bench/ffi/run.sh serializes runs that share a work directory.
              flock
              nixfmt
              # The TypeScript driver's tests run and type-check its output, and bundle a
              # caller with Bun and esbuild to check that unused output shakes out.
              nodejs
              typescript
              bun
              esbuild
            ];
            RUST2_TS_TOOLCHAIN = tsToolchain;
          };
          # `just ts bench` adds Chrome as an engine, and wasm-bindgen for moq-wasm's package,
          # which must match the moq revision's or fails. Apart for their size.
          devShells.bench = self.devShells.${system}.default.overrideAttrs (old: {
            nativeBuildInputs = old.nativeBuildInputs ++ [
              pkgs.chromium
              pkgs.wasm-bindgen-cli
            ];
            # `just ffi bench` builds moq's rs/moq-c, whose VAAPI bindings run bindgen.
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          });
          # Every foreign toolchain the UniFFI fixture suite runs, apart for their size.
          devShells.uniffi = self.devShells.${system}.default.overrideAttrs (old: {
            nativeBuildInputs = old.nativeBuildInputs ++ [
              pkgs.python3
              pkgs.kotlin
              pkgs.swift
              pkgs.go
              pkgs.dart
              pkgs.cmake
            ];
            CLASSPATH = "${pkgs.jna}/share/java/jna.jar:${kotlinx-coroutines}";
          });
          formatter = pkgs.nixfmt;
          checks.quest-tree = pkgs.runCommand "quest-tree" { nativeBuildInputs = [ quest-cli ]; } ''
            quest --root ${self} check
            touch "$out"
          '';
        }
      );
}
