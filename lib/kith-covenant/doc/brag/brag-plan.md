# Brag Plan: Kith-Covenant (narrated cut)

Invoked as `/brag --voice`, tone preset `cinematic`, landscape 1920x1080 at 30fps.
The reference implementation for every structural choice is the finished
AutonomousTrust cut at `../../autonomous_trust/doc/brag/`; the shape and rigor are
copied, none of the content is.

## What is this project?

Kith-Covenant (KC) is a protocol for cooperation among communities that do not
share ends and cannot appeal to any authority standing above all of them. It sits
on top of ethne, the polity tier, which may in turn be fed by autonomous-trust.
That substrate handles anonymous reputation *inside* a single community. This tier
handles the other scope: known identity, tied to a specific named human whose
social standing is at stake, binding one community to another across a boundary
neither side rules.

## The angle

**The oldest problem in governance, the inversion that cracks it, and the code
that runs it.** The video moves in one line from a 1259 BCE treaty to a Rust enum
to a live audit commons, and lets the viewer see that the historical argument and
the implementation are the same argument.

The division of labor is the one the AutonomousTrust narrated cut established:
the screen carries the *evidence* — dates, tables, source paths, enum variants,
content-addressed CIDs, lens readouts — and the voice carries the *argument*. The
voice never reads what is on screen, and the screen never repeats a sentence the
voice says.

What makes this distinctly the inter-community tier rather than the trust
substrate below it: nothing in this video is a mesh, a score, or an anonymous
peer. Every frame is about named parties, their own grounds, and a thin procedural
grammar crossing a boundary neither of them rules.

## Hook (first 2-4 seconds)

Monospace slug `PARITY TREATY · HITTITE AND EGYPTIAN · 1259 BCE`, a rule, then
**The oldest problem in governance.**

The voice says neither line. It states the condition underneath them instead, so
the picture and the narration carry two different halves of the same claim.

## Key moments (the middle)

- **The inversion.** A two-column table: what bounded conflict requires, and the
  cooperation primitives that fall out when each ingredient is inverted. Four
  rows, one per beat pair.
- **Rule 1, named.** Four cards — the Trinity, Allah, *Shöñgwa'ya'di:söñ*, the
  Dharma — each under the community that names it, with the sangha's ≈23-century
  run beneath them.
- **The filter, as code.** The verbatim `AntiSpiralFeature` enum from the Rust
  source, each variant arriving beside its plain-language gloss.
- **The ceremony.** Alpha and Beta each seal their own intra-community rite, with
  real DIDs and real archive CIDs; only then does one thin
  `InterCommunityWitness` record cross between them. The audit trail tallies to
  11 entries, reloaded from JSON, all verifying.
- **The lenses.** The project's own capture-signal dashboard, six panels, real
  values over a seeded, crypto-verified commons.

## Outro / punchline

Wordmark **Kith-Covenant**, then *Cooperation without a sovereign.* — the title of
`doc/concept.md`, which the voice deliberately never says.

## Tone

- Preset: **cinematic**
- Creative direction: *a documentary on an argument — long holds, restrained type,
  the evidence left on screen long enough to read.*
- Interpretation: full-bleed dark frames on the project's own dashboard palette,
  crossfades rather than cuts, nothing that editorialises. The two rules are
  stated, not argued for.
- Voice: `am_michael` (Kokoro) at speed 0.96 — measured, unhurried, declarative.
  Not a trailer announcer.

## Format: landscape — 1920x1080
## Duration: 52.0s, set by the measured narration

The 15–25s creative law is deliberately exceeded under the skill's own voiceover
carve-out: *"Scene durations must flex to match the generated audio … Do not
hardcode scene lengths when voiceover is present; let the voice set the pace."*
Seven Kokoro clips measure 46.46s of speech; with ~0.6s of air between lines and a
1.2s tail that lands the cut at 52.0s. The first pass of the script ran to 52.4s
of speech alone and was cut back line by line until the total landed here.

## Visual identity (from the project)

Lifted from the project's own dashboard theme, dark mode:
`src/python/dashboard/static/index.html`

- Plane `#0d0d0d`; surface `#1a1a19`; elevated `#232321`; border lifted to `#34342f`
- Ink `#ffffff`; ink-2 `#c3c2b7`
- Series `#3987e5`; good `#0ca30c`; warning `#fab219`
- Three tokens are lifted from the shipped values because they fail WCAG AA at
  video type sizes over `--surface`, with hue and the ink > ink-2 > muted ordering
  preserved:
  - muted `#898781` → `#a3a199` (shipped value is 4.85:1 — passing, but with no margin)
  - series-1 `#3987e5` → `#5a9df0` for text only (4.78:1); the chart stroke keeps `#3987e5`
  - critical `#d03b3b` → `#e8736b` (shipped value is 3.62:1 — **fails** AA)
- Type stacks are generic on purpose; the composition ships no `@font-face`, so
  the renderer resolves them deterministically (Roboto / JetBrains Mono).

## Where every on-screen fact comes from

Nothing on screen is invented. Each item, with its source:

| On screen | Source |
|---|---|
| Parity treaty, Hittite and Egyptian, 1259 BCE | `README.md`; `doc/concept.md` |
| "The oldest problem in governance." | `README.md` — "The problem is the oldest one in governance." |
| The four inversion rows (peers / symmetric rules / protected channels / defined ending → the four primitives) | `doc/concept.md`, "The central move" |
| Never-target categories: life support, rescue, children, communications | `doc/concept.md` |
| the Trinity · Allah · *Shöñgwa'ya'di:söñ* · the Dharma, with their communities | `README.md` rule 1; `doc/concept.md` |
| ≈23 centuries, the Buddhist sangha, "the single longest-running voluntary cross-community network in the set" | `doc/substrate.md` |
| `pub enum AntiSpiralFeature { … }`, all five variants verbatim | `src/rust/kc-ceremony/src/inter/anti_spiral.rs` |
| The five plain-language glosses | `README.md` rule 3, verbatim |
| Alpha / Beta DIDs, intra-rite CIDs, `InterCommunityWitness` CID, `alpha-beta-compact`, the five audit-trail record types and their counts, "11 entries · reloaded from JSON · all verify OK" | a real run of `src/python/examples/two_communities_demo.py` (transcript below) |
| "✓ 25 entries verify"; sponsor taint 50% / sponsees 2 / tainted 1; composition drift 2; witness counts 5 → 3 → 1 with "witnesses declined · gaps widening"; feature concern `EnemyLoveOperational` 1 and `NonUltimacyOfCompact` 1 with 2 evidence CIDs; rejection pattern 2 citing `FallenActorAnthropology`; standing **Anathema** with 1 conduct record, 0 rites, 0 disputes | the live dashboard, served from `src/python/dashboard` over the commons built by `dashboard/seed.py` |
| "Adjudication is the mediator pool's work." | `src/python/dashboard/static/index.html`, verbatim |
| "Cooperation without a sovereign." | title of `doc/concept.md` |

### On running the demo

`run-demo.sh` requires a `conda` env named `kith-covenant`; there is no conda on
this machine, and the dashboard's `fastapi`/`uvicorn` are not installed. Rather
than fall back to source-reading, the dashboard was **actually run**: the compiled
`kith_covenant` extension is importable under the system Python 3.11, so
`dashboard/seed.py` builds and verifies its commons unchanged, and a stdlib-only
stand-in for `dashboard/app.py` (same routes, same projections, written to the
session scratchpad, nothing added to the repo) served `dashboard/static/index.html`
unmodified. Every number in the lens scene was read off that rendered page and off
the JSON its endpoints returned. `src/python/examples/two_communities_demo.py`,
`capture_signals_demo.py` and `escalation_demo.py` all ran to `SUCCESS` directly.

Not run: `cargo test --workspace` (no build tree present; a full workspace build
was not worth the disk on a partition at 90%). The README's figure of 379 unit and
property tests is therefore cited but **not** put on screen.

## Share copy (draft)

Kith-Covenant. Two communities with different gods, different languages and
different loyalties, cooperating with no authority standing above either — the
problem the Hittites and Egyptians already had in 1259 BCE, worked out as a
protocol with fifteen Rust crates behind it.

## Audio direction

- Role: **narration-led.** Music is a bed underneath the voice, not a co-star.
- Music: `happy-beats-business-moves-vol-12-by-ende-dot-app.mp3` (ende.app),
  tempo 109.96 BPM, 117.36s — the track the `cinematic` preset calls for.
- Music treatment: the source is bright for a bed sitting under speech, so what
  plays is a low-passed copy (`ffmpeg -af lowpass=f=2600`, measured **−7.4 dB**
  above 4 kHz against the original). The unfiltered file is kept beside it in
  `composition/assets/music/`.
- Level: enters at 0.10 under the hook, rises to **0.13** by 1.0s and holds there
  beneath every narration line, opens to 0.16 across the lockup, and fades to
  silence over the last 1.2s. The gaps between lines are only ~0.6s — too short to
  open a bed into without audible pumping — so the level simply stays down for the
  whole argument rather than breathing between lines.
- Voice track: per-scene WAVs on their own tracks at gain 1.0, so every scene
  boundary is timed to measured audio rather than guessed.
- Music cue guidance: the bundled preset analyses only the first 25s of the track.
  Strong cues inside that window are used as strong cues; past 25s the grid is
  **extrapolated** on the measured spacing, `t = 0.5573 + n × 0.5455`, and used
  only for smaller entrances. Narration wins any conflict.
- Audio-reactive treatment: **none.** This is a typographic and documentary cut,
  not a mesh; there is nothing here for bass energy to breathe.
- SFX posture: sparse and warm, only from the low-high-frequency-risk list in
  `sfx-analysis.md` — `ui/rollover2` for sequential arrivals, `interface/bong_001`
  for the two structural reveals, `interface/click_002` for record rows,
  `impact/impactSoft_medium_001` once on the wordmark. Nothing editorialises.
- Restraint rule: no risers, no swells. The voice never raises. If a cue and a
  word collide, the word wins.

## Voiceover script

Seven clips, `hyperframes tts --voice am_michael --speed 0.96`. Each is written to
complement what is on screen rather than read it.

1. **hook** — "Two communities. No shared creed, no shared ruler, and nothing standing above them both."
2. **inversion** — "The move is an inversion. Don't ask how strangers cooperate. Ask how they have fought within limits."
3. **rule-one** — "Each community anchors to whatever it holds above itself. The compact stays neutral on what that is."
4. **rule-two** — "The compact itself never becomes the ultimate. Membership needs five features a tradition already carries."
5. **ceremony** — "Recognition runs in two layers. Each side seals its own rite. A thin grammar crosses the boundary."
6. **lenses** — "Every act lands in a signed commons. The lenses surface flags with provenance, and render no verdict."
7. **lockup** — "Kith-Covenant. Cooperation among communities that answer to something above themselves."

### As synthesized

Transcribed from the delivered WAVs with `whisper.cpp` (`ggml-base.en`), which is
the authority for what the voice actually carries. Unlike the AutonomousTrust run,
**Kokoro dropped nothing**: all seven clips came back complete, word for word.

1. *"two communities. No shared creed, no shared ruler, and nothing standing above them both."*
2. *"The move is an inversion. Don't ask how strangers cooperate. Ask how they have fought within limits."*
3. *"Each community anchors to whatever it holds above itself. The compact stays neutral on what that is."*
4. *"The compact itself never becomes the ultimate. Membership needs five features a tradition already carries."*
5. *"Recognition runs in two layers. Each side seals its own right. A thin grammar crosses the boundary."*
6. *"Every act lands in assigned commons. The lenses surface flags with provenance and render no verdict."*
7. *"'Kith Covenant,' 'cooperation among communities that answer to something above themselves.'"*

Two items in that list are ASR artifacts, not TTS defects: `base.en` has no way to
choose between the homophones *rite* / *right* (clip 5) or *a signed* / *assigned*
(clip 6), and it punctuates clip 7 as quoted speech. The audio is correct in all
three places. "Kith" is pronounced correctly by Kokoro — whisper recognizes the
word — so no phonetic respelling was needed.

An earlier draft of the script measured 52.4s of speech on its own, which would
have pushed the cut past 58s. Clips 1, 2, 4 and 5 were rewritten shorter and
regenerated; clip 1 in particular lost the spoken date, which now lives only in
the on-screen slug where it belongs.

## Storyboard

Durations are set by the measured narration clips; the beats below fix the order
and what must be visible.

### Scene 1 — Hook · 0 → 7.10
Slug, rule, then the claim. No panels, no product. Holds for five seconds.
Sequential/interaction: none.
Audio intent: bed enters at 0.10; the voice is the first thing you notice.
Transition: crossfade → Scene 2

### Scene 2 — The inversion · 7.10 → 14.35
Two-column table. Column heads, then four rows, one every other beat (1.091s — the
readable spacing, since the grid itself is 0.545s). Source file cited at the foot.
Sequential/interaction: yes — four rows, all persisting.
Audio intent: one quiet tick per row; the voice states the move, the table shows it.
Transition: crossfade → Scene 3

### Scene 3 — Rule 1, named · 14.35 → 21.50
Four cards arrive on the grid; cards three and four land on strong cues 17.468 and
18.559. The ≈23-century figure settles beneath them on strong cue 19.650.
Sequential/interaction: yes — four cards, then the callout.
Audio intent: quiet ticks. Nothing about this scene is emphatic; the claim is
simply stated.
Transition: crossfade → Scene 4

### Scene 4 — The filter, as code · 21.50 → 29.45
The source panel opens at 21.832, then each enum variant lights and its gloss
arrives beside it, every other beat from strong cue 22.923.
Sequential/interaction: yes — five paired reveals.
Audio intent: one warm accent as the file opens, then ticks.
Transition: crossfade → Scene 5

### Scene 5 — The ceremony · 29.45 → 37.40
Alpha enters from the left, Beta from the right, each with its own DID and its own
sealed intra-rite CID. Only at 32.742 does the `InterCommunityWitness` bar scale
in between them. The audit-trail tally and the re-verification land at 34.924.
Sequential/interaction: yes — two cards, then the bridge, then the tally.
Audio intent: record clicks for the two rites, one warm accent for the crossing.
Transition: crossfade → Scene 6

### Scene 6 — The lenses · 37.40 → 45.10
The project's own dashboard: topbar with the verify badge, the mediator-pool note,
then six panels every other beat from 38.197. The decay line draws itself 5 → 3 → 1.
Sequential/interaction: yes — six cards plus one drawn line.
Audio intent: the quietest ticks in the cut. Nothing animates a verdict.
Transition: crossfade → Scene 7

### Scene 7 — Lockup · 45.10 → 52.00
Wordmark at 45.834, rule, then the tagline at 47.470 and the music credit at
48.561. Bed opens a hair, then fades to silence.
Audio intent: settle and release.

**Music mood for this video:** cinematic — low, filtered, subordinate to speech.

**Audio summary:** a low-passed bed sits at 0.13 under seven narration clips for
the whole argument, opens a hair across the lockup, and fades out under the last
line.

---

## Build notes (final figures)

**Final duration: 52.0s.** Seven Kokoro clips (`am_michael`, speed 0.96) totaling
46.464s of speech, plus ~0.6s of air between lines.

| Clip | Measured | Starts at | Ends |
|---|---|---|---|
| `vo-1-hook` | 5.973s | 0.90 | 6.87 |
| `vo-2-inversion` | 6.549s | 7.47 | 14.02 |
| `vo-3-rule-one` | 6.635s | 14.63 | 21.27 |
| `vo-4-rule-two` | 7.296s | 21.87 | 29.17 |
| `vo-5-ceremony` | 7.339s | 29.77 | 37.11 |
| `vo-6-lenses` | 7.083s | 37.71 | 44.79 |
| `vo-7-lockup` | 5.589s | 45.40 | 50.99 |

Scene spans that follow from those clips: hook 0–7.10 · inversion 7.10–14.35 ·
anchors 14.35–21.50 · filter 21.50–29.45 · ceremony 29.45–37.40 ·
lenses 37.40–45.10 · lockup 45.10–52.00.

### Where the picture meets the word

- Inversion rows ride the grid under VO 2: 8.194 / 9.285 / 10.376 / 11.467, one
  every other beat.
- The Haudenosaunee and Buddhist cards land on strong cues **17.468** and
  **18.559**, inside *"the compact stays neutral on what that is"* — the two
  substrates furthest from the viewer's default are the ones that land on the beat.
- The ≈23-century figure lands on strong cue **19.650**, in the tail of VO 3.
- The source file opens at **21.832**, and the first enum variant lights on strong
  cue **22.923**, which is where VO 4 reaches *"never becomes the ultimate."*
- The two intra-rites seal at 30.014 and 31.105; the inter-community record does
  not cross until **32.742**, under *"a thin grammar crosses the boundary."* The
  order on screen is the order the protocol requires.
- Six lens cards on alternating beats 38.197 → 43.652; the decay line draws from
  40.6 over 1.0s.
- Wordmark on the grid at **45.834**; the tagline at **47.470** — the voice says
  neither.

### Audio outcome (measured off the render)

Voice spans sit at ≈ −23.5 dB mean with peaks −3.3 to −4.8 dB; the gaps between
lines drop to −32 to −38 dB (bed only), so speech clears the music by roughly 9 dB
throughout, matching the AutonomousTrust cut. The bed's energy above 4 kHz is
−7.4 dB against the unfiltered source.

### Captions deliberately not written

There are no floating captions in this cut. Everything on screen is either a label
on evidence (a column head, a source path, a panel title, a key) or one of the two
lines the voice never says: the hook and the closing tagline. Any sentence the
narration carries was kept off the screen rather than duplicated.

### Deliverables

- `brag.mp4` — 1920x1080, 52.0s, h264 + AAC 48 kHz stereo, 3.4 MB
- `brag.jpg` — poster pulled at 20.8s (Rule 1, all four substrates settled), baked
  as frame 0 of the MP4 so every platform's thumbnail grabber picks it up
- `brag.webm` — VP9 + Opus, 2.8 MB
- `composition-brief.md`, `share-copy.txt`, `CREDITS.md`, `build.sh`, `composition/`

`hyperframes check`: **0 errors**, 64/64 text checks pass WCAG AA, 0 layout issues
across 9 samples, 0 motion errors. The one remaining warning is
`composition_file_too_large` — accepted rather than split, same as the
AutonomousTrust cut, since the scenes share one palette and one timeline.

Note: `check` is flaky on this machine under concurrent load and will
intermittently report `Navigation timeout of 10000 ms exceeded`. That is the
headless browser starting slowly, not a composition defect; `--timeout 30000` is
baked into `package.json` and `build.sh` so the gate is reproducible.
