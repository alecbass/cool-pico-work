{
  description = "Cool Raspberri Pi Pico Work";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-24.11";
  };

  outputs = inputs@{ self, nixpkgs }: let
    supportedSystems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
    forAllSystems = f: nixpkgs.lib.genAttrs supportedSystems (system: f {
      inherit system;
      pkgs = import nixpkgs { inherit system; };
    });
  in {
    devShell = forAllSystems({ pkgs, system }: 
      pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          pkg-config
        ];

        buildInputs = with pkgs; [
          clang-tools
          cmake
          wget
          openssl
          # Compilation

          # Rust build dependencies
          # cargo
          # rustc
          # rustup

          gcc-arm-embedded-13

          # NOTE: It would be good to have these enabled but pico_setup.sh fails?
          # libcxx
          # glibc
          # libgcc

          # OpenOCD compilation
          automake
          autoconf
          texinfo
          libtool
          libftdi1
          libusb1
          udev
          minicom
        ]
        ++ nixpkgs.lib.optionals (pkgs.stdenv.isDarwin) [
          libiconv
          darwin.apple_sdk.frameworks.Security
          darwin.apple_sdk.frameworks.SystemConfiguration
      ];
    }
  );
  };
}
