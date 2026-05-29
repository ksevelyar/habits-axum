{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
        with pkgs; {
          packages.default = pkgs.rustPlatform.buildRustPackage {
            pname = "habits-axum";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            SQLX_OFFLINE = "true";
            doCheck = false;
          };

          devShells.default = mkShell {
            buildInputs = [
              cargo-watch
              websocat
              sqlx-cli
              rust-analyzer
              (rust-bin.stable.latest.default.override {
                extensions = ["rust-src"];
              })

              (writeShellScriptBin "ci" ''
                set -euo pipefail
                cargo sqlx migrate run
                cargo sqlx prepare
                cargo fmt --all -- --check --color always
                cargo clippy --all-features --workspace -- -D warnings
                cargo test

                nix build
              '')
            ];
            DATABASE_URL = "postgres://postgres:postgres@localhost:5432/habits_axum";
            ORIGIN = "http://habits.lcl:3000";
            RUST_LOG = "info";
            JWT_SECRET = "very secret";
          };
        }
    );
}
