<!--
  Copyright 2026 Sean M. Brennan and contributors
  Licensed under the Apache License, Version 2.0.
-->

# The whistleblower source: anonymity and threshold custody

§8 of [protocol-components.md](protocol-components.md) asks for two things the v1
observer slice did not build:

> **Threshold-held source records**: whistleblower identity recoverable only with
> mediator-panel quorum and only for named cause. Source protection must be
> structurally stronger than individual mediator integrity.

and, in the constitutional note on the jester function, the standing that makes this
non-optional: "its implementation cannot be deferred to 'after MVP.'"

This document records what was built, why the pieces are shaped the way they are, and
what the construction does *not* achieve.

## What the v1 design actually protected

v1 stored a `source_commitment` — a hash the reception panel held the preimage of
out-of-band — and gated any reveal behind a panel quorum citing cause. The intended
weakness was correlation: a determined analyst comparing panels, timing, and
references could narrow the source set.

The real weakness was larger, in two directions.

**Nothing read the commitment.** Outside the record definition, `source_commitment`
appeared in three test constructors and one binding setter, and in no rule anywhere.
`authorize_source_reveal` checked that the reference resolved to a whistleblower
report, that `cause` was non-empty, and that the panel was a non-party mediator
quorum — and never hashed `disclosed_source`. A panel could therefore append **any
name at all** as the source of a filing and both integrity passes would pass it. The
commitment was decorative. (The ethne tier found the same shape in its
`substrate.anchor`: a field that looks load-bearing because it is genesis-immutable,
and is read by nothing.)

**Nothing bound a source to a filing.** The signers of a `WhistleblowerFiled` entry
are the reception panel. The source signed nothing, so the archive held no evidence
that an insider existed. A panel of non-party mediators could manufacture insider
complaints against a rival community — a filing with `source_commitment =
Cid::of_bytes(b"anything")` verified and authorized cleanly.

Both were encoded in the test suite as correct behaviour. Two green tests —
`whistleblower_non_party_panel_authorized` and
`source_reveal_requires_cause_and_valid_panel` — asserted exactly these outcomes. The
measurement of the hole did not need new probes; it needed reading what the existing
suite was asserting.

## The construction

### The insider ring (`kc-core/src/ring.rs`)

A source proves it is an insider of the accused community with a **linkable ring
signature** (LSAG) over that community's keys. `RingSigned<T>` is deliberately shaped
like `Signed<T>`: payload, the CID of its canonical-CBOR encoding, and a signature
that is checked only after the CID is confirmed. What changes is the signature — a
ring of Schnorr challenge/response pairs that closes on itself, plus a key image.

The ring is the accused community's **resolvable key set** at `filed_at`: its root key
plus every live speaker delegation. The authority pass computes that set itself and
requires the offered ring to equal it exactly — a narrowed ring (a smaller suspect
set) and a padded ring (a foreign key) are both refused. `kc_archive::insider_keys` is
public precisely so a source assembles its filing against the same definition the rule
will check it against; two implementations of "the insider set" would refuse every
honest filing.

The challenge chain binds the payload CID, the whole ring **in order**, and the key
image. So a signature cannot be lifted onto another payload, replayed against a
different ring, or paired with a different key image.

A ring of one is refused rather than produced. A ring of one names its signer; building
one would hand a source a signed confession while telling them they were anonymous.

**Ed25519, not a second keypair.** An Ed25519 private key is a seed, and the scalar is
the clamped first half of its SHA-512 — which `ed25519_dalek::hazmat::ExpandedSecretKey`
derives. Recovering that scalar is what lets a ring signature and an ordinary signature
share one identity key, so an anonymous filing needs no key-distribution ceremony and
the compact stays Ed25519-only.

**The second generator.** Linkability needs a generator `H` whose discrete log with
respect to the basepoint `B` is unknown. This is not a formality: if `H = h·B` for a
known `h`, anyone computes `h·P = h·x·B = x·H` for every ring member's public key and
reads off which one matches the published image — the signature would be transparent
rather than anonymous. `h_point` derives `H` by try-and-increment over a
domain-separated Blake3 hash of a fixed ASCII string, then clears torsion.

### The key image, and what it is for

`I = x·H` is determined by the signer's key alone and reveals nothing about which ring
member holds it. It buys three things:

- **An anonymous source tag.** `WhistleblowerSummary.source_tag` publishes it, and the
  observer dashboard's `distinct_sources_by_accused` counts distinct images per accused
  community. Ten filings from ten insiders and ten filings from one are different
  situations; without this they are the same number. The second may be a channel being
  flooded to drown a real report.
- **Determinism, which is exculpation.** See below.
- Nothing else. It is not a reputation handle and carries no standing.

### Threshold custody (`kc-core/src/custody.rs`)

The source identity is held under **Feldman verifiable secret sharing**. A random
scalar is split by a degree `k-1` polynomial; the coefficients are committed to as
curve points on the public record; the identity (as a nonce-bound `SourcePreimage`) is
encrypted under a key derived from the secret; and each share is sealed to one
panelist. Recovery is Lagrange interpolation from any k shares.

Feldman rather than plain Shamir, for one reason: **the sharing is checkable at the
moment it is published.** Each share verifies against the public commitments on its
own, so a dealer who hands out inconsistent shares is caught at filing time, and a
single honest holder can demonstrate their own share is bad. Under plain Shamir the
fraud surfaces only when a recovery is attempted — possibly years later, at the moment
the record matters most.

Shares are sealed **on the record**, so custody is auditable: anyone can confirm the n
shares went to exactly the n panelists and nobody else, without opening any of them.
The authority pass requires the holder set to equal the reception panel exactly.

The threshold must be a **majority** of the custodians. An earlier draft required
`k ≥ panel_min`, which testing showed to be wrong: for a panel of exactly `panel_min`
that is unanimity, so one unavailable panelist would make the source permanently
unnameable. A majority is the precise reading of "structurally stronger than individual
mediator integrity" — no minority of the panel can reconstruct the source, whatever it
agrees among itself. The separate rule that a reveal be co-signed by `panel_min`
non-party mediators still applies, so both gates hold at once.

### The reveal, which now has to open something

`authorize_source_reveal` still checks the reference, the cause, and the panel. It then
recovers the preimage from the offered shares, verifying each against the filing's
published commitments, and requires:

- the recovered preimage hashes to the filing's `source_commitment` — the rule that
  finally reads that field;
- `disclosed_source` equals the recovered identity;
- that identity is one of the accused community's resolvable insiders.

Naming somebody therefore requires the shares, and the somebody can only be a member of
the ring the source proved membership in. Framing is bounded to the ring.

### Exculpation, the one remedy without a panel

Threshold custody bounds who can be named; it does not stop a colluding quorum from
naming the wrong ring member. Because key images are deterministic, the named member
computes their own, publishes it with a Chaum–Pedersen proof that the same secret
satisfies both `P = x·B` and `I = x·H`, and anyone can see it differs from the image on
the filing. The member clears themselves without surrendering their key.

`SourceExculpated` needs **no panel, no quorum, no permission** — deliberately. The
remedy for being wrongly named must not run through the body that named you. What the
archive checks is only that the claim is honest arithmetic: the claimant is in the
filing's ring, the proof verifies against their resolvable key, and the claimed image
differs from the filing's. An "exculpation" whose image *matches* the filing is a
confession, and is refused rather than recorded as a denial.

## What this does not achieve

Stated plainly, because each is a real limit rather than a rough edge.

**The crypto is hand-rolled and unaudited.** The ring signature, the Feldman sharing,
the key agreement, and the Chaum–Pedersen proof are implemented here, over
`curve25519-dalek` and `chacha20poly1305`. No published ring-signature crate operates
on Ed25519 identity keys: the candidates are on secp256k1 (wrong curve), on Ristretto
(which forces a second keypair per member), or wrapped in a Monero-transaction API.
Trusting an unaudited low-adoption crate is not better than code that can be read, and
working on the keys the compact already has is what makes an anonymous filing possible
at all. But the code has had no external review. It is covered by sign/verify tests
across every ring position, tamper and reorder rejections, key-image determinism and
distinctness, all k-subsets of a small sharing, and the refusals listed above — which
is evidence of correctness against known failures, not a proof against unknown ones.

**The insider set is speakers, not members.** kith-covenant models communities and
their delegated speakers; it has no individual-member roster. So the ring is the
community's root key plus its live delegations, and an insider with no delegation
cannot file. A community with a single key has no anonymous channel at all (it can open
one by delegating a speaker, which is cheap). The named future feed is a member roll
from the polity tier, which does model membership.

**Key images link a source across all its filings.** The tag is global, not per-ring, so
one insider filing against two different communities is linkable across those filings.
This is the cost of flood detection, and it is a real disclosure — a pseudonym, not an
identity, but a durable one.

**Custody sealing reuses the signing key.** Key agreement runs against a panelist's
Ed25519 identity key, mirroring what libsodium exposes as
`crypto_sign_ed25519_pk_to_curve25519`, with domain-separated derivation. Using one key
for two primitives is a compromise taken to keep the Ed25519-only model and avoid a
distribution ceremony. The named successor is a separate encryption key carried in the
mediator registry (a §4 change).

**A quorum can still frame a ring member.** Exculpation makes framing answerable, not
impossible. And it is answerable only by a member who is *present to answer*: a source
who has since died or left cannot be defended by this mechanism, and neither can a
member who does not learn of the reveal.

**Transport is out of scope.** How a source reaches a reception panel without the act of
contacting being observable is item 8.3, and it is deployment-gated: operational
infrastructure, not a record shape.

## Where the code is

| Piece | Location |
|---|---|
| Ring signature, key image, DLEq proof | `src/rust/kc-core/src/ring.rs` |
| Feldman VSS, sealed shares, preimage | `src/rust/kc-core/src/custody.rs` |
| `InsiderClaim`, `SourceExculpation`, record shapes | `src/rust/kc-observer/src/record.rs` |
| `insider_keys` + the three authority rules | `src/rust/kc-archive/src/authority.rs` |
| `source_tag`, `exculpations`, `distinct_sources_by_accused` | `src/rust/kc-reputation/` |
| Python surface | `src/python/kith_covenant/authoring.py` |
| Worked example incl. every refusal | `src/python/examples/observer_demo.py` |
