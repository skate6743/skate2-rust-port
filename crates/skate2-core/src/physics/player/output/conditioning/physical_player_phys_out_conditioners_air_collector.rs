//! `AirCollector82df8e18` and Reset82df4720 read the same constructed `PhysOut`.
use super::{
    super::{
        PhysOutAccessError, PhysOutComponent, PhysicalPlayerPhysOut, SkeletonRootMatrix,
        SkeletonRootVector, ZERO,
        scoring::{ContextGroups, Scorable},
    },
    collector_base::{CollectorBase, RotationTracker},
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct AirGroundData {
    pub(super) hips_position: SkeletonRootVector,
    pub(super) height_above_ground: f32,
    pub(super) relative_height_above_ground: f32,
    pub(super) is_above_ground: bool,
    pub(super) is_above_grass: bool,
    pub(super) is_above_stairs: bool,
}
impl AirGroundData {
    fn new() -> Self {
        Self {
            hips_position: ZERO,
            height_above_ground: 0.0,
            relative_height_above_ground: 0.0,
            is_above_ground: false,
            is_above_grass: false,
            is_above_stairs: false,
        }
    }
    /// Set82df44d8 with the original constructor's NULL takeoff argument. This
    /// reads already retained Ground data; it never issues a collision query.
    fn set_takeoff(&mut self, output: &PhysicalPlayerPhysOut) -> Result<(), AirResetError> {
        let ground = output
            .concrete
            .ground
            .as_ref()
            .ok_or(AirResetError::Missing(PhysOutComponent::Ground))?;
        *self = Self::from_ground(ground, None);
        Ok(())
    }
}
pub(super) struct AirGapData {
    pub(super) current_gap_start_position: SkeletonRootVector,
    pub(super) current_gap_distance: f32,
    pub(super) accumulated_gap_distance: f32,
    pub(super) maximum_gap_distance: f32,
    pub(super) gap_count: u32,
    pub(super) is_currently_gapping: bool,
}
impl AirGapData {
    fn new() -> Self {
        Self {
            current_gap_start_position: ZERO,
            current_gap_distance: 0.0,
            accumulated_gap_distance: 0.0,
            maximum_gap_distance: 0.0,
            gap_count: 0,
            is_currently_gapping: false,
        }
    }
}
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent original air scoring flags"
)]
pub(super) struct AirCollector {
    pub(super) base: CollectorBase,
    pub(super) takeoff_deck_position: SkeletonRootVector,
    pub(super) takeoff_normal: SkeletonRootVector,
    pub(super) takeoff_ground: AirGroundData,
    pub(super) gaps: [AirGapData; 4],
    pub(super) first_scorable: Scorable,
    pub(super) air_scorable: Scorable,
    pub(super) takeoff_altitude: f32,
    pub(super) highest_altitude: f32,
    pub(super) horizontal_distance: f32,
    pub(super) height_to_peak: f32,
    pub(super) total_height_gain: f32,
    pub(super) total_trick_points: f32,
    pub(super) air_grab_time: f32,
    #[cfg(feature = "skate3-runtime")]
    pub(super) skate3_rotation: Option<crate::skate3_native::scoring_runtime::rotation::Rotation>,
    /// TU3 AirCollector byte set by each Update2 (82DA8AA4): the air output's locked trajectory.
    #[cfg(feature = "skate3-runtime")]
    pub(super) skate3_locked: bool,
    pub(super) player_spin_tracker: RotationTracker,
    pub(super) board_spin_tracker: RotationTracker,
    pub(super) initial_context: ContextGroups,
    pub(super) accumulated_context: ContextGroups,
    pub(super) trick_count: u32,
    pub(super) tweaked_grab_sequence_count: u32,
    pub(super) player_spin180_count: i32,
    pub(super) player_num_flips: i32,
    pub(super) air_frames: i32,
    pub(super) horizontal_distance_points: f32,
    pub(super) height_to_peak_points: f32,
    pub(super) total_height_gain_points: f32,
    pub(super) player_spin_points: f32,
    pub(super) player_flip_points: f32,
    pub(super) most_recent_trick_point_scalar: f32,
    pub(super) in_air_point_scalar: f32,
    pub(super) banked_spin_value_degrees: f32,
    pub(super) started_switched: bool,
    pub(super) started_fakie: bool,
    pub(super) started_from_stationary: bool,
    pub(super) started_from_manual: bool,
    pub(super) started_from_grind: bool,
    pub(super) tweak_occurred_in_air: bool,
    pub(super) grab_occurred_in_air: bool,
    pub(super) accumulating_both_board_and_player_spin: bool,
    pub(super) has_flip_attempt_been_reported: bool,
    pub(super) most_recent_trick_point_scalar_is_set: bool,
}
impl AirCollector {
    /// Retain this prefix before calling reset: each failed output access leaves
    /// the already executed constructor/reset writes visible in the allocation.
    pub(super) fn before_reset_sim(base: CollectorBase) -> Self {
        let rotation = || RotationTracker {
            last_frame_matrix: Some(SkeletonRootMatrix::from_columns([ZERO; 4])),
            accumulated_angle_radians: 0.0,
            rotation_type: 0,
        };
        Self {
            base,
            takeoff_deck_position: ZERO,
            takeoff_normal: ZERO,
            takeoff_ground: AirGroundData::new(),
            gaps: std::array::from_fn(|_| AirGapData::new()),
            first_scorable: Scorable::new(),
            air_scorable: Scorable::new(),
            takeoff_altitude: 0.0,
            highest_altitude: 0.0,
            horizontal_distance: 0.0,
            height_to_peak: 0.0,
            total_height_gain: 0.0,
            total_trick_points: 0.0,
            air_grab_time: 0.0,
            #[cfg(feature = "skate3-runtime")]
            skate3_rotation: None,
            #[cfg(feature = "skate3-runtime")]
            skate3_locked: false,
            player_spin_tracker: rotation(),
            board_spin_tracker: rotation(),
            initial_context: ContextGroups::new(),
            accumulated_context: ContextGroups::new(),
            trick_count: 0,
            tweaked_grab_sequence_count: 0,
            player_spin180_count: 0,
            player_num_flips: 0,
            air_frames: 0,
            horizontal_distance_points: 0.0,
            height_to_peak_points: 0.0,
            total_height_gain_points: 0.0,
            player_spin_points: 0.0,
            player_flip_points: 0.0,
            most_recent_trick_point_scalar: 0.0,
            in_air_point_scalar: 0.0,
            banked_spin_value_degrees: 0.0,
            started_switched: false,
            started_fakie: false,
            started_from_stationary: false,
            started_from_manual: false,
            started_from_grind: false,
            tweak_occurred_in_air: false,
            grab_occurred_in_air: false,
            accumulating_both_board_and_player_spin: false,
            has_flip_attempt_been_reported: false,
            most_recent_trick_point_scalar_is_set: false,
        }
    }
    pub(super) fn reset(&mut self, output: &PhysicalPlayerPhysOut) -> Result<(), AirResetError> {
        if !Arc::ptr_eq(&self.base.output, output.occurrence()) {
            return Err(AirResetError::ForeignOutput);
        }
        self.base.reset_analysis();
        #[cfg(feature = "skate3-runtime")]
        { self.skate3_rotation = None; }
        output
            .check_current_guard()
            .map_err(AirResetError::Access)?;
        self.takeoff_deck_position = output
            .concrete
            .skateboard_reckoning
            .as_ref()
            .ok_or(AirResetError::Missing(
                PhysOutComponent::SkateboardReckoning,
            ))?
            .deck_position;
        self.takeoff_normal = self
            .base
            .global_metrics
            .as_ref()
            .ok_or(AirResetError::MissingMetrics)?
            .lock()
            .map_err(|_| AirResetError::MetricsPoisoned)?
            .takeoff_normal;
        self.takeoff_altitude = self.takeoff_deck_position.xyz[1];
        self.highest_altitude = self.takeoff_altitude;
        self.takeoff_ground.set_takeoff(output)?;
        for gap in &mut self.gaps {
            *gap = AirGapData::new();
        }
        self.horizontal_distance = 0.0;
        self.height_to_peak = 0.0;
        self.total_height_gain = 0.0;
        self.total_trick_points = 0.0;
        self.air_grab_time = 0.0;
        self.horizontal_distance_points = 0.0;
        self.height_to_peak_points = 0.0;
        self.total_height_gain_points = 0.0;
        self.player_spin_points = 0.0;
        self.player_flip_points = 0.0;
        output
            .check_current_guard()
            .map_err(AirResetError::Access)?;
        self.player_spin_tracker.reset(
            output
                .concrete
                .skeleton
                .as_ref()
                .ok_or(AirResetError::Missing(PhysOutComponent::Skeleton))?
                .skeleton_root_matrix,
        );
        output
            .check_current_guard()
            .map_err(AirResetError::Access)?;
        self.board_spin_tracker.reset(
            output
                .concrete
                .skateboard_reckoning
                .as_ref()
                .ok_or(AirResetError::Missing(
                    PhysOutComponent::SkateboardReckoning,
                ))?
                .transform,
        );
        self.accumulating_both_board_and_player_spin = true;
        self.has_flip_attempt_been_reported = false;
        self.trick_count = 0;
        self.tweaked_grab_sequence_count = 0;
        self.player_num_flips = 0;
        self.player_spin180_count = 0;
        self.air_frames = 0;
        self.started_switched = false;
        self.started_fakie = false;
        self.started_from_stationary = false;
        self.started_from_manual = false;
        self.started_from_grind = false;
        self.tweak_occurred_in_air = false;
        self.grab_occurred_in_air = false;
        self.initial_context.reset();
        self.accumulated_context.reset();
        self.banked_spin_value_degrees = 0.0;
        self.in_air_point_scalar = 1.0;
        self.air_scorable.reset();
        self.first_scorable.reset();
        self.most_recent_trick_point_scalar = 1.0;
        self.most_recent_trick_point_scalar_is_set = false;
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AirResetError {
    ForeignOutput,
    Access(PhysOutAccessError),
    Missing(PhysOutComponent),
    MetricsPoisoned,
    MissingMetrics,
}
