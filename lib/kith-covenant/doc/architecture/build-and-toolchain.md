*Previous: [The AutonomousTrust integration](autonomous-trust-integration.md)*

<!--
  Copyright 2026 Sean M. Brennan and contributors
  Licensed under the Apache License, Version 2.0.
-->

# Build, toolchain, and verification

**Reference document.** How this workspace is built, which toolchain choices
are deliberate and why, what breaks when they are ignored, and the verification
discipline the slices are held to. Build, toolchain, and verification are taken
together because a change in one reaches the others. For the protocol itself
see [protocol-components.md](protocol-components.md); for outstanding scope see
[`TODO.md`](../../TODO.md).

Kith-covenant and the
[ethne](../../../ethne/doc/architecture/build-and-toolchain.md) polity tier are
separate workspaces that bind the same Python package family and share
`kc-core`, so the toolchain decisions below were taken for both at once and
should stay in step. Where the ethne copy of this document differs, the
difference is noted.

Several entries exist because the obvious thing was tried and was silently wrong.
That is the useful part, so it is kept.

## Build environment

The supported build is the per-component conda env. `environment.yaml` pins the
interpreter, `rust`, `maturin` and `pytest` from conda-forge, and
`run-tests.sh` errors early naming whatever is missing.

Two facts are not optional:

- **`CARGO_TARGET_DIR` must be off virtiofs.** On the repo mount cargo fails
  outright with `Bad address (os error 14)`, not merely slowly. Point it at
  scratch.
- **Run `maturin develop` from the repository root before pytest.** From a
  crate directory it builds a differently named module and leaves the existing
  `_core.abi3.so` in place, so pytest then runs green against a stale
  extension. The failure mode is a *bogus pass*, which is worse than a missing
  module, since one slice was verified against a three-day-old build this way.

`cargo clippy` **is** available in the development sandbox, via `sudo apt-get
install -y rust-clippy`. The apt package and the installed `rustc` must be the
same version, both 1.85.1 here, which is also the declared floor; a mismatched
pair refuses to run. The earlier reading, that clippy needed `rustup component
add` and so could not be had from apt cargo/rustc, was wrong.

It is worth knowing what that cost, because reporting a check as unrunnable is not
neutral. Two `type_complexity` sites entered `kc-pyo3` during a slice that reported
clippy as not run, and were found only once it was run. Nothing was checking.

**Always pass `--all-targets`.** Without it clippy lints library targets only and
every `#[cfg(test)]` module goes unlinted, which on this tier is most of the
interesting code.

**A count of reported warnings is not a count of warnings.** An `#[allow]`
removes a site from the report, so a tier reads clean while holding
suppressions nobody has revisited. Five `type_complexity` sites sat behind bare
allows here and were invisible to every ordinary run, including the clean ones.
To see what is behind a decision rather than what is silent, override the
attributes.

```bash
cargo clippy --workspace --all-targets -- --force-warn clippy::type_complexity
```

That separates a lint class that was *fixed* from one that was *silenced*.
Three states hide behind one clean run: fixed, silenced, and unlinted. A clean
ordinary run cannot tell them apart, and neither can a reader of one.

## The toolchain floor

`rust-version = "1.85"` and `resolver = "3"`. Both numbers came out of
correcting a declaration that did nothing, in two steps, where the second was
only visible once the first landed. The sequence matters because the same
defect is easy to reintroduce. A floor binds only when it is declared,
inherited, and resolvable.

**Step 1. Make it apply.** `[workspace.package]` declared `rust-version =
"1.75"` while all fourteen member crates inherited only `edition.workspace =
true`, so **nothing** checked the claim, not cargo and not clippy. It surfaced
as clippy recommending `Option::is_none_or` (Rust 1.82) at thirteen sites. All
fourteen crates now carry `rust-version.workspace = true`.

**Step 2. Make it true.** With the constraint live, reading the resolved graph
for the first time showed 1.75 could never have built.

| Crate | Declares | Reached via |
|---|---|---|
| `constant_time_eq` 0.4.2 | **1.85** | `blake3`, which every `Cid` goes through |
| `cpufeatures` 0.3.0 | 1.85 | both workspaces |
| `zeroize` 1.9.0 | 1.85 | ethne |
| `ed25519-dalek` 2.2.0 | 1.81 | both workspaces |

The real floor was already 1.85, ten minor versions above the declared 1.75, and
`blake3` is not optional. Both workspaces now declare 1.85 with the reason in the
manifest.

**Two decisions had rested on the false number and are void.** `pyo3` was held
at 0.22 partly on MSRV grounds, and 0.29 would have been fine all along. And
`chacha20poly1305` was pinned at 0.10 with the comment "0.11 raises the MSRV
above this workspace's rust-version", which was simply untrue, since 0.11.0
declares exactly 1.85. The comment now states the real reason to hold, which is
that 0.11 brings a new `aead` major and touches the sealed-share path.

**A correct floor is only a snapshot**, so `resolver = "3"` (the MSRV-aware
resolver, stable since Cargo 1.84) keeps the next `cargo update` from raising
the real floor silently. Neither `Cargo.lock` changed when it landed, the graph
today already fitting under 1.85, so it was demonstrated rather than assumed.
With the floor temporarily set to 1.78, the two resolvers part company.

| Setting | `cargo update --dry-run` |
|---|---|
| `resolver = "3"` | **downgrades** `base64ct` 1.8.3 to 1.6.0, `ed25519-dalek` 2.2.0 to 2.1.1, `half` 2.7.1 to 2.4.1, `proptest` 1.11.0 to 1.8.0, naming each one's required Rust |
| `resolver = "2"` | **upgrades** `wasip2` to a version "requires Rust 1.87.0", notes it, and proceeds |

The second row is the failure mode this closes. Both floors were restored
afterward and `Cargo.lock` verified unchanged by the experiment.

**When a clippy suggestion names a newer API, check `rust-version` first, and check
that the number means anything.**

## Lint policy

`pyo3` is at **0.29**. The 0.22 to 0.27 migration happened the day the warnings
were raised, when a clippy run by Sean reported **71 `useless_conversion`
warnings in `kc-pyo3`** and 73 in `en-pyo3`, one per exported function, every
span on a `PyResult<...>` return type, with no `.into()` in any body of ours.

**The premise was verified at source level before starting**, because clippy
was at that point believed unrunnable here and the changelog never mentions
this fix, so searching it would have been inconclusive.
`pyo3-macros-backend/src/quotes.rs::ok_wrap` emits the identity
`map_err(Into::<PyErr>::into)` in **both** versions, spanned at `*output_span`,
which is the return type of the function and exactly where the warnings
pointed. 0.22.6 has no suppression there; 0.27 adds
`#[allow(clippy::useless_conversion)]`. So the class disappears by upgrade
rather than by our own suppression, and `pyo3-macros-backend v0.27.2` was
confirmed in the compiled graph, since the crate carrying the allow is the one
that has to have built these.

The migration surface was measured before anything was touched and was small,
being two `PyBytes::new_bound` here, four equivalents in ethne, **six lines
across both tiers**. Neither crate used `IntoPy`/`ToPyObject`, which is the
real break in 0.23. What made this its own slice rather than a version bump is
the deprecation of the `*_bound` constructors, and there were only six.

**0.27 to 0.29 was predicted to "buy only currency" and did not.** The predicted
risk, 0.28 reworking `#[pymodule]` onto PEP 489 multi-phase initialization, was the
non-event its own migration guide says it is. Two other things were real.

*The deprecation, which the compiler does say.* 0.28 makes the automatic
`FromPyObject` for `#[pyclass]` types that implement `Clone` opt-in, and every
such class warns, thirty-eight in `kc-pyo3` and seventeen in `en-pyo3`. Per
class the choice is `from_py_object` to keep the conversion or
`skip_from_py_object` to drop it, and the deciding question is whether the
class is ever converted *from* Python. Neither tier has a setter, a `set_all`,
or a single by-value `#[pyclass]` parameter, so the automatic impl was dead in
both and all fifty-five take `skip_from_py_object`. A `&T` parameter is not
evidence against this: those go through the `PyRef` borrow path in pyo3 and
never needed `FromPyObject`. Nearly all thirty-eight are the `*Summary` and
`*Report` views, which the seam only ever hands out.

*The contract change, which nothing warns about.* Before 0.28 a module opted
**in** to free-threaded support with `gil_used = false`; from 0.28 modules
allow it by default and must opt **out**. The bump therefore hands a module a
free-threaded claim silently, with no warning marking it. Both `_core` modules
now declare `#[pymodule(gil_used = true)]`, stating what they required before
the bump rather than judging their thread-safety, because it cannot be tested
here, where the sandbox interpreter is a GIL build (`Py_GIL_DISABLED=0`) and
never loads a module free-threaded. Lifting the declaration needs a
free-threaded interpreter.

**There is no `type_complexity` residue.** The two warnings once recorded here as
unchased were an undercount twice over. The full run found six, and overriding the
allows found five more that no ordinary run could report. All eleven are fixed by
naming the concept the tuple stands for, so `--force-warn clippy::type_complexity`
reports zero. Three of those names (`PoolSeat`, `ObserverFixture`,
`WhistleblowerFixture`) replaced a bare `#[allow]` on a fixture sitting beside
other fixtures that already had names, which is how a file ends up with both
idioms and no stated preference.

**The arity survivors are stated rather than folded into a clean number.**
`--force-warn clippy::too_many_arguments` reports forty-two sites, overwhelmingly
the `kc-pyo3` seam, where the signature *is* the Python surface. The one library
site that was not a seam signature, `check_roster_caps`, took nine parameters of
which the last three were the same triple all three caps types carry; it now takes
`RosterLimits` and each caps type projects into it. The projection is one-way on
purpose, since the three caps types are deliberately distinct so that a caller
swapping two of them fails to compile.

**The thirteen `Option::is_none_or` sites** were declined once on MSRV grounds
and then permitted once the floor was corrected. A whole-tree sweep found
**exactly thirteen** `.map_or(true,...)` occurrences, matching the count from
clippy, which is what made a mechanical replacement safe: a `Result::map_or`
among them would have failed to compile, `is_none_or` being `Option`-only.
`authority.rs` reads better for it,
`reversal_conditions.as_ref().is_none_or(|s| s.trim().is_empty())` now saying
"absent or blank" in the words of the rule.

## Feature gating across the two workspaces

`kc-core`'s `ring` and `custody` modules sit behind a default-on
**`source-protection`** feature so the ethne tier stops linking §8 crypto it
never calls. The rationale, the measured effect (the `en-core` closure in ethne
dropping from 61 crates to 53), and the honest limit (`curve25519-dalek` stays,
reached through `ed25519-dalek`, which `kc-core` needs unconditionally) are in
[whistleblower-crypto.md](whistleblower-crypto.md).

The build mechanics belong here, because they generalize past this one feature:
**`default-features = false` written on a member crate is silently ignored**
when the dependency is inherited with `workspace = true` and the
`workspace.dependencies` entry does not itself specify it. Cargo emits a
warning saying this "could become a hard error in the future" and then builds
anyway. All five of the first-pass opt-outs were inert and the measured crate
count did not move. The flag belongs on `workspace.dependencies` in **both**
root manifests, with the crates that do need the feature opting back in.
Inheritance, unification, and placement each decide whether an opt-out means
anything.

**Feature unification is the constraint to reason about**, not the gate: one crate
anywhere in the graph enabling a default restores it for the whole graph, which is
why `kc-conduct` and `kc-identity` opt out on the KC side even though the consumer
that needs them off is in the other workspace.

## What the integrity passes cannot see

`kc-archive` does not compact. Since ethne's D32 the archive is a newtype over
`kc-log`, so the generation counter, the generation-safe cursors, the
incremental segments and `iter_from` all exist underneath it, and the dump
envelope records the generation. None of it is **forwarded onto
`InMemoryArchive`**: `dump_json` / `load_json` write and read the whole log,
every append goes to the tail, and nothing prunes. That is the deliberate
choice, not an absence, and it is the only reason KC does not have a defect the
ethne tier measured as D20. The §8 whistleblower-source records are the obvious
pressure toward adding pruning, and the machinery to do it safely is already in
the layer below.

Two facts about the archive's own surface belong beside that, because neither has
another home and both were found the same way.

**The dump envelope is at version 2.** `dump_json` writes the `kc-log` envelope,
which carries `generation` and `start` alongside `version` and `entries`. The two
unlinked `version: 1` literals this crate used to hold, one in the writer and one
in the reader, are gone, and both sides read `kc_log::DUMP_VERSION`. Version 1
dumps still load, asserted in `kc-log`'s `a_version_1_envelope_still_loads` and in
`test_bindings.py::test_a_version_1_dump_still_loads`. A version 2 dump will not
load in a build from before the move, which is the direction that matters.

**`resolve_did` used to resolve registry keys in hash order.** Found by ethne's
D32 while measuring the archive fork. The four registry steps of
`InMemoryArchive::resolve_did` (witness, mediator, examiner, observer) scanned the
CID-keyed entry index and took the first match, so where two rosters named one DID
with different keys, which is representable because only `CommunityIdentity` binds
a DID to its pubkey, a **retired key still resolved and still verified**, about half
the time. `community_identity`, documented "the most recent", read the same index in
descending CID order and returned the earlier of two publications 104 times in 200.
Both now read the log in append order, and the registry steps share
`store::latest_rosters` with `authority::mediator_encryption_keys`, which had been
the only function in the crate applying latest-issued-wins although the module doc
claimed the archive did it generally. **No existing test broke**: nothing in the
suite had ever appended two rosters for one community, which is why the defect
survived. Four tests in `tests::append_order_resolution` cover it now. It is the
same cover as the argument-record findings, the absence of a test rather than a
wrong test, and it is the reason a slice that adds a rule is expected to say which
test would have caught its absence.

In ethne, an incremental save and a compaction that were each correct in
isolation did not **compose**. A host that pruned resurrected seven dropped
observations on reload, lost the pruning record entirely, and the reloaded log
still passed *both* integrity passes, the receipt re-verifying precisely
because every CID it named was back. A measured absence lengthened from 230
days to 300, a storage bug manufacturing a governance signal. The fix was
archive generations plus `(generation, index)` cursors, with one branch owned
by the store (*same generation implies append; changed implies rewrite in
full*) and the cursor held by the store rather than the caller, because a
caller keeping its own cursor is the hazard itself.

**The transferable lesson is a verification one. Both integrity passes on this
tier are blind to this class of bug by construction.** They check that the
entries present are well-formed and authorized, never that the entries *absent*
were meant to be. A KC pruning slice therefore needs a
round-trip-across-a-compaction test, not more authority rules.

## Test hazards

**A refused record still moves the derived state a later record reads.**
`current_incapable_keys`, `current_rung` and `current_sanction_level` do not
consult authority, so a negative-control fixture must run its control first or
on a target of its own. This bit twice while writing the tests for one slice.

**The order of the checks decides which refusal an operator sees**, and it
decides which rule a test actually exercises. Two first-cut fixtures tripped an
earlier rule than the one under test, one a `panel_min` that masked the
composition rule and one a rotation gap that masked the continuous-service
union. A test that fires on the wrong refusal passes for the wrong reason.

**Print the message of each refusal as a matter of course.** One "holder with
no registered encryption key" case fired on the reception-panel rule instead,
because a holder who is not a registered mediator is normally caught earlier as
a *signer*. Reaching the intended rule needed a panel naming somebody who never
signs. Nothing but the message text would have shown that.

**`bad()` is defined per test module**, so patching the wrong one silently produces
no output at all.

## Verification practice

Every slice runs negative controls and reports them, including the ones that
did not fire. A control has to be isolated, ordered, and reported before it is
worth anything. The standing form is **control 10, deleting each new rule in
turn and confirming that its test flips to failing**, so no rule ships with a
test that cannot fail. Files mutated for a control are restored bit-identically
and the restoration is verified with `diff -q`.

Four rules, each learned by getting it wrong:

**A control that mutates more than the rule proves less.** A first, sloppier version
of one control deleted a quorum check outright and broke four unrelated tests. It
was redone to isolate exactly the rule under test.

**The order of the checks in the control script is load-bearing.** `cargo test`
prints `error: test failed, to rerun...` when a *test* fails, so a build-error
check placed first matches every correctly flipped control and reports all of them
as build failures. Nine controls were briefly misread this way.

**A control that fails nothing is a result, not an omission.** Mutating away
the self-CID exclusion broke no test, because it is shadowed by the subject
exclusion added in the same slice. It was kept as the contract of the helper,
with the shadowing stated in the code, rather than shipped as an untestable
rule or deleted as dead.

**Measure before designing, and let the measurement become the regression
test.** Repeatedly the measured hole was larger than the wording of the item.
The custody item was filed as key-reuse hygiene and was a live defect that
authorized sham custody with zero custodians able to open a share; the
member-roll item was filed as a missing feed and the larger half was that the
insider set was read from the archive tip, so one backdated record invalidated
any filing against the accused.

**A status list checked against other status lists reproduces their errors.**
The §11 implementation backlog was reconciled once against the per-component
"v1 to v4" notes and read as done. Checked against the **code** instead, one
row reading "credentialing-panel rotation, sponsor-stake tracking, and
probationary-status enforcement" turned out to be three items wearing one
bullet, of which the first did not exist, the second was built, and the third
was computed and enforced by nothing. It had read as done because a third of it
was.

**A recurring shape worth naming:** a value that no rule reads. It has appeared
four times across the two tiers, as `substrate.anchor` in ethne,
`source_commitment` in KC, the fictional MSRV floor above, and the recipient
key of a sealed share, which the record did not even carry. When a field is
added, check that something refuses on it.

## Working conventions

- **Never `git checkout -- <file>`** to undo a temporary patch. Both this tier and
  ethne are wholly uncommitted, so the command discards real work rather than a
  mutation. Copy the file to scratch first and restore from there.
- **Never `git commit`.** Sean commits.
- **`TODO.md` and `.claude/` are untracked by convention.** Do not add them.
- ethne depends on `kc-core`, so a change to `core`, `ring` or `custody` reaches the
  other workspace. Hold its baseline as part of verifying a KC slice.

---

*Next: [Kith-Covenant whitepaper](../whitepaper/whitepaper.md)*
