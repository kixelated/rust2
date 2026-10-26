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
  # Only crane's variants evaluate moq's flake, which copies the whole checkout.
  moq = builtins.getFlake "path:${src}";
  system = builtins.currentSystem;
  # moq's own nixpkgs and rust-overlay, from its lockfile.
  lock = builtins.fromJSON (builtins.readFile "${src}/flake.lock");
  input = name: builtins.fetchTree lock.nodes.${lock.nodes.root.inputs.${name}}.locked;
  pkgs = import (input "nixpkgs") {
    inherit system;
    overlays = [ (import (input "rust-overlay")) ];
  };
  inherit (pkgs) lib;

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

  # crane as moq builds it.
  crane = lib.mapAttrs (name: _: moq.packages.${system}.${name}) packages;

  # crane with no dependency layer: every compilation goes through mbx, on a
  # store outside the sandbox. `cargo` on PATH is mbx's shim.
  mbxCargo = pkgs.runCommand "mbx-cargo" { } ''
    mkdir -p $out/bin
    ln -s ${builtins.storePath mbx}/libexec/mbx/cargo $out/bin/cargo
  '';
  craneMbx = lib.mapAttrs (
    _: drv:
    drv.overrideAttrs (old: {
      cargoArtifacts = null;
      nativeBuildInputs = [ mbxCargo ] ++ old.nativeBuildInputs;
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
        # Cargo's release default, which the other ways build with; buildRustCrate's is 1.
        defaultCodegenUnits = 16;
        defaultCrateOverrides = pkgs.defaultCrateOverrides // overrides;
      };
      # Crates whose build scripts need more than nixpkgs' overrides give them.
      overrides = { };
      plan = pkgs.callPackage "${src}/Cargo.nix" {
        buildRustCrateForPkgs =
          _: crate: buildRustCrate (crate // { extraRustcOpts = rustflags ++ (crate.extraRustcOpts or [ ]); });
      };
    in
    lib.mapAttrs (name: features: plan.workspaceMembers.${name}.build.override { inherit features; }) packages;
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
