//! Ordered UpdateSkateboard82D3A900: steering, manual, wall launch or Slide forces.
use super::*;
use crate::skate3_native::physics::ground_runtime::GroundInputObservations;
use skate3_core::{
    physics::{
        force_queue::QueuedPointForce,
        manual::controller::{self, ManualAngleMeasurement},
    },
    player::slide_state::{SlideInput, angular_correction, sliding_force},
    riding::{
        collision_response::{CollisionResponsePhysical, signed_angle},
        ground_contact_response::WallRidePhysical,
        ground_force::{self, GroundForceInput},
        steering,
    },
};
pub(super) fn board(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    // The preceding Skeleton call captured the actual unblended board target.
    // Re-capture at Slide's own82D3A924 call; no tick elapses between these writes.
    let target = skater.animated_skeleton.board_frames.animation_target;
    skater
        .skeleton_air
        .capture_physics_error(&physics.board, &target);
    let p = &skater.player_input.processed;
    let t = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Slide requires current BoardToolkit")?;
    let surface_index =
        p.surface_mode_2540
            .checked_sub(1)
            .ok_or("Slide requires the selected SurfacePhysics profile")? as usize;
    let (surface, material) = skater
        .slide_state
        .surfaces
        .get(surface_index)
        .ok_or_else(|| format!("Invalid Slide surface mode {}", p.surface_mode_2540))?;
    // All four actual wheel query volumes share the material copied from2552.
    physics.settings.wheel_material = *material;
    let edge = skater.ground_lifecycle.edge;
    let mut input = skater.ground_settings.input(
        t,
        p,
        &skater.animation_input,
        &skater.ground.pumping,
        skater
            .ground
            .pumping_settings
            .mode(p.state_variant_index_2528)?
            .unintentional_scalar,
        &physics.riding,
        &skater.animated_skeleton,
        physics.settings.step.base_truck_transforms,
        GroundInputObservations {
            manual_drag_2724: skater.ground_lifecycle.manual_drag_2724,
            trajectory_state_bits: p.external_physics_1616.flags,
            edge_flags: edge.map_or(0, |e| e.flags),
            edge_point: edge.map_or([0.0; 4], |e| e.point),
        },
    );
    let settings = skater.ground_settings.board();
    let state = &mut skater.slide_state.state;
    let tilt = steering::calculate_tilt(
        settings.steering,
        input.steering,
        Some(&mut state.steering_push),
        Some(&mut state.damped_turn),
    );
    skater.ground.steering.update(
        tilt,
        settings.steering.tilt_blending,
        p.flags_2468,
        p.flags_2472,
    );
    input.manual.powersliding = true;
    let manual = controller::calculate(
        &mut skater.ground.manual,
        settings.manual,
        settings.manual_mode,
        &input.manual,
        &mut Angle,
    )
    .map_err(|e| format!("Slide manual controller: {e:?}"))?;
    let normal = raw(p.vectors_464_480_496_512_528[0]);
    let up = raw(p.vectors_544_560_592_608[0]);
    let velocity = raw(p.vectors_400_416[0]);
    let mut contact_frame = input.contact;
    contact_frame.scalar_2756 = 0.0;
    let response = skater.ground_runtime.contact_response_with_previous(
        contact_frame,
        WallRidePhysical {
            board_normal: normal,
            up,
            velocity,
            board_mass: t.total_mass,
            gravity: p.gravity_2648,
            speed: p.scalar_2652,
            contact_count: p.wheel_count_2556 as i32,
        },
        [0.0; 4],
    );
    state.wall_riding = response.active_2731;
    if response.animated_board_2708 {
        skater
            .ground_runtime
            .set_animated_velocity(&mut physics.board, response.vector_2688);
        for body in physics.board.bodies_mut() {
            body.inertia.linear_drag = 0.0;
        }
        let mut launch = super::super::air_phase::launch_info(physics, skater)?;
        launch.start_velocity = response.vector_2688;
        launch.player_jumped = true;
        let input = super::super::air_phase::selector_input(physics, skater)?;
        skater.trajectory.launch(launch, input, &physics.world)?;
        let grind_context = super::super::air_trajectory::GrindContext::from_processed(
            &skater.player_input.processed,
            skater
                .player_input
                .toolkit
                .as_ref()
                .ok_or("Slide trajectory requires current board toolkit")?
                .deck[3],
        );
        skater
            .trajectory
            .update(input, &physics.world, grind_context)?;
        //82D3AB10 branches directly to the epilogue. No ordinary force tail.
        return Ok(());
    }
    let slide = SlideInput {
        velocity,
        normal,
        side: t.deck[0],
        effective_forward: t.effective[2],
        reference_forward: t.forward,
        angular_velocity: raw(p.vectors_720_784_800_816_832_864[0]),
        absolute_speed: t.absolute_speed,
        surface_speed: p.scalar_2656,
        slide: skater.animation_input.fields.slide,
        elapsed: p.state_timer_2664,
        wheel_hardness: p.scalar_2764,
    };
    let displacement = angular_correction(&skater.slide_state.settings, surface, slide);
    skater
        .ground_runtime
        .apply_angular_displacement(&mut physics.board, displacement);
    let manual = manual
        .angular_displacement
        .map(|v| v * skater.slide_state.manual_scalar);
    skater
        .ground_runtime
        .apply_angular_displacement(&mut physics.board, manual);
    let f = &input.ground_force;
    let balance = f.balance_2720;
    //82D3AEC0..AF10 preserves the strict scalar branches and fsel signs.
    let front_factor = if balance > 0.0 {
        0.0
    } else {
        fsel(balance, 1.0, 2.0)
    };
    let rear_factor = fsel(balance, if balance > 0.0 { 2.0 } else { 1.0 }, 0.0);
    let front = ground_force::calculate(
        settings.ground_force,
        &GroundForceInput {
            argument_1: f.argument_1_2752,
            application_z: f.ground_scalar_1216,
            argument_3: 0.0,
            argument_4: front_factor,
            balance,
            surface_speed: f.surface_speed_2656,
            axis_384: f.axis_384,
            velocity_400: f.velocity_400,
            axis_544: f.axis_544,
        },
    );
    let rear = ground_force::calculate(
        settings.ground_force,
        &GroundForceInput {
            argument_1: f.argument_1_2752,
            application_z: -f.ground_scalar_1240,
            argument_3: f.ground_scalar_1236 * 0.0,
            argument_4: rear_factor,
            balance,
            surface_speed: f.surface_speed_2656,
            axis_384: f.axis_384,
            velocity_400: f.velocity_400,
            axis_544: f.axis_544,
        },
    );
    let sliding = sliding_force(&skater.slide_state.settings, surface, slide);
    let n = physics.riding.reckoning.ground_normal;
    let collision = skater
        .ground_runtime
        .calculate_collision_force(CollisionResponsePhysical {
            flags_2472: p.flags_2472,
            collision_displacement: raw(p.collision_pose_error_736),
            velocity: raw(p.vectors_400_416[1]),
            forward: t.travel_direction,
            up,
            ground_normal: [n.x, n.y, n.z, 0.0],
            time_step: p.timestep_2604,
            mass: t.total_mass,
        });
    let q = physics.board.forces_mut();
    if let Some(hit) = collision {
        q.append(QueuedPointForce {
            tag: 15,
            force_world: xyz(hit.force_2528),
            point_body: xyz(hit.point_2544),
        });
        // Original Slide caller discards the collision angular output.
    } else {
        q.append(point_force(4, front));
        q.append(point_force(5, rear));
        q.append(response.tag_16_force);
        q.append(sliding);
    }
    Ok(())
}
struct Angle;
impl ManualAngleMeasurement for Angle {
    type Error = std::convert::Infallible;
    fn angle_between(&mut self, a: [f32; 4], b: [f32; 4], c: [f32; 4]) -> Result<f32, Self::Error> {
        Ok(signed_angle(xyz(a), xyz(b), xyz(c)))
    }
}
fn point_force(tag: u32, f: [f32; 8]) -> QueuedPointForce {
    QueuedPointForce {
        tag,
        force_world: Vector3::new(f[0], f[1], f[2]),
        point_body: Vector3::new(f[4], f[5], f[6]),
    }
}
fn xyz(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn raw(v: [u32; 4]) -> [f32; 4] {
    v.map(f32::from_bits)
}
fn fsel(test: f32, a: f32, b: f32) -> f32 {
    if test >= -0.0 { a } else { b }
}
