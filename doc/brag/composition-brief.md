# Hyperframes Composition Brief: MUUDD (narrated cut)

## Objective

Create a short, narrated launch-style brag video for **MUUDD** — the physical
layer of a user-owned communications network — built entirely out of figures the
project has already published.

## Output

- Composition directory: `lib/muudd/brag-output/composition/`
- Rendered video: `lib/muudd/brag-output/brag.mp4`
- Format: landscape — 1920x1080, 30fps
- Duration: **45.6s**, set by the measured narration (see the voiceover carve-out
  below), not by the default 15–25s law

## Source Material

- Project root: `/home/user/Software/continuous-space/lib/muudd`
- Primary files read:
  - `README.md`
  - `doc/muudd.md`, `doc/hardware_design.md`, `doc/the-stack.md`
  - `doc/rural-network.md` (appendix A — the worked county deployment; this is
    where nearly every on-screen figure comes from)
  - `doc/narrative-order.json`, `docs/network_simulation.md`
  - `run-simulation.sh`, `baseline-results.json`
  - `lib/autonomous_trust/examples/appalachia/scenario.py` (the surveyed node
    coordinates the map is drawn from)
  - `apps/agora/README.md`, `apps/stele/CODE_SHARING_PLAN.md` (read, and
    deliberately not shown — see Scope)
- Product name: **MUUDD** (Mesh-routed User-owned User-controlled Dynamic Domains)
- Tagline / strongest claim: *"Hardware that moves bits without asking permission."*
- Key visual moment to recreate: the Braxton County node map, built from the
  project's own surveyed latitudes and longitudes, with the worked Sutton ↔
  Burnsville link budget beside it.

### Copy that must appear verbatim

- `Payback never arrives.`
- `Hardware that moves bits without asking permission.`
- `The physical layer neither knows nor cares which are present.`
- `who may cooperate?` / `what is a community?` /
  `how do communities treat one another?`
- Every numeric readout listed in the "Show the thing" table of `brag-plan.md`.

### Scope — the one thing this video must not get wrong

MUUDD proper is **only** the physical layer. AutonomousTrust, ethne and
Kith-Covenant are three optional tiers above it, each answering a question the
layer below it cannot, and none of them is required. The separability *is* the
design claim. The three tiers appear exactly once, in scene 6, as dim outlined
rows above a solid base — never as part of MUUDD, and never as features.

Consequences for the composition:

- Do **not** show `apps/agora` or `apps/stele`. Agora is an AutonomousTrust/ethne
  client; Stele is an unbuilt design plan. No Flutter build was attempted.
- Do **not** put `baseline-results.json` on screen. Those are AutonomousTrust
  protocol measurements (identity convergence, reputation stability, negotiation
  RTT, bandwidth overhead), not physical-layer ones.

## Creative Direction

- Tone preset: **cinematic**
- Creative direction: *a surveyor's report, read at night.*
- Interpretation: wide dark frames, monospace figures, long holds, no flourish.
  Nothing celebrates and nothing sells; the cost comparison lands hard because the
  numbers land hard. Restraint tightens further under narration — fewer captions,
  more silence around the voice.
- Angle: **priced to the connector.** The convincing thing about the physical layer
  is not that it is principled but that the arithmetic is finished. The video is a
  costing: a ridge map from the project's own surveyed coordinates and three
  readouts a person could buy equipment from, turning at the end on the scoping
  claim that all of it is the *whole* of MUUDD.
- Hook: slug `BRAXTON COUNTY, WEST VIRGINIA · 516 SQ MI · 24 PEOPLE PER SQUARE
  MILE`, then **Payback never arrives.** The voice says neither.
- Outro / punchline: wordmark **MUUDD**, the expansion, and *Hardware that moves
  bits without asking permission.*
- Avoid:
  - Generic SaaS language
  - Abstract filler visuals
  - Any figure not traceable to the project's own documents
  - Any suggestion that the upper tiers are part of MUUDD

## Visual Identity

MUUDD ships no UI and no stylesheet, so unlike the sibling AutonomousTrust cut
there is no project palette to lift. The palette is authored from the subject — a
ridge at night, a backbone link, a solar array — and stated here so it is
reproducible.

- Background `#0a0e0d` · surface `#111917` · elevated `#18211e` · border `#27332f`
- Ridge silhouette `#1c2622` / `#141c19` (decorative; carries no reading)
- Text `#e6ece9` · muted `#c6d2cd` · dim `#aebab5` — the latter two lifted for
  WCAG AA at video type sizes
- Backbone link / ridgetop relay / 5.8 GHz `#6fd2e0`
- Valley relay `#8fc79a`
- Solar / power `#f2ba5c`
- Fiber column `#f08c7a`
- Display font: generic sans stack. Body/figures: generic mono stack. No font ships
  with the composition and a named family without an `@font-face` is a lint error.
- Visual references from the project: the four node tiers of
  `doc/hardware_design.md`; the surveyed coordinates and node ids of
  `examples/appalachia/scenario.py`; the table layout of
  `doc/rural-network.md` § Cost Comparison.

## Storyboard

Use the storyboard in `brag-output/brag-plan.md` as the creative contract.

1. **Hook** — 6.0s — slug, then **Payback never arrives.**, then its attribution
2. **The county assembles** — 7.45s — 8 ridgetop + 12 valley relays at real
   coordinates, links drawing in, counters running
2b. The readout column carries the four node tiers through scene 2, so it is
   never an empty panel.
3. **The link budget** — 7.2s — nine worked rows, ending on `LINK MARGIN 12.6 dB`
   and `450+ Mbps aggregate`
4. **The power budget** — 7.6s — seven solar-sizing rows, `≈2.8 DAYS AUTONOMY`,
   `$4,920 PER NODE`
5. **The ledger** — 5.45s — FIBER against MESH, five paired rows
6. **The stack** — 6.6s — three dim optional tiers above one solid base, plus the
   verbatim caption
7. **Lockup** — 5.6s — wordmark, expansion, tagline

## Audio

- Audio role: **narration-led** — a low bed under the voice, never a co-star.
- Audio arc: bed enters very low under the hook, opens to 0.22 only in the gaps
  between lines, ducks to 0.13 for the full span of every voice clip, and fades to
  silence over the last 1.0s.
- Music: `assets/music/happy-beats-business-moves-vol-10-by-ende-dot-app-lowpass.mp3`
  (ende.app, 109.96 BPM, 60s). Chosen so this cut does not repeat the
  AutonomousTrust cut's "Beauty Flow" bed.
- Music treatment: the track is bright and upbeat and ducking alone left it in the
  speech band, so what plays is a **low-passed copy** (`ffmpeg -af
  "lowpass=f=2600"`) with the unfiltered original kept beside it. Never above
  **0.22**; ducked to 0.13 under every voice span; 1.0s fade to silence from
  44.6s. `CREDITS.md` carries the command and the reasoning.
- Music cue guidance: the bundled preset at
  `assets/music/cues/…music-cues.json` covers only 0–25s, so the composition is
  timed against `npx hyperframes beats` run on the wired project, which writes
  217 beats for the whole track to
  `beats/assets/music/happy-beats-business-moves-vol-10-by-ende-dot-app.mp3.json`.
  It resolves the track on a 0.2725s half-beat grid (reported 220 BPM); the pulse
  used here is every second point, 0.545s, which agrees with the preset to within
  0.04s inside the preset's own window. Narration wins every conflict.
- Audio-reactive treatment: **subtle** — bass energy breathes the relay halos and
  the link stroke opacity on the map. No waveform, bars, or pulsing type.
- Audio-coupled moments:
  - Ridgetop arrivals (scene 2) — eight beat-grid entrances
  - `LINK MARGIN 12.6 dB` (scene 3) — beat lock at 18.52s (strength 0.85)
  - Power column swap (scene 4) — beat lock at 20.70s (strength 0.90)
  - `≈2.8 DAYS AUTONOMY` (scene 4) — beat lock at 25.06s (strength 0.82)
  - Tier rows (scene 6) — alternating beats 34.34 / 35.43 / 36.52 / 37.61
  - Tagline (scene 7) — beat lock at 42.52s, the strongest beat in the track (1.00)
- SFX selection guidance: sparse, warm, and **low high-frequency-risk only**, per
  `skills/brag/assets/sfx/sfx-analysis.json`. Chosen: `ui/rollover2.ogg` (low) for
  node arrivals, `interface/bong_001.ogg` (low) for the two locked figures and the
  tier rows, `impact/impactSoft_medium_001.ogg` (low) for column swaps and the
  lockup. `interface/drop_001.ogg` was considered and rejected — medium HF risk.
- SFX analysis guidance:
  `/home/user/.claude/plugins/cache/brag/brag/0.2.2/skills/brag/assets/sfx/sfx-analysis.md`
- Audio files: already copied into `composition/assets/{music,sfx,voice}/`.

## Voiceover (enabled — `/brag --voice`)

Seven Kokoro clips, `am_michael` at speed 0.96, on their own ascending track
indices at gain 1.0, already generated into `composition/assets/voice/`. Scene
boundaries are derived from the **measured** WAV durations; nothing is guessed.
Script and the transcribed "As synthesized" pass are in `brag-plan.md`.

The 15–25s creative law is deliberately exceeded under the skill's own carve-out:
*"Scene durations must flex to match the generated audio … Do not hardcode scene
lengths when voiceover is present; let the voice set the pace."*

The narration carries the argument; the frame carries the evidence. No line of
narration reads a figure that is on screen at the same moment.

## Hyperframes Instructions

Load `hyperframes-core`, `hyperframes-animation`, `hyperframes-creative`,
`hyperframes-keyframes`, `hyperframes-cli`. `/brag` owns this workflow — do not
enter the `hyperframes` entry-point intent interview.

Requirements:

- One paused GSAP timeline registered at `window.__timelines["main"]`.
- Three `.clip` sections only (hook, survey frame, lockup); every sub-scene inside
  the survey frame is an opacity/position tween on a child, never a clip.
- Everything spatial precomputed in JS — no tween measures the DOM.
- Show real project material in at least one scene (scene 2 and 3 both do).
- All text passes WCAG AA at video type sizes; `check` is the single gate and must
  report **0 errors**.
- Keep the composition deterministic: no clocks, no unseeded random, no `repeat: -1`.
- Run `npx --yes hyperframes@0.8.47 check --timeout 30000` before render. The
  default 10s navigation timeout is flaky when several renders share the box; the
  failure is the headless browser starting slowly, not the composition.
