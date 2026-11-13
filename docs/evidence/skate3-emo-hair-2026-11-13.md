# Skate 3's emo hair in Back Alley Pete's — November 13, 2026

PC port addition. With a linked Skate 3 data folder and Skate 3's Skate.Create Upgrade Pack
linked, Back Alley Pete's Hair offers that pack's two emo hair styles beside Skate 2's. Skate 3's
other hair styles stay out: they do not fit Skate 2's heads.

## The parts

The pack's clothing database (`cas_db_xfd1ca62f_0NN.recipe`, which also holds the parts of the
packs before it) adds eight parts to the `Hair` slot (2). Each style has a full model and a hat
model (`HairModelType` `full` and `hat`) of the same `HairStyle`:

| Model | Asset id | Gender | `HairModelType` | `HairStyle` | Material |
|---|---|---|---|---|---|
| `hair_emo_short` | `0x82a29d00b562571f` | male | full | emo short | `male_emo_short_mat` `0x2c7f3817004700a3` |
| `hair_emo_short_hat` | `0x82a29d00b5625710` | male | hat | emo short | `male_emo_short_mat` |
| `hair_emo_long` | `0x82a29d00b562574a` | male | full | emo long | `emo_long_material` `0x2c7f3817004700a8` |
| `hair_emo_long_hat` | `0x82a29d00b562574d` | male | hat | emo long | `emo_long_material` |
| `female_hair_emo_short` | `0xd99e3231b2d8c33e` | female | full | emo short | `female_emo_short_mat` `0x2c7f38170047008d` |
| `female_hair_emo_short_hat` | `0xd99e3231b2d8c33c` | female | hat | emo short | `female_emo_short_mat` |
| `female_hair_emo_long` | `0xd99e3231b2d8c339` | female | full | emo long | `female_emo_long_material` `0x2c7f381700470020` |
| `female_hair_emo_long_hat` | `0xd99e3231b2d8c33d` | female | hat | emo long | `female_emo_long_material` |

Every one is colourizable, no slot default, costs -1 (free, `GetReceiptBalance` reads it as 0) and
requires the hairline shadow (`RequiresHairlineShadowStyle` `on`). They carry a DLC id and an
unlock, which the pack join clears for parts whose files the pack carries, as for its clothing.

Skate 2's own database uses the same attributes and values (`HairModelType` `full` and `hat`,
`RequiresHairlineShadowStyle` `on` and `off`, the rostrals' `HairlineShadowStyle` `on`), so the
face's hairline shadow (`GetRostralID`) and the hat swap (`CASAssembler::Correct`: a hat whose
`HairRequired` is `hat` takes the hair part of the same style, gender and `HairModelType`) treat
them as Skate 2's own.

## The join

`Skate3Data::clothing_for` (`crates/skate2-core/src/skate3/skate3.rs`) joins Skate 3's parts of
the Slappy's slots and, of the `Hair` slot, only the parts whose `HairStyle` is one of
`JOINED_HAIR_STYLES` (`emo short`, `emo long`). The joined parts come after Skate 2's in the slot
and read as Skate 3's (`CacPartDb::is_skate3`). Their models and textures load from the pack's
archives, which join the game's mounts with the rest of the linked data.

## The menu

- With any Skate 3 hair joined, Back Alley Pete's Hair opens Hair Pools (`EditSkaterMenu::HairPools`)
  with a Skate 2 row and a Skate 3 row, as Slappy's Merchandise opens Merchandise Pools. Each
  opens the Hair menu; its style selector steps through the chosen game's styles of the skater's
  sex (Skate 2's list keeps Bald first for a male skater). Without them the Hair row opens the
  Hair menu directly, as the original does.
- A list without the worn style (Skate 3's while a Skate 2 style is worn, or the other way round)
  starts at its first style, or at its last going back.
- The selector names a Skate 3 style in Skate 3's English for `ID_CAC_HAIRSTYLE_<STYLE>` when the
  linked data has it, else by the style's own words (`Emo Short`, `Emo Long`).
- A saved skater who wears one once the data or the pack is unlinked shows the slot's default hair
  (`wearable_parts`, `fill_missing_parts`), as Skate 3 clothing does.
