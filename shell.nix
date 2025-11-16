# shell.nix
#
# Use this file with `nix-shell` to create a development environment
# for the 'handsfreectl' project.

{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  name = "handsfreectl-dev";

  buildInputs = [
    pkgs.cargo
    pkgs.rustc
    pkgs.gcc # cc required by some rust dependencies' build scripts
  ];

  shellHook = ''
    echo "Entered handsfreectl development shell."
  '';
}
