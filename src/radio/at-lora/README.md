# at-lora: AutonomousTrust fallback over LoRa

An AutonomousTrust (AT) extension that carries allowlisted AT traffic over
muudd-radiod when the IP path to a peer is down. It ships in two runtimes that
speak the same wire format and share one conformance corpus: the C library
`libat_lora` and the Python distribution `autonomous_trust.lora`.

- LoRa travels in addition to IP, never instead of it. When IP heals, the IP
  copy arrives and the LoRa copy is a duplicate AT absorbs.
- Only proto-format cohorts. The JSON envelope costs two to three times the
  airtime, and the transport sees only ciphertext, so it cannot be compressed.
  A JSON cohort is logged once and nothing goes on the air.
- Only a node with a local radio. A radiod with no driver never holds
  capabilities and refuses sends, so such a node announces nothing and stays
  inert.
- IP never depends on the radio. A missing, failed or misconfigured radiod
  leaves the node exactly as it is without the extension.

## Layout

| Path | Holds |
|---|---|
| `c/lora/` | The C extension: framing, reassembly, address map, policy, config, the radiod link, the engine and the AT glue; `test/` has its unit tests and `tools/at_lora_probe.c` is the interop probe |
| `python/` | `autonomous_trust.lora`: the same units in `_python/`, the extension in `_python/extension.py`, and the tests |
| `conformance/` | The `lora` protocol's scenarios, its Python plug-in and its C adapter |
| `test-conformance.sh` | AT's conformance run with the lora corpus and both adapters plugged in |

## Build

C. The extension builds inside AT's CMake project:

    cmake -S <AT>/src/c -B build -DAT_EXTERNAL_EXTENSIONS=$PWD/c/lora \
          -DMUUDD_RADIO_DIR=<muudd>/src/radio
    cmake --build build -j8

`libat_lora.so` lands in `build/extensions/lora/`, with `libat_lora.a` and the
probe `at_lora_probe` beside it. The radiod consumer client is compiled in from
`MUUDD_RADIO_DIR` (it defaults to this tree's `src/radio`); protoc and
protobuf-c must be installed. Linking the `.so` registers the extension from
load-time constructors. A static consumer must call `at_lora_link()`, the
anchor in `at_lora.h`, so the linker keeps those constructors.

Python. Put `python/` and `src/radio/client/python` on `PYTHONPATH`, or install
both distributions (`autonomous_trust_lora` and `muudd_radio`). AT finds the
extension through its `_at_extension.py` marker or the
`autonomous_trust.extensions` entry point.

## Configuration

`<cfg_dir>/lora.cfg.json`. With no file the extension is loaded and inert. A
file that is not a JSON object, has an unknown field, a value out of range or
of the wrong type, or names `network/lora_addr` in `allow` refuses the network
start with an ERROR naming the field, as AT's other config gates do.

    {"socket": "/run/muudd/radio.sock", "port": 10,
     "ip_quiet_s": 65, "max_fragments": 8, "nack_gap_s": 20, "nack_hold_s": 120,
     "reassembly_s": 90, "announce_s": 600,
     "allow": ["identity/group_key_update", "identity/group_partition_probe",
               "identity/group_partition_response", "identity/peer_dm"]}

| Field | Default | Range |
|---|---|---|
| `socket` | `/run/muudd/radio.sock` | non-empty path |
| `port` | none, required | 3 to 255 |
| `ip_quiet_s` | 65 | 10 to 86400 |
| `max_fragments` | 8 | 1 to 16 |
| `nack_gap_s` | 20 | 1 to 600, less than `reassembly_s` |
| `nack_hold_s` | 120 | 1 to 3600 |
| `reassembly_s` | 90 | 2 to 3600 |
| `announce_s` | 600 | 30 to 86400 |
| `allow` | the four above | at most 32 distinct `process/function` strings, each part 1 to 64 characters of `[a-z0-9_]` |

Omitted fields take their defaults. An omitted `allow` means the default list;
`[]` means nothing is allowed.

radiod needs a matching port, claimed `consumer` by this extension and
configured without `ip_fallback`, so it gets the driver's full MTU and radiod
never bridges it:

    "ports": {"10": {"name": "at", "share_pct": 40, "policy": "consumer"}}

radiod's airtime budget is the only rate limit. Over-budget sends are refused,
never queued. If `port` names a radiod port that is not configured, taken by
another client or claimed under another policy, the extension logs one ERROR
naming the radiod setting to fix, stays inert, and retries on the next
reconnect.

## The allowlist rule

A LoRa copy duplicates whatever also arrived over IP, so only verbs that are
idempotent or replay-guarded belong in `allow`. AT has no registry of wire
verbs to check this against; the operator holds the rule. The four defaults
qualify:

| Verb | Why it is safe to duplicate |
|---|---|
| `identity/group_key_update` | Idempotent: applying the same key update twice leaves the same group. A member that misses a rotation cannot read the cohort when it returns, which is why it is a default. |
| `identity/group_partition_probe` | Idempotent: a repeated probe starts the same recovery. |
| `identity/group_partition_response` | Idempotent: a repeated response changes nothing the first did not. |
| `identity/peer_dm` | Replay-guarded: it carries a freshness sequence that rejects a repeat. |

Slash and exclusion notices are not in the default list. `network/lora_addr`
is refused outright: addresses are learned over the group channel, never by
LoRa.

## How it decides

A frame the core sends also goes by LoRa when the engine's policy says so.

Unicast, as `DATA_PEER`, when all six hold:

1. The link is up: connected, port claimed, radio up, a local driver present
   and an MTU of at least 64.
2. The address map holds an unconflicted LoRa address for the peer.
3. The peer is quiet on IP: the core's transport has heard nothing from it for
   at least `ip_quiet_s`. A peer never heard is measured from network start.
4. `process/function` is on the allowlist.
5. The group is proto format.
6. The frame fits in `max_fragments` at the port MTU.

Otherwise nothing is sent by LoRa. A refusal from radiod is counted, and the
IP send happens either way.

Fan-out. The quiet set is the group members that pass conditions 2 and 3.

- GROUP channel: one `DATA_GROUP` when the quiet set is not empty and
  conditions 1, 4, 5 and 6 hold, and the node has a group tag.
- BROADCAST channel: one `DATA_BCAST` when the quiet set is not empty and
  conditions 1, 4 and 6 hold. BROADCAST frames are always JSON envelopes in AT,
  whatever the group's format, so condition 5 does not apply.
- With nobody quiet, nothing goes on the air.

There is no recipient list. Every member that hears a group or broadcast frame
gets it.

"Heard" is the core's liveness record of frames that arrived through its own
transport. Frames an extension injected, this one's included, do not refresh
it, so LoRa traffic never convinces a peer to look reachable over IP. Presence
still counts every frame.

## Address announcements

Each node tells the group its LoRa address with `network/lora_addr`, on the
encrypted GROUP channel through the core transport, carrying
`{"lora_addr": <u32>, "issued_ms": <wall-clock ms>}`. The sender is the
envelope's authenticated `from_whom`, so a node announces only itself. The
address is `node_addr` from radiod's status; a status without capabilities, or
with `node_addr` 0, means no local radio and nothing is announced.

An announcement is due when:

- the node has a group (never before), the link is up and `node_addr` is not 0,
  and then
- the address changed, or the link went down and came back since the last one,
  or `announce_s` has passed, or the roster gained a member (at most once a
  minute).

Announcing is two-phase. The engine reports that one is due; the glue queues it
for the network process; only after that succeeded is it recorded as sent
(`lora.announce.sent`, the time, the roster, the persisted `issued_ms`). A
queue that is full logs a WARN (at most once a minute) and defers it to the
next period. The announcement is signed by its sender like any group message.

An announcement is accepted only when its envelope's signature verifies against
the key the receiver holds for the sender (an unverified one is refused and
counts `lora.announce.unverified`: anyone holding the group key could forge
it), only from a member of the receiver's roster, and only when its
`issued_ms` is greater than the one stored for that UUID; an older one counts
`lora.announce.stale`.

- A node issues `max(wall-clock ms, last issued + 1)` and persists the last as
  `self_issued_ms`, so a node whose clock is behind after a reboot, with no
  RTC, is not refused as stale by peers holding its older announcement.
- Conflicts fail closed. When two UUIDs hold one address, both are marked
  conflicted, frames from that address are dropped and counted
  (`lora.rx.conflict`), sends to either UUID skip LoRa, and a WARN names both.
  The mark clears when one announces a different address with a newer
  `issued_ms`.
- A radio moved from a departed node: when an announcement is accepted, any
  other entry holding the same address whose UUID is no longer on the roster is
  removed, so a holder that will never announce again cannot block it.
- Exclusion: when reputation cuts a peer off, its entry is removed. It is
  readmitted by its next announcement.
- Entries for UUIDs not in the group are kept and unused.
- A full map (256 entries) makes room for a new member by dropping the entry
  off the roster with the oldest `issued_ms`. When every entry is a member, the
  announcement is not kept, counts `lora.announce.full`, and a WARN is logged
  at most once a minute.

The map is `<data_dir>/lora_peers.json`:

    {"version": 1, "self_issued_ms": N, "peers": {"<uuid>": {"addr": A, "issued_ms": M}}}

It is written atomically (temp file, synced, then renamed, then the directory
synced) and loaded at start, so the fallback works after a reboot even when IP
fails before the next announcement. An unreadable or corrupt file logs a WARN
and starts empty; a bad entry in an otherwise good file is skipped, with one
WARN naming it, and the rest are kept.

## Frames

All frames ride the one radiod port. Integers are big-endian. Fragment payload
is the port MTU less the header; the column shows MTU 233.

| Type | Header | Payload per fragment |
|---|---|---|
| `DATA_PEER` (1) | ver/type (1), msg_id (2), frag (1) | 229 |
| `DATA_GROUP` (2) | ver/type (1), msg_id (2), frag (1), group_tag (4) | 225 |
| `DATA_BCAST` (3) | ver/type (1), msg_id (2), frag (1) | 229 |
| `NACK` (4) | ver/type (1), msg_id (2), missing (2) | none |

- ver/type: the high nibble is the version (1), the low nibble the type. An
  unknown version or type is dropped and counted.
- frag: the high nibble is the fragment index, the low nibble the fragment
  count less one, so a message has at most 16 fragments. All fragments of a
  message carry the same count; a disagreeing fragment drops the reassembly.
- msg_id: a per-sender counter that wraps, starting at a random value so a
  restart does not reuse ids a receiver still remembers. One counter covers all
  three data types.
- group_tag: the first 4 bytes of the sender's group UUID. A `DATA_GROUP`
  whose tag is not the receiver's own is dropped and counted before
  reassembly; a node in no group drops them all.
- missing: a bitmap of the fragment indexes the receiver lacks (bit i is
  fragment i).

Sending. A message's fragments go to radiod back to back. `DATA_PEER` goes
with `want_ack`; `DATA_GROUP`, `DATA_BCAST` and `NACK` do not. If radiod
refuses a fragment (`OVER_BUDGET`, `RADIO_DOWN`, `TOO_BIG`, `NOT_CLAIMED`,
`NO_ROUTE`, or a timeout waiting 2 s for the result), the rest of that message
is abandoned and the refusal counted by reason. A `DATA_PEER` message is held
for `nack_hold_s` after its last fragment to answer at most one NACK, by
resending the fragments the bitmap names; holds are capped at 16 messages and
the oldest goes first.

Receiving. The sender's UUID comes from radiod's `Received.src` through the
address map; an unknown or conflicted source is dropped and counted. A repeat
of a completed or dropped (source, msg_id) is dropped for 600 s (256 entries).
Reassembly is keyed on (source, msg_id), at most 16 in progress and 4 per
source; past either cap the oldest is evicted and counted. A `DATA_PEER`
reassembly that has gone `nack_gap_s` without a fragment sends one NACK to the
source, and one still incomplete `reassembly_s` after its first fragment is
dropped. A complete message is handed to the network process on its channel,
PEER, GROUP or BROADCAST, attributed to the mapped UUID; the core refuses an
unknown or excluded peer, and on GROUP a sender outside the group is counted
(`net.ext/group_decrypt_fail`) and does not raise a partition signal.

The link never calls radiod while holding its lock, so the network process never
blocks on the radio. It reconnects with backoff from 0.1 s doubling to 2 s and
re-claims the port. A port MTU below 64 leaves the extension inert, with one
WARN per connection. If radiod goes away between two fragments of a message,
the rest of that message is abandoned and counted, and the next message goes
out normally once the link is back.

## AT core seams

The extension needs five generic hooks in AT, in both runtimes; each leaves AT
unchanged with nothing registered, and `lora` is in AT's conformance protocol
enum.

1. `also_unicast`: called with the sealed frame for every unicast the transport
   sends. It cannot stop or alter the send.
2. `fanout`: called with the frame as sent on the GROUP or BROADCAST channel,
   and the node's group.
3. Transport liveness: seconds since a peer was last heard through the
   transport, or since network start (`net_liveness_quiet_for` in C,
   `NetworkProcess.transport_quiet_for` in Python).
4. Injection by channel: delivery of a received frame on PEER, GROUP or
   BROADCAST attributed to a peer UUID (`handle_inbound_ext`,
   `NetworkProcess.inject_inbound`). It refuses an unknown or excluded peer.
5. Declared network wire verbs (`net_ext_t.wire_verbs`,
   `Extension.network_wire_verbs`), admitted on the PEER and GROUP paths, and
   the C `check_config` hook that can refuse network start. `NET_EXT_MAX` is 8.

The C radio client library gained `mr_client_connected()`, which the link uses
to tell a live connection from a dropped one; the protocol did not change.

## Observability

Counters, the same names in both runtimes:

| Group | Names |
|---|---|
| Transmit | `lora.tx.frames`, `lora.tx.msgs`, `lora.tx.refused.<REASON>`, `lora.tx.abandoned`, `lora.tx.result_dropped` |
| Receive | `lora.rx.frames`, `lora.rx.delivered`, `lora.rx.duplicate`, `lora.rx.unknown_src`, `lora.rx.conflict`, `lora.rx.foreign_tag`, `lora.rx.bad_frame`, `lora.rx.reasm_timeout`, `lora.rx.evicted` |
| NACK | `lora.nack.sent`, `lora.nack.served` |
| Announce | `lora.announce.sent`, `lora.announce.accepted`, `lora.announce.stale`, `lora.announce.unverified`, `lora.announce.full` |

`lora.tx.abandoned` also counts a message refused locally because the link's
job queue (16 messages) is full; IP sends are unaffected.
`lora.tx.result_dropped` counts send results the link threw away because its
result queue (32) was full, so the refusals behind them are missing from
`lora.tx.refused.<REASON>`. The core adds
`net.ext/group_decrypt_fail`.

The local verb `network/lora_stats` returns the counters, the link state and
the address map (UUID, address, conflicted) as `lora_stats_resp` to the
message's `return_to`. It is refused from the wire. An INFO summary line goes
out every five minutes. There is no shutdown summary, because AT has no network
stop hook.

## Limits

- A node behind a driverless radiod gets no fallback.
- LoRa cannot reach a peer that is fully down; radiod's budget caps the waste.
- Duplicate absorption depends on the allowlist rule.
- Meshtastic's hop limit bounds how far a fallback frame travels.
- An asymmetric IP outage (we hear the peer, it does not hear us) uses LoRa in
  one direction only.
- A `group_tag` collision with a foreign cohort costs counted decrypt
  failures, nothing more.
- A message of more than `max_fragments` fragments is never sent by LoRa.
  Admission and `full_history` do not travel by LoRa.
- `ip_quiet_s` and AT's presence timings are set together. A healthy idle peer
  can go up to two heartbeat intervals (60 s at AT's default 30 s) without an IP
  frame, so `ip_quiet_s` must be longer than that or LoRa carries traffic while
  IP works. After an outage starts, the peer's next heartbeat can take another
  60 s to go out over LoRa, so AT's `AT_PRESENCE_ABSENT_SEC` must be longer than
  `ip_quiet_s` plus 60 s, with room for airtime, or the peer is marked absent
  first. The defaults, 65 s and 150 s, meet both. Change one and check the
  other.

## Tests

| Command | Runs |
|---|---|
| `python/run-tests.sh` | Python unit tests (`tests/a_unit`) |
| `ctest --test-dir build -R '^lora_' -j8` | The extension's C unit tests, in the C build above (the filter leaves out AT's own tests, which share the build) |
| `cd python && python -m pytest tests -q -m 'not slow'` | Python units plus the link, interop and announcement integration tests, the first two against real radiod processes on stub drivers |
| `cd python && python -m pytest tests -q -m slow` | The AT-node test: two real AT nodes and two radiods, IP cut and healed, radiod killed and LoRa used again once it is back |
| `./test-conformance.sh` | AT's conformance run with the `lora` corpus, C and Python |

The interop test needs the C probe built (`build/extensions/lora/at_lora_probe`)
and the radiod binaries in `src/radio/build`. It exchanges a three-fragment
unicast between the C engine and the Python engine over two real radiods, with
one fragment dropped and recovered by NACK. The announcement test signs a
`lora_addr` announcement in each runtime and verifies it in the other, through
AT's native library; it is skipped when that library is not built. A full C AT
node over LoRa is a step in `src/radio/test/bench-checklist.md`.

`test-conformance.sh` forwards its arguments to AT's `scripts/test-conformance.sh`
(for instance `--no-clean`), and honours `AT_ROOT` and
`AT_CONFORMANCE_BUILD_DIR`.
