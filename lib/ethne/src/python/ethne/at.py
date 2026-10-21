#  Copyright 2026 Sean M. Brennan and contributors
#
#   Licensed under the Apache License, Version 2.0 (the "License");
#   you may not use this file except in compliance with the License.
#   You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
#   Unless required by applicable law or agreed to in writing, software
#   distributed under the License is distributed on an "AS IS" BASIS,
#   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#   See the License for the specific language governing permissions and
#   limitations under the License.

"""The autonomous_trust substrate, from Python (decisions **D5**-**D8**, **D33**).

An `AtProvider` folds an autonomous_trust event stream into the observed
gateway-reputation subtree and presents it as a substrate a polity can be chartered
from. It only *reports* what AT observes and never confers standing: what comes out is
a `FoundingCandidate`, which is input to a proposal the polity itself must ratify.

    from ethne.at import AtProvider, did_from_pubkey

    at = AtProvider()
    at.observe_peer(node_key, reputation=0.83, operator_bound=True,
                    guardian_pubkey=operator_key, operator_attested_at=now)
    at.observe_peer(other_key, reputation=0.71)          # a machine with no operator

    for change in at.changes():                         # folds, then drains
        print(change.kind, change.member)

    candidate = at.founding_candidate()
    proposal = propose_from_candidate(candidate, polity_did, min_reputation=0.5)

**Feed it AT's wire fields and let the fold reduce them.** `guardian_pubkey` becomes a
guardian edge and `operator_attested_at` an attendance stamp *only* when
`operator_bound` is true, because either one on a node whose operator credential never
verified attests to nothing (**D8**). Passing them with `operator_bound=False` is not an
error — it is exactly what AT sends for an unverified node — and the roll then reads it
as the unguarded machine it is, which is what the office rule of **D15** exists for.

That reduction is the reason this module exists rather than leaving callers to build the
roll themselves from the same fields: hand-rolled, it silently reported a guardian and
an attendance stamp for precisely that node, which reads an unguarded machine as guarded
and attended and looks entirely well-formed while doing it.

## The live couplings

A bare `AtProvider()` folds a **replayed** feed — the events you hand it — which is what
offline founding and tests use, and it needs no autonomous_trust linkage at all. The live
couplings exist only in a build made with `maturin develop --features at-ffi`:

    if AT_FFI:
        at = AtProvider.attached(q_in="/at-in", q_out="/at-out")
    else:
        at = AtProvider()       # replay a captured view

Ask `AT_FFI` (or `at_ffi_available()`) rather than catching an exception: in a default
build those constructors are **absent**, not present-and-failing. `ATTACHED`, `EMBEDDED`
and `LOG_LEVELS` below are `None` / empty in that build for the same reason.

Two properties of an `at-ffi` build worth knowing before you write against it: the
provider is **thread-bound** (the C handles are raw pointers, so it may only be used
from the thread that made it), and `AtProvider.embedded` should be given a `log_file`
under any harness that captures stderr — without one the daemon keeps this process's
stderr for its whole subsystem tree, and a harness reading that pipe to EOF hangs.
"""
from __future__ import annotations

from ethne import _core

#: Whether this build linked autonomous_trust. `False` in the ordinary wheel, where the
#: replayed-feed provider is fully available and the live constructors do not exist.
AT_FFI: bool = _core.at_ffi_available()

at_ffi_available = _core.at_ffi_available

AtProvider = _core.AtProvider
SubstrateChange = _core.SubstrateChange
FoundingCandidate = _core.FoundingCandidate

#: The `did:key` for a raw 32-byte Ed25519 **public** key — the one mapping between an
#: autonomous_trust identity and its ethne/kith-covenant DID. A peer's AT signing key
#: *is* its member identity, and `did:key` embeds the key, so no registry is involved.
did_from_pubkey = _core.did_from_ed25519_pubkey

#: The movement kinds `AtProvider.changes` reports (**D16**). A change's `kind` is one of
#: these, and its payload getters are `None` where they do not apply.
SUBSTRATE_CHANGES = (
    "MemberAppeared",
    "MemberVanished",
    "ReputationChanged",
    "GuardianEdgeChanged",
    "AttendanceStamped",
    "BoundaryOpacityChanged",
)

# --- the at-ffi half, bound only when the feature compiled it in -----------------
#
# Read through these names rather than reaching for the attributes directly, so a module
# that merely imports `ethne.at` keeps importing in either build.

#: `AtProvider.attached` in an at-ffi build, else `None`.
ATTACHED = getattr(AtProvider, "attached", None)
#: `AtProvider.attached_existing` in an at-ffi build, else `None`.
ATTACHED_EXISTING = getattr(AtProvider, "attached_existing", None)
#: `AtProvider.embedded` in an at-ffi build, else `None`.
EMBEDDED = getattr(AtProvider, "embedded", None)

#: AT's log levels by name, for `AtProvider.embedded`. Empty without `at-ffi`, since
#: there is nothing to configure the logging of.
LOG_LEVELS = (
    {"Debug": 1, "Info": 2, "Warning": 3, "Error": 4, "Critical": 5} if AT_FFI else {}
)

__all__ = [
    "AT_FFI",
    "ATTACHED",
    "ATTACHED_EXISTING",
    "EMBEDDED",
    "LOG_LEVELS",
    "SUBSTRATE_CHANGES",
    "AtProvider",
    "FoundingCandidate",
    "SubstrateChange",
    "at_ffi_available",
    "did_from_pubkey",
]
