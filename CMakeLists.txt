cmake_minimum_required(VERSION 3.13)
# --- User Configuration ---

# 1. Pico SDK Path: MUST be set correctly.
#    Set via environment: export PICO_SDK_PATH=/path/to/pico-sdk
#    Or via CMake argument: cmake .. -DPICO_SDK_PATH=/path/to/pico-sdk
set(PICO_SDK_PATH "$ENV{PICO_SDK_PATH}" CACHE PATH "Path to the Raspberry Pi Pico SDK source directory")
if(NOT PICO_SDK_PATH OR NOT EXISTS "${PICO_SDK_PATH}/pico_sdk_init.cmake")
    message(FATAL_ERROR "PICO_SDK_PATH is not set or invalid ('${PICO_SDK_PATH}'). Please set the environment variable or use -DPICO_SDK_PATH=...")
endif()
message(STATUS "Using Pico SDK source from: ${PICO_SDK_PATH}")
message(STATUS "Using ARM embedded toolchain from: ${ARM_EMBEDDED_DIR}")

# --- Toolchain Configuration ---
# Ensure ARM GCC toolchain executables (arm-none-eabi-gcc etc.) are in your PATH
# We explicitly pass these to the external project's CMake configuration.
# set(TOOLCHAIN_PREFIX "arm-none-eabi-")
set(TOOLCHAIN_PREFIX "${ARM_EMBEDDED_DIR}/bin/arm-none-eabi-")
message(STATUS "Toolchain prefix: ${TOOLCHAIN_PREFIX}")
set(CMAKE_C_COMPILER   "${TOOLCHAIN_PREFIX}gcc")
set(CMAKE_CXX_COMPILER "${TOOLCHAIN_PREFIX}g++")
set(CMAKE_ASM_COMPILER "${TOOLCHAIN_PREFIX}gcc")
set(CMAKE_AR           "${TOOLCHAIN_PREFIX}ar")
set(CMAKE_OBJCOPY      "${TOOLCHAIN_PREFIX}objcopy")
set(CMAKE_OBJDUMP      "${TOOLCHAIN_PREFIX}objdump")

include(${PICO_SDK_PATH}/external/pico_sdk_import.cmake)
include(pico_sdk_import.cmake)

set(LIBRARY_NAME jartis)

project(
    ${LIBRARY_NAME} 
    LANGUAGES C CXX ASM
)

set(CMAKE_C_STANDARD 11)
set(CMAKE_CXX_STANDARD 17)

# 2. SDK Libraries to Build: Semicolon-separated list. Adjust as needed.
set(PICO_SDK_TARGET_LIBS
    pico_stdlib         # Base library, almost always needed
    hardware_gpio       # For GPIO control
    # hardware_timer      # For hardware timers, delays
    hardware_pwm        # For PWM output
    # hardware_adc        # For ADC input
    # hardware_spi        # For SPI communication
    # hardware_i2c        # For I2C communication
    hardware_clocks
    # pico_multicore      # For second core usage
    # pico_cyw43_arch_none     # For wireless - alternate is pico_cyw43_arch_lwip_threadsafe_background
    # pico_cyw43_driver   # For wireless
    pico_cyw43_arch_lwip_threadsafe_background
    # pico_cyw43_arch_lwip_poll
    # pico_lwip           # For wireless, check out the pico-sdk/src/rp2_common/pico_lwip directory for build libraries
    # pico_lwip_arch
    # pico_lwip_nosys
    # tinyusb_device      # If using USB device stack (requires submodules initialized)
    # tinyusb_board       # If using USB device stack
    # Add other libraries like hardware_dma, hardware_uart, hardware_watchdog etc.
    CACHE STRING "Semicolon-separated list of Pico SDK library targets to build"
)
message(STATUS "Planning to build SDK libraries: ${PICO_SDK_TARGET_LIBS}")

pico_sdk_init()

#
# Compile the Duinotech e-ink library
#

# Add compilation subdirectories
add_subdirectory(jartis-c/eink/examples)
add_subdirectory(jartis-c/eink/lib/Config)
add_subdirectory(jartis-c/eink/lib/e-Paper)
add_subdirectory(jartis-c/eink/lib/Fonts)
add_subdirectory(jartis-c/eink/lib/GUI)

# Add header directories
include_directories(jartis-c/eink/examples)
include_directories(jartis-c/eink/lib/Config)
include_directories(jartis-c/eink/lib/e-Paper)
# include_directories(jartis-c/eink/lib/Fonts)
include_directories(jartis-c/eink/lib/GUI)

set(EINK_TARGET_LIBS
    examples
    ePaper
    GUI
    Fonts
    Config
)

#
# Compile the final program
#

# add_library(${LIBRARY_NAME} jartis/src/jartis.c jartis/src/jartis.h)
add_executable(${LIBRARY_NAME} jartis-c/main.c jartis-c/atmospheric_sensor.c jartis-c/atmospheric_sensor.h jartis-c/servo.c jartis-c/servo.h jartis-c/eink.h jartis-c/eink.c)
pico_enable_stdio_usb(${LIBRARY_NAME} 1)
pico_enable_stdio_uart(${LIBRARY_NAME} 1)
pico_add_extra_outputs(${LIBRARY_NAME})

target_include_directories(${LIBRARY_NAME} PRIVATE jartis-c)
target_link_libraries(${LIBRARY_NAME} ${PICO_SDK_TARGET_LIBS} ${EINK_TARGET_LIBS})

