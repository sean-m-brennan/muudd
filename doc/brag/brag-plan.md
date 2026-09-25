# Brag Plan: MUUDD (narrated cut)

Invoked as `/brag --voice`, so narration is on for this run only. Sibling run:
`lib/autonomous_trust/doc/brag/` (AutonomousTrust, "beauty-flow" bed). This cut
uses a different bed, a different angle, and stays strictly on the physical layer.

## What is this project?

MUUDD — Mesh-routed User-owned User-controlled Dynamic Domains — is a
communications network that belongs to the people using it. Solar-powered radios
on ridgetops and rooftops, owned by the households they serve, passing traffic to
each other directly instead of through a carrier's equipment.

**Scope, which the video has to get right.** MUUDD proper is *only* the physical
layer: hardware that moves bits without asking permission. Three further tiers can
stack on top — AutonomousTrust (who may cooperate?), ethne (what is a community?),
Kith-Covenant (how do communities treat one another?) — and each answers a question
the layer below it cannot. None of the three is required. A network can run with
none, with one, or with all three, and the physical layer neither knows nor cares
which. That separability *is* the design claim, so the video shows the three tiers
exactly once, as optional rows above a solid base, and never confuses them with
MUUDD itself. Each of the three is getting its own video in this batch.

## The angle

**Priced to the connector.**

The most convincing thing about the physical layer is not that it is principled.
It is that the arithmetic is finished. The project has worked one Appalachian
county end to end — terrain, link budget, solar sizing, bill of materials, cost
against fiber — and every figure in this video comes off that page. So the video
is a survey document read aloud: a ridge map built from the project's own surveyed
node coordinates, and three readouts beside it that a person could actually buy
equipment from.

The turn at the end is the scoping claim. Everything the viewer has just watched —
the map, the link budget, the power budget, the ledger — is the *whole* of MUUDD.
Three tiers sit above it and not one of them is required.

This is not a mesh-networking explainer. It is a costing.

## Hook (first 2-4 seconds)

Dark frame. Monospace slug:
`BRAXTON COUNTY, WEST VIRGINIA · 516 SQ MI · 24 PEOPLE PER SQUARE MILE`
then the line, verbatim from the study:

> **Payback never arrives.**

with its attribution underneath (`doc/rural-network.md · the fiber arithmetic`).
The voice does not say that line. It says what it costs instead.

## Key moments (the middle)

- **The county assembles.** Eight ridgetop relays land one per beat at their real
  surveyed coordinates, then twelve valley relays, then the links between them.
  The Sutton ↔ Burnsville backbone hop stays lit.
- **The link budget.** Nine rows of the worked 22 km / 5.8 GHz example land beside
  the map, ending on `LINK MARGIN 12.6 dB` and `450+ Mbps`.
- **The power budget.** 70 W continuous, worked backwards from December: 1,680 Wh
  a day, 700 W of panel, a 200 Ah LiFePO4 bank, ≈2.8 days of autonomy.
- **The ledger.** Fiber against mesh, side by side. $38,200,000 against $1,278,607.
  $46,000–$78,000 per demand point against $527. $70 a month against $2.12.
- **The stack.** The map recedes. Three optional tiers above one solid base.

## Outro / punchline

Wordmark **MUUDD**, the expansion, and *Hardware that moves bits without asking
permission.* — verbatim from `README.md`.

## Tone

- Preset: **cinematic**
- Creative direction: *a surveyor's report, read at night.* Restrained, unhurried,
  and entirely made of numbers that already exist.
- Interpretation: wide dark frames, long holds, monospace figures, no flourish.
  Nothing celebrates, nothing sells. The cost comparison is allowed to land hard
  because the numbers do the landing.
- Voice: `am_michael` (Kokoro) at speed 0.96 — the same voice as the
  AutonomousTrust cut, so the batch reads as one family.

## Format: landscape — 1920x1080, 30fps
## Duration: 45.6s, set by the measured narration

The 15–25s creative law is deliberately exceeded under the skill's own voiceover
carve-out: *"Scene durations must flex to match the generated audio … Do not
hardcode scene lengths when voiceover is present; let the voice set the pace."*
The seven measured clips total 39.10s of speech; with a 1.0s lead-in, ~0.70s
between lines and a 1.3s tail the composition runs 45.6s. The bed is 60s, so the
music never runs out.

## "Show the thing"

The project has two Flutter apps (`apps/agora`, `apps/stele`), and **neither is
shown**: Agora is an AutonomousTrust/ethne client and Stele is an unbuilt design
plan, so showing either would blur exactly the boundary this video exists to draw.
No Flutter build was attempted.

The real thing shown instead is the physical layer's own material:

| On screen | Source |
|---|---|
| 8 ridgetop + 12 valley node positions, by name, at their surveyed lat/lon | `lib/autonomous_trust/examples/appalachia/scenario.py` — `HILLTOP_NODES`, `VALLEY_NODES`, driven by `run-simulation.sh` |
| `516 SQ MI · 24 PEOPLE PER SQUARE MILE`, `3,465 UN- OR UNDERSERVED LOCATIONS` | `doc/rural-network.md` § Overview |
| "Payback never arrives." | `doc/rural-network.md` § Overview, verbatim |
| Link budget: 22 km, 138.1 dB FSPL, 30 dBm, +60 dB, −3 dB, −1.3 dB, −52.4 dBm, −65 dBm, 12.6 dB margin, 450+ Mbps | `doc/rural-network.md` § Worked Example: Sutton to Burnsville Link |
| Power: 70 W, 1,680 Wh/day, 3.0 peak sun hours (December), 700 W required, 2 × 400 W, 200 Ah LiFePO4 @ 24 V, ≈2.8 days, $4,920/node | `doc/rural-network.md` § Tier 1: Hilltop Relay Node (BOM + solar sizing) |
| Ledger: $38,200,000 / $1,278,607; $46,000–$78,000 / $527; $70 / $2.12; 3–5 years / 6–12 months; community ownership Rarely / Yes | `doc/rural-network.md` § Cost Comparison: Fiber vs Mesh (Model A) and § Monthly Operating Costs |
| Node tiers: ridgetop $4,920, valley $2,000, household $200–600, gateways 4 of 71 | `doc/hardware_design.md` § Four tiers of node; unit costs from the BOM tables and § Full County Deployment Estimate (26 hilltop + 45 valley relays, 4 fiber gateways) in `doc/rural-network.md` |
| The three optional tiers and their questions | `README.md`, `doc/the-stack.md` |
| "The physical layer neither knows nor cares which are present." | `README.md`, lightly re-tensed from "neither knows nor cares which" |
| "Hardware that moves bits without asking permission." | `README.md`, verbatim |

**Nothing on screen is invented.** The only authored geometry is the ridge
silhouette behind the node map, which is decorative and carries no reading; it is
not a terrain profile and is not labelled as one.

`baseline-results.json` was read and **deliberately left out**: its figures
(`identity_convergence_s`, `reputation_stability_stddev`, `negotiation_rtt_mean_s`,
`bandwidth_overhead_fraction`) are AutonomousTrust protocol measurements, not
physical-layer ones. Putting them on screen would blur the tier boundary the video
is built to defend. They belong in the AutonomousTrust video, which already exists.

## Visual identity

MUUDD ships no UI and no stylesheet, so there is no palette to lift — unlike the
AutonomousTrust cut, which took its colours verbatim from the project's dashboard
CSS. The palette here is authored, and it is authored *from the subject*: a ridge
at night, a backbone link, and a solar array.

- Background `#0a0e0d`; surface `#111917`; elevated `#18211e`; border `#27332f`
- Ridge silhouette `#1c2622` / `#141c19`
- Text `#e6ece9`; muted `#c6d2cd`; dim `#aebab5` (both lifted for WCAG AA at
  video type sizes, the same adaptation the AutonomousTrust cut made)
- Backbone link / 5.8 GHz `#6fd2e0`; ridgetop relay `#6fd2e0`
- Valley relay `#8fc79a`
- Solar / power `#f2ba5c`
- Fiber column (the expensive one) `#f08c7a`
- Display and mono are generic stacks: no font ships with this composition and a
  named family without an `@font-face` is a lint error.

## Share copy (draft)

MUUDD: the physical layer, priced to the connector. Fiber to Braxton County, West
Virginia costs $38.2 million. The same county, in solar radios the households own,
costs $1.28 million — and three tiers of trust, community and covenant can stack
on top, none of them required.

## Audio direction

- Role: **narration-led.** Music is a bed underneath the voice, never a driver.
- Music: `happy-beats-business-moves-vol-10-by-ende-dot-app.mp3` (ende.app),
  109.96 BPM, 60s. Chosen against the AutonomousTrust cut's "Beauty Flow" so the
  two videos do not repeat, and treated as a low, filtered bed — the track is
  upbeat at full level and is never played anywhere near it here.
- Music treatment: a **low-passed copy** of the track (`lowpass=f=2600`) at 0.22
  in the gaps between narration lines, ducked to **0.13** for the full span of
  every voice clip, entering very low under the hook and fading to silence over
  the last 1.0s. Never above 0.22.
- Voice track: seven per-scene WAVs on their own track indices at gain 1.0, so
  every scene boundary is timed to measured audio rather than guessed.
- Music cue guidance: the bundled preset
  (`composition/assets/music/cues/…music-cues.json`) has a planning window of only
  0–25s, so every timing in the finished composition is instead taken from
  `npx hyperframes beats` run on the wired project, which writes 217 beats for the
  whole track to
  `composition/beats/assets/music/happy-beats-business-moves-vol-10-by-ende-dot-app.mp3.json`.
  That detector resolves this track on a 0.2725s half-beat grid (reported 220 BPM);
  the musical pulse used here is every second point, 0.545s, and it agrees with the
  preset to within 0.04s inside the preset's own window. Narration wins every
  conflict; a cue is only used where it falls inside a natural gap or on a word the
  picture is already meeting.
- Audio-reactive treatment: subtle. Bass energy breathes the relay halos and the
  link stroke opacity on the map. No waveform, no bars, nothing that competes with
  speech.
- SFX posture: sparse, warm, low high-frequency-risk only
  (`sfx-analysis.json`: `rollover2` low, `bong_001` low,
  `impactSoft_medium_001` low). `drop_001` was considered for the panel swaps and
  rejected — it rates **medium** HF risk.
- Restraint rule: no risers, no swells. If a cue and a word collide, the word wins.

## Voiceover script

Seven clips, `hyperframes tts --voice am_michael --speed 0.96`. Each is written to
carry the argument while the screen carries the evidence, so the voice never reads
a figure that is already on the frame.

1. **hook** — "Ridge and hollow, and a population too thin to pay for a trench."
2. **backbone** — "So the households build it themselves. Solar radios on the
   ridges, owned by the people they serve."
3. **link** — "Ridge to ridge, above the canopy. The arithmetic is worked before
   anybody buys a radio."
4. **power** — "Sized against December rather than the annual average. Against the
   longest outage, not a single night."
5. **cost** — "Three percent of the fiber buildout, and nobody has to earn a return
   on it."
6. **stack** — "Three more tiers can sit above this one. Not one of them is
   required to send a message."
7. **lockup** — "The alternative, built from the bottom, and owned all the way
   down."

Line 4 is `doc/hardware_design.md` § Power, near-verbatim ("Size the panel against
the darkest month rather than the annual average, size the battery against the
longest expected outage rather than a single night"). Line 5's "three percent" is
`doc/rural-network.md` verbatim ("Both mesh models cost approximately 3% of the
fiber buildout"). Line 7's "built from the bottom" is `doc/muudd.md` verbatim.

### As synthesized

Transcribed from the delivered WAVs (`hyperframes transcribe`, whisper.cpp
`small.en`). Unlike the AutonomousTrust run, Kokoro dropped nothing — all seven
clips came back complete:

1. *"ridge and hollow, and a population too thin to pay for a trench."*
2. *"so the households build it themselves. Solar radios on the ridges owned by the people they serve."*
3. *"Ridge to ridge. Above the canopy. The arithmetic is worked before anybody buys a radio."*
4. *"Sized against December rather than the annual average. Against the longest outage, not a single night."*
5. *"3% of the fiber build out and nobody has to earn a return on it."*
6. *"Three more tiers can sit above this one. Not one of them is required to send a message."*
7. *"The alternative built from the bottom and owned all the way down."*

Two lines were **rewritten and regenerated** after the first transcription pass:

- Line 2 first read "Solar radios on the ridgetops"; the transcript came back as
  *"rigid tops"*. Changed to "on the ridges", which transcribes cleanly and is no
  worse a line.
- Line 7 first opened "MUUDD. The alternative built from the bottom …"; the
  transcript rendered the acronym as *"Mood"*. The name was removed from the
  narration entirely.

**Caveat on both, recorded rather than hidden:** whisper is unreliable on proper
nouns and coined acronyms, so *"rigid tops"* and *"Mood"* may well be ASR
artifacts rather than Kokoro defects — the transcript is evidence about the
transcript as much as about the audio. Neither rewrite rests on the transcript
alone. Line 7 stands on its own editorial ground: the wordmark **MUUDD** is on
screen at exactly that moment, and the skill's rule is that the voice does not
read the visible text, so the acronym did not belong in the line regardless. Line
2 lost nothing by the change. No other clip was regenerated on the strength of a
transcription oddity.

## Storyboard

Durations are set by the measured clips. Beat times are the bundled preset's grid
inside 0–25s and its continuation (≈0.545s spacing) past it.

### Scene 1 — Hook · 0 → 6.0s
Slug fades up at 0.40. **Payback never arrives.** lands on beat 1.37 and holds
through VO 1 (1.00 → 5.44). Attribution line under it at 2.46. All three clear at
5.55.
Audio: bed enters at 0.10 and stays there; the voice is the first thing you notice.
Transition: dramatic cut → Scene 2

### Scene 2 — The county assembles · 5.9 → 13.35s
The survey frame enters: topbar (MUUDD · physical layer · Braxton County WV),
ridge map left, empty readout column right, source strip along the bottom.
Eight ridgetop relays arrive one per beat with their real ids
(`sutton_hilltop`, `burnsville_hilltop`, `powell_mtn`, `otter_creek_ridge`,
`flatwoods_hilltop`, `elk_ridge`, `birch_mtn`, `centralia_ridge`), then twelve
valley relays in a fast stagger, then the backbone and access links draw in.
The readout column is not left empty: it carries the four node tiers the map is
made of — ridgetop relay $4,920, valley relay $2,000, household endpoint $200–600,
and gateways, 4 of 71 nodes, the tier the design refuses to depend on. Counter
runs relays 0 → 20; the mode chip reads SURVEY.
Sequential/interaction: yes — eight arrivals on the beat grid, links following,
and four tier rows at a slower 1.09s so they can be read.
Audio: quiet competence under VO 2; one soft tick per ridgetop, two for the valley
block.
Transition: clean cut → Scene 3

### Scene 3 — The link budget · 13.35 → 20.55s
The Sutton ↔ Burnsville hop lights and its callout appears on the map. The readout
column fills with the nine worked rows. `LINK MARGIN 12.6 dB` is beat-locked to
strong cue **18.01s** and lifts to the link colour; `450+ Mbps aggregate` follows.
Source strip: `doc/rural-network.md · Worked Example: Sutton to Burnsville Link`.
Sequential/interaction: yes — nine rows, then the margin, then the throughput.
Audio: no sting on the rows. One quiet bell on the margin.
Transition: column swap → Scene 4

### Scene 4 — The power budget · 20.55 → 28.15s
Column swaps to the solar sizing, beat-locked to strong cue **20.74s**. Seven rows
land; the relay dots on the map pick up a warm solar ring. `≈2.8 DAYS AUTONOMY` is
beat-locked to strong cue **24.01s**. `$4,920 PER NODE` closes it.
Source strip: `doc/rural-network.md · Tier 1: Hilltop Relay Node · solar sizing, 38.7°N`.
Sequential/interaction: yes — seven rows, then the autonomy figure, then the cost.
Audio: one quiet bell on the autonomy figure; nothing else.
Transition: column swap → Scene 5

### Scene 5 — The ledger · 28.15 → 33.60s
Column swaps to two columns, FIBER against MESH, five rows. The fiber column is
warm red; the mesh column is the link colour. Rows land on the extended grid at
28.92 / 29.47 / 30.01 / 30.56 / 31.10.
Source strip: `doc/rural-network.md · Cost Comparison: Fiber vs Mesh · Model A`.
Sequential/interaction: yes — five paired rows, all persisting.
Audio: bed opens a little in the gap before VO 5. No cue on any number.
Transition: the map recedes → Scene 6

### Scene 6 — The stack · 33.60 → 40.20s
Map and readout recede to 0.22. Four rows build **upward** from a solid base, on
alternating beats 34.37 / 35.46 / 36.55 / 37.64:

```
KITH-COVENANT     how do communities treat one another?     OPTIONAL
ETHNE             what is a community?                       OPTIONAL
AUTONOMOUSTRUST   who may cooperate?                          OPTIONAL
MUUDD             hardware that moves bits                    THIS LAYER
```

The three upper rows are outlined and dim; only the base row is filled. Caption at
38.73: *The physical layer neither knows nor cares which are present.*
Audio: light tick per row, quiet.
Transition: clean → Scene 7

### Scene 7 — Lockup · 40.0 → 45.6s
Survey frame recedes to 0.18. Wordmark **MUUDD** at 40.91, the expansion
(*Mesh-routed · User-owned · User-controlled · Dynamic Domains*) at 41.46, and the
tagline *Hardware that moves bits without asking permission.* at 42.55. The voice
says none of the three. Music fades to silence from 44.6.
Audio: settle and release.

**Music mood for this video:** cinematic — low, filtered, subordinate to speech.

**Audio summary:** A low, low-passed bed sits under seven narration clips,
opening to 0.22 in the gaps and ducking to 0.13 for every spoken span, with four
quiet cues in the whole video and a fade to silence under the closing line.

**The bed is filtered as well as ducked.** At its own level this track is bright
and upbeat, and ducking alone left its hats and sibilance sitting in the speech
band. What plays is a low-passed copy (`ffmpeg -af "lowpass=f=2600"`), with the
unfiltered original kept beside it in `composition/assets/music/`; the filter
takes the top off without touching the pulse, which is all the bed carries here.
`CREDITS.md` has the command and the reasoning. With the filter in place the bed
opens to 0.22 rather than 0.30.

---

## Build notes (final figures)

**Final duration: 45.6s.** Seven Kokoro clips (`am_michael`, speed 0.96) totalling
39.10s of speech, plus a 1.00s lead-in, ~0.70s between lines, and a 1.29s tail.

| Clip | Measured | Starts at | Ends |
|---|---|---|---|
| `vo-1-hook` | 4.437s | 1.00 | 5.44 |
| `vo-2-backbone` | 6.784s | 6.14 | 12.92 |
| `vo-3-link` | 6.549s | 13.62 | 20.17 |
| `vo-4-power` | 6.869s | 20.87 | 27.74 |
| `vo-5-cost` | 4.715s | 28.44 | 33.16 |
| `vo-6-stack` | 5.696s | 33.86 | 39.56 |
| `vo-7-lockup` | 4.053s | 40.26 | 44.31 |

Scene spans that follow: hook 0–6.0 · assemble 5.9–13.35 · link 13.35–20.55 ·
power 20.55–28.15 · ledger 28.15–33.60 · stack 33.60–40.20 · lockup 40.0–45.6.

### Where the picture meets the word

All times below are measured beats from `hyperframes beats`; the strengths are
that detector's normalized values.

- Ridgetop arrivals ride the beat grid under VO 2: 6.79, 7.34, 7.88, 8.43, 8.97,
  9.52, 10.06, 10.61 — one per beat, ids readable because they persist.
- The node-tier rows beside them arrive at half that rate, 7.88 / 8.97 / 10.06 /
  11.15, because they are text and text must not outrun reading.
- `LINK MARGIN 12.6 dB` lands on **18.52s** (strength 0.85), in VO 3's pause after
  *"above the canopy."*
- The power column swap is locked to **20.70s** (strength 0.90), which is the
  natural gap between VO 3 and VO 4.
- `≈2.8 DAYS AUTONOMY` lands on **25.06s** (strength 0.82), under *"against the
  longest outage."*
- The ledger panel opens on **28.34s** (strength 0.84) and its rows run every beat
  from 28.88 to 31.61, then hold as a full set for the rest of the scene.
- Tier rows on alternating beats 34.34 / 35.43 / 36.52 / 37.61, base row last.
- Wordmark on 40.88 (0.67), expansion on 41.43 (0.92), and the tagline on
  **42.52s** — the single strongest beat in the track (1.00).

### Audio outcome (measured off the delivered render)

Voice spans sit at ≈ −24.6 dB mean with peaks −5.2 to −5.3 dB; the gaps between
lines drop to ≈ −34.6 to −35.7 dB (bed only), so speech clears the music by about
**10 dB** throughout. The final fade reaches −47 dB. Ten decibels is a wider
margin than the AutonomousTrust cut's eight, which is the low-pass filter doing
its work as much as the ducking.

### Gates

- `brag-plan.md` — this file, with the full beat-by-beat storyboard.
- `npx hyperframes check --timeout 30000` — **0 errors**, 111/111 text checks pass
  WCAG AA, 0 layout issues across 9 samples, 0 motion and 0 runtime findings. The
  one remaining warning is `composition_file_too_large` (777 lines as the linter
  counts them), accepted
  rather than split for the same reason the AutonomousTrust cut accepted it: the
  composition is a single continuous survey frame and splitting it would separate
  timings that have to be read together. The `--timeout 30000` is pinned in
  `composition/package.json` and `build.sh` — the default 10s navigation timeout
  is flaky when several renders share the machine, and that failure is the
  headless browser starting slowly rather than anything in the composition.
- `brag.mp4` — exists, 3.6 MB, poster baked as frame 0.
- `share-copy.txt` — written, with the ende.app attribution block.

### Deliverables

- `brag.mp4` — 1920x1080, 45.600s, h264 + AAC 48 kHz stereo, 30 fps, 3.6 MB
  (`ffprobe`-verified on the delivered file, not on intent)
- `brag.jpg` — poster pulled at **33.2s**, the settled ledger frame with all five
  paired rows and the footnote landed: the single most postable still in the cut,
  because it carries the whole argument on its own. Baked as frame 0 of the MP4
  (verified: SSIM 0.988 against `brag.jpg`, while frame 1 measures 0.741, so the
  bake replaced only the first frame).
- `brag.webm` — vp9/opus transcode, 2.7 MB
- `composition-brief.md`, `share-copy.txt`, `CREDITS.md`, `build.sh`, `composition/`
  (including `assets/voice/` as source, `assets/music/` with both the filtered bed
  and the unfiltered original, `beats/` and `snapshots/`)
