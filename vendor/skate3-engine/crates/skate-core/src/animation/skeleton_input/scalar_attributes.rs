//! Complete contiguous scalar handlers82BDACF0..82BDB694, plus the two
//! independently verified scalar PushContact/PushSpeed flag handlers.
//! Input order is the packet's MG-then-tree order; no timing/value threshold
//! filters are added. process_attributes::process supplies the complete ordered
//! dispatch, kind3 events, remaining scalar handlers and finalization.
use super::catalog::{self, ScalarAttribute};
use crate::animation::output::{
    attributes::{AnimationAttribute, AttributeName},
    setup_physics::PhysicsPacketBinding,
};

/// These flag words are shared with other ProcessedPhysIn stages. They must
/// be seeded by the real preceding reset/consumer stages, not zeroed here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScalarAttributeInputs {
    pub flags2468: u32,
    pub flags2472: u32,
    pub flags2476: u32,
    pub flags2484: u32,
    pub flags2488: u32,
    /// Native+2492: left1,right2,up3,down4. Unknown prior enum values survive.
    pub board_adjust: u32,
    /// Native2720,2672,2640,2728,2676 respectively.
    pub balance: f32,
    pub spin: f32,
    pub body_spin: f32,
    pub brake: f32,
    pub turn: f32,
    /// Native2908/2912.
    pub turn_scale: f32,
    pub magnitude_scale: f32,
    /// Native2880/2864; scalar handlers replace XYZ independently, preserving W.
    pub animation_end_com: [f32; 4],
    pub animation_translation: [f32; 4],
    /// Native2896/2904/2900.
    pub animation_time: f32,
    pub animation_physics_blend_seconds: f32,
    pub cadence_end_percent: f32,
    /// Native2712/2716/2732.
    pub raw_turn: f32,
    pub hard_turn: f32,
    pub slide: f32,
}

impl ScalarAttributeInputs {
    /// Represented ProcessedPhysIn fields written by Reset82BF9EF0.
    /// This is the reset stage, not an implicit reset in attribute dispatch.
    pub fn reset(previous_flags2468: u32, previous_flags2488: u32) -> Self {
        Self {
            flags2468: (previous_flags2468 & 8) | 0x2000,
            flags2472: 0,
            flags2476: 0,
            flags2484: 0,
            flags2488: previous_flags2488 & 0x1fffff,
            board_adjust: 0,
            balance: 0.0,
            spin: 0.0,
            body_spin: 0.0,
            brake: 0.0,
            turn: 0.0,
            turn_scale: 1.0,
            magnitude_scale: 1.0,
            animation_end_com: [0.0; 4],
            animation_translation: [0.0; 4],
            animation_time: 0.0,
            animation_physics_blend_seconds: 0.0,
            cadence_end_percent: -1.0,
            raw_turn: 0.0,
            hard_turn: 0.0,
            slide: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimationControlOutput {
    /// AnimOut+0..16: selected Slide/Grind name, copied as five exact words.
    pub grind_name: AttributeName,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DispatchError {
    /// Active kind3 requires node lookup82530D80 even for unknown event names.
    EventConsumerUnavailable {
        name: AttributeName,
    },
    KnownScalarUnavailable {
        attribute: ScalarAttribute,
    },
    UninitializedScalar {
        attribute: ScalarAttribute,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PacketDispatchError {
    pub attribute_index: usize,
    pub cause: DispatchError,
}

/// Dispatch actual packet attributes into their physical consumer fields.
/// On failure earlier attribute effects remain; the failed handler performs no
/// writes here. Success means these supported handlers ran, not that native
/// attribute finalization or Skeleton::ProcessData has completed.
pub fn dispatch_supported_packet(
    binding: &PhysicsPacketBinding<'_>,
    input: &mut ScalarAttributeInputs,
    output: &mut AnimationControlOutput,
) -> Result<(), PacketDispatchError> {
    for (attribute_index, attribute) in binding.packet().attributes.entries().iter().enumerate() {
        dispatch_attribute(attribute, input, output).map_err(|cause| PacketDispatchError {
            attribute_index,
            cause,
        })?;
    }
    Ok(())
}

pub fn dispatch_attribute(
    attribute: &AnimationAttribute,
    input: &mut ScalarAttributeInputs,
    output: &mut AnimationControlOutput,
) -> Result<(), DispatchError> {
    if attribute.kind != 0 {
        if attribute.kind == 3 && attribute.status & 12 != 0 {
            return Err(DispatchError::EventConsumerUnavailable {
                name: attribute.name,
            });
        }
        return Ok(());
    }
    let Some(entry) = catalog::lookup(attribute.name) else {
        return Ok(());
    };
    let scalar = || {
        attribute.payload.0[0]
            .map(f32::from_bits)
            .ok_or(DispatchError::UninitializedScalar { attribute: *entry })
    };
    match entry.name {
        "SlideBoard" | "SlideNose" | "SlideTail" | "Grind5050" | "Grind5_O" | "GrindNose" => {
            output.grind_name = attribute.name
        }
        "GrabWorld" => input.flags2476 |= 1 << 22,
        "BoardAdjustLeft" => input.board_adjust = 1,
        "BoardAdjustRight" => input.board_adjust = 2,
        "BoardAdjustUp" => input.board_adjust = 3,
        "BoardAdjustDown" => input.board_adjust = 4,
        "GrindFacingForwards" => output.flags |= 1 << 31,
        "GrindFacingBackwards" => output.flags &= !(1 << 31),
        "NoInput" => {
            output.flags |= 1 << 30;
            input.flags2472 |= 1 << 23;
        }
        "ExitGrind" => input.flags2476 |= 1 << 29,
        "NewAutoPump" => output.flags |= 1 << 29,
        //82BDBC30..48: scalar value is not tested.
        "PlayerControlledPump" => input.flags2476 |= 2,
        "EnteringCoffin" => {
            input.flags2472 |= 2;
            input.flags2476 |= 1 << 30;
        }
        "InCoffin" => {
            input.flags2472 |= 1;
            input.flags2476 |= 1 << 30;
        }
        "LeavingCoffin" => input.flags2476 |= 0xc000_0000,
        "PlayingLanding" => input.flags2476 |= 1 << 28,
        "Balance" => input.balance = -scalar()?,
        "Spin" => input.spin = -scalar()?,
        "KickTurn" => output.flags |= 1 << 28,
        "Anticipating" => output.flags |= 1 << 27,
        "BodySpin" => input.body_spin = -scalar()?,
        "ManualBrake" => input.flags2468 |= 1 << 29,
        "Brake" => input.brake = scalar()?,
        "Turn" => input.turn = -scalar()?,
        "IsStumbling" => input.flags2484 |= 1 << 10,
        "IsWipeoutPushOffImpulse" => input.flags2484 |= 1 << 8,
        "IsWipeoutPushOff" => input.flags2484 |= 1 << 9,
        "IsMuteRFoot" => input.flags2484 |= 1 << 7,
        "IsMuteLFoot" => input.flags2484 |= 1 << 6,
        "IsForceDownRFoot" => input.flags2484 |= 1 << 3,
        "IsForceDownLFoot" => input.flags2484 |= 1 << 2,
        "TurnScale" => input.turn_scale = scalar()?,
        "MagScale" => input.magnitude_scale = scalar()?,
        "AnimEndCOMX" => input.animation_end_com[0] = scalar()?,
        "AnimEndCOMY" => input.animation_end_com[1] = scalar()?,
        "AnimEndCOMZ" => input.animation_end_com[2] = scalar()?,
        "AnimTransX" => input.animation_translation[0] = scalar()?,
        "AnimTransY" => input.animation_translation[1] = scalar()?,
        "AnimTransZ" => input.animation_translation[2] = scalar()?,
        "AnimTime" => input.animation_time = scalar()?,
        "AnimPhysBlendSec" => input.animation_physics_blend_seconds = scalar()?,
        "IsAnimInterpToEdge" => input.flags2488 |= 1 << 27,
        "CadenceEndPercent" => input.cadence_end_percent = scalar()?,
        "OB_Traj" => input.flags2476 |= 1 << 15,
        "OB_Air" => input.flags2476 |= 1 << 7,
        "Carving" => input.flags2468 |= 1 << 31,
        "RawTurn" => input.raw_turn = -scalar()?,
        "HardTurn" => input.hard_turn = -scalar()?,
        "Slide" => input.slide = -scalar()?,
        "FrontFlip" => input.flags2468 |= 1 << 7,
        "BackFlip" => input.flags2468 |= 1 << 6,
        "Grabbing" => input.flags2468 |= 1 << 5,
        "WantsLeftAirGrab" | "WantsRightAirGrab" => input.flags2468 |= 1 << 4,
        "FlipTrick" => input.flags2472 |= 1 << 22,
        "WipeOutRecover" => input.flags2472 |= 1 << 20,
        "ControlledWipeout" => input.flags2472 |= 1 << 19,
        // Independent scalar markers. Neither reads strength or queues force.
        "PushContact" => input.flags2488 |= 1 << 29,
        "PushSpeed" => input.flags2488 |= 1 << 23,
        _ => return Err(DispatchError::KnownScalarUnavailable { attribute: *entry }),
    }
    Ok(())
}
