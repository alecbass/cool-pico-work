#!/usr/bin/env bash

set -e

# C lib directory when running normally with the ARM embedded toolchain installed
c_lib_dir="/usr/lib/arm-none-eabi/newlib/thumb/v6-m/nofp/"

# Nix flocation
arm_embedded_dir=$GCC_ARM_EMBEDDED_TOOLCHAIN

if [[ -z $arm_embedded_dir ]]; then
    echo "GCC_ARM_EMBEDDED_TOOLCHAIN is not set. Needed to compile the Pico C library."
    exit 1
fi

c_lib_dir="${arm_embedded_dir}/arm-none-eabi/lib/thumb/v6-m/nofp/"

if [[ ! -d build ]]; then
    mkdir build
fi

cd build || exit 1

# Build C library
cp "${PICO_SDK_PATH}/external/pico_sdk_import.cmake" ..

export CMAKE_LIBRARY_PATH="$CMAKE_LIBRARY_PATH:$c_lib_dir"

# Exporting compile commands creates a compile_commands.json that lets clangd find header files
cmake \
    -DPICO_BOARD=pico_w \
    -DCMAKE_EXPORT_COMPILE_COMMANDS=1 \
    -DARM_EMBEDDED_DIR="$arm_embedded_dir" \
    -DPICO_SDK_PATH="$PICO_SDK_PATH" \
    ..
make

# NOTE: Make will fail as it attempts to build a .uf2 file
# We want to find recreate the steps to get a .a file and then link it into our Rust binary
#
# The following commands are copied from /app/CMakeFiles/jartis.dir/build.make

# cd /app/CMakeFiles/jartis.dir || exit 1
# cmake -E cmake_link_script CMakeFiles/jartis.dir/link.txt
echo "Created static libjartis.a library"

# Return to the root directory
cd .. || exit 1

static_lib_file="build/libjartis.a"
static_lib_target_dir="target/thumbv6m-none-eabi/debug/deps/"

if [[ ! -f "$static_lib_file" ]]; then
    echo "Failed to create static library"
    exit 1
fi

if [[ ! -d "$static_lib_target_dir" ]]; then
    # Create a target directory to move the static library into
    mkdir -p "$static_lib_target_dir"
fi

# Make libjartis.a available to be linked to Rust
cp "$static_lib_file" "$static_lib_target_dir"
echo "Copied static library to target directory"
