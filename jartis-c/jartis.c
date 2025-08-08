#include "hardware/gpio.h"
#include "pico/cyw43_arch.h"
#include "pico/stdlib.h"
#include <stdio.h>
#include <stdlib.h>

#include "servo.h"

// const char ssid[] = "A Network";
// const char pass[] = "A Password";

Servo* servo;

void gpio_callback(uint pin, uint32_t events) {
    // Put the GPIO event(s) that just happened into event_str
    // so we can print it
    // gpio_event_string(event_str, events);
    printf("GPIO THING in %d\n", pin);
    // printf("GPIO %d %s\n", gpio, event_str);

    if (servo != NULL) {
        shuffleServo(servo);
    }
}

void initButtonIrq(const uint buttonActionPin, const uint buttonListenerPin) {
    // Current goes out through GIO16
    gpio_init(buttonActionPin);
    gpio_set_dir(buttonActionPin, GPIO_OUT);
    gpio_pull_up(buttonActionPin);
    gpio_put(buttonActionPin, true); // Make sure the pin has a current

    // And in through GPIO17
    gpio_init(buttonListenerPin);
    gpio_set_dir(buttonListenerPin, GPIO_IN);
    gpio_put(buttonListenerPin, false); // Make sure the pin is low to start with

    // Listening pin
    uint32_t interruptFlags = GPIO_IRQ_EDGE_FALL; // \ GPIO_IRQ_EDGE_RISE;
    gpio_set_irq_enabled_with_callback(buttonListenerPin, GPIO_IRQ_EDGE_RISE | GPIO_IRQ_EDGE_FALL, true,
                                       &gpio_callback);
}

int main() {
    stdio_init_all();
    printf("yooooeeeee lol\n");

    /** The pin the custom LED is connected to */
    const uint LED_PIN = 14;
    const uint SERVO_PIN = 2;
    const uint BUTTON_ACTION_PIN = 16;
    const uint BUTTON_LISTENER_PIN = 17;

    gpio_init(LED_PIN);
    gpio_set_dir(LED_PIN, GPIO_OUT);
    gpio_put(LED_PIN, true);

    initButtonIrq(BUTTON_ACTION_PIN, BUTTON_LISTENER_PIN);

    servo = initServo(SERVO_PIN);
    moveServo(servo, 0.0);

    if (cyw43_arch_init()) {
        printf("Wi-Fi init failed");
        return -1;
    }

    while (true) {
        cyw43_arch_gpio_put(CYW43_WL_GPIO_LED_PIN, 1);
        gpio_put(LED_PIN, false);
        printf("disabled\n");
        sleep_ms(1000);
        cyw43_arch_gpio_put(CYW43_WL_GPIO_LED_PIN, 0);
        gpio_put(LED_PIN, true);
        sleep_ms(1000);
        printf("enabled\n");
    }

    if (servo != NULL) {
        free(servo);
    }

    return 0;
}
