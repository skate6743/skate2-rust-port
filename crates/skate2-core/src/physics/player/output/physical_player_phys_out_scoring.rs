//! Complete retained Scoring2 constructor fields (82e2c768) and Reset82e2a3f8.
//! Native empty lists retain all four constructed Scorable elements. Reset clears
//! counts, preserving inactive elements and context UID storage exactly.
use super::{SkeletonRootVector, ZERO};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct IntentTuple {
    pub(super) skater_intent: i32,
    pub(super) value: f32,
    pub(super) start_frame: i32,
    pub(super) end_frame: i32,
}
impl IntentTuple {
    pub(super) fn new() -> Self {
        Self {
            skater_intent: 0,
            value: 1.0,
            start_frame: 0,
            end_frame: -1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TrickInput {
    pub(super) gesture: i32,
    pub(super) button: i32,
    pub(super) connector: Option<i32>,
}
impl TrickInput {
    pub(super) fn new() -> Self {
        Self {
            gesture: 0,
            button: 0,
            connector: Some(0),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TrickSettings {
    pub(super) trick_name_hal: Option<std::sync::Arc<str>>,
    pub(super) base_score: i32,
    pub(super) trick_index: u32,
    pub(super) scorable_id: i32,
    pub(super) skater_intent_tuples: [IntentTuple; 6],
    pub(super) fakie_variant_id: i32,
    pub(super) switch_variant_id: Option<i32>,
    pub(super) trick_type: i32,
    pub(super) is_valid: bool,
    pub(super) difficulty: i32,
    pub(super) trick_tutorial_type: i32,
    pub(super) trick_inputs: [TrickInput; 4],
    pub(super) trick_inputs_goofy: [TrickInput; 4],
    pub(super) trick_description_hal: Option<std::sync::Arc<str>>,
    pub(super) substitute_trick_group: u32,
    pub(super) min_trick_time: f32,
    pub(super) trick_book_sorting_index: u32,
    pub(super) trick_gesture: i32,
    pub(super) trick_attrib_key: u64,
    pub(super) max_impending_trick_frames: u32,
}
impl TrickSettings {
    pub(super) fn new() -> Self {
        Self {
            trick_name_hal: None,
            base_score: 0,
            trick_index: 0,
            scorable_id: -1,
            skater_intent_tuples: std::array::from_fn(|_| IntentTuple::new()),
            fakie_variant_id: -1,
            switch_variant_id: Some(0),
            trick_type: 0,
            is_valid: false,
            difficulty: 3,
            trick_tutorial_type: 0,
            trick_inputs: std::array::from_fn(|_| TrickInput::new()),
            trick_inputs_goofy: std::array::from_fn(|_| TrickInput::new()),
            trick_description_hal: None,
            substitute_trick_group: 0,
            min_trick_time: 0.0,
            trick_book_sorting_index: 1,
            trick_gesture: 0,
            trick_attrib_key: 0,
            max_impending_trick_frames: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
pub(super) struct Scorable {
    pub(super) trick_scoring_settings: TrickSettings,
    pub(super) point_rate_scalar: f32,
    pub(super) point_scalar: f32,
    pub(super) score: f32,
    pub(super) start_tick: u32,
    pub(super) min_trick_time_in_ticks: u32,
    pub(super) is_beginning_announced: bool,
    pub(super) is_ending_announced: bool,
    pub(super) was_a_tap_trick: bool,
    pub(super) started_switched: bool,
    pub(super) started_fakie: bool,
    pub(super) had_object_between_board_and_skater: bool,
}
impl Scorable {
    pub(super) fn new() -> Self {
        Self {
            trick_scoring_settings: TrickSettings::new(),
            point_rate_scalar: 0.0,
            point_scalar: 0.0,
            score: 0.0,
            start_tick: 0,
            min_trick_time_in_ticks: 0,
            is_beginning_announced: false,
            is_ending_announced: false,
            was_a_tap_trick: false,
            started_switched: false,
            started_fakie: false,
            had_object_between_board_and_skater: false,
        }
    }
    /// PhysicsScoring allocations are not cleared, unlike Sim-owned output.
    /// Share the record/algorithms while retaining unwritten-field provenance.
    pub(super) fn new_noncleared() -> Self {
        let mut value=Self::new();
        value.trick_scoring_settings.switch_variant_id=None;
        for input in value.trick_scoring_settings.trick_inputs.iter_mut().chain(value.trick_scoring_settings.trick_inputs_goofy.iter_mut()) {
            input.connector=None;
        }
        value
    }
    pub(super) fn reset(&mut self) {
        self.trick_scoring_settings.is_valid = false;
        self.point_rate_scalar = 0.0;
        self.point_scalar = 0.0;
        self.score = 0.0;
        self.start_tick = 0;
        self.min_trick_time_in_ticks = 0;
        self.is_beginning_announced = false;
        self.is_ending_announced = false;
        self.was_a_tap_trick = false;
        self.started_switched = false;
        self.started_fakie = false;
        self.had_object_between_board_and_skater = false;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ContextUid {
    pub(super) unique_id: u32,
    pub(super) entity_type: i32,
}
impl ContextUid {
    pub(super) fn new() -> Self {
        Self {
            unique_id: 0,
            entity_type: -1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ContextUids {
    pub(super) uids: [ContextUid; 8],
    pub(super) count: u32,
}
impl ContextUids {
    pub(super) fn new() -> Self {
        Self {
            uids: std::array::from_fn(|_| ContextUid::new()),
            count: 0,
        }
    }
    pub(super) fn reset(&mut self) {
        self.count = 0;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ContextData {
    pub(super) uid_data: [ContextUids; 2],
}
impl ContextData {
    pub(super) fn new() -> Self {
        Self {
            uid_data: std::array::from_fn(|_| ContextUids::new()),
        }
    }
    pub(super) fn reset(&mut self) {
        for value in &mut self.uid_data {
            value.reset();
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ContextGroups {
    pub(super) context_sensitive_data: [ContextData; 5],
}
impl ContextGroups {
    pub(super) fn new() -> Self {
        Self {
            context_sensitive_data: std::array::from_fn(|_| ContextData::new()),
        }
    }
    pub(super) fn reset(&mut self) {
        for value in &mut self.context_sensitive_data {
            value.reset();
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Threshold {
    pub(super) value: f32,
    pub(super) was_achieved: [bool; 2],
}
impl Threshold {
    pub(super) fn new() -> Self {
        Self {
            value: 0.0,
            was_achieved: [false; 2],
        }
    }
    pub(super) fn reset(&mut self) {
        self.value = 0.0;
        self.was_achieved = [false; 2];
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Thresholds {
    pub(super) score_thresholds: [Threshold; 15],
}
impl Thresholds {
    pub(super) fn new() -> Self {
        Self {
            score_thresholds: std::array::from_fn(|_| Threshold::new()),
        }
    }
    pub(super) fn reset(&mut self) {
        for value in &mut self.score_thresholds {
            value.reset();
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_field_names,
    reason = "retain original PDB field meanings across output schemas"
)]
pub(super) struct AirGap {
    pub(super) current_gap_distance: f32,
    pub(super) accumulated_gap_distance: f32,
    pub(super) maximum_gap_distance: f32,
}
impl AirGap {
    pub(super) fn new() -> Self {
        Self {
            current_gap_distance: 0.0,
            accumulated_gap_distance: 0.0,
            maximum_gap_distance: 0.0,
        }
    }
    pub(super) fn reset(&mut self) {
        self.current_gap_distance = 0.0;
        self.accumulated_gap_distance = 0.0;
        self.maximum_gap_distance = 0.0;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BoneDamage {
    pub(super) pos: SkeletonRootVector,
    pub(super) damage_level: i32,
    pub(super) damage_level_last_frame: i32,
}
impl BoneDamage {
    pub(super) fn new() -> Self {
        Self {
            pos: ZERO,
            damage_level: -1,
            damage_level_last_frame: -1,
        }
    }
    pub(super) fn reset(&mut self) {
        self.pos = ZERO;
        self.damage_level = -1;
        self.damage_level_last_frame = -1;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BoneSet {
    pub(super) bones: [BoneDamage; 25],
}
impl BoneSet {
    pub(super) fn new() -> Self {
        Self {
            bones: std::array::from_fn(|_| BoneDamage::new()),
        }
    }
    pub(super) fn reset(&mut self) {
        for value in &mut self.bones {
            value.reset();
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct EntityData {
    pub(super) unique_id: u32,
    pub(super) entity_type: i32,
    pub(super) score: u32,
}
impl EntityData {
    pub(super) fn new() -> Self {
        Self {
            unique_id: 0,
            entity_type: -1,
            score: 0,
        }
    }
    pub(super) fn reset(&mut self) {
        self.unique_id = 0;
        self.entity_type = -1;
        self.score = 0;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct EntitySet {
    pub(super) collided_entities: [EntityData; 8],
    pub(super) count: u32,
    pub(super) score: u32,
    pub(super) num_new_collisions_this_frame: u32,
}
impl EntitySet {
    pub(super) fn new() -> Self {
        Self {
            collided_entities: std::array::from_fn(|_| EntityData::new()),
            count: 0,
            score: 0,
            num_new_collisions_this_frame: 0,
        }
    }
    pub(super) fn reset(&mut self) {
        for value in &mut self.collided_entities {
            value.reset();
        }
        self.count = 0;
        self.score = 0;
        self.num_new_collisions_this_frame = 0;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct AirDrop {
    pub(super) duration_in_frames: u32,
    pub(super) drop_time_score: u32,
    pub(super) drop_height_score: u32,
    pub(super) drop_height: f32,
}
impl AirDrop {
    pub(super) fn new() -> Self {
        Self {
            duration_in_frames: 0,
            drop_time_score: 0,
            drop_height_score: 0,
            drop_height: 0.0,
        }
    }
    pub(super) fn reset(&mut self) {
        self.duration_in_frames = 0;
        self.drop_time_score = 0;
        self.drop_height_score = 0;
        self.drop_height = 0.0;
    }
}

#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_field_names,
    reason = "retain original PDB field meanings across output schemas"
)]
pub(super) struct Bail {
    pub(super) bones: BoneSet,
    pub(super) entity_sets: [EntitySet; 3],
    pub(super) current_air_drop: AirDrop,
    pub(super) max_air_drop: AirDrop,
    pub(super) accumulated_air_drop: AirDrop,
    pub(super) score: u32,
    pub(super) bail_duration_score: u32,
    pub(super) bone_score: u32,
    pub(super) bonus_score: u32,
    pub(super) bail_speed_score: u32,
    pub(super) rotation_score: u32,
    pub(super) body_tweak_score: u32,
    pub(super) current_body_tweak_score: u32,
    pub(super) num_body_tweaks_performed: u32,
    pub(super) bone_score_for_damage_level: [u32; 6],
    pub(super) moving_wipeout_duration_in_seconds: f32,
    pub(super) overall_wipeout_duration_in_seconds: f32,
    pub(super) skeleton_wipeout_over_duration_in_seconds: f32,
    pub(super) severe_wipeout_duration_in_seconds: f32,
    pub(super) bail_speed: f32,
    pub(super) rotation: f32,
    pub(super) current_body_tweak: i32,
    pub(super) just_performed_body_tweak: bool,
}
impl Bail {
    pub(super) fn new() -> Self {
        Self {
            bones: BoneSet::new(),
            entity_sets: std::array::from_fn(|_| EntitySet::new()),
            current_air_drop: AirDrop::new(),
            max_air_drop: AirDrop::new(),
            accumulated_air_drop: AirDrop::new(),
            score: 0,
            bail_duration_score: 0,
            bone_score: 0,
            bonus_score: 0,
            bail_speed_score: 0,
            rotation_score: 0,
            body_tweak_score: 0,
            current_body_tweak_score: 0,
            num_body_tweaks_performed: 0,
            bone_score_for_damage_level: [0; 6],
            moving_wipeout_duration_in_seconds: 0.0,
            overall_wipeout_duration_in_seconds: 0.0,
            skeleton_wipeout_over_duration_in_seconds: 0.0,
            severe_wipeout_duration_in_seconds: 0.0,
            bail_speed: 0.0,
            rotation: 0.0,
            current_body_tweak: -1,
            just_performed_body_tweak: false,
        }
    }
    pub(super) fn reset(&mut self) {
        self.bones.reset();
        for value in &mut self.entity_sets {
            value.reset();
        }
        self.current_air_drop.reset();
        self.max_air_drop.reset();
        self.accumulated_air_drop.reset();
        self.score = 0;
        self.bail_duration_score = 0;
        self.bone_score = 0;
        self.bonus_score = 0;
        self.bail_speed_score = 0;
        self.rotation_score = 0;
        self.body_tweak_score = 0;
        self.current_body_tweak_score = 0;
        self.num_body_tweaks_performed = 0;
        self.bone_score_for_damage_level = [0; 6];
        self.moving_wipeout_duration_in_seconds = 0.0;
        self.overall_wipeout_duration_in_seconds = 0.0;
        self.skeleton_wipeout_over_duration_in_seconds = 0.0;
        self.severe_wipeout_duration_in_seconds = 0.0;
        self.bail_speed = 0.0;
        self.rotation = 0.0;
        self.current_body_tweak = -1;
        self.just_performed_body_tweak = false;
    }
}

#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
pub(super) struct AirPeriod {
    pub(super) initial_trick: Scorable,
    pub(super) accumulated_context_data: ContextGroups,
    pub(super) initial_context_data: ContextGroups,
    pub(super) end_context_data: ContextGroups,
    pub(super) big_trick_achievements: Thresholds,
    pub(super) air_gap_data: [AirGap; 4],
    pub(super) in_air_frames: i32,
    pub(super) horizontal_distance: f32,
    pub(super) horizontal_distance_points: f32,
    pub(super) height_to_peak: f32,
    pub(super) height_to_peak_points: f32,
    pub(super) total_height_gain: f32,
    pub(super) total_height_gain_points: f32,
    pub(super) player_spin_points: f32,
    pub(super) player_flip_points: f32,
    pub(super) grab_time: f32,
    pub(super) player_spin180_count: i32,
    pub(super) player_num_flips: i32,
    pub(super) trick_count: u32,
    pub(super) just_landed: bool,
    pub(super) landing_was_sketchy: bool,
    pub(super) tweak_occured: bool,
    pub(super) grab_occured: bool,
    pub(super) started_fakie: bool,
    pub(super) started_switched: bool,
    pub(super) wiped_out: bool,
    pub(super) went_offboard: bool,
    pub(super) ran_out: bool,
    pub(super) landed_succesfully: bool,
}
impl AirPeriod {
    pub(super) fn new() -> Self {
        Self {
            initial_trick: Scorable::new(),
            accumulated_context_data: ContextGroups::new(),
            initial_context_data: ContextGroups::new(),
            end_context_data: ContextGroups::new(),
            big_trick_achievements: Thresholds::new(),
            air_gap_data: std::array::from_fn(|_| AirGap::new()),
            in_air_frames: 0,
            horizontal_distance: 0.0,
            horizontal_distance_points: 0.0,
            height_to_peak: 0.0,
            height_to_peak_points: 0.0,
            total_height_gain: 0.0,
            total_height_gain_points: 0.0,
            player_spin_points: 0.0,
            player_flip_points: 0.0,
            grab_time: 0.0,
            player_spin180_count: 0,
            player_num_flips: 0,
            trick_count: 0,
            just_landed: false,
            landing_was_sketchy: false,
            tweak_occured: false,
            grab_occured: false,
            started_fakie: false,
            started_switched: false,
            wiped_out: false,
            went_offboard: false,
            ran_out: false,
            landed_succesfully: false,
        }
    }
    pub(super) fn reset(&mut self) {
        self.initial_trick.reset();
        self.accumulated_context_data.reset();
        self.initial_context_data.reset();
        self.end_context_data.reset();
        self.big_trick_achievements.reset();
        for value in &mut self.air_gap_data {
            value.reset();
        }
        self.in_air_frames = 0;
        self.horizontal_distance = 0.0;
        self.horizontal_distance_points = 0.0;
        self.height_to_peak = 0.0;
        self.height_to_peak_points = 0.0;
        self.total_height_gain = 0.0;
        self.total_height_gain_points = 0.0;
        self.player_spin_points = 0.0;
        self.player_flip_points = 0.0;
        self.grab_time = 0.0;
        self.player_spin180_count = 0;
        self.player_num_flips = 0;
        self.trick_count = 0;
        self.just_landed = false;
        self.landing_was_sketchy = false;
        self.tweak_occured = false;
        self.grab_occured = false;
        self.started_fakie = false;
        self.started_switched = false;
        self.wiped_out = false;
        self.went_offboard = false;
        self.ran_out = false;
        self.landed_succesfully = false;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Scorables {
    pub(super) num_entries: i32,
    pub(super) elements: [Scorable; 4],
}
impl Scorables {
    pub(super) fn new() -> Self {
        Self {
            num_entries: 0,
            elements: std::array::from_fn(|_| Scorable::new()),
        }
    }
    pub(super) fn reset(&mut self) {
        self.num_entries = 0;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ObjectGrind {
    pub(super) entity_key: u64,
    pub(super) entity_type: i32,
    pub(super) unique_id: u32,
    pub(super) is_valid: bool,
}
impl ObjectGrind {
    pub(super) fn new() -> Self {
        Self {
            entity_key: u64::MAX,
            entity_type: -1,
            unique_id: 0,
            is_valid: false,
        }
    }
    pub(super) fn reset(&mut self) {
        self.entity_key = u64::MAX;
        self.entity_type = -1;
        self.unique_id = 0;
        self.is_valid = false;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct GrindInfo {
    pub(super) spline_pathed_guid: u64,
    pub(super) spline_local_guid: u64,
    pub(super) object_info: ObjectGrind,
    pub(super) length: f32,
}
impl GrindInfo {
    pub(super) fn new() -> Self {
        Self {
            spline_pathed_guid: u64::MAX,
            spline_local_guid: u64::MAX,
            object_info: ObjectGrind::new(),
            length: 0.0,
        }
    }
    pub(super) fn reset(&mut self) {
        self.spline_pathed_guid = u64::MAX;
        self.spline_local_guid = u64::MAX;
        self.object_info.reset();
        self.length = 0.0;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct CharacterDamage {
    pub(super) areas: [DamageArea; 7],
}
impl CharacterDamage {
    pub(super) fn new() -> Self {
        Self {
            areas: std::array::from_fn(|_| DamageArea::new()),
        }
    }
    pub(super) fn reset(&mut self) {
        for value in &mut self.areas {
            value.reset();
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct DamageArea {
    pub(super) damage_level: i8,
    pub(super) intensity: u8,
}
impl DamageArea {
    pub(super) fn new() -> Self {
        Self {
            damage_level: 0,
            intensity: 0,
        }
    }
    pub(super) fn reset(&mut self) {
        self.damage_level = 0;
        self.intensity = 0;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct PatternMatches {
    pub(super) did_match: [bool; 3],
}
impl PatternMatches {
    pub(super) fn new() -> Self {
        Self {
            did_match: [false; 3],
        }
    }
    pub(super) fn reset(&mut self) {
        self.did_match = [false; 3];
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct DynamicObjectSet {
    pub(super) is_set: [bool; 17],
}
impl DynamicObjectSet {
    pub(super) fn new() -> Self {
        Self {
            is_set: [false; 17],
        }
    }
    pub(super) fn reset(&mut self) {
        self.is_set = [false; 17];
    }
}

#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
pub(super) struct Scoring2 {
    pub(super) line_score: i32,
    pub(super) session_score: i32,
    pub(super) sequence_score: i32,
    pub(super) trick_score: i32,
    pub(super) wipedout_trick_score: i32,
    pub(super) run_out_lost_trick_score: i32,
    pub(super) offboard_in_air_lost_trick_score: i32,
    pub(super) hud_sequence_score: i32,
    pub(super) pre_multiplied_hud_sequence_score: i32,
    pub(super) line_timer_seconds: f32,
    pub(super) line_timer_points: i32,
    pub(super) momentum_meter_seconds: f32,
    pub(super) momentum_meter_points: i32,
    pub(super) momentum_meter_multiplier: f32,
    pub(super) manual_length: f32,
    pub(super) manual_time: f32,
    pub(super) powerslide_length: f32,
    pub(super) powerslide_time: f32,
    pub(super) finished_manual_length: f32,
    pub(super) finished_powerslide_length: f32,
    pub(super) finished_sequence_trick_count: u32,
    pub(super) finished_sequence_tapped_trick_count: u32,
    pub(super) finished_sequence_stored_trick_count: u32,
    pub(super) finished_sequence_start_visual_game_tick: u32,
    pub(super) finished_sequence_start_sim_game_tick: u32,
    pub(super) finished_sequence_replay_start_frame_number: u32,
    pub(super) finished_sequence_score: u32,
    pub(super) finished_sequence_duration_in_seconds: f32,
    pub(super) current_sequence_trick_count: u32,
    pub(super) current_sequence_tapped_trick_count: u32,
    pub(super) current_sequence_stored_trick_count: u32,
    pub(super) live_capped_sequence_trick_count: u32,
    pub(super) banked_point_change_this_frame: i32,
    pub(super) audio_banked_point_change: i32,
    pub(super) distance_travelled_this_frame: f32,
    pub(super) finished_handplant_duration_in_seconds: f32,
    pub(super) raw_scorable_id: i32,
    /// Skate 3 physics: the scorable's audio fields from the linked Skate 3 record.
    pub(super) skate3_scorable_audio: Option<(i32, i32, bool)>,
    pub(super) raw_scorable_started: i32,
    pub(super) raw_scorable_ended: i32,
    pub(super) raw_started_revert: i32,
    pub(super) raw_finished_revert: i32,
    pub(super) raw_started_manual: i32,
    pub(super) raw_finished_manual: i32,
    pub(super) raw_started_powerslide: i32,
    pub(super) raw_finished_powerslide: i32,
    pub(super) current_gesture: i32,
    pub(super) current_offboard_duration_in_seconds: f32,
    pub(super) ended_offboard_duration_in_seconds: f32,
    pub(super) hippy_jump180_count: i32,
    pub(super) finished_hippy_jump180_count: i32,
    pub(super) tricks_begun: Scorables,
    pub(super) tricks_ended: Scorables,
    pub(super) tricks_cancelled: Scorables,
    pub(super) tricks_converted_from: Scorables,
    pub(super) tricks_converted_to: Scorables,
    pub(super) active_tricks: Scorables,
    pub(super) tricks_in_current_sequence: [u8; super::conditioners::SCORABLE_CAPACITY],
    pub(super) trick_types_in_current_sequence: [u8; 14],
    pub(super) tricks_in_completed_sequence: [u8; super::conditioners::SCORABLE_CAPACITY],
    pub(super) trick_types_in_completed_sequence: [u8; 14],
    pub(super) finished_sequence_pattern_matches: PatternMatches,
    pub(super) current_sequence_pattern_matches: PatternMatches,
    pub(super) finished_sequence_scorables: Scorables,
    pub(super) current_sequence_scorables: Scorables,
    pub(super) air_finished_data: AirPeriod,
    pub(super) current_air_data: AirPeriod,
    pub(super) ho_m_bail_data: Bail,
    pub(super) current_context_data: ContextGroups,
    pub(super) character_damage_data: CharacterDamage,
    pub(super) big_trick_achieved_data: Thresholds,
    pub(super) current_frame_grind_object_info: ObjectGrind,
    pub(super) current_grind_info: GrindInfo,
    pub(super) finished_grind_info: GrindInfo,
    pub(super) current_sequence_dynamic_object_set: DynamicObjectSet,
    pub(super) finished_sequence_dynamic_object_set: DynamicObjectSet,
    pub(super) line_timer_finished: bool,
    pub(super) momentum_meter_finished: bool,
    pub(super) sequence_completed: bool,
    pub(super) sequence_completed_successfully: bool,
    pub(super) sequence_begun: bool,
    pub(super) in_sequence: bool,
    pub(super) just_wiped_out: bool,
    pub(super) just_finished_grind: bool,
    pub(super) just_finished_manual: bool,
    pub(super) just_finished_powerslide: bool,
    pub(super) is_in_state_grinding: bool,
    pub(super) is_in_state_ground: bool,
    pub(super) is_in_state_air: bool,
    pub(super) is_in_state_other: bool,
    pub(super) is_in_state_handplant: bool,
    pub(super) is_in_state_offboard: bool,
    pub(super) wipeout_interrupted_trick: bool,
    pub(super) just_took_off: bool,
    pub(super) air_finished: bool,
    pub(super) just_reactivated_scoring: bool,
    pub(super) just_started_line_timer: bool,
    pub(super) has_been_reset: bool,
    pub(super) ho_m_bail_completed: bool,
    pub(super) ho_m_bail_in_progress: bool,
    pub(super) ho_m_is_effect_enabled: bool,
    pub(super) is_hippy_flipping: bool,
    pub(super) just_performed_gesture: bool,
    pub(super) just_finished_handplant: bool,
    pub(super) is_attempting_a_body_flip: bool,
    pub(super) is_hippy_jumping: bool,
    pub(super) is_offboard_and_not_hippy_jumping: bool,
    pub(super) was_offboard_and_not_hippy_jumping: bool,
    pub(super) is_onboard_or_hippy_jumping: bool,
    pub(super) was_onboard_or_hippy_jumping: bool,
    pub(super) is_running_out: bool,
}
impl Scoring2 {
    #[expect(
        clippy::too_many_lines,
        reason = "complete native output constructor retains all 110 typed fields"
    )]
    pub(super) fn new() -> Self {
        Self {
            line_score: 0,
            session_score: 0,
            sequence_score: 0,
            trick_score: 0,
            wipedout_trick_score: 0,
            run_out_lost_trick_score: 0,
            offboard_in_air_lost_trick_score: 0,
            hud_sequence_score: 0,
            pre_multiplied_hud_sequence_score: 0,
            line_timer_seconds: 0.0,
            line_timer_points: 0,
            momentum_meter_seconds: 0.0,
            momentum_meter_points: 0,
            momentum_meter_multiplier: 1.0,
            manual_length: 0.0,
            manual_time: 0.0,
            powerslide_length: 0.0,
            powerslide_time: 0.0,
            finished_manual_length: 0.0,
            finished_powerslide_length: 0.0,
            finished_sequence_trick_count: 0,
            finished_sequence_tapped_trick_count: 0,
            finished_sequence_stored_trick_count: 0,
            finished_sequence_start_visual_game_tick: 0,
            finished_sequence_start_sim_game_tick: 0,
            finished_sequence_replay_start_frame_number: 0,
            finished_sequence_score: 0,
            finished_sequence_duration_in_seconds: 0.0,
            current_sequence_trick_count: 0,
            current_sequence_tapped_trick_count: 0,
            current_sequence_stored_trick_count: 0,
            live_capped_sequence_trick_count: 0,
            banked_point_change_this_frame: 0,
            audio_banked_point_change: 0,
            distance_travelled_this_frame: 0.0,
            finished_handplant_duration_in_seconds: 0.0,
            raw_scorable_id: -1,
            skate3_scorable_audio: None,
            raw_scorable_started: -1,
            raw_scorable_ended: -1,
            raw_started_revert: -1,
            raw_finished_revert: -1,
            raw_started_manual: -1,
            raw_finished_manual: -1,
            raw_started_powerslide: -1,
            raw_finished_powerslide: -1,
            current_gesture: 35,
            current_offboard_duration_in_seconds: 0.0,
            ended_offboard_duration_in_seconds: 0.0,
            hippy_jump180_count: 0,
            finished_hippy_jump180_count: 0,
            tricks_begun: Scorables::new(),
            tricks_ended: Scorables::new(),
            tricks_cancelled: Scorables::new(),
            tricks_converted_from: Scorables::new(),
            tricks_converted_to: Scorables::new(),
            active_tricks: Scorables::new(),
            tricks_in_current_sequence: [0; super::conditioners::SCORABLE_CAPACITY],
            trick_types_in_current_sequence: [0; 14],
            tricks_in_completed_sequence: [0; super::conditioners::SCORABLE_CAPACITY],
            trick_types_in_completed_sequence: [0; 14],
            finished_sequence_pattern_matches: PatternMatches::new(),
            current_sequence_pattern_matches: PatternMatches::new(),
            finished_sequence_scorables: Scorables::new(),
            current_sequence_scorables: Scorables::new(),
            air_finished_data: AirPeriod::new(),
            current_air_data: AirPeriod::new(),
            ho_m_bail_data: Bail::new(),
            current_context_data: ContextGroups::new(),
            character_damage_data: CharacterDamage::new(),
            big_trick_achieved_data: Thresholds::new(),
            current_frame_grind_object_info: ObjectGrind::new(),
            current_grind_info: GrindInfo::new(),
            finished_grind_info: GrindInfo::new(),
            current_sequence_dynamic_object_set: DynamicObjectSet::new(),
            finished_sequence_dynamic_object_set: DynamicObjectSet::new(),
            line_timer_finished: false,
            momentum_meter_finished: false,
            sequence_completed: false,
            sequence_completed_successfully: false,
            sequence_begun: false,
            in_sequence: false,
            just_wiped_out: false,
            just_finished_grind: false,
            just_finished_manual: false,
            just_finished_powerslide: false,
            is_in_state_grinding: false,
            is_in_state_ground: false,
            is_in_state_air: false,
            is_in_state_other: false,
            is_in_state_handplant: false,
            is_in_state_offboard: false,
            wipeout_interrupted_trick: false,
            just_took_off: false,
            air_finished: false,
            just_reactivated_scoring: false,
            just_started_line_timer: false,
            has_been_reset: false,
            ho_m_bail_completed: false,
            ho_m_bail_in_progress: false,
            ho_m_is_effect_enabled: false,
            is_hippy_flipping: false,
            just_performed_gesture: false,
            just_finished_handplant: false,
            is_attempting_a_body_flip: false,
            is_hippy_jumping: false,
            is_offboard_and_not_hippy_jumping: false,
            was_offboard_and_not_hippy_jumping: false,
            is_onboard_or_hippy_jumping: false,
            was_onboard_or_hippy_jumping: false,
            is_running_out: false,
        }
    }
    #[expect(
        clippy::too_many_lines,
        reason = "source reset visits all output fields while preserving inactive list elements"
    )]
    pub(super) fn reset(&mut self) {
        self.line_score = 0;
        self.session_score = 0;
        self.sequence_score = 0;
        self.trick_score = 0;
        self.wipedout_trick_score = 0;
        self.run_out_lost_trick_score = 0;
        self.offboard_in_air_lost_trick_score = 0;
        self.hud_sequence_score = 0;
        self.pre_multiplied_hud_sequence_score = 0;
        self.line_timer_seconds = 0.0;
        self.line_timer_points = 0;
        self.momentum_meter_seconds = 0.0;
        self.momentum_meter_points = 0;
        self.momentum_meter_multiplier = 1.0;
        self.manual_length = 0.0;
        self.manual_time = 0.0;
        self.powerslide_length = 0.0;
        self.powerslide_time = 0.0;
        self.finished_manual_length = 0.0;
        self.finished_powerslide_length = 0.0;
        self.finished_sequence_trick_count = 0;
        self.finished_sequence_tapped_trick_count = 0;
        self.finished_sequence_stored_trick_count = 0;
        self.finished_sequence_start_visual_game_tick = 0;
        self.finished_sequence_start_sim_game_tick = 0;
        self.finished_sequence_replay_start_frame_number = 0;
        self.finished_sequence_score = 0;
        self.finished_sequence_duration_in_seconds = 0.0;
        self.current_sequence_trick_count = 0;
        self.current_sequence_tapped_trick_count = 0;
        self.current_sequence_stored_trick_count = 0;
        self.live_capped_sequence_trick_count = 0;
        self.banked_point_change_this_frame = 0;
        self.audio_banked_point_change = 0;
        self.distance_travelled_this_frame = 0.0;
        self.finished_handplant_duration_in_seconds = 0.0;
        self.raw_scorable_id = -1;
        self.skate3_scorable_audio = None;
        self.raw_scorable_started = -1;
        self.raw_scorable_ended = -1;
        self.raw_started_revert = -1;
        self.raw_finished_revert = -1;
        self.raw_started_manual = -1;
        self.raw_finished_manual = -1;
        self.raw_started_powerslide = -1;
        self.raw_finished_powerslide = -1;
        self.current_gesture = 35;
        self.current_offboard_duration_in_seconds = 0.0;
        self.ended_offboard_duration_in_seconds = 0.0;
        self.hippy_jump180_count = 0;
        self.finished_hippy_jump180_count = 0;
        self.tricks_begun.reset();
        self.tricks_ended.reset();
        self.tricks_cancelled.reset();
        self.tricks_converted_from.reset();
        self.tricks_converted_to.reset();
        self.active_tricks.reset();
        self.tricks_in_current_sequence = [0; super::conditioners::SCORABLE_CAPACITY];
        self.trick_types_in_current_sequence = [0; 14];
        self.tricks_in_completed_sequence = [0; super::conditioners::SCORABLE_CAPACITY];
        self.trick_types_in_completed_sequence = [0; 14];
        self.finished_sequence_pattern_matches.reset();
        self.current_sequence_pattern_matches.reset();
        self.finished_sequence_scorables.reset();
        self.current_sequence_scorables.reset();
        self.air_finished_data.reset();
        self.current_air_data.reset();
        self.ho_m_bail_data.reset();
        self.current_context_data.reset();
        self.character_damage_data.reset();
        self.big_trick_achieved_data.reset();
        self.current_frame_grind_object_info.reset();
        self.current_grind_info.reset();
        self.finished_grind_info.reset();
        self.current_sequence_dynamic_object_set.reset();
        self.finished_sequence_dynamic_object_set.reset();
        self.line_timer_finished = false;
        self.momentum_meter_finished = false;
        self.sequence_completed = false;
        self.sequence_completed_successfully = false;
        self.sequence_begun = false;
        self.in_sequence = false;
        self.just_wiped_out = false;
        self.just_finished_grind = false;
        self.just_finished_manual = false;
        self.just_finished_powerslide = false;
        self.is_in_state_grinding = false;
        self.is_in_state_ground = false;
        self.is_in_state_air = false;
        self.is_in_state_other = false;
        self.is_in_state_handplant = false;
        self.is_in_state_offboard = false;
        self.wipeout_interrupted_trick = false;
        self.just_took_off = false;
        self.air_finished = false;
        self.just_reactivated_scoring = false;
        self.just_started_line_timer = false;
        self.has_been_reset = false;
        self.ho_m_bail_completed = false;
        self.ho_m_bail_in_progress = false;
        self.ho_m_is_effect_enabled = false;
        self.is_hippy_flipping = false;
        self.just_performed_gesture = false;
        self.just_finished_handplant = false;
        self.is_attempting_a_body_flip = false;
        self.is_hippy_jumping = false;
        self.is_offboard_and_not_hippy_jumping = false;
        self.was_offboard_and_not_hippy_jumping = false;
        self.is_onboard_or_hippy_jumping = false;
        self.was_onboard_or_hippy_jumping = false;
        self.is_running_out = false;
    }
}

/// PDB BailScoringOutData (0x4f0), copied by HOMResultsData.Init82569948.
impl Bail {
    pub(super) fn save_image(&self)->Option<crate::challenge::hom_save::BailImage> {
        let mut out=crate::challenge::hom_save::BailImage::empty();
        for (i,bone) in self.bones.bones.iter().enumerate() {
            for (lane,value) in bone.pos.xyz.into_iter().enumerate() {out.set(i*32+lane*4,value.to_bits());}
            if let Some(w)=bone.pos.w {out.set(i*32+12,w.to_bits());}
            else {
                // Native Part.GetTransform826c3480 loads adjacent body metadata
                // into W. The portable save keeps the empty image's zero byte
                // representation and marks its provenance; PhysOut stays unknown.
                out.unrecorded_bone_w|=1<<i;
            }
            out.set(i*32+16,bone.damage_level.cast_unsigned());out.set(i*32+20,bone.damage_level_last_frame.cast_unsigned());
        }
        for (i,set) in self.entity_sets.iter().enumerate() {
            let base=0x320+i*0x6c;
            for (j,entity) in set.collided_entities.iter().enumerate() {
                out.set(base+j*12,entity.unique_id);out.set(base+j*12+4,entity.entity_type.cast_unsigned());out.set(base+j*12+8,entity.score);
            }
            out.set(base+0x60,set.count);out.set(base+0x64,set.score);out.set(base+0x68,set.num_new_collisions_this_frame);
        }
        for (i,drop) in [&self.current_air_drop,&self.max_air_drop,&self.accumulated_air_drop].into_iter().enumerate() {
            let base=0x464+i*16;out.set(base,drop.duration_in_frames);out.set(base+4,drop.drop_time_score);
            out.set(base+8,drop.drop_height_score);out.set(base+12,drop.drop_height.to_bits());
        }
        for (i,value) in [self.score,self.bail_duration_score,self.bone_score,self.bonus_score,self.bail_speed_score,
            self.rotation_score,self.body_tweak_score,self.current_body_tweak_score,self.num_body_tweaks_performed].into_iter().enumerate() {out.set(0x494+i*4,value);}
        for (i,value) in self.bone_score_for_damage_level.into_iter().enumerate() {out.set(0x4b8+i*4,value);}
        for (i,value) in [self.moving_wipeout_duration_in_seconds,self.overall_wipeout_duration_in_seconds,
            self.skeleton_wipeout_over_duration_in_seconds,self.severe_wipeout_duration_in_seconds,self.bail_speed,self.rotation].into_iter().enumerate() {out.set(0x4d0+i*4,value.to_bits());}
        out.set(0x4e8,self.current_body_tweak.cast_unsigned());out.bytes[0x4ec]=u8::from(self.just_performed_body_tweak);Some(out)
    }
}
