//! WipeoutScorer Update82df6cb8 / UpdateInWipeout82df5bc0 and native lifecycle.
use super::{
    super::{ZERO, collision::SkeletonCollision, scoring::Scoring2},
    scoring_attributes::{ScoreError, curve, word},
    scoring_external::ScoringExternalInput,
    scoring_wipeout_bones::DamageTuning,
    wipeout_score::WipeoutScorer,
};
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    physics::FilteredPhysicsState,
    reckoning_numeric::{HostReckoningNumeric, ReckoningVector, try_length},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::Arc;

pub(super) struct WipeoutFrame<'a> {
    pub filtered: FilteredPhysicsState,
    pub collision: &'a SkeletonCollision,
    pub hips_matrix: SkeletonRootMatrix,
    pub system_position: SkeletonRootVector,
    pub system_velocity: SkeletonRootVector,
    pub skeleton_wipeout_over: bool,
    pub heavy_landing_kind: i32,
    pub heavy_landing_speed: f32,
    pub gesture: [f32; 2],
    /// Skate 3 physics only (TU3 WipeoutScorer): PhysOut_State+68/+69, Skeleton+580,
    /// Air+452, SystemReckoning+16 velocity and +64 position.
    pub skate3_countdown: bool,
    pub skate3_teleport_requested: bool,
    pub skate3_response_strength: f32,
    pub skate3_air_locked: bool,
    pub centre_of_mass_velocity: SkeletonRootVector,
    pub damped_centre_of_mass: SkeletonRootVector,
}
pub(super) struct WipeoutTuning {
    pub damage: DamageTuning,
    pub ignore_stationary_rotation: bool,
    pub minimum_bank_degrees: f32,
    pub heavy_landing_reset_frames: i32,
    pub forced_render_damage_level: i32,
    pub damage_fade_in: f32,
}
impl WipeoutScorer {
    fn reset_body_tweak(&mut self) {
        self.current_body_tweak = -1;
        self.body_tweak_valid = false;
        self.just_performed_body_tweak = false;
        self.body_tweak_duration_frames = 0;
    }
    pub(super) fn reset_wipeout(&mut self, matrix: SkeletonRootMatrix) -> Result<(), ScoreError> {
        self.air_start_position = ZERO;
        self.air_apex_position = ZERO;
        self.bail_start_velocity = ZERO;
        self.overall_wipeout_duration_frames = 0;
        self.moving_wipeout_duration_frames = 0;
        self.banked_combined_rotation = 0.0;
        self.skeleton_wipeout_over_duration_frames = 0;
        self.current_combined_rotation = 0.0;
        self.severe_wipeout_duration_frames = 0;
        self.max_combined_rotation = 0.0;
        self.bail_speed_score = 0;
        self.current_rotation_score = 0;
        self.max_rotation_score = 0;
        self.body_tweak_score = 0;
        self.current_body_tweak_score = 0;
        self.num_body_tweaks_performed = 0;
        self.current_air_drop.reset();
        self.max_air_drop.reset();
        self.accumulated_air_drop.reset();
        self.is_wiping_out = false;
        self.is_skeleton_wipeout_over = false;
        self.is_in_severe_wipeout = false;
        self.is_airborne = false;
        self.pedestrian
            .as_mut()
            .ok_or(ScoreError::Missing("pedestrian scorer"))?
            .reset_tracker()?;
        self.vehicle
            .as_mut()
            .ok_or(ScoreError::Missing("vehicle scorer"))?
            .reset_tracker()?;
        self.dmo
            .published_value_mut()
            .ok_or(ScoreError::Missing("DMO scorer"))?
            .base
            .reset_tracker()?;
        self.pitch_tracker.reset(matrix);
        self.yaw_tracker.reset(matrix);
        self.roll_tracker.reset(matrix);
        for bone in &mut self.bones {
            bone.position = ZERO;
            bone.position_last_frame = ZERO;
            bone.damage_level = -1;
            bone.damage_level_last_frame = -1;
            bone.damage_amount = 0.0;
            bone.contact_surface_id = 0;
            bone.is_airborne = false;
        }
        self.reset_body_tweak();
        // Preserve heavy-landing memory and the seven render-damage areas.
        Ok(())
    }
    fn start_air(&mut self, position: SkeletonRootVector) {
        self.air_start_position = position;
        self.air_apex_position = position;
        self.current_air_drop.reset();
        self.reset_body_tweak();
    }
    fn end_air(&mut self) {
        let current = &self.current_air_drop;
        let total = &mut self.accumulated_air_drop;
        total.duration_in_frames = total
            .duration_in_frames
            .wrapping_add(current.duration_in_frames);
        total.drop_time_score = current.drop_time_score.wrapping_add(total.drop_time_score);
        total.drop_height_score = current
            .drop_height_score
            .wrapping_add(total.drop_height_score);
        total.drop_height = current.drop_height + total.drop_height;
        let max = &mut self.max_air_drop;
        max.duration_in_frames = max.duration_in_frames.max(current.duration_in_frames);
        max.drop_time_score = max.drop_time_score.max(current.drop_time_score);
        max.drop_height_score = max.drop_height_score.max(current.drop_height_score);
        max.drop_height = if max.drop_height - current.drop_height >= 0.0 {
            max.drop_height
        } else {
            current.drop_height
        };
        self.current_air_drop.reset();
        self.reset_body_tweak();
    }
    fn update_air(
        &mut self,
        position: SkeletonRootVector,
        gesture: [f32; 2],
    ) -> Result<(), ScoreError> {
        if position.xyz[1] > self.air_apex_position.xyz[1] {
            self.air_apex_position = position;
        }
        self.current_air_drop.duration_in_frames =
            self.current_air_drop.duration_in_frames.wrapping_add(1);
        self.current_air_drop.drop_height = self.air_apex_position.xyz[1] - position.xyz[1];
        self.current_air_drop.drop_time_score = curve(
            &self.attributes,
            2180,
            self.current_air_drop.duration_in_frames as f32 * f32::from_bits(0x3c88_8889),
        )? as i64 as u32;
        self.current_air_drop.drop_height_score =
            curve(&self.attributes, 2260, self.current_air_drop.drop_height)? as i64 as u32;
        self.update_body_tweaks(gesture)
    }
    pub(super) fn update_wipeout(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        input: &ScoringExternalInput,
        frame: &WipeoutFrame<'_>,
        tuning: &WipeoutTuning,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        #[cfg(feature = "skate3-runtime")]
        if self.attributes.skate3_enabled() {
            return self.update_wipeout_skate3(vault, input, frame, tuning, out);
        }
        let was_wiping_out = self.is_wiping_out;
        self.is_wiping_out = frame.filtered == FilteredPhysicsState::Wipeout;
        if was_wiping_out != self.is_wiping_out {
            if self.is_wiping_out {
                self.reset_wipeout(frame.hips_matrix)?;
                self.damage_feet_after_landing(&tuning.damage)?;
                self.is_wiping_out = true;
                self.bail_start_velocity = frame.system_velocity;
                let speed = try_length(
                    self.bail_start_velocity.operand(),
                    &mut HostReckoningNumeric,
                )
                .map_err(ScoreError::Numeric)?;
                self.bail_speed_score = curve(&self.attributes, 2340, speed)? as i64 as u32;
            } else {
                out.ho_m_bail_completed = true;
                self.output_bail(out)?;
            }
        }
        if self.is_wiping_out {
            let was_airborne = self.is_airborne;
            self.update_bones(frame.collision, &tuning.damage)?;
            if self.is_airborne && !was_airborne {
                self.start_air(frame.system_position);
            } else if was_airborne && !self.is_airborne {
                self.update_air(frame.system_position, frame.gesture)?;
                self.end_air();
            }
            self.update_entities(vault, input)?;
            if self.is_airborne {
                self.update_air(frame.system_position, frame.gesture)?;
            }
            if tuning.ignore_stationary_rotation && frame.skeleton_wipeout_over {
                // Source updates just the matrices here, retaining accumulated angles.
                self.pitch_tracker.last_frame_matrix = Some(frame.hips_matrix);
                self.yaw_tracker.last_frame_matrix = Some(frame.hips_matrix);
                self.roll_tracker.last_frame_matrix = Some(frame.hips_matrix);
            } else {
                self.banked_combined_rotation = self
                    .pitch_tracker
                    .update_with_banking(frame.hips_matrix, tuning.minimum_bank_degrees)?
                    + self.banked_combined_rotation;
                self.banked_combined_rotation = self
                    .yaw_tracker
                    .update_with_banking(frame.hips_matrix, tuning.minimum_bank_degrees)?
                    + self.banked_combined_rotation;
                self.banked_combined_rotation = self
                    .roll_tracker
                    .update_with_banking(frame.hips_matrix, tuning.minimum_bank_degrees)?
                    + self.banked_combined_rotation;
            }
            let degrees = f32::from_bits(0x4265_2ee1);
            let yaw = (self.yaw_tracker.accumulated_angle_radians * degrees).abs();
            let pitch = (self.pitch_tracker.accumulated_angle_radians * degrees).abs();
            let roll = (self.roll_tracker.accumulated_angle_radians * degrees).abs();
            self.current_combined_rotation = ((yaw + pitch) + roll) + self.banked_combined_rotation;
            self.current_rotation_score =
                curve(&self.attributes, 2420, self.current_combined_rotation)? as i64 as u32;
            self.max_combined_rotation =
                if self.current_combined_rotation - self.max_combined_rotation >= 0.0 {
                    self.current_combined_rotation
                } else {
                    self.max_combined_rotation
                };
            self.max_rotation_score = self.max_rotation_score.max(self.current_rotation_score);
            self.overall_wipeout_duration_frames =
                self.overall_wipeout_duration_frames.wrapping_add(1);
            if !frame.skeleton_wipeout_over {
                self.moving_wipeout_duration_frames =
                    self.moving_wipeout_duration_frames.wrapping_add(1);
            }
            self.is_skeleton_wipeout_over |= frame.skeleton_wipeout_over;
            if self.is_skeleton_wipeout_over {
                self.skeleton_wipeout_over_duration_frames =
                    self.skeleton_wipeout_over_duration_frames.wrapping_add(1);
            }
            if !self.is_in_severe_wipeout
                && self.bone_score(None)? > word(&self.attributes, 0x8fb0_3ecd_5da2_6421)?
            {
                self.is_in_severe_wipeout = true;
            }
            if self.is_in_severe_wipeout {
                self.severe_wipeout_duration_frames =
                    self.severe_wipeout_duration_frames.wrapping_add(1);
            }
            out.ho_m_bail_in_progress = true;
            self.output_bail(out)?;
        } else {
            self.check_heavy_landing(
                frame.heavy_landing_kind,
                frame.heavy_landing_speed,
                tuning.heavy_landing_reset_frames,
            );
        }
        self.update_character_damage(
            input.reset_player_damage,
            tuning.forced_render_damage_level,
            tuning.damage_fade_in,
            out,
        )
    }
}
