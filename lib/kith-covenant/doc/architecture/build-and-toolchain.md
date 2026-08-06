<!--
  Copyright 2026 Sean M. Brennan and contributors
  Licensed under the Apache License, Version 2.0.
-->

# Build, toolchain, and verification

**Reference document.** How this workspace is built, which toolchain choices are
deliberate and why, what breaks when they are ignored, and the verification
discipline the slices are held to. For the protocol itself see
[protocol-components.md](protocol-components.md); for outstanding scope see
[`TODO.md`](../../TODO.md).

Kith-covenant and the [ethne](../../../ethne/doc/architecture/build-and-toolchain.md)
polity tier are separate workspaces that bind the same Python package family and
share `kc-core`, so the toolchain decisions below were taken for both at once and
should stay in step. Where ethne's copy of this document differs, the difference is
noted.

Several entries exist because the obvious thing was tried and was silently wrong.
That is the useful part, so it is kept.

## Build environment

The supported build is the per-component conda env: `environment.yaml` pins the
interpreter, `rust`, `maturin` and `pytest` from conda-forge, and `run-tests.sh`
errors early naming whatever is missing.

Two facts are not optional:

- **`CARGO_TARGET_DIR` must be off virtiofs.** On the repo mount cargo fails
  outright with `Bad address (os error 14)`, not merely slowly. Point it at
  scratch.
- **Run `maturin develop` from the repository root before pytest.** From a crate
  directory it builds a differently named module and leaves the existing
  `_core.abi3.so` in place, so pytest then runs green against a stale extension.
  The failure mode is a *bogus pass*, which is worse than a missing module: one
  slice was verified against a three-day-old build this way.

`cargo clippy` is not installed in the development sandbox and cannot be installed
there (apt cargo/rustc, no rustup). It is therefore reported as **not run**, never
as clean; Sean re-runs it. Every acceptance claim that depends on clippy is stated
as unverifiable here rather than assumed.

## The toolchain floor

`rust-version = "1.85"` and `resolver = "3"`. Both numbers came out of correcting a
declaration that did nothing, in two steps, where the second was only visible once
the first landed. The sequence matters because the same defect is easy to
reintroduce.

**Step 1: make it apply.** `[workspace.package]` declared `rust-version = "1.75"`
while all fourteen member crates inherited only `edition.workspace = true`, so
**nothing** checked the claim, not cargo and not clippy. It surfaced as clippy
recommending `Option::is_none_or` (Rust 1.82) at thirteen sites. All fourteen crates
now carry `rust-version.workspace = true`.

**Step 2: make it true.** With the constraint live, reading the resolved graph for
the first time showed 1.75 could never have built:

| Crate | Declares | Reached via |
|---|---|---|
| `constant_time_eq` 0.4.2 | **1.85** | `blake3`, which every `Cid` goes through |
| `cpufeatures` 0.3.0 | 1.85 | both workspaces |
| `zeroize` 1.9.0 | 1.85 | ethne |
| `ed25519-dalek` 2.2.0 | 1.81 | both workspaces |

The real floor was already 1.85, ten minor versions above the declared 1.75, and
`blake3` is not optional. Both workspaces now declare 1.85 with the reason in the
manifest.

**Two decisions had rested on the false number and are void.** `pyo3` was held at
0.22 partly on MSRV grounds, and 0.29 would have been fine all along. And
`chacha20poly1305` was pinned at 0.10 with the comment "0.11 raises the MSRV above
this workspace's rust-version", which was simply untrue: 0.11.0 declares exactly
1.85. The comment now states the real reason to hold, which is that 0.11 brings a
new `aead` major and touches the sealed-share path.

**A correct floor is only a snapshot**, so `resolver = "3"` (the MSRV-aware
resolver, stable since Cargo 1.84) keeps the next `cargo update` from raising the
real floor silently. Neither `Cargo.lock` changed when it landed, today's graph
already fitting under 1.85, so it was demonstrated rather than assumed. With the
floor temporarily set to 1.78:

| Setting | `cargo update --dry-run` |
|---|---|
| `resolver = "3"` | **downgrades** `base64ct` 1.8.3 to 1.6.0, `ed25519-dalek` 2.2.0 to 2.1.1, `half` 2.7.1 to 2.4.1, `proptest` 1.11.0 to 1.8.0, naming each one's required Rust |
| `resolver = "2"` | **upgrades** `wasip2` to a version "requires Rust 1.87.0", notes it, and proceeds |

The second row is the failure mode this closes. Both floors were restored
afterward and `Cargo.lock` verified unchanged by the experiment.

**When a clippy suggestion names a newer API, check `rust-version` first, and check
that the number means anything.**

## Lint policy

`pyo3` is at **0.27**, migrated from 0.22 the day the warnings were raised. Sean's
clippy run reported **71 `useless_conversion` warnings in `kc-pyo3`** and 73 in
`en-pyo3`, one per exported function, every span on a `PyResult<...>` return type,
with no `.into()` in any body of ours.

**The premise was verified at source level before starting**, because clippy cannot
run in the sandbox and the changelog never mentions this fix, so searching it would
have been inconclusive. `pyo3-macros-backend/src/quotes.rs::ok_wrap` emits the
identity `map_err(Into::<PyErr>::into)` in **both** versions, spanned at
`*output_span`, which is the function's return type and exactly where the warnings
pointed. 0.22.6 has no suppression there; 0.27 adds
`#[allow(clippy::useless_conversion)]`. So the class disappears by upgrade rather
than by our own suppression, and `pyo3-macros-backend v0.27.2` was confirmed in the
compiled graph, since the crate carrying the allow is the one that has to have
built these.

The migration surface was measured before anything was touched and was small: two
`PyBytes::new_bound` here, four equivalents in ethne, **six lines across both
tiers**. Neither crate used `IntoPy`/`ToPyObject`, which is 0.23's real break. What
made this its own slice rather than a version bump is the deprecation of the
`*_bound` constructors, and there were only six.

**0.29 is unblocked** by the 1.85 floor and buys only currency: 0.27 already removed
the warning class.

The residue is **2 `type_complexity` warnings** in `kc-pyo3` on tuple-heavy returns.
No mechanical fix; recorded rather than chased.

**The thirteen `Option::is_none_or` sites** were declined once on MSRV grounds and
then permitted once the floor was corrected. A whole-tree sweep found **exactly
thirteen** `.map_or(true, ...)` occurrences, matching clippy's count, which is what
made a mechanical replacement safe: a `Result::map_or` among them would have failed
to compile, `is_none_or` being `Option`-only. `authority.rs` reads better for it,
`reversal_conditions.as_ref().is_none_or(|s| s.trim().is_empty())` now saying
"absent or blank" in the words of the rule.

## Feature gating across the two workspaces

`kc-core`'s `ring` and `custody` modules sit behind a default-on
**`source-protection`** feature so the ethne tier stops linking §8 crypto it never
calls. The rationale, the measured effect (ethne's `en-core` closure dropping from
61 crates to 53), and the honest limit (`curve25519-dalek` stays, reached through
`ed25519-dalek`, which `kc-core` needs unconditionally) are in
[whistleblower-crypto.md](whistleblower-crypto.md).

The build mechanics belong here, because they generalize past this one feature:
**`default-features = false` written on a member crate is silently ignored** when
the dependency is inherited with `workspace = true` and the `workspace.dependencies`
entry does not itself specify it. Cargo emits a warning saying this "could become a
hard error in the future" and then builds anyway. All five of the first-pass
opt-outs were inert and the measured crate count did not move. The flag belongs on
`workspace.dependencies` in **both** root manifests, with the crates that do need
the feature opting back in.

**Feature unification is the constraint to reason about**, not the gate: one crate
anywhere in the graph enabling a default restores it for the whole graph, which is
why `kc-conduct` and `kc-identity` opt out on the KC side even though the consumer
that needs them off is in the other workspace.

## What the integrity passes cannot see

`kc-archive` does not compact. `dump_json` / `load_json` write and read the **whole**
log, there is no incremental append, no generation counter, and nothing prunes. This
is worth stating rather than leaving implicit, because it is the only reason KC does
not have a defect the ethne tier measured as D20, and §8's whistleblower-source
records are the obvious pressure toward adding pruning.

In ethne, an incremental save and a compaction that were each correct in isolation
did not **compose**. A host that pruned resurrected seven dropped observations on
reload, lost the pruning record entirely, and the reloaded log still passed *both*
integrity passes, the receipt re-verifying precisely because every CID it named was
back. A measured absence lengthened from 230 days to 300: a storage bug
manufacturing a governance signal. The fix was archive generations plus
`(generation, index)` cursors, with one branch owned by the store (*same generation
implies append; changed implies rewrite in full*) and the cursor held by the store
rather than the caller, because a caller keeping its own cursor is the hazard
itself.

**The transferable lesson is a verification one: both of KC's integrity passes are
blind to this class of bug by construction.** They check that the entries present
are well-formed and authorized, never that the entries *absent* were meant to be. A
KC pruning slice therefore needs a round-trip-across-a-compaction test, not more
authority rules.

## Test hazards

**A refused record still moves the derived state a later record reads.**
`current_incapable_keys`, `current_rung` and `current_sanction_level` do not consult
authority, so a negative-control fixture must run its control first or on a target
of its own. This bit twice while writing one slice's tests.

**The order of the checks decides which refusal an operator sees**, and it decides
which rule a test actually exercises. Two first-cut fixtures tripped an earlier rule
than the one under test: a `panel_min` that masked the composition rule, and a
rotation gap that masked the continuous-service union. A test that fires on the
wrong refusal passes for the wrong reason.

**Print each refusal's message as a matter of course.** One "holder with no
registered encryption key" case fired on the reception-panel rule instead, because a
holder who is not a registered mediator is normally caught earlier as a *signer*.
Reaching the intended rule needed a panel naming somebody who never signs. Nothing
but the message text would have shown that.

**`bad()` is defined per test module**, so patching the wrong one silently produces
no output at all.

## Verification practice

Every slice runs negative controls and reports them, including the ones that did not
fire. The standing form is **control 10: delete each new rule in turn and confirm
its test flips to failing**, so no rule ships with a test that cannot fail. Files
mutated for a control are restored bit-identically and the restoration is verified
with `diff -q`.

Four rules, each learned by getting it wrong:

**A control that mutates more than the rule proves less.** A first, sloppier version
of one control deleted a quorum check outright and broke four unrelated tests. It
was redone to isolate exactly the rule under test.

**The order of the checks in the control script is load-bearing.** `cargo test`
prints `error: test failed, to rerun ...` when a *test* fails, so a build-error
check placed first matches every correctly flipped control and reports all of them
as build failures. Nine controls were briefly misread this way.

**A control that fails nothing is a result, not an omission.** Mutating away the
self-CID exclusion broke no test, because it is shadowed by the subject exclusion
added in the same slice. It was kept as the helper's contract with the shadowing
stated in the code, rather than shipped as an untestable rule or deleted as dead.

**Measure before designing, and let the measurement become the regression test.**
Repeatedly the measured hole was larger than the item's wording: the custody item
was filed as key-reuse hygiene and was a live defect that authorized sham custody
with zero custodians able to open a share; the member-roll item was filed as a
missing feed and the larger half was that the insider set was read from the archive
tip, so one backdated record invalidated any filing against the accused.

**A status list checked against other status lists reproduces their errors.**
The §11 implementation backlog was reconciled once against the per-component
"v1 to v4" notes and read as done. Checked against the **code** instead, one row
reading "credentialing-panel rotation, sponsor-stake tracking, and
probationary-status enforcement" turned out to be three items wearing one
bullet: the first did not exist, the second was built, and the third was
computed and enforced by nothing. It had read as done because a third of it was.

**A recurring shape worth naming:** a value that no rule reads. It has appeared four
times across the two tiers, as ethne's `substrate.anchor`, KC's `source_commitment`,
the fictional MSRV floor above, and a sealed share's recipient key, which the record
did not even carry. When a field is added, check that something refuses on it.

## Working conventions

- **Never `git checkout -- <file>`** to undo a temporary patch. Both this tier and
  ethne are wholly uncommitted, so the command discards real work rather than a
  mutation. Copy the file to scratch first and restore from there.
- **Never `git commit`.** Sean commits.
- **`TODO.md` and `.claude/` are untracked by convention.** Do not add them.
- ethne depends on `kc-core`, so a change to `core`, `ring` or `custody` reaches the
  other workspace. Hold its baseline as part of verifying a KC slice.
