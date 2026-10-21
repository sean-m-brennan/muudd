# Credits

## Music

**"Happy Beats / Business Moves Vol. 12"** by **ende.app** (Sascha Ende) —
https://ende.app/en

Bundled with the `/brag` skill at
`skills/brag/assets/music/happy-beats-business-moves-vol-12-by-ende-dot-app.mp3`
(1:58, tempo 109.96 BPM).

### License (verified 2026-09-18)

Verified directly at <https://ende.app/en/standard-license>. The ende.app catalog
is released under **Creative Commons Attribution 4.0 International (CC BY 4.0)**,
with commercial use explicitly permitted, and the composer grants an additional
**"attribution optional"** waiver on top of the license. Remixing, cutting,
looping and filtering are expressly allowed, which covers the low-pass treatment
below. Attribution is given anyway, on screen in the closing frames and in
`share-copy.txt`.

**Two hard prohibitions, which apply to this video:**

1. **Never register this music with YouTube Content ID** or any equivalent
   system. Doing so would let the registrant issue copyright claims against every
   other user of the track; the composer states this results in a ban and legal
   action. Check that any distributor used to publish this video does not
   auto-enroll uploads in Content ID.
2. Do not upload the track unchanged as a standalone song to Spotify, Apple Music
   or similar.

This differs from the Kevin MacLeod bed used by `continuous-space` and
`autonomous_trust`, which is CC BY 4.0 **with no waiver**; there, attribution is
mandatory and must travel with the video.

### Treatment

The track is bright for a bed under speech, so the composition plays a low-passed
copy. Both files are kept in `composition/assets/music/`; start from the
unfiltered original if the bed is ever re-cut.

```bash
ffmpeg -y -i composition/assets/music/happy-beats-business-moves-vol-12-by-ende-dot-app.mp3 \
    -af "lowpass=f=2600" -c:a libmp3lame -q:a 4 \
    composition/assets/music/happy-beats-business-moves-vol-12-by-ende-dot-app.lowpass2600.mp3
```

Measured effect: **−7.4 dB** of energy above 4 kHz against the original
(−33.4 dB → −40.8 dB mean over the first 30s, high-passed at 4 kHz).

The whole 117.36s file ships, uncut. This cut uses only the first 50.0s: the bed
holds at 0.13 under the narration, lifts to 0.16 after the last word, and fades
to silence over 48.8-50.0s.

The cue preset
`happy-beats-business-moves-vol-12-by-ende-dot-app.music-cues.json` ships beside
it, also from the `/brag` skill.

## Sound effects

From the bundled `/brag` skill library, all CC0 (Kenney.nl). Only picks from the
"low high-frequency risk" list in `sfx-analysis.md`:

- `ui/rollover2.ogg` — sequential arrivals
- `interface/bong_001.ogg` — the two structural reveals (the pinned reading, the
  attestation record)
- `interface/click_002.ogg` — the sealed rite, the severity line, harness rows
- `impact/impactSoft_medium_001.ogg` — the wordmark

## Narration

Synthesized voiceover, Kokoro `am_michael` at speed 0.96, via
`hyperframes@0.8.47 tts`, generated 2026-10-01 into `composition/assets/voice/`:
**seven clips**, 44.203s of speech in all (`vo-1-hook`, `vo-2-stopped`,
`vo-3-pinned`, `vo-4-null`, `vo-5-widened`, `vo-6-harness`, `vo-7-lockup`). These
WAVs are **source, not build output**; `build.sh` never regenerates them. The
previous cut's seven clips are kept in `previous-2026-09-18/`.

Verified against the delivered audio with whisper.cpp (`ggml-small.en`); every
clip came back complete. Script and measured durations are in `brag-plan.md`.

## Vendored runtime

`composition/assets/vendor/gsap.min.js` — GSAP, copied from the AutonomousTrust
composition in this repo so the render has no network dependency.

## Project material

Everything on screen comes from Kith-Covenant's own documentation, source and a
real run of its capture-resistance harness demo: the K1 lapses and the alert's
three inputs, the 100/400/400/400 gap test, the 43-year empty office,
`ActivityAttestation`, the sixteen `ErosionAlert` fields and the harness rows.
The per-item source table is in `brag-plan.md`.
