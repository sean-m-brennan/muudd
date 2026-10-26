/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#ifndef LORA_EXT_PRIV_H
#define LORA_EXT_PRIV_H

/* The glue's seam below `start`: the engine over a given link, with the
 * glue's own environment (roster, liveness, group format and tag, clocks)
 * read from @p ctx. `start` uses it with the radiod link; tests use it with
 * a fake link, so the hooks can be driven without a radio. */

#include "network/net_proc_priv.h"

#include "lora/lora_config.h"
#include "lora/lora_engine.h"
#include "lora/lora_link.h"

/* Make the extension live over @p link for @p ctx, its address map at
 * @p addrmap_path. @return the engine (owned by the glue), or NULL. */
lora_engine_t *lora_ext_attach(net_thread_ctx_t *ctx, const lora_cfg_t *cfg,
                               lora_link_ops_t link, const char *addrmap_path);

/* Back to inert: frees the engine, stops the radiod link if `start` made one. */
void lora_ext_detach(void);

#endif /* LORA_EXT_PRIV_H */
