//! Selected input and board-controller cvar references, linked labels and startup serializer.
//! Static initializers82f56b88/82f57230..7390; Link82a89ca0/82a8a010;
//! manager Init82a8ac38. Runtime references and captured targets share ownership.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained cvar services await production boot ownership; corpus integration is exercised under tests"
    )
)]
#[path = "../../animation/graphs/motion/callbacks/actor_motion_body_spin_attributes.rs"]
pub(crate) mod body_spin_motion;
#[path = "../../animation/graphs/motion/callbacks/actor_motion_body_tilt_attributes.rs"]
pub(crate) mod body_tilt;
#[path = "../../animation/graphs/motion/callbacks/actor_motion_crouching_attributes.rs"]
pub(crate) mod crouching;
#[path = "input_cvar_dist_com_to_board.rs"]
mod dist_com_to_board;
#[path = "../../animation/graphs/motion/callbacks/actor_motion_pumping_attributes.rs"]
pub(crate) mod motion_pumping;
#[path = "../../animation/graphs/motion/callbacks/actor_motion_power_sliding_attributes.rs"]
pub(crate) mod power_sliding;
#[path = "../../animation/graphs/motion/callbacks/actor_motion_set_turning_attributes.rs"]
pub(crate) mod set_turning;
use crate::action_graph_input_listener::fill::{
    ActionInputBoolCvar, ActionInputFillCvars, ActionInputFloatCvar,
};
use std::sync::{Arc, Mutex};

#[path = "input_cvar_breadcrumbs.rs"]
pub(crate) mod breadcrumbs;
#[path = "input_cvar_collision_status.rs"]
mod collision_status;
#[path = "input_cvar_current_animation.rs"]
pub(crate) mod current_animation;
#[path = "../../physics/player/lifecycle/physical_player_physics_damped_com_cvars.rs"]
pub(crate) mod damped_com;
#[path = "input_cvar_grind_states.rs"]
pub(crate) mod grind_states;
#[path = "input_cvar_grind_update.rs"]
pub(crate) mod grind_update;
#[path = "input_cvar_ground_skateboard.rs"]
pub(crate) mod ground_skateboard;
#[path = "../../physics/player/ground/physical_player_ground_wipeout_cvars.rs"]
pub(crate) mod ground_wipeout;
#[path = "../../physics/player/wipeout/physical_player_post_wipeout_head_tracking_runtime.rs"]
pub(crate) mod head_tracking_live;
#[path = "input_cvar_ini.rs"]
mod ini;
#[path = "../../physics/player/lifecycle/physical_player_physics_landing_quality_cvars.rs"]
pub(crate) mod landing_quality;
#[path = "input_cvar_living_world_dmo.rs"]
pub(crate) mod living_world_dmo;
#[path = "input_cvar_living_world_grind.rs"]
pub(crate) mod living_world_grind;
#[path = "input_cvar_offboard_air_state.rs"]
pub(crate) mod offboard_air_state;
#[path = "input_cvar_pumping.rs"]
pub(crate) mod pumping;
#[path = "input_cvar_pushing.rs"]
pub(crate) mod pushing;
#[path = "../input_recognizer_cvars.rs"]
pub(crate) mod recognizer;
#[path = "input_cvar_skateboard_hold.rs"]
pub(crate) mod skateboard_hold;
#[path = "../../physics/player/lifecycle/physical_player_physics_stairs_cvars.rs"]
mod stairs;
#[cfg(test)]
#[path = "input_cvar_runtime_tests.rs"]
mod tests;
#[path = "input_cvar_toolkit.rs"]
pub(crate) mod toolkit;
#[path = "input_cvar_replay_editor.rs"]
mod replay_editor;

#[derive(Clone, Copy, Debug)]
enum Value {
    Float(f32),
    Boolean(bool),
    Vector2([f32; 4]),
    Enumeration {
        value: i32,
        tags: &'static [(&'static str, i32)],
    },
    Integer {
        value: i32,
        minimum: i32,
        maximum: i32,
    },
}
#[derive(Clone, Copy)]
struct Definition {
    section: &'static str,
    label: &'static str,
    title: &'static str,
    initial: Value,
}
// ParseEnumTags82a87dc0: these selected initializer tags have sequential values.
const GESTURE_TAGS: &[(&str, i32)] = &[
    ("Sky", 0),
    ("What", 1),
    ("Snap", 2),
    ("ThumbsDown", 3),
    ("PumpHigh", 4),
    ("Fonz", 5),
    ("Bruce", 6),
    ("Shrug", 7),
    ("PumpLow", 8),
    ("Devil", 9),
    ("FU", 10),
    ("ThrowArms", 11),
    ("DoubleGun", 12),
    ("Peace", 13),
    ("Shaka", 14),
    ("Spock", 15),
    ("AirGuitar", 16),
    ("Airplane", 17),
    ("Flex", 18),
    ("Boxing", 19),
    ("Clothesline", 20),
    ("Paper", 21),
    ("Rock", 22),
    ("Scissors", 23),
    ("Dunno", 24),
    ("FingerPoint", 25),
    ("FingerWag", 26),
    ("Fists", 27),
    ("FlipTable", 28),
    ("Freedom", 29),
    ("GetOuttaHere", 30),
    ("Push", 31),
    ("RaiseTheRoof", 32),
    ("Wings", 33),
    ("Axe", 34),
];
// kForcedGrindSurface initializer82f907c0 tag string, sequential values.
const GRIND_SURFACE_TAGS: &[(&str, i32)] = &[
    ("as detected", 0),
    ("smooth", 1),
    ("rough", 2),
    ("slow", 3),
    ("slippery", 4),
    ("very slow", 5),
    ("unrideable", 6),
    ("doNotAlign", 7),
];
// Stable API indices; source initializer order is recorded separately below.
static DEFINITIONS: [Definition; 1286] = [
    Definition {
        section: "Animation",
        label: "kUseSprintButtonInput",
        title: "Use Sprint Button Input",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Controller",
        label: "kUseTriggersForExplicitWipeout",
        title: "Use triggers for explicit wipeout",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Controller",
        label: "kUseReleaseForWipeout",
        title: "Also use release for wipeout",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Controller",
        label: "kTimeToConsiderWipeout",
        title: "Time to consider wipeout",
        initial: Value::Float(f32::from_bits(0x3e4c_cccd)),
    },
    Definition {
        section: "Controller",
        label: "kNewPushAllowance",
        title: "New Push Allowance Time",
        initial: Value::Float(f32::from_bits(0x3e99_999a)),
    },
    Definition {
        section: "Controller",
        label: "kNewToggleOffboardAllowance",
        title: "New Toggle Offboard Allowance Time",
        initial: Value::Float(f32::from_bits(0x3cf5_c28f)),
    },
    Definition {
        section: "Physics_Skateboard_Controller",
        label: "kSKC_UseHandDrives",
        title: "Use Hand Drives",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Input",
        label: "kDisplayLeftStickInput",
        title: "Display Left Stick Input",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Input",
        label: "kDisplayRightStickInput",
        title: "Display Right Stick Input",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Input",
        label: "kDisplayLeftStickGestures",
        title: "Display Left Stick Gesture",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Input",
        label: "kDisplayRightStickGestures",
        title: "Display Right Stick Gesture",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Input",
        label: "kInputHistoryDeadZone",
        title: "Dead Zone",
        initial: Value::Vector2([
            f32::from_bits(0x3dcc_cccd),
            f32::from_bits(0x3dcc_cccd),
            0.0,
            0.0,
        ]),
    },
    Definition {
        section: "Input",
        label: "kFastestGestureRatio",
        title: "Fastest Ratio",
        initial: Value::Float(f32::from_bits(0x3fe0_0000)),
    },
    Definition {
        section: "Input",
        label: "kSlowestGestureRatio",
        title: "Slowest Ratio",
        initial: Value::Float(f32::from_bits(0x408c_cccd)),
    },
    Definition {
        section: "Input",
        label: "kMinError",
        title: "Minimum Error",
        initial: Value::Float(f32::from_bits(0x3e19_999a)),
    },
    Definition {
        section: "Input",
        label: "kMaxError",
        title: "Maximum Error",
        initial: Value::Float(f32::from_bits(0x3e19_999a)),
    },
    Definition {
        section: "Animation",
        label: "kUseVBR",
        title: "Use VBR Codec",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Manual",
        label: "kShowVelRemoval",
        title: "Show Vel Removal",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Animation",
        label: "kUseJobsForSkaterAnimUpdate",
        title: "Use Jobs for Skater Anim Update",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_States_StateChanger",
        label: "kFallThroughWorldResetFrames",
        title: "Fall Through World Reset Frames",
        initial: Value::Integer {
            value: 400,
            minimum: 0,
            maximum: 1000,
        },
    },
    Definition {
        section: "State",
        label: "kbUseGeneratedStateGraph",
        title: "Use Generated State Graph",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "State",
        label: "kbTestDeterminism",
        title: "Use Ghost To Test Determinism",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_States_StateChanger",
        label: "kDisableSkitching",
        title: "kDisableSkitching",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Skeleton",
        label: "kSB_UseRagdollJointLimits",
        title: "Use Ragdoll Joint Limits",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Cheats",
        label: "kBouncyRagdollCheat",
        title: "Bouncy Ragdoll",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Cheats",
        label: "kBouncyRagdollCheatRestitution",
        title: "Bouncy Ragdoll Amount",
        initial: Value::Float(f32::from_bits(0x3f7d_70a4)),
    },
    Definition {
        section: "Animation_BlendCullThresh",
        label: "kSkaterLowBlendCullThresh",
        title: "Skater Low BlendCullThresh",
        initial: Value::Float(f32::from_bits(0x3c23_d70a)),
    },
    Definition {
        section: "Animation_BlendCullThresh",
        label: "kSkaterHighBlendCullThresh",
        title: "Skater High BlendCullThresh",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    Definition {
        section: "Physics_Skateboard_Drives",
        label: "kSkateboardSoftDriveDisp",
        title: "SoftDrive Disp",
        initial: Value::Float(20000.0),
    },
    Definition {
        section: "Physics_Skateboard_Drives",
        label: "kSkateboardSoftDriveStr",
        title: "SoftDrive Str",
        initial: Value::Float(10.0),
    },
    Definition {
        section: "Physics_States_PhysicsAir",
        label: "kbDrawPhysicsAirDebugInfo",
        title: "kbDrawPhysicsAirDebugInfo",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Conditioners_Grinds",
        label: "kMinApproachNonGrindFrames",
        title: "Min Approach Non Grind Frames",
        initial: Value::Integer {
            value: 30,
            minimum: 0,
            maximum: 60,
        },
    },
    Definition {
        section: "Physics_Wipeout",
        label: "kUpVelocityScaleLower",
        title: "Lower Body Scale vel Up ",
        initial: Value::Float(0.0),
    },
    Definition {
        section: "Physics_Wipeout",
        label: "kUpVelocityScaleUpper",
        title: "UpperBody Scale vel Up",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    Definition {
        section: "Physics_Wipeout",
        label: "kMaxSkeletonPartVelocity",
        title: "Max Skeleton Part Velocity",
        initial: Value::Float(20.0),
    },
    Definition {
        section: "Animation_MovingObjects",
        label: "kMOAngleMaxDelta",
        title: "Angle Max Delta per Tick",
        initial: Value::Float(2.0),
    },
    Definition {
        section: "Physics_Grinds_Balance",
        label: "kLeanMinNonGrindFrames",
        title: "Lean Min Non Grind Frames",
        initial: Value::Integer {
            value: 20,
            minimum: 0,
            maximum: 100,
        },
    },
    Definition {
        section: "Physics_Grinds_Balance",
        label: "kCurbExitStartTime",
        title: "Curb Exit Start Time",
        initial: Value::Float(2.0),
    },
    Definition {
        section: "Animation_Gestures",
        label: "kEnableDebugGestures",
        title: "Enable Debug Gestures",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Animation_Gestures",
        label: "kAnimUpGesture",
        title: "Up Gesture",
        initial: Value::Enumeration {
            value: 34,
            tags: GESTURE_TAGS,
        },
    },
    Definition {
        section: "Animation_Gestures",
        label: "kAnimDownGesture",
        title: "Down Gesture",
        initial: Value::Enumeration {
            value: 33,
            tags: GESTURE_TAGS,
        },
    },
    Definition {
        section: "Animation_Gestures",
        label: "kAnimLeftGesture",
        title: "Left Gesture",
        initial: Value::Enumeration {
            value: 32,
            tags: GESTURE_TAGS,
        },
    },
    Definition {
        section: "Animation_Gestures",
        label: "kAnimRightGesture",
        title: "Right Gesture",
        initial: Value::Enumeration {
            value: 31,
            tags: GESTURE_TAGS,
        },
    },
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kEnableRepushingTimeLimit",
        title: "Enable Repushing Time Limit",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Animation_Wipeouts",
        label: "kDebugWipeoutConditions",
        title: "Debug Wipeout Conditions",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Animation_Locomotion",
        label: "kTimeToLandToWipeout",
        title: "kTimeToLandToWipeout",
        initial: Value::Float(1.4),
    },
    Definition {
        section: "Animation_Locomotion",
        label: "kMinTimeInAirToWipeOut",
        title: "kMinTimeInAirToWipeOut",
        initial: Value::Float(0.8),
    },
    Definition {
        section: "Animation_Locomotion",
        label: "kBezierInfluencePercentage",
        title: "Bezier Influence Percentage",
        initial: Value::Float(f32::from_bits(0x3ea8_f5c3)),
    },
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kSA_UseWorldYForNormal",
        title: "UseWorldY Normal",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Animation_Locomotion",
        label: "kLocoWipeoutDelta",
        title: "Wipeout distance delta",
        initial: Value::Float(f32::from_bits(0x3f33_3333)),
    },
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kFeetLineTestRadius",
        title: "LineTestRadius",
        initial: Value::Float(f32::from_bits(0x3cf5_c28f)),
    },
    Definition {
        section: "Render_Skaters",
        label: "kSkaterFadeInTime",
        title: "Fade-in time (seconds)",
        initial: Value::Float(1.0),
    },
    Definition {
        section: "Render_Skaters",
        label: "kSkaterFadeOutTime",
        title: "Fade-out time (seconds)",
        initial: Value::Float(1.0),
    },
    Definition {
        section: "Physics_States_Skitching",
        label: "kTimeToAvoidVehicleAfterSkitching",
        title: "TimeToAvoidVehicle",
        initial: Value::Float(f32::from_bits(0x4000_0000)),
    },
    Definition {
        section: "Physics_Skateboard",
        label: "kGroundMaxCosAngleDelta",
        title: "Cos of max angle change",
        initial: Value::Float(f32::from_bits(0x3f6f_5c29)),
    },
    Definition {
        section: "Physics_Skateboard",
        label: "kGroundProportionalBlend",
        title: "Blend to extreme ground normal",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    Definition {
        section: "Physics_Skateboard",
        label: "kDrawJump",
        title: "Draw Jump Info",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Skeleton",
        label: "kSkeletonDisplayTrajectoryNode",
        title: "Draw TrajectoryNode",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Skeleton",
        label: "kZeroTrajectoryTransform",
        title: "Zero Trajectory",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Animation",
        label: "kShowDangerZone",
        title: "ShowDangerZone",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Skeleton",
        label: "kLandingAdjustFramesToBlendAwayOffset",
        title: "Frames to blend away offset",
        initial: Value::Float(f32::from_bits(0x4170_0000)),
    },
    Definition {
        section: "Physics_Skeleton",
        label: "kDebugLandingAdjust",
        title: "Show Landing Adjust Debug",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Skeleton",
        label: "kDebugUsephyscogland",
        title: "Use phys com on landing",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Skeleton",
        label: "kLandingAdjustFirstFrameHack",
        title: "Landing Adjust first frame hack",
        initial: Value::Float(f32::from_bits(0x3c75_c28f)),
    },
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kbLOD_NegateY",
        title: "NegateY",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kbLOD_ShowLandingAdjust",
        title: "Show Landing Adjust",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Animation",
        label: "kUseXYZCentreOfMass",
        title: "Use XYZ Centre Of Mass",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "AI_LivingWorld",
        label: "kMaxVechileObjectSize",
        title: "Max Vechicle test size",
        initial: Value::Float(15.0),
    },
    Definition {
        section: "AI_DynamicObject",
        label: "kFeedDMOGrindSplines",
        title: "Feed DMO Grind Splines",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "AI_LivingWorld",
        label: "kMaxDynamicObjectSize",
        title: "Max DMO test size",
        initial: Value::Float(15.0),
    },
    Definition {
        section: "Debug_Physics_GrindDetector",
        label: "kGrindableDetectionRadius",
        title: "Grind Detection Radius",
        initial: Value::Float(f32::from_bits(0x3f99_999a)),
    },
    Definition {
        section: "Debug_Physics_GrindDetector",
        label: "kDebugDrawNearbyGrindPrimatives",
        title: "Do Nearby Grind Debug",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Grinds",
        label: "kEnableUsualGrinds",
        title: "kEnableUsualGrinds",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Grinds",
        label: "kDeckFlickerScore",
        title: "Deck Grind Flicker Score",
        initial: Value::Integer {
            value: 50,
            minimum: 0,
            maximum: 1000,
        },
    },
    Definition {
        section: "Physics_Grinds",
        label: "kTruckFlickerScore",
        title: "Truck Grind Flicker Score",
        initial: Value::Integer {
            value: 20,
            minimum: 0,
            maximum: 1000,
        },
    },
    Definition {
        section: "Physics_Grinds",
        label: "kTooMuchFlickerThresh",
        title: "Too Much Flicker Thresh",
        initial: Value::Integer {
            value: 151,
            minimum: 0,
            maximum: 100_000,
        },
    },
    Definition {
        section: "Physics_Grinds",
        label: "kStateFlickerAvoidGrindFrames",
        title: "State Flicker Avoid Grind Frames",
        initial: Value::Integer {
            value: 90,
            minimum: 0,
            maximum: 300,
        },
    },
    Definition {
        section: "Physics_States_StateChanger",
        label: "kGrindJumpAirFrames",
        title: "Grind Jump Air Frames",
        initial: Value::Integer {
            value: 10,
            minimum: 0,
            maximum: 60,
        },
    },
    Definition {
        section: "Physics_States_StateChanger",
        label: "kStateChangerMinTimeInAir",
        title: "Min Time In Air",
        initial: Value::Float(f32::from_bits(0x3da3_d70a)),
    },
    // Ref830e0c40, initializer82f9e8b8, table82fd62ac; null callback.
    Definition {
        section: "Physics_Skateboard",
        label: "kbDrawJumpDebugInfo",
        title: "kbDrawJumpDebugInfo",
        initial: Value::Boolean(false),
    },
    // Ref8311e518, initializer82fa0418, table82fd6454; null callback.
    Definition {
        section: "Debug_Physics_Conditioners",
        label: "kGroundStairsFrames",
        title: "Ground Stairs Frames",
        initial: Value::Integer {
            value: 9,
            minimum: 0,
            maximum: 10,
        },
    },
    // Ref830e0b24, initializer82f9e868, table82fd62a8.
    Definition {
        section: "Physics_Skateboard",
        label: "kDrawVertTurningInput",
        title: "Draw Vert Turning Input",
        initial: Value::Boolean(false),
    },
    // Ref830e0af8, initializer82f9e770, table82fd629c, nullcallback.
    Definition {
        section: "Physics_States_StateChanger",
        label: "kMaxGrindIllegalFrames",
        title: "MaxGrindIllegalFrames",
        initial: Value::Integer {
            value: 20,
            minimum: 1,
            maximum: 240,
        },
    },
    // Ref830e0d38, initializer82f9e9b8, table82fd62b8, null callback.
    Definition {
        section: "Physics_Jumping",
        label: "kSpewJumpStuff",
        title: "spew jump stuff",
        initial: Value::Boolean(false),
    },
    // Ref8308dd8c, initializer82f7af88, table82fd4400; null callback.
    Definition {
        section: "Physics_Debug",
        label: "kSkaterLookDirection",
        title: "Show Skater Look Direction",
        initial: Value::Boolean(false),
    },
    // Ref8308d94c, initializer82f7afd8, table82fd4404; null callback.
    Definition {
        section: "Physics_Debug",
        label: "kSkateboardVelDirection",
        title: "Show Vel Direction",
        initial: Value::Boolean(false),
    },
    // Ref830db76c, initializer82f98038, table82fd5d90; null callback.
    Definition {
        section: "Physics_Reckoning_BodySpin",
        label: "kBodySpinDerivativeScalar",
        title: "BodySpin Derivative",
        initial: Value::Float(f32::from_bits(0x3fc0_0000)),
    },
    // Ref830db3f8, initializer82f98098, table82fd5d94; null callback.
    Definition {
        section: "Physics_Reckoning_BodySpin",
        label: "kBodySpinBlend",
        title: "BodySpin Blend",
        initial: Value::Float(1.0),
    },
    // Ref830db984, initializer82f980f8, table82fd5d98; null callback.
    Definition {
        section: "Physics_Reckoning_BodySpin",
        label: "kDerivativeThreshold",
        title: "Derivative Threshold",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    // Ref830db374, initializer82f981b8, table82fd5da0; null callback.
    Definition {
        section: "Physics_Reckoning_BodySpin",
        label: "kBodySpinInputBlend",
        title: "Input Blend",
        initial: Value::Float(f32::from_bits(0x3e4c_cccd)),
    },
    // Ref830dc7a0, initializer82f9c070, table82fd6064; null callback.
    Definition {
        section: "Physics_Reckoning",
        label: "kShowReckoningInfo",
        title: "Show Reckoning Info",
        initial: Value::Boolean(false),
    },
    // Ref830dd2d4, initializer82f9c110, table82fd606c; null callback.
    Definition {
        section: "Physics_Reckoning",
        label: "kShowCoffinInfo",
        title: "Show Coffin Info",
        initial: Value::Boolean(false),
    },
    // Ref830dc078, initializer82f9c0c0, table82fd6068; null callback.
    Definition {
        section: "Physics_Reckoning",
        label: "kShowBodyTiltInfo",
        title: "Show BodyTilt Info",
        initial: Value::Boolean(false),
    },
    // 93: ref8308e538, init82f7d648, table82fd4628; raw null callback.
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kOffboardDriveScalar",
        title: "OB Drive Scalar",
        initial: Value::Float(f32::from_bits(0x4000_0000)),
    },
    // 94: ref8308e474, init82f7d6f8, table82fd4630; raw null callback.
    Definition {
        section: "Physics_Skeleton",
        label: "kSoftDamping",
        title: "Soft Damping",
        initial: Value::Float(f32::from_bits(0x4348_0000)),
    },
    // 95: ref8308e378, init82f7d878, table82fd4640; raw null callback.
    Definition {
        section: "Physics_Skeleton_IK",
        label: "kDrawPreFeetIK",
        title: "Draw Pre FeetIK Info",
        initial: Value::Boolean(false),
    },
    // 96: ref8308ed90, init82f7d918, table82fd4648; raw null callback.
    Definition {
        section: "Physics_Skeleton_IK",
        label: "kDrawFeetOnDeckIK",
        title: "Draw Feet On Deck Info",
        initial: Value::Boolean(false),
    },
    // 97: ref8308ea98, init82f7da08, table82fd4654; raw null callback.
    Definition {
        section: "Physics_Skeleton_IK",
        label: "kToeContactThreshold",
        title: "NormalY to abort push",
        initial: Value::Float(f32::from_bits(0x3f33_3333)),
    },
    // 98: ref8308e200, init82f7da68, table82fd4658; raw null callback.
    Definition {
        section: "Physics_Skeleton_IK",
        label: "kTargetNormalBlendSpeed",
        title: "Target Norm Blend Speed",
        initial: Value::Float(f32::from_bits(0x3e4c_cccd)),
    },
    // 99: ref8308e39c, init82f7db28, table82fd4660; raw null callback.
    Definition {
        section: "Physics_Skeleton",
        label: "kEnableIKContactOffset",
        title: "Enable IK Contact Offset",
        initial: Value::Boolean(true),
    },
    // 100: ref8308e674, init82f7dc68, table82fd4670; raw null callback.
    Definition {
        section: "Physics_Skeleton_IK",
        label: "kMaxIKOffsetLength",
        title: "Max IK Offset Length",
        initial: Value::Float(f32::from_bits(0x3f33_3333)),
    },
    // 101: ref8308ea24, init82f7dcc8, table82fd4674; raw null callback.
    Definition {
        section: "Physics_Skeleton_IK",
        label: "kMaxIKLocalDelta",
        title: "Max IK Local Delta",
        initial: Value::Float(f32::from_bits(0x3ccc_cccd)),
    },
    // 102: ref8308e0a0, init82f7dd28, table82fd4678; raw null callback.
    Definition {
        section: "Physics_Skeleton_IK",
        label: "kEXIK_MaxAngle",
        title: "Max Ground Angle",
        initial: Value::Float(f32::from_bits(0x3f35_c28f)),
    },
    // 103: ref8308e610, init82f7dd88, table82fd467c; raw null callback.
    Definition {
        section: "Physics_Skeleton",
        label: "kDrawIKContactDebug",
        title: "Draw Feet IK Contact Offset Debug",
        initial: Value::Boolean(false),
    },
    // 104: ref8308ec74, init82f7ddd8, table82fd4680; raw null callback.
    Definition {
        section: "Physics_Skeleton",
        label: "kMaxIKContactOffset",
        title: "Max IK Contact Offset",
        initial: Value::Float(f32::from_bits(0x3e99_999a)),
    },
    // 105: ref8308e8dc, init82f7de38, table82fd4684; raw null callback.
    Definition {
        section: "Physics_Skeleton_IK",
        label: "kDOPrePhysIK",
        title: "AA Do Pre Phys IK",
        initial: Value::Boolean(true),
    },
    // 106: ref8308e2c4, init82f7de88, table82fd4688; raw null callback.
    Definition {
        section: "Physics_Skeleton_IK",
        label: "kMaxLengthTestLeg",
        title: "AA Max Length Leg IK",
        initial: Value::Float(f32::from_bits(0x3f4c_cccd)),
    },
    // 107: ref8308e3e4, init82f7dee8, table82fd468c; raw null callback.
    Definition {
        section: "Physics_Skeleton_IK",
        label: "kMaxLengthTestArm",
        title: "AA Max Length Arm IK",
        initial: Value::Float(f32::from_bits(0x3f80_0000)),
    },
    // 108: ref8308e1d4, init82f7df48, table82fd4690; raw null callback.
    Definition {
        section: "Physics_Skeleton",
        label: "kMaxIKContactPenetrationOffset",
        title: "Max IK Contact Penetration Offset",
        initial: Value::Float(f32::from_bits(0x3e99_999a)),
    },
    // 109: ref8308e314, init82f7dfa8, table82fd4694; raw null callback.
    Definition {
        section: "Physics_Skeleton",
        label: "kSidewaysOffset",
        title: "Sideways Contact Offset",
        initial: Value::Float(f32::from_bits(0x3e8f_5c29)),
    },
    // 110: ref8308e2f0, init82f7e008, table82fd4698; raw null callback.
    Definition {
        section: "Physics_Skeleton",
        label: "kSidewaysOffsetDraw",
        title: "Draw Sideways Contact Offset",
        initial: Value::Boolean(false),
    },
    // Pumping init82f9c1c0, table82fd6074; null callback.
    Definition {
        section: "Physics_Pumping",
        label: "kDrawDeckAngleCompression",
        title: "Draw Deck Angle Compression",
        initial: Value::Boolean(false),
    },
    // Pumping init82f9c210, table82fd6078; null callback.
    Definition {
        section: "Physics_Pumping",
        label: "kDrawNewPump",
        title: "Draw New Pump",
        initial: Value::Boolean(false),
    },
    // 113: ref8308d858, init82f7b7a0, table82fd4460; null callback.
    Definition {
        section: "Physics_Surfaces",
        label: "kForcedWorldSurface",
        title: "World Surface",
        initial: Value::Enumeration {
            value: 0,
            tags: &[
                ("as detected", 0),
                ("smooth", 1),
                ("rough", 2),
                ("slow", 3),
                ("slippery", 4),
                ("very slow", 5),
                ("unrideable", 6),
                ("doNotAlign", 7),
            ],
        },
    },
    // 114: ref830d9620, init82f8f028, table82fd5718; null callback.
    Definition {
        section: "Physics_Skateboard",
        label: "kDrawSBJoints",
        title: "Draw Joints",
        initial: Value::Boolean(false),
    },
    // 115: ref8305328c, init82f6c450, table82fd2414; null callback.
    Definition {
        section: "Animation",
        label: "cv_IndividualFullbindpose",
        title: "Individual full(on)/facial(off) bindpose",
        initial: Value::Boolean(false),
    },
    // 116: ref8308dbf4, init82f7bc00, table82fd4490; null callback.
    Definition {
        section: "Physics_Debug",
        label: "kDrawSpeedWobbleInfo",
        title: "Draw Speed Wobble Info",
        initial: Value::Boolean(false),
    },
    // 117: ref8308db08, init82f7b138, table82fd4414; null callback.
    Definition {
        section: "Physics_Skateboard",
        label: "kDrawWallRide",
        title: "Draw Wall Ride Info",
        initial: Value::Boolean(false),
    },
    // 118/119: init82f7b278/b2c8, tables4424/4428; null callbacks.
    Definition {
        section: "Physics_Skateboard",
        label: "kDrawPumpForceIntention",
        title: "Draw Pump Force Intention",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Skateboard",
        label: "kDrawAbsorptionCalc",
        title: "Draw Absorption Calc",
        initial: Value::Boolean(false),
    },
    // 120..123: init82f7b648/b6a8/b6f8/b750, tables4450..445c; null callbacks.
    Definition {
        section: "Physics_Cheats",
        label: "kSpeedCheatSpeed",
        title: "Speed Cheat Speed (kph)",
        initial: Value::Float(90.0), // native GUI range0..200
    },
    Definition {
        section: "Physics_Cheats",
        label: "kActivateSpeedCheat",
        title: "Activate Speed Cheat",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Manual",
        label: "kManualForceMaxFramesSpinningTheSameWay",
        title: "max spin frames",
        initial: Value::Integer {
            value: 30,
            minimum: 0,
            maximum: 120,
        },
    },
    Definition {
        section: "Physics_SpeedConservation",
        label: "kSpeedConservationDebugDisplay",
        title: "Draw Debug Info",
        initial: Value::Boolean(false),
    },
    // 124: ref830a30d4, init82f815a8, table82fd493c; null callback.
    Definition {
        section: "Animation_Skater",
        label: "kGestureMappingDebug",
        title: "Gesture Mapping Debug",
        initial: Value::Boolean(false),
    },
    //125..127: init82f8f0c8/f118/f178; tables5720/24/28; null callbacks.
    Definition {
        section: "Physics_Hacks",
        label: "kApplyFakeTorque",
        title: "Apply Fake Torque",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Hacks",
        label: "kFakeTorqueValue",
        title: "Fake Torque Value",
        initial: Value::Float(f32::from_bits(0xc0c1_999a)),
    }, // GUI -9..9
    Definition {
        section: "Physics_Hacks",
        label: "kFakeTorqueExponent",
        title: "Fake Torque Exponent",
        initial: Value::Integer {
            value: -1,
            minimum: -10,
            maximum: 5,
        },
    },
    //128: init82f8ef78/ref830d7258/table82fd5710; null callback, GUI -1..0.
    Definition {
        section: "Physics_Skateboard",
        label: "kDotToStopPush",
        title: "Stop push DP",
        initial: Value::Float(f32::from_bits(0xbf40_0000)),
    },
    // 129
    Definition {
        section: "Physics_Skeleton",
        label: "kDrawContactInfo",
        title: "Draw Contact Info",
        initial: Value::Boolean(false),
    },
    // 130
    Definition {
        section: "Physics_Skeleton_Collision",
        label: "kDrawAudioCollisionInfo",
        title: "Draw Audio Info",
        initial: Value::Boolean(false),
    },
    // 131: ref8308e4f0, init82f7c3c8, table82fd4538; null callback.
    Definition {
        section: "Physics_Skeleton",
        label: "kDrawSkateboardError",
        title: "Draw SkateboardError",
        initial: Value::Boolean(false),
    },
    // 132: ref8308e7c8, init82f7bf50, table82fd44f8; null callback.
    Definition {
        section: "Physics_Skeleton_Collision",
        label: "kDrawBodyPartCollision",
        title: "Draw Body Part Info",
        initial: Value::Boolean(false),
    },
    // 133: ref8308e5b4, init82f7bfa0, table82fd44fc; null callback.
    Definition {
        section: "Physics_Skeleton_Collision",
        label: "kAllowAnyContact",
        title: "Allow Any Contact",
        initial: Value::Boolean(false),
    },
    // 134: ref8308e7ec, init82f7bff0, table82fd4500; null callback.
    Definition {
        section: "Physics_Skeleton_Collision",
        label: "kDrawCollisionUpdate",
        title: "Draw Collision Update",
        initial: Value::Boolean(false),
    },
    // 135: ref8308ea50, init82f7c040, table82fd4504; null callback.
    Definition {
        section: "Physics_Skeleton_Collision",
        label: "kDebugAlwayCollide",
        title: "Always COLLIDE",
        initial: Value::Boolean(false),
    },
    // 136: ref8308eec4, init82f7c090, table82fd4508; null callback.
    Definition {
        section: "Physics_Skeleton_Collision",
        label: "kShowGroundPlane",
        title: "Draw Groundplane Info",
        initial: Value::Boolean(false),
    },
    // 137: ref8308e83c, init82f7c0e0, table82fd450c; null callback.
    Definition {
        section: "Physics_Skeleton_Collision",
        label: "kSmallObjectForceScalarHack",
        title: "Small DMO force scalar",
        initial: Value::Float(f32::from_bits(0x3f00_0000)),
    },
    // 138: ref8308e6e8, init82f7c140, table82fd4510; null callback.
    Definition {
        section: "Physics_Skeleton_Collision",
        label: "kSCMaxDistOffBoard",
        title: "GroundPlane max OB",
        initial: Value::Float(f32::from_bits(0x3e99_999a)),
    },
    // 139: ref8308ed48, init82f7d058, table82fd45e0; null callback.
    Definition {
        section: "Physics_Skeleton",
        label: "kPS_UseNormalsForProxyOffset",
        title: "Use Coll Normals",
        initial: Value::Boolean(false),
    },
    // 140: ref8308e810, init82f7d0a8, table82fd45e4; null callback.
    Definition {
        section: "Physics_Skeleton",
        label: "kPS_ScaleHookToHipsY",
        title: "Scale Y Disp",
        initial: Value::Float(f32::from_bits(0x3f00_0000)),
    },
    // 141: ref8308e164, init82f7c558, table82fd454c; null callback.
    Definition {
        section: "Physics_Skeleton",
        label: "kPSK_DrawHookToHips",
        title: "Draw HookToHips",
        initial: Value::Boolean(false),
    },
    // 142
    Definition {
        section: "Physics_Skateboard",
        label: "kDrawAntiFlipTorque",
        title: "Draw Anti Flip Torque",
        initial: Value::Boolean(false),
    },
    // 143
    Definition {
        section: "Physics_Skateboard",
        label: "kDrawVertTurning",
        title: "Draw Vert Turning Info",
        initial: Value::Boolean(false),
    },
    // 144: ref/init/table 8308d8d8 /82f7b438 /82fd4438; null callback.
    Definition {
        section: "Physics_Manual",
        label: "kShowManualInfo",
        title: "Draw Manual Info",
        initial: Value::Boolean(false),
    },
    // 145: ref/init/table 8308ddb0 /82f7b5e8 /82fd444c; GUI0..1; null callback.
    Definition {
        section: "Physics_Manual",
        label: "kWrongWheelScalar",
        title: "Wrong Wheel Scalar",
        initial: Value::Float(0.5),
    },
    // 146: ref/init/table 830d809c /82f8f078 /82fd571c; null callback.
    Definition {
        section: "Physics_Skateboard",
        label: "kbDrawNudgeDebugInfo",
        title: "kbDrawNudgeDebugInfo",
        initial: Value::Boolean(false),
    },
    // 147: ref/init/table 830d8c08 /82f8f1d0 /82fd572c; GUI0..2; null callback.
    Definition {
        section: "Physics_States_PhysicsGround",
        label: "kAntiFlipNudgeMaxDeckSpeed",
        title: "Anti Flip Nudge Max Deck Speed",
        initial: Value::Float(f32::from_bits(0x3f8c_cccd)),
    },
    // 148: ref/init/table 830d8958 /82f8f230 /82fd5730; GUI0..200; null callback.
    Definition {
        section: "Physics_States_PhysicsGround",
        label: "kAntiFlipNudge",
        title: "Anti Flip Nudge",
        initial: Value::Float(50.0),
    },
    // 149: ref/init/table 830d8f8c /82f8f290 /82fd5734; null callback.
    Definition {
        section: "Physics_States_PhysicsGround",
        label: "kMinAntiFlipFrames",
        title: "Min Anti Flip Frames",
        initial: Value::Integer {
            value: 12,
            minimum: 0,
            maximum: 100,
        },
    },
    // 150: ref/init/table 830d9ad4/82f8f468/82fd5748; null callback.
    Definition {
        section: "Physics_States_PhysicsGround",
        label: "kHangFrames",
        title: "Hang Frames",
        initial: Value::Integer {
            value: 20,
            minimum: 0,
            maximum: 100,
        },
    },
    // 151: ref/init/table 830d76d8/82f8f518/82fd5750; null callback.
    Definition {
        section: "Physics_States_PhysicsGround",
        label: "kHungUpWipeoutFrames",
        title: "Hung Up Wipeout Frames",
        initial: Value::Integer {
            value: 20,
            minimum: 0,
            maximum: 100,
        },
    },
    // 152: ref/init/table 830d8170/82f8f750/82fd5768; null callback.
    Definition {
        section: "Physics_States_PhysicsGround",
        label: "kMaxGroundPinFrames",
        title: "Max Ground Pin Frames",
        initial: Value::Integer {
            value: 90,
            minimum: 0,
            maximum: 180,
        },
    },
    // 153: ref/init/table 830d7904/82f8f7a8/82fd576c; null callback.
    Definition {
        section: "Physics_States_PhysicsGround",
        label: "kMaxGroundPinStartFrames",
        title: "Max Ground Pin StartFrames",
        initial: Value::Integer {
            value: 30,
            minimum: 0,
            maximum: 180,
        },
    },
    // 154: original live post-physics cvar; null callback, see evidence packet.
    Definition {
        section: "Physics_Skateboard",
        label: "kMinDPWithZToBeConsideredDeckEnd",
        title: "DP Skateboard Contact",
        initial: Value::Float(f32::from_bits(0x3f6b_851f)),
    },
    // 155: original live post-physics cvar; null callback, see evidence packet.
    Definition {
        section: "Physics_Skateboard",
        label: "kFreeSkateboardWheelInertia",
        title: "Free wheel inertia",
        initial: Value::Float(f32::from_bits(0x3d23_d70a)),
    },
    // 156: original live post-physics cvar; null callback, see evidence packet.
    Definition {
        section: "Physics_Skateboard",
        label: "kSkateboard_LineDistToUseNormal",
        title: "Line Dist For GNormal",
        initial: Value::Float(f32::from_bits(0x3d8f_5c29)),
    },
    // 157: original live post-physics cvar; null callback, see evidence packet.
    Definition {
        section: "Physics_ATEST",
        label: "kMinBadVelY",
        title: "Min y vel bad",
        initial: Value::Float(f32::from_bits(0x41c8_0000)),
    },
    // 158: original live post-physics cvar; null callback, see evidence packet.
    Definition {
        section: "Physics_ATEST",
        label: "kMaxGoodVelY",
        title: "Min y vel GOOD",
        initial: Value::Float(f32::from_bits(0x40a0_0000)),
    },
    Definition {
        section: "Physics_Cheats",
        label: "kWipeoutDisableAllWipeouts",
        title: "Disable ALL wipeouts ",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Wipeouts_Debug",
        label: "kWipeoutDisplayDebugInfo",
        title: "Display Debug Info ",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[0],
        title: "Skeleton Contact",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[1],
        title: "Skeleton Displacment",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[2],
        title: "Planar Deck Acceleration",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[3],
        title: "Deck Flipped",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[4],
        title: "Upside down no grabbing",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[5],
        title: "Bad Angle Landing",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[6],
        title: "Landed Too Hard",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[7],
        title: "Hit Vehicle",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[8],
        title: "Air To Hard Grind",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[9],
        title: "Hard Grind Engagement",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[10],
        title: "Boardslide Stop On Rail",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[11],
        title: "Off Balance",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[12],
        title: "Hung Up On Rail",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[13],
        title: "Off Balance Grind",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[14],
        title: "Hard Horizontal Grind",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[15],
        title: "Upside down falling",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[16],
        title: "Skeleton Opposing contacts",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[17],
        title: "Auto Exit Failure",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[18],
        title: "Skeleton Squashed",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[19],
        title: "Skeleton Leaning Contact",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[20],
        title: "Deck Opposing Contacts",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[21],
        title: "Sensitive Skeleton",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[22],
        title: "Bad Skateboard",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[23],
        title: "Too Fast OffBoard",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[24],
        title: "OffBoard Failure",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[25],
        title: "BadGround",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[26],
        title: "Death Leap",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[27],
        title: "OB Landing Too Fast",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[28],
        title: "OB Not Upright",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: crate::wipeout::WIPEOUT_ENABLE_CVAR_LABELS[29],
        title: "OB Landed On Deck",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Debug",
        label: "kbEnableGhostByDefault",
        title: "kbEnableGhostByDefault",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Debug_Scoring_DebugDraw",
        label: "kDebugRenderLocalSimPlayer",
        title: "kDebugRenderLocalSimPlayer",
        initial: Value::Boolean(true),
    },
    //193: init82f7c418/table82fd453c; null callback.
    Definition {
        section: "Physics_Skeleton",
        label: "kDrawSkeletonCollisionWipeout",
        title: "Draw Skeleton Collision Wipeout",
        initial: Value::Boolean(false),
    },
    //194: init82f7c6e8/table82fd4560; null callback.
    Definition {
        section: "Physics_ATEST",
        label: "kWipeoutOnCollision",
        title: "Wipeout On COllision",
        initial: Value::Boolean(false),
    },
    //195..201: init82fa00d8..0318, table82fd6430..6448; null callbacks.
    Definition {
        section: "Physics_Physout_DampedComBlend",
        label: "kDampedComBlend",
        title: "Damped Com Blend",
        initial: Value::Float(f32::from_bits(0x3d4c_cccd)),
    },
    Definition {
        section: "Physics_Physout_DampedComBlend",
        label: "kDampedComVelBlend",
        title: "Damped ComVel Blend",
        initial: Value::Float(f32::from_bits(0x3d4c_cccd)),
    },
    Definition {
        section: "Physics_Physout_DampedComBlend",
        label: "kDampedComBlendY",
        title: "Damped Com Blend Y",
        initial: Value::Float(f32::from_bits(0x3cf5_c28f)),
    },
    Definition {
        section: "Physics_Physout_DampedComBlend",
        label: "kDampedComVelBlendY",
        title: "Damped ComVel Blend Y",
        initial: Value::Float(f32::from_bits(0x3cf5_c28f)),
    },
    Definition {
        section: "Physics_Physout_DampedComBlend",
        label: "kDampedComErrorBlend",
        title: "Damped Com Error Blend",
        initial: Value::Float(f32::from_bits(0x3da3_d70a)),
    },
    Definition {
        section: "Physics_Physout_DampedComBlend",
        label: "kDampedComErrorIterBlend",
        title: "Error Iteration Blend",
        initial: Value::Float(f32::from_bits(0x3d4c_cccd)),
    },
    Definition {
        section: "Physics_Physout_DampedComBlend",
        label: "kDampedComDraw",
        title: "Damped Com Draw",
        initial: Value::Boolean(false),
    },
    //202: initializer82f7e0a8, table82fd46a0; false, null callback.
    Definition {
        section: "Physics_Systems_Wobble",
        label: "kDebugDrawWobble",
        title: "Debug Draw Wobble",
        initial: Value::Boolean(false),
    },
    //203: initializer82f7d8c8, table82fd4644; false, null callback.
    Definition {
        section: "Physics_Skeleton_IK",
        label: "kDrawPostFeetIK",
        title: "Draw Post FeetIK Info",
        initial: Value::Boolean(false),
    },
    //204: initializer82f7d9b8, table82fd4650; false, null callback.
    Definition {
        section: "Physics_Skeleton_IK",
        label: "kDrawPostPhysIK",
        title: "Draw Post Physics",
        initial: Value::Boolean(false),
    },
    // 205: native definition; source table order is retained below.
    Definition {
        section: "Physics_Landing",
        label: "kBodySpinThresh",
        title: "Body Spin Thresh",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    // 206: native definition; source table order is retained below.
    Definition {
        section: "Physics_Landing",
        label: "kLoBodySpin",
        title: "Lo Body Spin",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    // 207: native definition; source table order is retained below.
    Definition {
        section: "Physics_Landing",
        label: "kHiBodySpin",
        title: "Hi Body Spin",
        initial: Value::Float(f32::from_bits(0x40e0_0000)),
    },
    // 208: native definition; source table order is retained below.
    Definition {
        section: "Physics_Landing",
        label: "kLoSideSpeed",
        title: "Lo Side Speed",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    // 209: native definition; source table order is retained below.
    Definition {
        section: "Physics_Landing",
        label: "kHiSideSpeed",
        title: "Hi Side Speed",
        initial: Value::Float(f32::from_bits(0x4120_0000)),
    },
    // 210: native definition; source table order is retained below.
    Definition {
        section: "Physics_Landing",
        label: "kStraightSpeedThresh",
        title: "Straight Speed Thresh",
        initial: Value::Float(f32::from_bits(0x4000_0000)),
    },
    // 211: native definition; source table order is retained below.
    Definition {
        section: "Physics_Landing",
        label: "kStraightAngleThresh",
        title: "Straight Angle Thresh",
        initial: Value::Float(f32::from_bits(0x3e4c_cccd)),
    },
    // 212: native definition; source table order is retained below.
    Definition {
        section: "Physics_Landing",
        label: "kForceSpinThresh",
        title: "Force Spin Thresh",
        initial: Value::Float(f32::from_bits(0x3e99_999a)),
    },
    // 213: native definition; source table order is retained below.
    Definition {
        section: "Physics_Skeleton_IK",
        label: "kDoNewPostPhysicsIK",
        title: "Do New PP IK",
        initial: Value::Boolean(true),
    },
    // 214: native definition; source table order is retained below.
    Definition {
        section: "Physics_Skeleton_IK",
        label: "kDrawNewPostPhysicsIK",
        title: "Draw New PP IK",
        initial: Value::Boolean(false),
    },
    // 215: native definition; source table order is retained below.
    Definition {
        section: "Physics_Skeleton_HeadAdjust",
        label: "kDebugForceLookAt",
        title: "Debug Force Look At",
        initial: Value::Boolean(false),
    },
    // 216: native definition; source table order is retained below.
    Definition {
        section: "Physics_Skeleton_HeadAdjust",
        label: "kHeadTrackingMaxAngleToLookAtTargetXYZ",
        title: "Max Angle To TargetXYZ",
        initial: Value::Float(f32::from_bits(0x4270_0000)),
    },
    // 217: native definition; source table order is retained below.
    Definition {
        section: "Physics_Skeleton_HeadAdjust",
        label: "kHeadTrackingClampIdeal",
        title: "Clamp Ideal",
        initial: Value::Boolean(true),
    },
    // 218: native definition; source table order is retained below.
    Definition {
        section: "Physics_Skeleton_HeadAdjust",
        label: "kHeadTrackingLargestUpAngle",
        title: "Largest angle to look down",
        initial: Value::Float(f32::from_bits(0x4234_0000)),
    },
    // 219: native definition; source table order is retained below.
    Definition {
        section: "Physics_Skeleton_HeadAdjust",
        label: "kDrawHeadAdjustInfo",
        title: "Draw Head Adjust Info",
        initial: Value::Boolean(false),
    },
    // 220: native definition; source table order is retained below.
    Definition {
        section: "Physics_Conditioners_Physics",
        label: "kMaxStairTrickDeckNoseUp",
        title: "Max Stair Trick Deck Nose Up",
        initial: Value::Float(f32::from_bits(0x3e4c_cccd)),
    },
    // 221: native definition; source table order is retained below.
    Definition {
        section: "Physics_Conditioners_Physics",
        label: "kMinStairTrickSpeed",
        title: "Min Stair Trick Speed",
        initial: Value::Float(f32::from_bits(0x3fc0_0000)),
    },
    // 222: native definition; source table order is retained below.
    Definition {
        section: "Physics_Skeleton",
        label: "kUpdatePoseBuffer",
        title: "Update Pose Buffer",
        initial: Value::Boolean(true),
    },
    // 223: native definition; source table order is retained below.
    Definition {
        section: "Physics_Skeleton_HeadAdjust",
        label: "kFractionalHeadAdjust",
        title: "FractionalHeadAdjust",
        initial: Value::Float(f32::from_bits(0x3e4c_cccd)),
    },
    // 224: native definition; source table order is retained below.
    Definition {
        section: "Physics_Skeleton_HeadAdjust",
        label: "kFractionalSpineAdjust",
        title: "FractionalSpineAdjust",
        initial: Value::Float(f32::from_bits(0x3f19_999a)),
    },
    // 225/226: initializer tables82fd64a4/64a8, null callbacks.
    Definition {
        section: "Physics_Animation",
        label: "kDrawCarveInfo",
        title: "Draw Carve Info",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Animation",
        label: "kDrawTiltInfo",
        title: "Draw Tilt Info",
        initial: Value::Boolean(false),
    },
    // 227..229: initializers82f85a70/5ac0/5b10, tables82fd4ea0/4ea4/4ea8.
    Definition {
        section: "Animation_Skater_BodyTilt",
        label: "kUseTilt",
        title: "Use Body Tilt",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Animation_Skater_BodyTilt",
        label: "kSetTilt",
        title: "Hard Code Tilt",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Animation_Skater_BodyTilt",
        label: "kTiltX",
        title: "TiltX",
        initial: Value::Float(0.0),
    },
    // 230: initializer82f85b70, table82fd4eac; no callback.
    Definition {
        section: "Animation",
        label: "kbSlideDebugDisplay",
        title: "Slide Debug Display",
        initial: Value::Boolean(false),
    },
    // 231: initializer82f85e40, table82fd4ecc; no callback.
    Definition {
        section: "Animation_Skater_InAir",
        label: "kEnableLandingOnBoardBodySpin",
        title: "Enable Landing On Board Body Spin",
        initial: Value::Boolean(true),
    },
    // 232/233:82f86950/69a0, tables82fd4f48/4f4c; null callbacks.
    Definition {
        section: "Controller",
        label: "kUseAutoPump",
        title: "Enable AutoPump",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Pumping",
        label: "kTimeToConsiderPumpingMode",
        title: "Player Pumping Extra Time",
        initial: Value::Float(0.5),
    },
    // 234..236:82f86528/6578/65c8, tables82fd4f18/4f1c/4f20.
    Definition {
        section: "Animation_Skater_Carving",
        label: "kUseMagMap",
        title: "Use Mag Maps",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Animation_Skater_Carving",
        label: "kUseAngleMap",
        title: "Draw Angle Maps",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Animation_Skater_Carving",
        label: "kUseAngleOffset",
        title: "Use Angle Offsets",
        initial: Value::Boolean(true),
    },
    // 237:82f86328, table82fd4f00, null callback.
    Definition {
        section: "Controller_Animation_Skater",
        label: "kDistComToBoard",
        title: "Set Dist COM to Board",
        initial: Value::Float(f32::from_bits(0x3f26_6666)),
    },
    // 238:82f85f98, table82fd4ed4, null callback.
    Definition {
        section: "Animation_Skater_Pumping",
        label: "kUsePumping",
        title: "Use Pumping",
        initial: Value::Boolean(true),
    },
    // 239:82f56930, table82fd1038, reference83047fdc, null callback.
    Definition {
        section: "Animation_Skater_InAir",
        label: "kOffBoardAirPrelandTime",
        title: "OffBoard Air Preland Time",
        initial: Value::Float(f32::from_bits(0x3f99_999a)),
    },
    // 240:82f967f0, table82fd5c6c, reference830da824, null callback.
    Definition {
        section: "Physics_Trajectory",
        label: "kbDebugDraw",
        title: "Debug Draw",
        initial: Value::Boolean(false),
    },
    // 241:82f96890, table82fd5c74, reference830dadf0, null callback.
    Definition {
        section: "Physics_Trajectory",
        label: "kShowTrajectories",
        title: "Show Trajectories",
        initial: Value::Boolean(false),
    },
    //242..244 BodySpin Air: initializers82f98158/8218/8278, table5d9c/5da4/5da8.
    Definition {
        section: "Physics_Reckoning_BodySpin",
        label: "kBodySpinInputFade",
        title: "Input Fade",
        initial: Value::Float(f32::from_bits(0x3f75_c28f)),
    },
    Definition {
        section: "Physics_Reckoning_BodySpin",
        label: "kMaxBodySpinSpeedToForceAutoAdjust",
        title: "Spinspeed for autoalways",
        initial: Value::Float(f32::from_bits(0x3ca3_d70a)),
    },
    Definition {
        section: "Physics_Reckoning_BodySpin",
        label: "kMaxAutoSpinMag",
        title: "Max Auto Spin",
        initial: Value::Float(2.0),
    },
    //245: initializer82f8ea18/table82fd56d4/reference830d7ad0, null callback.
    Definition {
        section: "Physics_States_PhysicsAir",
        label: "kPhysicsAirJumpFixFrames",
        title: "Physics Air Jump Fix Frames",
        initial: Value::Integer {
            value: 12,
            minimum: 0,
            maximum: 30,
        },
    },
    //246/247: initializers82f8e9b8/ea70, table82fd56d0/56d8.
    Definition {
        section: "Physics_States_PhysicsAir",
        label: "kPhysicsAirJumpFixMaxError",
        title: "Physics Air Jump Fix Max Error",
        initial: Value::Float(3.0),
    },
    Definition {
        section: "Physics_States_PhysicsAir",
        label: "kJumpVelMinYError",
        title: "Physics Air Jump MaxErrorY",
        initial: Value::Float(-3.0),
    },
    //248..251: OBA initializers82f8ebf0/eb90/ec50/ecb0.
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kOBA_BoardPosDelta",
        title: "kOBA_BoardPosDelta",
        initial: Value::Float(f32::from_bits(0xbf4c_cccd)),
    },
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kOBA_TrajPosDelta",
        title: "kOBA_TrajPosDelta",
        initial: Value::Float(f32::from_bits(0xbf26_6666)),
    },
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kTrajectoryConeAngleScalar",
        title: "ConeAngleScalar",
        initial: Value::Float(f32::from_bits(0x3ecc_cccd)),
    },
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kOBA_NumTrajForMount",
        title: "Num mount Traj",
        initial: Value::Integer {
            value: 5,
            minimum: 1,
            maximum: 8,
        },
    },
    // 252: initializer82f96840, table82fd5c70, reference830da52c; null callback.
    Definition {
        section: "Physics_Trajectory",
        label: "kShowTrajectorySelect",
        title: "Show Trajectory Info",
        initial: Value::Boolean(false),
    },
    // 253: initializer82f968e0, table82fd5c78, reference830daf88; null callback.
    Definition {
        section: "Physics_Trajectory",
        label: "kShowRawTrajectory",
        title: "Show Raw Trajectory",
        initial: Value::Boolean(false),
    },
    // 254: initializer82f96980, table82fd5c80, reference830da96c; null callback.
    Definition {
        section: "Physics_Trajectory",
        label: "kSelectCentralTrajectory",
        title: "No Trajectory Help",
        initial: Value::Boolean(false),
    },
    // 255: initializer82f96a20, table82fd5c88, reference830da67c; null callback.
    Definition {
        section: "Physics_Trajectory",
        label: "kShowWallInfo",
        title: "Show Wall Info",
        initial: Value::Boolean(false),
    },
    // 256: initializer82f7d758, table82fd4634, reference8308edfc.
    Definition {
        section: "Physics_Debug",
        label: "kRigidRagdoll",
        title: "RigidRagDoll",
        initial: Value::Boolean(false),
    },
    // 257: initializer82f7d828, table82fd463c, reference8308e6a0.
    Definition {
        section: "Physics_Skeleton",
        label: "kFollowAnimInRagdoll",
        title: "Follow Anim in Ragdoll",
        initial: Value::Boolean(false),
    },
    // 258: initializer82f8fa80, table82fd578c, reference830d863c; null callback.
    Definition {
        section: "Physics_Wipeout",
        label: "kWipeoutDebugDisplay",
        title: "Debug Display",
        initial: Value::Boolean(false),
    },
    // 259: initializer82f8fd70, table82fd57ac, reference830d9eb8; null callback.
    Definition {
        section: "Physics_Wipeout",
        label: "kWipeControlMaxAngleRotY",
        title: "MaxAngleRotY",
        initial: Value::Float(8.0),
    },
    // 260: initializer82f8fdd0, table82fd57b0, reference830d86e4; null callback.
    Definition {
        section: "Physics_Wipeout",
        label: "kWipeControlMaxAngleRotX",
        title: "MaxAngleRotX",
        initial: Value::Float(8.0),
    },
    // 261: initializer82f8fe30, table82fd57b4, reference830d7730; null callback.
    Definition {
        section: "Physics_Wipeout",
        label: "kWipeoutTorqueForce",
        title: "TorqueForceScalar",
        initial: Value::Float(f32::from_bits(0x4019_999a)),
    },
    // 262: initializer82f8fe90, table82fd57b8, reference830d7aa4; null callback.
    Definition {
        section: "Physics_Wipeout",
        label: "kWipeControlMaxTorque",
        title: "Max Torque",
        initial: Value::Float(0.5),
    },
    // 263: initializer82f8fef0, table82fd57bc, reference830d8070; null callback.
    Definition {
        section: "Physics_Wipeout",
        label: "kWipeAutoZAxisScalar",
        title: "Auto Z Scalar",
        initial: Value::Float(0.5),
    },
    // 264: initializer82f8ff50, table82fd57c0, reference830d9ce4; null callback.
    Definition {
        section: "Physics_Wipeout",
        label: "kWipeAutoYAxisScalar",
        title: "Auto Y Scalar",
        initial: Value::Float(0.5),
    },
    // 265: initializer82f8ffb0, table82fd57c4, reference830d9110; null callback.
    Definition {
        section: "Physics_Wipeout",
        label: "kMinControlToStopAutoTwist",
        title: "Min Control for no autotwist",
        initial: Value::Float(0.5),
    },
    // 266: init82f7ba20/table82fd447c/ref8308dab0; GUI0..1, null callback.
    Definition {
        section: "Physics_ATEST",
        label: "kOrthonalTolerance",
        title: "Orthogonal Tolerance",
        initial: Value::Float(f32::from_bits(0x3c23_d70a)),
    },
    // 267: init82f7d588/table82fd4620/ref8308eb0c; GUI0..20, null callback.
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kALC_HardDriveDisp",
        title: "Hard Drive Disp",
        initial: Value::Float(f32::from_bits(0x3f80_0000)),
    },
    // 268: init82f7d5e8/table82fd4624/ref8308e0cc; GUI0..20, null callback.
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kALC_HardDriveStr",
        title: "Hard Drive Str",
        initial: Value::Float(f32::from_bits(0x4120_0000)),
    },
    // 269: initializer82f7ce38, table82fd45c8; null callback.
    Definition {
        section: "Physics_Skeleton",
        label: "kDoWipeoutCameraRootMtxBlend",
        title: "CAMERA wipeout MTX Blend",
        initial: Value::Boolean(false),
    },
    // 270: initializer82f7cdd8, table82fd45c4; null callback.
    Definition {
        section: "Physics_Skeleton",
        label: "kHeightOffsetY",
        title: "Height offset Y",
        initial: Value::Float(f32::from_bits(0xbf4c_cccd)),
    },
    // 271: initializer82f8fc50, table82fd57a0; null callback.
    Definition {
        section: "Physics_Wipeout",
        label: "kWO_DragStart",
        title: "Drag when slow",
        initial: Value::Float(f32::from_bits(0x3d4c_cccd)),
    },
    // 272: initializer82f8fcb0, table82fd57a4; null callback.
    Definition {
        section: "Physics_Wipeout",
        label: "kWO_TimeToFadeInDrag",
        title: "TimeToFadeInDrag",
        initial: Value::Float(3.0),
    },
    // 273: initializer82f8fd10, table82fd57a8; null callback.
    Definition {
        section: "Physics_Wipeout",
        label: "kWO_MaxDrag",
        title: "Max Drag",
        initial: Value::Float(0.5),
    },
    // 274: init82f8fbf0/table82fd579c/ref830d6fa0; null callback.
    Definition {
        section: "Physics_Wipeout",
        label: "kMaxAISkaterBailDurationInSeconds",
        title: "Max AI Skater Bail Duration In Seconds",
        initial: Value::Float(8.0),
    },
    // 275: init82f90010/table82fd57c8/ref830d70cc; null callback.
    Definition {
        section: "Physics_Wipeout",
        label: "kMinUserWipeoutRecoverSec",
        title: "Min User Wipeout Recover Sec",
        initial: Value::Float(f32::from_bits(0x3fcc_cccd)),
    },
    // 276: init82f90070/table82fd57cc/ref830d71d4; null callback.
    Definition {
        section: "Physics_Wipeout",
        label: "kMinTimeToBeImpaledForTeleport",
        title: "Max Time To Be Impaled",
        initial: Value::Float(0.5),
    },
    // 277: init82f900d0/table82fd57d0/ref830d93ec; null callback.
    Definition {
        section: "Physics_Wipeout",
        label: "kMaxWipeoutImpulseFrames",
        title: "Max Wipeout Impulse Frames",
        initial: Value::Integer {
            value: 21,
            minimum: 0,
            maximum: 100,
        },
    },
    // 278: init82f90128/table82fd57d4/ref830d81f4; float1 range0..10, null callback.
    Definition {
        section: "Physics_Wipeout",
        label: "kWipeout_SlowSpeed",
        title: "SlowSpeed",
        initial: Value::Float(1.0),
    },
    // 279: init82f83900/table82fd4da0/ref830a4000; booltrue, null callback.
    Definition {
        section: "Animation_Skater",
        label: "kUseBump",
        title: "Use Bump",
        initial: Value::Boolean(true),
    },
    //280/281: init82f6c4f0/540, table82fd241c/2420; null callbacks.
    Definition {
        section: "Animation",
        label: "cv_NoHandAnimation",
        title: "Hands in BindPose",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Animation",
        label: "cv_NoFaceAnimation",
        title: "Face in Bindpose",
        initial: Value::Boolean(false),
    },
    // 282: init82f75d80..ee0/table82fd4094..40a0; null callback.
    Definition {
        section: "Input_Hardware_Vibrataion_360",
        label: "kLowFreqScale",
        title: "Low Wireless Freq Scale",
        initial: Value::Float(f32::from_bits(0x3f99_999a)),
    },
    // 283: init82f75d80..ee0/table82fd4094..40a0; null callback.
    Definition {
        section: "Input_Hardware_Vibrataion_360",
        label: "kLowFreqOffset",
        title: "Low Wireless Freq Offset",
        initial: Value::Float(f32::from_bits(0xbca3_d70a)),
    },
    // 284: init82f75d80..ee0/table82fd4094..40a0; null callback.
    Definition {
        section: "Input_Hardware_Vibrataion_360",
        label: "kHighFreqScale",
        title: "High Wireless Freq Scale",
        initial: Value::Float(f32::from_bits(0x3f86_6666)),
    },
    // 285: init82f75d80..ee0/table82fd4094..40a0; null callback.
    Definition {
        section: "Input_Hardware_Vibrataion_360",
        label: "kHighFreqOffset",
        title: "High Wireless Freq Offset",
        initial: Value::Float(f32::from_bits(0x3d4c_cccd)),
    },
    // 286: init82f7c200; table82fd4518. Range0..1, null callback.
    Definition {
        section: "Physics_Skeleton_Collision",
        label: "kSCDistToBeOnSamePlane",
        title: "A Dist to be same plane",
        initial: Value::Float(f32::from_bits(0x3cf5_c28f)),
    },
    // 287: init82f7c1a0; table82fd4514. Range-1..0, null callback.
    Definition {
        section: "Physics_Skeleton_Collision",
        label: "kSCDotValueToBeOpposing",
        title: "A Dot To Be Opposing",
        initial: Value::Float(f32::from_bits(0xbf7d_70a4)),
    },
    // 288: initializer82f8dd70, table82fd561c, reference830d6b64; null callback.
    Definition {
        section: "Animation",
        label: "cv_EnableBlendspacePrint",
        title: "Enable blendspace print info",
        initial: Value::Boolean(false),
    },
    // 289: initializer82f8ddc0, table82fd5620, reference830d6b1c; null callback.
    Definition {
        section: "Animation",
        label: "cv_EnableBlendspaceJobs",
        title: "Enable blendspace job use",
        initial: Value::Boolean(true),
    },
    // 290: initializer82f8de60, table82fd5628, reference830d6ac0; null callback.
    Definition {
        section: "Animation",
        label: "kUseBlendSpaceOptimizations",
        title: "kUseBlendSpaceOptimizations",
        initial: Value::Boolean(true),
    },
    // 291: initializer82f8deb0, table82fd562c, reference830d6ae4; null callback.
    Definition {
        section: "Animation",
        label: "kUseSlicingProjection3",
        title: "Use Slicing Projection Technique 3Dim",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_Simulation",
        label: "kEnableFreezing",
        title: "Enable Freezing",
        initial: Value::Boolean(true),
    },
    // initializer82f8b110, table82fd5398, reference830a6c54; null callback.
    Definition {
        section: "Debug_CharacterAssetManager",
        label: "kSchemaLoadFromBinary",
        title: "Load Binary Schema Files",
        initial: Value::Boolean(true),
    },
    //294: init82f5ce90/table82fd1578/ref8304e694; null callback.
    Definition {
        section: "Debug",
        label: "kEnableJuiceConnectionRetry",
        title: "Enable Juice Connection Retry",
        initial: Value::Boolean(false),
    },
    //295: initializer82fa0ea8/table82fd64dc/ref8311eb78; null callback.
    Definition {
        section: "Camera_Debug",
        label: "kStandingOffset",
        title: "Standing Offset",
        initial: Value::Float(f32::from_bits(0x3e99_999a)),
    },
    //296: init82f6bda0/table82fd23b8/ref830530c0; null callback.
    Definition {
        section: "ROPA_Core",
        label: "cv_EnableRopa",
        title: "Enable ROPA",
        initial: Value::Boolean(true),
    },
    //297: initializer82f6a490/table82fd2238; null callback.
    Definition {
        section: "Physics_Pools_SkaterPool",
        label: "kSkaterPoolUpdateInParallel",
        title: "Update In Parallel",
        initial: Value::Boolean(false),
    },
    //298: initializer82f56570/table82fd1008/ref83047e44; null callback.
    Definition {
        section: "Cheats",
        label: "kBouncyBigBlackEnabled",
        title: "Bouncy Big Black Enabled",
        initial: Value::Boolean(true),
    },
    //299: initializer82f6a620/table82fd224c; null callback.
    Definition {
        section: "Physics_MaxIterations",
        label: "kMaxSinglePlayerIterations",
        title: "Max Single Player Iterations",
        initial: Value::Integer {
            value: 50,
            minimum: 25,
            maximum: 200,
        },
    },
    //300: initializer82f6a678/table82fd2250; null callback.
    Definition {
        section: "Physics_MaxIterations",
        label: "kMaxOnlineIterations",
        title: "Max Online Iterations",
        initial: Value::Integer {
            value: 25,
            minimum: 25,
            maximum: 200,
        },
    },
    //301: initializer82f692a8/table82fd214c; null callback.
    Definition {
        section: "Physics_World",
        label: "kNumLineBatches",
        title: "Number of Line Batches",
        initial: Value::Integer {
            value: 65,
            minimum: 1,
            maximum: 100,
        },
    },
    //302: initializer82f69300/table82fd2150; null callback.
    Definition {
        section: "Physics_World",
        label: "kNumBoxBatches",
        title: "Number of Box Batches",
        initial: Value::Integer {
            value: 8,
            minimum: 1,
            maximum: 100,
        },
    },
    //303: initializer82f693b0/table82fd2158; null callback.
    Definition {
        section: "Physics_World",
        label: "kNumBatchTests",
        title: "Max number of tests per batch",
        initial: Value::Integer {
            value: 16,
            minimum: 1,
            maximum: 100,
        },
    },
    //304: initializer82f69408/table82fd215c; null callback.
    Definition {
        section: "Physics_World",
        label: "kNumSkaterTrajectoryBatches",
        title: "Number of Skater Traj Batches",
        initial: Value::Integer {
            value: 27,
            minimum: 1,
            maximum: 100,
        },
    },
    //305: initializer82fa1e58/table82fd65d0/ref8311ec20; null callback.
    Definition {
        section: "Camera_Debug_Replay",
        label: "kNumSimCameras",
        title: "kNumSimCameras",
        initial: Value::Integer {
            value: 1,
            minimum: 1,
            maximum: 1,
        },
    },
    //306: initializer82fa2708/table82fd6640/ref8311f58c; null callback.
    Definition {
        section: "Camera_Debug_GrindDetector",
        label: "kbEnableGrindDetector",
        title: "Enable grind detector",
        initial: Value::Boolean(true),
    },
    //307: initializer82fa2528/table82fd6628/ref8311f18c; null callback.
    Definition {
        section: "Camera_Debug",
        label: "kbShowFrontsideAngle",
        title: "Show frontside angle",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Animation_Skater_InAir",
        label: "kShowSkaterInAirDebugText",
        title: "Debug Text",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Animation_Skater_InAir",
        label: "kLegAdjustUp",
        title: "Enable Leg Adjust Up",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Animation_Skater_InAir",
        label: "kLegAdjustDown",
        title: "Enable Leg Adjust Down",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Animation_Skater_InAir",
        label: "kLegAdjustPreLand",
        title: "Enable Leg Adjust PreLand",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Animation_Skater_InAir",
        label: "kLegAdjustCustomPreLandHack",
        title: "Custom Preland Earlier",
        initial: Value::Float(f32::from_bits(0x3d75_c28f)),
    },
    Definition {
        section: "Animation_Skater_InAir",
        label: "kLegAdjustUseCustomPreLandHack",
        title: "Use Custom Preland Hack",
        initial: Value::Boolean(true),
    },
    //314: initializer82fa0f08/table82fd64e0/ref8311ed2c; null callback.
    Definition {
        section: "Camera_Debug",
        label: "kbLockShot",
        title: "Lock shot",
        initial: Value::Boolean(false),
    },
    //315: initializer82fa2008/table82fd65e8/ref8311edb4; null callback.
    Definition {
        section: "Camera_Debug",
        label: "kbNPCThreatTest",
        title: "NPC Threat Test",
        initial: Value::Boolean(false),
    },
    //316: initializer82fa1eb0/table82fd65d4/ref8311ed50; null callback.
    Definition {
        section: "Camera_Debug_DropPredictor",
        label: "kbDropTestRadius",
        title: "Radius of drop test line",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    //317: initializer82fa2348/table82fd6610/ref8311f168; null callback.
    Definition {
        section: "Camera_Debug",
        label: "kbShowMirroring",
        title: "Show mirroring",
        initial: Value::Boolean(false),
    },
    //318: initializer82fa26b8/table82fd663c/ref8311f61c; null callback.
    Definition {
        section: "Camera_Debug",
        label: "kbShowTurning",
        title: "Show turning values",
        initial: Value::Boolean(false),
    },
    //319: initializer82fa32c8/table82fd66d4/ref8311f0fc; raw r8=0 callback.
    Definition {
        section: "Camera_Debug",
        label: "kbShotDatabaseLiveChanges",
        title: "Shot database live changes",
        initial: Value::Boolean(false),
    },
    //320: initializer82fa2398/table82fd6614/ref8311f448; raw r8=0 callback.
    Definition {
        section: "Camera_Debug",
        label: "kbShowAnchorDebug",
        title: "Show anchor debug",
        initial: Value::Boolean(false),
    },
    //321: initializer82fa29a8/table82fd6660/ref8311ee6c; raw r8=0 callback.
    Definition {
        section: "Camera_Avoidance",
        label: "kbAvoidanceElevationEnabled",
        title: "Avoidance elevation enabled",
        initial: Value::Boolean(false),
    },
    //322: initializer82fa2c28/table82fd6680/ref8311f120; raw r8=0 callback.
    Definition {
        section: "Camera_Debug",
        label: "kbShowTrajectoryInfo",
        title: "Show trajectory info",
        initial: Value::Boolean(false),
    },
    //323: initializer82fa2e08/table82fd6698/ref8311f5f8; raw r8=0 callback.
    Definition {
        section: "Camera_Dynamics",
        label: "kbHeadingAccClamp",
        title: "Heading acceleration clamping",
        initial: Value::Boolean(true),
    },
    //324: initializer82fa2e58/table82fd669c/ref8311f520; raw r8=0 callback.
    Definition {
        section: "Camera_Dynamics",
        label: "kbHeadingSpeedClamp",
        title: "Heading speed clamping",
        initial: Value::Boolean(true),
    },
    //325: initializer82fa2ea8/table82fd66a0/ref8311f544; raw r8=0 callback.
    Definition {
        section: "Camera_Dynamics",
        label: "kbHeadingSmoothing",
        title: "Heading smoothing",
        initial: Value::Boolean(true),
    },
    //326: initializer82fa2ef8/table82fd66a4/ref8311f5d4; raw r8=0 callback.
    Definition {
        section: "Camera_Dynamics",
        label: "kbElevationAccClamp",
        title: "Elevation acceleration clamping",
        initial: Value::Boolean(true),
    },
    //327: initializer82fa2f48/table82fd66a8/ref8311f320; raw r8=0 callback.
    Definition {
        section: "Camera_Dynamics",
        label: "kbElevationSpeedClamp",
        title: "Elevation speed clamping",
        initial: Value::Boolean(true),
    },
    //328: initializer82fa2f98/table82fd66ac/ref8311f400; raw r8=0 callback.
    Definition {
        section: "Camera_Dynamics",
        label: "kbElevationSmoothing",
        title: "Elevation smoothing",
        initial: Value::Boolean(true),
    },
    //329: initializer82f4e1b8/table82fd0368/ref8303dbec; true/null callback.
    Definition {
        section: "Stream",
        label: "kEnableTextureStream",
        title: "Enable Texture Stream",
        initial: Value::Boolean(true),
    },
    //330: init82fa28a8/table82fd6654/ref8311f664; bits .1/.05/.5, rawr10=0.
    Definition {
        section: "Render_ClipPlane",
        label: "kNearClipPlaneDistance",
        title: "Near clip plane distance (m)",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    //331: init82fa2850/table82fd6650/ref8311eefc; rawr10=0 callback.
    Definition {
        section: "Camera_Debug",
        label: "kMaxPositionRetries",
        title: "Max positioning attempts",
        initial: Value::Integer {
            value: 18,
            minimum: 1,
            maximum: 36,
        },
    },
    //332: init82fa2958/table82fd665c/ref8311f224; false/rawr8=0 callback.
    Definition {
        section: "Camera",
        label: "kbHeavyweightHardCutForAllCameras",
        title: "Do heavyweight hard cut checking for all cameras",
        initial: Value::Boolean(false),
    },
    //333: init82fa2ae8/table82fd6670/ref8311f46c; false/rawr8=0 callback.
    Definition {
        section: "Camera_Debug",
        label: "kbShowTrackerDebug",
        title: "Show tracker debug",
        initial: Value::Boolean(false),
    },
    //334: init82fa2b38/table82fd6674/ref8311f4d8; false/rawr8=0 callback.
    Definition {
        section: "Camera_Debug",
        label: "kbShowAvoidancePosition",
        title: "Show position (avoidance)",
        initial: Value::Boolean(false),
    },
    //335: init82fa2b88/table82fd6678/ref8311f2fc; false/rawr8=0 callback.
    Definition {
        section: "Camera_Debug",
        label: "kbShowIntendedPosition",
        title: "Show position (intended)",
        initial: Value::Boolean(false),
    },
    //336: initializer82fa2fe8/table82fd66b0/ref8311f2b4; true/null callback.
    Definition {
        section: "Camera_Dynamics",
        label: "kbPitchAccClamp",
        title: "Pitch acceleration clamping",
        initial: Value::Boolean(true),
    },
    //337: initializer82fa3038/table82fd66b4/ref8311f5b0; false/null callback.
    Definition {
        section: "Camera_Dynamics",
        label: "kbPitchSpeedClamp",
        title: "Pitch speed clamping",
        initial: Value::Boolean(false),
    },
    //338: initializer82fa3088/table82fd66b8/ref8311f424; true/null callback.
    Definition {
        section: "Camera_Dynamics",
        label: "kbPitchSmoothing",
        title: "Pitch smoothing",
        initial: Value::Boolean(true),
    },
    //339: initializer82fa30d8/table82fd66bc/ref8311f568; true/null callback.
    Definition {
        section: "Camera_Dynamics",
        label: "kbYawAccClamp",
        title: "Yaw acceleration clamping",
        initial: Value::Boolean(true),
    },
    //340: initializer82fa3128/table82fd66c0/ref8311f640; false/null callback.
    Definition {
        section: "Camera_Dynamics",
        label: "kbYawSpeedClamp",
        title: "Yaw speed clamping",
        initial: Value::Boolean(false),
    },
    //341: initializer82fa3178/table82fd66c4/ref8311f290; true/null callback.
    Definition {
        section: "Camera_Dynamics",
        label: "kbYawSmoothing",
        title: "Yaw smoothing",
        initial: Value::Boolean(true),
    },
    //342: initializer82f569f8..82f56a30/table82fd1044; true/null callback.
    Definition {
        section: "Physics_Debug",
        label: "kbEnableAdjustSpawnPosition",
        title: "Enable Adjust Spawn Position",
        initial: Value::Boolean(true),
    },
    //343: initializer82f85bc0/table82fd4eb0, between SlideDebug and RetrieveYaw.
    Definition {
        section: "Animation",
        label: "kbDrawTrickHeightDebugInfo",
        title: "kbDrawTrickHeightDebugInfo",
        initial: Value::Boolean(false),
    },
    //344: initializer82f85fe8/table82fd4ed8/ref830a4570, null callback.
    Definition {
        section: "Animation_Skater",
        label: "kCapDvAtTopSpeed",
        title: "Cap push acc. at top speed",
        initial: Value::Boolean(false),
    },
    //345: initializer82f838a0/table82fd4d9c/ref read82caabb0, no callback.
    Definition {
        section: "Animation_Skater",
        label: "kMongoPushToAnticThresh",
        title: "MongoPush to Antic thresh",
        initial: Value::Float(-0.5),
    },
    //346: initializer82f92af8/table82fd5998/ref82d9c2d8, null callback.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kOB_MaxLandingSpeedForBlend",
        title: "Max Landing Speed Blend",
        initial: Value::Float(10.0),
    },
    //347: initializer82f988a0/table82fd5df0/ref830db2c4; range0..1, null callback.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kOB_IK_MaxFeetYDist",
        title: "Max Y IK Dist",
        initial: Value::Float(f32::from_bits(0x3e99_999a)),
    },
    //348: initializer82f86038/table82fd4edc, live Begin82cd6534/Update82cd6880.
    Definition {
        section: "Animation_Skater",
        label: "kAllowHolding",
        title: "Allow Push Holding",
        initial: Value::Boolean(true),
    },
    //349: initializer82fa21a8/table82fd65fc; null callback.
    Definition {
        section: "Camera_Avoidance",
        label: "kbPedestrianAvoidance",
        title: "Pedestrian avoidance",
        initial: Value::Boolean(true),
    },
    //350: initializer82fa21f8/table82fd6600; null callback.
    Definition {
        section: "Camera_Avoidance",
        label: "kbSkaterAvoidance",
        title: "Skater avoidance",
        initial: Value::Boolean(true),
    },
    //351: initializer82fa2248/table82fd6604; source range0..100, null callback.
    Definition {
        section: "Camera_Avoidance",
        label: "kActorSearchRadius",
        title: "Actor search radius",
        initial: Value::Float(10.0),
    },
    //352: initializer82fa29f8/table82fd6664; null callback.
    Definition {
        section: "Camera_Avoidance",
        label: "kbInterleaveAvoidance",
        title: "Interleave avoidance",
        initial: Value::Boolean(true),
    },
    //353: initializer82fa2a48/table82fd6668; null callback.
    Definition {
        section: "Camera_Avoidance",
        label: "kbEnableCollisionAvoidance",
        title: "Enable collision avoidance",
        initial: Value::Boolean(true),
    },
    //354: initializer82fa2c78/table82fd6684; null callback.
    Definition {
        section: "Camera_Debug",
        label: "kbShowAvoidanceDebug",
        title: "Show avoidance debug",
        initial: Value::Boolean(false),
    },
    //355: initializer82fa31c8/table82fd66c8; source range0..0.5, null callback.
    Definition {
        section: "Camera_Dynamics_Settings",
        label: "kInsignificantSpeed",
        title: "Insignificant speed",
        initial: Value::Float(f32::from_bits(0x3e19_999a)),
    },
    //356: initializer82f82fe0/table82fd4d14/ref830a4d04; null callback.
    Definition {
        section: "Animation_Skater",
        label: "kUseDebugAnimationsMode",
        title: "Debug Animations Mode",
        initial: Value::Boolean(false),
    },
    // 357: initializer82f895a8/table82fd51f0/ref830a6200; null callback.
    Definition {
        section: "BreadCrumbs",
        label: "kbEnableNavMeshConstraints",
        title: "Enable NavMesh Constraints",
        initial: Value::Boolean(true),
    },
    // 358: initializer82f895f8/table82fd51f4/ref830a61c4; null callback.
    Definition {
        section: "BreadCrumbs",
        label: "kEnableWipeoutSpotBreadCrumb",
        title: "Enable Wipeout Spot Bread Crumb",
        initial: Value::Boolean(true),
    },
    // 359/360: initializers82f7bb40/bba0, tables82fd4488/448c.
    Definition {
        section: "Physics_Skateboard_Drives",
        label: "kSkateboardLinSoftDriveDisp",
        title: "Linear Soft Disp",
        initial: Value::Float(5000.0),
    },
    Definition {
        section: "Physics_Skateboard_Drives",
        label: "kSkateboardLinSoftDriveStr",
        title: "Linear Soft Str",
        initial: Value::Float(0.5),
    },
    // 361..363: initializers82f985f0/8640/86a0, tables82fd5dd0/d4/d8.
    Definition {
        section: "Physics_Skateboard_Controller",
        label: "kSKC_HandDriveDisp",
        title: "Hand Drive Dsp",
        initial: Value::Float(5.0),
    },
    Definition {
        section: "Physics_Skateboard_Controller",
        label: "kSKC_HandDriveStr",
        title: "Hand Drive Str",
        initial: Value::Float(5.0),
    },
    Definition {
        section: "Physics_Skateboard_Controller",
        label: "kSKC_UseSoftHook",
        title: "Use Hook Soft",
        initial: Value::Boolean(true),
    },
    // 364: initializer82f56830/table82fd102c/ref83047f68; range0..10, null callback.
    Definition {
        section: "BreadCrumbs",
        label: "kfInsignificantSquareVelocity",
        title: "Insignificant Square Velocity",
        initial: Value::Float(0.25),
    },
    // 365: initializer82f98f98/table82fd5e3c/ref830db160; range0..1, null callback.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kOBIK_IKMaxYFootNormal",
        title: "IK MaxYFootNormal",
        initial: Value::Float(f32::from_bits(0x3f66_6666)),
    },
    // 366: initializer82f98ff8/table82fd5e40/ref830db840; range-1..1, null callback.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kYHeightToBeOffGround",
        title: "kYHeightToBeOffGround",
        initial: Value::Float(f32::from_bits(0x3db8_51ec)),
    },
    // 367: initializer82f89550/table82fd51ec; range0..1200, null callback.
    Definition {
        section: "BreadCrumbs",
        label: "kMinTicksAfterResetForBreadCrumb",
        title: "Min Ticks After Reset For BreadCrumb",
        initial: Value::Integer {
            value: 120,
            minimum: 0,
            maximum: 1200,
        },
    },
    // 368: initializer82f98d58/table82fd5e24/ref830db4f8; range-1..1.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kYMinHeightToBeOffGround",
        title: "Foot Thickness",
        initial: Value::Float(f32::from_bits(0x3ca3_d70a)),
    },
    // 369: initializer82f98ed8/table82fd5e34/ref830db18c; range0..1.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kOBIK_IKMaxYDistUp",
        title: "IK MaxYDist",
        initial: Value::Float(f32::from_bits(0x3ecc_cccd)),
    },
    // 370: initializer82f98f38/table82fd5e38/ref830db598; range0..1.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kOBIK_IKMaxYLiftUp",
        title: "IK MaxYLift",
        initial: Value::Float(f32::from_bits(0x3da3_d70a)),
    },
    // 371: initializer82f98e78/table82fd5e30; range0..1.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kOBIK_TimeToBlendOutFixedFootPos",
        title: "Fixed Foot BlendOut Time",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    // 372: initializer82f98800/table82fd5de8; native default true.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kEnableOffBoardFeetIK",
        title: "Enable FeetIK",
        initial: Value::Boolean(true),
    },
    // 373: initializer82f98db8/table82fd5e28; range0..1.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kOBIK_IKBlendVal",
        title: "IK BlendVal",
        initial: Value::Float(f32::from_bits(0x3ecc_cccd)),
    },
    // 374: initializer82f7dac8/table82fd465c; range0..1.
    Definition {
        section: "Physics_Skeleton_IK",
        label: "kMaxNormalAngleDelta",
        title: "Max Norm Blend Delta",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    // 375: initializer82f92d20/table82fd59b0; range0.001..1.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kOB_MaxSkeletonDisp",
        title: "Max Allowable Skel Disp",
        initial: Value::Float(f32::from_bits(0x3c23_d70a)),
    },
    // 376: initializer82f92d80/table82fd59b4; range0.000001..1.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kOB_MinTrajW",
        title: "Min Traj",
        initial: Value::Float(f32::from_bits(0x3ba3_d70a)),
    },
    // 377: initializer82f92de0/table82fd59b8; native default false.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kOB_DebugCollResponseA",
        title: "Debug Coll Response A",
        initial: Value::Boolean(false),
    },
    // 378: initializer82f92b58/table82fd599c; native default true.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kDoCOMVelBlendOut",
        title: "Do Com blendout",
        initial: Value::Boolean(true),
    },
    // 379: initializer82f92ba8/table82fd59a0; range0..30.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kFramesToBlendOutStartingComVel",
        title: "Frames To Blend Vel",
        initial: Value::Integer {
            value: 10,
            minimum: 0,
            maximum: 30,
        },
    },
    // 380: initializer82f7ce88/table82fd45cc/ref8308e868; range0.001..1.
    Definition {
        section: "Physics_Skeleton",
        label: "kMaxSkeletonDisp",
        title: "Max Allowable Skel Disp",
        initial: Value::Float(f32::from_bits(0x3c23_d70a)),
    },
    // 381: initializer82f7cee8/table82fd45d0; range0..1.
    Definition {
        section: "Physics_Skeleton",
        label: "kCollisionPushFraction",
        title: "Collision repulsion fraction",
        initial: Value::Float(f32::from_bits(0x3e4c_cccd)),
    },
    // 382: ref830dd070/init82f9bf78/table82fd6058; integer range0..60.
    Definition {
        section: "Physics_Teleport",
        label: "kFramesForTeleportCountdown",
        title: "Frames To Stay in state",
        initial: Value::Integer {
            value: 10,
            minimum: 0,
            maximum: 60,
        },
    },
    // 383: ref8308e648/init82f7cf98/table82fd45d8; float range0..1.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kOB_GroundMaxDelta",
        title: "Ground Max Delta",
        initial: Value::Float(f32::from_bits(0x3d23_d70a)),
    },
    // 384: ref8308e588/init82f7cff8/table82fd45dc; float range0..1.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kOB_GroundPropBlend",
        title: "Ground Pos Blend",
        initial: Value::Float(f32::from_bits(0x3e4c_cccd)),
    },
    // 385: init82f92e90/table82fd59c0.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kEnableOffBoardWipeouts",
        title: "Enable OB wipeout",
        initial: Value::Boolean(true),
    },
    // 386: init82f9c3c0/table82fd608c.
    Definition {
        section: "Physics_Wipeouts_Debug",
        label: "kWipeoutDisplayOffBoardInfo",
        title: "Display Offboard Info",
        initial: Value::Boolean(false),
    },
    // 387: init82f9c500/table82fd609c.
    Definition {
        section: "Physics_Wipeouts",
        label: "kWipeoutOffboardEnabled",
        title: "Offboard Enabled",
        initial: Value::Boolean(true),
    },
    // 388: ref8308dd10/init82f7b9c0/table82fd4478; float range0..1.
    Definition {
        section: "Physics_Skateboard",
        label: "kFreeSkateboardAngularDamping",
        title: "Free Skateboard Intertia",
        initial: Value::Float(0.25),
    },
    // 389: init82f8de10/table82fd5624/reference830d6b40; null callback.
    Definition {
        section: "Animation",
        label: "kUseSlicingProjectionN",
        title: "Use Slicing Projection Technique nDim",
        initial: Value::Boolean(true),
    },
    // 390: initializer82f86088/table82fd4ee0, null callback.
    Definition {
        section: "Debug_Physics",
        label: "kUseFineKickturn",
        title: "Use Fine Kickturn",
        initial: Value::Boolean(false),
    },
    // 391: initializer82f860d8/table82fd4ee4, null callback; GUI range0..1.
    Definition {
        section: "Controller_KickTurn",
        label: "kKickTurnSmoothing",
        title: "Smoothing",
        initial: Value::Float(0.5),
    },
    //392: init82f7cbc8/table82fd45ac/ref8308ee70, null callback.
    Definition {
        section: "Physics_Skeleton",
        label: "kMaxProxyYDist",
        title: "Max Proxy Y Adjust",
        initial: Value::Float(f32::from_bits(0x3e99_999a)),
    },
    //393: init82f98c98/table82fd5e1c/ref830dbc5c, null callback.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kSlopeEffectScalar",
        title: "Slope Effect Scalar",
        initial: Value::Float(f32::from_bits(0x3e4c_cccd)),
    },
    //394: init82f7cf48/table82fd45d4/ref8308ec50, null callback.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kDrawOffBoardAirSkeleton",
        title: "Show Air Skeleton",
        initial: Value::Boolean(false),
    },
    //395..399: table82fd5ca0/a4/a8/ac/b8, null callbacks.
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kbOB_AIR_DebugDraw",
        title: "Debug Draw Trajectories",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kOB_AIR_TrajectoryMaxTime",
        title: "TrajectoryMaxTime",
        initial: Value::Float(2.0),
    },
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kOB_AIR_TrajectoryMaxErrorStart",
        title: "TrajectoryMaxErrorStart",
        initial: Value::Float(0.25),
    },
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kOB_AIR_TrajectoryMaxErrorEnd",
        title: "TrajectoryMaxErrorEnd",
        initial: Value::Float(f32::from_bits(0x3f99_999a)),
    },
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kOB_Debug_Traj_Fail",
        title: "Debug Traj Fail",
        initial: Value::Boolean(false),
    },
    //400: initializer82f99058, null callback.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kbLandingOnDeck_DebugDraw",
        title: "Debug Draw",
        initial: Value::Boolean(false),
    },
    //401: initializer82f990a8, null callback.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kbLandingOnDeck_DebugDrawTraj",
        title: "Debug Draw Traj Test",
        initial: Value::Boolean(false),
    },
    //402: initializer82f990f8, null callback.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kLOD_MinTimeToJumpOnDeck",
        title: "Min time to land on deck",
        initial: Value::Float(f32::from_bits(0x3ecc_cccd)),
    },
    //403: initializer82f99158, null callback.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kLOD_MinTimeAdjustProp",
        title: "Min time adjust prop",
        initial: Value::Float(0.75),
    },
    //404: initializer82f991b8, null callback.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kLOD_MaxTimeAdjustProp",
        title: "Max time adjust prop",
        initial: Value::Float(f32::from_bits(0x3fa6_6666)),
    },
    //405: initializer82f99378, null callback.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kLOD_TrajOffSetY",
        title: "kTrajOffSetY",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    //406: initializer82f993d8, null callback.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kLOD_TrajRadius",
        title: "kTrajRadius",
        initial: Value::Float(f32::from_bits(0x3e99_999a)),
    },
    //407: initializer82f99438, null callback.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kLOD_TrajError",
        title: "TrajError",
        initial: Value::Float(0.5),
    },
    //408: initializer82f992b8, null callback.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kMaxLandingError",
        title: "Max Landing Error",
        initial: Value::Float(0.5),
    },
    //409: initializer82f99318, null callback.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kMaxLandingErrorAI",
        title: "Max Landing Error (AI)",
        initial: Value::Float(f32::from_bits(0x3f0c_cccd)),
    },
    //410: initializer82f93ad0/table82fd5a4c; null callback.
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kRunoutUpwardVelocityNudge",
        title: "Runout Upward Nudge",
        initial: Value::Float(2.8),
    },
    //411: initializer82f93b30/table82fd5a50; null callback.
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kRunoutZXVelocityReduction",
        title: "Runout ZX Scalar",
        initial: Value::Float(0.9),
    },
    //412: initializer82f93b90/table82fd5a54; null callback.
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kJumpForwardDistance",
        title: "Jump Forward Modifier",
        initial: Value::Float(1.0),
    },
    //413: initializer82f93bf0/table82fd5a58; null callback.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kOffboardAirMaxLandOnBoardAssist",
        title: "Offboard Air Max Land on Board assist",
        initial: Value::Float(0.5),
    },
    //414: initializer82f93c50/table82fd5a5c; null callback.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kStandingJumpMaxLandOnBoardAssist",
        title: "Standing Jump Max Land on Board assist",
        initial: Value::Float(1.0),
    },
    //415: initializer82f93cb0/table82fd5a60; null callback.
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kRunoutTransitionTime",
        title: "Runout Trans time",
        initial: Value::Float(0.2),
    },
    //416: initializer82f93d10/table82fd5a64; null callback.
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kMinTimeInAirForRunoff",
        title: "Min time in state",
        initial: Value::Float(0.1),
    },
    //417: initializer82f93d70/table82fd5a68; null callback.
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kRunoutTimeToLandScalar",
        title: "TimeToLand Scalar",
        initial: Value::Float(0.5),
    },
    //418: initializer82f93dd0/table82fd5a6c; null callback.
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kRunOutMinAngleForWipeoutAir",
        title: "Angle Air Wipeout",
        initial: Value::Float(120.0),
    },
    //419: initializer82f93e30/table82fd5a70; null callback.
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kRunOutMinAngleForWipeout",
        title: "MinAngle Wipeout",
        initial: Value::Float(55.0),
    },
    //420: initializer82f93e90/table82fd5a74; null callback.
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kOffBoarAirYSpeedForWipeout",
        title: "YSpeed Wipeout",
        initial: Value::Float(-10.0),
    },
    //421: initializer82f93ef0/table82fd5a78; null callback.
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kOBAir_MinUpVectorY",
        title: "Min Up Vect Y",
        initial: Value::Float(0.9),
    },
    //422: initializer82f93f50/table82fd5a7c; null callback.
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kRunoutProportionalAngle",
        title: "Proportional Angle",
        initial: Value::Float(0.2),
    },
    //423: initializer82f93fb0/table82fd5a80; null callback.
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kRunOutMaxAngleForLocomotion",
        title: "Max Angle Locomotion",
        initial: Value::Float(10.0),
    },
    //424: initializer82f94010/table82fd5a84; null callback.
    Definition {
        section: "Physics_States_OffboardAir",
        label: "kMinTimeInStateForSkateboardWipeout",
        title: "Min t for deck wipe o",
        initial: Value::Float(0.1),
    },
    //425: initializer82f99218, null callback.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kUseAccurateIKError",
        title: "Use AC IK Error",
        initial: Value::Boolean(true),
    },
    //426: initializer82f99268, null callback.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kLOD_ZeroY",
        title: "Zero Y",
        initial: Value::Boolean(true),
    },
    //427: initializer82f6a918; null callback.
    Definition {
        section: "Physics_OffBoard",
        label: "kbDisplayBadTriangleInfo",
        title: "Display Bad Triangle Info",
        initial: Value::Boolean(false),
    },
    //428: initializer82f6a968; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kVertMinYTriangleNormal",
        title: "Min Y for a vert tri",
        initial: Value::Float(f32::from_bits(0x3e99999a)),
    },
    //429: initializer82f6a9c8; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kAdditionalGapFromVertTri",
        title: "Additional Dist From Vert",
        initial: Value::Float(f32::from_bits(0x3dcccccd)),
    },
    //430: initializer82f6aa28; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kGeomAnalyzerMinBadY",
        title: "Geom MinBadY",
        initial: Value::Float(f32::from_bits(0xbc23d70a)),
    },
    //431: initializer82f6ab08; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kSSTestRadius",
        title: "Test Radius",
        initial: Value::Float(f32::from_bits(0x3e4ccccd)),
    },
    //432: initializer82f6ab68; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kSSTestHeight",
        title: "Test Height",
        initial: Value::Float(f32::from_bits(0x3f800000)),
    },
    //433: initializer82f6abc8; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kSSStepRadius",
        title: "Step Radius",
        initial: Value::Float(f32::from_bits(0x3da3d70a)),
    },
    //434: initializer82f6ac28; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kSSNormYThreshold",
        title: "Norm Y Threshold",
        initial: Value::Float(f32::from_bits(0x3f4ccccd)),
    },
    //435: initializer82f6ace8; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kSSTrajMaxTime",
        title: "Traj Max Time",
        initial: Value::Float(f32::from_bits(0x40000000)),
    },
    //436: initializer82f6ad48; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kSSTrajectoryRadius",
        title: "Trajectory Radius",
        initial: Value::Float(f32::from_bits(0x3d4ccccd)),
    },
    //437: initializer82f6ada8; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kNbTrajWhenSprinting",
        title: "kNbTrajWhenSprinting",
        initial: Value::Integer {
            value: 3,
            minimum: 1,
            maximum: 5,
        },
    },
    //438: initializer82f8e120; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kDistToDeathJumpDown",
        title: "Death Jump To Contact Dist",
        initial: Value::Float(f32::from_bits(0x40000000)),
    },
    //439: initializer82f8e180; null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kShoeOffset",
        title: "Shoe Offset size",
        initial: Value::Float(f32::from_bits(0x3c75c28f)),
    },
    //440: initializer82f8e240; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kGrindVerticalSinAngle",
        title: "kGrindVerticalSinAngle",
        initial: Value::Float(f32::from_bits(0x3f4ccccd)),
    },
    //441: initializer82f8e2a0; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kReduceSpeedAcceptRatio",
        title: "kReduceSpeedAcceptRatio",
        initial: Value::Float(f32::from_bits(0x3f333333)),
    },
    //442: initializer82f8e300; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kGrindAddedDistTest",
        title: "kGrindaddedDistRatio",
        initial: Value::Float(f32::from_bits(0x3f8ccccd)),
    },
    //443: initializer82f8e360; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kMinDistToSamePoint",
        title: "kMinDistToSamePoint",
        initial: Value::Float(f32::from_bits(0x3d4ccccd)),
    },
    //444: initializer82f8e3c0; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kCosSpreadAngle",
        title: "kCosSpreadAngle",
        initial: Value::Float(f32::from_bits(0x3f7ae148)),
    },
    //445: initializer82f8e420; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kGrindBoxHalfSize",
        title: "kGrindBoxHalfSize",
        initial: Value::Float(f32::from_bits(0x40000000)),
    },
    //446: initializer82f8e480; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kAcceptableDistFromDest",
        title: "kAcceptableDistFromDest",
        initial: Value::Float(f32::from_bits(0x3e19999a)),
    },
    //447: initializer82f8e4e0; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kHeightMargin",
        title: "kHeightMargin",
        initial: Value::Float(f32::from_bits(0x3d4ccccd)),
    },
    //448: initializer82f8e5a0; null callback.
    Definition {
        section: "Physics_OffBoard_StepSelector",
        label: "kGrindLowStepHeight",
        title: "kGrindLowStepHeight",
        initial: Value::Float(f32::from_bits(0x3e19999a)),
    },
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kbLOD_State_DebugDraw",
        title: "Debug Draw State",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kbLOD_EnableFeetIK",
        title: "Enable Feet IK",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kLOD_MinDotFotFeetIK",
        title: "Min Dot For IK",
        initial: Value::Float(f32::from_bits(0x3f7851ec)),
    },
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kLOD_TimeToEnableIk",
        title: "Time to Enable IK",
        initial: Value::Float(f32::from_bits(0x3e4ccccd)),
    },
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kLOD_EnableSpin",
        title: "Enable Spin",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kLOD_AngleTransitionTime",
        title: "Transition Time",
        initial: Value::Float(f32::from_bits(0x3e8f5c29)),
    },
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kLOD_TimeForLandingWipeOut",
        title: "T ForLandingWipeO",
        initial: Value::Float(f32::from_bits(0x3d8f5c29)),
    },
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kbLOD_DeckYOffset",
        title: "deck Y offet",
        initial: Value::Float(f32::from_bits(0xbe19999a)),
    },
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kbLOD_WipeoutTime",
        title: "wipeout time to land",
        initial: Value::Float(f32::from_bits(0x3ca3d70a)),
    },
    //458: initializer82f984f0, .data82fd5dc4, null callback.
    Definition {
        section: "Physics_Cheats",
        label: "kSKC_GrabCheat",
        title: "Board Retrieval Fun",
        initial: Value::Boolean(false),
    },
    //459: initializer82fa23e8/table82fd6618/ref8311efb8; null callback.
    Definition {
        section: "Camera_Debug",
        label: "kbShowTransferAngleDebug",
        title: "Show transfer angle debug",
        initial: Value::Boolean(false),
    },
    //460:82f7cc78/table82fd45b4, r10=0 callback, min0/max1.
    Definition {
        section: "Animation_Locomotion",
        label: "kSkel_SpeedScalarOnEnterLoc",
        title: "SpeedScalar on LocEnter",
        initial: Value::Float(1.0),
    },
    //461:82f92f30/table82fd59c8, r10=0 callback, min0/max20.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kMaxLandOnBoardAssist",
        title: "Max Land on Board assist",
        initial: Value::Float(2.0),
    },
    //462: initializer82fa2488/table82fd6620/ref8311ee90; null callback.
    Definition {
        section: "Camera_Debug",
        label: "kbShowAirCollision",
        title: "Show air collision tests",
        initial: Value::Boolean(false),
    },
    //463: initializer82fa2758/table82fd6644/ref8311ef70; null callback.
    Definition {
        section: "Camera_Debug",
        label: "kbEnableAirCameraCollision",
        title: "Enable Air Camera Collision",
        initial: Value::Boolean(false),
    },
    //464: initializer82f81548/table82fd4938; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kCorrectionAngleSign",
        title: "kCorrectionAngleSign",
        initial: Value::Float(f32::from_bits(0xbf800000)),
    },
    //465: initializer82f81388/table82fd4924; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kEnableJumpHeightAdjust",
        title: "Enable Jump COM Height Adjust",
        initial: Value::Boolean(true),
    },
    //466: initializer82f81428/table82fd492c; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kJumpHeight",
        title: "Jump Height",
        initial: Value::Float(f32::from_bits(0x3fd9999a)),
    },
    //467: initializer82f81488/table82fd4930; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kTimeRatio",
        title: "kTimeRatio",
        initial: Value::Float(f32::from_bits(0x3f400000)),
    },
    //468: initializer82f81158/table82fd490c; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kDisplayLocomotionDebug",
        title: "Display Debug Info",
        initial: Value::Boolean(false),
    },
    //469: initializer82f814e8/table82fd4934; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "k2ndDegreEqRatio",
        title: "k2ndDegreEqRatio",
        initial: Value::Float(1.0),
    },
    //470: initializer82f80b48/table82fd48c4; raw r10=null callback.
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kAdditionalAngleAway",
        title: "AdditionalAngleDegs",
        initial: Value::Float(0.0),
    },
    //471: initializer82f80ba8/table82fd48c8; raw r10=null callback.
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kOBCOLL_MaxTurnAngleDegs",
        title: "Max Turn Per Frame Degs",
        initial: Value::Float(2.5),
    },
    //472: initializer82f80c08/table82fd48cc; raw r10=null callback.
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kOBCOLL_MaxAutoStartTurnAngle",
        title: "Max Angle to start",
        initial: Value::Float(60.0),
    },
    //473: initializer82f80c68/table82fd48d0; raw r10=null callback.
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kOBCOLL_MaxAutoStartTurnAngleAir",
        title: "Max Angle to start Air",
        initial: Value::Float(15.0),
    },
    //474: initializer82f7fbe0; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kUseSprintButton",
        title: "Use Sprint Button",
        initial: Value::Boolean(true),
    },
    //475: initializer82f807e8; raw null callback.
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kOB_MaxCollisionDisp",
        title: "Max displacment",
        initial: Value::Float(f32::from_bits(0x3cf5c28f)),
    },
    //476: initializer82f80a28; raw null callback.
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kOBCOLL_DisplacementBlend",
        title: "Displacement Blend",
        initial: Value::Float(f32::from_bits(0x3f666666)),
    },
    //477: initializer82f80a88; raw null callback.
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kMaxTimeToAvoidCollision",
        title: "Time To Avoid Collision",
        initial: Value::Float(f32::from_bits(0x3e2e147b)),
    },
    //478: initializer82f80ae8; raw null callback.
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kOBCOLL_MinDisp",
        title: "Min Disp",
        initial: Value::Float(f32::from_bits(0x3ba3d70a)),
    },
    //479: initializer82f80908; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kSkaterAnimMaxRawTime",
        title: "Max Raw Time",
        initial: Value::Float(f32::from_bits(0x3ca3d70a)),
    },
    //480: initializer82f812c8; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kLocomotionStartYOffset",
        title: "Base Start Y Offset",
        initial: Value::Float(0.5),
    },
    //481: initializer82f81328; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kLocomotionMinApexHeight",
        title: "Min Apex Height",
        initial: Value::Float(f32::from_bits(0x3dcccccd)),
    },
    //482: initializer82f7ff10; raw null callback.
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kMaxFramesForStepSelectorToFail",
        title: "Max Frames For Fail",
        initial: Value::Integer {
            value: 3,
            minimum: 0,
            maximum: 120,
        },
    },
    //483: initializer82f80848; raw null callback.
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kOBCOLL_DebugSpreadAngleMin",
        title: "Spread Angle Min",
        initial: Value::Float(5.0),
    },
    //484: initializer82f808a8; raw null callback.
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kOBCOLL_DebugSpreadAngleMax",
        title: "Spread Angle Max",
        initial: Value::Float(25.0),
    },
    //485: initializer82f80968; raw null callback.
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kJumpMaxStepHeight",
        title: "Jump Max Height",
        initial: Value::Float(f32::from_bits(0x3f99999a)),
    },
    //486: initializer82f809c8; raw null callback.
    Definition {
        section: "Animation_Locomotion_Collision",
        label: "kRunMaxStepHeight",
        title: "Run Max Height",
        initial: Value::Float(f32::from_bits(0x3f19999a)),
    },
    //487: initializer82f803d8/table82fd4874; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kDistTresh4Oscil",
        title: "kDistTresh4Oscil",
        initial: Value::Float(f32::from_bits(0x3e99999a)),
    },
    //488: initializer82f80798/table82fd489c; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kEnableLocomotionFutureAssert",
        title: "Enable Future ASSERT",
        initial: Value::Boolean(true),
    },
    //489: initializer82f80438/table82fd4878; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kMaxDecelStepRatio",
        title: "kMaxDecelStepRatio",
        initial: Value::Float(f32::from_bits(0x3e99999a)),
    },
    //490: initializer82f80738/table82fd4898; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kCorrectionTurnSign",
        title: "kCorrectionTurnSign",
        initial: Value::Float(1.0),
    },
    //491: initializer82f804f8/table82fd4880; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kMaxDecelStop",
        title: "Max Deceleration Stop Now",
        initial: Value::Float(1.0),
    },
    //492: initializer82f80498/table82fd487c; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kMaxDecel",
        title: "Max Deceleration",
        initial: Value::Float(f32::from_bits(0x3e19999a)),
    },
    //493: initializer82f80558/table82fd4884; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kMaxAccel",
        title: "Max Acceleration per Frame",
        initial: Value::Float(f32::from_bits(0x3e4ccccd)),
    },
    //494: initializer82f80388/table82fd4870; raw null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kLocColl_DebugDraw",
        title: "General Debug Draw",
        initial: Value::Boolean(false),
    },
    // 495: native initializer0x82f7ff68, raw r10=null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kInclineAngle",
        title: "Incline angle",
        initial: Value::Float(f32::from_bits(0x40900000)),
    },
    // 496: native initializer0x82f800e8, raw r10=null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kStairsSlopeTresh",
        title: "Stairs Slope Tresh",
        initial: Value::Float(f32::from_bits(0x3e19999a)),
    },
    // 497: native initializer0x82f80088, raw r10=null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kInclineSlopeTresh",
        title: "Incline Slope Tresh",
        initial: Value::Float(f32::from_bits(0x3d8f5c29)),
    },
    // 498: native initializer0x82f801a8, raw r10=null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kAutoJumpTresh",
        title: "Auto Jump down Tresh",
        initial: Value::Float(f32::from_bits(0x3f333333)),
    },
    // 499: native initializer0x82f80208, raw r10=null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kLand2FTTresh",
        title: "Land 2FT Speed Tresh",
        initial: Value::Float(f32::from_bits(0x40900000)),
    },
    // 500: native initializer0x82f80328, raw r10=null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "zInclineTresh",
        title: "Z Incline Tresh",
        initial: Value::Float(f32::from_bits(0x40133333)),
    },
    // 501: native initializer0x82f802c8, raw r10=null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "zStairsTresh",
        title: "Z Stairs Tresh",
        initial: Value::Float(f32::from_bits(0x3fd9999a)),
    },
    // 502: native initializer0x82f7ffc8, raw r10=null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kInclineMaxSlopeDown",
        title: "Incline max slope DOWN",
        initial: Value::Float(f32::from_bits(0x3e99999a)),
    },
    // 503: native initializer0x82f80028, raw r10=null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kInclineMaxSlopeUp",
        title: "Incline max slope UP",
        initial: Value::Float(f32::from_bits(0x3f000000)),
    },
    // 504: native initializer0x82f80148, raw r10=null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kJumpStairsTresh",
        title: "kJumpStairsTresh",
        initial: Value::Float(f32::from_bits(0x3f99999a)),
    },
    // 505: native initializer0x82f80268, raw r10=null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kLand1FTTresh",
        title: "Land 1FT Speed Tresh",
        initial: Value::Float(f32::from_bits(0x40366666)),
    },
    //506: initializer82f86618/table82fd4f24; raw r10=null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kTwoFootLandingImpactThreshold",
        title: "Impact for 2-Foot Landing From Offboard Air",
        initial: Value::Float(f32::from_bits(0x3f66_6666)),
    },
    //507: initializer82f99508; raw r10=0 callback.
    Definition {
        section: "Physics_Systems_OBLocationTest",
        label: "kOBLT_TestRadius",
        title: "Line Test Radius",
        initial: Value::Float(f32::from_bits(0x3e28f5c3)),
    },
    //508: initializer82f99568; raw r10=0 callback.
    Definition {
        section: "Physics_Systems_OBLocationTest",
        label: "kOBLT_NumLines",
        title: "Num Line Tests",
        initial: Value::Integer {
            value: 3,
            minimum: 1,
            maximum: 10,
        },
    },
    //509: initializer82f995c0; raw r10=0 callback.
    Definition {
        section: "Physics_Systems_OBLocationTest",
        label: "kOBLT_StartHeight",
        title: "Line Start Height",
        initial: Value::Float(f32::from_bits(0x3f333333)),
    },
    //510: initializer82f99620; raw r10=0 callback.
    Definition {
        section: "Physics_Systems_OBLocationTest",
        label: "kOBLT_LineLength",
        title: "Line Length",
        initial: Value::Float(f32::from_bits(0x3f0ccccd)),
    },
    //511: initializer82f99680; raw r10=0 callback.
    Definition {
        section: "Physics_Systems_OBLocationTest",
        label: "kOBLT_MinNormalDotTurn",
        title: "MinDotTurn",
        initial: Value::Float(f32::from_bits(0xbf666666)),
    },
    //512: initializer82f996e0; raw r10=0 callback.
    Definition {
        section: "Physics_Systems_OBLocationTest",
        label: "kOBLT_MinYNormalTest",
        title: "MinYNormalTest",
        initial: Value::Float(f32::from_bits(0x3e99999a)),
    },
    //513: initializer82f99740; raw r10=0 callback.
    Definition {
        section: "Physics_Systems_OBLocationTest",
        label: "kOBLT_TurnSpeed",
        title: "Turn Speed",
        initial: Value::Float(2.0),
    },
    //514: initializer82f997a0; raw r10=0 callback.
    Definition {
        section: "Physics_Systems_OBLocationTest",
        label: "kOBLT_TrajEndLength",
        title: "TrajLength",
        initial: Value::Float(f32::from_bits(0x3f4ccccd)),
    },
    //515: initializer82f99800; raw r10=0 callback.
    Definition {
        section: "Physics_Systems_OBLocationTest",
        label: "kOBLT_TrajStartHeight",
        title: "TrajStartHeight",
        initial: Value::Float(1.5),
    },
    //516: initializer82f99860; raw r10=0 callback.
    Definition {
        section: "Physics_Systems_OBLocationTest",
        label: "kOBLT_TrajEndHeight",
        title: "TrajEndHeight",
        initial: Value::Float(0.5),
    },
    //517: initializer82f82f40/table82fd4d0c; raw r8=null callback.
    Definition {
        section: "Animation_Locomotion_Debug",
        label: "kDebugLocoCommonConditions",
        title: "Debug Loco Common Conditions",
        initial: Value::Boolean(false),
    },
    //518: initializer82f83800/table82fd4d94; raw r8=null callback.
    Definition {
        section: "Animation_Locomotion_Debug",
        label: "kDebugLocoMotionConditions",
        title: "Debug Loco Motion Conditions",
        initial: Value::Boolean(false),
    },
    //519: initializer82f4e168 / table82fd0364.
    Definition {
        section: "Stream",
        label: "kDisableStreamFocusMovement",
        title: "Disable stream focus movement",
        initial: Value::Boolean(false),
    },
    //520: initializer82f4e310 / table82fd0378.
    Definition {
        section: "Stream",
        label: "kSimMaxFocusCollections",
        title: "Sim Max Focus Collections",
        initial: Value::Integer {
            value: 4,
            minimum: 0,
            maximum: 9,
        },
    },
    //521: initializer82f4e3c0 / table82fd0380.
    Definition {
        section: "Stream",
        label: "kEnableStreamXml",
        title: "Enable Stream Xml",
        initial: Value::Boolean(true),
    },
    //522: initializer82f96070 / table82fd5c18.
    Definition {
        section: "Physics_HandPlant",
        label: "kHandPlantDisplayDebug",
        title: "Display Manager Debug",
        initial: Value::Boolean(false),
    },
    //523: initializer82f8e870 / table82fd56bc.
    Definition {
        section: "Physics_Debug",
        label: "kFixSkateboardInAir",
        title: "Fix Skateboard In Air",
        initial: Value::Boolean(false),
    },
    //524..530: initializers82f8ed08..82f8ef18 / table82fd56f4..82fd570c.
    Definition {
        section: "Physics_Skateboard",
        label: "kDrawLandingInfo",
        title: "Draw Landing Info",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Skateboard",
        label: "kbDrawKnownAirDebugInfo",
        title: "kbDrawKnownAirDebugInfo",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Wipeouts",
        label: "kDrawUpsideWipeout",
        title: "Draw Upside Down Wipeout",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_Trajectory",
        label: "kAirAlignFrames",
        title: "Align Frames",
        initial: Value::Float(3.0),
    },
    Definition {
        section: "Physics_Trajectory",
        label: "kBodySpinAgainstSpeedScalar",
        title: "Scalar applied if going wrong way",
        initial: Value::Float(0.005),
    },
    Definition {
        section: "Physics_Trajectory",
        label: "kFakieAlignAngle",
        title: "Fakie Align Angle",
        initial: Value::Float(45.0),
    },
    Definition {
        section: "Physics_Trajectory",
        label: "kMinAirTimeToFootPlant",
        title: "MinAirTimeToFootPlant",
        initial: Value::Float(0.3),
    },
    //531/532: initializers82f9bf18/82f9c020, tables82fd6054/82fd6060.
    Definition {
        section: "Physics_States_StateChanger",
        label: "kTimeForKnownAirWarning",
        title: "Time For Known Air Warning",
        initial: Value::Float(0.04),
    },
    Definition {
        section: "Physics_States_StateChanger",
        label: "kbShowStateChangerDebugWarnings",
        title: "kbShowStateChangerDebugWarnings",
        initial: Value::Boolean(false),
    },
    //533: ref830d7cd0, initializer82f8fa30, table82fd5788; null callback.
    Definition {
        section: "Physics_Debug",
        label: "kApplyAntiFLipTOrqueTest",
        title: "kApplyAntiFLipTOrqueTest",
        initial: Value::Boolean(false),
    },
    //534: initializer82f839a0/table82fd4da8, range0..1, null callback.
    Definition {
        section: "Animation_Locomotion",
        label: "kJumpCloseToLanding",
        title: "Jump Close to Landing Time",
        initial: Value::Float(f32::from_bits(0x3e99_999a)),
    },
    Definition {
        section: "Physics_Systems_Mount",
        label: "kBoardMountTestLinearThreshold",
        title: "Board Mount Linear Threshold",
        initial: Value::Float(0.2),
    },
    Definition {
        section: "Physics_Systems_Mount",
        label: "kBoardMountTestAngularThreshold",
        title: "Board Mount Angular Threshold",
        initial: Value::Float(25.0),
    },
    Definition {
        section: "Physics_Systems_Mount",
        label: "kBoardMountAirTestLinearThreshold",
        title: "Board Mount Air Linear Threshold",
        initial: Value::Float(0.3),
    },
    Definition {
        section: "Physics_Systems_Mount",
        label: "kBoardMountAirTestAngularThreshold",
        title: "Board Mount Air Angular Threshold",
        initial: Value::Float(40.0),
    },
    //539: initializer82f8eb30/table82fd56e0/reference830d97a8, null callback;
    // GUI range 0..20 (min/max unused by float targets).
    Definition {
        section: "Physics_States_PhysicsAir",
        label: "kPhysicsAirMountLaunchTrajMaxChange",
        title: "Mount Launch Traj Max Change",
        initial: Value::Float(4.0),
    },
    //540..546: LandingOffBoardAir tuning, initializers82f86ab0..82f86cf0 in
    // table82fd4f58..4f70 (after kShowManualOutTimer4f54), null callbacks.
    //540: reference830a33c0; GUI range 0..40.
    Definition {
        section: "Animation_Skater_InAir",
        label: "kLandingVelocityTwoFootThreshold",
        title: "OBLanding Velocity Two Foot Threshold",
        initial: Value::Float(5.0),
    },
    //541: reference830a3d4c; default0x3d4ccccd, GUI range 0..1.
    Definition {
        section: "Animation_Skater_InAir",
        label: "kLandingImpactMaxBlendDelta",
        title: "OBLanding Impact Max Blend Delta",
        initial: Value::Float(f32::from_bits(0x3d4c_cccd)),
    },
    //542: reference830a3564; default0x3ca3d70a, GUI range 0..1.
    Definition {
        section: "Animation_Skater_InAir",
        label: "kLandingImpactMinBlendDelta",
        title: "OBLanding Impact Min Blend Delta",
        initial: Value::Float(f32::from_bits(0x3ca3_d70a)),
    },
    //543: reference830a34b4; default0x3d23d70a, GUI range 0..1.
    Definition {
        section: "Animation_Skater_InAir",
        label: "kLandingLandPoseBlendDelta",
        title: "OBLanding Land Pose Blend Delta",
        initial: Value::Float(f32::from_bits(0x3d23_d70a)),
    },
    //544: reference830a36f8; GUI range 0..2.
    Definition {
        section: "Animation_Skater_InAir",
        label: "kLandingTimeToBlendToTwoFoot",
        title: "OBLanding Time to Blend to Two Foot",
        initial: Value::Float(0.5),
    },
    //545: reference830a3830; GUI range 0..2.
    Definition {
        section: "Animation_Skater_InAir",
        label: "kLandingTimeToWaitToBlendToTwoFoot",
        title: "OBLanding Time to Wait to Blend to Two Foot",
        initial: Value::Float(0.5),
    },
    //546: reference830a3964; default0x3e4ccccd, GUI range 0..1.
    Definition {
        section: "Animation_Skater_InAir",
        label: "kLandingTimeToBlendToLandPose",
        title: "OBLanding Time to Blend to Land Pose",
        initial: Value::Float(f32::from_bits(0x3e4c_cccd)),
    },
    //547: initializer82f92e30 at table82fd59bc (after kOB_DebugCollResponseA),
    // reference830d90e4; default0x3f666666, GUI range 0..1, null callback.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kOB_MinUpVectorY",
        title: "Min Up Vect Y",
        initial: Value::Float(f32::from_bits(0x3f66_6666)),
    },
    //548: initializer82f86478 at table82fd4f10 (after kUsePhysInducedGrindCrouch),
    // reference830a3dc0; GUI range 0..2, null callback.
    Definition {
        section: "Animation_Slowdown",
        label: "kSlowMotionScale",
        title: "Slow Motion On Trick Scale",
        initial: Value::Float(1.0),
    },
    //549..551: SetOffBoardCollision gates, initializers82f7d1a8/d1f8/d248 at
    // table82fd45f0..45f8 (after kShowPhysicsSkeleton), null callbacks.
    //549: reference8308ebe4.
    Definition {
        section: "Physics_Skeleton",
        label: "kSkeletonOB_UseLightDMOGroup",
        title: "Use LightDMO Group",
        initial: Value::Boolean(true),
    },
    //550: reference8308e5d8.
    Definition {
        section: "Physics_Skeleton",
        label: "kSkeletonOB_HaveBodyCollision",
        title: "Have Body Collision",
        initial: Value::Boolean(true),
    },
    //551: reference8308eae8.
    Definition {
        section: "Physics_Skeleton",
        label: "kSkeletonOB_CoreCollideWithLW",
        title: "Core Collide with LW",
        initial: Value::Boolean(true),
    },
    //552..559: OffBoardFeetIK::CalculateGroundCorrectionAndWipeout82dcd328
    // gates, table82fd5df4..5e14 (after kOB_IK_MaxFeetYDist), null callbacks.
    //552: initializer82f98900, reference830db7e8; GUI range 0..10.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kHeightAboveGroundToWipeout",
        title: "Height Above Ground To Wipeout",
        initial: Value::Float(1.0),
    },
    //553: initializer82f98960, reference830db0dc; GUI range 0..1.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kHeightAboveGroundFootTest",
        title: "HeightAboveGroundFootTest",
        initial: Value::Float(f32::from_bits(0x3e38_51ec)),
    },
    //554: initializer82f989c0, reference830db298; GUI range 0..1.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kYNormalValueToWipeout",
        title: "Y Normal To Wipeout",
        initial: Value::Float(f32::from_bits(0x3f19_999a)),
    },
    //555: initializer82f98a20, reference830db958; GUI range 0..1.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kOBIK_SpeedAwayFromBadFoot",
        title: "Speed Away From Bad Foot",
        initial: Value::Float(f32::from_bits(0x3ba3_d70a)),
    },
    //556: initializer82f98a80, reference830db108; GUI range 0x38d1b717..1.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kOBIK_MinTrajValue",
        title: "Min Traj Val",
        initial: Value::Float(f32::from_bits(0x3c23_d70a)),
    },
    //557: initializer82f98b40, reference830dbc04.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kOBIK_FramesToWipeoutForBadGround",
        title: "Frames For Bad Ground",
        initial: Value::Integer {
            value: 20,
            minimum: 0,
            maximum: 60,
        },
    },
    //558: initializer82f98b98, reference830db6e8.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kOBIK_FramesToWipeoutForFeetNotDown",
        title: "Frames For Feet Not Down",
        initial: Value::Integer {
            value: 40,
            minimum: 0,
            maximum: 120,
        },
    },
    //559: initializer82f98bf0, reference830db348.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kOBIK_FramesForGroundCorrection",
        title: "Max Frames For Ground",
        initial: Value::Integer {
            value: 30,
            minimum: 0,
            maximum: 60,
        },
    },
    //560: initializer82f88940 at table82fd512c (after kbSkitchShimmyDebugDisplay),
    // reference830a4078; GUI range 0..1, null callback.
    Definition {
        section: "Animation_Tricks_FootPlants",
        label: "kFootPlantTimeToLandThreshold",
        title: "Time to land threshold",
        initial: Value::Float(0.5),
    },
    //561: initializer82f94a60 at table82fd5afc, reference830d84f4; null callback.
    Definition {
        section: "Physics_States_Boneless",
        label: "kDoJump",
        title: "Do Jump",
        initial: Value::Boolean(true),
    },
    //562: initializer82f94b10 at table82fd5b04, reference830d95f4; GUI range -1..1.
    Definition {
        section: "Physics_States_Boneless",
        label: "kTrajPosDelta",
        title: "kTrajPosDelta",
        initial: Value::Float(0.25),
    },
    //563: initializer82f94b70 at table82fd5b08, reference830d75a8; GUI range 0..100.
    Definition {
        section: "Physics_States_Boneless",
        label: "kMaxJumpErrorBoneless",
        title: " MaxJumpVelErrorBoneless",
        initial: Value::Float(10.0),
    },
    //564: initializer82f7ccd8 at table82fd45b8, reference8308ebc0; null callback.
    Definition {
        section: "Physics_States_Boneless",
        label: "kDebugUseRawBonePos",
        title: "kDebugUseRawBonePos",
        initial: Value::Boolean(true),
    },
    //565: initializer82f96ea0 at table82fd5cbc, reference830daa38; null callback.
    Definition {
        section: "Physics_States_FootPlant",
        label: "kDebugDrawFootPlant",
        title: "Debug Draw",
        initial: Value::Boolean(false),
    },
    //566: initializer82f96ef0 at table82fd5cc0, reference830da774; GUI range 0.01..1.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kFootPlantTrajRadius",
        title: "kFootPlantTrajRadius",
        initial: Value::Float(0.04),
    },
    //567: initializer82f96f50 at table82fd5cc4, reference830da7a0; GUI range 0.1..2.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kFootPlantTrajMaxTime",
        title: "kFootPlantTrajMaxTime",
        initial: Value::Float(1.0),
    },
    //568: initializer82f96fb0 at table82fd5cc8, reference830da7f8; GUI range 0.5..5.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kFootPlantTrajError",
        title: "kFootPlantTrajError",
        initial: Value::Float(1.0),
    },
    //569: initializer82f97010 at table82fd5ccc, reference830dad40; GUI range 0..5.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kFPM_TimeToLandToDisableFootCollision",
        title: "Time to disable collision",
        initial: Value::Float(0.2),
    },
    //570: initializer82f970d0 at table82fd5cd4, reference830da8cc; GUI range -1..1.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kFPYFootOffset",
        title: "kFPYFootOffset",
        initial: Value::Float(0.0),
    },
    //571: initializer82f97130 at table82fd5cd8, reference830da848; GUI range 0..4.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kTrajStartTestTimeAdjust",
        title: "kTrajStartTestTimeAdjust",
        initial: Value::Float(2.0),
    },
    //572: initializer82f97190 at table82fd5cdc, reference830dabac; GUI range 0..1.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kFP_MinTimeAfterStart",
        title: "Min Time After Start",
        initial: Value::Float(0.0),
    },
    //573: initializer82f971f0 at table82fd5ce0, reference830daaac; GUI range 0..1.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kFP_MinTimeToStart",
        title: "Min Time To Start",
        initial: Value::Float(0.11),
    },
    //574: initializer82f97250 at table82fd5ce4, reference830dab04; GUI range 0..1.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kFP_MaxTimeToStart",
        title: "Max Time To Start",
        initial: Value::Float(0.3),
    },
    //575: initializer82f972b0 at table82fd5ce8, reference830da748; GUI range 0..10.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kFP_GrindBoxSize",
        title: "Grind Box Size",
        initial: Value::Float(2.0),
    },
    //576: initializer82f973d0 at table82fd5cf4, reference830daf0c; GUI range 0..1.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kCOMVelLegScalar",
        title: "kCOMVelLegScalar",
        initial: Value::Float(1.0),
    },
    //577: initializer82f97430 at table82fd5cf8, reference830da874; GUI range 0..1.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kIKBlendInSpeed",
        title: "kIKBlendInSpeed",
        initial: Value::Float(0.12),
    },
    //578: initializer82f97490 at table82fd5cfc, reference830dacc4; GUI range 0..1.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kIKBlendInMaxTimeToLand",
        title: "kIKBlendInMaxTimeToLand",
        initial: Value::Float(0.3),
    },
    //579: initializer82f974f0 at table82fd5d00, reference830daa0c; GUI range 0..180.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kMaxEntryAngleDegrees",
        title: "kMaxEntryAngleDegrees",
        initial: Value::Float(35.0),
    },
    //580: initializer82f97550 at table82fd5d04, reference830da6c4; GUI range 0..1.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kFPM_MaxOffsetDelta",
        title: "kFPM_MaxOffsetDelta",
        initial: Value::Float(0.15),
    },
    //581: initializer82f975b0 at table82fd5d08, reference830da91c; null callback.
    Definition {
        section: "Physics_States_FootPlant",
        label: "kIKFootPrepare",
        title: "IK foot before landing",
        initial: Value::Boolean(true),
    },
    //582: initializer82f97600 at table82fd5d0c, reference830dad6c; range 0..10, null callback.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kFramesToDisableFeetCollision",
        title: "kFramesToDisableFeetCollision",
        initial: Value::Integer {
            value: 4,
            minimum: 0,
            maximum: 10,
        },
    },
    //583: initializer82f97658 at table82fd5d10, reference830dadc4; GUI range 0..1.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kYTrajectoryOffset",
        title: "kYTrajectoryOffset",
        initial: Value::Float(0.2),
    },
    //584: initializer82f85c10 at table82fd4eb4, reference830a34e4; GUI range 0..45.
    Definition {
        section: "Animation_RetrieveBoard",
        label: "kMaxRetrieveBoardYawChange",
        title: "Max Yaw Change",
        initial: Value::Float(5.0),
    },
    //585: initializer82f85c70 at table82fd4eb8, reference830a35ac; GUI range 0..300.
    Definition {
        section: "Animation_RetrieveBoard",
        label: "kMaxRetrieveBoardBlendFrames",
        title: "Blend Frames",
        initial: Value::Float(15.0),
    },
    //586: initializer82f85cd0 at table82fd4ebc, reference830a4374; GUI range 0..300.
    Definition {
        section: "Animation_RetrieveBoard",
        label: "kMaxRetrieveBoardBlendOutFrames",
        title: "Blend Frames",
        initial: Value::Float(10.0),
    },
    //587: initializer82f85d30 at table82fd4ec0, reference830a3658; GUI range 0..300.
    Definition {
        section: "Animation_RetrieveBoard",
        label: "kMaxRetrieveBoardPlayOutFrames",
        title: "Max RetrieveBoard Out Frames",
        initial: Value::Float(10.0),
    },
    //588: initializer82f85d90 at table82fd4ec4, reference830a3d1c; GUI range 0..300.
    Definition {
        section: "Animation_RetrieveBoard",
        label: "kMaxDropBoardPlayOutFrames",
        title: "Max DropBoard Out Frames",
        initial: Value::Float(30.0),
    },
    //589: initializer82f73ca0 at table82fd2ae0, reference830595a0; GUI range 0..kFocusBoxUnloadDistance.
    Definition {
        section: "Input",
        label: "kMinTimeUseSessionMarkerHeld",
        title: "Min Time Use Session Marker Held",
        initial: Value::Float(0.2),
    },
    //590: initializer82f73d00 at table82fd2ae4, reference830595cc; GUI minimum 0.
    Definition {
        section: "Input",
        label: "kMaxTimeUseSessionMarkerHeld",
        title: "Max Time Use Session Marker Held",
        initial: Value::Float(1.0),
    },
    //591: initializer82f73d60 at table82fd2ae8, reference830595f8; GUI range 0..5000.
    Definition {
        section: "Input",
        label: "kMinDistanceForMinTimeUseSessionMarkerHeld",
        title: "Min Distance For Min Time Use Session Marker Held",
        initial: Value::Float(100.0),
    },
    //592: initializer82f73dc0 at table82fd2aec, reference83059624; GUI range 0..5000.
    Definition {
        section: "Input",
        label: "kMaxDistanceForMaxTimeUseSessionMarkerHeld",
        title: "Max Distance For Max Time Use Session Marker Held",
        initial: Value::Float(1000.0),
    },
    //593: initializer82f73e20 at table82fd2af0, reference83059650; GUI range 0..20.
    Definition {
        section: "Input",
        label: "kMinDistanceForTeleport",
        title: "Min Distance for Teleport",
        initial: Value::Float(0.5),
    },
    //594: initializer82f73e80 at table82fd2af4, reference830594f8; null callback.
    Definition {
        section: "Input",
        label: "kDebugSessionTeleport",
        title: "Always Allow Immediate Teleport",
        initial: Value::Boolean(false),
    },
    //595: initializer82f73ed0 at table82fd2af8, reference8305951c; null callback.
    Definition {
        section: "SessionMarker",
        label: "kDisableSessionMarkerPreConditions",
        title: "Disable Session Marker Pre Conditions",
        initial: Value::Boolean(false),
    },
    //596: initializer82f7b3d8 at table82fd4434, reference8308dbc8; GUI range 0..10.
    Definition {
        section: "Physics_Skateboard",
        label: "kMaxJumpError",
        title: " MaxJumpVelError",
        initial: Value::Float(7.0),
    },
    //597: initializer82f92c60 at table82fd59a8, reference830d7200; GUI range 0..10.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kOB_MaxJumpDistance",
        title: "Jump Distance",
        initial: Value::Float(1.0),
    },
    //598: initializer82f92cc0 at table82fd59ac, reference830d9494; GUI range 0..10.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kOB_MinJumpDistance",
        title: "Min Jump Distance",
        initial: Value::Float(0.75),
    },
    //599: initializer82f999c0 at table82fd5eb0, reference830dc9d8; GUI range 0.001..1.
    Definition {
        section: "Physics_Systems_OBJumpingTest",
        label: "kOBJT_TestRadius",
        title: "Line Test Radius",
        initial: Value::Float(0.165),
    },
    //600: initializer82f99ad0 at table82fd5ebc, reference830dc71c; GUI range 0.001..10.
    Definition {
        section: "Physics_Systems_OBJumpingTest",
        label: "kOBJT_MaxJumpHeight",
        title: "Max Jump Up Height",
        initial: Value::Float(1.0),
    },
    //601: initializer82f99cb0 at table82fd5ed0, reference830dc344; GUI range 0..4.
    Definition {
        section: "Physics_Systems_OBJumpingTest",
        label: "kOBJT_MaxJumpDist",
        title: "Absolute Max Jump Dist",
        initial: Value::Float(2.0),
    },
    //602: initializer82f99b30 at table82fd5ec0, reference830dd09c; GUI range 0.001..10.
    Definition {
        section: "Physics_Systems_OBJumpingTest",
        label: "kOBJT_MaxJumpDown",
        title: "Max Jump Down Dist",
        initial: Value::Float(5.0),
    },
    //603: initializer82f99bf0 at table82fd5ec8, reference830dd1dc; GUI range 0..2.
    Definition {
        section: "Physics_Systems_OBJumpingTest",
        label: "kOBJT_MinDistFromWall",
        title: "Min dist from wall",
        initial: Value::Float(0.3),
    },
    //604: initializer82f99c50 at table82fd5ecc, reference830dca78; GUI range 0..2.
    Definition {
        section: "Physics_Systems_OBJumpingTest",
        label: "kOBJT_MinDistFromTopOfWall",
        title: "Min dist from edge",
        initial: Value::Float(0.1),
    },
    //605: initializer82f99b90 at table82fd5ec4, reference830dd5f0; GUI range -1..1.
    Definition {
        section: "Physics_Systems_OBJumpingTest",
        label: "kOBJT_MinGroundNormalY",
        title: "MinGroundNormalY",
        initial: Value::Float(0.71),
    },
    //606: initializer82f99970 at table82fd5eac, reference830dc6f8; null callback.
    Definition {
        section: "Physics_Systems_OBJumpingTest",
        label: "kOBJT_DebugDraw",
        title: "Debug Draw",
        initial: Value::Boolean(false),
    },
    //607: initializer82f99920 at table82fd5ea8, reference830dcc18; null callback.
    Definition {
        section: "Physics_Systems_OBJumpingTest",
        label: "kOBJT_DebugTestBadTraj",
        title: "Force Bad Traj",
        initial: Value::Boolean(false),
    },
    //608: initializer82f951d8 at table82fd5b50, reference830d74b0; GUI range 0..20.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kOffboardJumpMaxLandOnBoardAssist",
        title: "OBJump Air Max Land on Board assist",
        initial: Value::Float(0.5),
    },
    //609: initializer82f95238 at table82fd5b54, reference830d8bdc; GUI range 0..20.
    Definition {
        section: "Physics_States_LandingOnDeck",
        label: "kOffboardStandingJumpMaxLandOnBoardAssist",
        title: "OBJump Jump Max Land on Board assist",
        initial: Value::Float(1.75),
    },
    //610: initializer82f9e4e0 at table82fd6280, reference830e0cb4; GUI range 0..2.
    Definition {
        section: "Physics_Teleport",
        label: "kLWTestRadius",
        title: "LW Test Radius",
        initial: Value::Float(f32::from_bits(0x3f19_999a)),
    },
    //611: initializer82f9e720 at table82fd6298, reference830e0e74; null callback.
    Definition {
        section: "Physics_Debug",
        label: "kbShowCollisionStateChecks",
        title: "Show collision state checks",
        initial: Value::Boolean(false),
    },
    //612: initializer82f91458 at table82fd58a4, reference830d96c0; GUI range 0..10000.
    Definition {
        section: "Physics_States_5050",
        label: "k5050CenterForce",
        title: "Center Force",
        initial: Value::Float(800.0),
    },
    //613: initializer82f9b218 at table82fd5fc4, reference830dbfd0; GUI range 0..1.
    Definition {
        section: "Physics_States_5050",
        label: "k5050ContactDist",
        title: "Contact Dist",
        initial: Value::Float(f32::from_bits(0x3e05_1eb8)),
    },
    //614: initializer82f917b8 at table82fd58c8, reference830d959c; GUI range 0..1.
    Definition {
        section: "Physics_States_5050",
        label: "k5050ControlEpsilon",
        title: "5050 Control Epsilon",
        initial: Value::Float(f32::from_bits(0x3df5_c28f)),
    },
    //615: initializer82f916f8 at table82fd58c0, reference830d8900; GUI range 0..1.
    Definition {
        section: "Physics_States_5050",
        label: "k5050ControlMatBlend",
        title: "5050 Control Mat Blend",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    //616: initializer82f91758 at table82fd58c4, reference830d9b00; GUI range 0..1.
    Definition {
        section: "Physics_States_5050",
        label: "k5050ControlStrength",
        title: "5050 Control Strength",
        initial: Value::Float(f32::from_bits(0x3f2e_147b)),
    },
    //617: initializer82f915d8 at table82fd58b4, reference830d9fbc; GUI range 0..1500.
    Definition {
        section: "Physics_States_5050",
        label: "k5050ExitLift",
        title: "Exit Lift",
        initial: Value::Float(0.0),
    },
    //618: initializer82f91638 at table82fd58b8, reference830d7bfc; GUI range 0..1500.
    Definition {
        section: "Physics_States_5050",
        label: "k5050ExitLift_Rail",
        title: "Exit Lift - Rail",
        initial: Value::Float(125.0),
    },
    //619: initializer82f914b8 at table82fd58a8, reference830d7124; GUI range 0..500.
    Definition {
        section: "Physics_States_5050",
        label: "k5050ExitNudgeForce",
        title: "Exit Nudge Force",
        initial: Value::Float(10.0),
    },
    //620: initializer82f91518 at table82fd58ac, reference830d95c8; GUI range 0..500.
    Definition {
        section: "Physics_States_5050",
        label: "k5050ExitNudgeForce_Rail",
        title: "Exit Nudge Force - Rail",
        initial: Value::Float(100.0),
    },
    //621: initializer82f91578 at table82fd58b0, reference830d7b4c; GUI range 0..4.
    Definition {
        section: "Physics_States_5050",
        label: "k5050ExitNudgeSpeed",
        title: "Exit Nudge Speed",
        initial: Value::Float(1.0),
    },
    //622: initializer82f912d8 at table82fd5894, reference830d9b2c; GUI range 0..400.
    Definition {
        section: "Physics_States_5050",
        label: "k5050Friction_FatRail",
        title: "Friction Fat Rail",
        initial: Value::Float(40.0),
    },
    //623: initializer82f91338 at table82fd5898, reference830d98d4; GUI range 0..400.
    Definition {
        section: "Physics_States_5050",
        label: "k5050Friction_Ledge",
        title: "Friction Ledge",
        initial: Value::Float(40.0),
    },
    //624: initializer82f91278 at table82fd5890, reference830d8a60; GUI range 0..200.
    Definition {
        section: "Physics_States_5050",
        label: "k5050Friction_ThinRail",
        title: "Friction Thin Rail",
        initial: Value::Float(50.0),
    },
    //625: initializer82f9b2d8 at table82fd5fcc, reference830dc774; GUI range 0..0.20000000298023224.
    Definition {
        section: "Physics_States_5050",
        label: "k5050MaxSeparation",
        title: "Max Separation",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    //626: initializer82f9b278 at table82fd5fc8, reference830dc898; GUI range 0..1.
    Definition {
        section: "Physics_States_5050",
        label: "k5050MaxSteer",
        title: "Max Steer",
        initial: Value::Float(f32::from_bits(0x3f66_6666)),
    },
    //627: initializer82f91818 at table82fd58cc, reference830d9d68; GUI range 0..1.
    Definition {
        section: "Physics_States_5050",
        label: "k5050SideJumpScalar",
        title: "Side Jump Scalar",
        initial: Value::Float(0.5),
    },
    //628: initializer82f913f8 at table82fd58a0, reference830d83f4; GUI range 0..0.20000000298023224.
    Definition {
        section: "Physics_States_5050",
        label: "k5050TruckDown",
        title: "Truck Down",
        initial: Value::Float(f32::from_bits(0x3d8f_5c29)),
    },
    //629: initializer82f91f38 at table82fd5918, reference830da014; GUI range 0..10000.
    Definition {
        section: "Physics_States_5o",
        label: "k5_OCenterForce",
        title: "Center Force",
        initial: Value::Float(3000.0),
    },
    //630: initializer82f91e78 at table82fd5910, reference830d9e8c; GUI range 0..0.5.
    Definition {
        section: "Physics_States_5o",
        label: "k5_OCenterToTruck",
        title: "Center To Truck",
        initial: Value::Float(f32::from_bits(0x3e6f_9db2)),
    },
    //631: initializer82f9b4b8 at table82fd5fe0, reference830dd5c4; GUI range 0..0.5.
    Definition {
        section: "Physics_GrindDetector_5_O",
        label: "k5_OContactDist",
        title: "5_O Contact Dist",
        initial: Value::Float(f32::from_bits(0x3e05_1eb8)),
    },
    //632: initializer82f92298 at table82fd593c, reference830d9394; GUI range 0..1.
    Definition {
        section: "Physics_States_5o",
        label: "k5_OControlStrength",
        title: "Control Strength",
        initial: Value::Float(f32::from_bits(0x3f2e_147b)),
    },
    //633: initializer82f920b8 at table82fd5928, reference830d7460; GUI range 0..1500.
    Definition {
        section: "Physics_States_5o",
        label: "k5_OExitLift",
        title: "Exit Lift",
        initial: Value::Float(0.0),
    },
    //634: initializer82f92118 at table82fd592c, reference830d9f34; GUI range 0..1500.
    Definition {
        section: "Physics_States_5o",
        label: "k5_OExitLift_Rail",
        title: "Exit Lift - Rail",
        initial: Value::Float(125.0),
    },
    //635: initializer82f91f98 at table82fd591c, reference830d8760; GUI range 0..500.
    Definition {
        section: "Physics_States_5o",
        label: "k5_OExitNudgeForce",
        title: "Exit Nudge Force",
        initial: Value::Float(10.0),
    },
    //636: initializer82f91ff8 at table82fd5920, reference830d7050; GUI range 0..500.
    Definition {
        section: "Physics_States_5o",
        label: "k5_OExitNudgeForce_Rail",
        title: "Exit Nudge Force - Rail",
        initial: Value::Float(120.0),
    },
    //637: initializer82f92058 at table82fd5924, reference830d73e4; GUI range 0..4.
    Definition {
        section: "Physics_States_5o",
        label: "k5_OExitNudgeSpeed",
        title: "Exit Nudge Speed",
        initial: Value::Float(f32::from_bits(0x3f66_6666)),
    },
    //638: initializer82f91db8 at table82fd5908, reference830d9b84; GUI range 0..400.
    Definition {
        section: "Physics_States_5o",
        label: "k5_OFriction_FatRail",
        title: "Friction Fat Rail",
        initial: Value::Float(40.0),
    },
    //639: initializer82f91e18 at table82fd590c, reference830d722c; GUI range 0..400.
    Definition {
        section: "Physics_States_5o",
        label: "k5_OFriction_Ledge",
        title: "Friction Ledge",
        initial: Value::Float(40.0),
    },
    //640: initializer82f91d58 at table82fd5904, reference830d9210; GUI range 0..200.
    Definition {
        section: "Physics_States_5o",
        label: "k5_OFriction_ThinRail",
        title: "Friction Thin Rail",
        initial: Value::Float(50.0),
    },
    //641: initializer82f9ba90 at table82fd6020, reference830dbefc; GUI range 0..50.
    Definition {
        section: "Physics_Grinds_Control",
        label: "k5_OGrabGrindHoldUpMultiplier",
        title: "5_O Grab Grind Hold Up Multiplier",
        initial: Value::Float(10.0),
    },
    //642: initializer82f9b9d0 at table82fd6018, reference830dcb20; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Control",
        label: "k5_OGravity",
        title: "Gravity Control Strength",
        initial: Value::Float(f32::from_bits(0x3f35_c28f)),
    },
    //643: initializer82f9b458 at table82fd5fdc, reference830dc510; GUI range 0..1.
    Definition {
        section: "Physics_GrindDetector_5_O",
        label: "k5_OLeanForward",
        title: "5_O Lean Forward",
        initial: Value::Float(f32::from_bits(0x3f2e_147b)),
    },
    //644: initializer82f92238 at table82fd5938, reference830d7d4c; GUI range 0..1.
    Definition {
        section: "Physics_States_5o",
        label: "k5_OMatBlend",
        title: "Mat Blend",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    //645: initializer82f9b970 at table82fd6014, reference830dcf3c; GUI range -1..0.
    Definition {
        section: "Physics_Grinds_Control",
        label: "k5_OMaxDeckDown",
        title: "Deck Down Control Limit",
        initial: Value::Float(-0.25),
    },
    //646: initializer82f9b910 at table82fd6010, reference830dcc94; GUI range 0..5.
    Definition {
        section: "Physics_Grinds_Control",
        label: "k5_OMaxDeckUp",
        title: "Deck Up Control Limit",
        initial: Value::Float(f32::from_bits(0x3e85_1eb8)),
    },
    //647: initializer82f92178 at table82fd5930, reference830d77e0; GUI range 0..1.
    Definition {
        section: "Physics_States_5o",
        label: "k5_OMinDecksideSideness",
        title: "Min Deckside Sideness",
        initial: Value::Float(f32::from_bits(0x3e6b_851f)),
    },
    //648: initializer82f922f8 at table82fd5940, reference830d849c; GUI range 0..1.
    Definition {
        section: "Physics_States_5o",
        label: "k5_ONoise",
        title: "Noise",
        initial: Value::Float(f32::from_bits(0x3db8_51ec)),
    },
    //649: initializer82f9ba30 at table82fd601c, reference830dcaf4; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Control",
        label: "k5_OPryOffOffset",
        title: "5_O Pry Off Offset",
        initial: Value::Float(0.5),
    },
    //650: initializer82f921d8 at table82fd5934, reference830d9934; GUI range 0..1.
    Definition {
        section: "Physics_States_5o",
        label: "k5_OSideJumpScalar",
        title: "Side Jump Scalar",
        initial: Value::Float(0.5),
    },
    //651: initializer82f91ed8 at table82fd5914, reference830d76ac; GUI range 0..0.20000000298023224.
    Definition {
        section: "Physics_States_5o",
        label: "k5_OTruckDown",
        title: "Truck Down",
        initial: Value::Float(f32::from_bits(0x3d8f_5c29)),
    },
    //652: initializer82f9aed0 at table82fd5fa0, reference830dbffc; GUI range 0..90.
    Definition {
        section: "Physics_GrindDetector",
        label: "kAboveEntryAngle",
        title: "Above Entry Angle Deg",
        initial: Value::Float(17.0),
    },
    //653: initializer82f9a600 at table82fd5f3c, reference830dd7c0; GUI range 0..1.
    Definition {
        section: "Physics_States_5050",
        label: "kAirEngagement5050",
        title: "Air Engagement Stickyness",
        initial: Value::Float(f32::from_bits(0x3ecc_cccd)),
    },
    //654: initializer82f9a720 at table82fd5f48, reference830dc4c0; GUI range 0..1.
    Definition {
        section: "Physics_States_5_O",
        label: "kAirEngagement5_O",
        title: "Air Engagement Stickyness",
        initial: Value::Float(f32::from_bits(0x3ecc_cccd)),
    },
    //655: initializer82f9a780 at table82fd5f4c, reference830dc6a0; GUI range 0..1.
    Definition {
        section: "Physics_States_Backslash",
        label: "kAirEngagementBackslash",
        title: "Air Engagement Stickyness",
        initial: Value::Float(f32::from_bits(0x3f4c_cccd)),
    },
    //656: initializer82f9a660 at table82fd5f40, reference830dbe80; GUI range 0..1.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kAirEngagementBoardslide",
        title: "Air Engagement Stickyness",
        initial: Value::Float(f32::from_bits(0x3e4c_cccd)),
    },
    //657: initializer82f9a6c0 at table82fd5f44, reference830dd018; GUI range 0..1.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kAirEngagementTipslide",
        title: "Air Engagement Stickyness",
        initial: Value::Float(f32::from_bits(0x3f4c_cccd)),
    },
    //658: initializer82f90460 at table82fd57f8, reference830d9310; GUI range 0..1.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kAlignmentForceDamping",
        title: "Alignment Force Damping",
        initial: Value::Float(f32::from_bits(0x3e08_3127)),
    },
    //659: initializer82f90228 at table82fd57e0, reference830d77b4; GUI range 0..1.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kAngularDamping",
        title: "Angular Damping",
        initial: Value::Float(f32::from_bits(0x3d75_c28f)),
    },
    //660: initializer82f90340 at table82fd57ec, reference830d752c; GUI range 0..1.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kAngularWheelDamping",
        title: "Angular Wheel Damping",
        initial: Value::Float(0.5),
    },
    //661: initializer82f9b638 at table82fd5ff0, reference830dc95c; range 0..120.
    Definition {
        section: "Physics_GrindDetector_5_O",
        label: "kAnimWeightGroundFrames",
        title: "Anim Weight Ground Frames",
        initial: Value::Integer {
            value: 30,
            minimum: 0,
            maximum: 120,
        },
    },
    //662: initializer82f8abb0 at table82fd5334, reference830a6afc; GUI range 0..1.
    Definition {
        section: "Physics_States_Grinds_Geometry",
        label: "kAvgCurbHeight",
        title: "Avg Curb Height",
        initial: Value::Float(f32::from_bits(0x3e61_47ae)),
    },
    //663: initializer82f92538 at table82fd5958, reference830d918c; GUI range 0..10000.
    Definition {
        section: "Physics_States_Backslash",
        label: "kBackslashCenterForce",
        title: "Center Force",
        initial: Value::Float(3000.0),
    },
    //664: initializer82f92478 at table82fd5950, reference830d7024; GUI range 0..0.5.
    Definition {
        section: "Physics_States_Backslash",
        label: "kBackslashDeckToGrindPoint",
        title: "Deck Center To Grind Point",
        initial: Value::Float(f32::from_bits(0x3ec7_ae14)),
    },
    //665: initializer82f92658 at table82fd5964, reference830d79ac; GUI range 0..1500.
    Definition {
        section: "Physics_States_Backslash",
        label: "kBackslashExitLift",
        title: "Exit Lift",
        initial: Value::Float(125.0),
    },
    //666: initializer82f92598 at table82fd595c, reference830d878c; GUI range 0..500.
    Definition {
        section: "Physics_States_Backslash",
        label: "kBackslashExitNudgeForce",
        title: "Exit Nudge Force",
        initial: Value::Float(100.0),
    },
    //667: initializer82f925f8 at table82fd5960, reference830d73b8; GUI range 0..4.
    Definition {
        section: "Physics_States_Backslash",
        label: "kBackslashExitNudgeSpeed",
        title: "Exit Nudge Speed",
        initial: Value::Float(0.75),
    },
    //668: initializer82f923b8 at table82fd5948, reference830d6fcc; GUI range 0..400.
    Definition {
        section: "Physics_States_Backslash",
        label: "kBackslashFriction_FatRail",
        title: "Friction Fat Rail",
        initial: Value::Float(80.0),
    },
    //669: initializer82f92418 at table82fd594c, reference830d7788; GUI range 0..400.
    Definition {
        section: "Physics_States_Backslash",
        label: "kBackslashFriction_Ledge",
        title: "Friction Ledge",
        initial: Value::Float(80.0),
    },
    //670: initializer82f92358 at table82fd5944, reference830d8c8c; GUI range 0..200.
    Definition {
        section: "Physics_States_Backslash",
        label: "kBackslashFriction_ThinRail",
        title: "Friction Thin Rail",
        initial: Value::Float(70.0),
    },
    //671: initializer82f924d8 at table82fd5954, reference830d93c0; GUI range 0..0.20000000298023224.
    Definition {
        section: "Physics_States_Backslash",
        label: "kBackslashGrindPointDown",
        title: "Grind Point Down",
        initial: Value::Float(f32::from_bits(0x3ccc_cccd)),
    },
    //672: initializer82f92778 at table82fd5970, reference830d9a2c; GUI range 0..1.
    Definition {
        section: "Physics_States_Backslash",
        label: "kBackslashMatBlend",
        title: "Backslash Mat Blend",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    //673: initializer82f927d8 at table82fd5974, reference830d9d3c; GUI range 0..1.
    Definition {
        section: "Physics_States_Backslash",
        label: "kBackslashNoise",
        title: "Noise",
        initial: Value::Float(f32::from_bits(0x3db8_51ec)),
    },
    //674: initializer82f97f38 at table82fd5d84, reference830db798; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Jumping",
        label: "kBackslashSideJump",
        title: "Backslash Side Jump",
        initial: Value::Float(f32::from_bits(0x3ddf_d8ae)),
    },
    //675: initializer82f926b8 at table82fd5968, reference830d6ff8; GUI range 0..3.
    Definition {
        section: "Physics_States_Backslash",
        label: "kBackslashSideJumpScalar",
        title: "Side Jump Scalar",
        initial: Value::Float(f32::from_bits(0x3f66_6666)),
    },
    //676: initializer82f92718 at table82fd596c, reference830d9bb0; GUI range 0..200.
    Definition {
        section: "Physics_States_Backslash",
        label: "kBackslashTiltAngleDeg",
        title: "Backslash Tilt (deg)",
        initial: Value::Float(42.0),
    },
    //677: initializer82f9b790 at table82fd6000, reference830dd61c; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Control",
        label: "kBoardControlLRDeckControl",
        title: "Board Control LR Deck Control",
        initial: Value::Float(f32::from_bits(0x3f59_999a)),
    },
    //678: initializer82f90da8 at table82fd585c, reference830d824c; GUI range 0..1500.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kBoardslideExitLift",
        title: "Exit Lift",
        initial: Value::Float(125.0),
    },
    //679: initializer82f90ce8 at table82fd5854, reference830d738c; GUI range 0..500.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kBoardslideExitNudgeForce",
        title: "Exit Nudge Force",
        initial: Value::Float(90.0),
    },
    //680: initializer82f90d48 at table82fd5858, reference830d8c60; GUI range 0..4.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kBoardslideExitNudgeSpeed",
        title: "Exit Nudge Speed",
        initial: Value::Float(0.75),
    },
    //681: initializer82f90c28 at table82fd584c, reference830d8cb8; GUI range 0..400.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kBoardslideFriction_FatRail",
        title: "Friction Fat Rail",
        initial: Value::Float(55.0),
    },
    //682: initializer82f90c88 at table82fd5850, reference830d8ab8; GUI range 0..400.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kBoardslideFriction_Ledge",
        title: "Friction Ledge",
        initial: Value::Float(60.0),
    },
    //683: initializer82f90bc8 at table82fd5848, reference830d91b8; GUI range 0..200.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kBoardslideFriction_ThinRail",
        title: "Friction Thin Rail",
        initial: Value::Float(55.0),
    },
    //684: initializer82f91218 at table82fd588c, reference830d7c4c; GUI range 0..6.
    Definition {
        section: "Physics_Grinds_Jumping",
        label: "kBoardslideJumpVert",
        title: "boardslide Jump Vertical",
        initial: Value::Float(f32::from_bits(0x4027_0a3d)),
    },
    //685: initializer82f90ec8 at table82fd5868, reference830d84c8; GUI range 0..1.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kBoardslideMatBlend",
        title: "Boardslide Mat Blend",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    //686: initializer82f90f28 at table82fd586c, reference830d9268; GUI range 0..1.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kBoardslideNoise",
        title: "Noise",
        initial: Value::Float(f32::from_bits(0x3db8_51ec)),
    },
    //687: initializer82f90e68 at table82fd5864, reference830d8594; GUI range 0..3.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kBoardslideSideJumpScalar",
        title: "Side Jump Scalar",
        initial: Value::Float(f32::from_bits(0x3f66_6666)),
    },
    //688: initializer82f90f88 at table82fd5870, reference830d8470; GUI range 0..200.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kBoardslideTranslationForce",
        title: "Translation Control",
        initial: Value::Float(25.0),
    },
    //689: initializer82f90e08 at table82fd5860, reference830d8fdc; GUI range 0..400.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kBoardslideWipeoutDeckSpeed",
        title: "Wipeout Deck Speed",
        initial: Value::Float(136.0),
    },
    //690: initializer82f9b730 at table82fd5ffc, reference830dce20; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Control",
        label: "kBodyTwistLRDeckControl",
        title: "Body Twist LR Deck Control",
        initial: Value::Float(f32::from_bits(0x3f4a_3d71)),
    },
    //691: initializer82f9ae10 at table82fd5f98, reference830dcc68; GUI range 0..90.
    Definition {
        section: "Physics_GrindDetector",
        label: "kChangeEntryAngle",
        title: "Change Entry Angle Deg",
        initial: Value::Float(24.0),
    },
    //692: initializer82f8aaf0 at table82fd532c, reference830a685c; GUI range 0..1.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kCoarseCurbParam",
        title: "Coarse curb param",
        initial: Value::Float(f32::from_bits(0x3f26_6666)),
    },
    //693: initializer82f9b0a0 at table82fd5fb4, reference830dc1c0; GUI range 0..1.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kContactDist_Boardslide",
        title: "Contact Dist",
        initial: Value::Float(f32::from_bits(0x3d75_c28f)),
    },
    //694: initializer82f9b338 at table82fd5fd0, reference830dbf80; GUI range 0..1.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kContactDist_Tipslide",
        title: "Contact Dist",
        initial: Value::Float(f32::from_bits(0x3d75_c28f)),
    },
    //695: initializer82f99ee8 at table82fd5ef0, reference830dc53c; GUI range 0..4.
    Definition {
        section: "Physics_Grinds_Balance",
        label: "kCopingExitStartTime",
        title: "Coping Exit Start Time",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    //696: initializer82f90810 at table82fd5820, reference830d7284; GUI range 0..300.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kCopingSupportForce",
        title: "Coping Support Force",
        initial: Value::Float(120.0),
    },
    //697: initializer82f90520 at table82fd5800, reference830d8f34; GUI range 0..8.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kCurbFrictionScalar",
        title: "Curb Friction Scalar",
        initial: Value::Float(f32::from_bits(0x3ff3_3333)),
    },
    //698: initializer82f8ac10 at table82fd5338, reference830a67d8; GUI range 0..1.
    Definition {
        section: "Physics_States_Grinds_Geometry",
        label: "kCurbHeightRange",
        title: "Curb Height Range",
        initial: Value::Float(f32::from_bits(0x3ca3_d70a)),
    },
    //699: initializer82f8ac70 at table82fd533c, reference830a6a54; GUI range 0..1.
    Definition {
        section: "Physics_States_Grinds_Geometry",
        label: "kCurbHighSideRange",
        title: "Curb High Side Range",
        initial: Value::Float(f32::from_bits(0x3d0f_5c29)),
    },
    //700: initializer82f8ab50 at table82fd5330, reference830a6830; GUI range 0.9800000190734863..1.
    Definition {
        section: "Physics_States_Grinds_Geometry",
        label: "kCurbNormalLikeness",
        title: "Curb Normal Likeness",
        initial: Value::Float(f32::from_bits(0x3f7d_70a4)),
    },
    //701: initializer82f9b7f0 at table82fd6004, reference830dc410; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Control",
        label: "kDeckControlLRDeadzone",
        title: "Board Control LR Deadzone",
        initial: Value::Float(0.25),
    },
    //702: initializer82f90188 at table82fd57d8, reference830d785c; boolean.
    Definition {
        section: "Physics_Grinds",
        label: "kDisplayPhysOutGrinds",
        title: "Display PhysOut_Grinds",
        initial: Value::Boolean(false),
    },
    //703: initializer82f8a920 at table82fd5318, reference830a6a04; boolean.
    Definition {
        section: "Physics_Debug",
        label: "kDrawGrindGeometryTest",
        title: "Draw Grind Geometry Test",
        initial: Value::Boolean(false),
    },
    //704: initializer82f8a8d0 at table82fd5314, reference830a694c; boolean.
    Definition {
        section: "Physics_Debug",
        label: "kDrawGrindIntersectors",
        title: "Draw Grind Intersectors",
        initial: Value::Boolean(false),
    },
    //705: initializer82f9b690 at table82fd5ff4, reference830dc9b4; boolean.
    Definition {
        section: "Physics_Debug",
        label: "kDrawNearbyGrindPrimatives",
        title: "Draw Nearby Grind Primatives",
        initial: Value::Boolean(false),
    },
    //706: initializer82f908d0 at table82fd5828, reference830d7e44; GUI range 0..200.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kExitAssistForce",
        title: "Exit Assist Force",
        initial: Value::Float(65.0),
    },
    //707: initializer82f99e88 at table82fd5eec, reference830dc468; GUI range 0..4.
    Definition {
        section: "Physics_Grinds_Balance",
        label: "kFairlySteepExitStartTime",
        title: "Fairly Steep Exit Start Time",
        initial: Value::Float(f32::from_bits(0x3f4c_cccd)),
    },
    //708: initializer82f8aa90 at table82fd5328, reference830a6b30; GUI range 0..0.20000000298023224.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kFineGeometryDepth",
        title: "Fine Geometry Depth",
        initial: Value::Float(f32::from_bits(0x3d8f_5c29)),
    },
    //709: initializer82f8aa30 at table82fd5324, reference830a68ac; GUI range 0..0.20000000298023224.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kFineGeometryDist",
        title: "Fine Geometry Distance",
        initial: Value::Float(f32::from_bits(0x3db8_51ec)),
    },
    //710: initializer82f9b6e0 at table82fd5ff8, reference830dbd90; boolean.
    Definition {
        section: "Physics_Grinds_Control",
        label: "kFlipTruckGrindControl",
        title: "Truck Grind Control Flip",
        initial: Value::Boolean(false),
    },
    //711: initializer82f9a2a8 at table82fd5f18, reference830dc86c; GUI range -1..0.
    Definition {
        section: "Physics_Grinds_Balance",
        label: "kForceExitNormalThresh",
        title: "Force Exit Normal Thresh",
        initial: Value::Float(f32::from_bits(0xbdcc_cccd)),
    },
    //712: initializer82f9a1e8 at table82fd5f10, reference830dc8e8; GUI range 0..90.
    Definition {
        section: "Physics_Grinds_Balance",
        label: "kForceExitRideOutTiltDeg",
        title: "Force Exit Ride Out Tilt Deg",
        initial: Value::Float(15.0),
    },
    //713: initializer82f9a248 at table82fd5f14, reference830dd648; GUI range 0..90.
    Definition {
        section: "Physics_Grinds_Balance",
        label: "kForceExitRollOutTiltDeg",
        title: "Force Exit Roll Out Tilt Deg",
        initial: Value::Float(28.0),
    },
    //714: initializer82f9a188 at table82fd5f0c, reference830dc840; GUI range 0..10.
    Definition {
        section: "Physics_Grinds_Balance",
        label: "kForceExitTestDown",
        title: "Force Exit Test Down",
        initial: Value::Float(5.0),
    },
    //715: initializer82f9a128 at table82fd5f08, reference830dbf54; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Balance",
        label: "kForceExitTestOut",
        title: "Force Exit Test Out",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    //716: initializer82f907c0 at table82fd581c, reference830d75d4; enumeration.
    Definition {
        section: "Physics_Surfaces",
        label: "kForcedGrindSurface",
        title: "Grind Surface",
        initial: Value::Enumeration {
            value: 0,
            tags: GRIND_SURFACE_TAGS,
        },
    },
    //717: initializer82f90b10 at table82fd5840, reference830d9a58; GUI range 0..0.5.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kGrindCommonExitSpeed",
        title: "Exit Speed",
        initial: Value::Float(f32::from_bits(0x3e19_999a)),
    },
    //718: initializer82f904c0 at table82fd57fc, reference830d7cf4; GUI range 0..10.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kGrindCommonFrictionScalar",
        title: "General Friction Scalar",
        initial: Value::Float(f32::from_bits(0x3ef5_c28f)),
    },
    //719: initializer82f97c98 at table82fd5d68, reference830db900; GUI range 0..6.
    Definition {
        section: "Physics_Grinds_Jumping",
        label: "kGrindCommonJumpSide",
        title: "Jump Side Control",
        initial: Value::Float(f32::from_bits(0x3fe6_6666)),
    },
    //720: initializer82f90ab0 at table82fd583c, reference830d7e18; GUI range 0..6.
    Definition {
        section: "Physics_Grinds_Jumping",
        label: "kGrindCommonJumpVert",
        title: "Jump Vertical",
        initial: Value::Float(2.5),
    },
    //721: initializer82f97cf8 at table82fd5d6c, reference830dba88; GUI range 0.8999999761581421..1.7999999523162842.
    Definition {
        section: "Physics_Grinds_Jumping",
        label: "kGrindCommonJumpVertScalar",
        title: "Jump Vert Global Scalar",
        initial: Value::Float(f32::from_bits(0x3f83_d70a)),
    },
    //722: initializer82f902e0 at table82fd57e8, reference830d9908; GUI range 0..1.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kGrindCrouchScalar",
        title: "Crouch Scalar",
        initial: Value::Float(f32::from_bits(0x3ecc_cccd)),
    },
    //723: initializer82f9aa60 at table82fd5f70, reference830dc194; range 0..120.
    Definition {
        section: "Physics_GrindInvestigator",
        label: "kGrindExitFrames",
        title: "Grind Exit Frames",
        initial: Value::Integer {
            value: 70,
            minimum: 0,
            maximum: 120,
        },
    },
    //724: initializer82f905e0 at table82fd5808, reference830d9468; GUI range 0..10.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kGrindFriction_Rough",
        title: "Friction Rough",
        initial: Value::Float(f32::from_bits(0x3fd9_999a)),
    },
    //725: initializer82f906a0 at table82fd5810, reference830d8044; GUI range 0..10.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kGrindFriction_Slippery",
        title: "Friction Slippery",
        initial: Value::Float(f32::from_bits(0x3f19_999a)),
    },
    //726: initializer82f90640 at table82fd580c, reference830d9d10; GUI range 0..10.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kGrindFriction_Slow",
        title: "Friction Slow",
        initial: Value::Float(f32::from_bits(0x4026_6666)),
    },
    //727: initializer82f90580 at table82fd5804, reference830d7654; GUI range 0..10.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kGrindFriction_Smooth",
        title: "Friction Smooth",
        initial: Value::Float(f32::from_bits(0x3fa6_6666)),
    },
    //728: initializer82f90760 at table82fd5818, reference830d8a08; GUI range 0..10.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kGrindFriction_Unrideable",
        title: "Friction Unrideable",
        initial: Value::Float(5.0),
    },
    //729: initializer82f90700 at table82fd5814, reference830d933c; GUI range 0..10.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kGrindFriction_VerySlow",
        title: "Friction VerySlow",
        initial: Value::Float(4.0),
    },
    //730: initializer82f90400 at table82fd57f4, reference830d98a8; GUI range 0..1.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kGrindReckoningSmoothnessInGrind",
        title: "Reckoning Smoothness In Grind",
        initial: Value::Float(f32::from_bits(0x3f33_3333)),
    },
    //731: initializer82f903a0 at table82fd57f0, reference830d8984; GUI range 0..1.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kGrindReckoningSmoothnessOnExit",
        title: "Reckoning Smoothness On Exit",
        initial: Value::Float(f32::from_bits(0x3f66_6666)),
    },
    //732: initializer82f9a420 at table82fd5f28, reference830dd540; GUI range 0..1.
    Definition {
        section: "Physics_States_5050",
        label: "kGroundEngagement5050",
        title: "Ground Engagement Stickyness",
        initial: Value::Float(f32::from_bits(0x3e4c_cccd)),
    },
    //733: initializer82f9a540 at table82fd5f34, reference830dc294; GUI range 0..1.
    Definition {
        section: "Physics_States_5_O",
        label: "kGroundEngagement5_O",
        title: "Ground Engagement Stickyness",
        initial: Value::Float(f32::from_bits(0x3e4c_cccd)),
    },
    //734: initializer82f9a5a0 at table82fd5f38, reference830dc5d4; GUI range 0..1.
    Definition {
        section: "Physics_States_Backslash",
        label: "kGroundEngagementBackslash",
        title: "Ground Engagement Stickyness",
        initial: Value::Float(f32::from_bits(0x3e4c_cccd)),
    },
    //735: initializer82f9a480 at table82fd5f2c, reference830dc2ec; GUI range 0..1.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kGroundEngagementBoardslide",
        title: "Ground Engagement Stickyness",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    //736: initializer82f9a4e0 at table82fd5f30, reference830dd3cc; GUI range 0..1.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kGroundEngagementTipslide",
        title: "Ground Engagement Stickyness",
        initial: Value::Float(f32::from_bits(0x3ecc_cccd)),
    },
    //737: initializer82f9b1c0 at table82fd5fc0, reference830dcbec; range 0..120.
    Definition {
        section: "Physics_GrindInvestigator",
        label: "kGroundToBoardslideFrames",
        title: "Ground To Boardslide Frames",
        initial: Value::Integer {
            value: 10,
            minimum: 0,
            maximum: 120,
        },
    },
    //738: initializer82f9aab8 at table82fd5f74, reference830dc494; GUI range 0..2.
    Definition {
        section: "Physics_GrindInvestigator",
        label: "kGroundToBoardslideSpeed",
        title: "Ground To Boardslide Speed",
        initial: Value::Float(f32::from_bits(0x3f4c_cccd)),
    },
    //739: initializer82f9abd8 at table82fd5f80, reference830dd41c; GUI range 0..25.
    Definition {
        section: "Physics_GrindInvestigator",
        label: "kHorizontalGrindAvoidSpeed",
        title: "Horizontal Grind Avoid Speed",
        initial: Value::Float(3.5),
    },
    //740: initializer82f9a840 at table82fd5f54, reference830dc2c0; GUI range 0..10.
    Definition {
        section: "Physics_Grinds",
        label: "kHorizontalGrindWipeoutSpeed",
        title: "Horizontal Grind Wipeout Speed",
        initial: Value::Float(7.0),
    },
    //741: initializer82f97d58 at table82fd5d70, reference830dbc30; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Jumping",
        label: "kJumpEnergyCost",
        title: "Jump Energy Cost",
        initial: Value::Float(f32::from_bits(0x3f19_999a)),
    },
    //742: initializer82f97bd8 at table82fd5d60, reference830dbb04; GUI range 0..0.20000000298023224.
    Definition {
        section: "Physics_Grinds_Jumping",
        label: "kJumpEnergyRegeneration",
        title: "Jump Energy Regeneration",
        initial: Value::Float(f32::from_bits(0x3b65_6042)),
    },
    //743: initializer82f97e78 at table82fd5d7c, reference830db424; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Jumping",
        label: "kJumpNormalVertFraction",
        title: "Jump Normal Vert Fraction",
        initial: Value::Float(0.5),
    },
    //744: initializer82f9ab18 at table82fd5f78, reference830dcc3c; GUI range 0..1.
    Definition {
        section: "Physics_GrindInvestigator",
        label: "kLooseEndRiseThresh",
        title: "Loose End Rise Thresh",
        initial: Value::Float(f32::from_bits(0x3e42_8f5c)),
    },
    //745: initializer82f9a0c8 at table82fd5f04, reference830dcaa4; GUI range 0..45.
    Definition {
        section: "Physics_Grinds_Balance",
        label: "kMaxBalanceAngleDeg",
        title: "Max Balance Difference from Deck [deg]",
        initial: Value::Float(14.0),
    },
    //746: initializer82f9af90 at table82fd5fa8, reference830dd4a0; GUI range 0..20.
    Definition {
        section: "Physics_GrindDetector",
        label: "kMaxCopingDV0",
        title: "Max Coping DV 0",
        initial: Value::Float(f32::from_bits(0x40a6_6666)),
    },
    //747: initializer82f9aff0 at table82fd5fac, reference830dd474; GUI range 0..20.
    Definition {
        section: "Physics_GrindDetector",
        label: "kMaxCopingDV1",
        title: "Max Coping DV 1",
        initial: Value::Float(f32::from_bits(0x4033_3333)),
    },
    //748: initializer82f9b5d8 at table82fd5fec, reference830dd258; GUI range 0..1.
    Definition {
        section: "Physics_GrindDetector_5_O",
        label: "kMaxCos_5_O",
        title: "Max Cos 5_O",
        initial: Value::Float(f32::from_bits(0x3f67_ef9e)),
    },
    //749: initializer82f9a3c0 at table82fd5f24, reference830dc394; GUI range 0..5.
    Definition {
        section: "Physics_Grinds_Engagement",
        label: "kMaxDropInSpeed",
        title: "Max Drop In Speed",
        initial: Value::Float(f32::from_bits(0x3fb9_999a)),
    },
    //750: initializer82f90288 at table82fd57e4, reference830d8dec; range 0..300.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kMaxInvExitFrames",
        title: "Max Inv. Exit Frames",
        initial: Value::Integer {
            value: 35,
            minimum: 0,
            maximum: 300,
        },
    },
    //751: initializer82f9b518 at table82fd5fe4, reference830dd56c; GUI range 0..1.
    Definition {
        section: "Physics_GrindDetector_5_O",
        label: "kMaxTiltFor_5_O",
        title: "Max Tilt For 5_O",
        initial: Value::Float(f32::from_bits(0x3eeb_851f)),
    },
    //752: initializer82f9b160 at table82fd5fbc, reference830dd22c; GUI range 0..1.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kMinBoardslideDeckTilt",
        title: "Min Boardslide Deck Tilt",
        initial: Value::Float(f32::from_bits(0x3f26_6666)),
    },
    //753: initializer82f9bca0 at table82fd6038, reference830dc650; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Coping",
        label: "kMinCopingVert",
        title: "Min Coping Vert",
        initial: Value::Float(f32::from_bits(0x3f59_999a)),
    },
    //754: initializer82f9b3f8 at table82fd5fd8, reference830dbed0; GUI range -0.5..0.5.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kMinDeckDZTipslideDropIn",
        title: "Min DeckDZ Tipslide Drop In",
        initial: Value::Float(0.0),
    },
    //755: initializer82f90b70 at table82fd5844, reference830d7f20; range 0..180.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kMinGrindFrames",
        title: "Min Grind Frames",
        initial: Value::Integer {
            value: 60,
            minimum: 0,
            maximum: 180,
        },
    },
    //756: initializer82f9b100 at table82fd5fb8, reference830dd144; GUI range 0..3.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kMinGrindSpeedBoardslide",
        title: "Min Grind Speed",
        initial: Value::Float(0.75),
    },
    //757: initializer82f9ac38 at table82fd5f84, reference830dd0c8; range 0..120.
    Definition {
        section: "Physics_GrindInvestigator",
        label: "kMinNonGroundStairFrames",
        title: "Min Non-Ground Stair Frames",
        initial: Value::Integer {
            value: 20,
            minimum: 0,
            maximum: 120,
        },
    },
    //758: initializer82f97e18 at table82fd5d78, reference830db1b8; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Jumping",
        label: "kMinSideJumpLowEnergy",
        title: "Min Side Jump Low Energy",
        initial: Value::Float(f32::from_bits(0x3ed7_0a3d)),
    },
    //759: initializer82f9b398 at table82fd5fd4, reference830dc600; GUI range 0..10.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kMinSpeedWeightIgnore",
        title: "Min Speed Weight Ignore",
        initial: Value::Float(3.0),
    },
    //760: initializer82f909f0 at table82fd5834, reference830d795c; GUI range 0..20.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kNoiseFullSpeed",
        title: "Noise Full Speed",
        initial: Value::Float(2.0),
    },
    //761: initializer82f90990 at table82fd5830, reference830d7308; GUI range 0..20.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kNoiseScalar",
        title: "Noise Scalar",
        initial: Value::Float(f32::from_bits(0x3d75_c28f)),
    },
    //762: initializer82f99f48 at table82fd5ef4, reference830dc43c; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Balance",
        label: "kNormalSmoothness",
        title: "Normal Smoothness",
        initial: Value::Float(f32::from_bits(0x3f6c_cccd)),
    },
    //763: initializer82f9a958 at table82fd5f60, reference830dd044; GUI range 0..1.
    Definition {
        section: "Physics_Grinds",
        label: "kOffBalanceGrindThresh",
        title: "Off Balance Grind Thresh",
        initial: Value::Float(0.5),
    },
    //764: initializer82f90870 at table82fd5824, reference830d90b8; GUI range 0..100.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kPinForce",
        title: "Pin Force",
        initial: Value::Float(40.0),
    },
    //765: initializer82f90fe8 at table82fd5874, reference830d9544; GUI range 0..1.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kPrepareJumpClearance",
        title: "Prepare Jump Clearance",
        initial: Value::Float(f32::from_bits(0x3df5_c28f)),
    },
    //766: initializer82f91048 at table82fd5878, reference830d78d8; GUI range 0..300.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kPrepareJumpForce",
        title: "Prepare Jump Force",
        initial: Value::Float(201.0),
    },
    //767: initializer82f91168 at table82fd5884, reference830d8810; boolean.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kShowTruckAvoid",
        title: "Show Truck Avoid",
        initial: Value::Boolean(false),
    },
    //768: initializer82f97db8 at table82fd5d74, reference830dbbac; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Jumping",
        label: "kSideJumpEnergy",
        title: "Side Jump Energy",
        initial: Value::Float(f32::from_bits(0x3e9e_b852)),
    },
    //769: initializer82f9b050 at table82fd5fb0, reference830dc4ec; boolean.
    Definition {
        section: "Physics_GrindDetector",
        label: "kSpamGrindEntryAngle",
        title: "kSpamGrindEntryAngle",
        initial: Value::Boolean(false),
    },
    //770: initializer82f9ae70 at table82fd5f9c, reference830dc6cc; GUI range 0..90.
    Definition {
        section: "Physics_GrindDetector",
        label: "kStayEntryAngle",
        title: "Stay Entry Angle Deg",
        initial: Value::Float(90.0),
    },
    //771: initializer82f8a9d0 at table82fd5320, reference830a6804; GUI range 0..4.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kTestHeightHeuristic",
        title: "Grind Collision Test Height Heuristic",
        initial: Value::Float(f32::from_bits(0x3f87_ae14)),
    },
    //772: initializer82f91a58 at table82fd58e4, reference830d8a8c; GUI range 0..1500.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kTipslideExitLift",
        title: "Exit Lift",
        initial: Value::Float(125.0),
    },
    //773: initializer82f91998 at table82fd58dc, reference830d819c; GUI range 0..500.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kTipslideExitNudgeForce",
        title: "Exit Nudge Force",
        initial: Value::Float(100.0),
    },
    //774: initializer82f919f8 at table82fd58e0, reference830d7b20; GUI range 0..4.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kTipslideExitNudgeSpeed",
        title: "Exit Nudge Speed",
        initial: Value::Float(0.75),
    },
    //775: initializer82f918d8 at table82fd58d4, reference830d9850; GUI range 0..400.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kTipslideFriction_FatRail",
        title: "Friction Fat Rail",
        initial: Value::Float(60.0),
    },
    //776: initializer82f91938 at table82fd58d8, reference830d89dc; GUI range 0..400.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kTipslideFriction_Ledge",
        title: "Friction Ledge",
        initial: Value::Float(60.0),
    },
    //777: initializer82f91878 at table82fd58d0, reference830d9710; GUI range 0..200.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kTipslideFriction_ThinRail",
        title: "Friction Thin Rail",
        initial: Value::Float(60.0),
    },
    //778: initializer82f9a8a0 at table82fd5f58, reference830dcf68; range 0..60.
    Definition {
        section: "Physics_Grinds",
        label: "kTipslideGrabFrames",
        title: "Tipslide Grab Frames",
        initial: Value::Integer {
            value: 8,
            minimum: 0,
            maximum: 60,
        },
    },
    //779: initializer82f91b18 at table82fd58ec, reference830d99d4; GUI range 0..1000.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kTipslideGrabNudgeForce",
        title: "Grab Nudge Force",
        initial: Value::Float(39.0),
    },
    //780: initializer82f9a8f8 at table82fd5f5c, reference830dc218; GUI range 0..10.
    Definition {
        section: "Physics_Grinds",
        label: "kTipslideGrabSlow",
        title: "Tipslide Grab Slow",
        initial: Value::Float(f32::from_bits(0x3fe6_6666)),
    },
    //781: initializer82f91cf8 at table82fd5900, reference830d80c0; GUI range 0..6.
    Definition {
        section: "Physics_Grinds_Jumping",
        label: "kTipslideJumpVert",
        title: "tipslide Jump Vertical",
        initial: Value::Float(f32::from_bits(0x4027_0a3d)),
    },
    //782: initializer82f91c38 at table82fd58f8, reference830d8344; GUI range 0..1.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kTipslideMatBlend",
        title: "Tipslide Mat Blend",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    //783: initializer82f91c98 at table82fd58fc, reference830d7150; GUI range 0..1.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kTipslideNoise",
        title: "Noise",
        initial: Value::Float(f32::from_bits(0x3db8_51ec)),
    },
    //784: initializer82f9a008 at table82fd5efc, reference830dd674; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Balance",
        label: "kTipslideNormalSmoothness",
        title: "Tipslide Normal Smoothness",
        initial: Value::Float(f32::from_bits(0x3f59_999a)),
    },
    //785: initializer82f97ed8 at table82fd5d80, reference830dbb80; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Jumping",
        label: "kTipslideSideJump",
        title: "Tipslide Side Jump",
        initial: Value::Float(f32::from_bits(0x3e94_fdf4)),
    },
    //786: initializer82f91b78 at table82fd58f0, reference830d82f4; GUI range 0..3.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kTipslideSideJumpScalar",
        title: "Side Jump Scalar",
        initial: Value::Float(0.5),
    },
    //787: initializer82f91ab8 at table82fd58e8, reference830d8eb0; GUI range 0..300.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kTipslideStabilityNudgeForce",
        title: "Stability Nudge Force",
        initial: Value::Float(37.0),
    },
    //788: initializer82f91bd8 at table82fd58f4, reference830d9694; GUI range 0..200.
    Definition {
        section: "Physics_States_Tipslide",
        label: "kTipslideTiltAngleDeg",
        title: "Tipslide Tilt (deg)",
        initial: Value::Float(20.0),
    },
    //789: initializer82f97b80 at table82fd5d5c, reference830db690; range 0..200.
    Definition {
        section: "Physics_Grinds_Jumping",
        label: "kTrajSelectorAvoidGrindFrames",
        title: "Traj Selector Avoid Grind Frames",
        initial: Value::Integer {
            value: 10,
            minimum: 0,
            maximum: 200,
        },
    },
    //790: initializer82f91108 at table82fd5880, reference830d7d20; GUI range 0..400.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kTruckAvoidForce",
        title: "Truck Avoid Force",
        initial: Value::Float(10.0),
    },
    //791: initializer82f910a8 at table82fd587c, reference830da098; GUI range 0..1.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kTruckClearance",
        title: "Truck Clearance",
        initial: Value::Float(f32::from_bits(0x3e23_d70a)),
    },
    //792: initializer82f90930 at table82fd582c, reference830d7fec; GUI range 0..1000.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kTruckControlCompensationForce",
        title: "Truck Control Compensation Force",
        initial: Value::Float(600.0),
    },
    //793: initializer82f9b8b0 at table82fd600c, reference830dc0ec; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Control",
        label: "kTruckGrindControlClampedSmoothness",
        title: "Truck Grind Control Clamped Smoothness",
        initial: Value::Float(f32::from_bits(0x3f42_8f5c)),
    },
    //794: initializer82f9b850 at table82fd6008, reference830dcdac; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Control",
        label: "kTruckGrindControlSmoothness",
        title: "Truck Grind Control Smoothness",
        initial: Value::Float(f32::from_bits(0x3f68_f5c3)),
    },
    //795: initializer82f99fa8 at table82fd5ef8, reference830dd3a0; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Balance",
        label: "kUpVectorSmoothness",
        title: "Up Vector Smoothness",
        initial: Value::Float(f32::from_bits(0x3f4c_cccd)),
    },
    //796: initializer82f911b8 at table82fd5888, reference830d8f60; GUI range 0..400.
    Definition {
        section: "Physics_States_Boardslide",
        label: "kVelDampingScalar",
        title: "Vel Damping Force",
        initial: Value::Float(20.0),
    },
    //797: initializer82f8a970 at table82fd531c, reference830a69d8; GUI range 0..20.
    Definition {
        section: "Physics_States_GrindCommon",
        label: "kWheelClearAngleDeg",
        title: "Wheel Clearance landing angle [deg]",
        initial: Value::Float(16.0),
    },
    //798: initializer82fa0a28 at table82fd649c, reference8311e8bc; range 0..60.
    Definition {
        section: "Physics_Conditioners_Grinds",
        label: "kMinChromosomeEqualityFrames",
        title: "Chromosome Equality Frames",
        initial: Value::Integer {
            value: 0,
            minimum: 0,
            maximum: 60,
        },
    },
    //799: initializer82fa0a80 at table82fd64a0, reference8311ea1c; range 0..60.
    Definition {
        section: "Physics_Conditioners_Grinds",
        label: "kMinChromosomeEqualityFrames_Scoring",
        title: "Chromosome Equality Frames (scoring)",
        initial: Value::Integer {
            value: 12,
            minimum: 0,
            maximum: 60,
        },
    },
    //800: initializer82fa0890 at table82fd6484, reference8311e59c; boolean.
    Definition {
        section: "Physics_Grinds",
        label: "kbGrindConditionerDebugDraw",
        title: "kbGrindConditionerDebugDraw",
        initial: Value::Boolean(false),
    },
    //801: initializer82fa09c8 at table82fd6498, reference8311ce14; GUI range -1..1.
    Definition {
        section: "Physics_Conditioners_Grinds",
        label: "kDeckTiltForUp",
        title: "Deck Tilt For Up",
        initial: Value::Float(f32::from_bits(0xbdcc_cccd)),
    },
    //802: initializer82f928d8 at table82fd5980, reference830d9c34; GUI range 0..1.
    Definition {
        section: "Physics_States_Nonspecific",
        label: "kNonspecificSideJumpScalar",
        title: "Side Jump Scalar",
        initial: Value::Float(f32::from_bits(0x3f33_3333)),
    },
    //803: initializer82f92938 at table82fd5984, reference830d7880; GUI range 0..1.
    Definition {
        section: "Physics_States_Nonspecific",
        label: "kNonspecificReckoningSmoothness",
        title: "Reckoning Smoothness",
        initial: Value::Float(f32::from_bits(0x3f7a_e148)),
    },
    //804: initializer82f9bd00 at table82fd603c, reference830dc168; range 0..200.
    Definition {
        section: "Physics_States_StateChanger",
        label: "kNonspecificCollExitFrames",
        title: "Nonspecific Coll Exit Frames",
        initial: Value::Integer {
            value: 60,
            minimum: 0,
            maximum: 200,
        },
    },
    //805: initializer82f9bd58 at table82fd6040, reference830dc3c0; range 0..60.
    Definition {
        section: "Physics_States_StateChanger",
        label: "kNonspecificCollFreeStickyFrames",
        title: "Nonspecific Coll Sticky Frames",
        initial: Value::Integer {
            value: 2,
            minimum: 0,
            maximum: 60,
        },
    },
    //806: initializer82f9be08 at table82fd6048, reference830dcfec; range 0..60.
    Definition {
        section: "Physics_States_StateChanger",
        label: "kNonspecificToGroundNearGrindFrames",
        title: "Nonspecific To Ground Near Grind Frames",
        initial: Value::Integer {
            value: 10,
            minimum: 0,
            maximum: 60,
        },
    },
    //807: initializer82f92998 at table82fd5988, reference830d8b08; GUI range 0..200.
    Definition {
        section: "Debug_Physics_States_Nonspecific",
        label: "kPlayerWeight",
        title: "Player Weight",
        initial: Value::Float(40.0),
    },
    //808: initializer82f92838 at table82fd5978, reference830d9760; boolean.
    Definition {
        section: "Physics_States_Nonspecific",
        label: "kDoCollisionResponse",
        title: "Do Collision Response",
        initial: Value::Boolean(true),
    },
    //809: initializer82f92888 at table82fd597c, reference830d8420; boolean.
    Definition {
        section: "Physics_States_Nonspecific",
        label: "kDoWipeoutsTest",
        title: "Do Wipeout Test",
        initial: Value::Boolean(true),
    },
    //810: initializer82fa20a8 at table82fd65f0, reference8311ed84; GUI range 0..0.5.
    Definition {
        section: "Camera_Dynamics_Settings",
        label: "kInsignificantGrindSpeed",
        title: "Insignificant grind speed",
        initial: Value::Float(f32::from_bits(0x3e99_999a)),
    },
    //811: initializer82f8a6b0 at table82fd52fc, reference830a6aac; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Pairs",
        label: "kMinPairDist",
        title: "Min Pair Dist",
        initial: Value::Float(f32::from_bits(0x3c23_d70a)),
    },
    //812: initializer82f8a710 at table82fd5300, reference830a6a28; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Pairs",
        label: "kMaxPairDist",
        title: "Max Pair Dist",
        initial: Value::Float(f32::from_bits(0x3dcc_cccd)),
    },
    //813: initializer82f8a770 at table82fd5304, reference830a68fc; GUI range 0..1.
    Definition {
        section: "Physics_Grinds_Pairs",
        label: "kMaxPairDistY",
        title: "Max Pair DistY",
        initial: Value::Float(f32::from_bits(0x3ca3_d70a)),
    },
    //814: initializer82f8a7d0 at table82fd5308, reference830a6970; boolean.
    Definition {
        section: "Physics_Debug",
        label: "kDrawGrindPairDetection",
        title: "Draw Grind Pair Detection",
        initial: Value::Boolean(false),
    },
    //815: initializer82f96a70 at table82fd5c8c, reference830dae14; boolean.
    Definition {
        section: "Physics_Trajectory",
        label: "kShowGrindInfo",
        title: "Show Grind Info",
        initial: Value::Boolean(false),
    },
    //816: initializer82f96b80 at table82fd5c98, reference830da6a0; boolean.
    Definition {
        section: "Physics_Grinds",
        label: "kRenderGrindLandings",
        title: "Show Grind Landings",
        initial: Value::Boolean(false),
    },
    //817: initializer82f96bd0 at table82fd5c9c, reference830da550; boolean.
    Definition {
        section: "Physics_Grinds",
        label: "kRenderGrindLandingUp",
        title: "Show Grind Landing Up",
        initial: Value::Boolean(false),
    },
    //818: initializer82f97840 at table82fd5d34, reference830db4d4; boolean.
    Definition {
        section: "Physics_Grinds_AirAssist",
        label: "kDrawGrindAssistInfo",
        title: "Draw Grind Air Adjust",
        initial: Value::Boolean(false),
    },
    //819: initializer82f97890 at table82fd5d38, reference830db524; boolean.
    Definition {
        section: "Physics_Grinds_AirAssist",
        label: "kGrindAssistActive",
        title: "Grind Air Adjust Active",
        initial: Value::Boolean(true),
    },
    //820: initializer82f978e0 at table82fd5d3c, reference830dbcb4; boolean.
    Definition {
        section: "Physics_Grinds_AirAssist",
        label: "kGrindAssistApplyOffset",
        title: "Grind Air Apply Pos Offset",
        initial: Value::Boolean(true),
    },
    //821: initializer82f97930 at table82fd5d40, reference830db9d4; boolean.
    Definition {
        section: "Physics_Grinds_AirAssist",
        label: "kGrindAssistApplyAngleOffset",
        title: "Grind Air Apply Angle Offset",
        initial: Value::Boolean(true),
    },
    //822: initializer82f97310 at table82fd5cec, reference830da5cc; GUI range 0..1.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kFP_MaxDistToAdjustFootPlant",
        title: "Max Foot Adjust",
        initial: Value::Float(0.25),
    },
    //823: initializer82f97370 at table82fd5cf0, reference830da71c; GUI range 0..1.
    Definition {
        section: "Physics_States_FootPlant_consts",
        label: "kFP_MaxDistBelowCollisionForGrind",
        title: "Grind Max Dist below coll",
        initial: Value::Float(f32::from_bits(0x3cf5_c28f)),
    },
    //824: initializer82f86428 at table82fd4f0c, reference830a446c; boolean.
    Definition {
        section: "Animation_Skater",
        label: "kUsePhysInducedGrindCrouch",
        title: "Use Phys Induced Grind Crouch",
        initial: Value::Boolean(true),
    },
    //825: initializer82f863d8 at table82fd4f08, reference830a3e44; boolean.
    Definition {
        section: "Animation_Skater",
        label: "kShowGrindCrouchText",
        title: "Show Grind Crouch Text",
        initial: Value::Boolean(false),
    },
    //826: initializer82f86388 at table82fd4f04, reference830a3d9c; boolean.
    Definition {
        section: "Animation_Skater",
        label: "kShowGrindControlFade",
        title: "Show Grind Control Fade",
        initial: Value::Boolean(false),
    },
    //827: initializer82f73128 at table82fd2a20, reference83059028; boolean.
    Definition {
        section: "DMO",
        label: "kLoadDmoLocatorFromStream",
        title: "Load DMO Locators From Stream",
        initial: Value::Boolean(true),
    },
    //828: initializer82f8c008 at table82fd5484, reference830a71ec; boolean.
    Definition {
        section: "AI_DynamicObject",
        label: "kEnableDMOLocatorLoading",
        title: "kEnableDMOLocatorLoading",
        initial: Value::Boolean(true),
    },
    //829: initializer82f8c3a0 at table82fd54b0, reference830a71c8; boolean.
    Definition {
        section: "DMO_Debug_Locators",
        label: "kEnableTheHackToRemoveBadLocators",
        title: "Enable the Hack to Remove Bad Locators",
        initial: Value::Boolean(true),
    },
    //830: initializer82f8ca30 at table82fd5500, reference830a73d0; boolean.
    Definition {
        section: "DMO_SpawnTest",
        label: "kShowSpawnTestCapsule",
        title: "Show Capsule",
        initial: Value::Boolean(false),
    },
    //831: initializer82f8ca80 at table82fd5504, reference830a70d0; GUI range 0.01..2.
    Definition {
        section: "DMO_SpawnTest",
        label: "kSpawnTestCapsuleWidth",
        title: "Capsule Radius",
        initial: Value::Float(f32::from_bits(0x3f33_3333)),
    },
    //832: initializer82f8cae0 at table82fd5508, reference830a7210; GUI range 0..4.
    Definition {
        section: "DMO_SpawnTest",
        label: "kSpawnTestCapsuleHeight",
        title: "Capsule Height",
        initial: Value::Float(f32::from_bits(0x4033_3333)),
    },
    //833: initializer82f8cb40 at table82fd550c, reference830a771c; GUI range 0..1.
    Definition {
        section: "DMO_SpawnTest",
        label: "kSpawnTestCapsuleFeetPadding",
        title: "Capsule Feet Padding",
        initial: Value::Float(0.0),
    },
    //834: initializer82f8cba0 at table82fd5510, reference830a781c; GUI range 0..1.
    Definition {
        section: "DMO_SpawnTest",
        label: "kSpawnTestActorAABBHeadOffset",
        title: "Actor AABB Head Padding",
        initial: Value::Float(f32::from_bits(0x3e4c_cccd)),
    },
    //835: initializer82f8cc00 at table82fd5514, reference830a73f4; GUI range 0..20.
    Definition {
        section: "DMO_SpawnTest",
        label: "kSpawnTestCapsuleMinDistance",
        title: "Capsule minimum distance from DMO",
        initial: Value::Float(8.0),
    },
    //836: initializer82f5d648 at table82fd15dc, reference8304ea58; boolean.
    Definition {
        section: "AI_Spawner",
        label: "kEvaluateCensusForOneComponentPerFrame",
        title: "Evaluate Census for One Component Per Frame",
        initial: Value::Boolean(true),
    },
    //837: initializer82f5d698 at table82fd15e0, reference8304ea7c; boolean.
    Definition {
        section: "AI_Spawner",
        label: "kEvaluateCullForAllComponentsPerFrame",
        title: "Evaluate Cull for All Components Per Frame",
        initial: Value::Boolean(false),
    },
    //838: initializer82f5d7f0 at table82fd15f0, reference8304eb24; integer.
    Definition {
        section: "AI_Spawner",
        label: "kMAXDYNAMICOBJECTS",
        title: "Maximum Dynamic Objects Allowed",
        initial: Value::Integer {
            value: 49,
            minimum: 0,
            maximum: 49,
        },
    },
    //839: initializer82f5d848 at table82fd15f4, reference8304eb50; boolean.
    Definition {
        section: "AI_Spawner",
        label: "kEnableCull",
        title: "Enable Cull",
        initial: Value::Boolean(true),
    },
    //840: initializer82f5d898 at table82fd15f8, reference8304eb74; boolean.
    Definition {
        section: "AI_Spawner",
        label: "kEnableSpawn",
        title: "Enable Spawn",
        initial: Value::Boolean(true),
    },
    //841: initializer82f5d9d8 at table82fd1608, reference8304ec04; boolean.
    Definition {
        section: "AI_Spawner",
        label: "kEnableSpawn_DynamicObjects",
        title: "Enable Spawn Dynamic Objects",
        initial: Value::Boolean(true),
    },
    //842: initializer82f5dbb8 at table82fd1620, reference8304ecdc; GUI range 0.01..40.
    Definition {
        section: "AI_Spawner",
        label: "kCloseByLocatorSearchRadius",
        title: "Close By DMO Locator Search Radius",
        initial: Value::Float(20.0),
    },
    //843: initializer82f89af8 at table82fd5244, reference830a62fc; boolean.
    Definition {
        section: "AI_DynamicObject",
        label: "kEnableDMOModelLoading",
        title: "kEnableDMOModelLoading",
        initial: Value::Boolean(true),
    },
    //844: initializer82f8d368 at table82fd5578, reference830a74c0; boolean.
    Definition {
        section: "AI_DynamicObject",
        label: "kEnableDMOBankIDLookup",
        title: "Enable DMO Bank ID Lookup",
        initial: Value::Boolean(true),
    },
    //845: initializer82f9f0a8 at table82fd6324, reference830e102c; boolean.
    Definition {
        section: "DMO",
        label: "kEnableMainDMOBank",
        title: "Enable Main DMO Bank",
        initial: Value::Boolean(true),
    },
    //846: initializer82f69df0 at table82fd21e4, reference830523d0; boolean.
    Definition {
        section: "Physics_Debug_DMO",
        label: "kDmoGroundVerbose",
        title: "DMO Ground Verbose",
        initial: Value::Boolean(false),
    },
    //847: initializer82f69e40 at table82fd21e8, reference830523f4; boolean.
    Definition {
        section: "Physics_Debug_DMO",
        label: "kDrawDmoGroundTriangles",
        title: "Draw DMO Ground Triangles",
        initial: Value::Boolean(false),
    },
    //848: initializer82f69e90 at table82fd21ec, reference83052418; boolean.
    Definition {
        section: "Physics_Debug_DMO",
        label: "kDrawDmoGroundContacts",
        title: "Draw DMO Ground Contacts",
        initial: Value::Boolean(false),
    },
    //849: initializer82f69ee0 at table82fd21f0, reference8305243c; integer.
    Definition {
        section: "Physics_Debug_DMO",
        label: "kDmoGroundTrianglesToAllocate",
        title: "Triangles To Allocate",
        initial: Value::Integer {
            value: 0,
            minimum: 0,
            maximum: 10000,
        },
    },
    //850: initializer82f6a0d0 at table82fd2208, reference83052524; boolean.
    Definition {
        section: "Physics_Collision_Pipelines",
        label: "kRunDmoGroundCollisionPipeline",
        title: "Run DMO Ground Collision Pipeline",
        initial: Value::Boolean(true),
    },
    //851: initializer82f6a1c0 at table82fd2214, reference83052590; boolean.
    Definition {
        section: "Physics_Collision_Pipelines",
        label: "kRunSkaterDmoCollisionPipeline",
        title: "Run Skater DMO Collision Pipeline",
        initial: Value::Boolean(true),
    },
    //852: initializer82f6a2b0 at table82fd2220, reference830525fc; boolean.
    Definition {
        section: "Physics_Collision_Pipelines",
        label: "kRunDmoDmoCollisionPipeline",
        title: "Run DMO DMO Collision Pipeline",
        initial: Value::Boolean(true),
    },
    //853: initializer82f6a4e0 at table82fd223c, reference830526f8; boolean.
    Definition {
        section: "Physics_Pools_DmoPool",
        label: "kDmoPoolUpdateInParallel",
        title: "Update In Parallel",
        initial: Value::Boolean(false),
    },
    //854: initializer82f6b4f8 at table82fd232c, reference83052cdc; boolean.
    Definition {
        section: "Physics_Dmo",
        label: "kIgnoreVolumeEnableFlag",
        title: "Ignore Volume Enabled Flag",
        initial: Value::Boolean(true),
    },
    //855: initializer82f8c160 at table82fd5494, reference830a7634.
    Definition {
        section: "AI_DynamicObject",
        label: "kFeedDMOGrabSplines",
        title: "Feed DMO Grab Splines",
        initial: Value::Boolean(true),
    },
    //856..858: PlayerGrabSpline line validation; initializers82f982d8/330/390.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kNumGrabSplineLineTests",
        title: "Num Grab Spline Line Tests",
        initial: Value::Integer {
            value: 3,
            minimum: 1,
            maximum: 20,
        },
    },
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kGrabSplineLineTestRadius",
        title: "Grab Spline Line Test Thickness (m)",
        initial: Value::Float(0.2),
    },
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kGrabSplineLineTestVerticalOffset",
        title: "Grab Spline Line Test Vertical Offset (m)",
        initial: Value::Float(0.0),
    },
    //859: initializer82f92a48/table82fd5990, reference830d9644.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kDrawOffBoardGrab",
        title: "Show Grabbing",
        initial: Value::Boolean(false),
    },
    //860: initializer82f92f90/table82fd59cc, reference830d982c; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kOBPushingDebugDraw",
        title: "Debug Draw",
        initial: Value::Boolean(false),
    },
    //861: initializer82f92fe0/table82fd59d0, reference830d74dc; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kStayInState",
        title: "Stay In State",
        initial: Value::Boolean(false),
    },
    //862: initializer82f931c0/table82fd59e8, reference830d8e18; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kEnableRepushingTimeLimitOnExit",
        title: "Enable Repushing Time Limit On Exit",
        initial: Value::Boolean(true),
    },
    //863: initializer82f93210/table82fd59ec, reference830d8dc8; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kEnableEmergencyExit",
        title: "Enable Emergency Exit",
        initial: Value::Boolean(true),
    },
    //864: initializer82f93260/table82fd59f0, reference830d8a34; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kPushingMaxArmLengthForIK",
        title: "Max Arm Length",
        initial: Value::Float(f32::from_bits(0x3f266666)),
    },
    //865: initializer82f932c0/table82fd59f4, reference830d8278; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kOBP_MaxTimePartialRagdoll",
        title: "Max Time Ragdoll",
        initial: Value::Float(f32::from_bits(0x3ecccccd)),
    },
    //866: initializer82f93320/table82fd59f8, reference830d8734; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kMOVOBJ_ForceSmoothing",
        title: "Force Smoothing",
        initial: Value::Float(f32::from_bits(0x3dcccccd)),
    },
    //867: initializer82f933e0/table82fd5a00, reference830d83c8; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kPushFromTurnScalar",
        title: "pushfromturnscalar",
        initial: Value::Float(f32::from_bits(0xbe800000)),
    },
    //868: initializer82f93440/table82fd5a04, reference830d8018; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kOBP_CollisionVelScalar",
        title: "coll vel scalar",
        initial: Value::Float(f32::from_bits(0x3f000000)),
    },
    //869: initializer82f934a0/table82fd5a08, reference830d913c; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kMaxJerkPerFrame",
        title: "Max jerk per frame",
        initial: Value::Float(f32::from_bits(0x40800000)),
    },
    //870: initializer82f93500/table82fd5a0c, reference830d99b0; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kOBP_EnableCollResponse",
        title: "Enable Coll Response",
        initial: Value::Boolean(true),
    },
    //871: initializer82f93550/table82fd5a10, reference830d7bd0; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kMOVOBJ_Crouch_Scalar",
        title: "Crouch scalar",
        initial: Value::Float(f32::from_bits(0x3dcccccd)),
    },
    //872: initializer82f935b0/table82fd5a14, reference830d987c; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kMOVOBJ_Crouch_Scalar_Initial",
        title: "Initial Crouch scalar",
        initial: Value::Float(f32::from_bits(0x3f000000)),
    },
    //873: initializer82f93610/table82fd5a18, reference830d78ac; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kMOVOBJ_HandsZOffset",
        title: "Hands Z Offset",
        initial: Value::Float(f32::from_bits(0x3e000000)),
    },
    //874: initializer82f93670/table82fd5a1c, reference830d8edc; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kMOVOBJ_MaxHeightAboveGround",
        title: "Max Height Above Ground",
        initial: Value::Float(f32::from_bits(0x3ccccccd)),
    },
    //875: initializer82f936d0/table82fd5a20, reference830d9cb8; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kMOVOBJ_ApproxSpeedBlend",
        title: "Approx Speed Blend",
        initial: Value::Float(f32::from_bits(0x3e19999a)),
    },
    //876: initializer82f93730/table82fd5a24, reference830d7d78; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kMOVOBJ_worldPositionMinDelta",
        title: "Min World Position Delta",
        initial: Value::Float(f32::from_bits(0x3c23d70a)),
    },
    //877: initializer82f93790/table82fd5a28, reference830d94ec; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kMOVOBJ_worldPositionMinBlend",
        title: "Min World Position Blend",
        initial: Value::Float(f32::from_bits(0x3dcccccd)),
    },
    //878: initializer82f937f0/table82fd5a2c, reference830d89b0; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kMOVOBJ_NoSpeedTimeout",
        title: "No Effects From Speed Timeout",
        initial: Value::Float(f32::from_bits(0x3dcccccd)),
    },
    //879: initializer82f93850/table82fd5a30, reference830d7500; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kMOVOBJ_HandsLevelToBeginIKBlend",
        title: "Time To Start IK Hands Blend",
        initial: Value::Float(f32::from_bits(0x3dcccccd)),
    },
    //880: initializer82f938b0/table82fd5a34, reference830d7704; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kMOVOBJ_MinHookToHips",
        title: "MinHook2HipsForCollisionReaction",
        initial: Value::Float(f32::from_bits(0x3ba3d70a)),
    },
    //881: initializer82f93910/table82fd5a38, reference830d8518; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kMOVOBJ_MinHookToHipsForExit",
        title: "MinHook2HipsForExit",
        initial: Value::Float(f32::from_bits(0x3e4ccccd)),
    },
    //882: initializer82f93970/table82fd5a3c, reference830d8660; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kMOVOBJ_CollisionBlendIn",
        title: "Collision Blend In",
        initial: Value::Float(f32::from_bits(0x3f000000)),
    },
    //883: initializer82f93a30/table82fd5a44, reference830d7988; null callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kEnableOffBoardPushingWipeouts",
        title: "Enable Wipeouts",
        initial: Value::Boolean(true),
    },
    //884: initializer82f939d0, reference830d8444; raw r10=0 callback.
    Definition {
        section: "Physics_States_OffBoard_Pushing",
        label: "kMOVOBJ_CollisionBlendOut",
        title: "Collision Blend Out",
        initial: Value::Float(f32::from_bits(0x3f73_3333)),
    },
    //885: initializer82f69800; null callback verified from raw ABI.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kOffBoardInteractableObjectNumLineTests",
        title: "Interactable Object Detection Num Line Tests",
        initial: Value::Integer {
            value: 5,
            minimum: 0,
            maximum: 10,
        },
    },
    //886: initializer82f69858; null callback verified from raw ABI.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kOffBoardInteractableObjectLineTestFatness",
        title: "Interactable Object Detection Line Test Fatness",
        initial: Value::Float(f32::from_bits(0x3e4c_cccd)),
    },
    //887: initializer82f698b8; null callback verified from raw ABI.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kOffBoardInteractableObjectLineTestsMaxHeight",
        title: "Interactable Object Detection Line Test Max Height",
        initial: Value::Float(f32::from_bits(0x3fe6_6666)),
    },
    //888: initializer82f69918; null callback verified from raw ABI.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kOffBoardInteractableObjectLineTestDistance",
        title: "Interactable Object Detection Line Test Distance",
        initial: Value::Float(2.0),
    },
    //889: initializer82f69978; null callback verified from raw ABI.
    Definition {
        section: "Physics_States_OffBoard",
        label: "kOffBoardDrawInteractableObjectLineTests",
        title: "Draw Interactable Object Detection Line Tests",
        initial: Value::Boolean(false),
    },
    //890: ref830a3754/init82f86770/table82fd4f34; GUI0..1, null callback.
    Definition {
        section: "Animation_MovingObjects",
        label: "kMOStrengthMaxDelta",
        title: "Strength Max Delta",
        initial: Value::Float(f32::from_bits(0x3c23_d70a)),
    },
    //891: ref830a3e6c/init82f86830/table82fd4f3c; GUI0..1, null callback.
    Definition {
        section: "Animation_MovingObjects",
        label: "kMOSpeedMaxDelta",
        title: "Speed Max Delta",
        initial: Value::Float(f32::from_bits(0x3cf5_c28f)),
    },
    //892: ref830a345c/init82f86890/table82fd4f40; GUI0..5, null callback.
    Definition {
        section: "Animation_MovingObjects",
        label: "kMORawSpeedMax",
        title: "Raw Speed Max",
        initial: Value::Float(2.5),
    },
    //893: ref830a4e78/init82f868f0/table82fd4f44; GUI0..1, null callback.
    Definition {
        section: "Animation_MovingObjects",
        label: "kMOCrouchMaxDelta",
        title: "Crouch Max Delta",
        initial: Value::Float(f32::from_bits(0x3cf5_c28f)),
    },
    //894: ref83054f04/init82f6eb00/table82fd264c; null callback.
    Definition {
        section: "Render_Settings",
        label: "kRenderLivingWorldOpacityEnabled",
        title: "Enable Livingworld Distance Based Opacity",
        initial: Value::Boolean(true),
    },
    //895: ref83054f28/init82f6eb50/table82fd2650; GUI1..120, raw r10=0.
    Definition {
        section: "Render_Settings",
        label: "kLivingWorldTransparancyFadeinFrames",
        title: "Living World Transparancy Fadein Frames",
        initial: Value::Float(30.0),
    },
    //896: ref830db56c/init82f98e18/table82fd5e2c; GUI0..1, null callback.
    Definition {
        section: "Physics_Systems_OffboardIK",
        label: "kOBIK_IKMaxDist",
        title: "IK MaxDist",
        initial: Value::Float(f32::from_bits(0x3da3_d70a)),
    },
    //897: ref830d97d4/init82f8f800/table82fd5770; GUI0..20, null callback.
    Definition {
        section: "Physics_States_Skitching",
        label: "kSearchRadius",
        title: "Search Radius",
        initial: Value::Float(5.0),
    },
    //898: ref830d998c/init82f8f860/table82fd5774; null callback.
    Definition {
        section: "Physics_States_PhysicsGround",
        label: "kDrawGrabbableObjectTest",
        title: "Grabable Object Draw",
        initial: Value::Boolean(false),
    },
    //899: ref83052250/init82f699c8/table82fd21a0; null callback.
    Definition {
        section: "AI_Vehicle",
        label: "kUseAuthoredVehicleGrabSplines",
        title: "Use Authored Grab Splines",
        initial: Value::Boolean(true),
    },
    //900: ref830a6eb8/init82f8b558/table82fd53e8; null callback.
    Definition {
        section: "AI_Vehicle",
        label: "kFeedVehicleGrabSplines",
        title: "Feed Vehicle Grab Splines",
        initial: Value::Boolean(true),
    },
    //901..903: initializers82f8f8b0/82f8f910/82f8f970, null callbacks.
    Definition {
        section: "Physics_States_Skitching",
        label: "kSkitchMaxEntryAngle",
        title: " kSkitchMaxEntryAngle",
        initial: Value::Float(60.0),
    },
    Definition {
        section: "Physics_States_Skitching",
        label: "kSkitchMaxHorizontalEntryAngle",
        title: " kSkitchMaxHorizontalEntryAngle",
        initial: Value::Float(40.0),
    },
    Definition {
        section: "Physics_States_Skitching",
        label: "kSkitchMinApproachSpeed",
        title: "SkitchMinApproachSpeed",
        initial: Value::Float(0.5),
    },
    //904: ref8304fb58/initializer82f649d8; null callback.
    Definition {
        section: "AI_RAIN",
        label: "kEnableKDTreeRemovals",
        title: "Enable Removals from the KDTrees",
        initial: Value::Boolean(false),
    },
    //905..907: source initializers82f64738/82f64790/82f64848; null callbacks.
    Definition {
        section: "AI_LivingWorld_Recipe",
        label: "kMaxInitialPassModelLoads",
        title: "Max Initial Base Model Load",
        initial: Value::Integer {
            value: 15,
            minimum: 0,
            maximum: 64,
        },
    },
    Definition {
        section: "AI_LivingWorld_Recipe",
        label: "kMaxSecondaryPassModelLoads",
        title: "Max Secondary Base Model Load",
        initial: Value::Integer {
            value: 10,
            minimum: 0,
            maximum: 64,
        },
    },
    Definition {
        section: "AI_LivingWorld_Recipe",
        label: "kUseForcedRecipes",
        title: "Use Forced Recipes",
        initial: Value::Boolean(false),
    },
    //908/909: initializers82f5dc68/82f5df70; refs8304ed2c/8304eea0; null callbacks.
    Definition {
        section: "AI_Spawner",
        label: "kIgnoreWorldPainterEntityCategories",
        title: "Ignore World Painter Entity Categories",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "AI_Spawner",
        label: "kIgnoreFECensusSliders",
        title: "Ignore FE Census Sliders",
        initial: Value::Boolean(false),
    },
    //910: initializer82fa43e0/table82fd67b0, reference8311fce0; null callback.
    Definition {
        section: "AI_RAIN",
        label: "kDebugOverrideIntersectionRules",
        title: "Override Intersection Rules",
        initial: Value::Boolean(false),
    },
    //911: initializer82f8b818/table82fd5408, reference830a6f90; null callback.
    Definition {
        section: "AI_Vehicle",
        label: "kCheckVehiclesInParkingLaneBeforePullingOver",
        title: "Check Vehicles in the Parking Lane Before Pulling Over",
        initial: Value::Boolean(false),
    },
    //912: initializer82f8bfa8/table82fd5480, reference830a6fb4; GUI0..10, r10=0.
    Definition {
        section: "AI_Vehicle",
        label: "kBrakeLightOnDelay",
        title: "Brake Light Turn On Delay",
        initial: Value::Float(0.5),
    },
    //913: initializer82f6a530/table82fd2240, reference8305271c; null callback.
    Definition {
        section: "Physics_Pools_VehiclePool",
        label: "kVehiclePoolUpdateInParallels",
        title: "Update In Parallel",
        initial: Value::Boolean(false),
    },
    //914: init82f8b758/table82fd5400/ref830a6edc; GUI.001..10000, null callback.
    Definition {
        section: "AI_LivingWorld_Collision_Vehicle",
        label: "kVehicleWorldCollisionDensity",
        title: "World Collision Density",
        initial: Value::Float(500.0),
    },
    //915: init82fa39e8/table82fd6738/ref83120300; null callback.
    Definition {
        section: "AI_LivingWorld_Collision",
        label: "kLWClampVelocity",
        title: "Clamp velocity of LW bodies",
        initial: Value::Boolean(true),
    },
    //916: init82fa3a38/table82fd673c/ref8311fd4c; GUI0..1000, null callback.
    Definition {
        section: "AI_LivingWorld_Collision",
        label: "kLWClampMaxVelocity",
        title: "Clamp velocity of LW bodies",
        initial: Value::Float(60.0),
    },
    //917: init82f6a210/table82fd2218/ref830525b4; null callback.
    Definition {
        section: "Physics_Collision_Pipelines",
        label: "kRunSkaterVehicleCollisionPipeline",
        title: "Run Skater Vehicle Collision Pipeline",
        initial: Value::Boolean(true),
    },
    //918: init82f6a350/table82fd2228/ref83052644; null callback.
    Definition {
        section: "Physics_Collision_Pipelines",
        label: "kRunDmoVehicleCollisionPipeline",
        title: "Run DMO Vehicle Collision Pipeline",
        initial: Value::Boolean(true),
    },
    //919: init82f8b658/table82fd53f4/ref830a6e48; GUI .1..1000, raw r10=null.
    Definition {
        section: "Vehicle_Spawner",
        label: "kSpawnVehicleDMOCheckRadius",
        title: "Vehicle Spawn DMO check radius",
        initial: Value::Float(20.0),
    },
    //920: init82fa37b8/table82fd671c/ref83120218; null callback.
    Definition {
        section: "AI_Prop",
        label: "kAllowContainedProps",
        title: "Allow Contained Props",
        initial: Value::Boolean(true),
    },
    //921: init82f5d320/table82fd15b8/ref8304e8dc; null callback.
    Definition {
        section: "AI_LivingWorld",
        label: "kbUseGeneratedStateGraphForLW",
        title: "Use Generated State Graph for LW",
        initial: Value::Boolean(true),
    },
    //922..924: init82f8b450/4a8/500, table82fd53dc/e0/e4. Source titles
    // for LoFi/NoFi are swapped; retain the original labels/values verbatim.
    Definition {
        section: "AI_Vehicle_Sys",
        label: "kHiFiVehiclesFrameCount",
        title: "FrameRate For HiFi Vehicles",
        initial: Value::Integer {
            value: 30,
            minimum: 0,
            maximum: 60,
        },
    },
    Definition {
        section: "AI_Vehicle_Sys",
        label: "kLoFiVehiclesFrameCount",
        title: "FrameRate For NoFi Vehicles",
        initial: Value::Integer {
            value: 10,
            minimum: 0,
            maximum: 60,
        },
    },
    Definition {
        section: "AI_Vehicle_Sys",
        label: "kNoFiVehiclesFrameCount",
        title: "FrameRate For LoFi Vehicles",
        initial: Value::Integer {
            value: 3,
            minimum: 0,
            maximum: 60,
        },
    },
    //925: init82f5de70/table82fd1640/ref8304ee24; GUI0..24, null callback.
    Definition {
        section: "AI_Spawner",
        label: "kInitialSpawnVehicleCount",
        title: "FrameSaver Initial Spawn Vehicle Allowance",
        initial: Value::Integer {
            value: 15,
            minimum: 0,
            maximum: 24,
        },
    },
    //926: init82f5d8e8/table82fd15fc/ref8304eb98, null callback.
    Definition {
        section: "AI_Spawner",
        label: "kEnableSpawn_Vehicles",
        title: "Enable Spawn Vehicles",
        initial: Value::Boolean(true),
    },
    //927..929: init82f64ad8/4b28/4b78, table82fd1ca0/a4/a8; null callbacks.
    Definition {
        section: "Debug_Scoring_Wipeouts",
        label: "kDisableVehicleAvoidanceOfSkaterDuringAnyBail",
        title: "kDisableVehicleAvoidanceOfSkaterDuringAnyBail",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Debug_Scoring_Wipeouts",
        label: "kDisableVehicleAvoidanceOfSkaterDuringAirborneBail",
        title: "kDisableVehicleAvoidanceOfSkaterDuringAirborneBail",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Debug_Scoring_Wipeouts",
        label: "kDisableVehicleAvoidanceOfSkater",
        title: "kDisableVehicleAvoidanceOfSkater",
        initial: Value::Boolean(false),
    },
    //930..935:82f5d230,82f5d478..d638; original refs8304e870,e984,e9a8,e9d4,ea00,ea2c.
    Definition {
        section: "AI_LivingWorld",
        label: "kDrawLWPosTest",
        title: "Debug draw pos test",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "AI_LivingWorld_Physics",
        label: "kDrawInsideVehicleTest",
        title: "Draw inside vehicle test",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "AI_LivingWorld_Physics",
        label: "kInsideVehicleTest_X",
        title: "X-size",
        initial: Value::Float(f32::from_bits(0x3ecccccd)),
    },
    Definition {
        section: "AI_LivingWorld_Physics",
        label: "kInsideVehicleTest_YSize",
        title: "Y-Size",
        initial: Value::Float(f32::from_bits(0x3f266666)),
    },
    Definition {
        section: "AI_LivingWorld_Physics",
        label: "kInsideVehicleTest_Z",
        title: "Z-size",
        initial: Value::Float(0.5),
    },
    Definition {
        section: "AI_LivingWorld_Physics",
        label: "kInsideVehicleTest_YOffset",
        title: "Y offset",
        initial: Value::Float(0.),
    },
    //936..946: HandPlant initializers82f94a10,82f96010/c0,82f96290..96510.
    Definition {
        section: "Physics_HandPlant",
        label: "kHandPlantStateDisplayDebug",
        title: "Display State Debug",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_HandPlant",
        label: "kHandPlantApexUpBlend",
        title: "Apex Up Blend",
        initial: Value::Float(1.0),
    },
    Definition {
        section: "Physics_HandPlant",
        label: "kHandPlantGroundNormalY",
        title: "Ground Normal Y",
        initial: Value::Float(f32::from_bits(0x3f0c_cccd)),
    },
    Definition {
        section: "Physics_HandPlant",
        label: "kHandPlantUseIk",
        title: "Use Ik",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Physics_HandPlant",
        label: "kHandPlantMaxArmLength",
        title: "IK Max Arm Length",
        initial: Value::Float(f32::from_bits(0x3f26_6666)),
    },
    Definition {
        section: "Physics_HandPlant_PositionSelector",
        label: "kHandPlantPosDisplayDebug",
        title: "Display Debug",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_HandPlant_PositionSelector",
        label: "kHandPlantHalfBoxX",
        title: "Half Box X",
        initial: Value::Float(2.0),
    },
    Definition {
        section: "Physics_HandPlant_PositionSelector",
        label: "kHandPlantHalfBoxY",
        title: "Half Box Y",
        initial: Value::Float(4.0),
    },
    Definition {
        section: "Physics_HandPlant_PositionSelector",
        label: "kHandPlantHalfBoxZ",
        title: "Half Box Z",
        initial: Value::Float(2.0),
    },
    Definition {
        section: "Physics_HandPlant_PositionSelector",
        label: "kHPS_MinDotWithHorizPlane",
        title: "Min Dot With X",
        initial: Value::Float(f32::from_bits(0x3f35_c28f)),
    },
    Definition {
        section: "Physics_HandPlant_PositionSelector",
        label: "kHPS_DrawPrimitives",
        title: "Draw Primitives",
        initial: Value::Boolean(false),
    },
    //947..955: source82f949b0,82f96120,82f96230,82f96670..96790,
    //82f98750,82f96560; same live target registry as all other physics cVars.
    Definition {
        section: "Physics_States_HandPlant",
        label: "kHP_UpVectorBlend",
        title: "UpVectorBlend",
        initial: Value::Float(1.0),
    },
    Definition {
        section: "Physics_HandPlant",
        label: "kMinTimeFromApexToGround",
        title: "Min Time To Ground",
        initial: Value::Float(0.5),
    },
    Definition {
        section: "Physics_HandPlant",
        label: "kFramesOfControlVector",
        title: "Frames for Control",
        initial: Value::Float(6.0),
    },
    Definition {
        section: "Physics_States_HandPlant_Traj",
        label: "kDistFromGrindToStartTrajectories",
        title: "Dist to Start traj",
        initial: Value::Float(0.1),
    },
    Definition {
        section: "Physics_States_HandPlant_Traj",
        label: "kHPTS_AngleDelta",
        title: "Angle Delta",
        initial: Value::Float(0.24),
    },
    Definition {
        section: "Physics_States_HandPlant_Traj",
        label: "kHPTS_LandingOffset",
        title: "LandingOffset",
        initial: Value::Float(0.56),
    },
    Definition {
        section: "Physics_States_HandPlant_Traj",
        label: "kHPTS_MinDistToLanding",
        title: "MinYDistTOLand",
        initial: Value::Float(0.5),
    },
    Definition {
        section: "Physics_Systems_SurfaceQuery",
        label: "kSurfaceQueryRadius",
        title: "kSurfaceQueryRadius",
        initial: Value::Float(0.01),
    },
    Definition {
        section: "Physics_States_HandPlant",
        label: "kbHP_TRAJ_SELECTOR_DebugDraw",
        title: "Debug Draw ",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Physics_States_HandPlant",
        label: "kHP_UpVectorMaxDelta",
        title: "UpVectorMaxDelta",
        initial: Value::Float(1.0),
    },
    // 957..959: initializers82f88690/82f88740/82f887a0; raw PPC confirms
    // null callbacks, false debug flags, delta0x3d23d70a (bounds0..1).
    Definition {
        section: "Animation_Skitching",
        label: "kSkitchDrawDebugInfo",
        title: "kSkitchDrawDebugInfo",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Animation_Skitching",
        label: "kSkitchMaxAbsorbSpeedDelta",
        title: "Max Absorb Speed Delta",
        initial: Value::Float(f32::from_bits(0x3d23_d70a)),
    },
    Definition {
        section: "Animation_Skitching",
        label: "kbSkitchEnterDebugDisplay",
        title: "Skitch Enter Debug Display",
        initial: Value::Boolean(false),
    },
    // 960: initializer0x82f94110.
    Definition {
        section: "Physics_States_Skitching",
        label: "kSkitchingUseBetterSteerForceNormals",
        title: "Use Better Steer/Force",
        initial: Value::Boolean(false),
    },
    // 961: initializer0x82f940c0.
    Definition {
        section: "Physics_States_Skitching",
        label: "kForceSkitchStick",
        title: "Force Skitch Stick",
        initial: Value::Boolean(false),
    },
    // 962: initializer0x82f94160.
    Definition {
        section: "Physics_States_Skitching",
        label: "kSkitchingAllowManuals",
        title: "Allow Manuals",
        initial: Value::Boolean(true),
    },
    // 963: initializer0x82f941b0.
    Definition {
        section: "Physics_States_Skitching",
        label: "kNumberOfFramesToDisableHandsAfterSkitch",
        title: "Frames to disable hands",
        initial: Value::Integer {
            value: 5,
            minimum: 0,
            maximum: 30,
        },
    },
    // 964: initializer0x82f94208.
    Definition {
        section: "Physics_States_Skitching",
        label: "kEnableFakeFollowPointPhysics",
        title: "Enable Fake FollowPoint Physics",
        initial: Value::Boolean(true),
    },
    // 965: initializer0x82f94258.
    Definition {
        section: "Physics_States_Skitching",
        label: "kEnableCentrifugalAcceleration",
        title: "Enable Centrifugal Acceleration",
        initial: Value::Boolean(true),
    },
    // 966: initializer0x82f943c8.
    Definition {
        section: "Physics_States_Skitching",
        label: "kSkitchMinTurnForExit",
        title: "Min Turn For Exit",
        initial: Value::Float(0.5),
    },
    // 967: initializer0x82f94428.
    Definition {
        section: "Physics_States_Skitching",
        label: "kStickHandSlowDownEffect",
        title: "Stick Slow Down",
        initial: Value::Float(1.0),
    },
    // 968: initializer0x82f94488.
    Definition {
        section: "Physics_States_Skitching",
        label: "kDeckBlendAmount",
        title: "Deck Blend Amount",
        initial: Value::Float(0.25),
    },
    // 969: initializer0x82f944e8.
    Definition {
        section: "Physics_States_Skitching",
        label: "kDeckMaxOffsetDelta",
        title: "Deck Offset Delta ",
        initial: Value::Float(10.0),
    },
    // 970: initializer0x82f94548.
    Definition {
        section: "Physics_States_Skitching",
        label: "kMinDeckOffsetAngle",
        title: "Min Deck Offset ",
        initial: Value::Float(2.0),
    },
    // 971: initializer0x82f945a8.
    Definition {
        section: "Physics_States_Skitching",
        label: "kSkitchEnableIK",
        title: "Enable IK",
        initial: Value::Boolean(true),
    },
    // 972: initializer0x82f945f8.
    Definition {
        section: "Physics_States_Skitching",
        label: "kSkitchForceLeftHand",
        title: "ForceLeftHand",
        initial: Value::Boolean(false),
    },
    // 973: initializer0x82f94648.
    Definition {
        section: "Physics_States_Skitching",
        label: "kSkitchForceRightHand",
        title: "ForceRightHand",
        initial: Value::Boolean(false),
    },
    // 974: initializer0x82f94698.
    Definition {
        section: "Physics_States_Skitching",
        label: "kSkitchCrouchSpringScalar",
        title: "Crouch spring scalar",
        initial: Value::Float(0.1),
    },
    // 975: initializer0x82f946f8.
    Definition {
        section: "Physics_States_Skitching",
        label: "kSkitchingApplyStandardTilt",
        title: "Apply Standard Tilt",
        initial: Value::Boolean(true),
    },
    // 976: initializer0x82f94748.
    Definition {
        section: "Physics_States_Skitching",
        label: "kSkitchingApplySpeedWobble",
        title: "Apply Speed Wobble",
        initial: Value::Boolean(true),
    },
    // 977: initializer0x82f94798.
    Definition {
        section: "Physics_States_Skitching",
        label: "kFramesToExitSkitching",
        title: "Frames To Exit Skitching",
        initial: Value::Integer {
            value: 3,
            minimum: 0,
            maximum: 10,
        },
    },
    // 978: initializer0x82f948b0.
    Definition {
        section: "Physics_States_Skitching",
        label: "kSkitchingUseTorque",
        title: "Use Torque to Autosteer Skater",
        initial: Value::Boolean(true),
    },
    // 979: initializer0x82f94950.
    Definition {
        section: "Physics_States_Skitching",
        label: "kSkitchDebugHandSpeed",
        title: "Debug Hand Speed",
        initial: Value::Float(0.0),
    },
    // 980: initializer0x82f886e0.
    Definition {
        section: "Animation_Skitching",
        label: "kSkitchMaxCrouchDelta",
        title: "Max Crouch Delta",
        initial: Value::Float(0.02),
    },
    // 981: initializer0x82f887f0.
    Definition {
        section: "Animation_Skitching",
        label: "kbSkitchShimmyDebugDisplay",
        title: "Skitch Shimmy Debug Display",
        initial: Value::Boolean(false),
    },
    // 982: native initializer0x82f9d490.
    Definition {
        section: "Debug_Scoring",
        label: "kOverrideScoreMultiplier",
        title: "kOverrideScoreMultiplier",
        initial: Value::Boolean(false),
    },
    // 983: native initializer0x82f9d4e0.
    Definition {
        section: "Debug_Scoring",
        label: "kOverrideScoreMultiplierValue",
        title: "kOverrideScoreMultiplierValue",
        initial: Value::Integer {
            value: 3,
            minimum: 0,
            maximum: 3,
        },
    },
    // 984: native initializer0x82f9d590.
    Definition {
        section: "Debug_Scoring",
        label: "kMinAnimScalarForSketchyLanding",
        title: "kMinAnimScalarForSketchyLanding",
        initial: Value::Float(0.5),
    },
    // 985: native initializer0x82f9d5f0.
    Definition {
        section: "Debug_Scoring",
        label: "kMakeAllLandingsSketchy",
        title: "kMakeAllLandingsSketchy",
        initial: Value::Boolean(false),
    },
    // 986: native initializer0x82f9d7d0.
    Definition {
        section: "Debug_Scoring_Contextual",
        label: "kScoreEntityTypeInvalidDMOs",
        title: "kScoreEntityTypeInvalidDMOs",
        initial: Value::Boolean(false),
    },
    // 987: native initializer0x82f9d820.
    Definition {
        section: "Debug_Scoring_Contextual",
        label: "kSkaterContextRadius",
        title: "kSkaterContextRadius",
        initial: Value::Float(0.8),
    },
    // 988: native initializer0x82f9d880.
    Definition {
        section: "Debug_Scoring_Contextual",
        label: "kPedContextRadius",
        title: "kPedContextRadius",
        initial: Value::Float(0.55),
    },
    // 989: native initializer0x82f9d8e0.
    Definition {
        section: "Debug_Scoring",
        label: "kMinDistancePerFrameToAccumulate",
        title: "kMinDistancePerFrameToAccumulate",
        initial: Value::Float(f32::from_bits(0x3a83126f)),
    },
    // 990: native initializer0x82f9d940.
    Definition {
        section: "Debug_Scoring_Ground",
        label: "kAllowSlowManualsToAlwaysScore",
        title: "kAllowSlowManualsToAlwaysScore",
        initial: Value::Boolean(true),
    },
    // 991: native initializer0x82f9d990.
    Definition {
        section: "Debug_Scoring_Ground",
        label: "kAllowSlowManualsToStillScoreOnceBegun",
        title: "kAllowSlowManualsToStillScoreOnceBegun",
        initial: Value::Boolean(true),
    },
    // 992: native initializer0x82f9d9e0.
    Definition {
        section: "Debug_Scoring_Ground",
        label: "kMinFramesBeforeEndingSequenceAfterLanding",
        title: "kMinFramesBeforeEndingSequenceAfterLanding",
        initial: Value::Integer {
            value: 3,
            minimum: 0,
            maximum: 600,
        },
    },
    // 993: native initializer0x82f9da38.
    Definition {
        section: "Debug_Scoring_Ground",
        label: "kExtraFramesThatOffGroundWheelsKeepSequenceActive",
        title: "kExtraFramesThatOffGroundWheelsKeepSequenceActive",
        initial: Value::Integer {
            value: 5,
            minimum: 0,
            maximum: 600,
        },
    },
    // 994..1002: original HoM references82f9db20/def8/dda0/de98/de40/dd50/dfb8/df58/ddf0.
    Definition {
        section: "Debug_Scoring_Wipeouts",
        label: "kForceCharacterRenderDamageLevel",
        title: "kForceCharacterRenderDamageLevel",
        initial: Value::Enumeration {
            value: 0,
            tags: &[
                ("Disabled", 0),
                ("OK", 1),
                ("Scrape", 2),
                ("Bruise", 3),
                ("Sprain", 4),
                ("Dislocate", 5),
                ("Break Part", 6),
                ("Break All", 7),
            ],
        },
    },
    Definition {
        section: "Debug_Scoring_Wipeouts",
        label: "kDamageScalar",
        title: "kDamageScalar",
        initial: Value::Float(1.0),
    },
    Definition {
        section: "Debug_Scoring_Wipeouts",
        label: "kEnableBoneDamagePropogation",
        title: "kEnableBoneDamagePropogation",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Debug_Scoring_Wipeouts",
        label: "kMinDamageToPropogate",
        title: "kMinDamageToPropogate",
        initial: Value::Float(0.1),
    },
    Definition {
        section: "Debug_Scoring_Wipeouts",
        label: "kNumFramesToResetHeavyLandingData",
        title: "kNumFramesToResetHeavyLandingData",
        initial: Value::Integer {
            value: 3,
            minimum: 1,
            maximum: 10,
        },
    },
    Definition {
        section: "Debug_Scoring_Wipeouts",
        label: "kUseBoneAccelerationForForce",
        title: "kUseBoneAccelerationForForce",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Debug_Scoring_Wipeouts",
        label: "kDamageFadeInRate",
        title: "kDamageFadeInRate",
        initial: Value::Float(1.0),
    },
    Definition {
        section: "Debug_Scoring_Wipeouts",
        label: "kMinAngleToBankInDegrees",
        title: "kMinAngleToBankInDegrees",
        initial: Value::Float(30.0),
    },
    Definition {
        section: "Debug_Scoring_Wipeouts",
        label: "kIgnoreHoMRotationWhenStationary",
        title: "kIgnoreHoMRotationWhenStationary",
        initial: Value::Boolean(true),
    },
    // Original World producer references82f69a18/9a68/9ab8.
    Definition {
        section: "Debug_Scoring_Wipeouts",
        label: "kIsHoMEffectEnabled",
        title: "kIsHoMEffectEnabled",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Debug_Scoring_Wipeouts",
        label: "kIsHoMEffectForceEnabled",
        title: "kIsHoMEffectForceEnabled",
        initial: Value::Boolean(false),
    },
    Definition {
        section: "Debug_Scoring_Spatial",
        label: "kbDoSpatialScoreTest",
        title: "kbDoSpatialScoreTest",
        initial: Value::Boolean(false),
    },
    // Original frontend initializers82f59598/82f59688/82f57c30/82f57bd0.
    Definition {
        section: "FE",
        label: "kbAllowTrickDisplay",
        title: "AllowTrickDisplay",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "FE",
        label: "kTrickDisplayNollieTime",
        title: "Trick Display Nollie Delay",
        initial: Value::Integer {
            value: 140,
            minimum: 0,
            maximum: 300,
        },
    },
    Definition {
        section: "FE",
        label: "kfJoltThreshold",
        title: "3D HUD - Jolt Activation Height",
        initial: Value::Float(f32::from_bits(0x3f3ae148)),
    },
    Definition {
        section: "FE",
        label: "kfJoltDistance",
        title: "3D HUD - Jolt Distance",
        initial: Value::Float(50.),
    },
    // Native initializers82f57b70/82f57c90; same shared HUD jolt as phone.
    Definition {
        section: "FE",
        label: "kfJoltSpeed",
        title: "3D HUD - Initial Jolt Speed",
        initial: Value::Float(9.98),
    },
    Definition {
        section: "FE",
        label: "kfJoltRecovery",
        title: "3D HUD - Jolt Recovery",
        initial: Value::Float(0.3258),
    },
    // 1012..1015: source initializers82f5d740/82f5d938/82f5da28/82f5dec8.
    Definition {
        section: "AI_Spawner",
        label: "kMAXPEDESTRIANS",
        title: "Maximum Pedestrians Allowed",
        initial: Value::Integer {
            value: 39,
            minimum: 0,
            maximum: 39,
        },
    },
    Definition {
        section: "AI_Spawner",
        label: "kEnableSpawn_Pedestrians",
        title: "Enable Spawn Pedestrians",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "AI_Spawner",
        label: "kEnableSpawn_Security",
        title: "Enable Spawn Security",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "AI_Spawner",
        label: "kInitialSpawnPedestrianCount",
        title: "FrameSaver Initial Spawn Pedestrian Allowance",
        initial: Value::Integer {
            value: 12,
            minimum: 0,
            maximum: 36,
        },
    },
    // Native pedestrian navigation initializers82fa6af8/6e48..70c8/8cf8.
    Definition {
        section: "Pedestrian_Navigation",
        label: "kRoadNavTargetTimeSpeed",
        title: "Wander Target Time Speed",
        initial: Value::Float(0.5),
    },
    Definition {
        section: "Pedestrian_Navigation",
        label: "kEnableNoRoadWanderNavigation",
        title: "No Road Wander Enable",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Pedestrian_Navigation",
        label: "kNoRoadWanderFarNumAttempts",
        title: "No Road Wander Far Num Attempts",
        initial: Value::Integer {
            value: 5,
            minimum: 0,
            maximum: 30,
        },
    },
    Definition {
        section: "Pedestrian_Navigation",
        label: "kNoRoadWanderFarDistance",
        title: "No Road Wander Far Distance",
        initial: Value::Float(40.),
    },
    Definition {
        section: "Pedestrian_Navigation",
        label: "kNoRoadWanderFarAngle",
        title: "No Road Wander Far Angle",
        initial: Value::Float(90.),
    },
    Definition {
        section: "Pedestrian_Navigation",
        label: "kNoRoadWanderCloseNumAttempts",
        title: "No Road Wander Close Num Attempts",
        initial: Value::Integer {
            value: 9,
            minimum: 0,
            maximum: 30,
        },
    },
    Definition {
        section: "Pedestrian_Navigation",
        label: "kNoRoadWanderCloseDistance",
        title: "No Road Wander Close Distance",
        initial: Value::Float(10.),
    },
    Definition {
        section: "Pedestrian_Navigation",
        label: "kNoRoadWanderCloseAngle",
        title: "No Road Wander Close Angle",
        initial: Value::Float(270.),
    },
    Definition {
        section: "Pedestrian_Navigation",
        label: "kNoRoadWanderFailDistance",
        title: "No Road Wander Fail Distance",
        initial: Value::Float(3.),
    },
    Definition {
        section: "Pedestrian_Pathogen",
        label: "kPathogenYTolerance",
        title: "Y Tolerance for pathing nav cells",
        initial: Value::Float(5.),
    },
    // Native Pathogen boundary cvars82fa36c8/3718/3768.
    Definition {
        section: "AI_Pathogen_AStar",
        label: "kBoundaryTest1",
        title: "Do AStar Boundary Test 1",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "AI_Pathogen_AStar",
        label: "kBoundaryTest2",
        title: "Do AStar Boundary Test 2",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "AI_Pathogen_AStar",
        label: "kBoundaryTest3",
        title: "Do AStar Boundary Test 3",
        initial: Value::Boolean(true),
    },
    // Native MeshNavigation switches82fa8df8/9238.
    Definition {
        section: "Pedestrian_Pathogen_AStar",
        label: "kAllowAStar",
        title: "0. Allow AStar",
        initial: Value::Boolean(true),
    },
    Definition {
        section: "Pedestrian_Pathogen_AStar",
        label: "kOffMeshAnchorArrivalRadius",
        title: "OM: OffMesh Anchor Arrival Radius",
        initial: Value::Float(1.5),
    },
    // Road targeting82fa6b58/6bb8/6a98/6c18.
    Definition {
        section: "Pedestrian_Navigation",
        label: "kValidRoadTargetDistance",
        title: "Road Wander ValidRoadTargetDistance",
        initial: Value::Float(50.),
    },
    Definition {
        section: "Pedestrian_Navigation",
        label: "kValidRoadTargetAngle",
        title: "Road Wander ValidRoadTargetAngle",
        initial: Value::Float(90.),
    },
    Definition {
        section: "Pedestrian_Navigation",
        label: "kRoadNavCrossWalkTime",
        title: "Road Wander Cross Walk Time",
        initial: Value::Float(3.),
    },
    Definition {
        section: "Pedestrian_Navigation",
        label: "kRoadNavAllowIncompletePaths",
        title: "Road Wander Allow Incomplete Paths",
        initial: Value::Boolean(false),
    },
    // NavRig initializers82fa3a98/3b58/3e18/9948/9b88/9be8/9798.
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kNavRigDistance",
        title: "Tuner: Nav Rig Starting Distance", initial: Value::Float(10.) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kNavRigPhysicsCacheDistance",
        title: "Physics: NavRig Cache Physics Distance", initial: Value::Float(0.5) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kDoPhysicsCaching",
        title: "Physics: Do NavRig Caching", initial: Value::Boolean(true) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kDoPathValidation",
        title: "Func: Do NavRig PathValidation", initial: Value::Boolean(true) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kMaxNavRigTopologyAngle",
        title: "Tuner: Max NavRig Topology Adjust Angle", initial: Value::Float(45.) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kMaxNavRigTopologyTestDistance",
        title: "Max Nav Rig TopologyDistance", initial: Value::Float(2.5) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kBatchNavRigLineTests",
        title: "Batch NavRig Line Tests", initial: Value::Boolean(false) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kNavRigPhysicsDistance",
        title: "Physics: Nav Rig Starting Physics Distance", initial: Value::Float(10.) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kNavRigPhysicsLineFatness",
        title: "Physics: Physics Line Fatness", initial: Value::Float(0.3) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kNavRigPhysicsLineYClearance",
        title: "Physics: Physics Line Y Clearance", initial: Value::Float(0.3) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kNavRigTestStartOffset",
        title: "Physics: Start Test Offset", initial: Value::Float(0.05) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kNavRigDownTestStartOffset",
        title: "Surface: Test Start Offset", initial: Value::Float(1.) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kNavRigDownTestStartHeight",
        title: "Surface: Test Start Height", initial: Value::Float(0.) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kNavRigDownTestHeight",
        title: "Surface: Test Height", initial: Value::Float(2.) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kPhysicsDoPhysicsTests",
        title: "Func: Do NavRig Physics Tests", initial: Value::Boolean(true) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kNavRigVolumeLineFatness",
        title: "Physics: Volume Line Fatness", initial: Value::Float(0.45) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kNavRigVolumeLineYClearance",
        title: "Physics: Volume Line Y Clearance", initial: Value::Float(0.25) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kPhysicsDoVolumeTests",
        title: "Func: Do NavRig Volume Tests", initial: Value::Boolean(true) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kPedNavRigVolumeVelocityScalar",
        title: "Ped Nav Rig Volume Velocity Scalar", initial: Value::Float(0.1) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kPedNavRigVolumeMaxVelShift",
        title: "Ped Nav Rig Volume Max Vel Shift", initial: Value::Float(1.) },
    // Spatial82fa6580/67f0/6850, jump82faa178/a408/a458. Native float
    // UI ranges: .001..1, 0..2.5, 0..20, 0..5 respectively; the shared
    // Float serializer does not impose UI-range clamps on loaded values.
    Definition { section: "Pedestrian_SpatialAdjust", label: "kYShiftTimeValue",
        title: "Y Shift Time", initial: Value::Float(0.12) },
    Definition { section: "Pedestrian_SpatialAdjust", label: "kSlidingClampAmount",
        title: "Collision sliding clamp amount", initial: Value::Float(0.1) },
    Definition { section: "Pedestrian_SpatialAdjust", label: "kPullbackSpeed",
        title: "Pullback Speed", initial: Value::Float(12.) },
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kJumpArrivalRadius",
        title: "Jump arrival radius", initial: Value::Float(0.1) },
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kJumpUseLedge",
        title: "TR: Use ledge for trajectory calculations", initial: Value::Boolean(false) },
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kJumpUseLanding",
        title: "TR: Use landing for trajectory calculations", initial: Value::Boolean(true) },
    // JumpNow constructors82fa9f38..a278: UI ranges0..5,.01..5,0..5,
    // .01..5,.01..1,.01..1,0..5,0..1. Runtime reads retain authored values.
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kNavRigJumpUpHeightMinHeight",
        title: "Min jump up height", initial: Value::Float(0.3) },
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kNavRigJumpUpHeightMaxHeight",
        title: "Max jump up height", initial: Value::Float(1.) },
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kNavRigJumpDownHeightMinHeight",
        title: "Min jump down height", initial: Value::Float(0.5) },
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kNavRigJumpDownHeightMaxHeight",
        title: "Max jump down height", initial: Value::Float(1.) },
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kNavRigJumpMinHorizontalRatio",
        title: "Horizontal Velocity ratio at min height", initial: Value::Float(0.8) },
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kNavRigJumpMaxHorizontalRatio",
        title: "Horizontal Velocity ratio at max height", initial: Value::Float(0.4) },
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kJumpLedgeClearance",
        title: "Jump ledge clearance height", initial: Value::Float(0.15) },
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kJumpLedgeFailureDistance",
        title: "Jump ledge fallback distance", initial: Value::Float(0.5) },
    // Physics static constructor82f6a580.
    Definition { section: "Physics_Pools_PedestrianPool", label: "kPedestrianPoolUpdateInParallel",
        title: "Update In Parallel", initial: Value::Boolean(false) },
    //1070..1075: initializers82f69fd8/6a120/6a260/6a300/6a3a0/6a3f0.
    Definition { section: "Physics_Debug_Pedestrian", label: "kDrawPedestrianGroundContacts",
        title: "Draw Pedestrian Ground Contacts", initial: Value::Boolean(false) },
    Definition { section: "Physics_Collision_Pipelines", label: "kRunPedestrianGroundCollisionPipeline",
        title: "Run Pedestrian Ground Collision Pipeline", initial: Value::Boolean(true) },
    Definition { section: "Physics_Collision_Pipelines", label: "kRunSkaterPedestrianCollisionPipeline",
        title: "Run Skater Pedestrian Collision Pipeline", initial: Value::Boolean(true) },
    Definition { section: "Physics_Collision_Pipelines", label: "kRunDmoPedestrianCollisionPipeline",
        title: "Run DMO Pedestrian Collision Pipeline", initial: Value::Boolean(true) },
    Definition { section: "Physics_Collision_Pipelines", label: "kRunPedestrianPedestrianCollisionPipeline",
        title: "Run Pedestrian Pedestrian Collision Pipeline", initial: Value::Boolean(true) },
    Definition { section: "Physics_Collision_Pipelines", label: "kRunPedestrianVehicleCollisionPipeline",
        title: "Run Pedestrian Vehicle Collision Pipeline", initial: Value::Boolean(true) },
    //1076..1088: source initializers82fa7178..82fa7868.
    Definition { section: "Pedestrian_Navigation_Modifier", label: "kAvoidOnNavMeshOnly",
        title: "NM: Only avoid onto NavMesh of same or lower score", initial: Value::Boolean(false) },
    Definition { section: "Pedestrian_Navigation_Modifier", label: "kNavMeshLargeAvoidDistance",
        title: "NM: Distance for navmesh check in large avoid", initial: Value::Float(1.4) },
    Definition { section: "Pedestrian_Navigation_Modifier", label: "kNavMeshSmallAvoidDistance",
        title: "NM: Distance for navmesh check in small avoid", initial: Value::Float(0.8) },
    Definition { section: "Pedestrian_Navigation_Modifier", label: "kAvoidNavMeshYTolerance",
        title: "NM: Nav Mesh Y tolerance for avoid checks", initial: Value::Float(0.75) },
    Definition { section: "Pedestrian_Navigation_Modifier_SkaterAvoidance", label: "kDebugSkaterAvoidanceNavModifier_AvoidNPCSkaters",
        title: "Avoid NPC Skaters", initial: Value::Boolean(true) },
    Definition { section: "Pedestrian_Navigation_Modifier_SkaterAvoidance", label: "kDebugSkaterAvoidanceNavModifier_PredictTime",
        title: "PredictTime", initial: Value::Float(0.75) },
    Definition { section: "Pedestrian_Navigation_Modifier_SkaterAvoidance", label: "kDebugSkaterAvoidanceNavModifier_OuterAvoidRadius",
        title: "OuterAvoidRadius", initial: Value::Float(3.0) },
    Definition { section: "Pedestrian_Navigation_Modifier_SkaterAvoidance", label: "kDebugSkaterAvoidanceNavModifier_InnerAvoidRadius",
        title: "InnerAvoidRadius", initial: Value::Float(1.0) },
    Definition { section: "Pedestrian_Navigation_Modifier_SkaterAvoidance", label: "kSkaterAvoidanceNavModifier_influenceradius",
        title: "SkaterAvoidance Influence Radius", initial: Value::Float(10.0) },
    Definition { section: "Pedestrian_Navigation_Modifier_SkaterAvoidance", label: "kSkaterAvoidanceNavModifier_FrontAvoidHalfAngle",
        title: "SkaterAvoidance Front Avoid Half-Angle", initial: Value::Float(0.5236) },
    Definition { section: "Pedestrian_Navigation_Modifier_SkaterAvoidance", label: "kDebugSkaterAvoidanceNavModifier_MinVelocityToStartAvoid",
        title: "Min Velocity To trigger ANY avoid", initial: Value::Float(1.0) },
    Definition { section: "Pedestrian_Navigation_Modifier_SkaterAvoidance", label: "kDebugSkaterAvoidanceNavModifier_MinVelocityForBigAvoid",
        title: "Min Velocity To trigger big avoid", initial: Value::Float(5.0) },
    Definition { section: "Pedestrian_Navigation_Modifier_SkaterAvoidance", label: "kDebugSkaterAvoidanceNavModifier_AllowHandleAvoidSkater",
        title: "Allow Handle Avoid Skater", initial: Value::Boolean(true) },
    //1089..1091: Init82e9ef00; original LoFi/NoFi display titles are swapped.
    Definition { section: "Pedestrian_Sys", label: "kHiFiPedsFrameCount",
        title: "FrameRate For HiFi Peds", initial: Value::Integer { value:30, minimum:0, maximum:60 } },
    Definition { section: "Pedestrian_Sys", label: "kLoFiPedsFrameCount",
        title: "FrameRate For NoFi Peds", initial: Value::Integer { value:10, minimum:0, maximum:60 } },
    Definition { section: "Pedestrian_Sys", label: "kNoFiPedsFrameCount",
        title: "FrameRate For LoFi Peds", initial: Value::Integer { value:3, minimum:0, maximum:60 } },
    //1092..1101: original navigation direction/jump static constructors.
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kJumpDownLedgeDifference",
        title: "Jump down ledge distance difference", initial: Value::Float(0.0) },
    Definition { section: "Pedestrian_Pathogen", label: "kNavRigBestRay_DoSmoothing",
        title: "Do Best NavRay Smoothing", initial: Value::Boolean(false) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kBestNavDirectionDistanceInital",
        title: "Tuner: Best NavDirection Distance Initial", initial: Value::Float(1.5) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kBestNavDirectionDistanceScalar",
        title: "Tuner: Best NavDirection Distance Scalar", initial: Value::Float(0.25) },
    Definition { section: "Pedestrian_Pathogen_NavRig", label: "kNavRigUsageVelocityThreshold",
        title: "Don't do nav rig adjustments below this speed", initial: Value::Float(1.4) },
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kNavRigJumpHeightHeight",
        title: "Height of jump height test", initial: Value::Float(2.0) },
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kNavRigJumpHeightLead",
        title: "Lead of jump test", initial: Value::Float(1.5) },
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kNavRigJumpOverride",
        title: "Override navrig best distance for jumps", initial: Value::Boolean(true) },
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kCheckForJumpNavMesh",
        title: "NM: Only jump onto nav mesh", initial: Value::Boolean(true) },
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kJumpNavMeshYTolerance",
        title: "NM: Y tolerance for nav mesh", initial: Value::Float(1.0) },
    //1102..1103: ctor suffix, static initializers82fa66e0/82faad90.
    Definition { section: "Pedestrian_SpatialAdjust", label: "kGroundPlaneTestDistanceUpDefault",
        title: "Ground Plane Test Distance Up Height - Default", initial: Value::Float(1.6) },
    Definition { section: "DMO_HandProps", label: "kAllowHandProps",
        title: "Allow spawn hand props", initial: Value::Boolean(true) },
    Definition { section: "Pedestrian_Navigation", label: "kDoRAINPedIteration",
        title: "Do RAIN Pedestrian Wander", initial: Value::Boolean(true) },
    Definition { section: "Pedestrian_Chase", label: "kAllowIncompletePathInChase",
        title: "Allow incomplete AStar path in chase", initial: Value::Boolean(true) },
    Definition { section: "Pedestrian_Chase", label: "kChaseNavFailDistance",
        title: "Give up if pathing fails within this distance", initial: Value::Float(10.0) },
    Definition { section: "Pedestrian_Collision", label: "kNumberOfRecoveryAttempts",
        title: "Number of attempts before forcing collision recovery", initial: Value::Integer { value:5, minimum:0, maximum:50 } },
    Definition { section: "Pedestrian_Collision", label: "kForceRecoveryIfCollidingWithSkater",
        title: "kForceRecoveryIfCollidingWithSkater", initial: Value::Boolean(true) },
    //82fa6188/82fa61e8, GUI ranges .01..100.
    Definition { section: "Pedestrian_LivingWorld_Collision_Pedestrian", label: "kSkaterCollisionsIgnoranceRadiusFactor",
        title: "Collisions Ignorance Radius Factor: Skaters", initial: Value::Float(16.0) },
    Definition { section: "Pedestrian_LivingWorld_Collision_Pedestrian", label: "kPedCollisionsIgnoranceRadiusFactor",
        title: "Collisions Ignorance Radius Factor: Other Peds", initial: Value::Float(2.0) },
    //1111/1112: native82fa8a28/82fab9a0.
    Definition { section: "Pedestrian_Animation_Debug", label: "kUseTestMotionGraph",
        title: "Use Test MotionGraph on All Pedestrians", initial: Value::Boolean(false) },
    Definition { section: "Animation_BlendCullThresh", label: "kPedBlendCullThresh",
        title: "Ped BlendCullThresh", initial: Value::Float(0.1) },
    //1113..1117:82fa5050/50a8/5100,82fa4ce0,82f5d798.
    Definition { section: "AI_Prop_Sys", label: "kHiFiPropsFrameCount",
        title: "FrameRate For HiFi Prop", initial: Value::Integer { value:30,minimum:0,maximum:60 } },
    Definition { section: "AI_Prop_Sys", label: "kLoFiPropsFrameCount",
        title: "FrameRate For NoFi Prop", initial: Value::Integer { value:10,minimum:0,maximum:60 } },
    Definition { section: "AI_Prop_Sys", label: "kNoFiPropsFrameCount",
        title: "FrameRate For LoFi Prop", initial: Value::Integer { value:3,minimum:0,maximum:60 } },
    Definition { section: "AI_Prop", label: "kEnablePatrolPointSpawn",
        title: "Enable Patrol Point Waypoint Registry", initial: Value::Boolean(true) },
    Definition { section: "AI_Spawner", label: "kMAXPROPS",
        title: "Maximum Props Allowed", initial: Value::Integer { value:255,minimum:0,maximum:255 } },
    //1118..1120:82fa6410/6470/6520.
    Definition { section: "Pedestrian_Spawner", label: "kPedestrianSpawnDensity",
        title: "Pedestrian Spawn Empty Radius", initial: Value::Float(8.0) },
    Definition { section: "Pedestrian_Spawner", label: "kSpawnPedDMOCheckRadius",
        title: "Pedestrian Spawn DMO check radius", initial: Value::Float(12.5) },
    Definition { section: "Pedestrian_Spawner", label: "kSpawnPedDMOPadding",
        title: "Mimimum distance from a DMO to spawn", initial: Value::Float(0.5) },
    //82faa4a8/4f8,82faabc0/ac20/ac80; float ranges .01..5,0..2.5,0..2.5,0..2.
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kJumpShiftCollision",
        title: "CV: Shift collision volume up on jump", initial: Value::Boolean(true) },
    Definition { section: "Pedestrian_Pathogen_NavRig_Jump", label: "kJumpShiftAmount",
        title: "CV: Collision volume shift amount", initial: Value::Float(0.25) },
    Definition { section: "Pedestrian_Plugin", label: "kPluginCollisionBodyShiftAmount",
        title: "Amount to shift world collision body on plugin interaction", initial: Value::Float(0.5) },
    Definition { section: "Pedestrian_Plugin", label: "kPluginLockTolerance",
        title: "Distance from waypoint at which collision volume becomes locked", initial: Value::Float(0.1) },
    Definition { section: "Pedestrian_Plugin", label: "kPluginUnlockTolerance",
        title: "Distance from a locked collision volume at which volume becomes unlocked", initial: Value::Float(0.15) },
    //82f5df20.
    Definition { section: "AI_Spawner", label: "kIgnoreMinDistForPropSpawn",
        title: "Override minimim distance for prop spawn", initial: Value::Boolean(false) },
    Definition { section: "AI_Prop", label: "kEnableSitPointSpawn", title: "Enable Sit Point Waypoint Registry", initial: Value::Boolean(true) },
    Definition { section: "AI_Prop", label: "kEnableVendingMachinePointSpawn", title: "Enable Vending Machine Point Waypoint Registry", initial: Value::Boolean(true) },
    Definition { section: "AI_Prop", label: "kEnableATMPointSpawn", title: "Enable ATM Point Waypoint Registry", initial: Value::Boolean(true) },
    Definition { section: "AI_Prop", label: "kEnableWaterfountainPointSpawn", title: "Enable Waterfountain Point Waypoint Registry", initial: Value::Boolean(true) },
    Definition { section: "AI_Prop", label: "kEnableNewspaperPointSpawn", title: "Enable Newspaper Box Point Waypoint Registry", initial: Value::Boolean(true) },
    Definition { section: "AI_Prop", label: "kEnableTrashBinPointSpawn", title: "Enable Garbage Bin Point Waypoint Registry", initial: Value::Boolean(true) },
    Definition { section: "AI_Prop", label: "kEnableLookAtPointSpawn", title: "Enable LookAt Point Waypoint Registry", initial: Value::Boolean(true) },
    Definition { section: "AI_Prop", label: "kEnableSpectatorPointSpawn", title: "Enable Spectator Point Waypoint Registry", initial: Value::Boolean(true) },
    Definition { section: "AI_Prop", label: "kEnableGatherPointSpawn", title: "Enable GatherPoint Waypoint Registry", initial: Value::Boolean(true) },
    Definition { section: "AI_Prop", label: "kEnableConversationSpawn", title: "Enable Conversation Waypoint Registry", initial: Value::Boolean(true) },
    //82f63ee8,82fa87a8,82f8cc60. Native float ranges .01...1 and0..3600.
    Definition { section: "Animation", label: "kBranchWindow", title: "Branch Frame Window", initial: Value::Float(0.05) },
    Definition { section: "Pedestrian_Animation_Debug", label: "kAssertOnMissingAnim", title: "Assert On Missing Anim", initial: Value::Boolean(false) },
    Definition { section: "DMO_HandProps", label: "kHandPropCullTimer", title: "Hand Props Culling Time", initial: Value::Float(30.) },
    //82fa9028/9080/90d8/9190: each native integer range3..500.
    Definition { section: "Pedestrian_Pathogen_AStar", label: "kMaxAStarNoFiPathDepth", title: "3. Max AStar NoFi PathDepth per test", initial: Value::Integer { value:15, minimum:3, maximum:500 } },
    Definition { section: "Pedestrian_Pathogen_AStar", label: "kMaxAStarLoFiPathDepth", title: "4. Max AStar LoFi PathDepth per test", initial: Value::Integer { value:30, minimum:3, maximum:500 } },
    Definition { section: "Pedestrian_Pathogen_AStar", label: "kMaxAStarHiFiPathDepth", title: "5. Max AStar HiFi PathDepth per test", initial: Value::Integer { value:40, minimum:3, maximum:500 } },
    Definition { section: "Pedestrian_Pathogen_AStar", label: "kMaxAStarPathDepthForSecurityGuard", title: "  Max AStar PathDepth for Security Guard", initial: Value::Integer { value:24, minimum:3, maximum:500 } },
    //82faaed0 range0..1.5.
    Definition { section: "DMO_HandProps", label: "kHandPropThrowTimeOut", title: "Time to wait to release a hand prop when dropping due to collision or avoid", initial: Value::Float(0.2) },
    //82f5e8b0.
    Definition { section: "Pedestrian_Chase", label: "kDisableTakedowns", title: "Disable Chaser Takedowns", initial: Value::Boolean(false) },
    //82fa89d8/82fabc48.
    Definition { section: "Pedestrian_Animation_Debug", label: "kEnablePedestrianMotionGraphPools", title: "Enable Pedestrian Motion Graph Pools", initial: Value::Boolean(true) },
    Definition { section: "Animation_Debug", label: "kGenerateFootTagsForAllPeds", title: "Generate Foot Tags For All Peds", initial: Value::Boolean(false) },
    //82fa5fd8 /82f5f300 (float range0..10).
    Definition { section: "Pedestrian_EntityKnowledge", label: "kEntityKnowledgeSuppressed", title: "01.Suppress recording of EntityKnowledge", initial: Value::Boolean(false) },
    Definition { section: "Pedestrian_Mood", label: "kWarnInterestTimer", title: "Warn interest timer", initial: Value::Float(3.5) },
    //82fa6028 range0..10 /82faad40.
    Definition { section: "Pedestrian_LivingWorld", label: "kMinRotateToTargetAngle", title: "Min Rotate To Target Angle", initial: Value::Float(1.) },
    Definition { section: "Pedestrian_Plugin", label: "kPluginShiftWithDMO", title: "DMO: Shift with DMO if it moves less than shift tolerance", initial: Value::Boolean(true) },
    //82faae30.
    Definition { section: "DMO_HandProps", label: "kDoHandPropPositionPrediction", title: "Use hand velocity to better predict hand prop position", initial: Value::Boolean(true) },
    //82fa5820 range0..50.
    Definition { section: "Pedestrian", label: "kDefaultNavMeshCheckYTolerance", title: "Default NavMesh cell Y Tolerance", initial: Value::Float(1.) },
    //82fa8538/8598 and82fa7da8..8168, source General/RulesOfRoad tunings.
    Definition { section: "Pedestrian_Navigation_Modifier_RulesOfRoad", label: "kRORSpring_PredictTime", title: "PredictTime", initial: Value::Float(6.) },
    Definition { section: "Pedestrian_Navigation_Modifier_RulesOfRoad", label: "kRORSpring_KDRange", title: "KD Near Range", initial: Value::Float(4.) },
    Definition { section: "Pedestrian_Navigation_Modifier_RulesOfRoad", label: "kDebugGeneralNavModifier_AvoidSpeed", title: "Avoid Speed", initial: Value::Float(5.) },
    Definition { section: "Pedestrian_Navigation_Modifier_RulesOfRoad", label: "kDebugGeneralNavModifier_RotRSpeed", title: "RotR Speed", initial: Value::Float(0.5) },
    Definition { section: "Pedestrian_Navigation_Modifier_RulesOfRoad", label: "kDebugGeneralNavModifier_OncomingAngle", title: "Oncoming Angle", initial: Value::Float(3.) },
    Definition { section: "Pedestrian_Navigation_Modifier_RulesOfRoad", label: "kDebugGeneralNavModifier_OncomingAvoidAngle", title: "Oncoming Avoid Angle", initial: Value::Float(10.) },
    Definition { section: "Pedestrian_Navigation_Modifier_RulesOfRoad", label: "kDebugGeneralNavModifier_OncomingAvoidDistance", title: "Oncoming Avoid Distance", initial: Value::Float(3.) },
    Definition { section: "Pedestrian_Navigation_Modifier_RulesOfRoad", label: "kDebugGeneralNavModifier_OuterAvoidRadius", title: "OuterAvoidRadius", initial: Value::Float(3.) },
    Definition { section: "Pedestrian_Navigation_Modifier_RulesOfRoad", label: "kDebugGeneralNavModifier_InnerAvoidRadius", title: "InnerAvoidRadius", initial: Value::Float(1.) },
    Definition { section: "Pedestrian_Navigation_Modifier_RulesOfRoad", label: "kDebugGeneralNavModifier_HeightTolerance", title: "Height Tolerance", initial: Value::Float(3.) },
    Definition { section: "Pedestrian_Navigation_Modifier_RulesOfRoad", label: "kDebugGeneralNavModifier_IntersectTolerance", title: "Intersect Tolerance", initial: Value::Float(1.5) },
    Definition { section: "Pedestrian_Navigation_Modifier_RulesOfRoad", label: "kDebugGeneralNavModifier_OncomingWidth", title: "Oncoming Width", initial: Value::Float(1.) },
    Definition { section: "Pedestrian_Navigation_Modifier_RulesOfRoad", label: "kDebugGeneralNavModifier_FrontAvoidHalfAngle", title: "Front Avoid Half-Angle", initial: Value::Float(0.52359998) },
    //82fa78b8/7978/79d8/7a38/7ae8: ranges.1..30,0..15,0..5,0..5.
    Definition { section: "Pedestrian_Navigation_Modifier_DMOAvoidance", label: "kDMOAvoidanceNavModifier_influenceradius", title: "DMOAvoidance Influence Radius", initial: Value::Float(8.) },
    Definition { section: "Pedestrian_Navigation_Modifier_DMOAvoidance", label: "kDebugDMOAvoidanceNavModifier_MinVelocityToStartAvoid", title: "Min Velocity To trigger ANY avoid", initial: Value::Float(0.75) },
    Definition { section: "Pedestrian_Navigation_Modifier_DMOAvoidance", label: "kDMOAvoidanceCornerAvoidDistance", title: "Corner Avoid Distance", initial: Value::Float(0.75) },
    Definition { section: "Pedestrian_Navigation_Modifier_DMOAvoidance", label: "kDMOAvoidanceMaxAvoidJumpDistance", title: "Max Avoid Jump Distance", initial: Value::Float(0.5) },
    Definition { section: "Pedestrian_Navigation_Modifier_DMOAvoidance", label: "kDebugDMOAvoidanceNavModifier_AllowHandleAvoidDMO", title: "Allow Handle Avoid DMO", initial: Value::Boolean(true) },
    //82fa74c8/7b88/7be8/7c48/7ca8. Ranges .1..360,.1..10,.1..10,0..50,0..50.
    Definition { section: "Pedestrian_Navigation_Modifier_BigBlackAvoidance", label: "kDebugBigBlackAvoidanceNavModifier_maxTurnAngle", title: "Max Turn Angle: Degrees", initial: Value::Float(8.) },
    Definition { section: "Pedestrian_Navigation_Modifier_Vehicle", label: "kVehicleJumpBackDistance", title: "JumpBack Distance", initial: Value::Float(0.4) },
    Definition { section: "Pedestrian_Navigation_Modifier_Vehicle", label: "kVehicleVelocityJumpBackMinSpeed", title: "JumpBack Vehicle Min Velocity", initial: Value::Float(0.3) },
    Definition { section: "Pedestrian_Navigation_Modifier_Vehicle", label: "kVehicleFleeDistance", title: "Flee Distance", initial: Value::Float(10.) },
    Definition { section: "Pedestrian_Navigation_Modifier_Vehicle", label: "kVehicleFleeSpeed", title: "Flee Speed", initial: Value::Float(3.) },
    //1177..1182:82faaf30/af90/b050/b0b0/b110/b230. Ranges0..3,0..3,0..30,0..30,.01..1.5708,.01..1.5708.
    Definition { section:"DMO_HandProps",label:"kHandPropDisposeThrowTimeOut",title:"Time from end of animation to release a hand prop when performing a light throw",initial:Value::Float(0.91667002) },
    Definition { section:"DMO_HandProps",label:"kHandPropAttackThrowTimeOut",title:"AT: Time from end of animation to release a hand prop when performing an attack throw",initial:Value::Float(0.83329999) },
    Definition { section:"DMO_HandProps",label:"kHandPropDisposeForce",title:"Force to apply when throwing a hand prop into a trashbin",initial:Value::Float(5.) },
    Definition { section:"DMO_HandProps",label:"kHandPropAttackThrowForce",title:"AT: Force to apply when throwing a hand prop at a skater",initial:Value::Float(10.) },
    Definition { section:"DMO_HandProps",label:"kHandPropAttackForwardAngle",title:"AT: Half-angle to do a forward throw animation",initial:Value::Float(0.52359998) },
    Definition { section:"DMO_HandProps",label:"kHandPropThrowAngleTolerance",title:"Angle tolerance for prop dispose",initial:Value::Float(0.1) },
    //82f53b68/82f53b18.
    Definition { section:"FE",label:"kbAllowStatsAnytime",title:"Allow Stats anytime",initial:Value::Boolean(false) },
    Definition { section:"FE",label:"kbAllowPersonalBestsAnytime",title:"Allow Personal Bests anytime",initial:Value::Boolean(false) },
    //1185..1188:82faaff0/b290/b2f0/b350; ranges0..30 then0..3.
    Definition { section:"DMO_HandProps",label:"kHandPropThrowForce",title:"Force to apply when throwing a hand prop due to collision/avoid",initial:Value::Float(1.) },
    Definition { section:"DMO_HandProps",label:"kHandPropDisownDistanceX",title:"Pedestrian to hand prop X distance for collision re-enable",initial:Value::Float(0.5) },
    Definition { section:"DMO_HandProps",label:"kHandPropDisownDistanceY",title:"Pedestrian to hand prop Y distance for collision re-enable",initial:Value::Float(2.) },
    Definition { section:"DMO_HandProps",label:"kHandPropDisownDistanceZ",title:"Pedestrian to hand prop Z distance for collision re-enable",initial:Value::Float(0.5) },

    //1189:82fa59c8;1190:82fa8c08.
    Definition { section:"Pedestrian_Mood",label:"kMoodsSuppressed",title:"03.Suppress recording of moods",initial:Value::Boolean(false) },
    Definition { section:"Pedestrian_Animation_Debug_HeadTracking",label:"kEnableHeadTracking",title:"Enable Head Tracking",initial:Value::Boolean(true) },
    //1191..1198: ground/default probe constructors82fa6680/6740/9d08/9d68/
    //9dc8/5ce8/3cf8/3d58. All section+label pairs are unique.
    Definition { section:"Pedestrian_SpatialAdjust",label:"kGroundPlaneTestDistanceDown",title:"Ground Plane Test Distance Down",initial:Value::Float(2.) },
    Definition { section:"Pedestrian_SpatialAdjust",label:"kGroundPlaneTestDistanceUpKnockDown",title:"Ground Plane Test Distance Up Height - KnockDown",initial:Value::Float(0.3) },
    Definition { section:"Pedestrian_Pathogen_NavRig_Jump",label:"kNavRigJumpForwardFatness",title:"Fatness of jump forward test",initial:Value::Float(0.5) },
    Definition { section:"Pedestrian_Pathogen_NavRig_Jump",label:"kNavRigJumpForwardHeight",title:"Height of jump forward test",initial:Value::Float(1.8) },
    Definition { section:"Pedestrian_Pathogen_NavRig_Jump",label:"kNavRigJumpHeightFatness",title:"Fatness of jump height test",initial:Value::Float(0.) },
    Definition { section:"Pedestrian_Mood",label:"kTazeLineStartOffset",title:"Offset for taze line test",initial:Value::Float(0.5) },
    Definition { section:"Pedestrian_Pathogen_NavRig_Jump",label:"kNavRigJumpForwardDistance",title:"Nav Rig Starting Jump Forward Distance",initial:Value::Float(20.) },
    Definition { section:"Pedestrian_Pathogen_NavRig_Jump",label:"kNavRigJumpHeightDistance",title:"Nav Rig Starting Jump Height Distance",initial:Value::Float(20.) },
    Definition {section:"Pedestrian_Mood",label:"kAllMoodBehaviourSupressed",title:"10.Suppress all mood based behaviour",initial:Value::Boolean(false)},
    Definition {section:"Pedestrian_Mood",label:"kShowMoodRandomRolls",title:"04.Show probability-test events",initial:Value::Boolean(false)},
    //1201: initializer82fa8648, native range0..0.99.
    Definition {section:"Pedestrian_SecurityGuard",label:"kMagicSecurityGuardMeshCostThreshold",title:"Security Guard NavMesh Cost Threshold",initial:Value::Float(0.51)},
    //1202/1203: original82f5e570/82faab60, ranges0..25 and0..120.
    Definition {section:"Pedestrian_Plugin",label:"kPluginTransferYTolerance",title:"Y Tolerance for plugin transfer",initial:Value::Float(1.8)},
    Definition {section:"Pedestrian_Plugin",label:"kPluginRetryTimer",title:"Time between plugin-use retries",initial:Value::Float(30.)},
    Definition {section:"Pedestrian_Mood",label:"kHOMSlamThreshold",title:"Minimum HOM score to trigger slam event",initial:Value::Integer{value:750,minimum:0,maximum:10000}},
    Definition {section:"Pedestrian_Chase",label:"kJackedTimer",title:"Jacked Timer",initial:Value::Float(4.5)},
    Definition {section:"FE",label:"kbShowJackedOverlay",title:"Show Jacked Overlay",initial:Value::Boolean(true)},
    //1207: initializer82f5f1e0, native range0..3.1415901.
    Definition {section:"AI_Prop",label:"kInFrontOfWaypointAngle",title:"Half-angle to consider 'in front of' a waypoint (radians)",initial:Value::Float(0.52359879)},
    //1208 initializer82f5ea50, native0..30.
    Definition{section:"Pedestrian_Plugin",label:"kPluginUnlockTimer",title:"Fallback time for collision volume unlock",initial:Value::Float(2.)},
    //1209 initializer82f5eb00, native range0..2.
    Definition{section:"Pedestrian_Spectate",label:"kSpectateMaxDelay",title:"Max Spectate Delay (sec)",initial:Value::Float(0.5)},
    //1210 initializer82faace0, native range0..2.
    Definition{section:"Pedestrian_Plugin",label:"kPluginDMOShiftTolerance",title:"DMO: Distance a DMO can move in one frame without interrupting a plugin",initial:Value::Float(0.005)},
    //1211 initializer82f5e950, native range0..120.
    Definition{section:"Pedestrian_Plugin",label:"kMinPluginTimer",title:"Min Time between like states",initial:Value::Float(30.)},
    //1212 initializer82f5d2d0.
    Definition{section:"AI_LivingWorld",label:"kEnableActorTracker",title:"Enable Actor Tracker",initial:Value::Boolean(true)},
    //1213/1214 initializers82f6ed60/82f6edb0.
    Definition{section:"Animation",label:"kEnablePedestrianFaceAnim",title:"Enable Pedestrian Face Anim",initial:Value::Boolean(true)},
    Definition{section:"Animation",label:"kEnablePedestrianHandAnim",title:"Enable Pedestrian Hand Anim",initial:Value::Boolean(true)},
    //1215 initializer82f5f240, native range0..3.1415901.
    Definition{section:"Pedestrian_Mood",label:"kNavigatingWithHeadTrackingAngle",title:"Half-angle to continue walking while reacting (radians)",initial:Value::Float(0.95993102)},
    //1216 initializer82f5f2a0, native range0..10.
    Definition{section:"Pedestrian_Mood",label:"kNavigatingWithHeadTrackingPredict",title:"Predict time for body turns while reacting",initial:Value::Float(1.)},
    //1217..1219 initializers82fab170/82fab1d0/82f5f360.
    Definition{section:"DMO_HandProps",label:"kHandPropAttackPerturbRadius",title:"AT: Radius to perturb attack throw position",initial:Value::Float(0.25)},
    Definition{section:"DMO_HandProps",label:"kHandPropAttackYOffset",title:"AT: Y offset from centre of mass",initial:Value::Float(1.)},
    Definition{section:"Pedestrian_Mood",label:"kGreetInterestTimer",title:"Greet interest timer",initial:Value::Float(3.5)},
    //1220 initializer82f5f410, native range0..25.
    Definition{section:"Pedestrian_Chase",label:"kChaseInterceptYTolerance",title:"Y Tolerance for intercept calculations",initial:Value::Float(1.)},
    //1221/1222 initializers82f6eff0/82f6f040.
    Definition{section:"Animation",label:"kEnableMarqueeFaceAnim",title:"Enable Marquee Pedestrian Face Anim",initial:Value::Boolean(false)},
    Definition{section:"Animation",label:"kEnableMarqueeHandAnim",title:"Enable Marquee Pedestrian Hand Anim",initial:Value::Boolean(true)},
    //1223 initializer82fa5f78, native range0..5.
    Definition{section:"Pedestrian_Chase",label:"kTakedownYTolerance",title:"Takedown Y Tolerance",initial:Value::Float(1.25)},
    //1224..1226 initializers82f8c058/0b0/108; original Lo/No titles reversed.
    Definition{section:"AI_DynamicObject_Sys",label:"kHiFiDynamicObjectsFrameCount",title:"FrameRate For HiFi DynamicObject",initial:Value::Integer { value: 30, minimum: 0, maximum: 60 }},
    Definition{section:"AI_DynamicObject_Sys",label:"kLoFiDynamicObjectsFrameCount",title:"FrameRate For NoFi DynamicObject",initial:Value::Integer { value: 10, minimum: 0, maximum: 60 }},
    Definition{section:"AI_DynamicObject_Sys",label:"kNoFiDynamicObjectsFrameCount",title:"FrameRate For LoFi DynamicObject",initial:Value::Integer { value: 3, minimum: 0, maximum: 60 }},
    //1227 initializer82f8cdc0, native range0..5.
    Definition{section:"DMO_ContainedProps",label:"kContainedWaypointYTolerance",title:"Y Tolerance for nav mesh tests",initial:Value::Float(0.25)},

    //1228..1230 DMO UpdateTransforms reset volumes; initializers82f8c810/870/8d0 range.01..2.
    Definition{section:"DMO_Resetting",label:"kSkaterTestCapsuleWidth",title:"Skater Test Capsule Width",initial:Value::Float(0.35)},
    Definition{section:"DMO_Resetting",label:"kSkateboardTestCapsuleWidth",title:"Skateboard Capsule Width",initial:Value::Float(0.15)},
    Definition{section:"DMO_Resetting",label:"kSkateboardTestCapsuleHeight",title:"Skateboard Capsule Height",initial:Value::Float(0.63)},

    //1231/1232: original constructors82f54710/82f59ba0.
    Definition{section:"Character_CAS",label:"kCorrectRostral",title:"Correct Rostral model during assembly",initial:Value::Boolean(true)},
    Definition{section:"FE_CAC",label:"kShowPreviewDelay",title:"Preview Part Delay",initial:Value::Integer{value:500,minimum:0,maximum:1000}},
    //1233..1243: ROPA original constructors82f71880..82f71bc8 and82f71690.
    Definition{section:"ROPA_Core",label:"cv_MaxDTRatio",title:"Max DT Ratio",initial:Value::Float(3.0)},
    Definition{section:"ROPA_Core",label:"cv_MinDTIntegration",title:"Min DT Integration (100th of ms)",initial:Value::Float(0.01)},
    Definition{section:"ROPA_Core",label:"cv_FixedDT",title:"Fixed DT",initial:Value::Boolean(true)},
    Definition{section:"ROPA_Tuning",label:"cv_Gravity",title:"Gravity",initial:Value::Float(2.0)},
    Definition{section:"ROPA_Tuning",label:"cv_VerticalSpeedDampening",title:"Vertical Speed Dampening",initial:Value::Float(0.3075)},
    Definition{section:"ROPA_Tuning",label:"cv_Compression",title:"Compression tolerance",initial:Value::Float(0.057)},
    Definition{section:"ROPA_Tuning",label:"cv_VertDistanceScale",title:"Vertical Distance to Driver scale",initial:Value::Float(0.375)},
    Definition{section:"ROPA_Tuning",label:"cv_NormDistanceScale",title:"Normal Distance to Driver scale",initial:Value::Float(1.5)},
    Definition{section:"ROPA_Tuning",label:"cv_NbIterations",title:"Nb Iterations",initial:Value::Integer{value:1,minimum:0,maximum:10}},
    Definition{section:"ROPA_Debug",label:"cv_UseConstraintLengthOnNormals",title:"Use Constraint Length On Normals",initial:Value::Boolean(true)},
    Definition{section:"ROPA_Tuning",label:"cv_NbWipeOutFrames",title:"Nb WipeOut frames",initial:Value::Integer{value:8,minimum:1,maximum:20}},
    //1244: FE fade initializer82f58320, native range .005..1.
    Definition{section:"FE",label:"kFadeSpeed",title:"Fade Speed",initial:Value::Float(0.028)},
    //1245: CameraMan.Update suffix; initializer82fa27a8.
    Definition{section:"Camera",label:"kCameraNotReadyDuration",title:"Camera not ready duration",initial:Value::Integer{value:30,minimum:0,maximum:300}},
    //1246: SkeletonBody::CreateHatVolume; initializer82f7d2e8/table82fd4600, null callback.
    Definition{section:"Physics_Skeleton_Hat",label:"kEnableHatCollision",title:"Enable Hat Collision",initial:Value::Boolean(true)},
    //1247/1248: cTeleportManager::TeleportRequiresLoadScreen; initializers82f73848/82f738a8, native range1..400.
    Definition{section:"Stream",label:"kMinSimRadiusForNoLoadScreenOnTelelport",title:"Min Sim Radius for No Load Screen on Teleport",initial:Value::Float(20.)},
    Definition{section:"Stream",label:"kMinPresRadiusForNoLoadScreenOnTelelport",title:"Min Pres Radius for No Load Screen on Teleport",initial:Value::Float(30.)},
    //1250/1251: original initializers82f6dda8/82f5ad78.
    // Entries below retain their stable API indices.
    //1249: DynamicObject::IsUpright, initializer 0x82f8c7b0.
    Definition{section:"DMO_Uprighting",label:"kDynamicObjectUprightAngle",title:"Upright Threshold Angle",initial:Value::Float(45.)},
    Definition{section:"Debug_Scoring_Wipeouts",label:"kIsHoMSlowMotionEnabled",title:"Is HoM SlowMotion Enabled",initial:Value::Boolean(true)},
    Definition{section:"FE",label:"kHOMAnimationTime",title:"HOM - Time to animate skeleton for",initial:Value::Float(2.)},
    //1252: cChallengeObjectiveGroup.Update/OnWipeoutOver, initializer82f8d5e0.
    Definition{section:"Debug_Scoring_Wipeouts",label:"kIsHoMChallengeEnabled",title:"Is HoM Challenge Enabled",initial:Value::Boolean(true)},
    //1253: initializer82f64c68, default0/range0..90 (f1/f2/f3).
    Definition{section:"Pedestrian_Chase_Achievements",label:"kRunningManAchievementTime",title:"Amount of offboard chase time required for Running Man",initial:Value::Float(0.)},
    //1254..1257: AI respawn initializers82f4cc28/cd38/cd88/de58.
    Definition{section:"AI_Path_Following_Respawn",label:"kbAlwaysRespawnOnPath",title:"Always Respawn Skaters On Paths (not locators)",initial:Value::Boolean(false)},
    Definition{section:"AI_Path_Following_Respawn",label:"kRespawnBehindInRaceChallenges",title:"kRespawnBehindInRaceChallenges",initial:Value::Boolean(true)},
    Definition{section:"AI_Path_Following_Respawn",label:"kRespawnBehindAlways",title:"kRespawnBehindAlways",initial:Value::Boolean(false)},
    Definition{section:"AI_Path_Following_Stuck",label:"kMaxNodeAdvanceForBlockingObject",title:"kMaxNodeAdvanceForBlockingObject",initial:Value::Integer{value:8,minimum:0,maximum:256}},
    //1258: initializer82f6a170/table82fd2210, reference8305256c; null callback.
    Definition {
        section: "Physics_Collision_Pipelines",
        label: "kRunSkaterSkaterCollisionPipeline",
        title: "Run Skater Skater Collision Pipeline",
        initial: Value::Boolean(true),
    },
    //1259..1269: ReplayEditorBackEnd/FrontEndState_Replay2 live references.
    // Initializers82f53ab8 and82f59ef0..82f5a250; distinct section/label pairs.
    Definition{section:"ReplayEditorTweaks",label:"kMarkerDetectionThreshold",title:"Marker Detection Threshold",initial:Value::Float(0.0089)},
    Definition{section:"ReplayEditorTweaks",label:"kfScrubMaxSpeedScalar",title:"Scrub Multiplier",initial:Value::Float(1.)},
    Definition{section:"ReplayEditorTweaks",label:"kfScrubSmoothing",title:"Scrub Smoothing",initial:Value::Float(0.25)},
    Definition{section:"ReplayEditorTweaks",label:"kfZoomDelta",title:"Zoom Delta",initial:Value::Float(0.5)},
    Definition{section:"ReplayEditorTweaks",label:"kfFreeCamLS",title:"Free Cam LS",initial:Value::Float(0.8)},
    Definition{section:"ReplayEditorTweaks",label:"kfFreeCamRS",title:"Free Cam LS",initial:Value::Float(0.2)},
    Definition{section:"ReplayEditorTweaks",label:"kfFreeCamHeightDelta",title:"Free Cam Heightdelta",initial:Value::Float(0.015)},
    Definition{section:"ReplayEditorTweaks",label:"kfOrbitCamLS",title:"Orbit Cams LS",initial:Value::Float(0.2)},
    Definition{section:"ReplayEditorTweaks",label:"kfOrbitCamRS",title:"Orbit Cams RS",initial:Value::Float(3.)},
    Definition{section:"ReplayEditorTweaks",label:"kfOrbitCamHeightDelta",title:"Orbit Cam Heightdelta",initial:Value::Float(0.01)},
    Definition{section:"ReplayEditorTweaks",label:"kfOrbitMultiplier",title:"Orbit Multiplier",initial:Value::Float(2.5)},
    // Debug initializer82f688c8.
    Definition{section:"Debug",label:"kbIsReplaySteppingEnabled",title:"A - Enable stepping frame by frame",initial:Value::Boolean(false)},
    // ICELITE camera collision initializers82f5c218/82f5c2d8.
    Definition{section:"NIS_Camera_LITE",label:"kICELITEMaxCameraDistance",title:"Max. camera/anchor distance",initial:Value::Float(40.)},
    Definition{section:"NIS_Camera_LITE",label:"kICELITEEnableCollision",title:"Enable replay camera collision",initial:Value::Boolean(true)},
    //1273..1281: native BigBlack spawn82f5e078..e310 and avoidance82fa7478.
    Definition{section:"AI_Spawner",label:"kBigBlackSpawnZOffset",title:"Big Black spawn Z offset",initial:Value::Float(1.5)},
    Definition{section:"AI_Spawner",label:"kBigBlackSpawnXOffset",title:"Big Black spawn X offset",initial:Value::Float(1.5)},
    Definition{section:"AI_Spawner",label:"kBigBlackSpawnMinBreadCrumbDistance",title:"Big Black spawn Min Bread Crumb Distance From Skater",initial:Value::Float(2.5)},
    Definition{section:"AI_Spawner",label:"kBigBlackSpawnMaxBreadCrumbDistance",title:"Big Black spawn Max Bread Crumb Distance From Skater",initial:Value::Float(30.)},
    Definition{section:"AI_Spawner",label:"kBigBlackSpawnLineTestDirections",title:"Big Black spawn Line Test Directions",initial:Value::Integer{value:8,minimum:0,maximum:32}},
    Definition{section:"AI_Spawner",label:"kBigBlackSpawnLineTestHalfHeight",title:"Big Black spawn Vertical Line Test Half Height",initial:Value::Float(1.5)},
    Definition{section:"AI_Spawner",label:"kBigBlackSpawnHorizontalLineTestHeight",title:"Big Black spawn Horizontal Line Test Height",initial:Value::Float(1.5)},
    Definition{section:"AI_Spawner",label:"kBigBlackSpawnHorizontalLineTestBuffer",title:"Big Black spawn Horizontal Line Test Buffer",initial:Value::Float(0.5)},
    Definition{section:"Pedestrian_Navigation_Modifier_BigBlackAvoidance",label:"kDebugBigBlackAvoidanceNavModifier_AllowHandleAvoidBigBlack",title:"Allow Handle Avoid BigBlack",initial:Value::Boolean(true)},
    //1282: native spectator initializer82f89d20, integer range1..50.
    Definition{section:"LivingWorld_Spectator",label:"kMaxNumForceSpectators",title:"Max Num Force Spectator",initial:Value::Integer{value:24,minimum:1,maximum:50}},
    //1283..1285: crowd presentation initializers82f6b8f8/b948/b9a8, floats-10000..10000.
    Definition{section:"Render_Debug_Crowd",label:"kCrowdPresEnable",title:"Enable",initial:Value::Boolean(true)},
    Definition{section:"Render_Debug_Crowd",label:"kCrowdImposterPN",title:"Projection Near",initial:Value::Float(-10.)},
    Definition{section:"Render_Debug_Crowd",label:"kCrowdImposterPF",title:"Projection Far",initial:Value::Float(10.)},
];
const CVAR_COUNT: usize = DEFINITIONS.len();
#[path = "input_cvar_camera_anchor.rs"]
pub(crate) mod camera_anchor;
#[path = "input_cvar_camera_anchor_debug.rs"]
pub(crate) mod camera_anchor_debug;
#[path = "input_cvar_camera_dynamics.rs"]
pub(crate) mod camera_dynamics;
#[path = "input_cvar_camera_elevation.rs"]
pub(crate) mod camera_elevation;
#[path = "input_cvar_camera_framing.rs"]
pub(crate) mod camera_framing;
#[path = "input_cvar_camera_graph.rs"]
pub(crate) mod camera_graph;
#[path = "input_cvar_camera_instances.rs"]
pub(crate) mod camera_instances;
#[path = "input_cvar_camera_positioner.rs"]
pub(crate) mod camera_positioner;
#[path = "input_cvar_camera_rig_debug.rs"]
pub(crate) mod camera_rig_debug;
#[path = "input_cvar_camera_shots.rs"]
pub(crate) mod camera_shots;
#[path = "input_cvar_camera_update.rs"]
pub(crate) mod camera_update;
#[path = "input_cvar_known_air.rs"]
pub(crate) mod known_air;
#[path = "input_cvar_offboard_jumping.rs"]
mod offboard_jumping;
#[path = "input_cvar_offboard_location.rs"]
mod offboard_location;
#[path = "input_cvar_scoring.rs"]
pub(crate) mod scoring;
#[path = "input_cvar_skitch.rs"]
pub(crate) mod skitch;
// Static initializer tables82fd1058..1078,150c..153c,2a24,5dc8,5790..5798,62b0,6494.
const REGISTRATION_ORDER: [usize; CVAR_COUNT] = [
    519, 329, 520, 521, 298, 239, 51, 52, 342, 20, 21, 18, 0, 1, 364, 2, 3, 4, 5, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 294, 191, 301, 302, 303, 304, 297, 292, 299, 300, 427, 428, 429, 430, 431, 432,
    433, 434, 435, 436, 437, 296, 115, 280, 281, 16, 589, 590, 591, 592, 593, 594, 595, 282, 283,
    284, 285, 84, 85, 157, 158, 142, 117, 143, 56, 118, 119, 54, 55, 596, 144, 145, 120, 121, 122,
    123, 113, 154, 155, 156, 388, 266, 28, 29, 359, 360, 116, 130, 132, 133, 134, 135, 136, 137,
    138, 287, 286, 131, 193, 129, 141, 222, 59, 57, 194, 223, 224, 58, 60, 61, 62, 63, 64, 65, 392,
    460, 564, 270, 269, 380, 381, 394, 383, 384, 139, 140, 549, 550, 551, 24, 25, 23, 267, 268, 93,
    66, 94, 256, 257, 95, 203, 96, 204, 97, 98, 374, 99, 214, 213, 100, 101, 102, 103, 104, 105,
    106, 107, 108, 109, 110, 202, 474, 49, 482, 495, 502, 503, 497, 496, 504, 498, 499, 505, 501,
    500, 494, 487, 489, 492, 491, 493, 490, 488, 475, 483, 484, 479, 485, 486, 476, 477, 478, 470,
    471, 472, 473, 26, 27, 45, 46, 48, 47, 468, 480, 481, 465, 466, 467, 469, 464, 124, 517, 44,
    356, 518, 345, 279, 534, 227, 228, 229, 230, 343, 584, 585, 586, 587, 588, 38, 231, 238, 344,
    348, 390, 391, 237, 548, 234, 235, 236, 506, 308, 309, 310, 311, 312, 313, 890, 35, 891, 892,
    893, 232, 233, 540, 541, 542, 543, 544, 545, 546, 560, 39, 40, 41, 42, 367, 357, 358, 70, 71,
    293, 67, 68, 69, 288, 289, 389, 290, 291, 438, 439, 440, 441, 442, 443, 444, 445, 446, 447,
    448, 523, 30, 246, 245, 247, 539, 249, 248, 250, 251, 524, 525, 526, 527, 528, 529, 530, 128,
    17, 114, 146, 125, 126, 127, 147, 148, 149, 150, 151, 152, 153, 533, 258, 32, 33, 34, 274, 271,
    272, 273, 259, 260, 261, 262, 263, 264, 265, 275, 276, 277, 278, 346, 378, 379, 597, 598, 375,
    376, 377, 547, 385, 461, 43, 410, 411, 412, 413, 414, 415, 416, 417, 418, 419, 420, 421, 422,
    423, 424, 561, 562, 563, 449, 450, 451, 452, 453, 454, 455, 456, 457, 608, 609, 522, 240, 252,
    241, 253, 254, 255, 395, 396, 397, 398, 399, 565, 566, 567, 568, 569, 570, 571, 572, 573, 574,
    575, 576, 577, 578, 579, 580, 581, 582, 583, 219, 215, 217, 218, 216, 86, 87, 88, 242, 89, 243,
    244, 372, 347, 552, 553, 554, 555, 556, 557, 558, 559, 393, 368, 373, 371, 369, 370, 365, 366,
    400, 401, 402, 403, 404, 425, 426, 408, 409, 405, 406, 407, 507, 508, 509, 510, 511, 512, 513,
    514, 515, 516, 607, 606, 599, 600, 602, 605, 603, 604, 601, 458, 6, 361, 362, 363, 36, 37, 72,
    73, 74, 75, 76, 77, 19, 78, 531, 382, 22, 532, 90, 92, 91, 111, 112, 159, 160, 386, 387, 162,
    161, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180,
    181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 610, 611, 82, 81, 79, 53, 192, 83, 50, 535,
    536, 537, 538, 195, 196, 197, 198, 199, 200, 201, 80, 205, 206, 207, 208, 209, 210, 211, 212,
    220, 221, 31, 225, 226, 295, 314, 305, 316, 315, 349, 350, 351, 317, 320, 459, 462, 307, 318,
    306, 463, 1245, 331, 330, 332, 321, 352, 353, 333, 334, 335, 322, 354, 323, 324, 325, 326, 327, 328,
    336, 337, 338, 339, 340, 341, 355, 319,
    // 612..797: grind references, appended; their unique section+label pairs
    // make relative link order unobservable.
    612, 613, 614, 615, 616, 617, 618, 619, 620, 621, 622, 623, 624, 625, 626, 627, 628, 629, 630,
    631, 632, 633, 634, 635, 636, 637, 638, 639, 640, 641, 642, 643, 644, 645, 646, 647, 648, 649,
    650, 651, 652, 653, 654, 655, 656, 657, 658, 659, 660, 661, 662, 663, 664, 665, 666, 667, 668,
    669, 670, 671, 672, 673, 674, 675, 676, 677, 678, 679, 680, 681, 682, 683, 684, 685, 686, 687,
    688, 689, 690, 691, 692, 693, 694, 695, 696, 697, 698, 699, 700, 701, 702, 703, 704, 705, 706,
    707, 708, 709, 710, 711, 712, 713, 714, 715, 716, 717, 718, 719, 720, 721, 722, 723, 724, 725,
    726, 727, 728, 729, 730, 731, 732, 733, 734, 735, 736, 737, 738, 739, 740, 741, 742, 743, 744,
    745, 746, 747, 748, 749, 750, 751, 752, 753, 754, 755, 756, 757, 758, 759, 760, 761, 762, 763,
    764, 765, 766, 767, 768, 769, 770, 771, 772, 773, 774, 775, 776, 777, 778, 779, 780, 781, 782,
    783, 784, 785, 786, 787, 788, 789, 790, 791, 792, 793, 794, 795, 796, 797,
    // 798..809: grind conditioner and Nonspecific references, appended; unique
    // section+label pairs keep relative link order unobservable.
    798, 799, 800, 801, 802, 803, 804, 805, 806, 807, 808, 809,
    // 810: the camera grind-ledge reference, appended.
    810, // 811..817: TrajectorySelector and TrajectoryBoxTest grind references, appended.
    811, 812, 813, 814, 815, 816, 817,
    // 818..821: GrindAirAdjust air-assist references, appended; 822..823: the
    // FootPlant CheckForGrinds references. Unique section+label pairs keep
    // relative link order unobservable.
    818, 819, 820, 821, 822, 823,
    // 824..826: Animation_Skater grind crouch and control-fade references, appended;
    // unique section+label pairs keep relative link order unobservable.
    824, 825, 826,
    // 827..829: DMO locator loading references, appended; unique section+label
    // pairs keep relative link order unobservable.
    827, 828, 829,
    // 830..845: LivingWorld DMO spawn-test, census and model-loading references,
    // appended; unique section+label pairs keep relative link order unobservable.
    830, 831, 832, 833, 834, 835, 836, 837, 838, 839, 840, 841, 842, 843, 844, 845,
    // 846..854: DMO pool, DMO pipeline and DMO debug physics references,
    // appended; unique section+label pairs keep relative link order unobservable.
    846, 847, 848, 849, 850, 851, 852, 853, 854, 855, 856, 857, 858, 859, 860, 861, 862, 863, 864,
    865, 866, 867, 868, 869, 870, 871, 872, 873, 874, 875, 876, 877, 878, 879, 880, 881, 882, 883,
    884, 885, 886, 887, 888, 889,
    // Unique section+label pairs; preserve the two source initializers' order.
    894, 895, // Unique section+label; fixed-foot range reference.
    896, // Unique labels; no existing target aliases.
    897, 898, 899, 900, 901, 902, 903, 904, 905, 906, 907, 908, 909, 910, 911, 912, 913, 914, 915,
    916, 917, 918, 919, 920, 921, 922, 923, 924, 925, 926, 927, 928, 929, 930, 931, 932, 933, 934,
    935, 936, 937, 938, 939, 940, 941, 942, 943, 944, 945, 946, 947, 948, 949, 950, 951, 952, 953,
    954, 955, 956, // Unique Animation_Skitching labels, source slots82fd5108/5110/5114.
    957, 958, 959, 960, 961, 962, 963, 964, 965, 966, 967, 968, 969, 970, 971, 972, 973, 974, 975,
    976, 977, 978, 979, 980, 981,
    // Unique native scoring section+label pairs; retain initializer order.
    982, 983, 984, 985, 986, 987, 988, 989, 990, 991, 992, 993, 994, 995, 996, 997, 998, 999, 1000,
    1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008, 1009, 1010, 1011, 1012, 1013, 1014, 1015, 1016,
    1017, 1018, 1019, 1020, 1021, 1022, 1023, 1024, 1025, 1026, 1027, 1028, 1029, 1030, 1031, 1032,
    1033, 1034, 1035, 1036, 1037, 1041, 1038, 1039, 1040,
    1042, 1046, 1047, 1048, 1043, 1050, 1044, 1051, 1049, 1052, 1045,
    1053, 1054,
    // Unique native section/labels in original initializer relative order.
    1055, 1056, 1057, 1058, 1059, 1060,
    1061, 1062, 1063, 1064, 1065, 1066, 1067, 1068, 1069,
    1070, 1071, 1072, 1073, 1074, 1075,
    1076, 1077, 1078, 1079, 1080, 1081, 1082, 1083, 1084, 1085, 1086, 1087, 1088,
    1089, 1090, 1091,
    1092, 1093, 1094, 1095, 1096, 1097, 1098, 1099, 1100, 1101,
    1102, 1103,
    1104,
    1105, 1106,
    1107, 1108, 1109, 1110,
    1111, 1112, 1113, 1114, 1115, 1116, 1117,
    1118, 1119, 1120, 1121, 1122, 1123, 1124, 1125, 1126,
    1127,1128,1129,1130,1131,1132,1133,1134,1135,1136,
    1137,1138,1139,
    1140,1141,1142,1143,
    1144,
    1145,
    1146,1147,
    1148,1149,
    1150,1151,1152,1153,1154,1155,1156,1157,1158,1159,1160,1161,1162,1163,1164,1165,1166,
    1167,1168,1169,1170,1171,
    1172,1173,1174,1175,1176,
    1177,1178,1185,1179,1180,1181,1182,1186,1187,1188,1184,1183,1189,1190,
    1197,1198,1196,1191,1192,1193,1194,1195,
    1199,1200,
    1201,
    1202,1203,
    1204,1205,1206,
    1207,1208,1209,1210,1211,1212,1213,1214,1215,1216,1217,1218,1219,1220,1221,1222,1223,1224,1225,1226,1227,1228,1229,1230,1231,1232,1233,1234,1235,1236,1237,1238,1239,1240,1241,1242,1243,1244,
    1246,
    1247,1248,1249,1250,1251,1252,1253,
    1254,1255,1256,1257,
    1258, // Unique section/label; no existing target alias.
    1259,1260,1261,1262,1263,1264,1265,1266,1267,1268,1269,1270,1271,1272,
    1273,1274,1275,1276,1277,1278,1279,1280,1281,
    1282, // Unique section/label; no existing target alias.
    1283,1284,1285,
];
type Target = Arc<Mutex<Value>>;
impl InputCvarReferences {
    pub(crate) fn try_ai_always_respawn_on_path(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1254)}
    pub(crate) fn try_ai_respawn_behind_in_races(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1255)}
    pub(crate) fn try_ai_respawn_behind_always(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1256)}
    pub(crate) fn try_ai_max_node_advance_for_blocking_object(&self)->Result<i32,InputCvarError>{self.try_current_integer(1257)}
    pub(crate) fn try_frontend_fade_speed(&self)->Result<f32,InputCvarError>{self.try_current_float(1244)}
    pub(crate) fn try_running_man_achievement_time(&self)->Result<f32,InputCvarError>{self.try_current_float(1253)}
    /// `Sk8::Physics::kEnableHatCollision`, sampled by `CreateHatVolume` 0x82b4fae8.
    pub(crate) fn try_enable_hat_collision(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1246)}
    /// `Sk8::kMinSimRadiusForNoLoadScreenOnTelelport` and `kMinPresRadiusForNoLoadScreenOnTelelport`,
    /// the radii `TeleportRequiresLoadScreen` 0x828a9420 tests without a large load radius.
    pub(crate) fn try_min_sim_radius_for_no_load_screen_on_teleport(&self)->Result<f32,InputCvarError>{self.try_current_float(1247)}
    pub(crate) fn try_min_pres_radius_for_no_load_screen_on_teleport(&self)->Result<f32,InputCvarError>{self.try_current_float(1248)}
    pub(crate) fn try_ropa_maxdt_ratio(&self)->Result<f32,InputCvarError>{self.try_current_float(1233)}
    pub(crate) fn try_ropa_mindt_integration(&self)->Result<f32,InputCvarError>{self.try_current_float(1234)}
    pub(crate) fn try_ropa_fixeddt(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1235)}
    pub(crate) fn try_ropa_gravity(&self)->Result<f32,InputCvarError>{self.try_current_float(1236)}
    pub(crate) fn try_ropa_vertical_speed_dampening(&self)->Result<f32,InputCvarError>{self.try_current_float(1237)}
    pub(crate) fn try_ropa_compression(&self)->Result<f32,InputCvarError>{self.try_current_float(1238)}
    pub(crate) fn try_ropa_vert_distance_scale(&self)->Result<f32,InputCvarError>{self.try_current_float(1239)}
    pub(crate) fn try_ropa_norm_distance_scale(&self)->Result<f32,InputCvarError>{self.try_current_float(1240)}
    pub(crate) fn try_ropa_nb_iterations(&self)->Result<i32,InputCvarError>{self.try_current_integer(1241)}
    pub(crate) fn try_ropa_use_constraint_length_on_normals(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1242)}
    pub(crate) fn try_ropa_nb_wipe_out_frames(&self)->Result<i32,InputCvarError>{self.try_current_integer(1243)}
    pub(crate) fn try_cas_correct_rostral(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1231)}
    pub(crate) fn try_cas_preview_delay(&self)->Result<i32,InputCvarError>{self.try_current_integer(1232)}
    pub(crate) fn try_hand_prop_dispose_throw_timeout(&self)->Result<f32,InputCvarError>{self.try_current_float(1177)}
    pub(crate) fn try_hand_prop_attack_throw_timeout(&self)->Result<f32,InputCvarError>{self.try_current_float(1178)}
    pub(crate) fn try_hand_prop_dispose_force(&self)->Result<f32,InputCvarError>{self.try_current_float(1179)}
    pub(crate) fn try_hand_prop_attack_throw_force(&self)->Result<f32,InputCvarError>{self.try_current_float(1180)}
    pub(crate) fn try_hand_prop_attack_forward_angle(&self)->Result<f32,InputCvarError>{self.try_current_float(1181)}
    pub(crate) fn try_hand_prop_throw_force(&self)->Result<f32,InputCvarError>{self.try_current_float(1185)}
    pub(crate) fn try_hand_prop_disown_distance_x(&self)->Result<f32,InputCvarError>{self.try_current_float(1186)}
    pub(crate) fn try_hand_prop_disown_distance_y(&self)->Result<f32,InputCvarError>{self.try_current_float(1187)}
    pub(crate) fn try_hand_prop_disown_distance_z(&self)->Result<f32,InputCvarError>{self.try_current_float(1188)}
    pub(crate) fn try_hand_prop_throw_angle_tolerance(&self)->Result<f32,InputCvarError>{self.try_current_float(1182)}
    pub(crate) fn try_allow_stats_anytime(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1183)}
    pub(crate) fn try_allow_personal_bests_anytime(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1184)}
}
type GestureReferenceCopies = Arc<Mutex<Option<[Option<Target>; 2]>>>;
#[derive(Clone)]
pub(crate) struct InputCvarReferences {
    targets: Arc<Mutex<[Option<Target>; CVAR_COUNT]>>,
    gesture_copies: GestureReferenceCopies,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputCvarError {
    Lock,
    Unlinked(&'static str),
    Uncaptured(&'static str),
    WrongType,
    IniFieldTooLong,
}
impl InputCvarReferences {
    pub(crate) fn try_pedestrian_min_rotate_to_target_angle(&self)->Result<f32,InputCvarError>{self.try_current_float(1150)}
    pub(crate) fn try_pedestrian_plugin_shift_with_dmo(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1151)}
    pub(crate) fn try_pedestrian_hand_prop_position_prediction(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1152)}
    pub(crate) fn try_pedestrian_default_mesh_y_tolerance(&self)->Result<f32,InputCvarError>{self.try_current_float(1153)}
    pub(crate) fn try_big_black_spawn(&self)->Result<(f32,f32,f32,f32,i32,f32,f32,f32),InputCvarError>{
        Ok((self.try_current_float(1273)?,self.try_current_float(1274)?,self.try_current_float(1275)?,
            self.try_current_float(1276)?,self.try_current_integer(1277)?,self.try_current_float(1278)?,
            self.try_current_float(1279)?,self.try_current_float(1280)?))
    }
    pub(crate) fn try_big_black_avoidance_enabled(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1281)}
    pub(crate) fn try_max_num_force_spectators(&self)->Result<i32,InputCvarError>{self.try_current_integer(1282)}
    pub(crate) fn try_crowd_presentation_enabled(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1283)}
    pub(crate) fn try_crowd_imposter_near(&self)->Result<f32,InputCvarError>{self.try_current_float(1284)}
    pub(crate) fn try_crowd_imposter_far(&self)->Result<f32,InputCvarError>{self.try_current_float(1285)}
    pub(crate) fn try_pedestrian_big_black_turn_angle(&self)->Result<f32,InputCvarError>{self.try_current_float(1172)}
    pub(crate) fn try_pedestrian_vehicle_avoidance_tuning(&self)->Result<crate::pedestrian_navigation_modifiers::vehicle_avoidance::VehicleTuning,InputCvarError>{
        Ok(crate::pedestrian_navigation_modifiers::vehicle_avoidance::VehicleTuning{
            jump_distance:self.try_current_float(1173)?,jump_minimum_speed:self.try_current_float(1174)?,
            flee_distance:self.try_current_float(1175)?,flee_speed:self.try_current_float(1176)?,
        })
    }
    pub(crate) fn try_pedestrian_dmo_avoidance_tuning(&self)->Result<crate::pedestrian_navigation_modifiers::dmo_avoidance::DmoAvoidanceTuning,InputCvarError>{
        Ok(crate::pedestrian_navigation_modifiers::dmo_avoidance::DmoAvoidanceTuning{
            influence_radius:self.try_current_float(1167)?,minimum_start_speed:self.try_current_float(1168)?,
            corner_distance:self.try_current_float(1169)?,max_jump_distance:self.try_current_float(1170)?,
            allow_avoid:self.try_current_boolean(1171)?,
        })
    }
    pub(crate) fn try_pedestrian_road_prediction(&self)->Result<(f32,f32),InputCvarError>{Ok((self.try_current_float(1154)?,self.try_current_float(1155)?))}
    pub(crate) fn try_pedestrian_general_avoidance_tuning(&self)->Result<crate::pedestrian_navigation_modifiers::general_avoidance::GeneralTuning,InputCvarError>{
        Ok(crate::pedestrian_navigation_modifiers::general_avoidance::GeneralTuning{
            avoid_speed:self.try_current_float(1156)?,road_speed:self.try_current_float(1157)?,
            oncoming_angle:self.try_current_float(1158)?,oncoming_avoid_angle:self.try_current_float(1159)?,
            oncoming_avoid_distance:self.try_current_float(1160)?,outer_radius:self.try_current_float(1161)?,
            inner_radius:self.try_current_float(1162)?,height_tolerance:self.try_current_float(1163)?,
            intersect_tolerance:self.try_current_float(1164)?,oncoming_width:self.try_current_float(1165)?,
            front_half_angle:self.try_current_float(1166)?,
        })
    }
    pub(crate) fn try_pedestrian_entity_knowledge_suppressed(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1148)}
    pub(crate) fn try_pedestrian_warn_interest_timer(&self)->Result<f32,InputCvarError>{self.try_current_float(1149)}
    pub(crate) fn try_enable_pedestrian_motion_graph_pools(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1146)}
    pub(crate) fn try_generate_foot_tags_for_all_peds(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1147)}
    pub(crate) fn try_pedestrian_disable_takedowns(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1145)}
    pub(crate) fn try_hand_prop_throw_timeout(&self)->Result<f32,InputCvarError>{self.try_current_float(1144)}
    /// HiFi, LoFi, NoFi, SecurityGuard; native reads preserve integer bits.
    pub(crate) fn try_pedestrian_astar_depths(&self)->Result<[u32;4],InputCvarError>{
        Ok([self.try_current_integer(1142)? as u32,self.try_current_integer(1141)? as u32,
            self.try_current_integer(1140)? as u32,self.try_current_integer(1143)? as u32])
    }
    pub(crate) fn try_animation_branch_window(&self) -> Result<f32, InputCvarError> { self.try_current_float(1137) }
    pub(crate) fn try_pedestrian_assert_on_missing_animation(&self) -> Result<bool, InputCvarError> { self.try_current_boolean(1138) }
    pub(crate) fn try_dmo_hand_prop_cull_timer(&self) -> Result<f32, InputCvarError> { self.try_current_float(1139) }
    pub(crate) fn try_prop_factory_callbacks(&self)->Result<[bool;11],InputCvarError>{
        Ok([self.try_current_boolean(1134)?,self.try_current_boolean(1116)?,self.try_current_boolean(1127)?,self.try_current_boolean(1128)?,self.try_current_boolean(1129)?,self.try_current_boolean(1130)?,self.try_current_boolean(1131)?,self.try_current_boolean(1132)?,self.try_current_boolean(1133)?,self.try_current_boolean(1135)?,self.try_current_boolean(1136)?])
    }

    pub(crate) fn try_ignore_min_dist_for_prop_spawn(&self) -> Result<bool, InputCvarError> { self.try_current_boolean(1126) }
    pub(crate) fn try_pedestrian_jump_shift_collision(&self) -> Result<bool, InputCvarError> { self.try_current_boolean(1121) }
    pub(crate) fn try_pedestrian_jump_shift_amount(&self) -> Result<f32, InputCvarError> { self.try_current_float(1122) }
    pub(crate) fn try_pedestrian_plugin_collision_body_shift_amount(&self) -> Result<f32, InputCvarError> { self.try_current_float(1123) }
    pub(crate) fn try_pedestrian_plugin_lock_tolerance(&self) -> Result<f32, InputCvarError> { self.try_current_float(1124) }
    pub(crate) fn try_pedestrian_plugin_unlock_tolerance(&self) -> Result<f32, InputCvarError> { self.try_current_float(1125) }
    pub(crate) fn try_pedestrian_spawn_density(&self) -> Result<f32, InputCvarError> { self.try_current_float(1118) }
    pub(crate) fn try_pedestrian_spawn_dmo_check_radius(&self) -> Result<f32, InputCvarError> { self.try_current_float(1119) }
    pub(crate) fn try_pedestrian_spawn_dmo_padding(&self) -> Result<f32, InputCvarError> { self.try_current_float(1120) }
    pub(crate) fn try_prop_execution_rates(&self)->Result<[u32;3],InputCvarError>{
        Ok([self.try_current_integer(1113)? as u32,self.try_current_integer(1114)? as u32,self.try_current_integer(1115)? as u32])
    }
    pub(crate) fn try_enable_patrol_point_spawn(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1116)}
    pub(crate) fn try_max_props(&self)->Result<u32,InputCvarError>{Ok(self.try_current_integer(1117)? as u32)}
    pub(crate) fn try_pedestrian_use_test_motion_graph(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1111)
    }
    pub(crate) fn try_pedestrian_blend_cull_threshold(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(1112)
    }
    pub(crate) fn try_pedestrian_collision_ignorance_radius_factor(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(1110)
    }
    pub(crate) fn try_pedestrian_skater_collision_ignorance_radius_factor(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(1109)
    }
    pub(crate) fn try_pedestrian_max_recovery_attempts(&self) -> Result<u32, InputCvarError> {
        Ok(self.try_current_integer(1107)? as u32)
    }
    pub(crate) fn try_pedestrian_force_recovery_with_skater(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1108)
    }
    pub(crate) fn try_pedestrian_allow_incomplete_path_in_chase(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1105)
    }
    pub(crate) fn try_pedestrian_chase_nav_fail_distance(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(1106)
    }
    /// Original static constructor82fa6da8.
    pub(crate) fn try_pedestrian_do_rain_iteration(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1104)
    }
    pub(crate) fn try_pedestrian_ground_test_distance_up_default(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(1102)
    }
    pub(crate) fn try_pedestrian_allow_hand_props(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1103)
    }
    pub(crate) fn try_pedestrian_navigation_intent_tuning(&self)
        -> Result<crate::pedestrian_navigation_modifiers::intent::NavigationIntentTuning,InputCvarError> {
        Ok(crate::pedestrian_navigation_modifiers::intent::NavigationIntentTuning {
            jump:crate::pedestrian_navigation::rig::JumpTuning {
                distance:self.try_current_float(1098)?,
                height_offset:self.try_current_float(1097)?,
                min_up:self.try_current_float(1061)?,
                max_up:self.try_current_float(1062)?,
                min_down:self.try_current_float(1063)?,
                max_down:self.try_current_float(1064)?,
                down_ledge_difference:self.try_current_float(1092)?,
            },
            jump_override:self.try_current_boolean(1099)?,
            distance_initial:self.try_current_float(1094)?,
            distance_scalar:self.try_current_float(1095)?,
            usage_velocity_threshold:self.try_current_float(1096)?,
            smooth_best_ray:self.try_current_boolean(1093)?,
            check_jump_mesh:self.try_current_boolean(1100)?,
            jump_mesh_y_tolerance:self.try_current_float(1101)?,
        })
    }
    pub(crate) fn try_pedestrian_execution_rates(&self) -> Result<[u32;3],InputCvarError> {
        Ok([self.try_current_integer(1089)? as u32,
            self.try_current_integer(1090)? as u32,self.try_current_integer(1091)? as u32])
    }
    pub(crate) fn try_pedestrian_skater_avoidance_tuning(&self)
        -> Result<crate::pedestrian_navigation_modifiers::skater_avoidance::SkaterAvoidanceTuning, InputCvarError>
    {
        Ok(crate::pedestrian_navigation_modifiers::skater_avoidance::SkaterAvoidanceTuning {
            nav_mesh_only: self.try_current_boolean(1076)?,
            large_avoid_distance: self.try_current_float(1077)?,
            small_avoid_distance: self.try_current_float(1078)?,
            nav_mesh_y_tolerance: self.try_current_float(1079)?,
            avoid_npc_skaters: self.try_current_boolean(1080)?,
            predict_time: self.try_current_float(1081)?,
            outer_radius: self.try_current_float(1082)?,
            inner_radius: self.try_current_float(1083)?,
            influence_radius: self.try_current_float(1084)?,
            forward_angle: self.try_current_float(1085)?,
            minimum_start_speed: self.try_current_float(1086)?,
            big_avoid_speed: self.try_current_float(1087)?,
            allow_avoid: self.try_current_boolean(1088)?,
        })
    }
    /// InitWorld827b4360's own pedestrian AssemblyPool flag, not the skater
    /// or DynamicAssemblyPool parallel-update setting.
    pub(crate) fn try_pedestrian_pool_update_in_parallel(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1069)
    }
    pub(crate) fn try_pedestrian_jump_tuning(&self)
        -> Result<crate::pedestrian_adjust_owners::jump_trajectory::JumpTuning, InputCvarError>
    {
        Ok(crate::pedestrian_adjust_owners::jump_trajectory::JumpTuning {
            ledge_failure_distance: self.try_current_float(1068)?,
            ledge_clearance: self.try_current_float(1067)?,
            min_horizontal_ratio: self.try_current_float(1065)?,
            max_horizontal_ratio: self.try_current_float(1066)?,
            up_min_height: self.try_current_float(1061)?,
            up_max_height: self.try_current_float(1062)?,
            down_min_height: self.try_current_float(1063)?,
            down_max_height: self.try_current_float(1064)?,
            use_ledge: self.try_current_boolean(1059)?,
            use_landing: self.try_current_boolean(1060)?,
        })
    }
    /// Current linked targets for SpatialAdjust::Update82e745a0. Values are
    /// fetched each call, retaining startup config overrides and live changes.
    pub(crate) fn try_pedestrian_spatial_adjust_tuning(&self)
        -> Result<crate::pedestrian_adjust_owners::SpatialAdjustTuning, InputCvarError>
    {
        Ok(crate::pedestrian_adjust_owners::SpatialAdjustTuning {
            pullback_speed: self.try_current_float(1057)?,
            y_shift_time: self.try_current_float(1055)?,
            sliding_clamp: self.try_current_float(1056)?,
            jump_use_landing: self.try_current_boolean(1060)?,
            jump_use_ledge: self.try_current_boolean(1059)?,
            jump_arrival_radius: self.try_current_float(1058)?,
        })
    }
    /// Original volume probe defaults82fa95d8/9698/9998; shared start offset.
    pub(crate) fn try_pedestrian_volume_tests(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1052)
    }
    pub(crate) fn try_pedestrian_avoidance_expansion(&self) -> Result<[f32; 2], InputCvarError> {
        Ok([self.try_current_float(1053)?, self.try_current_float(1054)?])
    }
    pub(crate) fn try_pedestrian_volume_probe_tuning(&self) -> Result<[f32; 3], InputCvarError> {
        Ok([self.try_current_float(1050)?, self.try_current_float(1051)?, self.try_current_float(1045)?])
    }
    pub(crate) fn try_pedestrian_physics_tests(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1049)
    }
    pub(crate) fn try_security_guard_mesh_cost_threshold(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(1201)
    }
    pub(crate) fn try_plugin_transfer_y_tolerance(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(1202)
    }
    pub(crate) fn try_plugin_retry_timer(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(1203)
    }
    /// AddTestsToGroundDMOBatch82e789c8. Starting jump distances are cvars
    /// (GetStartingJumpForward/HeightDistance82e4d730/748), not previous hits.
    pub(crate) fn try_pedestrian_ground_dmo_tuning(&self)
        -> Result<crate::pedestrian_ground_dmo_batches::GroundDmoTuning, InputCvarError>
    {
        Ok(crate::pedestrian_ground_dmo_batches::GroundDmoTuning {
            distance_up_default: self.try_current_float(1102)?,
            distance_up_knock_down: self.try_current_float(1192)?,
            distance_down: self.try_current_float(1191)?,
            jump_forward_distance: self.try_current_float(1197)?,
            jump_forward_fatness: self.try_current_float(1193)?,
            jump_forward_height: self.try_current_float(1194)?,
            test_start_offset: self.try_current_float(1045)?,
            jump_height_distance: self.try_current_float(1198)?,
            jump_height_height: self.try_current_float(1097)?,
            jump_height_lead: self.try_current_float(1098)?,
            jump_height_fatness: self.try_current_float(1195)?,
            taze_line_start_offset: self.try_current_float(1196)?,
        })
    }
    pub(crate) fn try_pedestrian_probe_tuning(
        &self,
    ) -> Result<crate::pedestrian_navigation::rig::NavRigProbeTuning, InputCvarError> {
        Ok(crate::pedestrian_navigation::rig::NavRigProbeTuning {
            distance: self.try_current_float(1042)?,
            fatness: self.try_current_float(1043)?,
            y_clearance: self.try_current_float(1044)?,
            start_offset: self.try_current_float(1045)?,
            down_start_offset: self.try_current_float(1046)?,
            down_start_height: self.try_current_float(1047)?,
            down_height: self.try_current_float(1048)?,
        })
    }
    pub(crate) fn try_pedestrian_rig_tuning(
        &self,
    ) -> Result<crate::pedestrian_navigation::rig::NavRigTuning, InputCvarError> {
        Ok(crate::pedestrian_navigation::rig::NavRigTuning {
            distance: self.try_current_float(1035)?,
            cache_distance: self.try_current_float(1036)?,
            physics_caching: self.try_current_boolean(1037)?,
            path_validation: self.try_current_boolean(1038)?,
            topology_angle: self.try_current_float(1039)?,
            topology_distance: self.try_current_float(1040)?,
            batch_tests: self.try_current_boolean(1041)?,
        })
    }
    pub(crate) fn try_pedestrian_road_tuning(
        &self,
    ) -> Result<crate::pedestrian_navigation::road::RoadNavigationTuning, InputCvarError> {
        Ok(crate::pedestrian_navigation::road::RoadNavigationTuning {
            valid_distance: self.try_current_float(1031)?,
            valid_angle: self.try_current_float(1032)?,
            timer_speed: self.try_current_float(1016)?,
            crosswalk_time: self.try_current_float(1033)?,
            allow_incomplete: self.try_current_boolean(1034)?,
        })
    }

    pub(crate) fn try_pedestrian_mesh_tuning(
        &self,
    ) -> Result<crate::pedestrian_navigation::mesh::MeshNavigationTuning, InputCvarError> {
        Ok(crate::pedestrian_navigation::mesh::MeshNavigationTuning {
            allow_astar: self.try_current_boolean(1029)?,
            y_tolerance: self.try_current_float(1025)?,
            off_mesh_anchor_radius: self.try_current_float(1030)?,
            boundary_tests: self.try_pathogen_boundary_tests()?,
        })
    }

    pub(crate) fn try_pathogen_boundary_tests(&self) -> Result<[bool; 3], InputCvarError> {
        Ok([
            self.try_current_boolean(1026)?,
            self.try_current_boolean(1027)?,
            self.try_current_boolean(1028)?,
        ])
    }

    pub(crate) fn try_pedestrian_wander_tuning(
        &self,
    ) -> Result<crate::pedestrian_navigation::wander::NoRoadWanderTuning, InputCvarError> {
        Ok(crate::pedestrian_navigation::wander::NoRoadWanderTuning {
            timer_speed: self.try_current_float(1016)?,
            enabled: self.try_current_boolean(1017)?,
            far_attempts: self.try_current_integer(1018)?,
            far_distance: self.try_current_float(1019)?,
            far_angle: self.try_current_float(1020)?,
            close_attempts: self.try_current_integer(1021)?,
            close_distance: self.try_current_float(1022)?,
            close_angle: self.try_current_float(1023)?,
            fail_distance: self.try_current_float(1024)?,
            y_tolerance: self.try_current_float(1025)?,
        })
    }

    /// CensusMan native initializers82f5d740/82f5d938/82f5da28/82f5dec8.
    pub(crate) fn try_max_pedestrians(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(1012)
    }
    pub(crate) fn try_enable_spawn_pedestrians(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1013)
    }
    pub(crate) fn try_enable_spawn_security(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(1014)
    }
    pub(crate) fn try_initial_spawn_pedestrian_count(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(1015)
    }
    pub(crate) fn try_skitch_draw_debug(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(957)
    }
    pub(crate) fn try_skitch_absorb_delta(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(958)
    }
    pub(crate) fn try_skitch_enter_debug(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(959)
    }
    pub(crate) fn try_jump_close_to_landing(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(534)
    }
    pub(crate) fn try_handplant_display_debug(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(522)
    }
    pub(crate) fn try_disable_stream_focus_movement(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(519)
    }
    pub(crate) fn try_enable_stream_xml(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(521)
    }
    pub(crate) fn try_sim_max_focus_collections(&self) -> Result<usize, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        let target = targets[520]
            .as_ref()
            .ok_or(InputCvarError::Unlinked("kSimMaxFocusCollections"))?;
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            // Native comparison uses the unsigned mValue word.
            Value::Integer { value, .. } => Ok(value as u32 as usize),
            _ => Err(InputCvarError::WrongType),
        }
    }
    pub(crate) fn try_landing_accurate_ik(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(425)
    }
    pub(crate) fn try_landing_zero_ik_y(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(426)
    }

    pub(crate) fn try_landing_debug(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(400)
    }
    pub(crate) fn try_landing_debug_trajectory(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(401)
    }
    pub(crate) fn try_landing_min_jump_time(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(402)
    }
    pub(crate) fn try_landing_min_time_adjust(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(403)
    }
    pub(crate) fn try_landing_max_time_adjust(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(404)
    }
    pub(crate) fn try_landing_trajectory_offset_y(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(405)
    }
    pub(crate) fn try_landing_trajectory_radius(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(406)
    }
    pub(crate) fn try_landing_trajectory_error(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(407)
    }
    pub(crate) fn try_landing_max_error(&self, ai: bool) -> Result<f32, InputCvarError> {
        self.try_current_float(if ai { 409 } else { 408 })
    }

    pub(crate) fn try_max_proxy_y_dist(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(392)
    }
    pub(crate) fn try_slope_effect_scalar(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(393)
    }
    /// CalculateGroundCorrectionAndWipeout82dcd328 live reads, defs552..559.
    pub(crate) fn try_height_above_ground_to_wipeout(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(552)
    }
    pub(crate) fn try_height_above_ground_foot_test(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(553)
    }
    pub(crate) fn try_y_normal_value_to_wipeout(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(554)
    }
    pub(crate) fn try_obik_speed_away_from_bad_foot(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(555)
    }
    pub(crate) fn try_obik_min_traj_value(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(556)
    }
    pub(crate) fn try_obik_frames_to_wipeout_for_bad_ground(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(557)
    }
    pub(crate) fn try_obik_frames_to_wipeout_for_feet_not_down(
        &self,
    ) -> Result<i32, InputCvarError> {
        self.try_current_integer(558)
    }
    pub(crate) fn try_obik_frames_for_ground_correction(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(559)
    }
    pub(crate) fn try_draw_offboard_air_skeleton(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(394)
    }

    /// OffBoard.PostPhysics 0x82d872a0, initializer82f92e90.
    pub(crate) fn try_enable_offboard_wipeouts(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(385)
    }
    /// Wipeout.CheckForOffBoardWipeout 0x82dded58, initializer82f9c3c0.
    pub(crate) fn try_wipeout_display_offboard_info(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(386)
    }
    /// Wipeout.CheckForOffBoardWipeout 0x82ddec94, initializer82f9c500.
    pub(crate) fn try_wipeout_offboard_enabled(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(387)
    }
    /// Wipeout.CheckForOffBoardWipeout 0x82ddefdc; registered table82fd6088.
    pub(crate) fn try_wipeout_display_debug_info(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(160)
    }
    /// Boneless.Update 0x82da0db0; initializer82f94a60/table82fd5afc.
    pub(crate) fn try_boneless_do_jump(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(561)
    }
    /// Boneless.DoJump 0x82d9ec40; initializer82f94b10/table82fd5b04.
    pub(crate) fn try_boneless_traj_pos_delta(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(562)
    }
    /// CalcBonelessJump 0x82d97920; initializer82f94b70/table82fd5b08.
    pub(crate) fn try_boneless_max_jump_error(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(563)
    }
    /// UpdateRootTransformsFixedPart 0x82b426f0; initializer82f7ccd8/table82fd45b8.
    pub(crate) fn try_debug_use_raw_bone_pos(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(564)
    }
    /// CheckForGrinds/CalculateLanding/StartFootPlantGround/UpdateFootPlantGround debug gate.
    pub(crate) fn try_debug_draw_foot_plant(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(565)
    }
    /// UpdatePrediction 0x82dc1698: SetupFakeBodyPrediction radius.
    pub(crate) fn try_foot_plant_traj_radius(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(566)
    }
    /// UpdatePrediction 0x82dc16ac: SetupFakeBodyPrediction duration.
    pub(crate) fn try_foot_plant_traj_max_time(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(567)
    }
    /// UpdatePrediction 0x82dc1690: both SetupFakeBodyPrediction error ratios.
    pub(crate) fn try_foot_plant_traj_error(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(568)
    }
    /// CalculateLanding 0x82dbc650: two-frame foot collision disable gate.
    pub(crate) fn try_fpm_time_to_land_to_disable_foot_collision(
        &self,
    ) -> Result<f32, InputCvarError> {
        self.try_current_float(569)
    }
    /// UpdatePrediction 0x82dc14d8: up-vector start offset.
    pub(crate) fn try_fp_y_foot_offset(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(570)
    }
    /// UpdatePrediction 0x82dc1654: COM-velocity start step scalar.
    pub(crate) fn try_traj_start_test_time_adjust(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(571)
    }
    /// UpdatePrediction 0x82dc13d4: targeting hold.
    pub(crate) fn try_fp_min_time_after_start(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(572)
    }
    /// UpdatePrediction 0x82dc1364: targeting start lower bound.
    pub(crate) fn try_fp_min_time_to_start(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(573)
    }
    /// UpdatePrediction 0x82dc137c: targeting start upper bound.
    pub(crate) fn try_fp_max_time_to_start(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(574)
    }
    /// CheckForGrinds 0x82dbbb20: TrajectoryBoxTest half extent.
    pub(crate) fn try_fp_grind_box_size(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(575)
    }
    /// CalculateLanding 0x82dbc11c: landing-velocity leg weight.
    pub(crate) fn try_com_vel_leg_scalar(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(576)
    }
    /// CalculateLanding 0x82dbc530: IK blend-in step.
    pub(crate) fn try_ik_blend_in_speed(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(577)
    }
    /// CalculateLanding 0x82dbc334: IK preparation time gate.
    pub(crate) fn try_ik_blend_in_max_time_to_land(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(578)
    }
    /// CalculateLanding 0x82dbc154: both entry-angle clamps, in degrees.
    pub(crate) fn try_max_entry_angle_degrees(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(579)
    }
    /// CalculateLanding 0x82dbc478: IK target step clamp.
    pub(crate) fn try_fpm_max_offset_delta(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(580)
    }
    /// CalculateLanding 0x82dbc368: IK preparation enable.
    pub(crate) fn try_ik_foot_prepare(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(581)
    }
    /// StartFootPlantGround 0x82dbcf00: foot collision disable base.
    pub(crate) fn try_frames_to_disable_feet_collision(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(582)
    }
    /// UpdateFootPlantGround 0x82dbd0c8: take-off Y offset.
    pub(crate) fn try_y_trajectory_offset(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(583)
    }
    /// ToggleBoard::UpdateYawPitch 0x82cab0c4: per-update retrieval yaw clamp, degrees.
    pub(crate) fn try_max_retrieve_board_yaw_change(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(584)
    }
    /// ToggleBoard drop/retrieval transition, blend-in and blend-out frames.
    pub(crate) fn try_max_retrieve_board_blend_frames(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(585)
    }
    /// ToggleBoard EndChannelPrematurely blend frames.
    pub(crate) fn try_max_retrieve_board_blend_out_frames(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(586)
    }
    /// ToggleBoard::UpdateRetrievalState 0x82cbbb58: retrieve-out play frames.
    pub(crate) fn try_max_retrieve_board_play_out_frames(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(587)
    }
    /// ToggleBoard::UpdateDropState 0x82cbc0e8: drop/throw play frames.
    pub(crate) fn try_max_drop_board_play_out_frames(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(588)
    }
    /// PlayerUI::UpdateSessionMarker 0x828af7c4: hold time at or below the near distance.
    pub(crate) fn try_min_time_use_session_marker_held(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(589)
    }
    /// PlayerUI::UpdateSessionMarker 0x828af7cc: hold time at or beyond the far distance.
    pub(crate) fn try_max_time_use_session_marker_held(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(590)
    }
    /// PlayerUI::UpdateSessionMarker 0x828af7c8: near distance of the hold-time ramp.
    pub(crate) fn try_min_distance_for_min_time_use_session_marker_held(
        &self,
    ) -> Result<f32, InputCvarError> {
        self.try_current_float(591)
    }
    /// PlayerUI::UpdateSessionMarker 0x828af7d4: far distance of the hold-time ramp.
    pub(crate) fn try_max_distance_for_max_time_use_session_marker_held(
        &self,
    ) -> Result<f32, InputCvarError> {
        self.try_current_float(592)
    }
    /// PlayerUI::UpdateSessionMarker 0x828af688: strict minimum marker distance.
    pub(crate) fn try_min_distance_for_teleport(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(593)
    }
    /// PlayerUI::UpdateSessionMarker 0x828af5c8: bypasses IsUseAllowed and the hold time.
    pub(crate) fn try_debug_session_teleport(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(594)
    }
    /// SessionMarker::Update 0x828acd58: skips the no-skate-zone set precondition.
    pub(crate) fn try_disable_session_marker_pre_conditions(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(595)
    }
    /// CameraObstacles actor search; initializer82fa21a8/table82fd65fc.
    pub(crate) fn try_pedestrian_avoidance(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(349)
    }
    /// CameraObstacles actor search; initializer82fa21f8/table82fd6600.
    pub(crate) fn try_skater_avoidance(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(350)
    }
    /// CameraObstacles actor search; initializer82fa2248/table82fd6604.
    pub(crate) fn try_actor_search_radius(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(351)
    }
    /// Camera avoidance scheduling; initializer82fa29f8/table82fd6664.
    pub(crate) fn try_interleave_avoidance(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(352)
    }
    /// Camera collision avoidance; initializer82fa2a48/table82fd6668.
    pub(crate) fn try_enable_collision_avoidance(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(353)
    }
    /// Camera avoidance debug; initializer82fa2c78/table82fd6684.
    pub(crate) fn try_show_avoidance_debug(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(354)
    }
    /// Camera avoidance speed gate; initializer82fa31c8/table82fd66c8.
    pub(crate) fn try_insignificant_speed(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(355)
    }

    /// `PushCycle::Begin82cd6534` and `Update82cd6880` sample the live target.
    pub(crate) fn try_allow_holding(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(348)
    }
    /// `KickTurnSteering::Update`82cc5860 samples the linked bool each tick.
    pub(crate) fn try_use_fine_kickturn(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(390)
    }
    /// `KickTurnSteering::Update`82cc5860 samples the linked float each tick.
    pub(crate) fn try_kickturn_smoothing(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(391)
    }
    /// MongoPushFootToFar82caa870 samples this live float after GetFootPos.
    pub(crate) fn try_mongo_push_to_antic_thresh(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(345)
    }
    /// ComputeFirstPushStrength.Update82cc57e0 reads target830a4570+0xcd.
    pub(crate) fn try_cap_dv_at_top_speed(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(344)
    }

    /// Fallback `SetTrickHeight::Begin` reads this linked debug target.
    pub(crate) fn try_draw_trick_height_debug_info(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(343)
    }

    /// Live target read at `UpdateGameStreamFileLoaded824e21d4`.
    pub(crate) fn try_enable_texture_stream(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(329)
    }

    /// Live target at `CharacterAssetManager::UpdateRequest82d14c68`.
    pub(crate) fn try_schema_load_from_binary(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(293)
    }
    /// `Step_Solver3` reads the actual target after static RF reset; no null default.
    pub(crate) fn try_enable_freezing(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(292)
    }
    pub(crate) fn try_blend_space_print(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(288)
    }
    pub(crate) fn try_blend_space_jobs(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(289)
    }
    /// Generic `BlendSpace::UpdateParametrisation82d76a08` target.
    pub(crate) fn try_blend_space_slicing_n(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(389)
    }
    pub(crate) fn try_blend_space_optimizations(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(290)
    }
    pub(crate) fn try_blend_space_slicing3(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(291)
    }

    /// Live `DoSimRestore827e6834/6838` sample of the existing linked target.
    pub(crate) fn try_individual_full_bind_pose(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(115)
    }
    pub(crate) fn try_no_hand_animation(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(280)
    }
    pub(crate) fn try_no_face_animation(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(281)
    }
    pub(crate) fn try_draw_animation_carve(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(225)
    }
    pub(crate) fn try_draw_animation_tilt(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(226)
    }

    pub(crate) fn try_do_new_post_physics_ik(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(213)
    }
    pub(crate) fn try_draw_new_post_physics_ik(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(214)
    }
    pub(crate) fn try_draw_post_feet_ik(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(203)
    }
    pub(crate) fn try_draw_post_physics_ik(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(204)
    }
    /// Native initializer82f815a8; mapping samples this only after a hit/output copy.
    pub(crate) fn try_gesture_mapping_debug(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(124)
    }
    /// Source initializer82f8f028; live Ground control debug gate.
    pub(crate) fn try_draw_board_joints(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(114)
    }
    /// Source initializer82f7e0a8; late Skeleton wobble debug gate.
    pub(crate) fn try_debug_draw_skeleton_wobble(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(202)
    }
    pub(crate) fn try_draw_speed_wobble_info(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(116)
    }

    pub(crate) fn try_forced_world_surface(&self) -> Result<u32, InputCvarError> {
        self.try_current_enum(113)
            .map(|value| u32::from_ne_bytes(value.to_ne_bytes()))
    }
    /// Initializer82f7c648/table82fd4558, reference8308e27c; live target read.
    pub(crate) fn try_show_danger_zone(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(59)
    }
    /// Initializer82f7c698/table82fd455c; live `ProcessData` trajectory debug flag.
    pub(crate) fn try_skeleton_display_trajectory_node(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(57)
    }
    /// Reference830dadf0, initializer82f96890/table82fd5c74; sampled per candidate.
    pub(crate) fn try_show_trajectories(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(241)
    }
    /// Actual signed threshold sampled by Air launch and jump correction.
    pub(crate) fn try_physics_air_jump_fix_frames(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(245)
    }
    /// Reference830d97a8, read once by `PhysicsAir::InitMountTrajectory`82d8adc8.
    pub(crate) fn try_physics_air_mount_launch_traj_max_change(
        &self,
    ) -> Result<f32, InputCvarError> {
        self.try_current_float(539)
    }
    /// Reference830a4078, read by `ShouldPrepareOneFootAirForFootplant`82ca77fc
    /// only after a non-negative foot-plant landing time.
    pub(crate) fn try_foot_plant_time_to_land_threshold(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(560)
    }
    /// `kShowSkaterInAirDebugText` target +0xcd (LandingOffBoardAir Begin/Update).
    pub(crate) fn try_show_skater_in_air_debug_text(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(308)
    }
    /// LandingOffBoardAir::Begin82cb4610 compares |landing velocity| against it.
    pub(crate) fn try_landing_velocity_two_foot_threshold(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(540)
    }
    /// LandingOffBoardAir::Update82ca6340 impact blend clamp maximum.
    pub(crate) fn try_landing_impact_max_blend_delta(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(541)
    }
    /// LandingOffBoardAir::Update82ca6340 impact blend clamp minimum.
    pub(crate) fn try_landing_impact_min_blend_delta(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(542)
    }
    /// LandingOffBoardAir::Update82ca6340 per-frame CloseToLand increment.
    pub(crate) fn try_landing_land_pose_blend_delta(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(543)
    }
    /// LandingOffBoardAir::Update82ca6340 time-to-land gate for the impact blend.
    pub(crate) fn try_landing_time_to_blend_to_two_foot(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(544)
    }
    /// LandingOffBoardAir::Update82ca6340 elapsed-time gate for the impact blend.
    pub(crate) fn try_landing_time_to_wait_to_blend_to_two_foot(
        &self,
    ) -> Result<f32, InputCvarError> {
        self.try_current_float(545)
    }
    /// LandingOffBoardAir::Update82ca6340 time-to-land gate for CloseToLand.
    pub(crate) fn try_landing_time_to_blend_to_land_pose(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(546)
    }
    /// Reference830da824; sampled independently by max-time and private Launch.
    pub(crate) fn try_trajectory_debug_draw(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(240)
    }
    /// Initializer82f7c848/table82fd4570; live trajectory identity override.
    pub(crate) fn try_zero_trajectory_transform(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(58)
    }
    /// Initializer82f9e908/table82fd62b0, reference830e0bc4; live skitch timer.
    pub(crate) fn try_time_to_avoid_vehicle_after_skitching(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(53)
    }
    /// Initializer82f563b8/table82fd0ff4, reference83047d78; live float target.
    pub(crate) fn try_skater_fade_in_time(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(51)
    }
    /// Initializer82f56418/table82fd0ff8, reference83047da4; live float target.
    pub(crate) fn try_skater_fade_out_time(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(52)
    }
    /// Initializer82f9ea08/table82fd62bc, reference830e0d98; live float target.
    pub(crate) fn try_feet_line_test_radius(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(50)
    }
    /// `ConditionLineTestResultsForGround`82dcd20c reads this linked target.
    /// Initializer82f988a0/table82fd5df0/ref830db2c4.
    pub(crate) fn try_ob_ik_max_feet_y_dist(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(347)
    }
    /// `OffBoardFeetIK::ClampFootNormal` reads this linked target at
    /// 0x82dcdb10 and again at 0x82dcdc08/7c on the clamped branch.
    pub(crate) fn try_ob_ik_max_y_foot_normal(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(365)
    }
    /// `OffBoardFeetIK::CalculateFeetDownState` reads this linked target.
    pub(crate) fn try_y_height_to_be_off_ground(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(366)
    }
    /// `OffBoardFeetIK::CalculateNewYPos` first linked value at 0x82dc8b68.
    pub(crate) fn try_y_min_height_to_be_off_ground(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(368)
    }
    /// `OffBoardFeetIK::CalculateNewYPos` second linked value at 0x82dc8b70.
    pub(crate) fn try_ob_ik_max_y_lift_up(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(370)
    }
    /// `OffBoardFeetIK::CalculateNewYPos` third linked value at 0x82dc8b74.
    pub(crate) fn try_ob_ik_max_y_dist_up(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(369)
    }
    /// CalculateIKTargets82dd0a50/acc and 82dd0c10/c68: planted-foot range.
    pub(crate) fn try_ob_ik_max_dist(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(896)
    }
    pub(crate) fn try_skitch_search_radius(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(897)
    }
    pub(crate) fn try_draw_grabbable_object_test(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(898)
    }
    pub(crate) fn try_use_authored_vehicle_grab_splines(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(899)
    }
    pub(crate) fn try_feed_vehicle_grab_splines(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(900)
    }
    pub(crate) fn try_skitch_max_entry_angle(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(901)
    }
    pub(crate) fn try_skitch_max_horizontal_entry_angle(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(902)
    }
    pub(crate) fn try_skitch_min_approach_speed(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(903)
    }
    pub(crate) fn try_rain_enable_kd_tree_removals(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(904)
    }
    pub(crate) fn try_recipe_initial_load_limit(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(905)
    }
    pub(crate) fn try_recipe_secondary_load_limit(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(906)
    }
    pub(crate) fn try_recipe_use_forced(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(907)
    }
    /// `OffBoardFeetIK::CalculateIKTargets` reads only when a fixed-foot
    /// blend-out timer is positive, at 0x82dd0b84/0x82dd0d30.
    pub(crate) fn try_ob_ik_time_to_blend_out_fixed_foot_pos(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(371)
    }
    /// `OffBoardFeetIK::Update` reads this after CalculateIKTargets.
    pub(crate) fn try_enable_off_board_feet_ik(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(372)
    }
    /// `OffBoardFeetIK::ApplyIK` state 2 samples this before local setters.
    pub(crate) fn try_ob_ik_blend_val(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(373)
    }
    /// `SkeletonIK::SetTargetNormal` conditional prior-normal clamp.
    pub(crate) fn try_max_normal_angle_delta(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(374)
    }
    /// `SkeletonIK::SetTargetNormal` reads this after setting the normal flag.
    pub(crate) fn try_target_normal_blend_speed(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(98)
    }
    /// BadGroundCorrection samples trajectory threshold before displacement.
    pub(crate) fn try_ob_min_traj_w(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(376)
    }
    pub(crate) fn try_ob_max_skeleton_disp(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(375)
    }
    /// Skeleton::UpdateRootTransformsOffBoard captures this target at
    /// 0x82b44fa8, then reads its value after other vector work at 0x44fc4.
    pub(crate) fn try_capture_max_skeleton_disp(
        &self,
    ) -> Result<InputCvarFloatTarget, InputCvarError> {
        self.try_query_target(380).map(InputCvarFloatTarget)
    }
    /// Read only on the displacement correction branch at 0x82b450d4.
    pub(crate) fn try_collision_push_fraction(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(381)
    }
    /// ResetSystems StateChanger::TeleportReset reads this after seven
    /// counter stores; initializer82f9bf78/table82fd6058.
    pub(crate) fn try_frames_for_teleport_countdown(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(382)
    }
    /// OffBoard root update captures these targets before reloading its
    /// correction/P vectors, then reads values in the same max/blend order.
    pub(crate) fn try_capture_ob_ground_max_delta(
        &self,
    ) -> Result<InputCvarFloatTarget, InputCvarError> {
        self.try_query_target(383).map(InputCvarFloatTarget)
    }
    pub(crate) fn try_capture_ob_ground_prop_blend(
        &self,
    ) -> Result<InputCvarFloatTarget, InputCvarError> {
        self.try_query_target(384).map(InputCvarFloatTarget)
    }
    /// Read only when a valid response direction opposes the correction.
    pub(crate) fn try_ob_debug_coll_response_a(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(377)
    }
    pub(crate) fn try_do_com_vel_blend_out(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(378)
    }
    pub(crate) fn try_frames_to_blend_out_starting_com_vel(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(379)
    }
    /// Initializer82f810f8/table82fd4908, exact default3ea8f5c3; live query.
    pub(crate) fn try_bezier_influence_percentage(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(47)
    }
    /// Initializer82f80ea8/table82fd48e8, source defaultfalse; live query.
    pub(crate) fn try_sa_use_world_y_for_normal(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(48)
    }
    /// Initializer82f7fc30/table82fd4834, reference830a4630; live query.
    pub(crate) fn try_loco_wipeout_delta(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(49)
    }

    /// Initializer82f82f90/table82fd4d10; live boolean reference830a402c.
    pub(crate) fn try_debug_wipeout_conditions(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(44)
    }

    /// `IsInDebugAnimationsMode82c8cba0` reads the live target at 830a4d04.
    pub(crate) fn try_use_debug_animations_mode(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(356)
    }

    /// Initializer82f80d88/table82fd48dc; live reference830a4c98, default3fb33333.
    /// GUI bounds0..5 do not clamp startup values.
    pub(crate) fn try_time_to_land_to_wipeout(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(45)
    }

    /// Initializer82f80de8/table82fd48e0; live reference830a2884, default3f4ccccd.
    pub(crate) fn try_min_time_in_air_to_wipeout(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(46)
    }

    /// Initializer82f93170/table82fd59e4; Pushing.Reset reads live target82d921b8.
    pub(crate) fn try_enable_repushing_time_limit(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(43)
    }

    /// Initializer82f85df0/table82fd4ec8; live boolean reference830a385c.
    pub(crate) fn try_enable_debug_gestures(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(38)
    }
    /// Enum initializer82f88d20; each call samples its current linked target.
    pub(crate) fn try_debug_gesture_up(&self) -> Result<i32, InputCvarError> {
        self.try_current_enum(39)
    }
    /// Enum initializer82f88d70; each call samples its current linked target.
    pub(crate) fn try_debug_gesture_down(&self) -> Result<i32, InputCvarError> {
        self.try_current_enum(40)
    }
    /// Enum initializer82f88dc0; each call samples its current linked target.
    pub(crate) fn try_debug_gesture_left(&self) -> Result<i32, InputCvarError> {
        self.try_current_enum(41)
    }
    /// Enum initializer82f88e10; each call samples its current linked target.
    pub(crate) fn try_debug_gesture_right(&self) -> Result<i32, InputCvarError> {
        self.try_current_enum(42)
    }
    fn try_current_enum(&self, index: usize) -> Result<i32, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        let target = targets[index]
            .as_ref()
            .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))?;
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Enumeration { value, .. } => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }

    /// Initializer82f99dd0/table82fd5ee4. `GrindManager` reads target3874/value3878,
    /// stores the incremented counter3880, then samples the separate curb target.
    pub(crate) fn try_lean_min_non_grind_frames(&self) -> Result<i32, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        let target = targets[36]
            .as_ref()
            .ok_or(InputCvarError::Unlinked("kLeanMinNonGrindFrames"))?;
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Integer { value, .. } => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
    /// Initializer82f99e28/table82fd5ee8. Fresh target load82dd3884 follows the
    /// lean counter store; GUI range0..4 does not clamp startup values.
    pub(crate) fn try_curb_exit_start_time(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(37)
    }
    /// Reference830a3910, initializer82f867d0/table82fd4f38. Motion constructor
    /// samples this live value even when the condition XML supplies an override.
    pub(crate) fn try_mo_angle_max_delta(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(35)
    }
    pub(crate) fn try_mo_strength_max_delta(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(890)
    }
    pub(crate) fn try_living_world_opacity_settings(
        &self,
    ) -> Result<crate::LivingWorldOpacitySettings, InputCvarError> {
        let fade_in_frames = self.try_current_float(895)?;
        let enabled = self.try_current_boolean(894)?;
        Ok(crate::LivingWorldOpacitySettings {
            enabled,
            fade_in_frames,
        })
    }
    /// Update82ca3ee4..3f00 captures raw-speed, crouch and speed targets before
    /// reading any of their values. Relinking must not change those captures.
    pub(crate) fn try_mo_motion_limits(&self) -> Result<[f32; 3], InputCvarError> {
        let captured = {
            let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
            let capture = |index: usize| {
                targets[index]
                    .as_ref()
                    .cloned()
                    .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))
            };
            [capture(892)?, capture(893)?, capture(891)?]
        };
        let mut values = [0.0; 3];
        for (value, target) in values.iter_mut().zip(captured) {
            let Value::Float(current) = *target.lock().map_err(|_| InputCvarError::Lock)? else {
                return Err(InputCvarError::WrongType);
            };
            *value = current;
        }
        Ok(values)
    }
    /// Wipeout.Enter82d8ec60/64 captures upper then lower target pointers before
    /// either value read82d8ec68/6c. Keep both allocations alive across relinking.
    pub(crate) fn try_wipeout_up_velocity_scales(&self) -> Result<(f32, f32), InputCvarError> {
        let (upper, lower) = {
            let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
            let upper = targets[33]
                .as_ref()
                .ok_or(InputCvarError::Unlinked("kUpVelocityScaleUpper"))?
                .clone();
            let lower = targets[32]
                .as_ref()
                .ok_or(InputCvarError::Unlinked("kUpVelocityScaleLower"))?
                .clone();
            (upper, lower)
        };
        let Value::Float(upper_value) = *upper.lock().map_err(|_| InputCvarError::Lock)? else {
            return Err(InputCvarError::WrongType);
        };
        let Value::Float(lower_value) = *lower.lock().map_err(|_| InputCvarError::Lock)? else {
            return Err(InputCvarError::WrongType);
        };
        Ok((upper_value, lower_value))
    }
    /// Initializer82f8fad0/table82fd5790. Range0..1 is GUI metadata only.
    pub(crate) fn try_up_velocity_scale_lower(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(32)
    }
    /// Initializer82f8fb30/table82fd5794, exact default bits3dcccccd.
    pub(crate) fn try_up_velocity_scale_upper(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(33)
    }
    /// Initializer82f8fb90/table82fd5798. Range1..100 is GUI metadata only.
    pub(crate) fn try_max_skeleton_part_velocity(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(34)
    }
    /// Reference8311e9f0, initializer82fa0970/table82fd6494. The Grinds
    /// conditioner samples the current linked integer before its +1 store.
    pub(crate) fn try_min_approach_non_grind_frames(&self) -> Result<i32, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        let target = targets[31]
            .as_ref()
            .ok_or(InputCvarError::Unlinked("kMinApproachNonGrindFrames"))?;
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Integer { value, .. } => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
    /// References8308d880/8308da34; separate fresh reads in soft-drive setup.
    /// Initializers82f7ba80/bae0 (table82fd4480/4484) set defaults20000/10.
    pub(crate) fn try_skateboard_soft_drive_disp(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(28)
    }
    pub(crate) fn try_skateboard_soft_drive_strength(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(29)
    }
    /// Initializer82f8e968 at table82fd56cc; sampled late in Air.Enter.
    pub(crate) fn try_draw_physics_air_debug_info(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(30)
    }
    /// OffBoard.Enter82d9c2d8 samples the linked target only when last != 504.
    pub(crate) fn try_ob_max_landing_speed_for_blend(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(346)
    }
    /// OffBoard CheckForWipeoutTriggers82d91984 compares it against the
    /// effective-animation up row's Y lane.
    pub(crate) fn try_ob_min_up_vector_y(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(547)
    }
    /// SlowMotionController Begin82ca8eb4/Update82ca90f8 scale the timescale graph.
    pub(crate) fn try_slow_motion_scale(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(548)
    }
    /// SetOffBoardCollision82b4f0a8: LightDMO routing for the six limb Parts.
    pub(crate) fn try_skeleton_ob_use_light_dmo_group(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(549)
    }
    /// SetOffBoardCollision82b4f108: normal collision for the eight body Parts.
    pub(crate) fn try_skeleton_ob_have_body_collision(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(550)
    }
    /// SetOffBoardCollision82b4f188: core Parts collide with LW, else LightDMO.
    pub(crate) fn try_skeleton_ob_core_collide_with_lw(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(551)
    }
    /// Update5 82e05914..5954; initializers82f9ec48..ed68.
    pub(crate) fn try_board_mount_thresholds(
        &self,
        air: bool,
    ) -> Result<(f32, f32), InputCvarError> {
        let linear = self.try_current_float(535)?;
        let angular = self.try_current_float(536)?;
        if air {
            Ok((self.try_current_float(537)?, self.try_current_float(538)?))
        } else {
            Ok((linear, angular))
        }
    }
    fn try_current_float(&self, index: usize) -> Result<f32, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        let target = targets[index]
            .as_ref()
            .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))?;
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Float(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
    /// `GetBlendCullThresh82c8b6d8` selects exactly one live reference from the
    /// actual player flag. Initializers82f80cc8/d28 both have NULL callbacks.
    pub(crate) fn try_skater_blend_cull_threshold(&self, low: bool) -> Result<f32, InputCvarError> {
        let index = if low { 26 } else { 27 };
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        let target = targets[index]
            .as_ref()
            .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))?;
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Float(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }

    /// Reference8308e738, initializer82f7d538; fresh target at ragdoll setup.
    pub(crate) fn try_use_ragdoll_joint_limits(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(23)
    }

    /// Reference8308ef14, initializer82f7d3c8; sampled after joint limits.
    pub(crate) fn try_bouncy_ragdoll_cheat(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(24)
    }

    /// Reference8308e424, initializer82f7d418. Source range0..1 is GUI metadata;
    /// startup Deserialize does not clamp the current target or invoke a callback.
    pub(crate) fn try_bouncy_ragdoll_restitution(&self) -> Result<f32, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        let target = targets[25]
            .as_ref()
            .ok_or(InputCvarError::Unlinked("kBouncyRagdollCheatRestitution"))?;
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Float(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }

    /// Reference830dc914, initializer82f9bfd0; conditional `StateChanger` read.
    pub(crate) fn try_disable_skitching(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(22)
    }

    /// Reference830dd6a0, initializer82f9be60; source target current integer+d8.
    pub(crate) fn try_fall_through_world_reset_frames(&self) -> Result<i32, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        let target = targets[19]
            .as_ref()
            .ok_or(InputCvarError::Unlinked("kFallThroughWorldResetFrames"))?;
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Integer { value, .. } => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }

    /// `InitPlayer826000b0` rereads this reference for each generated graph.
    pub(crate) fn try_use_generated_state_graph(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(20)
    }

    /// `AdjustSpawnPosition`825f1a20 reads the linked current boolean when offline.
    pub(crate) fn try_enable_adjust_spawn_position(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(342)
    }

    /// Reference8304808c; `InitPlayer` tests this before enabling either graph.
    pub(crate) fn try_test_determinism(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(21)
    }

    fn try_current_boolean(&self, index: usize) -> Result<bool, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        let target = targets[index]
            .as_ref()
            .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))?;
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Boolean(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }

    /// Reference constructors run before manager: defaults exist, mTarget remains NULL.
    pub(crate) fn new_static_initialized() -> Self {
        Self {
            targets: Arc::new(Mutex::new(std::array::from_fn(|_| None))),
            gesture_copies: Arc::new(Mutex::new(None)),
        }
    }
    pub(crate) fn fill_provider(&self) -> InputCvarProvider {
        InputCvarProvider {
            references: self.clone(),
            captured: std::array::from_fn(|_| None),
        }
    }
}
struct Label {
    definition: Definition,
    target: Target,
}

impl crate::simulation_assembly_backend::constraint_session::system_logic::StateChangerCvars
    for InputCvarReferences
{
    type Error = InputCvarError;
    fn fall_through_world_reset_frames(&mut self) -> Result<i32, Self::Error> {
        self.try_fall_through_world_reset_frames()
    }
    fn disable_skitching(&mut self) -> Result<bool, Self::Error> {
        self.try_disable_skitching()
    }
    fn minimum_air_time(&mut self) -> Result<f32, Self::Error> {
        self.try_state_changer_min_air_time()
    }
    fn grind_jump_air_frames(&mut self) -> Result<i32, Self::Error> {
        self.try_grind_jump_air_frames()
    }
    fn known_air_warning_time(&mut self) -> Result<f32, Self::Error> {
        self.try_current_float(531)
    }
    fn state_debug_warnings(&mut self) -> Result<bool, Self::Error> {
        self.try_current_boolean(532)
    }
    fn nonspecific_frames(
        &mut self,
        which: crate::simulation_assembly_backend::constraint_session::system_logic::NonspecificFrames,
    ) -> Result<i32, Self::Error> {
        use crate::simulation_assembly_backend::constraint_session::system_logic::NonspecificFrames as F;
        use grind_states::GrindCvar as C;
        self.try_grind_int(match which {
            F::ToGroundNearGrind => C::kNonspecificToGroundNearGrindFrames,
            F::CollFreeSticky => C::kNonspecificCollFreeStickyFrames,
            F::CollExit => C::kNonspecificCollExitFrames,
        })
    }
}
pub(crate) struct InputCvarManager {
    labels: Vec<Label>,
}
impl InputCvarManager {
    pub(crate) fn new() -> Self {
        Self { labels: Vec::new() }
    }
    /// `cVarMan::Init` visits head-inserted static references in reverse order.
    /// Link reuses existing section+label targets without resetting current values.
    pub(crate) fn try_link(
        &mut self,
        references: &InputCvarReferences,
    ) -> Result<(), InputCvarError> {
        let mut refs = references
            .targets
            .lock()
            .map_err(|_| InputCvarError::Lock)?;
        for index in REGISTRATION_ORDER.into_iter().rev() {
            let definition = DEFINITIONS[index];
            let target = if let Some(label) = self.labels.iter().find(|entry| {
                entry.definition.section == definition.section
                    && entry.definition.label == definition.label
            }) {
                Arc::clone(&label.target)
            } else {
                let target = Arc::new(Mutex::new(definition.initial));
                self.labels.push(Label {
                    definition,
                    target: Arc::clone(&target),
                });
                target
            };
            refs[index] = Some(target);
        }
        // Source sections sort by name, labels by title (equal titles insert after).
        self.labels
            .sort_by_key(|label| (label.definition.section, label.definition.title));
        Ok(())
    }
    /// Deserialize82a8ad50 after actual boot file admission8272e490.
    /// None is `FileLoader`'s actual load failure; some bytes are parsed, never ignored.
    /// Only these selected registered labels are owned by this bounded manager.
    pub(crate) fn try_deserialize_startup(
        &mut self,
        bytes: Option<&[u8]>,
    ) -> Result<(), InputCvarError> {
        let Some(bytes) = bytes else {
            return Ok(());
        };
        let file = ini::parse(bytes)?;
        for label in &self.labels {
            if let Some(bytes) = file.get(label.definition.label.as_bytes()) {
                let value = match label.definition.initial {
                    Value::Boolean(_) => Value::Boolean(ini::integer(bytes) != 0),
                    Value::Float(_) => Value::Float(ini::float(bytes)),
                    Value::Integer { .. } => {
                        let mut target = label.target.lock().map_err(|_| InputCvarError::Lock)?;
                        let Value::Integer { value, .. } = &mut *target else {
                            return Err(InputCvarError::WrongType);
                        };
                        *value = ini::integer(bytes);
                        continue;
                    }
                    Value::Enumeration { tags, .. } => Value::Enumeration {
                        // ReadLabel82a8a5a8 uses GetInt even for enums: no tag lookup/clamp.
                        value: ini::integer(bytes),
                        tags,
                    },
                    Value::Vector2(_) => {
                        let Some(xy) = ini::vector2(bytes) else {
                            continue;
                        };
                        let mut target = label.target.lock().map_err(|_| InputCvarError::Lock)?;
                        let Value::Vector2(current) = &mut *target else {
                            return Err(InputCvarError::WrongType);
                        };
                        current[0] = xy[0];
                        current[1] = xy[1];
                        continue;
                    }
                };
                // ReadLabel stores current only; no min/max clamp or callback.
                *label.target.lock().map_err(|_| InputCvarError::Lock)? = value;
            }
        }
        Ok(())
    }
}
pub(crate) struct InputCvarProvider {
    references: InputCvarReferences,
    captured: [Option<Target>; CVAR_COUNT],
}
fn float_index(key: ActionInputFloatCvar) -> usize {
    match key {
        ActionInputFloatCvar::NewPushAllowance => 4,
        ActionInputFloatCvar::TimeToConsiderWipeout => 3,
        ActionInputFloatCvar::NewToggleOffboardAllowance => 5,
    }
}
fn boolean_index(key: ActionInputBoolCvar) -> usize {
    match key {
        ActionInputBoolCvar::UseTriggersForExplicitWipeout => 1,
        ActionInputBoolCvar::UseReleaseForWipeout => 2,
        ActionInputBoolCvar::UseSprintButtonInput => 0,
    }
}
impl InputCvarProvider {
    fn capture(&mut self, index: usize) -> Result<(), InputCvarError> {
        let refs = self
            .references
            .targets
            .lock()
            .map_err(|_| InputCvarError::Lock)?;
        self.captured[index] = Some(Arc::clone(
            refs[index]
                .as_ref()
                .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))?,
        ));
        Ok(())
    }
    fn read(&self, index: usize) -> Result<Value, InputCvarError> {
        let target = self.captured[index]
            .as_ref()
            .ok_or(InputCvarError::Uncaptured(DEFINITIONS[index].label))?;
        Ok(*target.lock().map_err(|_| InputCvarError::Lock)?)
    }
}
impl ActionInputFillCvars for InputCvarProvider {
    type Error = InputCvarError;
    fn capture_float(&mut self, key: ActionInputFloatCvar) -> Result<(), Self::Error> {
        self.capture(float_index(key))
    }
    fn capture_boolean(&mut self, key: ActionInputBoolCvar) -> Result<(), Self::Error> {
        self.capture(boolean_index(key))
    }
    fn float(&mut self, key: ActionInputFloatCvar) -> Result<f32, Self::Error> {
        match self.read(float_index(key))? {
            Value::Float(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
    fn boolean(&mut self, key: ActionInputBoolCvar) -> Result<bool, Self::Error> {
        match self.read(boolean_index(key))? {
            Value::Boolean(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
}

impl std::fmt::Display for InputCvarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lock => f.write_str("input cvar owner lock poisoned"),
            Self::Unlinked(label) => write!(f, "input cvar target not linked: {label}"),
            Self::Uncaptured(label) => write!(f, "input cvar target not captured: {label}"),
            Self::WrongType => f.write_str("input cvar target has wrong type"),
            Self::IniFieldTooLong => {
                f.write_str("INI field exceeds selected terminated 255-byte buffer support")
            }
        }
    }
}

/// Captured cVarBoolRef target, with current value at +CD.
pub(crate) struct InputCvarBooleanTarget(Target);
impl InputCvarBooleanTarget {
    pub(crate) fn try_value(&self) -> Result<bool, InputCvarError> {
        match *self.0.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Boolean(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
}
pub(crate) struct InputCvarFloatTarget(Target);
impl InputCvarFloatTarget {
    pub(crate) fn try_value(&self) -> Result<f32, InputCvarError> {
        match *self.0.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Float(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
}
impl crate::simulation_assembly_backend::constraint_session::physical_skateboard_controller::SkateboardControllerCvars for InputCvarReferences {
    type Error = InputCvarError;
    type Target = InputCvarBooleanTarget;
    fn capture_use_hand_drives(&mut self) -> Result<Self::Target, Self::Error> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        Ok(InputCvarBooleanTarget(Arc::clone(targets[6].as_ref().ok_or(InputCvarError::Unlinked("kSKC_UseHandDrives"))?)))
    }
    fn use_hand_drives(&mut self, target: &Self::Target) -> Result<bool, Self::Error> {
        target.try_value()
    }
}

impl InputCvarReferences {
    /// Reference830480b0, initializer82f56ae8; rereads the current linked target.
    pub(crate) fn try_use_jobs_for_skater_anim_update(&self) -> Result<bool, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        let target = targets[18]
            .as_ref()
            .ok_or(InputCvarError::Unlinked("kUseJobsForSkaterAnimUpdate"))?;
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Boolean(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }

    /// Actual `kUseVBR8305904c`; each source call rereads the reference and target.
    pub(crate) fn try_use_vbr(&self) -> Result<bool, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        let target = targets[16]
            .as_ref()
            .ok_or(InputCvarError::Unlinked("kUseVBR"))?;
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Boolean(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
}

impl InputCvarReferences {
    /// `kShowVelRemoval830d7584`, initializer82f8efd8; source current bool read.
    pub(crate) fn try_show_vel_removal(&self) -> Result<bool, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        let target = targets[17]
            .as_ref()
            .ok_or(InputCvarError::Unlinked("kShowVelRemoval"))?;
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Boolean(value) => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
}

impl InputCvarReferences {
    pub(crate) fn try_draw_jump_debug_info(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(79)
    }
    /// Native reference8311e518 is sampled only in the Ground-to-Air hysteresis branch.
    pub(crate) fn try_ground_stairs_frames(&self) -> Result<i32, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        let target = targets[80]
            .as_ref()
            .ok_or(InputCvarError::Unlinked("kGroundStairsFrames"))?;
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Integer { value, .. } => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
}

impl InputCvarReferences {
    pub(crate) fn try_draw_vert_turning_input(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(81)
    }
}

impl InputCvarReferences {
    pub(crate) fn try_max_grind_illegal_frames(&self) -> Result<i32, InputCvarError> {
        let targets = self.targets.lock().map_err(|_| InputCvarError::Lock)?;
        let target = targets[82]
            .as_ref()
            .ok_or(InputCvarError::Unlinked("kMaxGrindIllegalFrames"))?;
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Integer { value, .. } => Ok(value),
            _ => Err(InputCvarError::WrongType),
        }
    }
}

impl InputCvarReferences {
    pub(crate) fn try_spew_jump_stuff(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(83)
    }
}

impl InputCvarReferences {
    /// Ref8308dd8c sampled late in board output after velocity-direction debug.
    pub(crate) fn try_skater_look_direction(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(84)
    }
    /// Ref8308d94c sampled only after both board direction stores.
    pub(crate) fn try_skateboard_vel_direction(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(85)
    }
}

#[path = "input_cvar_reckoning.rs"]
pub(crate) mod reckoning;

#[path = "input_cvar_wallride.rs"]
pub(crate) mod wallride;

#[path = "input_cvar_collision_response.rs"]
mod collision_response;
#[path = "input_cvar_fake_torque.rs"]
pub(crate) mod fake_torque;

#[path = "input_cvar_pumping_forces.rs"]
pub(crate) mod pumping_forces;

#[path = "input_cvar_speed_model.rs"]
pub(crate) mod speed_model;

#[path = "input_cvar_ground_foot_force.rs"]
pub(crate) mod ground_foot_force;

#[path = "input_cvar_antiflip_torque.rs"]
pub(crate) mod antiflip_torque;

#[path = "input_cvar_heading_torque.rs"]
pub(crate) mod heading_torque;

#[path = "../../physics/skeleton/postphysics/skeleton_postphysics_history_input_live.rs"]
pub(crate) mod postphysics_history_input;

#[path = "input_cvar_skeleton_postphysics_live.rs"]
pub(crate) mod skeleton_postphysics_live;

#[path = "input_cvar_manual_effect.rs"]
pub(crate) mod manual_effect;

#[path = "input_cvar_antiflip_nudge.rs"]
pub(crate) mod antiflip_nudge;

impl InputCvarReferences {
    pub(crate) fn try_draw_contact_info(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(129)
    }
    pub(crate) fn try_draw_audio_collision_info(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(130)
    }
}

#[path = "input_cvar_ground_hang_pinning.rs"]
pub(crate) mod ground_hang_pinning;

#[path = "input_cvar_board_post_live.rs"]
pub(crate) mod board_post_live;

#[path = "input_cvar_board_post_outer.rs"]
pub(crate) mod board_post_outer;

#[path = "input_cvar_wipeout_decision.rs"]
pub(crate) mod wipeout_decision;

impl InputCvarReferences {
    pub(crate) fn try_enable_ghost_by_default(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(191)
    }
    pub(crate) fn try_debug_render_local_sim_player(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(192)
    }
}

#[path = "input_cvar_air_jump.rs"]
pub(crate) mod air_jump;

#[path = "input_cvar_air_com_launch.rs"]
pub(crate) mod air_com_launch;

#[path = "input_cvar_selector_scoring.rs"]
pub(crate) mod selector_scoring;

#[path = "input_cvar_ragdoll_drives.rs"]
pub(crate) mod ragdoll_drives;

#[path = "input_cvar_wipeout_control.rs"]
pub(crate) mod wipeout_control;

#[path = "input_cvar_physical_constructor.rs"]
pub(crate) mod physical_constructor;

impl InputCvarReferences {
    pub(crate) fn try_motion_use_bump(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(279)
    }
}

#[path = "input_cvar_pad_vibration.rs"]
pub(crate) mod pad_vibration;

/// Existing `ConvertAToI82bc3ce0` reused by Juice's distinct INI grammar.
pub(crate) fn native_ini_integer(bytes: &[u8]) -> i32 {
    ini::integer(bytes)
}
impl InputCvarReferences {
    /// `JuiceManager::Start826e36d0` samples this target before the enabled gate.
    pub(crate) fn try_juice_connection_retry(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(294)
    }
}

impl InputCvarReferences {
    pub(crate) fn try_enable_ropa(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(296)
    }
}

impl InputCvarReferences {
    pub(crate) fn try_skater_pool_parallel(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(297)
    }
    pub(crate) fn try_bouncy_big_black_enabled(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(298)
    }
}

impl InputCvarReferences {
    /// `Island.Step_UpdatePrePhysics1`: `kMaxSinglePlayerIterations` for a single player island,
    /// else `kMaxOnlineIterations`.
    pub(crate) fn try_island_max_iterations(
        &self,
        single_player: bool,
    ) -> Result<i32, InputCvarError> {
        self.try_current_integer(if single_player { 299 } else { 300 })
    }
}

impl InputCvarReferences {
    fn try_query_target(&self, index: usize) -> Result<Target, InputCvarError> {
        self.targets.lock().map_err(|_| InputCvarError::Lock)?[index]
            .clone()
            .ok_or(InputCvarError::Unlinked(DEFINITIONS[index].label))
    }
    fn try_query_value(target: &Target) -> Result<u32, InputCvarError> {
        match *target.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Integer { value, .. } => Ok(u32::from_ne_bytes(value.to_ne_bytes())),
            _ => Err(InputCvarError::WrongType),
        }
    }
    pub(crate) fn try_ground_query_counts(&self) -> Result<[u32; 4], InputCvarError> {
        // World827b19ac..19f8: captures and scalar reads are distinct operations.
        let line = self.try_query_target(301)?;
        let boxes = self.try_query_target(302)?;
        let trajectory = self.try_query_target(304)?;
        let line = Self::try_query_value(&line)?;
        let boxes = Self::try_query_value(&boxes)?;
        let tests = self.try_query_target(303)?;
        let trajectory = Self::try_query_value(&trajectory)?.wrapping_add(90);
        let tests = Self::try_query_value(&tests)?;
        Ok([line, boxes, trajectory, tests])
    }
    pub(crate) fn try_dynamic_query_counts(
        &self,
        flags: u32,
        other: u32,
    ) -> Result<[u32; 5], InputCvarError> {
        let (trajectory, step) = if flags & 1 != 0 {
            (
                Self::try_query_value(&self.try_query_target(304)?)?.wrapping_add(90),
                18,
            )
        } else {
            (other, other)
        };
        let line = self.try_query_target(301)?;
        let boxes = self.try_query_target(302)?;
        let tests = self.try_query_target(303)?;
        let line = Self::try_query_value(&line)?;
        let boxes = Self::try_query_value(&boxes)?;
        let tests = Self::try_query_value(&tests)?;
        Ok([line, boxes, trajectory, step, tests])
    }
}

impl InputCvarReferences {
    pub(crate) fn try_ob_air_debug_draw(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(395)
    }
    pub(crate) fn try_ob_air_trajectory_max_time(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(396)
    }
    pub(crate) fn try_ob_air_trajectory_max_error_start(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(397)
    }
    pub(crate) fn try_ob_air_trajectory_max_error_end(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(398)
    }
    pub(crate) fn try_ob_debug_trajectory_fail(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(399)
    }
}

impl InputCvarReferences {
    pub(crate) fn try_step_display_bad_triangles(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(427)
    }
    pub(crate) fn try_step_vertical_normal_y(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(428)
    }
    pub(crate) fn try_step_additional_gap(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(429)
    }
    pub(crate) fn try_step_minimum_bad_y(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(430)
    }
    pub(crate) fn try_step_test_radius(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(431)
    }
    pub(crate) fn try_step_test_height(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(432)
    }
    pub(crate) fn try_step_step_radius(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(433)
    }
    pub(crate) fn try_step_normal_y_threshold(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(434)
    }
    pub(crate) fn try_step_maximum_time(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(435)
    }
    pub(crate) fn try_step_trajectory_radius(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(436)
    }
    pub(crate) fn try_step_sprint_trajectory_count(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(437)
    }
    pub(crate) fn try_step_death_jump_distance(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(438)
    }
    pub(crate) fn try_step_shoe_offset(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(439)
    }
    pub(crate) fn try_step_vertical_sine(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(440)
    }
    pub(crate) fn try_step_reduce_speed_ratio(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(441)
    }
    pub(crate) fn try_step_added_distance_ratio(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(442)
    }
    pub(crate) fn try_step_minimum_distance(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(443)
    }
    pub(crate) fn try_step_cosine_spread(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(444)
    }
    pub(crate) fn try_step_box_half_size(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(445)
    }
    pub(crate) fn try_step_acceptable_distance(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(446)
    }
    pub(crate) fn try_step_height_margin(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(447)
    }
    pub(crate) fn try_step_low_step_height(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(448)
    }
}

#[path = "input_cvar_landing_state.rs"]
pub(crate) mod landing_state;

impl InputCvarReferences {
    pub(crate) fn try_skc_grab_cheat(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(458)
    }
}

impl InputCvarReferences {
    pub(crate) fn try_camera_show_transfer_angle(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(459)
    }
}

impl InputCvarReferences {
    pub(crate) fn try_skeleton_speed_scalar_on_enter_locomotion(
        &self,
    ) -> Result<f32, InputCvarError> {
        self.try_current_float(460)
    }
    pub(crate) fn try_locomotion_max_land_on_board_assist(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(461)
    }
}

impl InputCvarReferences {
    pub(crate) fn try_camera_show_air_collision(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(462)
    }
    pub(crate) fn try_camera_enable_air_collision(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(463)
    }
}

impl InputCvarReferences {
    pub(crate) fn try_loco_correction_angle_sign(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(464)
    }
    pub(crate) fn try_loco_enable_jump_height_adjust(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(465)
    }
    pub(crate) fn try_loco_jump_height(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(466)
    }
    pub(crate) fn try_loco_time_ratio(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(467)
    }
    pub(crate) fn try_loco_display_debug(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(468)
    }
    pub(crate) fn try_capture_loco_second_degree_ratio(
        &self,
    ) -> Result<InputCvarFloatTarget, InputCvarError> {
        self.try_query_target(469).map(InputCvarFloatTarget)
    }
}

impl InputCvarReferences {
    pub(crate) fn try_create_locomotion_sprint_button(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(474)
    }
    pub(crate) fn try_loco_collision_additional_angle(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(470)
    }
    pub(crate) fn try_loco_collision_max_turn_angle(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(471)
    }
    pub(crate) fn try_loco_collision_max_start_angle(
        &self,
        jumping: bool,
    ) -> Result<f32, InputCvarError> {
        self.try_current_float(if jumping { 473 } else { 472 })
    }
    pub(crate) fn try_create_locomotion_maximum_collision_displacement(
        &self,
    ) -> Result<f32, InputCvarError> {
        self.try_current_float(475)
    }
    pub(crate) fn try_create_locomotion_displacement_blend(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(476)
    }
    pub(crate) fn try_create_locomotion_avoidance_duration(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(477)
    }
    pub(crate) fn try_create_locomotion_minimum_displacement(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(478)
    }
    pub(crate) fn try_create_locomotion_maximum_raw_time(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(479)
    }
    pub(crate) fn try_create_locomotion_start_y_offset(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(480)
    }
    pub(crate) fn try_create_locomotion_minimum_apex_height(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(481)
    }
    pub(crate) fn try_create_locomotion_failure_frame_count(&self) -> Result<i32, InputCvarError> {
        self.try_current_integer(482)
    }
    pub(crate) fn try_create_locomotion_minimum_spread_degrees(
        &self,
    ) -> Result<f32, InputCvarError> {
        self.try_current_float(483)
    }
    pub(crate) fn try_create_locomotion_maximum_spread_degrees(
        &self,
    ) -> Result<f32, InputCvarError> {
        self.try_current_float(484)
    }
    pub(crate) fn try_create_locomotion_jump_maximum_step_height(
        &self,
    ) -> Result<f32, InputCvarError> {
        self.try_current_float(485)
    }
    pub(crate) fn try_create_locomotion_run_maximum_step_height(
        &self,
    ) -> Result<f32, InputCvarError> {
        self.try_current_float(486)
    }
}

impl InputCvarReferences {
    pub(crate) fn try_loco_update_oscillation_threshold(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(487)
    }
    pub(crate) fn try_loco_update_future_assert(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(488)
    }
    pub(crate) fn try_capture_loco_update_max_decel_step_ratio(
        &self,
    ) -> Result<InputCvarFloatTarget, InputCvarError> {
        self.try_query_target(489).map(InputCvarFloatTarget)
    }
    pub(crate) fn try_loco_update_correction_turn_sign(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(490)
    }
    pub(crate) fn try_loco_update_max_decel_stop(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(491)
    }
    pub(crate) fn try_loco_update_max_decel(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(492)
    }
    pub(crate) fn try_loco_update_max_accel(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(493)
    }
    pub(crate) fn try_loco_update_general_debug(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(494)
    }
}

impl InputCvarReferences {
    pub(crate) fn try_loco_action_initial_thresholds(&self) -> Result<[f32; 4], InputCvarError> {
        //82c8aec4..aed4 capture all targets before aed8..aee4 value loads.
        let angle = InputCvarFloatTarget(self.try_query_target(495)?);
        let stairs = InputCvarFloatTarget(self.try_query_target(496)?);
        let incline = InputCvarFloatTarget(self.try_query_target(497)?);
        let jump = InputCvarFloatTarget(self.try_query_target(498)?);
        Ok([
            angle.try_value()?,
            stairs.try_value()?,
            incline.try_value()?,
            jump.try_value()?,
        ])
    }
    pub(crate) fn try_loco_action_threshold(
        &self,
        threshold: crate::actor_animation_construction::locomotion_action::ActionThreshold,
    ) -> Result<f32, InputCvarError> {
        use crate::actor_animation_construction::locomotion_action::ActionThreshold as C;
        self.try_current_float(match threshold {
            C::LandTwoFeet => 499,
            C::InclineDistance => 500,
            C::StairsDistance => 501,
            C::InclineMaxDown => 502,
            C::InclineMaxUp => 503,
            C::JumpStairsDistance => 504,
            C::LandOneFoot => 505,
        })
    }
}

impl InputCvarReferences {
    pub(crate) fn try_two_foot_landing_impact_threshold(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(506)
    }
}

impl InputCvarReferences {
    pub(crate) fn try_debug_loco_common_conditions(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(517)
    }
    pub(crate) fn try_debug_loco_motion_conditions(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(518)
    }
}

#[path = "input_cvar_handplant.rs"]
pub(crate) mod handplant;

impl InputCvarReferences {
    pub(crate) fn try_pedestrian_moods_suppressed(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1189)}
    pub(crate) fn try_pedestrian_enable_head_tracking(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1190)}
}

impl InputCvarReferences {
 pub(crate) fn try_all_mood_behaviour_suppressed(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1199)}
 pub(crate) fn try_show_mood_random_rolls(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1200)}
}

impl InputCvarReferences {
 pub(crate) fn try_pedestrian_hom_slam_threshold(&self)->Result<i32,InputCvarError>{self.try_current_integer(1204)}
 pub(crate) fn try_jacked_timer(&self)->Result<f32,InputCvarError>{self.try_current_float(1205)}
 pub(crate) fn try_show_jacked_overlay(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1206)}
 pub(crate) fn try_in_front_of_waypoint_angle(&self)->Result<f32,InputCvarError>{self.try_current_float(1207)}
}

impl InputCvarReferences { pub(crate) fn try_plugin_unlock_timer(&self)->Result<f32,InputCvarError>{self.try_current_float(1208)} }
impl InputCvarReferences { pub(crate) fn try_spectate_max_delay(&self)->Result<f32,InputCvarError>{self.try_current_float(1209)} }
impl InputCvarReferences { pub(crate) fn try_plugin_dmo_shift_tolerance(&self)->Result<f32,InputCvarError>{self.try_current_float(1210)} }
impl InputCvarReferences { pub(crate) fn try_min_plugin_timer(&self)->Result<f32,InputCvarError>{self.try_current_float(1211)} }
impl InputCvarReferences { pub(crate) fn try_enable_actor_tracker(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1212)} }
impl InputCvarReferences {
    pub(crate) fn try_enable_pedestrian_face_anim(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1213)}
    pub(crate) fn try_enable_pedestrian_hand_anim(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1214)}
}

impl InputCvarReferences {
    pub(crate) fn try_navigating_head_tracking_angle(&self)->Result<f32,InputCvarError>{self.try_current_float(1215)}
    pub(crate) fn try_navigating_head_tracking_predict(&self)->Result<f32,InputCvarError>{self.try_current_float(1216)}
    pub(crate) fn try_hand_prop_attack_perturb_radius(&self)->Result<f32,InputCvarError>{self.try_current_float(1217)}
    pub(crate) fn try_hand_prop_attack_y_offset(&self)->Result<f32,InputCvarError>{self.try_current_float(1218)}
    pub(crate) fn try_greet_interest_timer(&self)->Result<f32,InputCvarError>{self.try_current_float(1219)}
    pub(crate) fn try_chase_intercept_y_tolerance(&self)->Result<f32,InputCvarError>{self.try_current_float(1220)}
    pub(crate) fn try_enable_marquee_face_anim(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1221)}
    pub(crate) fn try_enable_marquee_hand_anim(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1222)}
    pub(crate) fn try_takedown_y_tolerance(&self)->Result<f32,InputCvarError>{self.try_current_float(1223)}
    pub(crate) fn try_dynamic_object_execution_rates(&self)->Result<[u32;3],InputCvarError>{
        Ok([self.try_current_integer(1224)? as u32,self.try_current_integer(1225)? as u32,self.try_current_integer(1226)? as u32])
    }
    pub(crate) fn try_dmo_reset_capsule_tuning(&self)->Result<[f32;3],InputCvarError>{
        Ok([self.try_current_float(1228)?,self.try_current_float(1229)?,self.try_current_float(1230)?])
    }
    pub(crate) fn try_contained_waypoint_y_tolerance(&self)->Result<f32,InputCvarError>{self.try_current_float(1227)}
}

impl InputCvarReferences { pub(crate) fn try_dynamic_object_upright_angle(&self)->Result<f32,InputCvarError>{self.try_current_float(1249)} }

impl InputCvarReferences {
    pub(crate) fn try_hom_slow_motion_enabled(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1250)}
    pub(crate) fn try_hom_animation_time(&self)->Result<f32,InputCvarError>{self.try_current_float(1251)}
    pub(crate) fn try_hom_challenge_enabled(&self)->Result<bool,InputCvarError>{self.try_current_boolean(1252)}
}
