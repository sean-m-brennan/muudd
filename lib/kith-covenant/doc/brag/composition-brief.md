# Composition brief — Kith-Covenant (narrated)

Handoff brief for `composition/index.html`. Read `brag-plan.md` first for the
angle, the sourcing table and the measured narration timings; this file is the
build contract.

## Fixed parameters

| | |
|---|---|
| Resolution | 1920 x 1080, landscape, 30fps |
| Duration | 52.0s, set by seven measured Kokoro clips (46.464s of speech) |
| Tone | `cinematic` — restrained, unhurried, no hype |
| Voice | Kokoro `am_michael`, speed 0.96, narration-led |
| Runtime | GSAP, single paused timeline, `window.__timelines["main"]` |
| Fonts | generic `system-ui` / `ui-monospace` stacks; no `@font-face` is shipped |
| CLI | pinned `hyperframes@0.8.47` |

## Structure

Seven `.clip` sections, each on its own track index, crossfading (0.45s in,
0.35s out) rather than cutting. Nothing is a sub-composition; the whole cut is one
file, which is why `composition_file_too_large` stands as an accepted warning.

| # | Section id | Track | Start | Duration | Carries |
|---|---|---|---|---|---|
| 1 | `#hook` | 0 | 0 | 7.30 | slug, rule, the claim |
| 2 | `#inv` | 1 | 7.10 | 7.45 | the inversion table, 4 rows |
| 3 | `#anch` | 2 | 14.35 | 7.35 | 4 substrate cards + the sangha figure |
| 4 | `#filt` | 3 | 21.50 | 8.15 | the `AntiSpiralFeature` enum + 5 glosses |
| 5 | `#cer` | 4 | 29.45 | 8.15 | 2 intra-rites, the witness bridge, the tally |
| 6 | `#lens` | 5 | 37.40 | 7.90 | the dashboard, 6 lens panels |
| 7 | `#lock` | 6 | 45.10 | 6.90 | wordmark, tagline, music credit |

Every element inside a section starts at `opacity: 0` (set at t=0 in the `HIDDEN`
list) and is brought up by its own tween. No tween measures the DOM; everything
spatial is CSS.

## Audio tracks

| Track | Contents | Level |
|---|---|---|
| 10 | music bed (low-passed copy) | tweened: 0.10 → 0.13 → 0.16 → 0 |
| 11 | `rollover2` sequential ticks | 0.16 – 0.18 |
| 12 | `bong_001` structural accents | 0.20 – 0.22 |
| 13 | `click_002` record rows | 0.20 – 0.22 |
| 14 | `impactSoft_medium_001` wordmark | 0.26 |
| 20–26 | the seven narration WAVs, one track each | 1.0 |

Voice clips never share a track index, so each scene boundary can be derived from
a measured duration rather than guessed.

## Beat grid

`happy-beats-business-moves-vol-12`, tempo 109.96 BPM. The bundled cue preset
analyses only the first 25s of the track. Inside that window, strong cues are used
as strong cues (17.468, 18.559, 19.650, 21.832, 22.923). Past 25s the grid is
**extrapolated** as `t = 0.5573 + n × 0.5455` and used only for smaller entrances.
Sequential text always snaps every *other* beat (1.091s), because the grid itself
is 0.545s — below the readable threshold.

## Colour contract

Tokens are lifted from `src/python/dashboard/static/index.html` (dark theme). Three
are raised for WCAG AA at video type sizes, preserving hue and the
ink > ink-2 > muted ordering:

- `--muted` `#898781` → `#a3a199`
- `--series-1-text` `#5a9df0` (text only; the chart stroke keeps `#3987e5`)
- `--critical` `#d03b3b` → `#e8736b` (the shipped value fails AA at 3.62:1)

`check` enforces this and reports 64/64 text checks passing.

## Editorial rules this composition obeys

1. **The voice never reads the screen.** Every narration line was checked against
   every on-screen string; where they converged, the screen text was changed or
   cut, not the voice.
2. **Nothing on screen is invented.** Each figure, name, CID and source path comes
   from the project's own documentation, source, or a real run of its demos — see
   the sourcing table in `brag-plan.md`.
3. **Nothing editorialises.** No risers, no swells, no verdict animation. The lens
   scene in particular is descriptive only, which is what the dashboard itself
   says it is.
4. **State the claims directly.** The prose in the narration, the captions and the
   share copy inherits the project's own stance, per the Documentation Voice rule
   in the root `CLAUDE.md`. No hedges, no soft-apologetics, no framing that
   presents the claim as one option among several.

## Rebuild

```bash
./build.sh                 # check, render, poster, webm
SKIP_RENDER=1 ./build.sh   # transcode an existing brag.mp4 only
```

The narration WAVs in `composition/assets/voice/` are **source**, not build
output: `build.sh` skips the TTS backend entirely and never regenerates them.
