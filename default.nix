{ pkgs ? import <nixpkgs> {} }:

let
  rust-overlay = (import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"));
  nixpkgs = import <nixpkgs> { overlays = [ rust-overlay ]; };
  rust-bin = nixpkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
  rustPlatform = pkgs.makeRustPlatform {
    cargo = rust-bin;
    rustc = rust-bin;
  };
in
rustPlatform.buildRustPackage {
  name = "typssg";

  src = ./.;

  nativeBuildInputs = with pkgs; [
    rust-analyzer
  ];
  buildInputs = [];

  cargoLock.lockFile = ./Cargo.lock;

  RUSTUP_HOME = toString ./.rustup;
  CARGO_HOME = toString ./.cargo;
}
