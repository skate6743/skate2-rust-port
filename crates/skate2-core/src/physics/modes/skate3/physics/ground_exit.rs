//!Complete PhysicsGround Exit82D37B20, on the real board and controller owners.
use super::{Skate3PhysicsState, Skate3ControllerState};
use skate3_core::air::state::clamp_jump_velocity;
pub(super) fn exit(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) {
    let state = &mut skater.ground.state;
    state.steering_damped_turn_2644 = 0.0;
    state.steering_push_scalar_2640 = 1.0;
    skater.ground.pumping.reset();
    for body in physics.board.bodies_mut() {
        body.inertia.linear_drag = 0.0;
    }
    physics.settings.wheel_material = physics.settings.standard_wheel_material;
    if state.flag_2708 {
        let p = &skater.player_input.processed;
        let gravity = [0.0, p.gravity_2648, 0.0, 0.0];
        let reference =
            std::array::from_fn(|i| gravity[i].mul_add(p.timestep_2604, state.vector_2688[i]));
        let velocity = clamp_jump_velocity(reference, p.vectors_400_416[0].map(f32::from_bits));
        skater
            .ground_runtime
            .set_animated_velocity(&mut physics.board, velocity);
    }
}
