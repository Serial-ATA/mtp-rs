cmake_minimum_required(VERSION 3.10)
project(mtp-plugin C)

set(CMAKE_C_STANDARD 11)

set(WIRESHARK_VERSION_MAJOR 4)
set(WIRESHARK_VERSION_MINOR 4)

# --- Rust ---
set(RUST_CRATE_PATH ${CMAKE_SOURCE_DIR})
set(RUST_LIB_NAME mtp_epan)

execute_process(
        COMMAND cargo build --release
        WORKING_DIRECTORY ${RUST_CRATE_PATH}
        RESULT_VARIABLE CARGO_RESULT
)

if(NOT CARGO_RESULT EQUAL 0)
    message(FATAL_ERROR "Cargo build failed!")
endif()

set(RUST_OUTPUT_LIB ${RUST_CRATE_PATH}/../target/release/lib${RUST_LIB_NAME}.so)
# -------------------------


# --- Wireshark Plugin Dependencies ---
find_package(PkgConfig REQUIRED)
pkg_check_modules(WIRESHARK REQUIRED wireshark)
pkg_check_modules(GLIB REQUIRED glib-2.0)

include_directories(${WIRESHARK_INCLUDE_DIRS} ${GLIB_INCLUDE_DIRS})
# ------------------------------------


# --- C Plugin Compilation and Linking ---
add_library(mtp SHARED mtp-epan.c
        mtp-epan.h)

target_compile_definitions(mtp PRIVATE
        -DHAVE_CONFIG_H
        -D_GNU_SOURCE
)

target_link_libraries(mtp
        ${WIRESHARK_LIBRARIES}
        ${GLIB_LIBRARIES}
        ${RUST_OUTPUT_LIB}
)
# -----------------------------------------


# --- Final Plugin Output ---
set_target_properties(mtp PROPERTIES
        PREFIX ""
        SUFFIX ".so"
        INSTALL_RPATH "$ORIGIN"
)

install(TARGETS mtp
        DESTINATION ${CMAKE_INSTALL_PREFIX}/lib/wireshark/plugins/${WIRESHARK_VERSION_MAJOR}.${WIRESHARK_VERSION_MINOR}/epan
)

install(FILES ${RUST_OUTPUT_LIB}
        DESTINATION lib
)
