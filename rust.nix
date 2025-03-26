{
  description = "Flake utils demo";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
  let 
  overrides = (builtins.fromTOML (builtins.readFile (self + "/rust-toolchain.toml")));

  in
  flake-utils.lib.eachDefaultSystem (system:
      let
      pkgs = import nixpkgs { inherit system; };

      rust = pkgs.stdenv.mkDerivation {
        nativeBuildInputs = with nixpkgs; [ pkg-config ];
        buildInputs = with nixpkgs; [
          clang
          llvmPackages.bintools
          rustup
        ];

        RUSTC_VERSION = overrides.toolchain.channel;
        
        # https://github.com/rust-lang/rust-bindgen#environment-variables
        LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ nixpkgs.llvmPackages_latest.libclang.lib ];
        
        shellHook = ''
          export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
          export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
        '';

        # Add precompiled library to rustc search path
        RUSTFLAGS = (builtins.map (a: ''-L ${a}/lib'') [
          # add libraries here (e.g. pkgs.libvmi)
        ]);
        
        # LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (buildInputs ++ nativeBuildInputs);
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath ([ pkgs.clang pkgs.llvmPackages.bintools pkgs.rustup pkgs.pkg-config ]);
        
        # Add glibc, clang, glib, and other headers to bindgen search path
        BINDGEN_EXTRA_CLANG_ARGS =
        # Includes normal include path
        (builtins.map (a: ''-I"${a}/include"'') [
          # add dev libraries here (e.g. pkgs.libvmi.dev)
          pkgs.glibc.dev
        ])
        # Includes with special directory paths
        ++ [
          ''-I"${pkgs.llvmPackages_latest.libclang.lib}/lib/clang/${pkgs.llvmPackages_latest.libclang.version}/include"''
          ''-I"${pkgs.glib.dev}/include/glib-2.0"''
          ''-I${pkgs.glib.out}/lib/glib-2.0/include/''
        ];
      };

      in
      {
        packages = rec {
          rust = rust;
          default = rust;
        };
        apps = rec {
          rust = flake-utils.lib.mkApp { drv = self.packages.${system}.rust; };
          default = rust;
        };
      }
    );
}
