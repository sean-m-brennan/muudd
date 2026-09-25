# Credits

## Music

**"Happy Beats / Business Moves Vol. 12"** by **ende.app** — https://ende.app/en

Bundled with the `/brag` skill at
`skills/brag/assets/music/happy-beats-business-moves-vol-12-by-ende-dot-app.mp3`
(1:58, tempo 109.96 BPM). The skill's own `assets/music/README.md` notes that the
exact licence terms are still to be verified and documented, so no licence is
asserted here; the attribution block travels with the video regardless, in
`share-copy.txt` and on screen in the closing frames.

### Treatment

The track is bright for a bed sitting underneath speech, so what plays in the
composition is a low-passed copy. Both files are kept in
`composition/assets/music/`; the unfiltered original is the one to start from if
the bed is ever re-cut.

```bash
ffmpeg -y -i composition/assets/music/happy-beats-business-moves-vol-12-by-ende-dot-app.mp3 \
    -af "lowpass=f=2600" -c:a libmp3lame -q:a 4 \
    composition/assets/music/happy-beats-business-moves-vol-12-by-ende-dot-app.lowpass2600.mp3
```

Measured effect: **−7.4 dB** of energy above 4 kHz against the original
(−33.4 dB → −40.8 dB mean over the first 30s, high-passed at 4 kHz). The point is
to clear the speech presence band so the bed reads as a bed rather than as a
second voice; the level tweens (0.13 under every narration line) do the rest.

The whole 117.36s file ships, uncut. The render uses only the first 52.0s and the
bed fades to silence at 52.0s.

The cue preset
`happy-beats-business-moves-vol-12-by-ende-dot-app.music-cues.json` ships alongside
it, also from the `/brag` skill.

## Sound effects

From the bundled `/brag` skill library (`skills/brag/assets/sfx/`), all CC0
(Kenney.nl). Only picks from the "low high-frequency risk" list in
`sfx-analysis.md` were used:

- `ui/rollover2.ogg` — sequential arrivals
- `interface/bong_001.ogg` — the two structural reveals
- `interface/click_002.ogg` — archive record rows
- `impact/impactSoft_medium_001.ogg` — the wordmark

## Narration

Synthesised voiceover, Kokoro `am_michael` at speed 0.96, generated once into
`composition/assets/voice/`. These seven WAVs are **source, not build output** —
`build.sh` deliberately skips the TTS backend, so they must be tracked alongside
the composition rather than regenerated.

Verified against the delivered audio with `whisper.cpp` (`ggml-base.en`); the
"As synthesized" transcript is in `brag-plan.md`. All seven clips came back
complete.

## Vendored runtime

`composition/assets/vendor/gsap.min.js` — GSAP, copied from the AutonomousTrust
composition in this repo so the render has no network dependency.

## Project material

Everything on screen — the 1259 BCE treaty, the inversion table, the four named
substrates, the sangha's ≈23-century run, the `AntiSpiralFeature` enum, the
ceremony DIDs and CIDs, and every lens value — comes from Kith-Covenant's own
documentation, source and demos. The per-item sourcing table is in `brag-plan.md`.

## Music licence — CONFIRMED 2026-09-18

Verified directly at <https://ende.app/en/standard-license>. The earlier note in
this file saying the terms were unverified is superseded.

The ende.app catalogue (Sascha Ende) is released under **Creative Commons
Attribution 4.0 International (CC BY 4.0)**, with commercial use explicitly
permitted, and the composer grants an additional **"attribution optional"**
waiver on top of the licence. Remixing, cutting, looping and filtering are
expressly allowed, which covers the low-pass treatment applied to the bed here.
Attribution is given anyway.

**Two hard prohibitions, which apply to this video:**

1. **Never register this music with YouTube Content ID** or any equivalent
   system. Doing so would let the registrant issue copyright claims against
   every other user of the track; the composer states this results in a ban and
   legal action. Check that any distributor used to publish this video does not
   auto-enroll uploads in Content ID.
2. Do not upload the track unchanged as a standalone song to Spotify, Apple
   Music or similar.

Note this differs from the Kevin MacLeod bed used by `continuous-space` and
`autonomous_trust`, which is CC BY 4.0 **with no waiver** — there, attribution is
mandatory and must travel with the video.
