#!/usr/bin/env bash

cd /app/pico/openocd && src/openocd -f interface/cmsis-dap.cfg -c 'adapter speed 5000' -f target/rp2040.cfg -s tcl
