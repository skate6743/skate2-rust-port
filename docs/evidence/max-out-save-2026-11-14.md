# Extras' Max Out Save — November 14, 2026

PC port addition. Extras' last row, after the original's rows (and the advanced replay tutorial
when the film pack is there), reads Max Out Save. A on it opens the progression popup titled Max
Out Save:

> Are you sure you want to max out this savegame? This action unlocks everything in storyline and
> can't be undone unless you have a backup save.

Its options are Yes and No, with No highlighted. Up and Down move between them, A answers and B
answers No. No leaves the save as it is. Yes maxes the save out and writes SKATER.P; the popup then
reads Save maxed out! (OK closes it), or Could not save with the store's error.

Code: `ui/extras.rs` (the row and the popup's pages), `session/selected_session_extras.rs`
(`max_out_save`), `challenge/max_out.rs` (the career), `session/edit_skater_shop.rs`
(`clothing_unlocks`, `own_everything`), and `marker_ui/extras_movie.rs` (the popup).

## What the save gets

The profile's career is read as the career's start reads it: a new career's challenge system, with
the profile's UNLOCKABLES, Sk8ChallengeSystem (dynamic hulls) and SignPost savables loaded over it.
Then, in this order:

1. **Every progression challenge completed** (358 in Skate 2's data, the 717 collections less the
   359 flagged not progression; 419 with six Skate 2 DLC packs linked), each as `ChallengeComplete`
   (`0x825e00f8`) completes one with progression enabled: its completion BLOG entry, its awards,
   its place in the completed map, its photo articles and its groups' exits. The state machines the
   completions would queue are not run; the unlocks their handlers make come from step 5.
2. **Each completed challenge's save values** as the best play leaves them: `Highest Rank` 1, and
   `Killed It`, `Spot Found`, `Unlocked`, `PlayerOwns`, `PropertyOwned` and `Paid Entry Fee` set.
   The other values (best times, points, a declined spot) stay as they are.
3. **Point pools**: `Cash` at 9,000,000 (within the pool's bounds, layout words 2 and 3), and
   `Properties Purchased`, `Pools Drained`, `Pools Found`, `Spots Owned`, `Pros Completed`,
   `Sponsorships Earned`, `AnywhereFilms` and `Circulate Pool` at their maximum (word 3).
4. **Rankings**: each ranking group at the ranking of its highest level (the rankings of the
   group's id, word 2, by level, word 3).
5. **Unlocks**, none of them left new:
   - the literal first argument of each unlock native in the progression handlers' sources (910,
     read up to each line's `--` comment): 29 free-skate characters
     (`UnlockCharacterForFreeskate`, through `characters_marquee` as the native maps the name), 19
     locations, 3 gestures, 28 clothing unlocks (`UnlockUnlockableByName`) and 78 sponsor unlocks
     (`UnlockSponsorByName`);
   - Big Black (`big_black`, the Extras code's character; its handler call is commented out),
     `all_characters_unlock` and `all_locations_unlock`;
   - every `Unlock` (category 15) and `SponsorUnlock` (category 16) name of the parts Edit Skater
     offers (no DLC id, `DLCFilter`), with a linked Skate 3 data folder's clothing: 25 and 84 in
     Skate 2's catalog, six of those sponsor names in no handler. No part waits for the four pro
     clothing unlocks whose handler calls are commented out (Gallant, Hsu, McKay, Smith).

   The unlock list holds 500 items; Max Out Save adds at most 166.
6. **Dynamic hulls**, each activity in the state the whole career leaves it in: a `Locked` or
   `UnderConstruction` one `Unlocked` (the state a bought property takes: the MegaPark extension and
   Dyrdek's park), Slappy's `Unlock`, a `Closed` one `Open`, the GvR `AddedGvR`, the pools the player
   can have drained (below `service_draining`, `IsServiceableByUser`) and the dam
   (`ol_fa_svm_21_accumulation_megadam_locator`) `Drained`, and the skate stoppers the player can
   have removed (below `service_uncapping`) and the two scripts remove (`mikey_uncap`,
   `dt_madridplaza_0`) `Uncapped`. Any other activity keeps its state.
7. **Clothing owned**: with a created skater, every part the shop sells and the skater does not own
   joins the CAC record's inventory, as a purchase adds one (`AddPartToInventory`: component types
   from 4): offered, with a cost, not used in game. The skater's gender's parts and the ungendered
   ones go first, then the other gender's (Edit Skater can switch it), while the inventory stays
   below the record's 6,000. Skate 2's catalog sells 2,686 such parts (1,305 male, 1,062 female,
   319 unisex); with Skate 3's clothing and six of its DLC packs linked, a male skater's inventory
   went from 14 parts to 5,704. The log line counts the parts that did not fit.

UNLOCKABLES, Sk8ChallengeSystem, SignPost and the CAC record are written in one SKATER.P
replacement, each read back before it commits (`save_memcard_and_record`); the profile data, the
settings and the music player are saved as the Extras codes save them, and the profile's other
savables are kept.

## What stays as it is

- The story's message settings and the position the career continues from (SignPost's MESSAGES and
  POSITION leaves are written back as loaded).
- The new-profile flag. A new profile still starts the new career's way: Create-a-Skater, then the
  career's first spawn. Edit Skater starts a new career from a fresh skater (`load_record` is not
  used for a new career), so clothing owned on a profile without a career yet is not in that
  skater's inventory; Max Out Save after the career's first save owns it.
- Statistics and Hall of Meat: the awards' statistic updates are not applied.

## Verified

Release build, scripted runs on scratch saves (`SKATE2_DIAGNOSTIC_MAIN_MENU_SCRIPT`,
`SKATE2_TRACE_MAX_OUT`, `SKATE2_TRACE_CAREER`), with six Skate 2 DLC packs, Skate 3's data and six
of its DLC packs linked:

- The row is Extras' eighth (with the film pack) and Up from the first row reaches it. A opens the
  popup on No; Up moves to Yes on screen; A on No closes it and SKATER.P stays byte for byte as it
  was; Yes shows the result popup.
- Yes on a profile with a created skater: `cash 9000000, 419 challenges completed, 212 of 500
  unlock slots, 5690 parts joined the inventory`. Continue Career then loads `419 completed, 0
  available, cash 9000000, 212 unlocks` and plays. The hull trace lists the 68 activities: Dyrdek's
  park and the MegaPark extension `Unlocked`, five gates open (Slappy's, the San Van tunnel, the
  hotel alley, the training facility's exterior, the pool gate) and the GvR added, ten pools and
  fountains and the dam `Drained`, forty skate stoppers `Uncapped`; the rest have one state.
- Edit Skater's T-Shirts list shows every shirt owned. Free Skate's Character Selection holds Dem
  Bones (unlocked by name only) after Max Out Save and not before.
- A new profile: `0 parts joined` (no skater yet); New Career then loads the maxed career and
  starts at `J_Gamestart`.
- Max Out Save on a maxed save changes nothing: the same counts, no parts joined, every hull
  already in its state.
