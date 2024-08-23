cmake_minimum_required(VERSION 3.13)

include(pico/pico-sdk/pico_sdk_import.cmake)

project(hello C CXX ASM)

set(CMAKE_C_STANDARD 11)

set(CMAKE_CXX_STANDARD 17)

pico_sdk_init()

target_include_directories(PRIVATE ${CMAKE_CURRENT_LIST_DIR} )

set(SOURCE_FILES hello.c Structure.h)
add_library(hello STATIC ${SOURCE_FILES})

target_link_libraries(hello pico_cyw43_arch_lwip_threadsafe_background pico_stdlib)
