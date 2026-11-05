//! Remaining scalar fields and flags82BDB6A4..82BDC4B4. Values retain their
//! preceding ProcessData/reset state until the corresponding attribute writes.
use super::scalar_attributes::{AnimationControlOutput, ScalarAttributeInputs};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExtendedAttributes {
    pub flags2480: u32,
    /// Processed2796..2820.
    pub grind_translation: f32,
    pub grind_stability_nudge: f32,
    pub grind_up_down: f32,
    pub grind_grab_min_height: f32,
    pub physical_body_spin: f32,
    pub world_grab_y: f32,
    pub world_grab_z: f32,
    /// Processed2680..2692,2704..2708,2784..2792.
    pub offboard_turn: f32,
    pub offboard_magnitude: f32,
    pub biped_world_x: f32,
    pub biped_world_z: f32,
    pub look_x: f32,
    pub look_y: f32,
    pub object_move_z: f32,
    pub object_move_x: f32,
    pub object_move_rotation: f32,
    /// Processed2824/2828,2840/2844,2916/2920,2928/2932.
    pub wipeout_control: [f32; 2],
    pub offboard_jump: [f32; 2],
    pub wipeout_gesture: [f32; 2],
    pub body_adjust: [f32; 2],
    /// Processed2936/2940/2944; AnimOut24.
    pub biped_start_angle: f32,
    pub biped_spin_angle: f32,
    pub biped_animation_time: f32,
    pub footstep_strength: f32,
    /// Processed2624/2628/2632/2744.
    pub jump_strength: f32,
    pub jump_controls: [f32; 2],
    pub revert_direction: f32,
}

impl ExtendedAttributes {
    /// Reset82BF9EF0 zeros every represented ProcessedPhysIn field here.
    /// AnimOut24 belongs to another owner and survives this reset.
    pub fn reset(footstep_strength: f32) -> Self {
        Self {
            flags2480: 0,
            grind_translation: 0.0,
            grind_stability_nudge: 0.0,
            grind_up_down: 0.0,
            grind_grab_min_height: 0.0,
            physical_body_spin: 0.0,
            world_grab_y: 0.0,
            world_grab_z: 0.0,
            offboard_turn: 0.0,
            offboard_magnitude: 0.0,
            biped_world_x: 0.0,
            biped_world_z: 0.0,
            look_x: 0.0,
            look_y: 0.0,
            object_move_z: 0.0,
            object_move_x: 0.0,
            object_move_rotation: 0.0,
            wipeout_control: [0.0; 2],
            offboard_jump: [0.0; 2],
            wipeout_gesture: [0.0; 2],
            body_adjust: [0.0; 2],
            biped_start_angle: 0.0,
            biped_spin_angle: 0.0,
            biped_animation_time: 0.0,
            footstep_strength,
            jump_strength: 0.0,
            jump_controls: [0.0; 2],
            revert_direction: 0.0,
        }
    }
}

/// Special jump/revert handlers and finalization are in attribute_finalization.
/// Return false only for a name outside this contiguous scalar region.
pub fn dispatch(
    name: &str,
    scalar: impl Fn() -> Result<f32, String>,
    input: &mut ScalarAttributeInputs,
    extra: &mut ExtendedAttributes,
    output: &mut AnimationControlOutput,
) -> Result<bool, String> {
    match name {
        "Wipeout" => input.flags2468 |= 0x40000,
        "jump" => input.flags2468 |= 0x400000,
        "DangerZone" => input.flags2472 |= 0x8000,
        "LateTrick" => input.flags2472 |= 0x20,
        "NoDangerZone" => input.flags2472 |= 0x10,
        "ChristAir" => input.flags2472 |= 0x40,
        "PhysGrindTranslation" => extra.grind_translation = scalar()?,
        "PhysGrindStabilityNudge" => extra.grind_stability_nudge = scalar()?,
        "PhysGrindUpDown" => extra.grind_up_down = scalar()?,
        "PhysGrindGrabMinHeight" => extra.grind_grab_min_height = scalar()?,
        "PhysBodySpin" => extra.physical_body_spin = scalar()?,
        "world_grab_y" => extra.world_grab_y = scalar()?,
        "world_grab_z" => extra.world_grab_z = scalar()?,
        "NewHandPlant" => extra.flags2480 |= 0x20000000,
        "AnimCommittedHandPlant" => extra.flags2480 |= 0x10000000,
        "DisableFeetTargetIK" => extra.flags2480 |= 0x08000000,
        "OB_Jump" => input.flags2476 |= 0x80000,
        "OB_StandingJump" => input.flags2476 |= 0x40000,
        "OB_Sprint" => input.flags2484 |= 0x20000,
        "OB_Turn" => extra.offboard_turn = -scalar()?,
        "OB_Mag" => extra.offboard_magnitude = scalar()?,
        "OB_BipedWorldX" => extra.biped_world_x = scalar()?,
        "OB_BipedWorldZ" => extra.biped_world_z = scalar()?,
        "OB_LookAtX" => extra.look_x = scalar()?,
        "OB_LookAtY" => extra.look_y = scalar()?,
        "OB_ObjectMvZ" => extra.object_move_z = scalar()?,
        "OB_ObjectMvX" => extra.object_move_x = scalar()?,
        "OB_ObjectMvRot" => extra.object_move_rotation = scalar()?,
        "OneFootAir" => extra.flags2480 |= 0x04000000,
        "WipeoutControlX" => extra.wipeout_control[0] = scalar()?,
        "WipeoutControlY" => extra.wipeout_control[1] = scalar()?,
        "OB_Mounting" => extra.flags2480 |= 0x80000,
        "OB_Unmounting" => extra.flags2480 |= 0x40000,
        "OB_Dismount" => extra.flags2480 |= 0x20000,
        "OB_HoldBoard" => input.flags2476 |= 0x4000,
        "OB_ReleaseBoard" => input.flags2476 = (input.flags2476 & !0x3000) | 0x2000,
        "OB_ThrowBoard" => input.flags2476 |= 0x3000,
        "OB_RetrieveBoard" => input.flags2476 |= 0x800,
        "OB_RetrievingBoard" => input.flags2476 |= 0x400,
        "OB_DroppingBoard" => input.flags2476 |= 0x200,
        "BipedBoardOnGround" => input.flags2484 |= 1,
        "IsGroundMount" => input.flags2488 |= 0x80000000,
        "FootJump" => extra.flags2480 |= 0x2000,
        "AnimSkateboard" => extra.flags2480 |= 0x4000,
        "HippyJumping" => extra.flags2480 |= 0x1000,
        "PushTricking" => extra.flags2480 |= 0x800,
        "KickoutDismount" => extra.flags2480 |= 0x100,
        "RunoutDismount" => extra.flags2480 |= 0x80,
        "FixLeftFoot" => extra.flags2480 |= 0x40,
        "FixRightFoot" => extra.flags2480 |= 0x20,
        "DoLandOnBoard" => extra.flags2480 |= 0x10,
        "TransitioningOnOffBoard" => extra.flags2480 |= 4,
        "JumpAboutToTakeOff" => input.flags2484 |= 0x40000000,
        "JumpInProgress" => input.flags2484 |= 0x20000000,
        "OB_JumpX" => extra.offboard_jump[0] = -scalar()?,
        "OB_JumpZ" => extra.offboard_jump[1] = scalar()?,
        "SkaterOffBoard" => input.flags2484 |= 0x08000000,
        "IsDark" => input.flags2484 |= 0x200000,
        "GrindTrick" => input.flags2484 |= 0x100000,
        "Shoving" => input.flags2484 |= 0x80000,
        "ShoveDirection" => input.flags2484 |= 0x40000,
        "BipedStartAngle" => {
            input.flags2484 |= 0x4000;
            extra.biped_start_angle = scalar()?;
        }
        "BipedSpinAngle" => extra.biped_spin_angle = scalar()?,
        "BipedAnimTime" => extra.biped_animation_time = scalar()?,
        "InManualZone" => input.flags2484 |= 0x2000,
        "OB_Toggle" => input.flags2484 |= 0x1000,
        "TrickFromManual" => input.flags2484 |= 0x800,
        "WipeoutGestureStickX" => extra.wipeout_gesture[0] = scalar()?,
        "WipeoutGestureStickY" => extra.wipeout_gesture[1] = scalar()?,
        "StrongWipeoutArms" => input.flags2484 |= 0x20,
        "StrongWipeoutLegs" => input.flags2484 |= 0x10,
        "UnderflipBoardContact" => output.flags |= 0x04000000,
        "AudibleFootStepStrength" => extra.footstep_strength = scalar()?,
        "IsTakeDownByBoard" => output.flags |= 0x02000000,
        "DropInLock" => output.flags |= 0x01000000,
        "DropInRelease" => output.flags |= 0x00800000,
        "OB_DropIn" => {
            output.flags |= 0x00400000;
            input.flags2488 |= 0x10000000;
        }
        "BodyAdjustZ" => extra.body_adjust[1] = scalar()?,
        "BodyAdjustX" => extra.body_adjust[0] = scalar()?,
        "OneFootIntent" => input.flags2488 |= 0x02000000,
        "FootPlanting" => extra.flags2480 |= 0x00800000,
        _ => return Ok(false),
    }
    Ok(true)
}
