#include "DEV_Config.h"

#ifndef EINK_H
#define EINK_H
typedef struct Eink {
    UBYTE* BlackImage;
    UBYTE* RedImage;
} Eink;

Eink* initEink();
void printTemperature(Eink* eink, TemperatureReading reading);
#endif
