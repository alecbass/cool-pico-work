#include <hardware/irq.h>
#include <stdio.h>

#include "pico/stdlib.h"
#include "pico/cyw43_arch.h"

// const char ssid[] = "A Network";
// const char pass[] = "A Password";

int connectToWifi() {
    stdio_init_all();
    printf("yooooeeeee lol\n");

    // gpio_init(25);
    //
    // cyw43_arch_gpio_put(CYW43_WL_GPIO_LED_PIN, 1);
    // sleep_ms(250);
    // cyw43_arch_gpio_put(CYW43_WL_GPIO_LED_PIN, 0);
    // sleep_ms(250);
    
    // NOTE(alec): I think (THINK) that cortex-m-rt's entry entrypoint
    // https://github.com/rust-embedded/cortex-m/blob/master/cortex-m-rt/src/lib.rs
    // is taking the interrupt which the C side of things expects to be free when calling
    // cyw43_arch_init


    for (uint num = 0; num <= 51; num++ ) {
        bool hasClaimed = user_irq_is_claimed(num);
        printf("irq %d claimed: %d\n", num, hasClaimed);
    }
    uint irqNum = 0;
    user_irq_claim(irqNum);

    if (cyw43_arch_init()) {
        printf("Wi-Fi init failed");
        return -1;
    }

    while (true) {
        cyw43_arch_gpio_put(CYW43_WL_GPIO_LED_PIN, 1);
        sleep_ms(1000);
        cyw43_arch_gpio_put(CYW43_WL_GPIO_LED_PIN, 0);
        sleep_ms(1000);
    }

    // if (cyw43_arch_init_with_country(CYW43_COUNTRY_UK)) {
    //     return 1;
    // }
    
    cyw43_arch_enable_sta_mode();
    
    // if (cyw43_arch_wifi_connect_timeout_ms(ssid, pass, CYW43_AUTH_WPA2_AES_PSK, 10000)) {
    //     printf("failed to connect\n");
    //     return 1;
    // }
    printf("connected\n");

    return 24;
}

int main() {
    connectToWifi();
    return 0;
}
