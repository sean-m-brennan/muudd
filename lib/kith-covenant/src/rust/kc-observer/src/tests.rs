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

use kc_core::{
    keys, Alg, Cid, DidKey, PrivKey, PubKey, RevealedShare, SourcePreimage, ThresholdCustody,
    Timestamp,
};

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

/// A filing with a real ring attestation and real threshold custody. Returns the
/// report plus the panel keys, so a caller can open shares.
#[allow(clippy::type_complexity)]
fn filing(claim: &str) -> (WhistleblowerReport, Vec<(DidKey, PubKey, PrivKey)>) {
    let accused = did();
    // Three insider keys for the accused community; the source is index 1.
    let insiders: Vec<(PrivKey, PubKey)> = (0..3)
        .map(|_| keys::generate(Alg::Ed25519).unwrap())
        .collect();
    let ring: Vec<PubKey> = insiders.iter().map(|(_, pk)| pk.clone()).collect();
    let source_did = DidKey::from_pubkey(&ring[1]).unwrap();

    let preimage = SourcePreimage::new(source_did.as_str());
    let commitment = preimage.commitment().unwrap();

    let panel: Vec<(DidKey, PubKey, PrivKey)> = (0..3)
        .map(|_| {
            let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
            (DidKey::from_pubkey(&pk).unwrap(), pk, sk)
        })
        .collect();
    // These holders are `did:key`s, so the `did:key` resolver returns each one's own
    // key. A real §8 filing resolves against the mediator registry's encryption keys.
    let holders: Vec<DidKey> = panel.iter().map(|(d, _, _)| d.clone()).collect();
    let bytes = kc_core::canonical::to_canonical_cbor(&preimage).unwrap();
    let custody = ThresholdCustody::seal(
        &bytes,
        &holders,
        kc_core::didkey_encryption_resolver,
        2,
    )
    .unwrap();

    let insider = WhistleblowerReport::attest(
        accused.clone(),
        commitment,
        claim,
        t(1000),
        ring,
        1,
        &insiders[1].0,
    )
    .unwrap();

    let wb = WhistleblowerReport {
        accused_community: accused,
        source_commitment: commitment,
        claim: claim.into(),
        references: vec![],
        reception_panel: panel.iter().map(|(d, _, _)| d.clone()).collect(),
        filed_at: t(1000),
        insider,
        custody,
    };
    (wb, panel)
}

#[test]
fn whistleblower_and_reveal_roundtrip_json() {
    let (wb, panel) = filing("coerced labor in the maintenance rota");
    let back: WhistleblowerReport =
        serde_json::from_str(&serde_json::to_string(&wb).unwrap()).unwrap();
    assert_eq!(wb, back);
    // The attestation survives the wire, which is what the authority pass re-checks.
    back.insider.verify().unwrap();

    let shares: Vec<RevealedShare> = panel[..2]
        .iter()
        .map(|(d, _, sk)| wb.custody.open_share(d, sk).unwrap())
        .collect();
    let rev = SourceReveal {
        report_cid: Cid::of_bytes(b"wb"),
        panel: vec![did(), did()],
        cause: "the source recanted and the claim is now contested".into(),
        disclosed_source: "did:key:zSomeMember".into(),
        shares,
        revealed_at: t(2000),
    };
    let back: SourceReveal = serde_json::from_str(&serde_json::to_string(&rev).unwrap()).unwrap();
    assert_eq!(rev, back);
}

#[test]
fn a_filing_carries_a_verifiable_insider_attestation() {
    let (wb, _panel) = filing("the rota is coerced");
    wb.insider.verify().unwrap();
    // Bound to this claim and this filing.
    assert_eq!(
        wb.insider.payload.claim_digest,
        WhistleblowerReport::claim_digest(&wb.claim).unwrap()
    );
    assert_eq!(wb.insider.payload.accused_community, wb.accused_community);
    assert_eq!(wb.insider.payload.source_commitment, wb.source_commitment);
    assert_eq!(wb.insider.payload.filed_at, wb.filed_at);
    // The ring hides the signer among three.
    assert_eq!(wb.insider.ring.len(), 3);
}

#[test]
fn an_attestation_does_not_transfer_to_another_claim() {
    // The panel receives an attestation for one accusation and tries to file a
    // different, graver one under it.
    let (wb, _panel) = filing("the rota is coerced");
    let swapped = WhistleblowerReport { claim: "they are poisoning the water".into(), ..wb };
    assert_ne!(
        swapped.insider.payload.claim_digest,
        WhistleblowerReport::claim_digest(&swapped.claim).unwrap()
    );
    // The ring signature itself still verifies — it is over the original claim
    // digest — so catching this is the authority pass's job, and it has what it
    // needs to do so.
    swapped.insider.verify().unwrap();
}

#[test]
fn custody_needs_a_quorum_to_name_the_source() {
    let (wb, panel) = filing("coerced labor");
    let open = |i: usize| wb.custody.open_share(&panel[i].0, &panel[i].2).unwrap();
    // One panelist alone recovers nothing.
    assert!(wb.custody.recover_preimage(&[open(0)], &wb.source_commitment).is_err());
    // Two — the threshold — recover the source, and it opens the commitment.
    let preimage = wb
        .custody
        .recover_preimage(&[open(0), open(2)], &wb.source_commitment)
        .unwrap();
    // The recovered identity is the ring member who signed.
    let signer_did = DidKey::from_pubkey(&wb.insider.ring[1]).unwrap();
    assert_eq!(preimage.identity, signer_did.as_str());
}

#[test]
fn exculpation_clears_a_wrongly_named_member() {
    let (wb, _panel) = filing("coerced labor");
    let report_cid = Cid::of(&wb).unwrap();
    // The true source's own exculpation attempt reproduces the filing's key image —
    // it is a confession, not a denial.
    let (accused_priv, accused_pub) = keys::generate(Alg::Ed25519).unwrap();
    let innocent_did = DidKey::from_pubkey(&accused_pub).unwrap();
    let exc =
        SourceExculpation::prove(report_cid, innocent_did, &accused_priv, t(3000)).unwrap();
    exc.verify_proof(&accused_pub).unwrap();
    assert_ne!(&exc.claimed_key_image, wb.source_tag());
    // The proof is bound to its key: another member cannot borrow it.
    let (_, other_pub) = keys::generate(Alg::Ed25519).unwrap();
    assert!(exc.verify_proof(&other_pub).is_err());
}

#[test]
fn the_true_sources_exculpation_reproduces_the_filings_tag() {
    // Same machinery, opposite outcome: the source cannot deny its own filing,
    // because its honest key image is the one on the record.
    let insiders: Vec<(PrivKey, PubKey)> = (0..3)
        .map(|_| keys::generate(Alg::Ed25519).unwrap())
        .collect();
    let ring: Vec<PubKey> = insiders.iter().map(|(_, pk)| pk.clone()).collect();
    let attestation = WhistleblowerReport::attest(
        did(),
        Cid::of_bytes(b"commit"),
        "claim",
        t(10),
        ring,
        2,
        &insiders[2].0,
    )
    .unwrap();
    let exc = SourceExculpation::prove(
        Cid::of_bytes(b"report"),
        DidKey::from_pubkey(&insiders[2].1).unwrap(),
        &insiders[2].0,
        t(20),
    )
    .unwrap();
    exc.verify_proof(&insiders[2].1).unwrap();
    assert_eq!(exc.claimed_key_image, attestation.signature.key_image);
}

#[test]
fn source_exculpation_roundtrips_json() {
    let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
    let exc = SourceExculpation::prove(
        Cid::of_bytes(b"report"),
        DidKey::from_pubkey(&pk).unwrap(),
        &sk,
        t(50),
    )
    .unwrap();
    let back: SourceExculpation =
        serde_json::from_str(&serde_json::to_string(&exc).unwrap()).unwrap();
    assert_eq!(exc, back);
    back.verify_proof(&pk).unwrap();
}
