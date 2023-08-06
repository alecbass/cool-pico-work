#!/usr/bin/env bash

RUN_OPENOCD="src/openocd -f interface/cmsis-dap.cfg -c 'adapter speed 5000' -f target/rp2040.cfg -s tcl"



# Run the RUN_OPENOCD command in the background
cd openocd && $RUN_OPENOCD &

# Install debug dependencies
apt install -y gdb-multiarch libudev-dev gcc-arm-none-eabi usbutils udev minicom
cargo install elf2uf2-rs

# Copy udev configuration for OpenOCD
if [[ ! -d /etc/udev ]]; then
    mkdir -p /etc/udev/rules.d
fi

# cp -r /app/udev/rules.d/* /etc/udev/rules.d

sed '127,130d' < /etc/init.d/udev > /etc/init.d/udev.tmp
mv /etc/init.d/udev.tmp /etc/init.d/udev
sed '145,147d' < /etc/init.d/udev > /etc/init.d/udev.tmp
mv /etc/init.d/udev.tmp /etc/init.d/udev
chmod 777 /etc/init.d/udev

