#include <stdio.h>

#include <pico/stdio.h>
#include "pico/stdlib.h"
#include "pico/cyw43_arch.h"

const char ssid[] = "A Network";
const char pass[] = "A Password";

// NOTE: For some reason, Jartis cannot compile without the _fini symbol
// Taken from 
/* Make sure you have C linkage when defining in c++ file */
// extern "C"
void _fini()
{
    /* Either leave empty, or infinite loop here */
    while (true)
        __asm volatile ("NOP");
}

int connectToWifi() {
    // stdio_init_all();

    if (cyw43_arch_init_with_country(CYW43_COUNTRY_UK)) {
        // printf("failed to initialise\n");
        return 1;
    }
    
    // cyw43_arch_enable_sta_mode();
    // 
    // if (cyw43_arch_wifi_connect_timeout_ms(ssid, pass, CYW43_AUTH_WPA2_AES_PSK, 10000)) {
    //     printf("failed to connect\n");
    //     return 1;
    // }

    return 24;
}

