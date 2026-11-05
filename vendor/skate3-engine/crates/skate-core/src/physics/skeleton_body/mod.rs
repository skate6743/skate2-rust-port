//! Physical skater assembly and its postphysics observations.
//! The body has 26 parts; animation and COM weighting use its first 24.
mod record;
pub use record::SkeletonPhysicalRecord;
mod construction;
pub use construction::{
    BoneSettings, HatGeometry, SkeletonBodyDefinition, SkeletonBodySettings, SkeletonPart,
};
mod runtime;
pub use runtime::SkeletonBody;
mod joints;
pub use joints::{JointBone, JointSettings, SkeletonJoint, SkeletonJointSettings, SkeletonJoints};
mod drive_dynamics;
pub use drive_dynamics::{
    AnimationDriveSettings, BoneDriveDynamics, BoneDriveSettings, DriveInterpolation,
};
mod drive_frames;
pub use drive_frames::{bone_drive_frames, prepare_bone_drive_frames};
mod targets;
pub use targets::{SkeletonTargets, TARGET_COUNT};
mod target_update;
pub use target_update::{ExtraTargetPositions, SkeletonTargetInput, SkeletonTargetUpdate};
mod drives;
pub use drives::{
    BoneDrives, SkeletonDriveBatch, SkeletonDriveIdentity, SkeletonDriveSettings, SkeletonDrives,
};
mod collision_mode;
pub use collision_mode::{SkeletonCollisionMode, SkeletonCollisionSettings, SkeletonPartCollision};
mod collision_feedback;
mod collision_filter;
mod collision_update;
mod collision_vector;
pub use collision_feedback::{
    BoneContact, CollisionResponseSettings, ContactPlane, ContactRegion, SkeletonCollisionFeedback,
    SkeletonCollisionInput, SkeletonContactBody, SkeletonContactFlags, SkeletonContactReport,
    SkeletonFeedbackSettings, SpecificContact, collision_response,
};
mod errors;
pub use errors::{SkeletonNormalError, SkeletonPoseErrors};

pub const PART_COUNT: usize = 26;
pub const ANIMATION_PART_COUNT: usize = 24;
pub const JOINT_COUNT: usize = 22;
#[cfg(test)]
#[path = "tests/collision_feedback.rs"]
mod collision_feedback_tests;
