SHELL := /usr/bin/env bash
BUILD_DIR := target/thumbv6m-none-eabi/debug
DEBUG_TARGET := build/jartis.elf

all: build

build:
	# Build binary to bin
	./c_build.sh Debug
	# arm-none-eabi-objcopy --output-target binary "${BUILD_DIR}/jartis" "${BUILD_DIR}/jartis.bin"
	# cargo build

debug: build
	gdb -q -x openocd.gdb ${DEBUG_TARGET}

.PHONY: all build
