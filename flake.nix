{
  description = "{{project-description}}";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1";
    flake-parts.url = "github:hercules-ci/flake-parts";
    process-compose-flake.url = "github:Platonic-Systems/process-compose-flake";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nur = {
      url = "github:nix-community/NUR";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      imports = [ ./nix/services.nix ];

      flake.overlays.default = final: prev: {
        # rustfmt comes from nightly; rustfmt.toml uses nightly-only options.
        # Everything else comes from rust-toolchain.toml, which CI uses too.
        rustToolchain =
          with inputs.fenix.packages.${prev.stdenv.hostPlatform.system};
          combine [
            (fromToolchainFile {
              file = ./rust-toolchain.toml;
              sha256 = "sha256-p8h3Sl/YRByZfZTAKXdsvF6xEenXKrXSVvpphmZENH4=";
            })
            latest.rustfmt
          ];
      };

      perSystem =
        { config, system, pkgs, ... }:
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              inputs.self.overlays.default
              inputs.nur.overlays.default
            ];
          };

          devShells.default = pkgs.mkShell {
            inputsFrom = [ config.devShells.services ];
            packages = with pkgs; [
              rustToolchain
              rust-analyzer
              openssl
              pkg-config
              cargo-deny
              cargo-edit
              cargo-nextest
              cargo-zigbuild
              cargo-shear
              tokio-console
              gnuplot
              typos
              git-cliff
              nur.repos.goreleaser.goreleaser
              zig_0_13
              curl
              prek
            ];

            env = {
              # Required by rust-analyzer
              RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";
            };
          };
        };
    };
}
