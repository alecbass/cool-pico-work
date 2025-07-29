#include <hardware/gpio.h>
#include <hardware/irq.h>
#include <stdio.h>
#include "pico/stdlib.h"
#include "pico/cyw43_arch.h"

// const char ssid[] = "A Network";
// const char pass[] = "A Password";
// extern void do_thing();

int connectToWifi() {
    stdio_init_all();
    printf("yooooeeeee lol\n");

    /** The pin the custom LED is connected to */
    const uint LED_PIN = 14;

    gpio_init(LED_PIN);
    gpio_set_dir(LED_PIN, GPIO_OUT);
    gpio_put(LED_PIN, true);
    // for (int i = 0; i < 10; i++) {
    //     gpio_put(LED_PIN, i % 2);
    //     sleep_ms(500);
    // }

    // NOTE(alec): I think (THINK) that cortex-m-rt's entry entrypoint
    // https://github.com/rust-embedded/cortex-m/blob/master/cortex-m-rt/src/lib.rs
    // is taking the interrupt which the C side of things expects to be free when calling
    // cyw43_arch_init

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

    printf("connected\n");

    return 0;
}

int main() {
    return connectToWifi();
}
