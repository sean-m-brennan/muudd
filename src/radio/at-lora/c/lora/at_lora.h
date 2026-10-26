/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* libat_lora: AutonomousTrust traffic over muudd-radiod when the IP path to a
 * peer is down. Linking it registers the extension from load-time
 * constructors; a static link must reference at_lora_link() (or link the
 * archive whole) so those constructors survive. */
#ifndef AT_LORA_H
#define AT_LORA_H

/** Anchor: a static consumer calls this so the linker keeps the extension. */
void at_lora_link(void);

#endif
