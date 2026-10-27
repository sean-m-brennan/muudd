# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
#
# libat_lora: AutonomousTrust traffic over muudd-radiod when the IP path to a
# peer is down. Built inside AutonomousTrust's CMake project, named in
# AT_EXTERNAL_EXTENSIONS, so at_extension_library and the core's flags and
# include directories apply as they do to an in-tree extension.

# radiod's Apache-2.0 consumer client, built here from muudd's sources as
# position-independent objects: the client, its frame codec and the protobuf-c
# messages generated from radio.proto. An OBJECT library, so the objects go
# into libat_lora.so and libat_lora.a both; as a static library of its own it
# was left out of libat_lora.a, which no static consumer could then link.
if(NOT MUUDD_RADIO_DIR)
    get_filename_component(MUUDD_RADIO_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../.." ABSOLUTE)
endif()
find_program(LORA_PROTOC protoc REQUIRED)
find_program(LORA_PROTOC_GEN_C protoc-gen-c REQUIRED)
find_path(LORA_PROTOBUF_C_INCLUDE protobuf-c/protobuf-c.h REQUIRED)
find_library(LORA_PROTOBUF_C_LIB protobuf-c REQUIRED)
set(LORA_GEN ${CMAKE_CURRENT_BINARY_DIR}/gen)
file(MAKE_DIRECTORY ${LORA_GEN})
add_custom_command(
    OUTPUT ${LORA_GEN}/radio.pb-c.c ${LORA_GEN}/radio.pb-c.h
    COMMAND ${LORA_PROTOC} --plugin=protoc-gen-c=${LORA_PROTOC_GEN_C}
            --c_out=${LORA_GEN} -I${MUUDD_RADIO_DIR}/proto radio.proto
    DEPENDS ${MUUDD_RADIO_DIR}/proto/radio.proto
    COMMENT "Generating protobuf-c sources for radio.proto (at-lora)")
add_library(at_lora_radio OBJECT
    ${MUUDD_RADIO_DIR}/client/c/mr_client.c
    ${MUUDD_RADIO_DIR}/radiod/mr_framing.c
    ${MUUDD_RADIO_DIR}/radiod/mr_common.c
    ${LORA_GEN}/radio.pb-c.c)
set_target_properties(at_lora_radio PROPERTIES POSITION_INDEPENDENT_CODE ON)
target_compile_definitions(at_lora_radio PRIVATE _GNU_SOURCE)
# AT's flags include -Wswitch-enum; radiod's sources switch on protobuf oneof
# cases with a default and are built under their own flags in muudd.
target_compile_options(at_lora_radio PRIVATE -Wno-switch-enum)
target_include_directories(at_lora_radio PUBLIC
    ${MUUDD_RADIO_DIR}/client/c ${MUUDD_RADIO_DIR}/radiod ${LORA_GEN} ${LORA_PROTOBUF_C_INCLUDE})
target_link_libraries(at_lora_radio PUBLIC ${LORA_PROTOBUF_C_LIB} pthread)

at_extension_library(lora
    SOURCES
        lora_frame.c
        lora_reasm.c
        lora_addrmap.c
        lora_policy.c
        lora_config.c
        lora_link.c
        lora_engine.c
        lora_ext.c
)

target_link_libraries(at_lora PRIVATE at_lora_radio)
target_link_libraries(at_lora_static PUBLIC at_lora_radio)
target_include_directories(at_lora_static PUBLIC $<TARGET_PROPERTY:at_lora_radio,INTERFACE_INCLUDE_DIRECTORIES>)
target_include_directories(at_lora PRIVATE $<TARGET_PROPERTY:at_lora_radio,INTERFACE_INCLUDE_DIRECTORIES>)

at_extension_tests(lora
    lora_frame_test
    lora_reasm_test
    lora_addrmap_test
    lora_policy_test
    lora_link_test
    lora_engine_test
    lora_ext_test
)

# The `lora` conformance protocol: its corpus and its C adapter. The Python
# half is the plug-in in ../../conformance; ../../test-conformance.sh runs both.
at_conformance_adapter(
    PROTOCOL lora
    RUN at_lora_conf_run
    SOURCES ../../conformance/c/lora_adapter.c
    LIBS at_lora_static
    CORPUS ../../conformance)

# A test tool on the C engine for the interop tests; not installed.
get_filename_component(_root "${CMAKE_CURRENT_SOURCE_DIR}" DIRECTORY)
add_executable(at_lora_probe tools/at_lora_probe.c)
target_link_libraries(at_lora_probe PRIVATE at_lora_static at_lora_radio jansson sodium uuid)
target_include_directories(at_lora_probe PRIVATE ${_root})
