# Brag Plan: MUUDD (narrated cut, re-concept 2026-10-01)

Invoked as `/brag --voice`, so narration is on for this run only. This is a full
re-concept of the 2026-09-18 cut, which is kept in `previous-2026-09-18/` (plan,
brief, composition, voice). The CSS tokens, the music bed, the SFX set, the
audio-data file and the audio-reactive helper are carried over; the angle, the
storyboard, the script and every scene are new.

## Rubric

1. **What is the project?** MUUDD (Mesh-routed User-owned User-controlled Dynamic
   Domains) is the physical layer of a communications network that belongs to the
   people using it: solar-powered radios on ridgetops and rooftops, owned by the
   households they serve, passing traffic to each other directly. Three optional
   tiers (AutonomousTrust, ethne, Kith-Covenant) can stack on top, and none is
   required.
2. **Most impressive claim.** The design target for a ridgetop relay is zero site
   visits a year, and every other choice in Part I follows from that target.
3. **Visual hook.** `Target: zero.` under a slug placing one relay on one ridge.
4. **What to show from the project.** There is no UI. The real material is the
   Tier 1 relay's bill of materials and solar sizing, the radio ladder, the
   routing model, the unikernel image and the repair rules, all from Part I and
   appendix A.
5. **Shortest satisfying video.** Set by the narration: 49.5s (see below).
6. **Tone.** Preset `cinematic`, direction *a capability film about one relay
   through one winter, with nobody coming*.
7. **Audio.** Narration-led. The low-passed ende.app bed from the previous cut,
   ducked under every line, with a few quiet low-HF cues.
8. **Share caption (draft).** One ridgetop relay, designed so nobody has to
   climb up to it. That is the whole of MUUDD; everything above it is optional.
9. **User flow worth showing.** A relay's year: it is powered through December,
   it talks on bands nobody has to license, one of its neighbors dies and traffic
   goes around, it restarts itself, and when somebody does have to go they know
   what to bring.

## The angle

**One relay, nobody coming.**

The previous cut was a costing (fiber against mesh, priced to the connector). The
current Part I documentation makes a different and stronger argument, and this
cut follows it: `doc/hardware_design.md` says the design optimizes for *the number
of times per year somebody has to go look at it, and the target is zero*, because
the man who climbs the hill in February is a volunteer with a job. Every
physical-layer chapter then answers that constraint in its own register: power
sized against the worst month (hardware), commodity parts on unlicensed spectrum
(hardware), failures that stay local and routing with no map (software
architecture), an image with nothing to log in to that answers a fault by
restarting (software architecture), and diagnosis from a warm room so that a
repair is one trip (hardware, Repairability).

So the video follows one Tier 1 relay through those answers, and turns at the
end on ownership: a node only someone else can fix belongs to them, and this one
belongs to the households it serves. The three upper tiers appear once, as
optional rows above a solid base, and are never presented as part of MUUDD.

This is a capability film, not a costing and not a mesh explainer. The fiber
comparison is deliberately absent.

## Hook (first 3 seconds)

Night ridge, one mast on the crest, its status LED breathing with the bass. The
slug `RIDGETOP RELAY · BRAXTON COUNTY, WEST VIRGINIA · 38.7°N`, then
`SITE VISITS PER YEAR`, then **Target: zero.** The voice says none of it. It says
who would have to make the climb instead.

## Key moments

- **Power, from December.** The site drawing's panels and battery light up, a
  small inset shows December's sun arc under June's, and the worked solar sizing
  lands row by row, ending on a 14% panel margin.
- **Radios on a shelf.** The dishes, sector, TVWS yagi and LoRa whip light up;
  the beams reach to both ridges; four bands land with what each is for. Amateur
  mesh is named as excluded, with the reason.
- **A relay dies.** The site becomes a backbone schematic. A packet crosses via
  the middle ridgetop, that ridgetop dies on the beat, the households behind it
  go dark, and the next packet goes the other way. Beside it: what a node knows,
  and the one thing it does not.
- **Nothing to log in to.** Back at the site, the enclosure is ringed by a
  restart loop that draws under the word "reboot", next to the image's spec.
- **From a warm room.** The LoRa whip lights, telemetry pulses run down to a
  household, and the card ends on *1 trip, not 3*.
- **The stack.** Three optional tiers above one solid base.

## Outro

Wordmark **MUUDD**, the expansion, and *Hardware that moves bits without asking
permission.* (README.md, verbatim). The voice says something else: *Nobody bills
for it. Nobody can switch it off from a distance.*

## Tone

- Preset: **cinematic**
- Creative direction: *one relay through one winter, with nobody coming.*
- Interpretation: wide dark frames, monospace figures, long holds, nothing
  celebratory. The relay dying lands as routine, with one dry hit and no sting.
- Voice: Kokoro `am_michael` at speed 0.96, measured and calm, not an announcer.
  Short declaratives and fragments. The voice carries the argument; the frame
  carries the evidence; the voice never reads on-screen text.

## Format: landscape, 1920x1080, 30fps
## Duration: 49.5s, set by the measured narration

The default 15-25s law is exceeded under the skill's voiceover carve-out ("Scene
durations must flex to match the generated audio"). Eight clips total 43.09s of
speech; with a 0.80s lead-in, 0.60s between lines and a 1.40s tail the
composition runs 49.5s. The bed is 60s long, so it never runs out.

## Visual identity

Unchanged from the previous cut (MUUDD ships no UI or stylesheet; the palette is
authored from the subject): background `#0a0e0d`, surface `#111917`, elevated
`#18211e`, border `#27332f`; text `#e6ece9`, muted `#c6d2cd`, dim `#aebab5`;
link/radio `#6fd2e0`, valley relay `#8fc79a`, solar `#f2ba5c`, fault `#f08c7a`.
Generic sans and mono stacks; no font ships with the composition.

## Audio direction

- Role: narration-led; the bed sits under the voice.
- Music: `happy-beats-business-moves-vol-10-by-ende-dot-app-lowpass.mp3` (the
  previous cut's low-passed copy, `lowpass=f=2600`). Very low under the hook,
  0.22 in the gaps, 0.13 under every voice span, fading to silence over 48.5 to
  49.5s.
- Music cue guidance: the bundled preset covers only 0-25s, so timings are taken
  from the existing `npx hyperframes beats` file
  (`composition/beats/assets/music/…mp3.json`, 217 points on a 0.2725s grid).
  Strong locks: 20.70s (0.90) for the relay dying, 30.52s (0.91) for "an
  evening", 38.15s (0.98) for a tier row. Narration wins every conflict.
- Audio-reactive: subtle. Bass breathes the status-LED halos (hook and site) and
  the backhaul beams. No waveform, bars or pulsing type.
- SFX: sparse and low-HF only (`impactSoft_medium_001`, `bong_001`,
  `rollover2`, all rated low in `sfx-analysis.json`): card swaps, one dry hit on
  the relay dying, a soft tick when the packet reroutes, quiet bongs on four
  locked figures and the four tier rows, one impact on the lockup.

## Voiceover script (as generated and measured)

Generated with `npx --yes hyperframes@0.8.47 tts "<line>" --voice am_michael
--speed 0.96`. Durations are `ffprobe` measurements of the WAVs. Each clip was
transcribed back (`hyperframes transcribe`, whisper `small.en`) and every word of
every line was delivered; the word counts match the script exactly.

| # | Clip | Line | Measured | Start | End |
|---|---|---|---|---|---|
| 1 | `vo-1-hook.wav` | "Miles from anyone, on a ridge. Whoever climbs up here in February is a volunteer with a job." | 6.805s | 0.80 | 7.61 |
| 2 | `vo-2-power.wav` | "So it is sized against the darkest month, not the average. Nothing with a fan." | 5.333s | 8.21 | 13.54 |
| 3 | `vo-3-radio.wav` | "Shelf parts, on unlicensed spectrum. There is no permission to withdraw." | 4.800s | 14.14 | 18.94 |
| 4 | `vo-4-failure.wav` | "When a relay dies, the houses behind it go quiet, and everyone else routes around it." | 5.845s | 19.54 | 25.39 |
| 5 | `vo-5-image.wav` | "Nothing on it to log in to. Its answer to a fault is to reboot, and become new again." | 5.888s | 25.99 | 31.88 |
| 6 | `vo-6-repair.wav` | "Anyone who does go reads the site from a warm room first." | 3.968s | 32.48 | 36.45 |
| 7 | `vo-7-owned.wav` | "A node only someone else can fix belongs to them. This one belongs to the households it serves." | 6.400s | 37.05 | 43.45 |
| 8 | `vo-8-lockup.wav` | "Nobody bills for it. Nobody can switch it off from a distance." | 4.053s | 44.05 | 48.10 |

Total speech 43.09s. Two earlier drafts were generated and discarded for length
(56.7s, then 45.5s of speech): line 3 lost "Every radio is" and "No license, so",
line 6 was cut from a 6.3s sentence, and line 7 dropped "Everything above it is
optional" because the stack scene says it on screen.

Where the lines come from (paraphrase, never invented):

1. `doc/software_architecture.md` "nobody within five miles of it";
   `doc/hardware_design.md` "the man who has to climb the hill in February is a
   volunteer with a job".
2. `doc/hardware_design.md` "Size the panel against the darkest month rather
   than the annual average"; "rules out anything with a fan".
3. `doc/hardware_design.md` "commodity parts on unlicensed spectrum ...
   replaceable from a shelf"; "A license is a permission, and a permission is a
   thing that can be withdrawn."
4. `doc/hardware_design.md` "a dead relay takes out the households behind it and
   reroutes everyone else."
5. `doc/software_architecture.md` "It cannot be logged into, because there is
   nothing to log in to"; "a system whose answer to confusion is to become new
   again."
6. `doc/hardware_design.md` "a failing site can be characterized from a warm room
   before anyone drives out".
7. `doc/hardware_design.md` "A node that cannot be fixed by the people who own it
   is a node owned by whoever can fix it."; `README.md` "owned by the households
   they serve".
8. `doc/muudd.md` "Nobody bills for it, or can switch it off from a distance."

## Storyboard (final times)

| # | Scene | Window | VO | What is on screen |
|---|---|---|---|---|
| 1 | Hook | 0.00-7.95 | 1 | Night ridge, one mast. Slug, `SITE VISITS PER YEAR`, **Target: zero.** (beat 2.70), cite `doc/hardware_design.md` |
| 2 | Power | 7.95-13.88 | 2 | Site sheet enters. Panels and battery lit; Dec/Jun sun-arc inset; seven sizing rows; `PANEL MARGIN OVER DECEMBER 14 %` (beat-locked 11.97); foot: battery target, three days through an ice storm |
| 3 | Radios | 13.88-19.26 | 3 | Antennas lit, beams to both ridges, four callouts; four band rows on every third beat; foot: amateur mesh excluded, Part 97 |
| 4 | A relay dies | 19.26-25.61 | 4 | Backbone schematic. Packet via middle ridgetop; that relay dies at 20.70 (beat-locked, one dry hit); households behind it dim under "go quiet"; rows *what a node knows*; packet reroutes on 23.97; `A WRONG DECISION COSTS a retransmission` |
| 5 | The image | 25.61-32.15 | 5 | Site returns, enclosure ringed; six image rows; foot on the debugging trade; restart ring draws under "reboot"; `REVIEW BY ONE VOLUNTEER an evening` (beat-locked 30.52) |
| 6 | Remote diagnosis | 32.15-36.75 | 6 | LoRa whip lit; dashed telemetry path to a household with four pulses; four *reported* fields; `CLIMBING THE HILL 1 trip, not 3` (beat 35.15) |
| 7 | The stack | 36.75-43.76 | 7 | Three optional tiers on beats 37.06 / 38.15 / 39.25, the MUUDD base on 40.34; caption *The physical layer neither knows nor cares which.* |
| 8 | Lockup | 43.76-49.50 | 8 | Wordmark 43.88, expansion 45.79, tagline 46.88 (beat-locked), music credit 47.43 |

Every voice clip sits wholly inside its scene window; the nearest margin is
0.30s (VO 6 ends 36.45, scene 6 ends 36.75).

Clips: `#hook` 0-8.1, `#site` 7.8-36.95 (scenes 2-6 as one continuous sheet),
`#stack` 36.55-43.95, `#lockup` 43.6-49.5. Every text swap inside the sheet (mode
chip, panel title, source strip) is a stacked label whose opacity is set at the
scene boundary, so seeking to any frame shows the right label.

**Poster:** 24.8s, the settled failure frame: dead relay marked, its households
dark, all four *knows* rows and the retransmission line landed, packet on the
alternate path.

## Source table

Every on-screen figure and every spoken claim, against the document it comes
from as that document stands on 2026-10-01.

| On screen / spoken | Source and quote |
|---|---|
| Visit target 0; `Target: zero.` | `doc/hardware_design.md` lead: "We optimize for the number of times per year somebody has to go look at it, and the target is zero." |
| Braxton County, West Virginia; 38.7°N | `doc/rural-network.md` Overview and "Solar Sizing Worked Example (38.7°N latitude, Braxton County)" |
| 70 W load; 1,680 Wh/day | `doc/rural-network.md` Tier 1: "Total continuous load: 70 W → 1,680 Wh/day" |
| December 3.0 h; June 5.5 h | same: "approximately 3.0 peak sun hours per day in December (worst month) and 5.5 hours in June" |
| 80 % efficiency; 700 W required | same: "Panel capacity = 1,680 / (3.0 × 0.80) = 700 W" |
| 2 × 400 W fitted; 14 % margin | same: "Two 400 W panels (800 W total) provide 14% margin over the minimum 700 W requirement" |
| 200 Ah LiFePO₄ · 24 V | same BOM and battery sizing |
| Battery target: three days through an ice storm | `doc/hardware_design.md` Power: "Three days of autonomy through an ice storm"; `doc/rural-network.md`: "3 days autonomy for extended cloud cover / ice storms" (the target only; see documentation problems) |
| Solar and battery, no grid | `doc/muudd.md` Hardware: "a node has to run without the grid" |
| Site drawing (two dishes, sector, TVWS, LoRa, enclosure, two panels, mast) | `doc/rural-network.md` Tier 1 BOM; `doc/hardware_design.md` "two backhaul radios pointed at neighboring ridges, one sector antenna serving the valley below, and a low-bandwidth long-range radio" |
| Commodity parts · Part 15, no license | `doc/hardware_design.md`: "commodity parts on unlicensed spectrum, purchasable by anyone and replaceable from a shelf"; "Unlicensed operation under Part 15" |
| Lower bands bend around the ridge; higher bands carry more | `doc/hardware_design.md` Choosing a radio: "lower frequencies bend around obstacles and pass through foliage, where higher frequencies carry more data" |
| 900 MHz LoRa: monitoring, telemetry, emergency text, kilobits | same: "throughput measured in kilobits ... network monitoring, telemetry from the solar systems, emergency text" |
| 470-698 MHz TV whitespace: valley to ridge, no line of sight, 10-15 km | same: "between 470 and 698 MHz ... over ten to fifteen kilometers without line of sight ... For valley-to-ridgetop links" |
| 2.4 / 5 GHz WiFi: relay to the houses around it, under 2 km | same: "from a relay to the houses around it, over distances under a couple of kilometers" |
| 5.8 GHz point-to-point: ridge to ridge, 20-50 km | same: "Ridgetop to ridgetop ... links of twenty to fifty kilometers" |
| Amateur mesh excluded, Part 97 | same: "Part 97 prohibits encrypting amateur transmissions" |
| Each ridgetop links to at least two others; traffic takes the alternate path | `doc/rural-network.md` Ridgetop backbone chain: "Each hilltop relay connects to at least two other hilltop relays ... traffic reroutes through the alternate backbone path" |
| Households behind a dead relay go quiet; everyone else reroutes | `doc/hardware_design.md` Repairability: "a dead relay takes out the households behind it and reroutes everyone else" |
| Routing without a map; no route server, no controller, no consensus about paths | `doc/software_architecture.md` Routing without a map: "There is no route server, no controller, and no consensus about paths to be lost." |
| Knows its neighbors / what they told it / how its traffic got through; not the topology | same: "No node knows the topology ... A node knows its own neighbors, knows what they have recently told it, and knows how well its own recent traffic actually got through." |
| A wrong decision costs a retransmission | same, What a node can know: "a wrong one costs a retransmission rather than an outage" |
| Unikernel: the application, and only the OS it calls | same: "The application and exactly the operating system functions it calls are compiled together into a single bootable image." |
| Six programs, a few hundred lines each; review in an evening | same, Small pieces: "six programs of a few hundred lines each ... are [reviewable by a volunteer in an evening]" |
| No shell; no package manager | same: "an image with no shell, no package manager" |
| A few MB; a fraction of a second; byte-for-byte reproducible | same: "boots in a fraction of a second, occupies a few megabytes, and is byte-for-byte reproducible from source" |
| One immutable image | same: "the image is immutable" |
| Debugging is harder without a shell; trade accepted | same: "debugging is harder without a shell ... We accept that trade" |
| Nothing to log in to; reboot, become new again (VO 5) | same: "nothing to log in to"; "a system whose answer to confusion is to become new again" |
| Long-range low-bandwidth radio on every node; battery voltage, panel current, link states, uptime | `doc/hardware_design.md` Repairability: "The long-range low-bandwidth radio on every node ... reporting battery voltage, panel current, radio link states and uptime" |
| Every part is a catalog part | same: "Every part is a catalog part." |
| 1 trip, not 3 | same: "Climbing the hill with the right part is the whole difference between one trip and three." |
| Fixing and owning (VO 7) | same: "A node that cannot be fixed by the people who own it is a node owned by whoever can fix it."; `README.md` "owned by the households they serve" |
| Three optional tiers and their questions; "neither knows nor cares which" | `README.md` and `doc/the-stack.md` ("None of the three is required ... the physical layer neither knows nor cares which") |
| MUUDD expansion; "Hardware that moves bits without asking permission." | `README.md` title and second paragraph, verbatim |
| Nobody bills for it; nobody can switch it off from a distance (VO 8) | `doc/muudd.md` lead: "Nobody bills for it, or can switch it off from a distance." |

**Authored, carries no reading:** the ridge silhouettes, the proportions of the
site drawing, the schematic's node positions (labeled *schematic*; it is not a
map of Braxton County), and the household drawing.

**Deliberately left out:** the battery autonomy figure (`≈2.8 days`, shown in the
previous cut) and the valley-relay power budget, because the documented
arithmetic did not close (see below; it does now, at about 2.3 and 1.1 days, so
a future cut could show them); CBRS 3.5 GHz, because it is not a Part 15
band; the fiber cost comparison, which belonged to the previous angle.

## Documentation problems noticed (reported, not edited)

*All five were resolved in the documentation by 2026-10-02; the list is kept as the
record of what this cut avoided.*

1. **Ridgetop battery autonomy.** `doc/rural-network.md` (Tier 1 battery sizing)
   says a 200 Ah LiFePO₄ bank at 24 V "provides approximately 2.8 days of
   autonomy with 80% depth of discharge". 200 Ah × 24 V = 4,800 Wh; at 80% DoD
   that is 3,840 Wh, or 2.3 days at 1,680 Wh/day. 2.8 days is the figure at 100%
   DoD. The 420 Ah requirement on the line above also omits DoD (at 80% it would
   be about 525 Ah at 12 V). `doc/hardware_design.md` Power repeats the claim
   that this bank "supplies" three days at 80% DoD.
2. **Valley relay power budget.** `doc/rural-network.md` Tier 2: a 35 W load is
   840 Wh/day, but the single 200 W panel yields 480 Wh in December, so the budget
   does not close in the worst month, which contradicts the "size against the
   darkest month" rule in `doc/hardware_design.md`. The stated "approximately
   1.7 days autonomy" from a 100 Ah 12 V battery (1,200 Wh, 960 Wh at 80% DoD)
   also does not follow; it is 1.1 to 1.4 days.
3. **CBRS listed as unlicensed.** `doc/hardware_design.md` lists 3.5 GHz shared
   spectrum and then says "Every band above is unlicensed ... Unlicensed operation
   under Part 15". CBRS is governed by Part 96 with SAS registration (as
   `doc/rural-network.md` reference 11 says), not Part 15.
4. **Admission model contradicts the stack chapter.** `doc/rural-network.md`
   "Trust-based network admission" has a node vouched in to receive "a signed
   certificate", with routing that forwards "only through nodes with valid trust
   credentials", and says "All MUUDD traffic is end-to-end encrypted via
   AutonomousTrust". `doc/the-stack.md` rejects the credential answer and makes
   AutonomousTrust optional ("Admission needs no certificate authority"), and
   `doc/software_architecture.md` says a mesh with no trust layer "routes on link
   quality alone".
5. **README chapter list out of step with `doc/narrative-order.json`.** The README
   Part II list omits "First contact" (narrative-order n=12), so the README counts
   32 chapters against the JSON's 33, and the README's appendix C says "design
   decisions D1 to D34" where the JSON says "D1 to D35".
