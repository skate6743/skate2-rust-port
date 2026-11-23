# Dynamic hulls (career gates, pool water, skate stoppers), 2026-10-09

All addresses are the debug xex (`sk82_na_m.xex`). Everything below was read in IDA
(decompiler, and disassembly for the 64-bit attribute keys, which Hex-Rays splits).

## 1. What the native does

`Sk8::Challenge::cChallengeDynamicHullManager` (`packages/sk8/sk8challenge/dynamichull`) is a
challenge sub-controller (slot 30 of `Sk8ChallengeSystem::GameInit` `0x825aa948`'s
`RegisterSubController` calls, after the DMO manager and before the BE manager). It owns its
own `cPathedHullLoader` (constructor `0x82dab850`: base path
`data\content\missions.big|data\content\missions`, `GetHashValue32`, load flags 7, file names
clamped to 30 bytes) and an activity oct tree.

### Data (`challenge_global_data`)

| Key | Type | Use |
| --- | --- | --- |
| `0x0F2FE38E547C5960` | `tDynamicHullMapping[]` (8 bytes: `LookupName` char*, `RequiredHull.HullName` char*) | state → mission hull |
| `0x4C91113947CBBBED` | Text | default state (`AddActivity`) |
| `0xF1C6D5FA4A23951E` | `tLocationID` `Location` | box centre |
| `0x2C5FB57A89E8CDE2` | Float | box half extent (radius; 250 by default) |
| `0xB9E5F5414F748C65` | `tActivityLoadMask` | game types that load the hull (4 career, 6 career + free skate) |
| `0xE2A4965BA631D2C3` | Bool | essential (kept by `FilterNonEssentialDynamicHulls`) |
| `0x58E806F8DF17D732` | `tDynamicHullUnlock` (32 bytes: `LockEntry` RefSpec, `LookupName` char* at +0x18) | free-skate unlock |
| `0xFD40E4A31027C984` | Bool `DebugOnly` | `CreateActivity` skips the activity |

`challenges` Bool `0x198A0E276E56A9D6` adds `eAssetFlags_DMOTreatAsStatic` (0x10) to the hull's
asset flags (`PerformBBoxTests` `0x82db2f74`; `AssetPaths::eAssetFlags`: HullLoader 2,
ChallengeHull 4, DMOBank 8, DMOTreatAsStatic 0x10, DMODisallowPersistance 0x20,
DMODisallowPriorityTracking 0x40, DMOForceCull 0x80; dynamic hulls always carry 0xE0).

The shipped data has 61 leaf activities with real mappings (scratch `hull_data2.py` decode),
among them:

| Activity | States → hull | Location (radius) | Mask | Unlock |
| --- | --- | --- | --- | --- |
| `dannys_megapark_extension` (Fun Track) | Locked → `megapark_extension_gates` | `F_SVM_FunTrack` (250) | 6 | `fe_locations` `megapark` → Unlocked |
| `slappys` | Locked → `PJ03_SlappysGate` | `m_1a_01_postchallenge_prosarrivespawn_carroll` (250) | 4 | |
| `unlock_hotel_alley` | Locked → `PJ03_HotelAlley` | `ol_fa_slappys_05_slappys_gaps` (250) | 4 | |
| `slappys_endgame_unlock` | Locked → `PJ03_EndGameUnlock` | `unlock_endgame_locator01` (250) | 6 | `fe_locations` `default` → Unlocked |
| `sanvan_tunnel` | Locked → `SVM_01_TunnelLock` | `F_SVM_CougarMountain` (250) | 6 | `svpeak` → Unlocked |
| `permission_pool_gate` | Closed / NIS / Open → `m_2b_01_goto_gate_{closed,NIS,open}` | `m_2b_01_goto_gate_locator` (250) | 4 | |
| `training_facility_exterior` | Locked / Unlocked → `TrainingFacilityExteriorAbando` / `…New_10` | `F_OT_TrainingFacility` (300) | 6, essential | `trainingfacility` → Unlocked |
| `gvr_unlock` | NoGvR / AddedGvR → `OPEN` / `GVR` | `F_WF_GVR` (500) | 6, essential | `gvr` → AddedGvR |
| `dyrdek_skatepark` | UnderConstruction → `UnderConstruction` | `m_7a_02_jam_monster_public_park_signup_locator` (500) | 6 | `clubhouse` → Unlocked |
| pools (`ur_*_pool_drain`, `pj_school_pool_drain`, fountains, dam) | Undrained → water hull | teleport locators (250) | 4 | |
| skate stoppers (`*_cap`, `dt_*`, `mcz_*`, …) | Capped → caps hull | cap locators (100) | 4 | |

Every mapped hull ships in `data\content\missions.big` (Sim, Pres and an empty Tex stream)
except `Banners` (`training_facility`'s `testing` state), whose missing stream loads nothing.
A hull's `cSim_Global` holds collision (`Volume`, `ClusteredMesh`, `KdTreeMappedArray`),
`cModelData`/`InstanceData` and, for some, spline and DMO records; its `cPres_Global` holds
texture arenas and one model arena (meshes, `ROptiMeshData`, instances).

### Runtime

* `AddActivity` (`0x82db2ae8`): an activity with mappings joins `AllDynamicHullActivities`; when
  `LookupValues` has no state for it, its default state is set (`GetHashValue32`).
* `GameLoad(flags)` (`0x82da48e8`): `GameTypeFlags = flags`; only for 2 (free skate) and 4
  (career) is the game loaded: every activity with a valid location gets its box
  (`ActivityUtils::GenerateLocationActivityBBox` `0x82cd89c0`: location ± radius, radius
  `kDefaultLocationTrackingRadius` when not above 0) in the tree. Party Play (16), online (8) and
  the front end (1) load nothing (`GenerateGameModeSignature` `0x825a4228`).
* `Update(tester)` (`0x82db4dd0`): in a loaded game and with a valid tester,
  `PerformBBoxTests` (`0x82db2e38`) at the tester's base; then every `DispatchCallbacks` entry's
  `OnComplete(0x1234ABCD)`; then the loader's `Update`.
* `PerformBBoxTests` (`kEnableDynamicHullLoading` on): `GenerateTesterBBox` against the tree
  (`GetCollisions` sorts the indices); every colliding activity whose `RequiresHullLoad` gives a
  mapping loads its hull (`LoadHull(name, registerOnLoad 1, callback taken out of
  LoadCallbackMap, highPriority 0, flags 0xE0|0x10)`) and enters `LoadedDynamicHulls`; every
  activity the box left unloads its loaded hull (`UnloadHull(name)`).
* `RequiresHullLoad` (`0x82da96b0`): mask & (`OverrideGameFlags` or `GameTypeFlags`), essential
  or non-essential allowed, not loaded, a valid lookup value, a mapping of it
  (`DetermineDynamicHull` `0x82da16c0`).
* `GetLookupValue` (`0x82da6590`): `GetDynamicHullUnlockValue` (`0x825a4560`: only with
  `GameModeFlags` 2, a `LockEntry` naming a collection, and the profile having unlocked its
  layout name or `all_locations_unlock` in category 2 → `GetHashValue32(LookupName)`), else
  `LookupValues[key]`, else `kInvalidHashValue32`.
* `SetDynamicHullState(key, state, callback)` (`0x82db2ca0`): a loaded hull of another valid
  state is unloaded and leaves `LoadedDynamicHulls`; the state is set; a callback waits in
  `LoadCallbackMap` when the new state needs a hull (and no callback waits yet), else it goes to
  `DispatchCallbacks`. **Native defect:** `Update` calls every `DispatchCallbacks` entry on every
  update and never empties the vector, while `cCPDSetStateLoadCallback::OnComplete` deletes
  itself; the port calls each entry once.
* `FilterNonEssentialDynamicHulls` (`0x82da6490`) unloads the non-essential loaded hulls and
  clears `AllowNonEssentialHulls`; `AllowNonEssentialDynamicHulls` (`0x82da4760`) sets it again.
  Create-a-Spot challenges call them around their setup.
* `GameUnload` (`0x82da6470`) → `Reset` (`0x82da63d8`): tree and collisions cleared, every
  loaded hull unloaded, game flags cleared (callbacks kept). The manager is a sub-controller, and
  `cActivityManager::Update` (`0x82cdba20`) updates every sub-controller in any game (the front
  end's simulation, and `GameModule::GameUnload` `0x8273d1b0`'s `UnloadUpdate` `0x825a43e8` loop),
  so the loader works through the unloads; `Update` skips the box test and the callbacks outside a
  loaded game but always updates the loader. `GameModule::GameLoad` (`0x8273be50`) calls
  `GameUnload` again and, once the stream focus is in, loops the asset manager's and this
  manager's `Update` until `IsAllUnloadComplete` and `IsAllUnloaded` (`0x82da1680`, the loader
  tracks no hull).
* Loading: `StateLoading::TeleportLoadState` (`0x8275fdb8`) pushes `kPAUSE_STATE_TELEPORT`,
  sends the teleport once the focus is in and pulls the pause; `DoNextTransition`
  (`0x82776260`) → `PostLoadingState` (`0x82765d08`) → `DynamicHullState` (`0x82763be8`):
  `SpotTestAgainstPoint` (`0x82db4318`) at the stream focus and `kPAUSE_STATE_TELEPORT` again,
  `SpotTestUpdate` each update until `IsLoading` (`0x82da16b0`: a hull short of
  `LoadCompleteNotify`) is false, the pause pulled on exit. No simulation update (and so no box
  test) runs between the spot test and the skater's arrival. A teleport without a load screen
  (`StateLoading::DoTeleport` `0x827767e8`) skips the loading states.
* Save: `SaveData` (`0x82da97f8`) packs `LookupValues` (`PackHullStates(false)`); `LoadData`
  (`0x82db4330`) unpacks into the map without clearing it.

### What changes the states

| Driver | Native | Port |
| --- | --- | --- |
| Progression scripts | `DynamicHull.SetupDynamicHullState` (`0x82db41f8`, no callback): slappys, sanvan_tunnel, permission_pool_gate, gvr_unlock, training_facility_exterior, slappys_endgame_unlock, mikey_uncap, dt_madridplaza_0, the dam | `dynamic_hull::set_state` |
| Presentation event 33 | `PlayNextPresentationEvent` (`0x82e08668`): `StringToKey(arg1)`, state arg2, a `cCPDSetStateLoadCallback`; the group waits for its `OnComplete` (`0x82e08a30`) | `presentation.rs` `SET_DYNAMIC_HULL_STATE` |
| Property purchase (Fun Track) | `FrontEndState_ChallengeMap::ConfirmPurchaseCallback` (`0x826751f8`): `PropertyCost` subtracted, `PlayerOwns` 1, state `Unlocked`, `CompleteChallenge(key, 1)` | `dynamic_hull::purchase_property`, from the career map's purchase screen (below) |
| Services | `UncapRailsViaProgression` (`0x8289c730`), `DrainPoolViaProgression` (`0x8289cc90`), `SetCappingOnJacked` (`0x8289ca48`, from `ChaseMan::JackSkater`) | the first two as Lua natives; `dynamic_hull::set_capping_on_jacked`, not called (§4) |
| Free skate | `GetDynamicHullUnlockValue` reads the profile's location unlocks | ported |

Readers of `TestForDynamicHullState` (`0x82da9670`): the minimap manager's
`ReevaluateActivityPrereqs` (`0x825a5bd0`) adds the map's four location groups (general, plazas,
skateparks, spawn points) only once `slappys` is `Unlock`, and `FEStuffMenu::PopulateMenus`
(`0x8263e2c0`) lists the locations the same way (both already in `map_icons.rs` /
`stuff_menu.rs`); `GetExtraEntryFee` (`0x825d03e0`: +100 per `Capped` uncap hull, 0x8276cb50);
the services (`GetNumDrainedPoolsInWorld` `0x8289c958` for the drain stats).

### The career map's property purchase

* The Stuff menu's Property category (`ID_STUFFMENU_CATEGORY_PROPERTY`,
  `PopulateMenuItemsVector_Property` `0x826328a8`) lists the property list's (progression list
  74) available or completed challenges that have a `PropertyCost`. Two are sold:
  `dannys_megapark_extension` ($50,000, made available by Circulate Stage One's
  `ActivateChallengeByTitle`) and `dyrdek_skatepark` (progression title `Dyrdek's Skatepark`,
  $200,000).
* A on an item (the menu picker sounds `core_a_button` before `OnMenuSelect`):
  `TriggerInfoComponent_PurchaseProperty` (`0x8265f0f8`) keeps the challenge, enters the info
  component with substate `PurchaseProperty` and sets screen states 72 (the letterbox) and 22,
  `challenge_info.SetMode(4)` (`challenge_info.apt` `0x482e`, the purchase branch at `0x4fb4`).
  `SetMode` turns every element off; mode 4 then shows the header (`GetChallengeTitle`), the
  description (`GetChallengeDescription` `0x8264d2f0`), the image
  (`LoadRX2Texture(GetChallengeLocationImage)`, `0x82623b50`:
  `fe\source\images\locations\<FELargeImage>_sm`), the price (`GetPropertyPriceOrPaid`
  `0x8264db58`: `FormatCurrency(PropertyCost)` (`0x82981318`) with the literal prefix, or
  `ID_CIC_PURCHASED` once owned) and the picker's one option: `ID_COMMON_TELEPORT` when owned,
  else `ID_CIC_PURCHASE_PROPERTY` with the wallet (`mCash`: `Cash_GetFormattedCashString`
  `0x826055a8`, the same `FormatCurrency`, of `Cash_GetPlayerCashTotal` `0x8260b008`; the value
  right of the auto-sized `totalbetHALText`). `UpdatePositions` (`0x3ec4`) stacks them from the
  header down: header `_y + _height + 10`; image, `+ _height + 10`; description at `+ 5`, then
  `+ floor(16 * maxscroll) + 15`; price, `+ floor(16 * maxscroll) + 5`; picker,
  `+ GetHeight() + 10`; wallet. A header that wraps (`maxscroll > 1`) goes to its `smalltext`
  frame, and to `smalltextcenter` when it then fits a line.
* A on the option (`core_a_button`), `OnMenuSelectChallengeComponentProperty` (`0x82679600`, item
  0): an owned property (`IsPropertyOwned` `0x8264daa0`: `PlayerOwns` above 0) teleports there
  (the Big Black dismissal popup first while that service runs, else the input block, the fade
  and `FadeComplete`, `DoTeleport`); otherwise `GetCash < PropertyCost` opens
  `Property_PurchaseFailed` (`gPopupTemplate` row 215: `ID_COMMON_WARNING_TITLE`,
  `ID_PROPERTY_PURCHASE_FAILED`, OK, no callback), else `Property_PurchaseConfirm` (216:
  `ID_CIC_PURCHASE_PROPERTY`, `ID_PAUSEMENU_CONFIRM`, No / Yes, `ConfirmPurchaseCallback`); both
  style 0, not cancellable, the first option selected. The popup (FE state 34, sk8popup) sounds
  `core_popup` on entering (`FrontEndState_Popup::OnEnter` `0x82658474`), has no dimmer, its
  picker wraps (`core_nav_up` / `core_nav_down`) and A sounds `core_a_button`.
* `ConfirmPurchaseCallback` (`0x826751f8`) with Yes: `SubtractCash(PropertyCost)`,
  `SetChallengeSaveData(PlayerOwns, 1)`, `SetDynamicHullState(activity, "Unlocked")`,
  `CompleteChallenge(id, 1)`, then (no Big Black service) the input block, the fade and
  `DoTeleport` (`TeleportToChallenge` to the property's `Location`). The completion moves the
  property's progression state machine (`Trigger_Danny_Way_Purchase`: Initial, Activation, FMV,
  Results, Final; `Trigger_Dyrdek_Park_Purchase` has an NIS in place of the FMV): Danny's plays
  `FMV - Unlock Danny Mega Park`, then the unlock overlay, and the autosave follows the overlay.

### `SetCappingOnJacked`

`ChaseMan::JackSkater` (`0x8270fd08`), called by the jacking guard's `JackSkaterIfAppropriate`
behaviour (`0x8270bf50`) with itself as the chaser, calls `cServiceManager::SetCappingOnJacked`
(`0x8289ca48`) for a local skater jacked in single player with no main challenge, when the
chaser's `Pedestrian::mChallengeOwner` is set. Only `cChallengeCharacterManager::TestBBoxAgainstTree`
(`0x825ab590`) sets that owner, on the characters a challenge spawns. The function walks the
owner's local-data cap list (`challenge_local_data` `0xE1A63BC6CB47832C`) and sets `Capped` on
the listed challenge whose global data `Location` is a valid location nearest the skater
(squared distance, the first of equals). In the shipped data six challenges have a cap list
(`downtown_artdeco`, `downtown_tiered`, `dt_2`, `dt_9_theatre`, `dt_madridplaza`,
`ot_01_cityhall`) and none of them spawns characters, while none of the 141 challenges that spawn
characters has one: no jack in the shipped game caps anything.

## 2. What the port does now

* `crates/skate2-core/src/challenge/dynamic_hull.rs`: the whole manager (init/`AddActivity`,
  `GameLoad`, `GameUnload`/`Reset`, `Update`, `PerformBBoxTests`, `RequiresHullLoad`,
  `DetermineDynamicHull`, `GetLookupValue` with `GetDynamicHullUnlockValue`,
  `SetDynamicHullState` with `LoadCallbackMap`/`DispatchCallbacks`,
  `FilterNonEssentialDynamicHulls`, `SpotTestAgainstPoint`, the loader callbacks), the Lua
  natives (including `FilterNonEssentialDynamicHulls`, `AllowNonEssentialDynamicHulls`,
  `Service.UncapRailsViaProgression`), `purchase_property` and `set_capping_on_jacked` (with
  `CAP_LIST`; nothing calls it, §4). Outputs: `Output::LoadDynamicHull` / `UnloadDynamicHull`.
* `challenge/system.rs`: `init`, `update` (before the effects manager), `game_load` via
  `activity::game_load`, and session entry points (`game_unload_dynamic_hulls`,
  `spot_test_dynamic_hulls`, `dynamic_hull_load_complete`, `diagnostic_add_cash`,
  `diagnostic_activate_challenge` (`Progression.ActivateChallengeByTitle`, run in the
  progression state), free skate's `game_load_free_skate_dynamic_hulls` /
  `update_dynamic_hulls`).
* `challenge/stuff_menu.rs` (`ChallengeMap`): a Property item opens the info component in its
  purchase substate (`Selection::Property`, `Info::Property`); its option is the purchase
  (`select_property`: the owned teleport, `MapPopup::PurchaseFailed` or `PurchaseConfirm`); a
  popup takes the input (`update_popup`: wrap, sounds, A closes it, Yes runs `confirm_purchase`,
  `dynamic_hull::purchase_property`, then the map's teleport to the property).
  `ChallengeMapView` carries the purchase texts (`InfoView::property`: description, image, the
  price or none once owned, the wallet while for sale) and the popup. The map keeps the
  `FE_SOUND_*` its pickers and states play, which `CareerSession::update_challenge_map` hands to
  the career's front-end sounds (the career map played none before).
* `presentation.rs`: event 33 and `set_state_load_complete`.
* `signpost.rs`: `complete_challenge_by_key` (`CompleteChallenge(key, true)` `0x825e1550`).
* `signpost_progress.rs`: `GetExtraEntryFee`'s uncap part.
* `session/streaming/selected_hull_loader.rs`: optional load and unload callbacks,
  `IsLoading`, `IsAllUnloaded`, and the presentation collections handed to the presentation
  (`presentations`; registered at once rather than after the presentation thread's
  `cHullCollectionRegistrationCallback`). Its update skipped loaded manifests, so a hull's
  `UnloadStream` never reached `cGameStream::Update` and the first unload held the loader's
  one unload slot for good (every later unload, the purchased gate's included, waited behind
  it); a hull's manifests are now updated while it unloads.
* `session/selected_session_terrain.rs` (+ trait): the dynamic hull manager's own loader.
* `session/selected_session_career.rs`: requests and completions after each career update,
  `StateLoading::DynamicHullState` for travels (`try_dynamic_hull_state`), the game unload, and
  free skate's manager (a profile free skate's whole challenge system since 2026-11-21,
  `free_skate_challenges`; an online one's `try_update_free_skate_dynamic_hulls`). The
  port's travel lands a few simulation updates after its spot test, so the manager's update is
  held (`DynamicHulls::teleport_hold`) while a travel is in flight, as the native's paused
  simulation is; without it the box test at the skater's old place unloaded what the spot test
  had loaded, and back (seen live: `PJ01_SchoolPool` / `UR05_Community_Pool` loading in turn).
* `session/selected_session_public_facade.rs`: the map travel / front-end game start waits for
  the dynamic hulls under the fade, then for the presentation again; a game start after a game
  unload first waits for the loader to be empty (`GameModule::GameLoad`'s loop); the main menu's
  frames update the loader with the front end's simulation; `hull_presentations`.
* `session/selected_session_freeskate.rs`: profile free skate builds the free-skate challenge
  system for the dynamic hulls; the quit unloads them.
* Diagnostics (`live_skating_client.rs`, any front-end demo): `SKATE2_DIAGNOSTIC_TRAVEL=x,y,z,yaw`
  (`;`-separated, with `SKATE2_DIAGNOSTIC_TRAVEL_TICK=t;t`) runs the map's travel there;
  `SKATE2_DIAGNOSTIC_CASH=<n>` at `SKATE2_DIAGNOSTIC_CASH_TICK` adds cash (`AddCash(n, 0)`);
  `SKATE2_DIAGNOSTIC_ACTIVATE_CHALLENGE=<title>` at `SKATE2_DIAGNOSTIC_ACTIVATE_TICK` runs the
  progression's `ActivateChallengeByTitle`; in the career demo, `SKATE2_DIAGNOSTIC_BUY_PROPERTY=t;t`
  buys from each tick with pad input only, one press per 40 ticks: Start, down to the hub's Map,
  A, down to Property, A, A on its first item, A on the option, then on the popup OK (and B out of
  the info component, the category and the map, then Start) or down to Yes and A. The earlier
  `SKATE2_DIAGNOSTIC_PURCHASE_*` (a direct `purchase_property`) is gone. `SKATE2_TRACE_HULLS`
  prints each hull presentation registered.
* `rendering/presentation.rs`: `HullPresentation`, `PresentationScene::hulls`.
* PC: `hull_presentation.rs` (reads a hull's `cPres_Global` from its own stream),
  `city_streaming.rs` (a hull registration change rebuilds the draw list),
  `render_inspection.rs`, `live_skating_view.rs`, `live_skating_client.rs`.
* PC purchase screen: `marker_ui/map_movie.rs` (`InfoPanel` / `PropertyPanel`;
  `configure_property` is `SetMode(4)` with `UpdatePositions`, `set_header` its `smalltext` /
  `smalltextcenter`; the popup is sk8popup, asset 25, through `configure_popup`, with its intro,
  outro and their callbacks, each new popup state starting its own intro); `marker_ui.rs`
  (`property_panel` loads the location image, none when the archive lacks it, and formats the
  price and the wallet with `localizer::format_currency`; the popup draws over the map; the
  dimmer is off while it is open, as FE state 34's `mbDimmer` is).

## 3. Verification

Builds: `cargo build --release -p skate2-pc`.

Core tests on the shipped data (`cargo test --release -p skate2-core --lib -- --ignored
dynamic_hull`, `challenge::tests::dynamic_hulls`): a new career at `J_Gamestart` loads exactly
`PJ03_EndGameUnlock`, `PJ03_HotelAlley`, `PJ03_SlappysGate`, `UR04_Hotel_Pool`,
`UR05_Community_Pool` (flags `0xE0`); the progression script's
`DynamicHull.SetupDynamicHullState('slappys', 'Unlock')`, run in the progression state, unloads
Slappy's gate and leaving the box unloads the rest; at `F_SVM_FunTrack` the gates load; the
Purchase is refused with the start cash, then bought ($50,000 → $0), unloads the gates and the
saved state keeps them away in the next game; free skate at `J_Gamestart` loads only
`PJ03_EndGameUnlock`, the `megapark` location unlock removes the Fun Track gates, Party Play
loads nothing; presentation event 33 waits for `m_2b_01_goto_gate_NIS` (loaded with the
callback) and then plays on. All five pass. Slappy's own unlock comes from a state machine
after chapter 1a (with the Hall of Meat tutorial's activation), which the live runs below do
not reach. The 49 ignored real-data `challenge::` tests pass; the crate's other tests: 5511
pass, and the 10 that fail are in other areas (grinding kernel, input cvars, skitch, vehicle
queries, DMO pool, drive Jacobian, the normal arena adapter's AiPath registry order, head
tracking).

Live, on scratch saves (`career_run.sh` / `run_perf.sh`, `SKATE2_TRACE_HULLS`, captures every
1-3 s):

* New career (`gate_new`): under the start's fade the five hulls above load and register one
  render instance each (Slappy's gate `[1595.2, 100.2, -511.1]..[1597.5, 103.5, -508.4]`).
  Travel to 15 m south-east of Slappy's gateway: the closed metal gate stands in it. Travel to the
  Fun Track: `megapark_extension_gates` loads (`[1182.5, 191.4, -1201.5]..[1214.5, 211.1,
  -1138.8]`, three gates), the other five unload, and the south gate stands closed
  in front of the skater.
* Purchase there (`SKATE2_DIAGNOSTIC_PURCHASE_CASH=50000`): `(cash, cost, bought) = (50300, 50000,
  true)`, the gates' unload and `DynamicHullState { dannys_megapark_extension, Unlocked }`, the
  FMV, the unlock overlay (the gate still behind it while the overlay holds the simulation), A,
  `career: autosaved (cash 300, orientation true at [1209.9, 207.8, -1202.9])`. With the
  simulation back the gate is gone and the skater's next push rides through the opening and
  down into the Fun Track.
* Continue Career on a copy of that save (`gate_cont`): `SignPost loaded (1 completed, 44
  available)`, `continue at [1209.9, 207.8, -1202.9] (saved orientation true)`; the gates never
  load again, also after the travels away and back to the Fun Track, where the opening stands
  empty; Slappy's gate is still closed (its state is untouched).
* Profile free skate on a fresh profile (`gate_free`, start at Slappy's): only
  `PJ03_EndGameUnlock` loads there; at the Fun Track the gates load (the profile has not
  unlocked `megapark`), and ten seconds of pushes from 9 m stop the board against the south
  gate (`skater at (1211.37, -1199.75)`, 0.9 m short of its plane): the hull collides. At
  Slappy's the gateway is open, as the data's career-only mask has it.
* Party Play (`gate_party`: Spot Battle at Danny's Megaramp, spawned at `[1144.4, 206.9,
  -1158.6]`, inside the gates' box, `SKATE2_DIAGNOSTIC_PARTY_PLAY_DISTINCT` so the two later
  players load different skaters): no dynamic hull is asked for, and after the travel to the Fun
  Track the south gate's opening stands empty, where free skate on a fresh profile has it
  closed. (Without `_DISTINCT` the party start stops on `party skater recipe:
  RecipeCacheHitPending`, as other Party Play runs today did.)
* Free skate, the pause menu's Quit and a second free skate (`gate_fq`,
  `SKATE2_DIAGNOSTIC_FREESKATE_QUIT`): the quit's `GameUnload` asks for the Fun Track gates'
  unload, and the second game's start (Downtown, with no hull near) lands once the loader has
  let them go (`Free Skate again: skater at (155.6, 429.7)`).
* Before the travel hold, the travel to Slappy's loaded `PJ01_SchoolPool` and unloaded
  `UR05_Community_Pool` (the spot test), then the reverse (the box test at the old place), then
  again; with it each travel loads and unloads once.

Parity (`parity.sh gate_parity`, the no-profile boot spawn at `F_UR_PermissionPool`, which runs
no challenge system): PARITY IDENTICAL (6000 lines).

Through the menus only (2026-10-10, `buy2`: new career, `SKATE2_DIAGNOSTIC_ACTIVATE_CHALLENGE=
dannys_megapark_extension` at 3300 for Circulate Stage One's activation, `SKATE2_DIAGNOSTIC_BUY_PROPERTY
=3400;4800`, `SKATE2_DIAGNOSTIC_CASH=50000` at 4600): before the activation the tutorial's Stuff
menu has no rows at all; after it, Property (1) lists Fun Track; its page shows the photo, the
description, $50,000, Purchase Property and Total Cash $300; A raises `PurchaseFailed`, OK closes
it; with $50,300 the confirmation opens on No, Down, Yes: `DynamicHullState
{ dannys_megapark_extension, Unlocked }`, the start's hulls unload for the travel, the
`Unlock_DannyMegaPark` FMV, the Rewards overlay ($300) at the Fun Track, the autosave. The save's
orientation was the start alley's crumb: the skater stood off-board after the travel, and
`GetSuggestedRespawnPosition` falls back to the newest crumb when the wipeout spot fails its
constraints (the crumbs are never cleared by a teleport; new ones wait for
`kMinTicksAfterResetForBreadCrumb`), which the native shares. Final parity on the combined
build: PARITY IDENTICAL (6000 lines).

## 4. Open

* The purchase screen's Big Black branches (`eSk8FEPopup_BB_DismissalConfirm` with
  `DismissAndTeleportCallback` before an owned property's teleport or after a purchase) are not
  taken: the port's living world runs no Big Black service, so the native's other branch (the
  input block, the fade, `DoTeleport`) always runs.
* The services' stats (24-28) and achievements (`Pull_The_Plug`, `New_San_Van_Hero`) belong to
  the unported stats back end.
* `SetCappingOnJacked` is ported but not called: the owner cannot be resolved the native way
  yet. Missing: (1) the jacking guard: the native passes the guard whose
  `JackSkaterIfAppropriate` behaviour ran; the port has no such behaviour, and
  `living_world_chase_frame.rs` infers the jack from the chasee's `group_end_reason == 1` and
  takes the first guard among the chasers, keeping no id; (2) `Pedestrian::mChallengeOwner`: the
  port's pedestrians carry no owner; the challenge character manager's spawn
  (`LivingWorldOp::Spawn`, answered with the entity) knows the activity and could give it;
  (3) the chase frame's jack always takes the no-main-challenge branch (`0x40`,
  `SetIsJacked(true)`), where the native clears both during a main challenge. With the shipped
  data the call would cap nothing (§1), so the wiring changes no shipped behaviour (decision:
  whether the living world owner adds (1)-(3)).
* The replay editor's restore buffer (`GenerateDynamicHullRestoreBuffer`,
  `StateReplay::DynamicHullState`) belongs to the unported replay editor.
* `DispatchCallbacks` is called once per entry (native defect above).
* No-profile free skate (the port's boot spawn the diagnostics and parity use) runs no
  challenge system, so no hulls; the native has no such mode. Profile free skate does.
* A hull's presentation collections register when the loader starts them; the native registers
  them through the presentation thread's callback a little later.
