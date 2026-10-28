//! Natives of the objectives' Lua state
//! (`cChallengeObjectivesManager::PerformLuaCBinding` `0x82d4faf8`). They read
//! the objective being evaluated from the closure data
//! (`tObjectiveClosureData`).

use super::{
    binding::{get_bool_or, get_float, get_int, get_int_or, get_string, get_string_or, int},
    dmo,
    keys::{class, hash64},
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    mission::{HomBail, MissionFrame},
    objective_data::ObjectiveData,
    objectives::{self, Objective, PlayerState},
    system::World,
};

type R = LuaResult<Vec<Value>>;

fn objective(world: &World) -> Option<&Objective> {
    let closure = world.objective_closure?;
    let g = world.objectives.group_index(closure.group)?;
    world.objectives.groups[g].objectives.get(closure.objective)
}

/// The state of the evaluated objective's player, as `GetPhysOut(mPlayerID)` reads it: its own
/// where the challenge keeps every player's (`World::player_states`), else the local player's.
fn player(world: &World) -> &PlayerState {
    objective(world)
        .and_then(|o| world.player_states.iter().find(|(player, _)| *player == o.player))
        .map_or(&world.player, |(_, state)| state)
}

fn objective_mut(world: &mut World) -> Option<&mut Objective> {
    let closure = world.objective_closure?;
    let g = world.objectives.group_index(closure.group)?;
    world.objectives.groups[g].objectives.get_mut(closure.objective)
}

fn boolean(value: bool) -> R {
    Ok(vec![Value::Boolean(value)])
}

// ---- ObjectiveTrigger -----------------------------------------------------

/// `ObjectiveTrigger.EnteredVolume(id, name)` (`0x82d4ace8`): the name is
/// not read.
fn entered_volume(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let id = get_float(call, 0);
    boolean(objective(world).is_some_and(|o| o.trigger_volumes.iter().any(|&v| objectives::is_trigger_volume(v, id))))
}

/// `ObjectiveTrigger.EnteredVolumeInCurrentSequence(id, name)` (`0x82d4ad78`).
fn entered_volume_in_sequence(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let id = get_float(call, 0);
    boolean(objective(world).is_some_and(|o| o.sequence_volumes.iter().any(|&v| objectives::is_trigger_volume(v, id))))
}

/// `ObjectiveTrigger.IsInVolume(id, name)` (`0x82d4c478`): a trigger of the
/// challenge group the player currently collides with (by global or local id).
fn is_in_volume(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let id = get_float(call, 0);
    boolean(
        player(world)
            .colliding_triggers
            .iter()
            .any(|&(guid, local)| objectives::is_trigger_volume(guid, id) || objectives::is_trigger_volume(local, id)),
    )
}

// ---- ObjectiveLocomotion (PhysOut_Scoring2 / PhysOut_SkateboardMotion) ------

fn is_on_board(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(player(world).onboard_or_hippy_jumping)
}

fn is_off_board(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(player(world).offboard_and_not_hippy_jumping)
}

/// `WentOnBoard` (`0x82d48e98`).
fn went_on_board(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(player(world).was_offboard_and_not_hippy_jumping && player(world).onboard_or_hippy_jumping)
}

/// `WentOffBoard` (`0x82d48f88`).
fn went_off_board(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(player(world).was_onboard_or_hippy_jumping && player(world).offboard_and_not_hippy_jumping)
}

/// `IsStoppped` (`0x82d49078`): surface speed at or under
/// `kSkaterStopThreshold`.
fn is_stopped(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(player(world).surface_speed <= world.stop_threshold)
}

fn is_moving(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(player(world).surface_speed > world.stop_threshold)
}

fn get_speed(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Number(player(world).surface_speed)])
}

fn is_on_ground(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(player(world).on_ground)
}

fn is_in_air(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(player(world).in_air)
}

fn is_in_sequence(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(player(world).in_sequence)
}

/// `SequenceCompleted` (`0x82d48c08`): completed successfully and not by a
/// teleport (`FILTERED_PHYSICS_STATE_TELEPORT`).
fn sequence_completed(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(player(world).sequence_completed_successfully && !player(world).teleporting)
}

/// `GetNumPushes` (`0x82d48a58`).
fn get_num_pushes(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    #[expect(clippy::cast_precision_loss, reason = "PushInt")]
    Ok(vec![Value::Number(objective(world).map_or(0, |o| o.num_pushes) as f32)])
}

// ---- ObjectiveHOM -----------------------------------------------------------

/// `GetWipeoutDuration` (`0x82d477b0`).
fn get_wipeout_duration(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Number(player(world).moving_wipeout_duration)])
}

// ---- Wipeout ----------------------------------------------------------------

/// `IsWipingOut` (`0x82d45b20`): `PhysOut_Filtered::mFilteredPhysicsState` is
/// `FILTERED_PHYSICS_STATE_WIPEOUT` (4).
fn is_wiping_out(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(player(world).wiping_out)
}

// ---- Manual -----------------------------------------------------------------

/// `IsManualing` (`0x82d452e8`): `PhysOut_Scoring2::mManualLength > 0`.
fn is_manualing(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(player(world).manual_length > 0.0)
}

/// `NoseManualTime` (`0x82d45668`): `mManualTime` while the current trick
/// animation's `mIsNoseManual` bit (27) is set, else 0.
fn nose_manual_time(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let time = if (player(world).anim_trick_bits >> 27) & 1 == 1 { player(world).manual_time } else { 0.0 };
    Ok(vec![Value::Number(time)])
}

/// `TailManualTime` (`0x82d45940`): `mManualTime` while `mIsTailManual` (bit
/// 26) is set, else 0.
fn tail_manual_time(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let time = if (player(world).anim_trick_bits >> 26) & 1 == 1 { player(world).manual_time } else { 0.0 };
    Ok(vec![Value::Number(time)])
}

// ---- Tricks -----------------------------------------------------------------

/// `DidSpecificTrick(name)` (`0x82d4ae08`) ->
/// `cChallengeObjective::DidSpecificTrick` (`0x82d4a558`): the 64-bit
/// `StringHash64` of the name among the tricks the objective saw complete
/// while active.
fn did_specific_trick(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let key = hash64(&get_string(call, 0).unwrap_or_default());
    boolean(objective(world).is_some_and(|o| o.completed_tricks.contains(&key)))
}

/// `DidSpecificTrickThisFrame(name)` (`0x82d46dc8`): a valid, non-tap entry of
/// this frame's `mTricksEnded` whose 64-bit `mTrickAttribKey` is
/// `Attrib::StringToKey(name)`.
fn did_specific_trick_this_frame(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let key = hash64(&get_string(call, 0).unwrap_or_default());
    boolean(player(world).tricks_ended.contains(&key))
}

/// `DidSpecificTrickGroup(name)` (`0x82d4ae98`) ->
/// `cChallengeObjective::DidSpecificTrickGroup` (`0x82d4a4e8`): a trick the
/// objective saw complete while active.
fn did_specific_trick_group(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = hash64(&get_string(call, 0).unwrap_or_default());
    let tricks: Vec<u64> = objective(world).map(|o| o.completed_tricks.iter().copied().collect()).unwrap_or_default();
    let db = std::sync::Arc::clone(&world.db);
    boolean(tricks.into_iter().any(|t| world.trick_groups.is_trick_in_group(&db, t, group)))
}

/// `DidSpecificTrickGroupThisFrame(name)` (`0x82d46f18`): a trick of this
/// frame's `mTricksEnded`.
fn did_specific_trick_group_this_frame(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = hash64(&get_string(call, 0).unwrap_or_default());
    let tricks = player(world).tricks_ended.clone();
    let db = std::sync::Arc::clone(&world.db);
    boolean(tricks.into_iter().any(|t| world.trick_groups.is_trick_in_group(&db, t, group)))
}

// ---- ObjectivePoints ----------------------------------------------------------

/// `ObjectivePoints.GetSequencePoints()` (`0x82d46018`): `mLastSequencePoints`.
fn get_sequence_points(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(objective(world).map_or(0, |o| o.last_sequence_points.cast_signed()))])
}

/// `ObjectivePoints.GetAccumulatedPoints()` (`0x82d460b0`): the objective's `mTotalPoints`.
fn get_accumulated_points(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(objective(world).map_or(0, |o| o.total_points.cast_signed()))])
}

/// `ObjectivePoints.GetGroupAccumulatedPoints()` (`0x82d46148`): the closure group's
/// `GetAccumulatedPoints`.
fn get_group_accumulated_points(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let points = world.objective_closure.map_or(0, |c| objectives::accumulated_points(world, c.group));
    Ok(vec![int(points.cast_signed())])
}

/// `ObjectivePoints.IsSequenceCompletePoints()` (`0x82d461e8`): the closure's
/// `IsSequenceComplete` (+8).
fn is_sequence_complete_points(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(world.objective_closure.is_some_and(|c| c.is_sequence_complete))
}

/// `ObjectivePoints.GetGroupPointsRequirement()` (`0x82d462e8`): the closure group's
/// `PointsRequirement` (+0x4C).
fn get_group_points_requirement(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let group = world.objective_closure.and_then(|c| world.objectives.group_index(c.group));
    Ok(vec![int(group.map_or(0, |g| world.objectives.groups[g].points_requirement.cast_signed()))])
}

/// `ObjectivePoints.IsSequenceNewlyCompletePoints()` (`0x82d46250`): sequence points
/// (`IsSequenceComplete`, +8) for an objective the pass completed
/// (`ObjectiveUtils::IsHandlingObjectiveNewlyCompletedObjective` `0x82d43f70`, +9).
fn is_sequence_newly_complete_points(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(world.objective_closure.is_some_and(|c| c.is_sequence_complete && c.is_newly_completed))
}

/// `ObjectivePoints.AwardSequencePointsImmediately(award = false)` (`0x82d46380`): the
/// objective's `mAwardSequenceCompletePointsImmediately`, which its completion test then spends.
fn award_sequence_points_immediately(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let award = get_bool_or(call, 0, false);
    if let Some(o) = objective_mut(world) {
        o.award_sequence_points_immediately = award;
    }
    Ok(Vec::new())
}

// ---- Grind ------------------------------------------------------------------

/// `Grind.DidSpecificGrind(id)` (`0x82d464a8`) -> `cChallengeObjective::DidSpecificGrind`
/// (`0x82d43fe0`): a completed grind's spline ID as a float equals the id.
#[expect(clippy::cast_precision_loss, reason = "(float)_u64tod")]
#[expect(clippy::float_cmp, reason = "the native compares exactly")]
fn did_specific_grind(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let id = get_float(call, 0);
    boolean(objective(world).is_some_and(|o| o.completed_grinds.iter().any(|&spline| spline as f64 as f32 == id)))
}

/// `Grind.GetLastGrindLength()` (`0x82d46538`): `mGrindLength`.
fn get_last_grind_length(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Number(objective(world).map_or(0.0, |o| o.grind_length))])
}

/// `Grind.GetTotalGrindLength()` (`0x82d465a8`): `mTotalGrindLength`.
fn get_total_grind_length(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Number(objective(world).map_or(0.0, |o| o.total_grind_length))])
}

/// `Grind.GetMaxGrindLengthInSequence()` (`0x82d46618`).
fn get_max_grind_length_in_sequence(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Number(objective(world).map_or(0.0, |o| o.max_grind_length_in_sequence))])
}

/// `Grind.GetTotalGrindLengthInSequence()` (`0x82d46688`).
fn get_total_grind_length_in_sequence(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Number(objective(world).map_or(0.0, |o| o.total_grind_length_in_sequence))])
}

// ---- DMOs -------------------------------------------------------------------

/// `AddDMOVolumeTester(luaID)` (`0x82d4e4f0`): a trigger proxy for the DMO,
/// recorded on the objective (`AddDMOProxyID` `0x82d4e450`).
fn add_dmo_volume_tester(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let proxy = dmo::add_trigger_proxy(world, get_float(call, 0));
    if proxy != -1
        && let Some(o) = objective_mut(world)
        && !o.dmo_proxy_ids.contains(&proxy)
    {
        o.dmo_proxy_ids.push(proxy);
    }
    Ok(vec![int(proxy)])
}

/// `IsDMOInVolume(proxy, volumeID, name)` (`0x82d45d50`).
fn is_dmo_in_volume(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let proxy = get_int(call, 0);
    let volume = get_float(call, 1);
    boolean(dmo::is_in_volume(world, proxy, volume))
}

/// `IsDMOTesterReady(proxy)` (`0x82d45ca0`): the interrogater's answer is
/// always true.
fn is_dmo_tester_ready(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    boolean(true)
}

/// `ReleaseDMOVolumeTester(proxy)` (`0x82d4d368`) -> `RemoveDMOProxyID`
/// (`0x82d4d230`) with release.
fn release_dmo_volume_tester(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let proxy = get_int_or(call, 0, -1);
    if proxy != -1 {
        if let Some(o) = objective_mut(world)
            && let Some(at) = o.dmo_proxy_ids.iter().position(|&p| p == proxy)
        {
            o.dmo_proxy_ids.remove(at);
        }
        dmo::release_trigger_proxy(world, proxy);
    }
    Ok(Vec::new())
}

/// `IsGrabbingDMO()` (`0x82d45bf8`): `PhysOut_OffBoard::mIsGrabbingObject`.
fn is_grabbing_dmo(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(player(world).grabbing_object)
}

// ---- ObjectiveSessionMarker -------------------------------------------------

fn just_set_session_marker(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(objective(world).is_some_and(|o| o.just_set_session_marker))
}

fn just_used_session_marker(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(objective(world).is_some_and(|o| o.just_used_session_marker))
}

// ---- ObjectiveGameInfo / ObjectiveInfo ---------------------------------------

/// `ObjectiveGameInfo.SetFractionValue(value = 0)` (`0x82d48960`): the
/// objective's `mFractionNumerator` (+0x504), marking it dirty (+0x628).
fn set_fraction_value(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let value = get_int_or(call, 0, 0);
    if let Some(o) = objective_mut(world) {
        o.fraction_numerator = value;
        o.dirty = true;
    }
    Ok(Vec::new())
}

/// `ObjectiveGameInfo.GetFractionValue()` (`0x82d489e8`).
fn get_fraction_value(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(objective(world).map_or(0, |o| o.fraction_numerator))])
}

/// `ObjectiveInfo.GetTotalNumCompletedObjectives(includeOPO = true,
/// includeFractional = true)` (`0x82d494c8`) through
/// `cChallengeObjectiveGroup::GetNumCompletedObjectives` (`0x82d43e10`):
/// the closure group's complete objectives, its points objective only with
/// the first flag and those whose fraction denominator is above 1 only with
/// the second.
fn get_total_num_completed_objectives(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let include_opo = get_bool_or(call, 0, true);
    let include_fractional = get_bool_or(call, 1, true);
    let group = world.objective_closure.and_then(|c| world.objectives.group_index(c.group)).map(|g| &world.objectives.groups[g]);
    let count = group.map_or(0, |g| {
        g.objectives
            .iter()
            .enumerate()
            .filter(|(_, o)| o.state == objectives::Status::Complete)
            .filter(|&(i, o)| {
                if g.points_objective == Some(i) {
                    include_opo
                } else {
                    let denominator = world
                        .db
                        .attribute(class::CHALLENGE_OBJECTIVE, o.key, objectives::field::FRACTION_DENOMINATOR)
                        .and_then(|a| a.i16(0))
                        .unwrap_or(0);
                    denominator <= 1 || include_fractional
                }
            })
            .count()
    });
    Ok(vec![int(i32::try_from(count).unwrap_or(i32::MAX))])
}

/// `ObjectiveInfo.NumUniqueTrickDone()` (`0x82d49558`): the closure group's completed unique
/// tricks, plus the sequence's one not added to them yet.
fn num_unique_trick_done(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let count = world.objective_closure.map_or(0, |closure| objectives::num_unique_tricks_done(world, closure.group));
    Ok(vec![int(i32::try_from(count).unwrap_or(i32::MAX))])
}

// ---- the objective's records and its skater's output --------------------------

/// `eAnimGestureNames_Count`: no gesture.
const GESTURE_NONE: i32 = 35;
/// `eScorableID_coffin`.
const SCORABLE_COFFIN: i32 = 60;

fn data(world: &World) -> Option<&ObjectiveData> {
    objective(world).map(|o| &o.data)
}

/// `ISkaterManager::FindByIndex` of the objective's player, then `IActor::GetPhysOut`: that
/// skater's completed output this update. The native asserts the skater exists.
fn objective_frame(world: &World) -> Option<&MissionFrame> {
    let player = objective(world)?.player;
    world.skaters.iter().find(|frame| frame.player == player)
}

/// The skater's `PhysOut_Scoring2::mHoM_BailData` (`+0x3d70`) and its `mScore`.
fn hom_bail(world: &World) -> Option<(u32, HomBail)> {
    objective_frame(world).map(|frame| (frame.hom_score, frame.hom_bail))
}

fn count(n: usize) -> R {
    Ok(vec![int(i32::try_from(n).unwrap_or(i32::MAX))])
}

/// `Gesture.DidAGesture()` (`0x82d486f8`): `mGestureData.mUniqueGestures` is not empty.
fn did_a_gesture(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(data(world).is_some_and(|d| !d.unique_gestures.is_empty()))
}

/// `Gesture.GetNumUniqueGesturesDone()` (`0x82d48780`).
fn get_num_unique_gestures_done(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    count(data(world).map_or(0, |d| d.unique_gestures.len()))
}

/// `Gesture.IsGesturing()` (`0x82d487f8`): the skater's `mCurrentGesture` is not
/// `eAnimGestureNames_Count`.
fn is_gesturing(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(objective_frame(world).is_some_and(|frame| frame.current_gesture != GESTURE_NONE))
}

/// `Grind.IsGrinding()` (`0x82d466f8`): the skater's `PhysOut_Filtered::mIsGrinding`.
fn is_grinding(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(objective_frame(world).is_some_and(|frame| frame.filtered_grinding))
}

/// `Grind.IsGrindingGUID(id)` (`0x82d467c8`): grinding a spline whose pathed or local GUID, each
/// `(float)(double)`, is the id.
fn is_grinding_guid(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let id = get_float(call, 0);
    boolean(objective_frame(world).is_some_and(|frame| {
        frame.filtered_grinding
            && (objectives::is_trigger_volume(frame.filtered_grind_pathed_guid, id)
                || objectives::is_trigger_volume(frame.filtered_grind_local_guid, id))
    }))
}

/// `ObjectiveAchievements.DidAchievement(type, threshold)` (`0x82d495e8`): a big-trick
/// achievement of the type at the threshold or above.
fn did_big_trick_achievement(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let (achievement_type, threshold) = (get_int(call, 0), get_int(call, 1));
    boolean(data(world).is_some_and(|d| d.big_trick_achievements.iter().any(|&(t, level)| t == achievement_type && level >= threshold)))
}

/// `ObjectiveAir.GetAirTime()` (`0x82d46908`): `mAirData.mLastAirTime`.
fn get_air_time(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Number(data(world).map_or(0.0, |d| d.last_air_time))])
}

/// `ObjectiveAir.GetTotalAirTime()` (`0x82d46978`).
fn get_total_air_time(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Number(data(world).map_or(0.0, |d| d.total_air_time))])
}

/// `ObjectiveAir.GetAirHeight()` (`0x82d469e8`): `mLastAirHeight`.
fn get_air_height(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Number(data(world).map_or(0.0, |d| d.last_air_height))])
}

/// `ObjectiveAir.GetDegreesOfSpin()` (`0x82d46ac8`).
fn get_degrees_of_spin(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Number(data(world).map_or(0.0, |d| d.degrees_of_spin))])
}

/// `ObjectiveAir.GetNumBodyFlips()` (`0x82d46b38`), pushed as a float.
#[expect(clippy::cast_precision_loss, reason = "the native's (float) of the count")]
fn get_num_body_flips(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Number(data(world).map_or(0, |d| d.num_body_flips) as f32)])
}

/// `ObjectiveAir.GetNumSkatersAiredOver()` (`0x82d46bb8`).
fn get_num_skaters_aired_over(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(data(world).map_or(0, |d| d.num_skaters_aired_over))])
}

/// `ObjectiveContextual.DidAchievement(entityType, achievementType)` (`0x82d49778`).
fn did_contextual_achievement(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let (entity_type, achievement_type) = (get_int(call, 0), get_int(call, 1));
    boolean(data(world).is_some_and(|d| d.contextual.iter().any(|c| c.achievement_type == achievement_type && c.entity_type == entity_type)))
}

/// `ObjectiveContextual.DidAchievementOfGroup(group, achievementType)` (`0x82d496b0`).
fn did_contextual_achievement_of_group(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let (group, achievement_type) = (get_int(call, 0), get_int(call, 1));
    boolean(data(world).is_some_and(|d| d.contextual.iter().any(|c| c.achievement_type == achievement_type && c.group == group)))
}

/// `ObjectiveContextual.GetNumAchievements(entityType, achievementType)` (`0x82d498f0`).
fn get_num_contextual_achievements(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let (entity_type, achievement_type) = (get_int(call, 0), get_int(call, 1));
    count(data(world).map_or(0, |d| d.contextual.iter().filter(|c| c.achievement_type == achievement_type && c.entity_type == entity_type).count()))
}

/// `ObjectiveGameInfo.GetNumSkaters()` (`0x82d488d8`): `IPlayerInfo::GetNumPlayers`
/// (`PlayersBackEnd` `0x8255aa58`).
fn get_num_skaters(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(world.players.as_deref().map_or(0, |players| players.num_players()))])
}

/// `ObjectiveHOM.GetScore()` (`0x82d47528`): `mHoM_BailData.mScore`.
fn hom_get_score(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(hom_bail(world).map_or(0, |(score, _)| score).cast_signed())])
}

/// `ObjectiveHOM.GetDamageScore()` (`0x82d47af0`): `mBoneScore`.
fn hom_get_damage_score(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(hom_bail(world).map_or(0, |(_, bail)| bail.bone_score).cast_signed())])
}

/// `ObjectiveHOM.GetBonusScore()` (`0x82d47a20`): `mBonusScore`.
fn hom_get_bonus_score(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(hom_bail(world).map_or(0, |(_, bail)| bail.bonus_score).cast_signed())])
}

/// `ObjectiveHOM.GetRotationScore()` (`0x82d47950`): `mRotationScore`.
fn hom_get_rotation_score(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(hom_bail(world).map_or(0, |(_, bail)| bail.rotation_score).cast_signed())])
}

/// `ObjectiveHOM.GetNumBodyTweaksDone()` (`0x82d47ef8`): `mNumBodyTweaksPerformed`.
fn hom_get_num_body_tweaks_done(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(hom_bail(world).map_or(0, |(_, bail)| bail.num_body_tweaks_performed).cast_signed())])
}

/// `ObjectiveHOM.DidSpecificBodyTweakThisFrame(tweak)` (`0x82d47fc8`): `mCurrentBodyTweak` is the
/// tweak.
fn hom_did_specific_body_tweak_this_frame(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let tweak = get_int(call, 0);
    boolean(hom_bail(world).is_some_and(|(_, bail)| bail.current_body_tweak == tweak))
}

/// `ObjectiveHOM.GetWipeoutSpeed()` (`0x82d47880`): `mBailSpeed`.
fn hom_get_wipeout_speed(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Number(hom_bail(world).map_or(0.0, |(_, bail)| bail.bail_speed))])
}

/// `ObjectiveHOM.GetMaxAirDrop()` (`0x82d475f8`): `mMaxAirDrop.mDropHeight`.
fn hom_get_max_air_drop(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Number(hom_bail(world).map_or(0.0, |(_, bail)| bail.max_air_drop_height))])
}

/// `ObjectiveHOM.GetMaxAirTime()` (`0x82d476c8`): `mAccumulatedAirDrop.mDurationInFrames` over 60.
#[expect(clippy::cast_precision_loss, reason = "the native's (float) of the frame count")]
fn hom_get_max_air_time(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let frames = hom_bail(world).map_or(0, |(_, bail)| bail.accumulated_air_drop_frames);
    Ok(vec![Value::Number(frames as f32 * 0.016_666_668)])
}

/// `ObjectiveHOM.GetNumPedestriansHit()` (`0x82d48098`): `mEntitySets[0].mCount`.
fn hom_get_num_pedestrians_hit(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(hom_bail(world).map_or(0, |(_, bail)| bail.entity_counts[0]).cast_signed())])
}

/// `ObjectiveHOM.GetNumVehiclesHit()` (`0x82d48240`): `mEntitySets[1].mCount`.
fn hom_get_num_vehicles_hit(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(hom_bail(world).map_or(0, |(_, bail)| bail.entity_counts[1]).cast_signed())])
}

/// `ObjectiveHOM.GetNumDMOsHit()` (`0x82d483e8`): `mEntitySets[2].mCount`.
fn hom_get_num_dmos_hit(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(hom_bail(world).map_or(0, |(_, bail)| bail.entity_counts[2]).cast_signed())])
}

/// `ObjectiveHOM.GetBoneState(bone)` (`0x82d47bc0`): the bone's `mDamageLevel`; a bone past 24 is
/// the native's assertion, read past the bones, and 0 here.
fn hom_get_bone_state(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let bone = get_int(call, 0);
    let level = hom_bail(world).and_then(|(_, bail)| usize::try_from(bone).ok().and_then(|i| bail.bone_damage_levels.get(i).copied()));
    Ok(vec![int(level.unwrap_or(0))])
}

/// `ObjectiveHOM.GetNumBonesAtDamageLevel(level)` (`0x82d47cb8`): the 25 bones at the level.
fn hom_get_num_bones_at_damage_level(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let level = get_int(call, 0);
    count(hom_bail(world).map_or(0, |(_, bail)| bail.bone_damage_levels.iter().filter(|&&l| l == level).count()))
}

/// `ObjectiveHOM.GetNumBonesGTOEDamageLevel(level)` (`0x82d47dd8`): the bones at the level or above.
fn hom_get_num_bones_gtoe_damage_level(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let level = get_int(call, 0);
    count(hom_bail(world).map_or(0, |(_, bail)| bail.bone_damage_levels.iter().filter(|&&l| l >= level).count()))
}

/// `ObjectiveHOM.IsLastBailTop10()` (`0x82d48590`): `IStatsBackEnd::IsLastBailATopBail`
/// (`StatsBackEnd` `0x828070b8`), the flag `StatsBackEnd::UpdateBailStats` sets for a bail entering
/// the top bails. This is the stored result of the last stats update, not a
/// fresh comparison of the currently accumulating score.
fn hom_is_last_bail_top10(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(world.hom_stats.lock().expect("HoM stats lock").last_top_bail)
}

/// `ObjectiveLocation.IsInLocation(name = "")` (`0x82d499a0`): the manager's `CachedLocationKey`
/// is the name's lower-case key (`Attrib::StringToLowerCaseKey`).
fn is_in_location(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let name = get_string_or(call, 0, "");
    boolean(world.objectives.cached_location_key == Some(hash64(&name.to_ascii_lowercase())))
}

/// `ObjectiveRound.GetRoundScore(round)` (`0x82d46c28`): `mRoundScore[round - 1]`; a round outside 1
/// to 5 is the native's assertion, read past the scores, and 0 here.
fn get_round_score(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let index = usize::try_from(get_int(call, 0) - 1).ok().filter(|&i| i < 5);
    Ok(vec![int(index.and_then(|i| data(world).map(|d| d.round_score[i])).unwrap_or(0))])
}

/// `ObjectiveRound.GetRoundRank(round)` (`0x82d46cc0`): `mRoundRank[round - 1]`.
fn get_round_rank(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let index = usize::try_from(get_int(call, 0) - 1).ok().filter(|&i| i < 5);
    Ok(vec![int(index.and_then(|i| data(world).map(|d| d.round_rank[i])).unwrap_or(0))])
}

/// `ObjectiveRound.GetOverallRank()` (`0x82d46d58`): `mOverallRank`.
fn get_overall_rank(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(data(world).map_or(0, |d| d.overall_rank))])
}

/// `ObjectiveTrigger.EnteredVolumesInOrder(id, ...)` (`0x82d4e5e8`): the leading number arguments,
/// then `cChallengeObjective::HasEnteredVolumesInOrder` (`0x82d4a7d8`): the objective's entered
/// volumes, in order, pass through the ids in turn (no ids passes at once).
fn entered_volumes_in_order(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let ids: Vec<f32> = (0..).map_while(|i| match call.arg(i) {
        Value::Number(n) => Some(*n),
        _ => None,
    })
    .collect();
    if ids.is_empty() {
        return boolean(true);
    }
    let volumes = objective(world).map_or_else(Vec::new, |o| o.trigger_volumes.clone());
    let mut at = 0;
    for volume in volumes {
        if objectives::is_trigger_volume(volume, ids[at]) {
            at += 1;
        }
        if at == ids.len() {
            return boolean(true);
        }
    }
    boolean(false)
}

/// `Reset.ClearContextualData()` (`0x82d44fd8`): the objective's `mContextualDataSet` empties.
fn clear_contextual_data(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    if let Some(o) = objective_mut(world) {
        o.data.contextual.clear();
    }
    Ok(Vec::new())
}

/// `Reset.ResetAllObjectives(resetCumulativeData = true)` (`0x82d4da68`): the closure group's
/// `ResetObjectives`.
fn reset_all_objectives(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let reset_cumulative = get_bool_or(call, 0, true);
    if let Some(closure) = world.objective_closure {
        objectives::reset_objectives(world, closure.group, reset_cumulative);
    }
    Ok(Vec::new())
}

/// The `Attrib::StringHash64` of every value of a table argument (`lua_next` order).
fn table_keys(lua: &Lua<World>, value: &Value) -> Vec<u64> {
    let Value::Table(table) = value else { return Vec::new() };
    let mut keys = Vec::new();
    let mut key = Value::Nil;
    while let Ok(Some((k, v))) = lua.table(*table).next(&key) {
        let name = match &v {
            Value::String(s) => String::from_utf8_lossy(s.as_bytes()).into_owned(),
            Value::Number(n) => super::lua::number::number_to_string(*n),
            _ => String::new(),
        };
        keys.push(hash64(&name));
        key = k;
    }
    keys
}

/// `Tricks.DidSpecificTrickSequence(groups, ignoreGroups = nil)` (`0x82d4e6e0`):
/// `cChallengeObjective::DidSpecificTrickSequence` (`0x82d4a630`): the objective's completed
/// tricks, in order, pass through the trick groups in turn; a trick of none of the ignored groups
/// that is not the next group's starts over. No groups passes at once.
fn did_specific_trick_sequence(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if !matches!(call.arg(0), Value::Table(_)) {
        return Err(lua.error("Tricks.DidSpecificTrickSequence: expected a table".to_owned()));
    }
    let sequence = table_keys(lua, call.arg(0));
    let ignore = table_keys(lua, call.arg(1));
    if sequence.is_empty() {
        return boolean(true);
    }
    let tricks: Vec<u64> = objective(world).map_or_else(Vec::new, |o| o.completed_tricks.iter().copied().collect());
    let db = std::sync::Arc::clone(&world.db);
    let mut at = 0;
    for trick in tricks {
        if world.trick_groups.is_trick_in_group(&db, trick, sequence[at]) {
            at += 1;
        } else if !ignore.iter().any(|&group| world.trick_groups.is_trick_in_group(&db, trick, group)) {
            at = 0;
        }
        if at == sequence.len() {
            return boolean(true);
        }
    }
    boolean(false)
}

/// `Tricks.GetNumTricksDoneInTrickGroup(group)` (`0x82d4af28`):
/// `cChallengeObjective::GetNumTricksDoneInTrickGroup` (`0x82d4a5c0`): the completed tricks in the
/// trick group.
fn get_num_tricks_done_in_trick_group(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = hash64(&get_string(call, 0).unwrap_or_default());
    let tricks: Vec<u64> = objective(world).map_or_else(Vec::new, |o| o.completed_tricks.iter().copied().collect());
    let db = std::sync::Arc::clone(&world.db);
    count(tricks.into_iter().filter(|&trick| world.trick_groups.is_trick_in_group(&db, trick, group)).count())
}

/// `Tricks.IsInCoffin()` (`0x82d47080`): a valid active trick of the skater is the coffin.
fn is_in_coffin(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    boolean(objective_frame(world).is_some_and(|frame| frame.active_tricks.iter().any(|&(valid, id)| valid && id == SCORABLE_COFFIN)))
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("ObjectiveTrigger", "EnteredVolume", entered_volume),
        ("ObjectiveTrigger", "EnteredVolumeInCurrentSequence", entered_volume_in_sequence),
        ("ObjectiveTrigger", "IsInVolume", is_in_volume),
        ("ObjectiveLocomotion", "IsOnBoard", is_on_board),
        ("ObjectiveLocomotion", "IsOffBoard", is_off_board),
        ("ObjectiveLocomotion", "WentOnBoard", went_on_board),
        ("ObjectiveLocomotion", "WentOffBoard", went_off_board),
        ("ObjectiveLocomotion", "IsStoppped", is_stopped),
        ("ObjectiveLocomotion", "IsMoving", is_moving),
        ("ObjectiveLocomotion", "GetSpeed", get_speed),
        ("ObjectiveLocomotion", "IsOnGround", is_on_ground),
        ("ObjectiveLocomotion", "IsInAir", is_in_air),
        ("ObjectiveLocomotion", "IsInSequence", is_in_sequence),
        ("ObjectiveLocomotion", "GetNumPushes", get_num_pushes),
        ("ObjectiveLocomotion", "SequenceCompleted", sequence_completed),
        ("ObjectiveHOM", "GetWipeoutDuration", get_wipeout_duration),
        ("Wipeout", "IsWipingOut", is_wiping_out),
        ("Manual", "IsManualing", is_manualing),
        ("Manual", "NoseManualTime", nose_manual_time),
        ("Manual", "TailManualTime", tail_manual_time),
        ("Tricks", "DidSpecificTrick", did_specific_trick),
        ("Tricks", "DidSpecificTrickThisFrame", did_specific_trick_this_frame),
        ("Tricks", "DidSpecificTrickGroup", did_specific_trick_group),
        ("Tricks", "DidSpecificTrickGroupThisFrame", did_specific_trick_group_this_frame),
        ("ObjectivePoints", "GetSequencePoints", get_sequence_points),
        ("ObjectivePoints", "GetAccumulatedPoints", get_accumulated_points),
        ("ObjectivePoints", "GetGroupAccumulatedPoints", get_group_accumulated_points),
        ("ObjectivePoints", "IsSequenceCompletePoints", is_sequence_complete_points),
        ("ObjectivePoints", "GetGroupPointsRequirement", get_group_points_requirement),
        ("ObjectivePoints", "IsSequenceNewlyCompletePoints", is_sequence_newly_complete_points),
        ("ObjectivePoints", "AwardSequencePointsImmediately", award_sequence_points_immediately),
        ("Grind", "DidSpecificGrind", did_specific_grind),
        ("Grind", "GetLastGrindLength", get_last_grind_length),
        ("Grind", "GetTotalGrindLength", get_total_grind_length),
        ("Grind", "GetMaxGrindLengthInSequence", get_max_grind_length_in_sequence),
        ("Grind", "GetTotalGrindLengthInSequence", get_total_grind_length_in_sequence),
        ("DMOs", "AddDMOVolumeTester", add_dmo_volume_tester),
        ("DMOs", "IsDMOInVolume", is_dmo_in_volume),
        ("DMOs", "IsDMOTesterReady", is_dmo_tester_ready),
        ("DMOs", "ReleaseDMOVolumeTester", release_dmo_volume_tester),
        ("DMOs", "IsGrabbingDMO", is_grabbing_dmo),
        ("ObjectiveSessionMarker", "JustSetSessionMarker", just_set_session_marker),
        ("ObjectiveSessionMarker", "JustUsedSessionMarker", just_used_session_marker),
        ("ObjectiveGameInfo", "SetFractionValue", set_fraction_value),
        ("ObjectiveGameInfo", "GetFractionValue", get_fraction_value),
        ("ObjectiveInfo", "GetTotalNumCompletedObjectives", get_total_num_completed_objectives),
        ("ObjectiveInfo", "NumUniqueTrickDone", num_unique_trick_done),
        ("Gesture", "DidAGesture", did_a_gesture),
        ("Gesture", "GetNumUniqueGesturesDone", get_num_unique_gestures_done),
        ("Gesture", "IsGesturing", is_gesturing),
        ("Grind", "IsGrinding", is_grinding),
        ("Grind", "IsGrindingGUID", is_grinding_guid),
        ("ObjectiveAchievements", "DidAchievement", did_big_trick_achievement),
        ("ObjectiveAir", "GetAirTime", get_air_time),
        ("ObjectiveAir", "GetTotalAirTime", get_total_air_time),
        ("ObjectiveAir", "GetAirHeight", get_air_height),
        ("ObjectiveAir", "GetDegreesOfSpin", get_degrees_of_spin),
        ("ObjectiveAir", "GetNumBodyFlips", get_num_body_flips),
        ("ObjectiveAir", "GetNumSkatersAiredOver", get_num_skaters_aired_over),
        ("ObjectiveContextual", "DidAchievement", did_contextual_achievement),
        ("ObjectiveContextual", "DidAchievementOfGroup", did_contextual_achievement_of_group),
        ("ObjectiveContextual", "GetNumAchievements", get_num_contextual_achievements),
        ("ObjectiveGameInfo", "GetNumSkaters", get_num_skaters),
        ("ObjectiveHOM", "GetScore", hom_get_score),
        ("ObjectiveHOM", "GetDamageScore", hom_get_damage_score),
        ("ObjectiveHOM", "GetBonusScore", hom_get_bonus_score),
        ("ObjectiveHOM", "GetRotationScore", hom_get_rotation_score),
        ("ObjectiveHOM", "GetNumBodyTweaksDone", hom_get_num_body_tweaks_done),
        ("ObjectiveHOM", "DidSpecificBodyTweakThisFrame", hom_did_specific_body_tweak_this_frame),
        ("ObjectiveHOM", "GetWipeoutSpeed", hom_get_wipeout_speed),
        ("ObjectiveHOM", "GetMaxAirDrop", hom_get_max_air_drop),
        ("ObjectiveHOM", "GetMaxAirTime", hom_get_max_air_time),
        ("ObjectiveHOM", "GetNumPedestriansHit", hom_get_num_pedestrians_hit),
        ("ObjectiveHOM", "GetNumVehiclesHit", hom_get_num_vehicles_hit),
        ("ObjectiveHOM", "GetNumDMOsHit", hom_get_num_dmos_hit),
        ("ObjectiveHOM", "GetBoneState", hom_get_bone_state),
        ("ObjectiveHOM", "GetNumBonesAtDamageLevel", hom_get_num_bones_at_damage_level),
        ("ObjectiveHOM", "GetNumBonesGTOEDamageLevel", hom_get_num_bones_gtoe_damage_level),
        ("ObjectiveHOM", "IsLastBailTop10", hom_is_last_bail_top10),
        ("ObjectiveLocation", "IsInLocation", is_in_location),
        ("ObjectiveRound", "GetRoundScore", get_round_score),
        ("ObjectiveRound", "GetRoundRank", get_round_rank),
        ("ObjectiveRound", "GetOverallRank", get_overall_rank),
        ("ObjectiveTrigger", "EnteredVolumesInOrder", entered_volumes_in_order),
        ("Reset", "ClearContextualData", clear_contextual_data),
        ("Reset", "ResetAllObjectives", reset_all_objectives),
        ("Tricks", "DidSpecificTrickSequence", did_specific_trick_sequence),
        ("Tricks", "GetNumTricksDoneInTrickGroup", get_num_tricks_done_in_trick_group),
        ("Tricks", "IsInCoffin", is_in_coffin),
    ]
}

/// `cChallengeObjectivesManager::PerformLuaCBinding` (`0x82d4faf8`) modules.
pub const MODULES: [&str; 21] = [
    "ObjectivePoints",
    "ObjectiveTrigger",
    "Grind",
    "ObjectiveAir",
    "ObjectiveRound",
    "Tricks",
    "ObjectiveHOM",
    "ObjectiveSessionMarker",
    "Gesture",
    "ObjectiveGameInfo",
    "ObjectiveLocomotion",
    "DMOs",
    "ObjectiveInfo",
    "ObjectiveAchievements",
    "ObjectiveContextual",
    "ObjectiveLocation",
    "Wipeout",
    "Manual",
    "PowerSlide",
    "Reset",
    "Stance",
];

/// `cChallengeObjectivesManager::PerformLuaGlobalBinding` (`0x82d42cd0`).
pub const INIT_VARS: &str = "function _G.InitVars( table, name, value ) if( table[ name ] == nil ) then table[ name ] = value; end end";
