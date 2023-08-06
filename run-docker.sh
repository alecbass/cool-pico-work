#!/usr/bin/env bash

echo "Restarting udev - requiring sudo access"
sudo service udev restart
sudo udevadm trigger

docker run -it --privileged -v=/dev/bus/usb:/dev/bus/usb -v=/run/udev/control:/run/udev/control pico
