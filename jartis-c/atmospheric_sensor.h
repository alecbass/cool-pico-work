#ifndef ATMOSPHERIC_SENSOR_H
#define ATMOSPHERIC_SENSOR_H
typedef struct {
    float temperature;
} TemperatureReading;

TemperatureReading getTemperatureReading(void);
#endif
