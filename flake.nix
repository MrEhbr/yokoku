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
        # Everything else is stable, and clippy must match its rustc.
        rustToolchain =
          with inputs.fenix.packages.${prev.stdenv.hostPlatform.system};
          combine [
            stable.clippy
            stable.rustc
            stable.cargo
            stable.rust-src
            stable.llvm-tools
            latest.rustfmt
            targets.x86_64-apple-darwin.stable.rust-std
            targets.aarch64-apple-darwin.stable.rust-std
            targets.x86_64-unknown-linux-gnu.stable.rust-std
            targets.aarch64-unknown-linux-gnu.stable.rust-std
            targets.wasm32-unknown-unknown.stable.rust-std
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
