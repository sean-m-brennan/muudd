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

//! The loop's decisions, driven over a **simulated** timeline.
//!
//! That this file exists at all is the argument for splitting `en-host` from the
//! binary: every one of these runs a year of host behaviour in microseconds, at
//! times chosen to land exactly on cadence boundaries. None of it would be reachable
//! if `tick` read a clock.

use en_archive::Archive;
use en_charter::{
    Commitment, FoundingProposal, MemberReading, Observation, ObservationPruning, PolityCharter,
    Substrate,
};
use en_core::{keys, Alg, DidKey, PrivKey, Signed, Timestamp};
use en_cohesion::ErosionLevel;
use en_substrate::{FoundingCandidate, MemberCandidate};

use crate::{DueAction, HostConfig, HostCycle};

const DAY: i64 = 86_400;

fn t(s: i64) -> Timestamp {
    Timestamp::from_second(s).unwrap()
}

fn id() -> (PrivKey, DidKey) {
    let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
    (sk, DidKey::from_pubkey(&pk).unwrap())
}

fn substrate() -> Substrate {
    Substrate { name: "tradition".into(), statement: "answers to what it did not author".into(), anchor: None }
}

struct Fixture {
    polity: DidKey,
    ids: Vec<(PrivKey, DidKey)>,
    /// Three of them, rotated by `observe` (**D28**): the retention floor counts
    /// distinct watchers, so a lone monitor filing repeatedly pins the horizon and
    /// no `Prune` ever comes due.
    monitors: Vec<(PrivKey, DidKey)>,
    observed: usize,
    archive: Archive<CharterEntry>,
}

use en_charter::CharterEntry;

impl Fixture {
    /// A chartered polity of `n` members, founded at t=100.
    fn new(n: usize) -> Self {
        let polity = id().1;
        let ids: Vec<(PrivKey, DidKey)> = (0..n).map(|_| id()).collect();
        let roll: Vec<DidKey> = ids.iter().map(|(_, d)| d.clone()).collect();
        let prop = FoundingProposal::convener_asserted(
            polity.clone(),
            substrate(),
            vec![Commitment {
                key: "mutual-aid".into(),
                description: "we answer for one another".into(),
            }],
            roll.clone(),
            vec![],
            t(50),
        );
        // The WHOLE roll ratifies. Since D29 that is what inducts them, so a
        // fixture meaning "a healthy polity" has to say so: a listed founder who
        // never signed is un-inducted, and transmission decay would grade this
        // polity `Notice` for a reason none of these tests are about.
        let charter: PolityCharter = prop.ratify(roll.clone(), t(100)).unwrap();
        let signers: Vec<(&str, &PrivKey)> =
            ids.iter().map(|(k, d)| (d.as_str(), k)).collect();
        let mut archive = Archive::new();
        archive
            .append(CharterEntry::PolityChartered(Signed::build(charter, &signers).unwrap()))
            .unwrap();
        let monitors = (0..3).map(|_| id()).collect();
        Fixture { polity, ids, monitors, observed: 0, archive }
    }

    /// A chartered polity of `n` members grounded on a consenting substrate anchor,
    /// which co-signs the founding (design **D23**) — the only kind of polity whose
    /// ground can later withdraw. Returns the anchor's identity with it.
    fn new_anchored(n: usize) -> (Self, (PrivKey, DidKey)) {
        let ground = id();
        let mut f = Self::new(n);
        let sub = Substrate {
            name: "the operators".into(),
            statement: "answers to what it did not author".into(),
            anchor: Some(ground.1.clone()),
        };
        let roll = f.roll();
        let prop = FoundingProposal::convener_asserted(
            f.polity.clone(),
            sub,
            vec![],
            roll.clone(),
            vec![],
            t(50),
        );
        let charter: PolityCharter = prop.ratify(roll.clone(), t(100)).unwrap();
        let mut signers: Vec<(&str, &PrivKey)> =
            f.ids.iter().map(|(k, d)| (d.as_str(), k)).collect();
        signers.push((ground.1.as_str(), &ground.0));
        f.archive = Archive::new();
        f.archive
            .append(CharterEntry::PolityChartered(Signed::build(charter, &signers).unwrap()))
            .unwrap();
        (f, ground)
    }

    /// The chartered ground withdrawing.
    fn disavow(&mut self, ground: &(PrivKey, DidKey), at_secs: i64) {
        let d = en_charter::SubstrateDisavowal {
            polity: self.polity.clone(),
            disavower: ground.1.clone(),
            rationale: "I do not stand above these people".into(),
            disavowed_at: t(at_secs),
        };
        self.archive
            .append(CharterEntry::SubstrateDisavowed(
                Signed::build(d, &[(ground.1.as_str(), &ground.0)]).unwrap(),
            ))
            .unwrap();
    }

    fn roll(&self) -> Vec<DidKey> {
        self.ids.iter().map(|(_, d)| d.clone()).collect()
    }

    /// Everyone present and well-regarded — a healthy substrate reading.
    fn healthy(&self) -> FoundingCandidate {
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
        }
    }

    /// Only the first member still visible — heavy attrition.
    fn collapsing(&self) -> FoundingCandidate {
        FoundingCandidate {
            members: vec![MemberCandidate {
                did: self.ids[0].1.clone(),
                reputation: Some(0.9),
                derived: false,
                guardian: None,
                attested_at: None,
            }],
            substrate: Some(substrate()),
        }
    }

    fn observe(&mut self, at: i64) {
        let m = &self.monitors[self.observed % self.monitors.len()];
        self.observed += 1;
        let record = Observation {
            polity: self.polity.clone(),
            observer: m.1.clone(),
            observed_at: t(at),
            readings: vec![MemberReading {
                member: self.ids[2].1.clone(),
                reputation: 0.9,
                guardian: None,
                attested_at: None,
            }],
            opaque_boundary: false,
        };
        let signers = vec![(m.1.as_str(), &m.0)];
        self.archive
            .append(CharterEntry::Observed(Signed::build(record, &signers).unwrap()))
            .unwrap();
    }

    /// Prune by a majority of the roll, and compact — what acting on a `Prune`
    /// recommendation looks like.
    fn prune(&mut self, horizon: Timestamp, at: i64) {
        let dropped = en_charter::prunable_observations(&self.archive, &self.polity, horizon);
        let record = ObservationPruning {
            polity: self.polity.clone(),
            horizon,
            dropped: dropped.len(),
            digest: en_charter::pruning_digest(&dropped).unwrap(),
            rationale: "retention policy".into(),
            deciders: vec![self.ids[0].1.clone(), self.ids[1].1.clone()],
            pruned_at: t(at),
        };
        let signers: Vec<(&str, &PrivKey)> =
            vec![(self.ids[0].1.as_str(), &self.ids[0].0), (self.ids[1].1.as_str(), &self.ids[1].0)];
        self.archive
            .append(CharterEntry::ObservationsPruned(Signed::build(record, &signers).unwrap()))
            .unwrap();
        self.archive = en_charter::apply_prunings(&self.archive, &self.polity, t(at)).unwrap();
    }

    fn cycle(&self) -> HostCycle {
        HostCycle::new(HostConfig::new(self.polity.clone()))
    }
}

fn kinds(actions: &[DueAction]) -> Vec<&'static str> {
    actions.iter().map(DueAction::kind).collect()
}

// --- cadence ------------------------------------------------------------------

#[test]
fn everything_is_due_on_the_first_tick() {
    let f = Fixture::new(3);
    let mut c = f.cycle();
    let due = c.tick(&f.archive, &f.healthy(), t(1_000));
    assert_eq!(kinds(&due), vec!["Save", "Cohesion"], "no observations yet, so nothing to prune");
}

#[test]
fn a_duty_waits_for_its_cadence_and_then_comes_due() {
    let f = Fixture::new(3);
    let mut c = f.cycle();
    let cadence = c.config().cohesion_cadence_seconds;
    c.tick(&f.archive, &f.healthy(), t(1_000));

    // One second short.
    let due = c.tick(&f.archive, &f.healthy(), t(1_000 + cadence - 1));
    assert!(!kinds(&due).contains(&"Cohesion"));
    // Exactly on it.
    let due = c.tick(&f.archive, &f.healthy(), t(1_000 + cadence));
    assert!(kinds(&due).contains(&"Cohesion"));
}

#[test]
fn a_zero_cadence_means_every_tick() {
    let f = Fixture::new(3);
    let mut c = f.cycle();
    assert_eq!(c.config().save_cadence_seconds, 0);
    for at in [1_000, 1_001, 1_002] {
        assert!(kinds(&c.tick(&f.archive, &f.healthy(), t(at))).contains(&"Save"));
    }
}

#[test]
fn a_clock_that_goes_backwards_under_acts_rather_than_regrading() {
    let f = Fixture::new(3);
    let mut c = f.cycle();
    c.tick(&f.archive, &f.healthy(), t(10 * DAY));
    let due = c.tick(&f.archive, &f.healthy(), t(DAY));
    assert!(!kinds(&due).contains(&"Cohesion"), "the conservative direction");
}

// --- grading ------------------------------------------------------------------

#[test]
fn a_healthy_polity_grades_without_raising_a_concern() {
    let f = Fixture::new(3);
    let mut c = f.cycle();
    let due = c.tick(&f.archive, &f.healthy(), t(1_000));
    match &due[1] {
        DueAction::Cohesion(r) => assert_ne!(r.erosion, ErosionLevel::Concern),
        other => panic!("expected Cohesion, got {}", other.kind()),
    }
}

#[test]
fn collapse_comes_back_as_raise_concern_and_never_as_a_signed_record() {
    // The posture: the host is told a Concern is warranted and handed the report.
    // Signing it needs the observer's key, which `tick` does not have and must not.
    let f = Fixture::new(3);
    let mut c = f.cycle();
    let due = c.tick(&f.archive, &f.collapsing(), t(1_000));
    assert_eq!(kinds(&due), vec!["Save", "RaiseConcern"]);
    match &due[1] {
        DueAction::RaiseConcern(r) => assert_eq!(r.erosion, ErosionLevel::Concern),
        other => panic!("expected RaiseConcern, got {}", other.kind()),
    }
}

#[test]
fn exactly_one_cohesion_action_per_grade() {
    // Cohesion and RaiseConcern are the same duty at two levels, so a host matches
    // on one arm rather than having to know that a Concern also implies a grade.
    let f = Fixture::new(3);
    let mut c = f.cycle();
    let due = c.tick(&f.archive, &f.collapsing(), t(1_000));
    let n = kinds(&due).iter().filter(|k| matches!(**k, "Cohesion" | "RaiseConcern")).count();
    assert_eq!(n, 1);
}

#[test]
fn grading_supplies_every_read_model_the_archive_has() {
    // A component with no input reports nothing rather than complaining, which is
    // right for a library and wrong for a host that simply forgot to pass one. So
    // the observation-derived measure must actually be populated here.
    let mut f = Fixture::new(3);
    for i in 0..4 {
        f.observe(200 + i * 3_600);
    }
    let c = f.cycle();
    let report = c.grade(&f.archive, &f.healthy(), t(5 * DAY));
    assert!(report.observed > 0);
    // Trends and the window are wired: the window saw our four observations.
    let since = t(5 * DAY - c.config().observation_window_seconds);
    let w = en_charter::observation_window(&f.archive, &f.polity, since, t(5 * DAY));
    assert_eq!(w.reading_count(), 4);
}

#[test]
fn grade_omits_no_reading_and_says_so_itself() {
    // **D31, and the assertion this crate's reason for existing never had.** Every test
    // above checks that some particular measure came out populated, which catches a
    // forgotten input only where a fixture happens to exercise it. The report now
    // carries the answer directly, so the claim in this crate's own documentation --
    // that `grade` assembles *every* read-model the archive can supply -- is checkable
    // in one line and stays checkable as read-models are added.
    let mut f = Fixture::new(3);
    for i in 0..4 {
        f.observe(200 + i * 3_600);
    }
    let report = f.cycle().grade(&f.archive, &f.healthy(), t(5 * DAY));
    assert!(
        report.omitted_readings.is_empty(),
        "grade forgot: {:?}",
        report.omitted_readings
    );
    assert!(report.silent_measures().is_empty());
    assert!(!report.blind_to_concern(), "nothing this grade could have escalated on is dark");
}

#[test]
fn a_hand_assembled_grade_is_what_the_omission_list_is_for() {
    // The negative control, in the crate whose job is to make it unnecessary: the same
    // archive graded by hand, with the ground reading left out the way a caller off the
    // en-host path leaves it out. The level is identical to a healthy polity's, and the
    // only thing telling the two apart is the omission list.
    let (mut f, ground) = Fixture::new_anchored(3);
    f.disavow(&ground, 2 * DAY);
    let by_hand = en_cohesion::cohesion_report(
        &en_charter::charter_state(&f.archive, &f.polity, t(3 * DAY)).members,
        &f.healthy(),
        false,
        &en_cohesion::CohesionInputs::default(),
        t(3 * DAY),
        &en_cohesion::Thresholds::default(),
    );
    let by_cycle = f.cycle().grade(&f.archive, &f.healthy(), t(3 * DAY));
    assert_eq!(by_cycle.erosion, ErosionLevel::Concern);
    assert_eq!(by_hand.erosion, ErosionLevel::None, "the hole, unchanged");
    assert!(by_hand.blind_to_concern(), "and now it says so");
    assert!(by_hand.silent_measures().contains(&"ground_disavowed"));
}

#[test]
fn grading_supplies_the_ground_reading_too() {
    // The wiring most likely to rot silently: `grade` assembles the read-models, and a
    // component whose input it forgets reports health rather than complaining. So a
    // polity whose ground has actually withdrawn must come back a Concern from the
    // HOST, not merely from a hand-built `CohesionInputs`.
    let (mut f, ground) = Fixture::new_anchored(3);
    let before = f.cycle().grade(&f.archive, &f.healthy(), t(5 * DAY));
    assert!(!before.ground_disavowed);
    assert_eq!(before.erosion, ErosionLevel::None);

    f.disavow(&ground, 2 * DAY);
    let after = f.cycle().grade(&f.archive, &f.healthy(), t(5 * DAY));
    assert!(after.ground_disavowed);
    assert_eq!(after.erosion, ErosionLevel::Concern);
    // Nothing else about the polity changed: everyone is present and well-regarded.
    assert_eq!(after.observed, 3);
    assert_eq!(after.chartered, 3);
}

#[test]
fn a_withdrawn_ground_raises_the_concern_that_crosses_upward() {
    // The Concern is the artifact that reaches kith-covenant, so the host must
    // actually surface it as a duty rather than only grading it.
    let (mut f, ground) = Fixture::new_anchored(3);
    f.disavow(&ground, 2 * DAY);
    let mut c = f.cycle();
    let due = c.tick(&f.archive, &f.healthy(), t(5 * DAY));
    assert!(kinds(&due).contains(&"RaiseConcern"), "{:?}", kinds(&due));
}

// --- retention ----------------------------------------------------------------

#[test]
fn pruning_comes_due_only_once_there_is_something_worth_dropping() {
    let mut f = Fixture::new(3);
    // A year of observation, well past the 30-day retention horizon.
    for i in 0..150 {
        f.observe(DAY + i * DAY);
    }
    let mut c = f.cycle();
    let due = c.tick(&f.archive, &f.healthy(), t(400 * DAY));
    assert!(kinds(&due).contains(&"Prune"));
    match due.iter().find(|a| matches!(a, DueAction::Prune(_))).unwrap() {
        DueAction::Prune(rec) => {
            assert!(rec.dropped >= c.config().retention.min_dropped);
            // Not `as_of - 30 days`: observation stopped at day 150, so D19's
            // **witness floor** binds before the duration floor does, holding the
            // horizon at the third-most-recent reading. The host is handed the safe
            // horizon, not the one it asked for.
            assert_eq!(rec.horizon, t(148 * DAY));
            assert_eq!(rec.dropped, 147);
        }
        _ => unreachable!(),
    }
}

#[test]
fn a_young_log_is_not_told_to_prune() {
    let mut f = Fixture::new(3);
    for i in 0..5 {
        f.observe(1_000 + i * 60);
    }
    let mut c = f.cycle();
    let due = c.tick(&f.archive, &f.healthy(), t(2 * DAY));
    assert!(!kinds(&due).contains(&"Prune"));
}

#[test]
fn acting_on_a_prune_recommendation_quiets_it() {
    // The full loop of the retention duty: recommended, acted on, silent after.
    let mut f = Fixture::new(3);
    for i in 0..150 {
        f.observe(DAY + i * DAY);
    }
    let mut c = f.cycle();
    let as_of = t(400 * DAY);
    let due = c.tick(&f.archive, &f.healthy(), as_of);
    let DueAction::Prune(rec) = due.into_iter().find(|a| matches!(a, DueAction::Prune(_))).unwrap()
    else {
        unreachable!()
    };

    f.prune(rec.horizon, 400 * DAY);
    assert_eq!(f.archive.generation(), 1, "compaction advanced the generation");

    let next = c.tick(&f.archive, &f.healthy(), t(401 * DAY));
    assert!(!kinds(&next).contains(&"Prune"), "a receding horizon proposes nothing");
}

// --- lifecycle ----------------------------------------------------------------

#[test]
fn an_unchartered_log_is_persisted_but_not_graded() {
    let f = Fixture::new(3);
    let empty: Archive<CharterEntry> = Archive::new();
    let mut c = f.cycle();
    let due = c.tick(&empty, &f.healthy(), t(1_000));
    assert_eq!(kinds(&due), vec!["Save"], "nothing to grade, but the log still persists");
}

// --- the property that justifies the split ------------------------------------

#[test]
fn a_year_of_ticks_is_deterministic_and_replayable() {
    // Same inputs, same times, same actions — which is what a clock-free `tick`
    // buys, and what makes a host's behaviour auditable after the fact.
    let mut f = Fixture::new(3);
    for i in 0..200 {
        f.observe(DAY + i * DAY);
    }
    let run = || {
        let mut c = f.cycle();
        let mut log = Vec::new();
        for day in 0..365 {
            let due = c.tick(&f.archive, &f.healthy(), t(300 * DAY + day * DAY));
            log.push(kinds(&due).join(","));
        }
        log
    };
    let first = run();
    assert_eq!(first, run());
    // And it actually did the things, rather than being uniformly empty.
    assert!(first.iter().any(|l| l.contains("Prune")));
    // The grade here is `RaiseConcern`, not `Cohesion`, and correctly so: these
    // members were inducted at founding and have not re-affirmed in the ~300 days
    // since, which is well past the 180-day cadence, so D13's transmission decay
    // reaches Concern on its own. A polity that simply stops is what that component
    // exists to see.
    assert!(first.iter().all(|l| l.contains("RaiseConcern")));
    assert!(!first.iter().any(|l| l.contains(",Cohesion")));
}
