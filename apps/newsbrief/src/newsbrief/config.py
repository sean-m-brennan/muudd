"""Load newsbrief settings from a TOML file."""

from __future__ import annotations

import os
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

PROVIDERS = ("anthropic", "gemini", "groq")
DEFAULT_MODELS = {
    "anthropic": "claude-opus-5",
    "gemini": "gemini-2.5-flash",
    "groq": "openai/gpt-oss-120b",
}


@dataclass
class ModelConfig:
    """Settings for one class of LLM call (triage or context)."""

    model: str
    effort: str = ""
    max_tokens: int = 16000
    fallbacks: bool = True  # Anthropic only
    provider: str = "anthropic"


@dataclass
class Config:
    """All runtime settings, with paths already resolved."""

    db_path: Path
    profile_path: Path
    output_dir: Path
    feeds: list[str]
    triage: ModelConfig
    context: ModelConfig
    similarity_threshold: float = 0.35
    window_days: int = 3
    triage_batch_size: int = 15
    triage_max_per_run: int = 100
    brief_size: int = 5
    min_score: float = 5.0
    monthly_cap_usd: float = 10.0
    prices: dict[str, tuple[float, float]] = field(default_factory=dict)

    def profile(self) -> str:
        """Return the owner's profile text."""
        return self.profile_path.read_text(encoding="utf-8")


def _resolve(base: Path, value: str) -> Path:
    path = Path(value).expanduser()
    return path if path.is_absolute() else base / path


def _model(section: dict, default_effort: str, default_max: int) -> ModelConfig:
    provider = section.get("provider", "anthropic")
    if provider not in PROVIDERS:
        raise ValueError(f"unknown provider {provider!r}; expected one of {', '.join(PROVIDERS)}")
    return ModelConfig(
        provider=provider,
        model=section.get("model", DEFAULT_MODELS[provider]),
        effort=section.get("effort", default_effort),
        max_tokens=int(section.get("max_tokens", default_max)),
        fallbacks=bool(section.get("fallbacks", True)),
    )


def load_env(path: Path) -> None:
    """Set variables from a KEY=VALUE file, leaving ones already in the environment alone."""
    if not path.is_file():
        return
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.removeprefix("export ").split("=", 1)
        os.environ.setdefault(key.strip(), value.strip().strip("'\""))


def load(path: Path) -> Config:
    """Parse the config file at ``path``."""
    raw = tomllib.loads(path.read_text(encoding="utf-8"))
    base = path.resolve().parent
    paths = raw.get("paths", {})
    cluster = raw.get("cluster", {})
    triage = raw.get("triage", {})
    brief = raw.get("brief", {})
    return Config(
        db_path=_resolve(base, paths.get("db", "newsbrief.db")),
        profile_path=_resolve(base, paths.get("profile", "profile.md")),
        output_dir=_resolve(base, paths.get("output", "briefings")),
        feeds=list(raw.get("feeds", {}).get("urls", [])),
        triage=_model(triage, "low", 8000),
        context=_model(raw.get("context", {}), "high", 16000),
        similarity_threshold=float(cluster.get("similarity_threshold", 0.35)),
        window_days=int(cluster.get("window_days", 3)),
        triage_batch_size=int(triage.get("batch_size", 15)),
        triage_max_per_run=int(triage.get("max_stories_per_run", 100)),
        brief_size=int(brief.get("size", 5)),
        min_score=float(brief.get("min_score", 5.0)),
        monthly_cap_usd=float(raw.get("budget", {}).get("monthly_cap_usd", 10.0)),
        prices={k: (float(v[0]), float(v[1])) for k, v in raw.get("prices", {}).items()},
    )
