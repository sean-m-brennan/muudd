*Previous: [Design decisions D1 to D35](../design.md)*

<!--
  Copyright 2026 Sean M. Brennan and contributors
  Licensed under the Apache License, Version 2.0.
-->

# Build, toolchain, and verification

**Reference document.** How this tier is built, which toolchain choices are
deliberate and why, what breaks when they are ignored, and the verification
discipline the slices are held to. For the design decisions see
[`design.md`](../design.md); for outstanding scope see
[`TODO.md`](../../TODO.md).

Several entries here exist because the obvious thing was tried and was silently
wrong. Those are marked, because the reason is the useful part.

## Build environment

Firstly, the supported build is the per-component conda env, consistent with the
rest of continuous-space: `environment.yaml` pins `python==3.13.*`, `rust`,
`rust-src`, `maturin`, and `pytest` from conda-forge. `run-tests.sh` errors
early and names whatever is missing.

Two environment facts are not optional and cost real time when missed:

- **`CARGO_TARGET_DIR` must be off virtiofs.** A target directory on the repo
  mount is slow enough to change how the work feels, so point it at scratch.
- **Run `maturin develop` from the repository root, not from the `en-pyo3`
  crate directory.** From the crate directory it builds a differently named
  module and leaves the existing `_core.abi3.so` in place. This is not a build
  error, and pytest then runs green against a stale extension. One slice was
  verified against a three-day-old `_core.abi3.so` this way, so the failure mode
  is a *bogus pass*, not a missing module.

`cargo clippy` **is** available in the development sandbox, through `sudo
apt-get install -y rust-clippy`. The apt package and the installed `rustc` must
be the same version, both 1.85.1 here, which is also the declared floor; a
mismatched pair refuses to run. The earlier reading, that clippy needed `rustup
component add` and so could not be had from apt cargo/rustc, was wrong, and it
held long enough that lint counts in this tier were taken on trust rather than
measured.

**Always pass `--all-targets`.** Without it clippy lints library targets only
and every `#[cfg(test)]` module goes unlinted. The difference is not marginal. A
lib-only run of this workspace reported five sites where the full run reported
twelve, and all seven of the missing ones were test code. An inventory taken
that way understates itself while reading as complete.

**A count of reported warnings is not a count of warnings.** An `#[allow]`
removes a site from the report, so a tier can read clean while holding
suppressions nobody has revisited. To see what is behind a decision rather than
what is silent, override the attributes:

```bash
cargo clippy --workspace --all-targets -- --force-warn clippy::type_complexity
```

That distinguishes a lint class that was *fixed* from one that was *silenced*,
which a clean ordinary run cannot.

**`--all-targets` still does not reach the `at-ffi` code**, because that is a
feature gate rather than a target. Adding a bare `--all-features` does not work
either, since `en-at/build.rs` panics without `AT_LIB_DIR`, and the run dies
before it lints anything, which looks like a clippy failure rather than a
missing environment variable. Point it at a built AT:

```bash
AT_LIB_DIR=/path/to/autonomous_trust/src/c/build \
  cargo clippy --workspace --all-targets --all-features
```

Both the default and the all-features runs report zero.

## The toolchain floor

`rust-version = "1.85"` and `resolver = "3"` in the workspace manifest. Both
numbers were arrived at by correcting a declaration that did nothing, and the
sequence is worth keeping because the same defect is easy to reintroduce.

**The declared floor was inert.** `[workspace.package]` declared `rust-version =
"1.75"` while all ten member crates inherited only `edition.workspace = true`.
Neither cargo nor clippy ever applied it. It surfaced as clippy recommending
`.is_multiple_of()`, stabilized in **1.87**, twelve minor versions past the
declared floor, for a style lint. All ten crates now carry
`rust-version.workspace = true`.

**The floor was also fiction.** Once the constraint was live, the resolved graph
showed 1.75 could never have built, since `constant_time_eq` 0.4.2 (reached
through blake3, which every `Cid` goes through) and `cpufeatures` 0.3.0 declare
**1.85**, and ethne adds `zeroize` 1.9.0 at 1.85. So 1.85 is the real floor and
the manifest says why. `.is_multiple_of()` stays out of reach.

**A correct floor is only a snapshot**, which is what `resolver = "3"` (stable
since Cargo 1.84) addresses. It declines dependency versions requiring more Rust
than `rust-version` declares. `Cargo.lock` was unchanged when this landed,
current graph already fitting under 1.85, so it was verified with a paired
control rather than assumed. With the floor temporarily set to 1.78, resolver 3
downgrades four crates and names each one's required Rust, while resolver 2
*upgrades* `wasip2` to a version "requires Rust 1.87.0", notes it, and proceeds.
That second behavior is the hole this closes.

**When a clippy suggestion names a newer API, check `rust-version` before
applying it.**

## Lint policy

`pyo3` is at **0.29**. The upgrade past 0.22 was its own slice rather than a
version bump, because 0.23 deprecates the `*_bound` constructors `en-pyo3` uses
throughout. That migration was six lines across ethne and kith-covenant (two
`PyBytes::new_bound`, two `PyDict::new_bound`), neither crate having used
`IntoPy`/`ToPyObject`, which was 0.23's real break.

**0.27 to 0.29 was predicted to "buy only currency" and did not.** The predicted
risk, 0.28 reworking `#[pymodule]` onto PEP 489 multi-phase initialization, was
the non-event its own migration guide says it is. Two other things were real.

*The deprecation, which the compiler does say.* 0.28 makes the automatic
`FromPyObject` for `#[pyclass]` types that implement `Clone` opt-in, and every
such class warns, being seventeen in `en-pyo3` and thirty-eight in `kc-pyo3`.
The choice per class is `from_py_object` to keep the conversion or
`skip_from_py_object` to drop it, and the deciding question is whether the class
is ever converted *from* Python. Neither tier has a setter, a `set_all`, or a
single by-value `#[pyclass]` parameter, so the automatic impl was dead in both,
and all fifty-five take `skip_from_py_object`. A `&T` parameter is not evidence
against this, since those go through the `PyRef` borrow path and never needed
`FromPyObject` at all.

*The contract change, which nothing warns about.* Before 0.28 a module opted
**in** to free-threaded support with `gil_used = false`; from 0.28 modules allow
it by default and must opt **out**. So the bump hands a module a free-threaded
claim silently, and no warning marks it. Both `_core` modules now declare
`#[pymodule(gil_used = true)]`, which is what they required before the bump.
That is a statement of the status quo rather than a judgement about
thread-safety. It cannot be tested here, because the sandbox interpreter is a
GIL build (`Py_GIL_DISABLED=0`) and never loads a module free-threaded, and the
ethne seam hands out `FileStore`/`ArchiveStore` handles, whose two-writer
failure is silent (**D30**). Lifting the declaration needs a free-threaded
interpreter, not an argument.

**Never run `cargo clippy --fix` on `en-pyo3`.** The `#[pyfunction]` /
`#[pymethods]` expansion emitted an identity `PyErr -> PyErr` conversion, which
clippy reported as `useless_conversion` once per exported function, 73 of them.
Every span pointed at the `PyResult<...>` return type of a function and no body of
ours contained an `.into()`, so `--fix` offers to "apply 73 suggestions" and
would be editing return types. The premise was checked against
`pyo3-macros-backend/src/quotes.rs::ok_wrap` rather than the changelog, which
never mentions it. The identity `map_err(Into::<PyErr>::into)` is in both
versions, spanned at `*output_span`, and 0.27 adds an `#[allow]` where 0.22 had
none. The warnings are gone by upgrade, not by suppression.

**There is no `type_complexity` residue.** The three warnings once recorded here
as unchased were an undercount from a lib-only run; the full inventory was
twelve across the workspace, and all twelve are fixed by naming the concept the
tuple stands for (`MemberRow`, `RiteRow`, `WindowPoint`) rather than by an
`#[allow]`. `--force-warn clippy::type_complexity` reports zero sites, which is
the claim worth making. Not that the report is clean, but that nothing is
hiding.

The tuple-heavy returns are worth naming for a reason beyond the lint. A row
that crosses the Python seam in both directions, once as the return of a getter and
once as a parameter, has to agree with itself element for element, and a bare
tuple repeated in two signatures is a contract nothing states.

**One suggestion is refused on intent rather than on MSRV.**
`clippy::int_plus_one` wants `can_self_govern`'s `majority(n) + 1 <= n` written
as `majority(n) < n` (`en-charter/src/authority.rs`). The two are arithmetically
identical, but *majority plus one* is a named threshold across this tier (D14's
roll override, the contraction rules, expulsion, amendment removal), and the
shorter form no longer says which rule the function enforces. Kept, with a
targeted `#[allow]` carrying that reason.

Kith-covenant's `can_contract` is the same idiom and now carries the same
reasoned `#[allow]`. It had none, so it read as an oversight rather than as the
decision this is, which is the failure mode of refusing a lint in one tier and
not writing it down in the other.

**The arity survivors are stated, not folded into a clean number.**
`--force-warn clippy::too_many_arguments` reports twenty-two sites, nineteen of
them in `en-pyo3`, where the signature *is* the Python surface and collapsing it
into a params struct would change the API to quiet a lint. The rest are test
fixtures whose parameter count is the enumeration under test, each carrying its
own reason. `en-substrate`'s two `propose` variants were the real finding. Their
shared six parameters became `FoundingTerms`, which also retired the blanket
`#[allow]` on the private `build` they both call.

## Linking against autonomous_trust

`en-at`'s real coupling is behind the **`at-ffi`** feature. The default build
links no AT at all, which is what keeps the `cargo tree` decoupling property in
the README true.

**`libautonomous_trust.so` is under-linked**, and every consumer has to
compensate. It records `DT_NEEDED` for libc, libm, libstdc++ and libgcc_s
**only**, while taking symbols from libsodium, jansson, protobuf-c, OpenSSL and
libprotobuf. The AT examples themselves never notice, because `add_at_example` links
the **static** library plus `${DEPENDENCIES}`, which leaves the shared library
effectively untested as a link target. `build.rs` compensates with
`AT_LINK_LIBS`, defaulting to `sodium jansson protobuf-c uuid ssl crypto`, plus
`protobuf` for the C++ symbol `ShutdownProtobufLibrary`. The real fix is linking
those `PUBLIC` on the AT side.

`protobuf` was missing from that default list while `build.rs`'s own comment
named it, so `--features at-ffi` **could not link at all**. Two consequences are
worth carrying. A default list and a comment that disagree is a build that fails
for a reason nobody reads, and this failure looked like an AT bug rather than an
ethne one.

The `cargo:rustc-link-arg` rpath reaches only `en-at`'s own test targets. A
**downstream host binary needs `LD_LIBRARY_PATH`** pointing at the AT build
directory, and that is documented rather than fixed here, since the rpath of a
downstream binary is not this crate's to set.

## Test hazards

**the AT messaging layer keeps process-global state.** `AUTONOMOUS_TRUST_ROOT`
resolves every socket path and `messaging_assign` sets the single queue sends go
out through, so concurrent tests race on both, and Rust runs tests in parallel
by default. All socket tests take one mutex (`socket_lock`, poison recovered so
one failure does not cascade). This was found the expensive way. The suite
passed twice, then failed two or three tests on the first parallel run after new
tests landed. **The earlier green runs were luck**, and treating them as
evidence would have shipped a flaky suite.

**Orphaned AT daemons hold the harness stdout pipe open and hang it.** A
`timeout 60` run held a shell for ten minutes during measurement. Redirect to a
file and `pkill -f autonomous_trust` afterward. A stopped daemon also leaves
subsystem processes running (AT `ISSUES.md` §2.1.3); this tier live test kills
the ones it caused, identifying them exactly, since AT forks without exec and so
the `/proc/<pid>/exe` of an orphan is the test binary itself.

**Fixtures must not hold a single monitor.** D19's retention floor counts
distinct *watchers* per member since D28, so a fixture whose observations all
come from one monitor never clears it. This shipped twice, first as thirteen
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
result twice, in different slices, before it was written down. Re-run as a no-op
arm body, the same control flips correctly.

**A control that does not fire is reported as not-fired, never as passed.** The
justification for the thing it was probing is whatever measurement produced it,
not the control silence.

**Decide up front what "working" means at the level above.** For the at-ffi
decoder that was a `MemberCandidate` reaching `founding_candidate` with a
correctly unrated reputation, not "the decoder returned a struct". A slice
verified at its own boundary can be complete and useless.

A related asymmetry comes from the same family. An interface built for testing
should **not** copy the gating of the thing it stands in for. The AT
`app_events_test_support.{h,c}` helpers pass values through unsanitized, unlike
the real emitter, which zeroes an attendance stamp on an unbound node. A helper
that copied the emitter gating could not be used to test the *consumer's* own
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

---

*Next: [Justus hostis](../../../kith-covenant/doc/background/justus_hostis.md)*
