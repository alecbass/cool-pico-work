#!/usr/bin/env bash

RUN_OPENOCD="src/openocd -f interface/cmsis-dap.cfg -c 'adapter speed 5000' -f target/rp2040.cfg -s tcl"

# Install the OpenOCD project
if [[ ! -d openocd ]]; then
    git clone --branch rp2040-v0.12.0 https://github.com/raspberrypi/openocd.git
fi

# Build OpenOCD and its dependencies
apt update
apt install -y libusb-1.0-0-dev
cd openocd && ./bootstrap && ./configure --enable-cmsis-dap-v2 && make -j"$(nproc)" && make install

# Run the RUN_OPENOCD command in the background
cd openocd && $RUN_OPENOCD &

# Install debug dependencies
apt install -y gdb-multiarch libudev-dev gcc-arm-none-eabi usbutils udev minicom
cargo install elf2uf2-rs

# Copy udev configuration for OpenOCD
if [[ ! -d /etc/udev ]]; then
    mkdir -p /etc/udev/rules.d
fi

cp -r /app/udev/rules.d/* /etc/udev/rules.d
