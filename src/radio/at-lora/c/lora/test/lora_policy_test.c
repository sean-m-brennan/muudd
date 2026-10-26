/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
#define DEBUG_TESTS 1
#include "test_setup.h"

#include <string.h>

#include "lora/lora_config.h"
#include "lora/lora_policy.h"

DEFINE_TEST(test_unicast_needs_every_condition)
{
    lora_unicast_in_t in = {true, true, 90, 90, true, true, 100, 1832};
    ck_assert(lora_policy_unicast(&in));
    lora_unicast_in_t v;
    v = in; v.link_up = false;   ck_assert(!lora_policy_unicast(&v));
    v = in; v.have_addr = false; ck_assert(!lora_policy_unicast(&v));
    v = in; v.quiet_s = 89.9;    ck_assert(!lora_policy_unicast(&v));
    v = in; v.allowed = false;   ck_assert(!lora_policy_unicast(&v));
    v = in; v.proto = false;     ck_assert(!lora_policy_unicast(&v));
    v = in; v.frame_len = 1833;  ck_assert(!lora_policy_unicast(&v));
    v = in; v.frame_len = 0;     ck_assert(!lora_policy_unicast(&v));
}
END_TEST_DEFINITION()

DEFINE_TEST(test_fanout_broadcast_ignores_format)
{
    lora_fanout_in_t g = {LORA_CH_GROUP, true, 1, true, false, 100, 1800};
    ck_assert(!lora_policy_fanout(&g));
    g.channel = LORA_CH_BCAST;
    ck_assert(lora_policy_fanout(&g));
    g.n_quiet = 0;
    ck_assert(!lora_policy_fanout(&g));
}
END_TEST_DEFINITION()

DEFINE_TEST(test_config_defaults_and_refusals)
{
    lora_cfg_t c;
    char field[64], why[160];
    ck_assert_int_eq(lora_config_parse("{\"port\": 10}", &c, field, sizeof(field), why, sizeof(why)), 0);
    ck_assert_str_eq(c.socket, "/run/muudd/radio.sock");
    ck_assert_int_eq((int)c.ip_quiet_s, 90);
    ck_assert_int_eq((int)c.n_allow, 4);
    ck_assert(lora_config_allows(&c, "identity", "peer_dm"));
    ck_assert(!lora_config_allows(&c, "identity", "presence"));
    ck_assert_int_eq(lora_config_parse("{\"port\": true}", &c, field, sizeof(field), why, sizeof(why)), -1);
    ck_assert_str_eq(field, "port");
    ck_assert_int_eq(lora_config_parse("{\"port\": 10, \"nack_gap_s\": 90}", &c, field, sizeof(field),
                                       why, sizeof(why)), -1);
    ck_assert_str_eq(field, "nack_gap_s");
    ck_assert_int_eq(lora_config_parse("{\"port\": 10, \"allow\": [\"network/lora_addr\"]}", &c, field,
                                       sizeof(field), why, sizeof(why)), -1);
    ck_assert_str_eq(field, "allow");
    ck_assert_int_eq(lora_config_load("/nonexistent/lora.cfg.json", &c, field, sizeof(field), why,
                                      sizeof(why)), 1);
}
END_TEST_DEFINITION()

RUN_TESTS(LORA_POLICY,
          test_unicast_needs_every_condition,
          test_fanout_broadcast_ignores_format,
          test_config_defaults_and_refusals)
