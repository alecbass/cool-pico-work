SHELL := /usr/bin/env bash

BUILD_DIR := target/thumbv6m-none-eabi/debug

all: build

build:
	# Build binary to bin
	arm-none-eabi-objcopy --output-target binary "${BUILD_DIR}/cool-pico-work" "${BUILD_DIR}/cool-pico-work.bin"

.PHONY: all build
