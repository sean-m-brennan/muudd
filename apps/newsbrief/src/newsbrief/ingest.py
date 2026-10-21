"""Fetch RSS/Atom feeds and store new articles."""

from __future__ import annotations

import html
import re
import sqlite3
from collections.abc import Callable, Iterable
from datetime import UTC, datetime
from urllib.parse import urlparse

import feedparser

from . import db

USER_AGENT = "newsbrief/0.1 (+personal news reader)"
SUMMARY_LIMIT = 1000

_TAG = re.compile(r"<[^>]+>")
_SPACE = re.compile(r"\s+")


def strip_html(text: str) -> str:
    """Reduce feed HTML to plain text."""
    return _SPACE.sub(" ", html.unescape(_TAG.sub(" ", text or ""))).strip()


def entries_from_feed(parsed: feedparser.FeedParserDict, source: str) -> list[dict]:
    """Turn a parsed feed into article dicts ready for :func:`store_articles`."""
    fetched = db.now()
    articles = []
    for entry in parsed.entries:
        url = entry.get("link")
        title = strip_html(entry.get("title", ""))
        if not url or not title:
            continue
        stamp = entry.get("published_parsed") or entry.get("updated_parsed")
        published = db.iso(datetime(*stamp[:6], tzinfo=UTC)) if stamp else fetched
        articles.append(
            {
                "url": url,
                "source": source,
                "title": title,
                "summary": strip_html(entry.get("summary", ""))[:SUMMARY_LIMIT],
                "published": published,
                "fetched": fetched,
            }
        )
    return articles


def store_articles(conn: sqlite3.Connection, articles: Iterable[dict]) -> int:
    """Insert articles, skipping URLs already stored. Returns the number added."""
    added = 0
    for a in articles:
        cur = conn.execute(
            "INSERT OR IGNORE INTO articles (url, source, title, summary, published, fetched)"
            " VALUES (:url, :source, :title, :summary, :published, :fetched)",
            a,
        )
        added += cur.rowcount
    conn.commit()
    return added


def ingest(conn: sqlite3.Connection, feed_urls: Iterable[str], log: Callable[[str], None] = print) -> int:
    """Fetch every feed and store new articles. Returns the total added."""
    total = 0
    for url in feed_urls:
        parsed = feedparser.parse(url, agent=USER_AGENT)
        if parsed.get("bozo") and not parsed.entries:
            log(f"  skipped {url}: {parsed.get('bozo_exception')}")
            continue
        source = parsed.feed.get("title") or urlparse(url).netloc
        added = store_articles(conn, entries_from_feed(parsed, source))
        log(f"  {source}: {added} new")
        total += added
    return total
