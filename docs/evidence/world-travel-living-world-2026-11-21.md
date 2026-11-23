# A travel between worlds and the living world — November 21, 2026

## What happened

A travel from one linked Skate 3 map to another (Art Gallery to Danny Way's Hawaiian Dream) showed
objects in the air of the second map, which then fell onto it.

Traced with `SKATE2_TRACE_LIVING_WORLD_COUNTS` on a career travel to Art Gallery and from there to
Danny Way's Hawaiian Dream:

| Moment | Objects out | Locators |
| --- | --- | --- |
| In San Vanelona | 4 | 112 |
| In Art Gallery | 10 | 45 |
| Art Gallery unloaded, Hawaiian Dream loading | 10 | 18 |
| Three seconds later | 2 | 18 |

Art Gallery's ten objects were still out after its ground had unloaded and the next map's
locators were in.

## Why

- A streamed locator chunk's removal (`LWDynamicObjectLocatorMan::RemoveStreamedLocatorData`,
  0x82d3d618 in Skate 2 and 0x82c4f0a8 in Skate 3's title update 3) culls the chunk's objects
  only when its asset flags have bit 7 or the caller forces it
  (`RemoveLocatorsForStreamedLocatorChunk`: `RequestExplicitCullOfUserObject` under that flag);
  otherwise an object stays out and its locator with it, for the census to cull by distance.
- A stream collection's unload calls it without the force, so a world's objects outlive its
  ground until the census reaches them.
- San Vanelona's objects are far from an additional world's, so the census culls them at once. A
  linked Skate 3 data's maps all lie round the origin: the objects of the map that left are inside
  the census's range of the skater in the next one.

## What the port does

- Once a travel has unloaded an additional world, and once it has loaded one that stands in San
  Vanelona's place (a linked Skate 3 map), the living world resets as a game's unload resets it
  (`LivingWorld::Manager::Reset` 0x8271d3d8: every pedestrian, vehicle, object and prop culled
  through its own manager), under the travel's fade
  (`SelectedLiveSession::try_reset_living_world_for_travel`). The census then spawns the new
  world's own.
- In an online game the reset keeps the network game's living world, which spawns nothing; the
  game's static objects come out again as after any reset (`place_online_dmos`).

Checked with the same scripted travel on the build with the reset: the log has
`travel: living world reset` once Art Gallery has loaded, once it has unloaded and once the
Hawaiian Dream has loaded, and the first count after Art Gallery's unload is 1 object out (the
Hawaiian Dream's own) where it was 10.

## Free Skate Here

- The online hub's Free Skate Here (`GoToFreeSkateHere` 0x8267bee8) looked for the nearest free
  skate among `BAM_Sim.loc`'s locations alone. In an additional world none of those is the
  player's: the search answered with San Vanelona's nearest, measured across worlds.
- It now takes the free skates of the world the player is in: the main world's in the main world,
  and in an additional world (a downloadable content's, a linked Skate 3 map's) that world's own,
  each at its own location (`online_key_location`), nearest first, then by boundary as before.
- Checked with `SKATE2_DIAGNOSTIC_FREE_SKATE_HERE` (the choice alone, no search) on the same
  run: `freeskate_lighthousepark` in San Vanelona, `freeskate_dlc_artgallery` in Art Gallery and
  `freeskate_dlc_dwmc` in Danny Way's Hawaiian Dream.
- Checked over a session with a server on the same machine (the build's `--server`), one player:
  from the career in Art Gallery, Back opens the hub, Sign In signs in, then Free Skate and Free
  Skate Here. The client logs `public free skate here` with `freeskate_dlc_artgallery`'s key,
  the server `game 1 created: type 8` with that key, and the client `online area
  freeskate_dlc_artgallery at freeskate_artgallery_locator`; the screen shows the skater still in
  Art Gallery under the lobby's Freeskate card. From Danny Way's Hawaiian Dream the same steps
  give `freeskate_dlc_dwmc at freeskate_dannyway_megacompound_locator`. The 0.3.5 build run the
  same way from Art Gallery asks for `freeskate_soho_west` and unloads Art Gallery.
- With two players on that server, each in the career in Art Gallery: the second player's Free
  Skate Here joins the first's game (`player_b joined game 1 in slot 1`), and its screen shows
  both skaters in Art Gallery under a Freeskate card listing both names. A server on another
  machine was not used.
