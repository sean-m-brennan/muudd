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

/// The real autonomous_trust C linkage (decision **D6**: both coupling modes),
/// compiled only under the `at-ffi` feature — a deployment/integration build,
/// never the sandbox. The bodies bind `libautonomous_trust` via `extern "C"`
/// (`node.h` lifecycle + `message.h` `messaging_recv_on`) and decode the AT→app
/// message stream into [`AtEvent`]s.
#[cfg(feature = "at-ffi")]
pub mod ffi {
    use super::*;

    /// **Embed** an AT node: this process owns the daemon lifecycle
    /// (`at_node_init` / `at_node_start` / `at_node_run` / `at_node_shutdown`)
    /// and reads its own out-queue. Self-contained for standalone tools/tests.
    pub struct EmbeddedAtNode {
        // Owns the `at_node_config_t` + queue handles; see node.h.
    }

    impl AtEventSource for EmbeddedAtNode {
        fn poll(&mut self) -> KcResult<Vec<AtEvent>> {
            // TODO(at-ffi): messaging_recv_on(self.q_out, ...) -> decode -> AtEvent.
            unimplemented!("at-ffi embedded coupling: bind libautonomous_trust")
        }
    }

    /// **Attach** to an already-running daemon: read an existing `q_out`
    /// (`messaging_recv_on`) without owning the lifecycle. Fits a co-deployed
    /// fleet where the AT daemon is already up.
    pub struct AttachedAtDaemon {
        // Holds a borrowed queue_t handle to the running daemon's out-queue.
    }

    impl AtEventSource for AttachedAtDaemon {
        fn poll(&mut self) -> KcResult<Vec<AtEvent>> {
            // TODO(at-ffi): messaging_recv_on(existing q_out, ...) -> AtEvent.
            unimplemented!("at-ffi attached coupling: bind libautonomous_trust")
        }
    }
}
