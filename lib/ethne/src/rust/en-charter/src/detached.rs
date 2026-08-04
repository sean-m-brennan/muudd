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

//! **Signing without handing over the key** — two-phase authoring for records a
//! non-member must co-sign (design **D15**).
//!
//! A `GuardianChange` needs the named guardian's own signature: answerability is
//! accepted, never assigned. But the guardian is a *human*, and their key
//! deliberately lives in their own keystore rather than on the polity's authoring
//! host (one key per operator, stable across every node they guard, so a host that
//! could read it could impersonate them everywhere else). `Signed::build` wants a
//! private key per signer, so the only single-phase route is to move the key — which
//! is precisely the arrangement the key's location exists to prevent.
//!
//! So the payload travels instead:
//!
//! 1. [`pending`] — the archive's side builds the record and exports its canonical
//!    bytes, their CID, and a **legible description**. The description is not
//!    decoration: a seam that asks a human to sign opaque bytes has taught them to
//!    sign anything, which is worse than no seam.
//! 2. The guardian signs those bytes with their own key, in their own process.
//! 3. [`assemble`] — the archive's side puts the signatures onto the payload and
//!    **verifies each one before returning**, so a wrong or corrupted signature fails
//!    here rather than surfacing later as an unauthorized record.
//!
//! `kc-core` is untouched by this: the envelope is assembled from `Signed`'s own
//! public fields, so kith-covenant is unaffected.

use en_core::{canonical, Cid, DidKey, KcError, KcResult, PubKey, Signature, SignatureEntry, Signed};
use serde::Serialize;

use crate::entry::didkey_resolver;
use crate::record::{GuardianChange, GuardianEvidence, GuardianKind};

/// A record built but not yet signed: what the guardian is being asked to sign, in
/// the three forms the exchange needs.
#[derive(Debug, Clone)]
pub struct Pending<T> {
    /// The payload itself, to be handed back to [`assemble`] unchanged.
    pub payload: T,
    /// The exact bytes to sign — canonical CBOR, the same encoding
    /// [`Signed::build`] would have signed.
    pub bytes: Vec<u8>,
    /// The content address of those bytes, for referring to this exchange without
    /// carrying it.
    pub cid: Cid,
    /// What the payload says, in words. Show this to whoever is signing.
    pub description: String,
}

/// Prepare a payload for detached signing.
pub fn pending<T: Serialize>(payload: T, description: String) -> KcResult<Pending<T>> {
    let bytes = canonical::to_canonical_cbor(&payload)?;
    let cid = Cid::of_bytes(&bytes);
    Ok(Pending { payload, bytes, cid, description })
}

/// Attach detached signatures to a payload, verifying each against the payload's own
/// canonical bytes.
///
/// `signatures` is `(signer did:key, signature bytes)`. Every ethne signer is a
/// `did:key`, which embeds its public key, so a signature can be checked here with no
/// registry and no trust in the caller.
///
/// Fails if any signature does not verify, if a DID is malformed, or if the set is
/// empty — an envelope with no signatures verifies nowhere, and producing one would
/// only defer the error to `verify_all`.
pub fn assemble<T: Serialize>(
    payload: T,
    signatures: &[(String, Vec<u8>)],
) -> KcResult<Signed<T>> {
    if signatures.is_empty() {
        return Err(KcError::Key(
            "a detached envelope needs at least one signature".into(),
        ));
    }
    let bytes = canonical::to_canonical_cbor(&payload)?;
    let cid = Cid::of_bytes(&bytes);
    let mut entries = Vec::with_capacity(signatures.len());
    for (did, sig_bytes) in signatures {
        let pk: PubKey = didkey_resolver(did)
            .ok_or_else(|| KcError::Key(format!("{did} is not a resolvable did:key")))?;
        let sig = Signature { alg: pk.alg, bytes: sig_bytes.clone() };
        // Verified here, not later: the point of assembling on this side is that the
        // caller learns immediately whether the guardian actually signed *this*.
        en_core::keys::verify(&pk, &bytes, &sig).map_err(|_| {
            KcError::Key(format!(
                "the detached signature from {did} does not verify against this payload \
                 (wrong payload, wrong key, or a corrupted signature)"
            ))
        })?;
        entries.push(SignatureEntry {
            signer_did: did.clone(),
            alg: pk.alg,
            signature: sig_bytes.clone(),
        });
    }
    Ok(Signed { payload, payload_cid: cid, signatures: entries })
}

/// What a guardian change says, in words, for the human being asked to co-sign it.
///
/// Deliberately spells out the *consequence* rather than only the fields: someone
/// signing this is accepting that a machine answers to them, and the sentence should
/// say so. Includes whether the polity claims substrate backing, because "nobody has
/// observed this edge" is exactly the sort of thing a signer should see before
/// agreeing to it.
pub fn describe_guardian_change(gc: &GuardianChange) -> String {
    let member = gc.member.as_str();
    let polity = gc.polity.as_str();
    let act = match (gc.kind, &gc.guardian) {
        (GuardianKind::Designate, Some(g)) => format!(
            "designate {member} a derived (machine) party answerable to {}",
            g.as_str()
        ),
        (GuardianKind::Designate, None) => {
            format!("designate {member} a derived (machine) party answerable to nobody")
        }
        (GuardianKind::Rotate, Some(g)) => {
            format!("make {member} answerable to {} instead", g.as_str())
        }
        (GuardianKind::Rotate, None) => format!("rotate {member}'s guardian (none named)"),
        (GuardianKind::Release, _) => {
            format!("record that {member} no longer answers to anyone")
        }
    };
    let backing = match &gc.evidence {
        Some(GuardianEvidence::Observed(ob)) => format!(
            "observed by {} at {}",
            ob.payload.observer.as_str(),
            ob.payload.observed_at
        ),
        Some(GuardianEvidence::Unbacked { reason }) => {
            format!("NOT observed by any monitor — declared reason: {reason}")
        }
        None => "no substrate evidence carried".into(),
    };
    let deciders: Vec<&str> = gc.deciders.iter().map(DidKey::as_str).collect();
    format!(
        "polity {polity}: {act}.\n\
         decided at {} by {} decider(s): {}\n\
         substrate: {backing}\n\
         rationale: {}",
        gc.decided_at,
        deciders.len(),
        if deciders.is_empty() { "(none)".into() } else { deciders.join(", ") },
        if gc.rationale.trim().is_empty() { "(none given)" } else { &gc.rationale },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use en_core::{keys, Alg, Timestamp};

    fn did_and_key() -> (DidKey, en_core::PrivKey) {
        let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        (DidKey::from_pubkey(&pk).unwrap(), sk)
    }

    fn change(polity: &DidKey, member: &DidKey, guardian: &DidKey) -> GuardianChange {
        GuardianChange {
            polity: polity.clone(),
            kind: GuardianKind::Designate,
            member: member.clone(),
            guardian: Some(guardian.clone()),
            evidence: Some(GuardianEvidence::Unbacked { reason: "test".into() }),
            rationale: "the operator accepted answerability".into(),
            deciders: vec![polity.clone()],
            decided_at: Timestamp::from_second(1_700_000_000).unwrap(),
        }
    }

    #[test]
    fn a_detached_signature_assembles_and_verifies_like_a_built_one() {
        let (polity, polity_key) = did_and_key();
        let (member, _) = did_and_key();
        let (guardian, guardian_key) = did_and_key();
        let gc = change(&polity, &member, &guardian);

        let p = pending(gc.clone(), describe_guardian_change(&gc)).unwrap();
        // Each signer signs the exported bytes in its own process; here, two.
        let sigs = vec![
            (polity.as_str().to_string(), keys::sign(&polity_key, &p.bytes).unwrap().bytes),
            (guardian.as_str().to_string(), keys::sign(&guardian_key, &p.bytes).unwrap().bytes),
        ];
        let signed = assemble(p.payload, &sigs).expect("assembles");

        // Indistinguishable from the single-phase route: same CID, and it verifies
        // against the same resolver the archive uses.
        assert_eq!(signed.payload_cid, p.cid);
        signed.verify(didkey_resolver).expect("verifies like any other envelope");
        let built = Signed::build(gc, &[
            (polity.as_str(), &polity_key),
            (guardian.as_str(), &guardian_key),
        ])
        .unwrap();
        assert_eq!(signed.payload_cid, built.payload_cid);
        assert_eq!(signed.signatures, built.signatures, "ed25519 is deterministic");
    }

    #[test]
    fn a_signature_over_different_bytes_is_refused_at_assembly() {
        let (polity, polity_key) = did_and_key();
        let (member, _) = did_and_key();
        let (guardian, guardian_key) = did_and_key();
        let gc = change(&polity, &member, &guardian);
        let p = pending(gc, String::new()).unwrap();

        // The guardian signed some *other* payload — a stale exchange, or a swap.
        let other = keys::sign(&guardian_key, b"a different payload entirely").unwrap();
        let sigs = vec![
            (polity.as_str().to_string(), keys::sign(&polity_key, &p.bytes).unwrap().bytes),
            (guardian.as_str().to_string(), other.bytes),
        ];
        let err = assemble(p.payload, &sigs).expect_err("must not assemble");
        assert!(err.to_string().contains("does not verify"), "{err}");
    }

    #[test]
    fn one_flipped_bit_is_refused_at_assembly() {
        let (polity, polity_key) = did_and_key();
        let (member, _) = did_and_key();
        let (guardian, guardian_key) = did_and_key();
        let gc = change(&polity, &member, &guardian);
        let p = pending(gc, String::new()).unwrap();
        let mut sig = keys::sign(&guardian_key, &p.bytes).unwrap().bytes;
        sig[0] ^= 0x01;
        let sigs = vec![
            (polity.as_str().to_string(), keys::sign(&polity_key, &p.bytes).unwrap().bytes),
            (guardian.as_str().to_string(), sig),
        ];
        assert!(assemble(p.payload, &sigs).is_err());
    }

    #[test]
    fn an_unresolvable_signer_and_an_empty_set_are_refused() {
        let (polity, _) = did_and_key();
        let (member, _) = did_and_key();
        let (guardian, _) = did_and_key();
        let gc = change(&polity, &member, &guardian);

        let p = pending(gc.clone(), String::new()).unwrap();
        let err = assemble(p.payload, &[("not-a-did".into(), vec![0u8; 64])]).unwrap_err();
        assert!(err.to_string().contains("not a resolvable did:key"), "{err}");

        let p2 = pending(gc, String::new()).unwrap();
        let err = assemble(p2.payload, &[]).unwrap_err();
        assert!(err.to_string().contains("at least one signature"), "{err}");
    }

    #[test]
    fn the_description_says_what_is_being_accepted_and_what_backs_it() {
        let (polity, _) = did_and_key();
        let (member, _) = did_and_key();
        let (guardian, _) = did_and_key();
        let mut gc = change(&polity, &member, &guardian);

        let d = describe_guardian_change(&gc);
        assert!(d.contains("answerable to"), "the consequence, not just the fields: {d}");
        assert!(d.contains(member.as_str()) && d.contains(guardian.as_str()));
        // The override must be visible to whoever is signing — this is the case where
        // a human should think twice.
        assert!(d.contains("NOT observed"), "{d}");

        gc.evidence = None;
        assert!(describe_guardian_change(&gc).contains("no substrate evidence"));
    }
}
