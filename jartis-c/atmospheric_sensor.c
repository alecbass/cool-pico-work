#include "atmospheric_sensor.h"

TemperatureReading getTemperatureReading(void) {
    TemperatureReading reading = {.temperature = 16.0};
    return reading;
}
