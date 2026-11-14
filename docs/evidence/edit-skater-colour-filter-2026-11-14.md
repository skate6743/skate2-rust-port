# Edit Skater's Apply Color — November 14, 2026

PC port addition. In a shirts list (T-Shirts, Button Shirts, Hoodies, Jackets, Sweaters) or the
pants list, the button help reads Y Apply Color while a part is highlighted (not the Remove row).
Y on the part:

- wears it as A does, with the shop's verdict on it: a locked part sounds `core_error` and stays
  off, one the balance does not cover opens `ID_CAC_NOT_ENOUGH_CASH`, a new one joins the cart;
- opens its wear styles menu on its colour. A part with a colour of its own (a customizable part)
  opens its own colour row, as A does. Any other opens a colour row of the clothing colours: the
  part's colour filter, in the colour row's place, then the Style and Graphic rows the part has.

On the filter row, Left and Right step through the colours and the garment wears each at once; A
opens the colour picker on the filter's colour; B goes back to the list with the part worn.

## The filter

The row holds the tops' colours (seven: blue, green, black, white, red, yellow, purple), with the
swatches the customizable shirts' colour row shows. Their tints are for the customizable parts'
textures (white's base is 1.8, 1.8, 2.2), so they are not the filter's: on another garment, white's
would brighten it about twice.

A filter makes the garment's own colours on screen its colour's swatch times as bright, channel by
channel, over the row's white swatch (0.9, the swatch whose least channel is largest). White leaves
the colours as they are and is no filter at all; the row stands on white for a garment with none.

The cloth programs (`cacstamp_cloth`, `cacstamp_cloth_ropa`) square the diffuse they multiply by a
customization colour's base, and their tone curve takes the square root again, so a filter keeps
its colour on screen squared as that base. Its highlight is 0: the cloth programs read none.

The colour picker on the filter row works on the colour on screen. The wheel's white at intensity 1
is no filter; past 1 the filter brightens the garment. Its lit swatch is the row's colour nearest
the one picked.

## The record

- Two customization colours, keys 13 (shirts, the outer torso) and 14 (pants), each bound to its
  garment by model and material id as a zone's colour is bound to its part; the colour capacity
  becomes 15. Neither is a Skate 2 zone, so `colour_ids` and an outfit's twelve colours hold
  neither.
- A garment keeps its filter while it is worn. Assembly drops a filter bound to another part than
  the one its slot wears: another garment's preview shows none, and wearing another garment (A or Y
  on it, or the Remove row) leaves the filter behind for good. The worn garment's preview, and B
  out of the list, show it again.
- The Style row moves the filter to the wear style it puts on.
- The cart giving a slot back the undo point's garment (Edit Cart, Back to Store, the online game
  closing the editor) gives back the undo point's filter with it.
- A gender switch keeps no filter for the outfit put away (an outfit holds twelve colours).
- The skater wears the filter in the game, and other players see it online: the character sent
  keeps every customization colour.

## Diagnostics

- `SKATE2_DIAGNOSTIC_COLOUR_FILTER`: each filter change, and the size of the recipe the record
  keeps on leaving the editor (8192 bytes at most).
- `SKATE2_DIAGNOSTIC_EDIT_SKATER_APPLY=<step>,...`: after `SKATE2_DIAGNOSTIC_EDIT_SKATER_PATH` and
  `_PART` reach a part, a step every 30 ticks: A, B, X, Y, the D-pad's U, D, L, R, S (a capture,
  `<capture>_s<n>.png`), `-` (none) and `H<ticks>` (the left stick held right).

## Verified

Scripted runs at 1920x1080 on a test save:

- A Skate 2 T-shirt (white, not owned): the button help reads Y Apply Color left of A Select and
  B Back. Y wears it (25 to the cart) and opens Wear Styles on the Color row, lit on white, the
  shirt unchanged; red, yellow and green go on; A opens Pick Color on green, and a picked colour
  stays after A. Back in the list the worn shirt keeps it; another shirt's preview shows none, the
  worn one's shows it again. Y on another shirt (worn) leaves the filter behind: the first shirt's
  preview then shows none and Y on it stands on white. Red, then B out of the list: the shirt
  wears red.
- Skate 2 jeans: red and purple; another pair's preview shows none; out of the list the jeans keep
  purple.
- A Skate 2 hoodie with three wear styles: the filter stays on through hood up and worn open; the
  T-shirt under the open hoodie keeps its own colours.
- The Customizable Tee: Y opens its own Color row and Add a graphic, as A does.
- A Skate 3 T-shirt (female): blue and yellow.
- A bought shirt with red, the editor closed: the save holds colour 13 bound to the shirt. The
  record's recipe was 3224 bytes. Loaded again: the skater wears it, Y's row stands on red, and B
  out of the list puts it back after another shirt's preview. In the game the skater wears it, and
  online a player spectating the skater sees it.
