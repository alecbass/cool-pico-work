#include <stdio.h>
#include "pico/stdlib.h"
#include "pico/cyw43_arch.h"

// TESTS
#include <sys/time.h>
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

const char ssid[] = "A Network";
const char pass[] = "A Password";

int connectToWifi() {
    printf("Helloooo\n");
    return 5;
    // if (cyw43_arch_init_with_country(CYW43_COUNTRY_UK)) {
    //     printf("failed to initialise\n");
    //     return 1;
    // }
    //
    // cyw43_arch_enable_sta_mode();
    //
    // if (cyw43_arch_wifi_connect_timeout_ms(ssid, pass, CYW43_AUTH_WPA2_AES_PSK, 10000)) {
    //     printf("failed to connect\n");
    //     return 1;
    // }
    //
    // return 0;
}

