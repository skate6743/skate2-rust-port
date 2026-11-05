//! Readable camera implementation candidates. Prior generated-code validation
//! is withdrawn; native correctness and full gameplay integration are unverified.

mod anchors;
mod angle_tracker;
mod avoidance;
mod compass;
mod compass_movement;
mod compass_orbit;
mod direction;
mod drop_predictor;
mod frame_composer;
mod look_input;
mod manager;
mod manager_air;
mod manager_blends;
mod manager_controls;
mod manager_frame;
mod manager_incline;
mod manager_output;
mod manager_rig;
mod manager_state;
mod manager_subject;
mod manual_cam;
mod orientation_math;
mod path_evaluator;
mod path_prediction;
mod positioner;
mod rig;
mod rig_framing;
mod rig_initialization;
mod rig_orientation;
mod rig_positioning;
mod rig_tracking;
mod shake;
mod shake_curve;
mod shake_update;
mod shot;
mod shot_manager;
mod shot_orientation;
mod slow_motion;
mod subject;
mod subject_pose;
mod tracker;
mod trajectory_query;
mod vector_tracker;

pub use anchors::{AnchorInputs, AnchorState, Anchors};
pub use angle_tracker::{AngleTracker, normalize_angle};
pub use avoidance::{AvoidancePath, AvoidanceSettings, AvoidanceSubject, RigAvoidance};
pub use compass::{Compass, CompassInputs, CompassPoseInputs, CompassSettings};
pub use direction::direction_to_angles;
pub use drop_predictor::{DropCollisionProvider, DropPredictor, DropSettings};
pub use frame_composer::{FrameComposer, FrameSettings, FrameSubject};
pub use look_input::{LookInput, LookSettings};
pub use manager::{CameraMan, ManagerSettings};
pub use manager_frame::CameraFrame;
pub use manager_state::ManagerState;
pub use manager_subject::ManagerSubject;
pub use manual_cam::{ManualCam, ManualCamMatrix, ManualCamSettings};
pub use path_evaluator::{MovingObstacleProvider, PathEvaluator, TrajectoryCollisionRequest};
pub use path_prediction::{PathObstacle, PredictionPath, candidate_collision_time};
pub use positioner::{
    FatLine, FatLineResult, Positioner, PositionerCollisionProvider, PositionerConfig,
    position_from_angles, project_hit,
};
pub use rig::{Breadcrumbs, Rig, RigFields, RigSettings};
pub use rig_framing::{RigFraming, RigMode, RigModeSubject};
pub use rig_orientation::{OrientationSettings, OrientationTrackerSettings, RigOrientation};
pub use rig_positioning::{
    DistanceTrackingSettings, RigPositioning, RigPositioningSettings, RigPositioningSubject,
};
pub use rig_tracking::{
    AnchorRigTracking, AnchorTrackingSettings, AnchorTrackingSubject, AngleTrackingSettings,
    AngularRigTracking,
};
pub use rig_tracking::{ReferenceHeightSubject, ReferenceHeightTracking};
pub use shake::{ShakeEffect, ShakeSamples, ShakeSettings};
pub use shot::{Shot, blend_interval, filter_blend_value, interpolate_float};
pub use shot_manager::{ShotDatabase, ShotDefinition, ShotEnvironment, ShotManager, ShotPlacement};
pub use slow_motion::{SimulationRateRequest, SlowMotionController, SlowMotionSettings};
pub use subject::{ReferencePointInputs, Subject};
pub use subject_pose::{PublishedSubjectPose, SubjectPoseInputs, SubjectPosePublisher};
pub use tracker::{ScalarTracker, ScalarTrackerParameters};
pub use trajectory_query::TrajectoryQuery;
pub use vector_tracker::{VectorTracker, clamp_magnitude};
