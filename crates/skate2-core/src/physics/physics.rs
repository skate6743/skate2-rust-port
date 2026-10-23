//! Native animation-to-physics handoff recovered from Skate 2.
//!
//! The retail runtime stores this state in the 10,928-byte
//! `Sk8::Physics::AnimOutPhysIn` console structure. This module models the
//! behavior as readable Rust data instead of preserving console offsets or
//! translated instructions.

use crate::animation::AnimationMatrix;
use skate2_formats::pegasus::PhysicsSurface;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt;

pub const PHYSICS_STEP_SECONDS: f32 = 1.0 / 60.0;
pub const DEFAULT_COMPRESSION: f32 = 0.5;
const WORLD_UP: [f32; 3] = [0.0, 1.0, 0.0];
const AFFINE_IDENTITY_ORIENTATION: AnimationMatrix = [
    1.0, 0.0, 0.0, 0.0, // x axis
    0.0, 1.0, 0.0, 0.0, // y axis
    0.0, 0.0, 1.0, 0.0, // z axis
    0.0, 0.0, 0.0, 0.0, // zero translation
];
/// Default of `kMaxJumpError`; the reference compares it with squared error.
pub const DEFAULT_MAXIMUM_JUMP_VELOCITY_ERROR_SQUARED: f32 = 7.0;
pub const IDENTITY_MATRIX: AnimationMatrix = [
    1.0, 0.0, 0.0, 0.0, // x axis
    0.0, 1.0, 0.0, 0.0, // y axis
    0.0, 0.0, 1.0, 0.0, // z axis
    0.0, 0.0, 0.0, 1.0, // translation
];

/// Fixed authored curve used throughout the physics attribute database.
#[derive(Clone, Debug, PartialEq)]
pub struct PointGraph<const N: usize> {
    pub x: [f32; N],
    pub y: [f32; N],
}

impl<const N: usize> Default for PointGraph<N> {
    fn default() -> Self {
        Self {
            x: [0.0; N],
            y: [0.0; N],
        }
    }
}

impl<const N: usize> PointGraph<N> {
    /// Reference: `PointGraphEval::Evaluate` at `0x824e2c40`.
    /// Endpoint/search branches preserve ordered comparisons: an unordered last
    /// endpoint comparison selects the last sample. Interpolation uses the
    /// source's fused multiply-add; nonpositive or unordered widths select the
    /// upper sample. This does not claim native exceptional FP payload parity.
    ///
    /// # Panics
    ///
    /// Panics when instantiated with zero authored points (a checked host
    /// boundary before the native out-of-bounds read), or if no interval exists.
    /// The latter stops at the native assertion rather than modeling a debugger
    /// resuming past it and returning the first sample.
    #[must_use]
    pub fn evaluate(&self, value: f32) -> f32 {
        assert!(N > 0, "a point graph must contain at least one sample");
        if value < self.x[0] {
            return self.y[0];
        }
        if value.partial_cmp(&self.x[N - 1]) != Some(Ordering::Less) {
            return self.y[N - 1];
        }
        let upper = (1..N)
            .find(|&index| value < self.x[index])
            .expect("point graph did not contain the selected interval");
        let lower = upper - 1;
        let width = self.x[upper] - self.x[lower];
        if width > 0.0 {
            let delta = value - self.x[lower];
            let slope = (self.y[upper] - self.y[lower]) / width;
            slope.mul_add(delta, self.y[lower])
        } else {
            self.y[upper]
        }
    }
}

/// Embedded curves from `physics_jump.default` consumed by
/// `Skateboard::Toolkit_CalcGroundJump`.
#[derive(Clone, Debug, PartialEq)]
pub struct JumpPhysicsCurves {
    pub vertical_response: PointGraph<16>,
    pub jump_y_scalar_vs_ground_normal_y: PointGraph<8>,
    pub jump_speed_scalar_vs_angle: PointGraph<8>,
    pub min_height_vs_speed: PointGraph<8>,
    pub max_height_vs_speed: PointGraph<8>,
}

/// Scalar settings from `physics_jump.default` used by the grounded jump
/// solver. The executable retains only hashed lookups for these fields, so
/// names describe their proven use at `Skateboard::Toolkit_CalcGroundJump`
/// rather than claiming unverified original schema spelling.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GroundJumpTuning {
    /// Surface speed which maps to `1.0` on the two height-versus-speed curves.
    pub height_curve_reference_speed: f32,
    /// Shared low-speed endpoint used by both jump-height ranges.
    pub low_speed_jump_height: f32,
    /// Upper endpoint selected by `MinHeightVsSpeed`.
    pub minimum_jump_height: f32,
    /// Upper endpoint selected by `MaxHeightVsSpeed`.
    pub maximum_jump_height: f32,
    /// Enables the ground-angle `VerticalResponse` curve.
    pub use_vertical_response_curve: bool,
    /// Lower bound applied after evaluating `VerticalResponse`.
    pub minimum_vertical_response: f32,
    /// Cap for the ground-normal horizontal compensation factor.
    pub maximum_horizontal_compensation: f32,
    /// Scale applied to the animation-authored forward jump adjustment.
    pub forward_adjustment_scale: f32,
    /// Scale applied to the animation-authored sideways jump adjustment.
    pub sideways_adjustment_scale: f32,
}

/// Live values needed for the vertical portion of a grounded jump. Positions
/// and vectors use the native PC world's ordinary three-component floats.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundJumpVerticalInput {
    pub surface_speed: f32,
    pub jump_strength: f32,
    pub gravity: f32,
    pub jump_normal: [f32; 3],
    pub centre_of_mass: [f32; 3],
    pub ground_position: [f32; 3],
    pub reckoning_up: [f32; 3],
}

/// Intermediate values retained explicitly so later board-state integration
/// can be tested without recreating console temporaries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundJumpVerticalSolution {
    pub normalized_surface_speed: f32,
    pub minimum_height: f32,
    pub maximum_height: f32,
    pub target_height: f32,
    pub current_height: f32,
    pub vertical_response: f32,
    pub launch_speed: f32,
}

/// Inputs to the horizontal velocity reconciliation performed before the
/// grounded jump's vertical impulse is added.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundJumpTangentialInput {
    pub current_velocity: [f32; 3],
    pub prepared_jump_velocity: [f32; 3],
    pub jump_normal: [f32; 3],
    /// `kMaxJumpError` is compared directly with squared vector error in the
    /// reference, despite the `CVar`'s non-squared display name.
    pub maximum_velocity_error_squared: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundJumpTangentialSolution {
    pub current_tangent: [f32; 3],
    pub prepared_tangent: [f32; 3],
    pub selected_tangent: [f32; 3],
    pub normalized_direction_angle: f32,
    pub corrected: bool,
}

/// Complete live input to the normal grounded-jump launch calculation.
///
/// The nested height input keeps that independently testable stage visible;
/// its jump normal is also shared by tangent reconciliation. The remaining
/// fields describe velocity and animation adjustment in the board's local X/Z
/// frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundJumpInput {
    pub vertical: GroundJumpVerticalInput,
    pub current_velocity: [f32; 3],
    pub prepared_jump_velocity: [f32; 3],
    pub maximum_velocity_error_squared: f32,
    pub body_z_axis: [f32; 3],
    pub jump_adjustment_x: f32,
    pub jump_adjustment_z: f32,
}

/// Source-level result of `Skateboard::Toolkit_CalcGroundJump`'s normal jump
/// path. Intermediate values are exposed to make archive-driven tuning and
/// board-state integration independently verifiable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundJumpSolution {
    pub vertical: GroundJumpVerticalSolution,
    pub tangential: GroundJumpTangentialSolution,
    pub launch_velocity: [f32; 3],
    pub vertical_compensation: f32,
    pub compensated_velocity: [f32; 3],
    pub velocity_along_jump_normal: f32,
    pub body_right_axis: [f32; 3],
    pub body_forward_axis: [f32; 3],
    pub new_velocity: [f32; 3],
    pub delta_velocity: [f32; 3],
    pub centre_of_mass_height: f32,
    pub do_jump: bool,
}

/// Bit-preserving storage for the four lanes of PDB type
/// `rw::math::vpu::Vector3`.
///
/// The reference type is 16 bytes and its single `__vector4` member is copied
/// in full at the physics-air boundaries. The fourth lane is retained even
/// though its geometric meaning has not yet been established.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C, align(16))]
pub struct PhysicsAirVector3 {
    pub lanes: [f32; 4],
}

/// Exact outer dependencies of
/// `PhysState_PhysicsAir::LaunchTrajectorySelector`.
///
/// `Skeleton::GetLaunchInfo`, jump-velocity VMX math, COM-trajectory
/// construction, and selector internals remain deliberately opaque. Reads are
/// methods so the wrapper can preserve their native conditional order.
pub trait PhysicsAirTrajectoryLaunchCollaborator {
    /// `Skeleton::GetLaunchInfo` at `0x82b46098`.
    fn populate_skeleton_launch_info(&mut self, launch_info: &mut TrajectoryLaunchInfo);

    /// `ProcessedPhysIn::mFramesSinceJumpCorrect` at `+0x920`.
    fn frames_since_jump_correct(&mut self) -> i32;

    /// Live `kPhysicsAirJumpFixFrames` value read at `0x82d99050`.
    fn physics_air_jump_fix_frames(&mut self) -> i32;

    /// `PhysState_PhysicsAir::CalcVelFromJump(int)` at `0x82d8af08`.
    fn calculate_velocity_from_jump(&mut self, frames_since_jump_correct: i32)
    -> PhysicsAirVector3;

    /// Late read of `PhysState_PhysicsAir::mUseCentreOfMassVel` at `+0xad`.
    fn use_centre_of_mass_velocity(&mut self) -> bool;

    /// `PhysState_PhysicsAir::GetComTrajectoryLaunchInfo` at `0x82d84050`.
    ///
    /// The native helper begins by calling `Skeleton::GetLaunchInfo` again;
    /// the collaborator owns that second snapshot and the unrecovered SIMD
    /// calculations which follow it.
    fn populate_com_trajectory_launch_info(&mut self, launch_info: &mut TrajectoryLaunchInfo);

    /// `TrajectorySelector::Launch` at `0x82dc4098`.
    fn launch(&mut self, launch_info: &mut TrajectoryLaunchInfo);
}

/// Runs the exact outer orchestration of
/// `PhysState_PhysicsAir::LaunchTrajectorySelector`.
///
/// Reference: `0x82d99010..0x82d990b4`. The signed frame gate uses strict
/// less-than. Its early branch replaces all four `start_velocity` lanes and
/// suppresses the COM-flag read; the late COM helper may replace the complete
/// launch record. `player_jumped` is forced false immediately before Launch.
pub fn launch_physics_air_trajectory_selector<C: PhysicsAirTrajectoryLaunchCollaborator>(
    collaborator: &mut C,
) {
    let mut launch_info = TrajectoryLaunchInfo::default();
    collaborator.populate_skeleton_launch_info(&mut launch_info);

    let frames_since_jump_correct = collaborator.frames_since_jump_correct();
    let jump_fix_frames = collaborator.physics_air_jump_fix_frames();
    if frames_since_jump_correct < jump_fix_frames {
        launch_info.start_velocity =
            collaborator.calculate_velocity_from_jump(frames_since_jump_correct);
    } else if collaborator.use_centre_of_mass_velocity() {
        collaborator.populate_com_trajectory_launch_info(&mut launch_info);
    }

    launch_info.player_jumped = false;
    collaborator.launch(&mut launch_info);
}

impl PhysicsAirVector3 {
    /// Adapts a legacy three-lane reconstruction to the PDB's 16-byte vector.
    ///
    /// Positive zero in the unresolved fourth lane is an explicit
    /// compatibility policy, not a recovered native result.
    #[must_use]
    pub const fn from_xyz_with_positive_zero_w(value: [f32; 3]) -> Self {
        Self {
            lanes: [value[0], value[1], value[2], 0.0],
        }
    }

    #[must_use]
    pub const fn xyz(self) -> [f32; 3] {
        [self.lanes[0], self.lanes[1], self.lanes[2]]
    }
}

/// Native 0x110-byte representation of the PDB-named trajectory launch record
/// populated by `Skeleton::GetLaunchInfo`.
///
/// Reference: `TrajectoryLaunchInfo::TrajectoryLaunchInfo` at `0x82d83cf8`
/// and `PhysState_PhysicsAir::LaunchTrajectorySelector` at `0x82d99010`.
/// Each PDB `Vector3` retains its otherwise-uninterpreted fourth SIMD lane.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct TrajectoryLaunchInfo {
    pub reckoning_orientation: AnimationMatrix,
    pub inverse_reckoning_orientation: AnimationMatrix,
    pub start_velocity: PhysicsAirVector3,
    pub velocity_before_jump: PhysicsAirVector3,
    pub animated_centre_of_mass_position: PhysicsAirVector3,
    pub animated_board_position: PhysicsAirVector3,
    pub board_position: PhysicsAirVector3,
    pub centre_of_mass_position: PhysicsAirVector3,
    pub trajectory_start_position_override: PhysicsAirVector3,
    pub board_position_override: PhysicsAirVector3,
    pub cone_angle_x: f32,
    pub cone_angle_z: f32,
    pub time_delta: f32,
    pub player_jumped: bool,
    pub use_trajectory_start_position_override: bool,
    pub number_of_trajectories: u16,
}

/// Reference: `TrajectoryLaunchInfo::TrajectoryLaunchInfo` at `0x82d83cf8`.
/// The game's affine orientations use a zero translation vector; every other
/// field starts at zero.
impl Default for TrajectoryLaunchInfo {
    fn default() -> Self {
        Self {
            reckoning_orientation: AFFINE_IDENTITY_ORIENTATION,
            inverse_reckoning_orientation: AFFINE_IDENTITY_ORIENTATION,
            start_velocity: PhysicsAirVector3::default(),
            velocity_before_jump: PhysicsAirVector3::default(),
            animated_centre_of_mass_position: PhysicsAirVector3::default(),
            animated_board_position: PhysicsAirVector3::default(),
            board_position: PhysicsAirVector3::default(),
            centre_of_mass_position: PhysicsAirVector3::default(),
            trajectory_start_position_override: PhysicsAirVector3::default(),
            board_position_override: PhysicsAirVector3::default(),
            cone_angle_x: 0.0,
            cone_angle_z: 0.0,
            time_delta: 0.0,
            player_jumped: false,
            use_trajectory_start_position_override: false,
            number_of_trajectories: 0,
        }
    }
}

pub const MAX_TRAJECTORY_CANDIDATES: usize = 7;
const INITIAL_TRAJECTORY_SCORE: f32 = -100_000_000.0;

/// Native form of the PDB-proven 64-byte `tTrajectory` record. The reference
/// stores each three-component value in a padded vector; source-level code
/// needs only its ballistic meaning.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BallisticTrajectory {
    pub initial_position: [f32; 3],
    pub initial_velocity: [f32; 3],
    pub gravity: [f32; 3],
    pub duration: f32,
}

/// Authored predictor controls. `radius` comes from
/// `physics_reckoning.default`; the remaining values come from
/// `physics_trajectory.default`. `maximum_time` is consumed while constructing
/// a trajectory, while the error ratios control adaptive chord length.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrajectoryPredictionTuning {
    pub radius: f32,
    pub maximum_time: f32,
    pub maximum_error_start: f32,
    pub maximum_error_end: f32,
}

/// Source-level equivalent of the PDB-proven `TrajectoryResult` record.
/// A negative collision time is the reference's no-hit sentinel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrajectoryPredictionResult {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub ground_normal: [f32; 3],
    pub collision_time: f32,
    pub collision_frame: i32,
    pub surface_id: u32,
}

impl Default for TrajectoryPredictionResult {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            normal: WORLD_UP,
            ground_normal: WORLD_UP,
            collision_time: -1.0,
            collision_frame: -1,
            surface_id: 0,
        }
    }
}

/// Inputs queued by `Predictor::AddQuery` at `0x82e4b868` after asynchronous
/// job ownership has been removed from the gameplay-facing representation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrajectoryPredictionQuery {
    pub trajectory: BallisticTrajectory,
    pub radius: f32,
    pub maximum_error_start: f32,
    pub maximum_error_end: f32,
    pub calculate_ground_normal: bool,
}

/// Nearest result of one fat line segment against the collision collection.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrajectorySweepHit {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub surface_id: u32,
}

/// Triangle copied into the reference predictor's fixed 64-element cache.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrajectoryGroundTriangle {
    pub vertices: [[f32; 3]; 3],
    pub surface_id: u32,
}

/// Geometry boundary used by the native trajectory walk. The reference backs
/// both operations with `ClusteredMeshCollection`; retaining the two semantic
/// queries prevents scheduler and console-memory details leaking into physics.
pub trait TrajectoryCollisionWorld {
    type Error;

    /// Returns the nearest collision of a radius-fat segment, if any.
    ///
    /// # Errors
    ///
    /// Returns a backend-specific error if collision geometry cannot be read.
    fn sweep_sphere_segment(
        &mut self,
        start: [f32; 3],
        end: [f32; 3],
        radius: f32,
    ) -> Result<Option<TrajectorySweepHit>, Self::Error>;

    /// Collects overlapping ground-normal candidates in traversal order.
    ///
    /// # Errors
    ///
    /// Returns a backend-specific error if collision geometry cannot be read.
    fn triangles_overlapping_aabb(
        &mut self,
        minimum: [f32; 3],
        maximum: [f32; 3],
        capacity: usize,
    ) -> Result<Vec<TrajectoryGroundTriangle>, Self::Error>;
}

/// Native data retained for one trajectory after prediction and score
/// calculation. The fields mirror the semantic PDB types
/// `PredictionResults` and `TrajectorySelector::TrajectoryData`; predictor
/// job bookkeeping is deliberately kept outside the gameplay state. The
/// trajectory velocity retains all four native vector lanes so second-pass
/// writeback does not discard a lane supplied by the predictor.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrajectoryCandidate {
    pub predicted_trajectory: BallisticTrajectory,
    pub trajectory_velocity: PhysicsAirVector3,
    pub landing_normal: [f32; 3],
    pub landing_velocity: [f32; 3],
    pub landing_position: [f32; 3],
    pub collision_normal: [f32; 3],
    pub collision_time: f32,
    pub collision_frame: i32,
    pub score: f32,
    pub is_wall_ride: bool,
}

/// Authored values consumed by trajectory initialization and selection.
/// Their archive loading remains separate so this state machine can be
/// tested against reference traces and real VLT data.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrajectorySelectionTuning {
    pub landing_time_bonus: PointGraph<8>,
    pub landing_centre_of_mass_scalar_vs_slope: PointGraph<8>,
    pub grind_penalty_vs_distance_to_grind: PointGraph<8>,
    pub landing_force_scalar_vs_dot_velocity_normal: PointGraph<8>,
    pub cone_angle_z_vs_speed: PointGraph<8>,
    pub speed_factor_minimum: f32,
    pub speed_factor_maximum: f32,
    pub score_middle_bonus: f32,
    pub score_landing_force: f32,
    pub score_landing_direction: f32,
    pub score_grind: f32,
    pub score_transition: f32,
    pub score_surface_unrideable: f32,
    pub score_surface_do_not_align: f32,
    pub grind_detection_radius: f32,
    pub grind_tip_scalar: f32,
    pub grind_offset: f32,
    pub grind_maximum_speed_squared_into_ledge: f32,
    pub grind_maximum_speed_squared_into_grind: f32,
    pub grind_maximum_speed_down_onto_grind: f32,
    pub grind_lock_to_tip_distance: f32,
    pub grind_lock_to_centre_distance: f32,
    pub grind_lock_ledge_scalar_squared: f32,
    pub grind_lock_ledge_scalar: f32,
    pub grind_lock_ledge_low_side_scalar_squared: f32,
    pub grind_lock_ledge_low_side_scalar: f32,
    pub grind_landing_velocity_scalar: f32,
    pub grind_landing_maximum_angle: f32,
    pub grind_adjustment_maximum_angle: f32,
    /// Behavior-identified 80-byte `PointNegGraphData8` used by
    /// `FindBestGrind`; its original schema name is absent from the PDB.
    pub grind_minimum_angle_improvement_vs_height_delta: PointGraph<8>,
    pub maximum_trajectory_adjustment: f32,
    pub minimum_valid_collision_time: f32,
    pub minimum_time_after_apex_for_transition: f32,
    pub minimum_normal_delta_for_second_pass: f32,
    pub cone_angle_x_second_pass: f32,
    pub cone_angle_z_second_pass: f32,
}

/// Authored wallride gates and scoring values used by the trajectory selector.
/// The first two values live in `physics_feet.default`; the remainder live in
/// `physics_trajectory.default`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrajectoryWallRideTuning {
    pub maximum_collision_normal_y: f32,
    pub maximum_ground_wall_normal_dot: f32,
    pub boost_vs_height: PointGraph<4>,
    pub test_distance: f32,
    pub minimum_height_from_ground: f32,
    pub maximum_landing_slope: f32,
    pub score_per_height: f32,
}

/// Results supplied when the native prediction jobs for a pass have all
/// completed. Score construction, grind analysis, and wall-ride bonuses are
/// separate recovered algorithms; this structure is their typed boundary.
#[derive(Clone, Copy, Debug)]
pub struct TrajectoryPredictionBatch<'a> {
    pub candidates: &'a [TrajectoryCandidate],
    pub grind_bonus_awarded: bool,
}

/// Named boolean state imported from the PDB layout of
/// `Sk8::Physics::TrajectorySelector`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TrajectorySelectorFlag {
    Ready,
    QueryInFlight,
    HasJustChanged,
    AiTrajectoryFollowing,
    GrindBonusAwarded,
    TargetingGrind,
    TargetingWallRide,
    SuggestedLandingNormalValid,
    MostVerticalGrindNormalValid,
}

/// Human-readable control state for `TrajectorySelector::Launch` and
/// `TrajectorySelector::Update`.
///
/// This models pass scheduling, readiness, winner selection, the early-hit
/// branch, and second-pass launch exactly. It does not pretend that the
/// collision predictor or trajectory score functions are already recovered;
/// completed, scored candidates enter through `TrajectoryPredictionBatch`.
#[derive(Clone, Debug, PartialEq)]
pub struct TrajectorySelectorState {
    pub launch_info: TrajectoryLaunchInfo,
    pub active_trajectories: usize,
    pub pass: u16,
    pub best_trajectory: Option<usize>,
    pub flags: BTreeSet<TrajectorySelectorFlag>,
    pub suggested_landing_normal: [f32; 3],
    pub most_vertical_grind_normal: [f32; 3],
    pub candidates: [TrajectoryCandidate; MAX_TRAJECTORY_CANDIDATES],
    pub winning_candidate: Option<TrajectoryCandidate>,
}

impl Default for TrajectorySelectorState {
    fn default() -> Self {
        Self {
            launch_info: TrajectoryLaunchInfo::default(),
            active_trajectories: 0,
            pass: 0,
            best_trajectory: None,
            flags: BTreeSet::new(),
            suggested_landing_normal: [0.0; 3],
            most_vertical_grind_normal: WORLD_UP,
            candidates: [TrajectoryCandidate::default(); MAX_TRAJECTORY_CANDIDATES],
            winning_candidate: None,
        }
    }
}

impl TrajectorySelectorState {
    /// Reference: public `TrajectorySelector::Launch` at `0x82dc4098` followed
    /// by its private dispatch at `0x82dc2800`.
    ///
    /// A launch request is ignored while a predictor pass is outstanding.
    /// `adjust_starting_velocity` is the still-separate
    /// `AdjustStartingVelOnVerts` boundary and is invoked in the reference
    /// order: after copying launch info, before generating predictions.
    pub fn launch(
        &mut self,
        mut launch_info: TrajectoryLaunchInfo,
        tuning: &TrajectorySelectionTuning,
        adjust_starting_velocity: impl FnOnce(&mut TrajectoryLaunchInfo),
    ) -> bool {
        if self.has(TrajectorySelectorFlag::QueryInFlight) {
            return false;
        }

        self.most_vertical_grind_normal = WORLD_UP;
        self.flags
            .remove(&TrajectorySelectorFlag::MostVerticalGrindNormalValid);
        self.pass = 0;
        self.best_trajectory = None;
        self.flags.remove(&TrajectorySelectorFlag::Ready);
        self.flags
            .remove(&TrajectorySelectorFlag::GrindBonusAwarded);
        adjust_starting_velocity(&mut launch_info);
        self.launch_info = launch_info;
        self.dispatch_prediction_pass(tuning);
        true
    }

    /// Reference: `TrajectorySelector::Reset` at `0x82db5f78`.
    pub fn reset(&mut self) {
        self.flags.remove(&TrajectorySelectorFlag::Ready);
        self.flags
            .remove(&TrajectorySelectorFlag::AiTrajectoryFollowing);
        self.flags
            .remove(&TrajectorySelectorFlag::SuggestedLandingNormalValid);
        self.flags
            .remove(&TrajectorySelectorFlag::MostVerticalGrindNormalValid);
        self.flags.remove(&TrajectorySelectorFlag::HasJustChanged);
        self.pass = 0;
    }

    #[must_use]
    pub fn has(&self, flag: TrajectorySelectorFlag) -> bool {
        self.flags.contains(&flag)
    }

    /// Advances selection when the current prediction pass is ready. `None`
    /// represents the reference future's not-ready state and preserves the
    /// last usable result.
    ///
    /// Reference: `TrajectorySelector::Update` at `0x82dc6528` and
    /// `IsSecondPassRequired` at `0x82db98a0`.
    ///
    /// # Panics
    ///
    /// Panics if a completed internal prediction batch has no active
    /// trajectory, is shorter than the active count, or violates the
    /// reference collision-time invariant.
    pub fn update(
        &mut self,
        completed: Option<TrajectoryPredictionBatch<'_>>,
        current_ground_normal: [f32; 3],
        tuning: &TrajectorySelectionTuning,
    ) -> bool {
        self.flags.remove(&TrajectorySelectorFlag::HasJustChanged);
        let Some(completed) = completed else {
            return self.has(TrajectorySelectorFlag::Ready);
        };
        if !self.has(TrajectorySelectorFlag::QueryInFlight) {
            return self.has(TrajectorySelectorFlag::Ready);
        }
        assert!(
            self.active_trajectories > 0,
            "a selector pass needs a trajectory"
        );
        assert!(
            completed.candidates.len() >= self.active_trajectories,
            "prediction batch is shorter than the active trajectory count"
        );

        if self.pass == 2 {
            self.flags.insert(TrajectorySelectorFlag::HasJustChanged);
        }
        self.flags.remove(&TrajectorySelectorFlag::QueryInFlight);
        self.pass += 1;
        if completed.grind_bonus_awarded {
            self.flags.insert(TrajectorySelectorFlag::GrindBonusAwarded);
        } else {
            self.flags
                .remove(&TrajectorySelectorFlag::GrindBonusAwarded);
        }

        let candidates = &completed.candidates[..self.active_trajectories];
        let mut best_index = 0;
        let mut best_score = INITIAL_TRAJECTORY_SCORE;
        for (index, candidate) in candidates.iter().enumerate() {
            if candidate.score > best_score {
                best_score = candidate.score;
                best_index = index;
            }
        }
        let winner = candidates[best_index];
        self.best_trajectory = Some(best_index);
        self.winning_candidate = Some(winner);
        if winner.is_wall_ride {
            self.flags.insert(TrajectorySelectorFlag::TargetingWallRide);
        } else {
            self.flags
                .remove(&TrajectorySelectorFlag::TargetingWallRide);
        }
        if completed.grind_bonus_awarded && best_index == 0 {
            self.flags.insert(TrajectorySelectorFlag::TargetingGrind);
        } else {
            self.flags.remove(&TrajectorySelectorFlag::TargetingGrind);
        }

        assert!(
            winner.collision_time.abs() < 100.0,
            "reference trajectory collision time invariant was violated"
        );
        if winner.collision_time < tuning.minimum_valid_collision_time {
            self.suggested_landing_normal = winner.collision_normal;
            self.flags
                .insert(TrajectorySelectorFlag::SuggestedLandingNormalValid);
            self.flags.remove(&TrajectorySelectorFlag::Ready);
            self.flags.remove(&TrajectorySelectorFlag::HasJustChanged);
            return false;
        }

        self.flags.insert(TrajectorySelectorFlag::Ready);
        let landing_angle = angle_between3(winner.landing_normal, current_ground_normal);
        let requires_second_pass = !self.has(TrajectorySelectorFlag::AiTrajectoryFollowing)
            && self.pass == 1
            && self.active_trajectories >= 2
            && landing_angle >= tuning.minimum_normal_delta_for_second_pass
            && !self.has(TrajectorySelectorFlag::TargetingGrind);
        if requires_second_pass {
            self.launch_info.start_velocity = winner.trajectory_velocity;
            self.launch_info.cone_angle_x = tuning.cone_angle_x_second_pass;
            self.launch_info.cone_angle_z = tuning.cone_angle_z_second_pass;
            self.dispatch_prediction_pass(tuning);
            self.pass = 2;
        }
        self.has(TrajectorySelectorFlag::Ready)
    }

    fn dispatch_prediction_pass(&mut self, tuning: &TrajectorySelectionTuning) {
        self.flags
            .remove(&TrajectorySelectorFlag::TargetingWallRide);
        self.active_trajectories =
            usize::from(self.launch_info.number_of_trajectories).min(MAX_TRAJECTORY_CANDIDATES);
        self.candidates = initialize_trajectory_candidates(&self.launch_info, tuning);
        self.flags.insert(TrajectorySelectorFlag::QueryInFlight);
    }
}

/// Builds the velocity fan consumed by the trajectory prediction jobs.
///
/// Reference: `TrajectorySelector::InitTrajectoryInfo` at `0x82db9018`.
/// Candidate zero preserves `startVel`. Remaining candidates are distributed
/// uniformly around the X/Z cone, whose longitudinal angle is modified by the
/// authored speed curve. As in the reference, a generated candidate may lose
/// speed but never exceed the magnitude of `startVel`.
#[must_use]
/// # Panics
/// Panics if the host numeric adapter unexpectedly rejects an operation.
pub fn initialize_trajectory_candidates(
    launch_info: &TrajectoryLaunchInfo,
    tuning: &TrajectorySelectionTuning,
) -> [TrajectoryCandidate; MAX_TRAJECTORY_CANDIDATES] {
    use crate::{
        reckoning_numeric::HostReckoningNumeric, reckoning_sin_cos::HostReckoningSinCosNumeric,
        skeleton_root_transforms::SkeletonRootVector,
    };
    use trajectory_fan::{FanAttributes, FanLaunch, FanStore, initialize_fan};
    struct Attributes<'a>(&'a TrajectorySelectionTuning);
    impl FanAttributes for Attributes<'_> {
        type Error = std::convert::Infallible;
        fn cone_z_graph(&mut self, input: f32) -> Result<f32, Self::Error> {
            Ok(self.0.cone_angle_z_vs_speed.evaluate(input))
        }
        fn speed_bounds(&mut self) -> Result<(f32, f32), Self::Error> {
            Ok((self.0.speed_factor_minimum, self.0.speed_factor_maximum))
        }
    }
    let mut candidates = [TrajectoryCandidate::default(); MAX_TRAJECTORY_CANDIDATES];
    initialize_fan(
        FanLaunch {
            start: SkeletonRootVector::known(launch_info.start_velocity.lanes),
            before_jump: SkeletonRootVector::known(launch_info.velocity_before_jump.lanes),
            cone_x: launch_info.cone_angle_x,
            cone_z: launch_info.cone_angle_z,
            jumped: launch_info.player_jumped,
            count: launch_info.number_of_trajectories,
        },
        &mut Attributes(tuning),
        &mut HostReckoningNumeric,
        &mut HostReckoningSinCosNumeric,
        |store| {
            if let FanStore::Velocity(index, value) = store {
                candidates[index].trajectory_velocity = PhysicsAirVector3 {
                    lanes: value
                        .known_lanes()
                        .expect("public fan inputs have known lanes"),
                };
            }
        },
    )
    .expect("host fan has no integer conversion or failing attribute/rounding service");
    candidates
}

#[path = "physics_trajectory_fan.rs"]
pub(crate) mod trajectory_fan;

/// Inputs retained by `TrajectorySelector::CalculateCOMTrajectory` after a
/// winner has been selected.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CentreOfMassTrajectoryInput {
    pub winning_trajectory: BallisticTrajectory,
    pub collision_frame: i32,
    pub landing_normal: [f32; 3],
    pub launch_centre_of_mass_position: [f32; 3],
    pub start_trajectory_position: [f32; 3],
    pub up_vector: [f32; 3],
}

/// Repositions the winning prediction onto the current centre of mass and
/// adjusts its launch velocity so the authored landing offset is retained.
///
/// Reference: `TrajectorySelector::CalculateCOMTrajectory` at `0x82dc02f8`
/// and `tTrajectory::AdjustTrajectory` at `0x82db6e78`.
#[must_use]
pub fn calculate_centre_of_mass_trajectory(
    input: CentreOfMassTrajectoryInput,
    tuning: &TrajectorySelectionTuning,
) -> BallisticTrajectory {
    let mut trajectory = input.winning_trajectory;
    if input.collision_frame <= 0 {
        return trajectory;
    }

    let initial_offset = subtract3(
        input.launch_centre_of_mass_position,
        trajectory.initial_position,
    );
    let vertical_offset = dot3(
        subtract3(
            input.start_trajectory_position,
            input.launch_centre_of_mass_position,
        ),
        input.up_vector,
    )
    .abs();
    let landing_scalar = tuning
        .landing_centre_of_mass_scalar_vs_slope
        .evaluate(input.landing_normal[1]);
    let desired_landing_offset = scale3(input.landing_normal, vertical_offset * landing_scalar);
    let adjustment = subtract3(desired_landing_offset, initial_offset);

    trajectory.initial_position = input.launch_centre_of_mass_position;
    trajectory.adjust(
        input.collision_frame,
        adjustment,
        tuning.maximum_trajectory_adjustment,
    );
    trajectory
}

#[path = "physics_trajectory_numeric.rs"]
mod trajectory_numeric;

#[derive(Debug)]
pub(crate) enum TrajectoryPredictionError<G, N> {
    Geometry(G),
    Numeric(N),
}

impl BallisticTrajectory {
    /// Reference: `PredictionResults::GetCollisionPosition` at `0x82d8a648`.
    #[must_use]
    pub fn position_at_time(&self, time: f32) -> [f32; 3] {
        debug_assert!(time.abs() < 100.0, "trajectory sample time is out of range");
        trajectory_numeric::position(self, time)
    }

    /// Reference: `tTrajectory::_GetTimeAtHighestPos` at `0x82db7198`.
    #[must_use]
    pub fn time_at_highest_position(&self) -> f32 {
        let upward_speed = self.initial_velocity[1];
        let downward_acceleration = -self.gravity[1];
        if upward_speed < 0.0 || downward_acceleration <= 0.0 {
            0.0
        } else {
            upward_speed / downward_acceleration
        }
    }

    /// Reference: `tTrajectory::AdjustTrajectory` at `0x82db6e78`.
    #[allow(clippy::cast_precision_loss)]
    pub fn adjust(&mut self, frame: i32, adjustment: [f32; 3], maximum_adjustment: f32) {
        debug_assert!(maximum_adjustment >= 0.0);
        if frame <= 0 {
            return;
        }
        let seconds = frame as f32 * PHYSICS_STEP_SECONDS;
        let mut velocity_adjustment = scale3(adjustment, seconds.recip());
        let adjustment_length_squared = length_squared3(velocity_adjustment);
        if adjustment_length_squared > maximum_adjustment * maximum_adjustment {
            velocity_adjustment = scale3(
                velocity_adjustment,
                maximum_adjustment / adjustment_length_squared.sqrt(),
            );
        }
        self.initial_velocity = add3(self.initial_velocity, velocity_adjustment);
    }

    /// Reference: `tTrajectory::Get_dTWithErrorR` at `0x827bf868`.
    ///
    /// A ballistic chord deviates from the parabola by `|g| * dT^2 / 8`.
    /// The reference solves that identity directly and requires gravity to be
    /// vertical and non-positive.
    #[must_use]
    #[allow(clippy::float_cmp)] // These are the reference's exact debug invariants.
    pub fn time_step_for_error_radius(&self, error_radius: f32) -> f32 {
        match trajectory_numeric::time_step(
            self,
            error_radius,
            &mut trajectory_numeric::LegacyTrajectoryNumeric,
        ) {
            Ok(value) => value,
            Err(never) => match never {},
        }
    }

    /// Reference: `tTrajectory::CalculateTimeOfCollision` at `0x827bce08`.
    ///
    /// The fat segment query supplies a point on a chord, not its exact time
    /// on the parabola. With normal gravity, the game walks forward in fixed
    /// physics frames until squared distance begins increasing, then selects
    /// the preceding frame. The effectively linear fallback retains the
    /// reference's unusual whole-unit frame conversion.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    pub fn refine_collision_time(
        &self,
        collision_position: [f32; 3],
        previous_time: f32,
    ) -> (f32, i32) {
        match trajectory_numeric::collision_time(
            self,
            collision_position,
            previous_time,
            &mut trajectory_numeric::LegacyTrajectoryNumeric,
        ) {
            Ok(value) => value,
            Err(never) => match never {},
        }
    }
}

/// Runs the collision portion of `LocalClusteredMeshTrajectoryTest` and
/// `tTrajectory::WalkTrajectory<TrajectoryWalkFunctor>` in source form.
///
/// Reference addresses: `0x827c2270`, `0x827c1e58`, and `0x827c1c78`.
/// The adaptive step is interpolated from the two authored error ratios. Short
/// successive chords remain accumulated until their displacement exceeds the
/// authored start-error radius, matching the reference's strict comparison.
///
/// # Errors
///
/// Returns the collision world's query error unchanged.
#[allow(clippy::cast_precision_loss, clippy::float_cmp)]
pub fn predict_ballistic_trajectory<W: TrajectoryCollisionWorld + ?Sized>(
    query: TrajectoryPredictionQuery,
    world: &mut W,
) -> Result<TrajectoryPredictionResult, W::Error> {
    match predict_ballistic_trajectory_with_numeric(
        query,
        world,
        &mut trajectory_numeric::LegacyTrajectoryNumeric,
    ) {
        Ok(result) => Ok(result.unwrap_or_default()),
        Err(TrajectoryPredictionError::Geometry(error)) => Err(error),
        Err(TrajectoryPredictionError::Numeric(never)) => match never {},
    }
}

/// Same source walker with caller-owned, fallible primitive numerical services.
#[allow(clippy::cast_precision_loss, clippy::float_cmp)]
pub(crate) fn predict_ballistic_trajectory_with_numeric<
    W: TrajectoryCollisionWorld + ?Sized,
    N: crate::reckoning_numeric::ReckoningNumeric,
>(
    query: TrajectoryPredictionQuery,
    world: &mut W,
    numeric: &mut N,
) -> Result<Option<TrajectoryPredictionResult>, TrajectoryPredictionError<W::Error, N::Error>> {
    let trajectory = query.trajectory;
    let (time_step_start, time_step_end) = if trajectory.gravity[1] < 0.0 {
        (
            trajectory_numeric::time_step(
                &trajectory,
                query.maximum_error_start * query.radius,
                numeric,
            )
            .map_err(TrajectoryPredictionError::Numeric)?,
            trajectory_numeric::time_step(
                &trajectory,
                query.maximum_error_end * query.radius,
                numeric,
            )
            .map_err(TrajectoryPredictionError::Numeric)?,
        )
    } else {
        (trajectory.duration, trajectory.duration)
    };

    let mut time = 0.0_f32;
    let mut segment_start = trajectory_numeric::position(&trajectory, time);
    let mut time_step = time_step_start;
    let mut segment_end = trajectory_numeric::position(&trajectory, time + time_step);
    // Walk827c1f78..1fe0 multiplies errorStart * radius * errorStart * radius.
    let chord_threshold =
        ((query.maximum_error_start * query.radius) * query.maximum_error_start) * query.radius;
    let mut result = None;

    while trajectory.duration > time {
        let chord = subtract3(segment_end, segment_start);
        if trajectory_numeric::dot(chord, chord, numeric)
            .map_err(TrajectoryPredictionError::Numeric)?
            > chord_threshold
        {
            if let Some(hit) = world
                .sweep_sphere_segment(segment_start, segment_end, query.radius)
                .map_err(TrajectoryPredictionError::Geometry)?
            {
                let (collision_time, collision_frame) =
                    trajectory_numeric::collision_time(&trajectory, hit.position, time, numeric)
                        .map_err(TrajectoryPredictionError::Numeric)?;
                let ground_normal = if query.calculate_ground_normal {
                    let extent = [query.radius; 3];
                    let minimum = subtract3(hit.position, extent);
                    let maximum = add3(hit.position, extent);
                    let triangles = world
                        .triangles_overlapping_aabb(
                            minimum,
                            maximum,
                            MAX_TRAJECTORY_GROUND_TRIANGLES,
                        )
                        .map_err(TrajectoryPredictionError::Geometry)?;
                    let frame_time = collision_frame as f32 * PHYSICS_STEP_SECONDS;
                    let landing_velocity = std::array::from_fn(|axis| {
                        trajectory.gravity[axis]
                            .mul_add(frame_time, trajectory.initial_velocity[axis])
                    });
                    calculate_trajectory_ground_normal(landing_velocity, &triangles)
                } else {
                    hit.normal
                };
                result = Some(TrajectoryPredictionResult {
                    position: hit.position,
                    normal: hit.normal,
                    ground_normal,
                    collision_time,
                    collision_frame,
                    surface_id: hit.surface_id,
                });
                break;
            }
            segment_start = segment_end;
        }

        let interpolation = trajectory_numeric::reciprocal(trajectory.duration, numeric)
            .map_err(TrajectoryPredictionError::Numeric)?
            * time;
        time_step = time_step_end.mul_add(interpolation, (1.0 - interpolation) * time_step_start);
        time += time_step;
        segment_end = trajectory_numeric::position(&trajectory, time + time_step);
    }

    Ok(result)
}

fn admit_aggregate_trajectory_result(
    nearest: &mut Option<TrajectoryPredictionResult>,
    candidate: TrajectoryPredictionResult,
) {
    if candidate.collision_time >= 0.0
        && match nearest {
            Some(current) => current.collision_time > candidate.collision_time,
            None => true,
        }
    {
        *nearest = Some(candidate);
    }
}

/// Runs a trajectory prediction independently against each collision provider
/// and returns the first strictly nearest complete result.
///
/// The provider slice must be in native aggregate order: Ground, DMO, then
/// Vehicle. `AggregateTrajectoryBatch::AddTest` at `0x827bc050` submits the
/// complete test to every provider; `AggregateTrajectoryBatch::GetResult` at
/// `0x827bc0f0` admits only nonnegative collision times and replaces the current
/// result only for a strictly smaller time. Consequently equal-time results,
/// including either signed zero, retain the earlier provider. Each provider's
/// sweep and fixed triangle cache remain isolated behind its own
/// [`TrajectoryCollisionWorld`].
///
/// # Errors
///
/// Returns the first provider error and does not query later providers. Native
/// collision batches do not expose an equivalent recoverable geometry-read
/// error; this short circuit is the safe host-side boundary.
pub fn predict_ballistic_trajectory_across_providers<E>(
    query: TrajectoryPredictionQuery,
    providers: &mut [&mut dyn TrajectoryCollisionWorld<Error = E>],
) -> Result<TrajectoryPredictionResult, E> {
    let mut nearest = None;
    for provider in providers {
        let candidate = predict_ballistic_trajectory(query, *provider)?;
        admit_aggregate_trajectory_result(&mut nearest, candidate);
    }
    Ok(nearest.unwrap_or_default())
}

const MAX_TRAJECTORY_GROUND_TRIANGLES: usize = 64;

/// Reference: `tTrajectory::CalculateGroundNormal<Triangle, 64>` at
/// `0x827bf5a0`.
///
/// Upward-facing triangles always participate. Steeper faces participate only
/// while the landing velocity points into them. The most vertical eligible
/// face defines a hemisphere, then faces within a 60-degree (`dot > 0.5`)
/// cone are accumulated and normalized.
#[must_use]
pub fn calculate_trajectory_ground_normal(
    landing_velocity: [f32; 3],
    triangles: &[TrajectoryGroundTriangle],
) -> [f32; 3] {
    let mut eligible = Vec::with_capacity(triangles.len().min(MAX_TRAJECTORY_GROUND_TRIANGLES));
    let mut most_vertical_normal = WORLD_UP;
    let mut most_vertical_dot = -1.0_f32;

    for (index, triangle) in triangles
        .iter()
        .take(MAX_TRAJECTORY_GROUND_TRIANGLES)
        .enumerate()
    {
        let first_edge = subtract3(triangle.vertices[1], triangle.vertices[0]);
        let second_edge = subtract3(triangle.vertices[2], triangle.vertices[0]);
        let normal = normalize_safe3(cross3(first_edge, second_edge));
        if normal[1] > 0.7 || dot3(normal, landing_velocity) <= 0.0 {
            let vertical_dot = dot3(normal, WORLD_UP);
            if vertical_dot > most_vertical_dot {
                most_vertical_normal = normal;
                most_vertical_dot = vertical_dot;
            }
            eligible.push(index);
        }
    }

    if eligible.is_empty() {
        return WORLD_UP;
    }

    let mut accumulated = [0.0; 3];
    for index in eligible {
        let triangle = &triangles[index];
        let first_edge = subtract3(triangle.vertices[1], triangle.vertices[0]);
        let second_edge = subtract3(triangle.vertices[2], triangle.vertices[0]);
        let normal = normalize_safe3(cross3(first_edge, second_edge));
        if dot3(most_vertical_normal, normal) > 0.5 {
            accumulated = add3(accumulated, normal);
        }
    }
    normalize_safe3(accumulated)
}

/// Inputs to the directly recovered portion of
/// `TrajectorySelector::CalculateTrajectoryScores`. Grind, surface, and
/// transition analysis remain independent collaborators so each recovered
/// routine can be traced and differentially tested on its own.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrajectoryScoreInput {
    pub candidate_index: usize,
    pub pass: u16,
    pub on_vert: bool,
    pub select_central_trajectory: bool,
    pub collision_time: f32,
    pub landing_normal: [f32; 3],
    pub landing_velocity: [f32; 3],
    pub grind_or_penalty_score: f32,
    pub surface_score: f32,
    pub transition_score: f32,
}

/// Named score terms retained for differential tests and future reference
/// traces. An invalid negative collision time leaves every term at zero.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrajectoryScore {
    pub valid_collision: bool,
    pub time_bonus: f32,
    pub landing_force: f32,
    pub landing_direction: f32,
    pub middle_bonus: f32,
    pub grind_or_penalty: f32,
    pub surface: f32,
    pub transition: f32,
    pub total: f32,
}

/// Result of the selector's surface classification. Some authored surface
/// types deliberately replace the collision normal used by the remaining
/// landing score terms.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrajectorySurfaceScore {
    pub physics_surface: Option<PhysicsSurface>,
    pub score: f32,
    pub collision_normal: [f32; 3],
}

/// A line segment returned by the selector's nearby-grind query.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GrindTestSegment {
    pub start: [f32; 3],
    pub end: [f32; 3],
}

/// Named intermediate retained so reference traces can compare both the
/// geometric query and its authored curve evaluation.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrajectoryGrindPenalty {
    pub distance_to_grind: f32,
    pub score: f32,
}

/// Axis-aligned nearby-grind query built around the predicted collision point.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GrindTrajectoryQuery {
    pub minimum: [f32; 3],
    pub maximum: [f32; 3],
    pub takeoff_position: [f32; 3],
}

/// State mutations and score returned by the recovered positive-grind path.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrajectoryGrindScore {
    pub query: GrindTrajectoryQuery,
    pub grind_test_results_valid: bool,
    pub grind_bonus_awarded: bool,
    pub score: f32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TrajectoryWallRideDecision {
    #[default]
    NotWall,
    NoGroundBelow,
    GroundNormalRejected,
    PenalizedNearGrind,
    PenalizedApproach,
    PenalizedTooLow,
    Adjusted,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrajectoryWallRideInput {
    pub collision_time: f32,
    pub collision_frame: i32,
    pub collision_normal: [f32; 3],
    pub near_grind: bool,
    pub highest_wall_ride_collision_y: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WallRideGroundHit {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrajectoryWallRideScore {
    pub decision: TrajectoryWallRideDecision,
    pub probe_start: [f32; 3],
    pub probe_end: [f32; 3],
    pub height_from_ground: f32,
    pub landing_slope: f32,
    pub trajectory_adjustment: [f32; 3],
    pub highest_wall_ride_collision_y: f32,
    pub score: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrajectoryTransitionScoreInput {
    pub on_vert: bool,
    pub transition_input: f32,
    pub ground_normal: [f32; 3],
    pub ground_velocity_damped: [f32; 3],
    pub collision_normal: [f32; 3],
    pub collision_time: f32,
    pub grind_or_penalty_score: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrajectoryTransitionScore {
    pub direction: f32,
    pub time_from_apex: f32,
    pub apex_penalty: f32,
    pub alignment: f32,
    pub grind_or_penalty_score: f32,
    pub grind_bonus_cancelled: bool,
    pub score: f32,
}

/// Reconstructs the inline vert-transition score in
/// `TrajectorySelector::CalculateTrajectoryScores` at `0x82dc62c4`.
#[must_use]
pub fn calculate_trajectory_transition_score(
    trajectory: &BallisticTrajectory,
    input: TrajectoryTransitionScoreInput,
    tuning: &TrajectorySelectionTuning,
) -> TrajectoryTransitionScore {
    if !input.on_vert {
        return TrajectoryTransitionScore {
            grind_or_penalty_score: input.grind_or_penalty_score,
            ..TrajectoryTransitionScore::default()
        };
    }

    let direction = if input.transition_input >= 0.5 {
        1.0
    } else {
        -1.0
    };
    let grind_bonus_cancelled = input.grind_or_penalty_score > 0.0 && direction < 0.0;
    let grind_or_penalty_score = if grind_bonus_cancelled {
        tuning.grind_penalty_vs_distance_to_grind.evaluate(0.0)
    } else {
        input.grind_or_penalty_score
    };
    let horizontal_ground_velocity = normalize_safe3([
        input.ground_velocity_damped[0],
        0.0,
        input.ground_velocity_damped[2],
    ]);
    let alignment = dot3(input.collision_normal, horizontal_ground_velocity);
    let time_from_apex = (input.collision_time - trajectory.time_at_highest_position()).abs();
    let apex_penalty = if time_from_apex >= tuning.minimum_time_after_apex_for_transition {
        0.0
    } else {
        -500.0
    };
    let score = alignment
        * ((1.0 - input.ground_normal[1].abs()) * direction * tuning.score_transition)
        + apex_penalty;

    TrajectoryTransitionScore {
        direction,
        time_from_apex,
        apex_penalty,
        alignment,
        grind_or_penalty_score,
        grind_bonus_cancelled,
        score,
    }
}

/// Reconstructs `TrajectorySelector::CalculateWallRideScore` at `0x82dc0558`.
/// Debug-overlay-only branches are intentionally omitted.
pub fn calculate_trajectory_wall_ride_score<F>(
    trajectory: &mut BallisticTrajectory,
    input: TrajectoryWallRideInput,
    tuning: &TrajectoryWallRideTuning,
    trajectory_tuning: &TrajectorySelectionTuning,
    line_test: F,
) -> TrajectoryWallRideScore
where
    F: FnOnce([f32; 3], [f32; 3]) -> Option<WallRideGroundHit>,
{
    let mut result = TrajectoryWallRideScore {
        highest_wall_ride_collision_y: input.highest_wall_ride_collision_y,
        ..TrajectoryWallRideScore::default()
    };
    if !matches!(
        input.collision_normal[1]
            .abs()
            .partial_cmp(&tuning.maximum_collision_normal_y),
        Some(Ordering::Less)
    ) {
        return result;
    }

    let collision_position = if input.collision_time >= 0.0 {
        trajectory.position_at_time(input.collision_time)
    } else {
        [0.0; 3]
    };
    result.probe_start = add3(
        collision_position,
        scale3(input.collision_normal, tuning.test_distance),
    );
    result.probe_end = add3(result.probe_start, [0.0, -10.0, 0.0]);
    let Some(ground_hit) = line_test(result.probe_start, result.probe_end) else {
        result.decision = TrajectoryWallRideDecision::NoGroundBelow;
        return result;
    };
    if !matches!(
        tuning
            .maximum_ground_wall_normal_dot
            .partial_cmp(&dot3(ground_hit.normal, input.collision_normal)),
        Some(Ordering::Greater)
    ) {
        result.decision = TrajectoryWallRideDecision::GroundNormalRejected;
        return result;
    }

    result.height_from_ground = (ground_hit.position[1] - result.probe_start[1]).abs();
    let weighted_height_score = result.height_from_ground * tuning.score_per_height;
    if input.near_grind {
        result.decision = TrajectoryWallRideDecision::PenalizedNearGrind;
        result.score = weighted_height_score - 1000.0;
        return result;
    }

    let launch_to_collision = subtract3(trajectory.initial_position, collision_position);
    let horizontal_distance =
        length_squared3([launch_to_collision[0], 0.0, launch_to_collision[2]]).sqrt();
    result.landing_slope = if horizontal_distance > 0.01 {
        launch_to_collision[1] / horizontal_distance
    } else {
        launch_to_collision[1]
    };
    if !matches!(
        result
            .landing_slope
            .partial_cmp(&tuning.maximum_landing_slope),
        Some(Ordering::Less | Ordering::Equal)
    ) {
        result.decision = TrajectoryWallRideDecision::PenalizedApproach;
        result.score = weighted_height_score - 1000.0;
        return result;
    }
    if !matches!(
        result
            .height_from_ground
            .partial_cmp(&tuning.minimum_height_from_ground),
        Some(Ordering::Greater | Ordering::Equal)
    ) {
        result.decision = TrajectoryWallRideDecision::PenalizedTooLow;
        result.score = result.height_from_ground - 1000.0;
        return result;
    }

    result.decision = TrajectoryWallRideDecision::Adjusted;
    result.score = weighted_height_score;
    result.trajectory_adjustment = [
        0.0,
        tuning.boost_vs_height.evaluate(result.height_from_ground),
        0.0,
    ];
    if collision_position[1] > result.highest_wall_ride_collision_y {
        result.highest_wall_ride_collision_y = collision_position[1];
    }
    trajectory.adjust(
        input.collision_frame,
        result.trajectory_adjustment,
        trajectory_tuning.maximum_trajectory_adjustment,
    );
    result
}

/// Reconstructs the orchestration in `TrajectorySelector::CalculateGrindScore`
/// at `0x82dc5c38` around the still-separate grind spatial index and trajectory
/// adjustment algorithms.
///
/// The query callback returns `None` for an empty box test or its native result
/// payload otherwise. Only a nonempty result accepted by the analysis callback
/// earns `ScoreGrind`.
pub fn calculate_trajectory_grind_score<R, Q, A>(
    trajectory: &BallisticTrajectory,
    collision_time: f32,
    takeoff_position: [f32; 3],
    tuning: &TrajectorySelectionTuning,
    query_nearby_grinds: Q,
    analyze_and_adjust: A,
) -> TrajectoryGrindScore
where
    Q: FnOnce(GrindTrajectoryQuery) -> Option<R>,
    A: FnOnce(R) -> bool,
{
    debug_assert!(collision_time >= 0.0, "grind score requires a collision");
    let collision_position = trajectory.position_at_time(collision_time);
    let radius = [tuning.grind_detection_radius; 3];
    let query = GrindTrajectoryQuery {
        minimum: subtract3(collision_position, radius),
        maximum: add3(collision_position, radius),
        takeoff_position,
    };
    let Some(results) = query_nearby_grinds(query) else {
        return TrajectoryGrindScore {
            query,
            ..TrajectoryGrindScore::default()
        };
    };
    let grind_bonus_awarded = analyze_and_adjust(results);

    TrajectoryGrindScore {
        query,
        grind_test_results_valid: true,
        grind_bonus_awarded,
        score: if grind_bonus_awarded {
            tuning.score_grind
        } else {
            0.0
        },
    }
}

/// Reconstructs `TrajectorySelector::CalculateGrindPenalty` at `0x82db9ee0`.
///
/// A valid query measures the collision position against every returned line
/// segment and caps the distance at 1000. A prediction without a collision
/// (negative time) uses the zero vector, matching the reference's SIMD
/// selection. An invalid query also evaluates the curve at 1000.
#[must_use]
pub fn calculate_trajectory_grind_penalty(
    grind_query_valid: bool,
    grind_segments: &[GrindTestSegment],
    collision_time: f32,
    collision_position: [f32; 3],
    tuning: &TrajectorySelectionTuning,
) -> TrajectoryGrindPenalty {
    let distance_to_grind = if grind_query_valid {
        let point = if collision_time >= 0.0 {
            collision_position
        } else {
            [0.0; 3]
        };
        let minimum_squared_distance =
            grind_segments
                .iter()
                .fold(1_000_000.0_f32, |best, segment| {
                    let start_offset = subtract3(point, segment.start);
                    let end_offset = subtract3(point, segment.end);
                    let direction = normalize_safe3(subtract3(segment.end, segment.start));
                    let projection = dot3(direction, start_offset);
                    let perpendicular = subtract3(start_offset, scale3(direction, projection));
                    length_squared3(perpendicular)
                        .min(length_squared3(start_offset))
                        .min(length_squared3(end_offset))
                        .min(best)
                });
        minimum_squared_distance.sqrt()
    } else {
        1000.0
    };
    let score = tuning
        .grind_penalty_vs_distance_to_grind
        .evaluate(distance_to_grind);
    debug_assert!(
        score <= 0.0,
        "grind penalty curve returned a positive score"
    );

    TrajectoryGrindPenalty {
        distance_to_grind,
        score,
    }
}

/// Reconstructs `TrajectorySelector::CalculateSurfaceScore` at `0x82db9dd8`.
///
/// The Pegasus surface word stores the five-bit physics classification in
/// bits 7..11. Unrideable and do-not-align surfaces force the collision normal
/// to the processed physics up vector; stair surfaces receive the same score
/// as do-not-align but intentionally retain their actual collision normal.
#[must_use]
pub fn calculate_trajectory_surface_score(
    packed_surface: u32,
    collision_normal: [f32; 3],
    up_vector: [f32; 3],
    tuning: &TrajectorySelectionTuning,
) -> TrajectorySurfaceScore {
    let raw_surface = u8::try_from((packed_surface >> 7) & 0x1f).unwrap_or_default();
    let physics_surface = PhysicsSurface::from_raw(raw_surface);
    let (score, collision_normal) = match physics_surface {
        Some(PhysicsSurface::Unrideable) => (tuning.score_surface_unrideable, up_vector),
        Some(PhysicsSurface::DoNotAlign) => (tuning.score_surface_do_not_align, up_vector),
        Some(PhysicsSurface::Stair) => (tuning.score_surface_do_not_align, collision_normal),
        _ => (0.0, collision_normal),
    };

    TrajectorySurfaceScore {
        physics_surface,
        score,
        collision_normal,
    }
}

/// Reconstructs the selector's proven score composition around the separate
/// grind, surface, and vert-transition collaborators.
///
/// Reference: `TrajectorySelector::CalculateTrajectoryScores` at
/// `0x82dc5e00`.
#[must_use]
pub fn calculate_trajectory_score(
    input: TrajectoryScoreInput,
    tuning: &TrajectorySelectionTuning,
) -> TrajectoryScore {
    if input.collision_time < 0.0 {
        return TrajectoryScore::default();
    }

    let dot_velocity_normal = dot3(input.landing_normal, input.landing_velocity);
    let landing_force_scalar = tuning
        .landing_force_scalar_vs_dot_velocity_normal
        .evaluate(dot_velocity_normal.abs());
    let landing_force = dot_velocity_normal * tuning.score_landing_force * landing_force_scalar;
    let landing_direction = length_squared3(cross3(
        input.landing_normal,
        normalize_safe3(input.landing_velocity),
    ))
    .sqrt()
        * tuning.score_landing_direction
        * landing_force_scalar;
    let middle_bonus = if input.select_central_trajectory && input.candidate_index == 0 {
        1_000_000.0
    } else if input.candidate_index == 0 && input.pass == 1 && !input.on_vert {
        tuning.score_middle_bonus
    } else {
        0.0
    };
    let time_bonus = tuning.landing_time_bonus.evaluate(input.collision_time);

    // Preserve the reference's scalar addition order at 0x82dc64c8.
    let mut total = landing_direction + 1.0;
    total += landing_force;
    total += middle_bonus;
    total += input.grind_or_penalty_score;
    total += input.transition_score;
    total += time_bonus;
    total += input.surface_score;

    TrajectoryScore {
        valid_collision: true,
        time_bonus,
        landing_force,
        landing_direction,
        middle_bonus,
        grind_or_penalty: input.grind_or_penalty_score,
        surface: input.surface_score,
        transition: input.transition_score,
        total,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SkateboardJumpInfo {
    pub new_velocity: [f32; 3],
    pub centre_of_mass_height: f32,
    pub do_jump: bool,
}

/// One force and its local application point on the skateboard.
///
/// Reference: PDB type `Skateboard::SkateboardForce` is 32 bytes: the 16-byte
/// `mWorldSpaceForce` at `+0x0` and `mLocalSpacePos` at `+0x10`, as consumed by
/// `Skateboard::AddSkateboardForce` at `0x82b38ce0`. This native semantic type
/// intentionally stores only each vector's three meaningful components and
/// does not reproduce the console vector lane/padding.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SkateboardForce {
    pub world_space_force: [f32; 3],
    pub local_space_position: [f32; 3],
}

/// Force kinds emitted by the non-jump branch of ground animation.
///
/// The discriminants are the exact `eSkateboardForceType` values loaded at
/// `0x82d8a96c..0x82d8a9bc`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum SkateboardForceKind {
    SlideFriction = 1,
    Brake = 2,
    FrontFoot = 4,
    RearFoot = 5,
    CollisionResponse = 15,
}

/// A typed entry in the skateboard's per-update force queue.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppliedSkateboardForce {
    pub kind: SkateboardForceKind,
    pub force: SkateboardForce,
}

/// Native capacity of `Skateboard::mSkateBoardForcesAdded`.
pub const MAX_APPLIED_SKATEBOARD_FORCES: usize = 21;

/// Capped force queue used by `Skateboard::AddSkateboardForce`.
///
/// Reference: `0x82b38cec..0x82b38d08` silently skips each insertion whose
/// zero-based queue length is already at least 21. Earlier insertions in the
/// same update remain present when a later insertion reaches the cap.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkateboardForceQueue {
    forces: Vec<AppliedSkateboardForce>,
}

impl SkateboardForceQueue {
    #[must_use]
    pub fn forces(&self) -> &[AppliedSkateboardForce] {
        &self.forces
    }

    /// Reference: `Skateboard::AddSkateboardForce` at `0x82b38ce0`.
    pub fn add(&mut self, kind: SkateboardForceKind, force: SkateboardForce) {
        if self.forces.len() < MAX_APPLIED_SKATEBOARD_FORCES {
            self.forces.push(AppliedSkateboardForce { kind, force });
        }
    }
}

/// Unrecovered force calculations used by the ground-animation wrapper.
///
/// This boundary deliberately exposes the reference calls without supplying
/// plausible force math. `calculate_collision_force` models the collision
/// force output and boolean result used by this caller. The reference also
/// passes a separate writable torque output, then discards it, and passes a
/// null new-velocity output.
pub trait GroundAnimationSkateboardForces {
    /// Reference: `Skateboard::Toolkit_CalcFrontFootForce` at `0x82b28b70`.
    /// Its PDB parameters are named `timeSinceLanding` and `absorption`.
    fn calculate_front_foot_force(
        &mut self,
        time_since_landing: f32,
        absorption: f32,
    ) -> SkateboardForce;

    /// Reference: `Skateboard::Toolkit_CalcRearFootForce` at `0x82b28d98`.
    /// Its PDB parameters are named `timeSinceLanding` and `absorption`.
    fn calculate_rear_foot_force(
        &mut self,
        time_since_landing: f32,
        absorption: f32,
    ) -> SkateboardForce;

    /// Reference: `Skateboard::Toolkit_CalcBraking` at `0x82b26df0`.
    fn calculate_braking_force(&mut self) -> SkateboardForce;

    /// Reference: `Skateboard::ToolKit_CalcSlideFriction` at `0x82b28260`.
    fn calculate_slide_friction_force(&mut self, state_timer: f32) -> SkateboardForce;

    /// Reference: `Skateboard::Toolkit_CalcLinearDrag` at `0x82b27078`.
    fn calculate_linear_drag(&mut self) -> f32;

    /// Reference: `Skateboard::Toolkit_CalcCollisionForce` at `0x82b27530`.
    ///
    /// This caller supplies writable force and torque outputs but does not
    /// initialize or subsequently read the torque output. Its new-velocity
    /// output is always null, so it is intentionally absent from this typed
    /// boundary. The PDB torque type is the 16-byte
    /// `rw::math::vpu::Vector3`; `MaybeUninit<[f32; 3]>` preserves its three
    /// semantic components and uninitialized-input property without console
    /// ABI lane/padding.
    fn calculate_collision_force(
        &mut self,
        collision_force: &mut SkateboardForce,
        discarded_torque: &mut std::mem::MaybeUninit<[f32; 3]>,
    ) -> bool;

    /// Reference: the `SetLinearDrag` call at `0x82d8a9c4`.
    fn set_linear_drag(&mut self, linear_drag: f32);
}

/// Runs the non-jump force branch of
/// `PhysState_GroundAnimation::UpdateSkateboard`.
///
/// Reference: `0x82d8a8a0..0x82d8a9cc`. All calculations intentionally run
/// before the collision result is inspected. A collision queues type 15
/// instead of attempting the four ordinary additions and suppresses the
/// linear-drag write; entries already in the queue remain untouched. The jump
/// path returns before this sequence in the reference and remains a separate
/// API.
pub fn update_ground_animation_non_jump<B: GroundAnimationSkateboardForces>(
    skateboard: &mut B,
    state_timer: f32,
    force_queue: &mut SkateboardForceQueue,
) {
    let front_foot = skateboard.calculate_front_foot_force(1.0, 0.0);
    let rear_foot = skateboard.calculate_rear_foot_force(1.0, 0.0);
    let braking = skateboard.calculate_braking_force();
    let slide_friction = skateboard.calculate_slide_friction_force(state_timer);
    let linear_drag = skateboard.calculate_linear_drag();
    let mut collision_response = SkateboardForce::default();
    let mut discarded_torque = std::mem::MaybeUninit::uninit();
    let collided =
        skateboard.calculate_collision_force(&mut collision_response, &mut discarded_torque);

    if collided {
        force_queue.add(SkateboardForceKind::CollisionResponse, collision_response);
        return;
    }

    force_queue.add(SkateboardForceKind::FrontFoot, front_foot);
    force_queue.add(SkateboardForceKind::RearFoot, rear_foot);
    force_queue.add(SkateboardForceKind::SlideFriction, slide_friction);
    force_queue.add(SkateboardForceKind::Brake, braking);
    skateboard.set_linear_drag(linear_drag);
}

/// Unrecovered skateboard operations orchestrated by the physics-air state.
///
/// This boundary preserves the recovered caller without inventing steering,
/// collision, or jump-correction math. As in the ground-animation boundary,
/// the discarded collision torque retains only the three semantic components
/// of the PDB's 16-byte `rw::math::vpu::Vector3`; console ABI lane/padding is
/// intentionally absent.
pub trait PhysicsAirSkateboard {
    /// Reference: `Skateboard::UpdateSteeringTilt` at `0x82b29de0`.
    fn update_steering_tilt(&mut self, tilt: f32);

    /// Reference: `Skateboard::Toolkit_CalcCollisionForce` at `0x82b27530`.
    ///
    /// The caller supplies zeroed force storage, a separate uninitialized
    /// writable torque output, and a null new-velocity output. The unused null
    /// output is intentionally absent from this typed boundary.
    fn calculate_collision_force(
        &mut self,
        collision_force: &mut SkateboardForce,
        discarded_torque: &mut std::mem::MaybeUninit<[f32; 3]>,
    ) -> bool;

    /// Reference: `PhysState_PhysicsAir::CalcVelFromJump(int)` at
    /// `0x82d8af08`.
    fn calculate_velocity_from_jump(&mut self, frames_since_jump: i32) -> [f32; 3];

    /// Reference: `Skateboard::SetVelocity` at `0x82b29ee0`.
    fn set_velocity(&mut self, velocity: [f32; 3]);
}

/// Runs the skateboard portion of `PhysState_PhysicsAir::Update`.
///
/// Reference: `PhysState_PhysicsAir::UpdateSkateboard` at
/// `0x82d990b8`, specifically `0x82d990dc..0x82d9919c`. Steering is always
/// updated with positive zero before collision is calculated. A collision
/// queues force type 15 and suppresses jump correction even if the capped
/// queue silently rejects that insertion. On a miss, jump velocity is
/// recalculated only for the strict signed comparison `frames < limit`, and
/// the resulting velocity is applied immediately.
pub fn update_physics_air_skateboard<B: PhysicsAirSkateboard>(
    skateboard: &mut B,
    frames_since_jump_correct: i32,
    jump_fix_frames: i32,
    force_queue: &mut SkateboardForceQueue,
) {
    skateboard.update_steering_tilt(0.0);

    let mut collision_response = SkateboardForce::default();
    let mut discarded_torque = std::mem::MaybeUninit::uninit();
    if skateboard.calculate_collision_force(&mut collision_response, &mut discarded_torque) {
        force_queue.add(SkateboardForceKind::CollisionResponse, collision_response);
        return;
    }

    if frames_since_jump_correct < jump_fix_frames {
        let velocity = skateboard.calculate_velocity_from_jump(frames_since_jump_correct);
        skateboard.set_velocity(velocity);
    }
}

/// Exact bit pattern of the fixed step used to advance the physics-air COM
/// trajectory at `0x82d9f734..0x82d9f79c`.
pub const PHYSICS_AIR_FIXED_STEP_SECONDS: f32 = f32::from_bits(0x3c88_8889);

/// Build-scoped `physics_airstates.default` value read through key
/// `c052d34b6b052f44` at `0x82d9f580..0x82d9f600`.
pub const DEBUG_REFERENCE_PHYSICS_AIR_NORMAL_ANGLE_LIMIT: f32 = f32::from_bits(0x3fce_147b);

/// Build-scoped `physics_airstates.default` value read through key
/// `724c206c94a73043` at `0x82d9f60c..0x82d9f63c`.
pub const DEBUG_REFERENCE_PHYSICS_AIR_NORMAL_BLEND: f32 = f32::from_bits(0x3dcc_cccd);

/// Build-scoped `physics_airstates.default` byte read through key
/// `fd9ce5e1f36ccf2e` at `0x82d9f6d4..0x82d9f704`.
pub const DEBUG_REFERENCE_PHYSICS_AIR_COM_CORRECTION_ENABLED: bool = false;

/// Persistent source-level fields used by `PhysState_PhysicsAir::Update`.
///
/// PDB type `Sk8::Physics::PhysState_PhysicsAir` is `0xc0` bytes. These fields
/// are `mLandingNormal` at `+0x90`, `mTimeInState` at `+0xa0`, `mMaxY` at
/// `+0xa8`, `mUseCentreOfMassVel` at `+0xad`, and `mTrajectoryTimer` at
/// `+0xb0`. Console padding and base-class pointers are deliberately absent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsAirUpdateState {
    pub landing_normal: PhysicsAirVector3,
    pub time_in_state: f32,
    pub max_y: f32,
    pub use_centre_of_mass_velocity: bool,
    pub trajectory_timer: i32,
}

/// Unrecovered operations orchestrated by the outer physics-air update.
///
/// This keeps normalized-angle VMX work, trajectory launch internals, COM
/// correction, ballistic COM stepping, skeleton transforms, and skateboard
/// force math outside the recovered control-flow wrapper. Methods which read
/// live processed input or tuning remain methods rather than a precomputed
/// input record so the wrapper preserves the reference's conditional and late
/// reads.
pub trait PhysicsAirUpdateCollaborator {
    /// `SkeletonController::UpdateAirCollision` at `0x82d9f4cc`.
    fn update_air_collision(&mut self);

    /// PDB field `ProcessedPhysIn::mIsSkeletonInContact`, mask `0x00008000`.
    fn is_skeleton_in_contact(&mut self) -> bool;

    /// PDB field `ProcessedPhysIn::mPhysicsState` at `+0x8e4`.
    fn physics_state_is_nonzero(&mut self) -> bool;

    /// PDB field `TrajectorySelector::mJustStartedQuery` at `+0x2392`.
    fn trajectory_query_just_started(&mut self) -> bool;

    /// `PhysState_PhysicsAir::LaunchTrajectorySelector` at `0x82d9f514`.
    /// The native callee owns its omitted launch-record and COM-trajectory
    /// state and writes none of the reduced state represented here.
    fn launch_trajectory_selector(&mut self);

    /// `TrajectorySelector::Update` at `0x82d9f524`.
    fn update_trajectory_selector(&mut self);

    /// Snapshots `Reckoning::mUpVector` (`+0x1b0`) at `0x82d9f528..0x82d9f544`.
    fn reckoning_up_vector(&mut self) -> PhysicsAirVector3;

    /// `TrajectorySelector::GetSuggestedLandingNormal` at `0x82d9f548`.
    fn suggested_landing_normal(&mut self, fallback: PhysicsAirVector3) -> PhysicsAirVector3;

    /// `Math::AngleBetweenVectors` plus the inline normalization at
    /// `0x82d9f568..0x82d9f5d4`. Its VMX rounding remains unrecovered.
    fn normalized_angle_between(
        &mut self,
        landing_normal: PhysicsAirVector3,
        fallback_up: PhysicsAirVector3,
    ) -> f32;

    /// Live `physics_airstates.default` lookup by key `c052d34b6b052f44`.
    fn landing_normal_angle_limit(&mut self) -> f32;

    /// Live `physics_airstates.default` lookup by key `724c206c94a73043`.
    fn landing_normal_blend(&mut self) -> f32;

    /// PDB field `ProcessedPhysIn::mStateTimer` at `+0x974`.
    fn state_timer(&mut self) -> f32;

    /// PDB field `ProcessedPhysIn::mIsSkateboardInContact`, mask `0x00004000`.
    fn is_skateboard_in_contact(&mut self) -> bool;

    /// Reads `kbDrawPhysicsAirDebugInfo` at `0x82d9f670..0x82d9f684`.
    fn draw_physics_air_debug_info(&mut self) -> bool;

    /// Reads the debug-overlay singleton only when the debug cVar is enabled.
    fn debug_overlay_is_available(&mut self) -> bool;

    /// Represents the debug-build invariant failure at `0x82d9f698..0x82d9f6ac`.
    fn report_missing_debug_overlay(&mut self);

    /// `PhysState_PhysicsAir::CalculateBodySpinSpeed` at `0x82d9f6b8`.
    fn calculate_body_spin_speed(&mut self) -> f32;

    /// `Reckoning::UpdateAirStates` at `0x82d9f6d0`.
    fn update_air_states(
        &mut self,
        landing_normal: PhysicsAirVector3,
        normal_blend: f32,
        target_body_spin_speed: f32,
        target_body_flip_speed: f32,
    );

    /// Live `physics_airstates.default` byte lookup by key
    /// `fd9ce5e1f36ccf2e` at `0x82d9f6d4..0x82d9f704`.
    fn add_centre_of_mass_correction_enabled(&mut self) -> bool;

    /// `PhysState_PhysicsAir::AddCOMCorrection` at `0x82d9f714`.
    ///
    /// The caller reads `mUseCentreOfMassVel` only after this call returns,
    /// but the exact callee writes none of the reduced state represented here.
    fn add_centre_of_mass_correction(&mut self);

    /// Advances the state-owned `tTrajectory` and returns its new position.
    /// The VMX multiply/add order at `0x82d9f724..0x82d9f7a0` remains opaque.
    fn advance_centre_of_mass_trajectory(&mut self, fixed_step: f32) -> PhysicsAirVector3;

    /// `Skeleton::UpdateKnownAir` at `0x82d9f7b4`.
    fn update_known_air(&mut self, centre_of_mass_position: PhysicsAirVector3);

    /// `Skeleton::UpdateAnimatedSkateboard` at `0x82d9f7c8`.
    fn update_animated_skateboard(&mut self, use_fast_blend: bool);

    /// Delegates to the already-separated
    /// `PhysState_PhysicsAir::UpdateSkateboard` wrapper at `0x82d9f7d0`.
    fn update_skateboard(&mut self);

    /// Writes `Skeleton::mAddSkateboardErrorToSkeleton` (`+0x3dd4`).
    fn set_add_skateboard_error_to_skeleton(&mut self, enabled: bool);

    /// Calls `Skateboard::GetTransform` and returns `wAxis.y`.
    fn skateboard_transform_y(&mut self) -> f32;

    /// Late read of `ProcessedPhysIn::mTimeDelta` (`+0x93c`).
    fn time_delta(&mut self) -> f32;
}

/// Runs the recovered outer orchestration of `PhysState_PhysicsAir::Update`.
///
/// Reference: `0x82d9f4b0..0x82d9f824`. The wrapper preserves the native
/// short-circuit reads, call order, state-write order, and unordered floating
/// comparisons without implementing the still-opaque vector/transform math.
pub fn update_physics_air<C: PhysicsAirUpdateCollaborator>(
    state: &mut PhysicsAirUpdateState,
    collaborator: &mut C,
) {
    collaborator.update_air_collision();

    // addic./bgt at 0x82d9f4d4: title code runs in 32-bit mode, so CR0 compares
    // the low word as signed. A blocked relaunch retries every frame while <= 0.
    state.trajectory_timer = state.trajectory_timer.wrapping_sub(1);
    if state.trajectory_timer <= 0
        && !collaborator.is_skeleton_in_contact()
        && collaborator.physics_state_is_nonzero()
        && !collaborator.trajectory_query_just_started()
    {
        collaborator.launch_trajectory_selector();
        state.trajectory_timer = 7;
    }

    collaborator.update_trajectory_selector();

    let fallback_up = collaborator.reckoning_up_vector();
    let landing_normal = collaborator.suggested_landing_normal(fallback_up);
    state.landing_normal = landing_normal;
    let normalized_angle = collaborator.normalized_angle_between(landing_normal, fallback_up);
    let angle_limit = collaborator.landing_normal_angle_limit();

    // PPC `fcmpu` followed by `ble` takes the lookup path for unordered
    // comparisons too. Express that as "not ordered-greater" rather than `<=`.
    let mut normal_blend = if normalized_angle > angle_limit {
        0.0
    } else {
        collaborator.landing_normal_blend()
    };

    let state_timer = collaborator.state_timer();
    // The second `fcmpu`/`ble` likewise sends NaN, -0, and +0 to zero blend.
    if !matches!(state_timer.partial_cmp(&0.0), Some(Ordering::Greater)) {
        normal_blend = 0.0;
    } else if collaborator.is_skateboard_in_contact() {
        state.use_centre_of_mass_velocity = false;
    }

    if collaborator.draw_physics_air_debug_info() && !collaborator.debug_overlay_is_available() {
        collaborator.report_missing_debug_overlay();
    }

    let target_body_spin_speed = collaborator.calculate_body_spin_speed();
    collaborator.update_air_states(landing_normal, normal_blend, target_body_spin_speed, 0.0);

    if collaborator.add_centre_of_mass_correction_enabled() {
        collaborator.add_centre_of_mass_correction();
    }

    if state.use_centre_of_mass_velocity {
        let centre_of_mass_position =
            collaborator.advance_centre_of_mass_trajectory(PHYSICS_AIR_FIXED_STEP_SECONDS);
        collaborator.update_known_air(centre_of_mass_position);
    } else {
        collaborator.update_animated_skateboard(false);
    }

    collaborator.update_skateboard();
    collaborator.set_add_skateboard_error_to_skeleton(true);

    let skateboard_y = collaborator.skateboard_transform_y();
    // Native `fcmpu old,new; ble` selects the newly sampled Y on a tie or an
    // unordered comparison. This also preserves the new operand's zero sign.
    let selected_max_y = if state.max_y > skateboard_y {
        state.max_y
    } else {
        skateboard_y
    };
    let old_time_in_state = state.time_in_state;
    state.max_y = selected_max_y;
    let time_delta = collaborator.time_delta();
    state.time_in_state = time_delta + old_time_in_state;
}

/// Persistent fields owned by `PhysState_GroundAnimation` around its board
/// update. This intentionally omits the base-state pointers from the console
/// class and models only gameplay state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GroundAnimationState {
    pub jump_info: SkateboardJumpInfo,
    pub jumped_this_frame: bool,
    pub jump_velocity: [f32; 3],
}

impl GroundAnimationState {
    /// Reference: the first write in `PhysState_GroundAnimation::Update` at
    /// `0x82d98d7c`.
    pub fn begin_update(&mut self) {
        self.jumped_this_frame = false;
    }

    /// Captures the trajectory selector's possibly revised launch velocity
    /// after its synchronous `Launch`/`Update` pair.
    pub fn finish_trajectory_update(&mut self, selector_launch: &TrajectoryLaunchInfo) {
        self.jump_velocity = selector_launch.start_velocity.xyz();
    }
}

/// Side effects requested by the successful jump branch of
/// `PhysState_GroundAnimation::UpdateSkateboard`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundAnimationJumpHandoff {
    pub board_velocity: [f32; 3],
    pub linear_drag: f32,
    pub trajectory_launch: TrajectoryLaunchInfo,
}

/// Installs a calculated normal jump into the ground-animation state and
/// prepares the launch record passed to `TrajectorySelector::Launch`.
///
/// The skeleton supplies orientation, positions, and pre-jump motion first;
/// the state then replaces `startVel` with the computed board velocity and
/// marks `playerJumped`, exactly as the reference branch does.
pub fn prepare_ground_animation_jump(
    state: &mut GroundAnimationState,
    jump: &GroundJumpSolution,
    mut skeleton_launch: TrajectoryLaunchInfo,
) -> Option<GroundAnimationJumpHandoff> {
    state.jump_info = SkateboardJumpInfo {
        new_velocity: jump.new_velocity,
        centre_of_mass_height: jump.centre_of_mass_height,
        do_jump: jump.do_jump,
    };
    if !jump.do_jump {
        return None;
    }

    state.jumped_this_frame = true;
    // The existing grounded-jump solver recovers only XYZ. Positive zero in
    // lane four is an explicit compatibility boundary, not a parity claim.
    skeleton_launch.start_velocity =
        PhysicsAirVector3::from_xyz_with_positive_zero_w(jump.new_velocity);
    skeleton_launch.player_jumped = true;
    Some(GroundAnimationJumpHandoff {
        board_velocity: jump.new_velocity,
        linear_drag: 0.0,
        trajectory_launch: skeleton_launch,
    })
}

/// Reconciles animation-prepared and current velocity along the riding
/// surface.
///
/// Reference: `Skateboard::Toolkit_CalcGroundJump` at `0x82b316a8` through
/// `0x82b31918`. Both velocities are rejected from the jump normal. The
/// prepared tangent is used unless its squared difference from the current
/// tangent exceeds `kMaxJumpError` by ordered comparison. The correction bound
/// is prepared-tangent length multiplied by the angle curve (0x82b3189c..31908).
/// This existing XYZ helper uses host square root/acos; it does not certify the
/// native estimate/refinement bits or runtime attribute sampling order.
#[must_use]
pub fn calculate_ground_jump_tangential_velocity(
    input: GroundJumpTangentialInput,
    jump_speed_scalar_vs_angle: &PointGraph<8>,
) -> GroundJumpTangentialSolution {
    let current_tangent = reject_from_normal(input.current_velocity, input.jump_normal);
    let prepared_tangent = reject_from_normal(input.prepared_jump_velocity, input.jump_normal);
    let difference = subtract3(current_tangent, prepared_tangent);
    if length_squared3(difference).partial_cmp(&input.maximum_velocity_error_squared)
        != Some(std::cmp::Ordering::Greater)
    {
        return GroundJumpTangentialSolution {
            current_tangent,
            prepared_tangent,
            selected_tangent: prepared_tangent,
            normalized_direction_angle: 0.0,
            corrected: false,
        };
    }

    let current_direction = normalize_safe3(current_tangent);
    let prepared_direction = normalize_safe3(prepared_tangent);
    let normalized_direction_angle = dot3(current_direction, prepared_direction)
        .clamp(-1.0, 1.0)
        .acos()
        * std::f32::consts::FRAC_2_PI;
    let speed_scalar = jump_speed_scalar_vs_angle.evaluate(normalized_direction_angle);
    let maximum_length = length_squared3(prepared_tangent).sqrt() * speed_scalar;
    GroundJumpTangentialSolution {
        current_tangent,
        prepared_tangent,
        selected_tangent: clamp_vector_length3(current_tangent, maximum_length),
        normalized_direction_angle,
        corrected: true,
    }
}

/// Calculates the height-derived velocity used by a normal grounded jump.
///
/// This is the source-level equivalent of the height and vertical-response
/// portion of `Skateboard::Toolkit_CalcGroundJump` at `0x82b31960` through
/// `0x82b31dfc`.
#[must_use]
pub fn calculate_ground_jump_vertical(
    input: GroundJumpVerticalInput,
    tuning: GroundJumpTuning,
    curves: &JumpPhysicsCurves,
) -> GroundJumpVerticalSolution {
    let normalized_surface_speed =
        (input.surface_speed / tuning.height_curve_reference_speed).clamp(0.0, 1.0);
    let minimum_height = lerp(
        tuning.low_speed_jump_height,
        tuning.minimum_jump_height,
        curves
            .min_height_vs_speed
            .evaluate(normalized_surface_speed),
    );
    let maximum_height = lerp(
        tuning.low_speed_jump_height,
        tuning.maximum_jump_height,
        curves
            .max_height_vs_speed
            .evaluate(normalized_surface_speed),
    );
    let target_height = lerp(minimum_height, maximum_height, input.jump_strength);
    let centre_from_ground = subtract3(input.centre_of_mass, input.ground_position);
    let current_height = dot3(centre_from_ground, input.reckoning_up);
    let remaining_height = (target_height - current_height).max(0.0);
    let ballistic_speed = (-2.0 * input.gravity * remaining_height).sqrt();

    let curve_response = if tuning.use_vertical_response_curve {
        let normalized_ground_angle = (input.jump_normal[1].clamp(-1.0, 1.0).acos()
            * std::f32::consts::FRAC_2_PI)
            .clamp(0.0, 1.0);
        curves.vertical_response.evaluate(normalized_ground_angle)
    } else {
        1.0
    };
    let vertical_response = curve_response.max(tuning.minimum_vertical_response);
    GroundJumpVerticalSolution {
        normalized_surface_speed,
        minimum_height,
        maximum_height,
        target_height,
        current_height,
        vertical_response,
        launch_speed: ballistic_speed * vertical_response,
    }
}

/// Calculates the complete normal grounded-jump velocity in domain terms.
///
/// Reference: `Skateboard::Toolkit_CalcGroundJump` at `0x82b315a8`. After the
/// independently recovered tangent and height stages, the game boosts world Y
/// from `JumpYScalarVsGroundNormalY`, builds board-right as
/// `normalize(world_up x body_z)`, reconstructs board-forward as
/// `body_right x world_up`, and applies animation X/Z adjustment in that
/// frame. The returned delta is the value written to `mJumpDV`.
#[must_use]
pub fn calculate_ground_jump(
    input: GroundJumpInput,
    tuning: GroundJumpTuning,
    curves: &JumpPhysicsCurves,
) -> GroundJumpSolution {
    let tangential = calculate_ground_jump_tangential_velocity(
        GroundJumpTangentialInput {
            current_velocity: input.current_velocity,
            prepared_jump_velocity: input.prepared_jump_velocity,
            jump_normal: input.vertical.jump_normal,
            maximum_velocity_error_squared: input.maximum_velocity_error_squared,
        },
        &curves.jump_speed_scalar_vs_angle,
    );
    let vertical = calculate_ground_jump_vertical(input.vertical, tuning, curves);
    let launch_velocity = add3(
        tangential.selected_tangent,
        scale3(input.vertical.jump_normal, vertical.launch_speed),
    );

    let jump_y_scalar = curves
        .jump_y_scalar_vs_ground_normal_y
        .evaluate(input.vertical.jump_normal[1]);
    let vertical_compensation = (launch_velocity[1] * (jump_y_scalar - 1.0))
        .clamp(0.0, tuning.maximum_horizontal_compensation);
    let mut compensated_velocity = launch_velocity;
    compensated_velocity[1] += vertical_compensation;
    let velocity_along_jump_normal = dot3(compensated_velocity, input.vertical.jump_normal);

    let body_right_axis = normalize_safe3(cross3(WORLD_UP, input.body_z_axis));
    let body_forward_axis = cross3(body_right_axis, WORLD_UP);
    let sideways_adjustment = scale3(
        body_right_axis,
        -input.jump_adjustment_x * tuning.sideways_adjustment_scale * velocity_along_jump_normal,
    );
    let forward_adjustment = scale3(
        body_forward_axis,
        input.jump_adjustment_z * tuning.forward_adjustment_scale * velocity_along_jump_normal,
    );
    let new_velocity = add3(
        compensated_velocity,
        add3(sideways_adjustment, forward_adjustment),
    );

    GroundJumpSolution {
        vertical,
        tangential,
        launch_velocity,
        vertical_compensation,
        compensated_velocity,
        velocity_along_jump_normal,
        body_right_axis,
        body_forward_axis,
        new_velocity,
        delta_velocity: subtract3(new_velocity, input.current_velocity),
        centre_of_mass_height: 0.0,
        do_jump: true,
    }
}

fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    (end - start).mul_add(amount, start)
}

fn subtract3(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn add3(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn dot3(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

fn cross3(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn reject_from_normal(vector: [f32; 3], normal: [f32; 3]) -> [f32; 3] {
    let projection = dot3(vector, normal);
    subtract3(vector, scale3(normal, projection))
}

fn scale3(vector: [f32; 3], scalar: f32) -> [f32; 3] {
    [vector[0] * scalar, vector[1] * scalar, vector[2] * scalar]
}

fn length_squared3(vector: [f32; 3]) -> f32 {
    dot3(vector, vector)
}

fn normalize_safe3(vector: [f32; 3]) -> [f32; 3] {
    let length_squared = length_squared3(vector);
    if length_squared == 0.0 {
        [0.0; 3]
    } else {
        scale3(vector, length_squared.sqrt().recip())
    }
}

fn angle_between3(left: [f32; 3], right: [f32; 3]) -> f32 {
    let left = normalize_safe3(left);
    let right = normalize_safe3(right);
    dot3(left, right).clamp(-1.0, 1.0).acos()
}

fn clamp_vector_length3(vector: [f32; 3], maximum_length: f32) -> [f32; 3] {
    debug_assert!(maximum_length >= 0.0);
    let length = length_squared3(vector).sqrt();
    if length < 1.0 / 65_536.0 {
        vector
    } else {
        scale3(vector, length.min(maximum_length) / length)
    }
}

/// Global animation/physics settings read from the game's attribute database.
/// The three jump fields are the exact inputs used after animation attributes
/// have supplied `MinJump`, `TrickHeight`, and the lowercase `jump` marker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimationPhysicsTuning {
    pub use_jump_strength_endpoints: bool,
    pub jump_strength_lower_endpoint: f32,
    pub jump_strength_upper_endpoint: f32,
}

impl Default for AnimationPhysicsTuning {
    fn default() -> Self {
        Self {
            use_jump_strength_endpoints: false,
            jump_strength_lower_endpoint: 0.0,
            jump_strength_upper_endpoint: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimationTrickData {
    pub name: Option<String>,
    pub gesture: Option<String>,
    pub flags: BTreeSet<TrickFlag>,
    pub tweak_x: f32,
    pub tweak_y: f32,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TrickFlag {
    Switching,
    Landing,
    RideIdle,
    Wipeout,
    Trick,
    Grabbing,
    TailManual,
    NoseManual,
    BacksideRevert,
    FrontsideRevert,
    BacksidePowerslide,
    FrontsidePowerslide,
}

impl AnimationTrickData {
    #[must_use]
    pub fn named_trick(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            flags: BTreeSet::from([TrickFlag::Trick]),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// One-frame outputs owned by the motion graph before `Actor::SetUpPhysics`
/// transfers them into the physics input and clears the source.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MotionGraphOutput {
    pub trick: AnimationTrickData,
    pub scalar_attributes: Vec<ScalarAnimationAttribute>,
    pub contact_attributes: Vec<ContactAnimationAttribute>,
}

impl MotionGraphOutput {
    pub fn drain_into(&mut self, input: &mut AnimationPhysicsInput) {
        input.trick = std::mem::take(&mut self.trick);
        input.scalar_attributes = std::mem::take(&mut self.scalar_attributes);
        input.contact_attributes = std::mem::take(&mut self.contact_attributes);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimationPhysicsInput {
    pub global_pose: Vec<AnimationMatrix>,
    pub local_pose: Vec<AnimationMatrix>,
    pub time_delta_seconds: f32,
    pub compression: f32,
    pub flags: BTreeSet<AnimationPhysicsFlag>,
    pub trick: AnimationTrickData,
    pub scalar_attributes: Vec<ScalarAnimationAttribute>,
    pub contact_attributes: Vec<ContactAnimationAttribute>,
    pub trajectory_bone_index: Option<usize>,
    pub push_reference_bone_name: Option<String>,
    pub truck_tightness: f32,
    pub wheel_hardness: f32,
}

/// An ordered scalar attribute. The reference fixed vector preserves order and
/// permits duplicate names, so a map would lose observable behavior.
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarAnimationAttribute {
    pub name: String,
    pub value: f32,
}

impl ScalarAnimationAttribute {
    #[must_use]
    pub fn new(name: impl Into<String>, value: f32) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

/// Ordered type-3 animation attribute used for foot and hand contacts. The
/// bone name and transition scalar are decoded from the authored 24-byte
/// payload before entering the native physics layer.
#[derive(Clone, Debug, PartialEq)]
pub struct ContactAnimationAttribute {
    pub name: String,
    pub bone_name: String,
    pub value: f32,
}

impl ContactAnimationAttribute {
    #[must_use]
    pub fn new(name: impl Into<String>, bone_name: impl Into<String>, value: f32) -> Self {
        Self {
            name: name.into(),
            bone_name: bone_name.into(),
            value,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AnimationPhysicsFlag {
    BoardFlipped,
    NextStepValid,
    Mirrored,
    SwitchStance,
    Fakie,
    WeightForwards,
    RegularStance,
    RequestCollideWorldOnly,
    RunningOut,
    OffBoardJumping,
    RequestWipeout,
    AirDismountRevert,
    LocomotionBigAir,
    WasLocomotion,
    IsLocomotion,
}

/// Human-readable result of `Skeleton::ProcessAnimAttributes`. The original
/// packs these into fields and several bit words in `ProcessedPhysIn`; named
/// values keep the native implementation auditable.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProcessedAnimationIntent {
    pub spin: f32,
    pub body_spin: f32,
    pub foot_brake: f32,
    pub turn: f32,
    pub off_board_turn: f32,
    pub off_board_look_at_x: f32,
    pub off_board_look_at_y: f32,
    pub raw_turn: f32,
    pub hard_turn: f32,
    pub slide: f32,
    pub balance: f32,
    pub move_object_z: f32,
    pub move_object_x: f32,
    pub move_object_rotation: f32,
    pub grind_translation: f32,
    pub grind_stability_nudge: f32,
    pub grind_up_down: f32,
    pub grind_grab_loose_end_height: f32,
    pub physics_body_spin: f32,
    pub world_grab_y: f32,
    pub world_grab_z: f32,
    pub wipeout_control_x: f32,
    pub wipeout_control_y: f32,
    pub off_board_jump_x: f32,
    pub off_board_jump_z: f32,
    pub minimum_jump: Option<f32>,
    pub trick_height: Option<f32>,
    pub start_revert: Option<f32>,
    pub jump_strength: f32,
    pub grind_animation: Option<String>,
    pub grind_facing: Option<GrindFacing>,
    pub contact_bone_name: Option<String>,
    pub animation_push_speed: f32,
    pub contact_flags: BTreeSet<ContactIntentFlag>,
    pub hand_grabs: BTreeSet<HandGrab>,
    pub flags: BTreeSet<AnimationIntentFlag>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrindFacing {
    Forwards,
    Backwards,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HandGrab {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ContactIntentFlag {
    Push,
    PushUsesReferenceBone,
    PushStarted,
    PushEnded,
    Brake,
    BrakeStarted,
    BrakeEnded,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AnimationIntentFlag {
    ManualBrake,
    Wipeout,
    PrepareJump,
    Jump,
    FlipTrick,
    Grabbing,
    WantsAirGrab,
    GrabWorld,
    NoInput,
    ExitGrind,
    NewAutoPump,
    EnteringCoffin,
    InCoffin,
    LeavingCoffin,
    PlayingLanding,
    KickTurn,
    Anticipating,
    OffBoardTrajectory,
    OffBoardCenterOfMass,
    OffBoardAir,
    Carving,
    FrontFlip,
    BackFlip,
    WipeoutRecover,
    ControlledWipeout,
    Reverting,
    DangerZone,
    LateTrick,
    NoDangerZone,
    ChristAir,
    NewHandPlant,
    AnimationCommittedHandPlant,
    DisableFeetTargetIk,
    OffBoardJump,
    OffBoardStandingJump,
    PlayerControlledPump,
    OneFootAir,
    OffBoardMounting,
    OffBoardUnmounting,
    Dismount,
    HoldBoard,
    ReleaseBoard,
    ThrowBoard,
    RetrieveBoard,
    RetrievingBoard,
    DroppingBoard,
    FootJump,
    FullyAnimatedSkateboard,
    HippyJumping,
    PushTricking,
    KickoutDismount,
    RunoutDismount,
    FixLeftFoot,
    FixRightFoot,
    LandOnBoard,
    TransitioningOnOffBoard,
    OffBoardSteering,
    OffBoardJumpAboutToTakeOff,
    OffBoardJumpInProgress,
    SkaterOffBoard,
    RunMount,
    SprintMount,
}

impl ProcessedAnimationIntent {
    /// Interprets the reference's exact case-sensitive scalar attribute keys
    /// in source order, matching `Skeleton::ProcessAnimAttributes`.
    #[must_use]
    pub fn from_scalar_attributes(attributes: &[ScalarAnimationAttribute]) -> Self {
        let mut intent = Self::default();
        let mut revert_direction = 1.0;
        for attribute in attributes {
            if intent.apply_scalar_attribute(attribute, &mut revert_direction) {
                continue;
            }
            if intent.apply_on_board_attribute(&attribute.name) {
                continue;
            }
            intent.apply_off_board_attribute(attribute);
        }
        if intent.flags.contains(&AnimationIntentFlag::Reverting) {
            intent.start_revert = Some(revert_direction);
        }
        intent
    }

    fn apply_contact_attributes(
        &mut self,
        attributes: &[ContactAnimationAttribute],
        push_reference_bone_name: Option<&str>,
        trajectory_translation: Option<[f32; 3]>,
        time_delta_seconds: f32,
    ) {
        for attribute in attributes {
            self.contact_bone_name = Some(attribute.bone_name.clone());
            match attribute.name.as_str() {
                "push_contact" => {
                    self.contact_flags.insert(ContactIntentFlag::Push);
                    self.set_contact_flag(
                        ContactIntentFlag::PushUsesReferenceBone,
                        push_reference_bone_name == Some(attribute.bone_name.as_str()),
                    );
                    let started = attribute.value.to_bits() == 1.0_f32.to_bits();
                    self.set_contact_flag(ContactIntentFlag::PushStarted, started);
                    self.set_contact_flag(
                        ContactIntentFlag::PushEnded,
                        attribute.value.to_bits() == (-1.0_f32).to_bits(),
                    );
                    self.animation_push_speed = if started {
                        trajectory_translation.map_or(0.0, |translation| {
                            translation
                                .into_iter()
                                .map(|value| value * value)
                                .sum::<f32>()
                                .sqrt()
                                / time_delta_seconds
                        })
                    } else {
                        0.0
                    };
                }
                "brake_contact" => {
                    self.contact_flags.insert(ContactIntentFlag::Brake);
                    self.set_contact_flag(
                        ContactIntentFlag::BrakeStarted,
                        attribute.value.to_bits() == 1.0_f32.to_bits(),
                    );
                    self.set_contact_flag(
                        ContactIntentFlag::BrakeEnded,
                        attribute.value.to_bits() == (-1.0_f32).to_bits(),
                    );
                }
                "right_hand_grab" | "left_hand_grab" => {
                    self.hand_grabs
                        .insert(if attribute.bone_name == "righthand" {
                            HandGrab::Right
                        } else {
                            HandGrab::Left
                        });
                }
                _ => {}
            }
        }
    }

    fn apply_jump_strength(&mut self, tuning: AnimationPhysicsTuning) {
        let authored_height = self.trick_height.unwrap_or(0.0);
        if self.flags.contains(&AnimationIntentFlag::Jump) {
            self.jump_strength = if !tuning.use_jump_strength_endpoints {
                authored_height
            } else if authored_height <= tuning.jump_strength_lower_endpoint {
                self.minimum_jump.unwrap_or(0.0)
            } else if authored_height >= tuning.jump_strength_upper_endpoint {
                1.0
            } else {
                authored_height
            };
        }
        if self.flags.contains(&AnimationIntentFlag::HippyJumping) {
            self.jump_strength = authored_height;
        }
    }

    fn set_flag(&mut self, flag: AnimationIntentFlag) {
        self.flags.insert(flag);
    }

    fn set_contact_flag(&mut self, flag: ContactIntentFlag, enabled: bool) {
        if enabled {
            self.contact_flags.insert(flag);
        } else {
            self.contact_flags.remove(&flag);
        }
    }

    fn apply_scalar_attribute(
        &mut self,
        attribute: &ScalarAnimationAttribute,
        revert_direction: &mut f32,
    ) -> bool {
        let value = attribute.value;
        match attribute.name.as_str() {
            "Spin" => self.spin = -value,
            "BodySpin" => self.body_spin = -value,
            "Brake" => self.foot_brake = value,
            "Turn" => self.turn = -value,
            "OB_Turn" => self.off_board_turn = -value,
            "OB_LookAtX" => self.off_board_look_at_x = value,
            "OB_LookAtY" => self.off_board_look_at_y = value,
            "RawTurn" => self.raw_turn = -value,
            "HardTurn" => self.hard_turn = -value,
            "Slide" => self.slide = -value,
            "Balance" => self.balance = -value,
            "OB_ObjectMvZ" => self.move_object_z = value,
            "OB_ObjectMvX" => self.move_object_x = value,
            "OB_ObjectMvRot" => self.move_object_rotation = value,
            "PhysGrindTranslation" => self.grind_translation = value,
            "PhysGrindStabilityNudge" => self.grind_stability_nudge = value,
            "PhysGrindUpDown" => self.grind_up_down = value,
            "PhysGrindGrabMinHeight" => self.grind_grab_loose_end_height = value,
            "PhysBodySpin" => self.physics_body_spin = value,
            "world_grab_y" => self.world_grab_y = value,
            "world_grab_z" => self.world_grab_z = value,
            "WipeoutControlX" => self.wipeout_control_x = value,
            "WipeoutControlY" => self.wipeout_control_y = value,
            "OB_JumpX" => self.off_board_jump_x = -value,
            "OB_JumpZ" => self.off_board_jump_z = value,
            "MinJump" => self.minimum_jump = Some(value),
            "TrickHeight" => self.trick_height = Some(value),
            "RevertDir" => *revert_direction = value,
            "GrindFacingForwards" => self.grind_facing = Some(GrindFacing::Forwards),
            "GrindFacingBackwards" => self.grind_facing = Some(GrindFacing::Backwards),
            "SlideBoard" | "SlideNose" | "SlideTail" | "Grind5050" | "Grind5_O" | "GrindNose" => {
                self.grind_animation = Some(attribute.name.clone());
            }
            _ => return false,
        }
        true
    }

    fn apply_on_board_attribute(&mut self, name: &str) -> bool {
        let flag = match name {
            "ManualBrake" => AnimationIntentFlag::ManualBrake,
            "Wipeout" => AnimationIntentFlag::Wipeout,
            "PrepareJump" => AnimationIntentFlag::PrepareJump,
            "jump" => AnimationIntentFlag::Jump,
            "FlipTrick" => AnimationIntentFlag::FlipTrick,
            "Grabbing" => AnimationIntentFlag::Grabbing,
            "WantsLeftAirGrab" | "WantsRightAirGrab" => AnimationIntentFlag::WantsAirGrab,
            "GrabWorld" => AnimationIntentFlag::GrabWorld,
            "NoInput" => AnimationIntentFlag::NoInput,
            "ExitGrind" => AnimationIntentFlag::ExitGrind,
            "NewAutoPump" => AnimationIntentFlag::NewAutoPump,
            "EnteringCoffin" => AnimationIntentFlag::EnteringCoffin,
            "InCoffin" => AnimationIntentFlag::InCoffin,
            "LeavingCoffin" => AnimationIntentFlag::LeavingCoffin,
            "PlayingLanding" => AnimationIntentFlag::PlayingLanding,
            "KickTurn" => AnimationIntentFlag::KickTurn,
            "Anticipating" => AnimationIntentFlag::Anticipating,
            "Carving" => AnimationIntentFlag::Carving,
            "FrontFlip" => AnimationIntentFlag::FrontFlip,
            "BackFlip" => AnimationIntentFlag::BackFlip,
            "WipeOutRecover" => AnimationIntentFlag::WipeoutRecover,
            "ControlledWipeout" => AnimationIntentFlag::ControlledWipeout,
            "Revert" => AnimationIntentFlag::Reverting,
            "DangerZone" => AnimationIntentFlag::DangerZone,
            "LateTrick" => AnimationIntentFlag::LateTrick,
            "NoDangerZone" => AnimationIntentFlag::NoDangerZone,
            "ChristAir" => AnimationIntentFlag::ChristAir,
            "NewHandPlant" => AnimationIntentFlag::NewHandPlant,
            "AnimCommittedHandPlant" => AnimationIntentFlag::AnimationCommittedHandPlant,
            "DisableFeetTargetIK" => AnimationIntentFlag::DisableFeetTargetIk,
            "PlayerControlledPump" => AnimationIntentFlag::PlayerControlledPump,
            "OneFootAir" => AnimationIntentFlag::OneFootAir,
            _ => return false,
        };
        self.set_flag(flag);
        true
    }

    fn apply_off_board_attribute(&mut self, attribute: &ScalarAnimationAttribute) {
        let flag = match attribute.name.as_str() {
            "OB_Traj" => AnimationIntentFlag::OffBoardTrajectory,
            "OB_Com" => AnimationIntentFlag::OffBoardCenterOfMass,
            "OB_Air" => AnimationIntentFlag::OffBoardAir,
            "OB_Jump" => AnimationIntentFlag::OffBoardJump,
            "OB_StandingJump" => AnimationIntentFlag::OffBoardStandingJump,
            "OB_Mounting" => AnimationIntentFlag::OffBoardMounting,
            "OB_Unmounting" => AnimationIntentFlag::OffBoardUnmounting,
            "OB_Dismount" => AnimationIntentFlag::Dismount,
            "OB_HoldBoard" => AnimationIntentFlag::HoldBoard,
            "OB_ReleaseBoard" => AnimationIntentFlag::ReleaseBoard,
            "OB_ThrowBoard" => AnimationIntentFlag::ThrowBoard,
            "OB_RetrieveBoard" => AnimationIntentFlag::RetrieveBoard,
            "OB_RetrievingBoard" => AnimationIntentFlag::RetrievingBoard,
            "OB_DroppingBoard" => AnimationIntentFlag::DroppingBoard,
            "FootJump" => AnimationIntentFlag::FootJump,
            "AnimSkateboard" => AnimationIntentFlag::FullyAnimatedSkateboard,
            "HippyJumping" => AnimationIntentFlag::HippyJumping,
            "PushTricking" => AnimationIntentFlag::PushTricking,
            "KickoutDismount" => AnimationIntentFlag::KickoutDismount,
            "RunoutDismount" => AnimationIntentFlag::RunoutDismount,
            "FixLeftFoot" => AnimationIntentFlag::FixLeftFoot,
            "FixRightFoot" => AnimationIntentFlag::FixRightFoot,
            "DoLandOnBoard" => AnimationIntentFlag::LandOnBoard,
            "TransitioningOnOffBoard" => AnimationIntentFlag::TransitioningOnOffBoard,
            "OBL_SteerMagnitude" if attribute.value > 0.2 => AnimationIntentFlag::OffBoardSteering,
            "JumpAboutToTakeOff" => AnimationIntentFlag::OffBoardJumpAboutToTakeOff,
            "JumpInProgress" => AnimationIntentFlag::OffBoardJumpInProgress,
            "SkaterOffBoard" => AnimationIntentFlag::SkaterOffBoard,
            "RunMount" => AnimationIntentFlag::RunMount,
            "SprintMount" => AnimationIntentFlag::SprintMount,
            _ => return,
        };
        self.set_flag(flag);
    }
}

impl AnimationPhysicsInput {
    /// Selected constructor/reset projection with owned pose buffers.
    /// This does not construct the complete native `AnimOutPhysIn` owner.
    /// Reset825ecd6c..a4 stores identity axes and an entirely zero W axis.
    #[must_use]
    pub fn reset(bone_count: usize) -> Self {
        Self {
            global_pose: vec![AFFINE_IDENTITY_ORIENTATION; bone_count],
            local_pose: vec![AFFINE_IDENTITY_ORIENTATION; bone_count],
            time_delta_seconds: PHYSICS_STEP_SECONDS,
            compression: DEFAULT_COMPRESSION,
            flags: BTreeSet::new(),
            trick: AnimationTrickData::default(),
            scalar_attributes: Vec::new(),
            contact_attributes: Vec::new(),
            trajectory_bone_index: None,
            push_reference_bone_name: None,
            truck_tightness: 0.0,
            wheel_hardness: 0.0,
        }
    }

    /// Installs the paired global/local animation buffers required by the
    /// reference skeleton physics update.
    ///
    /// # Errors
    ///
    /// Returns [`PhysicsHandoffError`] unless both buffers have the bone count
    /// established at reset time.
    pub fn set_animation_pose(
        &mut self,
        global_pose: &[AnimationMatrix],
        local_pose: &[AnimationMatrix],
    ) -> Result<(), PhysicsHandoffError> {
        let expected = self.global_pose.len();
        if global_pose.len() != expected || local_pose.len() != expected {
            return Err(PhysicsHandoffError::PoseBoneCountMismatch {
                expected,
                global: global_pose.len(),
                local: local_pose.len(),
            });
        }
        self.global_pose.copy_from_slice(global_pose);
        self.local_pose.copy_from_slice(local_pose);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PhysicsState {
    #[default]
    Invalid,
    PhysicsGround,
    SlideGround,
    RevertGround,
    GroundAnimation,
    Skitching,
    FollowPath,
    PhysicsAir,
    KnownAir,
    WipeoutGround,
    GrindBoardslide,
    Grind5050,
    GrindTipslide,
    GrindFiveO,
    GrindBackslash,
    OffBoard,
    OffBoardPushing,
    OffBoardAir,
    OffBoardJump,
    OffBoardLocomotion,
    LandingOnDeck,
    HandPlant,
    FootPlant,
    Boneless,
    Sleeping,
    Nonspecific,
    Teleporting,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PhysicsCategory {
    #[default]
    Invalid,
    Ground,
    Air,
    Wipeout,
    Grind,
    OffBoard,
    GroundTrick,
    Other,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FilteredPhysicsState {
    #[default]
    Invalid,
    Ground,
    Air,
    Grind,
    Wipeout,
    Teleport,
    OffBoard,
    OffBoardAir,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GrindType {
    #[default]
    None,
    FiftyFifty,
    Boardslide,
    Tipslide,
    FiveO,
    Backslash,
}

/// The three `physics_airstates.default` values consulted by
/// `PhysicalPlayerStateChanger::ConditionIsOffGround`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GroundStateTuning {
    /// Height that unconditionally counts as clear of the riding surface.
    pub clear_ground_height: f32,
    /// Lower height that counts only after wheel contact has been absent long
    /// enough.
    pub recent_contact_ground_height: f32,
    /// Required time without wheel contact for the lower-height test.
    pub wheel_contact_grace_seconds: f32,
}

/// Result of the grind investigation consumed by the state changer. A grind
/// may be geometrically available but suppressed while dismounting or doing a
/// one-foot air.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GrindCandidate {
    pub is_possible: bool,
    pub grind_type: GrindType,
    pub dismounting: bool,
    pub doing_one_foot_air: bool,
}

impl GrindCandidate {
    fn suggested_state(self) -> Option<PhysicsState> {
        if !self.is_possible || self.dismounting || self.doing_one_foot_air {
            return None;
        }
        match self.grind_type {
            GrindType::None => None,
            GrindType::FiftyFifty => Some(PhysicsState::Grind5050),
            GrindType::Boardslide => Some(PhysicsState::GrindBoardslide),
            GrindType::Tipslide => Some(PhysicsState::GrindTipslide),
            GrindType::FiveO => Some(PhysicsState::GrindFiveO),
            GrindType::Backslash => Some(PhysicsState::GrindBackslash),
        }
    }
}

/// Named inputs used when the current state is `PHYSICS_GROUND`. This is the
/// source-level equivalent of the relevant `ProcessedPhysIn` flags; it does
/// not expose the original packed words or console structure offsets.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GroundStateFlag {
    HippyJumping,
    RightFootOnDeck,
    LeftFootOnDeck,
    RequestHandplant,
    FoundHandplantPosition,
    OffBoardTrajectory,
    OffBoardComMotion,
    OffBoardAir,
    WipingOut,
    TrajectorySelectorReady,
    DeckInContact,
    TruckInContact,
    SkateboardAnimated,
    GrabbingObject,
    GrabSplineValid,
    DisableSkitching,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GroundStateInput {
    pub flags: BTreeSet<GroundStateFlag>,
    pub frames_since_jump: u32,
    pub start_revert: f32,
    pub grind: GrindCandidate,
    pub num_wheels_in_contact: u8,
    pub wheel_height_to_ground: f32,
    pub time_since_wheels_touching: f32,
    pub teleport_countdown: u32,
    pub slide: f32,
}

impl GroundStateInput {
    /// Reconstructs the `PHYSICS_GROUND` case of
    /// `PhysicalPlayerStateChanger::CalcSuggestedState` at `0x82ddbd58`.
    /// Checks intentionally remain in reference priority order.
    #[must_use]
    pub fn suggested_state(&self, tuning: GroundStateTuning) -> PhysicsState {
        if self.has(GroundStateFlag::HippyJumping)
            && (!self.has(GroundStateFlag::RightFootOnDeck)
                || !self.has(GroundStateFlag::LeftFootOnDeck))
        {
            return PhysicsState::LandingOnDeck;
        }
        let handplant_has_clearance =
            self.has(GroundStateFlag::RequestHandplant) && self.num_wheels_in_contact <= 2;
        if self.has(GroundStateFlag::FoundHandplantPosition)
            && (handplant_has_clearance || self.num_wheels_in_contact == 0)
        {
            return PhysicsState::HandPlant;
        }
        if self.has(GroundStateFlag::OffBoardTrajectory) {
            return PhysicsState::OffBoard;
        }
        if self.has(GroundStateFlag::OffBoardComMotion) {
            return PhysicsState::OffBoardLocomotion;
        }
        if self.has(GroundStateFlag::OffBoardAir) {
            return PhysicsState::OffBoardAir;
        }
        if self.has(GroundStateFlag::WipingOut) {
            return PhysicsState::WipeoutGround;
        }
        if self.frames_since_jump == 1 {
            return if self.has(GroundStateFlag::TrajectorySelectorReady) {
                PhysicsState::KnownAir
            } else {
                PhysicsState::PhysicsAir
            };
        }
        if self.start_revert != 0.0 {
            return PhysicsState::RevertGround;
        }
        if let Some(state) = self.grind.suggested_state() {
            return state;
        }
        if !self.has_board_contact()
            && self.is_clear_of_ground(tuning)
            && self.teleport_countdown == 0
        {
            return PhysicsState::PhysicsAir;
        }
        if self.has(GroundStateFlag::SkateboardAnimated) && self.teleport_countdown == 0 {
            return PhysicsState::GroundAnimation;
        }
        if self.has(GroundStateFlag::GrabbingObject)
            && self.has(GroundStateFlag::GrabSplineValid)
            && !self.has(GroundStateFlag::DisableSkitching)
        {
            return PhysicsState::Skitching;
        }
        if self.slide != 0.0 {
            return PhysicsState::SlideGround;
        }
        PhysicsState::PhysicsGround
    }

    fn has(&self, flag: GroundStateFlag) -> bool {
        self.flags.contains(&flag)
    }

    fn has_board_contact(&self) -> bool {
        self.num_wheels_in_contact > 0
            || self.has(GroundStateFlag::DeckInContact)
            || self.has(GroundStateFlag::TruckInContact)
    }

    fn is_clear_of_ground(&self, tuning: GroundStateTuning) -> bool {
        self.wheel_height_to_ground > tuning.clear_ground_height
            || (self.wheel_height_to_ground > tuning.recent_contact_ground_height
                && self.time_since_wheels_touching > tuning.wheel_contact_grace_seconds)
    }
}

/// Readable portion of the reference `ProcessedPhysIn` working state. It is
/// reset at the start of `Update1_ProcessInput`, then populated from the
/// animation handoff before state-specific board/body simulation runs.
#[derive(Clone, Debug, PartialEq)]
pub struct ProcessedPhysicsInput {
    pub time_delta_seconds: f32,
    pub gravity: f32,
    pub compression: f32,
    pub flipped_controls_scalar: f32,
    pub truck_tightness: f32,
    pub wheel_hardness: f32,
    pub frames_since_jump: u32,
    pub frames_since_correct_jump: u32,
    pub board_flipped: bool,
    pub riding_switch: bool,
    pub physics_state: PhysicsState,
    pub physics_category: PhysicsCategory,
    pub filtered_state: FilteredPhysicsState,
    pub grind_type: GrindType,
    pub animation: ProcessedAnimationIntent,
}

impl ProcessedPhysicsInput {
    /// Reference: `ProcessedPhysIn::Reset` at `0x82b39098`.
    #[must_use]
    pub fn reset() -> Self {
        Self {
            time_delta_seconds: PHYSICS_STEP_SECONDS,
            gravity: -9.8,
            compression: 0.0,
            flipped_controls_scalar: 1.0,
            truck_tightness: 0.0,
            wheel_hardness: 0.0,
            frames_since_jump: 100,
            frames_since_correct_jump: 1_000,
            board_flipped: false,
            riding_switch: false,
            physics_state: PhysicsState::Invalid,
            physics_category: PhysicsCategory::Invalid,
            filtered_state: FilteredPhysicsState::Invalid,
            grind_type: GrindType::None,
            animation: ProcessedAnimationIntent::default(),
        }
    }

    /// Applies the semantic portion of `ProcessAnimOut` followed by the
    /// reconstructed scalar/state and contact portions of
    /// `ProcessAnimAttributes`.
    pub fn process_animation_handoff(
        &mut self,
        input: &AnimationPhysicsInput,
        tuning: AnimationPhysicsTuning,
    ) {
        self.time_delta_seconds = input.time_delta_seconds;
        self.compression = input.compression;
        self.truck_tightness = input.truck_tightness;
        self.wheel_hardness = input.wheel_hardness;
        self.board_flipped = input.flags.contains(&AnimationPhysicsFlag::BoardFlipped);
        self.riding_switch = input.flags.contains(&AnimationPhysicsFlag::SwitchStance);
        self.animation = ProcessedAnimationIntent::from_scalar_attributes(&input.scalar_attributes);
        let trajectory_translation = input
            .trajectory_bone_index
            .and_then(|index| input.local_pose.get(index))
            .map(|matrix| [matrix[12], matrix[13], matrix[14]]);
        self.animation.apply_contact_attributes(
            &input.contact_attributes,
            input.push_reference_bone_name.as_deref(),
            trajectory_translation,
            input.time_delta_seconds,
        );
        self.animation.apply_jump_strength(tuning);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhysicsHandoffError {
    PoseBoneCountMismatch {
        expected: usize,
        global: usize,
        local: usize,
    },
}

impl fmt::Display for PhysicsHandoffError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid animation-to-physics handoff: {self:?}")
    }
}

impl std::error::Error for PhysicsHandoffError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_builds_identity_pose_buffers_and_reference_defaults() {
        let input = AnimationPhysicsInput::reset(3);
        assert_eq!(input.global_pose, vec![AFFINE_IDENTITY_ORIENTATION; 3]);
        assert_eq!(input.local_pose, vec![AFFINE_IDENTITY_ORIENTATION; 3]);
        assert!((input.time_delta_seconds - PHYSICS_STEP_SECONDS).abs() < f32::EPSILON);
        assert!((input.compression - DEFAULT_COMPRESSION).abs() < f32::EPSILON);
        assert!(input.trick.is_empty());
        assert!(!input.flags.contains(&AnimationPhysicsFlag::IsLocomotion));
    }

    #[test]
    fn rejects_pose_buffers_with_the_wrong_bone_count() {
        let mut input = AnimationPhysicsInput::reset(2);
        let error = input
            .set_animation_pose(&[IDENTITY_MATRIX], &[IDENTITY_MATRIX; 2])
            .unwrap_err();
        assert_eq!(
            error,
            PhysicsHandoffError::PoseBoneCountMismatch {
                expected: 2,
                global: 1,
                local: 2,
            }
        );
    }

    #[test]
    fn motion_graph_outputs_are_one_shot() {
        let mut output = MotionGraphOutput {
            trick: AnimationTrickData::named_trick("kickflip"),
            scalar_attributes: vec![ScalarAnimationAttribute::new("compression", 0.75)],
            contact_attributes: vec![ContactAnimationAttribute::new(
                "push_contact",
                "righttoebase",
                1.0,
            )],
        };
        let mut input = AnimationPhysicsInput::reset(1);
        output.drain_into(&mut input);
        assert_eq!(input.trick.name.as_deref(), Some("kickflip"));
        assert!(input.trick.flags.contains(&TrickFlag::Trick));
        assert_eq!(input.scalar_attributes[0].name, "compression");
        assert!((input.scalar_attributes[0].value - 0.75).abs() < f32::EPSILON);
        assert!(output.trick.is_empty());
        assert!(output.scalar_attributes.is_empty());
        assert!(output.contact_attributes.is_empty());
        assert_eq!(input.contact_attributes.len(), 1);
    }

    #[test]
    fn processes_reference_animation_attributes() {
        let attributes = [
            ScalarAnimationAttribute::new("Spin", 0.4),
            ScalarAnimationAttribute::new("Balance", -0.25),
            ScalarAnimationAttribute::new("Brake", 0.7),
            ScalarAnimationAttribute::new("PrepareJump", 1.0),
            ScalarAnimationAttribute::new("FlipTrick", 0.0),
            ScalarAnimationAttribute::new("SlideNose", 1.0),
        ];
        let intent = ProcessedAnimationIntent::from_scalar_attributes(&attributes);
        assert!((intent.spin + 0.4).abs() < f32::EPSILON);
        assert!((intent.balance - 0.25).abs() < f32::EPSILON);
        assert!((intent.foot_brake - 0.7).abs() < f32::EPSILON);
        assert!(intent.flags.contains(&AnimationIntentFlag::PrepareJump));
        assert!(intent.flags.contains(&AnimationIntentFlag::FlipTrick));
        assert_eq!(intent.grind_animation.as_deref(), Some("SlideNose"));
    }

    #[test]
    fn preserves_duplicate_order_thresholds_and_revert_direction() {
        let attributes = [
            ScalarAnimationAttribute::new("Spin", 0.1),
            ScalarAnimationAttribute::new("Spin", 0.6),
            ScalarAnimationAttribute::new("Grind5050", 1.0),
            ScalarAnimationAttribute::new("SlideTail", 1.0),
            ScalarAnimationAttribute::new("Revert", 1.0),
            ScalarAnimationAttribute::new("RevertDir", -1.0),
            ScalarAnimationAttribute::new("OBL_SteerMagnitude", 0.2),
        ];
        let intent = ProcessedAnimationIntent::from_scalar_attributes(&attributes);
        assert!((intent.spin + 0.6).abs() < f32::EPSILON);
        assert_eq!(intent.grind_animation.as_deref(), Some("SlideTail"));
        assert_eq!(intent.start_revert, Some(-1.0));
        assert!(
            !intent
                .flags
                .contains(&AnimationIntentFlag::OffBoardSteering)
        );

        let steering =
            ProcessedAnimationIntent::from_scalar_attributes(&[ScalarAnimationAttribute::new(
                "OBL_SteerMagnitude",
                0.200_001,
            )]);
        assert!(
            steering
                .flags
                .contains(&AnimationIntentFlag::OffBoardSteering)
        );
    }

    #[test]
    fn processes_off_board_and_grind_scalars_without_sign_guessing() {
        let attributes = [
            ScalarAnimationAttribute::new("OB_Turn", 0.3),
            ScalarAnimationAttribute::new("OB_JumpX", -0.4),
            ScalarAnimationAttribute::new("PhysGrindTranslation", 0.75),
            ScalarAnimationAttribute::new("world_grab_z", 1.25),
            ScalarAnimationAttribute::new("WipeoutControlY", -0.8),
            ScalarAnimationAttribute::new("OB_Jump", 1.0),
            ScalarAnimationAttribute::new("AnimSkateboard", 1.0),
        ];
        let intent = ProcessedAnimationIntent::from_scalar_attributes(&attributes);
        assert!((intent.off_board_turn + 0.3).abs() < f32::EPSILON);
        assert!((intent.off_board_jump_x - 0.4).abs() < f32::EPSILON);
        assert!((intent.grind_translation - 0.75).abs() < f32::EPSILON);
        assert!((intent.world_grab_z - 1.25).abs() < f32::EPSILON);
        assert!((intent.wipeout_control_y + 0.8).abs() < f32::EPSILON);
        assert!(intent.flags.contains(&AnimationIntentFlag::OffBoardJump));
        assert!(
            intent
                .flags
                .contains(&AnimationIntentFlag::FullyAnimatedSkateboard)
        );
    }

    #[test]
    fn processes_typed_contact_attributes_and_trajectory_speed() {
        let mut input = AnimationPhysicsInput::reset(1);
        input.local_pose[0][12] = 0.03;
        input.local_pose[0][13] = 0.04;
        input.trajectory_bone_index = Some(0);
        input.push_reference_bone_name = Some("righttoebase".to_owned());
        input.contact_attributes = vec![
            ContactAnimationAttribute::new("push_contact", "righttoebase", 1.0),
            ContactAnimationAttribute::new("brake_contact", "lefttoebase", -1.0),
            ContactAnimationAttribute::new("right_hand_grab", "righthand", 1.0),
            ContactAnimationAttribute::new("left_hand_grab", "lefthand", 1.0),
        ];

        let mut processed = ProcessedPhysicsInput::reset();
        processed.process_animation_handoff(&input, AnimationPhysicsTuning::default());
        let intent = &processed.animation;
        assert!(intent.contact_flags.contains(&ContactIntentFlag::Push));
        assert!(
            intent
                .contact_flags
                .contains(&ContactIntentFlag::PushUsesReferenceBone)
        );
        assert!(
            intent
                .contact_flags
                .contains(&ContactIntentFlag::PushStarted)
        );
        assert!(!intent.contact_flags.contains(&ContactIntentFlag::PushEnded));
        assert!((intent.animation_push_speed - 3.0).abs() < f32::EPSILON);
        assert!(intent.contact_flags.contains(&ContactIntentFlag::Brake));
        assert!(
            !intent
                .contact_flags
                .contains(&ContactIntentFlag::BrakeStarted)
        );
        assert!(
            intent
                .contact_flags
                .contains(&ContactIntentFlag::BrakeEnded)
        );
        assert_eq!(intent.contact_bone_name.as_deref(), Some("lefthand"));
        assert_eq!(
            intent.hand_grabs,
            BTreeSet::from([HandGrab::Left, HandGrab::Right])
        );
    }

    #[test]
    fn applies_reference_jump_strength_endpoints_and_hippy_override() {
        let tuning = AnimationPhysicsTuning {
            use_jump_strength_endpoints: true,
            jump_strength_lower_endpoint: 0.2,
            jump_strength_upper_endpoint: 0.8,
        };
        let evaluate = |height: f32, hippy: bool| {
            let mut attributes = vec![
                ScalarAnimationAttribute::new("jump", 1.0),
                ScalarAnimationAttribute::new("MinJump", 0.35),
                ScalarAnimationAttribute::new("TrickHeight", height),
            ];
            if hippy {
                attributes.push(ScalarAnimationAttribute::new("HippyJumping", 1.0));
            }
            let mut intent = ProcessedAnimationIntent::from_scalar_attributes(&attributes);
            intent.apply_jump_strength(tuning);
            intent.jump_strength
        };

        assert!((evaluate(0.1, false) - 0.35).abs() < f32::EPSILON);
        assert!((evaluate(0.6, false) - 0.6).abs() < f32::EPSILON);
        assert!((evaluate(0.9, false) - 1.0).abs() < f32::EPSILON);
        assert!((evaluate(0.9, true) - 0.9).abs() < f32::EPSILON);

        let mut disabled = ProcessedAnimationIntent::from_scalar_attributes(&[
            ScalarAnimationAttribute::new("jump", 1.0),
            ScalarAnimationAttribute::new("MinJump", 0.35),
            ScalarAnimationAttribute::new("TrickHeight", 0.1),
        ]);
        disabled.apply_jump_strength(AnimationPhysicsTuning::default());
        assert!((disabled.jump_strength - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn grounded_state_uses_reference_priority_order() {
        let input = GroundStateInput {
            flags: BTreeSet::from([
                GroundStateFlag::HippyJumping,
                GroundStateFlag::LeftFootOnDeck,
                GroundStateFlag::FoundHandplantPosition,
                GroundStateFlag::RequestHandplant,
                GroundStateFlag::WipingOut,
            ]),
            frames_since_jump: 1,
            start_revert: 1.0,
            slide: 1.0,
            ..GroundStateInput::default()
        };
        assert_eq!(
            input.suggested_state(GroundStateTuning::default()),
            PhysicsState::LandingOnDeck
        );

        let handplant = GroundStateInput {
            flags: BTreeSet::from([
                GroundStateFlag::FoundHandplantPosition,
                GroundStateFlag::RequestHandplant,
                GroundStateFlag::WipingOut,
            ]),
            num_wheels_in_contact: 2,
            ..GroundStateInput::default()
        };
        assert_eq!(
            handplant.suggested_state(GroundStateTuning::default()),
            PhysicsState::HandPlant
        );
    }

    #[test]
    fn grounded_state_applies_two_stage_ground_clearance() {
        let tuning = GroundStateTuning {
            clear_ground_height: 0.3,
            recent_contact_ground_height: 0.1,
            wheel_contact_grace_seconds: 0.2,
        };
        let mut input = GroundStateInput {
            wheel_height_to_ground: 0.2,
            time_since_wheels_touching: 0.21,
            ..GroundStateInput::default()
        };
        assert_eq!(input.suggested_state(tuning), PhysicsState::PhysicsAir);

        input.time_since_wheels_touching = 0.2;
        assert_eq!(input.suggested_state(tuning), PhysicsState::PhysicsGround);
        input.wheel_height_to_ground = 0.31;
        assert_eq!(input.suggested_state(tuning), PhysicsState::PhysicsAir);
        input.flags.insert(GroundStateFlag::DeckInContact);
        assert_eq!(input.suggested_state(tuning), PhysicsState::PhysicsGround);
        input.flags.remove(&GroundStateFlag::DeckInContact);
        input.teleport_countdown = 1;
        assert_eq!(input.suggested_state(tuning), PhysicsState::PhysicsGround);
    }

    #[test]
    fn grounded_state_maps_grind_types_and_suppression() {
        let base = GroundStateInput {
            grind: GrindCandidate {
                is_possible: true,
                grind_type: GrindType::Tipslide,
                ..GrindCandidate::default()
            },
            ..GroundStateInput::default()
        };
        assert_eq!(
            base.suggested_state(GroundStateTuning::default()),
            PhysicsState::GrindTipslide
        );
        assert_eq!(
            GroundStateInput {
                grind: GrindCandidate {
                    doing_one_foot_air: true,
                    ..base.grind
                },
                slide: 0.5,
                ..base.clone()
            }
            .suggested_state(GroundStateTuning::default()),
            PhysicsState::SlideGround
        );
    }

    #[test]
    fn point_graph_clamps_and_interpolates_like_reference() {
        let graph = PointGraph {
            x: [0.0, 1.0, 3.0, 4.0],
            y: [2.0, 4.0, 8.0, 10.0],
        };
        assert!((graph.evaluate(-1.0) - 2.0).abs() < f32::EPSILON);
        assert!((graph.evaluate(0.5) - 3.0).abs() < f32::EPSILON);
        assert!((graph.evaluate(2.0) - 6.0).abs() < f32::EPSILON);
        assert!((graph.evaluate(4.0) - 10.0).abs() < f32::EPSILON);
        assert!((graph.evaluate(8.0) - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn point_graph_skips_repeated_coordinates_without_dividing() {
        let graph = PointGraph {
            x: [0.0, 0.0, 2.0],
            y: [1.0, 3.0, 5.0],
        };
        assert!((graph.evaluate(-0.5) - 1.0).abs() < f32::EPSILON);
        assert!((graph.evaluate(0.0) - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn point_graph_interpolation_keeps_the_native_fused_negative_residual() {
        // Slope=(1+2^-23), delta=(1-2^-23), yLo=-1. The source
        // fmadds at 0x824e2d28 retains -2^-46; separate operations yield +0.
        let graph = PointGraph {
            x: [0.0, 1.0],
            y: [-1.0, f32::from_bits(0x3400_0000)],
        };
        let query = f32::from_bits(0x3f7f_fffe);
        assert_eq!(graph.evaluate(query).to_bits(), 0xa880_0000);
        let slope = graph.y[1] - graph.y[0];
        let rounded_product = slope * query;
        assert_eq!((rounded_product + graph.y[0]).to_bits(), 0);
    }

    #[test]
    fn point_graph_unordered_endpoint_comparisons_select_original_raw_samples() {
        let mut graph = PointGraph {
            x: [0.0, 1.0, 2.0],
            y: [
                f32::from_bits(0x8000_0000),
                7.0,
                f32::from_bits(0x7fc1_2345),
            ],
        };
        assert_eq!(graph.evaluate(f32::NAN).to_bits(), 0x7fc1_2345);
        assert_eq!(graph.evaluate(f32::INFINITY).to_bits(), 0x7fc1_2345);
        assert_eq!(graph.evaluate(f32::NEG_INFINITY).to_bits(), 0x8000_0000);
        graph.x[2] = f32::NAN;
        assert_eq!(graph.evaluate(0.5).to_bits(), 0x7fc1_2345);
        // The first endpoint comparison precedes the unordered last one.
        assert_eq!(graph.evaluate(-1.0).to_bits(), 0x8000_0000);
    }

    #[test]
    fn point_graph_unordered_search_coordinate_is_skipped_then_width_selects_upper() {
        let graph = PointGraph {
            x: [0.0, f32::NAN, 2.0, 3.0],
            y: [1.0, 9.0, f32::from_bits(0x8000_0000), 30.0],
        };
        // x[1] does not satisfy query<x[1]. At upper=2 the width is NaN:
        // native BO4/BI25 at 0x824e2d10 selects y[2], without division.
        assert_eq!(graph.evaluate(1.0).to_bits(), 0x8000_0000);
        let first_unordered = PointGraph {
            x: [f32::NAN, 1.0, 2.0],
            y: [1.0, 13.0, 21.0],
        };
        assert_eq!(first_unordered.evaluate(0.5).to_bits(), 13.0_f32.to_bits());
    }

    #[test]
    fn point_graph_equal_duplicate_and_descending_coordinates_keep_search_order() {
        let duplicate = PointGraph {
            x: [0.0, 1.0, 1.0, 3.0],
            y: [0.0, 99.0, 10.0, 30.0],
        };
        assert_eq!(duplicate.evaluate(1.0).to_bits(), 10.0_f32.to_bits());
        let descending_segment = PointGraph {
            x: [0.0, 2.0, 1.0, 3.0],
            y: [0.0, 77.0, 10.0, 30.0],
        };
        assert_eq!(
            descending_segment.evaluate(2.0).to_bits(),
            20.0_f32.to_bits()
        );
        let descending_endpoints = PointGraph {
            x: [3.0, 2.0, 1.0],
            y: [11.0, 22.0, 33.0],
        };
        assert_eq!(
            descending_endpoints.evaluate(2.5).to_bits(),
            11.0_f32.to_bits()
        );
        assert_eq!(
            descending_endpoints.evaluate(3.0).to_bits(),
            33.0_f32.to_bits()
        );
        let single = PointGraph {
            x: [1.0],
            y: [-0.0],
        };
        for query in [0.0, 1.0, 2.0, f32::NAN] {
            assert_eq!(single.evaluate(query).to_bits(), 0x8000_0000);
        }
    }

    #[test]
    #[should_panic(expected = "point graph must contain at least one sample")]
    fn point_graph_empty_storage_is_a_host_preflight_not_a_native_negative_index_read() {
        let _ = PointGraph::<0> { x: [], y: [] }.evaluate(0.0);
    }

    #[test]
    fn calculates_ground_jump_height_and_ballistic_speed() {
        let curves = JumpPhysicsCurves {
            vertical_response: constant_graph(1.0),
            jump_y_scalar_vs_ground_normal_y: constant_graph(1.0),
            jump_speed_scalar_vs_angle: constant_graph(1.0),
            min_height_vs_speed: constant_graph(1.0),
            max_height_vs_speed: constant_graph(1.0),
        };
        let tuning = GroundJumpTuning {
            height_curve_reference_speed: 27.8,
            low_speed_jump_height: 0.95,
            minimum_jump_height: 1.33,
            maximum_jump_height: 1.71,
            use_vertical_response_curve: true,
            minimum_vertical_response: 0.1,
            maximum_horizontal_compensation: 1.5,
            forward_adjustment_scale: 0.18,
            sideways_adjustment_scale: 0.2,
        };
        let solution = calculate_ground_jump_vertical(
            GroundJumpVerticalInput {
                surface_speed: 0.0,
                jump_strength: 0.5,
                gravity: -9.8,
                jump_normal: [0.0, 1.0, 0.0],
                centre_of_mass: [2.0, 1.0, 4.0],
                ground_position: [2.0, 0.05, 4.0],
                reckoning_up: [0.0, 1.0, 0.0],
            },
            tuning,
            &curves,
        );

        assert!(solution.normalized_surface_speed.abs() < f32::EPSILON);
        assert!((solution.minimum_height - 1.33).abs() < 1.0e-6);
        assert!((solution.maximum_height - 1.71).abs() < 1.0e-6);
        assert!((solution.target_height - 1.52).abs() < 1.0e-6);
        assert!((solution.current_height - 0.95).abs() < 1.0e-6);
        assert!((solution.vertical_response - 1.0).abs() < f32::EPSILON);
        assert!((solution.launch_speed - (-2.0_f32 * -9.8 * 0.57).sqrt()).abs() < 1.0e-5);
    }

    #[test]
    fn clamps_ground_jump_speed_input_and_vertical_response() {
        let curves = JumpPhysicsCurves {
            vertical_response: constant_graph(0.05),
            jump_y_scalar_vs_ground_normal_y: constant_graph(1.0),
            jump_speed_scalar_vs_angle: constant_graph(1.0),
            min_height_vs_speed: constant_graph(0.5),
            max_height_vs_speed: constant_graph(0.25),
        };
        let tuning = GroundJumpTuning {
            height_curve_reference_speed: 10.0,
            low_speed_jump_height: 1.0,
            minimum_jump_height: 2.0,
            maximum_jump_height: 3.0,
            use_vertical_response_curve: true,
            minimum_vertical_response: 0.1,
            ..GroundJumpTuning::default()
        };
        let solution = calculate_ground_jump_vertical(
            GroundJumpVerticalInput {
                surface_speed: 20.0,
                jump_strength: 1.0,
                gravity: -10.0,
                jump_normal: [1.0, 0.0, 0.0],
                centre_of_mass: [0.0; 3],
                ground_position: [0.0; 3],
                reckoning_up: [0.0, 1.0, 0.0],
            },
            tuning,
            &curves,
        );

        assert!((solution.normalized_surface_speed - 1.0).abs() < f32::EPSILON);
        assert!((solution.minimum_height - 1.5).abs() < f32::EPSILON);
        assert!((solution.maximum_height - 1.5).abs() < f32::EPSILON);
        assert!((solution.vertical_response - 0.1).abs() < f32::EPSILON);
        assert!((solution.launch_speed - 30.0_f32.sqrt() * 0.1).abs() < 1.0e-6);
    }

    #[test]
    fn keeps_prepared_ground_jump_tangent_within_error_limit() {
        let solution = calculate_ground_jump_tangential_velocity(
            GroundJumpTangentialInput {
                current_velocity: [4.0, 2.0, 0.0],
                prepared_jump_velocity: [4.0, 5.0, 0.0],
                jump_normal: [0.0, 1.0, 0.0],
                maximum_velocity_error_squared: 0.0,
            },
            &constant_graph(0.5),
        );

        assert_vec3_close(solution.current_tangent, [4.0, 0.0, 0.0]);
        assert_vec3_close(solution.prepared_tangent, [4.0, 0.0, 0.0]);
        assert_vec3_close(solution.selected_tangent, [4.0, 0.0, 0.0]);
        assert!(!solution.corrected);
    }

    #[test]
    fn clamps_current_ground_jump_tangent_after_large_direction_error() {
        let solution = calculate_ground_jump_tangential_velocity(
            GroundJumpTangentialInput {
                current_velocity: [2.0, 4.0, 0.0],
                prepared_jump_velocity: [0.0, 3.0, 2.0],
                jump_normal: [0.0, 1.0, 0.0],
                maximum_velocity_error_squared: 1.0,
            },
            &constant_graph(0.5),
        );

        assert_vec3_close(solution.current_tangent, [2.0, 0.0, 0.0]);
        assert_vec3_close(solution.prepared_tangent, [0.0, 0.0, 2.0]);
        assert!((solution.normalized_direction_angle - 1.0).abs() < 1.0e-6);
        // Original 318fc scales curve0.5 by prepared length2 before clamping.
        assert_vec3_close(solution.selected_tangent, [1.0, 0.0, 0.0]);
        assert!(solution.corrected);
    }

    #[test]
    fn unordered_ground_jump_error_preserves_prepared_tangent() {
        let base = GroundJumpTangentialInput {
            current_velocity: [2.0, 4.0, 0.0],
            prepared_jump_velocity: [0.0, 3.0, 2.0],
            jump_normal: [0.0, 1.0, 0.0],
            maximum_velocity_error_squared: 1.0,
        };
        for input in [
            GroundJumpTangentialInput {
                maximum_velocity_error_squared: f32::NAN,
                ..base
            },
            GroundJumpTangentialInput {
                current_velocity: [f32::NAN, 4.0, 0.0],
                ..base
            },
            GroundJumpTangentialInput {
                prepared_jump_velocity: [f32::NAN, 3.0, 2.0],
                ..base
            },
        ] {
            let result = calculate_ground_jump_tangential_velocity(input, &constant_graph(0.5));
            assert!(!result.corrected);
            assert_eq!(
                result.selected_tangent.map(f32::to_bits),
                result.prepared_tangent.map(f32::to_bits)
            );
            assert_eq!(result.normalized_direction_angle.to_bits(), 0);
        }
    }

    #[test]
    fn composes_flat_ground_jump_in_the_board_frame() {
        let solution = flat_ground_jump_solution();

        assert_vec3_close(solution.launch_velocity, [2.0, 4.0, 3.0]);
        assert!((solution.vertical_compensation - 1.5).abs() < 1.0e-6);
        assert_vec3_close(solution.compensated_velocity, [2.0, 5.5, 3.0]);
        assert!((solution.velocity_along_jump_normal - 5.5).abs() < 1.0e-6);
        assert_vec3_close(solution.body_right_axis, [1.0, 0.0, 0.0]);
        assert_vec3_close(solution.body_forward_axis, [0.0, 0.0, 1.0]);
        assert_vec3_close(solution.new_velocity, [0.9, 5.5, 3.99]);
        assert_vec3_close(solution.delta_velocity, [-1.1, 5.5, 0.99]);
        assert!(solution.centre_of_mass_height.abs() < f32::EPSILON);
        assert!(solution.do_jump);
    }

    #[test]
    fn ground_animation_jump_prepares_and_finishes_trajectory_handoff() {
        let jump = flat_ground_jump_solution();
        let mut state = GroundAnimationState {
            jumped_this_frame: true,
            jump_velocity: [9.0; 3],
            ..GroundAnimationState::default()
        };
        state.begin_update();
        assert!(!state.jumped_this_frame);

        let skeleton_launch = TrajectoryLaunchInfo {
            start_velocity: PhysicsAirVector3::from_xyz_with_positive_zero_w([-1.0; 3]),
            velocity_before_jump: PhysicsAirVector3::from_xyz_with_positive_zero_w([2.0, 0.0, 3.0]),
            time_delta: PHYSICS_STEP_SECONDS,
            ..TrajectoryLaunchInfo::default()
        };
        let handoff = prepare_ground_animation_jump(&mut state, &jump, skeleton_launch)
            .expect("normal jump should enter the launch branch");
        assert!(state.jumped_this_frame);
        assert_vec3_close(state.jump_info.new_velocity, jump.new_velocity);
        assert_vec3_close(handoff.board_velocity, jump.new_velocity);
        assert!(handoff.linear_drag.abs() < f32::EPSILON);
        assert_vec3_close(
            handoff.trajectory_launch.start_velocity.xyz(),
            jump.new_velocity,
        );
        assert_eq!(
            handoff.trajectory_launch.start_velocity.lanes[3].to_bits(),
            0
        );
        assert!(handoff.trajectory_launch.player_jumped);
        assert_vec3_close(
            handoff.trajectory_launch.velocity_before_jump.xyz(),
            skeleton_launch.velocity_before_jump.xyz(),
        );

        let selected_launch = TrajectoryLaunchInfo {
            start_velocity: PhysicsAirVector3::from_xyz_with_positive_zero_w([1.0, 6.0, 4.0]),
            ..handoff.trajectory_launch
        };
        state.finish_trajectory_update(&selected_launch);
        assert_vec3_close(state.jump_velocity, selected_launch.start_velocity.xyz());
    }

    #[test]
    fn ground_animation_non_jump_keeps_force_path_available() {
        let mut jump = flat_ground_jump_solution();
        jump.do_jump = false;
        let mut state = GroundAnimationState::default();
        assert!(
            prepare_ground_animation_jump(&mut state, &jump, TrajectoryLaunchInfo::default())
                .is_none()
        );
        assert!(!state.jumped_this_frame);
        assert!(!state.jump_info.do_jump);
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum GroundAnimationForceCall {
        FrontFoot(u32, u32),
        RearFoot(u32, u32),
        Braking,
        SlideFriction(u32),
        LinearDrag,
        Collision(([u32; 3], [u32; 3])),
        SetLinearDrag(u32),
    }

    #[derive(Debug, Default)]
    struct RecordingGroundAnimationBoard {
        collided: bool,
        calls: Vec<GroundAnimationForceCall>,
    }

    impl GroundAnimationSkateboardForces for RecordingGroundAnimationBoard {
        fn calculate_front_foot_force(
            &mut self,
            time_since_landing: f32,
            absorption: f32,
        ) -> SkateboardForce {
            self.calls.push(GroundAnimationForceCall::FrontFoot(
                time_since_landing.to_bits(),
                absorption.to_bits(),
            ));
            marked_skateboard_force(1.0)
        }

        fn calculate_rear_foot_force(
            &mut self,
            time_since_landing: f32,
            absorption: f32,
        ) -> SkateboardForce {
            self.calls.push(GroundAnimationForceCall::RearFoot(
                time_since_landing.to_bits(),
                absorption.to_bits(),
            ));
            marked_skateboard_force(2.0)
        }

        fn calculate_braking_force(&mut self) -> SkateboardForce {
            self.calls.push(GroundAnimationForceCall::Braking);
            marked_skateboard_force(3.0)
        }

        fn calculate_slide_friction_force(&mut self, state_timer: f32) -> SkateboardForce {
            self.calls.push(GroundAnimationForceCall::SlideFriction(
                state_timer.to_bits(),
            ));
            marked_skateboard_force(4.0)
        }

        fn calculate_linear_drag(&mut self) -> f32 {
            self.calls.push(GroundAnimationForceCall::LinearDrag);
            0.75
        }

        fn calculate_collision_force(
            &mut self,
            collision_force: &mut SkateboardForce,
            discarded_torque: &mut std::mem::MaybeUninit<[f32; 3]>,
        ) -> bool {
            self.calls
                .push(GroundAnimationForceCall::Collision(skateboard_force_bits(
                    *collision_force,
                )));
            *collision_force = marked_skateboard_force(5.0);
            discarded_torque.write([6.0, 0.0, 0.0]);
            self.collided
        }

        fn set_linear_drag(&mut self, linear_drag: f32) {
            self.calls.push(GroundAnimationForceCall::SetLinearDrag(
                linear_drag.to_bits(),
            ));
        }
    }

    #[test]
    fn ground_animation_non_jump_preserves_calculation_and_force_order() {
        let state_timer = 2.25_f32;
        let mut board = RecordingGroundAnimationBoard::default();
        let mut queue = SkateboardForceQueue::default();

        update_ground_animation_non_jump(&mut board, state_timer, &mut queue);

        assert_eq!(
            board.calls,
            vec![
                GroundAnimationForceCall::FrontFoot(1.0_f32.to_bits(), 0.0_f32.to_bits()),
                GroundAnimationForceCall::RearFoot(1.0_f32.to_bits(), 0.0_f32.to_bits()),
                GroundAnimationForceCall::Braking,
                GroundAnimationForceCall::SlideFriction(state_timer.to_bits()),
                GroundAnimationForceCall::LinearDrag,
                GroundAnimationForceCall::Collision(skateboard_force_bits(
                    SkateboardForce::default(),
                )),
                GroundAnimationForceCall::SetLinearDrag(0.75_f32.to_bits()),
            ]
        );
        assert_eq!(
            queue.forces(),
            &[
                AppliedSkateboardForce {
                    kind: SkateboardForceKind::FrontFoot,
                    force: marked_skateboard_force(1.0),
                },
                AppliedSkateboardForce {
                    kind: SkateboardForceKind::RearFoot,
                    force: marked_skateboard_force(2.0),
                },
                AppliedSkateboardForce {
                    kind: SkateboardForceKind::SlideFriction,
                    force: marked_skateboard_force(4.0),
                },
                AppliedSkateboardForce {
                    kind: SkateboardForceKind::Brake,
                    force: marked_skateboard_force(3.0),
                },
            ]
        );
    }

    #[test]
    fn ground_animation_collision_preserves_existing_queue_and_suppresses_drag_write() {
        let mut board = RecordingGroundAnimationBoard {
            collided: true,
            ..RecordingGroundAnimationBoard::default()
        };
        let mut queue = SkateboardForceQueue::default();
        let existing = AppliedSkateboardForce {
            kind: SkateboardForceKind::Brake,
            force: marked_skateboard_force(0.25),
        };
        queue.add(existing.kind, existing.force);

        update_ground_animation_non_jump(&mut board, 9.0, &mut queue);

        assert_eq!(
            board.calls,
            vec![
                GroundAnimationForceCall::FrontFoot(1.0_f32.to_bits(), 0.0_f32.to_bits()),
                GroundAnimationForceCall::RearFoot(1.0_f32.to_bits(), 0.0_f32.to_bits()),
                GroundAnimationForceCall::Braking,
                GroundAnimationForceCall::SlideFriction(9.0_f32.to_bits()),
                GroundAnimationForceCall::LinearDrag,
                GroundAnimationForceCall::Collision(skateboard_force_bits(
                    SkateboardForce::default(),
                )),
            ]
        );
        assert_eq!(
            queue.forces(),
            &[
                existing,
                AppliedSkateboardForce {
                    kind: SkateboardForceKind::CollisionResponse,
                    force: marked_skateboard_force(5.0),
                },
            ]
        );
    }

    #[test]
    fn skateboard_force_queue_silently_drops_each_insertion_after_twenty_one() {
        let expected_new_kinds = [
            &[
                SkateboardForceKind::FrontFoot,
                SkateboardForceKind::RearFoot,
                SkateboardForceKind::SlideFriction,
            ][..],
            &[
                SkateboardForceKind::FrontFoot,
                SkateboardForceKind::RearFoot,
            ][..],
            &[SkateboardForceKind::FrontFoot][..],
            &[][..],
        ];

        for (initial_length, expected) in (18..=21).zip(expected_new_kinds) {
            let mut queue = SkateboardForceQueue::default();
            for _ in 0..initial_length {
                queue.add(
                    SkateboardForceKind::CollisionResponse,
                    SkateboardForce::default(),
                );
            }
            let mut board = RecordingGroundAnimationBoard::default();

            update_ground_animation_non_jump(&mut board, 0.0, &mut queue);

            assert_eq!(queue.forces().len(), MAX_APPLIED_SKATEBOARD_FORCES);
            let new_kinds = queue.forces()[initial_length..]
                .iter()
                .map(|force| force.kind)
                .collect::<Vec<_>>();
            assert_eq!(new_kinds, expected);
            assert_eq!(
                board.calls.last(),
                Some(&GroundAnimationForceCall::SetLinearDrag(0.75_f32.to_bits()))
            );
        }
    }

    #[test]
    fn ground_animation_forwards_state_timer_bits_without_sanitizing() {
        for state_timer in [-0.0_f32, f32::from_bits(0x7fc1_2345)] {
            let mut board = RecordingGroundAnimationBoard::default();
            update_ground_animation_non_jump(
                &mut board,
                state_timer,
                &mut SkateboardForceQueue::default(),
            );
            assert!(
                board
                    .calls
                    .contains(&GroundAnimationForceCall::SlideFriction(
                        state_timer.to_bits()
                    ))
            );
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum PhysicsAirSkateboardCall {
        SteeringTilt(u32),
        Collision(([u32; 3], [u32; 3])),
        CalcVelFromJump(i32),
        SetVelocity([f32; 3]),
    }

    #[derive(Debug, Default)]
    struct RecordingPhysicsAirSkateboard {
        collided: bool,
        collision_response: SkateboardForce,
        jump_velocity: [f32; 3],
        calls: Vec<PhysicsAirSkateboardCall>,
    }

    impl PhysicsAirSkateboard for RecordingPhysicsAirSkateboard {
        fn update_steering_tilt(&mut self, tilt: f32) {
            self.calls
                .push(PhysicsAirSkateboardCall::SteeringTilt(tilt.to_bits()));
        }

        fn calculate_collision_force(
            &mut self,
            collision_force: &mut SkateboardForce,
            discarded_torque: &mut std::mem::MaybeUninit<[f32; 3]>,
        ) -> bool {
            self.calls
                .push(PhysicsAirSkateboardCall::Collision(skateboard_force_bits(
                    *collision_force,
                )));
            *collision_force = self.collision_response;
            discarded_torque.write([91.0, 92.0, 93.0]);
            self.collided
        }

        fn calculate_velocity_from_jump(&mut self, frames_since_jump: i32) -> [f32; 3] {
            self.calls
                .push(PhysicsAirSkateboardCall::CalcVelFromJump(frames_since_jump));
            self.jump_velocity
        }

        fn set_velocity(&mut self, velocity: [f32; 3]) {
            self.calls
                .push(PhysicsAirSkateboardCall::SetVelocity(velocity));
        }
    }

    #[test]
    fn physics_air_collision_queues_only_type_fifteen_and_suppresses_jump_fix() {
        let collision_response = marked_skateboard_force(15.0);
        let mut board = RecordingPhysicsAirSkateboard {
            collided: true,
            collision_response,
            jump_velocity: [1.0, 2.0, 3.0],
            ..RecordingPhysicsAirSkateboard::default()
        };
        let mut queue = SkateboardForceQueue::default();

        update_physics_air_skateboard(&mut board, -1, 12, &mut queue);

        assert_eq!(
            board.calls,
            vec![
                PhysicsAirSkateboardCall::SteeringTilt(0.0_f32.to_bits()),
                PhysicsAirSkateboardCall::Collision(skateboard_force_bits(
                    SkateboardForce::default(),
                )),
            ]
        );
        assert_eq!(0.0_f32.to_bits(), 0x0000_0000);
        assert_eq!(
            queue.forces(),
            &[AppliedSkateboardForce {
                kind: SkateboardForceKind::CollisionResponse,
                force: collision_response,
            }]
        );
    }

    #[test]
    fn physics_air_full_queue_collision_still_suppresses_jump_fix() {
        let mut queue = SkateboardForceQueue::default();
        for _ in 0..MAX_APPLIED_SKATEBOARD_FORCES {
            queue.add(
                SkateboardForceKind::FrontFoot,
                marked_skateboard_force(0.25),
            );
        }
        let original_queue = queue.clone();
        let mut board = RecordingPhysicsAirSkateboard {
            collided: true,
            collision_response: marked_skateboard_force(15.0),
            jump_velocity: [1.0, 2.0, 3.0],
            ..RecordingPhysicsAirSkateboard::default()
        };

        update_physics_air_skateboard(&mut board, -1, 12, &mut queue);

        assert_eq!(queue, original_queue);
        assert_eq!(
            board.calls,
            vec![
                PhysicsAirSkateboardCall::SteeringTilt(0.0_f32.to_bits()),
                PhysicsAirSkateboardCall::Collision(skateboard_force_bits(
                    SkateboardForce::default(),
                )),
            ]
        );
    }

    #[test]
    fn physics_air_jump_fix_uses_strict_signed_frame_comparison() {
        let cases = [
            (11, 12, true),
            (12, 12, false),
            (-1, 12, true),
            (0, 0, false),
            (-1, 0, true),
        ];

        for (frames_since_jump_correct, jump_fix_frames, should_correct) in cases {
            let jump_velocity = [4.0, -5.0, 6.0];
            let mut board = RecordingPhysicsAirSkateboard {
                jump_velocity,
                ..RecordingPhysicsAirSkateboard::default()
            };
            let mut queue = SkateboardForceQueue::default();

            update_physics_air_skateboard(
                &mut board,
                frames_since_jump_correct,
                jump_fix_frames,
                &mut queue,
            );

            let mut expected_calls = vec![
                PhysicsAirSkateboardCall::SteeringTilt(0.0_f32.to_bits()),
                PhysicsAirSkateboardCall::Collision(skateboard_force_bits(
                    SkateboardForce::default(),
                )),
            ];
            if should_correct {
                expected_calls.push(PhysicsAirSkateboardCall::CalcVelFromJump(
                    frames_since_jump_correct,
                ));
                expected_calls.push(PhysicsAirSkateboardCall::SetVelocity(jump_velocity));
            }
            assert_eq!(board.calls, expected_calls);
            assert!(queue.forces().is_empty());
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum PhysicsAirTrajectoryLaunchCall {
        PopulateSkeleton,
        ReadFrames,
        ReadLimit,
        CalcVelFromJump(i32),
        ReadUseCentreOfMassVelocity,
        PopulateComTrajectory,
        Launch,
    }

    #[derive(Debug)]
    struct RecordingPhysicsAirTrajectoryLaunch {
        frames: i32,
        limit: i32,
        use_centre_of_mass_velocity: bool,
        skeleton_snapshot: TrajectoryLaunchInfo,
        com_snapshot: TrajectoryLaunchInfo,
        jump_velocity: PhysicsAirVector3,
        skeleton_input: Option<TrajectoryLaunchInfo>,
        launched: Option<TrajectoryLaunchInfo>,
        calls: Vec<PhysicsAirTrajectoryLaunchCall>,
    }

    impl Default for RecordingPhysicsAirTrajectoryLaunch {
        fn default() -> Self {
            Self {
                frames: 12,
                limit: 12,
                use_centre_of_mass_velocity: false,
                skeleton_snapshot: TrajectoryLaunchInfo::default(),
                com_snapshot: TrajectoryLaunchInfo::default(),
                jump_velocity: PhysicsAirVector3::default(),
                skeleton_input: None,
                launched: None,
                calls: Vec::new(),
            }
        }
    }

    impl PhysicsAirTrajectoryLaunchCollaborator for RecordingPhysicsAirTrajectoryLaunch {
        fn populate_skeleton_launch_info(&mut self, launch_info: &mut TrajectoryLaunchInfo) {
            self.calls
                .push(PhysicsAirTrajectoryLaunchCall::PopulateSkeleton);
            self.skeleton_input = Some(*launch_info);
            *launch_info = self.skeleton_snapshot;
        }

        fn frames_since_jump_correct(&mut self) -> i32 {
            self.calls.push(PhysicsAirTrajectoryLaunchCall::ReadFrames);
            self.frames
        }

        fn physics_air_jump_fix_frames(&mut self) -> i32 {
            self.calls.push(PhysicsAirTrajectoryLaunchCall::ReadLimit);
            self.limit
        }

        fn calculate_velocity_from_jump(
            &mut self,
            frames_since_jump_correct: i32,
        ) -> PhysicsAirVector3 {
            self.calls
                .push(PhysicsAirTrajectoryLaunchCall::CalcVelFromJump(
                    frames_since_jump_correct,
                ));
            self.jump_velocity
        }

        fn use_centre_of_mass_velocity(&mut self) -> bool {
            self.calls
                .push(PhysicsAirTrajectoryLaunchCall::ReadUseCentreOfMassVelocity);
            self.use_centre_of_mass_velocity
        }

        fn populate_com_trajectory_launch_info(&mut self, launch_info: &mut TrajectoryLaunchInfo) {
            self.calls
                .push(PhysicsAirTrajectoryLaunchCall::PopulateComTrajectory);
            *launch_info = self.com_snapshot;
        }

        fn launch(&mut self, launch_info: &mut TrajectoryLaunchInfo) {
            self.calls.push(PhysicsAirTrajectoryLaunchCall::Launch);
            self.launched = Some(*launch_info);
        }
    }

    fn marked_air_vector(word: u32) -> PhysicsAirVector3 {
        PhysicsAirVector3 {
            lanes: [
                f32::from_bits(word),
                f32::from_bits(word.wrapping_add(1)),
                f32::from_bits(word.wrapping_add(2)),
                f32::from_bits(word.wrapping_add(3)),
            ],
        }
    }

    fn launch_vector_words(info: &TrajectoryLaunchInfo) -> [[u32; 4]; 8] {
        [
            vector_bits(info.start_velocity),
            vector_bits(info.velocity_before_jump),
            vector_bits(info.animated_centre_of_mass_position),
            vector_bits(info.animated_board_position),
            vector_bits(info.board_position),
            vector_bits(info.centre_of_mass_position),
            vector_bits(info.trajectory_start_position_override),
            vector_bits(info.board_position_override),
        ]
    }

    fn marked_launch_info(first_word: u32) -> TrajectoryLaunchInfo {
        TrajectoryLaunchInfo {
            start_velocity: marked_air_vector(first_word),
            velocity_before_jump: marked_air_vector(first_word.wrapping_add(0x10)),
            animated_centre_of_mass_position: marked_air_vector(first_word.wrapping_add(0x20)),
            animated_board_position: marked_air_vector(first_word.wrapping_add(0x30)),
            board_position: marked_air_vector(first_word.wrapping_add(0x40)),
            centre_of_mass_position: marked_air_vector(first_word.wrapping_add(0x50)),
            trajectory_start_position_override: marked_air_vector(first_word.wrapping_add(0x60)),
            board_position_override: marked_air_vector(first_word.wrapping_add(0x70)),
            player_jumped: true,
            ..TrajectoryLaunchInfo::default()
        }
    }

    #[test]
    fn trajectory_launch_record_matches_pdb_layout_and_constructor() {
        use std::mem::{align_of, offset_of, size_of};

        assert_eq!(size_of::<PhysicsAirVector3>(), 0x10);
        assert_eq!(align_of::<PhysicsAirVector3>(), 0x10);
        assert_eq!(size_of::<TrajectoryLaunchInfo>(), 0x110);
        assert_eq!(align_of::<TrajectoryLaunchInfo>(), 0x10);
        assert_eq!(
            offset_of!(TrajectoryLaunchInfo, reckoning_orientation),
            0x00
        );
        assert_eq!(
            offset_of!(TrajectoryLaunchInfo, inverse_reckoning_orientation),
            0x40
        );
        assert_eq!(offset_of!(TrajectoryLaunchInfo, start_velocity), 0x80);
        assert_eq!(offset_of!(TrajectoryLaunchInfo, velocity_before_jump), 0x90);
        assert_eq!(
            offset_of!(TrajectoryLaunchInfo, animated_centre_of_mass_position),
            0xa0
        );
        assert_eq!(
            offset_of!(TrajectoryLaunchInfo, animated_board_position),
            0xb0
        );
        assert_eq!(offset_of!(TrajectoryLaunchInfo, board_position), 0xc0);
        assert_eq!(
            offset_of!(TrajectoryLaunchInfo, centre_of_mass_position),
            0xd0
        );
        assert_eq!(
            offset_of!(TrajectoryLaunchInfo, trajectory_start_position_override),
            0xe0
        );
        assert_eq!(
            offset_of!(TrajectoryLaunchInfo, board_position_override),
            0xf0
        );
        assert_eq!(offset_of!(TrajectoryLaunchInfo, cone_angle_x), 0x100);
        assert_eq!(offset_of!(TrajectoryLaunchInfo, cone_angle_z), 0x104);
        assert_eq!(offset_of!(TrajectoryLaunchInfo, time_delta), 0x108);
        assert_eq!(offset_of!(TrajectoryLaunchInfo, player_jumped), 0x10c);
        assert_eq!(
            offset_of!(TrajectoryLaunchInfo, use_trajectory_start_position_override),
            0x10d
        );
        assert_eq!(
            offset_of!(TrajectoryLaunchInfo, number_of_trajectories),
            0x10e
        );

        let default = TrajectoryLaunchInfo::default();
        assert_eq!(
            default.reckoning_orientation.map(f32::to_bits),
            AFFINE_IDENTITY_ORIENTATION.map(f32::to_bits)
        );
        assert_eq!(
            default.inverse_reckoning_orientation.map(f32::to_bits),
            AFFINE_IDENTITY_ORIENTATION.map(f32::to_bits)
        );
        assert_eq!(launch_vector_words(&default), [[0; 4]; 8]);
        assert_eq!(default.cone_angle_x.to_bits(), 0);
        assert_eq!(default.cone_angle_z.to_bits(), 0);
        assert_eq!(default.time_delta.to_bits(), 0);
        assert!(!default.player_jumped);
        assert!(!default.use_trajectory_start_position_override);
        assert_eq!(default.number_of_trajectories, 0);
    }

    #[test]
    fn physics_air_early_jump_velocity_replaces_all_lanes_and_suppresses_com_read() {
        let skeleton_snapshot = marked_launch_info(0x7f81_0100);
        let jump_velocity = PhysicsAirVector3 {
            lanes: [
                -0.0,
                f32::from_bits(0x7fc1_2345),
                f32::from_bits(0xff80_0000),
                f32::from_bits(0x7fa5_4321),
            ],
        };
        let mut collaborator = RecordingPhysicsAirTrajectoryLaunch {
            frames: -1,
            limit: 12,
            use_centre_of_mass_velocity: true,
            skeleton_snapshot,
            jump_velocity,
            ..RecordingPhysicsAirTrajectoryLaunch::default()
        };

        launch_physics_air_trajectory_selector(&mut collaborator);

        assert_eq!(
            collaborator.calls,
            vec![
                PhysicsAirTrajectoryLaunchCall::PopulateSkeleton,
                PhysicsAirTrajectoryLaunchCall::ReadFrames,
                PhysicsAirTrajectoryLaunchCall::ReadLimit,
                PhysicsAirTrajectoryLaunchCall::CalcVelFromJump(-1),
                PhysicsAirTrajectoryLaunchCall::Launch,
            ]
        );
        assert_eq!(
            launch_vector_words(&collaborator.skeleton_input.expect("constructor input")),
            [[0; 4]; 8]
        );
        let launched = collaborator.launched.expect("selector launch record");
        assert_eq!(
            vector_bits(launched.start_velocity),
            vector_bits(jump_velocity)
        );
        assert_eq!(
            launch_vector_words(&launched)[1..],
            launch_vector_words(&skeleton_snapshot)[1..]
        );
        assert!(!launched.player_jumped);
    }

    #[test]
    fn physics_air_launch_uses_strict_signed_frame_gate() {
        let cases = [
            (11, 12, true),
            (12, 12, false),
            (-1, 0, true),
            (0, 0, false),
            (i32::MIN, i32::MAX, true),
            (i32::MAX, i32::MIN, false),
        ];

        for (frames, limit, early) in cases {
            let mut collaborator = RecordingPhysicsAirTrajectoryLaunch {
                frames,
                limit,
                ..RecordingPhysicsAirTrajectoryLaunch::default()
            };
            launch_physics_air_trajectory_selector(&mut collaborator);

            assert_eq!(
                collaborator.calls,
                if early {
                    vec![
                        PhysicsAirTrajectoryLaunchCall::PopulateSkeleton,
                        PhysicsAirTrajectoryLaunchCall::ReadFrames,
                        PhysicsAirTrajectoryLaunchCall::ReadLimit,
                        PhysicsAirTrajectoryLaunchCall::CalcVelFromJump(frames),
                        PhysicsAirTrajectoryLaunchCall::Launch,
                    ]
                } else {
                    vec![
                        PhysicsAirTrajectoryLaunchCall::PopulateSkeleton,
                        PhysicsAirTrajectoryLaunchCall::ReadFrames,
                        PhysicsAirTrajectoryLaunchCall::ReadLimit,
                        PhysicsAirTrajectoryLaunchCall::ReadUseCentreOfMassVelocity,
                        PhysicsAirTrajectoryLaunchCall::Launch,
                    ]
                }
            );
        }
    }

    #[test]
    fn physics_air_late_com_snapshot_wins_and_player_jumped_is_forced_false() {
        let skeleton_snapshot = marked_launch_info(0x3f00_0100);
        let mut com_snapshot = marked_launch_info(0x7fc1_1000);
        com_snapshot.reckoning_orientation[15] = f32::from_bits(0x7fa1_2345);
        com_snapshot.number_of_trajectories = 7;
        let mut collaborator = RecordingPhysicsAirTrajectoryLaunch {
            frames: 12,
            limit: 12,
            use_centre_of_mass_velocity: true,
            skeleton_snapshot,
            com_snapshot,
            ..RecordingPhysicsAirTrajectoryLaunch::default()
        };

        launch_physics_air_trajectory_selector(&mut collaborator);

        assert_eq!(
            collaborator.calls,
            vec![
                PhysicsAirTrajectoryLaunchCall::PopulateSkeleton,
                PhysicsAirTrajectoryLaunchCall::ReadFrames,
                PhysicsAirTrajectoryLaunchCall::ReadLimit,
                PhysicsAirTrajectoryLaunchCall::ReadUseCentreOfMassVelocity,
                PhysicsAirTrajectoryLaunchCall::PopulateComTrajectory,
                PhysicsAirTrajectoryLaunchCall::Launch,
            ]
        );
        let launched = collaborator.launched.expect("selector launch record");
        assert_eq!(
            launch_vector_words(&launched),
            launch_vector_words(&com_snapshot)
        );
        assert_eq!(
            launched.reckoning_orientation[15].to_bits(),
            com_snapshot.reckoning_orientation[15].to_bits()
        );
        assert_eq!(launched.number_of_trajectories, 7);
        assert!(!launched.player_jumped);
    }

    #[test]
    fn physics_air_late_false_com_keeps_the_first_skeleton_snapshot() {
        let skeleton_snapshot = marked_launch_info(0x7fc2_1000);
        let mut collaborator = RecordingPhysicsAirTrajectoryLaunch {
            frames: 12,
            limit: 12,
            use_centre_of_mass_velocity: false,
            skeleton_snapshot,
            ..RecordingPhysicsAirTrajectoryLaunch::default()
        };

        launch_physics_air_trajectory_selector(&mut collaborator);

        assert_eq!(
            collaborator.calls,
            vec![
                PhysicsAirTrajectoryLaunchCall::PopulateSkeleton,
                PhysicsAirTrajectoryLaunchCall::ReadFrames,
                PhysicsAirTrajectoryLaunchCall::ReadLimit,
                PhysicsAirTrajectoryLaunchCall::ReadUseCentreOfMassVelocity,
                PhysicsAirTrajectoryLaunchCall::Launch,
            ]
        );
        let launched = collaborator.launched.expect("selector launch record");
        assert_eq!(
            launch_vector_words(&launched),
            launch_vector_words(&skeleton_snapshot)
        );
        assert_eq!(
            launched.reckoning_orientation.map(f32::to_bits),
            skeleton_snapshot.reckoning_orientation.map(f32::to_bits)
        );
        assert!(!launched.player_jumped);
    }

    type VectorBits = [u32; 4];

    fn vector_bits(value: PhysicsAirVector3) -> VectorBits {
        value.lanes.map(f32::to_bits)
    }

    #[derive(Clone, Debug, PartialEq)]
    enum PhysicsAirUpdateCall {
        UpdateAirCollision,
        ReadSkeletonContact,
        ReadPhysicsState,
        ReadQueryJustStarted,
        LaunchTrajectorySelector,
        UpdateTrajectorySelector,
        ReadReckoningUp,
        SuggestedLandingNormal(VectorBits),
        NormalizedAngle(VectorBits, VectorBits),
        ReadAngleLimit,
        ReadNormalBlend,
        ReadStateTimer,
        ReadSkateboardContact,
        ReadDebugEnabled,
        ReadDebugOverlay,
        ReportMissingDebugOverlay,
        CalculateBodySpinSpeed,
        UpdateAirStates(VectorBits, u32, u32, u32),
        ReadComCorrectionEnabled,
        AddComCorrection,
        AdvanceComTrajectory(u32),
        UpdateKnownAir(VectorBits),
        UpdateAnimatedSkateboard(bool),
        UpdateSkateboard,
        SetAddSkateboardError(bool),
        ReadSkateboardY,
        ReadTimeDelta,
    }

    #[allow(clippy::struct_excessive_bools)]
    #[derive(Debug)]
    struct RecordingPhysicsAirUpdate {
        skeleton_contact: bool,
        physics_state_nonzero: bool,
        query_just_started: bool,
        reckoning_up: PhysicsAirVector3,
        suggested_normal: PhysicsAirVector3,
        normalized_angle: f32,
        angle_limit: f32,
        normal_blend: f32,
        state_timer: f32,
        skateboard_contact: bool,
        debug_enabled: bool,
        debug_overlay_available: bool,
        body_spin_speed: f32,
        com_correction_enabled: bool,
        advanced_com_position: PhysicsAirVector3,
        skateboard_y: f32,
        time_delta: f32,
        calls: Vec<PhysicsAirUpdateCall>,
    }

    impl Default for RecordingPhysicsAirUpdate {
        fn default() -> Self {
            Self {
                skeleton_contact: false,
                physics_state_nonzero: true,
                query_just_started: false,
                reckoning_up: PhysicsAirVector3 {
                    lanes: [0.0, 1.0, 0.0, 0.0],
                },
                suggested_normal: PhysicsAirVector3 {
                    lanes: [0.0, 1.0, 0.0, 0.0],
                },
                normalized_angle: 0.0,
                angle_limit: DEBUG_REFERENCE_PHYSICS_AIR_NORMAL_ANGLE_LIMIT,
                normal_blend: DEBUG_REFERENCE_PHYSICS_AIR_NORMAL_BLEND,
                state_timer: 1.0,
                skateboard_contact: false,
                debug_enabled: false,
                debug_overlay_available: true,
                body_spin_speed: 2.0,
                com_correction_enabled: false,
                advanced_com_position: PhysicsAirVector3 {
                    lanes: [3.0, 4.0, 5.0, 0.0],
                },
                skateboard_y: 10.0,
                time_delta: 0.25,
                calls: Vec::new(),
            }
        }
    }

    impl PhysicsAirUpdateCollaborator for RecordingPhysicsAirUpdate {
        fn update_air_collision(&mut self) {
            self.calls.push(PhysicsAirUpdateCall::UpdateAirCollision);
        }

        fn is_skeleton_in_contact(&mut self) -> bool {
            self.calls.push(PhysicsAirUpdateCall::ReadSkeletonContact);
            self.skeleton_contact
        }

        fn physics_state_is_nonzero(&mut self) -> bool {
            self.calls.push(PhysicsAirUpdateCall::ReadPhysicsState);
            self.physics_state_nonzero
        }

        fn trajectory_query_just_started(&mut self) -> bool {
            self.calls.push(PhysicsAirUpdateCall::ReadQueryJustStarted);
            self.query_just_started
        }

        fn launch_trajectory_selector(&mut self) {
            self.calls
                .push(PhysicsAirUpdateCall::LaunchTrajectorySelector);
        }

        fn update_trajectory_selector(&mut self) {
            self.calls
                .push(PhysicsAirUpdateCall::UpdateTrajectorySelector);
        }

        fn reckoning_up_vector(&mut self) -> PhysicsAirVector3 {
            self.calls.push(PhysicsAirUpdateCall::ReadReckoningUp);
            self.reckoning_up
        }

        fn suggested_landing_normal(&mut self, fallback: PhysicsAirVector3) -> PhysicsAirVector3 {
            self.calls
                .push(PhysicsAirUpdateCall::SuggestedLandingNormal(vector_bits(
                    fallback,
                )));
            self.suggested_normal
        }

        fn normalized_angle_between(
            &mut self,
            landing_normal: PhysicsAirVector3,
            fallback_up: PhysicsAirVector3,
        ) -> f32 {
            self.calls.push(PhysicsAirUpdateCall::NormalizedAngle(
                vector_bits(landing_normal),
                vector_bits(fallback_up),
            ));
            self.normalized_angle
        }

        fn landing_normal_angle_limit(&mut self) -> f32 {
            self.calls.push(PhysicsAirUpdateCall::ReadAngleLimit);
            self.angle_limit
        }

        fn landing_normal_blend(&mut self) -> f32 {
            self.calls.push(PhysicsAirUpdateCall::ReadNormalBlend);
            self.normal_blend
        }

        fn state_timer(&mut self) -> f32 {
            self.calls.push(PhysicsAirUpdateCall::ReadStateTimer);
            self.state_timer
        }

        fn is_skateboard_in_contact(&mut self) -> bool {
            self.calls.push(PhysicsAirUpdateCall::ReadSkateboardContact);
            self.skateboard_contact
        }

        fn draw_physics_air_debug_info(&mut self) -> bool {
            self.calls.push(PhysicsAirUpdateCall::ReadDebugEnabled);
            self.debug_enabled
        }

        fn debug_overlay_is_available(&mut self) -> bool {
            self.calls.push(PhysicsAirUpdateCall::ReadDebugOverlay);
            self.debug_overlay_available
        }

        fn report_missing_debug_overlay(&mut self) {
            self.calls
                .push(PhysicsAirUpdateCall::ReportMissingDebugOverlay);
        }

        fn calculate_body_spin_speed(&mut self) -> f32 {
            self.calls
                .push(PhysicsAirUpdateCall::CalculateBodySpinSpeed);
            self.body_spin_speed
        }

        fn update_air_states(
            &mut self,
            landing_normal: PhysicsAirVector3,
            normal_blend: f32,
            target_body_spin_speed: f32,
            target_body_flip_speed: f32,
        ) {
            self.calls.push(PhysicsAirUpdateCall::UpdateAirStates(
                vector_bits(landing_normal),
                normal_blend.to_bits(),
                target_body_spin_speed.to_bits(),
                target_body_flip_speed.to_bits(),
            ));
        }

        fn add_centre_of_mass_correction_enabled(&mut self) -> bool {
            self.calls
                .push(PhysicsAirUpdateCall::ReadComCorrectionEnabled);
            self.com_correction_enabled
        }

        fn add_centre_of_mass_correction(&mut self) {
            self.calls.push(PhysicsAirUpdateCall::AddComCorrection);
        }

        fn advance_centre_of_mass_trajectory(&mut self, fixed_step: f32) -> PhysicsAirVector3 {
            self.calls.push(PhysicsAirUpdateCall::AdvanceComTrajectory(
                fixed_step.to_bits(),
            ));
            self.advanced_com_position
        }

        fn update_known_air(&mut self, centre_of_mass_position: PhysicsAirVector3) {
            self.calls
                .push(PhysicsAirUpdateCall::UpdateKnownAir(vector_bits(
                    centre_of_mass_position,
                )));
        }

        fn update_animated_skateboard(&mut self, use_fast_blend: bool) {
            self.calls
                .push(PhysicsAirUpdateCall::UpdateAnimatedSkateboard(
                    use_fast_blend,
                ));
        }

        fn update_skateboard(&mut self) {
            self.calls.push(PhysicsAirUpdateCall::UpdateSkateboard);
        }

        fn set_add_skateboard_error_to_skeleton(&mut self, enabled: bool) {
            self.calls
                .push(PhysicsAirUpdateCall::SetAddSkateboardError(enabled));
        }

        fn skateboard_transform_y(&mut self) -> f32 {
            self.calls.push(PhysicsAirUpdateCall::ReadSkateboardY);
            self.skateboard_y
        }

        fn time_delta(&mut self) -> f32 {
            self.calls.push(PhysicsAirUpdateCall::ReadTimeDelta);
            self.time_delta
        }
    }

    fn physics_air_update_state() -> PhysicsAirUpdateState {
        PhysicsAirUpdateState {
            landing_normal: PhysicsAirVector3 { lanes: [9.0; 4] },
            time_in_state: 1.5,
            max_y: 6.0,
            use_centre_of_mass_velocity: false,
            trajectory_timer: 1,
        }
    }

    #[test]
    fn physics_air_vector_preserves_the_pdb_sixteen_byte_storage() {
        assert_eq!(std::mem::size_of::<PhysicsAirVector3>(), 16);
    }

    #[test]
    fn physics_air_outer_update_preserves_full_call_and_write_order() {
        let fallback = PhysicsAirVector3 {
            lanes: [
                -0.0,
                f32::from_bits(0x7fc1_2345),
                2.0,
                f32::from_bits(0xdead_beef),
            ],
        };
        let suggested = PhysicsAirVector3 {
            lanes: [0.25, 0.75, -0.5, f32::from_bits(0x7fa1_2345)],
        };
        let com_position = PhysicsAirVector3 {
            lanes: [7.0, 8.0, 9.0, f32::from_bits(0x8000_0000)],
        };
        let mut collaborator = RecordingPhysicsAirUpdate {
            reckoning_up: fallback,
            suggested_normal: suggested,
            normalized_angle: 1.0,
            state_timer: 2.0,
            debug_enabled: true,
            debug_overlay_available: false,
            body_spin_speed: -3.5,
            com_correction_enabled: true,
            advanced_com_position: com_position,
            skateboard_y: 12.0,
            ..RecordingPhysicsAirUpdate::default()
        };
        let mut state = physics_air_update_state();
        state.use_centre_of_mass_velocity = true;

        update_physics_air(&mut state, &mut collaborator);

        assert_eq!(state.trajectory_timer, 7);
        assert_eq!(vector_bits(state.landing_normal), vector_bits(suggested));
        assert!(state.use_centre_of_mass_velocity);
        assert_eq!(state.max_y.to_bits(), 12.0_f32.to_bits());
        assert_eq!(state.time_in_state.to_bits(), 1.75_f32.to_bits());
        assert_eq!(PHYSICS_AIR_FIXED_STEP_SECONDS.to_bits(), 0x3c88_8889);
        assert_eq!(
            collaborator.calls,
            vec![
                PhysicsAirUpdateCall::UpdateAirCollision,
                PhysicsAirUpdateCall::ReadSkeletonContact,
                PhysicsAirUpdateCall::ReadPhysicsState,
                PhysicsAirUpdateCall::ReadQueryJustStarted,
                PhysicsAirUpdateCall::LaunchTrajectorySelector,
                PhysicsAirUpdateCall::UpdateTrajectorySelector,
                PhysicsAirUpdateCall::ReadReckoningUp,
                PhysicsAirUpdateCall::SuggestedLandingNormal(vector_bits(fallback)),
                PhysicsAirUpdateCall::NormalizedAngle(
                    vector_bits(suggested),
                    vector_bits(fallback),
                ),
                PhysicsAirUpdateCall::ReadAngleLimit,
                PhysicsAirUpdateCall::ReadNormalBlend,
                PhysicsAirUpdateCall::ReadStateTimer,
                PhysicsAirUpdateCall::ReadSkateboardContact,
                PhysicsAirUpdateCall::ReadDebugEnabled,
                PhysicsAirUpdateCall::ReadDebugOverlay,
                PhysicsAirUpdateCall::ReportMissingDebugOverlay,
                PhysicsAirUpdateCall::CalculateBodySpinSpeed,
                PhysicsAirUpdateCall::UpdateAirStates(
                    vector_bits(suggested),
                    DEBUG_REFERENCE_PHYSICS_AIR_NORMAL_BLEND.to_bits(),
                    (-3.5_f32).to_bits(),
                    0.0_f32.to_bits(),
                ),
                PhysicsAirUpdateCall::ReadComCorrectionEnabled,
                PhysicsAirUpdateCall::AddComCorrection,
                PhysicsAirUpdateCall::AdvanceComTrajectory(0x3c88_8889),
                PhysicsAirUpdateCall::UpdateKnownAir(vector_bits(com_position)),
                PhysicsAirUpdateCall::UpdateSkateboard,
                PhysicsAirUpdateCall::SetAddSkateboardError(true),
                PhysicsAirUpdateCall::ReadSkateboardY,
                PhysicsAirUpdateCall::ReadTimeDelta,
            ]
        );
    }

    #[test]
    fn physics_air_trajectory_gate_compares_low_word_and_short_circuits() {
        let cases = [
            (2, false, true, false, &[][..], 1),
            (
                1,
                true,
                true,
                false,
                &[PhysicsAirUpdateCall::ReadSkeletonContact][..],
                0,
            ),
            (
                1,
                false,
                false,
                false,
                &[
                    PhysicsAirUpdateCall::ReadSkeletonContact,
                    PhysicsAirUpdateCall::ReadPhysicsState,
                ][..],
                0,
            ),
            (
                1,
                false,
                true,
                true,
                &[
                    PhysicsAirUpdateCall::ReadSkeletonContact,
                    PhysicsAirUpdateCall::ReadPhysicsState,
                    PhysicsAirUpdateCall::ReadQueryJustStarted,
                ][..],
                0,
            ),
            (
                0,
                true,
                true,
                false,
                &[PhysicsAirUpdateCall::ReadSkeletonContact][..],
                -1,
            ),
            (
                -1,
                true,
                true,
                false,
                &[PhysicsAirUpdateCall::ReadSkeletonContact][..],
                -2,
            ),
            (i32::MIN, false, true, false, &[][..], i32::MAX),
        ];

        for (timer, skeleton_contact, physics_state, query_started, reads, expected_timer) in cases
        {
            let mut state = physics_air_update_state();
            state.trajectory_timer = timer;
            let mut collaborator = RecordingPhysicsAirUpdate {
                skeleton_contact,
                physics_state_nonzero: physics_state,
                query_just_started: query_started,
                ..RecordingPhysicsAirUpdate::default()
            };

            update_physics_air(&mut state, &mut collaborator);

            let actual_reads = collaborator
                .calls
                .iter()
                .filter(|call| {
                    matches!(
                        call,
                        PhysicsAirUpdateCall::ReadSkeletonContact
                            | PhysicsAirUpdateCall::ReadPhysicsState
                            | PhysicsAirUpdateCall::ReadQueryJustStarted
                    )
                })
                .cloned()
                .collect::<Vec<_>>();
            assert_eq!(actual_reads, reads);
            assert_eq!(state.trajectory_timer, expected_timer);
            assert!(
                !collaborator
                    .calls
                    .iter()
                    .any(|call| matches!(call, PhysicsAirUpdateCall::LaunchTrajectorySelector))
            );
        }
    }

    #[test]
    fn physics_air_blocked_relaunch_retries_every_frame_until_contact_clears() {
        let mut state = physics_air_update_state();
        state.trajectory_timer = 0;
        for (skeleton_contact, expected_timer, launched) in
            [(true, -1, false), (true, -2, false), (false, 7, true)]
        {
            let mut collaborator = RecordingPhysicsAirUpdate {
                skeleton_contact,
                physics_state_nonzero: true,
                query_just_started: false,
                ..RecordingPhysicsAirUpdate::default()
            };

            update_physics_air(&mut state, &mut collaborator);

            assert_eq!(state.trajectory_timer, expected_timer);
            assert_eq!(
                collaborator
                    .calls
                    .contains(&PhysicsAirUpdateCall::LaunchTrajectorySelector),
                launched
            );
        }
    }

    #[test]
    fn physics_air_outer_launch_callback_owns_its_omitted_state() {
        for use_centre_of_mass_velocity in [false, true] {
            let mut state = physics_air_update_state();
            state.use_centre_of_mass_velocity = use_centre_of_mass_velocity;
            let mut collaborator = RecordingPhysicsAirUpdate::default();

            update_physics_air(&mut state, &mut collaborator);

            assert!(
                collaborator
                    .calls
                    .contains(&PhysicsAirUpdateCall::LaunchTrajectorySelector)
            );
        }
    }

    #[test]
    fn physics_air_fp_gates_preserve_unordered_and_zero_behavior() {
        let nan = f32::from_bits(0x7fc1_2345);
        let cases = [
            (2.0, 1.0, 1.0, false, 0.0_f32.to_bits(), false),
            (
                1.0,
                1.0,
                1.0,
                false,
                DEBUG_REFERENCE_PHYSICS_AIR_NORMAL_BLEND.to_bits(),
                true,
            ),
            (
                nan,
                1.0,
                1.0,
                false,
                DEBUG_REFERENCE_PHYSICS_AIR_NORMAL_BLEND.to_bits(),
                true,
            ),
            (
                1.0,
                nan,
                1.0,
                false,
                DEBUG_REFERENCE_PHYSICS_AIR_NORMAL_BLEND.to_bits(),
                true,
            ),
            (1.0, 2.0, nan, true, 0.0_f32.to_bits(), true),
            (1.0, 2.0, 0.0, true, 0.0_f32.to_bits(), true),
            (1.0, 2.0, -0.0, true, 0.0_f32.to_bits(), true),
        ];

        for (angle, limit, state_timer, contact, expected_blend, blend_was_read) in cases {
            let mut state = physics_air_update_state();
            state.trajectory_timer = 2;
            state.use_centre_of_mass_velocity = true;
            let mut collaborator = RecordingPhysicsAirUpdate {
                normalized_angle: angle,
                angle_limit: limit,
                state_timer,
                skateboard_contact: contact,
                ..RecordingPhysicsAirUpdate::default()
            };

            update_physics_air(&mut state, &mut collaborator);

            assert_eq!(
                collaborator
                    .calls
                    .contains(&PhysicsAirUpdateCall::ReadNormalBlend),
                blend_was_read
            );
            assert_eq!(
                collaborator.calls.iter().find_map(|call| match call {
                    PhysicsAirUpdateCall::UpdateAirStates(_, blend, _, _) => Some(*blend),
                    _ => None,
                }),
                Some(expected_blend)
            );
            assert_eq!(
                collaborator
                    .calls
                    .contains(&PhysicsAirUpdateCall::ReadSkateboardContact),
                state_timer > 0.0
            );
        }
    }

    #[test]
    fn physics_air_com_flag_is_read_after_correction_and_keeps_exact_branch() {
        let mut state = physics_air_update_state();
        state.trajectory_timer = 2;
        state.use_centre_of_mass_velocity = true;
        let mut collaborator = RecordingPhysicsAirUpdate {
            com_correction_enabled: true,
            ..RecordingPhysicsAirUpdate::default()
        };

        update_physics_air(&mut state, &mut collaborator);

        let contact = collaborator
            .calls
            .iter()
            .position(|call| *call == PhysicsAirUpdateCall::ReadSkateboardContact)
            .expect("positive state timer reads contact");
        let correction = collaborator
            .calls
            .iter()
            .position(|call| *call == PhysicsAirUpdateCall::AddComCorrection)
            .expect("enabled correction runs");
        let known = collaborator
            .calls
            .iter()
            .position(|call| matches!(call, PhysicsAirUpdateCall::UpdateKnownAir(_)))
            .expect("unchanged true flag selects known-air branch after correction");
        assert!(contact < correction && correction < known);
        assert!(
            !collaborator
                .calls
                .iter()
                .any(|call| matches!(call, PhysicsAirUpdateCall::UpdateAnimatedSkateboard(_)))
        );

        let mut state = physics_air_update_state();
        state.trajectory_timer = 2;
        state.use_centre_of_mass_velocity = true;
        let mut collaborator = RecordingPhysicsAirUpdate {
            skateboard_contact: true,
            com_correction_enabled: true,
            ..RecordingPhysicsAirUpdate::default()
        };
        update_physics_air(&mut state, &mut collaborator);
        assert!(!state.use_centre_of_mass_velocity);
        let correction = collaborator
            .calls
            .iter()
            .position(|call| *call == PhysicsAirUpdateCall::AddComCorrection)
            .expect("enabled correction still runs after contact clears the flag");
        let animated = collaborator
            .calls
            .iter()
            .position(|call| *call == PhysicsAirUpdateCall::UpdateAnimatedSkateboard(false))
            .expect("unchanged false flag selects the animated branch after correction");
        assert!(correction < animated);
        assert!(
            collaborator
                .calls
                .contains(&PhysicsAirUpdateCall::UpdateAnimatedSkateboard(false))
        );
        assert!(
            !collaborator
                .calls
                .iter()
                .any(|call| matches!(call, PhysicsAirUpdateCall::AdvanceComTrajectory(_)))
        );
    }

    #[test]
    fn physics_air_debug_overlay_read_is_short_circuited() {
        let mut state = physics_air_update_state();
        state.trajectory_timer = 2;
        let mut collaborator = RecordingPhysicsAirUpdate {
            debug_enabled: false,
            debug_overlay_available: false,
            ..RecordingPhysicsAirUpdate::default()
        };

        update_physics_air(&mut state, &mut collaborator);

        assert!(
            collaborator
                .calls
                .contains(&PhysicsAirUpdateCall::ReadDebugEnabled)
        );
        assert!(
            !collaborator
                .calls
                .contains(&PhysicsAirUpdateCall::ReadDebugOverlay)
        );
        assert!(
            !collaborator
                .calls
                .contains(&PhysicsAirUpdateCall::ReportMissingDebugOverlay)
        );
    }

    #[test]
    fn physics_air_max_y_selects_new_on_ties_and_unordered_comparisons() {
        let old_nan = f32::from_bits(0x7fc1_1111);
        let new_nan = f32::from_bits(0x7fc2_2222);
        let cases = [
            (0.0, -0.0, (-0.0_f32).to_bits()),
            (-0.0, 0.0, 0.0_f32.to_bits()),
            (old_nan, 2.0, 2.0_f32.to_bits()),
            (2.0, new_nan, new_nan.to_bits()),
            (3.0, 2.0, 3.0_f32.to_bits()),
        ];

        for (old_max, new_y, expected_bits) in cases {
            let mut state = physics_air_update_state();
            state.trajectory_timer = 2;
            state.max_y = old_max;
            let mut collaborator = RecordingPhysicsAirUpdate {
                skateboard_y: new_y,
                ..RecordingPhysicsAirUpdate::default()
            };

            update_physics_air(&mut state, &mut collaborator);

            assert_eq!(state.max_y.to_bits(), expected_bits);
            let error_write = collaborator
                .calls
                .iter()
                .position(|call| *call == PhysicsAirUpdateCall::SetAddSkateboardError(true))
                .expect("error flag write is present");
            let transform = collaborator
                .calls
                .iter()
                .position(|call| *call == PhysicsAirUpdateCall::ReadSkateboardY)
                .expect("transform sample is present");
            let time_delta = collaborator
                .calls
                .iter()
                .position(|call| *call == PhysicsAirUpdateCall::ReadTimeDelta)
                .expect("late time-delta read is present");
            assert!(error_write < transform && transform < time_delta);
        }
    }

    #[test]
    fn trajectory_selector_launch_is_guarded_and_caps_the_reference_batch() {
        let mut selector = TrajectorySelectorState {
            flags: BTreeSet::from([
                TrajectorySelectorFlag::Ready,
                TrajectorySelectorFlag::GrindBonusAwarded,
                TrajectorySelectorFlag::MostVerticalGrindNormalValid,
            ]),
            most_vertical_grind_normal: [1.0, 0.0, 0.0],
            ..TrajectorySelectorState::default()
        };
        let launch = TrajectoryLaunchInfo {
            start_velocity: PhysicsAirVector3::from_xyz_with_positive_zero_w([1.0, 2.0, 3.0]),
            number_of_trajectories: 9,
            ..TrajectoryLaunchInfo::default()
        };
        let tuning = TrajectorySelectionTuning::default();
        assert!(selector.launch(launch, &tuning, |info| {
            info.start_velocity.lanes[1] += 4.0;
        }));
        assert_eq!(selector.active_trajectories, MAX_TRAJECTORY_CANDIDATES);
        assert!(selector.has(TrajectorySelectorFlag::QueryInFlight));
        assert!(!selector.has(TrajectorySelectorFlag::Ready));
        assert!(!selector.has(TrajectorySelectorFlag::GrindBonusAwarded));
        assert_eq!(selector.best_trajectory, None);
        assert_vec3_close(selector.launch_info.start_velocity.xyz(), [1.0, 6.0, 3.0]);
        assert_vec3_close(selector.most_vertical_grind_normal, WORLD_UP);
        assert!(!selector.has(TrajectorySelectorFlag::MostVerticalGrindNormalValid));

        let mut called = false;
        assert!(!selector.launch(launch, &tuning, |_| called = true));
        assert!(!called);
    }

    #[test]
    fn trajectory_initializer_builds_the_reference_velocity_fan() {
        let launch = TrajectoryLaunchInfo {
            start_velocity: PhysicsAirVector3 {
                lanes: [0.0, 0.0, 10.0, f32::from_bits(0x7fa1_2345)],
            },
            cone_angle_x: 30.0,
            cone_angle_z: 30.0,
            number_of_trajectories: 5,
            ..TrajectoryLaunchInfo::default()
        };
        let tuning = TrajectorySelectionTuning {
            cone_angle_z_vs_speed: constant_graph(1.0),
            speed_factor_minimum: 10.0,
            speed_factor_maximum: 10.0,
            ..TrajectorySelectionTuning::default()
        };

        let candidates = initialize_trajectory_candidates(&launch, &tuning);
        assert_vec3_close(candidates[0].trajectory_velocity.xyz(), [0.0, 0.0, 10.0]);
        assert_eq!(
            candidates[0].trajectory_velocity.lanes[3].to_bits(),
            0x7fa1_2345
        );
        assert_vec3_close(candidates[1].trajectory_velocity.xyz(), [0.0, 0.0, 10.0]);
        assert!(candidates[1].trajectory_velocity.lanes[3].is_nan());
        assert_vec3_close(
            candidates[2].trajectory_velocity.xyz(),
            [4.472_136, 0.0, 8.944_272],
        );
        // Independent scalar/FMA source oracle; XMVectorSin(pi) is not zero.
        assert_vec3_close(
            candidates[3].trajectory_velocity.xyz(),
            [f32::from_bits(0xb5ae_b50b), 0.0, 5.0],
        );
        assert_vec3_close(
            candidates[4].trajectory_velocity.xyz(),
            [-4.472_136, 0.0, 8.944_272],
        );
        assert_vec3_close(candidates[5].trajectory_velocity.xyz(), [0.0; 3]);
    }

    #[test]
    fn trajectory_initializer_preserves_candidate_zero_when_count_is_zero() {
        let mut launch = marked_launch_info(0x7fc3_2000);
        launch.number_of_trajectories = 0;

        let candidates =
            initialize_trajectory_candidates(&launch, &TrajectorySelectionTuning::default());

        assert_eq!(
            vector_bits(candidates[0].trajectory_velocity),
            vector_bits(launch.start_velocity)
        );
        assert_eq!(candidates[1..], [TrajectoryCandidate::default(); 6]);
    }

    #[test]
    fn trajectory_initializer_uses_pre_jump_speed_after_a_player_jump() {
        let speed_curve = PointGraph {
            x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
            y: [0.0, 2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0],
        };
        let launch = TrajectoryLaunchInfo {
            start_velocity: PhysicsAirVector3::from_xyz_with_positive_zero_w([0.0, 0.0, 10.0]),
            velocity_before_jump: PhysicsAirVector3::from_xyz_with_positive_zero_w([
                0.0, 0.0, 20.0,
            ]),
            cone_angle_z: 15.0,
            player_jumped: true,
            number_of_trajectories: 3,
            ..TrajectoryLaunchInfo::default()
        };
        let tuning = TrajectorySelectionTuning {
            cone_angle_z_vs_speed: speed_curve,
            speed_factor_minimum: 0.0,
            speed_factor_maximum: 20.0,
            ..TrajectorySelectionTuning::default()
        };

        let candidates = initialize_trajectory_candidates(&launch, &tuning);
        assert_vec3_close(candidates[1].trajectory_velocity.xyz(), [0.0, 0.0, 10.0]);
        // Source cosine(pi) polynomial leaves a positive fused residual.
        assert_vec3_close(
            candidates[2].trajectory_velocity.xyz(),
            [0.0, 0.0, f32::from_bits(0x35a0_0000)],
        );
    }

    #[test]
    fn centre_of_mass_trajectory_repositions_and_preserves_the_landing_offset() {
        let winning_trajectory = BallisticTrajectory {
            initial_position: [0.0, 1.0, 0.0],
            initial_velocity: [1.0, 2.0, 3.0],
            gravity: [0.0, -9.8, 0.0],
            duration: 2.0,
        };
        let tuning = TrajectorySelectionTuning {
            landing_centre_of_mass_scalar_vs_slope: constant_graph(0.5),
            maximum_trajectory_adjustment: 4.0,
            ..TrajectorySelectionTuning::default()
        };
        let trajectory = calculate_centre_of_mass_trajectory(
            CentreOfMassTrajectoryInput {
                winning_trajectory,
                collision_frame: 30,
                landing_normal: WORLD_UP,
                launch_centre_of_mass_position: [0.0, 2.0, 0.0],
                start_trajectory_position: [0.0, 5.0, 0.0],
                up_vector: WORLD_UP,
            },
            &tuning,
        );

        assert_vec3_close(trajectory.initial_position, [0.0, 2.0, 0.0]);
        assert_vec3_close(trajectory.initial_velocity, [1.0, 3.0, 3.0]);
        assert_vec3_close(trajectory.gravity, winning_trajectory.gravity);
        assert!((trajectory.duration - winning_trajectory.duration).abs() < f32::EPSILON);

        let unchanged = calculate_centre_of_mass_trajectory(
            CentreOfMassTrajectoryInput {
                winning_trajectory,
                collision_frame: 0,
                ..CentreOfMassTrajectoryInput::default()
            },
            &tuning,
        );
        assert_eq!(unchanged, winning_trajectory);
    }

    #[test]
    fn ballistic_trajectory_adjustment_clamps_velocity_delta() {
        let mut trajectory = BallisticTrajectory {
            initial_velocity: [1.0, 2.0, 3.0],
            ..BallisticTrajectory::default()
        };
        trajectory.adjust(60, [10.0, 0.0, 0.0], 2.0);
        assert_vec3_close(trajectory.initial_velocity, [3.0, 2.0, 3.0]);
    }

    #[test]
    fn trajectory_error_radius_selects_the_reference_chord_step() {
        let trajectory = BallisticTrajectory {
            gravity: [0.0, -8.0, 0.0],
            ..BallisticTrajectory::default()
        };
        let time_step = trajectory.time_step_for_error_radius(0.125);
        assert!((time_step - 0.125_f32.sqrt()).abs() < 1.0e-6);
    }

    #[test]
    fn trajectory_collision_time_selects_the_nearest_physics_frame() {
        let trajectory = BallisticTrajectory {
            initial_position: [1.0, 2.0, 3.0],
            initial_velocity: [2.0, 4.0, 0.0],
            gravity: [0.0, -8.0, 0.0],
            duration: 1.0,
        };
        let expected_time = 6.0 * PHYSICS_STEP_SECONDS;
        let collision_position = trajectory.position_at_time(expected_time);
        let (collision_time, collision_frame) =
            trajectory.refine_collision_time(collision_position, 0.0);
        assert!((collision_time - expected_time).abs() < 1.0e-6);
        assert_eq!(collision_frame, 6);
    }

    #[test]
    fn trajectory_ground_normal_uses_the_most_vertical_normal_cone() {
        let triangles = [
            TrajectoryGroundTriangle {
                vertices: [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]],
                surface_id: 1,
            },
            TrajectoryGroundTriangle {
                vertices: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.5, -0.866_025_4]],
                surface_id: 2,
            },
            TrajectoryGroundTriangle {
                vertices: [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                surface_id: 3,
            },
        ];
        let normal = calculate_trajectory_ground_normal([1.0, -2.0, 0.0], &triangles);
        assert_vec3_close(normal, [0.0, 0.965_925_8, 0.258_819_04]);
        assert_vec3_close(
            calculate_trajectory_ground_normal([0.0, -1.0, 0.0], &[]),
            WORLD_UP,
        );
    }

    struct AggregateTrajectoryTestWorld {
        hit: Option<TrajectorySweepHit>,
        triangles: Vec<TrajectoryGroundTriangle>,
        sweep_calls: usize,
        box_capacities: Vec<usize>,
    }

    const UPWARD_TRIANGLE: [[f32; 3]; 3] = [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]];
    const NEGATIVE_X_TRIANGLE: [[f32; 3]; 3] = [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]];
    const POSITIVE_Z_TRIANGLE: [[f32; 3]; 3] = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];

    impl TrajectoryCollisionWorld for AggregateTrajectoryTestWorld {
        type Error = std::convert::Infallible;

        fn sweep_sphere_segment(
            &mut self,
            _start: [f32; 3],
            _end: [f32; 3],
            _radius: f32,
        ) -> Result<Option<TrajectorySweepHit>, Self::Error> {
            self.sweep_calls += 1;
            Ok(self.hit.take())
        }

        fn triangles_overlapping_aabb(
            &mut self,
            _minimum: [f32; 3],
            _maximum: [f32; 3],
            capacity: usize,
        ) -> Result<Vec<TrajectoryGroundTriangle>, Self::Error> {
            self.box_capacities.push(capacity);
            Ok(self.triangles.iter().copied().take(capacity).collect())
        }
    }

    fn aggregate_trajectory_query(calculate_ground_normal: bool) -> TrajectoryPredictionQuery {
        TrajectoryPredictionQuery {
            trajectory: BallisticTrajectory {
                initial_position: [0.0, 1.0, 0.0],
                initial_velocity: [4.0, 2.0, 0.0],
                gravity: [0.0, -8.0, 0.0],
                duration: 1.0,
            },
            radius: 0.1,
            maximum_error_start: 0.125,
            maximum_error_end: 0.25,
            calculate_ground_normal,
        }
    }

    fn aggregate_test_world(
        query: TrajectoryPredictionQuery,
        collision_frame: u8,
        normal: [f32; 3],
        surface_id: u32,
        triangle_vertices: [[f32; 3]; 3],
    ) -> AggregateTrajectoryTestWorld {
        AggregateTrajectoryTestWorld {
            hit: Some(TrajectorySweepHit {
                position: query
                    .trajectory
                    .position_at_time(f32::from(collision_frame) * PHYSICS_STEP_SECONDS),
                normal,
                surface_id,
            }),
            triangles: vec![
                TrajectoryGroundTriangle {
                    vertices: triangle_vertices,
                    surface_id,
                };
                MAX_TRAJECTORY_GROUND_TRIANGLES
            ],
            sweep_calls: 0,
            box_capacities: Vec::new(),
        }
    }

    fn aggregate_result(collision_time: f32, identity: u8) -> TrajectoryPredictionResult {
        let identity_float = f32::from(identity);
        TrajectoryPredictionResult {
            position: [identity_float, 1.0, 2.0],
            normal: [3.0, identity_float, 4.0],
            ground_normal: [5.0, 6.0, identity_float],
            collision_time,
            collision_frame: i32::from(identity),
            surface_id: u32::from(identity),
        }
    }

    #[test]
    fn aggregate_trajectory_empty_and_invalid_results_keep_no_hit() {
        let query = aggregate_trajectory_query(false);
        let mut providers: [&mut dyn TrajectoryCollisionWorld<Error = std::convert::Infallible>;
            0] = [];
        assert_eq!(
            predict_ballistic_trajectory_across_providers(query, &mut providers).unwrap(),
            TrajectoryPredictionResult::default()
        );

        let mut nearest = None;
        admit_aggregate_trajectory_result(&mut nearest, aggregate_result(-1.0, 1));
        admit_aggregate_trajectory_result(&mut nearest, aggregate_result(f32::NAN, 2));
        assert_eq!(nearest, None);
    }

    #[test]
    fn aggregate_trajectory_signed_zero_ties_keep_the_first_full_record() {
        let negative_zero = aggregate_result(-0.0, 10);
        let positive_zero = aggregate_result(0.0, 20);
        let mut nearest = None;
        admit_aggregate_trajectory_result(&mut nearest, negative_zero);
        admit_aggregate_trajectory_result(&mut nearest, positive_zero);
        assert_eq!(nearest, Some(negative_zero));
        assert_eq!(
            nearest.unwrap().collision_time.to_bits(),
            (-0.0_f32).to_bits()
        );

        let mut reversed = None;
        admit_aggregate_trajectory_result(&mut reversed, positive_zero);
        admit_aggregate_trajectory_result(&mut reversed, negative_zero);
        assert_eq!(reversed, Some(positive_zero));
        assert_eq!(
            reversed.unwrap().collision_time.to_bits(),
            0.0_f32.to_bits()
        );
    }

    #[test]
    fn aggregate_trajectory_equal_time_runs_every_provider_with_independent_caches() {
        let query = aggregate_trajectory_query(true);
        let mut ground = aggregate_test_world(query, 6, [1.0, 0.0, 0.0], 0x10, UPWARD_TRIANGLE);
        let mut dmo = aggregate_test_world(query, 6, [0.0, 0.0, 1.0], 0x20, NEGATIVE_X_TRIANGLE);
        let mut vehicle =
            aggregate_test_world(query, 6, [-1.0, 0.0, 0.0], 0x30, POSITIVE_Z_TRIANGLE);
        let expected_position = query
            .trajectory
            .position_at_time(6.0 * PHYSICS_STEP_SECONDS);
        let (expected_time, expected_frame) = query
            .trajectory
            .refine_collision_time(expected_position, 0.0);

        let result = {
            let mut providers: [&mut dyn TrajectoryCollisionWorld<Error = std::convert::Infallible>;
                3] = [&mut ground, &mut dmo, &mut vehicle];
            predict_ballistic_trajectory_across_providers(query, &mut providers).unwrap()
        };

        assert_eq!(ground.sweep_calls, 1);
        assert_eq!(dmo.sweep_calls, 1);
        assert_eq!(vehicle.sweep_calls, 1);
        assert_eq!(ground.box_capacities, vec![MAX_TRAJECTORY_GROUND_TRIANGLES]);
        assert_eq!(dmo.box_capacities, vec![MAX_TRAJECTORY_GROUND_TRIANGLES]);
        assert_eq!(
            vehicle.box_capacities,
            vec![MAX_TRAJECTORY_GROUND_TRIANGLES]
        );
        assert_eq!(
            result,
            TrajectoryPredictionResult {
                position: expected_position,
                normal: [1.0, 0.0, 0.0],
                ground_normal: WORLD_UP,
                collision_time: expected_time,
                collision_frame: expected_frame,
                surface_id: 0x10,
            }
        );
    }

    #[test]
    fn aggregate_trajectory_strictly_earlier_later_provider_replaces_full_record() {
        let query = aggregate_trajectory_query(true);
        let mut ground = aggregate_test_world(query, 9, [1.0, 0.0, 0.0], 0x10, UPWARD_TRIANGLE);
        let mut dmo = aggregate_test_world(query, 6, [0.0, 1.0, 0.0], 0x20, NEGATIVE_X_TRIANGLE);
        let mut vehicle =
            aggregate_test_world(query, 3, [0.25, 0.5, 0.75], 0x30, POSITIVE_Z_TRIANGLE);
        let expected_position = query
            .trajectory
            .position_at_time(3.0 * PHYSICS_STEP_SECONDS);
        let (expected_time, expected_frame) = query
            .trajectory
            .refine_collision_time(expected_position, 0.0);

        let result = {
            let mut providers: [&mut dyn TrajectoryCollisionWorld<Error = std::convert::Infallible>;
                3] = [&mut ground, &mut dmo, &mut vehicle];
            predict_ballistic_trajectory_across_providers(query, &mut providers).unwrap()
        };

        assert_eq!(ground.sweep_calls, 1);
        assert_eq!(dmo.sweep_calls, 1);
        assert_eq!(vehicle.sweep_calls, 1);
        assert_eq!(
            result,
            TrajectoryPredictionResult {
                position: expected_position,
                normal: [0.25, 0.5, 0.75],
                ground_normal: [0.0, 0.0, 1.0],
                collision_time: expected_time,
                collision_frame: expected_frame,
                surface_id: 0x30,
            }
        );
    }

    #[test]
    fn ballistic_predictor_stops_at_the_first_swept_hit() {
        struct TestWorld {
            hit: Option<TrajectorySweepHit>,
            triangles: Vec<TrajectoryGroundTriangle>,
            sweeps: usize,
        }

        impl TrajectoryCollisionWorld for TestWorld {
            type Error = std::convert::Infallible;

            fn sweep_sphere_segment(
                &mut self,
                _start: [f32; 3],
                _end: [f32; 3],
                _radius: f32,
            ) -> Result<Option<TrajectorySweepHit>, Self::Error> {
                self.sweeps += 1;
                Ok(self.hit.take())
            }

            fn triangles_overlapping_aabb(
                &mut self,
                _minimum: [f32; 3],
                _maximum: [f32; 3],
                capacity: usize,
            ) -> Result<Vec<TrajectoryGroundTriangle>, Self::Error> {
                Ok(self.triangles.iter().copied().take(capacity).collect())
            }
        }

        let trajectory = BallisticTrajectory {
            initial_position: [0.0, 1.0, 0.0],
            initial_velocity: [4.0, 2.0, 0.0],
            gravity: [0.0, -8.0, 0.0],
            duration: 1.0,
        };
        let collision_position = trajectory.position_at_time(6.0 * PHYSICS_STEP_SECONDS);
        let mut world = TestWorld {
            hit: Some(TrajectorySweepHit {
                position: collision_position,
                normal: WORLD_UP,
                surface_id: 0x2a,
            }),
            triangles: vec![TrajectoryGroundTriangle {
                vertices: [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]],
                surface_id: 0x2a,
            }],
            sweeps: 0,
        };
        let result = predict_ballistic_trajectory(
            TrajectoryPredictionQuery {
                trajectory,
                radius: 0.1,
                maximum_error_start: 0.125,
                maximum_error_end: 0.25,
                calculate_ground_normal: true,
            },
            &mut world,
        )
        .unwrap();

        assert_eq!(world.sweeps, 1);
        assert_eq!(result.surface_id, 0x2a);
        assert_eq!(result.collision_frame, 6);
        assert!((result.collision_time - 6.0 * PHYSICS_STEP_SECONDS).abs() < 1.0e-6);
        assert_vec3_close(result.position, collision_position);
        assert_vec3_close(result.normal, WORLD_UP);
        assert_vec3_close(result.ground_normal, WORLD_UP);
    }

    #[test]
    fn trajectory_score_composes_the_reference_direct_terms_in_order() {
        let tuning = TrajectorySelectionTuning {
            landing_time_bonus: constant_graph(3.0),
            landing_force_scalar_vs_dot_velocity_normal: constant_graph(2.0),
            score_middle_bonus: 5.0,
            score_landing_force: -0.5,
            score_landing_direction: -2.0,
            ..TrajectorySelectionTuning::default()
        };
        let input = TrajectoryScoreInput {
            candidate_index: 0,
            pass: 1,
            collision_time: 1.5,
            landing_normal: WORLD_UP,
            landing_velocity: [3.0, -4.0, 0.0],
            grind_or_penalty_score: 1.0,
            surface_score: 2.0,
            transition_score: 4.0,
            ..TrajectoryScoreInput::default()
        };

        let score = calculate_trajectory_score(input, &tuning);
        assert!(score.valid_collision);
        assert!((score.time_bonus - 3.0).abs() < 1.0e-6);
        assert!((score.landing_force - 4.0).abs() < 1.0e-6);
        assert!((score.landing_direction + 2.4).abs() < 1.0e-6);
        assert!((score.middle_bonus - 5.0).abs() < 1.0e-6);
        assert!((score.total - 17.6).abs() < 1.0e-5);

        let central = calculate_trajectory_score(
            TrajectoryScoreInput {
                select_central_trajectory: true,
                ..input
            },
            &tuning,
        );
        assert!((central.middle_bonus - 1_000_000.0).abs() < f32::EPSILON);
    }

    #[test]
    fn trajectory_score_rejects_a_negative_collision_time() {
        let score = calculate_trajectory_score(
            TrajectoryScoreInput {
                collision_time: -1.0,
                ..TrajectoryScoreInput::default()
            },
            &TrajectorySelectionTuning::default(),
        );
        assert_eq!(score, TrajectoryScore::default());
    }

    #[test]
    fn trajectory_surface_score_forces_up_for_unrideable_and_do_not_align() {
        let tuning = TrajectorySelectionTuning {
            score_surface_unrideable: -100.0,
            score_surface_do_not_align: -20.0,
            ..TrajectorySelectionTuning::default()
        };
        let collision_normal = [0.25, 0.5, 0.75];
        let up_vector = [0.0, 0.8, 0.6];

        let unrideable = calculate_trajectory_surface_score(
            u32::from(PhysicsSurface::Unrideable as u8) << 7,
            collision_normal,
            up_vector,
            &tuning,
        );
        assert_eq!(unrideable.physics_surface, Some(PhysicsSurface::Unrideable));
        assert!((unrideable.score + 100.0).abs() < f32::EPSILON);
        assert_vec3_close(unrideable.collision_normal, up_vector);

        let do_not_align = calculate_trajectory_surface_score(
            u32::from(PhysicsSurface::DoNotAlign as u8) << 7,
            collision_normal,
            up_vector,
            &tuning,
        );
        assert_eq!(
            do_not_align.physics_surface,
            Some(PhysicsSurface::DoNotAlign)
        );
        assert!((do_not_align.score + 20.0).abs() < f32::EPSILON);
        assert_vec3_close(do_not_align.collision_normal, up_vector);
    }

    #[test]
    fn trajectory_surface_score_keeps_the_stair_and_default_normals() {
        let tuning = TrajectorySelectionTuning {
            score_surface_do_not_align: 12.0,
            ..TrajectorySelectionTuning::default()
        };
        let collision_normal = [0.25, 0.5, 0.75];

        let stair = calculate_trajectory_surface_score(
            u32::from(PhysicsSurface::Stair as u8) << 7,
            collision_normal,
            WORLD_UP,
            &tuning,
        );
        assert_eq!(stair.physics_surface, Some(PhysicsSurface::Stair));
        assert!((stair.score - 12.0).abs() < f32::EPSILON);
        assert_vec3_close(stair.collision_normal, collision_normal);

        let unknown =
            calculate_trajectory_surface_score(31 << 7, collision_normal, WORLD_UP, &tuning);
        assert_eq!(unknown.physics_surface, None);
        assert!(unknown.score.abs() < f32::EPSILON);
        assert_vec3_close(unknown.collision_normal, collision_normal);
    }

    #[test]
    fn trajectory_grind_penalty_uses_the_nearest_recovered_grind_geometry() {
        let tuning = TrajectorySelectionTuning {
            grind_penalty_vs_distance_to_grind: PointGraph {
                x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 1000.0],
                y: [-100.0, -90.0, -80.0, -70.0, -60.0, -50.0, -40.0, 0.0],
            },
            ..TrajectorySelectionTuning::default()
        };
        let segments = [
            GrindTestSegment {
                start: [0.0, 0.0, 0.0],
                end: [4.0, 0.0, 0.0],
            },
            GrindTestSegment {
                start: [0.0, 20.0, 0.0],
                end: [4.0, 20.0, 0.0],
            },
        ];

        let penalty =
            calculate_trajectory_grind_penalty(true, &segments, 0.5, [2.0, 3.0, 0.0], &tuning);
        assert!((penalty.distance_to_grind - 3.0).abs() < 1.0e-6);
        assert!((penalty.score + 70.0).abs() < 1.0e-6);
    }

    #[test]
    fn trajectory_grind_penalty_preserves_invalid_query_and_miss_defaults() {
        let tuning = TrajectorySelectionTuning {
            grind_penalty_vs_distance_to_grind: PointGraph {
                x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 1000.0],
                y: [-100.0, -90.0, -80.0, -70.0, -60.0, -50.0, -40.0, 0.0],
            },
            ..TrajectorySelectionTuning::default()
        };
        let segment = GrindTestSegment {
            start: [1.0, 1.0, 0.0],
            end: [2.0, 1.0, 0.0],
        };

        let miss = calculate_trajectory_grind_penalty(
            true,
            &[segment],
            -1.0,
            [100.0, 100.0, 100.0],
            &tuning,
        );
        assert!((miss.distance_to_grind - 1.0).abs() < 1.0e-6);
        assert!((miss.score + 90.0).abs() < 1.0e-6);

        for invalid in [
            calculate_trajectory_grind_penalty(false, &[segment], 0.5, [1.0, 0.0, 0.0], &tuning),
            calculate_trajectory_grind_penalty(true, &[], 0.5, [1.0, 0.0, 0.0], &tuning),
        ] {
            assert!((invalid.distance_to_grind - 1000.0).abs() < f32::EPSILON);
            assert!(invalid.score.abs() < f32::EPSILON);
        }
    }

    #[test]
    fn trajectory_grind_score_queries_around_the_ballistic_collision_position() {
        let trajectory = BallisticTrajectory {
            initial_position: [1.0, 2.0, 3.0],
            initial_velocity: [2.0, 0.0, -1.0],
            gravity: [0.0, -2.0, 0.0],
            duration: 3.0,
        };
        let tuning = TrajectorySelectionTuning {
            score_grind: 5000.0,
            grind_detection_radius: 0.5,
            ..TrajectorySelectionTuning::default()
        };

        let score = calculate_trajectory_grind_score(
            &trajectory,
            2.0,
            [9.0, 8.0, 7.0],
            &tuning,
            |query| {
                assert_vec3_close(query.minimum, [4.5, -2.5, 0.5]);
                assert_vec3_close(query.maximum, [5.5, -1.5, 1.5]);
                assert_vec3_close(query.takeoff_position, [9.0, 8.0, 7.0]);
                Some(7_u32)
            },
            |result| result == 7,
        );

        assert!(score.grind_test_results_valid);
        assert!(score.grind_bonus_awarded);
        assert!((score.score - 5000.0).abs() < f32::EPSILON);
    }

    #[test]
    fn trajectory_grind_score_distinguishes_empty_and_rejected_results() {
        let trajectory = BallisticTrajectory::default();
        let tuning = TrajectorySelectionTuning {
            score_grind: 5000.0,
            ..TrajectorySelectionTuning::default()
        };

        let empty = calculate_trajectory_grind_score(
            &trajectory,
            0.0,
            [0.0; 3],
            &tuning,
            |_| Option::<u32>::None,
            |_| unreachable!("empty queries are never analyzed"),
        );
        assert!(!empty.grind_test_results_valid);
        assert!(!empty.grind_bonus_awarded);
        assert!(empty.score.abs() < f32::EPSILON);

        let rejected = calculate_trajectory_grind_score(
            &trajectory,
            0.0,
            [0.0; 3],
            &tuning,
            |_| Some(()),
            |()| false,
        );
        assert!(rejected.grind_test_results_valid);
        assert!(!rejected.grind_bonus_awarded);
        assert!(rejected.score.abs() < f32::EPSILON);
    }

    #[test]
    fn trajectory_transition_score_preserves_vert_direction_and_apex_penalty() {
        let trajectory = BallisticTrajectory {
            initial_velocity: [0.0, 4.0, 0.0],
            gravity: [0.0, -2.0, 0.0],
            ..BallisticTrajectory::default()
        };
        assert!((trajectory.time_at_highest_position() - 2.0).abs() < 1.0e-6);
        let tuning = TrajectorySelectionTuning {
            grind_penalty_vs_distance_to_grind: constant_graph(-100.0),
            score_transition: 100.0,
            minimum_time_after_apex_for_transition: 0.5,
            ..TrajectorySelectionTuning::default()
        };
        let input = TrajectoryTransitionScoreInput {
            on_vert: true,
            transition_input: 0.5,
            ground_normal: [0.0, 0.8, 0.6],
            ground_velocity_damped: [3.0, 7.0, 4.0],
            collision_normal: [1.0, 0.0, 0.0],
            collision_time: 2.1,
            grind_or_penalty_score: 20.0,
        };

        let score = calculate_trajectory_transition_score(&trajectory, input, &tuning);
        assert!((score.direction - 1.0).abs() < f32::EPSILON);
        assert!((score.time_from_apex - 0.1).abs() < 1.0e-5);
        assert!((score.apex_penalty + 500.0).abs() < f32::EPSILON);
        assert!((score.alignment - 0.6).abs() < 1.0e-6);
        assert!((score.score + 488.0).abs() < 1.0e-5);
        assert!(!score.grind_bonus_cancelled);
        assert!((score.grind_or_penalty_score - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn trajectory_transition_score_cancels_a_reverse_vert_grind_bonus() {
        let trajectory = BallisticTrajectory {
            initial_velocity: [0.0, 4.0, 0.0],
            gravity: [0.0, -2.0, 0.0],
            ..BallisticTrajectory::default()
        };
        let tuning = TrajectorySelectionTuning {
            grind_penalty_vs_distance_to_grind: constant_graph(-100.0),
            score_transition: 100.0,
            minimum_time_after_apex_for_transition: 0.5,
            ..TrajectorySelectionTuning::default()
        };
        let score = calculate_trajectory_transition_score(
            &trajectory,
            TrajectoryTransitionScoreInput {
                on_vert: true,
                transition_input: 0.2,
                ground_normal: [0.0, 0.8, 0.6],
                ground_velocity_damped: [3.0, 7.0, 4.0],
                collision_normal: [1.0, 0.0, 0.0],
                collision_time: 2.1,
                grind_or_penalty_score: 1000.0,
            },
            &tuning,
        );
        assert!((score.direction + 1.0).abs() < f32::EPSILON);
        assert!(score.grind_bonus_cancelled);
        assert!((score.grind_or_penalty_score + 100.0).abs() < f32::EPSILON);
        assert!((score.score + 512.0).abs() < 1.0e-5);

        let off_vert = calculate_trajectory_transition_score(
            &trajectory,
            TrajectoryTransitionScoreInput {
                grind_or_penalty_score: 42.0,
                ..TrajectoryTransitionScoreInput::default()
            },
            &tuning,
        );
        assert!(off_vert.score.abs() < f32::EPSILON);
        assert!((off_vert.grind_or_penalty_score - 42.0).abs() < f32::EPSILON);
    }

    fn wall_ride_test_tuning() -> TrajectoryWallRideTuning {
        TrajectoryWallRideTuning {
            maximum_collision_normal_y: 0.5,
            maximum_ground_wall_normal_dot: 0.5,
            boost_vs_height: PointGraph {
                x: [0.0, 1.0, 2.0, 3.0],
                y: [2.0; 4],
            },
            test_distance: 2.0,
            minimum_height_from_ground: 3.0,
            maximum_landing_slope: 0.5,
            score_per_height: 10.0,
        }
    }

    #[test]
    fn trajectory_wall_ride_rejects_non_walls_and_missing_or_related_ground() {
        let mut trajectory = BallisticTrajectory::default();
        let tuning = wall_ride_test_tuning();
        let trajectory_tuning = TrajectorySelectionTuning::default();
        let not_wall = calculate_trajectory_wall_ride_score(
            &mut trajectory,
            TrajectoryWallRideInput {
                collision_normal: [0.0, 0.5, 0.0],
                ..TrajectoryWallRideInput::default()
            },
            &tuning,
            &trajectory_tuning,
            |_, _| unreachable!("non-wall contacts do not cast a ground probe"),
        );
        assert_eq!(not_wall.decision, TrajectoryWallRideDecision::NotWall);

        let input = TrajectoryWallRideInput {
            collision_normal: [1.0, 0.0, 0.0],
            ..TrajectoryWallRideInput::default()
        };
        let no_ground = calculate_trajectory_wall_ride_score(
            &mut trajectory,
            input,
            &tuning,
            &trajectory_tuning,
            |_, _| None,
        );
        assert_eq!(
            no_ground.decision,
            TrajectoryWallRideDecision::NoGroundBelow
        );

        let rejected = calculate_trajectory_wall_ride_score(
            &mut trajectory,
            input,
            &tuning,
            &trajectory_tuning,
            |_, _| {
                Some(WallRideGroundHit {
                    normal: [1.0, 0.0, 0.0],
                    ..WallRideGroundHit::default()
                })
            },
        );
        assert_eq!(
            rejected.decision,
            TrajectoryWallRideDecision::GroundNormalRejected
        );
    }

    #[test]
    fn trajectory_wall_ride_scores_and_adjusts_a_valid_contact() {
        let mut trajectory = BallisticTrajectory {
            initial_position: [0.0, 2.0, 0.0],
            initial_velocity: [1.0, 0.0, 0.0],
            ..BallisticTrajectory::default()
        };
        let tuning = wall_ride_test_tuning();
        let trajectory_tuning = TrajectorySelectionTuning {
            maximum_trajectory_adjustment: 5.0,
            ..TrajectorySelectionTuning::default()
        };
        let score = calculate_trajectory_wall_ride_score(
            &mut trajectory,
            TrajectoryWallRideInput {
                collision_time: 2.0,
                collision_frame: 60,
                collision_normal: [1.0, 0.0, 0.0],
                highest_wall_ride_collision_y: 1.0,
                ..TrajectoryWallRideInput::default()
            },
            &tuning,
            &trajectory_tuning,
            |start, end| {
                assert_vec3_close(start, [4.0, 2.0, 0.0]);
                assert_vec3_close(end, [4.0, -8.0, 0.0]);
                Some(WallRideGroundHit {
                    position: [4.0, -2.0, 0.0],
                    normal: WORLD_UP,
                })
            },
        );

        assert_eq!(score.decision, TrajectoryWallRideDecision::Adjusted);
        assert!((score.height_from_ground - 4.0).abs() < 1.0e-6);
        assert!(score.landing_slope.abs() < 1.0e-6);
        assert_vec3_close(score.trajectory_adjustment, [0.0, 2.0, 0.0]);
        assert!((score.highest_wall_ride_collision_y - 2.0).abs() < 1.0e-6);
        assert!((score.score - 40.0).abs() < 1.0e-6);
        assert_vec3_close(trajectory.initial_velocity, [1.0, 2.0, 0.0]);
    }

    #[test]
    fn trajectory_wall_ride_preserves_all_three_penalty_formulas() {
        let tuning = wall_ride_test_tuning();
        let trajectory_tuning = TrajectorySelectionTuning::default();
        let ground_hit = WallRideGroundHit {
            position: [4.0, -2.0, 0.0],
            normal: WORLD_UP,
        };
        let base_input = TrajectoryWallRideInput {
            collision_time: 2.0,
            collision_frame: 60,
            collision_normal: [1.0, 0.0, 0.0],
            ..TrajectoryWallRideInput::default()
        };

        let mut level_trajectory = BallisticTrajectory {
            initial_position: [0.0, 2.0, 0.0],
            initial_velocity: [1.0, 0.0, 0.0],
            ..BallisticTrajectory::default()
        };
        let near_grind = calculate_trajectory_wall_ride_score(
            &mut level_trajectory,
            TrajectoryWallRideInput {
                near_grind: true,
                ..base_input
            },
            &tuning,
            &trajectory_tuning,
            |_, _| Some(ground_hit),
        );
        assert_eq!(
            near_grind.decision,
            TrajectoryWallRideDecision::PenalizedNearGrind
        );
        assert!((near_grind.score + 960.0).abs() < 1.0e-6);

        let mut steep_trajectory = BallisticTrajectory {
            initial_position: [0.0, 4.0, 0.0],
            initial_velocity: [1.0, -1.0, 0.0],
            ..BallisticTrajectory::default()
        };
        let steep = calculate_trajectory_wall_ride_score(
            &mut steep_trajectory,
            base_input,
            &tuning,
            &trajectory_tuning,
            |_, _| Some(ground_hit),
        );
        assert_eq!(
            steep.decision,
            TrajectoryWallRideDecision::PenalizedApproach
        );
        assert!((steep.landing_slope - 1.0).abs() < 1.0e-6);
        assert!((steep.score + 960.0).abs() < 1.0e-6);

        let too_low = calculate_trajectory_wall_ride_score(
            &mut level_trajectory,
            base_input,
            &tuning,
            &trajectory_tuning,
            |_, _| {
                Some(WallRideGroundHit {
                    position: [4.0, 0.0, 0.0],
                    normal: WORLD_UP,
                })
            },
        );
        assert_eq!(
            too_low.decision,
            TrajectoryWallRideDecision::PenalizedTooLow
        );
        assert!((too_low.score + 998.0).abs() < 1.0e-6);
    }

    #[test]
    fn trajectory_selector_keeps_first_best_and_rejects_an_early_hit() {
        let mut selector = TrajectorySelectorState::default();
        let launch = TrajectoryLaunchInfo {
            number_of_trajectories: 3,
            ..TrajectoryLaunchInfo::default()
        };
        let tuning = TrajectorySelectionTuning {
            minimum_valid_collision_time: 0.2,
            ..TrajectorySelectionTuning::default()
        };
        assert!(selector.launch(launch, &tuning, |_| {}));
        let candidates = [
            TrajectoryCandidate {
                score: 4.0,
                collision_time: 0.1,
                collision_normal: [0.0, 0.5, 0.5],
                landing_normal: WORLD_UP,
                ..TrajectoryCandidate::default()
            },
            TrajectoryCandidate {
                score: 4.0,
                collision_time: 1.0,
                landing_normal: WORLD_UP,
                ..TrajectoryCandidate::default()
            },
            TrajectoryCandidate {
                score: 3.0,
                collision_time: 1.0,
                landing_normal: WORLD_UP,
                ..TrajectoryCandidate::default()
            },
        ];
        let ready = selector.update(
            Some(TrajectoryPredictionBatch {
                candidates: &candidates,
                grind_bonus_awarded: false,
            }),
            WORLD_UP,
            &tuning,
        );

        assert!(!ready);
        assert_eq!(selector.best_trajectory, Some(0));
        assert!(!selector.has(TrajectorySelectorFlag::QueryInFlight));
        assert!(selector.has(TrajectorySelectorFlag::SuggestedLandingNormalValid));
        assert_vec3_close(selector.suggested_landing_normal, [0.0, 0.5, 0.5]);
    }

    #[test]
    fn trajectory_selector_launches_and_completes_the_reference_second_pass() {
        let mut selector = TrajectorySelectorState::default();
        let launch = TrajectoryLaunchInfo {
            number_of_trajectories: 2,
            ..TrajectoryLaunchInfo::default()
        };
        let tuning = TrajectorySelectionTuning {
            minimum_valid_collision_time: 0.2,
            minimum_normal_delta_for_second_pass: 0.5,
            cone_angle_x_second_pass: 0.25,
            cone_angle_z_second_pass: 0.75,
            ..TrajectorySelectionTuning::default()
        };
        assert!(selector.launch(launch, &tuning, |_| {}));
        let first_pass = [
            TrajectoryCandidate {
                score: 1.0,
                collision_time: 1.0,
                landing_normal: [1.0, 0.0, 0.0],
                ..TrajectoryCandidate::default()
            },
            TrajectoryCandidate {
                trajectory_velocity: PhysicsAirVector3 {
                    lanes: [3.0, 7.0, 2.0, f32::from_bits(0x7fc1_2345)],
                },
                score: 2.0,
                collision_time: 1.0,
                landing_normal: [1.0, 0.0, 0.0],
                ..TrajectoryCandidate::default()
            },
        ];
        assert!(selector.update(
            Some(TrajectoryPredictionBatch {
                candidates: &first_pass,
                grind_bonus_awarded: false,
            }),
            WORLD_UP,
            &tuning,
        ));
        assert_eq!(selector.pass, 2);
        assert!(selector.has(TrajectorySelectorFlag::Ready));
        assert!(selector.has(TrajectorySelectorFlag::QueryInFlight));
        assert_eq!(selector.best_trajectory, Some(1));
        assert_vec3_close(selector.launch_info.start_velocity.xyz(), [3.0, 7.0, 2.0]);
        assert_eq!(
            selector.launch_info.start_velocity.lanes[3].to_bits(),
            0x7fc1_2345
        );
        assert!((selector.launch_info.cone_angle_x - 0.25).abs() < f32::EPSILON);
        assert!((selector.launch_info.cone_angle_z - 0.75).abs() < f32::EPSILON);

        assert!(selector.update(None, WORLD_UP, &tuning));
        assert!(!selector.has(TrajectorySelectorFlag::HasJustChanged));
        let second_pass = [TrajectoryCandidate {
            score: 5.0,
            collision_time: 1.0,
            landing_normal: WORLD_UP,
            ..TrajectoryCandidate::default()
        }; 2];
        assert!(selector.update(
            Some(TrajectoryPredictionBatch {
                candidates: &second_pass,
                grind_bonus_awarded: false,
            }),
            WORLD_UP,
            &tuning,
        ));
        assert_eq!(selector.pass, 3);
        assert!(selector.has(TrajectorySelectorFlag::HasJustChanged));
        assert!(!selector.has(TrajectorySelectorFlag::QueryInFlight));
        assert_eq!(selector.best_trajectory, Some(0));
    }

    #[test]
    fn degenerate_body_forward_disables_only_authored_xz_adjustment() {
        let curves = JumpPhysicsCurves {
            vertical_response: constant_graph(1.0),
            jump_y_scalar_vs_ground_normal_y: constant_graph(1.0),
            jump_speed_scalar_vs_angle: constant_graph(10.0),
            min_height_vs_speed: constant_graph(1.0),
            max_height_vs_speed: constant_graph(1.0),
        };
        let solution = calculate_ground_jump(
            GroundJumpInput {
                vertical: GroundJumpVerticalInput {
                    surface_speed: 0.0,
                    jump_strength: 0.0,
                    gravity: -8.0,
                    jump_normal: [0.0, 1.0, 0.0],
                    centre_of_mass: [0.0; 3],
                    ground_position: [0.0; 3],
                    reckoning_up: [0.0, 1.0, 0.0],
                },
                current_velocity: [1.0, 0.0, 2.0],
                prepared_jump_velocity: [1.0, 0.0, 2.0],
                maximum_velocity_error_squared: 7.0,
                body_z_axis: [0.0, 1.0, 0.0],
                jump_adjustment_x: 1.0,
                jump_adjustment_z: 1.0,
            },
            GroundJumpTuning {
                height_curve_reference_speed: 1.0,
                low_speed_jump_height: 0.5,
                minimum_jump_height: 0.5,
                maximum_jump_height: 0.5,
                use_vertical_response_curve: false,
                minimum_vertical_response: 0.1,
                maximum_horizontal_compensation: 1.5,
                forward_adjustment_scale: 0.18,
                sideways_adjustment_scale: 0.2,
            },
            &curves,
        );

        assert_vec3_close(solution.body_right_axis, [0.0; 3]);
        assert_vec3_close(solution.body_forward_axis, [0.0; 3]);
        assert_vec3_close(solution.new_velocity, [1.0, 8.0_f32.sqrt(), 2.0]);
    }

    fn assert_vec3_close(actual: [f32; 3], expected: [f32; 3]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!((actual - expected).abs() < 1.0e-6);
        }
    }

    fn marked_skateboard_force(marker: f32) -> SkateboardForce {
        SkateboardForce {
            world_space_force: [marker, 0.0, 0.0],
            local_space_position: [0.0, marker, 0.0],
        }
    }

    fn skateboard_force_bits(force: SkateboardForce) -> ([u32; 3], [u32; 3]) {
        (
            force.world_space_force.map(f32::to_bits),
            force.local_space_position.map(f32::to_bits),
        )
    }

    fn constant_graph<const N: usize>(value: f32) -> PointGraph<N> {
        PointGraph {
            x: [0.0; N],
            y: [value; N],
        }
    }

    fn flat_ground_jump_solution() -> GroundJumpSolution {
        let curves = JumpPhysicsCurves {
            vertical_response: constant_graph(1.0),
            jump_y_scalar_vs_ground_normal_y: constant_graph(1.5),
            jump_speed_scalar_vs_angle: constant_graph(10.0),
            min_height_vs_speed: constant_graph(1.0),
            max_height_vs_speed: constant_graph(1.0),
        };
        calculate_ground_jump(
            GroundJumpInput {
                vertical: GroundJumpVerticalInput {
                    surface_speed: 0.0,
                    jump_strength: 0.5,
                    gravity: -10.0,
                    jump_normal: [0.0, 1.0, 0.0],
                    centre_of_mass: [0.0; 3],
                    ground_position: [0.0; 3],
                    reckoning_up: [0.0, 1.0, 0.0],
                },
                current_velocity: [2.0, 0.0, 3.0],
                prepared_jump_velocity: [2.0, 0.0, 3.0],
                maximum_velocity_error_squared: DEFAULT_MAXIMUM_JUMP_VELOCITY_ERROR_SQUARED,
                body_z_axis: [0.0, 0.0, 1.0],
                jump_adjustment_x: 1.0,
                jump_adjustment_z: 1.0,
            },
            GroundJumpTuning {
                height_curve_reference_speed: 1.0,
                low_speed_jump_height: 0.8,
                minimum_jump_height: 0.8,
                maximum_jump_height: 0.8,
                use_vertical_response_curve: true,
                minimum_vertical_response: 0.1,
                maximum_horizontal_compensation: 1.5,
                forward_adjustment_scale: 0.18,
                sideways_adjustment_scale: 0.2,
            },
            &curves,
        )
    }

    #[test]
    fn processed_input_reset_and_handoff_follow_reference_stages() {
        let mut animation = AnimationPhysicsInput::reset(1);
        animation.compression = 0.65;
        animation.truck_tightness = 0.3;
        animation.wheel_hardness = 0.7;
        animation.flags.insert(AnimationPhysicsFlag::BoardFlipped);
        animation.flags.insert(AnimationPhysicsFlag::SwitchStance);
        animation
            .scalar_attributes
            .push(ScalarAnimationAttribute::new("Spin", 0.25));

        let mut processed = ProcessedPhysicsInput::reset();
        assert!((processed.gravity + 9.8).abs() < f32::EPSILON);
        assert_eq!(processed.frames_since_jump, 100);
        assert_eq!(processed.frames_since_correct_jump, 1_000);
        assert_eq!(processed.physics_state, PhysicsState::Invalid);
        processed.process_animation_handoff(&animation, AnimationPhysicsTuning::default());

        assert!((processed.compression - 0.65).abs() < f32::EPSILON);
        assert!((processed.truck_tightness - 0.3).abs() < f32::EPSILON);
        assert!((processed.wheel_hardness - 0.7).abs() < f32::EPSILON);
        assert!(processed.board_flipped);
        assert!(processed.riding_switch);
        assert!((processed.animation.spin + 0.25).abs() < f32::EPSILON);
    }
}
