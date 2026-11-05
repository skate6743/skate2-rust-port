//! `Sk8::Physics::PhysOutModule_MissionSystem`: the challenge messages a
//! skater's completed physics output produces every simulation update
//! (`Update` `0x82e313a0`, `GenerateScoringMessages` `0x82e30368`), and the
//! objective natives' view of the same output.

use super::{
    objectives::PlayerState,
    system::{Event, Field, Message},
};

/// One `mTricksEnded` entry.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EndedTrick {
    /// `TrickScoringSettings::GetTrickName`.
    pub name: String,
    pub is_valid: bool,
    pub was_a_tap_trick: bool,
    pub trick_index: u32,
    pub trick_type: i32,
    pub score: f32,
    pub started_switched: bool,
    pub trick_attrib_key: u64,
}

/// `mFinishedGrindInfo`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FinishedGrind {
    pub length: f32,
    pub pathed_guid: u64,
    pub local_guid: u64,
}

/// A `ContextSensitiveDataGroups`' first UID lists (`mContextSensitiveData[group].mUIDData[0]`, the
/// only ones the objectives read): each group's (`mUniqueID`, `mType`) entries and `mCount`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ContextUidLists {
    pub uids: [[(u32, i32); 8]; 5],
    pub counts: [u32; 5],
}

/// `mAirFinishedData`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FinishedAir {
    pub landed_successfully: bool,
    /// The initial trick's scorable id, -1 when it is not valid.
    pub first_scorable_id: i32,
    pub landing_was_sketchy: bool,
    /// `mPlayerSpin180Count` (signed).
    pub player_spin180_count: i32,
    pub started_switched: bool,
    pub grab_occured: bool,
    pub tweak_occured: bool,
    /// `mInAirFrames`, `mHeightToPeak`, `mHorizontalDistance`, `mGrabTime` and
    /// `mPlayerNumFlips`.
    pub in_air_frames: i32,
    pub height_to_peak: f32,
    pub horizontal_distance: f32,
    pub grab_time: f32,
    pub player_num_flips: i32,
    /// `mBigTrickAchievements`' thresholds 5 to 14 (`EScoreThresholdType` `AirTime` to
    /// `VeryHighGap`): each `mWasAchieved`.
    pub big_trick_thresholds: [[bool; 2]; 10],
    /// `mInitialContextData`, `mEndContextData` and `mAccumulatedContextData`.
    pub initial_context: ContextUidLists,
    pub end_context: ContextUidLists,
    pub accumulated_context: ContextUidLists,
}

/// The finished sequence fields `GenerateScoringMessages` sends.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FinishedSequence {
    pub score: u32,
    pub start_visual_tick: u32,
    pub start_sim_tick: u32,
    pub replay_start_frame: u32,
    pub trick_count: u32,
    pub stored_trick_count: u32,
    pub successfully: bool,
}

/// `mHoM_BailData` (`Score::HoM::BailScoringOutData`, `PhysOut_Scoring2 + 0x3d70`): the fields the
/// `ObjectiveHOM` natives read.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HomBail {
    /// Complete native output retained for the statistics savable.
    pub save_image:Option<super::hom_save::BailImage>,
    /// `mBones.mBones[i].mDamageLevel` (`+0x10` of each 32-byte bone) of the 25 bones.
    pub bone_damage_levels: [i32; 25],
    pub score: u32,
    pub bone_scores: [u32;6],
    /// Duration, drop, air time, rotation, speed, tweaks, pedestrians, vehicles, DMOs.
    pub bonus_scores: [u32;9],
    pub overall_duration: f32,
    pub max_air_frames: u32,
    pub accumulated_air_drop_height:f32,
    pub rotation: f32,
    pub moving_duration: f32,
    pub skeleton_over_duration: f32,
    pub severe_duration: f32,
    /// `mEntitySets[i].mCount`: pedestrians, vehicles and DMOs hit.
    pub entity_counts: [u32; 3],
    pub security_guards_hit:u32,
    /// `mMaxAirDrop.mDropHeight`.
    pub max_air_drop_height: f32,
    /// `mAccumulatedAirDrop.mDurationInFrames`.
    pub accumulated_air_drop_frames: u32,
    /// `mBoneScore`, `mBonusScore`, `mRotationScore` and `mNumBodyTweaksPerformed`.
    pub bone_score: u32,
    pub bonus_score: u32,
    pub rotation_score: u32,
    pub num_body_tweaks_performed: u32,
    /// `mBailSpeed`.
    pub bail_speed: f32,
    /// `mCurrentBodyTweak` (`EWipeoutBodyTweakIndex`, -1 for none).
    pub current_body_tweak: i32,
}

/// The completed `PhysOut` a skater publishes for one simulation update.
#[derive(Clone, Debug, Default, PartialEq)]
#[expect(clippy::struct_excessive_bools, reason = "native output flags")]
pub struct MissionFrame {
    pub player: i32,
    pub local: bool,
    /// `Actor::GetPosition`: the skeleton root translation.
    pub position: [f32; 3],
    /// `cTriggerTester`: `GetBasePosition`, `GetTopPosition`,
    /// `GetCentreOfMass`.
    pub tester_base: [f32; 3],
    pub tester_top: [f32; 3],
    pub tester_centre_of_mass: [f32; 3],
    /// `cTriggerTester::IsValid`.
    pub tester_valid: bool,
    pub is_pushing: bool,
    pub is_new_auto_pump: bool,
    pub is_physically_pushing: bool,
    pub is_intentional_pump: bool,
    pub deck_speed: f32,
    pub surface_speed: f32,
    pub is_teleport_finished: bool,
    pub physics_requested_teleport: bool,
    /// `mFilteredPhysicsState == FILTERED_PHYSICS_STATE_TELEPORT`.
    pub teleporting: bool,
    pub is_regular_stance: bool,
    /// `mIsRidingSwitch`, which `PhysOutModule_Hud::UpdateStance`
    /// (`0x827c4898`) gives the tricks back end (`+0x28`, the trick display's
    /// `mCurrentStance`).
    pub is_riding_switch: bool,
    pub is_wipeout_teleport_pending: bool,
    pub time_until_wipeout_teleport: f32,
    pub is_grabbing_object: bool,
    pub pumping_acceleration: f32,
    pub finished_grind: Option<FinishedGrind>,
    pub finished_air: Option<FinishedAir>,
    pub finished_manual: Option<f32>,
    pub finished_powerslide: Option<f32>,
    pub tricks_ended: Vec<EndedTrick>,
    pub line_timer_finished: bool,
    pub just_wiped_out: bool,
    pub performed_gesture: Option<i32>,
    pub sequence_begun: bool,
    pub sequence_completed: Option<FinishedSequence>,
    pub banked_point_change: i32,
    pub hom_bail_in_progress: bool,
    pub hom_bail_completed: bool,
    pub hom_effect_enabled: bool,
    pub hom_body_tweak: Option<i32>,
    pub just_started_line_timer: bool,
    pub is_onboard_or_hippy_jumping: bool,
    pub was_onboard_or_hippy_jumping: bool,
    pub is_offboard_and_not_hippy_jumping: bool,
    pub was_offboard_and_not_hippy_jumping: bool,
    pub in_sequence: bool,
    pub sequence_completed_successfully: bool,
    pub is_in_state_ground: bool,
    pub is_in_state_air: bool,
    pub moving_wipeout_duration: f32,
    /// `PhysOut_Scoring2::mManualLength` and `mManualTime`.
    pub manual_length: f32,
    pub manual_time: f32,
    /// `PhysOut_Trick::mCurrentAnimData.mAnimTrickBits`.
    pub anim_trick_bits: u32,
    /// `mFilteredPhysicsState == FILTERED_PHYSICS_STATE_WIPEOUT` (4).
    pub wiping_out: bool,
    /// `mFinishedSequencePatternMatches` and `mCurrentSequencePatternMatches`.
    pub finished_pattern_matches: [bool; 3],
    pub current_pattern_matches: [bool; 3],
    /// `mCurrentAirData.mInitialTrick`'s scorable id, -1 when it is not valid.
    pub current_air_initial_scorable: i32,
    /// `mFinishedSequenceScore`, which stays after the update that finished it.
    pub finished_sequence_score: u32,
    pub line_score: i32,
    /// `Score::IsInACurrentTrick` (`0x82ded580`): `mActiveTricks` has entries.
    pub in_a_current_trick: bool,
    /// The `gScorableInfo` trick types of `mActiveTricks`' valid scorables
    /// (`Score::IsDoingATrickOfType` `0x82df2428`).
    pub active_trick_types: Vec<i32>,
    /// `PhysOut_Air::mIsSkateboardAirborne`.
    pub skateboard_airborne: bool,
    /// `mIsInState_Grinding` (`Score::IsSkaterGrinding` `0x82de95a0`).
    pub is_in_state_grinding: bool,
    /// `mFinishedGrindInfo.mLength`, which stays after the update that finished it.
    pub finished_grind_length: f32,
    /// `mTrickTypesInCompletedSequence`.
    pub trick_types_in_completed_sequence: [u8; 14],
    /// `mHoM_BailData.mScore`.
    pub hom_score: u32,
    /// The rest of `mHoM_BailData` the objective natives read.
    pub hom_bail: HomBail,
    /// `PhysOut_Filtered::mIsGrinding`, `mGrindPathedGuid` and `mGrindLocalGuid`.
    pub filtered_grinding: bool,
    pub filtered_grind_pathed_guid: u64,
    pub filtered_grind_local_guid: u64,
    /// `PhysOut_Scoring2::mCurrentGesture` (`eAnimGestureNames`, `Count` (35) for none).
    pub current_gesture: i32,
    /// `mActiveTricks`' entries: each `mIsValid` and `mScorableID`.
    pub active_tricks: Vec<(bool, i32)>,
    /// `mBigTrickAchievedData`: each threshold's `mWasAchieved`.
    pub big_trick_achieved: [[bool; 2]; 15],
    /// `mFinishedGrindInfo.mObjectInfo`: `mIsValid`, `mEntityType` and `mUniqueID`.
    pub finished_grind_object: (bool, i32, u32),
    /// The vote the player's input pressed this update (`ChallengeVotingManager::Update`'s
    /// states 139, 136, 137 and 138), which an online game's facts carry from its console.
    pub vote: Option<i32>,
}

/// The module's thresholds: `kSkaterStopThreshold`,
/// `kManualPumpingAccelerationThreshold` and the `physics_wipeout` default's
/// almost-over time (`0xb565383e5da5382b`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MissionTuning {
    pub stop_threshold: f32,
    pub manual_pumping_threshold: f32,
    pub wipeout_almost_over_time: f32,
}

impl Default for MissionTuning {
    fn default() -> Self {
        Self { stop_threshold: 1.2, manual_pumping_threshold: 0.1, wipeout_almost_over_time: 0.0 }
    }
}

/// `PhysOutModule_MissionSystem`'s retained state.
#[derive(Clone, Debug, Default)]
pub struct MissionSystem {
    push_message_delay: i32,
    push_attempts_since_last_message: i32,
    pushed_last_frame: bool,
    stopped: [bool; 9],
    wipeout_almost_over_sent: [bool; 9],
    manual_pumping: [bool; 9],
}

fn slot(player: i32) -> usize {
    usize::try_from(player).unwrap_or(0).min(8)
}

#[expect(clippy::cast_precision_loss, reason = "lua_Number")]
fn number(value: i32) -> Field {
    Field::Number(value as f32)
}

#[expect(clippy::cast_precision_loss, reason = "lua_Number")]
fn unsigned(value: u32) -> Field {
    Field::Number(value as f32)
}

fn message(name: &str, frame: &MissionFrame, mut fields: Vec<(&'static str, Field)>) -> Event {
    fields.insert(0, ("PlayerID", number(frame.player)));
    Event::Challenge(Message::new(name, fields))
}

impl MissionSystem {
    /// `Update` (`0x82e313a0`) for one skater.
    pub fn update(&mut self, frame: &MissionFrame, tuning: MissionTuning, send: &mut impl FnMut(Event)) {
        if frame.local {
            if frame.is_pushing {
                if !self.pushed_last_frame {
                    self.push_attempts_since_last_message += 1;
                }
                self.pushed_last_frame = true;
            } else {
                self.pushed_last_frame = false;
            }
            if self.push_message_delay == 0 && frame.is_physically_pushing {
                self.push_message_delay = 60;
                send(message("cMsgSkaterPushed", frame, vec![("PushAttempts", number(self.push_attempts_since_last_message))]));
                self.push_attempts_since_last_message = 0;
            }
            self.push_message_delay = (self.push_message_delay - 1).max(0);
            if frame.is_new_auto_pump {
                send(message("cMsgSkaterAutoPumped", frame, Vec::new()));
            }
        }
        self.generate_scoring_messages(frame, tuning, send);
        if frame.is_teleport_finished {
            send(message("cMsgSkaterTeleportComplete", frame, Vec::new()));
        }
        let i = slot(frame.player);
        if frame.deck_speed >= tuning.stop_threshold {
            if self.stopped[i] {
                send(message("cMsgSkaterMoving", frame, vec![("IsMoving", Field::Bool(true))]));
            }
            self.stopped[i] = false;
        } else {
            if !self.stopped[i] {
                send(message("cMsgSkaterMoving", frame, vec![("IsMoving", Field::Bool(false))]));
            }
            self.stopped[i] = true;
        }
    }

    /// `GenerateScoringMessages` (`0x82e30368`).
    #[expect(clippy::too_many_lines, reason = "one native function")]
    fn generate_scoring_messages(&mut self, frame: &MissionFrame, tuning: MissionTuning, send: &mut impl FnMut(Event)) {
        if let Some(grind) = frame.finished_grind {
            if std::env::var_os("SKATE2_TRACE_SCORE_SEQUENCE").is_some() {
                eprintln!("score: mission grind completed player {} length {} pathed {:#x}", frame.player, grind.length, grind.pathed_guid);
            }
            send(message(
                "cMsgGrindCompleted",
                frame,
                vec![
                    ("Wipedout", number(0)),
                    ("UsedSessionMarker", Field::Bool(frame.teleporting)),
                    ("GrindLength", Field::Number(grind.length)),
                    ("PathedSplineID", Field::Key(grind.pathed_guid)),
                    ("LocalSplineID", Field::Key(grind.local_guid)),
                ],
            ));
        }
        if let Some(air) = frame.finished_air {
            send(message(
                "cMsgAirCompleted",
                frame,
                vec![
                    ("Wipedout", number(i32::from(!air.landed_successfully))),
                    ("UsedSessionMarker", Field::Bool(frame.teleporting)),
                    ("FirstScorableId", number(air.first_scorable_id)),
                    ("WasSketchy", Field::Bool(air.landing_was_sketchy)),
                ],
            ));
        }
        if let Some(length) = frame.finished_manual {
            send(message("cMsgManualCompleted", frame, vec![("Wipedout", number(0)), ("ManualLength", Field::Number(length))]));
        }
        if let Some(length) = frame.finished_powerslide {
            send(message("cMsgPowerslideCompleted", frame, vec![("Wipedout", number(0)), ("PowerslideLength", Field::Number(length))]));
        }
        for (entry, trick) in frame.tricks_ended.iter().enumerate() {
            if !trick.is_valid || trick.was_a_tap_trick {
                continue;
            }
            let reversed = if frame.is_regular_stance { trick.started_switched } else { !trick.started_switched };
            #[expect(clippy::cast_possible_truncation, reason = "fctiwz")]
            let score = trick.score as i32;
            send(Event::Challenge(Message::new(
                "cMsgTrickComplete",
                vec![
                    ("TrickName", Field::Text(trick.name.clone())),
                    ("TrickIndex", unsigned(trick.trick_index)),
                    ("TrickType", number(trick.trick_type)),
                    ("PlayerID", number(frame.player)),
                    ("TrickScore", number(score)),
                    ("TrickReversed", number(i32::from(reversed))),
                    ("TrickEntry", number(i32::try_from(entry).unwrap_or(i32::MAX))),
                    ("TrickAttribKey", Field::Key(trick.trick_attrib_key)),
                ],
            )));
        }
        if frame.line_timer_finished {
            send(message("cMsgLineTimerExpired", frame, vec![("OnWipeOut", Field::Bool(frame.just_wiped_out))]));
        }
        if let Some(gesture) = frame.performed_gesture {
            send(message("cMsgGesturePerformed", frame, vec![("Gesture", number(gesture))]));
        }
        if frame.sequence_begun {
            send(message("cMsgSequenceBegin", frame, Vec::new()));
        }
        if let Some(sequence) = frame.sequence_completed {
            send(message(
                "cMsgSequenceComplete",
                frame,
                vec![
                    ("SequenceScore", unsigned(sequence.score)),
                    ("StartVisualGameTick", unsigned(sequence.start_visual_tick)),
                    ("StartTick", unsigned(sequence.replay_start_frame)),
                    ("TotalTrickCount", unsigned(sequence.trick_count)),
                    ("StoredTrickCount", unsigned(sequence.stored_trick_count)),
                    ("Wipedout", Field::Bool(!sequence.successfully)),
                    ("UsedSessionMarker", Field::Bool(frame.teleporting)),
                    ("StartSimGameTick", unsigned(sequence.start_sim_tick)),
                ],
            ));
        }
        if frame.banked_point_change > 0 {
            send(message("cMsgPointsAwarded", frame, vec![("Score", number(frame.banked_point_change))]));
        }
        let i = slot(frame.player);
        if !self.wipeout_almost_over_sent[i]
            && frame.is_wipeout_teleport_pending
            && frame.time_until_wipeout_teleport < tuning.wipeout_almost_over_time
        {
            send(message("cMsgWipeoutAlmostOver", frame, vec![("TimeToTeleport", Field::Number(frame.time_until_wipeout_teleport))]));
            self.wipeout_almost_over_sent[i] = true;
        }
        if frame.hom_bail_in_progress
            && let Some(tweak) = frame.hom_body_tweak
        {
            send(message("cMsgWipeoutBodyTweakPerformed", frame, vec![("BodyTweak", number(tweak))]));
        }
        if frame.hom_bail_completed {
            send(message("cMsgWipeoutOver", frame, Vec::new()));
            self.wipeout_almost_over_sent[i] = false;
        }
        if frame.just_started_line_timer {
            send(message("cMsgLineTimerStarted", frame, Vec::new()));
        }
        if frame.just_wiped_out {
            send(message("cMsgWipeout", frame, vec![("IsAPhysicsTeleport", Field::Bool(frame.physics_requested_teleport))]));
        }
        if frame.is_onboard_or_hippy_jumping && !frame.was_onboard_or_hippy_jumping {
            send(message("cMsgSkaterOnboard", frame, Vec::new()));
        } else if frame.is_offboard_and_not_hippy_jumping && !frame.was_offboard_and_not_hippy_jumping {
            send(message("cMsgSkaterOffboard", frame, Vec::new()));
        }
        if frame.is_grabbing_object {
            send(message("cMsgSkaterGrabbingObject", frame, Vec::new()));
        }
        if frame.pumping_acceleration > tuning.manual_pumping_threshold && frame.is_intentional_pump {
            if !self.manual_pumping[i] {
                send(message("cMsgSkaterManualPumped", frame, vec![("Acceleration", Field::Number(frame.pumping_acceleration))]));
            }
            self.manual_pumping[i] = true;
        } else {
            self.manual_pumping[i] = false;
        }
    }
}

/// What the objective natives read of the local skater's output.
#[must_use]
pub fn player_state(frame: &MissionFrame, colliding_triggers: Vec<(u64, u64)>) -> PlayerState {
    PlayerState {
        exists: true,
        onboard_or_hippy_jumping: frame.is_onboard_or_hippy_jumping,
        offboard_and_not_hippy_jumping: frame.is_offboard_and_not_hippy_jumping,
        was_onboard_or_hippy_jumping: frame.was_onboard_or_hippy_jumping,
        was_offboard_and_not_hippy_jumping: frame.was_offboard_and_not_hippy_jumping,
        in_sequence: frame.in_sequence,
        sequence_completed_successfully: frame.sequence_completed_successfully,
        on_ground: frame.is_in_state_ground,
        in_air: frame.is_in_state_air,
        surface_speed: frame.surface_speed,
        hom_bail_in_progress: frame.hom_bail_in_progress,
        hom_bail_completed: frame.hom_bail_completed,
        colliding_triggers,
        position: frame.position,
        tricks_ended: frame
            .tricks_ended
            .iter()
            .filter(|t| t.is_valid && !t.was_a_tap_trick)
            .map(|t| t.trick_attrib_key)
            .collect(),
        grabbing_object: frame.is_grabbing_object,
        moving_wipeout_duration: frame.moving_wipeout_duration,
        teleporting: frame.teleporting,
        tester_base: frame.tester_valid.then_some(frame.tester_base),
        manual_length: frame.manual_length,
        manual_time: frame.manual_time,
        anim_trick_bits: frame.anim_trick_bits,
        wiping_out: frame.wiping_out,
    }
}
