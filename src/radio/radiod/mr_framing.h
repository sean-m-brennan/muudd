/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* Length-prefixed framing used on the consumer socket and the driver pipes:
 * a 2-byte big-endian length, then that many bytes of protobuf. */
#ifndef MR_FRAMING_H
#define MR_FRAMING_H

#include <protobuf-c/protobuf-c.h>

#include "mr_common.h"

typedef struct {
    uint8_t buf[2 + MR_FRAME_MAX];
    size_t len;
} mr_decoder_t;

/* Called once per complete frame. The frame pointer is valid only for the
 * duration of the call. A non-zero return stops decoding and is returned by
 * mr_decoder_feed. */
typedef int (*mr_frame_cb)(void *ctx, const uint8_t *frame, size_t len);

void mr_decoder_init(mr_decoder_t *d);
int mr_decoder_feed(mr_decoder_t *d, const uint8_t *data, size_t n, mr_frame_cb cb, void *ctx);
int mr_frame_encode(const uint8_t *payload, size_t n, uint8_t *out, size_t cap);
int mr_frame_pack(const ProtobufCMessage *msg, uint8_t *out, size_t cap);
int mr_frame_write_fd(int fd, const uint8_t *buf, size_t n);

#endif
