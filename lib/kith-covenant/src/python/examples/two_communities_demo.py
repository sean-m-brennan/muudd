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

"""Python mirror of the Rust two_communities_demo binary.

Runs the full two-community credentialing ceremony via the PyO3 bindings,
verifies the archive, roundtrips through JSON, re-verifies, and prints a
human-readable audit trail.

Build the bindings first:
    maturin develop      # from the kith-covenant repo root
Then run:
    python src/python/examples/two_communities_demo.py
"""

from __future__ import annotations

import sys

from kith_covenant import Archive, run_two_community_ceremony


ALPHA_MOVEMENTS = [
    "We anchor above ourselves in the source we did not make.",
    "We know our members and our community are capable of harm.",
    "We recognize Beta as a community bound by its own substrate.",
    "We commit to truthfulness, restitution, and hospitality.",
    "This compact is not ultimate; something stands above it.",
]
BETA_MOVEMENTS = [
    "We stand beneath what we cannot author.",
    "We hold no illusion that any of us is without fault.",
    "We recognize Alpha as kindred in the work of dwelling together.",
    "We bind ourselves to patience, candor, and repair.",
    "Our agreements do not replace what we stand beneath.",
]


def main() -> int:
    print("=== kith-covenant two-community demo (Python) ===\n")

    result = run_two_community_ceremony(
        "Alpha", ALPHA_MOVEMENTS, "Beta", BETA_MOVEMENTS,
        compact_label="alpha-beta-compact",
    )

    print("Alpha community:")
    print(f"  DID: {result.alpha_did}")
    print(f"  identity CID: {result.alpha_identity_cid}")
    print(f"  intra-rite archive CID: {result.alpha_intra_cid}\n")

    print("Beta community:")
    print(f"  DID: {result.beta_did}")
    print(f"  identity CID: {result.beta_identity_cid}")
    print(f"  intra-rite archive CID: {result.beta_intra_cid}\n")

    print("Inter-community witness record:")
    print(f"  compact: {result.compact_label}")
    print(f"  archive CID: {result.witness_cid}")
    print(f"  total archive size: {result.archive_size} entries")
    print(f"  dumped JSON: {len(result.archive_json)} bytes\n")

    print("Reloading archive from JSON and re-verifying…")
    arch = Archive.load_json(result.archive_json)
    assert len(arch) == result.archive_size, (
        f"reload lost entries: {len(arch)} vs {result.archive_size}"
    )
    arch.verify_all()
    print(f"  reloaded archive: {len(arch)} entries, all verify OK\n")

    print("Audit trail:")
    by_kind: dict[str, list[str]] = {}
    for kind, cid in arch.entries():
        by_kind.setdefault(kind, []).append(cid)
    for kind in sorted(by_kind):
        cids = by_kind[kind]
        print(f"  {kind}: {len(cids)} entries")
        for c in cids:
            print(f"    {c}")

    print("\n✓ two_communities_demo (Python): SUCCESS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
