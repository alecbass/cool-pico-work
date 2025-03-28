# RDID Flasher program

PIN setup:

# Pico -> Button
* GPIO15 -> Bottom of button
* ADC_VREF -> Top of button

# Pico -> PiicoDev SSD1306
* VSYS -> +3V3 (Marked with -)
* GND -> GND (Marked with -)
* GPIO8 I2C0 SDA -> SDA
* GPIO9 I2C0 SCL -> SCL

# Pico -> PiicoDev RFID
VSYS -> +3V3
GND -> GND
