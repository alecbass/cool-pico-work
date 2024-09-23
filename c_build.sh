#!/usr/bin/env bash

static_lib_file="/app/build/libjartis.a"
static_lib_target_dir="/app/target/thumbv6m-none-eabi/debug/deps/"
c_lib_dir="/usr/lib/arm-none-eabi/newlib/thumb/v6-m/nofp/"
# c_lib_file="${c_lib_dir}/newlib/libc.a"
c_lib_file="${c_lib_dir}libc.a"

if [[ ! -f "$c_lib_file" ]]; then
    echo "Cannot find C standard library libc.a file"
    exit 1
fi

if [[ ! -d build ]]; then
    mkdir build
fi

cd build || exit 1

# Build C library
cp "${PICO_SDK_PATH}/external/pico_sdk_import.cmake" ..
cp "${PICO_EXAMPLES_PATH}/pico_w/wifi/lwipopts_examples_common.h" ../lwipopts.h
cmake -DPICO_BOARD=pico_w ..
make

# NOTE: Make will fail as it attempts to build a .uf2 file
# We want to find recreate the steps to get a .a file and then link it into our Rust binary
#
# The following commands are copied from /app/CMakeFiles/jartis.dir/build.make

# cd /app/CMakeFiles/jartis.dir || exit 1
cmake -E cmake_link_script CMakeFiles/jartis.dir/link.txt
echo "Created static library"
echo "IGNORE THE PREVIOUS WARNINGS. WE ONLY CARE THAT A libjartis.a FILE WAS CREATED"

if [[ ! -f "$static_lib_file" ]]; then
    echo "Failed to create static library"
    exit 1
fi

if [[ ! -d "$static_lib_target_dir" ]]; then
    # Create a target directory to move the static library into
    mkdir -p "$static_lib_target_dir"
fi

cp "$static_lib_file" "$static_lib_target_dir"

cp "$c_lib_file" "$static_lib_target_dir"
cp "${c_lib_dir}*.a" "$static_lib_target_dir"

cargo build
