import os

import pytest

from newsbrief import config
from newsbrief.config import ModelConfig
from newsbrief.llm import LLM, LLMError

from conftest import FakeOpenAIClient

GROQ = ModelConfig("openai/gpt-oss-120b", "low", 8000, provider="groq")
PRICES = {"openai/gpt-oss-120b": (0.15, 0.60)}


def test_openai_compatible_request_shape(conn):
    client = FakeOpenAIClient(lambda k: {"ok": True})
    text = LLM(conn, PRICES, 10.0, clients={"groq": client}).call(
        "triage", GROQ, "sys", "hi", schema={"type": "object"})
    assert text == '{"ok": true}'
    k = client.calls[0]
    assert k["model"] == "openai/gpt-oss-120b" and k["max_tokens"] == 8000
    assert k["messages"] == [{"role": "system", "content": "sys"}, {"role": "user", "content": "hi"}]
    assert k["reasoning_effort"] == "low"
    assert k["response_format"]["json_schema"]["schema"] == {"type": "object"}
    assert "betas" not in k and "extra_body" not in k


def test_openai_compatible_spend_counts_thinking(conn):
    LLM(conn, PRICES, 10.0, clients={"groq": FakeOpenAIClient(lambda k: "{}")}).call("triage", GROQ, "s", "u")
    row = conn.execute("SELECT model, input_tokens, output_tokens, usd FROM spend").fetchone()
    assert tuple(row[:3]) == ("openai/gpt-oss-120b", 1000, 200)
    assert row[3] == pytest.approx((1000 * 0.15 + 200 * 0.60) / 1_000_000)


def test_no_effort_omits_reasoning(conn):
    client = FakeOpenAIClient(lambda k: "{}")
    mc = ModelConfig("gemini-2.5-flash", "", 8000, provider="gemini")
    LLM(conn, {}, 10.0, clients={"gemini": client}).call("triage", mc, "s", "u")
    assert "reasoning_effort" not in client.calls[0] and "response_format" not in client.calls[0]


@pytest.mark.parametrize("kwargs", [{"finish_reason": "length"}, {"finish_reason": "content_filter"},
                                    {"refusal": "no"}])
def test_openai_compatible_errors(conn, kwargs):
    llm = LLM(conn, PRICES, 10.0, clients={"groq": FakeOpenAIClient(lambda k: "", **kwargs)})
    with pytest.raises(LLMError):
        llm.call("triage", GROQ, "s", "u")


def test_missing_key(conn, monkeypatch):
    monkeypatch.delenv("GROQ_API_KEY", raising=False)
    with pytest.raises(LLMError, match="GROQ_API_KEY"):
        LLM(conn, PRICES, 10.0).call("triage", GROQ, "s", "u")


def test_provider_config(tmp_path):
    path = tmp_path / "config.toml"
    path.write_text('[triage]\nprovider = "groq"\n[context]\nprovider = "gemini"\nmodel = "gemini-2.5-pro"\n')
    cfg = config.load(path)
    assert (cfg.triage.provider, cfg.triage.model) == ("groq", "openai/gpt-oss-120b")
    assert (cfg.context.provider, cfg.context.model) == ("gemini", "gemini-2.5-pro")
    path.write_text('[triage]\nprovider = "nope"\n')
    with pytest.raises(ValueError, match="unknown provider"):
        config.load(path)


def test_load_env(tmp_path, monkeypatch):
    monkeypatch.delenv("NB_TEST_A", raising=False)
    monkeypatch.setenv("NB_TEST_B", "kept")
    env = tmp_path / ".env"
    env.write_text('# keys\nexport NB_TEST_A="from file"\nNB_TEST_B=overridden\n')
    config.load_env(env)
    assert os.environ["NB_TEST_A"] == "from file" and os.environ["NB_TEST_B"] == "kept"
    monkeypatch.delenv("NB_TEST_A")
