cmake_minimum_required(VERSION 3.13)

include(pico_sdk_import.cmake)

project(jartis_project C CXX ASM)

set(CMAKE_C_STANDARD 11)

set(CMAKE_CXX_STANDARD 17)

pico_sdk_init()

set(SOURCE_FILES src/jartis.c src/jartis.h)

add_library(jartis STATIC ${SOURCE_FILES})

# add_executable(jartis ${SOURCE_FILES})

pico_enable_stdio_usb(jartis 1)

pico_enable_stdio_uart(jartis 1)

pico_add_extra_outputs(jartis)

target_include_directories(jartis PRIVATE ${CMAKE_CURRENT_LIST_DIR} )

target_link_libraries(jartis
    pico
    pico_cyw43_arch_lwip_threadsafe_background
    # pico_cyw43_arch_lwip_poll
    pico_stdlib
    pico_stdio
    pico_mutex
    pico_bootroom
    pico_time 
    pico_runtime_init
    pico_printf
    stdlib
    stdio
)
