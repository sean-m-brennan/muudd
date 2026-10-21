#  Copyright 2026 Sean M. Brennan and contributors
#
#   Licensed under the Apache License, Version 2.0 (the "License");
#   you may not use this file except in compliance with the License.
#   You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
#   Unless required by applicable law or agreed to in writing, software
#   distributed under the License is distributed on an "AS IS" BASIS,
#   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#   See the License for the specific language governing permissions and
#   limitations under the License.

"""D15's guardian co-signature, through the Python surface: substrate backing, and
both custody paths for the guardian's own key.

The rule-by-rule coverage is in the Rust `en-charter` tests; these confirm the
bindings and facades wire the whole chain — author -> verify_all -> check_authority ->
guardian_state.
"""
import pytest

from ethne import (
    Signer, designate_derived, rotate_guardian, record_observation,
    pending_guardian_change, append_guardian_change_signed, sign_detached,
)


def _seen(polity, monitor, member, guardian, at):
    """A monitor records that `member` is guarded by `guardian`, and returns the CID
    the guardian change will cite (and the record will copy)."""
    return record_observation(
        polity.archive, polity.did, monitor,
        [(member, 0.9, guardian.did if guardian else None, at - 10)],
        observed_at=at,
    )


# --- substrate backing --------------------------------------------------------

def test_an_observed_edge_authorizes(polity):
    machine = polity.founders[2].did
    monitor, human = Signer.generate(), Signer.generate()
    obs = _seen(polity, monitor, machine, human, 150)

    designate_derived(polity.archive, polity.did, machine, polity.founders[:2],
                      guardian=human, evidence_cid=obs, decided_at=200)
    polity.archive.verify_all()
    polity.archive.check_authority()
    st = polity.archive.guardian_state(polity.did, as_of=250)
    assert st.is_derived(machine) and st.guardian_of(machine) == human.did


def test_an_unobserved_edge_is_refused(polity):
    machine = polity.founders[2].did
    human = Signer.generate()
    # Neither evidence nor a declared override.
    designate_derived(polity.archive, polity.did, machine, polity.founders[:2],
                      guardian=human, decided_at=200)
    reasons = [why for _cid, why in polity.archive.authority_violations()]
    assert any("no substrate evidence" in r for r in reasons), reasons


def test_the_override_authorizes_and_must_say_why(polity):
    machine = polity.founders[2].did
    human = Signer.generate()
    designate_derived(polity.archive, polity.did, machine, polity.founders[:2],
                      guardian=human, unbacked_reason="bootstrap: no monitor yet",
                      decided_at=200)
    polity.archive.check_authority()

    with pytest.raises(ValueError, match="may not be empty"):
        designate_derived(polity.archive, polity.did, polity.founders[1].did,
                          polity.founders[:2], guardian=human, unbacked_reason="  ",
                          decided_at=210)


def test_evidence_and_an_override_are_mutually_exclusive(polity):
    machine = polity.founders[2].did
    monitor, human = Signer.generate(), Signer.generate()
    obs = _seen(polity, monitor, machine, human, 150)
    with pytest.raises(ValueError, match="not both"):
        designate_derived(polity.archive, polity.did, machine, polity.founders[:2],
                          guardian=human, evidence_cid=obs,
                          unbacked_reason="also this", decided_at=200)


def test_evidence_must_be_an_observation_in_this_charter(polity):
    machine = polity.founders[2].did
    human = Signer.generate()
    # A real CID, but of the charter record rather than an observation.
    charter_cid = next(iter(polity.archive.cids()))
    with pytest.raises(ValueError, match="not an observation"):
        designate_derived(polity.archive, polity.did, machine, polity.founders[:2],
                          guardian=human, evidence_cid=charter_cid, decided_at=200)


def test_evidence_showing_another_guardian_is_refused(polity):
    machine = polity.founders[2].did
    monitor, human, other = Signer.generate(), Signer.generate(), Signer.generate()
    obs = _seen(polity, monitor, machine, other, 150)
    designate_derived(polity.archive, polity.did, machine, polity.founders[:2],
                      guardian=human, evidence_cid=obs, decided_at=200)
    reasons = [why for _cid, why in polity.archive.authority_violations()]
    assert any("not the claimed" in r for r in reasons), reasons


# --- custody path B: detached signing -----------------------------------------

def test_a_detached_co_signature_authorizes_without_the_key_on_this_host(polity):
    """The guardian's private key is never passed to the archive: it signs the exported
    bytes and only the signature comes back."""
    machine = polity.founders[2].did
    monitor, human = Signer.generate(), Signer.generate()
    obs = _seen(polity, monitor, machine, human, 150)

    cid, payload, description = pending_guardian_change(
        polity.archive, polity.did, machine, "Designate", polity.founders[:2],
        guardian_did=human.did, evidence_cid=obs, decided_at=200,
    )
    # What the human is shown before signing says what they are accepting, and what
    # backs it — not just a hash.
    assert "answerable to" in description and human.did in description
    assert "observed by" in description
    assert isinstance(payload, bytes) and payload

    signatures = [sign_detached(s, payload) for s in polity.founders[:2]]
    signatures.append(sign_detached(human, payload))
    append_guardian_change_signed(polity.archive, cid, signatures)

    polity.archive.verify_all()
    polity.archive.check_authority()
    assert polity.archive.guardian_state(polity.did, as_of=250).guardian_of(machine) == human.did


def test_a_signature_over_other_bytes_is_refused_before_anything_is_appended(polity):
    machine = polity.founders[2].did
    monitor, human = Signer.generate(), Signer.generate()
    obs = _seen(polity, monitor, machine, human, 150)
    before = len(polity.archive.cids())

    cid, payload, _ = pending_guardian_change(
        polity.archive, polity.did, machine, "Designate", polity.founders[:2],
        guardian_did=human.did, evidence_cid=obs, decided_at=200,
    )
    bad = [sign_detached(s, payload) for s in polity.founders[:2]]
    bad.append((human.did, sign_detached(human, b"some other payload")[1]))
    with pytest.raises(RuntimeError, match="does not verify"):
        append_guardian_change_signed(polity.archive, cid, bad)
    assert len(polity.archive.cids()) == before, "nothing was appended"

    # NEGATIVE CONTROL: the exchange is still pending, so a corrected signature works.
    good = [sign_detached(s, payload) for s in polity.founders[:2]] + [sign_detached(human, payload)]
    append_guardian_change_signed(polity.archive, cid, good)
    polity.archive.check_authority()


def test_the_wrong_guardian_key_is_refused(polity):
    machine = polity.founders[2].did
    monitor, human, impostor = Signer.generate(), Signer.generate(), Signer.generate()
    obs = _seen(polity, monitor, machine, human, 150)
    cid, payload, _ = pending_guardian_change(
        polity.archive, polity.did, machine, "Designate", polity.founders[:2],
        guardian_did=human.did, evidence_cid=obs, decided_at=200,
    )
    # The impostor signs, and says so — a valid signature by the wrong signer. It
    # assembles (the signature is genuine), and the authority pass refuses it because
    # the named guardian did not consent.
    sigs = [sign_detached(s, payload) for s in polity.founders[:2]]
    sigs.append(sign_detached(impostor, payload))
    append_guardian_change_signed(polity.archive, cid, sigs)
    reasons = [why for _cid, why in polity.archive.authority_violations()]
    assert any("named guardian" in r for r in reasons), reasons


def test_an_unknown_pending_exchange_says_so(polity):
    with pytest.raises(ValueError, match="no pending guardian change"):
        append_guardian_change_signed(polity.archive, "bafyunknowncid", [])


def test_a_detached_rotation_needs_its_own_backing(polity):
    machine = polity.founders[2].did
    monitor, first, second = Signer.generate(), Signer.generate(), Signer.generate()
    designate_derived(polity.archive, polity.did, machine, polity.founders[:2],
                      guardian=first, evidence_cid=_seen(polity, monitor, machine, first, 150),
                      decided_at=200)
    obs2 = _seen(polity, monitor, machine, second, 250)
    rotate_guardian(polity.archive, polity.did, machine, second, polity.founders[:2],
                    evidence_cid=obs2, decided_at=300)
    polity.archive.verify_all()
    polity.archive.check_authority()
    assert polity.archive.guardian_state(polity.did, as_of=350).guardian_of(machine) == second.did
