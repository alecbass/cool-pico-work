# My cool Raspberry PI Pico work

Template copied from [The RP2040 Rust project template](https://github.com/rp-rs/rp2040-project-template)

Rewritten unified and RGB libraries from [Core Electronics' PiicoDev Starter Kit](https://core-electronics.com.au/piicodev-starter-kit-raspberry-pi-pico-guides-0)

Enjoy!!!!!!!!!

# NOTES
Using both rp-pico and embassy has provided some weird linker __INTERRUPTS linker errors
.boot2 memory potentially scuffed


# OpenOCD compilation notes
I had to remove the -Wstrict-prototypes and -Werror GCC flags from OpenOCD's Makefile
