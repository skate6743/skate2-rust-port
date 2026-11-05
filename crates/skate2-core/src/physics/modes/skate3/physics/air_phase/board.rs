//!82D34AB0 consumes the completed Air reckoning's collision normal1216.
use super::*;
use skate3_core::{
    air::state::PhysicsAirFrame, math::Vector3, physics::force_queue::QueuedPointForce,
    riding::collision_response::CollisionResponsePhysical,
};
pub(super) fn advance(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    frame: PhysicsAirFrame,
) -> Result<(), String> {
    let p = &skater.player_input.processed;
    skater.ground.steering.update(
        0.0,
        skater.air_settings.steering_blend,
        p.flags_2468,
        p.flags_2472,
    );
    let t = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Air collision requires the actual BoardToolkit")?;
    let normal = physics.riding.reckoning.ground_normal;
    if let Some(force) =
        skater
            .ground_runtime
            .calculate_collision_force(CollisionResponsePhysical {
                flags_2472: p.flags_2472,
                collision_displacement: p.collision_pose_error_736.map(f32::from_bits),
                velocity: p.vectors_400_416[1].map(f32::from_bits),
                forward: t.travel_direction,
                up: p.vectors_544_560_592_608[0].map(f32::from_bits),
                ground_normal: [normal.x, normal.y, normal.z, 0.0],
                time_step: p.timestep_2604,
                mass: t.total_mass,
            })
    {
        let xyz = |v: [f32; 4]| Vector3::new(v[0], v[1], v[2]);
        physics.board.forces_mut().append(QueuedPointForce {
            tag: 15,
            force_world: xyz(force.force_2528),
            point_body: xyz(force.point_2544),
        });
    } else if !skater.air_state.use_centre_of_mass_velocity
        && frame.frames_since_jump_correction_2576 < 12
    {
        let velocity = calculate_velocity_from_jump(
            &frame,
            frame.frames_since_jump_correction_2576,
            &mut AirMath,
        );
        skater
            .ground_runtime
            .set_animated_velocity(&mut physics.board, velocity);
    }
    Ok(())
}
