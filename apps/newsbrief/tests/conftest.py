import json
from pathlib import Path
from types import SimpleNamespace

import pytest

from newsbrief import db
from newsbrief.config import Config, ModelConfig


class FakeClient:
    """Stands in for anthropic.Anthropic; replies come from a callable(kwargs) -> dict | str."""

    def __init__(self, reply, model="claude-opus-5", stop_reason="end_turn"):
        self.reply = reply
        self.calls = []
        self.model = model
        self.stop_reason = stop_reason
        self.beta = SimpleNamespace(messages=SimpleNamespace(create=self._create))

    def _create(self, **kwargs):
        self.calls.append(kwargs)
        body = self.reply(kwargs)
        text = body if isinstance(body, str) else json.dumps(body)
        return SimpleNamespace(
            model=self.model,
            stop_reason=self.stop_reason,
            stop_details=None,
            content=[SimpleNamespace(type="thinking", thinking=""), SimpleNamespace(type="text", text=text)],
            usage=SimpleNamespace(input_tokens=1000, output_tokens=200,
                                  cache_read_input_tokens=0, cache_creation_input_tokens=0),
        )


@pytest.fixture
def conn():
    c = db.connect(":memory:")
    yield c
    c.close()


@pytest.fixture
def config(tmp_path: Path) -> Config:
    profile = tmp_path / "profile.md"
    profile.write_text("I run a small farm in Lane County, Oregon.")
    return Config(
        db_path=tmp_path / "test.db",
        profile_path=profile,
        output_dir=tmp_path / "briefings",
        feeds=[],
        triage=ModelConfig("claude-opus-5", "low", 8000),
        context=ModelConfig("claude-opus-5", "high", 16000),
        prices={"claude-opus-5": (5.0, 25.0)},
    )


def add_article(conn, url, title, summary="", source="Test Wire", published=None, story_id=None):
    stamp = published or db.now()
    conn.execute(
        "INSERT INTO articles (url, source, title, summary, published, fetched, story_id)"
        " VALUES (?, ?, ?, ?, ?, ?, ?)",
        (url, source, title, summary, stamp, stamp, story_id),
    )
    conn.commit()


class FakeOpenAIClient:
    """Stands in for openai.OpenAI (as used for Gemini and Groq)."""

    def __init__(self, reply, finish_reason="stop", refusal=None):
        self.reply = reply
        self.calls = []
        self.finish_reason = finish_reason
        self.refusal = refusal
        self.chat = SimpleNamespace(completions=SimpleNamespace(create=self._create))

    def _create(self, **kwargs):
        self.calls.append(kwargs)
        body = self.reply(kwargs)
        text = body if isinstance(body, str) else json.dumps(body)
        message = SimpleNamespace(content=text, refusal=self.refusal)
        return SimpleNamespace(
            model=kwargs["model"],
            choices=[SimpleNamespace(message=message, finish_reason=self.finish_reason)],
            # 50 thinking tokens are in total_tokens but not completion_tokens.
            usage=SimpleNamespace(prompt_tokens=1000, completion_tokens=150, total_tokens=1200),
        )
