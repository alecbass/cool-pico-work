# My cool Raspberry PI Pico work

Template copied from [The RP2040 Rust project template](https://github.com/rp-rs/rp2040-project-template)

Rewritten unified and RGB libraries from [Core Electronics' PiicoDev Starter Kit](https://core-electronics.com.au/piicodev-starter-kit-raspberry-pi-pico-guides-0)

Enjoy!!!!!!!!!

# NOTES
Using both rp-pico and embassy has provided some weird linker __INTERRUPTS linker errors
.boot2 memory potentially scuffed

Move into the `pico-sdk` submodule and run `git submodule update --init` to make cyw43 drivers available.

# OpenOCD compilation notes
I had to remove the -Wstrict-prototypes and -Werror GCC flags from OpenOCD's Makefile

# Reading UART
Minicom doesn't seem to read anything, so I've had more access using this method (lol)

```
sudo chmod a+rw /dev/ttyACM0
cat /dev/ttyACM0
```

# Programs
This repository contains some programs ready to flash onto a PICO. To run them, build with the given feature with
`cargo build` or `cargo run`.

* RFID_FLASHER - Reads and flashes RFID chips. (Doesn't actually lol, this is a work in progress)

# TODO
Make the RFID_FLASHER program work with classic RFID tags

# Mounting a USB on NixOS
* Have udisks2 installed with `services.udisks2.enable = true;` in your configuration
* Find the device with `lsblk` and mount it with `udisksctl mount -b /dev/sda1` or whichever path appears i.e. `/dev/sdb`
* You can also use `lsusb -t` for more information about connected USB devices

