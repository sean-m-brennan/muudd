# Hyperframes Composition Brief: Ethne (re-concept 2026-10-01)

## Objective

A narrated capability film for **Ethne**, the polity tier: one founding, followed
from emergent cluster to dissolution, showing that the founding must account for
everyone it left out. Narration-led, cinematic, 1920x1080, 48.0s.

## Output

- Composition directory: `composition/`
- Rendered video: `brag.mp4` (rendered later by `build.sh`, not in this run)
- Format: landscape, 1920x1080, 30fps
- Duration: **48.0s**, set by seven measured Kokoro clips (41.386s of speech),
  under the skill's voiceover carve-out.

## Source material

- Project root: `/home/user/Software/continuous-space/lib/muudd/lib/ethne`
- Primary files read: `README.md`, `doc/concept.md`, `doc/charter.md`,
  `doc/guardianship.md`, `doc/transmission.md`, `doc/cohesion.md`,
  `doc/uplift.md`, `doc/crates.md`, `doc/design.md` (D1-D40),
  `src/rust/en-uplift/src/lib.rs`, `src/rust/en-uplift/src/compact.rs`,
  `src/rust/en-uplift/Cargo.toml`
- Product name: **Ethne** (ἔθνη)
- Strongest claim: a founding must say what it excluded and why, permanently,
  and anyone holding the charter can re-run the judgment.
- Key visual: a persistent thirty-node cluster field beside a record panel
  carrying the real record kinds (`FoundingProposal`, `PolityCharter`,
  `BoundaryAttestation`, `BoundaryDissented`).

### Copy that must appear verbatim (or as condensed in the plan's source table)

- "A cluster is not a community."
- members? / stands for? / speaks for it?, with the concept chapter's answers
- "Ratify emergence, do not replace it"
- "a diktat rather than a people" / "the mask has no inside"
- candidate 30 · roll 3 · excluded 27; "the numbers must add up"
- "a faction of one reads 0.033"; corroborated · narrowed · self-attested
- "the dissenter alone" / "may still be filed" after dissolution

Every figure is sourced in the table in `brag-plan.md`.

## Creative direction

- Tone preset: **cinematic**, restrained.
- Creative direction: *a capability film about a constitution, told through one
  founding.*
- Angle: accountability rather than prevention. Ethne cannot stop three founders
  from chartering a thirty-node cluster as a polity of themselves; it makes the
  founding say so, permanently, and lets the excluded answer back.
- Hook: the cluster assembles; the boundary appears on the word *boundary* and
  will not hold still.
- Outro: wordmark, ἔθνη, and the governing discipline. The voice says
  *Not prevention. Accountability.* and never the tagline.
- Avoid: generic SaaS language, filler motion, celebrating the stamp or the
  dissolution, reading on-screen text aloud, the previous cut's argument spine.

## Visual identity

Reused from the 2026-09-18 cut and documented there and in `brag-plan.md` as
authored (Ethne ships no UI): ink background `#0b0d12`, paper text `#ece9e3`,
gold `#e3b25a` for the ratified thing, slate `#85a9cd` for what the substrate
proposes, paper white for the excluded member who speaks. Generic font stacks.

## Structure

One standalone `index.html`:

- `#frame` (track 1, 0-48): topbar, the cluster field (SVG, 30 nodes, 38 edges,
  laid out once from a fixed seed and frozen as constants), the empty record
  panel. Field state changes are tweens on these persistent elements.
- `#sc1`-`#sc6` (track 2): one clip per narrated scene, carrying the record panel
  content, the field status tag and the field legend for that scene.
- `#lockup` (track 3, 43.4-48).
- Audio: `#bgm` (track 10), five SFX (tracks 11-15), seven VO clips (tracks
  20-26), each VO wholly inside its scene clip.

## Scene summary

1. Hook — 0 → 8.75 — the cluster, the three questions, the drifting boundary
2. Ratify — 8.75 → 17.5 — ruled-out moves; proposal → co-signing → charter
3. Three inside thirty — 17.5 → 25.95 — the capture case; identical miniatures
4. The attestation — 25.95 → 30.8 — 30 / 3 / 27, reasons, genesis-immutable
5. The judgment, re-run — 30.8 → 37.0 — consent share, 0.033, founding basis
6. The excluded answer back — 37.0 → 43.4 — `BoundaryDissented`; the charter dissolves
7. Lockup — 43.4 → 48.0

## Audio

- Role: narration-led; the bed is low-passed and subordinate.
- Music: `assets/music/happy-beats-business-moves-vol-9-by-ende-dot-app.lowpass.mp3`
  (48.05s), clip 0-48, 0.11 under voice, 0.24 in gaps over 0.75s, faded out
  46.8-48.0.
- Cue source: the bundled preset beside the track. Every VO start is a beat.
  Strong-cue locks at 7.396, 14.756, 20.526, 28.444, 39.486, 43.700, 45.801.
- Audio-reactive: subtle, panel borders and boundary glow from `audio-data.js`
  (unchanged; ends 46.27s).
- SFX: `rollover2` 14.756, `bong_001` 20.526, `impactSoft_medium_003` 29.478,
  `click2` 38.429, `impactSoft_medium_001` 43.700.

## Gate

`npm run check` (`hyperframes check --timeout 30000`): **passed, 0 errors**.
Two lint warnings remain and are accepted: `composition_file_too_large` and
`timeline_track_too_dense` (six scene clips on track 2), since the film is one
continuous field and splitting it into sub-compositions would split the field.
Layout: 0 issues. Contrast: 60/60 text checks pass WCAG AA.
