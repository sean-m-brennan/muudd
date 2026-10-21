# newsbrief

A personal news briefing that runs on your own machine with your own API key. It
ranks stories by how much they matter to *you* (consequence, proximity,
actionability, durability), as described in a profile you write yourself. It never
learns from what you click.

## How it works

```
feeds ──ingest──▶ articles ──cluster──▶ stories ──triage──▶ scores ──brief──▶ briefings/YYYY-MM-DD.md
        (RSS)      SQLite    (local TF-IDF,        (LLM, cheap,     (LLM, top N only:
                              no API calls)         batched)         context + citations)
```

1. **Ingest** fetches RSS/Atom feeds and stores new articles.
2. **Cluster** groups articles about the same event into one *story*, locally. Fifty
   articles about one event become one story with fifty sources, which is where most
   of the volume goes away. Stories close after `window_days` without updates,
   unless you follow them.
3. **Triage** scores each open story 0–10 on the four dimensions against
   `profile.md`, in batches, at low effort. A story is re-scored only after it
   doubles in size.
4. **Brief** takes the top `size` stories above `min_score`, plus one consequential
   story from outside your usual circumstances, and asks the model for background,
   base rates, agreement, disputes, unknowns, and a note on emotionally loaded
   language, with every claim citing a numbered source. Followed stories get a
   "what's new" list without any API call. Nothing is briefed twice.

Every API call is recorded in the `spend` table. Once the month's total reaches
`monthly_cap_usd`, calls stop.

## Setup

```sh
uv venv && uv pip install -e '.[dev]'
mkdir ~/newsbrief && cd ~/newsbrief
newsbrief init            # writes config.toml and profile.md here
$EDITOR profile.md        # this is what the ranking uses
$EDITOR config.toml       # add local and trade-press feeds
export ANTHROPIC_API_KEY=...   # or `ant auth login`, or use Gemini/Groq (below)
newsbrief run --print
```

### Gemini and Groq

Each of `[triage]` and `[context]` takes a `provider`: `anthropic` (default),
`gemini` or `groq`. Put the keys in the environment or in a `.env` beside
`config.toml` (variables already set win):

```sh
GEMINI_API_KEY=...
GROQ_API_KEY=...
```

Both go through the providers' OpenAI-compatible endpoints. Use a model that
supports JSON-schema output (on Groq, `openai/gpt-oss-120b` or `openai/gpt-oss-20b`),
set `effort = ""` for models that don't reason, and add the model to `[prices]`;
a model missing from `[prices]` is recorded at $0 and doesn't count toward the cap.

## Commands

| Command | What it does |
|---|---|
| `newsbrief run [--print]` | ingest, cluster, triage, brief |
| `newsbrief ingest` / `cluster` / `triage` / `brief` | one step at a time |
| `newsbrief stories [--why]` | open stories by score, with the model's rationale |
| `newsbrief follow ID` / `unfollow ID` | track a story beyond the window |
| `newsbrief spend` | this month's API spend by step and model |

Run it from cron or a systemd timer, for example `0 6 * * * cd ~/newsbrief && newsbrief run`.

## Tuning

- **Clustering too aggressive or too loose:** adjust `similarity_threshold`. Check
  with `newsbrief stories` after `cluster`, which costs nothing.
- **Ranking off:** run `newsbrief stories --why` and fix the profile first. Most
  ranking problems are profile problems.
- **Cost:** triage is the high-volume step. It defaults to `claude-opus-5` at
  `effort = "low"`. You can point `[triage]` at a smaller model, but compare the
  `stories --why` output on the same day's news before switching.

## Known limitations (prototype)

- Clustering is TF-IDF over headline and summary. It misses paraphrased coverage
  that shares few words. Swapping in local sentence embeddings is the obvious next step.
- It only sees what feeds put in their summaries; it doesn't fetch full articles.
- No feedback loop yet. A planned `newsbrief rate ID mattered|didnt` would adjust
  the dimension weights in `triage.WEIGHTS`, never inferred from reading behavior.
- With a hosted model, your profile and the article text are sent to the provider
  on each call.

## Tests

```sh
pytest
```

The tests use a fake API client, so they make no network calls and cost nothing.
