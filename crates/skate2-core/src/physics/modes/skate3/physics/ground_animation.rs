//! GroundAnimation103 lifecycle82D33BF8/33DB8/33C70/34150, original Skate3TU3.
mod board;
mod settings;
pub(super) mod skeleton;
use super::{Skate3PhysicsState, Skate3ControllerState};
pub(crate) use settings::GroundAnimationSettings;
use skate3_core::{
    air::ground_jump::GroundJump,
    math::Vector3,
    player::input_phase::{AirOutputFields, ProcessedPhysicsInput},
};

#[derive(Default)]
pub(crate) struct GroundAnimationRuntime {
    pub jump: GroundJump,          //48..68
    pub launched: bool,            //80, reset at each update
    pub launch_velocity: [f32; 4], //96, selector launch-info2048
}
impl GroundAnimationRuntime {
    ///82D34150 writes only its active fields after the common PhysOut reset.
    pub fn fill(&self, p: &ProcessedPhysicsInput, out: &mut AirOutputFields) {
        if p.flags_2468 & 0x0040_0000 != 0 {
            out.jump_velocity_delta_112 = std::array::from_fn(|i| {
                (self.jump.velocity[i] - f32::from_bits(p.vectors_400_416[0][i])).to_bits()
            });
        }
        if self.launched {
            out.launched_442 = 1;
            out.launch_velocity_128 = self.launch_velocity.map(f32::to_bits);
        }
    }
}
pub(crate) fn enter(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    skater.ground_animation.launched = false;
    skater.ground_animation.launch_velocity = [0.0; 4];
    let life = &mut skater.ground_lifecycle;
    life.skeleton_elapsed_16505 = true;
    physics
        .board
        .hook_mut()
        .drive
        .enable_angular_only(&mut life.board_animated_290);
    if skater.wipeout.state.mode != 1 {
        skater.wipeout.state.mode = 1;
        skater.wipeout.state.balance = 0.0;
    }
    Ok(())
}
pub(crate) fn advance(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    skater.ground_animation.launched = false;
    let p = &skater.player_input.processed;
    let com = p.animation_com_to_deck_752.map(f32::from_bits);
    //82D33DB8 calls UpdateGroundStatesFixedHeading82D8C8F0 without the Ground job's
    //1136/1200 seed (82D4E250): the heading is retained while the deck is animated.
    let heading = physics.riding.reckoning_frames.heading;
    physics.riding.update_ground_reckoning_with_heading(
        &physics.board,
        super::riding_outputs::RidingPoseInputs {
            com_to_deck: Vector3::new(com[0], com[1], com[2]),
            body_spin: skater.animation_input.extra.physical_body_spin,
        },
        p.flags_2468,
        skater.animation_input.fields.balance,
        p.flags_2476 & 0x4000_0000 != 0,
        p,
        heading,
    );
    skeleton::advance(physics, skater)?;
    skater.skeleton_output.correction.pending = true;
    board::advance(physics, skater)
}
pub(crate) fn exit(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    board::set_drag(physics, 0.0);
    physics.settings.wheel_material = physics.settings.standard_wheel_material;
    if skater.ground_animation.jump.active {
        let p = &skater.player_input.processed;
        let mut reference = skater.ground_animation.jump.velocity;
        reference[1] = p.gravity_2648.mul_add(p.timestep_2604, reference[1]);
        let velocity = skate3_core::air::state::clamp_jump_velocity(
            reference,
            p.vectors_400_416[0].map(f32::from_bits),
        );
        skater
            .ground_runtime
            .set_animated_velocity(&mut physics.board, velocity);
    }
    skater.ground_animation.jump = GroundJump::default();
    Ok(())
}
