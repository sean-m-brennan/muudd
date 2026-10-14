"""SQLite storage. All timestamps are ISO 8601 UTC strings, so they sort as text."""

from __future__ import annotations

import sqlite3
from datetime import UTC, datetime, timedelta
from pathlib import Path

SCHEMA = """
CREATE TABLE IF NOT EXISTS stories (
    id           INTEGER PRIMARY KEY,
    title        TEXT NOT NULL,
    first_seen   TEXT NOT NULL,
    last_updated TEXT NOT NULL,
    followed_at  TEXT              -- NULL unless the owner follows this story
);

CREATE TABLE IF NOT EXISTS articles (
    id        INTEGER PRIMARY KEY,
    url       TEXT NOT NULL UNIQUE,
    source    TEXT NOT NULL,
    title     TEXT NOT NULL,
    summary   TEXT NOT NULL DEFAULT '',
    published TEXT NOT NULL,
    fetched   TEXT NOT NULL,
    story_id  INTEGER REFERENCES stories(id)
);
CREATE INDEX IF NOT EXISTS articles_story ON articles(story_id);

CREATE TABLE IF NOT EXISTS scores (
    story_id      INTEGER PRIMARY KEY REFERENCES stories(id),
    scored_at     TEXT NOT NULL,
    article_count INTEGER NOT NULL,  -- story size when scored; re-scored once it doubles
    consequence   REAL NOT NULL,
    proximity     REAL NOT NULL,
    actionability REAL NOT NULL,
    durability    REAL NOT NULL,
    overall       REAL NOT NULL,
    rationale     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS briefed (
    story_id   INTEGER NOT NULL REFERENCES stories(id),
    briefed_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS spend (
    id            INTEGER PRIMARY KEY,
    at            TEXT NOT NULL,
    purpose       TEXT NOT NULL,
    model         TEXT NOT NULL,
    input_tokens  INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    cache_read    INTEGER NOT NULL,
    cache_write   INTEGER NOT NULL,
    usd           REAL NOT NULL
);
"""


def connect(path: Path | str) -> sqlite3.Connection:
    """Open (and if needed create) the database at ``path``; ``":memory:"`` works for tests."""
    if str(path) != ":memory:":
        Path(path).parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(path)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys = ON")
    conn.executescript(SCHEMA)
    return conn


def iso(dt: datetime) -> str:
    """Format a datetime as a sortable UTC string."""
    return dt.astimezone(UTC).isoformat(timespec="seconds")


def now() -> str:
    return iso(datetime.now(UTC))


def days_ago(days: float) -> str:
    return iso(datetime.now(UTC) - timedelta(days=days))


def month_start() -> str:
    return iso(datetime.now(UTC).replace(day=1, hour=0, minute=0, second=0, microsecond=0))
