//! OffboardCollector82df8400/c250/c308. Uses actual skeleton and deck frames.
use super::{
    super::{scoring::Scoring2, simple::GroundRemainder},
    collectors::OffboardCollector,
    scoring_attributes::{ScoreError, curve, scalar},
    scoring_database::TrickDatabase,
};
use crate::{
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

pub(super) struct OffboardFrame<'a> {
    pub enabled: &'a [bool; 14],
    pub tick: u32,
    pub switched: bool,
    pub fakie: bool,
    pub landing_on_deck: bool,
    pub player_matrix: SkeletonRootMatrix,
    pub board_matrix: SkeletonRootMatrix,
    pub ground: &'a GroundRemainder,
    /// SystemReckoning +0 (damped velocity) and its up vector.
    pub system_velocity: SkeletonRootVector,
    pub system_up: SkeletonRootVector,
}

/// TU3 82DABE98: a hippy jump starts at a fifth of its points unless the system moves
/// faster than 4 m/s across its up vector.
#[cfg(feature = "skate3-runtime")]
fn skate3_hippy_point_scalar(velocity: SkeletonRootVector, up: SkeletonRootVector) -> f32 {
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let (v, u) = (velocity.xyz, up.xyz);
    let along = dot(v, u) / dot(u, u);
    let across = [v[0] - u[0] * along, v[1] - u[1] * along, v[2] - u[2] * along];
    if 16.0 > dot(across, across) { f32::from_bits(0x3e4c_cccd) } else { 1.0 }
}

impl OffboardCollector {
    #[cfg(feature = "skate3-runtime")]
    fn skate3(&self) -> bool {
        self.base.states.skate3_enabled()
    }
    #[cfg(not(feature = "skate3-runtime"))]
    fn skate3(&self) -> bool {
        false
    }

    pub(super) fn update_tricks(
        &mut self,
        db: &TrickDatabase,
        f: &OffboardFrame<'_>,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        self.total_trick_score = Some(0.0);
        let mut active = self.base.filtered_scorable_id == 234 && f.enabled[10];
        if !active && self.is_hippy_jumping && f.landing_on_deck {
            active = true;
        }
        if active {
            if self.is_hippy_jumping && self.skate3() {
                // TU3 UpdateHippyJumps 82DAC068: the count is the physics' landing half turns
                // and its points are halved.
                #[cfg(feature = "skate3-runtime")]
                {
                    self.hippy_jump180_count = f.ground.landing_half_turns;
                    out.hippy_jump180_count = self.hippy_jump180_count;
                    let points = curve(
                        &self.base.states,
                        880,
                        (self.hippy_jump180_count as f32 * 180.0).abs(),
                    )? * 0.5;
                    let change = points - self.hippy_jump180_points;
                    self.hippy_jump180_points = points;
                    if change > 0.0 {
                        self.hippy_jump_scorable.score = self
                            .hippy_jump_scorable
                            .point_scalar
                            .mul_add(change, self.hippy_jump_scorable.score);
                    }
                    let mut holder = self
                        .base
                        .score_holder
                        .as_ref()
                        .ok_or(ScoreError::Missing("offboard holder"))?
                        .lock()
                        .map_err(|_| ScoreError::Poisoned)?;
                    if !holder.trick_score_held() {
                        holder.trick_score += self.hippy_jump_scorable.score;
                    }
                }
            } else if self.is_hippy_jumping {
                self.player_spin_tracker.update(f.player_matrix)?;
                self.board_spin_tracker.update(f.board_matrix)?;
                let degrees = f32::from_bits(0x4265_2ee1);
                let board = self.board_spin_tracker.accumulated_angle_radians * degrees;
                // Native fmsubs82df852c, not two rounded products then SUB.
                let relative = self
                    .player_spin_tracker
                    .accumulated_angle_radians
                    .mul_add(degrees, -board);
                let turns = (relative.abs() + 80.0) * f32::from_bits(0x3bb6_0b61);
                let count = HostReckoningNumeric
                    .truncate_i32(turns)
                    .map_err(ScoreError::Numeric)?;
                self.hippy_jump180_count = if relative < 0.0 {
                    count.wrapping_neg()
                } else {
                    count
                };
                out.hippy_jump180_count = self.hippy_jump180_count;
                let points = curve(
                    &self.base.states,
                    880,
                    (self.hippy_jump180_count as f32 * 180.0).abs(),
                )?;
                let change = points - self.hippy_jump180_points;
                self.hippy_jump180_points = points;
                if change > 0.0 {
                    self.hippy_jump_scorable.score = self
                        .hippy_jump_scorable
                        .point_scalar
                        .mul_add(change, self.hippy_jump_scorable.score);
                }
                self.base
                    .score_holder
                    .as_ref()
                    .ok_or(ScoreError::Missing("offboard holder"))?
                    .lock()
                    .map_err(|_| ScoreError::Poisoned)?
                    .trick_score += self.hippy_jump_scorable.score;
            } else {
                let name = self
                    .base
                    .animation_trick
                    .name
                    .decode()
                    .map_err(|_| ScoreError::Invalid("hippy animation name"))?;
                #[cfg(feature = "skate3-runtime")]
                let point_scalar = if self.skate3() {
                    skate3_hippy_point_scalar(f.system_velocity, f.system_up)
                } else {
                    1.0
                };
                #[cfg(not(feature = "skate3-runtime"))]
                let point_scalar = 1.0;
                self.base.start_scorable(
                    db,
                    &name,
                    self.base.filtered_scorable_id,
                    point_scalar,
                    &mut self.hippy_jump_scorable,
                    f.tick,
                    0.0,
                    f.switched,
                    f.fakie,
                )?;
                self.base
                    .score_holder
                    .as_ref()
                    .ok_or(ScoreError::Missing("offboard holder"))?
                    .lock()
                    .map_err(|_| ScoreError::Poisoned)?
                    .attempt_beginning(&mut self.hippy_jump_scorable)?;
                self.hippy_jump180_points = 0.0;
                self.hippy_jump180_count = 0;
                if !self.skate3() {
                    self.player_spin_tracker.reset(f.player_matrix);
                    self.board_spin_tracker.reset(f.board_matrix);
                }
            }
        } else if self.is_hippy_jumping {
            self.base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("offboard holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?
                .announce_cancelled(&mut self.hippy_jump_scorable)?;
            self.hippy_jump_scorable.reset();
        }
        self.is_hippy_jumping = active;
        if active {
            out.is_hippy_jumping = true;
            if !self.hippy_jump_scorable.had_object_between_board_and_skater {
                self.base.check_hippy_flip(f.ground, out)?;
                if out.is_hippy_flipping {
                    // GetRawBaseScore 0x82de8e88 returns the base score whether or not the
                    // jump validated (its assert is a debug-build check).
                    let raw = self.hippy_jump_scorable.trick_scoring_settings.base_score as f32;
                    let bonus = scalar(&self.base.states, 0x1ca6_3104_2d29_2121)? * raw;
                    self.hippy_jump_scorable.score = self
                        .hippy_jump_scorable
                        .point_scalar
                        .mul_add(bonus, self.hippy_jump_scorable.score);
                    self.hippy_jump_scorable.had_object_between_board_and_skater = true;
                }
            }
        }
        self.base
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("offboard holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .update_active(&mut self.hippy_jump_scorable)?;
        // UpdateHippyJumps returns0 even when its holder receives points.
        self.total_trick_score = Some(0.0 + self.total_trick_score.unwrap());
        self.num_frames_in_collector = self.num_frames_in_collector.wrapping_add(1);
        Ok(())
    }
    pub(super) fn exit(
        &mut self,
        score_in_progress: bool,
        db: &TrickDatabase,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        out.ended_offboard_duration_in_seconds =
            self.num_frames_in_collector as f32 * f32::from_bits(0x3c88_8889);
        if self.is_hippy_jumping {
            let mut holder = self
                .base
                .score_holder
                .as_ref()
                .ok_or(ScoreError::Missing("offboard holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            if score_in_progress {
                // EndCurrentHippyJump 0x82dfb130: a jump that never validated ends as -1,
                // which EndTrick ignores (the debug build asserts, offboardcollector.cpp 90).
                let id = self.hippy_jump_scorable.valid_id().unwrap_or(-1);
                holder.announce_ending(db, &mut self.hippy_jump_scorable)?;
                holder.end_trick(id, self.hippy_jump_scorable.score);
                out.finished_hippy_jump180_count = self.hippy_jump180_count;
            } else {
                holder.announce_cancelled(&mut self.hippy_jump_scorable)?;
            }
        }
        Ok(())
    }
    pub(super) fn activity_sequence_worthy(&self) -> bool {
        self.hippy_jump_scorable.valid_id().is_some()
            && self.hippy_jump_scorable.is_beginning_announced
    }
    pub(super) fn line_decrement_fraction(&self) -> Result<f32, ScoreError> {
        scalar(
            &self.base.states,
            if self.activity_sequence_worthy() {
                0xf813_b960_d045_0300
            } else {
                0xb26a_66f7_634e_760e
            },
        )
    }
    pub(super) fn momentum_decrement_fraction(&self) -> Result<f32, ScoreError> {
        scalar(
            &self.base.states,
            if self.activity_sequence_worthy() {
                0x4bea_9c28_5a3c_da06
            } else {
                0x3a44_4577_c4d4_661e
            },
        )
    }
    pub(super) fn fill_output(&self, out: &mut Scoring2) {
        out.is_in_state_offboard = true;
    }
}
