/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#ifndef LORA_CONFIG_H
#define LORA_CONFIG_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* <cfg_dir>/lora.cfg.json. An absent file leaves the extension loaded and
 * inert; a malformed one refuses the network start, naming the field.
 *
 * Both runtimes agree on these edges: integers only (booleans and floats are
 * refused), numbers outside int64 or non-finite make the file unparseable,
 * a duplicate key keeps its last value, any string holding U+0000 or a lone
 * surrogate makes the file unparseable, and errors are reported in this
 * order: unknown key (document order), port, the numeric fields, the
 * nack_gap_s/reassembly_s pair, socket, allow. Field "(file)" means the text
 * is not a JSON object, or the file is unreadable or over 16 KiB. */

#define LORA_CFG_FILENAME "lora.cfg.json"
#define LORA_ALLOW_MAX 32
#define LORA_ALLOW_LEN 130

typedef struct {
    char socket[108];
    uint32_t port;
    unsigned ip_quiet_s, max_fragments, nack_gap_s, nack_hold_s, reassembly_s, announce_s;
    size_t n_allow;
    char allow[LORA_ALLOW_MAX][LORA_ALLOW_LEN];
} lora_cfg_t;

/* The defaults; port is 0 until a file sets it. */
void lora_config_defaults(lora_cfg_t *c);

/* Parse @p text. @return 0, or -1 with the offending field and a reason
 * written to @p field and @p why. */
int lora_config_parse(const char *text, lora_cfg_t *out, char *field, size_t field_len,
                      char *why, size_t why_len);

/* Read and parse @p path. @return 0, -1 as lora_config_parse, or 1 when the
 * file does not exist (inert). */
int lora_config_load(const char *path, lora_cfg_t *out, char *field, size_t field_len,
                     char *why, size_t why_len);

/* Whether "process/function" is on the allow list. */
bool lora_config_allows(const lora_cfg_t *c, const char *process, const char *function);

#endif
