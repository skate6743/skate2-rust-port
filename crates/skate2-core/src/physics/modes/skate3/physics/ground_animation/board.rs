//!82D33E30: jump launch or ordered foot/brake/friction/collision forces.
use super::*;
use skate3_core::{
    air::ground_jump::{self, GroundJumpInput},
    physics::force_queue::QueuedPointForce,
    riding::{
        braking::{self, BrakeInput, LinearDragInput},
        collision_response::CollisionResponsePhysical,
        ground_force::{self, GroundForceInput},
        grounded::drag::DRAG_FREQUENCY,
        slide_friction::{self, SlideFrictionInput},
    },
};

pub(super) fn set_drag(physics: &mut Skate3PhysicsState, drag: f32) {
    //BodySetLinearDrag82D9CD60 multiplies by native59.999996, for all seven parts.
    for body in physics.board.bodies_mut() {
        body.inertia.linear_drag = drag * DRAG_FREQUENCY;
    }
}
pub(super) fn advance(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let p = &skater.player_input.processed;
    let t = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("GroundAnimation requires current BoardToolkit")?;
    let s = &skater.ground_settings;
    skater.ground.steering.update(
        0.0,
        s.board().steering.tilt_blending,
        p.flags_2468,
        p.flags_2472,
    );
    physics.settings.wheel_material = s.wheel_material;
    let mode = *skater
        .ground_animation_settings
        .modes
        .get(p.state_variant_index_2528 as usize)
        .ok_or_else(|| {
            format!(
                "Invalid GroundAnimation physics mode {}",
                p.state_variant_index_2528
            )
        })?;
    skater.ground_animation.jump = ground_jump::calculate(
        GroundJumpInput {
            flags_2468: p.flags_2468,
            flags_2480: p.flags_2480,
            flags_2484: p.flags_2484,
            flags_2488: p.flags_2488,
            effective_forward: t.effective[2],
            forward: t.forward,
            current_velocity: p.vectors_400_416[0].map(f32::from_bits),
            ground_reference_position: p.vectors_464_480_496_512_528[1].map(f32::from_bits),
            reference_up: p.vectors_544_560_592_608[0].map(f32::from_bits),
            filtered_ground_normal: t.filtered_normal,
            animation_com_position: p.vectors_544_560_592_608[2].map(f32::from_bits),
            prepared_velocity: p.prepared_jump_704.map(f32::from_bits),
            jump_strength: skater.animation_input.extra.jump_strength,
            jump_controls: skater.animation_input.extra.jump_controls,
            gravity_y: p.gravity_2648,
            surface_speed: p.scalar_2656,
        },
        mode,
        &skater.ground_animation_settings.jump,
    );
    if skater.ground_animation.jump.active {
        let velocity = skater.ground_animation.jump.velocity;
        skater
            .ground_runtime
            .set_animated_velocity(&mut physics.board, velocity);
        skater.ground_animation.launched = true;
        set_drag(physics, 0.0);
        let mut info = super::super::air_phase::launch_info(physics, skater)?;
        info.start_velocity = velocity;
        info.player_jumped = true;
        //The native caller does not replace Fill's COM velocity144.
        let input = super::super::air_phase::selector_input(physics, skater)?;
        skater.trajectory.launch(info, input, &physics.world)?;
        let grind_context = super::super::air_trajectory::GrindContext::from_processed(
            &skater.player_input.processed,
            skater
                .player_input
                .toolkit
                .as_ref()
                .ok_or("Jump trajectory requires current board toolkit")?
                .deck[3],
        );
        skater
            .trajectory
            .update(input, &physics.world, grind_context)?;
        //Launch82D678DC stores packet at1920;128 is selector2048. Update's
        //second pass may replace that retained launch velocity before this read.
        skater.ground_animation.launch_velocity = skater
            .trajectory
            .selector
            .launch_info()
            .ok_or("GroundAnimation launch has no retained trajectory packet")?
            .start_velocity;
        return Ok(());
    }
    let settings = s.board();
    let balance = skater.animation_input.fields.balance;
    let front_scale = if balance <= 0.0 {
        if balance >= -0.0 { 1.0 } else { 2.0 }
    } else {
        0.0
    };
    let rear_scale = if balance >= -0.0 {
        if balance <= 0.0 { 1.0 } else { 2.0 }
    } else {
        0.0
    };
    let foot = |offset, absorption, amount| {
        ground_force::calculate(
            settings.ground_force,
            &GroundForceInput {
                argument_1: 1.0,
                application_z: offset,
                argument_3: absorption,
                argument_4: amount,
                balance,
                surface_speed: p.scalar_2656,
                axis_384: t.transverse_up,
                velocity_400: p.vectors_400_416[0].map(f32::from_bits),
                axis_544: p.vectors_544_560_592_608[0].map(f32::from_bits),
            },
        )
    };
    let front = foot(s.foot_force_offset, 0.0, front_scale);
    //Toolkit cache1240 is FootForceOffset,1236 is AbsorptionFootForceRearScalar.
    let rear = foot(-s.foot_force_offset, s.absorption_rear * 0.0, rear_scale);
    {
        static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_FOOT_FORCE").is_some()) {
            eprintln!(
                "skate3 foot force: state {} balance {balance:.4} scales {front_scale}/{rear_scale} front {:?} rear {:?} speed {:.3}",
                p.filtered_state_2524, &front, &rear, p.scalar_2656
            );
        }
    }
    let brake = braking::calculate_braking(
        BrakeInput {
            flags_2468: p.flags_2468,
            input_2728: skater.animation_input.fields.brake,
            signed_speed: p.scalar_2612,
            absolute_body_speed: t.absolute_speed,
            surface_factor: s.surface_braking_factor,
            direction: xyz(t.horizontal_forward),
        },
        settings.propulsion.braking,
    );
    let friction = slide_friction::calculate(
        settings.slide_friction,
        &SlideFrictionInput {
            heading_time: p.state_timer_2664,
            normal: p.vectors_464_480_496_512_528[0].map(f32::from_bits),
            velocity: p.vectors_400_416[0].map(f32::from_bits),
            side_axis: t.deck[0],
            surface_speed: p.scalar_2656,
            scalar_2764: p.scalar_2764,
        },
    );
    let normal = physics.riding.reckoning.ground_normal;
    let drag = braking::calculate_linear_drag(
        LinearDragInput {
            flags_2468: p.flags_2468,
            absolute_body_speed: t.absolute_speed,
            balance_2720: balance,
            scalar_2724: skater.ground_lifecycle.manual_drag_2724,
            comparison_scalar: normal.y,
        },
        settings.linear_drag,
    );
    let collision = skater
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
        });
    if let Some(collision) = collision {
        physics.board.forces_mut().append(QueuedPointForce {
            tag: 15,
            force_world: xyz(collision.force_2528),
            point_body: xyz(collision.point_2544),
        });
        //Collision branch preserves the preceding body's drag coefficient.
    } else {
        physics.board.forces_mut().append(point_force(4, front));
        physics.board.forces_mut().append(point_force(5, rear));
        physics.board.forces_mut().append(point_force(1, friction));
        physics.board.forces_mut().append(brake);
        set_drag(physics, drag);
    }
    Ok(())
}
fn point_force(tag: u32, v: [f32; 8]) -> QueuedPointForce {
    QueuedPointForce {
        tag,
        force_world: Vector3::new(v[0], v[1], v[2]),
        point_body: Vector3::new(v[4], v[5], v[6]),
    }
}
fn xyz(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
