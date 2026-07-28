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

//! A reference host for a polity: the loop, and the one clock in the tier.
//!
//! Every cadence in ethne is a query-time parameter, which is what makes the tier's
//! judgements reproducible: no crate that *decides* anything reads a clock.
//! **This binary is where that stops.** It is not a library, nothing depends on it,
//! and the single `time::now()` in its loop is the only clock read in ethne outside
//! a test — save for `en-pyo3`'s `*_at=None ⇒ now` convenience, which is a binding
//! affordance for Python callers rather than a judgement made from the time.
//!
//! Copy it and adapt it; it is deliberately short enough to read in one sitting.
//!
//! ```text
//! polity_host                 # a simulated year, exits 0
//! polity_host --ticks 30 --step 3600
//! polity_host --follow        # the real loop: wall clock, sleeps
//! ```

use std::path::PathBuf;

use en_archive::Archive;
use en_charter::{
    apply_prunings, majority, prunable_observations, pruning_digest, Affirmation, AffirmationKind,
    BoundarySource,
    CharterEntry, Commitment, FoundingProposal, MemberReading, Observation, ObservationPruning,
    Substrate,
};
use en_core::{keys, time, Alg, DidKey, KcResult, PrivKey, Signed, Timestamp};
use en_host::{DueAction, HostConfig, HostCycle};
use en_store::{ArchiveStore, FileStore};
use en_substrate::{FoundingCandidate, MemberCandidate};
use en_uplift::coherence_concern;

const DAY: i64 = 86_400;

struct Args {
    ticks: u32,
    step: i64,
    start: i64,
    follow: bool,
    path: PathBuf,
}

impl Args {
    fn parse() -> Self {
        let mut a = Args {
            ticks: 40,
            step: 10 * DAY,
            start: 200 * DAY,
            follow: false,
            path: std::env::temp_dir().join("polity_host_demo.log"),
        };
        let argv: Vec<String> = std::env::args().skip(1).collect();
        let mut i = 0;
        while i < argv.len() {
            match argv[i].as_str() {
                "--follow" => a.follow = true,
                "--ticks" => { i += 1; a.ticks = argv[i].parse().expect("--ticks N") }
                "--step" => { i += 1; a.step = argv[i].parse().expect("--step SECONDS") }
                "--start" => { i += 1; a.start = argv[i].parse().expect("--start SECONDS") }
                "--path" => { i += 1; a.path = PathBuf::from(&argv[i]) }
                "--help" | "-h" => {
                    println!("polity_host [--ticks N] [--step S] [--start S] [--path P] [--follow]");
                    std::process::exit(0);
                }
                other => panic!("unknown argument: {other}"),
            }
            i += 1;
        }
        a
    }
}

/// The keys a host holds. In a real deployment these come from custody the host
/// arranges; that they are generated here is the one thing this example fakes.
struct Keys {
    polity: DidKey,
    members: Vec<(PrivKey, DidKey)>,
    monitor: (PrivKey, DidKey),
    observer: (PrivKey, DidKey),
}

fn id() -> (PrivKey, DidKey) {
    let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
    (sk, DidKey::from_pubkey(&pk).unwrap())
}

fn substrate() -> Substrate {
    Substrate {
        name: "the long watch".into(),
        statement: "answers to what it did not author".into(),
        anchor: None,
    }
}

impl Keys {
    fn new(n: usize) -> Self {
        Keys {
            polity: id().1,
            members: (0..n).map(|_| id()).collect(),
            monitor: id(),
            observer: id(),
        }
    }

    fn roll(&self) -> Vec<DidKey> {
        self.members.iter().map(|(_, d)| d.clone()).collect()
    }

    /// A bare majority of the roll — what ratification and pruning both need.
    fn deciders(&self) -> Vec<DidKey> {
        self.roll().into_iter().take(majority(self.members.len())).collect()
    }

    fn decider_signers(&self) -> Vec<(&str, &PrivKey)> {
        self.members
            .iter()
            .take(majority(self.members.len()))
            .map(|(sk, did)| (did.as_str(), sk))
            .collect()
    }

    /// Every member renews their hold on the substrate. Self-signed: transmission is
    /// something a member does, not something done to them.
    fn reaffirmations(&self, at: Timestamp) -> KcResult<Vec<CharterEntry>> {
        self.members
            .iter()
            .map(|(sk, did)| {
                let record = Affirmation {
                    polity: self.polity.clone(),
                    member: did.clone(),
                    kind: AffirmationKind::ReAffirmation,
                    witnesses: vec![],
                    affirmed_at: at,
                };
                Ok(CharterEntry::MemberAffirmed(Signed::build(record, &[(did.as_str(), sk)])?))
            })
            .collect()
    }

    fn found(&self, at: Timestamp) -> KcResult<CharterEntry> {
        let prop = FoundingProposal::convener_asserted(
            self.polity.clone(),
            substrate(),
            vec![Commitment {
                key: "mutual-aid".into(),
                description: "we answer for one another".into(),
            }],
            self.roll(),
            vec![],
            at,
        );
        let charter = prop.ratify(self.deciders(), at)?;
        let signers = self.decider_signers();
        Ok(CharterEntry::PolityChartered(Signed::build(charter, &signers)?))
    }

    fn observation(&self, at: Timestamp) -> KcResult<CharterEntry> {
        let record = Observation {
            polity: self.polity.clone(),
            observer: self.monitor.1.clone(),
            observed_at: at,
            readings: self
                .roll()
                .into_iter()
                .map(|member| MemberReading {
                    member,
                    reputation: 0.9,
                    guardian: None,
                    attested_at: Some(at),
                })
                .collect(),
            opaque_boundary: false,
        };
        let signers = vec![(self.monitor.1.as_str(), &self.monitor.0)];
        Ok(CharterEntry::Observed(Signed::build(record, &signers)?))
    }

    /// Author a pruning at `horizon` on the roll-majority route.
    fn pruning(
        &self,
        archive: &Archive<CharterEntry>,
        horizon: Timestamp,
        at: Timestamp,
    ) -> KcResult<CharterEntry> {
        let dropped = prunable_observations(archive, &self.polity, horizon);
        let record = ObservationPruning {
            polity: self.polity.clone(),
            horizon,
            dropped: dropped.len(),
            digest: pruning_digest(&dropped)?,
            rationale: "retention policy".into(),
            deciders: self.deciders(),
            pruned_at: at,
        };
        let signers = self.decider_signers();
        Ok(CharterEntry::ObservationsPruned(Signed::build(record, &signers)?))
    }

    fn candidate(&self) -> FoundingCandidate {
        FoundingCandidate {
            members: self
                .roll()
                .into_iter()
                .map(|did| MemberCandidate {
                    did,
                    reputation: Some(0.9),
                    derived: false,
                    guardian: None,
                    attested_at: None,
                })
                .collect(),
            substrate: Some(substrate()),
            source: BoundarySource::Substrate,
        }
    }
}

/// Carry out one action. **This is the half `tick` deliberately does not do**: every
/// arm here either touches storage or uses a signing key.
fn perform(
    action: DueAction,
    keys: &Keys,
    archive: &mut Archive<CharterEntry>,
    store: &mut FileStore,
    as_of: Timestamp,
) -> KcResult<String> {
    Ok(match action {
        DueAction::Save => {
            let out = store.save(archive)?;
            if !out.wrote_anything() {
                "save: nothing new".to_string()
            } else if out.rewrote {
                format!("save: REWROTE {} entries ({} bytes) — the log was compacted",
                        out.appended, out.bytes)
            } else {
                format!("save: appended {} ({} bytes)", out.appended, out.bytes)
            }
        }
        DueAction::Cohesion(r) => {
            format!("cohesion: {:?} — {} of {} observed, {} lapsed on affirming",
                    r.erosion, r.observed, r.chartered, r.stale_affirmations)
        }
        DueAction::RaiseConcern(r) => {
            // The host signs it; `tick` only said it was warranted.
            let concern = coherence_concern(
                &r,
                keys.polity.clone(),
                keys.observer.1.clone(),
                &keys.observer.0,
                as_of,
            )?;
            format!("CONCERN raised to kith-covenant ({} bytes of signed record)",
                    concern.map(|c| serde_json_len(&c)).unwrap_or(0))
        }
        DueAction::Prune(rec) => {
            // The recommendation names a horizon; the polity ratifies it.
            archive.append(keys.pruning(archive, rec.horizon, as_of)?)?;
            *archive = apply_prunings(archive, &keys.polity, as_of)?;
            // The store must be told; its own generation check turns this into a
            // full rewrite rather than an append. That is D20's whole point.
            let out = store.save(archive)?;
            format!("pruned {} observations (keeping {}d); store {} {} bytes",
                    rec.dropped,
                    rec.seconds_retained / DAY,
                    if out.rewrote { "REWROTE" } else { "appended" },
                    out.bytes)
        }
    })
}

fn serde_json_len<T: serde::Serialize>(v: &T) -> usize {
    serde_json::to_string(v).map(|s| s.len()).unwrap_or(0)
}

fn main() -> KcResult<()> {
    let args = Args::parse();
    let keys = Keys::new(3);
    let mut store = FileStore::new(&args.path);
    let _ = std::fs::remove_file(&args.path); // a fresh demo each run

    println!("polity host — storage at {}", args.path.display());
    println!("{}", if args.follow {
        "following the wall clock (Ctrl-C to stop)".to_string()
    } else {
        format!("simulating {} ticks of {}d from t={}d",
                args.ticks, args.step / DAY, args.start / DAY)
    });

    // --- found, and persist the founding before anything else ----------------
    let mut archive: Archive<CharterEntry> = store.load()?;
    if archive.is_empty() {
        archive.append(keys.found(Timestamp::from_second(100).unwrap())?)?;
        en_charter::check_authority(&archive)?;
        let out = store.save(&archive)?;
        println!("founded and persisted ({} bytes)\n", out.bytes);
    }

    let mut cycle = HostCycle::new(HostConfig::new(keys.polity.clone()));
    let mut compactions = 0;

    // --- the loop ------------------------------------------------------------
    for n in 0..args.ticks {
        // THE ONE CLOCK IN THE TIER. Everything below this line is given a time
        // rather than reading one, which is why `en-host` is unit-testable at any
        // moment and this file is not.
        let as_of = if args.follow {
            time::now()
        } else {
            Timestamp::from_second(args.start + (n as i64) * args.step).unwrap()
        };

        // What the substrate reported since the last tick, recorded as evidence. A
        // real monitor reports far faster than the host wakes, so a tick folds in
        // the interval's worth — which is also what makes the log grow enough for
        // retention to have anything to say.
        let per_tick = (args.step / DAY).clamp(1, 24);
        for k in 0..per_tick {
            let at = Timestamp::from_second(as_of.as_second() - (per_tick - 1 - k) * DAY).unwrap();
            archive.append(keys.observation(at)?)?;
        }

        // And the members renew their hold on the substrate well inside the
        // affirmation cadence, so the polity is a healthy one rather than one that
        // has quietly stopped. (Drop this and D13's transmission decay reaches
        // Concern on its own within a year — which it should.)
        if n % 10 == 0 {
            for entry in keys.reaffirmations(as_of)? {
                archive.append(entry)?;
            }
        }

        for action in cycle.tick(&archive, &keys.candidate(), as_of) {
            let was_prune = matches!(action, DueAction::Prune(_));
            let line = perform(action, &keys, &mut archive, &mut store, as_of)?;
            // Steady state is the boring majority; print it periodically and let the
            // events that actually change something always show.
            let notable = was_prune || line.contains("CONCERN") || line.contains("REWROTE");
            if notable || n % 10 == 0 {
                println!("  t={:>5}d  {line}", as_of.as_second() / DAY);
            }
            if was_prune {
                compactions += 1;
            }
        }

        if args.follow {
            std::thread::sleep(std::time::Duration::from_secs(args.step.max(1) as u64));
        }
    }

    // --- restart: what a host actually depends on ----------------------------
    //
    // The check D20 exists for. Reload from storage alone and demand it match the
    // live log exactly. Before this slice the two silently diverged the moment a
    // pruning happened, and every verification pass still said the reloaded log was
    // fine.
    println!("\n--- restart ------------------------------------------------------");
    let mut reader = FileStore::new(&args.path);
    let reloaded: Archive<CharterEntry> = reader.load()?;
    let live: Vec<String> = archive.iter().map(|(c, _)| c.to_string()).collect();
    let back: Vec<String> = reloaded.iter().map(|(c, _)| c.to_string()).collect();

    println!("live    : {} entries, generation {}", archive.len(), archive.generation());
    println!("reloaded: {} entries, generation {}", reloaded.len(), reloaded.generation());
    println!("{} compaction(s) survived the round trip", compactions);

    en_charter::verify_all(&reloaded)?;
    en_charter::check_authority(&reloaded)?;
    assert_eq!(live, back, "storage and the live log must agree, CID for CID");
    println!("signatures verify, authority holds, and the two agree CID for CID");

    let _ = std::fs::remove_file(&args.path);
    Ok(())
}
