# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
cmake_minimum_required(VERSION 3.23)

# Prefer the active conda env's libraries over the system's, at link and at
# run time.
if(DEFINED ENV{CONDA_PREFIX})
    list(PREPEND CMAKE_PREFIX_PATH $ENV{CONDA_PREFIX})
    set(CMAKE_BUILD_RPATH "$ENV{CONDA_PREFIX}/lib")
    set(CMAKE_INSTALL_RPATH "$ENV{CONDA_PREFIX}/lib")
endif()

project(muudd_radio C)
set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
add_compile_definitions(_GNU_SOURCE)
add_compile_options(-Wall -Wextra -Werror=implicit-function-declaration)

option(MR_SANITIZE "Build with ASan and UBSan" OFF)
if(MR_SANITIZE)
    add_compile_options(-fsanitize=address,undefined -fno-omit-frame-pointer)
    add_link_options(-fsanitize=address,undefined)
endif()

find_path(SODIUM_INCLUDE_DIR sodium.h REQUIRED)
find_library(SODIUM_LIBRARY sodium REQUIRED)
find_path(JANSSON_INCLUDE_DIR jansson.h REQUIRED)
find_library(JANSSON_LIBRARY jansson REQUIRED)
find_path(PROTOBUF_C_INCLUDE_DIR protobuf-c/protobuf-c.h REQUIRED)
find_library(PROTOBUF_C_LIBRARY protobuf-c REQUIRED)
find_program(PROTOC protoc REQUIRED)
find_program(PROTOC_GEN_C protoc-gen-c REQUIRED)
find_path(CHECK_INCLUDE_DIR check.h)
find_library(CHECK_LIBRARY check)
find_library(SUBUNIT_LIBRARY subunit)
if(NOT CHECK_LIBRARY OR NOT CHECK_INCLUDE_DIR)
    message(WARNING "libcheck not found: unit tests will not be built")
endif()

set(MR_GEN_DIR ${CMAKE_CURRENT_BINARY_DIR}/gen)
file(MAKE_DIRECTORY ${MR_GEN_DIR})
add_custom_command(
    OUTPUT ${MR_GEN_DIR}/radio.pb-c.c ${MR_GEN_DIR}/radio.pb-c.h
    COMMAND ${PROTOC} --plugin=protoc-gen-c=${PROTOC_GEN_C}
            --c_out=${MR_GEN_DIR} -I${CMAKE_CURRENT_SOURCE_DIR}/proto radio.proto
    DEPENDS ${CMAKE_CURRENT_SOURCE_DIR}/proto/radio.proto
    COMMENT "Generating protobuf-c sources for radio.proto")

add_library(mr_core STATIC
    radiod/mr_common.c
    ${MR_GEN_DIR}/radio.pb-c.c)
target_sources(mr_core PRIVATE radiod/mr_framing.c)
target_sources(mr_core PRIVATE radiod/mr_airtime.c)
target_sources(mr_core PRIVATE radiod/mr_ledger.c)
target_sources(mr_core PRIVATE radiod/mr_dedupe.c)
target_sources(mr_core PRIVATE radiod/mr_config.c)
target_sources(mr_core PRIVATE radiod/mr_crypto.c)
target_sources(mr_core PRIVATE radiod/mr_ports.c radiod/mr_peers.c)
target_sources(mr_core PRIVATE radiod/core.c radiod/core_bridge.c radiod/core_relay.c)
target_include_directories(mr_core PUBLIC
    radiod ${MR_GEN_DIR}
    ${SODIUM_INCLUDE_DIR} ${JANSSON_INCLUDE_DIR} ${PROTOBUF_C_INCLUDE_DIR})
target_link_libraries(mr_core PUBLIC
    ${SODIUM_LIBRARY} ${JANSSON_LIBRARY} ${PROTOBUF_C_LIBRARY} m)

enable_testing()
function(mr_add_test name)
    if(CHECK_LIBRARY AND CHECK_INCLUDE_DIR)
        add_executable(${name} test/unit/${name}.c)
        target_include_directories(${name} PRIVATE ${CHECK_INCLUDE_DIR} test/unit)
        target_link_libraries(${name} PRIVATE mr_core ${CHECK_LIBRARY} pthread)
        if(SUBUNIT_LIBRARY)
            target_link_libraries(${name} PRIVATE ${SUBUNIT_LIBRARY})
        endif()
        add_test(NAME ${name} COMMAND ${name})
    endif()
endfunction()

mr_add_test(test_proto)
mr_add_test(test_framing)
mr_add_test(test_airtime)
mr_add_test(test_ledger)
mr_add_test(test_dedupe)
mr_add_test(test_config)
mr_add_test(test_crypto)
mr_add_test(test_ports)
mr_add_test(test_peers)
mr_add_test(test_core_local)
mr_add_test(test_core_bridge)
