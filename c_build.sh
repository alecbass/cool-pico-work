#!/usr/bin/env bash

set -e

# Load required environment variables in case we forgot to
source .env

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
cp "${PICO_EXAMPLES_PATH}/pico_w/wifi/lwipopts_examples_common.h" ../lwipopts.h

export CMAKE_LIBRARY_PATH="$CMAKE_LIBRARY_PATH:$c_lib_dir"

echo "DIR: $c_lib_dir"
echo "ARM dir: $arm_embedded_dir"
# Exporting compile commands creates a compile_commands.json that lets clangd find header files
cmake -DPICO_BOARD=pico_w -DCMAKE_EXPORT_COMPILE_COMMANDS=1 -DARM_EMBEDDED_DIR="$arm_embedded_dir" ..
make

# NOTE: Make will fail as it attempts to build a .uf2 file
# We want to find recreate the steps to get a .a file and then link it into our Rust binary
#
# The following commands are copied from /app/CMakeFiles/jartis.dir/build.make

# cd /app/CMakeFiles/jartis.dir || exit 1
cmake -E cmake_link_script CMakeFiles/jartis.dir/link.txt
echo "Created static library"
echo "IGNORE THE PREVIOUS WARNINGS. WE ONLY CARE THAT A libjartis.a FILE WAS CREATED"

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

cp "$static_lib_file" "$static_lib_target_dir"

# for file in $c_lib_dir; do
#     [[ -e $file ]] || continue # Empty directory
#
#     if [[ $file != *.a ]]; then
#         continue
#     fi
#
#     echo "Moving $file - will require sudo to copy the C library .a static libraries :("
#     sudo cp "${c_lib_dir}${file}" "$static_lib_target_dir"
# done

echo "C lib dir: $c_lib_dir"
