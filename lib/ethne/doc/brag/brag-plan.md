# Brag Plan: Ethne (narrated cut)

Invoked as `/brag --voice`, so narration is on for this run. Sibling runs:
AutonomousTrust (`../../autonomous_trust/doc/brag/`, bed *beauty-flow*), and
MUUDD and Kith-Covenant in parallel on other beds. Same family, different tier.

## What is this app?

Ethne (Greek ἔθνη, "peoples") is the **polity tier**: it turns a set of
cooperating agents into a community, being a bounded, self-governing entity that
stands for a substrate, acts with one voice, and persists across the turnover of
its members. It sits between a trust substrate below and a federation above, and
it produces the entity the compact above it presumes.

Source: `README.md` opening paragraph; `doc/concept.md` "From cooperation to
community".

## The angle

**Emergence proposes, the polity ratifies, and the compact recognizes.**

That sentence is the spine of the project and it is the spine of this video. The
argument runs in one direction and never doubles back:

1. A cluster is not a community, and the gap is not rhetorical — it is four
   specific properties the tier above takes for granted and the substrate below
   does not supply.
2. Closing the gap by imposing a structure betrays the trust layer; closing it
   by relabelling the cluster betrays the compact. The interesting engineering
   is the ratification step in between.
3. The charter is what ratification produces, and it must account for everyone
   the boundary showed and the roll does not name.
4. Membership is not reception: the rite is five movements, and it is what
   finally gates who decides anything.
5. What crosses upward is seven artifacts in a vocabulary the compact already
   has — and the confinement of that vocabulary to one crate is checked by
   `cargo tree`, not asserted in a README.

This is deliberately **not** AutonomousTrust's argument. AT's video is about a
network that excludes a liar with nobody at the controls. This one never shows a
trust graph, never shows an adversary, and never argues about detection. Its
subject is constitution: what a community *is*, and what keeps it one.

## Hook (first 2-4 seconds)

Dark frame. Monospace slug `MUUDD · PART III · THE POLITY TIER`, then the
opening sentence of `doc/concept.md`, verbatim:

> **Nine machines that reliably deal with each other are not a community.**

The voice does not say that line. It asks the first of the chapter's four
questions underneath it instead, so picture and narration carry two different
things at once.

## Key moments (the middle)

- **The four presumptions.** `BOUNDED · COHERENT · PERSISTENT · AGENTIVE` arrive
  as four cards with the chapter's own glosses. The substrate below produces
  none of them directly.
- **The spine.** Three cards: *emergence proposes · the polity ratifies · the
  compact recognizes*, each carrying what actually moves at that step.
- **The founding, and what it excluded.** A charter card saying the four things
  a charter says, beside a `BoundaryAttestation` readout — candidate 30, roll 3,
  excluded 27 — and the accounting gate: `claims 30, accounts for 1 → REFUSED`.
- **The rite.** Five numbered movements, from the recognition doctrine, in the
  chapter's own words.
- **`cargo tree`, on screen, real.** Two invocations run against this workspace
  during planning. `en-cohesion` reaches only `en-*`. `en-uplift` is the one
  crate that names `kc-ceremony`, `kc-conduct`, `kc-core`, `kc-identity`. Those
  four lines tint gold under the words *"and the build checks that."*

## Outro / punchline

Wordmark **Ethne** with **ἔθνη** beneath it, over the dimmed document, and the
governing discipline as the tagline the voice never says: *Ratify emergence, do
not replace it.*

## User flow worth showing

The tier is a Rust workspace plus a Python surface — no UI. The runnable
material is `src/python/examples/polity_demo.py` (a polity's life end to end) and
`src/rust/examples/polity_host/`, but the Python surface needs `maturin develop`
against a conda env that is not provisioned in this sandbox, so the demo could
not be executed and **no figure printed by it is quoted**.

What *was* executed is `cargo tree`, which needs no build:

```
cargo tree -p en-cohesion --edges normal --depth 1
cargo tree -p en-uplift   --edges normal --depth 1
```

Both ran successfully in `/home/user/Software/continuous-space/lib/muudd/lib/ethne`.
Their output is on screen in scene 6, verbatim apart from the local `(path)`
suffix `cargo tree` prints after each workspace crate, trimmed for legibility.
That is the "show the thing" scene: the decoupling the README claims, being
checked rather than claimed.

## Tone

- Preset: **cinematic**
- Creative direction: *a constitutional document, read aloud at the pace of
  somebody who has thought about it.*
- Interpretation: Wide dark frames, warm paper-white type over ink, long holds,
  no motion that is not carrying an argument. Nothing arrives that the voice is
  not already standing next to.
- Voice: `am_michael` (Kokoro) at speed 0.96 — measured, unhurried, never
  announcing.

## Format: landscape — 1920x1080, 30fps
## Duration: 46.0s, set by the measured narration

The 15–25s creative law is deliberately exceeded under the skill's own voiceover
carve-out: *"Scene durations must flex to match the generated audio … Do not
hardcode scene lengths when voiceover is present; let the voice set the pace."*
Seven clips total 38.51s of speech; the rest is breathing room, a 1.07s lead-in
and a 1.5s tail under the fade.

## Visual identity (authored, and why)

Ethne ships no UI, no CSS, and no images — the tier is markdown, Rust and
Python. There is no project palette to lift, so one is authored here and
documented rather than pretended to be sourced. It is deliberately a *document*
register, and deliberately not AutonomousTrust's operations-dashboard navy:

- Background `#0b0d12`; surface `#141821`; elevated `#1c212c`; border `#2b3242`
- Text `#ece9e3` (warm paper white); muted `#c9c4b8`; dim `#aba598`
- **Gold `#e3b25a`** — this tier. The polity, the charter, the ratified thing.
- **Slate `#85a9cd`** — the substrate below (autonomous-trust).
- **Jade `#86c9a5`** — the compact above (kith-covenant).
- **Rust `#e0705f`** — refusal, used once, on the rejected accounting.
- Display: generic sans stack; mono: generic mono stack. No `@font-face` ships
  with the composition, so no named family is requested.

All type is checked against WCAG AA at video sizes by `hyperframes check`.

## Every figure on screen, and where it comes from

| On screen | Source |
|---|---|
| "Nine machines that reliably deal with each other are not a community." | `doc/concept.md`, opening line, verbatim |
| `autonomous-trust / "who cooperates?"`, `ethne / "what is a community, and what keeps it one?"`, `kith-covenant / "how do communities treat one another?"` | `README.md`, the stack diagram, verbatim |
| CRATES 10 | `src/rust/`: en-archive, en-at, en-charter, en-cohesion, en-core, en-host, en-pyo3, en-store, en-substrate, en-uplift — the ten the README enumerates |
| DECISIONS D1–D35 | `TODO.md` ("decisions D1-D35"); `doc/uplift.md` ("Design decisions D1 to D35"). The README still reads D34 and is stale. |
| RUST 437 · PY 233 | `README.md` quick start: `cargo test --workspace --exclude en-pyo3` — 437 tests; `pytest src/python/tests/` — 233 tests |
| BOUNDED / COHERENT / PERSISTENT / AGENTIVE and their glosses | `doc/concept.md`, "The gap, precisely" |
| "emergent topological clusters rather than named, root-anchored, covenant-bearing entities" | `doc/concept.md`, same section, verbatim |
| "Emergence proposes · the polity ratifies · the compact recognizes" | `README.md` and `doc/concept.md`, verbatim |
| "the trust field, the cluster boundary, the earned reputations" | `doc/concept.md`, the block-quoted discipline, verbatim |
| The four things a charter says | `doc/charter.md`: "who the members are, what the community holds above itself, what it commits to, and how those commitments may later change" |
| candidate 30 · roll 3 · excluded 27 | `doc/charter.md`: three people inside a thirty-node cluster, "the other twenty-seven"; `BoundaryAttestation` holds "the candidate size the boundary actually showed and a reasoned exclusion for everyone it showed whom the roll does not name" |
| `claims 30, accounts for 1 → REFUSED` | `src/python/examples/polity_demo.py`, the D21 section: the comment `# claims 30, accounts for 1` and the printed line "a charter claiming 30 and accounting for 1 is refused" |
| "permanent and immutable" | `doc/charter.md`: "that statement is permanent and immutable" |
| The five movements and their glosses | `doc/transmission.md`: "the naming of the substrate, the acknowledgment of fallenness, the recognition of others as peers, the commitment to the common virtues as dispositions, and the submission of the polity itself, which is not the ultimate" |
| "Admission puts somebody on the roll. Induction is the distinct later step by which they acquire the substrate." | `doc/transmission.md`, condensed from its own two sentences |
| The seven border artifacts | `doc/uplift.md`, "The seven artifacts" |
| Both `cargo tree` blocks | executed in this workspace during planning; see above |
| "Ratify emergence, do not replace it." | `README.md`, the governing discipline, verbatim |

Nothing on screen is invented. Where a number exists in two places and the two
disagree (D34 vs D35), the newer document wins and the disagreement is recorded
above rather than smoothed over.

## Share copy (draft)

Ethne is the polity tier: it turns a set of cooperating agents into a community
with a ratified roll, a substrate it stands under, and offices it confers on
itself. Emergence proposes, the polity ratifies, and the compact recognizes.

## Audio direction

- Role: **narration-led.** Music is a bed underneath the voice, not a driver.
- Music: `happy-beats-business-moves-vol-9-by-ende-dot-app.mp3` by
  [ende.app](https://ende.app), bundled with the `/brag` skill. 114.84 BPM, beat
  interval ≈ 0.5225s. Cut to the first 48s with a stream copy, no re-encode.
- Treatment: the track is bright by nature, so it is **low-passed at 2600 Hz**
  (`ffmpeg -af "lowpass=f=2600"`; the unfiltered cut is kept beside it in
  `assets/music/` and the command is in `CREDITS.md`) and held low throughout —
  0.24 in the gaps between lines, ducked to **0.11** for the full span of every
  voice clip, 0.09 through the refusal, fading to silence over the last 1.2s.
  The filter is a tonal decision, not a level one: measured off the first
  render the gaps already sat 6–10 dB under the voice. What level could not fix
  was character — a cheerful business-moves cut arguing with a constitutional
  document read aloud. Rolling the top off leaves the pulse and the bass, which
  is all this edit asks of it. `assets/audio-data.js` is extracted from the
  filtered file, so what the picture breathes with is what the viewer hears.
- Voice track: per-scene WAVs on their own tracks at gain 1.0, so every scene
  boundary is timed to measured audio rather than guessed.
- Music cue guidance: cue preset at
  `skills/brag/assets/music/cues/happy-beats-business-moves-vol-9-by-ende-dot-app.music-cues.json`.
  Three strong-cue locks only; everything else rides the beat grid, and
  **narration wins any conflict.**
- Audio-reactive treatment: subtle. Bass energy breathes the panel border
  presence and a faint warmth behind the rail's lit tier. No bars, no spectrum,
  nothing that competes with speech.
- SFX posture: sparse and warm, low high-frequency-risk picks only
  (`rollover2`, `bong_001`, `impactSoft_medium_001`, `impactSoft_medium_003`,
  `click2`). A quiet tick per card, one soft impact on the refusal, one on the
  wordmark. Nothing editorialises.

## Voiceover script

Seven clips, `hyperframes tts --voice am_michael --speed 0.96`. Each is written
to stand *next to* the screen rather than read it.

1. **hook** — "Ask a cluster who its members are. Nobody agreed to an answer."
2. **gap** — "The tier above presumes four things of a community. A trust
   substrate supplies none of them."
3. **discipline** — "So this tier imposes nothing. It reads the emergent
   structure and offers it back to be ratified."
4. **charter** — "A founding is one signed document, and it must name everyone
   it left out."
5. **rite** — "Admission puts you on the roll. The rite is how you receive what
   the roll is a list of."
6. **border** — "Seven artifacts cross to the compact. Exactly one crate speaks
   its vocabulary, and the build checks that."
7. **lockup** — "Ethne. The tier that produces the entity the compact presumes."

One spelling note: clip 7 is generated from the text `Ethnee.` rather than
`Ethne.` Kokoro reads `Ethne` as *Ethan*; `Ethnee` gives ETH-nee, which is the
Greek. The wordmark on screen is `Ethne` either way.

### As synthesized

Unlike the AutonomousTrust run, **nothing was dropped**. Each line was written
short enough that Kokoro delivered it whole. Transcribed twice and independently
— `whisper.cpp base.en` and `faster-whisper base.en` — and the two agree
verbatim on all seven clips. The word onsets used for timing below come from
`faster-whisper`, which reports them; the delivered audio is the authority for
both:

1. *"Ask a cluster who its members are. Nobody agreed to an answer."*
2. *"The tier above presumes four things of a community. A trust substrate supplies none of them."*
3. *"So this tier imposes nothing. It reads the emergent structure and offers it back to be ratified."*
4. *"A founding is one signed document, and it must name everyone it left out."*
5. *"Admission puts you on the roll. The rite is how you receive what the roll is a list of."* — transcribed as "role" / "The right"; homophones in the transcriber, not in the delivery.
6. *"Seven artifacts cross to the compact. Exactly one crate speaks its vocabulary, and the build checks that."*
7. *"Ethne, the tier that produces the entity the compact presumes."* — transcribed "Ethney".

## Storyboard

Durations below are the measured clips. The beats fix order and what must be
visible.

### Scene 1 — Hook · 0 → 6.1s
Slug fades in; the concept.md sentence lands and holds through VO 1. No document
yet.
Sequential/interaction: none.
Audio intent: bed enters at 0.10 and stays there; the voice is the first thing
you notice.
Transition: dramatic → Scene 2

### Scene 2 — The gap · 6.1 → 13.0s
The document frame enters on a strong cue. Left rail draws the three-tier stack
with `ethne` lit. Main panel: `WHAT THE TIER ABOVE PRESUMES` and four cards
arriving on alternating beats, then the chapter's own footnote about topological
clusters.
Sequential/interaction: yes — four cards, ~1.05s apart, all persisting.
Audio intent: quiet competence under VO 2; one soft tick per card.
Transition: clean cut → Scene 3

### Scene 3 — The discipline · 13.0 → 20.5s
Cards clear. `RATIFY EMERGENCE, DO NOT REPLACE IT`, then the spine as three
cards on alternating beats, each carrying what moves at that step. The middle
card is gold; the outer two are slate and jade, matching the rail.
Sequential/interaction: yes — three cards, ~1.58s apart.
Audio intent: bed opens slightly in the gap before VO 3; ticks on cards 1 and 3.
Transition: clean cut → Scene 4

### Scene 4 — The founding, and what it excluded · 20.5 → 26.1s
Spine clears. The charter card lists the four things a charter says, revealed as
a fast stagger and held. Beside it the `BoundaryAttestation` readout counts to
candidate 30 / roll 3 / excluded 27, then the refusal stamp lands.
Sequential/interaction: yes — four lines, then a counter, then one stamp.
Audio intent: music at its lowest here; one dry impact on the refusal, nothing
else.
Transition: soft → Scene 5

### Scene 5 — The rite · 26.1 → 32.4s
Charter clears. `THE RITE — five movements`, the admission/induction line, then
five numbered rows on alternating beats.
Sequential/interaction: yes — five rows, ~1.05s apart, all persisting.
Audio intent: a light tick per movement; the bed does not lift.
Transition: clean → Scene 6

### Scene 6 — The border, checked · 32.4 → 40.2s
`WHAT CROSSES THE BORDER`: seven artifact chips stagger in and hold. Below them
a terminal panel opens and the two real `cargo tree` invocations print, the
`en-cohesion` block first, then `en-uplift`. On the last clause of VO 6 the four
`kc-*` lines tint gold and a single caption settles: *one crate, checked by the
build.*
Sequential/interaction: yes — seven chips, then two terminal blocks, then one
highlight.
Audio intent: the bed's only open moment, still under 0.26; one soft impact on
the highlight.
Transition: dim → Scene 7

### Scene 7 — Lockup · 40.2 → 46.0s
Document recedes to 0.20. Wordmark **Ethne**, then **ἔθνη**, then *Ratify
emergence, do not replace it.* Music credit settles last. Bed fades to silence.
Audio intent: settle and release.

**Music mood for this video:** cinematic — low, filtered, subordinate to speech.

**Audio summary:** A low bed sits under seven narration clips, ducking to 0.11
for every spoken span, opening to 0.26 in the gaps, dropping to 0.09 through the
refusal, and fading to silence under the closing line.

---

## Build notes (final figures)

Written after the narration was generated and measured.

**Final duration: 46.0s.** Seven Kokoro clips (`am_michael`, speed 0.96)
totaling **38.51s** of speech.

| Clip | Measured | Starts at | Ends |
|---|---|---|---|
| `vo-1-hook` | 4.245s | 1.068 | 5.313 |
| `vo-2-gap` | 5.931s | 6.339 | 12.270 |
| `vo-3-discipline` | 6.592s | 13.177 | 19.769 |
| `vo-4-charter` | 5.013s | 20.538 | 25.551 |
| `vo-5-rite` | 5.483s | 26.331 | 31.814 |
| `vo-6-border` | 7.275s | 32.659 | 39.934 |
| `vo-7-lockup` | 3.968s | 40.530 | 44.498 |

Every start is a beat-grid point of the track, so the voice itself is on the
pulse. Gaps between lines run 0.60s to 1.02s.

Scene spans that follow: hook 0–6.1 · gap 6.1–13.0 · discipline 13.0–20.5 ·
founding 20.5–26.1 · rite 26.1–32.4 · border 32.4–40.2 · lockup 40.2–46.0.

### Where the picture meets the word

- **Beat-lock 1** — the document frame enters on strong cue **6.339s**
  (intensity 1.00), which is also where VO 2 begins.
- The four presumption cards ride alternating beats 7.396 / 8.441 / 9.497 /
  **10.542** (the last is a strong cue at 1.00), under *"presumes four things of
  a community."* The footnote lands on strong cue 11.598.
- The spine cards land 14.756 / 16.335 / 17.914. Card 2, *the polity ratifies*,
  sits 0.4s ahead of the word *ratified* at 18.82 — the picture arrives first
  and the voice confirms it.
- The boundary readout counts from 20.54 and settles on **beat-lock 2**, strong
  cue **23.174s** (intensity 0.99), under *"it must name everyone it left out."*
  The refusal stamp follows at 24.741.
- The five movements ride alternating beats 26.854 / 27.910 / 28.955 / **30.012**
  (strong cue, 1.00) / 31.057.
- The seven artifact chips stagger from 32.659 and hold; the terminal blocks
  print at 35.271 and 36.862.
- **Beat-lock 3** — the four `kc-*` lines tint gold on **38.963s**, under *"and
  the build checks that"* (word *build* at 38.86).
- Wordmark on 41.065, ἔθνη on 42.121, tagline on strong cue **43.700s**.

### Audio outcome (measured off the render)

Measured off the **delivered** render (low-passed bed, 0.24 open / 0.11 duck):
voice spans sit at −22.7 to −23.6 dB mean with peaks −1.9 to −4.7 dB; the gaps
between lines drop to −30.1 to −33.2 dB (bed only), so speech clears the music
by 6.5 to 10 dB throughout. The last 0.6s under the fade reaches −38.9 dB.

For comparison, the same measurement on the unfiltered first render gave gaps of
−29.8 to −33.2 dB — the level posture was already right, which is why the fix
applied was tonal rather than another 3 dB.

### Deliverables

Verified with `ffprobe` against the delivered file, not against intent:

- `brag.mp4` — 1920x1080, 30/1 fps, **46.000000s**, h264 + AAC 48 kHz stereo,
  3.5 MB, `+faststart`
- `brag.jpg` — 1920x1080 poster pulled at **39.6s**, the settled `cargo tree`
  frame with the four `kc-*` lines already gold and the caption up. It is the
  one frame in the video that is real output rather than authored type, and it
  is postable on its own. Baked as frame 0 of the MP4 — verified by extracting
  frame 0 and differencing it against `brag.jpg` (mean absolute difference
  0.785/255, i.e. JPEG re-encode noise only).
- `brag.webm` — vp9/opus, 2.8 MB
- `composition-brief.md`, `share-copy.txt`, `CREDITS.md`, `build.sh`,
  `composition/` (with `assets/voice/`, `assets/music/`, `assets/sfx/` and
  `snapshots/`), `scripts/make_audio_data.py`

### What could not be done

- **The Python demo was not run.** `src/python/examples/polity_demo.py` needs
  `maturin develop` against the `ethne` conda env; conda is not on PATH in this
  sandbox and `import ethne` fails. No figure it prints is quoted. The one line
  taken from that file — `claims 30, accounts for 1 → REFUSED` — is taken from
  the *source text* (an inline comment and a `print` string), not from output.
- **`hyperframes transcribe` could not run**: the CLI's auto-clone of whisper.cpp
  had failed silently and left `~/.cache/hyperframes/whisper/` empty.
  `whisper.cpp base.en` was built out-of-band and `faster-whisper base.en`
  installed into the session scratchpad; both were run over all seven clips and
  agree verbatim. `base.en` mangles proper nouns — it renders *roll* as "role",
  *rite* as "The right", and *Ethne* as "Ethney" — which is an ASR artifact, not
  a delivery defect. The one genuine pronunciation problem, Kokoro reading
  `Ethne` as *Ethan*, was fixed by respelling the TTS input `Ethnee` while
  leaving the on-screen wordmark correct.
- **`hyperframes check` is flaky at the default 10s navigation timeout** on a
  loaded machine. The gate is run — and scripted in `build.sh` and
  `composition/package.json` — with `--timeout 30000`.
- **The Rust test suite was not run.** `437` and `233` are the README's figures,
  cited as such. `cargo tree` *was* run, and is what scene 6 shows.
