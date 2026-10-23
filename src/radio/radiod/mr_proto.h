/*
 * Copyright 2026 Sean M. Brennan and contributors
 * SPDX-License-Identifier: Apache-2.0
 */
/* The generated protobuf-c header, with short names for the message types
 * every unit uses. Enum values keep their generated names. */
#ifndef MR_PROTO_H
#define MR_PROTO_H

#include "radio.pb-c.h"

typedef Muudd__Radio__ConsumerToRadiod MrC2R;
typedef Muudd__Radio__RadiodToConsumer MrR2C;
typedef Muudd__Radio__RadiodToDriver MrR2D;
typedef Muudd__Radio__DriverToRadiod MrD2R;
typedef Muudd__Radio__BridgeDatagram MrDatagram;
typedef Muudd__Radio__BridgeFrame MrBridgeFrame;
typedef Muudd__Radio__Keepalive MrKeepalive;
typedef Muudd__Radio__Send MrSend;
typedef Muudd__Radio__SendResult MrSendResult;
typedef Muudd__Radio__Received MrReceived;
typedef Muudd__Radio__Meta MrMeta;
typedef Muudd__Radio__Caps MrCaps;
typedef Muudd__Radio__ClaimResult MrClaimResult;
typedef Muudd__Radio__RadioStatus MrRadioStatus;
typedef Muudd__Radio__Stats MrStats;

#endif
