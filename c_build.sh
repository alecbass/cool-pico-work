#!/usr/bin/env bash

# set -e

# Load required environment variables incase we forgot to
source .env

# C lib directory when running normally with the ARM embedded toolchain installed
c_lib_dir="/usr/lib/arm-none-eabi/newlib/thumb/v6-m/nofp/"

if [[ -d /nix/store/ ]]; then
    # Find the C lib directory when running on nix
    echo "Running on Nix, assuming that the gcc-arm-embedded package is installed"
    echo "Scanning /nix/store directory to find the gcc-arm-embedded directory"

    arm_embedded_dir=$(find /nix/store -type d -name "*.rel1")
    c_lib_dir="${arm_embedded_dir}/arm-none-eabi/lib/thumb/v6-m/nofp/"

    echo "C library directory found at ${c_lib_dir}"
fi

if [[ ! -d build ]]; then
    mkdir build
fi

cd build || exit 1

# Build C library
cp "${PICO_SDK_PATH}/external/pico_sdk_import.cmake" ..
cp "${PICO_EXAMPLES_PATH}/pico_w/wifi/lwipopts_examples_common.h" ../lwipopts.h

# Exporting compile commands creates a compile_commands.json that lets clangd find header files
cmake -DPICO_BOARD=pico_w -DCMAKE_EXPORT_COMPILE_COMMANDS=1 ..
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

echo "Statting $c_lib_dir"

for file in $(ls $c_lib_dir); do
    if [[ $file != *.a ]]; then
        continue
    fi

    echo "Moving $file - will require sudo to copy the C library .a static libraries :("
    sudo cp "${c_lib_dir}${file}" "$static_lib_target_dir"
done

echo "C lib dir: $c_lib_dir"

echo "C compilation complete!"
