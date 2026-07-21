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

use kc_core::{keys, Alg, Cid, DidKey, PubKey, Timestamp};

use crate::record::*;

fn t(s: i64) -> Timestamp {
    jiff::Timestamp::from_second(s).unwrap()
}

fn key() -> (DidKey, PubKey) {
    let (_, pk) = keys::generate(Alg::Ed25519).unwrap();
    (DidKey::from_pubkey(&pk).unwrap(), pk)
}

fn did() -> DidKey {
    key().0
}

#[test]
fn observer_entry_active_window() {
    let (od, opk) = key();
    let e = ObserverEntry {
        observer_did: od,
        observer_pubkey: opk,
        declared_perspective: "ecological-conservationist".into(),
        valid_from: t(100),
        valid_until: Some(t(200)),
    };
    assert!(!e.active_at(t(50)));
    assert!(e.active_at(t(100)));
    assert!(e.active_at(t(150)));
    assert!(!e.active_at(t(200)));
}

#[test]
fn observer_registry_roundtrips_json() {
    let (od, opk) = key();
    let x = ObserverRegistry {
        nominating_community: did(),
        entries: vec![ObserverEntry {
            observer_did: od,
            observer_pubkey: opk,
            declared_perspective: "labor-rights".into(),
            valid_from: t(10),
            valid_until: None,
        }],
        issued_at: t(10),
    };
    let back: ObserverRegistry = serde_json::from_str(&serde_json::to_string(&x).unwrap()).unwrap();
    assert_eq!(x, back);
}

#[test]
fn observer_report_and_response_roundtrip_json() {
    let rep = ObserverReport {
        subject_community: did(),
        observer_did: did(),
        declared_perspective: "labor-rights".into(),
        summary: "internal hierarchy far more authoritarian than peers".into(),
        references: vec![Cid::of_bytes(b"conduct1")],
        observed_at: t(500),
        reported_at: t(510),
    };
    let back: ObserverReport = serde_json::from_str(&serde_json::to_string(&rep).unwrap()).unwrap();
    assert_eq!(rep, back);

    let resp = ReportResponse {
        report_cid: Cid::of_bytes(b"report"),
        responding_community: did(),
        response: "our council structure is by consent; see our charter".into(),
        responded_at: t(600),
    };
    let back: ReportResponse =
        serde_json::from_str(&serde_json::to_string(&resp).unwrap()).unwrap();
    assert_eq!(resp, back);
}

#[test]
fn whistleblower_and_reveal_roundtrip_json() {
    let wb = WhistleblowerReport {
        accused_community: did(),
        source_commitment: Cid::of_bytes(b"commit"),
        claim: "coerced labor in the maintenance rota".into(),
        references: vec![],
        reception_panel: vec![did(), did(), did()],
        filed_at: t(1000),
    };
    let back: WhistleblowerReport =
        serde_json::from_str(&serde_json::to_string(&wb).unwrap()).unwrap();
    assert_eq!(wb, back);

    let rev = SourceReveal {
        report_cid: Cid::of_bytes(b"wb"),
        panel: vec![did(), did()],
        cause: "the source recanted and the claim is now contested".into(),
        disclosed_source: "did:key:zSomeMember".into(),
        revealed_at: t(2000),
    };
    let back: SourceReveal = serde_json::from_str(&serde_json::to_string(&rev).unwrap()).unwrap();
    assert_eq!(rev, back);
}
