# Darkcatch sounds against TU3 — November 23, 2026

The Skate 3 physics darkcatch sounds (`skate2-audio/src/player/contacts.rs`, `tricks.rs`,
`physics.rs`, `skate3_mix.rs`, `global_effects.rs`) read again against the TU3 executable and
the values of Skate 3's and Skate 2's attribute databases.

## What matches

| Sound | TU3 | Checked |
| --- | --- | --- |
| The darkcatch flag | controller +0x301 = SkaterData +0x98 bit 0x20 (0x824b17f0) | state flag 80 |
| Five-update grace | 0x824b17fc..0x824b1834: +0x308 set and +0x304 cleared in a darkcatch, +0x304 counted to 5, then +0x308 cleared | the same counts |
| Catch order | UpdateParams 0x824b8218: CheckForFootCatches, then CheckForFootScrapes (which holds the inverted latch +0x1E4) | the first update's catch is not inverted in either |
| Catch type by feet | PlayFootCatchHelper 0x824b9508: 0 feet first, 1 second, 2 both and reset | the same |
| Catch thresholds | `aud_splicer` footcatch: air time 25 ms, speed 0, level 1 from 2.0, level 2 from 5.0 | the same keys |
| Catch cues | 0x824c0b90: plain 1112.., 1115.., 1118..; inverted 1159.., 1162.., 1165.. by level | the same keys |
| Catch voice | 0x82497f48 / 0x82498248: stage 1, SubMix and two Sends (the environment send at 0, the effect bus) | `Skate3Splice` |
| Catch update | 0x824bebd8: volume output 8, pitch 1, azimuth 0, spread 0, reverb send output 21 | the same |
| Landing condition | LandGround 0x824bac14: +0x301 or +0x308 | the same |
| Landing cue | deck material when the deck touches (controller +0x266, +0x294), else 0; its ollie-and-land material (0x82494d78) wood or concrete; air time under 0.75 s: concrete 860 / 861, wood 862 / 863 | the same |
| Landing update | UpdateJumpsAndLands 0x824be900: volume output 3, pitch 1, reverb send output 15 | the same |
| Effect bus | 0x82491108 / 0x824916e8: bus 8 is the master mix; the local player's sound switches the bus on and sets its two filters and clip; 0x82491180 switches every bus off each update | the same |
| Flip-back scrape | CheckForFootScrapes 0x824b9a74..0x824b9ac0: when the darkcatch ends, the right foot's inverted scrape (cue 97) unless its scrape sounds | the same |
| Flip class members | Class_Flips 0x824afad8 called from 0x824cc564: members 7 to 11, 16 to 19, 21, 23 to 27 | the same order |

## What differed

| # | TU3 | The port | Change |
| --- | --- | --- | --- |
| 1 | The flip class's trick volume (member 21) is Skate 3's `TrickVol`, 10000 | Skate 2's, 16897 | the Skate 3 class reads its volumes from Skate 3's database |
| 2 | The flip class's last member is the flip's effect-table entry, 6 (`GlobalAttributeCollection` +0x8C, 0x824cc4d4) | read from `aud_senseofspeed`, which has no such attribute: 0 | read from the effect table |
| 3 | A foot scrape plays into the effect bus its table entry names (0x824b9864..0x824b98b8; bus 1, the catches' and the landing's) | the inverted scrapes played into the master mix | the inverted scrapes play into that bus |
| 4 | Destroy 0x824b81cc clears the inverted latch | kept | cleared |
| 5 | The controller's Reset 0x824b0be8..0x824b0bf0 clears the grace | the grace was the contacts' and survived a reset | the grace is the controller's and its reset clears it |

Bus 1 takes its own attributes: no clipping (100), a first filter of 150 to 500 Hz, gain 0.75 to
1.25, Q 0.25 to 1.5, and a second of 1000 to 7500 Hz, gain 0.25 to 2.5, Q 0.25 to 1.5, each a
random tenth step when the sound starts.

Differences 4 and 5 show only when a skater's audio state detaches during a darkcatch.

## Not verified

- None of the changes was compiled or heard.
- What Skate 3's flip module does with member 27.
- Skate 3's contacts mixer output 9 (the scrapes' volume) against Skate 2's, which the port reads.
