//! Physical publications consumed by riding animation, TU3 82DB6EC0.
//!
//! This receives the actual board, Pumping, SpeedWobble and Reckoning results.
//! It does not reconstruct the skater frame from the deck or rendered pose.
use crate::{
    animation::{crouching, ground_acceleration},
    input::{set_turning, turn_conditioner},
    math::Vector3,
    physics::{board_motion_output::BoardMotionOutput, native_arithmetic},
    riding::{pumping::state::PumpingState, speed_wobble::SpeedWobbleState},
};

/// The completed Reckoning owner's output required by animation publication.
/// There is intentionally no default: these are distinct physical frames.
#[derive(Clone, Copy, Debug)]
pub struct ReckoningFeedback {
    /// PhysOutSystemReckoning+64.
    pub system_position: Vector3,
    /// Reckoning+1152 -> PhysOutSystemReckoning+96 at82DB7068.
    pub system_up: Vector3,
    /// PhysOutDeck+48.
    pub board_position: Vector3,
    /// Reckoning+1576, before the Processed2468 bit20 sign correction.
    pub target_lean_angle: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct ControlFeedback {
    /// The complete ProcessedPhysIn+2468 flag word.
    pub processed_flags: u32,
    /// ProcessedPhysIn+2676 -> PhysOutGround+264.
    pub turn: f32,
    /// AnimOutPhysIn+10370 -> PhysOutAnimation+156.
    pub animation_mirrored: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct PhysicalFeedback {
    pub turning: set_turning::Physical,
    pub crouching: crouching::Physical,
    pub pumping_acceleration: f32,
    /// PhysOutAnimation112, conditioned actual deck acceleration82DF0508.
    pub ground_acceleration: [f32; 4],
    /// Original IsBumped82BA7310 evaluated with anim_motion/bumps.
    pub bumped: bool,
    /// Complete PhysOutAnimation32..60, retained for other native consumers.
    pub conditioned_turn: turn_conditioner::Output,
}

/// Postphysics publication followed by the animation turn conditioner82DEFF38.
/// Stateful filtering happens once per conditioner update, without a dt factor.
pub fn publish(
    state: &mut turn_conditioner::State,
    settings: &turn_conditioner::Settings,
    motion: &BoardMotionOutput,
    pumping: &PumpingState,
    wobble: &SpeedWobbleState,
    reckoning: ReckoningFeedback,
    controls: ControlFeedback,
    acceleration: ground_acceleration::Output,
) -> PhysicalFeedback {
    let lean = if controls.processed_flags & 0x0010_0000 != 0 {
        reckoning.target_lean_angle
    } else {
        -reckoning.target_lean_angle
    };
    let turn = turn_conditioner::update(
        state,
        turn_conditioner::Input {
            body_160: motion.speed,
            // Skateboard::FillPhysOut82C03230..3238 reads Wobble+36.
            body_176: f32::from_bits(wobble.0[5]),
            bundle_36_field_160: lean,
            bundle_32_field_264: controls.turn,
            animation_152: u8::from(controls.processed_flags & 0x8000_0000 != 0),
            animation_156: u8::from(controls.animation_mirrored),
        },
        settings,
    );
    let system = vector(reckoning.system_position);
    let board = vector(reckoning.board_position);
    //82DB7A38..7A54: vsubfp followed by vmsum3fp. The isolated native
    // arithmetic retains its documented host approximation boundary.
    let difference = core::array::from_fn(|lane| {
        flush_subnormal(flush_subnormal(system[lane]) - flush_subnormal(board[lane]))
    });
    let height = native_arithmetic::dot3(difference, vector(reckoning.system_up));
    PhysicalFeedback {
        turning: set_turning::Physical {
            field_32: turn[0],
            field_36: turn[1],
            field_52: turn[5],
            field_56: turn[6],
            field_60: turn[7],
            body_168: motion.forward_speed,
        },
        crouching: crouching::Physical {
            body_84: motion.linear_velocity.y,
            body_164: motion.ground_speed,
            //82C02CF0 copies Pumping+44, named mPumpPotential in S2 FillPhysOut.
            body_188: pumping.pumping,
            force_516: pumping.absorption,
            ground_force_520: pumping.ground_normal_absorption,
            minimum_crouch_528: pumping.minimum_crouch,
            deck_angle_532: pumping.deck_angle_absorption,
            animation_height_72: height,
        },
        pumping_acceleration: pumping.pump_acceleration,
        ground_acceleration: acceleration.acceleration,
        bumped: acceleration.bumped,
        conditioned_turn: turn,
    }
}

fn vector(value: Vector3) -> [f32; 4] {
    [value.x, value.y, value.z, 0.0]
}

fn flush_subnormal(value: f32) -> f32 {
    if value.to_bits() & 0x7f80_0000 == 0 {
        f32::from_bits(value.to_bits() & 0x8000_0000)
    } else {
        value
    }
}
