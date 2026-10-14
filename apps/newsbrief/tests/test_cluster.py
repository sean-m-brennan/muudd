from newsbrief import db
from newsbrief.cluster import cluster

from conftest import add_article

FED = "Federal Reserve raises interest rates by half a point to fight inflation"
FED_SUM = "The Federal Reserve raised its benchmark interest rate half a point, citing persistent inflation."
FED2 = "Federal Reserve raises interest rates half a point as inflation persists"
FED2_SUM = "Federal Reserve officials voted to raise the benchmark interest rate by half a point, saying inflation remains high."
FIRE = "Wildfire forces evacuations across northern California towns"
FIRE_SUM = "Thousands evacuated as a fast-moving wildfire spread through dry hills."


def test_same_event_clusters_and_others_stay_apart(conn):
    add_article(conn, "u1", FED, FED_SUM, source="A")
    add_article(conn, "u2", FED2, FED2_SUM, source="B")
    add_article(conn, "u3", FIRE, FIRE_SUM, source="C")
    attached, created = cluster(conn, threshold=0.35, window_days=3)
    assert (attached, created) == (1, 2)
    ids = dict(conn.execute("SELECT url, story_id FROM articles").fetchall())
    assert ids["u1"] == ids["u2"] != ids["u3"]


def test_later_run_attaches_to_open_story(conn):
    add_article(conn, "u1", FED, FED_SUM)
    cluster(conn, 0.35, 3)
    add_article(conn, "u2", FED2, FED2_SUM)
    assert cluster(conn, 0.35, 3) == (1, 0)


def test_followed_story_stays_open_past_window(conn):
    old = db.days_ago(30)
    add_article(conn, "u1", FED, FED_SUM, published=old)
    cluster(conn, 0.35, 3)
    add_article(conn, "u2", FED2, FED2_SUM)
    conn.execute("UPDATE stories SET followed_at = ?", (old,))
    assert cluster(conn, 0.35, 3) == (1, 0)


def test_closed_story_does_not_absorb(conn):
    add_article(conn, "u1", FED, FED_SUM, published=db.days_ago(30))
    cluster(conn, 0.35, 3)
    add_article(conn, "u2", FED2, FED2_SUM)
    assert cluster(conn, 0.35, 3) == (0, 1)
