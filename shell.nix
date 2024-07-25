let
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-24.05";
  pkgs = import nixpkgs { config = {}; overlays = []; };
in pkgs.mkShellNoCC {
  packages = with pkgs; [
    # Compilation
    # openssl
    pkg-config
    clang
    # glibc_multi

    # Debugging
    # gdb-multiarch
    # libudev-dev
    # gcc-arm-none-eabi
    # usbutils
    # gdb-multiarch

    # OpenOCD compilation
    automake
    autoconf
    # build-essential
    texinfo
    libtool
    # libftdi
    libftdi1
    libusb1
    udev
    minicom
  ];

  shellHook = ''
  '';

  buildInputs = with pkgs; [
  ];
}
