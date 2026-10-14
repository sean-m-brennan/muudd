"""Group articles into stories locally, with no API calls.

Each unassigned article joins the open story whose TF-IDF centroid it most
resembles, or starts a new story. A story is open while it has been updated
within the window, or indefinitely if the owner follows it.
"""

from __future__ import annotations

import sqlite3

import numpy as np
from sklearn.feature_extraction.text import TfidfVectorizer

from . import db

MAX_FEATURES = 4096


def _text(row: sqlite3.Row) -> str:
    # Headlines carry most of the signal, so count them twice.
    return f"{row['title']} {row['title']} {row['summary']}"


def cluster(conn: sqlite3.Connection, threshold: float, window_days: int) -> tuple[int, int]:
    """Assign every unclustered article to a story. Returns (attached, new_stories)."""
    pending = conn.execute(
        "SELECT id, title, summary, published FROM articles"
        " WHERE story_id IS NULL ORDER BY published, id"
    ).fetchall()
    if not pending:
        return 0, 0
    members = conn.execute(
        "SELECT a.title, a.summary, a.story_id FROM articles a"
        " JOIN stories s ON s.id = a.story_id"
        " WHERE s.last_updated >= ? OR s.followed_at IS NOT NULL",
        (db.days_ago(window_days),),
    ).fetchall()

    vectorizer = TfidfVectorizer(
        stop_words="english", max_features=MAX_FEATURES, sublinear_tf=True, dtype=np.float32
    )
    try:
        matrix = vectorizer.fit_transform([_text(r) for r in [*members, *pending]]).toarray()
    except ValueError:  # every document was empty or stop words
        matrix = np.zeros((len(members) + len(pending), 1), dtype=np.float32)

    # Running sums of member vectors, one row per open story.
    story_ids: list[int] = []
    rows: dict[int, int] = {}
    sums = np.zeros((0, matrix.shape[1]), dtype=np.float32)
    for vec, m in zip(matrix[: len(members)], members):
        if m["story_id"] not in rows:
            rows[m["story_id"]] = len(story_ids)
            story_ids.append(m["story_id"])
            sums = np.vstack([sums, vec])
        else:
            sums[rows[m["story_id"]]] += vec

    attached = created = 0
    for vec, article in zip(matrix[len(members) :], pending):
        best = -1
        if len(story_ids) and vec.any():
            norms = np.linalg.norm(sums, axis=1)
            norms[norms == 0] = 1.0
            sims = sums @ vec / norms
            if sims.max() >= threshold:
                best = int(sims.argmax())
        if best >= 0:
            story_id = story_ids[best]
            sums[best] += vec
            conn.execute(
                "UPDATE stories SET last_updated = MAX(last_updated, ?) WHERE id = ?",
                (article["published"], story_id),
            )
            attached += 1
        else:
            story_id = conn.execute(
                "INSERT INTO stories (title, first_seen, last_updated) VALUES (?, ?, ?)",
                (article["title"], article["published"], article["published"]),
            ).lastrowid
            story_ids.append(story_id)
            sums = np.vstack([sums, vec])
            created += 1
        conn.execute("UPDATE articles SET story_id = ? WHERE id = ?", (story_id, article["id"]))
    conn.commit()
    return attached, created
