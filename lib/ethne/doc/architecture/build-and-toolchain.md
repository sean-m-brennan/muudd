<!--
  Copyright 2026 Sean M. Brennan and contributors
  Licensed under the Apache License, Version 2.0.
-->

# Build, toolchain, and verification

**Reference document.** How this tier is built, which toolchain choices are
deliberate and why, what breaks when they are ignored, and the verification
discipline the slices are held to. For the tier's design decisions see
[`design.md`](../design.md); for outstanding scope see [`TODO.md`](../../TODO.md).

Several entries here exist because the obvious thing was tried and was silently
wrong. Those are marked, because the reason is the useful part.

## Build environment

The supported build is the per-component conda env, consistent with the rest of
sustainable-space: `environment.yaml` pins `python==3.13.*`, `rust`, `rust-src`,
`maturin`, and `pytest` from conda-forge. `run-tests.sh` errors early and names
whatever is missing.

Two environment facts are not optional and cost real time when missed:

- **`CARGO_TARGET_DIR` must be off virtiofs.** A target directory on the repo
  mount is slow enough to change how the work feels; point it at scratch.
- **Run `maturin develop` from the repository root, not from the `en-pyo3`
  crate directory.** From the crate directory it builds a differently named
  module and leaves the existing `_core.abi3.so` in place. This is not a build
  error: pytest then runs green against a stale extension. One slice was
  verified against a three-day-old `_core.abi3.so` this way, so the failure mode
  is a *bogus pass*, not a missing module.

`cargo clippy` is not installed in the development sandbox and cannot be
installed there (apt cargo/rustc, no rustup). It is therefore reported as
**not run**, never as clean. Sean re-runs it.

## The toolchain floor

`rust-version = "1.85"` and `resolver = "3"` in the workspace manifest. Both
numbers were arrived at by correcting a declaration that did nothing, and the
sequence is worth keeping because the same defect is easy to reintroduce.

**The declared floor was inert.** `[workspace.package]` declared
`rust-version = "1.75"` while all ten member crates inherited only
`edition.workspace = true`. Neither cargo nor clippy ever applied it. It
surfaced as clippy recommending `.is_multiple_of()`, stabilized in **1.87**,
twelve minor versions past the declared floor, for a style lint. All ten crates
now carry `rust-version.workspace = true`.

**The floor was also fiction.** Once the constraint was live, the resolved graph
showed 1.75 could never have built: `constant_time_eq` 0.4.2 (reached through
blake3, which every `Cid` goes through) and `cpufeatures` 0.3.0 declare
**1.85**, and ethne adds `zeroize` 1.9.0 at 1.85. So 1.85 is the real floor and
the manifest says why. `.is_multiple_of()` stays out of reach.

**A correct floor is only a snapshot**, which is what `resolver = "3"` (stable
since Cargo 1.84) addresses: it declines dependency versions requiring more Rust
than `rust-version` declares. `Cargo.lock` was unchanged when this landed,
today's graph already fitting under 1.85, so it was verified with a paired
control rather than assumed. With the floor temporarily set to 1.78, resolver 3
downgrades four crates and names each one's required Rust, while resolver 2
*upgrades* `wasip2` to a version "requires Rust 1.87.0", notes it, and proceeds.
That second behavior is the hole this closes.

**When a clippy suggestion names a newer API, check `rust-version` before
applying it.**

## Lint policy

`pyo3` is at **0.27**. The upgrade past 0.22 was its own slice rather than a
version bump, because 0.23 deprecates the `*_bound` constructors `en-pyo3` uses
throughout. The migration itself was six lines across ethne and kith-covenant
(two `PyBytes::new_bound`, two `PyDict::new_bound`), neither crate having used
`IntoPy`/`ToPyObject`, which was 0.23's real break. **0.29 is unblocked** by the
1.85 floor and buys only currency.

**Never run `cargo clippy --fix` on `en-pyo3`.** The `#[pyfunction]` /
`#[pymethods]` expansion emitted an identity `PyErr -> PyErr` conversion, which
clippy reported as `useless_conversion` once per exported function, 73 of them.
Every span pointed at a function's `PyResult<...>` return type and no body of
ours contained an `.into()`, so `--fix` offers to "apply 73 suggestions" and
would be editing return types. The premise was checked against
`pyo3-macros-backend/src/quotes.rs::ok_wrap` rather than the changelog, which
never mentions it: the identity `map_err(Into::<PyErr>::into)` is in both
versions, spanned at `*output_span`, and 0.27 adds an `#[allow]` where 0.22 had
none. The warnings are gone by upgrade, not by suppression.

The residue is **3 `type_complexity` warnings** on tuple-heavy returns in
`en-pyo3`. There is no mechanical fix; they are recorded rather than chased, and
are worth `type` aliases if that file is being touched anyway.

**One suggestion is refused on intent rather than on MSRV.**
`clippy::int_plus_one` wants `can_self_govern`'s `majority(n) + 1 <= n` written
as `majority(n) < n` (`en-charter/src/authority.rs`). The two are
arithmetically identical, but *majority plus one* is a named threshold across
this tier (D14's roll override, the contraction rules, expulsion, amendment
removal), and the shorter form no longer says which rule the function enforces.
Kept, with a targeted `#[allow]` carrying that reason.

## Linking against autonomous_trust

`en-at`'s real coupling is behind the **`at-ffi`** feature. The default build
links no AT at all, which is what keeps the `cargo tree` decoupling property in
the README true.

**`libautonomous_trust.so` is under-linked**, and every consumer has to
compensate. It records `DT_NEEDED` for libc, libm, libstdc++ and libgcc_s
**only**, while taking symbols from libsodium, jansson, protobuf-c, OpenSSL and
libprotobuf. AT's own examples never notice, because `add_at_example` links the
**static** library plus `${DEPENDENCIES}`, which leaves the shared library
effectively untested as a link target. `build.rs` compensates with
`AT_LINK_LIBS`, defaulting to `sodium jansson protobuf-c uuid ssl crypto`, plus
`protobuf` for the C++ symbol `ShutdownProtobufLibrary`. The real fix is linking
those `PUBLIC` on AT's side.

`protobuf` was missing from that default list while `build.rs`'s own comment
named it, so `--features at-ffi` **could not link at all**. Two consequences
worth carrying: a default list and a comment that disagree is a build that fails
for a reason nobody reads, and this failure looked like an AT bug rather than an
ethne one.

The `cargo:rustc-link-arg` rpath reaches only `en-at`'s own test targets. A
**downstream host binary needs `LD_LIBRARY_PATH`** pointing at the AT build
directory, and that is documented rather than fixed here, since the rpath of a
downstream binary is not this crate's to set.

## Test hazards

**AT's messaging layer keeps process-global state.** `AUTONOMOUS_TRUST_ROOT`
resolves every socket path and `messaging_assign` sets the single queue sends go
out through, so concurrent tests race on both, and Rust runs tests in parallel by
default. All socket tests take one mutex (`socket_lock`, poison recovered so one
failure does not cascade). This was found the expensive way: the suite passed
twice, then failed two or three tests on the first parallel run after new tests
landed. **The earlier green runs were luck**, and treating them as evidence would
have shipped a flaky suite.

**Orphaned AT daemons hold the harness's stdout pipe open and hang it.** A
`timeout 60` run held a shell for ten minutes during measurement. Redirect to a
file and `pkill -f autonomous_trust` afterward. A stopped daemon also leaves
subsystem processes running (AT `ISSUES.md` §2.1.3); ethne's live test kills the
ones it caused, identifying them exactly, since AT forks without exec and so an
orphan's `/proc/<pid>/exe` is the test binary itself.

**Fixtures must not hold a single monitor.** D19's retention floor counts
distinct *watchers* per member since D28, so a fixture whose observations all
come from one monitor never clears it. This shipped twice: first as thirteen
green tests that satisfied a floor whose stated justification was "somebody was
watching" using one monitor filing repeatedly, and then, after those were fixed,
as both reference hosts still holding one monitor, running 400 observations to
**zero compactions** while printing a compaction count as though one had run.
Rotate three.

**Attendance fixtures must draw from one pool.** A rite fixture that mints fresh
DIDs per movement reads as every attendee lost, so attrition fires on a healthy
rite.

**A refused record still moves derived state that a later record reads.**
Negative-control fixtures therefore put the control first, or on a target of its
own.

## Verification practice

Every slice runs negative controls and reports them, including the ones that did
not fire. Three rules have each been learned by getting them wrong:

**A control that does not compile is not a control.** Patching a match arm to
`if false` makes the match non-exhaustive, so the build fails, and a grep for
failing tests counts that as "the test cannot fail". This produced a false
result twice, in different slices, before it was written down. Re-run as a
no-op arm body, the same control flips correctly.

**A control that does not fire is reported as not-fired, never as passed.** The
justification for the thing it was probing is whatever measurement produced it,
not the control's silence.

**Decide up front what "working" means at the level above.** For the at-ffi
decoder that was a `MemberCandidate` reaching `founding_candidate` with a
correctly unrated reputation, not "the decoder returned a struct". A slice
verified at its own boundary can be complete and useless.

A related asymmetry, from the same family: an interface built for testing should
**not** copy the gating of the thing it stands in for. AT's
`app_events_test_support.{h,c}` helpers pass values through unsanitized, unlike
the real emitter, which zeroes an attendance stamp on an unbound node. A helper
that copied the emitter's gating could not be used to test the *consumer's* own
gating, which is exactly what a foreign consumer most needs to test. Both
directions are covered because the helper refuses to be careful.

## Working conventions

- **Never `git checkout -- <file>`** to undo a temporary patch. Both this tier
  and kith-covenant are wholly uncommitted, so the command discards real work
  rather than a mutation. Copy the file to scratch first and restore from there.
- **`TODO.md` and `.claude/` are untracked by convention.** Do not add them.
- `en-charter` must stay `kc-core`-only, and the default workspace graph must
  stay free of `rusqlite`. Both are `cargo tree`-checked properties, re-checked
  rather than assumed whenever a dependency is added (D31 added `en-host` to
  `en-pyo3` and re-checked both).
