#!/usr/bin/env bash

RUN_OPENOCD='openocd -f interface/cmsis-dap.cfg -f target/rp2040.cfg -s tcl'

# Install the OpenOCD project
if [[ ! -d "openocd" ]]; then
    git clone https://github.com/openocd-org/openocd.git
fi

# Build OpenOCD and its dependencies
apt update
apt install -y libusb-1.0-0-dev
cd openocd && ./bootstrap && ./configure --enable-cmsis-dap-v2 && make -j"$(nproc)" && make install

# Run the RUN_OPENOCD command in the background
cd openocd && $RUN_OPENOCD &

# Install debug dependencies
apt install -y gdb-multiarch libudev-dev gcc-arm-none-eabi
cargo install elf2uf2-rs
