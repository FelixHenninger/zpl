{
  description = "Glia dev environment";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        customRust = pkgs.rust-bin.stable.latest.default.override {
          # None at present
        };
      in
      with pkgs;
      {
        devShells.default = mkShell {
          nativeBuildInputs = [
          ] ++ (
            # Load alsa dependencies on Linux
            lib.optionals stdenv.isLinux [
              pkg-config
              alsa-oss
              alsa-lib
            ]
          );

          buildInputs = [
            customRust
            bacon
            python3
            python3Packages.pillow
            python3Packages.crc
            mob
          ] ++ (
            # Apple libraries if necessary
            lib.optional stdenv.isDarwin [
              libiconv
              darwin.apple_sdk.frameworks.AppKit
              darwin.apple_sdk.frameworks.AudioToolbox
              darwin.apple_sdk.frameworks.AudioUnit
              darwin.apple_sdk.frameworks.CoreAudio
              darwin.apple_sdk.frameworks.CoreFoundation
            ]
          );

          shellHook = ''
          '';
        };

        formatter = pkgs.nixpkgs-fmt;
      }
    );
}
