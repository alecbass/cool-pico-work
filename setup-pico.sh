#!/usr/bin/env bash

# Install Pico SDK
wget https://raw.githubusercontent.com/raspberrypi/pico-setup/master/pico_setup.sh
# Remove the sudo commands since we're in a Docker image
sed -e 's/sudo //' > tmp < pico_setup.sh
mv tmp pico_setup.sh
chmod +x pico_setup.sh
rm tmp
SKIP_VSCODE=1 ./pico_setup.sh

apt update
apt install -y libusb-1.0-0-dev
