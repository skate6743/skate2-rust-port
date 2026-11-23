# Free Skate's property teleporters — November 21, 2026

## What was missing

In a profile free skate the stadium's and the training facility's doors showed no prompt, so
neither could be entered or left on foot or on the board.

## The original

- A free-skate game runs the challenge system as every game does:
  `Sk8ChallengeSystem::GameLoad` (0x825a6618) with `GenerateGameModeSignature` (0x825a4228) 2,
  and progression off (`StartGameModule` 0x82633da0 enables it for `GM_SinglePlayer` alone).
- `cActivityManager::GameLoad(2)` (0x82cd92c0) loads its sub-controllers and
  `ReevaluatePrereqActivities` (0x82cdbaa8) runs the global activities that pass
  `EvaluateActivitiesForPrereqs` (0x82cd9e88) for the game type: those with the free-skate
  game-type flag, which the properties' teleporters carry (`ChallengeType` 45, `Global` 2:
  `c_3b_01_teleport_stadium_in`, `c_3b_01_teleport_stadium_ramps`,
  `c_3b_02_teleport_stadium_out`, `m_08_01_teleport_trainingfacility_in`,
  `m_08_01_teleport_trainingfacility_out`, and downloaded content's own).
- Each of them runs while `Sk8ChallengeSystem::IsChallengeActive` (0x825a2c78) holds. In a
  free-skate game that is either of:
  - the progression holding the challenge available (`cSignPostManager::IsChallengeActive`): the
    game carries the profile's progression although it runs none of it;
  - the unlockables holding the challenge by its name (`UnlockablesBackEndCore::IsUnlocked` of
    category 3, `CHALLENGES`, downloaded content included). No code unlocks that category; an
    unlock list does (`LoadUnlocksFromFiles` 0x82573100, a downloaded content's
    `data\unlocks\*.unlock`).
- A teleporter's graph: `setup_teleporter` makes its location indicator and its HUD group
  (`HUD2.SetupTeleportHUD`), `wait_for_teleport_volume` shows the D-pad up prompt while the
  local skater is in its trigger volume (`HUD2.ShowTeleportHUD`), D-pad up opens a Yes/No popup
  (`FrontEnd.ShowYesNoPopup`, or the exits' list where there are several), and Yes teleports the
  local skater to its exit (`Teleport.ToWorldFileLocation`).

## The port

- A profile free skate now holds that challenge system
  (`SelectedLiveSession::free_skate_challenges`, `ChallengeSystem::game_load_free_skate`): the
  global locations, the profile's unlockables, dynamic hull states and progression, then
  `GameLoad(2)`. It replaces the dynamic hull manager the free skate ran on its own; an online
  free skate keeps that one.
- It updates in the career's place for the update alone (`career_step`), so everything the game
  asks of "a career" still answers no in a free skate: the map, the saves, the HUD's settings,
  the replay and the objects' prompt are the free skate's own. Its scenes' frame is left out,
  and an autosave a script asks for saves nothing.
- `is_neutrino_challenge_active` takes the unlockables' `CHALLENGES` category in a free-skate
  game, which it left out.
- The presentation takes the teleporter's prompt and popup apart from the career's HUD
  (`take_free_skate_hud`, `FreeSkateHud`): the action overlay shows each sign-up state in order
  as the career's does, an object's prompt waiting while a teleporter's is up, and the popup is
  the progression popup. The popup takes the front end's input and the skater's, as in the
  career; D-pad up reaches the teleporter while its prompt shows.
- The game's unload ends it as it ends any game's challenge system.
- Left as they were: the progression's message settings are not read in a free skate (Hall of
  Meat stays available, and the training facility is the unsponsored one its map entry loads),
  and the progression is never saved from one.

## Verified

Scripted runs of main-menu Free Skate on the build with the change:

- A save maxed out by 0.3.5, six downloadable content packs linked: `free skate: SignPost loaded
  (413 completed, 6 available)`, seven teleporters at `wait_for_teleport_volume`. At the
  stadium's door the screen shows the D-pad up prompt `Enter Stadium`; D-pad up opens `Enter
  Stadium` / `Are you sure?` with No and Yes; Yes loads the stadium's inside behind the load
  screen. At the training facility's door in Old Town the same popup's Yes loads the training
  facility (`travel: additional world DIST_TrainingFacility_Online loading`), the skater
  standing inside it.
- A mid-career save that has not opened either, the same packs linked: the same seven run. One
  of the linked packs carries an unlock list naming the five teleporters above and
  `teleport_dyrdek_apartment` under `CHALLENGES`.
- No pack linked, the mid-career save: no teleporter runs. No pack linked, the maxed save: the
  five above run.
- The career on the maxed save is as before (`413 completed, 6 available`, its six teleporters
  and `service_security`).
- Inside the stadium, at `c_2b_01_teleport_stadium_in_locator02`: D-pad up opens `Exit Stadium` /
  `Are you sure?`, and Yes puts the skater outside at `c_2b_02_teleport_stadium_out_locator01`.
- Inside the stadium, at `c_2b_01_teleport_megaramp_locator`: D-pad up opens `Ramp Elevators` /
  `Pick A Ramp` with Tech Ramp, Side Ramp, Hip Ramp and Mega Ramp; the third puts the skater at
  `HipRampStart_global`, on the hip ramp's deck.
- Into the training facility and out again: inside it, at
  `m_08_01_teleport_trainingfacility_out_locator02`, D-pad up opens its exit's popup and Yes
  puts the skater back at its door in Old Town (`travel: additional world
  DIST_TrainingFacility_Online unloaded`).
- A free skate started, quit to the main menu through the pause menu (`free skate: profile
  saved`) and started again, AI skaters on, runs its teleporters again.

## Not verified

- A quit to the main menu from inside a property and the Replay Editor after a teleport were
  not run.
