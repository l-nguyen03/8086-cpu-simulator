{
  description = "8086 disassembler";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = {nixpkgs, ...}: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
  in {
    devShells.${system}.default = pkgs.mkShell {
      strictDeps = true;

      packages = with pkgs; [
        rustc
        cargo
        rustfmt
        clippy
        rust-analyzer

        gcc
        pkg-config
        llvmPackages.bintools
        nasm

        lldb

        cargo-watch
        cargo-nextest
      ];

      env = {
        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        RUST_BACKTRACE = "1";
      };
    };
  };
}
