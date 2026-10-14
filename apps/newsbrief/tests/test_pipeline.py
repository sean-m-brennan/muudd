import json

import pytest

from newsbrief import db, triage
from newsbrief.brief import build_briefing
from newsbrief.cluster import cluster
from newsbrief.llm import LLM, FALLBACK_BETA, BudgetExceeded, LLMError

from conftest import FakeClient, add_article


def seed(conn):
    add_article(conn, "https://x/water", "County approves water rights changes for irrigators",
                "Lane County board voted to cut irrigation allocations.", source="Gazette")
    add_article(conn, "https://x/celeb", "Pop star announces world tour dates",
                "The singer will play forty cities.", source="Gossip")
    cluster(conn, 0.35, 3)


def scorer(kwargs):
    """Score stories by whether they mention water."""
    user = kwargs["messages"][0]["content"]
    scores = []
    for block in user.split("<story ")[1:]:
        sid = int(block.split('"')[1])
        high = "water" in block
        v = 9 if high else 1
        scores.append({"story_id": sid, "consequence": v, "proximity": v, "actionability": v,
                       "durability": v, "rationale": "affects irrigation" if high else "no link"})
    scores.append({"story_id": 999, "consequence": 10, "proximity": 10, "actionability": 10,
                   "durability": 10, "rationale": "invented"})
    return {"scores": scores}


CONTEXT = {"why_it_matters": "Your farm irrigates.", "what_happened": "Allocations cut [1].",
           "background": "Drought.", "base_rate": "Rare.", "agreement": "All agree.",
           "disputes": "None.", "unknowns": "Start date.", "loading_note": "Coverage is proportionate."}


def test_request_shape(conn, config):
    client = FakeClient(lambda k: {"ok": True})
    LLM(conn, config.prices, 10.0, client).call("triage", config.triage, "sys", "hi", schema={"type": "object"})
    k = client.calls[0]
    assert k["model"] == "claude-opus-5"
    assert k["betas"] == [FALLBACK_BETA] and k["extra_body"] == {"fallbacks": "default"}
    assert k["output_config"]["effort"] == "low"
    assert k["output_config"]["format"]["type"] == "json_schema"
    assert k["system"][0]["cache_control"] == {"type": "ephemeral"}


def test_triage_scores_and_ignores_invented_ids(conn, config):
    seed(conn)
    llm = LLM(conn, config.prices, 10.0, FakeClient(scorer))
    assert triage.triage(conn, llm, config.triage, config.profile(), 3) == 2
    rows = dict(conn.execute("SELECT story_id, overall FROM scores").fetchall())
    assert sorted(rows.values()) == [1.0, 9.0]
    # Nothing changed, so nothing is re-scored.
    assert triage.stories_needing_triage(conn, 3, 100) == []


def test_story_rescored_after_doubling(conn, config):
    seed(conn)
    llm = LLM(conn, config.prices, 10.0, FakeClient(scorer))
    triage.triage(conn, llm, config.triage, config.profile(), 3)
    add_article(conn, "https://x/water2", "Water rights changes anger county irrigators",
                "Irrigators protested the county water allocation cuts.")
    cluster(conn, 0.35, 3)
    assert len(triage.stories_needing_triage(conn, 3, 100)) == 1


def test_briefing(conn, config):
    seed(conn)
    triage.triage(conn, LLM(conn, config.prices, 10.0, FakeClient(scorer)), config.triage, config.profile(), 3)
    client = FakeClient(lambda k: CONTEXT)
    path = build_briefing(conn, LLM(conn, config.prices, 10.0, client), config, log=lambda m: None)
    text = path.read_text()
    assert "## 1. County approves water rights" in text
    assert "**Why it matters to you.** Your farm irrigates." in text
    assert "[County approves water rights changes for irrigators](https://x/water)" in text
    # The low-proximity celebrity story is the outside-the-bubble pick.
    assert "Outside your usual view: Pop star" in text
    # Briefed stories don't repeat.
    second = build_briefing(conn, LLM(conn, config.prices, 10.0, client), config, log=lambda m: None)
    assert "Nothing new cleared" in second.read_text()


def test_followed_updates(conn, config):
    seed(conn)
    sid = conn.execute("SELECT story_id FROM articles WHERE url = 'https://x/water'").fetchone()[0]
    conn.execute("UPDATE stories SET followed_at = ? WHERE id = ?", (db.days_ago(1), sid))
    path = build_briefing(conn, LLM(conn, config.prices, 10.0, FakeClient(lambda k: CONTEXT)), config,
                          log=lambda m: None)
    assert "## Stories you follow" in path.read_text()


def test_budget_cap(conn, config):
    llm = LLM(conn, config.prices, 0.01, FakeClient(lambda k: "{}"))
    llm.call("triage", config.triage, "s", "u")  # 1000*5 + 200*25 = $0.01
    with pytest.raises(BudgetExceeded):
        llm.call("triage", config.triage, "s", "u")


def test_refusal_raises(conn, config):
    llm = LLM(conn, config.prices, 10.0, FakeClient(lambda k: "", stop_reason="refusal"))
    with pytest.raises(LLMError):
        llm.call("triage", config.triage, "s", "u")
