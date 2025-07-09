{
  description = "Cool Raspberri Pi Pico Work";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = inputs@{ self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      {
        devShells.default = with pkgs; mkShell {
          buildInputs = [
            rustup
            rust-bin.stable."1.88.0".default

            openssl
            pkg-config
            eza
            fd
            clang-tools
            cmake
            wget
            gcc-arm-embedded-13
            automake
            autoconf
            texinfo
            libtool
            libftdi1
            libusb1
            udev
            minicom

            # To talk over wires for OpenOCD
            libftdi1
            jimtcl # To compile OpenOCD

            neovim # IDE
            probe-rs-tools # For debugging
          ]
          ++ nixpkgs.lib.optionals (pkgs.stdenv.isDarwin) [
            libiconv
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          GCC_ARM_EMBEDDED_TOOLCHAIN = "${gcc-arm-embedded-13}";
        };
      }
    );
}
