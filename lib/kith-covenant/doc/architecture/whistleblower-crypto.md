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

The ring is the accused community's **resolvable key set** at `filed_at`: its root key,
every live speaker delegation, and every member on any roll it has published. The
authority pass computes that set itself, over the records preceding the filing in the
log, and requires the offered ring to equal it exactly — a narrowed ring (a smaller
suspect set) and a padded ring (a foreign key) are both refused.
`kc_archive::insider_keys` is public precisely so a source assembles its filing against
the same definition the rule will check it against; two implementations of "the insider
set" would refuse every honest filing. A source calls it against the log as it stands,
which is exactly the prefix its own filing will be judged on.

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
shares went to exactly the n panelists and nobody else, without opening any of them. The
authority pass requires the holder set to equal the reception panel exactly, **and** each
share's recorded `recipient_pubkey` to be the key that holder is registered under.

That second clause is what makes the first sentence true, and it was missing. A share
named its holder and did not record the key it was sealed to, so "the shares went to
exactly the n panelists" was a claim about the *names* on the record and nothing more —
auditable in appearance, unauditable in fact. See the sealing-key limit below.

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

**The insider set now includes members, and only ever grows.** kith-covenant models
communities and their delegated speakers and has no member roster of its own, so the
ring began as the root key plus live delegations — and an insider holding no delegation
was not in it. A `MemberRollPublished` record closes that: a community publishes the
individual key-holders it names as its own, and `insider_keys` unions them into the
ring. The polity tier is the intended feed (`en-uplift::member_roll` over its charter's
membership roll), but a roll is an ordinary community-signed record and a deployment
with no polity tier can publish one by hand.

Three properties of the roll are load-bearing:

- **Rolls are unioned, never superseded.** Every other registry here supersedes by
  latest `issued_at`; a roll does not. Once named an insider, always an insider. The
  alternative would hand an accused community a lever for shrinking the suspect set
  around a member the day after they left, which is the attack the whole mechanism
  exists to prevent. The cost is that a departed member stays in the ring and can
  still file — the direction to err in.
- **Naming who belongs is root-only.** No delegated speaker, `Envoy` included, may
  publish a roll.
- **A member's carried key must be what its DID resolves to.** Otherwise a roll names
  a real member against an outsider's key, putting a non-member in the ring under a
  member's name — and the ring is what bounds a reveal.

A community that publishes neither a roll nor a delegation still has no anonymous
channel, and `observer_dashboard.anonymity_set_by_community` reports that as the 1 it
is, thinnest first. That is a real reading and not a defect: the mechanism cannot
manufacture a suspect set out of a community of one.

**The insider set is a property of the log, not of when it was read.** The ring a
filing is checked against is computed over the records appearing *before that filing*
in insertion order — `insider_keys_upto` with the filing's own CID — and reveals and
exculpations are bounded by the filing's position too. Before that bound existed the
accused community could append one delegation backdated ahead of `filed_at` and every
filing against it stopped verifying (the recomputed set no longer equalled the offered
ring), or backdate a revocation and narrow the suspect set around a source after the
fact. Both were measured. Insertion order is authoritative for the same reason it is in
`current_rung`: authorizing an earlier record must never see a later one.

**A roll is still the accused speaking about itself.** The rule checks that the
publisher is the community named and nothing more; this tier cannot audit the polity
charter a roll was derived from, which is why `as_of` is carried as provenance rather
than as authority. Union-only bounds what that self-report can do — a community can
understate its roll, and understating only narrows the channel its *own* members have,
never retroactively — but a community that never publishes gives its members no
channel, and nothing here compels one.

**Key images link a source across all its filings.** The tag is global, not per-ring, so
one insider filing against two different communities is linkable across those filings.
This is the cost of flood detection, and it is a real disclosure — a pseudonym, not an
identity, but a durable one.

**Custody is sealed to a registered encryption key, and the pairing is enforced.**
`MediatorEntry` carries an `encryption_pubkey` distinct from the mediator's signing key
— still Ed25519, so the Edwards-form agreement path and the compact's one-algorithm key
model are unchanged, but it never signs anything. The authority pass refuses a
registration whose two keys are equal, since that is the reuse the field exists to end.

The larger half of this was not the reuse. `ThresholdCustody::seal` took
`(DidKey, PubKey)` pairs and **nothing anywhere checked that the two corresponded**, so
a filing could name exactly the reception panel and seal every share to one outsider's
key. Measured on the tree beforehand: `check_shape` passed, the holder set matched the
panel, Feldman's own share check passed, the archive authorized it — and **0 of 3** named
custodians could open a share while the substituted key opened all three and recovered
the source alone. The recipient key was not recorded anywhere, so nobody but a holder who
tried could discover it. Two harms, the first needing no collusion beyond whoever
assembled the filing: the lawful reveal path was permanently dead, because a reveal needs
k opened shares and nobody could open one — a complaint that looks properly filed and can
never be pursued; and the majority-threshold rule above was defeated by a single actor.

That is precisely the late-detection failure this module rejects plain Shamir for, in the
half it had not been applied to — and worse, because the record carried nothing to detect.
So: the sealing key is now **resolved, not supplied** (`kc_archive::mediator_encryption_resolver`
for §8; `kc_core::didkey_encryption_resolver` where a holder has no separate key), it is
**recorded** on each share as `recipient_pubkey`, and the authority pass requires it to
equal what that holder is registered under at `filed_at`. Both halves are load-bearing:
resolution keeps the honest caller from getting it wrong, and the recorded key is what a
hand-built record cannot lie about undetected — a guard living only in the emitter is one
a hand-built record walks around. `file_whistleblower` no longer accepts panel public keys
at all, which is where "unrepresentable" actually bites.

Judged at `filed_at`, so a community rotating a mediator's encryption key later cannot
retroactively invalidate an honest filing — the same log-order principle the ring is
bound by.

**A quorum can still frame a ring member.** Exculpation makes framing answerable, not
impossible. And it is answerable only by a member who is *present to answer*: a source
who has since died or left cannot be defended by this mechanism, and neither can a
member who does not learn of the reveal.

**Transport is out of scope.** How a source reaches a reception panel without the act of
contacting being observable is item 8.3, and it is deployment-gated: operational
infrastructure, not a record shape.

## Who has to carry it: the `source-protection` feature

Everything above lives in `kc-core`, which is also the crate that supplies plain
identities, signatures and CIDs to consumers with no §8 function at all. The ethne
polity tier is the case: `en-charter` depends on `kc-core` and on nothing else of
kith-covenant's, by design, and was linking the ring and custody crypto for a
whistleblower channel a polity tier never opens.

`ring` and `custody` are therefore behind a **default-on** `source-protection`
feature. Default-on because every kith-covenant consumer needs them and a footgun that
silently disables source anonymity would be worse than the dependency; the two modules
gate as one unit because `custody` uses `ring`'s `Bytes32`, and nothing else in
`kc-core` does. Switching it off leaves a crate that still signs, verifies, and
addresses records — 35 of `kc-core`'s 77 tests exercise exactly that surface and pass
with the feature off.

Two things about this are worth stating, because both contradict the obvious
expectation:

- **Only `chacha20poly1305` actually leaves.** `curve25519-dalek` is the more
  conspicuous dependency, but it also arrives through `ed25519-dalek`, which `kc-core`
  needs for signatures unconditionally. Gating removes the direct edge and not the
  crate. What goes is `chacha20poly1305` and the seven crates reached only through it
  (`aead`, `chacha20`, `cipher`, `inout`, `opaque-debug`, `poly1305`,
  `universal-hash`): ethne's `en-core` normal-dependency closure drops from 61 crates
  to 53.
- **The opt-out belongs on the workspace entry, not the member crate.** Cargo
  **ignores** `default-features = false` on a dependency inherited with
  `workspace = true` unless the `workspace.dependencies` entry specifies it too. It
  warns, and says this may become a hard error, but it builds — so an opt-out written
  in the member manifest looks right and does nothing. Both root manifests carry the
  flag; the four crates that need the feature (`kc-archive`, `kc-observer`, `kc-pyo3`,
  `kc-reputation`) opt back in by name.

The second point is also why `kc-conduct` and `kc-identity` opt out on the *KC* side.
Neither uses ring or custody, but ethne reaches both through `en-uplift`, and feature
unification means one crate enabling the default would restore the crypto to ethne's
entire graph. An opt-out is only as good as the least careful crate on the path.

## Where the code is

| Piece | Location |
|---|---|
| The `source-protection` feature gate | `src/rust/kc-core/Cargo.toml`, both root `Cargo.toml` files |
| Ring signature, key image, DLEq proof | `src/rust/kc-core/src/ring.rs` |
| Feldman VSS, sealed shares, preimage, the sealing-key resolvers | `src/rust/kc-core/src/custody.rs` |
| `InsiderClaim`, `SourceExculpation`, record shapes | `src/rust/kc-observer/src/record.rs` |
| `MemberRoll`, `RollMember` | `src/rust/kc-identity/src/roll.rs` |
| `MediatorEntry.encryption_pubkey` | `src/rust/kc-mediation/src/record.rs` |
| `insider_keys` / `insider_keys_upto`, `mediator_encryption_keys` + the authority rules | `src/rust/kc-archive/src/authority.rs` |
| `source_tag`, `exculpations`, `distinct_sources_by_accused`, `anonymity_set_by_community` | `src/rust/kc-reputation/` |
| The polity-tier feed (border artifact #6) | `ethne/src/rust/en-uplift/src/lib.rs` |
| Python surface | `src/python/kith_covenant/authoring.py` |
| Worked example incl. every refusal | `src/python/examples/observer_demo.py` |
