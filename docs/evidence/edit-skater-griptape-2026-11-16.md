# Edit Skater's Edit Griptape — November 16, 2026

PC port addition. In the deck list (Slappy's, Skateboards, Decks) the button help reads
Y Edit Griptape while a deck is highlighted. Y on the deck:

- wears it as A does, with the shop's verdict on it, as Apply Color does a garment
  (`edit-skater-colour-filter-2026-11-14.md`);
- opens a wear styles menu titled Edit Griptape that holds one row, Color: the clothing colours,
  as Apply Color's row. The camera stays the deck list's, and the board turns its top face to it.

On the row, Left and Right step through the colours and the griptape wears each at once; A opens
the colour picker on the griptape's colour; B goes back to the list, where the board shows its
graphic again. Edit Deck (A on the Custom Deck: Color and Add a graphic) is as it was.

## The griptape

- A deck is one mesh. Two deck models were read: the Custom Deck's (`cacstamp_cloth`, 484
  vertices, two texture coordinate sets) and the brand decks' (`defaultcharacter`, 472 vertices,
  one set). Each has 712 triangles, laid out over the first set in islands that share no vertex:
  - the top face, one island of 158 triangles within u 0.008 to 0.898, v 0.012 to 0.486;
  - the bottom face within v 0.512 to 0.994 (one island of 138 triangles on the brand deck; three
    of 50, 49 and 39 on the Custom Deck, whose second set places its graphic there);
  - the two rails, 208 triangles each, at u 0.921 to 0.939 and 0.954 to 0.973.
- The griptape is the top face (`crate::character::grip_surface`): of the islands that lie flat in
  the mesh's bind space (more than half of their area looks up or down), those on the higher
  side. Its colour goes over the rectangle of the first set they span. A deck where another face
  reaches into that rectangle, or with no two flat sides, has no griptape colour.
- The griptape's diffuse is dark: squared, as the character programs read it, its mean over the
  top face is 0.079 on both models in the editor (0.28 on screen) and 0.114 on the brand deck's
  part worn in the game.

## The colour

- Over the rectangle the programs draw the diffuse squared times the colour's tint over that
  mean: the griptape keeps its grain and takes the colour as a white garment takes a colour
  filter's. The rails and the bottom face keep their own colours.
- `cacstamp_cloth` and `cacstamp_shift` read the tint over the mean and the rectangle from the
  third and fourth custom rows, which hold a skin's tattoo choices and nothing for a cloth;
  `defaultcharacter` reads the same two rows of its constants, zero on every other mesh.
- The tint is a colour filter's: the colour on screen, squared.
- A griptape wears a tint as a surface does (2026-11-21, `grip_surface::worn`). The diffuse squared
  times the tint over the mean is the share of the light the griptape sends back, and at the
  picker's intensity 1 a full colour sent all of it back; past 1, up to 6.25 times it, which the
  bloom showed as a light of its own. Now:
  - a tint whose brightest channel is at or under 0.2 is worn as it is (the row's colours are);
  - past that the brightest channel bends toward 0.5 and never passes it
    (`0.2 + 0.3 * (1 - exp(-(tint - 0.2) / 0.3))`: 0.48 at the picker's intensity 1), the other
    channels keeping their ratio to it;
  - past the picker's intensity 1 the colour grows purer instead of brighter, each channel moved
    away from the colour's own grey by up to 0.4 of its distance at the greatest intensity (2.5),
    so a grey stays a grey and a hot pink loses the green in it;
  - the diffuse squared counts for no more than 1.5 times its mean over the top face, so a bright
    mark in the griptape takes the colour and does not shine. The programs read that ceiling from
    the fourth lane of the third custom row, which was 1 before.
  - Seen in a scripted run with `SKATE2_DIAGNOSTIC_GRIP_TINT=6.25,1.05,3.15` (a hot pink at the
    picker's greatest intensity) on the Custom Deck in direct sun: the griptape's pixels average
    191, 22, 142 of 255 and peak at 219, 26, 156, none at the top of the range. The tint goes on
    every deck with a griptape, whatever the skater's own colour. Edit Griptape's own menu was
    not walked again on this build.
- The row holds the tops' seven colours. A griptape wears each at four tenths of the colour a
  garment's filter has (red is 0.31, 0.04, 0.04 on screen), which a garment's colours are too
  vivid for. White is no colour: the deck's own griptape, where the row stands for a deck with
  none.
- The colour picker opens on the colour worn, so on a row colour it opens at that four tenths.
  Its white at intensity 1 is no colour.

## The view

- In the Edit Griptape menu every bone from `skateboard_root` down takes half a turn about the
  board's long axis (z in its meshes' bind space) through the middle of the deck, trucks and
  wheels as drawn, so the board rests where it did with its top face to the camera.
- The bones' matrices take a vertex as the character programs skin it (`skin_world`): at twice its
  stored position, 0.8 higher. The default board's stored positions span y -0.400 (the wheels) to
  -0.336 (the deck's top), 0 to 0.128 as skinned; it turns about y 0.064.

## The record

- One customization colour, key 15, bound to the deck it was picked for by model and material id
  as a colour filter is bound to its garment; the colour capacity becomes 16. It is no Skate 2
  zone, so `colour_ids` and an outfit's twelve colours do not hold it.
- A deck keeps its griptape colour while it is worn. Assembly drops one bound to another deck
  than the one worn: another deck's preview shows none, and wearing another deck leaves it behind
  for good.
- The Custom Deck's own colours are a part each; its Color row moves the griptape colour to the
  part it puts on.
- The deck drawn in the game is another part of the installed deck (`assemble_parts`), so the
  colour goes on the deck drawn, whichever of them that is, and is no colour of the whole part.
- The skater's deck wears it in the game: the character kept on leaving the editor holds every
  customization colour.

## Diagnostics

- `SKATE2_DIAGNOSTIC_GRIP`: each deck mesh's griptape (its rectangle and its diffuse's mean
  squared) as its scene is built.
- `SKATE2_DIAGNOSTIC_COLOUR_FILTER`: each change of the griptape's colour, with the filters'.

## Verified

Scripted runs at 1280x720 on a test save:

- A brand deck (Spectrum): the button help reads Y Edit Griptape left of A Select and B Back. Y
  opens Edit Griptape on the Color row, lit on white, the griptape its own grey; red, yellow and
  black go on at once; A opens Pick Color (X Undo Changes, A Accept, B Cancel) on the colour worn,
  and a picked colour stays after A. B goes back to the list with the graphic showing.
- Another deck highlighted, then the first again: Y's row stands on the colour. A on another deck
  leaves the colour behind: Y on the first deck then stands on white.
- The Custom Deck: A opens Edit Deck (Color, Add a graphic) as before. Y opens Edit Griptape; red
  and blue go on. Its own colour changed from blue to green in Edit Deck: Y's row still stands on
  blue.
- A red griptape on the brand deck, the editor closed: the record's recipe was 3244 bytes. In the
  game the skater's deck wears it; the rails, trucks and wheels keep their own colours.

These runs were made before two changes, which no scripted run has covered since:

- the board's turn takes the middle as skinned. A deck dumped in the deck list, projected with
  that turn, spans x 671 to 876 and y 45 to 684 on a 1280x720 screen, against x 676 to 873 and
  y 74 to 665 as it lies (the turn puts the deck's top where the wheels' far side was). With the
  stored middle it spanned x 520 to 921 at mid-height, as the earlier build drew it;
- the griptape wears the row's colours at four tenths.

Both were looked at in a play test of the build that has them.

## Not verified

- Another player's view of the griptape online.
- The Skate 3 pool's decks, and a skater with a griptape colour loaded with no Skate 3 data
  linked.
- The crate's tests were not run; `grip_surface`'s four were, compiled on their own.
