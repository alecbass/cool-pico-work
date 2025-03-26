{
  description = "Cool Raspberri Pi Pico Work";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    rust-flake.url = "./rust-flake/";
  };

  outputs = inputs@{ self, nixpkgs, rust-flake }: let
    supportedSystems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
    overrides = (builtins.fromTOML (builtins.readFile (self + "/rust-toolchain.toml")));
    pkgs = import nixpkgs;
    # rust-flake = import ./rust.nix;

    # rust = pkgs.stdenv.mkDerivation {
    #   nativeBuildInputs = with nixpkgs; [ pkg-config ];
    #   buildInputs = with nixpkgs; [
    #     clang
    #     llvmPackages.bintools
    #     rustup
    #   ];
    #
    #   RUSTC_VERSION = overrides.toolchain.channel;
    #   
    #   # https://github.com/rust-lang/rust-bindgen#environment-variables
    #   LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ nixpkgs.llvmPackages_latest.libclang.lib ];
    #   
    #   shellHook = ''
    #     export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
    #     export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
    #   '';
    #
    #   # Add precompiled library to rustc search path
    #   RUSTFLAGS = (builtins.map (a: ''-L ${a}/lib'') [
    #     # add libraries here (e.g. pkgs.libvmi)
    #   ]);
    #   
    #   # LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (buildInputs ++ nativeBuildInputs);
    #   LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath ([ pkgs.clang pkgs.llvmPackages.bintools pkgs.rustup pkgs.pkg-config ]);
    #   
    #   # Add glibc, clang, glib, and other headers to bindgen search path
    #   BINDGEN_EXTRA_CLANG_ARGS =
    #   # Includes normal include path
    #   (builtins.map (a: ''-I"${a}/include"'') [
    #     # add dev libraries here (e.g. pkgs.libvmi.dev)
    #     pkgs.glibc.dev
    #   ])
    #   # Includes with special directory paths
    #   ++ [
    #     ''-I"${pkgs.llvmPackages_latest.libclang.lib}/lib/clang/${pkgs.llvmPackages_latest.libclang.version}/include"''
    #     ''-I"${pkgs.glib.dev}/include/glib-2.0"''
    #     ''-I${pkgs.glib.out}/lib/glib-2.0/include/''
    #   ];
    # };

    forAllSystems = f: nixpkgs.lib.genAttrs supportedSystems (system: f {
      inherit system;
      # inherit rust;
      # inherit rust-flake;
      pkgs = import nixpkgs { inherit system; };

      rust = rust-flake;
    });
  in {
    devShell = forAllSystems({ pkgs, system, rust }: 
      pkgs.mkShell {
        RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

        buildInputs = with pkgs; [
          clang-tools
          cmake
          wget
          openssl
          # Compilation

          # Rust build dependencies
          rust

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

          neovim
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
