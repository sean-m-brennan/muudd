/*
  Copyright 2026 Sean M. Brennan and contributors

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

     http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

//! The `AtEventSource` seam (decision **D5**): the single point of contact with
//! autonomous_trust. It yields decoded [`AtEvent`]s; everything above it
//! ([`crate::AtSubstrateProvider`]) is pure Rust and unit-testable against the
//! in-memory [`MockEventSource`]. The real C linkage — both coupling modes of
//! decision **D6** — lives in the `at-ffi`-gated [`ffi`] module, so the default
//! build (and all in-sandbox tests) never link a daemon.

use std::collections::VecDeque;

use en_core::KcResult;

use crate::event::AtEvent;

/// A source of decoded AT events. Implementations must be **non-blocking**:
/// [`poll`](AtEventSource::poll) returns whatever has arrived since the last
/// call (possibly empty) and never waits on the network — matching AT's own
/// single-threaded, non-blocking process loop.
pub trait AtEventSource {
    /// Drain the events available since the last poll.
    fn poll(&mut self) -> KcResult<Vec<AtEvent>>;
}

/// A deterministic, dependency-free event source for tests and for offline
/// founding (feeding a known roll with no daemon at all — the seam that lets a
/// captured/synthesized AT view be replayed). Events queued are returned in FIFO
/// order and consumed on `poll`.
#[derive(Debug, Default)]
pub struct MockEventSource {
    queued: VecDeque<AtEvent>,
}

impl MockEventSource {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue one event.
    pub fn push(&mut self, event: AtEvent) {
        self.queued.push_back(event);
    }

    /// Queue a batch of events.
    pub fn extend<I: IntoIterator<Item = AtEvent>>(&mut self, events: I) {
        self.queued.extend(events);
    }

    /// Builder-style seeding.
    pub fn with(mut self, events: impl IntoIterator<Item = AtEvent>) -> Self {
        self.extend(events);
        self
    }
}

impl AtEventSource for MockEventSource {
    fn poll(&mut self) -> KcResult<Vec<AtEvent>> {
        Ok(self.queued.drain(..).collect())
    }
}

/// The real autonomous_trust C linkage, compiled only under the `at-ffi`
/// feature — a deployment/integration build, never the default one.
///
/// Binds AT's **flat, fixed-width** `app_events.h` ABI and nothing else. That
/// restraint is the point: `generic_msg_t` is a tagged union whose layout depends
/// on `public_identity_t`, `group_t`, `net_msg_t` and AT's ZTA build flag, so a
/// hand-written Rust mirror of it would corrupt silently the first time any of
/// those changed. Everything here is arrays of bytes and primitives.
///
/// # Both modes of D6 (2026-07-30)
///
/// [`AttachedAtDaemon`] reads a daemon someone else runs. [`EmbeddedAtNode`] owns
/// one, over AT's `app_node.h` — the flat lifecycle written for exactly this,
/// because `node.h` cannot be bound from another language: `at_node_init` and
/// `at_node_start` take a caller-allocated `at_node_t`, which embeds `logger_t`
/// and `queue_t` **by value**, so a Rust mirror would have to reproduce two opaque
/// C layouts. `run_autonomous_trust` is not a safe shortcut either — its declared
/// contract disagrees with `node.c`'s use of it, and its header and definition
/// disagree on a parameter type. Hence an opaque heap handle on the C side and
/// nothing mirrored here.
#[cfg(feature = "at-ffi")]
pub mod ffi {
    use std::ffi::{c_char, c_int, CString};

    use en_core::{KcError, KcResult};

    use crate::decode::{ObservedReading, PeerJoin, ReputationReading, AT_SIGNING_KEY_LEN, AT_UUID_LEN};
    use crate::event::AtEvent;
    use crate::source::AtEventSource;

    /// Matches `at_app_event_kind_t`. Values are frozen on the C side: new kinds
    /// are only ever appended, so an unknown tag is skipped rather than fatal.
    const KIND_PEER_OBSERVED: c_int = 1;
    const KIND_PEER_REPUTATION: c_int = 2;

    /// How many events to drain per `poll`. AT's own loop is non-blocking, so a
    /// short buffer costs an extra call, never a lost event.
    const BATCH: usize = 32;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct AtAppPeer {
        peer_uuid: [u8; AT_UUID_LEN],
        signing_pubkey: [u8; AT_SIGNING_KEY_LEN],
        rank: i32,
        operator_bound: bool,
        operator_attested_at: f64,
        /// Appended by AT's operator-key slice. It landed in the struct's
        /// existing tail padding, so every offset above is unchanged and only
        /// the SIZE moved (64 -> 96) — which is exactly why the assertions
        /// below check both, and why they are measured with a compiler rather
        /// than reasoned about.
        operator_pubkey: [u8; AT_SIGNING_KEY_LEN],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct AtAppReputation {
        peer_uuid: [u8; AT_UUID_LEN],
        score: f64,
        rated: bool,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    union AtAppData {
        peer: AtAppPeer,
        reputation: AtAppReputation,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct AtAppEvent {
        kind: c_int,
        data: AtAppData,
    }

    /// Opaque; the C side owns the allocation and the bound queue.
    #[repr(C)]
    struct AtAppEventsHandle {
        _private: [u8; 0],
    }

    extern "C" {
        fn at_app_events_open(q_in: *const c_char) -> *mut AtAppEventsHandle;
        fn at_app_events_open_existing() -> *mut AtAppEventsHandle;
        fn at_app_events_poll(
            handle: *mut AtAppEventsHandle,
            out: *mut AtAppEvent,
            max: usize,
        ) -> c_int;
        fn at_app_events_request_roster(
            handle: *mut AtAppEventsHandle,
            q_out: *const c_char,
        ) -> c_int;
        fn at_app_events_close(handle: *mut AtAppEventsHandle);
    }

    /// Opaque; the C side owns the `at_node_t` and the queue it bound.
    #[repr(C)]
    struct AtAppNodeHandle {
        _private: [u8; 0],
    }

    extern "C" {
        fn at_app_node_start(
            app_name: *const c_char,
            q_in: *const c_char,
            q_out: *const c_char,
            log_level: c_int,
            generate_config: bool,
            log_file: *const c_char,
        ) -> *mut AtAppNodeHandle;
        fn at_app_node_alive(node: *mut AtAppNodeHandle) -> bool;
        fn at_app_node_ready(node: *mut AtAppNodeHandle) -> bool;
        fn at_app_node_wait_ready(node: *mut AtAppNodeHandle, timeout_ms: c_int) -> bool;
        fn at_app_node_pid(node: *const AtAppNodeHandle) -> c_int;
        fn at_app_node_stop(node: *mut AtAppNodeHandle);
    }

    /// `app_events.h`'s `AT_APP_NOT_READY`: the daemon is there but has not bound
    /// its queue yet, which on a cold start is the ordinary case for the first
    /// ~210 ms. Distinct from the generic -1 so a caller can tell "wait" from
    /// "this will never work".
    const AT_APP_NOT_READY: c_int = -2;

    /// AT log levels, as `app_node.h` fixes them.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(i32)]
    pub enum AtLogLevel {
        Debug = 1,
        Info = 2,
        Warning = 3,
        Error = 4,
        Critical = 5,
    }

    // AT's `app_events_test_support.h` — the daemon's side of the wire, flat, so
    // a test here can play daemon. Sends go out through the queue this process
    // already assigned (which `AttachedAtDaemon::open` does), so a test may emit
    // *to the name it bound* and loop a synthetic event back over the real
    // socket. Values pass through unsanitized on purpose: a helper that copied
    // the emitter's gating could not test ours.
    #[cfg(test)]
    extern "C" {
        fn at_app_test_emit_peer(
            q_name: *const c_char,
            peer_uuid: *const u8,
            signing_pubkey: *const u8,
            rank: i32,
            operator_bound: bool,
            operator_attested_at: f64,
            // May be NULL for the ordinary "no guardian named" case; passed
            // through verbatim, so a test can emit a key with
            // `operator_bound` false and prove the consumer refuses it.
            operator_pubkey: *const u8,
        ) -> c_int;
        fn at_app_test_emit_reputation(
            q_name: *const c_char,
            peer_uuid: *const u8,
            score: f64,
            rated: bool,
        ) -> c_int;
    }

    /// **Attach** to an already-running daemon and read its app-facing stream.
    /// Fits a co-deployed fleet where AT is up and this process does not own it.
    pub struct AttachedAtDaemon {
        handle: *mut AtAppEventsHandle,
        q_out: Option<CString>,
        join: PeerJoin,
        roster_requested: bool,
    }

    impl AttachedAtDaemon {
        /// Bind the AT→app queue by name. `q_in` must be the same string the
        /// daemon was configured with; binding any other name receives nothing,
        /// silently — a mismatch is the fourth of the seven breaks AT's carrier
        /// had, and it cannot be detected from this side.
        ///
        /// `q_out` is the daemon's inbound queue, needed only to pull a full
        /// roster. `None` disables pulling, which means unrated peers will never
        /// be observed: `rated == false` crosses on the pull alone.
        pub fn open(q_in: &str, q_out: Option<&str>) -> KcResult<Self> {
            let name = cstr(q_in)?;
            // SAFETY: `name` outlives the call; the C side copies the string.
            let handle = unsafe { at_app_events_open(name.as_ptr()) };
            Self::wrap(handle, q_out, q_in)
        }

        /// Borrow a queue this process already bound (the header's
        /// `at_app_events_open_existing`). Two binders of one name fight over the
        /// same socket path — the second unlinks the first's — so a host that
        /// bound the queue itself must use this.
        pub fn open_existing(q_out: Option<&str>) -> KcResult<Self> {
            // SAFETY: no arguments; returns NULL on allocation failure only.
            let handle = unsafe { at_app_events_open_existing() };
            Self::wrap(handle, q_out, "(existing)")
        }

        fn wrap(
            handle: *mut AtAppEventsHandle,
            q_out: Option<&str>,
            what: &str,
        ) -> KcResult<Self> {
            if handle.is_null() {
                return Err(KcError::Key(format!(
                    "at_app_events: could not bind '{what}' \
                     (already bound, or the socket path is unavailable)"
                )));
            }
            let q_out = match q_out {
                Some(q) => Some(cstr(q)?),
                None => None,
            };
            Ok(Self {
                handle,
                q_out,
                join: PeerJoin::new(),
                roster_requested: false,
            })
        }

        /// Ask AT to re-emit its whole current view.
        ///
        /// Worth calling repeatedly, not once. A pull reports what AT knows *at
        /// that moment*, and a node that has not finished discovery honestly
        /// knows nothing — on a cold 3-node cohort the first successful pull
        /// returned 0 peers 23 seconds before the first admission. It is also the
        /// only path on which an unrated peer can cross.
        pub fn request_roster(&mut self) -> KcResult<()> {
            let Some(q_out) = self.q_out.as_ref() else {
                return Err(KcError::Key(
                    "at_app_events: no q_out configured, cannot pull a roster".into(),
                ));
            };
            // SAFETY: handle is non-null by construction; q_out outlives the call.
            let rc = unsafe { at_app_events_request_roster(self.handle, q_out.as_ptr()) };
            // The "(daemon not ready?)" in the old message was a guess, because the
            // C side answered -1 to both cases. It now says which, so the two stop
            // being one diagnosis: not-ready means retry, anything else does not.
            if rc == AT_APP_NOT_READY {
                return Err(KcError::Key(
                    "at_app_events: daemon has not bound its queue yet — not ready, retry"
                        .into(),
                ));
            }
            if rc != 0 {
                return Err(KcError::Key(
                    "at_app_events: roster request refused".into(),
                ));
            }
            self.roster_requested = true;
            Ok(())
        }

        /// Whether a roster pull has been accepted at least once. A consumer that
        /// has never pulled cannot have seen an unrated peer.
        pub fn roster_requested(&self) -> bool {
            self.roster_requested
        }

        /// Reputations held for peers not yet observed — see `PeerJoin` rule 1.
        pub fn held_reputations(&self) -> usize {
            self.join.held_reputations()
        }

        /// Close the handle now, ahead of whatever owns the queue it reads.
        /// Idempotent, and [`Drop`] then does nothing — needed because
        /// [`EmbeddedAtNode`] must close the reader before stopping the node that
        /// bound the queue.
        fn close_borrowed(&mut self) {
            if !self.handle.is_null() {
                // SAFETY: non-null here, and nulled immediately, so never twice.
                unsafe { at_app_events_close(self.handle) };
                self.handle = std::ptr::null_mut();
            }
        }
    }

    impl AtEventSource for AttachedAtDaemon {
        fn poll(&mut self) -> KcResult<Vec<AtEvent>> {
            let mut batch = [AtAppEvent {
                kind: 0,
                data: AtAppData {
                    peer: AtAppPeer {
                        peer_uuid: [0; AT_UUID_LEN],
                        signing_pubkey: [0; AT_SIGNING_KEY_LEN],
                        rank: 0,
                        operator_bound: false,
                        operator_attested_at: 0.0,
                        operator_pubkey: [0; AT_SIGNING_KEY_LEN],
                    },
                },
            }; BATCH];

            // SAFETY: `batch` is BATCH events long and we say so; the C side
            // writes at most that many and returns how many it wrote.
            let n = unsafe { at_app_events_poll(self.handle, batch.as_mut_ptr(), BATCH) };
            if n < 0 {
                return Err(KcError::Key("at_app_events_poll failed".into()));
            }

            let mut out = Vec::new();
            for raw in batch.iter().take(n as usize) {
                match raw.kind {
                    // SAFETY: the tag selects the union arm, which is the ABI's
                    // whole contract; both arms are plain Copy data.
                    KIND_PEER_OBSERVED => {
                        let p = unsafe { raw.data.peer };
                        out.extend(self.join.apply_observed(&ObservedReading {
                            peer_uuid: p.peer_uuid,
                            signing_pubkey: p.signing_pubkey,
                            rank: p.rank,
                            operator_bound: p.operator_bound,
                            operator_attested_at: p.operator_attested_at,
                            operator_pubkey: p.operator_pubkey,
                        }));
                    }
                    KIND_PEER_REPUTATION => {
                        let r = unsafe { raw.data.reputation };
                        out.extend(self.join.apply_reputation(&ReputationReading {
                            peer_uuid: r.peer_uuid,
                            score: r.score,
                            rated: r.rated,
                        }));
                    }
                    // A kind this build does not know. The C header freezes its
                    // values and only appends, so skipping is forward-compatible
                    // and an older consumer keeps decoding what it understood.
                    _ => {}
                }
            }
            Ok(out)
        }
    }

    impl Drop for AttachedAtDaemon {
        fn drop(&mut self) {
            // Null only when `close_borrowed` already ran (the embedded path).
            // Closing a handle opened with `open_existing` leaves the borrowed
            // queue itself open, per the header.
            self.close_borrowed();
        }
    }

    /// **Embed** an AT node: this process owns the daemon and reads its stream.
    /// Self-contained, for a standalone tool or an integration test that should
    /// not depend on a daemon being up already.
    ///
    /// Composition rather than duplication: `at_app_node_start` binds and assigns
    /// `q_in` itself, so the reader here is an [`AttachedAtDaemon`] built with
    /// `open_existing` — the same choice `app_node.h` documents, because two
    /// binders of one name fight over the socket path and the second unlinks the
    /// first's.
    pub struct EmbeddedAtNode {
        node: *mut AtAppNodeHandle,
        events: AttachedAtDaemon,
    }

    impl EmbeddedAtNode {
        /// Launch a daemon and attach to it.
        ///
        /// Returns as soon as the daemon is **forked**, not when it is reachable:
        /// measured, `start` returns in about **2 ms** and the daemon's queue is
        /// not bound for another **~210 ms**. Anything sent in that window goes
        /// nowhere.
        ///
        /// This used to say only that "nothing waits, so the first sends may find
        /// no recipient", which was the honest statement of a gap with no answer.
        /// There is now an answer: [`ready`](Self::ready) to ask, or
        /// [`wait_ready`](Self::wait_ready) to wait.
        ///
        /// Being ready is still not the same as knowing anything, so
        /// [`request_roster`](AttachedAtDaemon::request_roster) is *also* worth
        /// repeating: on a cold node an early pull truthfully answers "nothing
        /// yet". Readiness fixes the sends that vanished; it does not fill a
        /// roster that has not been discovered.
        pub fn start(
            app_name: &str,
            q_in: &str,
            q_out: &str,
            log_level: AtLogLevel,
            generate_config: bool,
        ) -> KcResult<Self> {
            Self::start_logging_to(app_name, q_in, q_out, log_level, generate_config, None)
        }

        /// As [`start`](Self::start), naming a file for the daemon's log.
        ///
        /// Not cosmetic, and the reason is worth knowing before embedding a node
        /// inside anything that captures output. With no log file,
        /// `autonomous_trust.c` sets `NO_STDERR_REDIRECT` on the daemon **and**
        /// `process_child_extra_flags` on every subsystem process it forks — so all
        /// of them keep this process's stderr, deliberately, so their logs reach a
        /// terminal.
        ///
        /// Under a harness that captures stderr through a pipe, that is a hang: the
        /// harness reads to EOF, and EOF does not come while any of those children
        /// holds the write end. Measured during this work — a `timeout 60` run held
        /// the shell for ten minutes. Naming a log file sends the daemon's whole
        /// process tree to `/dev/null` for stdio and leaves the pipe alone.
        ///
        /// `None` is the terminal-friendly behaviour and stays the default.
        pub fn start_logging_to(
            app_name: &str,
            q_in: &str,
            q_out: &str,
            log_level: AtLogLevel,
            generate_config: bool,
            log_file: Option<&str>,
        ) -> KcResult<Self> {
            let (name, qi, qo) = (cstr(app_name)?, cstr(q_in)?, cstr(q_out)?);
            let log = match log_file {
                Some(p) => Some(cstr(p)?),
                None => None,
            };
            let log_ptr = log.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
            // SAFETY: all three strings outlive the call — and only the call:
            // `app_node.h` copies them into the handle, so these may drop here
            // while the node lives on. (`node.h` keeps the caller's pointers,
            // which is exactly why a foreign host must not use it.) The C side
            // validates them (empty, q_in == q_out, too long to survive the
            // messaging layer, log level out of range) and returns NULL rather
            // than proceeding. A null log_file means stderr, with the process-tree
            // consequence documented on `start_logging_to`.
            let node = unsafe {
                at_app_node_start(
                    name.as_ptr(),
                    qi.as_ptr(),
                    qo.as_ptr(),
                    log_level as c_int,
                    generate_config,
                    log_ptr,
                )
            };
            if node.is_null() {
                return Err(KcError::Key(format!(
                    "at_app_node_start failed for '{app_name}' \
                     (bad argument, unwritable config/data dir, the daemon did not \
                     launch, or '{q_in}' did not bind)"
                )));
            }
            // The node bound q_in; borrow it rather than binding a second time.
            let events = match AttachedAtDaemon::open_existing(Some(q_out)) {
                Ok(e) => e,
                Err(e) => {
                    // SAFETY: non-null, and not yet stopped.
                    unsafe { at_app_node_stop(node) };
                    return Err(e);
                }
            };
            Ok(Self { node, events })
        }

        /// Whether the daemon is still running. Probed, not cached: a daemon that
        /// died looks exactly like one with nothing to say, and a consumer polling
        /// an empty queue would otherwise never learn.
        pub fn alive(&mut self) -> bool {
            // SAFETY: non-null by construction.
            unsafe { at_app_node_alive(self.node) }
        }

        /// Whether the daemon can actually be **reached** — alive *and* listening.
        ///
        /// [`alive`](Self::alive) reports a running process and is not wrong; this
        /// reports a two-way stream, which is the question a host that is about to
        /// send actually has. Non-blocking, so it belongs directly in a poll loop.
        pub fn ready(&mut self) -> bool {
            // SAFETY: non-null by construction.
            unsafe { at_app_node_ready(self.node) }
        }

        /// Wait up to `timeout` for [`ready`](Self::ready).
        ///
        /// For a tool or a test whose next step is pointless before the daemon can
        /// hear it. A host running its own loop should poll [`ready`](Self::ready)
        /// instead of blocking in here. Returns early — not at the end of the
        /// timeout — if the daemon dies while waiting.
        ///
        /// A `timeout` past `c_int::MAX` milliseconds is clamped rather than
        /// wrapped: a saturating cast keeps a `Duration::MAX` from arriving in C as
        /// a negative timeout, which would mean "check once and give up".
        pub fn wait_ready(&mut self, timeout: std::time::Duration) -> bool {
            let ms = timeout.as_millis().min(c_int::MAX as u128) as c_int;
            // SAFETY: non-null by construction.
            unsafe { at_app_node_wait_ready(self.node, ms) }
        }

        /// The daemon's pid, for logs.
        pub fn pid(&self) -> i32 {
            // SAFETY: non-null by construction.
            unsafe { at_app_node_pid(self.node) }
        }

        /// Ask the daemon to re-emit its whole current view. See
        /// [`AttachedAtDaemon::request_roster`] — including why once is not enough.
        pub fn request_roster(&mut self) -> KcResult<()> {
            self.events.request_roster()
        }

        /// Reputations held for peers not yet observed (`PeerJoin` rule 1).
        pub fn held_reputations(&self) -> usize {
            self.events.held_reputations()
        }
    }

    impl AtEventSource for EmbeddedAtNode {
        fn poll(&mut self) -> KcResult<Vec<AtEvent>> {
            self.events.poll()
        }
    }

    impl Drop for EmbeddedAtNode {
        fn drop(&mut self) {
            // Order matters and is the header's: the reader borrows the node's
            // queue, so it closes first. `events` is dropped after this body, so
            // close it explicitly before stopping the node.
            self.events.close_borrowed();
            // SAFETY: non-null by construction, stopped exactly once.
            unsafe { at_app_node_stop(self.node) };
        }
    }

    fn cstr(s: &str) -> KcResult<CString> {
        CString::new(s).map_err(|_| {
            KcError::Key("at_app_events: queue name contains an interior NUL".into())
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// The layout these `#[repr(C)]` mirrors must have, or every field read
        /// through the union is garbage — the shape of AT's own break #5, where a
        /// tag was read from the first eight payload bytes.
        ///
        /// **The numbers are not hand-computed.** They come from clang on
        /// `app_events.h` (2026-07-30): `at_app_peer_t` 64/8,
        /// `at_app_reputation_t` 32/8, `at_app_event_t` 72/8, `kind` at offset 0
        /// and `data` at offset 8 — the 4 bytes of padding after the `int` tag
        /// being exactly the kind of detail that is wrong when guessed.
        #[test]
        fn the_flat_abi_layout_matches_what_c_reports() {
            assert_eq!(std::mem::size_of::<AtAppPeer>(), 96);
            assert_eq!(std::mem::align_of::<AtAppPeer>(), 8);
            assert_eq!(std::mem::size_of::<AtAppReputation>(), 32);
            assert_eq!(std::mem::align_of::<AtAppReputation>(), 8);
            assert_eq!(std::mem::size_of::<AtAppEvent>(), 104);
            assert_eq!(std::mem::align_of::<AtAppEvent>(), 8);
            assert_eq!(std::mem::offset_of!(AtAppEvent, kind), 0);
            assert_eq!(std::mem::offset_of!(AtAppEvent, data), 8);
            // Field offsets within the arms, for the same reason.
            assert_eq!(std::mem::offset_of!(AtAppPeer, peer_uuid), 0);
            assert_eq!(std::mem::offset_of!(AtAppPeer, signing_pubkey), 16);
            assert_eq!(std::mem::offset_of!(AtAppPeer, rank), 48);
            assert_eq!(std::mem::offset_of!(AtAppPeer, operator_bound), 52);
            assert_eq!(std::mem::offset_of!(AtAppPeer, operator_attested_at), 56);
            assert_eq!(std::mem::offset_of!(AtAppPeer, operator_pubkey), 64);
            assert_eq!(std::mem::offset_of!(AtAppReputation, peer_uuid), 0);
            assert_eq!(std::mem::offset_of!(AtAppReputation, score), 16);
            assert_eq!(std::mem::offset_of!(AtAppReputation, rated), 24);
        }

        /// Binding a queue, polling it empty, and closing it — real FFI calls
        /// over a real unix socket, no daemon involved. `poll` on an empty queue
        /// must report 0 rather than blocking or erroring, since that is what a
        /// consumer sees on most ticks.
        #[test]
        fn an_empty_queue_polls_clean_over_a_real_socket() {
            let (mut src, _q, _guard) = bound("empty");
            let events = src.poll().expect("poll an empty queue");
            assert!(events.is_empty());
            assert_eq!(src.held_reputations(), 0);
            assert!(!src.roster_requested());
        }

        #[test]
        fn a_roster_pull_without_a_q_out_is_refused_not_ignored() {
            let (mut src, _q, _guard) = bound("no-qout");
            assert!(src.request_roster().is_err());
        }

        #[test]
        fn a_queue_name_with_an_interior_nul_is_rejected() {
            assert!(AttachedAtDaemon::open("bad\0name", None).is_err());
        }

        /// **These tests cannot run concurrently, and must say so.**
        ///
        /// The C messaging layer keeps two pieces of *process*-global state:
        /// `AUTONOMOUS_TRUST_ROOT` resolves every socket path, and
        /// `messaging_assign` sets the single queue that sends go out through
        /// (`at_app_events_open` calls it). Two of these tests running at once
        /// race on both — and since Rust runs tests in parallel by default, that
        /// is the norm, not the exception. They passed twice by luck before
        /// failing with `emit_peer failed`; `--test-threads=1` passed throughout,
        /// which is what identified it. Serializing is the fix; the flakiness was
        /// in the tests, not in the decoder.
        ///
        /// Poisoning is recovered from deliberately: one failing test should
        /// report its own failure, not cascade into every test after it.
        fn socket_lock() -> std::sync::MutexGuard<'static, ()> {
            static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
            LOCK.get_or_init(|| std::sync::Mutex::new(()))
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        }

        /// Set up a queue root and bind one, returning the source, its name, and
        /// the guard that keeps this test alone in the process.
        fn bound(tag: &str) -> (AttachedAtDaemon, CString, std::sync::MutexGuard<'static, ()>) {
            let guard = socket_lock();
            let root = std::env::temp_dir().join(format!("en-at-ffi-{tag}"));
            std::fs::create_dir_all(root.join("var/at")).expect("queue dir");
            std::env::set_var("AUTONOMOUS_TRUST_ROOT", &root);
            let name = format!("en_at_loop_{tag}");
            let src = AttachedAtDaemon::open(&name, None).expect("bind");
            (src, CString::new(name).unwrap(), guard)
        }

        fn emit_peer(q: &CString, uuid: &[u8; 16], key: &[u8; 32], rank: i32, bound_op: bool, at: f64) {
            emit_peer_guarded(q, uuid, key, rank, bound_op, at, None)
        }

        /// As [`emit_peer`], naming a guardian. Split out so the ordinary
        /// no-guardian call — the one every pre-existing test makes — stays
        /// exactly as short as it was: opting out has to be the easy path in
        /// the tests too, or the tests stop resembling the deployment.
        fn emit_peer_guarded(
            q: &CString,
            uuid: &[u8; 16],
            key: &[u8; 32],
            rank: i32,
            bound_op: bool,
            at: f64,
            guardian: Option<&[u8; 32]>,
        ) {
            // SAFETY: pointers are to live arrays of the lengths the C side
            // documents, and the queue name outlives the call. A null guardian
            // pointer is the documented "no key" case.
            let rc = unsafe {
                at_app_test_emit_peer(
                    q.as_ptr(),
                    uuid.as_ptr(),
                    key.as_ptr(),
                    rank,
                    bound_op,
                    at,
                    guardian.map_or(std::ptr::null(), |g| g.as_ptr()),
                )
            };
            assert_eq!(rc, 0, "emit_peer failed");
        }

        fn emit_rep(q: &CString, uuid: &[u8; 16], score: f64, rated: bool) {
            // SAFETY: as above.
            let rc = unsafe { at_app_test_emit_reputation(q.as_ptr(), uuid.as_ptr(), score, rated) };
            assert_eq!(rc, 0, "emit_reputation failed");
        }

        fn only_peer(events: &[AtEvent]) -> &crate::event::AtPeer {
            assert_eq!(events.len(), 1, "expected exactly one event: {events:?}");
            match &events[0] {
                AtEvent::PeerObserved(p) => p,
                other => panic!("expected PeerObserved, got {other:?}"),
            }
        }

        /// **The rung that was missing.** A synthetic peer and its score cross a
        /// real unix socket, through AT's own encode/decode, and arrive joined —
        /// so the `#[repr(C)]` mirrors, the union tag, the poll convention and the
        /// uuid join are all exercised by one assertion instead of assumed.
        #[test]
        fn a_peer_and_its_score_cross_the_socket_and_arrive_joined() {
            let (mut src, q, _guard) = bound("join");
            let uuid = [0x5b_u8; 16];
            let key = [0xf2_u8; 32];

            emit_peer(&q, &uuid, &key, 4, false, 0.0);
            let events = src.poll().expect("poll");
            let p = only_peer(&events);
            assert_eq!(p.signing_pubkey, key.to_vec());
            assert_eq!(p.rank, 4);
            // Nothing heard about reputation yet, which is not a score of 0.
            assert_eq!(p.reputation, None);

            emit_rep(&q, &uuid, 0.91, true);
            let events = src.poll().expect("poll");
            let p = only_peer(&events);
            // Joined on the uuid, and the identity half survived the join.
            assert_eq!(p.reputation, Some(0.91));
            assert_eq!(p.signing_pubkey, key.to_vec());
            assert_eq!(p.rank, 4);
            assert!(p.did().is_ok(), "a did:key derives from the delivered key");
        }

        /// The signal ethne D18 rests on, over the real transport: `rated=false`
        /// must arrive as *no reading*, not as the score riding beside it. The
        /// helper passes that score through precisely so this can be checked.
        #[test]
        fn an_unrated_score_crosses_as_no_reading() {
            let (mut src, q, _guard) = bound("unrated");
            let uuid = [0x77_u8; 16];
            let key = [0x88_u8; 32];

            emit_peer(&q, &uuid, &key, 0, false, 0.0);
            // AT's PREREP_NEUTRAL rides along, and must not be believed.
            emit_rep(&q, &uuid, 0.2, false);

            let events = src.poll().expect("poll");
            // Two messages, two upserts — the feed is upsert-only by design.
            assert_eq!(events.len(), 2);
            match events.last().unwrap() {
                AtEvent::PeerObserved(p) => assert_eq!(p.reputation, None),
                other => panic!("expected PeerObserved, got {other:?}"),
            }
        }

        /// Rule 1 over the real transport: a score for a peer never observed has
        /// no signing key to name, so it is held rather than dropped or guessed.
        #[test]
        fn a_score_arriving_first_is_held_until_the_peer_appears() {
            let (mut src, q, _guard) = bound("order");
            let uuid = [0x33_u8; 16];
            let key = [0x44_u8; 32];

            emit_rep(&q, &uuid, 0.65, true);
            let events = src.poll().expect("poll");
            assert!(events.is_empty(), "nothing nameable yet");
            assert_eq!(src.held_reputations(), 1);

            emit_peer(&q, &uuid, &key, 2, false, 0.0);
            let events = src.poll().expect("poll");
            assert_eq!(only_peer(&events).reputation, Some(0.65));
            assert_eq!(src.held_reputations(), 0);
        }

        /// The consumer's own gating, which is why the helper does not sanitize:
        /// a stamp on a node without the verified bound attests to nothing, and
        /// the decoder must drop it even when the wire carries it.
        #[test]
        fn a_stamp_on_an_unbound_node_is_dropped_at_this_end() {
            let (mut src, q, _guard) = bound("stamp");
            emit_peer(&q, &[0x11; 16], &[0x22; 32], 0, false, 1_721_800_000.0);
            let events = src.poll().expect("poll");
            let p = only_peer(&events);
            assert!(!p.operator_bound);
            assert_eq!(p.operator_attested_at, None);
            assert_eq!(p.attended_at(), None);
        }

        /// `at_app_node_start` validates before it touches the filesystem or forks
        /// anything, so these reach the real C guards without starting a daemon.
        /// No socket lock needed: nothing is bound and the root is never read.
        ///
        /// This used to add "which matters because a daemon cannot start in this
        /// sandbox at all (autonomous_trust ISSUES.md §2.1.1)". §2.1.1 is fixed and
        /// that is no longer true — see `a_live_daemon_starts_becomes_ready_and_stops`
        /// below, which does start one.
        #[test]
        fn embedding_refuses_one_queue_name_for_both_directions() {
            // The two directions are separate sockets; one name would have the
            // daemon receiving its own output.
            let err = EmbeddedAtNode::start("en_at", "same_q", "same_q", AtLogLevel::Error, false);
            assert!(err.is_err());
        }

        #[test]
        fn embedding_refuses_an_empty_queue_name() {
            assert!(EmbeddedAtNode::start("en_at", "", "app_to_at", AtLogLevel::Error, false).is_err());
            assert!(EmbeddedAtNode::start("", "at_to_app", "app_to_at", AtLogLevel::Error, false).is_err());
        }

        #[test]
        fn embedding_refuses_a_name_with_an_interior_nul() {
            assert!(
                EmbeddedAtNode::start("en\0at", "at_to_app", "app_to_at", AtLogLevel::Error, false)
                    .is_err()
            );
        }

        /// A queue name longer than AT's messaging layer keeps (`MSG_KEY_LEN - 1`
        /// = 63 bytes) is refused rather than truncated. A silently shortened name
        /// is a name the daemon does not send to — break #4 by another route, and
        /// invisible from here, since a truncating start would succeed and then
        /// deliver nothing.
        ///
        /// No positive control on this side: the longest accepted name gets past
        /// validation and *forks a daemon*, which cannot run in this sandbox. The
        /// boundary is controlled in AT's own `app_node_test.c`.
        #[test]
        fn embedding_refuses_a_name_too_long_to_survive_the_messaging_layer() {
            let long = "q".repeat(64);
            assert!(
                EmbeddedAtNode::start("en_at", &long, "app_to_at", AtLogLevel::Error, false)
                    .is_err()
            );
            assert!(
                EmbeddedAtNode::start("en_at", "at_to_app", &long, AtLogLevel::Error, false)
                    .is_err()
            );
            assert!(
                EmbeddedAtNode::start(&long, "at_to_app", "app_to_at", AtLogLevel::Error, false)
                    .is_err()
            );
        }

        /// The log-level guard in C rejects out-of-range values rather than
        /// clamping, and cannot be reached from here at all: `AtLogLevel` makes an
        /// invalid level unrepresentable. Recorded as a property, not a test —
        /// the C guard still earns its place for callers in other languages.
        #[test]
        fn the_log_level_scale_is_the_one_c_documents() {
            assert_eq!(AtLogLevel::Debug as i32, 1);
            assert_eq!(AtLogLevel::Info as i32, 2);
            assert_eq!(AtLogLevel::Warning as i32, 3);
            assert_eq!(AtLogLevel::Error as i32, 4);
            assert_eq!(AtLogLevel::Critical as i32, 5);
        }

        /// And the bound case, so the drop above is not passing vacuously.
        #[test]
        fn a_stamp_on_a_bound_node_survives_the_trip() {
            let (mut src, q, _guard) = bound("stamp-ok");
            emit_peer(&q, &[0x12; 16], &[0x23; 32], 0, true, 1_721_800_000.0);
            let events = src.poll().expect("poll");
            let p = only_peer(&events);
            assert!(p.operator_bound);
            assert_eq!(
                p.attended_at(),
                Some(en_core::Timestamp::from_second(1_721_800_000).unwrap())
            );
        }

        /// D15's source, end to end over the real socket: an opted-in node's
        /// guardian key survives AT's emitter, the flat ABI and this decoder,
        /// and derives a `did:key` a charter can name. Until AT's operator-key
        /// slice this was unreachable and the field was hard-coded `None`.
        #[test]
        fn a_guardian_key_survives_the_trip() {
            let (mut src, q, _guard) = bound("guardian-ok");
            emit_peer_guarded(
                &q,
                &[0x31; 16],
                &[0x32; 32],
                0,
                true,
                1_721_800_000.0,
                Some(&[0xA1; 32]),
            );
            let events = src.poll().expect("poll");
            let p = only_peer(&events);
            assert_eq!(p.guardian_pubkey.as_deref(), Some(&[0xA1u8; 32][..]));
            let g = p.guardian_did().expect("derivable").expect("present");
            assert_ne!(g, p.did().expect("node did"));
        }

        /// **The control.** A node that declined crosses the same wire with
        /// everything else intact and only the guardian absent. This is the one
        /// that would catch opting out becoming expensive.
        #[test]
        fn an_opted_out_node_crosses_intact_without_a_guardian() {
            let (mut src, q, _guard) = bound("guardian-optout");
            emit_peer(&q, &[0x41; 16], &[0x42; 32], 7, true, 1_721_800_000.0);
            let events = src.poll().expect("poll");
            let p = only_peer(&events);
            assert!(p.operator_bound);
            assert!(p.attended_at().is_some());
            assert_eq!(p.rank, 7);
            assert_eq!(p.guardian_pubkey, None);
            assert_eq!(p.guardian_did().expect("no error"), None);
        }

        /// **The whole slice, in one assertion.** A live AT daemon's opted-in
        /// peer becomes a `MemberCandidate` carrying a guardian DID — which is
        /// what D15 charters an authority on, and what has had no live source
        /// since the tier was built. The provider-level guardian test beside
        /// this one drives a hand-built `AtPeer` through `MockEventSource`; the
        /// point here is that the same thing now arrives off the socket.
        #[test]
        fn an_opted_in_peer_reaches_a_member_candidate_as_a_guardian_edge() {
            let (src, q, _guard) = bound("guardian-candidate");
            emit_peer_guarded(
                &q,
                &[0x61; 16],
                &[0x62; 32],
                0,
                true,
                1_721_800_000.0,
                Some(&[0xA1; 32]),
            );
            // A plain unguarded machine alongside it, so the assertion is that
            // the guardian edge VARIES with the opt-in rather than being set
            // for everyone.
            emit_peer(&q, &[0x63; 16], &[0x64; 32], 0, false, 0.0);
            emit_rep(&q, &[0x61; 16], 0.8, true);
            emit_rep(&q, &[0x63; 16], 0.8, true);

            // `founding_candidate` is the SubstrateProvider trait's, not an
            // inherent method — the tier's whole point is that ethne consumes
            // AT through that trait and nothing narrower.
            use en_substrate::SubstrateProvider;

            let mut provider = crate::AtSubstrateProvider::new(src);
            provider.refresh().expect("refresh");
            let fc = provider.founding_candidate().expect("candidate");

            let node_did = crate::event::signing_key_did(&[0x62; 32]).unwrap();
            let guardian_did = crate::event::signing_key_did(&[0xA1; 32]).unwrap();
            let opted_in = fc.members.iter().find(|m| m.did == node_did).unwrap();
            assert_eq!(opted_in.guardian.as_ref(), Some(&guardian_did));

            let plain_did = crate::event::signing_key_did(&[0x64; 32]).unwrap();
            let plain = fc.members.iter().find(|m| m.did == plain_did).unwrap();
            assert!(plain.guardian.is_none());
        }

        /// The far end of the double gate: AT already refuses this, and so does
        /// this decoder, so the guarantee holds even if the two are ever built
        /// from different revisions.
        #[test]
        fn a_guardian_key_on_an_unbound_node_is_dropped_at_this_end() {
            let (mut src, q, _guard) = bound("guardian-unbound");
            emit_peer_guarded(&q, &[0x51; 16], &[0x52; 32], 0, false, 0.0, Some(&[0xA1; 32]));
            let events = src.poll().expect("poll");
            let p = only_peer(&events);
            assert!(!p.operator_bound);
            assert_eq!(p.guardian_pubkey, None);
        }

        extern "C" {
            /// `sig` 0 sends nothing and only asks whether a pid exists.
            fn kill(pid: c_int, sig: c_int) -> c_int;
        }

        /// Kill processes AT orphaned, and return how many there were.
        ///
        /// A stopped daemon leaves its subsystem processes running, reparented to
        /// init — 8 per run, measured, and filed as autonomous_trust ISSUES.md
        /// §2.1.3. They are harmless to the assertions (a log file keeps them off
        /// this process's stderr) but a test that leaks eight processes every run
        /// is not one to leave in a suite.
        ///
        /// The handle is exact rather than a name match: AT forks without execing,
        /// so an orphan's `/proc/<pid>/exe` is **this test binary**. Nothing else
        /// can be that and have init for a parent. A `pkill AutonomousTrust` would
        /// have been shorter and would kill a developer's real node.
        fn kill_orphans_of_this_binary() -> usize {
            let me = std::process::id();
            let Ok(exe) = std::env::current_exe() else {
                return 0;
            };
            let Ok(entries) = std::fs::read_dir("/proc") else {
                return 0;
            };
            let mut killed = 0;
            for entry in entries.flatten() {
                let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
                    continue;
                };
                if pid == me {
                    continue;
                }
                // Same executable image...
                if std::fs::read_link(format!("/proc/{pid}/exe")).ok().as_ref() != Some(&exe) {
                    continue;
                }
                // ...and orphaned. Field 4 of `stat` is ppid; the comm in field 2
                // can contain spaces, so parse after its closing paren.
                let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
                    continue;
                };
                let Some(rest) = stat.rsplit_once(") ").map(|(_, r)| r) else {
                    continue;
                };
                if rest.split_whitespace().nth(1) != Some("1") {
                    continue;
                }
                // SAFETY: a plain kill on a pid we have just identified as an
                // orphaned fork of this very binary.
                unsafe { kill(pid as c_int, 9) };
                killed += 1;
            }
            killed
        }

        /// **The leg that was never covered: a real daemon.**
        ///
        /// Every other test here plays daemon over the test-support emitters, which
        /// covers the decoder and the wire and cannot cover the lifecycle. This
        /// starts an actual `autonomous_trust` process, waits for it to become
        /// reachable, reads from it, and stops it.
        ///
        /// It was blocked, and the block was real rather than environmental: AT
        /// ISSUES.md §2.1.1, a stack overflow in socket-path construction that
        /// aborted *this* process from inside the FFI call. Now fixed.
        ///
        /// **The assertions are on the lifecycle, not on event content.** A
        /// solitary node with no peers emits nothing, so `assert_eq!(events, 0)`
        /// would be asserting nothing at all; content is covered by the five
        /// end-to-end emitter tests above. What is worth asserting is the sequence
        /// that used to be unobservable: not-ready → ready, alive throughout, a
        /// reachable and empty stream, a roster pull that is refused before ready
        /// and accepted after, and a clean shutdown.
        ///
        /// Three hazards, each measured rather than guessed:
        ///
        /// 1. **The root must be short and must be ours.** After the §2.1.1 fix an
        ///    over-long root is a *refusal*, and this suite's own `temp_dir()` may
        ///    be a long scratchpad path — so a test that inherited it would fail to
        ///    start and, if it treated that as "skip", would be indistinguishable
        ///    from one that passed. The length is asserted below for that reason.
        /// 2. **The socket lock**, as for every other test here: AT's messaging
        ///    layer keeps process-global state and Rust runs tests in parallel.
        /// 3. **The daemon's stdio must not be this process's stderr** — see
        ///    [`EmbeddedAtNode::start_logging_to`]. This is the ten-minute hang.
        #[test]
        fn a_live_daemon_starts_becomes_ready_and_stops() {
            let _guard = socket_lock();

            // Literal "/tmp", not `temp_dir()`: TMPDIR may point somewhere long,
            // and the point is a root we know fits.
            let root = std::path::Path::new("/tmp/en-at-live");
            let root_len = root.as_os_str().len();
            assert!(
                root_len <= 90,
                "root is {root_len} bytes; over 90 AT refuses to build a socket path \
                 (ISSUES.md §2.1.1), and this test would be measuring the refusal"
            );

            let _ = std::fs::remove_dir_all(root);
            std::fs::create_dir_all(root.join("var/at")).expect("queue dir");
            std::env::set_var("AUTONOMOUS_TRUST_ROOT", root);
            let log = root.join("daemon.log");

            let mut node = EmbeddedAtNode::start_logging_to(
                "en_at_live",
                "at_to_app_live",
                "app_to_at_live",
                AtLogLevel::Info,
                true, // generate the configs; the root was just emptied
                Some(log.to_str().expect("ascii path")),
            )
            .expect("a live daemon starts — if this fails, read /tmp/en-at-live/daemon.log");

            // Alive immediately, and NOT yet reachable. The margin is what makes
            // this sound rather than racy: `start` returns in ~2 ms and the queue
            // is not bound for ~210 ms, two orders of magnitude apart.
            assert!(node.alive(), "forked");
            assert!(
                !node.ready(),
                "a just-forked daemon has not bound its queue yet — if this fires, \
                 either the daemon got much faster or `ready` is reporting `alive`"
            );
            // And the pull that motivated all of this is refused, by name.
            let refused = node.request_roster().expect_err("not ready yet");
            assert!(
                format!("{refused}").contains("not ready"),
                "the refusal should say WHY: {refused}"
            );

            // 10 s against a measured ~210 ms: generous enough not to be a timing
            // test, tight enough to fail rather than hang.
            let waited = std::time::Instant::now();
            assert!(
                node.wait_ready(std::time::Duration::from_secs(10)),
                "daemon never became reachable; log at {}",
                log.display()
            );
            let elapsed = waited.elapsed();
            assert!(node.ready(), "still ready once waited for");
            assert!(node.alive(), "and still running");

            // Reachable and empty: `poll` must succeed, and a lone node with no
            // peers has nothing to say. Ok-with-nothing is the assertion; a count
            // would be asserting noise.
            for _ in 0..5 {
                node.poll().expect("the stream is reachable");
            }

            // Now accepted, where it was refused a moment ago.
            node.request_roster()
                .expect("a roster pull goes through once the daemon is bound");

            let pid = node.pid();
            assert!(pid > 0, "a real pid");
            drop(node);

            // **Gone, not merely signalled.** This assertion found a defect and is
            // the reason `at_node_shutdown` now waits: it used to call `waitpid` on
            // a daemon that `daemonize`'s double fork had already given to init, so
            // waitpid returned ECHILD at once and `stop` returned with the daemon
            // still running and still holding its sockets. Measured, a full
            // shutdown takes ~5.3 s, so "stop then immediately start another node"
            // was racing a live process for the socket path.
            //
            // (`kill(pid, 0)` could in principle hit a recycled pid — not in the
            // microseconds between the daemon's exit and this line.)
            // SAFETY: signal 0 sends nothing and only tests for existence.
            let gone = unsafe { kill(pid, 0) };
            assert_eq!(gone, -1, "daemon {pid} outlived at_app_node_stop");

            // The daemon's own children are a different matter, and not fixed
            // here — see the helper, and ISSUES.md §2.1.3.
            let orphans = kill_orphans_of_this_binary();
            eprintln!(
                "live daemon pid {pid} became ready in {elapsed:?}; \
                 cleaned up {orphans} subsystem process(es) AT left behind"
            );
        }
    }
}
