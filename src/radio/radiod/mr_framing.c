/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#include <errno.h>
#include <string.h>
#include <unistd.h>

#include "mr_framing.h"

void mr_decoder_init(mr_decoder_t *d) { d->len = 0; }

static size_t declared_len(const mr_decoder_t *d)
{
    return ((size_t)d->buf[0] << 8) | d->buf[1];
}

int mr_decoder_feed(mr_decoder_t *d, const uint8_t *data, size_t n, mr_frame_cb cb, void *ctx)
{
    while (n > 0) {
        size_t want;
        if (d->len < 2) {
            want = 2 - d->len;
        } else {
            size_t flen = declared_len(d);
            if (flen > MR_FRAME_MAX)
                return MR_EFRAME;
            want = 2 + flen - d->len;
        }
        size_t take = n < want ? n : want;
        memcpy(d->buf + d->len, data, take);
        d->len += take;
        data += take;
        n -= take;
        if (d->len >= 2) {
            size_t flen = declared_len(d);
            if (flen > MR_FRAME_MAX)
                return MR_EFRAME;
            if (d->len == 2 + flen) {
                d->len = 0;
                int rc = cb(ctx, d->buf + 2, flen);
                if (rc != 0)
                    return rc;
            }
        }
    }
    return MR_OK;
}

int mr_frame_encode(const uint8_t *payload, size_t n, uint8_t *out, size_t cap)
{
    if (n > MR_FRAME_MAX || n + 2 > cap)
        return -1;
    out[0] = (uint8_t)(n >> 8);
    out[1] = (uint8_t)n;
    if (n)
        memcpy(out + 2, payload, n);
    return (int)(n + 2);
}

int mr_frame_pack(const ProtobufCMessage *msg, uint8_t *out, size_t cap)
{
    size_t n = protobuf_c_message_get_packed_size(msg);
    if (n > MR_FRAME_MAX || n + 2 > cap)
        return -1;
    protobuf_c_message_pack(msg, out + 2);
    out[0] = (uint8_t)(n >> 8);
    out[1] = (uint8_t)n;
    return (int)(n + 2);
}

int mr_frame_write_fd(int fd, const uint8_t *buf, size_t n)
{
    while (n > 0) {
        ssize_t w = write(fd, buf, n);
        if (w < 0) {
            if (errno == EINTR)
                continue;
            return MR_EIO;
        }
        buf += w;
        n -= (size_t)w;
    }
    return MR_OK;
}
