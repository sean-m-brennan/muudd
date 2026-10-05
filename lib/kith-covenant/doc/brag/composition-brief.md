# Composition brief — Kith-Covenant (narrated, "the quiet polity")

Handoff brief for `composition/index.html`. Read `brag-plan.md` first for the
angle, the source table and the measured narration; this file is the build
contract.

## Fixed parameters

| | |
|---|---|
| Resolution | 1920 x 1080, landscape, 30fps |
| Duration | 50.0s, set by seven measured Kokoro clips (44.203s of speech) |
| Tone | `cinematic` capability film — restrained, no verdicts |
| Voice | Kokoro `am_michael`, speed 0.96, narration-led |
| Runtime | GSAP, single paused timeline, `window.__timelines["main"]` |
| Fonts | generic `system-ui` / `ui-monospace` stacks; no `@font-face` shipped |
| CLI | pinned `hyperframes@0.8.47` |

## Structure

Seven `.clip` sections, one track each, crossfading (0.45s in, 0.35s out). One
file, no sub-compositions, so `composition_file_too_large` stands as an accepted
warning.

| # | Section | Track | Start | Duration | Carries |
|---|---|---|---|---|---|
| 1 | `#hook` | 0 | 0 | 7.00 | slug, *No adversary.*, the one-rite timeline |
| 2 | `#stop` | 1 | 6.65 | 8.12 | four lapses, the alert's three inputs, `None` |
| 3 | `#pin` | 2 | 14.42 | 7.60 | headline, 100/400/400/400 bars, rolling vs pinned |
| 4 | `#null` | 3 | 21.62 | 7.06 | the 43-year empty office, `ActivityAttestation` |
| 5 | `#wide` | 4 | 28.28 | 8.30 | the sixteen `ErosionAlert` fields in four kinds |
| 6 | `#harn` | 5 | 36.18 | 8.55 | real harness output, moves 8-13 |
| 7 | `#lock` | 6 | 44.37 | 5.63 | wordmark, tagline, music credit |

Every element inside a section starts at `opacity: 0` (the `HIDDEN` list at t=0)
and is brought up by its own tween. Nothing measures the DOM.

## Audio tracks

| Track | Contents | Level |
|---|---|---|
| 10 | music bed (low-passed copy), 50s | tweened 0.10 → 0.13 → 0.16 (47.9s) → 0 (48.8-50.0s) |
| 11 | `rollover2` sequential arrivals | 0.16-0.18 |
| 12 | `bong_001`, pinned reading and attestation record | 0.20 |
| 13 | `click_002`, rite, severity line, harness rows | 0.18-0.20 |
| 14 | `impactSoft_medium_001`, wordmark | 0.26 |
| 20-26 | the seven narration WAVs, one track each | 1.0 |

Each voice clip ends before its scene's crossfade begins (smallest margin 0.28s, clips 2-4;
clip 1 has 0.30s).

## Beat grid

`happy-beats-business-moves-vol-12`, 109.96 BPM, beat spacing 0.5455s. Beats and
strong cues are read straight from the shipped preset, whose arrays cover the
whole track. Strong-cue locks: 12.550 (severity), 18.564 (rolling reading),
22.372 / 22.930 / 24.555 / 26.738 (scene 4), 36.548 and the six harness rows
37.105 → 42.562, 44.745 (wordmark), 46.370 (tagline). Sequential text snaps every
other beat; the four gap bars go one per beat because their labels are
three-digit numbers that hold to the scene end.

## Editorial rules

1. The voice never reads the screen and the screen never repeats the voice.
2. Every figure traces to the docs, the code, or a real run (source table in
   `brag-plan.md`). Where docs and code disagree, the code wins: sixteen alert
   fields, not fourteen.
3. Nothing editorializes: no sting on `None`, no flourish on PASS.
4. The limit is stated as the project states it, directly: legible, not prevented.

## Rebuild

```bash
./build.sh                 # check, render, poster, webm
SKIP_RENDER=1 ./build.sh   # poster + transcode from an existing brag.mp4
```

The WAVs in `composition/assets/voice/` are source, not build output.
