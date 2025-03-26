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
          # RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          buildInputs = [
            rustup
            rust-bin.stable."1.85.1".default
            # rust-bin.stable."1.85.1".default.override {
            #   extensions = [ "rust-src "];
            #   targets = [ "thumbv6m-none-eabi" ];
            # }
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

            neovim
          ]
          ++ nixpkgs.lib.optionals (pkgs.stdenv.isDarwin) [
            libiconv
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          shellHook = ''
            cargo install ripgrep
          '';
        };
      }
    );
}
