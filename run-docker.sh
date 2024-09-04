#!/usr/bin/env bash

if [[ -z $(docker images | grep pico) ]]; then
    echo "Image does not exist - building..."
    docker build -t pico .
fi

# echo "Restarting udev - requiring sudo access"
# sudo service udev restart
# sudo udevadm trigger

docker run \
    -it \
    --privileged \
    --net=host \
    -v=/dev/bus/usb:/dev/bus/usb \
    -v=/run/udev/control:/run/udev/control \
    -v=./src:/app/src \
    -v=./openocd.gdb:/app/openocd.gdb \
    -v=./CMakeLists.txt:/app/CMakeLists.txt \
    -v=./jartis.c:/app/jartis.c \
    -v=./jartis.h:/app/jartis.h \
    -v=./c_build.sh:/app/c_build.sh \
    -v=./.cargo:/app/.cargo \
    -v=./build.rs:/app/build.rs \
    pico

