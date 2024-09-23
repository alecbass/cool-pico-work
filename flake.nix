{
  description = "Cool Raspberri Pi Pico Work";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = { self, nixpkgs }: let
    supportedSystems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};

    forEachSupportedSystem = callback:
      nixpkgs.lib.genAttrs supportedSystems (supportedSystem:
        callback {
          system = supportedSystem;
          pkgs = nixpkgs.legacyPackages.${supportedSystem};
          # pkgs = dream2nix.inputs.nixpkgs.legacyPackages.${supportedSystem};
        });

  in {
    devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
            cargo
            rustc
            rustup
            clang-tools
            libclang
            libcxx
            glibc
            libgcc
            cmake
        ];
        
        env = {};

        nativeBuildInputs = [
          pkgs.pkg-config
        ];

        buildInputs = [
          pkgs.openssl
          # Compilation
          # openssl
          pkgs.pkg-config
          pkgs.clang
          # pkgs.glibc_multi

          # pkgs.ebugging
          # pkgs.gdb-multiarch
          # pkgs.libudev-dev
          # pkgs.gcc-arm-none-eabi
          # pkgs.usbutils
          # pkgs.gdb-multiarch

          # OpenOCD compilation
          pkgs.automake
          pkgs.autoconf
          # pkgs.build-essential
          pkgs.texinfo
          pkgs.libtool
          # pkgs.libftdi
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
