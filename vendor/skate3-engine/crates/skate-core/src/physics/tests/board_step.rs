use super::*;
use crate::physics::{
    drive_frames::{
        default_live_body_orientations, default_live_body_transforms, default_truck_transforms,
    },
    drive_parameters::{RetailTruckDriveSettings, retail_truck_drive_dynamics},
    force_queue::QueuedPointForce,
    mass::default_skateboard_mass_properties,
    rigid_body::{RetailBodyRates, basis_from_quaternion, world_inverse_inertia},
};
use crate::riding::braking::{BrakeInput, BrakeSettings, calculate_braking};
use crate::riding::push::{PushInput, PushLimits, enqueue_push};

#[path = "contact_reports.rs"]
mod contact_reports;
#[path = "hook_step.rs"]
mod hook_step;

fn bodies() -> [BodySnapshot; BODY_COUNT] {
    let poses = default_live_body_transforms();
    let orientations = default_live_body_orientations();
    let masses = default_skateboard_mass_properties();
    core::array::from_fn(|i| {
        let basis = basis_from_quaternion(orientations[i]);
        BodySnapshot {
            state_flags: 4,
            inertia: masses[i].dynamics,
            rates: RetailBodyRates {
                orientation: orientations[i],
                basis,
                world_inverse_inertia: world_inverse_inertia(
                    basis,
                    masses[i].dynamics.inverse_tensor,
                ),
                position: poses[i].translation,
                linear_velocity: Vector3::ZERO,
                angular_velocity: Vector3::ZERO,
                force_acceleration: Vector3::ZERO,
                torque_acceleration: Vector3::ZERO,
                kinetic_energy: 0.0,
                cool_down: 0,
            },
        }
    })
}

// Deliberately inactive synthetic hook isolates the existing board-only cases.
// Its body values are test inputs, not a gameplay initialization contract.
fn inactive_hook() -> BoardHook {
    let mut body = bodies()[6];
    body.state_flags = 0;
    BoardHook {
        body,
        drive: super::super::hook_drive::HookDriveState::initial(),
    }
}

fn settings(iterations: u32) -> BoardStepSettings {
    BoardStepSettings {
        simulation: RetailSimulationStep::fixed_60_hz(30, 0.001, Vector3::ZERO),
        iterations,
        base_truck_transforms: default_truck_transforms(),
        truck_dynamics: retail_truck_drive_dynamics(RetailTruckDriveSettings::STOCK),
        force_point_y_offset: 0.0,
    }
}

fn momentum_z(bodies: &[BodySnapshot; BODY_COUNT]) -> f32 {
    bodies
        .iter()
        .map(|b| b.rates.linear_velocity.z / b.inertia.inverse_mass)
        .sum()
}

#[test]
fn push_is_delivered_through_constraints_into_all_seven_integrated_bodies() {
    let mut bodies = bodies();
    let before = bodies;
    let mut unforced = before;
    let mut queue = BoardForceQueue::default();
    let config = settings(25);
    let push = enqueue_push(
        PushInput {
            flags_2468: 0x0200_0000,
            flags_2472: 0,
            target_speed: 3.0,
            current_speed: 0.0,
            absolute_body_speed: 0.0,
            scale: f32::NAN,
            delta_seconds: config.simulation.time_step,
            direction: Vector3::new(0.0, 0.0, 1.0),
        },
        PushLimits {
            maximum_pushable_speed: 10.0,
            low_speed_change: 0.1,
            high_speed_change: 0.05,
        },
        &bodies.map(|b| b.inertia.inverse_mass),
        &mut queue,
    );
    let mut tick = BoardStep::default();
    // The preserved research explicitly characterizes noncoincident anchors
    // in the constructor pose (TRANSITION_PHYSICS_EVIDENCE.md). Compare the
    // push against that same unforced pose, not an invented settled board.
    BoardStep::default().advance(
        &mut unforced,
        &mut inactive_hook(),
        &BoardForceQueue::default(),
        &[],
        [0.0; 2],
        config,
    );
    tick.advance(
        &mut bodies,
        &mut inactive_hook(),
        &queue,
        &[],
        [0.0; 2],
        config,
    );
    for id in BodyId::ORDER {
        let after = bodies[id.index()].rates;
        assert!(
            after.linear_velocity.z > unforced[id.index()].rates.linear_velocity.z,
            "{id:?}"
        );
        assert_ne!(after.position, before[id.index()].rates.position, "{id:?}");
        assert_eq!(after.force_acceleration, Vector3::ZERO);
        assert_eq!(after.torque_acceleration, Vector3::ZERO);
    }
    let expected_momentum = push.vector.z * config.simulation.time_step;
    assert!((momentum_z(&bodies) - expected_momentum).abs() < 1e-4);
    assert_eq!(
        queue.entries().len(),
        1,
        "native consumer leaves queue intact"
    );
    assert_eq!(
        tick.reactions,
        [RetailReactionCorrections::default(); REACTION_COUNT]
    );
    // Clearing is owned by GeneralUpdate. A subsequent coast tick must not
    // repeat either the push or last frame's accumulated constraint impulses.
    queue.clear();
    tick.advance(
        &mut bodies,
        &mut inactive_hook(),
        &queue,
        &[],
        [0.0; 2],
        config,
    );
    assert!((momentum_z(&bodies) - expected_momentum).abs() < 1e-4);
}

#[test]
fn simulation_uses_the_supplied_iteration_count_and_one_timestep() {
    let mut zero_pass = bodies();
    let mut solved = zero_pass;
    let mut queue = BoardForceQueue::default();
    queue.append(QueuedPointForce {
        tag: 3,
        force_world: Vector3::new(0.0, 0.0, 60.0),
        point_body: Vector3::ZERO,
    });
    let mut config = settings(0);
    config.simulation.time_step = 1.0 / 120.0;
    config.simulation.frequency = 1.0; // StartFrame must replace a stale reciprocal.
    BoardStep::default().advance(
        &mut zero_pass,
        &mut inactive_hook(),
        &queue,
        &[],
        [0.0; 2],
        config,
    );
    config.iterations = 25;
    BoardStep::default().advance(
        &mut solved,
        &mut inactive_hook(),
        &queue,
        &[],
        [0.0; 2],
        config,
    );
    assert_eq!(zero_pass[0].rates.linear_velocity, Vector3::ZERO);
    assert!(solved[0].rates.linear_velocity.z > 0.0);
    assert!((momentum_z(&solved) - 0.5).abs() < 1e-4);
    assert!((momentum_z(&zero_pass) - 0.5).abs() < 1e-4);
}

#[test]
fn steering_changes_integrated_truck_wheel_and_deck_orientations() {
    let mut straight = bodies();
    let mut turning = straight;
    let queue = BoardForceQueue::default();
    BoardStep::default().advance(
        &mut straight,
        &mut inactive_hook(),
        &queue,
        &[],
        [0.0; 2],
        settings(25),
    );
    BoardStep::default().advance(
        &mut turning,
        &mut inactive_hook(),
        &queue,
        &[],
        [0.3; 2],
        settings(25),
    );
    for id in BodyId::ORDER {
        assert_ne!(
            straight[id.index()].rates.orientation,
            turning[id.index()].rates.orientation,
            "{id:?}"
        );
    }
}

#[test]
fn native_braking_force_reaches_whole_assembly_momentum() {
    for speed in [-4.0, 4.0] {
        let mut bodies = bodies();
        for body in &mut bodies {
            body.rates.linear_velocity.z = speed;
        }
        let before = momentum_z(&bodies);
        let brake = calculate_braking(
            BrakeInput {
                flags_2468: 0x4000_0000,
                input_2728: 0.5,
                signed_speed: speed,
                absolute_body_speed: speed.abs(),
                surface_factor: 0.7,
                direction: Vector3::new(0.0, 0.0, 1.0),
            },
            BrakeSettings {
                input_force: 200.0,
                override_force: 100.0,
                minimum_speed: 0.8,
            },
        );
        let mut queue = BoardForceQueue::default();
        queue.append(brake);
        let config = settings(25);
        BoardStep::default().advance(
            &mut bodies,
            &mut inactive_hook(),
            &queue,
            &[],
            [0.0; 2],
            config,
        );
        let after = momentum_z(&bodies);
        assert!(after.abs() < before.abs());
        assert!((after - before - brake.force_world.z * config.simulation.time_step).abs() < 1e-4);
    }
}

#[test]
fn no_active_bodies_skips_compile_solve_and_integration() {
    let mut bodies = bodies();
    for b in &mut bodies {
        b.state_flags = 0;
    }
    let before = bodies.map(|b| b.rates);
    BoardStep::default().advance(
        &mut bodies,
        &mut inactive_hook(),
        &BoardForceQueue::default(),
        &[],
        [0.3; 2],
        settings(25),
    );
    assert_eq!(before, bodies.map(|b| b.rates));
}

#[test]
fn contact_workspace_contains_this_ticks_queued_force() {
    let mut bodies = bodies();
    let point = bodies[6].rates.position;
    let collision = BoardCollision {
        body_a: CollisionBody::StaticWorld,
        body_b: CollisionBody::Board(BodyId::Deck),
        contact: RetailContactInput {
            position_on_a: point,
            position_on_b: point,
            normal: Vector3::new(0.0, -1.0, 0.0),
            restitution: 0.0,
            static_friction: 0.0,
            dynamic_friction: 0.0,
            tag: 0,
        },
    };
    let mut queue = BoardForceQueue::default();
    queue.append(QueuedPointForce {
        tag: 2,
        force_world: Vector3::new(0.0, -60.0, 0.0),
        point_body: Vector3::ZERO,
    });
    let mut tick = BoardStep::default();
    let config = settings(25);
    let mut free = bodies;
    BoardStep::default().advance(
        &mut free,
        &mut inactive_hook(),
        &queue,
        &[],
        [0.0; 2],
        config,
    );
    tick.advance(
        &mut bodies,
        &mut inactive_hook(),
        &queue,
        &[collision],
        [0.0; 2],
        config,
    );
    let row = tick.contacts[0];
    assert!(row.accumulated_impulse()[0] > 0.0);
    assert!(bodies[6].rates.linear_velocity.y.abs() < free[6].rates.linear_velocity.y.abs());
    let expected_target = config.simulation.time_step.powi(2) * 60.0;
    assert!((row.target_impulse()[0] - expected_target).abs() < 1e-9);
    assert_eq!(
        tick.reactions[WORLD_REACTION],
        RetailReactionCorrections::default()
    );
}
