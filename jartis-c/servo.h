#include <stdlib.h>

typedef struct {
    uint pin;
    float degree;
} Servo;

Servo* initServo(const uint servoPin);
void moveServo(Servo* servo, float degree);
void shuffleServo(Servo* servo);
