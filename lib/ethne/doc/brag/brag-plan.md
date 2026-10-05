# Brag Plan: Ethne (narrated cut, re-concept 2026-10-01)

Invoked as `/brag --voice`, so narration is on for this run. This is a full
re-concept. The 2026-09-18 cut (the argument spine: gap, discipline, charter,
rite, border, `cargo tree`) is backed up in `previous-2026-09-18/`. Its palette,
document shell, music treatment and ducking helper are reused here. Its angle
and storyboard are not.

## What is this app?

Ethne (Greek ἔθνη, "peoples") is the **polity tier**. It turns a set of
cooperating agents into a polity, "a bounded, self-governing community that
stands for something, acts with one voice, and persists across the turnover of
its members" (`doc/concept.md`). It reads the emergent structure and offers it
back to the members for ratification into a charter.

## The angle

**A founding that has to tell on itself.**

The previous cut argued the tier's architecture. This one follows a single
founding, and the scenario is the one `doc/charter.md` builds its central
chapter around: three founders inside a thirty-node cluster charter the whole
cluster as a polity of themselves. Ethne cannot stop that, and the documentation
says so: a genuinely separate group and a faction capturing the cluster look
the same, and telling them apart needs a judgment no rule can make. What it does
instead is remove the silence. The founding has to say whom it excluded and why,
permanently. Anyone holding the charter can re-run the legitimacy judgment from
the charter alone, and the excluded can answer back, alone, even after the
polity is gone.

That is the capability: **accountability rather than prevention**, which is the
charter chapter's own phrase for the construction. The video shows a mechanism
that admits its own limit and then makes the limit legible.

The picture carries the evidence (the thirty-node field that never cuts away,
the record panel beside it with the real record kinds and figures). The voice
carries the argument and never reads what is on screen.

## Hook (first 2-4 seconds)

Thirty nodes arrive in a field and wire themselves together. Beside them:
*A cluster is not a community.* Then three questions the cluster cannot answer
(`members?` / `stands for?` / `speaks for it?`), and on the word *boundary* a
dashed slate hull appears around the nodes and keeps shifting.

The voice says none of that text. It describes thirty machines that deal with
each other and a boundary that will not hold still.

## Key moments (the middle)

- **Ratify, not replace.** The two ruled-out moves (*impose a structure* /
  *relabel the cluster*), then `FoundingProposal` → a majority co-signs →
  `PolityCharter`. In the field the hull stops moving, goes solid, the nodes
  sign gold one by one, and the boundary turns gold.
- **Three inside thirty.** Everything but three nodes falls back. The gold hull
  still claims the whole cluster. The other 27 go dim: *unrecorded*. Two
  identical miniatures, *a genuinely separate group* = *a faction capturing it*.
- **The attestation.** `BoundaryAttestation` counts to candidate 30 · roll 3 ·
  exclusions 27, a tick lands under each excluded node, the two reasons the
  substrate handover distinguishes appear, and `GENESIS-IMMUTABLE` stamps.
- **The judgment, re-run.** Consent share = ratifiers ÷ everyone the boundary
  showed. The chapter's own worked case, a faction of one, reads 0.033. Founding
  basis: corroborated · narrowed · self-attested. No provider consulted.
- **The excluded answer back.** One of the 27 lights, ringed. `BoundaryDissented`:
  signed by the dissenter alone, no deciders, membership not required. The
  charter dissolves in the field and the dissent stays lit.

## Outro / punchline

Field recedes. **Ethne**, **ἔθνη**, and the governing discipline the voice never
says: *Ratify emergence, do not replace it.* The voice gives the other half:
*Not prevention. Accountability.*

## User flow worth showing

The tier has no UI. The flow is the life of one founding, recreated from the
record kinds `doc/charter.md` and `doc/crates.md` name: emergent cluster →
`FoundingProposal` → `PolityCharter` with its `BoundaryAttestation` → founding
legitimacy read model → `BoundaryDissented` → `CharterDissolution`. No program
output is shown or quoted. Every figure is from the documentation (table below).

## Tone

- Preset: **cinematic**, restrained.
- Creative direction: *a capability film about a constitution, told through one
  founding.*
- Interpretation: one persistent field, long holds, a record panel that changes
  beside it. Nothing celebrates; the stamp and the dissolution land flat.
- Voice: `am_michael` (Kokoro) at speed 0.96. Calm, measured, not an announcer.

## Format: landscape — 1920x1080, 30fps
## Duration: 48.0s, set by the measured narration

Seven clips total 41.39s of speech. The 15-25s creative law is exceeded under
the skill's voiceover carve-out (*"let the voice set the pace"*), within the
45-50s brief for this series.

## Visual identity (authored, reused from the previous cut)

Ethne ships no UI, CSS or images, so the palette is authored and documented as
such: background `#0b0d12`, surface `#141821`, elevated `#1c212c`, border
`#2b3242`; text `#ece9e3`, muted `#c9c4b8`, dim `#aba598`. **Slate `#85a9cd`** is
the substrate and what emergence proposes; **gold `#e3b25a`** is this tier, the
ratified thing; **paper white** marks the excluded member who speaks for himself.
Generic sans and mono stacks (no `@font-face` ships). 60/60 text checks pass
WCAG AA.

## Share copy (draft)

Three nodes inside a thirty-node cluster charter the whole cluster as a polity
of themselves. Ethne cannot stop that. It makes the founding say whom it left
out, permanently, and lets the excluded answer back.

## Audio direction

- Role: **narration-led.** The bed sits under the voice.
- Music: `happy-beats-business-moves-vol-9-by-ende-dot-app.lowpass.mp3`, the
  same low-passed 48.05s cut as the previous version (ende.app, CC BY 4.0 with
  an attribution-optional waiver; see `CREDITS.md`). 114.84 BPM, beat ≈ 0.5225s.
- Treatment: 0.10 → 0.11 under the hook, 0.11 under every voice span, opening to
  0.24 only in gaps longer than 0.75s, faded to silence over the last 1.2s
  (46.8 → 48.0).
- Music cue guidance: bundled preset
  `assets/music/happy-beats-business-moves-vol-9-by-ende-dot-app.music-cues.json`.
  Every narration clip starts on a beat-grid point. Strong-cue locks:
  **7.396** (hull appears on *boundary*), **14.756** (the members sign),
  **20.526** (the charter claims the cluster), **28.444** (attestation counters
  settle on *left out*), **39.486** (the dissent record, on *answer back*),
  **43.700** (wordmark with *Ethne*), **45.801** (tagline). Narration wins every
  conflict.
- Audio-reactive treatment: subtle. Bass breathes the panel borders and a faint
  gold glow on the boundary. `audio-data.js` is unchanged (it ends at 46.27s;
  the last frame holds through the fade).
- SFX: five, all low high-frequency-risk: `rollover2` (signing), `bong_001`
  (the claim), `impactSoft_medium_003` (the stamp), `click2` (the dissenter
  lights), `impactSoft_medium_001` (wordmark).
- Restraint rule: nothing marks the capture or the dissolution as drama.

## Voiceover script

Seven clips, `hyperframes tts --voice am_michael --speed 0.96`. Measured with
`ffprobe`; transcribed with `hyperframes transcribe` (whisper `small.en`), which
returned every word of every line, so nothing was dropped.

| # | File | Line | Measured |
|---|---|---|---|
| 1 | `vo-1-cluster.wav` | "Thirty machines deal with each other reliably. Ask who belongs, and all you get is a boundary that keeps moving." | 7.424s |
| 2 | `vo-2-ratify.wav` | "Ethne doesn't impose a structure, or relabel the cluster. The substrate proposes. The members ratify, by signing." | 8.021s |
| 3 | `vo-3-faction.wav` | "Nothing stops three of them chartering the whole cluster as a polity of themselves. A splinter and a capture look the same." | 7.573s |
| 4 | `vo-4-account.wav` | "So the founding must say who it left out, and why. Permanently." | 4.181s |
| 5 | `vo-5-rerun.wav` | "Anyone holding the charter can rerun that judgment and get the same answer, forever." | 5.547s |
| 6 | `vo-6-dissent.wav` | "And any one of the twenty-seven can answer back, alone, even after the polity is gone." | 5.781s |
| 7 | `vo-7-lockup.wav` | "Ethne. Not prevention. Accountability." | 2.859s |

Total speech **41.386s**. Clips 2 and 7 are generated from the spelling
`Ethnee` (Kokoro reads `Ethne` as *Ethan*; `Ethnee` gives ETH-nee). Every
on-screen spelling is `Ethne`.

A border beat ("six kinds of artifact carry the polity to the compact") was
written and measured (6.34s) and cut for length: it took the film to 54s and
belongs to a different film than this one.

## Storyboard (final, from the measured clips)

| Scene | Span | Narration | What is seen |
|---|---|---|---|
| 1 Hook | 0 → 8.75 | VO1 1.068 → 8.492 | 30 nodes and their edges arrive; *A cluster is not a community.*; three unanswerable questions on alternating beats; dashed slate hull on *boundary* (7.396), drifting |
| 2 Ratify | 8.75 → 17.5 | VO2 8.963 → 16.984 | ✕ impose a structure / ✕ relabel the cluster; `FoundingProposal` (12.655), hull fixes and goes solid (13.177), nodes sign gold (14.756), `PolityCharter`, hull gold (16.335) |
| 3 Three inside thirty | 17.5 → 25.95 | VO3 17.914 → 25.487 | 27 nodes fall back (18.437); *Charter · claims the cluster* (20.526); the 27 go dim, legend roll 3 / unrecorded 27 (21.595); two identical miniatures (22.64), `=` (23.174), *No rule can tell them apart.* (24.218) |
| 4 The attestation | 25.95 → 30.8 | VO4 26.331 → 30.512 | `BoundaryAttestation` counts 30 / 3 / 27, settling on 28.444; 27 ticks land under the excluded; two exclusion reasons (28.955); `GENESIS-IMMUTABLE` stamp (29.478) |
| 5 The judgment, re-run | 30.8 → 37.0 | VO5 31.057 → 36.604 | consent share formula (31.591); *a faction of one reads* 0.033 (33.17 → 34.226); founding basis chips (34.749); *no provider consulted · no cohesion input* (35.805) |
| 6 The excluded answer back | 37.0 → 43.4 | VO6 37.373 → 43.154 | one excluded node lights and is ringed (38.429); `BoundaryDissented` record, rows on alternating beats from 39.486; charter dissolves in the field, *after dissolution: may still be filed* (42.121) |
| 7 Lockup | 43.4 → 48.0 | VO7 43.700 → 46.559 | field recedes to 0.16; **Ethne** (43.7), **ἔθνη** (44.222), *Ratify emergence, do not replace it* (45.801), music credit (46.324) |

No clip overruns its scene. Gaps between lines: 0.47, 0.93, 0.84, 0.55, 0.77,
0.55s; every start is a beat of the bed.

Poster: **30.3s**, the settled attestation frame.

## Source table (every on-screen figure and spoken claim)

| On screen / spoken | Source |
|---|---|
| Thirty nodes, one cluster; "three of them … the whole cluster as a polity of themselves" | `doc/charter.md`: "nothing stops three men inside a thirty-node cluster from chartering the whole cluster as a polity of themselves, with the other twenty-seven unrecorded rather than merely outvoted" |
| "machines deal with each other reliably" | `doc/concept.md`: "Nine machines that reliably deal with each other are not a community, they are a cluster in a graph" (the count is the charter chapter's thirty) |
| "A cluster is not a community." | `doc/concept.md`, same sentence, condensed |
| "a boundary that keeps moving" | `doc/concept.md`: "no answer comes back that anyone agreed to, only a boundary that emerged and keeps moving" |
| `members?` no answer anyone agreed to · `stands for?` behavior is not a commitment · `speaks for it?` nobody | `doc/concept.md`, opening paragraph: the same three questions and answers |
| "Ratify emergence, do not replace it" | `README.md`: "The governing discipline is **ratify emergence, do not replace it**" |
| ✕ Impose a structure: a diktat rather than a people · ✕ Relabel the cluster: the mask has no inside | `doc/concept.md`: "a community handed down is a diktat rather than a people … We could have relabeled the emergent cluster … since the mask has no inside" |
| "The substrate proposes. The members ratify, by signing." | `doc/charter.md`: "The trust substrate proposes a boundary … and the members ratify it, or ratify something narrower, by signing" |
| `FoundingProposal` — no authority on its own; a majority of the proposed roll co-signs; `PolityCharter` — the founding signed document | `doc/charter.md`: "A proposal carries no authority on its own"; "Ratifying that proposal, with a majority of the proposed roll co-signing, yields a `PolityCharter`"; `doc/concept.md`: "A *charter* is the founding signed artifact of a polity" |
| roll 3 · unrecorded 27 | `doc/charter.md` (row 1) |
| a genuinely separate group = a faction capturing it; "a splinter and a capture look the same"; *No rule can tell them apart.* | `doc/charter.md`: "a genuinely separate group of three inside a cluster of thirty is indistinguishable from a faction capturing it, and telling them apart needs a substantive judgment no rule can make" |
| "So the founding must say who it left out, and why. Permanently." | `doc/charter.md`: "A founding must say what it excluded and why, that statement is permanent and immutable" |
| `BoundaryAttestation` on every proposal and every charter; candidate size the boundary showed 30 · ratified roll 3 · reasoned exclusions 27 | `doc/charter.md`, "Founding exclusions": "Every proposal and every charter carries a `BoundaryAttestation`, holding the candidate size the boundary actually showed and a reasoned exclusion for everyone it showed whom the roll does not name" |
| "3 + 27 = 30 · the numbers must add up" | `doc/charter.md`: "The numbers must add up, exclusions may not repeat, and nobody may be excluded and enrolled at once" |
| Exclusion reasons: below a reputation floor · no reading at all | `doc/guardianship.md`, "The substrate handover": "The exclusions record which distinction applied, so an excluded party can read the difference between falling below a floor and having no reading at all" |
| GENESIS-IMMUTABLE | `doc/charter.md`: "The attestation is genesis-immutable by construction" |
| "Read from the charter alone"; *no provider consulted · no cohesion input*; "Anyone holding the charter can rerun that judgment and get the same answer, forever." | `doc/charter.md`: "Our read model computes the founding legitimacy from the charter alone, with no provider consulted and no cohesion input, so anyone holding the charter can re-run the judgment forever and get the same answer" |
| consent share = ratifiers ÷ everyone the boundary showed; *a faction of one reads* 0.033 | `doc/charter.md`: "It reports a consent share, being ratifiers over everyone the boundary showed … and the faction of one described above reads 0.033" |
| founding basis: corroborated · narrowed · self-attested | `doc/charter.md`: "It also reports a founding basis: corroborated, narrowed, or self-attested" |
| `BoundaryDissented`; the reason recorded for an exclusion, rejected; signed by the dissenter alone; deciders none, and no majority; membership not required; "any one of the twenty-seven can answer back, alone" | `doc/charter.md`, "The excluded answer back": "A dissenter says either that he was never named at all, or that the reason recorded for his exclusion is rejected. It is signed by the dissenter alone, involves no deciders and no majority, and the dissenter need not be a member"; "a dissenter rejecting a reason must be among the exclusions" |
| after dissolution: may still be filed; "even after the polity is gone"; *Charter dissolved* | `doc/charter.md`: "A dissent may be filed after dissolution, because otherwise founders silence the excluded by winding the polity up" |
| "Not prevention. Accountability." | `doc/charter.md`: "The construction is accountability rather than prevention" |
| Ethne · ἔθνη · the polity tier | `README.md`, opening line |
| Chapter 17 — from cooperation to community; Chapter 18 — founding a polity | `doc/crates.md`: "chapters 17 through 22, beginning at [from cooperation to community]"; the README's chapter order puts *Founding a polity* second |

Nothing on screen is invented. The node layout, the hull, and the choice of
which node dissents are illustration, and are labeled as a cluster view rather
than as program output.

## Documentation problems noticed (reported, not edited)

*Items 1 to 3 were resolved in the documentation on 2026-10-02: every reference now
says D1 to D40, `crates.md` follows the code's numbering (roll five, rite six,
roster seven, compact records eight), and `uplift.md` covers the D40 records.*

1. **Decision range disagrees in four places.** `doc/design.md` says D1-D40 (D39
   and D40 locked 2026-10-01). `README.md` says "D1 to D38". `doc/concept.md`,
   `doc/charter.md`, `doc/guardianship.md`, `doc/transmission.md`,
   `doc/cohesion.md` and `doc/uplift.md` all still link "Design decisions D1 to
   D35".
2. **Border-artifact count and numbering.** `doc/uplift.md` and `doc/concept.md`
   say six kinds of artifact cross to the compact, matching the six in
   `src/rust/en-uplift/src/lib.rs` (handoff, concern, fission, envoy, member
   roll, rite, with fusion emitting fission's record types), plus a seventh, the
   relay roster, that goes to the substrate instead. But `doc/crates.md` numbers
   them differently: it calls `envoy_delegation` "the fourth border artifact",
   `member_roll` "border artifact six", and `rendezvous_roster` "border artifact
   seven", which leaves no slot for the rite and disagrees with the code's list,
   where the roll is fifth and the rite sixth.
3. **"and nothing else does" is now overtaken by D40.** `doc/uplift.md` says six
   kinds of artifact cross "and nothing else does", but D40 and
   `src/rust/en-uplift/src/compact.rs` add compact-member records the envoy and
   root sign at the border (anti-spiral claims, feature acceptance, sponsorship,
   examiner registry, anathema, lift). They are record types the compact already
   has, so the "own vocabulary" claim still holds, but the closed count does
   not. This film does not use the border count.
4. **Stale snapshots.** `composition/snapshots/` still holds frames of the
   2026-09-18 cut. They are not referenced by anything and will be replaced on
   the next snapshot run.
