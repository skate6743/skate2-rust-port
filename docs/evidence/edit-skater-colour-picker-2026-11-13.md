# Edit Skater's colour picker — November 13, 2026

PC port addition. A on a colour row whose colours tint their parts (clothing, hair, skin) opens a
colour picker in the editor's card, on the colour worn. The skater wears the colour as it changes;
A keeps it, B puts the old one back. Eye and deck colours keep the original rows: each is a part
of its own.

## Controls

| Control | Effect |
|---|---|
| Left stick | Moves the cursor over the hue and saturation wheel (squared response past a 0.2 deadzone, 1.2 wheel radii a second at full deflection) |
| D-pad | Steps the cursor 0.025 radii (`AptDUp`/`AptDDown`/`AptDLeft`/`AptDRight` repeats), sounding `core_nav_*` |
| Right stick (its longer axis) | Right or up brightens, left or down darkens |
| Right / left trigger | Brightens / darkens (`GP_RTrigger`/`GP_LTrigger` travel, 0.1 deadzone) |
| A | Keeps the colour and closes the picker (`core_a_button`) |
| B | Puts the colour the picker opened on back and closes it (`core_b_button`) |
| X | Puts that colour back and stays (`core_x_button`) |

The stick and trigger rates move the intensity slider 0.55 of its length a second at full
travel. `cas_slider` loops while a stick or trigger moves the colour, as under a morph slider's
stick, and stops at a bound or once they rest. An update integrates at most 0.1 s.

The frontend sample (`try_pause_menu_input`) adds `ManualLStickY` (207), `ManualRStickX` (204),
`ManualRStickY` (206), `AptDLeft` (152), `AptDRight` (153) and the gameplay map's `GP_LTrigger`
and `GP_RTrigger` (80, 81) to the states it reads; `ManualLStickX` (205) was already
`morph_axis`.

## The colours

A `cac_colour` row holds the highlight tint at +0, the swatch at +16 and the base tint at +32
(each three floats) and its name at +52. The CAS shaders read a zone's tints: cloth, skin and face
(`cacstamp_cloth`, `cacstamp_skin`, `cacstamp_face`) multiply their diffuse by the base, hair
(`cac_hair`) takes the base for its diffuse and the highlight for its specular colour, and
`cacstamp_shift` takes neither. The rows' own colours:

| Zone | Rows | Base and highlight |
|---|---|---|
| 0, 9, 10 (hats, tops, socks) and the bottom zone (tops' colours) | Swatch = base = highlight (tops, hats); socks' swatches are lighter than their tints | White: base 1.8, 1.8, 2.2 (tops) or 0.4, 0.4, 0.5 (socks), highlight 0 |
| 1 (hair, also set on 8) | Swatch lighter than both | Both set, highlight the lighter |
| 2 (skin, also set on 3, 4, 5) | Swatch = highlight | Base differs in hue |
| 6, 11 (eyes, decks) | Swatch only | 0: the colour is a part of its own |

A row is a row of tints when one of its colours has a tint that is not 0.

The picker sets one tint, the primary, and the other follows it:

- The primary is the base when a colour with a lit swatch has no highlight (tops, socks), else the
  highlight when one has no base, else whichever tint lies nearer the swatches (hats: base; hair
  and skin: highlight).
- The other tint follows the primary as the row's colour nearest the picked primary has it: that
  colour's ratio of the two, per channel (a channel under 0.02 takes the overall ratio). So a row's
  own colour picked is that colour again, and a skin colour's base keeps the calibration of the
  skin parts its nearest colour wears. Ratios stay within 0..=4, and every tint channel within
  0..=8, the bounds the replay archive and the online character keep.

The picker works in HSV on the primary. The wheel holds hue (red at the top, clockwise through
yellow, green, cyan, blue and magenta) and saturation (white at the centre). Intensity is the
value: the game's own tints pass 1 (white tops 2.2), so it runs to 2.5. The slider gives its
first 0.6 to intensity 0..1 and the rest to the brighter tints. A swatch shows the tint itself up
to intensity 1, then the full colour lightened toward white, half way at 2.5.

## The record

- A picked colour writes the zone's customization base and highlight, and those of the zones set
  with it (hair with 8, skin with 3 to 5), bound to the worn part as a row's own colour is
  (`SetColourForZone`).
- `colour_ids` takes the key of the row's colour nearest the picked primary. The game reads hair
  and skin colours by that colour's name: the face's hairline shadow (`GetRostralID`) and the
  skin parts (`SKIN_TONE`). The bottom zone has no `colour_ids` entry.
- The picked colour goes on the skater only. The row keeps its own colours and swatches, and no
  swatch is added for the picked colour. Its lit swatch is the row's colour that `colour_ids`
  names, the nearest. The bottom zone lights the colour whose base is nearest the one worn; a
  colour of the row's own is still found by its exact base.
- Left and Right from a picked colour step to the lit colour's neighbours and wear them, as from
  any of the row's own colours. At either end of the row, they wear the colour at that end.
- The online game closing the editor for the player puts back the colour an open picker opened on.
- A change of colours alone keeps the built skater: `EditSkaterSession` compares the record's
  assembled parts and customizations with the ones the skater was built from. When only the
  colours differ, the skater's materials take the new colours (`set_colours`), and the room keeps
  its GPU objects, whose CAS constants it writes each frame. Any other change loads the skater
  again, as a part, morph or stamp change did before. A skin or hair colour whose nearest colour
  changes wears other parts, so it loads again.

## On screen

- The picker opens and closes as a menu change does: the title panel flickers and the selection
  plays its morph mode again.
- The title reads Pick Color. The button help reads B Cancel, A Accept and X Undo Changes
  (`ID_COMMON_CANCEL`, `ID_COMMON_ACCEPT`, `ID_CAC_MORPHING_UNDO`); Start and its chevrons are
  hidden.
- A colour row of tints shows A Pick Color while it is selected.
- In the morph menu's place, from its first row: the colour row (`highlight_color`, arrows hidden),
  its own swatches, lit on the colour nearest the one picked.
- Below it, 168 units down, the wheel: 80 units in radius, with a cream rim, centred over the middle
  of the Intensity row's line. Its cursor is `highlight_color`'s swatch item in its selected (`on`)
  state, painted the colour picked.
- Under the wheel, 252 units down, an Intensity row (`highlight_morph`). The intensity's ramp
  replaces its line, under its knob, inside the line's width. The colour under the knob's middle
  is the intensity's along the knob's travel, black at its bottom and the brightest tint at its top,
  each held out to the frame's ends.
- A new colour moves the cursor, the knob, the ramp and the row's lit swatch without replaying
  their selection; a swatch's state is recorded and replayed only when it changes.

## The morph knob's travel

`highlight_morph`'s constructor (`0x20e4..0x20f5`) keeps `mMorphLength`, the line's `_width`
less the knob's, and `UpdateDisplay` (`0x2414..0x2440`) puts the knob's `_x` at the line's plus
`mMorphLength` times the value over 100. At 100 the knob therefore ends at the line's end. The
knob's `_width` is its first frame's (lit), whose glow piece is flipped. Every morph row takes this
travel: Edit Skater's body and face morphs, the Intensity row, and the Options rows on the same
control (Field of View, Brightness, the stick deadzones).
