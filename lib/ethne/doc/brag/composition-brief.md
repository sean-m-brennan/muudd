# Hyperframes Composition Brief: Ethne

## Objective

A narrated launch-style brag video for **Ethne**, the polity tier of the MUUDD
stack. Narration-led, cinematic, 1920x1080, 46.0s.

## Output

- Composition directory: `brag-output/composition/`
- Rendered video: `brag-output/brag.mp4`
- Format: landscape — 1920x1080, 30fps
- Duration: **46.0s**, set by seven measured Kokoro clips (38.51s of speech).
  This exceeds the skill's 15–25s creative law under its own voiceover
  carve-out: *"Do not hardcode scene lengths when voiceover is present; let the
  voice set the pace."*

## Source material

- Project root: `/home/user/Software/continuous-space/lib/muudd/lib/ethne`
- Primary files read: `README.md`, `TODO.md`, `doc/concept.md`,
  `doc/charter.md`, `doc/transmission.md`, `doc/cohesion.md`, `doc/uplift.md`,
  `doc/crates.md`, `doc/design.md`, `src/python/examples/polity_demo.py`,
  `src/rust/` (crate manifests), `run-tests.sh`
- Product name: **Ethne** (ἔθνη)
- Strongest claim: *Emergence proposes, the polity ratifies, and the compact
  recognizes.*
- Key real artifact to put on screen: the output of two `cargo tree` invocations
  run against this workspace — the machine-checked decoupling.

### Copy that must appear verbatim

- "Nine machines that reliably deal with each other are not a community."
- "who cooperates?" / "what is a community, and what keeps it one?" / "how do
  communities treat one another?"
- "emergent topological clusters rather than named, root-anchored,
  covenant-bearing entities"
- "Emergence proposes · the polity ratifies · the compact recognizes"
- "the trust field · the cluster boundary · the earned reputations"
- "Ratify emergence, do not replace it."
- The five movements, in the chapter's own words.
- Both `cargo tree` blocks, trimmed only of the local `(path)` suffix.

Full provenance for every figure is the table in `brag-plan.md`.

## Creative direction

- Tone preset: **cinematic**
- Creative direction: *a constitutional document, read aloud at the pace of
  somebody who has thought about it.*
- Interpretation: wide dark frames, warm paper-white type over ink, long holds,
  no motion that is not carrying an argument.
- Angle: the tier sits between a trust substrate and a federation and produces
  the entity the federation presumes. The video runs the argument in one
  direction — the gap, the discipline that closes it, the charter, the rite, the
  border — and ends on the check that proves the border is where the project
  says it is.
- Hook: the opening sentence of `doc/concept.md`, on screen, unread by the voice.
- Outro: the wordmark and the governing discipline.
- Avoid:
  - Generic SaaS language
  - Abstract filler visuals
  - Restating AutonomousTrust's argument — no trust graph, no adversary, no
    detection story
  - Hedging the project's own stance. The documentation writes from inside it;
    the narration inherits that register rather than describing it from outside.

## Visual identity

Ethne ships no UI, no CSS and no images. The palette below is **authored for
this video** and documented as such in `brag-plan.md` — it is not lifted from a
project stylesheet, because there is none.

- Background `#0b0d12`; surface `#141821`; elevated `#1c212c`
- Border `#2b3242`; border-muted `#3d4658`
- Text `#ece9e3`; muted `#c9c4b8`; dim `#aba598`
- Gold `#e3b25a` — this tier (polity, charter, ratified)
- Slate `#85a9cd` — the substrate below (autonomous-trust)
- Jade `#86c9a5` — the compact above (kith-covenant)
- Rust `#e0705f` — refusal, used exactly once
- Display font: generic sans stack. Body/mono: generic mono stack. No
  `@font-face` ships, so no named family is requested.

Visual references from the project: the three-tier stack diagram in `README.md`;
the numbered five movements of `doc/transmission.md`; the crate inventory of
`doc/crates.md`; real `cargo tree` output.

## Storyboard

The storyboard in `brag-output/brag-plan.md` is the creative contract.

1. **Hook** — 0 → 6.1s — slug, then the concept.md sentence.
2. **The gap** — 6.1 → 13.0s — the document frame, the three-tier rail, four
   presumption cards.
3. **The discipline** — 13.0 → 20.5s — "Ratify emergence, do not replace it",
   then the three-card spine.
4. **The founding** — 20.5 → 26.1s — the charter card, the boundary attestation
   readout, the refusal stamp.
5. **The rite** — 26.1 → 32.4s — five numbered movements.
6. **The border, checked** — 32.4 → 40.2s — seven artifact chips, then real
   `cargo tree` output, then the `kc-*` highlight.
7. **Lockup** — 40.2 → 46.0s — wordmark, ἔθνη, tagline, music credit.

## Audio

- Audio role: **cinematic support under narration.** The voice leads throughout.
- Audio arc: bed enters at 0.10 under the hook, ducks to 0.11 for every spoken
  span, opens to 0.24 in the gaps, drops to 0.09 through the refusal, fades to
  silence over the last 1.2s.
- Music: `happy-beats-business-moves-vol-9-by-ende-dot-app.mp3` (ende.app),
  114.84 BPM, cut to the first 48s with a stream copy, then low-passed at
  2600 Hz. The composition plays the `.lowpass.mp3`; the unfiltered cut is kept
  beside it in `assets/music/` and the command is recorded in `CREDITS.md`.
- Music treatment: low and filtered. The track is bright by nature and is
  deliberately used as a pulse under a document rather than a driver. The filter
  is tonal, not a level fix — the gaps already cleared the voice by 6–10 dB
  measured off the first render.
- Music cue guidance: bundled preset
  `assets/music/happy-beats-business-moves-vol-9-by-ende-dot-app.music-cues.json`.
  Beat interval ≈ 0.5225s. **Three** strong-cue locks: 6.339 (frame entry),
  23.174 (boundary readout settles), 38.963 (`kc-*` highlight). Everything else
  rides the beat grid. Narration wins any conflict.
- Audio-reactive treatment: subtle — bass energy breathes panel border presence
  and a faint warmth behind the lit tier in the rail. No bars, no spectrum, no
  pulsing type.
- Audio-coupled moments:
  - presumption cards / spine cards / rite movements — beat-grid arrivals
  - boundary readout counter — settles on a strong cue
  - `kc-*` highlight — strong cue, under the word it illustrates
  - wordmark and tagline — beat-grid
- SFX selection guidance: sparse, warm, low high-frequency risk only. Quiet
  ticks on card arrivals, one dry impact on the refusal, one soft impact on the
  `kc-*` highlight and one on the wordmark. Nothing that editorialises.
- SFX analysis guidance:
  `/home/user/.claude/plugins/cache/brag/brag/0.2.2/skills/brag/assets/sfx/sfx-analysis.md`
  — only `low` high-frequency-risk files are used.
- Voiceover: seven per-scene WAVs in `assets/voice/`, own tracks, gain 1.0.
  Tracked as **source**, not build output — `build.sh` never regenerates them.

## Hyperframes instructions

Standard composition contract: one paused root timeline registered on
`window.__timelines`, `data-start` + `data-duration` on every timed element,
`class="clip"` on timed scene sections, deterministic logic only (no
`Date.now()`, no `Math.random()`, no network fetches), local assets only.

Requirements honoured:

- Shows real project material: verbatim documentation copy throughout, and real
  `cargo tree` output executed against this workspace in scene 6.
- All text readable in the final render; `check`'s WCAG AA pass is the gate.
- Music and SFX layer present; audio-reactive layer wired from a pre-extracted
  `assets/audio-data.js` so renders stay deterministic and seek-safe.
- `npx hyperframes check --timeout 30000` run before render, with zero errors.
  The longer navigation timeout is required: the default 10s is not enough to
  start the headless browser on a loaded machine, and the failure that produces
  is the browser, not the composition.
