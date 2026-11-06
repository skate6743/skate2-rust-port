//! Natives of the challenge Lua state (`Lua::Manager::PerformLuaCBinding`
//! `0x8289b498`) that the port reconstructs. Each names its native address.

use super::{
    activity, assets,
    attrib_lua::{ObjectKind, Pointee, activity_of, attrib_location},
    binding::{get_bool, get_bool_or, get_float, get_int, get_int_or, get_string, get_string_or, int, int_u32},
    effects,
    hud::{self, CentralMessage},
    keys::hash_value32,
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    presentation::{self, PresentationEvent},
    signpost,
    system::{Event, Output, World},
    teleport,
    timers::TimerKind,
};

type R = LuaResult<Vec<Value>>;

fn none() -> R {
    Ok(Vec::new())
}

/// `Lua::Binding::GetUserData<cActivity>`: a `data` table's activity.
fn activity_arg(lua: &Lua<World>, call: &NativeCall, index: usize) -> Option<usize> {
    activity_of(lua, call.arg(index))
}

// ---- Pause ----------------------------------------------------------------

/// `Pause.DisablePauseMenuAndBackpack` (`0x82895000`).
fn disable_pause_menu(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.pause_menu_disabled = true;
    none()
}

/// `Pause.EnablePauseMenuAndBackpack` (`0x82895030`).
fn enable_pause_menu(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.pause_menu_disabled = false;
    none()
}

/// `Pause.PauseSimulation()` (`0x82888d78`): `cChallengePauseManager::PauseSimulation`
/// (`0x825b12f8`), `PauseMan::Push(kPAUSE_STATE_CHALLENGE)`.
fn pause_simulation(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.output(Output::ChallengePause { pause: true });
    Ok(vec![Value::Nil])
}

/// `Pause.UnpauseSimulation()` (`0x82888e28`): `cChallengePauseManager::UnpauseSimulation`
/// (`0x825b12a0`), `PauseMan::Pull(kPAUSE_STATE_CHALLENGE)`.
fn unpause_simulation(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.output(Output::ChallengePause { pause: false });
    Ok(vec![Value::Nil])
}

// ---- HUD2 -----------------------------------------------------------------

/// `HUD2.CreateGroup(name, count, components)` (`0x8289bef8`).
fn hud_create_group(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let name = get_string(call, 0).unwrap_or_default();
    let count = get_int(call, 1);
    let mut components = Vec::new();
    if let Value::Table(t) = call.arg(2) {
        let mut key = Value::Nil;
        while let Ok(Some((k, v))) = lua.table(*t).next(&key) {
            if i32::try_from(components.len()).unwrap_or(i32::MAX) >= count {
                break;
            }
            #[expect(clippy::cast_possible_truncation, reason = "GetInt")]
            let index = match v {
                Value::Number(n) => (n + 0.5) as i32,
                _ => 0,
            };
            if let Some(name) = hud::component_name(index) {
                components.push(name);
            }
            key = k;
        }
    }
    world.hud.create_group(&name, &components);
    if let Some(activity) = world.closure {
        world.hud_owners.push((activity, hash_value32(&name)));
    }
    none()
}

/// `HUD2.GetGroupHash(name)` (`0x828850b8`).
fn hud_get_group_hash(_: &mut Lua<World>, _: &mut World, call: &NativeCall) -> R {
    Ok(vec![int_u32(hash_value32(&get_string(call, 0).unwrap_or_default()))])
}

/// `HUD2.SetupObjectiveHUDData(data, objectiveGroup, groupName)` (`0x82899810`).
fn hud_setup_objective_data(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let activity = activity_arg(lua, call, 0);
    let group = objective_group(world, call.arg(1));
    let name = get_string(call, 2).unwrap_or_default();
    let challenge = activity.map_or(0, |a| world.activities.activities[a].key);
    if let Some(data) = world.hud.group_mut(&name).and_then(|g| g.objective.as_mut()) {
        data.challenge = challenge;
        data.group = group;
        // mDirty, then MakeDirty.
        data.dirty = true;
    }
    none()
}

/// `HUD2.SetActiveGroup(name)` (`0x82896260`).
fn hud_set_active_group(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.hud.add_group_to_active(&get_string(call, 0).unwrap_or_default());
    none()
}

/// `HUD2.Show(group, component, show)` (`0x828962f0`).
fn hud_show(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let component = get_int(call, 1);
    let show = get_bool_or(call, 2, false);
    if let Some(name) = hud::component_name(component) {
        world.hud.show(&group, name, show);
    }
    none()
}

/// `HUD2.DeleteGroup(name)` (`0x82898060`).
fn hud_delete_group(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let name = get_string(call, 0).unwrap_or_default();
    let hash = hash_value32(&name);
    if let Some(activity) = world.closure {
        world.hud_owners.retain(|&(a, h)| !(a == activity && h == hash));
    }
    world.hud.delete_group(&name);
    none()
}

/// `HUD2.TutorialInstructionSet(group, n, halId, instruction)` (`0x82899dc0`):
/// `SetInstruction` on the group's `tutorialinstruction` data (defaults
/// `"#FILL IN TUTORIAL HAL ID"` and `"nollie"`).
fn hud_tutorial_instruction_set(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let hal_id = get_string_or(call, 2, "#FILL IN TUTORIAL HAL ID");
    let instruction = get_string_or(call, 3, "nollie");
    if let Some(data) = world.hud.group_mut(&group).and_then(|g| g.tutorial.as_mut()) {
        data.set_instruction(&instruction, &hal_id);
    }
    none()
}

/// `NIS.GetPhotoNISCount(definition)` (`0x8288bc98`): `NISManager::GetPhotoCount` of the
/// definition's `SceneName`.
fn nis_get_photo_count(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let scene = attrib_location(&world.pointers, call.arg(0)).and_then(|at| world.db.text_at(at)).unwrap_or_default().to_owned();
    Ok(vec![int(world.host.nis_photo_count(&scene))])
}

/// `NIS.GetPhotoNISCountFromHighlightDefinition(tHighlightDefinition, useAlternate = false)`
/// (`0x828930a0`): `NISManager::GetPhotoCount` of the definition's NIS (`tNISPlaybackDefinition`
/// at +0x10) scene. With `useAlternate` the native first copies the alternate definition (+0x50)
/// over it in the Vault data; no shipped script passes it, and the port's Vault data is read
/// only, so the alternate's scene is counted without the copy.
fn nis_get_photo_count_from_highlight_definition(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let definition = if get_bool_or(call, 1, false) { 0x50 } else { 0x10 };
    let scene = attrib_location(&world.pointers, call.arg(0))
        .and_then(|at| world.db.text_at(at.offset_by(definition)))
        .unwrap_or_default()
        .to_owned();
    Ok(vec![int(world.host.nis_photo_count(&scene))])
}

/// `cMsgEnterRealTime_PostMessage_LuaBinding` (`0x828cc6a0`): with only the port's name, the
/// message enqueued there. The presentation thread's port receives it
/// (`PresThreadMsgMan::ReceiveEnterRealTimeMsg` `0x8277bd58`: `Presentation::EnterRealTime`).
fn post_enter_real_time(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if call.len() == 1 && get_string(call, 0).as_deref() == Some("PresThreadBroadcaster") {
        world.output(Output::EnterRealTime);
    }
    none()
}

/// The reconstructed `cMsg*` posting functions (`BindAllMessageExportersToLua` `0x828cf970`).
pub(crate) fn message_exporters() -> Vec<(&'static str, NativeFn<World>)> {
    vec![("cMsgEnterRealTime", post_enter_real_time)]
}

/// `FrontEnd.OpenPhotoSelection(count = 0)` (`0x82896cb0`): FE state 8's win state set, FE
/// state 5's picking (more than one photo) and photo count, then `PushState(5)`.
fn frontend_open_photo_selection(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.output(Output::OpenPhotoSelection { count: get_int_or(call, 0, 0) });
    none()
}

/// PC port: the calling activity is a linked Skate 3 data folder's game, which runs Skate 2's
/// challenge over its own spot (`crate::skate3`).
pub(crate) fn is_skate3_spot(world: &World) -> bool {
    world.closure.is_some_and(|activity| {
        crate::skate3::linked().is_some_and(|data| data.online_spot(world.activities.activities[activity].key).is_some())
    })
}

/// PC port: a scene the calling Skate 3 game cannot play. Skate 2's scenes are cut at San
/// Vanelona's spots and would film them there, so it goes as a definition that names no scene;
/// Skate 3's own (`crate::skate3::is_scene`) play at its spots.
pub(crate) fn is_skate2_scene_at_skate3_spot(world: &World, scene: &str) -> bool {
    is_skate3_spot(world) && !crate::skate3::is_scene(scene)
}

/// `NIS.PreloadNIS(definition, ...)` (`0x8289b9a8`): the named scenes of the leading
/// `tNISPlaybackDefinition`s, `NISManager::PreloadNISList`.
fn nis_preload(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let mut scenes = Vec::new();
    for index in 0..call.len() {
        let Value::LightUserData(_) = call.arg(index) else { break };
        let scene = attrib_location(&world.pointers, call.arg(index)).and_then(|at| world.db.text_at(at)).unwrap_or_default();
        if !scene.is_empty() && !is_skate2_scene_at_skate3_spot(world, scene) {
            scenes.push(scene.to_owned());
        }
    }
    world.output(Output::PreloadNisList(scenes));
    none()
}

/// `NIS.ClearPreloadedNIS()` (`0x8288bee8`): `NISManager::UnloadAnyPreloadedNISs`.
fn nis_clear_preloaded(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.output(Output::UnloadPreloadedNis);
    none()
}

/// `ChallengeMusic.PlayMusic(activity, trigger)` (`0x8288a160`): `ChallengeAudioManager::
/// PlayMusic` for the activity's unique ID.
fn challenge_music_play(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let challenge = activity_arg(lua, call, 0).map_or(0, |activity| world.activities.activities[activity].key);
    world.output(Output::TriggerMissionMusic { trigger: get_int(call, 1), challenge });
    none()
}

/// `ChallengeMusic.UpdateLapRaceMusic(activity, share, leg, legs)` (`0x8288a228`): the race's new
/// share of its legs, 0 without legs. Reaching a half, three quarters or the double 0.9 from an
/// old share at most that triggers `HalfWay` (2), `ThreeQuarters` (3) or `StraightAway` (4), the
/// last reached winning, through `ChallengeAudioManager::PlayMusic` for the activity.
fn challenge_music_update_lap_race(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let challenge = activity_arg(lua, call, 0).map_or(0, |activity| world.activities.activities[activity].key);
    let old = get_float(call, 1);
    let (leg, legs) = (get_int(call, 2), get_int(call, 3));
    if legs <= 0 {
        return Ok(vec![Value::Number(0.0)]);
    }
    #[expect(clippy::cast_precision_loss, reason = "the native (float) conversions")]
    let share = leg as f32 / legs as f32;
    let mut trigger = None;
    if share >= 0.5 && old <= 0.5 {
        trigger = Some(2);
    }
    if share >= 0.75 && old <= 0.75 {
        trigger = Some(3);
    }
    if f64::from(share) >= 0.9 && f64::from(old) <= 0.9 {
        trigger = Some(4);
    }
    if let Some(trigger) = trigger {
        world.output(Output::TriggerMissionMusic { trigger, challenge });
    }
    Ok(vec![Value::Number(share)])
}

/// `MapBackEndManager::AddChallengeObjectIcon(name, tTriggerInstance)` (`0x82577f80`) of the
/// volume's trigger instance (`cTriggerVolumeManager::GetTriggerInstance`), pushed as its
/// `lightuserdata` (null without an icon).
fn add_challenge_object_icon(world: &mut World, icon: &str, volume: f32) -> R {
    let centre = world.host.trigger_volume_center(volume);
    let id = super::map_icons::add_challenge_object_icon_at(world, icon, centre);
    Ok(vec![super::map_icons::icon_handle(world, id)])
}

/// `Map.AddRaceGate(volume, icon = "mini_gate")` (`0x8288c018`).
fn map_add_race_gate(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let volume = get_float(call, 0);
    let icon = get_string_or(call, 1, "mini_gate");
    add_challenge_object_icon(world, &icon, volume)
}

/// `Map.AddRaceCheckPoint(volume)` (`0x8288c0f8`).
fn map_add_race_check_point(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    add_challenge_object_icon(world, "mini_spotracegate", get_float(call, 0))
}

/// `Map.AddRaceFinish(volume)` (`0x8288c1c0`).
fn map_add_race_finish(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    add_challenge_object_icon(world, "mini_gate_final", get_float(call, 0))
}

/// `Map.RemoveRaceObject(icon)` (`0x8288c348`): `RemoveChallengeObjectIcon`.
fn map_remove_race_object(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(id) = world.pointers.object(call.arg(0), ObjectKind::MapIcon) {
        world.map_icons.remove_challenge_object_icon(id);
    }
    none()
}

/// `NIS.ResetNISPlayerIndexes()` (`0x8288bde0`): `NISManager::ResetActorIDs`.
fn nis_reset_player_indexes(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.output(Output::ResetNisActorIds);
    none()
}

/// `NIS.SubmitNISPlayerIndex(player)` (`0x8288be68`): `NISManager::AddActorID`.
fn nis_submit_player_index(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.output(Output::AddNisActorId(get_int(call, 0)));
    none()
}

/// `HUD2.ShowChallengeCompleteMessage(halId)` (`0x82896688`).
fn hud_show_challenge_complete_message(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.output(Output::ChallengeResultMessage { hal_id: get_string(call, 0).unwrap_or_default(), failed: false });
    none()
}

/// `HUD2.ShowChallengeFailedMessage(halId)` (`0x828966d8`).
fn hud_show_challenge_failed_message(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.output(Output::ChallengeResultMessage { hal_id: get_string(call, 0).unwrap_or_default(), failed: true });
    none()
}

/// The `PreFilmSetupHUD` group with its component (`HUD2.CreatePreFilmSetupHUD` `0x82896728`).
const PRE_FILM_SETUP_HUD: &str = "PreFilmSetupHUD";

fn create_pre_film_setup_hud(world: &mut World) {
    world.hud.create_group(PRE_FILM_SETUP_HUD, &[]);
    world.hud.add_component(PRE_FILM_SETUP_HUD, PRE_FILM_SETUP_HUD);
}

/// `HUD2.CreatePreFilmSetupHUD()` (`0x82896728`).
fn hud_create_pre_film_setup(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    if !world.hud.group_exists(PRE_FILM_SETUP_HUD) {
        create_pre_film_setup_hud(world);
    }
    none()
}

/// `HUD2.DeletePreFilmSetupHUD()` (`0x828967c0`).
fn hud_delete_pre_film_setup(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    if world.hud.group_exists(PRE_FILM_SETUP_HUD) {
        world.hud.delete_group(PRE_FILM_SETUP_HUD);
    }
    none()
}

/// `HUD2.ShowPreFilmSetupHUD(show = true)` (`0x82896818`): created when missing, then shown
/// and made active, or hidden and made inactive.
fn hud_show_pre_film_setup(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let show = get_bool_or(call, 0, true);
    if !world.hud.group_exists(PRE_FILM_SETUP_HUD) {
        create_pre_film_setup_hud(world);
    }
    if show {
        world.hud.add_group_to_active(PRE_FILM_SETUP_HUD);
        world.hud.show(PRE_FILM_SETUP_HUD, PRE_FILM_SETUP_HUD, true);
    } else {
        world.hud.show(PRE_FILM_SETUP_HUD, PRE_FILM_SETUP_HUD, false);
        world.hud.remove_group_from_active(PRE_FILM_SETUP_HUD);
    }
    none()
}

/// `HUD2.ShowTrickDisplayContext(visible)` (`0x8289a420`): "main"'s `TrickDisplayHUDData::
/// SetContextTrickDisplay` (`0x826b2eb0`).
fn hud_show_trick_display_context(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if world.hud.group_exists(hud::MAIN) {
        world.hud.context_trick_display = hud::ContextTrickDisplay(get_bool(call, 0));
    }
    none()
}

/// `HUD2.ObjectiveSetModifiedPosition(group, modified = false)` (`0x82899970`).
fn hud_objective_set_modified_position(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let modified = get_bool_or(call, 1, false);
    if let Some(data) = world.hud.group_mut(&group).and_then(|g| g.objective.as_mut())
        && data.is_modified_position != modified
    {
        data.is_modified_position = modified;
        data.dirty = true;
    }
    none()
}

/// `PhonelistHUDData::SetServiceIconStatus` (`0x826b3748`) of the `phonelist` group.
fn set_service_icon_status(world: &mut World, icon: usize, show: bool) {
    if let Some(data) = world.hud.group_mut("phonelist").and_then(|g| g.phonelist.as_mut()) {
        data.show_icon[icon] = show;
        data.dirty = true;
    }
}

/// `HUD2.ShowSecurityHUD(show = true)` (`0x82899c58`): the security service icon.
fn hud_show_security(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    set_service_icon_status(world, 0, get_bool_or(call, 0, true));
    none()
}

/// `HUD2.ShowPoolDrainHUD(show = true)` (`0x82899cd0`).
fn hud_show_pool_drain(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    set_service_icon_status(world, 1, get_bool_or(call, 0, true));
    none()
}

/// `HUD2.ShowUncapHUD(show = true)` (`0x82899d48`).
fn hud_show_uncap(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    set_service_icon_status(world, 2, get_bool_or(call, 0, true));
    none()
}

/// `HUD2.SetupDistanceHUDData(group, maxDistance, distance)` (`0x828987d8`): the group's
/// `DistanceMeterData`.
fn hud_setup_distance_data(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let max_distance = get_float(call, 1);
    let distance = get_float(call, 2);
    if let Some(data) = world.hud.group_mut(&group).and_then(|g| g.distance.as_mut()) {
        data.distance = distance;
        data.dirty = true;
        data.max_distance = max_distance;
    }
    none()
}

/// `HUD2.ShowCentralMessage(halId, colour, s1..s4, audioHook)` (`0x828963e8`).
fn hud_show_central_message(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let message = CentralMessage {
        hal_id: get_string(call, 0).unwrap_or_default(),
        colour: super::skate3_natives::central_colour(world, get_int_or(call, 1, 0)),
        args: [get_string_or(call, 2, ""), get_string_or(call, 3, ""), get_string_or(call, 4, ""), get_string_or(call, 5, "")],
        audio_hook: get_int_or(call, 6, 0),
    };
    world.output(Output::CentralMessage(message.clone()));
    world.hud.central_messages.push(message);
    none()
}

// ---- Online ---------------------------------------------------------------

/// `Online.GetLocalPlayerID()` (`0x8288a570`): the skater manager's local
/// index (single player: 0).
fn online_get_local_player_id(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(world.local_player)])
}

// ---- AI -------------------------------------------------------------------

/// `tLocationID::LocationName` behind a `lightuserdata`.
fn location_name(world: &World, value: &Value) -> Option<String> {
    let at = attrib_location(&world.pointers, value)?;
    world.db.text_at(at).map(str::to_owned)
}

/// `cLocationManager::GetLocationPosition` (`0x82a8e330`).
fn location_position(world: &World, name: &str) -> [f32; 3] {
    world.location(hash_value32(name)).map_or(world.default_location, |t| [t[12], t[13], t[14]])
}

/// `AI.SkaterDistanceFromLocation(player, location)` (`0x8288de40`).
fn ai_skater_distance_from_location(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int(call, 0);
    let mut distance = 0.0;
    if player <= 8
        && let Some(name) = location_name(world, call.arg(1))
        && let Some(p) = super::skate::skater(world, player).map(|frame| frame.position)
    {
        let target = location_position(world, &name);
        let d = [target[0] - p[0], target[1] - p[1], target[2] - p[2]];
        distance = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    }
    Ok(vec![Value::Number(distance)])
}

/// `AI.RepositionAI(player, location, posID = player, repositionMarker =
/// false, transitionTime = 1, setCamera = false)` (`0x8288d140`) →
/// `cChallengeTeleportManager::TeleportPlayer`. An empty location falls back
/// to a location in the fourth argument.
fn ai_reposition(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int(call, 0);
    let pos_id = get_int_or(call, 2, player);
    let reposition_marker = get_bool_or(call, 3, false);
    let location = location_name(world, call.arg(1))
        .filter(|n| !n.is_empty())
        .or_else(|| location_name(world, call.arg(3)))
        .unwrap_or_default();
    let transition_time = super::binding::get_float_or(call, 4, 1.0);
    let set_camera = get_bool_or(call, 5, false);
    if let Some(online) = world.online.as_mut() {
        if let Some(slot) = usize::try_from(player).ok().and_then(|index| online.teleporting.get_mut(index)) {
            *slot = true;
        }
        if set_camera {
            online.turn_player = Some(player);
        }
    }
    teleport::teleport_player(world, player, &location, pos_id, reposition_marker, transition_time, set_camera);
    none()
}

/// `AI.ClearTeleports` (`0x8288d2e8`).
fn ai_clear_teleports(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    teleport::clear_teleports(world);
    none()
}

/// `Teleport.TeleportLocalPlayerToLocationName(name, repositionMarker =
/// false, transitionTime = 1)` (`0x82db0640`), bound in both Lua states (the progression's
/// through `cSignPostManager::PerformLuaCBinding` `0x825e4d58`).
pub(crate) fn teleport_local_player_to_location_name(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(name) = get_string(call, 0) {
        //82db06f8 uses Simulation::GetLocalPlayerID, independent of spectator camera.
        let player = super::players::network_local_player(world);
        let reposition_marker = get_bool_or(call, 1, false);
        let transition_time = super::binding::get_float_or(call, 2, 1.0);
        teleport::teleport_player(world, player, &name, player, reposition_marker, transition_time, false);
    }
    none()
}

// ---- ChallengeSave --------------------------------------------------------

/// `ChallengeSave.GetSaveData(data, name)` (`0x82890538`) through `GetSaveData_helper`
/// (`0x82890478`): `cSignPostManager::GetChallengeSaveData` of the activity's challenge.
fn challenge_save_get(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(activity) = super::attrib_lua::activity_of(lua, call.arg(0)) else { return Ok(vec![int(0)]) };
    let key = world.activities.activities[activity].key;
    let name = get_string(call, 1).unwrap_or_default();
    Ok(vec![int(super::signpost_progress::challenge_save_data(world, key, &name))])
}

/// `ChallengeSave.SetSaveData(data, name, value)` (`0x828906b0`) through `SetSaveData_helper`
/// (`0x828905d8`): `cSignPostManager::SetChallengeSaveData`.
fn challenge_save_set(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(activity) = super::attrib_lua::activity_of(lua, call.arg(0)) else { return none() };
    let key = world.activities.activities[activity].key;
    let name = get_string(call, 1).unwrap_or_default();
    super::signpost_progress::set_challenge_save_data(world, key, &name, get_int(call, 2));
    none()
}

/// The activity `Lua::Binding::GetData` (`0x82aba578`) takes: a light userdata's pointer (the
/// activity a discovery message carries), else the data-provider userdata's.
fn data_activity(lua: &Lua<World>, world: &World, value: &Value) -> Option<usize> {
    world
        .pointers
        .object(value, ObjectKind::Activity)
        .and_then(|id| usize::try_from(id).ok())
        .or_else(|| activity_of(lua, value))
}

/// `ChallengeSave.GetOtherChallengeSaveData(activity, name)` (`0x82890588`): `GetSaveData_helper`
/// of an activity taken by `Lua::Binding::GetData`.
fn challenge_save_get_other(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(activity) = data_activity(lua, world, call.arg(0)) else { return Ok(vec![int(0)]) };
    let key = world.activities.activities[activity].key;
    let name = get_string(call, 1).unwrap_or_default();
    Ok(vec![int(super::signpost_progress::challenge_save_data(world, key, &name))])
}

/// `ChallengeSave.SetOtherChallengeSaveData(activity, name, value)` (`0x828906f0`):
/// `SetSaveData_helper` of an activity taken by `Lua::Binding::GetData`.
fn challenge_save_set_other(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(activity) = data_activity(lua, world, call.arg(0)) else { return none() };
    let key = world.activities.activities[activity].key;
    let name = get_string(call, 1).unwrap_or_default();
    super::signpost_progress::set_challenge_save_data(world, key, &name, get_int(call, 2));
    none()
}

// ---- Objectives -----------------------------------------------------------

fn objective_group(world: &World, value: &Value) -> Option<u32> {
    world.pointers.object(value, ObjectKind::ObjectiveGroup)
}

/// `Objectives.MakeObjectives(playerID)` (`0x82d53108`) through
/// `MakeObjectives_helper(lua, "Objectives")` (`0x82d52de0`): the activity's
/// objective group with every `Objectives` entry added in order.
fn objectives_make(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    make_objectives_helper(world, call, "Objectives")
}

/// `Objectives.MakeObjectivesForAI(playerID)` (`0x82d53150`): `MakeObjectives_helper(lua,
/// "ObjectivesForAI")`.
fn objectives_make_for_ai(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    make_objectives_helper(world, call, "ObjectivesForAI")
}

/// `Objectives.MakeObjectivesForPlayer(playerID)` (`0x82d53198`): `MakeObjectives_helper(lua,
/// "ObjectivesForPlayer")`.
fn objectives_make_for_player(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    make_objectives_helper(world, call, "ObjectivesForPlayer")
}

/// `MakeObjectives_helper(lua, name)` (`0x82d52de0`): the calling activity's group for the player
/// (its `ObjectiveFlow`, display and notification statuses), every entry of the challenge's
/// `name` list added in order, the points objective for a `PointRequirement` of at least 0, and
/// the local data's `UniqueTrickGroup` (a nonzero collection) tracked.
fn make_objectives_helper(world: &mut World, call: &NativeCall, name: &str) -> R {
    use super::objectives::challenge_field as cf;
    let Some(activity) = world.closure else { return Ok(vec![Value::Nil]) };
    let player = get_int_or(call, 0, -1);
    let key = world.activities.activities[activity].key;
    let db = std::sync::Arc::clone(&world.db);
    let c = super::keys::class::CHALLENGES;
    // tObjectiveFlow defaults to {AllAtStart, 1}.
    let (flow_type, max_active) = db
        .attribute(c, key, cf::OBJECTIVE_FLOW)
        .and_then(|a| a.value_bytes(0))
        .filter(|b| b.len() >= 8)
        .map_or((0, 1), |b| {
            (i32::from_be_bytes([b[0], b[1], b[2], b[3]]), i32::from_be_bytes([b[4], b[5], b[6], b[7]]))
        });
    let display = db.attribute(c, key, cf::DISPLAY_STATUSES).and_then(|a| a.u32(0)).unwrap_or(u32::MAX);
    let notification = db.attribute(c, key, cf::NOTIFICATION_STATUSES).and_then(|a| a.u32(0)).unwrap_or(u32::MAX);
    let show_active = db.attribute(c, key, cf::SHOW_ACTIVE_WITH_NOTIFICATIONS).and_then(|a| a.bool(0)).unwrap_or(false);
    let group =
        super::objectives::add_objective_challenge(world, activity, player, (flow_type, max_active), (display, notification, show_active));
    let objectives: Vec<u64> = db
        .attribute(c, key, super::keys::hash64(name))
        .map(|a| (0..a.len()).filter_map(|i| a.ref_spec(i)).map(|(_, k)| k).collect())
        .unwrap_or_default();
    for objective in objectives {
        super::objectives::add_objective(world, group, objective);
    }
    // PointRequirement, the zeroed default area when the challenge has none: a requirement of
    // at least 0 adds the points objective.
    let points = db.attribute(c, key, cf::POINTS_REQUIREMENT).and_then(|a| a.i32(0)).unwrap_or(0);
    if points >= 0 {
        super::objectives::add_points_objective(world, group, points.cast_unsigned());
    }
    let unique_trick_group = world.activities.activities[activity]
        .local_data
        .and_then(|(class, collection)| db.attribute(class, collection, UNIQUE_TRICK_GROUP))
        .and_then(|a| a.ref_spec(0))
        .map(|(_, collection)| collection)
        .filter(|&collection| collection != 0);
    if let Some(unique_trick_group) = unique_trick_group
        && let Some(i) = world.objectives.group_index(group)
    {
        world.objectives.groups[i].unique_trick_group = Some(unique_trick_group);
    }
    let handle = world.objectives.group_index(group).map_or(0, |i| world.objectives.groups[i].handle);
    Ok(vec![Value::LightUserData(handle)])
}

/// The group's objective keys, in order.
fn objective_keys(world: &World, group: u32) -> Vec<u64> {
    world.objectives.group_index(group).map_or_else(Vec::new, |i| world.objectives.groups[i].objectives.iter().map(|o| o.key).collect())
}

/// `Objectives.GetTotalNumObjectives(group)` (`0x82d43388`): the group's objectives, its
/// points objective included.
fn objectives_get_total_num(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let count = objective_group(world, call.arg(0)).map_or(0, |group| objective_keys(world, group).len());
    Ok(vec![int(i32::try_from(count).unwrap_or(i32::MAX))])
}

/// `Objectives.GetAccumulatedPoints(group)` (`0x82d4a008`): `cChallengeObjectiveGroup::
/// GetAccumulatedPoints` (`0x82d43b50`).
fn objectives_get_accumulated_points(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let points = objective_group(world, call.arg(0)).map_or(0, |group| super::objectives::accumulated_points(world, group));
    Ok(vec![int(points.cast_signed())])
}

/// `Objectives.GetCurrentNumCompletedObjectives(group, includeOPO = true, includeFractional =
/// true)` (`0x82d49ce8`): `cChallengeObjectiveGroup::GetNumCompletedObjectives` (`0x82d43e10`).
fn objectives_get_current_num_completed(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let include_opo = get_bool_or(call, 1, true);
    let include_fractional = get_bool_or(call, 2, true);
    let count = objective_group(world, call.arg(0))
        .map_or(0, |group| super::objectives::num_completed_objectives(world, group, include_opo, include_fractional));
    Ok(vec![int(i32::try_from(count).unwrap_or(i32::MAX))])
}

/// The `challenge_objective` attribute of the group's objective at `index`.
fn objective_attribute(world: &World, group: u32, index: i32, attribute: u64) -> Option<super::attrib_db::Attribute<'_>> {
    let key = *objective_keys(world, group).get(usize::try_from(index).ok()?)?;
    world.db.attribute(super::keys::class::CHALLENGE_OBJECTIVE, key, attribute)
}

/// `Objectives.GetFractionDenominator(group, index = 0)` (`0x82d49a38`): the objective's
/// fraction denominator (`cChallengeObjectiveGroup::GetFractionDenominator` `0x82d43528`), 0
/// when it has none.
fn objectives_get_fraction_denominator(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let index = get_int_or(call, 1, 0);
    let denominator = objective_group(world, call.arg(0))
        .and_then(|group| objective_attribute(world, group, index, super::objectives::field::FRACTION_DENOMINATOR))
        .and_then(|a| a.i16(0))
        .unwrap_or(0);
    Ok(vec![int(i32::from(denominator))])
}

/// `Objectives.SetFractionNumerator(group, index = 0, value = 0)` (`0x82d49ad0`):
/// `cChallengeObjectiveGroup::SetFractionNumerator` (`0x82d43488`), dirtying the objective.
fn objectives_set_fraction_numerator(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let (index, value) = (get_int_or(call, 1, 0), get_int_or(call, 2, 0));
    if let Some(group) = objective_group(world, call.arg(0))
        && let Some(g) = world.objectives.group_index(group)
        && let Some(o) = usize::try_from(index).ok().and_then(|i| world.objectives.groups[g].objectives.get_mut(i))
    {
        o.fraction_numerator = value;
        o.dirty = true;
    }
    none()
}

/// `Objectives.ChangeObjectivePointsRequirement(group, points = 0)` (`0x82d43408`): the group's
/// `PointsRequirement` (+0x4C).
fn objectives_change_points_requirement(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let points = get_int_or(call, 1, 0);
    if let Some(group) = objective_group(world, call.arg(0))
        && let Some(g) = world.objectives.group_index(group)
    {
        world.objectives.groups[g].points_requirement = points.cast_unsigned();
    }
    none()
}

/// `challenge_local_data.UniqueTrickGroup`.
const UNIQUE_TRICK_GROUP: u64 = 0x70db_0d89_20d1_6efe;

/// `Objectives.IsTrackedUniqueTrickGroup()` (`0x82d49ea8`): the closure activity's local data
/// names a trick group collection (`RefSpec::GetCollection`).
fn objectives_is_tracked_unique_trick_group(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let tracked = world
        .closure
        .and_then(|activity| world.activities.activities[activity].local_data)
        .and_then(|(class, collection)| world.db.attribute(class, collection, UNIQUE_TRICK_GROUP))
        .and_then(|a| a.ref_spec(0))
        .is_some_and(|(class, collection)| world.db.has_collection(class, collection));
    Ok(vec![Value::Boolean(tracked)])
}

/// `Objectives.GetNumObjectivesRequiresHighlight(group)` (`0x82d49d80`): the objectives whose
/// `RequiresHighlight` is set.
fn objectives_get_num_requires_highlight(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let count = objective_group(world, call.arg(0)).map_or(0, |group| {
        objective_keys(world, group)
            .into_iter()
            .filter(|&key| {
                world
                    .db
                    .attribute(super::keys::class::CHALLENGE_OBJECTIVE, key, super::objectives::field::REQUIRES_HIGHLIGHT)
                    .and_then(|a| a.bool(0))
                    .unwrap_or(false)
            })
            .count()
    });
    Ok(vec![int(i32::try_from(count).unwrap_or(i32::MAX))])
}

/// `Objectives.GetObjectiveFailureHALText(group, index, failedIndex)` (`0x82d4a140`): the HAL
/// text (definition +12) of the objective's plain failure at `failedIndex` when it has plain
/// failures, else of that failure-with-events condition's definition; nil without one.
fn objectives_get_failure_hal_text(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    use super::objectives::field;
    let (index, failed) = (get_int(call, 1), get_int(call, 2));
    let Some(group) = objective_group(world, call.arg(0)) else { return Ok(vec![Value::Nil]) };
    let failed = usize::try_from(failed).ok();
    let plain = objective_attribute(world, group, index, field::FAILURES).filter(|a| !a.is_empty());
    let definition = match plain {
        Some(failures) => failed.and_then(|i| failures.data(i)),
        None => objective_attribute(world, group, index, field::FAILURES_WITH_EVENTS)
            .and_then(|a| failed.and_then(|i| a.ref_spec(i)))
            .and_then(|(_, condition)| world.db.attribute(field::CONDITION_CLASS, condition, field::OBJECTIVE_DEFINITION))
            .and_then(|a| a.data(0)),
    };
    let text = definition.and_then(|at| world.db.text_at(at.offset_by(12)));
    Ok(vec![text.map_or(Value::Nil, Value::from)])
}

/// `challenge_objective.HighlightDefinition`.
const HIGHLIGHT_DEFINITION: u64 = 0x928f_9d69_3922_d585;

/// `Objectives.GetObjectiveNISPlaybackDefinition(group, index, alternate = false)`
/// (`0x82d49c18`): the objective's `HighlightDefinition` `tNISPlaybackDefinition` (`+0x50` for
/// the alternate, else `+0x10`).
fn objectives_get_nis_playback_definition(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let index = get_int(call, 1);
    let alternate = get_bool_or(call, 2, false);
    let definition = objective_group(world, call.arg(0))
        .and_then(|group| objective_attribute(world, group, index, HIGHLIGHT_DEFINITION))
        .and_then(|a| a.data(0));
    let pointee = definition.map_or(Pointee::DefaultData, |at| Pointee::Attrib(at.offset_by(if alternate { 0x50 } else { 0x10 })));
    Ok(vec![Value::LightUserData(world.pointers.handle(pointee))])
}

/// `Objectives.GetObjectiveHighlightDefinition(group, index)` (`0x82d49b58`): the objective's
/// `HighlightDefinition`, the zeroed default area (`0x90` bytes) when it has none.
fn objectives_get_highlight_definition(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let index = get_int(call, 1);
    let definition = objective_group(world, call.arg(0))
        .and_then(|group| objective_attribute(world, group, index, HIGHLIGHT_DEFINITION))
        .and_then(|a| a.data(0));
    let pointee = definition.map_or(Pointee::DefaultData, Pointee::Attrib);
    Ok(vec![Value::LightUserData(world.pointers.handle(pointee))])
}

/// `Objectives.AddPointsObjective(group, points)` (`0x82d535a0`):
/// `cChallengeObjectiveGroup::AddPointsObjective` (`0x82d51938`) under the calling activity's
/// name.
fn objectives_add_points_objective(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group =
        objective_group(world, call.arg(0)).ok_or_else(|| lua.error("Objectives.AddPointsObjective: expected an objective group".to_owned()))?;
    super::objectives::add_points_objective(world, group, get_int(call, 1).cast_unsigned());
    none()
}

/// `Objectives.SetEvaluationStatus(group, status)` (`0x82d4b6b8`).
fn objectives_set_evaluation_status(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(group) = objective_group(world, call.arg(0)) {
        super::objectives::set_evaluation_status(world, group, !matches!(call.arg(1), Value::Nil | Value::Boolean(false)));
    }
    none()
}

/// `Objectives.FailAllObjectiveStatus(group)` (`0x82d4d410`): every objective failed
/// (`cChallengeObjectiveGroup::FailAllObjectiveStatus` `0x82d4c8a8`).
fn objectives_fail_all(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(group) = objective_group(world, call.arg(0)) {
        let count = world.objectives.group_index(group).map_or(0, |index| world.objectives.groups[index].objectives.len());
        for index in 0..count {
            super::objectives::set_status(world, group, index, super::objectives::Status::Failed);
        }
    }
    none()
}

/// `Objectives.ForceObjectiveComplete(group, index)` (`0x82d51740`).
fn objectives_force_complete(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let (Some(group), Ok(index)) = (objective_group(world, call.arg(0)), usize::try_from(get_int(call, 1))) {
        super::objectives::force_complete(world, group, index);
    }
    none()
}

/// `Challenge.IsChallengeAboutToStartDuringLoad()` (`0x82880ac0`).
fn challenge_is_about_to_start_during_load(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Boolean(world.start_challenge_during_load)])
}

/// `Challenge.IsReplayingChallenge()` (`0x82880af0`).
fn challenge_is_replaying(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Boolean(world.replaying_challenge)])
}

/// `Challenge.LoseChallenge(activity)` (`0x82888628`): `cSignPostManager::LoseChallenge` of its
/// unique ID; the `<name>_failed` telemetry is not ported.
fn challenge_lose(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(activity) = activity_arg(lua, call, 0) {
        let key = world.activities.activities[activity].key;
        signpost::end_challenge_unwon(world, key, 1);
    }
    none()
}

/// `Challenge.AbandonChallenge(activity)` (`0x82888710`).
fn challenge_abandon(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(activity) = activity_arg(lua, call, 0) {
        let key = world.activities.activities[activity].key;
        signpost::end_challenge_unwon(world, key, 2);
    }
    none()
}

/// `Challenge.SetIsGotoSignupAvailable(available)` (`0x82888c28`).
fn challenge_set_is_goto_signup_available(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.goto_allows_signup = get_bool(call, 0);
    none()
}

/// `Challenge.IsSigningup()` (`0x82888cd0`): `cChallengeSignUpManager::IsSigningUp`.
fn challenge_is_signing_up(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Boolean(super::signup::is_signing_up(world))])
}

/// `ChallengeManager.SwitchChallengeCamera(active, tLocationID)` (`0x82895558`): the challenge
/// camera at the location's orientation, or off; on at an unknown location sends nothing.
fn challenge_manager_switch_challenge_camera(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    const IDENTITY_AXES: [f32; 16] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let active = get_bool(call, 0);
    let mut matrix = IDENTITY_AXES;
    if active {
        let location = location_name(world, call.arg(1)).and_then(|name| world.location(hash_value32(&name)));
        let Some(orientation) = location else { return none() };
        matrix = orientation;
    }
    world.output(Output::ChallengeCamera { active, matrix });
    none()
}

/// `eSk8Characters`: `Player`, `Unknown` and `Max`.
mod character {
    pub const PLAYER: i32 = 0;
    pub const UNKNOWN: i32 = 39;
    pub const MAX: i32 = 100;
}

/// A player's `characters_marquee` audio character (layout word 3), or `None` for a player
/// without a character.
fn marquee_character(world: &World, player: i32) -> Option<i32> {
    let character = world.players.as_deref()?.character(player);
    #[expect(clippy::cast_possible_wrap, reason = "eSk8Characters")]
    let audio = world.db.layout_word(crate::ai::skater_manager::CHARACTERS_MARQUEE, character, 3) as i32;
    (character as u32 != u32::MAX).then_some(audio)
}

fn num_players(world: &World) -> i32 {
    world.players.as_deref().map_or(0, |players| players.num_players())
}

/// `ChallengeSpeech::GetSkaterCharacterFromIndex` (`0x82888ed8`).
pub(crate) fn skater_character_from_index(world: &World, index: i32) -> i32 {
    if index <= 0 {
        return if index == 0 { character::PLAYER } else { character::MAX };
    }
    if index >= num_players(world) {
        return character::MAX;
    }
    marquee_character(world, index).unwrap_or(character::MAX)
}

/// `ChallengeAudioManager::PlaySkaterSpeech` (`0x82da2e48`): `PlayMissionSpeech` with the
/// player's character.
fn play_skater_speech(world: &mut World, player: i32, id: i32) {
    let character = if player > 0 && player < num_players(world) {
        marquee_character(world, player).unwrap_or(character::UNKNOWN)
    } else if player == 0 {
        character::PLAYER
    } else {
        character::UNKNOWN
    };
    world.output(Output::PlayNpcSpeech { character, id, param: 0 });
}

/// `ChallengeSpeech::PlayNPCSpeechUsingAttributeHelper` (`0x82889338`): a player's speech in
/// that player's voice (with a param, as the player's character), else the character's.
fn play_npc_speech_using_attribute(world: &mut World, character: i32, id: i32, player: i32, param: i32) {
    if player >= 0 {
        if param < 0 {
            play_skater_speech(world, player, id);
        } else {
            let character = skater_character_from_index(world, player);
            world.output(Output::PlayNpcSpeech { character, id, param });
        }
    } else {
        world.output(Output::PlayNpcSpeech { character, id, param });
    }
}

/// `tSpeechInfo` behind a `lightuserdata`: its `Character` and (short) speech ID.
fn speech_info(world: &World, value: &Value) -> Option<(i32, i32)> {
    let at = attrib_location(&world.pointers, value)?;
    let bytes = world.db.bytes(at, 6)?;
    Some((i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]), i32::from(i16::from_be_bytes([bytes[4], bytes[5]]))))
}

/// `ChallengeSpeech.PlayNPCSpeech(character, id)` (`0x82889a70`).
fn speech_play_npc(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let (character, id) = (get_int(call, 0), get_int(call, 1));
    world.output(Output::PlayNpcSpeech { character, id, param: 0 });
    none()
}

/// `ChallengeSpeech.PlaySkaterSpeech(player, id)` (`0x82889b30`).
fn speech_play_skater(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    play_skater_speech(world, get_int(call, 0), get_int(call, 1));
    none()
}

/// `ChallengeSpeech.PlayNPCSpeechUsingAttribute(tSpeechInfo, player = -1, param = -1)`
/// (`0x82889438`).
fn speech_play_npc_using_attribute(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some((character, id)) = speech_info(world, call.arg(0)) {
        play_npc_speech_using_attribute(world, character, id, get_int_or(call, 1, -1), get_int_or(call, 2, -1));
    }
    none()
}

/// `ChallengeSpeech.PlayNPCSpeechUsingOverrideAttribute(tSpeechInfo, player = -1, param = -1)`
/// (`0x828894c8`): an `Unknown` character speaks as the challenge's `AudioCharacterOverride`
/// when its global data has one (`PlayNPCSpeechOverrideHelper` `0x82882408`).
fn speech_play_npc_using_override_attribute(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some((character, id)) = speech_info(world, call.arg(0)) else { return none() };
    let override_key = super::keys::hash64("AudioCharacterOverride");
    let character = match world.closure {
        Some(activity)
            if world.activities.activities[activity]
                .global_data
                .is_some_and(|(c, k)| world.db.attribute(c, k, override_key).is_some()) =>
        {
            let challenge = world.activities.activities[activity].key;
            let audio = world.db.attribute(super::keys::class::CHALLENGES, challenge, override_key).and_then(|a| a.i32(0)).unwrap_or(0);
            if audio == character::MAX || character != character::UNKNOWN { character } else { audio }
        }
        _ => character,
    };
    play_npc_speech_using_attribute(world, character, id, get_int_or(call, 1, -1), get_int_or(call, 2, -1));
    none()
}

/// `ChallengeSpeech.PlayNPCSpeechUsingDynamicCompetitor(tSpeechInfo, character = -1,
/// player = -1, param = -1)` (`0x82889570`): the speech ID in the given character's voice.
fn speech_play_npc_using_dynamic_competitor(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some((_, id)) = speech_info(world, call.arg(0)) {
        play_npc_speech_using_attribute(world, get_int_or(call, 1, -1), id, get_int_or(call, 2, -1), get_int_or(call, 3, -1));
    }
    none()
}

/// `ChallengeSpeech.PlayNPCSpeechUsingAttributeAndSkaterIndex(tSpeechInfo, skater)`
/// (`0x82889618`): the skater's character as the speech's param.
fn speech_play_npc_using_attribute_and_skater_index(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let skater = get_int(call, 1);
    if let Some((character, id)) = speech_info(world, call.arg(0)) {
        let param = skater_character_from_index(world, skater);
        world.output(Output::PlayNpcSpeech { character, id, param });
    }
    none()
}

/// `Time.GetVisualGameTick()` (`0x82887138`): `Simulation::mTick`.
fn time_get_visual_game_tick(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int_u32(world.tick)])
}

/// `ChallengeSpeech.PlayFollowMeWarningSpeech(character, id, param)` (`0x82889048`):
/// `ChallengeAudioManager::PlaySpeech`.
fn speech_play_follow_me_warning(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let (character, id, param) = (get_int(call, 0), get_int(call, 1), get_int(call, 2));
    world.output(Output::PlayNpcSpeech { character, id, param });
    none()
}

/// `Objectives.ResetObjectives(group, resetCumulative = true)` (`0x82d4db10`).
fn objectives_reset_all(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(group) = objective_group(world, call.arg(0)) {
        let reset_cumulative = get_bool_or(call, 1, true);
        super::objectives::reset_objectives(world, group, reset_cumulative);
    }
    none()
}

/// `Objectives.ResetObjective(group, index, resetCumulative = true,
/// resetStatus = true)` (`0x82d4db98`).
fn objectives_reset_one(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(group) = objective_group(world, call.arg(0)) {
        let index = get_int_or(call, 1, 0);
        let reset_cumulative = get_bool_or(call, 2, true);
        let reset_status = get_bool_or(call, 3, true);
        if let Ok(index) = usize::try_from(index) {
            super::objectives::reset_objective(world, group, index, reset_cumulative, reset_status);
        }
    }
    none()
}

/// `Objectives.DeleteObjectiveGroup(group)` (`0x82d51698`).
fn objectives_delete_group(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(group) = objective_group(world, call.arg(0)) {
        super::objectives::delete_group(world, group);
    }
    none()
}

// ---- ChallengePresentation ------------------------------------------------

/// `ChallengePresentation.AddPresentationGroup(data)` (`0x82890758`).
fn presentation_add_group(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let key = activity_arg(lua, call, 0).map_or(0, |a| world.activities.activities[a].key);
    Ok(vec![Value::LightUserData(presentation::add_group(world, key))])
}

/// `ChallengePresentation.AddPresentationEvent(group, event)` (`0x828908c8`).
fn presentation_add_event(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(group) = world.pointers.object(call.arg(0), ObjectKind::PresentationGroup) else { return none() };
    let Some(location) = attrib_location(&world.pointers, call.arg(1)) else { return none() };
    if let Some(event) = PresentationEvent::read(&world.db, location) {
        presentation::add_event(world, group, event);
    }
    presentation::check_group_for_hud(world, group);
    none()
}

/// `ChallengePresentation.StartPresentation(group)` (`0x828909f0`).
fn presentation_start(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(group) = world.pointers.object(call.arg(0), ObjectKind::PresentationGroup) {
        presentation::start(world, group);
    }
    none()
}

/// `ChallengePresentation.RemoveAllPresentationGroups(data)` (`0x82890818`).
fn presentation_remove_all(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(a) = activity_arg(lua, call, 0) {
        let key = world.activities.activities[a].key;
        presentation::remove_groups(world, key);
    }
    none()
}

// ---- HOM ------------------------------------------------------------------

/// `HOM.EnableHOMVisuals(enable)` (`0x828945d8`) → `ChallengeHOMManager::EnableHOMVisuals`
/// (`0x825ae500`): `mHOMVisualsEnabled`, which `Physics::IsHallOfMeatEffectEnabled`
/// (`0x827acbd0`) reads on every scoring update.
fn hom_enable_visuals(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    super::hom::enable_visuals(world,world.closure,get_bool_or(call,0,false));
    none()
}

// ---- ChallengeAsset -------------------------------------------------------

/// `ChallengeAsset.IsChallengeAssetLoadComplete(data)` (`0x82e10528`).
fn asset_is_load_complete(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let complete = activity_arg(lua, call, 0).is_none_or(|a| assets::is_load_complete(world, a));
    Ok(vec![Value::Boolean(complete)])
}

/// `ChallengeAsset.LoadChallengeAssets(data)` (`0x82e12668`).
fn asset_load(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(a) = activity_arg(lua, call, 0) {
        assets::load_challenge_assets(world, a, true, false, 0);
    }
    none()
}

/// `ChallengeAsset.UnloadChallengeAssets(data)` (`0x82e0f850`).
fn asset_unload(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(a) = activity_arg(lua, call, 0) {
        assets::unload(world, a);
    }
    none()
}

/// `ChallengeAsset.PreloadChallengeAssetsByName(name)` (`0x82e126e8`).
pub(crate) fn asset_preload_by_name(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let name = get_string(call, 0).unwrap_or_default();
    if let Some(a) = world.activities.find(super::keys::hash64(&name)) {
        assets::load_challenge_assets(world, a, true, true, 0);
    }
    none()
}

// ---- Effects --------------------------------------------------------------

/// `Effects.StartEffect(effect, time)` (`0x82daa090`).
fn effects_start(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let effect = usize::try_from(get_int(call, 0)).unwrap_or(usize::MAX);
    effects::start_effect(world, effect, get_float(call, 1));
    none()
}

/// `Effects.ResetEffect(effect)` (`0x82daa100`).
fn effects_reset(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let effect = usize::try_from(get_int(call, 0)).unwrap_or(usize::MAX);
    effects::reset_effect(world, effect);
    none()
}

// ---- ChallengeTimer -------------------------------------------------------

/// `ChallengeTimer.AddCountdownTimer`/`AddHUDCountdownTimer(seconds)`
/// (`0x82db3c20`): owned by the running activity.
fn timer_add_countdown(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let owner = world.closure;
    Ok(vec![int(world.timers.add(TimerKind::CountDown, get_float(call, 0), owner))])
}

/// `ChallengeTimer.AddCountupTimer()` (`0x82db3cc8`).
fn timer_add_countup(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let owner = world.closure;
    Ok(vec![int(world.timers.add(TimerKind::CountUp, 0.0, owner))])
}

/// `ChallengeTimer.Remove*Timer(id)` (`0x82daa3f8`).
fn timer_remove(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.timers.remove(get_int(call, 0));
    none()
}

/// `ChallengeTimer.ResetCountdownTimer(id, seconds)` (`0x82da5678`).
fn timer_reset(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.timers.reset(get_int(call, 0), get_float(call, 1));
    none()
}

/// `ChallengeTimer.GetCountdownTime`/`GetCountupTime(id)` (`0x82da8580`).
fn timer_get_time(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![Value::Number(world.timers.time(get_int(call, 0)))])
}

/// `ChallengeTimer.PauseTimer(id)` (`0x82da5700`).
fn timer_pause(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.timers.set_paused(get_int(call, 0), true);
    Ok(vec![Value::Nil])
}

/// `ChallengeTimer.UnpauseTimer(id)` (`0x82da5768`).
fn timer_unpause(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.timers.set_paused(get_int(call, 0), false);
    Ok(vec![Value::Nil])
}

// ---- Challenge ------------------------------------------------------------

/// `Challenge.WinChallenge(data, playerID)` (`0x828949a0`).
fn challenge_win(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(a) = activity_arg(lua, call, 0) {
        let key = world.activities.activities[a].key;
        signpost::win_challenge(world, key);
    }
    none()
}

/// `Challenge.CompleteChallenge(data, processResults = true)` (`0x82894af0`).
fn challenge_complete(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(a) = activity_arg(lua, call, 0) else { return none() };
    let process = get_bool_or(call, 1, true);
    activity::complete_challenge(world, a);
    if process {
        signpost::process_challenge_results(world);
    }
    // 82894cf8..82894d70: this runs even when result processing is skipped
    // or produces no progression changes (e.g. a personal-best HoM replay).
    // CompleteChallenge left AI release waiting for a prerequisite pass;
    // this pass lets its normal release sequence restore mission sign-ups.
    activity::reevaluate_prereq_activities(world);
    none()
}

/// `ChallengeSpeech.PlayCameramanSpeech(id)` (`0x82888f98`; the progression's
/// `PlayProgressionReminder` `0x82889040` branches to it):
/// `ChallengeAudioManager::PlayCameramanSpeech` (`0x82da1bc8`),
/// `PlayMissionSpeech(eSk8Character_CameraMan, id, 0)`.
pub(crate) fn speech_play_cameraman(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.output(Output::PlayNpcSpeech { character: presentation::CAMERA_MAN, id: get_int(call, 0), param: 0 });
    none()
}

/// `EMPTY_NATIVE` (`0x82784530`, `li r3, 0; blr`): the folded no-op several names bind.
fn empty_native(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    none()
}

// ---- NIS ------------------------------------------------------------------

/// `NIS.KillNIS()` (`0x8288bd58`): `NISManager::KillNIS` (`0x828a2f88`).
fn nis_kill(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.output(Output::KillNis);
    none()
}

// ---- SessionMarker -----------------------------------------------------------

/// `SessionMarker.EnableSetSessionMarker()` (`0x82dad510`): `cChallengeTeleportManager::
/// RemoveSetSessionMarkerOverride` (`0x82dabee0`) of the running activity.
fn session_marker_enable_set(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    if let Some(activity) = world.closure {
        teleport::remove_session_marker_override(world, activity, false);
    }
    none()
}

/// `SessionMarker.DisableSetSessionMarker()` (`0x82db0a88`): `AddSetSessionMarkerOverride`
/// (`0x82daefa8`).
fn session_marker_disable_set(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    if let Some(activity) = world.closure {
        teleport::add_session_marker_override(world, activity, false);
    }
    none()
}

/// `SessionMarker.EnableUseSessionMarker()` (`0x82dad5c8`): `RemoveUseSessionMarkerOverride`
/// (`0x82dabf78`).
fn session_marker_enable_use(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    if let Some(activity) = world.closure {
        teleport::remove_session_marker_override(world, activity, true);
    }
    none()
}

/// `SessionMarker.DisableUseSessionMarker()` (`0x82db0b40`): `AddUseSessionMarkerOverride`
/// (`0x82daf060`).
fn session_marker_disable_use(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    if let Some(activity) = world.closure {
        teleport::add_session_marker_override(world, activity, true);
    }
    none()
}

// ---- Debug ----------------------------------------------------------------

/// `Debug.DebugText`/`DebugTextTimed`/`RemoveTextPermanent` (`0x82886af0`/`0x82886be0`/
/// `0x82886df8`): `cChallengeDebugManager`'s text calls, empty in this build.
fn debug_text(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    none()
}

/// `Debug.DebugTextPermanent(text, x, y)` (`0x82886ce8`): `DrawText2DPermanent`
/// (`0x82db3b98`) creates no shape in this build and pushes its null.
fn debug_text_permanent(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::LightUserData(0)])
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("ChallengeAsset", "ActivateChallengeAssetLoadScreen", empty_native),
        ("ChallengeAsset", "DeactivateChallengeAssetLoadScreen", empty_native),
        ("Debug", "DebugText", debug_text),
        ("Debug", "DebugTextTimed", debug_text),
        ("Debug", "DebugTextPermanent", debug_text_permanent),
        ("Debug", "RemoveTextPermanent", debug_text),
        ("Pause", "DisablePauseMenuAndBackpack", disable_pause_menu),
        ("Pause", "EnablePauseMenuAndBackpack", enable_pause_menu),
        ("Pause", "PauseSimulation", pause_simulation),
        ("Pause", "UnpauseSimulation", unpause_simulation),
        ("HUD2", "ShowTrickDisplayContext", hud_show_trick_display_context),
        ("NIS", "KillNIS", nis_kill),
        ("SessionMarker", "EnableSetSessionMarker", session_marker_enable_set),
        ("SessionMarker", "DisableSetSessionMarker", session_marker_disable_set),
        ("SessionMarker", "EnableUseSessionMarker", session_marker_enable_use),
        ("SessionMarker", "DisableUseSessionMarker", session_marker_disable_use),
        ("Objectives", "GetTotalNumObjectives", objectives_get_total_num),
        ("Objectives", "GetAccumulatedPoints", objectives_get_accumulated_points),
        ("Objectives", "GetCurrentNumCompletedObjectives", objectives_get_current_num_completed),
        ("Objectives", "GetFractionDenominator", objectives_get_fraction_denominator),
        ("Objectives", "SetFractionNumerator", objectives_set_fraction_numerator),
        ("Objectives", "ChangeObjectivePointsRequirement", objectives_change_points_requirement),
        ("Objectives", "IsTrackedUniqueTrickGroup", objectives_is_tracked_unique_trick_group),
        ("Objectives", "GetNumObjectivesRequiresHighlight", objectives_get_num_requires_highlight),
        ("Objectives", "GetObjectiveFailureHALText", objectives_get_failure_hal_text),
        ("Objectives", "GetObjectiveNISPlaybackDefinition", objectives_get_nis_playback_definition),
        ("Objectives", "GetObjectiveHighlightDefinition", objectives_get_highlight_definition),
        ("Objectives", "AddPointsObjective", objectives_add_points_objective),
        ("HUD2", "CreateGroup", hud_create_group),
        ("HUD2", "GetGroupHash", hud_get_group_hash),
        ("HUD2", "SetupObjectiveHUDData", hud_setup_objective_data),
        ("HUD2", "SetActiveGroup", hud_set_active_group),
        ("HUD2", "Show", hud_show),
        ("HUD2", "DeleteGroup", hud_delete_group),
        ("HUD2", "ShowCentralMessage", hud_show_central_message),
        ("HUD2", "SetupDistanceHUDData", hud_setup_distance_data),
        ("HUD2", "ShowChallengeCompleteMessage", hud_show_challenge_complete_message),
        ("HUD2", "ShowChallengeFailedMessage", hud_show_challenge_failed_message),
        ("HUD2", "CreatePreFilmSetupHUD", hud_create_pre_film_setup),
        ("HUD2", "DeletePreFilmSetupHUD", hud_delete_pre_film_setup),
        ("HUD2", "ShowPreFilmSetupHUD", hud_show_pre_film_setup),
        ("HUD2", "ObjectiveSetModifiedPosition", hud_objective_set_modified_position),
        ("HUD2", "ShowSecurityHUD", hud_show_security),
        ("HUD2", "ShowPoolDrainHUD", hud_show_pool_drain),
        ("HUD2", "ShowUncapHUD", hud_show_uncap),
        ("HUD2", "TutorialInstructionSet", hud_tutorial_instruction_set),
        ("Online", "GetLocalPlayerID", online_get_local_player_id),
        ("AI", "SkaterDistanceFromLocation", ai_skater_distance_from_location),
        ("AI", "RepositionAI", ai_reposition),
        ("AI", "ClearTeleports", ai_clear_teleports),
        ("ChallengeSave", "GetSaveData", challenge_save_get),
        ("ChallengeSave", "SetSaveData", challenge_save_set),
        ("ChallengeSave", "GetOtherChallengeSaveData", challenge_save_get_other),
        ("ChallengeSave", "SetOtherChallengeSaveData", challenge_save_set_other),
        ("Teleport", "TeleportLocalPlayerToLocationName", teleport_local_player_to_location_name),
        ("Objectives", "MakeObjectives", objectives_make),
        ("Objectives", "MakeObjectivesForAI", objectives_make_for_ai),
        ("Objectives", "MakeObjectivesForPlayer", objectives_make_for_player),
        ("Objectives", "SetEvaluationStatus", objectives_set_evaluation_status),
        ("Objectives", "FailAllObjectiveStatus", objectives_fail_all),
        ("Objectives", "ForceObjectiveComplete", objectives_force_complete),
        ("Time", "GetVisualGameTick", time_get_visual_game_tick),
        ("Challenge", "IsChallengeAboutToStartDuringLoad", challenge_is_about_to_start_during_load),
        ("Challenge", "IsReplayingChallenge", challenge_is_replaying),
        ("Challenge", "LoseChallenge", challenge_lose),
        ("Challenge", "AbandonChallenge", challenge_abandon),
        ("Challenge", "SetIsGotoSignupAvailable", challenge_set_is_goto_signup_available),
        ("Challenge", "IsSigningup", challenge_is_signing_up),
        ("Challenge", "ClearVisualIndicatorsByType", empty_native),
        ("Challenge", "SetAllChallengeVisualIndicators", empty_native),
        ("Challenge", "RemoveAllChallengeVisualIndicators", empty_native),
        ("ChallengeManager", "SwitchChallengeCamera", challenge_manager_switch_challenge_camera),
        ("ChallengeSpeech", "PlayNPCSpeech", speech_play_npc),
        ("ChallengeSpeech", "PlaySkaterSpeech", speech_play_skater),
        ("ChallengeSpeech", "PlayNPCSpeechUsingAttribute", speech_play_npc_using_attribute),
        ("ChallengeSpeech", "PlayNPCSpeechUsingOverrideAttribute", speech_play_npc_using_override_attribute),
        ("ChallengeSpeech", "PlayNPCSpeechUsingDynamicCompetitor", speech_play_npc_using_dynamic_competitor),
        ("ChallengeSpeech", "PlayNPCSpeechUsingAttributeAndSkaterIndex", speech_play_npc_using_attribute_and_skater_index),
        ("ChallengeSpeech", "PlayFollowMeWarningSpeech", speech_play_follow_me_warning),
        ("Objectives", "ResetObjectives", objectives_reset_all),
        ("Objectives", "ResetObjective", objectives_reset_one),
        ("Objectives", "DeleteObjectiveGroup", objectives_delete_group),
        ("ChallengePresentation", "AddPresentationGroup", presentation_add_group),
        ("ChallengePresentation", "AddPresentationEvent", presentation_add_event),
        ("ChallengePresentation", "StartPresentation", presentation_start),
        ("ChallengePresentation", "RemoveAllPresentationGroups", presentation_remove_all),
        ("HOM", "EnableHOMVisuals", hom_enable_visuals),
        ("ChallengeAsset", "IsChallengeAssetLoadComplete", asset_is_load_complete),
        ("ChallengeAsset", "LoadChallengeAssets", asset_load),
        ("ChallengeAsset", "UnloadChallengeAssets", asset_unload),
        ("ChallengeAsset", "PreloadChallengeAssetsByName", asset_preload_by_name),
        ("Effects", "StartEffect", effects_start),
        ("Effects", "ResetEffect", effects_reset),
        ("ChallengeTimer", "AddCountdownTimer", timer_add_countdown),
        ("ChallengeTimer", "AddHUDCountdownTimer", timer_add_countdown),
        ("ChallengeTimer", "AddCountupTimer", timer_add_countup),
        ("ChallengeTimer", "RemoveHUDCountdownTimer", timer_remove),
        ("ChallengeTimer", "RemoveCountdownTimer", timer_remove),
        ("ChallengeTimer", "RemoveCountupTimer", timer_remove),
        ("ChallengeTimer", "ResetCountdownTimer", timer_reset),
        ("ChallengeTimer", "GetCountdownTime", timer_get_time),
        ("ChallengeTimer", "GetCountupTime", timer_get_time),
        ("ChallengeTimer", "PauseTimer", timer_pause),
        ("ChallengeTimer", "UnpauseTimer", timer_unpause),
        ("Challenge", "WinChallenge", challenge_win),
        ("Challenge", "CompleteChallenge", challenge_complete),
        ("Trigger", "ResetTriggerVolumeState", trigger_reset_volume_state),
        ("NIS", "EnterNISUsingDefinition", nis_enter_using_definition),
        ("NIS", "ResetNISPlayerIndexes", nis_reset_player_indexes),
        ("NIS", "PreloadNIS", nis_preload),
        ("NIS", "GetPhotoNISCount", nis_get_photo_count),
        ("NIS", "GetPhotoNISCountFromHighlightDefinition", nis_get_photo_count_from_highlight_definition),
        ("FrontEnd", "OpenPhotoSelection", frontend_open_photo_selection),
        ("NIS", "ClearPreloadedNIS", nis_clear_preloaded),
        ("ChallengeMusic", "PlayMusic", challenge_music_play),
        ("ChallengeMusic", "UpdateLapRaceMusic", challenge_music_update_lap_race),
        ("Map", "AddRaceGate", map_add_race_gate),
        ("Map", "AddRaceCheckPoint", map_add_race_check_point),
        ("Map", "AddRaceFinish", map_add_race_finish),
        ("Map", "RemoveRaceObject", map_remove_race_object),
        ("NIS", "SubmitNISPlayerIndex", nis_submit_player_index),
    ]
}

/// `NIS.EnterNISUsingDefinition(definition, transition = 0, holdCamera = false, enableSkip =
/// true, restoreStreamer = false)` (`0x828956e0`): a definition with a scene runs it through
/// `NISManager::RunBasicNISDefinition` (transitions 3 and 4 complete with 4, others with none);
/// without one, `cMsgNISChallengeNotify` with notification 1.
fn nis_enter_using_definition(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(definition) = attrib_location(&world.pointers, call.arg(0)) else { return none() };
    let transition = if (3..=4).contains(&get_int_or(call, 1, 0)) { 4 } else { 0 };
    let hold_camera = get_bool_or(call, 2, false);
    let skip_enabled = get_bool_or(call, 3, true);
    let restore_streamer = get_bool_or(call, 4, false);
    let scene = world.db.text_at(definition).unwrap_or_default().to_owned();
    if scene.is_empty() || is_skate2_scene_at_skate3_spot(world, &scene) {
        world.send(Event::NisChallengeNotify { notification: 1 });
        return none();
    }
    let (ignore_breadcrumb, location) = nis_definition_overrides(world, definition);
    world.output(Output::RunNisDefinition { scene, ignore_breadcrumb, transition, hold_camera, skip_enabled, location, restore_streamer });
    none()
}

/// What `RunBasicNISDefinition` (`0x828a8a70`) reads of a `tNISPlaybackDefinition` besides its
/// scene: `IgnoreTeleportToLastBreadcrumb` (`+0x30`) and, with `UseLocationOverride` (`+0x6`),
/// the `LocationOverride` position and direction (`+0x10`, `+0x20`). Its `RandomPlaybackSpeed`
/// and `RandomCameraTrack` leave the playback overrides at their defaults.
pub(super) fn nis_definition_overrides(world: &World, definition: super::attrib_db::Location) -> (bool, Option<[[f32; 3]; 2]>) {
    let flag = |offset: u32| world.db.bytes(definition.offset_by(offset), 1).is_some_and(|b| b[0] != 0);
    let location = if flag(6) { world.db.bytes(definition.offset_by(16), 32) } else { None }.map(|b| {
        let float = |at: usize| f32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]);
        [[float(0), float(4), float(8)], [float(16), float(20), float(24)]]
    });
    (flag(48), location)
}

/// `Trigger.ResetTriggerVolumeState(volume)` (`0x82886a28`).
fn trigger_reset_volume_state(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.output(Output::ResetTriggerVolumeState { volume: get_float(call, 0) });
    none()
}

/// `sModuleList` (`0x82ff0330`), then the managers bound by
/// `Lua::Manager::PerformLuaCBinding` (`0x8289b498`).
pub const MODULES: [&str; 54] = [
    "Debug",
    "Time",
    "Entity",
    "Trick",
    "Random",
    "Challenge",
    "Broadcasters",
    "Film",
    "Pause",
    "HUD2",
    "FrontEnd",
    "ChallengeSpeech",
    "ChallengeReplay",
    "ChallengeManager",
    "ChallengeSFX",
    "ChallengeMusic",
    "Online",
    "Skate",
    "NIS",
    "Map",
    "AI",
    "Crowd",
    "Contest",
    "Trigger",
    "Camera",
    "Transition",
    "HOM",
    "Service",
    "CreateASpot",
    "SequenceConnector",
    "ChallengePresentation",
    "ChallengeSave",
    "ChallengeProgression",
    "HighlightReel",
    "Objectives",
    "Callouts",
    "Crowds",
    "Effects",
    "ChallengeTimer",
    "Teleport",
    "SessionMarker",
    "VisualIndicators",
    "FollowTheLeader",
    "Race",
    "DMO",
    "DynamicHull",
    "ChallengeAsset",
    "LivingWorld",
    "Progression",
    "Achievement",
    "Award",
    "Unlock",
    "GameMode",
    "Telemetry",
];
