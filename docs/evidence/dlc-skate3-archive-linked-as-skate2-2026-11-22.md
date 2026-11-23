# A Skate 3 pack's archive linked as Skate 2's — November 22, 2026

## The failure

A career start ended with

    controlled tick after 0 completed frames: Failed { stage: Career,
    detail: "data\\scene\\ice_002.bin: ICE group f0a38de7 trailing bytes" }

on some players' games and never on others.

## Cause

Link DLCs takes a content package or an archive extracted from one. A package is told apart by
its title id. An extracted archive has none, so `DlcPack::open` knew Skate 2's six packs by one
entry each (`KNOWN`): `data\db\dlc_maloof_money_cup.vlt` for the Maloof Money Cup Pack,
`data\unlocks\filmpack.unlock` for the Filmer Pack, and so on.

Skate 3's packs hold entries of the same names:

| Skate 3 pack archive | Entry Skate 2's pack is known by |
| --- | --- |
| `maloof_money_cup_00000000.big` | `data\db\dlc_maloof_money_cup.vlt` |
| `filmer_00000000.big` | `data\unlocks\filmpack.unlock` |

The picker tries a file as Skate 2's first and as Skate 3's only when that fails
(`dlc_link::open`). An archive extracted from Skate 3's Maloof Money Cup pack therefore linked
as Skate 2's "Maloof Money Cup Pack", whole, in the game's mounts. That archive also holds
`data\scene\scene_002.bin` and `data\scene\ice_002.bin` with Skate 3's camera elements (two more
than Skate 2's, `ice_camera::SKATE3_ELEMENTS`). `nis::directory::load_with_content` read
`ice_002.bin` with Skate 2's elements; the second group of the file (`f0a38de7`, context
`0x80000006`) is the first whose take has an interval on a channel Skate 3 added a flag to, and
the take ended four bytes short of its group.

Whether it failed depended on the other packs linked: Skate 2's Dyrdek's Fantasy Park Pack holds
its own `ice_002.bin`, and the first linked archive holding a path is the one read
(`MountSet::owner`). A Skate 3 pack linked from its package, or as Skate 3's, never showed its
scene files to the game (`skate3::dlc::game_view` passes its world, mission, model and texture
folders only).

Skate 2's packs' camera files and Skate 3's, read with Skate 2's elements:

| Archive | File | Result |
| --- | --- | --- |
| Skate 2 `skate1_classic_xbox2.big` | `ice_001.bin` | 652 groups |
| Skate 2 `rob_fantasy_plaza_xbox2.big` | `ice_002.bin` | 675 groups |
| Skate 2 `maloof_money_cup_xbox2.big` | `ice_003.bin` | 680 groups |
| Skate 3 `maloof_money_cup_00000000.big` | `ice_002.bin` | `ICE group f0a38de7 trailing bytes` |
| Skate 3 `dway_park_00000000.big` | `ice_003.bin` | the same |
| Skate 3 `player1p_00000000.big` | `ice_004.bin` | the same |
| Skate 3 `creator_00000000.big` | `ice_005.bin` | the same |
| Skate 3 `play_00000000.big` | `ice_006.bin` | the same |

## Fix

- `DlcPack::open`: an extracted archive is one of Skate 2's packs only in the format Skate 2's
  packs have (`BIG4`, `BigFormat::Named`; all six are), with a known pack's entry
  (`extracted_pack`). Skate 3's archives are `EB`. The picker then opens such a file as Skate
  3's pack, as it does a package of Skate 3's.
- `DlcLinks::mount_linked` (launch): a file `config.json` links as Skate 2's that is no pack of
  Skate 2's and opens as Skate 3's moves to the Skate 3 packs the file links (`skate3_dlc`),
  which `Skate3Link::mount_linked` reads next. Any other file that is no pack leaves the file as
  before.
- `nis::directory::load_with_content`: a pack's `scene_NNN.bin` or `ice_NNN.bin` that does not
  read is reported and left out, with the other file of its number, instead of ending the
  career's start. The game's own `scene_000.bin` and `ice_000.bin` still must read.

## Verified

- The failure, on the build before the fix: `config.json` linking only the archive extracted
  from Skate 3's Maloof Money Cup pack logged `downloadable content: Maloof Money Cup Pack` and
  the career start ended with the message above, to the letter.
- `dlc::tests::an_extracted_archive_is_a_pack_in_skate_2s_format_alone`: an archive in Skate 2's
  format with the entry is the pack; one in Skate 3's with the same entries is none.
- `dlc::tests::skate_3s_extracted_archive_is_no_pack_of_skate_2s` (ignored, over the archive
  extracted from Skate 3's Maloof Money Cup pack): `DlcPack::open` answers that it is no Skate 2
  downloadable content file, and it opens as Skate 3's pack.
- `ice_camera::tests::content_camera_files_read` (ignored, over Skate 2's six packages): every
  camera file of every pack reads.
- Not run in the game: the fixed build with the same `config.json` (the launch moving the link,
  and the career starting).
