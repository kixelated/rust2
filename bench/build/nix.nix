# moq's Nix packages, built each way nix.sh compares. Evaluated with --impure,
# since the checkout is a path the run edits.
{
  # moq's checkout, as an absolute path string: a path literal would copy it to
  # the store first. crate2nix's Cargo.nix sits at its root, beside the crates
  # it names by relative path.
  src,
  # mbx's store path, and the sandbox path its store is mounted at.
  mbx,
  mbxStore,
}:
let
  system = builtins.currentSystem;
  # moq's own nixpkgs, rust-overlay, and crane, from its lockfile.
  lock = builtins.fromJSON (builtins.readFile "${src}/flake.lock");
  locked = name: lock.nodes.${lock.nodes.root.inputs.${name}}.locked;
  pkgs = import (builtins.fetchTree (locked "nixpkgs")) {
    inherit system;
    overlays = [ (import (builtins.fetchTree (locked "rust-overlay"))) ];
  };
  inherit (pkgs) lib;
  crane' = with locked "crane"; builtins.getFlake "github:${owner}/${repo}/${rev}";

  # The toolchain moq's overlay builds with, which pins the same channel.
  channel = (lib.importTOML "${src}/rust-toolchain.toml").toolchain.channel;
  toolchain = pkgs.rust-bin.stable.${channel}.default;

  # What nix/overlay.nix passes cargo for each package; crate2nix and cargo
  # resolve the same features from it.
  packages = {
    moq-relay = [ "jemalloc" ];
    moq-cli = [ "jemalloc" ];
  };
  rustflags = [
    "-C"
    "force-frame-pointers=yes"
  ];

  # crane as moq builds it: its flake's overlay, keyed by cargo package.
  moq = pkgs.extend (import "${src}/nix/overlay.nix" { crane = crane'; });
  crane = lib.mapAttrs (name: _: moq.${name}) packages;

  # crane with no dependency layer: every compilation goes through mbx, on a
  # store outside the sandbox. `cargo` on PATH is mbx's shim, which finds the
  # real cargo after its own directory, so that directory goes on PATH as is.
  craneMbx = lib.mapAttrs (
    _: drv:
    drv.overrideAttrs (old: {
      cargoArtifacts = null;
      preConfigure = ''
        export PATH=${builtins.storePath mbx}/libexec/mbx:$PATH
      ''
      + (old.preConfigure or "");
      # mbx would move a target directory it finds into its store.
      CARGO_TARGET_DIR = "target";
      MBX_CACHE_DIR = mbxStore;
      MBX_GC_AUTO = "0";
      # The report lands in the output, where nix.sh counts hits and misses.
      preBuild = (old.preBuild or "") + ''
        export MBX_STATS_REPORT=$NIX_BUILD_TOP/mbx-report.json
      '';
      postInstall = (old.postInstall or "") + ''
        install -Dm644 $MBX_STATS_REPORT $out/share/mbx/report.json
      '';
    })
  ) crane;

  # One derivation per crate, with crate2nix's generated build plan.
  crate2nix =
    let
      buildRustCrate = pkgs.buildRustCrate.override {
        rustc = toolchain;
        cargo = toolchain;
        # Cargo's release default, which the other variants build with, so they
        # compare caching and not codegen. buildRustCrate's own is 1.
        defaultCodegenUnits = 16;
      };
      plan = pkgs.callPackage "${src}/Cargo.nix" {
        buildRustCrateForPkgs =
          _: crate:
          buildRustCrate (crate // { extraRustcOpts = rustflags ++ (crate.extraRustcOpts or [ ]); });
      };
    in
    lib.mapAttrs (
      name: features: plan.workspaceMembers.${name}.build.override { inherit features; }
    ) packages;
in
{
  inherit
    crane
    craneMbx
    crate2nix
    toolchain
    packages
    rustflags
    ;
}
