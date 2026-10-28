//! `Sk8::Challenge::cChallengeObjectivesManager` (`packages/sk8/sk8objectives`):
//! a challenge's objectives, each a set of Lua conditions (completion,
//! failure, hint) compiled from source into the objectives' own Lua state and
//! evaluated every frame or on the events their `ObjectiveTriggers` name.

use super::{
    assets,
    attrib_db::{AttribDb, Location},
    attrib_lua::{ObjectKind, Pointee},
    indicators,
    keys::{class, hash64},
    lua::{Lua, Value},
    presentation::{self, PresentationEvent},
    skate,
    system::{Event, Field, Message, Output, World},
};

/// `challenge_objective` fields.
pub mod field {
    pub const OBJECTIVE_DEFINITION: u64 = 0xe13d_9c32_6e4d_98d5;
    pub const POINTS_VALID: u64 = 0x41cb_f3f0_889d_e6b3;
    /// `tObjectiveDefinition[]`: plain failure conditions.
    pub const FAILURES: u64 = 0x40d4_0861_dd9a_6cc6;
    /// RefSpec[] into the condition class: failures with presentation events.
    pub const FAILURES_WITH_EVENTS: u64 = 0xb69a_f085_f7a6_d275;
    /// RefSpec[] into the condition class: hints.
    pub const HINTS: u64 = 0xecae_b8e6_19a6_8af3;
    pub const ACTIVATION_EVENTS: u64 = 0xe73c_9cf0_f75a_6fcd;
    pub const COMPLETION_EVENTS: u64 = 0xd404_ba33_19e2_c389;
    pub const FAILURE_EVENTS: u64 = 0x607e_c72f_9329_97c1;
    pub const ACTIVATION_MODE: u64 = 0xd733_2063_be5c_c0eb;
    pub const OBJECTIVE_TRIGGERS: u64 = 0xd496_5ed3_bd43_ec44;
    pub const SCORING_BOUNDARY: u64 = 0xfc7c_6353_408b_885c;
    pub const MASTER_OBJECTIVE: u64 = 0xbc7d_59ec_bb20_868d;
    pub const REQUIRES_HIGHLIGHT: u64 = 0xe0b7_7cde_569c_8c0b;
    /// Bool read as `0xB0531066`: run hints and failures before completion.
    pub const FAILURE_FIRST: u64 = 0xb053_1066_0476_7956;
    /// The condition class (`0x3159635da17ff60e`): definition and events.
    pub const CONDITION_CLASS: u64 = 0x3159_635d_a17f_f60e;
    pub const CONDITION_EVENTS: u64 = 0xbd96_b775_b855_e129;
    /// Short: the fraction's denominator in the objective HUD ("#%d/%d"); above
    /// 1 the result style is a fraction. The name is not recovered.
    pub const FRACTION_DENOMINATOR: u64 = 0x2b24_5ec6_d20a_64aa;
    /// Bool `DisplayInHUD`; an absent attribute counts as shown.
    pub const DISPLAY_IN_HUD: u64 = 0x014f_34b8_86d6_69a4;
    /// `tVisualEditorData[]`: the objective's visual indicators.
    pub const VISUAL_INDICATORS: u64 = 0x7ae8_fef7_7a54_bb91;
    /// `eVisualIndicatorMode`: who shows them.
    pub const VISUAL_INDICATOR_MODE: u64 = 0x16f3_50c5_3880_eaa9;
}

/// The `challenge_objective` `AddPointsObjective` (`0x82d51938`) makes a group's points
/// objective from.
const GLOBAL_POINTS_OBJECTIVE: &str = "global_points_objective";

/// `CompletedGrinds`' fixed capacity (`0x100` bytes of spline IDs at `+0xAE8`).
const COMPLETED_GRINDS_CAPACITY: usize = 32;

/// `eVisualIndicatorMode`: `Auto` indicators follow the objective's status, `Manual` ones
/// `OverrideVisualIndicatorStatus`.
pub mod visual_indicator_mode {
    pub const AUTO: i32 = 0;
    pub const MANUAL: i32 = 1;
}

/// `eObjectiveType`: how the objective HUD shows a result.
pub mod result_style {
    pub const FRACTION: i32 = 0;
    pub const CHECK_BOX: i32 = 1;
}

/// `challenges` fields read by `MakeObjectives_helper` (`0x82d52de0`).
pub mod challenge_field {
    pub const OBJECTIVE_FLOW: u64 = 0xce53_161a_0e75_4f51;
    pub const DISPLAY_STATUSES: u64 = 0x2837_e2dd_5910_6c10;
    pub const NOTIFICATION_STATUSES: u64 = 0x291d_6d1b_65ae_ffd2;
    pub const SHOW_ACTIVE_WITH_NOTIFICATIONS: u64 = 0xe82a_28be_1173_4cfa;
    pub const POINTS_REQUIREMENT: u64 = 0x0c0c_fefb_9fc6_eb73;
}

/// `eObjectiveStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Disabled = 1,
    Enabled = 2,
    Active = 4,
    Complete = 8,
    Failed = 16,
}

/// `tObjectiveTriggers` (20 bytes; `TriggerField` unused here).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[expect(clippy::struct_excessive_bools, reason = "native layout")]
pub struct Triggers {
    pub wiped_out: bool,
    pub sequence_complete: bool,
    pub trigger_enter: bool,
    pub challenge_complete: bool,
    pub group_objective_complete: bool,
    pub session_marker_set: bool,
    pub braked_or_pushed: bool,
    pub skater_state_change: bool,
    pub manual_pumped: bool,
    pub air_complete: bool,
    pub grind_complete: bool,
    pub fail_on_boundary_exit: bool,
    pub continuous_standard: bool,
    pub continuous_wipeout: bool,
}

impl Triggers {
    fn read(db: &AttribDb, objective: u64) -> Self {
        let b = db
            .attribute(class::CHALLENGE_OBJECTIVE, objective, field::OBJECTIVE_TRIGGERS)
            .and_then(|a| a.value_bytes(0))
            .unwrap_or_else(|| vec![0; 20]);
        let f = |i: usize| b.get(i).copied().unwrap_or(0) != 0;
        Self {
            wiped_out: f(0),
            sequence_complete: f(1),
            trigger_enter: f(2),
            challenge_complete: f(3),
            group_objective_complete: f(4),
            session_marker_set: f(5),
            braked_or_pushed: f(6),
            skater_state_change: f(7),
            manual_pumped: f(8),
            air_complete: f(9),
            grind_complete: f(10),
            fail_on_boundary_exit: f(11),
            continuous_standard: f(12),
            continuous_wipeout: f(13),
        }
    }

    /// `ObjectiveUtils::EvaluateObjectiveTrigger` (`0x82d42b80`).
    const fn matches(self, objective: Self) -> bool {
        (self.wiped_out && objective.wiped_out)
            || (self.sequence_complete && objective.sequence_complete)
            || (self.trigger_enter && objective.trigger_enter)
            || (self.session_marker_set && objective.session_marker_set)
            || (self.braked_or_pushed && objective.braked_or_pushed)
            || (self.group_objective_complete && objective.group_objective_complete)
            || (self.challenge_complete && objective.challenge_complete)
            || (self.skater_state_change && objective.skater_state_change)
            || (self.manual_pumped && objective.manual_pumped)
            || (self.air_complete && objective.air_complete)
            || (self.grind_complete && objective.grind_complete)
            || (self.continuous_standard && objective.continuous_standard)
            || (self.continuous_wipeout && objective.continuous_wipeout)
    }

    fn group_objective_complete() -> Self {
        Self { group_objective_complete: true, ..Self::default() }
    }
}

/// `cChallengeObjectiveScriptHandler` (`0x7c` bytes).
#[derive(Clone, Debug)]
pub struct ScriptHandler {
    /// `h_%016I64X_%s_%d`, at most 31 characters.
    pub function: String,
    /// `tCompiledLua::SourceLua`.
    pub source: String,
    pub value_on_error: bool,
    pub evaluated: u32,
    pub evaluated_true: u32,
}

impl ScriptHandler {
    fn new(key: u64, postfix: &str, index: usize, source: String, value_on_error: bool) -> Self {
        let mut function = format!("h_{key:016X}_{postfix}_{index}");
        function.truncate(31);
        Self { function, source, value_on_error, evaluated: 0, evaluated_true: 0 }
    }
}

/// `cChallengeObjective` (`0xf58` bytes), the parts the ported paths use.
#[derive(Clone, Debug)]
pub struct Objective {
    pub key: u64,
    pub unique_id: u64,
    pub player: i32,
    pub completion: ScriptHandler,
    pub points_valid: ScriptHandler,
    pub master: bool,
    pub failures: Vec<ScriptHandler>,
    pub failures_with_events: Vec<ScriptHandler>,
    pub hints: Vec<ScriptHandler>,
    pub failure_groups: Vec<Option<u32>>,
    pub hint_groups: Vec<Option<u32>>,
    pub activation_group: Option<u32>,
    pub completion_group: Option<u32>,
    pub failure_group: Option<u32>,
    pub state: Status,
    pub notify_window: u32,
    pub in_scoring_boundary: bool,
    pub dirty: bool,
    pub vars_in_vm: bool,
    pub trigger_volumes: Vec<u64>,
    pub sequence_volumes: Vec<u64>,
    pub just_used_session_marker: bool,
    pub just_set_session_marker: bool,
    pub num_pushes: u32,
    pub skater_state: u32,
    pub last_sequence_points: u32,
    /// `mTotalPoints`.
    pub total_points: u32,
    /// `mObjectiveResultStyle`.
    pub result_style: i32,
    /// `mFractionNumerator`.
    pub fraction_numerator: i32,
    /// `mTitle`: the HAL key and the comma-separated arguments the native
    /// formats it with (`LocalizerManager::FormatStringCustom`).
    pub title: (String, String),
    /// `CompletedTricks`: trick attrib keys while active, at most 32.
    pub completed_tricks: std::collections::VecDeque<u64>,
    /// `DMOProxyIDs`.
    pub dmo_proxy_ids: Vec<i32>,
    /// `VisualIndicatorGroup`.
    pub indicator_group: Option<u32>,
    /// `mGrindData`: `mGrindLength` (`+0x47C`), `mTotalGrindLength` (`+0x480`),
    /// `mMaxGrindLengthInSequence` (`+0x484`) and `mTotalGrindLengthInSequence` (`+0x488`).
    pub grind_length: f32,
    pub total_grind_length: f32,
    pub max_grind_length_in_sequence: f32,
    pub total_grind_length_in_sequence: f32,
    /// `CompletedGrinds`: the pathed and local spline IDs of the grinds completed while active.
    pub completed_grinds: Vec<u64>,
    /// `mAwardSequenceCompletePointsImmediately` (`+0xF50`).
    pub award_sequence_points_immediately: bool,
    /// The air, gesture, big-trick, contextual and round records.
    pub data: super::objective_data::ObjectiveData,
}

/// `cChallengeObjectiveGroup` (`0x210` bytes).
#[derive(Clone, Debug)]
pub struct ObjectiveGroup {
    pub id: u32,
    pub handle: usize,
    pub activity: usize,
    pub challenge: u64,
    pub player: i32,
    pub objectives: Vec<Objective>,
    pub evaluate: bool,
    pub points_objective: Option<usize>,
    pub points_requirement: u32,
    /// `tObjectiveFlow`: 0 all at start, 1 sequenced.
    pub flow_type: i32,
    pub max_active: i32,
    pub display_statuses: u32,
    pub notification_statuses: u32,
    pub show_active_with_notifications: bool,
    /// `UniqueTrickGroupKey`: the trick group whose tricks count once each, `None` (-1) untracked.
    pub unique_trick_group: Option<u64>,
    /// `CompletedUniqueTricks` (`fixed_vector<u64, 32>`).
    pub completed_unique_tricks: Vec<u64>,
    /// `SequenceCompletedTricks` (`fixed_vector<u64, 12>`): the current sequence's tricks.
    pub sequence_completed_tricks: Vec<u64>,
    /// `mLastCompletedUnqiueTrick`, `None` (-1) without one.
    pub last_completed_unique_trick: Option<u64>,
}

#[derive(Debug, Default)]
pub struct ObjectivesManager {
    next_id: u32,
    next_instance: u32,
    pub groups: Vec<ObjectiveGroup>,
    /// `CachedLocationKey`: the last `eFE_LocationTriggers` key of a `cMsgWorldPainter_KeyChange`
    /// (`WorldPainterKeyChangeMsg` `0x82d42d58`), `None` for the constructor's -1.
    pub cached_location_key: Option<u64>,
}

/// `tObjectiveClosureData`: what the objective natives see while a condition
/// runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Closure {
    pub group: u32,
    pub objective: usize,
    pub is_sequence_complete: bool,
    /// `IsNewlyCompletedObjective` (`+9`): `EvaluatePointsValid` testing an objective its pass
    /// completed.
    pub is_newly_completed: bool,
}

/// The local player's frame state the objective natives read
/// (`PhysOut_Scoring2`, `PhysOut_SkateboardMotion`, trigger collisions).
#[derive(Clone, Debug, Default, PartialEq)]
#[expect(clippy::struct_excessive_bools, reason = "native flags")]
pub struct PlayerState {
    pub exists: bool,
    pub onboard_or_hippy_jumping: bool,
    pub offboard_and_not_hippy_jumping: bool,
    pub was_onboard_or_hippy_jumping: bool,
    pub was_offboard_and_not_hippy_jumping: bool,
    pub in_sequence: bool,
    pub sequence_completed_successfully: bool,
    pub on_ground: bool,
    pub in_air: bool,
    /// `mSkateboardSurfaceSpeed`.
    pub surface_speed: f32,
    pub hom_bail_in_progress: bool,
    pub hom_bail_completed: bool,
    /// Challenge-group trigger volumes the player collides with:
    /// (`m_uiGuid`, `m_uiGuidLocal`).
    pub colliding_triggers: Vec<(u64, u64)>,
    /// The skater's world position (`IActor::GetPosition`).
    pub position: [f32; 3],
    /// `mTricksEnded` this frame: attrib keys of valid tricks that were not
    /// taps.
    pub tricks_ended: Vec<u64>,
    /// `PhysOut_OffBoard::mIsGrabbingObject`.
    pub grabbing_object: bool,
    /// `mHoM_BailData.mMovingWipeoutDurationInSeconds`.
    pub moving_wipeout_duration: f32,
    /// `PhysOut_Filtered::mFilteredPhysicsState` is `TELEPORT`.
    pub teleporting: bool,
    /// The trigger tester's `Actor::GetBasePosition` while it is valid.
    pub tester_base: Option<[f32; 3]>,
    /// `PhysOut_Scoring2::mManualLength` and `mManualTime`.
    pub manual_length: f32,
    pub manual_time: f32,
    /// `PhysOut_Trick::mCurrentAnimData.mAnimTrickBits`.
    pub anim_trick_bits: u32,
    /// `PhysOut_Filtered::mFilteredPhysicsState` is `WIPEOUT` (4).
    pub wiping_out: bool,
}

fn text(db: &AttribDb, class: u64, collection: u64, key: u64) -> Option<String> {
    db.attribute(class, collection, key)?.text(0).map(str::to_owned)
}

/// `tCompiledLua::SourceLua` of a `tObjectiveDefinition`/`tCompiledLua` value.
fn compiled_source(db: &AttribDb, class: u64, collection: u64, key: u64, index: usize) -> String {
    db.attribute(class, collection, key)
        .and_then(|a| a.data(index))
        .and_then(|at| db.text_at(at))
        .unwrap_or_default()
        .to_owned()
}

fn events(db: &AttribDb, class: u64, collection: u64, key: u64) -> Vec<PresentationEvent> {
    let Some(a) = db.attribute(class, collection, key) else { return Vec::new() };
    (0..a.len()).filter_map(|i| a.data(i)).filter_map(|at| PresentationEvent::read(db, at)).collect()
}

fn refs(db: &AttribDb, class: u64, collection: u64, key: u64) -> Vec<(u64, u64)> {
    db.attribute(class, collection, key)
        .map(|a| (0..a.len()).filter_map(|i| a.ref_spec(i)).collect())
        .unwrap_or_default()
}

/// A presentation group for a list of events, as the constructor builds one.
fn presentation_group(world: &mut World, challenge: u64, list: Vec<PresentationEvent>) -> Option<u32> {
    if list.is_empty() {
        return None;
    }
    let handle = presentation::add_group(world, challenge);
    let id = match world.pointers.get(handle) {
        Some(Pointee::Object { id, .. }) => id,
        _ => return None,
    };
    for e in list {
        presentation::add_event(world, id, e);
    }
    presentation::check_group_for_hud(world, id);
    Some(id)
}

/// `cChallengeObjective::cChallengeObjective` (`0x82d501a0`).
fn new_objective(world: &mut World, key: u64, challenge: u64, player: i32) -> Objective {
    let db = std::sync::Arc::clone(&world.db);
    let c = class::CHALLENGE_OBJECTIVE;
    let completion = ScriptHandler::new(key, "Obj", 0, compiled_source(&db, c, key, field::OBJECTIVE_DEFINITION, 0), true);
    let points_valid = ScriptHandler::new(key, "PointsValid", 0, compiled_source(&db, c, key, field::POINTS_VALID, 0), false);
    let activation_group = presentation_group(world, challenge, events(&db, c, key, field::ACTIVATION_EVENTS));
    let completion_group = presentation_group(world, challenge, events(&db, c, key, field::COMPLETION_EVENTS));
    let failure_group = presentation_group(world, challenge, events(&db, c, key, field::FAILURE_EVENTS));
    let mut failures_with_events = Vec::new();
    let mut failure_groups = Vec::new();
    for (i, (cc, ck)) in refs(&db, c, key, field::FAILURES_WITH_EVENTS).into_iter().enumerate() {
        failure_groups.push(presentation_group(world, challenge, events(&db, cc, ck, field::CONDITION_EVENTS)));
        let source = compiled_source(&db, cc, ck, field::OBJECTIVE_DEFINITION, 0);
        failures_with_events.push(ScriptHandler::new(key, "Failure", i, source, false));
    }
    let mut hints = Vec::new();
    let mut hint_groups = Vec::new();
    for (i, (cc, ck)) in refs(&db, c, key, field::HINTS).into_iter().enumerate() {
        hint_groups.push(presentation_group(world, challenge, events(&db, cc, ck, field::CONDITION_EVENTS)));
        let source = compiled_source(&db, cc, ck, field::OBJECTIVE_DEFINITION, 0);
        hints.push(ScriptHandler::new(key, "Hint", i, source, false));
    }
    let failures = db.attribute(c, key, field::FAILURES).map_or_else(Vec::new, |a| {
        (0..a.len())
            .map(|i| {
                let source = a.data(i).and_then(|at| db.text_at(at)).unwrap_or_default().to_owned();
                ScriptHandler::new(key, "Failure", i, source, false)
            })
            .collect()
    });
    let title = title(&db, c, key);
    let denominator = db.attribute(c, key, field::FRACTION_DENOMINATOR).and_then(|a| a.i16(0)).unwrap_or(0);
    // The local player's objective with visual indicators has a group under the challenge;
    // another player's has one when its indicators are Manual.
    let indicator_group = (!visual_indicators(&db, key).is_empty()
        && (player == world.local_player || visual_indicator_mode(&db, key) == visual_indicator_mode::MANUAL))
        .then(|| world.indicators.create_group(challenge));
    let manager = &mut world.objectives;
    manager.next_instance += 1;
    // UniqueID: StringHash64("0x%016I64x-%p") of the key and the instance.
    let unique_id = hash64(&format!("0x{key:016x}-{:08X}", manager.next_instance));
    Objective {
        key,
        unique_id,
        player,
        completion,
        points_valid,
        master: db.attribute(c, key, field::MASTER_OBJECTIVE).and_then(|a| a.bool(0)).unwrap_or(false),
        failures,
        failures_with_events,
        hints,
        failure_groups,
        hint_groups,
        activation_group,
        completion_group,
        failure_group,
        state: Status::Disabled,
        notify_window: 0,
        in_scoring_boundary: false,
        // Setting mTitle and mResult marks the objective dirty.
        dirty: true,
        vars_in_vm: false,
        trigger_volumes: Vec::new(),
        sequence_volumes: Vec::new(),
        just_used_session_marker: false,
        just_set_session_marker: false,
        num_pushes: 0,
        skater_state: 0,
        last_sequence_points: 0,
        total_points: 0,
        result_style: if denominator > 1 { result_style::FRACTION } else { result_style::CHECK_BOX },
        fraction_numerator: 0,
        title,
        completed_tricks: std::collections::VecDeque::new(),
        dmo_proxy_ids: Vec::new(),
        indicator_group,
        grind_length: 0.0,
        total_grind_length: 0.0,
        max_grind_length_in_sequence: 0.0,
        total_grind_length_in_sequence: 0.0,
        completed_grinds: Vec::new(),
        award_sequence_points_immediately: false,
        data: super::objective_data::ObjectiveData::default(),
    }
}

/// `ObjectiveInfo.VisualIndicatorMode` of an objective.
fn visual_indicator_mode(db: &AttribDb, key: u64) -> i32 {
    db.attribute(class::CHALLENGE_OBJECTIVE, key, field::VISUAL_INDICATOR_MODE).and_then(|a| a.i32(0)).unwrap_or(0)
}

/// `ObjectiveInfo.VisualIndicators` of an objective.
fn visual_indicators(db: &AttribDb, key: u64) -> Vec<Location> {
    db.attribute(class::CHALLENGE_OBJECTIVE, key, field::VISUAL_INDICATORS)
        .map_or_else(Vec::new, |a| (0..a.len()).filter_map(|i| a.data(i)).collect())
}

/// Every `VisualIndicators` entry into the objective's group (`AddVisualIndicator`) or out of
/// it (`RemoveVisualIndicator`).
fn show_visual_indicators(world: &mut World, indicator_group: u32, key: u64, show: bool) {
    for data in visual_indicators(&world.db, key) {
        if show {
            indicators::add_visual_indicator(world, indicator_group, data);
        } else {
            indicators::remove_visual_indicator(world, indicator_group, data);
        }
    }
}

/// The objective's activity, key and visual indicator group, when it has one.
fn objective_indicators(world: &World, group: u32, index: usize) -> Option<(usize, u64, u32)> {
    let g = &world.objectives.groups[world.objectives.group_index(group)?];
    let o = g.objectives.get(index)?;
    Some((g.activity, o.key, o.indicator_group?))
}

/// `cChallengeObjective::OverrideVisualIndicatorStatus` (`0x82d42d80`): a Manual objective's
/// indicators shown (once its challenge's hulls are in) or hidden.
pub fn override_visual_indicator_status(world: &mut World, group: u32, index: usize, show: bool) {
    let Some((activity, key, indicator_group)) = objective_indicators(world, group, index) else { return };
    if visual_indicator_mode(&world.db, key) == visual_indicator_mode::MANUAL && (!show || assets::are_hulls_loaded(world, activity)) {
        show_visual_indicators(world, indicator_group, key, show);
    }
}

/// `cChallengeObjectivesManager::ChallengeAssetLoadCompleteMsg` (`0x82d4a3c8`): the challenge's
/// active or enabled Auto objectives show their indicators (`ChallengeAssetLoadComplete`
/// `0x82d42fa0`).
pub fn challenge_asset_load_complete(world: &mut World, challenge: u64) {
    let shown: Vec<(u32, u64)> = world
        .objectives
        .groups
        .iter()
        .filter(|g| g.challenge == challenge)
        .flat_map(|g| g.objectives.iter())
        .filter(|o| matches!(o.state, Status::Active | Status::Enabled))
        .filter_map(|o| Some((o.indicator_group?, o.key)))
        .collect();
    for (indicator_group, key) in shown {
        if visual_indicator_mode(&world.db, key) == visual_indicator_mode::AUTO {
            show_visual_indicators(world, indicator_group, key, true);
        }
    }
}

impl ObjectivesManager {
    #[must_use]
    pub fn group_index(&self, id: u32) -> Option<usize> {
        self.groups.iter().position(|g| g.id == id)
    }
}

/// `AddObjectiveChallenge` (`0x82d4f7f8`): the activity's group for the
/// player, created on first use.
pub fn add_objective_challenge(world: &mut World, activity: usize, player: i32, flow: (i32, i32), masks: (u32, u32, bool)) -> u32 {
    if let Some(g) = world.objectives.groups.iter().find(|g| g.activity == activity && g.player == player) {
        return g.id;
    }
    world.objectives.next_id += 1;
    let id = world.objectives.next_id;
    let handle = world.pointers.handle(Pointee::Object { kind: ObjectKind::ObjectiveGroup, id });
    let challenge = world.activities.activities[activity].key;
    let group = ObjectiveGroup {
        id,
        handle,
        activity,
        challenge,
        player,
        objectives: Vec::new(),
        evaluate: true,
        points_objective: None,
        points_requirement: 0,
        flow_type: flow.0,
        max_active: flow.1,
        display_statuses: masks.0,
        notification_statuses: masks.1,
        show_active_with_notifications: masks.2,
        unique_trick_group: None,
        completed_unique_tricks: Vec::new(),
        sequence_completed_tricks: Vec::new(),
        last_completed_unique_trick: None,
    };
    let at = world.objectives.groups.iter().position(|g| g.activity > activity).unwrap_or(world.objectives.groups.len());
    world.objectives.groups.insert(at, group);
    id
}

/// `cChallengeObjectiveGroup::AddObjective` (`0x82d51850`).
pub fn add_objective(world: &mut World, group: u32, key: u64) {
    let Some(gi) = world.objectives.group_index(group) else { return };
    let (challenge, player) = (world.objectives.groups[gi].challenge, world.objectives.groups[gi].player);
    let objective = new_objective(world, key, challenge, player);
    let Some(gi) = world.objectives.group_index(group) else { return };
    world.objectives.groups[gi].objectives.push(objective);
    activate_objectives(world, group);
}

/// `cChallengeObjectiveGroup::AddPointsObjective(pointsRequirement)` (`0x82d51938`): the
/// requirement, the group's points objective made once from `global_points_objective` and
/// appended, then `ActivateObjectives`.
pub fn add_points_objective(world: &mut World, group: u32, requirement: u32) {
    let Some(gi) = world.objectives.group_index(group) else { return };
    world.objectives.groups[gi].points_requirement = requirement;
    if world.objectives.groups[gi].points_objective.is_none() {
        let (challenge, player) = (world.objectives.groups[gi].challenge, world.objectives.groups[gi].player);
        let objective = new_objective(world, hash64(GLOBAL_POINTS_OBJECTIVE), challenge, player);
        let Some(gi) = world.objectives.group_index(group) else { return };
        let g = &mut world.objectives.groups[gi];
        g.objectives.push(objective);
        g.points_objective = Some(g.objectives.len() - 1);
    }
    activate_objectives(world, group);
}

fn group_mut(world: &mut World, group: u32) -> Option<&mut ObjectiveGroup> {
    let i = world.objectives.group_index(group)?;
    Some(&mut world.objectives.groups[i])
}

/// `cChallengeObjective::SetObjectiveStatus` (`0x82d4bac8`).
pub fn set_status(world: &mut World, group: u32, index: usize, state: Status) {
    let window = world.objective_notification_window;
    let Some(g) = group_mut(world, group) else { return };
    let Some(o) = g.objectives.get_mut(index) else { return };
    if o.state != state {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "(__int64)(float)")]
        let frames = (window * 60.0) as u32;
        o.notify_window = frames;
    }
    o.state = state;
    o.dirty = true;
    let start = match state {
        Status::Active => o.activation_group,
        Status::Complete => o.completion_group,
        _ => None,
    };
    if let Some(p) = start {
        presentation::start(world, p);
    }
    // An Auto objective's indicators show while it is active or enabled and its
    // challenge's hulls are in.
    let Some((activity, key, indicator_group)) = objective_indicators(world, group, index) else { return };
    if visual_indicator_mode(&world.db, key) != visual_indicator_mode::AUTO {
        return;
    }
    if !matches!(state, Status::Active | Status::Enabled) {
        show_visual_indicators(world, indicator_group, key, false);
    } else if assets::are_hulls_loaded(world, activity) {
        show_visual_indicators(world, indicator_group, key, true);
    }
}

/// `cChallengeObjective::ActivateObjective` (`0x82d4c170`).
fn activate_objective(world: &mut World, group: u32, index: usize) -> bool {
    let db = std::sync::Arc::clone(&world.db);
    let Some(g) = group_mut(world, group) else { return false };
    let o = &mut g.objectives[index];
    match o.state {
        Status::Disabled => {
            let key = o.key;
            let mode = db.attribute(class::CHALLENGE_OBJECTIVE, key, field::ACTIVATION_MODE).and_then(|a| a.i32(0)).unwrap_or(0);
            if mode == 0 {
                set_status(world, group, index, Status::Active);
            } else {
                set_status(world, group, index, Status::Enabled);
                if let Some(g) = group_mut(world, group) {
                    g.objectives[index].in_scoring_boundary = false;
                }
                // ActivateObjective 0x82d4c230..0x82d4c248 rearms the scoring volume
                // even when the skater is still inside it. The next collision update
                // must activate the newly enabled objective (e.g. sponsor_me_03).
                #[expect(clippy::cast_precision_loss, reason = "ResetTriggerVolumeCollisionState 0x82e22688: (float)_u64tod")]
                let volume = scoring_boundary(&db, key) as f64 as f32;
                world.output(Output::ResetTriggerVolumeState { volume });
            }
            true
        }
        Status::Enabled | Status::Active => true,
        _ => false,
    }
}

fn objective_message(world: &mut World, class_name: &str, handle: usize, index: usize, extra: Vec<(&'static str, Field)>) {
    #[expect(clippy::cast_precision_loss, reason = "index")]
    let mut fields = vec![("ObjectiveGroup", Field::Pointer(handle)), ("ObjectiveIndex", Field::Number(index as f32))];
    fields.extend(extra);
    world.send(Event::Challenge(Message::new(class_name, fields)));
}

/// `cChallengeObjectiveGroup::ActivateObjectives` (`0x82d4c508`).
pub fn activate_objectives(world: &mut World, group: u32) {
    let Some(gi) = world.objectives.group_index(group) else { return };
    let g = &world.objectives.groups[gi];
    let (handle, flow, max_active, points, count) = (g.handle, g.flow_type, g.max_active, g.points_objective, g.objectives.len());
    if flow != 0 {
        let max_active = if max_active <= 0 { 1 } else { max_active };
        let mut active = 0;
        for i in 0..count {
            if Some(i) == points {
                continue;
            }
            if active >= max_active {
                let state = world.objectives.groups[gi].objectives[i].state;
                if matches!(state, Status::Enabled | Status::Active) {
                    set_status(world, group, i, Status::Disabled);
                    world.objectives.groups[gi].objectives[i].in_scoring_boundary = false;
                }
            } else if activate_objective(world, group, i) {
                active += 1;
                objective_message(world, "cMsgObjectiveActivate", handle, i, vec![("IsPointObjective", Field::Bool(false))]);
            }
        }
    } else {
        for i in 0..count {
            if activate_objective(world, group, i) {
                objective_message(world, "cMsgObjectiveActivate", handle, i, vec![("IsPointObjective", Field::Bool(false))]);
            }
        }
    }
    if let Some(p) = points
        && activate_objective(world, group, p)
    {
        objective_message(world, "cMsgObjectiveActivate", handle, p, vec![("IsPointObjective", Field::Bool(true))]);
    }
}

/// `SetEvaluationStatus` (`0x82d4b068`).
pub fn set_evaluation_status(world: &mut World, group: u32, status: bool) {
    let Some(g) = group_mut(world, group) else { return };
    if !g.evaluate && status {
        for o in &mut g.objectives {
            o.completion.evaluated = 0;
            o.completion.evaluated_true = 0;
            o.points_valid.evaluated = 0;
            o.points_valid.evaluated_true = 0;
            for h in o.failures.iter_mut().chain(o.failures_with_events.iter_mut()).chain(o.hints.iter_mut()) {
                h.evaluated = 0;
                h.evaluated_true = 0;
            }
        }
    }
    g.evaluate = status;
}

/// `eChallengeSkaterState`.
pub mod skater_state {
    pub const INVALID: u32 = 0;
    pub const ONBOARD: u32 = 1;
    pub const OFFBOARD: u32 = 2;
    pub const GRABBING_OBJECT: u32 = 3;
}

/// `cChallengeObjective::ResetObjectiveData(resetCumulativeData, resetObjectiveStatus)`
/// (`0x82d4cf18`) for the ported fields: `reset_status` runs `ResetObjectiveStatus`
/// (`0x82d4c050`) first; `reset_cumulative` also clears `mTotalPoints` and
/// `mTotalGrindLength`. The air, gesture, pumping, big-trick and contextual data it clears are
/// not ported.
fn reset_objective_data(world: &mut World, group: u32, index: usize, reset_cumulative: bool, reset_status: bool) {
    let Some(g) = group_mut(world, group) else { return };
    g.objectives[index].skater_state = skater_state::INVALID;
    if reset_status {
        reset_objective_status(world, group, index);
    }
    let Some(g) = group_mut(world, group) else { return };
    let o = &mut g.objectives[index];
    o.trigger_volumes.clear();
    o.sequence_volumes.clear();
    o.completed_grinds.clear();
    o.last_sequence_points = 0;
    o.grind_length = 0.0;
    o.max_grind_length_in_sequence = 0.0;
    o.total_grind_length_in_sequence = 0.0;
    o.completed_tricks.clear();
    o.just_set_session_marker = false;
    o.just_used_session_marker = false;
    o.num_pushes = 0;
    o.award_sequence_points_immediately = false;
    o.data.reset(reset_cumulative);
    if reset_cumulative {
        o.total_points = 0;
        o.total_grind_length = 0.0;
    }
}

/// `ClearPersistentData` (`0x82d4a938`): forget the vars table and release
/// the DMO proxies. Challenge natives run in the other Lua state, so the clear
/// is queued and applied to the objectives' state before its next evaluation.
fn clear_persistent_data(world: &mut World, group: u32, index: usize) {
    let Some(g) = group_mut(world, group) else { return };
    let o = &mut g.objectives[index];
    o.vars_in_vm = false;
    let id = o.unique_id;
    let proxies = std::mem::take(&mut o.dmo_proxy_ids);
    world.objective_vars_cleared.push(id);
    for proxy in proxies {
        super::dmo::release_trigger_proxy(world, proxy);
    }
}

/// `cChallengeObjective::ResetObjectiveStatus` (`0x82d4c050`): the presentation groups reset,
/// the scoring boundary's collision state reset (`cTriggerVolumeManager::
/// ResetTriggerVolumeCollisionState(tTriggerVolumeInstanceID)` `0x82e22688`, by the volume ID as
/// a float), then the status back to active or enabled.
fn reset_objective_status(world: &mut World, group: u32, index: usize) {
    let db = std::sync::Arc::clone(&world.db);
    let Some(g) = group_mut(world, group) else { return };
    let o = &mut g.objectives[index];
    let groups = [o.activation_group, o.completion_group, o.failure_group];
    o.in_scoring_boundary = false;
    let (key, state) = (o.key, o.state);
    for p in groups.into_iter().flatten() {
        presentation::reset_group(world, p);
    }
    #[expect(clippy::cast_precision_loss, reason = "(float)_u64tod")]
    let volume = scoring_boundary(&db, key) as f64 as f32;
    world.output(Output::ResetTriggerVolumeState { volume });
    if state != Status::Disabled {
        let mode = db.attribute(class::CHALLENGE_OBJECTIVE, key, field::ACTIVATION_MODE).and_then(|a| a.i32(0)).unwrap_or(0);
        set_status(world, group, index, if mode == 0 { Status::Active } else { Status::Enabled });
    }
}

/// `ObjectiveInfo.ActivationMode` of an objective.
fn activation_mode(db: &AttribDb, key: u64) -> i32 {
    db.attribute(class::CHALLENGE_OBJECTIVE, key, field::ACTIVATION_MODE).and_then(|a| a.i32(0)).unwrap_or(0)
}

/// `GetAllCurrentlyCollidingTriggers` (`0x82d4e268`): both lists become the
/// challenge-group triggers the player collides with, global then local id.
fn take_colliding_triggers(world: &mut World, group: u32, index: usize) {
    let player = world.objectives.group_index(group).map(|gi| world.objectives.groups[gi].player);
    let colliding = player
        .and_then(|player| world.player_states.iter().find(|(state_player, _)| *state_player == player))
        .map_or(&world.player, |(_, state)| state)
        .colliding_triggers
        .clone();
    let Some(g) = group_mut(world, group) else { return };
    let o = &mut g.objectives[index];
    o.trigger_volumes.clear();
    o.sequence_volumes.clear();
    for (guid, local) in colliding {
        o.trigger_volumes.push(guid);
        o.trigger_volumes.push(local);
        o.sequence_volumes.push(guid);
        o.sequence_volumes.push(local);
    }
}

/// Apply queued vars clears (`registry["h_<UniqueID>"] = nil`).
fn apply_vars_clears(lua: &mut Lua<World>, world: &mut World) {
    let registry = lua.registry();
    for id in std::mem::take(&mut world.objective_vars_cleared) {
        lua.table_mut(registry).set_str(&format!("h_{id:016X}"), Value::Nil);
    }
}

/// `cChallengeObjectiveGroup::ResetObjectives(resetCumulativeData)` (`0x82d4d4e8`): every
/// objective's data reset (`ResetObjectiveData(resetCumulativeData, false)`), back to disabled
/// with fresh vars, a tracked group's completed unique tricks cleared, then reactivated.
pub fn reset_objectives(world: &mut World, group: u32, reset_cumulative: bool) {
    let Some(gi) = world.objectives.group_index(group) else { return };
    for i in 0..world.objectives.groups[gi].objectives.len() {
        reset_objective_data(world, group, i, reset_cumulative, false);
        set_status(world, group, i, Status::Disabled);
        clear_persistent_data(world, group, i);
    }
    if let Some(g) = group_mut(world, group)
        && g.unique_trick_group.is_some()
    {
        g.completed_unique_tricks.clear();
    }
    activate_objectives(world, group);
}

/// `FillFirstUniqueTrickInSequence` (`0x82d43a20`): the sequence's first trick of the unique trick
/// group (`cChallengeTrickGroupManager::IsTrickInGroup`) not yet completed, while fewer than 32
/// are; none otherwise.
fn fill_first_unique_trick_in_sequence(world: &mut World, group: u32) {
    let Some(gi) = world.objectives.group_index(group) else { return };
    let g = &world.objectives.groups[gi];
    let (tricks, completed, unique_group) = (g.sequence_completed_tricks.clone(), g.completed_unique_tricks.clone(), g.unique_trick_group);
    let mut first = None;
    if let Some(unique_group) = unique_group {
        let db = std::sync::Arc::clone(&world.db);
        for trick in tricks {
            if world.trick_groups.is_trick_in_group(&db, trick, unique_group) && !completed.contains(&trick) && completed.len() < 32 {
                first = Some(trick);
                break;
            }
        }
    }
    world.objectives.groups[gi].last_completed_unique_trick = first;
}

/// `AddLastUniqueTrickToFilterList` (`0x82d4e8e8`): the last completed unique trick, if any, onto
/// the completed list; the list's length.
pub fn add_last_unique_trick_to_filter_list(world: &mut World, group: u32) -> usize {
    let Some(g) = group_mut(world, group) else { return 0 };
    if let Some(trick) = g.last_completed_unique_trick.take() {
        g.completed_unique_tricks.push(trick);
    }
    g.completed_unique_tricks.len()
}

/// `ObjectiveInfo.NumUniqueTrickDone`'s count (`0x82d49558`): the completed unique tricks, plus
/// the last one not added yet.
#[must_use]
pub fn num_unique_tricks_done(world: &World, group: u32) -> usize {
    world.objectives.group_index(group).map_or(0, |gi| {
        let g = &world.objectives.groups[gi];
        g.completed_unique_tricks.len() + usize::from(g.last_completed_unique_trick.is_some())
    })
}

/// `Objectives.ResetObjective(group, index, resetCumulative, resetStatus)`
/// (`0x82d4db98`): the objective's data (not its vars), then reactivation.
pub fn reset_objective(world: &mut World, group: u32, index: usize, reset_cumulative: bool, reset_status: bool) {
    let Some(gi) = world.objectives.group_index(group) else { return };
    if index >= world.objectives.groups[gi].objectives.len() {
        return;
    }
    reset_objective_data(world, group, index, reset_cumulative, reset_status);
    activate_objectives(world, group);
}

/// `DeleteObjectiveGroup` (`0x82d4f990`).
pub fn delete_group(world: &mut World, group: u32) {
    let Some(gi) = world.objectives.group_index(group) else { return };
    let g = world.objectives.groups.remove(gi);
    world.objective_vars_cleared.extend(g.objectives.iter().map(|o| o.unique_id));
    // ~cChallengeObjective 0x82d4cd40: DeleteGroup of its visual indicators.
    for indicator_group in g.objectives.iter().filter_map(|o| o.indicator_group) {
        world.indicators.delete_group(indicator_group);
    }
}

/// `ShutdownActivity` (`0x82d4dd48`): every group of the activity.
pub fn shutdown_activity(world: &mut World, activity: usize) {
    let ids: Vec<u32> = world.objectives.groups.iter().filter(|g| g.activity == activity).map(|g| g.id).collect();
    for id in ids {
        delete_group(world, id);
    }
}

/// `PersistentDataSerializer::PushVars` (`0x82eef1f8`): the objective's vars
/// table, `registry["h_<UniqueID>"]`.
fn push_vars(lua: &mut Lua<World>, world: &mut World, group: u32, index: usize) -> Value {
    let Some(gi) = world.objectives.group_index(group) else { return Value::Nil };
    let o = &mut world.objectives.groups[gi].objectives[index];
    o.vars_in_vm = true;
    let name = format!("h_{:016X}", o.unique_id);
    let registry = lua.registry();
    let existing = lua.table(registry).get_str(&name);
    if matches!(existing, Value::Table(_)) {
        return existing;
    }
    let t = lua.create_table();
    lua.table_mut(registry).set_str(&name, Value::Table(t));
    Value::Table(t)
}

/// `cChallengeObjectiveScriptHandler::ExecuteScript` (`0x82d4a9f0`) with
/// `Attach` (`0x82d44138`): compile `function %s( vars ) \n%s\n end` once per
/// function name, then call it with the vars table. An empty source or a
/// failed attach answers `ValueOnError`.
fn execute_script(lua: &mut Lua<World>, world: &mut World, handler: &ScriptHandler, vars: &Value) -> bool {
    let mut function = lua.get_global(&handler.function);
    if matches!(function, Value::Nil) {
        if handler.source.is_empty() {
            return handler.value_on_error;
        }
        let wrapped = format!("function {}( vars ) \n{}\n end", handler.function, handler.source);
        let attached = lua.load_source(wrapped.as_bytes(), &wrapped).and_then(|main| lua.call(world, &main, &[]).map(|_| ()));
        if let Err(e) = attached {
            world.output(Output::ScriptError { context: format!("objective {}", handler.function), error: e.to_string() });
            return handler.value_on_error;
        }
        function = lua.get_global(&handler.function);
    }
    match lua.call(world, &function, std::slice::from_ref(vars)) {
        Ok(results) => !matches!(results.first(), None | Some(Value::Nil | Value::Boolean(false))),
        Err(e) => {
            world.output(Output::ScriptError { context: format!("objective {}", handler.function), error: e.to_string() });
            handler.value_on_error
        }
    }
}

/// Which handler list a condition comes from.
#[derive(Clone, Copy)]
enum Which {
    Completion,
    Failure(usize),
    FailureWithEvents(usize),
    Hint(usize),
}

fn run_handler(lua: &mut Lua<World>, world: &mut World, group: u32, index: usize, which: Which, vars: &Value) -> bool {
    let Some(gi) = world.objectives.group_index(group) else { return false };
    let o = &world.objectives.groups[gi].objectives[index];
    let handler = match which {
        Which::Completion => o.completion.clone(),
        Which::Failure(i) => o.failures[i].clone(),
        Which::FailureWithEvents(i) => o.failures_with_events[i].clone(),
        Which::Hint(i) => o.hints[i].clone(),
    };
    let result = execute_script(lua, world, &handler, vars);
    if let Some(gi) = world.objectives.group_index(group) {
        let o = &mut world.objectives.groups[gi].objectives[index];
        let h = match which {
            Which::Completion => &mut o.completion,
            Which::Failure(i) => &mut o.failures[i],
            Which::FailureWithEvents(i) => &mut o.failures_with_events[i],
            Which::Hint(i) => &mut o.hints[i],
        };
        h.evaluated += 1;
        if result {
            h.evaluated_true += 1;
        }
    }
    result
}

/// `EvaluateFailureObjectives` (`0x82d4b158`): the index of the first failure
/// condition that holds, or -1.
fn evaluate_failures(lua: &mut Lua<World>, world: &mut World, group: u32, index: usize, vars: &Value) -> i32 {
    let Some(gi) = world.objectives.group_index(group) else { return -1 };
    let (with_events, plain) = {
        let o = &world.objectives.groups[gi].objectives[index];
        (o.failures_with_events.len(), o.failures.len())
    };
    for i in 0..with_events {
        if run_handler(lua, world, group, index, Which::FailureWithEvents(i), vars) {
            let gi = world.objectives.group_index(group).unwrap_or(0);
            if let Some(Some(p)) = world.objectives.groups.get(gi).and_then(|g| g.objectives[index].failure_groups.get(i).copied()) {
                presentation::start(world, p);
            }
            return i32::try_from(i).unwrap_or(i32::MAX);
        }
    }
    for i in 0..plain {
        if run_handler(lua, world, group, index, Which::Failure(i), vars) {
            let gi = world.objectives.group_index(group).unwrap_or(0);
            if let Some(p) = world.objectives.groups.get(gi).and_then(|g| g.objectives[index].failure_group) {
                presentation::start(world, p);
            }
            return i32::try_from(i).unwrap_or(i32::MAX);
        }
    }
    -1
}

/// `ExecuteHintObjectives` (`0x82d4b2c8`).
fn execute_hints(lua: &mut Lua<World>, world: &mut World, group: u32, index: usize, vars: &Value) {
    let Some(gi) = world.objectives.group_index(group) else { return };
    let count = world.objectives.groups[gi].objectives[index].hints.len();
    for i in 0..count {
        if run_handler(lua, world, group, index, Which::Hint(i), vars) {
            let gi = world.objectives.group_index(group).unwrap_or(0);
            if let Some(Some(p)) = world.objectives.groups.get(gi).and_then(|g| g.objectives[index].hint_groups.get(i).copied()) {
                presentation::start(world, p);
            }
            return;
        }
    }
}

/// `EvaluateCompletionObjective` (`0x82d4bfc0`): -2 when complete.
fn evaluate_completion(lua: &mut Lua<World>, world: &mut World, group: u32, index: usize, vars: &Value) -> i32 {
    if run_handler(lua, world, group, index, Which::Completion, vars) {
        set_status(world, group, index, Status::Complete);
        return -2;
    }
    -1
}

/// `DoFullCompletionTest` (`0x82d4d0d8`).
fn full_completion_test(lua: &mut Lua<World>, world: &mut World, group: u32, index: usize, sequence_complete: bool) -> i32 {
    let vars = push_vars(lua, world, group, index);
    let Some(gi) = world.objectives.group_index(group) else { return -1 };
    let key = world.objectives.groups[gi].objectives[index].key;
    let failure_first =
        world.db.attribute(class::CHALLENGE_OBJECTIVE, key, field::FAILURE_FIRST).and_then(|a| a.bool(0)).unwrap_or(false);
    let closure = Closure { group, objective: index, is_sequence_complete: sequence_complete, is_newly_completed: false };
    let previous = world.objective_closure.replace(closure);
    let result = if failure_first {
        execute_hints(lua, world, group, index, &vars);
        let failed = evaluate_failures(lua, world, group, index, &vars);
        if failed == -1 { evaluate_completion(lua, world, group, index, &vars) } else { failed }
    } else {
        let completed = evaluate_completion(lua, world, group, index, &vars);
        if completed == -1 {
            execute_hints(lua, world, group, index, &vars);
            evaluate_failures(lua, world, group, index, &vars)
        } else {
            completed
        }
    };
    // mAwardSequenceCompletePointsImmediately: cleared, and the objective's skater's
    // mFinishedSequenceScore (PhysOut_Scoring2 +0x68) joins mTotalPoints.
    if let Some(gi) = world.objectives.group_index(group) {
        let o = &mut world.objectives.groups[gi].objectives[index];
        if std::mem::take(&mut o.award_sequence_points_immediately) {
            let player = o.player;
            let score = skate::skater(world, player).map(|frame| frame.finished_sequence_score);
            if let Some(score) = score {
                let o = &mut world.objectives.groups[gi].objectives[index];
                o.total_points = o.total_points.wrapping_add(score);
            }
        }
    }
    world.objective_closure = previous;
    result
}

/// `cChallengeObjective::DoPointsValidTest(points, closureData)` (`0x82d4b398`): the
/// points-valid script in the objective's vars; a true result adds the points to
/// `mTotalPoints`.
fn do_points_valid_test(lua: &mut Lua<World>, world: &mut World, group: u32, index: usize, points: u32, closure: Closure) {
    let vars = push_vars(lua, world, group, index);
    let Some(gi) = world.objectives.group_index(group) else { return };
    let handler = world.objectives.groups[gi].objectives[index].points_valid.clone();
    let previous = world.objective_closure.replace(closure);
    let valid = execute_script(lua, world, &handler, &vars);
    world.objective_closure = previous;
    let Some(gi) = world.objectives.group_index(group) else { return };
    let o = &mut world.objectives.groups[gi].objectives[index];
    o.points_valid.evaluated += 1;
    if valid {
        o.points_valid.evaluated_true += 1;
        o.total_points = o.total_points.wrapping_add(points);
    }
}

/// `cChallengeObjectiveGroup::EvaluatePointsValid(points, isSequenceCompletePoints,
/// newlyCompletedObjectives, objectives)` (`0x82d4b740`) over the group's objectives: an
/// active one, or a complete one among the newly completed, takes `DoPointsValidTest`.
fn evaluate_points_valid(
    lua: &mut Lua<World>,
    world: &mut World,
    group: u32,
    points: u32,
    is_sequence_complete: bool,
    newly: Option<&[usize]>,
) {
    let Some(gi) = world.objectives.group_index(group) else { return };
    let count = world.objectives.groups[gi].objectives.len();
    for i in 0..count {
        let Some(gi) = world.objectives.group_index(group) else { return };
        let is_newly_completed = match world.objectives.groups[gi].objectives[i].state {
            Status::Active => false,
            Status::Complete if newly.is_some_and(|n| n.contains(&i)) => true,
            _ => continue,
        };
        let closure = Closure { group, objective: i, is_sequence_complete, is_newly_completed };
        do_points_valid_test(lua, world, group, i, points, closure);
    }
}

/// `EvaluateCompletionObjetive` (`0x82d4e978`); returns the objectives that
/// completed in this pass.
fn evaluate_group(lua: &mut Lua<World>, world: &mut World, group: u32, triggers: Triggers) -> Vec<usize> {
    let mut completed = Vec::new();
    let mut any = false;
    let Some(gi) = world.objectives.group_index(group) else { return completed };
    let count = world.objectives.groups[gi].objectives.len();
    let handle = world.objectives.groups[gi].handle;
    for i in 0..count {
        let Some(gi) = world.objectives.group_index(group) else { break };
        let (state, key) = {
            let o = &world.objectives.groups[gi].objectives[i];
            (o.state, o.key)
        };
        if state != Status::Active || !triggers.matches(Triggers::read(&world.db, key)) {
            continue;
        }
        let result = full_completion_test(lua, world, group, i, triggers.sequence_complete);
        if result != -2 {
            if result >= 0 {
                #[expect(clippy::cast_precision_loss, reason = "index")]
                let failed = Field::Number(result as f32);
                objective_message(world, "cMsgObjectiveFailed", handle, i, vec![("FailedIndex", failed)]);
            }
            continue;
        }
        let Some(gi) = world.objectives.group_index(group) else { break };
        if world.objectives.groups[gi].objectives[i].master {
            for j in 0..world.objectives.groups[gi].objectives.len() {
                if world.objectives.groups[gi].objectives[j].state != Status::Complete {
                    set_status(world, group, j, Status::Complete);
                    completed.push(j);
                }
            }
        }
        completed.push(i);
        let Some(gi) = world.objectives.group_index(group) else { break };
        let highlight =
            world.db.attribute(class::CHALLENGE_OBJECTIVE, key, field::REQUIRES_HIGHLIGHT).and_then(|a| a.bool(0)).unwrap_or(false);
        let is_points = world.objectives.groups[gi].points_objective == Some(i);
        // A points objective without a requirement completes without a message, but the pass
        // still counts a completion (0x82d4eb44).
        if is_points && world.objectives.groups[gi].points_requirement == 0 {
            any = true;
            continue;
        }
        objective_message(
            world,
            "cMsgObjectiveComplete",
            handle,
            i,
            vec![("RequiresHighlight", Field::Bool(highlight)), ("IsPointObjective", Field::Bool(is_points))],
        );
        any = true;
    }
    if any {
        on_objective_completed(world, group, &completed);
    }
    if all_complete_except_points(world, group) && triggers.group_objective_complete && completed.is_empty() {
        let player = world.objectives.group_index(group).map_or(0, |gi| world.objectives.groups[gi].player);
        #[expect(clippy::cast_precision_loss, reason = "player id")]
        let fields = vec![("ObjectiveGroup", Field::Pointer(handle)), ("PlayerID", Field::Number(player as f32))];
        world.send(Event::Challenge(Message::new("cMsgDidNotGetEnoughPoints", fields)));
    }
    completed
}

/// `OnObjectiveCompleted` (`0x82d4c918`).
fn on_objective_completed(world: &mut World, group: u32, completed: &[usize]) {
    let Some(gi) = world.objectives.group_index(group) else { return };
    let g = &world.objectives.groups[gi];
    if g.objectives.iter().all(|o| o.state == Status::Complete) {
        let handle = g.handle;
        world.send(Event::Challenge(Message::new("cMsgAllObjectiveComplete", vec![("ObjectiveGroup", Field::Pointer(handle))])));
    }
    if !completed.is_empty() {
        activate_objectives(world, group);
    }
}

/// `cChallengeObjectiveGroup::ForceComplete` (`0x82d4f480`): an objective not yet complete made
/// complete, `cMsgObjectiveComplete` without a highlight, then `OnObjectiveCompleted` of it.
pub fn force_complete(world: &mut World, group: u32, index: usize) {
    let Some(gi) = world.objectives.group_index(group) else { return };
    let g = &world.objectives.groups[gi];
    if g.objectives.get(index).is_none_or(|o| o.state == Status::Complete) {
        return;
    }
    let (handle, is_points) = (g.handle, g.points_objective == Some(index));
    set_status(world, group, index, Status::Complete);
    objective_message(
        world,
        "cMsgObjectiveComplete",
        handle,
        index,
        vec![("RequiresHighlight", Field::Bool(false)), ("IsPointObjective", Field::Bool(is_points))],
    );
    on_objective_completed(world, group, &[index]);
}

/// Diagnostics: an active objective completed as `EvaluateCompletionObjetive` (`0x82d4e978`)
/// completes one whose completion script holds: complete, `cMsgObjectiveComplete` with its
/// `RequiresHighlight`, then `OnObjectiveCompleted` of it.
pub fn diagnostic_complete(world: &mut World, group: u32, index: usize) {
    let Some(gi) = world.objectives.group_index(group) else { return };
    let g = &world.objectives.groups[gi];
    let Some(key) = g.objectives.get(index).filter(|o| o.state == Status::Active).map(|o| o.key) else { return };
    let (handle, is_points) = (g.handle, g.points_objective == Some(index));
    let highlight =
        world.db.attribute(class::CHALLENGE_OBJECTIVE, key, field::REQUIRES_HIGHLIGHT).and_then(|a| a.bool(0)).unwrap_or(false);
    set_status(world, group, index, Status::Complete);
    objective_message(
        world,
        "cMsgObjectiveComplete",
        handle,
        index,
        vec![("RequiresHighlight", Field::Bool(highlight)), ("IsPointObjective", Field::Bool(is_points))],
    );
    on_objective_completed(world, group, &[index]);
}

/// `AreAllCompleteExceptPointsObjective` (`0x82d43d60`): true only when the
/// points objective exists and is not complete while all others are.
fn all_complete_except_points(world: &World, group: u32) -> bool {
    let Some(gi) = world.objectives.group_index(group) else { return false };
    let g = &world.objectives.groups[gi];
    let mut has_points = false;
    for (i, o) in g.objectives.iter().enumerate() {
        if Some(i) == g.points_objective {
            has_points = true;
            if o.state == Status::Complete {
                return false;
            }
        } else if o.state != Status::Complete {
            return false;
        }
    }
    has_points
}

/// `cChallengeObjectiveGroup::GetDisplayedObjectives` (`0x82d43658`): the
/// objectives the HUD lists, at most `max`, in order. While a listed objective
/// is in its notification window (and `ShowActiveWithNotifications` is off),
/// enabled and active objectives are left out.
#[must_use]
pub fn displayed_objectives(world: &World, group: u32, max: usize) -> Vec<usize> {
    let Some(gi) = world.objectives.group_index(group) else { return Vec::new() };
    let g = &world.objectives.groups[gi];
    let shown = |o: &Objective| {
        world.db.attribute(class::CHALLENGE_OBJECTIVE, o.key, field::DISPLAY_IN_HUD).is_none_or(|a| a.bool(0).unwrap_or(false))
    };
    let notifying = |o: &Objective| {
        let state = o.state as u32;
        g.notification_statuses & state == state && o.notify_window != 0
    };
    let notify = !g.show_active_with_notifications && g.objectives.iter().any(|o| shown(o) && notifying(o));
    let mut listed = Vec::new();
    for (i, o) in g.objectives.iter().enumerate() {
        if listed.len() >= max {
            break;
        }
        let state = o.state as u32;
        let allowed = !notify || state & 6 == 0;
        if shown(o) && g.points_objective != Some(i) && allowed && (state == g.display_statuses & state || notifying(o)) {
            listed.push(i);
        }
    }
    listed
}

/// `cChallengeObjectiveGroup::GetAccumulatedPoints` (`0x82d43b50`): every
/// objective's `mTotalPoints` (its argument is unused).
#[must_use]
pub fn accumulated_points(world: &World, group: u32) -> u32 {
    world.objectives.group_index(group).map_or(0, |gi| world.objectives.groups[gi].objectives.iter().map(|o| o.total_points).sum())
}

/// `cChallengeObjectiveGroup::GetNumCompletedObjectives(includeOPO, includeFractional)`
/// (`0x82d43e10`): the complete objectives, the points objective only with `include_opo` and
/// those with a fraction denominator above 1 only with `include_fractional`.
#[must_use]
pub fn num_completed_objectives(world: &World, group: u32, include_opo: bool, include_fractional: bool) -> usize {
    let Some(gi) = world.objectives.group_index(group) else { return 0 };
    let g = &world.objectives.groups[gi];
    g.objectives
        .iter()
        .enumerate()
        .filter(|(_, o)| o.state == Status::Complete)
        .filter(|&(i, o)| {
            if g.points_objective == Some(i) {
                return include_opo;
            }
            let denominator =
                world.db.attribute(class::CHALLENGE_OBJECTIVE, o.key, field::FRACTION_DENOMINATOR).and_then(|a| a.i16(0)).unwrap_or(0);
            denominator <= 1 || include_fractional
        })
        .count()
}

/// `GetIfAnyObjectivesAreDirty`.
#[must_use]
pub fn any_dirty(world: &World, group: u32) -> bool {
    world.objectives.group_index(group).is_some_and(|gi| world.objectives.groups[gi].objectives.iter().any(|o| o.dirty))
}

/// `cChallengeObjectiveGroup::ResetDirty` (`0x82d439c0`).
pub fn reset_dirty(world: &mut World, group: u32) {
    if let Some(g) = group_mut(world, group) {
        for o in &mut g.objectives {
            o.dirty = false;
        }
    }
}

/// One evaluation pass and the `GroupObjectiveComplete` follow-up pass.
fn evaluate_with_follow_up(lua: &mut Lua<World>, world: &mut World, group: u32, triggers: Triggers) {
    let completed = evaluate_group(lua, world, group, triggers);
    if !completed.is_empty() {
        evaluate_group(lua, world, group, Triggers::group_objective_complete());
    }
}

/// `cChallengeObjectivesManager::Update` (`0x82d4fa58`) and
/// `cChallengeObjectiveGroup::Update` (`0x82d4f608`).
pub fn update(lua: &mut Lua<World>, world: &mut World) {
    apply_vars_clears(lua, world);
    let groups: Vec<_> = world.objectives.groups.iter().filter(|g| g.evaluate).map(|g| (g.id, g.player)).collect();
    for (group, player) in groups {
        // FindByIndex(ActivePlayer), 82d4f638..82d4f6b8. A group's continuous
        // triggers belong to its assigned actor, including AI/Party Play turns.
        if let Some(frame) = world.skaters.iter().find(|frame| frame.player == player) {
            let triggers = Triggers {
                continuous_standard: true,
                continuous_wipeout: world.hom_enabled && (frame.hom_bail_in_progress || frame.hom_bail_completed),
                ..Triggers::default()
            };
            evaluate_with_follow_up(lua, world, group, triggers);
        }
        if let Some(g) = group_mut(world, group) {
            for o in &mut g.objectives {
                // cChallengeObjective::Update 0x82d430d0.
                if o.notify_window != 0 {
                    o.notify_window -= 1;
                    if o.notify_window == 0 {
                        o.dirty = true;
                    }
                }
                // ClearSessionMarkerData 0x82d42d70.
                o.just_set_session_marker = false;
                o.just_used_session_marker = false;
            }
        }
    }
}

/// `ObjectiveInfo.ScoringBoundary`'s volume ID (its second eight bytes).
fn scoring_boundary(db: &AttribDb, key: u64) -> u64 {
    db.attribute(class::CHALLENGE_OBJECTIVE, key, field::SCORING_BOUNDARY)
        .and_then(|a| a.value_bytes(0))
        .map_or(0, |b| u64::from_be_bytes([b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]]))
}

/// `IsTriggerVolume`: a 64-bit volume id against a script's float id.
#[must_use]
#[expect(clippy::cast_precision_loss, reason = "(float)_u64tod")]
#[expect(clippy::float_cmp, reason = "the native compares exactly")]
pub fn is_trigger_volume(id: u64, volume: f32) -> bool {
    (id as f64 as f32) == volume
}

/// The objectives manager's Hermes handlers.
pub fn receive(lua: &mut Lua<World>, world: &mut World, event: &Event) {
    apply_vars_clears(lua, world);
    if let Event::TriggerCollision { enter, volume, player } = *event {
        let ids: Vec<u32> = world.objectives.groups.iter().filter(|g| g.evaluate && g.player == player).map(|g| g.id).collect();
        for group in ids {
            if enter {
                trigger_enter(world, group, volume);
                evaluate_with_follow_up(lua, world, group, Triggers { trigger_enter: true, ..Triggers::default() });
            } else {
                trigger_exit(world, group, volume);
            }
        }
        return;
    }
    let Event::Challenge(message) = event else { return };
    let number = |name: &str| -> Option<f32> {
        message.fields.as_ref()?.iter().find(|(n, _)| *n == name).and_then(|(_, v)| match v {
            Field::Number(n) => Some(*n),
            _ => None,
        })
    };
    #[expect(clippy::cast_possible_truncation, reason = "player id")]
    let player = number("PlayerID").map_or(-1, |p| p as i32);
    let kind = message.kind;
    let ids: Vec<u32> = world.objectives.groups.iter().filter(|g| g.evaluate && g.player == player).map(|g| g.id).collect();
    if kind == super::keys::hash32("cMsgSetMarker") || kind == super::keys::hash32("cMsgUseMarker") {
        let set = kind == super::keys::hash32("cMsgSetMarker");
        for group in ids {
            if let Some(g) = group_mut(world, group) {
                for o in g.objectives.iter_mut().filter(|o| o.state == Status::Active) {
                    if set {
                        o.just_set_session_marker = true;
                    } else {
                        o.just_used_session_marker = true;
                    }
                }
            }
            evaluate_with_follow_up(lua, world, group, Triggers { session_marker_set: true, ..Triggers::default() });
        }
    } else if kind == super::keys::hash32("cMsgSkaterOffboard") || kind == super::keys::hash32("cMsgSkaterOnboard") {
        let state = if kind == super::keys::hash32("cMsgSkaterOffboard") { skater_state::OFFBOARD } else { skater_state::ONBOARD };
        for group in ids {
            if let Some(g) = group_mut(world, group) {
                for o in &mut g.objectives {
                    o.skater_state = state;
                }
            }
            evaluate_with_follow_up(lua, world, group, Triggers { skater_state_change: true, ..Triggers::default() });
        }
    } else if kind == super::keys::hash32("cMsgSkaterGrabbingObject") {
        // OnSkaterGrabbingObject 0x82d52350.
        for group in ids {
            if let Some(g) = group_mut(world, group) {
                for o in &mut g.objectives {
                    o.skater_state = skater_state::GRABBING_OBJECT;
                }
            }
            evaluate_with_follow_up(lua, world, group, Triggers { skater_state_change: true, ..Triggers::default() });
        }
    } else if kind == super::keys::hash32("cMsgTrickComplete") {
        // OnTrickComplete 0x82d4ef58 -> cChallengeObjective::TrickCompleteMsg
        // 0x82d4dfd0, whose big-trick achievements read the output of a player the skater
        // manager counts.
        let key = message.fields.as_ref().and_then(|f| f.iter().find(|(n, _)| *n == "TrickAttribKey")).and_then(|(_, v)| match v {
            Field::Key(k) => Some(*k),
            _ => None,
        });
        let Some(key) = key else { return };
        #[expect(clippy::cast_possible_truncation, reason = "trick type")]
        let trick_type = number("TrickType").map_or(0, |t| t as i32);
        let counted = usize::try_from(player).is_ok_and(|p| p < world.skaters.len());
        let frame = world.skaters.iter().find(|f| f.player == player).filter(|_| counted).cloned();
        for group in ids {
            if let Some(g) = group_mut(world, group) {
                for o in g.objectives.iter_mut().filter(|o| o.state == Status::Active) {
                    if o.completed_tricks.len() == 32 {
                        o.completed_tricks.pop_front();
                    }
                    o.completed_tricks.push_back(key);
                    if let Some(frame) = &frame {
                        o.data.trick_completed(frame, trick_type);
                    }
                }
                if g.unique_trick_group.is_some() && g.sequence_completed_tricks.len() != 12 {
                    g.sequence_completed_tricks.push(key);
                }
            }
        }
    } else if kind == super::keys::hash32("cMsgSequenceBegin") {
        // OnSequenceBegin 0x82d51c78 -> SequenceBegin 0x82d50fc0.
        let db = std::sync::Arc::clone(&world.db);
        for group in ids {
            let Some(gi) = world.objectives.group_index(group) else { continue };
            for i in 0..world.objectives.groups[gi].objectives.len() {
                let o = &mut world.objectives.groups[gi].objectives[i];
                o.sequence_volumes.clear();
                o.last_sequence_points = 0;
                o.total_grind_length_in_sequence = 0.0;
                o.max_grind_length_in_sequence = 0.0;
                if activation_mode(&db, o.key) == 4 && o.state == Status::Enabled {
                    set_status(world, group, i, Status::Active);
                    take_colliding_triggers(world, group, i);
                }
            }
        }
    } else if kind == super::keys::hash32("cMsgSequenceComplete") {
        // OnSequenceComplete 0x82d51cf8: an active objective's sequence grind length and
        // points, the sequence-complete pass, EvaluatePointsValid over what it completed, the
        // group-complete pass, then SequenceCompletePostUpdate (0x82d4d038) of every objective;
        // a tracked group first fills its first unique trick, and every group's sequence tricks
        // clear at the end.
        let flag = |name: &str| {
            message.fields.as_ref().and_then(|f| f.iter().find(|(n, _)| *n == name)).is_some_and(|(_, v)| matches!(v, Field::Bool(true)))
        };
        let (wiped_out, used_marker) = (flag("Wipedout"), flag("UsedSessionMarker"));
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "sequence score")]
        let score = number("SequenceScore").unwrap_or(0.0) as u32;
        let db = std::sync::Arc::clone(&world.db);
        for group in ids {
            if let Some(g) = group_mut(world, group) {
                for o in g.objectives.iter_mut().filter(|o| o.state == Status::Active) {
                    if !wiped_out && !used_marker {
                        o.total_grind_length = o.total_grind_length_in_sequence + o.total_grind_length;
                        o.last_sequence_points = score;
                    }
                }
            }
            if !wiped_out && !used_marker {
                if world.objectives.group_index(group).is_some_and(|gi| world.objectives.groups[gi].unique_trick_group.is_some()) {
                    fill_first_unique_trick_in_sequence(world, group);
                }
                let completed = evaluate_group(lua, world, group, Triggers { sequence_complete: true, ..Triggers::default() });
                evaluate_points_valid(lua, world, group, score, true, Some(&completed));
                if !completed.is_empty() {
                    evaluate_group(lua, world, group, Triggers::group_objective_complete());
                }
            }
            let Some(gi) = world.objectives.group_index(group) else { continue };
            for i in 0..world.objectives.groups[gi].objectives.len() {
                let Some(gi) = world.objectives.group_index(group) else { break };
                let o = &mut world.objectives.groups[gi].objectives[i];
                if activation_mode(&db, o.key) == 4 && o.state == Status::Active {
                    o.completed_tricks.clear();
                    reset_objective_status(world, group, i);
                }
            }
            if let Some(g) = group_mut(world, group) {
                g.sequence_completed_tricks.clear();
            }
        }
    } else if kind == super::keys::hash32("cMsgGrindCompleted") {
        // OnGrindCompleted 0x82d524b0: each objective's GrindCompletedMsg (0x82d51140), with its
        // big-trick achievement and DMO grind context, then the grind-complete pass.
        let key = |name: &str| {
            message.fields.as_ref().and_then(|f| f.iter().find(|(n, _)| *n == name)).and_then(|(_, v)| match v {
                Field::Key(k) => Some(*k),
                _ => None,
            })
        };
        let wiped_out = number("Wipedout").is_some_and(|w| w != 0.0);
        let used_marker = message
            .fields
            .as_ref()
            .and_then(|f| f.iter().find(|(n, _)| *n == "UsedSessionMarker"))
            .is_some_and(|(_, v)| matches!(v, Field::Bool(true)));
        let length = number("GrindLength").unwrap_or(0.0);
        let splines = [key("PathedSplineID").unwrap_or(0), key("LocalSplineID").unwrap_or(0)];
        let frame = world.skaters.iter().find(|f| f.player == player).cloned();
        for group in ids {
            if let Some(g) = group_mut(world, group) {
                for o in g.objectives.iter_mut().filter(|o| o.state == Status::Active) {
                    if wiped_out || used_marker {
                        continue;
                    }
                    let total = o.total_grind_length_in_sequence + length;
                    o.grind_length = length;
                    o.total_grind_length_in_sequence = total;
                    if length > o.max_grind_length_in_sequence {
                        o.max_grind_length_in_sequence = length;
                    }
                    for spline in splines {
                        if !o.completed_grinds.contains(&spline) && o.completed_grinds.len() < COMPLETED_GRINDS_CAPACITY {
                            o.completed_grinds.push(spline);
                        }
                    }
                    if let Some(frame) = &frame {
                        o.data.grind_completed(frame);
                    }
                }
            }
            if !wiped_out && !used_marker {
                evaluate_with_follow_up(lua, world, group, Triggers { grind_complete: true, ..Triggers::default() });
            }
        }
    } else if kind == super::keys::hash32("cMsgAirCompleted") {
        // OnAirCompleted 0x82d52630: each active objective's AirCompletedMsg (0x82d51368) with
        // the skater's output after a clean landing, then the air-complete pass and, when it
        // completed objectives, the group-complete pass.
        let wiped_out = number("Wipedout").is_some_and(|w| w != 0.0);
        let used_marker = message
            .fields
            .as_ref()
            .and_then(|f| f.iter().find(|(n, _)| *n == "UsedSessionMarker"))
            .is_some_and(|(_, v)| matches!(v, Field::Bool(true)));
        let air = world.skaters.iter().find(|f| f.player == player).and_then(|f| f.finished_air);
        for group in ids {
            if let Some(air) = &air
                && !wiped_out
                && !used_marker
                && let Some(g) = group_mut(world, group)
            {
                for o in g.objectives.iter_mut().filter(|o| o.state == Status::Active) {
                    o.data.air_completed(air);
                }
            }
            if !wiped_out && !used_marker {
                let completed = evaluate_group(lua, world, group, Triggers { air_complete: true, ..Triggers::default() });
                if !completed.is_empty() {
                    evaluate_group(lua, world, group, Triggers::group_objective_complete());
                }
            }
        }
    } else if kind == super::keys::hash32("cMsgGesturePerformed") {
        // OnGesturePerformed 0x82d527e0 -> GesturePerformedMsg 0x82d515c8.
        #[expect(clippy::cast_possible_truncation, reason = "gesture")]
        let gesture = number("Gesture").map_or(0, |g| g as i32);
        for group in ids {
            if let Some(g) = group_mut(world, group) {
                for o in g.objectives.iter_mut().filter(|o| o.state == Status::Active) {
                    o.data.gesture_performed(gesture);
                }
            }
        }
    } else if kind == super::keys::hash32("cMsgPlayerResult") {
        // PlayerResultMsg 0x82d4a358 -> OnPlayerResult 0x82d43bc0: every group's objectives,
        // whatever their player.
        #[expect(clippy::cast_possible_truncation, reason = "the message's ints")]
        let (round, rank, score) = (
            number("Round").map_or(0, |r| r as i32),
            number("Rank").map_or(0, |r| r as i32),
            number("Score").map_or(0, |s| s as i32),
        );
        for g in &mut world.objectives.groups {
            for o in &mut g.objectives {
                o.data.player_result(round, rank, score);
            }
        }
    } else if kind == super::keys::hash32("cMsgPlayerOverallResult") {
        // PlayerOverallResultMsg 0x82d4a2e8 -> OnPlayerOverallResult 0x82d43c70.
        #[expect(clippy::cast_possible_truncation, reason = "the message's int")]
        let rank = number("OverallRank").map_or(0, |r| r as i32);
        for g in &mut world.objectives.groups {
            for o in &mut g.objectives {
                o.data.overall_rank = rank;
            }
        }
    } else if kind == super::keys::hash32("cMsgPointsAwarded") {
        // PointsAwardedMsg 0x82d4cca8: EvaluatePointsValid of the score, not as sequence
        // points, over the active objectives.
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "awarded score")]
        let score = number("Score").unwrap_or(0.0) as i32 as u32;
        for group in ids {
            evaluate_points_valid(lua, world, group, score, false, None);
        }
    } else if kind == super::keys::hash32("cMsgWipeout") {
        // OnWipeout 0x82d51a60 -> WipeoutStart 0x82d51070.
        let teleport = message
            .fields
            .as_ref()
            .and_then(|f| f.iter().find(|(n, _)| *n == "IsAPhysicsTeleport"))
            .is_some_and(|(_, v)| matches!(v, Field::Bool(true)));
        let db = std::sync::Arc::clone(&world.db);
        for group in ids {
            let Some(gi) = world.objectives.group_index(group) else { continue };
            for i in 0..world.objectives.groups[gi].objectives.len() {
                let (key, state) = {
                    let o = &world.objectives.groups[gi].objectives[i];
                    (o.key, o.state)
                };
                if state != Status::Enabled || teleport {
                    continue;
                }
                if activation_mode(&db, key) == 3 {
                    set_status(world, group, i, Status::Active);
                    take_colliding_triggers(world, group, i);
                }
                if Triggers::read(&db, key).continuous_wipeout {
                    clear_persistent_data(world, group, i);
                }
            }
        }
    } else if kind == super::keys::hash32("cMsgWipeoutOver") {
        // OnWipeoutOver 0x82d51ae0.
        let db = std::sync::Arc::clone(&world.db);
        for group in ids {
            if world.hom_enabled {
                evaluate_with_follow_up(lua, world, group, Triggers { wiped_out: true, ..Triggers::default() });
            }
            let Some(gi) = world.objectives.group_index(group) else { continue };
            for i in 0..world.objectives.groups[gi].objectives.len() {
                let (key, state) = {
                    let o = &world.objectives.groups[gi].objectives[i];
                    (o.key, o.state)
                };
                if activation_mode(&db, key) == 3 && state == Status::Active {
                    reset_objective_status(world, group, i);
                }
            }
        }
    } else if kind == super::keys::hash32("cMsgSkaterPushed") {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "push count")]
        let pushes = number("PushAttempts").unwrap_or(0.0) as u32;
        for group in ids {
            if let Some(g) = group_mut(world, group) {
                for o in g.objectives.iter_mut().filter(|o| o.state == Status::Active) {
                    o.num_pushes += pushes;
                }
            }
            evaluate_with_follow_up(lua, world, group, Triggers { braked_or_pushed: true, ..Triggers::default() });
        }
    }
}

/// `cChallengeObjective::TriggerEnterMsg` (`0x82d4de10`) for every objective.
fn trigger_enter(world: &mut World, group: u32, volume: u64) {
    let db = std::sync::Arc::clone(&world.db);
    let Some(gi) = world.objectives.group_index(group) else { return };
    for i in 0..world.objectives.groups[gi].objectives.len() {
        let (key, state, in_boundary) = {
            let o = &world.objectives.groups[gi].objectives[i];
            (o.key, o.state, o.in_scoring_boundary)
        };
        let mode = db.attribute(class::CHALLENGE_OBJECTIVE, key, field::ACTIVATION_MODE).and_then(|a| a.i32(0)).unwrap_or(0);
        if (mode == 1 && !in_boundary) || (mode == 2 && state == Status::Enabled) {
            let boundary = db
                .attribute(class::CHALLENGE_OBJECTIVE, key, field::SCORING_BOUNDARY)
                .and_then(|a| a.value_bytes(0))
                .map_or(0, |b| u64::from_be_bytes([b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]]));
            if boundary == volume {
                world.objectives.groups[gi].objectives[i].in_scoring_boundary = true;
                if state == Status::Enabled {
                    set_status(world, group, i, Status::Active);
                }
            }
        } else if state == Status::Active {
            let o = &mut world.objectives.groups[gi].objectives[i];
            if o.trigger_volumes.len() == 32 {
                o.trigger_volumes.remove(0);
            }
            o.trigger_volumes.push(volume);
            o.sequence_volumes.retain(|&v| v != volume);
            o.sequence_volumes.push(volume);
        }
    }
}

/// `cChallengeObjective::TriggerExitMsg` (`0x82d4bd58`): leaving the scoring
/// boundary fails or re-enables the objective.
fn trigger_exit(world: &mut World, group: u32, volume: u64) {
    let db = std::sync::Arc::clone(&world.db);
    let Some(gi) = world.objectives.group_index(group) else { return };
    let (handle, player) = (world.objectives.groups[gi].handle, world.objectives.groups[gi].player);
    for i in 0..world.objectives.groups[gi].objectives.len() {
        let (key, state, in_boundary) = {
            let o = &world.objectives.groups[gi].objectives[i];
            (o.key, o.state, o.in_scoring_boundary)
        };
        if !in_boundary {
            continue;
        }
        let boundary = db
            .attribute(class::CHALLENGE_OBJECTIVE, key, field::SCORING_BOUNDARY)
            .and_then(|a| a.value_bytes(0))
            .map_or(0, |b| u64::from_be_bytes([b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]]));
        if boundary != volume {
            continue;
        }
        world.objectives.groups[gi].objectives[i].in_scoring_boundary = false;
        let mode = db.attribute(class::CHALLENGE_OBJECTIVE, key, field::ACTIVATION_MODE).and_then(|a| a.i32(0)).unwrap_or(0);
        if mode == 1 && state == Status::Active {
            if Triggers::read(&db, key).fail_on_boundary_exit {
                set_status(world, group, i, Status::Failed);
                #[expect(clippy::cast_precision_loss, reason = "player id")]
                let fields = vec![("ObjectiveGroup", Field::Pointer(handle)), ("PlayerID", Field::Number(player as f32))];
                world.send(Event::Challenge(Message::new("cMsgLeftObjectiveScoringBoundary", fields)));
            } else {
                set_status(world, group, i, Status::Enabled);
            }
        }
    }
}

/// The objective's `mTitle`: its definition's HAL key (+12) and format
/// arguments (+16).
#[must_use]
pub fn title(db: &AttribDb, class: u64, objective: u64) -> (String, String) {
    let definition = db.attribute(class, objective, field::OBJECTIVE_DEFINITION).and_then(|a| a.data(0));
    let text = |offset: u32| definition.and_then(|at| db.text_at(at.offset_by(offset))).unwrap_or_default().to_owned();
    (text(12), text(16))
}

/// `text` of the objective's HUD string, for presentation.
#[must_use]
pub fn hal_string(db: &AttribDb, objective: u64) -> Option<String> {
    let at = db.attribute(class::CHALLENGE_OBJECTIVE, objective, field::OBJECTIVE_DEFINITION)?.data(0)?;
    db.text_at(at.offset_by(12)).map(str::to_owned)
}

#[must_use]
pub fn objective_name(db: &AttribDb, objective: u64) -> Option<String> {
    text(db, class::CHALLENGE_OBJECTIVE, objective, super::runtime::field::NAME)
}
