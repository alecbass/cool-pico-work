SHELL := /usr/bin/env bash
BUILD_DIR := target/thumbv6m-none-eabi/debug

all: build

build:
	# Build binary to bin
	./c_build.sh
	# arm-none-eabi-objcopy --output-target binary "${BUILD_DIR}/jartis" "${BUILD_DIR}/jartis.bin"
	cargo build

.PHONY: all build
