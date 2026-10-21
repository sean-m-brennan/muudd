# Hyperframes Composition Brief: MUUDD (narrated cut, re-concept 2026-10-01)

## Objective

A short narrated capability film for **MUUDD**, the physical layer of a
user-owned communications network: one ridgetop relay, designed so that nobody
has to climb up to it, followed through power, radios, failure, software and
repair, ending on ownership and the optional stack.

## Output

- Composition directory: `lib/muudd/doc/brag/composition/`
- Rendered video: `lib/muudd/doc/brag/brag.mp4` (rendered later by `build.sh`)
- Format: landscape, 1920x1080, 30fps
- Duration: **49.5s**, set by the eight measured narration clips (voiceover
  carve-out), not by the default 15-25s law

## Source Material

- Project root: `/home/user/Software/continuous-space/lib/muudd`
- Primary files read: `README.md`, `doc/narrative-order.json`, Part I
  (`doc/muudd.md`, `doc/audience_profile.md`, `doc/hardware_design.md`,
  `doc/device_model.md`, `doc/software_architecture.md`,
  `doc/anonymous_communication.md`, `doc/verifiable_infrastructure.md`,
  `doc/the-stack.md`) and appendix A `doc/rural-network.md`
- Product name: **MUUDD** (Mesh-routed User-owned User-controlled Dynamic Domains)
- Strongest claim: the design target is zero site visits a year
  (`doc/hardware_design.md`)
- Key visuals to recreate: the Tier 1 relay as its bill of materials describes
  it; the radio ladder; a backbone where each ridgetop links to at least two
  others; the unikernel image spec; the remote-diagnosis fields
- Copy that must appear verbatim:
  - `Target: zero.`
  - `The physical layer neither knows nor cares which.`
  - `who may cooperate?` / `what is a community?` /
    `how do communities treat one another?`
  - `Hardware that moves bits without asking permission.`
  - every figure in the source table of `brag-plan.md`

### Scope

MUUDD proper is only the physical layer. AutonomousTrust, ethne and
Kith-Covenant appear once, in scene 7, as dim outlined optional rows above a
solid base, never as features of MUUDD. Nothing from `lib/autonomous_trust/` is
shown or used; the schematic in scene 4 is authored and labeled *schematic*.

## Creative Direction

- Tone preset: **cinematic**
- Creative direction: *one relay through one winter, with nobody coming.*
- Interpretation: wide dark frames, monospace figures, fast-in then hold, no
  flourish. The relay dying is routine: one dry hit, no sting, no red flash.
- Angle: every physical-layer chapter answers the same constraint, a relay
  nobody should have to visit. The video shows those answers on one site.
- Hook: night ridge, one mast; slug, `SITE VISITS PER YEAR`, **Target: zero.**
- Outro: wordmark, expansion, *Hardware that moves bits without asking
  permission.* The voice says *Nobody bills for it. Nobody can switch it off from
  a distance.*
- Avoid: generic SaaS language; abstract filler; any figure not in the source
  table; any suggestion the upper tiers are part of MUUDD; reading on-screen
  text in the narration.

## Visual Identity

Carried over from the 2026-09-18 cut (authored; MUUDD ships no stylesheet):
background `#0a0e0d`, surface `#111917`, elevated `#18211e`, border `#27332f`;
text `#e6ece9`, muted `#c6d2cd`, dim `#aebab5`; link/radio `#6fd2e0`, valley
`#8fc79a`, solar `#f2ba5c`, fault `#f08c7a`. Generic sans and mono stacks (a
named family without `@font-face` is a lint error).

## Storyboard

`brag-plan.md` is the creative contract. Summary:

1. **Hook**, 0-7.95: ridge, mast, `Target: zero.`
2. **Power**, 7.95-13.88: site sheet; panels and battery lit; sun-arc inset;
   sizing rows; 14% margin
3. **Radios**, 13.88-19.26: antennas and beams; four bands; Part 97 exclusion
4. **A relay dies**, 19.26-25.61: schematic; packet, death, quiet households,
   reroute; what a node knows; a retransmission
5. **The image**, 25.61-32.15: enclosure, restart ring, image spec, an evening
6. **Remote diagnosis**, 32.15-36.75: LoRa telemetry to a household; 1 trip,
   not 3
7. **The stack**, 36.75-43.76: three optional tiers above MUUDD
8. **Lockup**, 43.76-49.5

## Audio

- Role: narration-led.
- Music: `assets/music/happy-beats-business-moves-vol-10-by-ende-dot-app-lowpass.mp3`
  (reused). 0.09 under the hook, 0.22 in gaps, 0.13 under voice, fade to 0 over
  48.5-49.5s. `data-duration` 49.5.
- Cue source: `beats/assets/music/happy-beats-business-moves-vol-10-by-ende-dot-app.mp3.json`
  (from `hyperframes beats`, reused). Locks marked `// beat-locked` in the
  timeline: 7.88, 11.97, 13.88, 20.70, 23.97, 30.52, 35.15, tier rows 37.06 /
  38.15 / 39.25 / 40.34, tagline 46.88.
- Audio-reactive: subtle; `assets/audio-data.js` (reused, covers 60s) drives the
  LED halos and the backhaul beam fill.
- SFX (reused, all low HF risk): `impactSoft_medium_001` for swaps, the relay
  dying and the lockup; `bong_001` for four locked figures and the tier rows;
  `rollover2` for the reroute.

## Voiceover

Eight Kokoro clips, `am_michael` at speed 0.96, in `composition/assets/voice/`
(`vo-1-hook` … `vo-8-lockup`), each on its own track at gain 1 and each wholly
inside its scene window. Lines, durations and start times are in `brag-plan.md`.

## Hyperframes Instructions

- One paused GSAP timeline at `window.__timelines["main"]`.
- Four `.clip` sections (hook, site sheet, stack, lockup); everything inside the
  site sheet is an opacity/transform tween on a child, never a clip.
- Text swaps are stacked labels toggled with `tl.set` opacity at scene
  boundaries, not `tl.call` text writes, so every seek lands on the right label.
- Spatial values precomputed at setup; no tween measures the DOM.
- Gate: `npm run check` (pins `hyperframes@0.8.47 check --timeout 30000`), zero
  errors. Result on 2026-10-01: 0 errors, 89/89 text checks pass WCAG AA, 0
  layout, motion or runtime findings; one accepted warning,
  `composition_file_too_large`.
