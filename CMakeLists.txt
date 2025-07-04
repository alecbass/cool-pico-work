cmake_minimum_required(VERSION 3.13)

# project(PicoSdkStaticLibsBuilder LANGUAGES NONE) # This project only orchestrates the SDK build

# --- User Configuration ---

# 1. Pico SDK Path: MUST be set correctly.
#    Set via environment: export PICO_SDK_PATH=/path/to/pico-sdk
#    Or via CMake argument: cmake .. -DPICO_SDK_PATH=/path/to/pico-sdk
set(PICO_SDK_PATH "$ENV{PICO_SDK_PATH}" CACHE PATH "Path to the Raspberry Pi Pico SDK source directory")
if(NOT PICO_SDK_PATH OR NOT EXISTS "${PICO_SDK_PATH}/pico_sdk_init.cmake")
    message(FATAL_ERROR "PICO_SDK_PATH is not set or invalid ('${PICO_SDK_PATH}'). Please set the environment variable or use -DPICO_SDK_PATH=...")
endif()
message(STATUS "Using Pico SDK source from: ${PICO_SDK_PATH}")

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
    stdlib
    pico_stdlib         # Base library, almost always needed
    # hardware_gpio       # For GPIO control
    # hardware_timer      # For hardware timers, delays
    # hardware_pwm        # For PWM output
    # hardware_adc        # For ADC input
    # hardware_spi        # For SPI communication
    # hardware_i2c        # For I2C communication
    pico_multicore      # For second core usage
    pico_cyw43_arch     # For wireless - alternate is pico_cyw43_arch_lwip_threadsafe_background
    pico_cyw43_driver   # For wireless
    pico_cyw43_arch_lwip_threadsafe_background
    pico_lwip           # For wireless, check out the pico-sdk/src/rp2_common/pico_lwip directory for build libraries
    pico_lwip_arch
    pico_lwip_nosys
    # tinyusb_device      # If using USB device stack (requires submodules initialized)
    # tinyusb_board       # If using USB device stack
    # Add other libraries like hardware_dma, hardware_uart, hardware_watchdog etc.
    CACHE STRING "Semicolon-separated list of Pico SDK library targets to build"
)
message(STATUS "Planning to build SDK libraries: ${PICO_SDK_TARGET_LIBS}")

# 3. Output Directory for Compiled Artifacts
# set(PICO_INSTALL_DIR ${CMAKE_CURRENT_BINARY_DIR}/pico_install)
# set(PICO_LIBS_DIR ${PICO_INSTALL_DIR}/lib)
# set(PICO_HEADERS_DIR ${PICO_INSTALL_DIR}/include)
# message(STATUS "Compiled libraries and headers will be placed in: ${PICO_INSTALL_DIR}")

# --- Toolchain Configuration ---
# Ensure ARM GCC toolchain executables (arm-none-eabi-gcc etc.) are in your PATH
# We explicitly pass these to the external project's CMake configuration.
# set(TOOLCHAIN_PREFIX "arm-none-eabi-")
set(TOOLCHAIN_PREFIX "${ARM_EMBEDDED_DIR}/bin/arm-none-eabi-")
set(CMAKE_C_COMPILER   "${TOOLCHAIN_PREFIX}gcc")
set(CMAKE_CXX_COMPILER "${TOOLCHAIN_PREFIX}g++")
set(CMAKE_ASM_COMPILER "${TOOLCHAIN_PREFIX}gcc")
set(CMAKE_AR           "${TOOLCHAIN_PREFIX}ar")
set(CMAKE_OBJCOPY      "${TOOLCHAIN_PREFIX}objcopy")
set(CMAKE_OBJDUMP      "${TOOLCHAIN_PREFIX}objdump")
#
# set(COMMON_COMPILE_FLAGS "-mcpu=cortex-m0plus -mthumb -O2") # Optimization level 2
# set(CMAKE_C_FLAGS   "${COMMON_COMPILE_FLAGS}")
# set(CMAKE_CXX_FLAGS "${COMMON_COMPILE_FLAGS} -nostdlib") # Often need -nostdlib for C++ SDK parts
# set(CMAKE_ASM_FLAGS "-mcpu=cortex-m0plus -mthumb")
# set(CMAKE_EXE_LINKER_FLAGS "-mcpu=cortex-m0plus -mthumb --specs=nosys.specs")
# #
# #
# # --- External Project: Build the Pico SDK ---
# include(ExternalProject)
# #
# set(SDK_BUILD_DIR ${CMAKE_CURRENT_BINARY_DIR}/_pico_sdk_build) # Temp build dir for SDK
#
# ExternalProject_Add(
#     pico_sdk_external_build # Name for this build step
#     SOURCE_DIR ${PICO_SDK_PATH}
#     BINARY_DIR ${SDK_BUILD_DIR}
#
#     # --- Configuration Arguments for SDK's CMake ---
#     CMAKE_ARGS
#         # Toolchain settings
#         -DCMAKE_SYSTEM_NAME=Generic
#         -DCMAKE_SYSTEM_PROCESSOR=arm
#         -DCMAKE_C_COMPILER=${CMAKE_C_COMPILER}
#         -DCMAKE_CXX_COMPILER=${CMAKE_CXX_COMPILER}
#         -DCMAKE_ASM_COMPILER=${CMAKE_ASM_COMPILER}
#         -DCMAKE_AR=${CMAKE_AR}
#         -DCMAKE_OBJCOPY=${CMAKE_OBJCOPY}
#         -DCMAKE_OBJDUMP=${CMAKE_OBJDUMP}
#         -DCMAKE_C_FLAGS=${CMAKE_C_FLAGS}
#         -DCMAKE_CXX_FLAGS=${CMAKE_CXX_FLAGS}
#         -DCMAKE_ASM_FLAGS=${CMAKE_ASM_FLAGS}
#         -DCMAKE_EXE_LINKER_FLAGS=${CMAKE_EXE_LINKER_FLAGS}
#         # Pico SDK settings
#         -DPICO_SDK_PATH=${PICO_SDK_PATH}
#         -DPICO_PLATFORM=rp2040
#         -DCMAKE_BUILD_TYPE=Release # Or Debug
#         -DBUILD_SHARED_LIBS=OFF      # Force static libraries
#         # Add other PICO_* options if needed, e.g.:
#         # -DPICO_USE_USB=1
#         # -DPICO_STDIO_UART=1
#         # -DPICO_STDIO_USB=0
#
#     # --- Build Step ---
#     # Build only the specified library targets within the SDK build dir
#     BUILD_COMMAND ${CMAKE_COMMAND} --build <BINARY_DIR> --target ${PICO_SDK_TARGET_LIBS}
#
#     # --- Install Step ---
#     # Copy the needed *.a files and headers to our clean install directory
#     INSTALL_COMMAND ${CMAKE_COMMAND} -E make_directory ${PICO_LIBS_DIR} ${PICO_HEADERS_DIR} &&
#                     # Use CMake's file GLOB to find and copy the built libraries
#                     # This is more robust than hardcoding paths which might change between SDK versions
#                     ${CMAKE_COMMAND} -E echo "Copying libraries..." &&
#                     file(GLOB_RECURSE lib_files RELATIVE <BINARY_DIR> <BINARY_DIR>/*.a <BINARY_DIR>/**/*.a) &&
#                     foreach(lib_file ${lib_files})
#                         # Check if the library name (without lib/.a) is in our target list
#                         # This logic is a bit simplified, relies on target names matching lib names
#                         string(REGEX REPLACE "^.*/lib([^/]+)\\.a$" "\\1" lib_name ${lib_file})
#                         list(FIND PICO_SDK_TARGET_LIBS "${lib_name}" _index)
#                         if(NOT _index EQUAL -1)
#                            message(STATUS "Copying library: ${lib_file}")
#                            ${CMAKE_COMMAND} -E copy <BINARY_DIR>/${lib_file} ${PICO_LIBS_DIR}/
#                         endif()
#                     endforeach()
#                     &&
#                     # Copy selected header directories (adjust which ones you need)
#                     ${CMAKE_COMMAND} -E echo "Copying headers..." &&
#                     ${CMAKE_COMMAND} -E copy_directory ${PICO_SDK_PATH}/src/common/pico_base/include/ ${PICO_HEADERS_DIR}/pico_base/ &&
#                     ${CMAKE_COMMAND} -E copy_directory ${PICO_SDK_PATH}/src/common/pico_stdlib/include/ ${PICO_HEADERS_DIR}/pico_stdlib/ &&
#                     ${CMAKE_COMMAND} -E copy_directory ${PICO_SDK_PATH}/src/rp2_common/pico_platform/include/ ${PICO_HEADERS_DIR}/pico_platform/ &&
#                     # Copy common hardware headers
#                     ${CMAKE_COMMAND} -E copy_directory ${PICO_SDK_PATH}/src/rp2_common/hardware_base/include/ ${PICO_HEADERS_DIR}/hardware_base/ &&
#                     # Copy specific hardware headers based on target libs (add more as needed)
#                     ${CMAKE_COMMAND} -E copy_directory ${PICO_SDK_PATH}/src/rp2_common/hardware_gpio/include/ ${PICO_HEADERS_DIR}/hardware_gpio/ &&
#                     ${CMAKE_COMMAND} -E copy_directory ${PICO_SDK_PATH}/src/rp2_common/hardware_timer/include/ ${PICO_HEADERS_DIR}/hardware_timer/ &&
#                     ${CMAKE_COMMAND} -E copy_directory ${PICO_SDK_PATH}/src/rp2_common/hardware_pwm/include/ ${PICO_HEADERS_DIR}/hardware_pwm/ &&
#                     ${CMAKE_COMMAND} -E copy_directory ${PICO_SDK_PATH}/src/rp2_common/hardware_adc/include/ ${PICO_HEADERS_DIR}/hardware_adc/ &&
#                     ${CMAKE_COMMAND} -E copy_directory ${PICO_SDK_PATH}/src/rp2_common/hardware_spi/include/ ${PICO_HEADERS_DIR}/hardware_spi/ &&
#                     ${CMAKE_COMMAND} -E copy_directory ${PICO_SDK_PATH}/src/rp2_common/hardware_i2c/include/ ${PICO_HEADERS_DIR}/hardware_i2c/ &&
#                     ${CMAKE_COMMAND} -E copy_directory ${PICO_SDK_PATH}/src/rp2_common/pico_multicore/include/ ${PICO_HEADERS_DIR}/pico_multicore/ &&
#                     # Copy TinyUSB headers if needed
#                     # ${CMAKE_COMMAND} -E copy_directory ${PICO_SDK_PATH}/lib/tinyusb/src/ ${PICO_HEADERS_DIR}/tinyusb/ && \
#                     # Copy generated config headers (CRITICAL!)
#                     ${CMAKE_COMMAND} -E copy_directory <BINARY_DIR>/generated/pico_base/include/pico ${PICO_HEADERS_DIR}/pico/ # Contains config_autogen.h
#
#     # --- Avoid Unneeded Steps ---
#     CONFIGURE_COMMAND "" # Disable explicit configure step, use CMAKE_ARGS instead
#     UPDATE_COMMAND ""    # Disable source update
#     PATCH_COMMAND ""     # Disable patching
#     DOWNLOAD_COMMAND ""  # Disable download
#
#     # Make install step part of the default build
#     STEP_TARGETS install
# )

# Add a top-level target that triggers the external project's install step
# add_custom_target(build_pico_sdk_libs ALL DEPENDS pico_sdk_external_build)
#
# # --- Final Message ---
# # Prints instructions after the build finishes successfully
# add_custom_command(TARGET build_pico_sdk_libs POST_BUILD
#     COMMAND ${CMAKE_COMMAND} -E echo "-----------------------------------------------------"
#     COMMAND ${CMAKE_COMMAND} -E echo "Pico SDK static libraries built successfully!"
#     COMMAND ${CMAKE_COMMAND} -E echo "Libraries (.a files) are in: ${PICO_LIBS_DIR}"
#     COMMAND ${CMAKE_COMMAND} -E echo "Headers (.h files) are in:   ${PICO_HEADERS_DIR}"
#     COMMAND ${CMAKE_COMMAND} -E echo " "
#     COMMAND ${CMAKE_COMMAND} -E echo "Configure your Rust project's build.rs:"
#     COMMAND ${CMAKE_COMMAND} -E echo "  1. Set env var PICO_PRECOMPILED_PATH=${PICO_INSTALL_DIR}"
#     COMMAND ${CMAKE_COMMAND} -E echo "  2. Add link search path: println!(\"cargo:rustc-link-search=native={}\", env::var(\"PICO_PRECOMPILED_PATH\").unwrap() + \"/lib\");"
#     COMMAND ${CMAKE_COMMAND} -E echo "  3. Link needed static libs:"
#     COMMAND ${CMAKE_COMMAND} -E echo "     println!(\"cargo:rustc-link-lib=static=pico_stdlib\");"
#     COMMAND ${CMAKE_COMMAND} -E echo "     println!(\"cargo:rustc-link-lib=static=hardware_gpio\"); // (Add others...)"
#     COMMAND ${CMAKE_COMMAND} -E echo "  4. Configure bindgen include paths, e.g.:"
#     COMMAND ${CMAKE_COMMAND} -E echo "     .clang_arg(format!(\"-I{}\", env::var(\"PICO_PRECOMPILED_PATH\").unwrap() + \"/include\"))"
#     COMMAND ${CMAKE_COMMAND} -E echo "-----------------------------------------------------"
#     COMMENT "Build complete. Use the paths above in your Rust project."
# )

pico_sdk_init()
add_library(${LIBRARY_NAME} jartis/src/jartis.c jartis/src/jartis.h)
# add_executable(${LIBRARY_NAME} test.c)
# pico_add_extra_outputs(${LIBRARY_NAME})
target_include_directories(${LIBRARY_NAME} PRIVATE jartis/src)
target_link_libraries(${LIBRARY_NAME} ${PICO_SDK_TARGET_LIBS})

