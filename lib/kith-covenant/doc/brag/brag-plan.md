# Brag Plan: Kith-Covenant (narrated cut, "the quiet polity")

Invoked as `/brag --voice`, tone preset `cinematic`, landscape 1920x1080 at 30fps.
Full re-concept, 2026-10-01. The previous cut (the inversion, the two rules, the
two-layer ceremony, the lenses) is backed up in `previous-2026-09-18/`; its CSS
tokens, audio bed, SFX and section pattern are reused here, and none of its
storyboard is. The register follows the AutonomousTrust capability film at
`../../autonomous_trust/doc/brag/`: one concrete scenario, the voice carrying the
argument, the picture carrying the evidence.

## Rubric

1. **What is it?** A protocol for cooperation among communities that do not share
   ends and cannot appeal to any authority standing above all of them, with a Rust
   implementation of all ten protocol components and a Python harness over it.
2. **Most impressive claim.** It went looking for the failure nobody causes and
   found it. A compact that simply stops read clean on every instrument it had.
   Now silence is evidence, and a harness scores that failure against criteria
   stated in advance.
3. **Visual hook.** One sealed rite on a timeline, then cadences passing with
   nobody answering them, under the line **No adversary.**
4. **What to show from the product.** The erosion alert's own inputs and fields,
   the `[100, 400, 400, 400]` gap reading from the kc-reputation test that pins
   it, `ActivityAttestation` verbatim, and the output of a real harness run.
5. **Shortest satisfying video.** Seven beats, 50.0s, set by the measured voice.
6. **Tone.** `cinematic`; creative direction: *a capability film about a failure
   with no villain, measured and unhurried, nothing that editorializes.*
7. **Audio.** Narration-led; the existing low-passed ende.app bed at 0.13 beneath
   the voice; sparse warm SFX on arrivals.
8. **Share caption.** See `share-copy.txt`.
9. **User flow worth showing.** The harness's quiet-polity scenario as a
   story: a polity seals one rite and never another, the old alert reads `None`,
   the pinned reading sees it, the signed null records it, and the harness scores it.

## The angle

**The failure with no adversary.** Every capture-resistance design assumes an
enemy. Kith-Covenant's own audit turned up a worse case: a compact that is never
captured, only stops. Rites lapse, offices empty, the observer corps files
nothing, and every act-triggered detector reads clean. The video follows that one
scenario from the hole to the fix to the test, so the viewer watches the project
find its own blind spot and close it.

What makes this specific to this tier: no mesh, no score, no peer. Communities,
cadences, signed records and a harness. And it states the project's own limit
directly in the last line, because the architecture states it directly: drift is
made expensive, slow and legible, which is forensic legibility for the remnant
rather than prevention.

## Hook (first 2-3 seconds)

Monospace slug `HARNESS SCENARIO · THE QUIET POLITY`, then **No adversary.** at
1.10s. Beneath it a timeline draws: one rite sealed, two re-affirmations due and
unanswered, *never another*. The voice says none of it. It says the condition
instead: a compact can fail without being captured.

## Key moments

- **The hole (K1).** Four lapses arrive one by one; beside them, the alert's three
  act-triggered inputs all read quiet and the severity lands as
  `ErosionSeverity::None`.
- **The pinned reference (E1).** Gaps 100, 400, 400, 400. The rolling reading says
  `false`; the founding-pinned reading says `true`. Headline: *Only absolute
  measures see sincere drift.*
- **The signed null.** The 43-year empty office that read clean, beside the
  `ActivityAttestation` struct whose empty `acts` list is the signed null.
- **What the alert reads now.** All sixteen `ErosionAlert` fields in declared
  order, sorted into four kinds; the recited-object pair, `unanswered_grievances`
  and `reporter_targeting` light as the voice names them.
- **The harness.** A real run: 14/14, and the six scenarios with no adversary,
  each PASS with its latency and damage against its stated bound.

## Outro

Wordmark **Kith-Covenant**, then *Cooperation without a sovereign.* (the title of
`doc/concept.md`). The voice says the name and the claim the screen never shows:
*drift made expensive, slow, and legible.*

## Tone

- Preset: **cinematic**
- Creative direction: capability film, documentary register.
- Interpretation: dark full-bleed frames on the project's own dashboard palette,
  crossfades, long holds on evidence, no verdict animation. Short declarative
  lines and fragments in the voice. Nothing celebratory.
- Voice: Kokoro `am_michael`, speed 0.96.

## Format: landscape — 1920x1080, 30fps
## Duration: 50.0s, set by the measured narration

The 15-25s creative law is exceeded under the skill's voiceover carve-out ("Scene
durations must flex to match the generated audio"). Seven clips measure 44.20s of
speech; with 0.55-0.65s of air between lines, a 0.8s lead and a 1.5s tail, the
cut lands at 50.0s. A first draft ran 45.44s of speech (51.4s total); clip 6 was
rewritten shorter.

## Visual identity (from the project)

Unchanged from the previous cut, lifted from `src/python/dashboard/static/index.html`
(dark theme): plane `#0d0d0d`, surface `#1a1a19`, border `#34342f`, ink `#ffffff`,
ink-2 `#c3c2b7`, series `#3987e5`, warning `#fab219`. Lifted for AA at video
sizes: muted `#a3a199`, series text `#5a9df0`, critical `#e8736b`. One token added:
`--good-text #3fc23f`, a text-weight variant of `--good #0ca30c` for the PASS
labels. Generic `system-ui` / `ui-monospace` stacks; the renderer resolves them
to Roboto / JetBrains Mono deterministically.

## Audio direction

- Role: **narration-led**; the bed sits under the voice, not beside it.
- Music: `happy-beats-business-moves-vol-12-by-ende-dot-app.lowpass2600.mp3`, the
  same low-passed copy as before (109.96 BPM). Clip duration cut to 50s.
- Treatment: 0.10 → 0.13 over the first second, held under every line, up to 0.16
  from 47.9s once the last word is out, fade to silence over 48.8-50.0s.
- Music cue guidance: the shipped preset's `beats` and `strongCues` cover the whole
  track (its `analysis.windowDuration` field says 25s, but the arrays run to
  112.9s and 101.5s), so every lock below is read from the preset, not
  extrapolated. Sequential text snaps every other beat (1.091s).
- Audio-reactive treatment: none. A typographic, evidence-led cut; nothing for
  bass energy to breathe.
- SFX: `rollover2` for sequential arrivals, `click_002` for record rows and the
  severity line, `bong_001` twice (the pinned reading, the attestation record),
  `impactSoft_medium_001` once on the wordmark.
- Restraint: no risers, no swells, no sting on `None` or on PASS. If a cue and a
  word collide, the word wins.

## Voiceover script (as generated and measured)

`npx hyperframes@0.8.47 tts "<line>" --voice am_michael --speed 0.96`. Durations
by ffprobe. Transcribed back with whisper.cpp (`ggml-small.en`): every clip came
back whole. The only ASR differences are homophones (*rite* heard as *right*) and
the numeral *200*; the audio is correct.

| # | Clip | Line | Measured | Starts | Ends |
|---|---|---|---|---|---|
| 1 | `vo-1-hook` | A compact between communities doesn't have to be captured to fail. It can simply stop. | 5.547s | 0.80 | 6.35 |
| 2 | `vo-2-stopped` | Kith-Covenant's defenses were built against somebody acting. A polity that merely stopped scored nothing at all. | 7.189s | 6.95 | 14.14 |
| 3 | `vo-3-pinned` | The fix is a reference the polity can't drag along behind it. A fixed cadence. The founding rite. | 6.635s | 14.70 | 21.34 |
| 4 | `vo-4-null` | So silence gets a signature: an account of every window, even one where nothing happened. | 6.101s | 21.90 | 28.00 |
| 5 | `vo-5-widened` | Then the audit kept going. Enforcement narrower than the recitation. A grievance buried. An accuser answered. | 7.360s | 28.55 | 35.91 |
| 6 | `vo-6-harness` | The criteria are fixed before the run. Once, two hundred runs returned a constant. It had been measuring nothing. | 7.595s | 36.45 | 44.05 |
| 7 | `vo-7-lockup` | Kith-Covenant. Drift made expensive, slow, and legible. | 3.776s | 44.70 | 48.48 |

Total speech 44.203s. No line reads on-screen text: the hook line, the headline,
the 43-year figure, the field names, the PASS rows and the tagline are all
screen-only, and the voice's sentences appear nowhere on screen.

## Storyboard (final times)

### Scene 1 — Hook · 0 → 6.65
Slug 0.30; **No adversary.** 1.103; track draws from 2.194; *one rite sealed*
2.739 (click); *re-affirmation due* 3.830 and 4.921; *never another* 5.467.
Transition: crossfade.

### Scene 2 — The polity that stopped · 6.65 → 14.42
Eyebrow *Finding K1*, title *A polity that simply stops.* Four lapses every other
beat (7.649 / 8.742 / 9.831 / 10.925). Alert panel 11.467 with its three inputs;
`severity ErosionSeverity::None` on strong cue **12.550**, under *"scored
nothing"*. Cite 13.1.

### Scene 3 — A reference that cannot move · 14.42 → 21.62
Headline *Only absolute measures see sincere drift.* Four gap bars one per beat
from strong cue 15.836 (three-digit labels, held to the scene end). Rolling
reading `false` on strong cue **18.564** (*"a fixed cadence"* at 18.81); pinned
reading `true` on beat **20.195** (*"the founding rite"* at 20.40). The test name
as cite, 20.74.

### Scene 4 — The signed null · 21.62 → 28.28
Eyebrow *Attested null*, title *An empty list, signed.* The empty-office panel
(0 mediators, 0 observers, 43 years, read clean) on strong cue 22.372; the
`ActivityAttestation` struct on 22.930; the `acts` row lights on 24.555 (*"every
window"*); its doc comment on 26.738 (*"nothing happened"*).

### Scene 5 — What the alert reads now · 28.28 → 36.18
Eyebrow `pub struct ErosionAlert`, title *Sixteen readings, four kinds.* Four
columns on the grid (28.923 / 29.469 / 30.560 / 31.651): fires on an act (4),
on omission or a pinned baseline (7), on intensification (3), against the object
its members recited (2). Highlights on the words: the recited-object pair 32.2
(*"recitation"*), `unanswered_grievances` 33.287 (*"grievance buried"*),
`reporter_targeting` 34.65 (*"accuser answered"*). Cite 35.2.

### Scene 6 — The harness · 36.18 → 44.37
Terminal panel with the demo command; the summary `14 supported scenarios, 14/14
passed` on strong cue 36.548; moves 8-13 on six strong cues every other beat
(37.105 → 42.562); the continuous-run footer on 43.105.

### Scene 7 — Lockup · 44.37 → 50.00
Wordmark on strong cue **44.745** (soft impact), rule 45.279, tagline 46.370,
music credit 47.468. Bed lifts a hair, then fades out.

**Poster:** 21.4s — scene 3 fully settled: the headline, all four bars, both
readings and the cite, before the crossfade at 21.62.

## Source table

| On screen / spoken | Source (current state) |
|---|---|
| "A compact ... doesn't have to be captured to fail. It can simply stop." | `doc/capture.md`, *The failure with no adversary*: "A compact does not have to be captured to stop working. It can simply stop." |
| "the quiet polity"; one rite sealed, never another | `doc/architecture/erosion-legibility.md` K4: "A community is founded, seals one rite, and never holds another"; scenario key `quiet-polity`, `src/python/kith_covenant/harness.py` `SCENARIOS` |
| "No adversary." | `harness.py` run output: "[move 8] the quiet polity (omission, no adversary)" |
| "defenses were built against somebody acting" | `doc/capture.md`: "Six of the seven moves above assume somebody is doing something"; erosion-legibility: "All seven capture moves are of that shape" (see documentation notes) |
| no rites sealed / no disputes filed / no observer reports published / every cadence lapsed | erosion-legibility K1, verbatim |
| `list_contraction` / `fast_contraction` / `fast_deviations`, `ErosionSeverity::None`; "scored nothing at all" | erosion-legibility K1: "It has three inputs, and all three are act-triggered ... scores `ErosionSeverity::None`"; `doc/capture.md`: "a polity that merely *stopped* scored nothing at all" |
| "Only absolute measures see sincere drift." | `doc/capture.md`, bold rule |
| Gaps 100 / 400 / 400 / 400; `latest_gap_widening false`; `gap_widening_since_founding true` | `src/rust/kc-reputation/src/tests.rs`, `the_rolling_gap_reading_goes_quiet_once_the_drift_becomes_the_baseline` (asserts all three); `analytics.rs` doc comments |
| "a fixed cadence. The founding rite." | erosion-legibility: instruments whose reference is "a fixed cadence, a fixed threshold, a pinned historical baseline"; `witness_count_declined` "compares against the first rite" |
| 0 mediators · 0 observers · 43 years · read clean on every instrument | erosion-legibility, remediation: "zero active mediators and zero active observers for forty-three years ... read clean on every instrument"; `tests.rs` `a_function_with_nobody_seated_is_reported` |
| `ActivityAttestation` fields and the "Legally empty" comment | `src/rust/kc-conduct/src/record.rs`, verbatim (field doc comments omitted except the `acts` one) |
| "an account of every window, even one where nothing happened" | `doc/jester.md`: "The corps owes an account of each window it holds seats for, *including an account of having seen nothing*" |
| Sixteen `ErosionAlert` fields, declared order, four kinds | `src/rust/kc-reputation/src/analytics.rs` `pub struct ErosionAlert` (16 fields; kinds from the field doc comments); README component 6 and `doc/crates.md`: "sixteen signals" |
| "enforcement narrower than the recitation", "a grievance buried", "an accuser answered" | erosion-legibility K5/K9 (substitution, `commitment_coverage`), K7 (`unanswered_grievances`, burial), K8 (`reporter_targeting`, "turn the ladder on the member that filed the evidence") |
| `14 supported scenarios, 14/14 passed`; moves 8-13 with latency, damage and bounds; `10 drawn rounds · seed 20260729 · no failures` | real run, 2026-10-01: `PYTHONPATH=. python3 src/python/examples/capture_harness_demo.py` (system Python 3.11, compiled `_core.abi3.so`, bytecode writing disabled) |
| "criteria are fixed before the run" | `doc/capture.md`: "scores fourteen scenarios against falsification criteria *stated in advance*" |
| "two hundred runs returned a constant ... measuring nothing" | `doc/capture.md`: "Running the harness two hundred times proved the scoring function was a constant ... it had been measuring nothing" |
| "Drift made expensive, slow, and legible." | erosion-legibility, *What is and is not claimable*: "It can make drift **expensive, slow, and legible**" |
| "Cooperation without a sovereign." | title of `doc/concept.md` |

Kept off screen: the README's unit-test count (listed, not run).

## Documentation notes (reported, not edited)

1. **Erosion-alert dimension count.** `doc/capture.md` says "The alert now carries
   fourteen dimensions, and only four of them fire on an act. Seven fire on
   omission ... three fire on intensification." The code (`ErosionAlert`,
   16 fields), README component 6, `doc/crates.md` ("sixteen signals") and
   erosion-legibility K9 (fifteenth and sixteenth dimensions,
   `commitment_coverage` and `commitment_contraction`) all say sixteen. The code
   settles it; the screen shows the sixteen fields. `capture.md` predates K9.
   `protocol-components.md` line 608 uses "fourteen dimensions" correctly, as the
   count *before* K9.
2. **How many capture moves assume an adversary.** `doc/capture.md` says six of
   the seven; erosion-legibility says all seven. The narration avoids the count.
3. **Cue preset metadata.** Not a project doc, but the previous plan relied on it:
   the music-cue JSON's `analysis.windowDuration` is 25s while its `beats` and
   `strongCues` run the whole track, so the previous cut's "extrapolated grid"
   was unnecessary.
