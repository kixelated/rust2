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
      url = "github:kixelated/quest/a73fb295a3679ff54205af68a1a7bb8ec5bdc7af";
      inputs.nixpkgs.follows = "nixpkgs";
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
                src = ./ts;
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
        in
        {
          packages.quest = quest-cli;
          # `just bench` enters this shell; the nightly toolchain stays off PATH
          # so stable remains the default.
          devShells.build-bench = pkgs.mkShell {
            inputsFrom = [ self.devShells.${system}.default ];
            packages =
              with pkgs;
              [
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
          packages.ts = rust2-ts;
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
              nixfmt
              # The TypeScript driver's tests run and type-check its output.
              nodejs
              typescript
            ];
            RUST2_TS_TOOLCHAIN = tsToolchain;
          };
          # `just ts bench` adds Bun and Chrome as engines, and wasm-bindgen for moq-wasm's
          # package, which must match the moq revision's or fails. Apart for their size.
          devShells.bench = self.devShells.${system}.default.overrideAttrs (old: {
            nativeBuildInputs = old.nativeBuildInputs ++ [
              pkgs.bun
              pkgs.chromium
              pkgs.wasm-bindgen-cli
            ];
          });
          formatter = pkgs.nixfmt;
          checks.quest-tree = pkgs.runCommand "quest-tree" { nativeBuildInputs = [ quest-cli ]; } ''
            quest --root ${self} check
            touch "$out"
          '';
        }
      );
}
