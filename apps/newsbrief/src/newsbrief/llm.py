"""Thin wrapper over the model providers that records every call against a monthly cap.

Anthropic is called through its own SDK. Gemini and Groq are called through their
OpenAI-compatible endpoints with the ``openai`` SDK.
"""

from __future__ import annotations

import os
import sqlite3
from dataclasses import dataclass
from typing import Any

import anthropic
import openai

from . import db
from .config import ModelConfig

FALLBACK_BETA = "server-side-fallback-2026-07-01"
# Cache writes and reads are billed relative to the base input price.
CACHE_WRITE_MULTIPLIER = 1.25
CACHE_READ_MULTIPLIER = 0.1

# provider -> (OpenAI-compatible base URL, environment variable holding the key)
OPENAI_COMPATIBLE = {
    "gemini": ("https://generativelanguage.googleapis.com/v1beta/openai/", "GEMINI_API_KEY"),
    "groq": ("https://api.groq.com/openai/v1", "GROQ_API_KEY"),
}


class BudgetExceeded(RuntimeError):
    """The month's spend has reached the configured cap."""


class LLMError(RuntimeError):
    """The model declined or ran out of tokens."""


@dataclass
class Usage:
    """Token counts for one call, normalized across providers."""

    model: str
    input_tokens: int
    output_tokens: int
    cache_read: int = 0
    cache_write: int = 0


def month_spend(conn: sqlite3.Connection) -> float:
    """Total recorded spend since the start of the current UTC month."""
    row = conn.execute(
        "SELECT COALESCE(SUM(usd), 0) FROM spend WHERE at >= ?", (db.month_start(),)
    ).fetchone()
    return float(row[0])


class LLM:
    """Makes calls with the owner's credentials and keeps a spend ledger in the database."""

    def __init__(
        self,
        conn: sqlite3.Connection,
        prices: dict[str, tuple[float, float]],
        monthly_cap_usd: float,
        client: Any = None,
        clients: dict[str, Any] | None = None,
    ):
        self.conn = conn
        self.prices = prices
        self.cap = monthly_cap_usd
        # Clients are created on first use, so a missing key only matters for providers in use.
        self._clients: dict[str, Any] = dict(clients or {})
        if client is not None:
            self._clients["anthropic"] = client

    def _client(self, provider: str) -> Any:
        if provider not in self._clients:
            if provider == "anthropic":
                self._clients[provider] = anthropic.Anthropic()
            else:
                base_url, env = OPENAI_COMPATIBLE[provider]
                key = os.environ.get(env)
                if not key:
                    raise LLMError(f"{provider}: set {env} in the environment or a .env next to config.toml")
                self._clients[provider] = openai.OpenAI(base_url=base_url, api_key=key)
        return self._clients[provider]

    def call(
        self,
        purpose: str,
        mc: ModelConfig,
        system: str,
        user: str,
        schema: dict | None = None,
    ) -> str:
        """Send one request and return the text of the reply (JSON text when ``schema`` is given)."""
        spent = month_spend(self.conn)
        if spent >= self.cap:
            raise BudgetExceeded(f"spent ${spent:.2f} of ${self.cap:.2f} this month")
        if mc.provider == "anthropic":
            return self._call_anthropic(purpose, mc, system, user, schema)
        return self._call_openai_compatible(purpose, mc, system, user, schema)

    def _call_anthropic(self, purpose: str, mc: ModelConfig, system: str, user: str, schema: dict | None) -> str:
        kwargs: dict[str, Any] = {
            "model": mc.model,
            "max_tokens": mc.max_tokens,
            # The system prompt (rubric + profile) is identical across calls, so cache it.
            "system": [{"type": "text", "text": system, "cache_control": {"type": "ephemeral"}}],
            "messages": [{"role": "user", "content": user}],
        }
        output_config: dict[str, Any] = {}
        if mc.effort:
            output_config["effort"] = mc.effort
        if schema:
            output_config["format"] = {"type": "json_schema", "schema": schema}
        if output_config:
            kwargs["output_config"] = output_config
        if mc.fallbacks:
            # If the model declines, the API reruns the request on a suitable fallback model.
            kwargs["betas"] = [FALLBACK_BETA]
            kwargs["extra_body"] = {"fallbacks": "default"}

        response = self._client("anthropic").beta.messages.create(**kwargs)
        u = response.usage
        self._record(purpose, Usage(
            model=getattr(response, "model", None) or mc.model,
            input_tokens=u.input_tokens,
            output_tokens=u.output_tokens,
            cache_read=getattr(u, "cache_read_input_tokens", 0) or 0,
            cache_write=getattr(u, "cache_creation_input_tokens", 0) or 0,
        ), mc.model)

        if response.stop_reason == "refusal":
            details = getattr(response, "stop_details", None)
            raise LLMError(f"{purpose}: declined ({getattr(details, 'category', None)})")
        if response.stop_reason == "max_tokens":
            raise LLMError(f"{purpose}: hit max_tokens={mc.max_tokens}; raise it in the config")
        return next(b.text for b in response.content if b.type == "text")

    def _call_openai_compatible(
        self, purpose: str, mc: ModelConfig, system: str, user: str, schema: dict | None
    ) -> str:
        kwargs: dict[str, Any] = {
            "model": mc.model,
            "max_tokens": mc.max_tokens,
            "messages": [{"role": "system", "content": system}, {"role": "user", "content": user}],
        }
        if mc.effort:
            # Only reasoning models accept this; leave effort empty for the others.
            kwargs["reasoning_effort"] = mc.effort
        if schema:
            kwargs["response_format"] = {
                "type": "json_schema",
                "json_schema": {"name": purpose, "schema": schema, "strict": True},
            }

        response = self._client(mc.provider).chat.completions.create(**kwargs)
        u = response.usage
        # completion_tokens can leave out thinking tokens, which are billed as output.
        output = max(u.completion_tokens or 0, (getattr(u, "total_tokens", 0) or 0) - u.prompt_tokens)
        # Cache discounts vary by provider, so prompt tokens count at full price; the cap errs high.
        self._record(purpose, Usage(model=mc.model, input_tokens=u.prompt_tokens, output_tokens=output), mc.model)

        choice = response.choices[0]
        if choice.finish_reason == "content_filter" or getattr(choice.message, "refusal", None):
            raise LLMError(f"{purpose}: declined ({choice.message.refusal or choice.finish_reason})")
        if choice.finish_reason == "length":
            raise LLMError(f"{purpose}: hit max_tokens={mc.max_tokens}; raise it in the config")
        return choice.message.content or ""

    def _record(self, purpose: str, usage: Usage, requested_model: str) -> None:
        price_in, price_out = self.prices.get(usage.model) or self.prices.get(requested_model) or (0.0, 0.0)
        usd = (
            usage.input_tokens * price_in
            + usage.cache_write * price_in * CACHE_WRITE_MULTIPLIER
            + usage.cache_read * price_in * CACHE_READ_MULTIPLIER
            + usage.output_tokens * price_out
        ) / 1_000_000
        self.conn.execute(
            "INSERT INTO spend (at, purpose, model, input_tokens, output_tokens, cache_read, cache_write, usd)"
            " VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (db.now(), purpose, usage.model, usage.input_tokens, usage.output_tokens,
             usage.cache_read, usage.cache_write, usd),
        )
        self.conn.commit()
