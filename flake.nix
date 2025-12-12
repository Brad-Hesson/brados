{
  description = "A very basic flake";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    brad-utils.url = "github:Brad-Hesson/brad-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };
  outputs = flakes: flakes.flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import flakes.nixpkgs { inherit system; };
      brad-utils = flakes.brad-utils.mkLib pkgs;
      fenix = flakes.fenix.packages.${system};
      crane = (flakes.crane.mkLib pkgs).overrideToolchain (fenix.combine [
        # complete means unstable rustc
        fenix.latest.toolchain
        fenix.targets.x86_64-unknown-uefi.latest.toolchain
        # fenix.complete.rust-src # need this for rust-analyzer
      ]);
      crateArgs = {
        # TODO: make a custom filter for files needed for server
        src = ./.;
        strictDeps = true;
        buildInputs = [ ];
        nativeBuildInputs = [ ];
      };
      cargoArtifacts = crane.buildDepsOnly crateArgs;
      crate = crane.buildPackage (crateArgs // { inherit cargoArtifacts; });
    in
    {
      devShells.default = crane.devShell {
        packages = [
          pkgs.qemu_kvm
        ];
        OVMF_FIRMWARE = pkgs.OVMF.fd;
        shellHook = brad-utils.vscodeSettingsHook { };
      };
    });
}
