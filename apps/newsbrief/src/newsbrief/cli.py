"""Command-line entry point."""

from __future__ import annotations

import argparse
import os
import sys
from importlib import resources
from pathlib import Path

from . import brief, cluster, config, db, ingest, triage
from .llm import LLM, BudgetExceeded, LLMError, month_spend


def log(message: str) -> None:
    print(message, file=sys.stderr)


def cmd_init(args: argparse.Namespace) -> int:
    target = Path(args.dir)
    target.mkdir(parents=True, exist_ok=True)
    for name in ("config.toml", "profile.md"):
        dest = target / name
        if dest.exists():
            log(f"exists, left alone: {dest}")
            continue
        dest.write_text(resources.files("newsbrief.templates").joinpath(name).read_text(encoding="utf-8"))
        log(f"wrote {dest}")
    log("Next: edit profile.md and the feed list in config.toml, then run `newsbrief run`.")
    return 0


def _llm(conn, cfg: config.Config) -> LLM:
    return LLM(conn, cfg.prices, cfg.monthly_cap_usd)


def cmd_ingest(conn, cfg, args) -> int:
    log(f"ingest: {ingest.ingest(conn, cfg.feeds, log=log)} new articles")
    return 0


def cmd_cluster(conn, cfg, args) -> int:
    attached, created = cluster.cluster(conn, cfg.similarity_threshold, cfg.window_days)
    log(f"cluster: {attached} articles joined existing stories, {created} new stories")
    return 0


def cmd_triage(conn, cfg, args) -> int:
    n = triage.triage(
        conn, _llm(conn, cfg), cfg.triage, cfg.profile(), cfg.window_days,
        cfg.triage_batch_size, cfg.triage_max_per_run, log=log,
    )
    log(f"triage: {n} stories scored")
    return 0


def cmd_brief(conn, cfg, args) -> int:
    path = brief.build_briefing(conn, _llm(conn, cfg), cfg, log=log)
    log(f"brief: wrote {path}")
    if args.print:
        print(path.read_text(encoding="utf-8"))
    return 0


def cmd_run(conn, cfg, args) -> int:
    for step in (cmd_ingest, cmd_cluster, cmd_triage, cmd_brief):
        step(conn, cfg, args)
    return 0


def cmd_stories(conn, cfg, args) -> int:
    rows = conn.execute(
        "SELECT s.id, s.title, s.followed_at, sc.overall, sc.rationale,"
        " (SELECT COUNT(*) FROM articles a WHERE a.story_id = s.id) AS n"
        " FROM stories s LEFT JOIN scores sc ON sc.story_id = s.id"
        " WHERE s.last_updated >= ? OR s.followed_at IS NOT NULL"
        " ORDER BY sc.overall IS NULL, sc.overall DESC, n DESC LIMIT ?",
        (db.days_ago(cfg.window_days), args.limit),
    ).fetchall()
    for r in rows:
        score = "  -  " if r["overall"] is None else f"{r['overall']:4.1f}"
        mark = "*" if r["followed_at"] else " "
        print(f"{r['id']:>6} {score} {mark} ({r['n']:>2}) {r['title']}")
        if args.why and r["rationale"]:
            print(f"{'':>15}{r['rationale']}")
    return 0


def cmd_follow(conn, cfg, args) -> int:
    value = db.now() if args.command == "follow" else None
    cur = conn.execute("UPDATE stories SET followed_at = ? WHERE id = ?", (value, args.story_id))
    conn.commit()
    if not cur.rowcount:
        log(f"no story {args.story_id}")
        return 1
    log(f"{args.command}ed story {args.story_id}")
    return 0


def cmd_spend(conn, cfg, args) -> int:
    print(f"This month: ${month_spend(conn):.4f} of ${cfg.monthly_cap_usd:.2f}")
    for r in conn.execute(
        "SELECT purpose, model, COUNT(*) calls, SUM(input_tokens) i, SUM(output_tokens) o,"
        " SUM(cache_read) cr, SUM(usd) usd FROM spend WHERE at >= ? GROUP BY purpose, model",
        (db.month_start(),),
    ):
        print(f"  {r['purpose']:<8} {r['model']:<24} {r['calls']:>4} calls  in {r['i']:>8}"
              f"  out {r['o']:>8}  cached {r['cr']:>8}  ${r['usd']:.4f}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="newsbrief", description=__doc__)
    parser.add_argument(
        "--config", default=os.environ.get("NEWSBRIEF_CONFIG", "config.toml"),
        help="path to config.toml (default: ./config.toml or $NEWSBRIEF_CONFIG)",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("init", help="write starter config.toml and profile.md")
    p.add_argument("dir", nargs="?", default=".")
    p.set_defaults(func=cmd_init, needs_db=False)

    for name, func, text in [
        ("ingest", cmd_ingest, "fetch feeds"),
        ("cluster", cmd_cluster, "group new articles into stories (local, no API calls)"),
        ("triage", cmd_triage, "score stories against your profile"),
        ("brief", cmd_brief, "write today's briefing"),
        ("run", cmd_run, "ingest, cluster, triage, and brief"),
        ("spend", cmd_spend, "show this month's API spend"),
    ]:
        p = sub.add_parser(name, help=text)
        p.set_defaults(func=func, needs_db=True)
        if name in ("brief", "run"):
            p.add_argument("--print", action="store_true", help="also print the briefing")

    p = sub.add_parser("stories", help="list open stories, highest score first")
    p.add_argument("--limit", type=int, default=30)
    p.add_argument("--why", action="store_true", help="show each score's rationale")
    p.set_defaults(func=cmd_stories, needs_db=True)

    for name in ("follow", "unfollow"):
        p = sub.add_parser(name, help=f"{name} a story by id")
        p.add_argument("story_id", type=int)
        p.set_defaults(func=cmd_follow, needs_db=True)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if not args.needs_db:
        return args.func(args)
    path = Path(args.config)
    if not path.exists():
        log(f"no config at {path}; run `newsbrief init` first")
        return 2
    config.load_env(path.resolve().parent / ".env")
    cfg = config.load(path)
    conn = db.connect(cfg.db_path)
    try:
        return args.func(conn, cfg, args)
    except (BudgetExceeded, LLMError) as e:
        log(f"stopped: {e}")
        return 1
    finally:
        conn.close()


if __name__ == "__main__":
    sys.exit(main())
