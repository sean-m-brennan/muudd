import feedparser

from newsbrief.ingest import entries_from_feed, store_articles, strip_html

RSS = """<?xml version="1.0"?>
<rss version="2.0"><channel><title>Valley Gazette</title>
<item><title>County approves water rights changes</title>
  <link>https://example.org/water</link>
  <description>&lt;p&gt;The board voted 4-1 &lt;b&gt;Tuesday&lt;/b&gt;.&lt;/p&gt;</description>
  <pubDate>Tue, 13 Oct 2026 18:00:00 GMT</pubDate></item>
<item><title>No link here</title></item>
</channel></rss>"""


def test_strip_html():
    assert strip_html("<p>A &amp; <b>B</b></p>") == "A & B"


def test_entries_and_dedupe(conn):
    articles = entries_from_feed(feedparser.parse(RSS), "Valley Gazette")
    assert len(articles) == 1
    a = articles[0]
    assert a["summary"] == "The board voted 4-1 Tuesday ."
    assert a["published"] == "2026-10-13T18:00:00+00:00"
    assert store_articles(conn, articles) == 1
    assert store_articles(conn, articles) == 0
