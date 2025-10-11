{
  description = "LZMA SDK sys bindings for Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      config.allowUnfree = true;
    };
  in {
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = with pkgs; [
        uasm
        clang
        stdenv.cc
      ];

      shellHook = ''
        export NIXPKGS_ALLOW_UNFREE=1
      '';
    };
  };
}
