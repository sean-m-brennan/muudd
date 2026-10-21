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

use kc_conduct::DecisionClass;
use kc_core::{keys, Alg, DidKey, Timestamp};

use crate::record::*;

fn t(s: i64) -> Timestamp {
    jiff::Timestamp::from_second(s).unwrap()
}

fn did() -> DidKey {
    let (_, pk) = keys::generate(Alg::Ed25519).unwrap();
    DidKey::from_pubkey(&pk).unwrap()
}

fn matter(k: &str) -> IncapableMatter {
    IncapableMatter { key: k.into(), description: format!("the {k} of any member") }
}

#[test]
fn amendment_kind_names_and_contraction() {
    assert_eq!(AmendmentKind::Add.as_str(), "Add");
    assert_eq!(AmendmentKind::Remove.as_str(), "Remove");
    assert!(!AmendmentKind::Add.is_contraction());
    assert!(AmendmentKind::Remove.is_contraction());
    assert_eq!(FastChange::Extend.as_str(), "Extend");
    assert!(FastChange::Contract.is_contraction());
    assert!(!FastChange::Extend.is_contraction());
}

#[test]
fn fast_period_contains_and_seconds() {
    let p = FastPeriod { label: "lent".into(), start: t(100), end: t(200) , recurrence: None };
    assert!(!p.contains(t(50)));
    assert!(p.contains(t(100)));
    assert!(p.contains(t(199)));
    assert!(!p.contains(t(200)));
    assert_eq!(p.seconds(), 100);
    let inverted = FastPeriod { label: "x".into(), start: t(200), end: t(100) , recurrence: None };
    assert_eq!(inverted.seconds(), 0);
    assert!(!p.is_recurring());
}

#[test]
fn recurring_fast_repeats_on_its_cycle() {
    // A 100s fast every 1000s from t=100: active in [100,200), [1100,1200), ...
    let p = FastPeriod {
        label: "annual".into(),
        start: t(100),
        end: t(200),
        recurrence: Some(1000),
    };
    assert!(p.is_recurring());
    assert!(!p.contains(t(50)));      // before the first occurrence
    assert!(p.contains(t(150)));      // first occurrence
    assert!(!p.contains(t(500)));     // in the gap
    assert!(p.contains(t(1100)));     // second occurrence, at its start
    assert!(p.contains(t(1199)));     // second occurrence, at its end
    assert!(!p.contains(t(1200)));    // second occurrence just ended
    assert!(p.contains(t(10_100)));   // a far-future occurrence
    // seconds() is the per-occurrence duration.
    assert_eq!(p.seconds(), 100);
}

#[test]
fn degenerate_recurrence_falls_back_to_single_window() {
    // A cycle shorter than the occurrence (would overlap) is treated as one-shot.
    let overlap = FastPeriod {
        label: "bad".into(),
        start: t(100),
        end: t(300),
        recurrence: Some(50),
    };
    assert!(!overlap.is_recurring());
    assert!(overlap.contains(t(250)));   // inside the single window
    assert!(!overlap.contains(t(350)));  // no recurrence — window ended
}

#[test]
fn genesis_roundtrips_json() {
    let g = ConstitutionGenesis {
        compact_incapable: vec![matter("internal-theology"), matter("internal-governance")],
        fast_periods: vec![FastPeriod { label: "annual".into(), start: t(1000), end: t(2000) , recurrence: None }],
        ratifiers: vec![did(), did()],
        genesis_at: t(10),
    };
    let back: ConstitutionGenesis = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
    assert_eq!(g, back);
}

#[test]
fn amendments_roundtrip_json() {
    let inc = CompactIncapableAmendment {
        kind: AmendmentKind::Add,
        matter: matter("aesthetic-preferences"),
        rationale: "clarifying a matter left implicit at genesis".into(),
        deciders: vec![did(), did()],
        decision_class: DecisionClass::SecondClass,
        decided_at: t(500),
    };
    let back: CompactIncapableAmendment =
        serde_json::from_str(&serde_json::to_string(&inc).unwrap()).unwrap();
    assert_eq!(inc, back);

    let fast = FastCadenceAmendment {
        change: FastChange::Contract,
        period: FastPeriod { label: "annual".into(), start: t(1000), end: t(1500) , recurrence: None },
        rationale: "shortening under pragmatic pressure".into(),
        deciders: vec![did(), did(), did()],
        decision_class: DecisionClass::SecondClass,
        decided_at: t(600),
    };
    let back: FastCadenceAmendment =
        serde_json::from_str(&serde_json::to_string(&fast).unwrap()).unwrap();
    assert_eq!(fast, back);
}
