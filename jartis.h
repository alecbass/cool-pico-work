include <stdio.h>
include "pico/stdlib.h"

// TESTS
#include <sys/times.h>
#include <unistd.h>
#include "pico.h"

#include "hardware/regs/m0plus.h"
#include "hardware/regs/resets.h"
#include "hardware/structs/mpu.h"
#include "hardware/structs/scb.h"
#include "hardware/structs/padsbank0.h"

#include "hardware/clocks.h"
#include "hardware/irq.h"
#include "hardware/resets.h"

#include "pico/mutex.h"
#include "pico/time.h"
#include "pico/runtime_init.h"

int connectToWifi();
