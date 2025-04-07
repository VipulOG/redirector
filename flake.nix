{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    flake-parts,
    rust-overlay,
    crane,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem = {
        config,
        self',
        inputs',
        pkgs,
        system,
        ...
      }: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {inherit system overlays;};

        rustToolchain = pkgs.rust-bin.stable.latest.default;
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          buildInputs = with pkgs; [openssl];
          nativeBuildInputs = with pkgs; [pkg-config];
        };

        redirector =
          (craneLib.buildPackage (commonArgs
            // {
              cargoArtifacts = craneLib.buildDepsOnly commonArgs;
              doCheck = false;
            }))
          .overrideAttrs (old: {
            meta =
              (old.meta or {})
              // {
                mainProgram = "redirector";
              };
          });
      in {
        packages = {
          default = redirector;
          redirector = redirector;
        };

        apps.default = {
          type = "app";
          program = "${redirector}/bin/redirector";
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [redirector];
          nativeBuildInputs = with pkgs; [
            rustToolchain
            rust-analyzer
            clippy
            rustfmt
          ];
        };
      };

      flake = {
        homeManagerModules.redirector = import ./.nix/home-module.nix;
        overlays.default = final: prev: {
          redirector = self.packages.${final.system}.redirector;
        };
      };
    };
}
