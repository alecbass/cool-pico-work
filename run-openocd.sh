#!/usr/bin/env bash

# If running outside the Docker environment, be sure to run as sudo

if [[ -d openocd/tcl ]]; then
    # Running from the project root directory
    cd openocd/tcl
fi

# cd openocd && src/openocd -f interface/cmsis-dap.cfg -c 'adapter speed 5000' -f target/rp2040.cfg -s tcl
../src/openocd -f interface/cmsis-dap.cfg -c 'adapter speed 5000' -f target/rp2040.cfg
