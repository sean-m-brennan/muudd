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

//! Two polities recognized into one kith-covenant compact, end to end.
//!
//! ```text
//! recognition_handoff        # exits 0
//! ```
//!
//! What it shows, in the order the two tiers meet:
//!
//! 1. Each polity founds itself, confers an Inductor, authorizes an envoy, and
//!    performs its own five-movement rite. All of that is ethne's, and KC never sees
//!    the charter log.
//! 2. The border emits what KC recognizes: a `CommunityIdentity`, the envoy
//!    `SpeakerDelegation`, a `MemberRoll`, and the intra rite, whose Movement 4 is the
//!    polity's own commitment list, key for key.
//! 3. KC seals each rite and admits both polities to a compact through its
//!    inter-community ceremony, with the envoy as the cross-boundary signer.
//! 4. KC's recited-object reading then grades the compact against the union of what
//!    the polities undertook at their own tier, which is the thing neither tier could
//!    say on its own.

use std::process::ExitCode;

use en_core::{KcResult, Timestamp};
use kc_archive::InMemoryArchive;
use kc_reputation::{commitment_coverage, compact_object};
use recognition_handoff::{admit_to_compact, found_polity, hand_off, seal_rite};

const YEAR: i64 = 365 * 86_400;

fn run() -> KcResult<()> {
    // A fixed clock: this is a demonstration, and a demonstration that drifts with the
    // wall clock is a demonstration of the wall clock.
    let founding = Timestamp::from_second(1_700_000_000)
        .map_err(|e| en_core::KcError::Archive(e.to_string()))?;
    let recognition = Timestamp::from_second(1_700_000_100)
        .map_err(|e| en_core::KcError::Archive(e.to_string()))?;

    println!("--- The polities, at their own tier -------------------------------");
    let alpha = found_polity(
        "Alpha",
        3,
        &[
            ("mutual-aid", "we answer for one another"),
            ("truthfulness", "we tell the truth about what we have done"),
        ],
        founding,
    )?;
    let beta = found_polity(
        "Beta",
        3,
        &[
            ("mutual-aid", "we answer for one another"),
            ("hospitality", "we receive the stranger"),
        ],
        founding,
    )?;
    for (name, p) in [("Alpha", &alpha), ("Beta", &beta)] {
        println!(
            "  {name}: {} member(s), Inductor {}, envoy {}",
            p.members.len(),
            short(&p.inductor.to_string()),
            short(&p.envoy.0.to_string())
        );
    }

    println!("\n--- What crosses the border ---------------------------------------");
    let mut kc = InMemoryArchive::new();
    for (name, p) in [("Alpha", &alpha), ("Beta", &beta)] {
        let reading = hand_off(p, &mut kc, founding)?;
        println!(
            "  {name}: {} with {} commitment(s) in force: {}",
            reading.appended.join(", "),
            reading.commitments.len(),
            reading.commitments.join(", ")
        );
    }

    println!("\n--- The rite KC holds ---------------------------------------------");
    let a_rite = seal_rite(&alpha, &mut kc, recognition)?;
    let b_rite = seal_rite(&beta, &mut kc, recognition)?;
    for (name, rite) in [("Alpha", &a_rite), ("Beta", &b_rite)] {
        let recitation = &rite.payload.recitation.payload;
        println!(
            "  {name}: recited by the Inductor {}, witnessed by {}, Movement 4 = {:?}",
            short(&recitation.speaker_did.to_string()),
            rite.payload.internal_witnesses.len(),
            recitation.movement_common_virtue.keys()
        );
    }
    println!("  The commitment keys are the polities' own. Neither tier translated them:");
    println!("  ethne's Commitment {{ key, description }} and KC's {{ key, text }} are");
    println!("  the same shape, so what the polity bound itself to is what KC reads.");

    println!("\n--- The compact ---------------------------------------------------");
    let sealed = admit_to_compact(
        "polity-compact",
        &[(&alpha, &a_rite), (&beta, &b_rite)],
        &mut kc,
        recognition,
    )?;
    println!(
        "  sealed: {} participant(s), {} sequenced rite(s), {} claim(s), {} acceptance(s)",
        sealed.payload.participants.len(),
        sealed.payload.sequenced_rites.len(),
        sealed.payload.claims.len(),
        sealed.payload.acceptances.len()
    );
    kc_archive::verify_all(&kc)?;
    kc_archive::check_authority(
        &kc,
        2,
        3,
        Default::default(),
        Default::default(),
        Default::default(),
    )?;
    println!("  the KC archive verifies and authorizes");

    println!("\n--- What the compact now holds ------------------------------------");
    let object = compact_object(&kc, Some(recognition));
    println!(
        "  recited object: {} commitment(s) across {} polit(ies): {}",
        object.size(),
        object.per_community.len(),
        object.keys.join(", ")
    );
    let coverage = commitment_coverage(&kc, Some(recognition), 5 * YEAR);
    println!(
        "  enforcement so far: {} act(s), so the reading is {} rather than a narrowing",
        coverage.acts,
        if coverage.testable(&Default::default()) { "testable" } else { "untestable" }
    );
    println!("  A compact that later stops enforcing part of that object is read against");
    println!("  what its polities undertook, not against a taxonomy the protocol fixed.");

    println!("\n✓ recognition_handoff: SUCCESS");
    Ok(())
}

fn short(did: &str) -> String {
    did.chars().rev().take(6).collect::<String>().chars().rev().collect()
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("recognition_handoff failed: {e}");
            ExitCode::FAILURE
        }
    }
}
