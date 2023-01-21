#!/usr/bin/env bash

TARGET=target/thumbv6m-none-eabi/release/cool-pico-work
OUT=/mnt/c/Users/alecb/Documents/out.uf2

cargo build --release && echo "Built!"
elf2uf2-rs "$TARGET" "$OUT"
