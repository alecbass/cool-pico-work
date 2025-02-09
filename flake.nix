{
  description = "Cool Raspberri Pi Pico Work";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-24.11";
  };

  outputs = { self, nixpkgs }: let
    supportedSystems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
  in {
    devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
            clang-tools
            cmake
            wget
        ];
        
        env = {};

        nativeBuildInputs = [
          pkgs.pkg-config
        ];

        buildInputs = [
          pkgs.openssl
          # Compilation

          # Rust build dependencies
          # pkgs.cargo
          # pkgs.rustc
          # pkgs.rustup

          pkgs.gcc-arm-embedded-13

          # NOTE: IT would be good to have these enabled but pico_setup.sh fails?
          # pkgs.libcxx
          # pkgs.glibc
          # pkgs.libgcc

          # OpenOCD compilation
          pkgs.automake
          pkgs.autoconf
          pkgs.texinfo
          pkgs.libtool
          pkgs.libftdi1
          pkgs.libusb1
          pkgs.udev
          pkgs.minicom
        ]
        ++ nixpkgs.lib.optionals (pkgs.stdenv.isDarwin) [
          pkgs.libiconv
          pkgs.darwin.apple_sdk.frameworks.Security
          pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
      ];
    };
  };
}
