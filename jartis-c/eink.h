#include "DEV_Config.h"

typedef struct Eink {
    UBYTE* BlackImage;
    UBYTE* RedImage;
} Eink;

Eink* initEink();
void printTemperature(Eink* eink, TemperatureReading reading);
