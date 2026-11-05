use super::*;
use crate::physics::{
    drive_frames::{AuthoredTransformInputs, authored_body_transforms, default_truck_transforms},
    drive_parameters::{RetailTruckDriveSettings, retail_truck_drive_dynamics},
    force_queue::QueuedPointForce,
};

fn masses() -> [RetailBodyMassProperties; BODY_COUNT] {
    core::array::from_fn(|i| RetailBodyMassProperties {
        local_mass_frame: RetailLocalMassFrame::IDENTITY,
        dynamics: RetailInertiaDynamics {
            inverse_tensor: Vector3::new(2.0 + i as f32, 3.0 + i as f32, 4.0 + i as f32),
            inverse_mass: 0.5,
            spherical: 0.0,
            maximum_linear_velocity: 1000.0,
            maximum_angular_velocity: 1000.0,
            linear_drag: 0.0,
            angular_drag: 0.0,
        },
    })
}

fn simulation(gravity: Vector3) -> RetailSimulationStep {
    RetailSimulationStep::fixed_60_hz(30, 0.001, gravity)
}

fn runtime(gravity: Vector3, mode: BoardMotion) -> BoardRuntime {
    BoardRuntime::new(
        masses(),
        authored_body_transforms(AuthoredTransformInputs::STOCK),
        RetailAffineTransform::IDENTITY,
        simulation(gravity),
        mode,
    )
}

fn settings(gravity: Vector3, iterations: u32) -> BoardStepSettings {
    BoardStepSettings {
        simulation: simulation(gravity),
        iterations,
        base_truck_transforms: default_truck_transforms(),
        truck_dynamics: retail_truck_drive_dynamics(RetailTruckDriveSettings::STOCK),
        force_point_y_offset: 0.0,
    }
}

fn close(actual: Vector3, expected: Vector3) {
    assert!(
        (actual.x - expected.x).abs() < 2e-5,
        "{actual:?} != {expected:?}"
    );
    assert!(
        (actual.y - expected.y).abs() < 2e-5,
        "{actual:?} != {expected:?}"
    );
    assert!(
        (actual.z - expected.z).abs() < 2e-5,
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn moving_board_transform_preserves_rates_and_pending_forces() {
    let mut board = runtime(Vector3::new(0., -9.8, 0.), BoardMotion::Active);
    for (i, body) in board.bodies_mut().iter_mut().enumerate() {
        body.rates.linear_velocity = Vector3::new(i as f32, 2., 3.);
        body.rates.angular_velocity = Vector3::new(1., i as f32, 2.);
        body.rates.force_acceleration = Vector3::new(5., 6., 7.);
        body.rates.torque_acceleration = Vector3::new(8., 9., 10.);
    }
    let before = *board.bodies();
    let parts = board.part_transforms();
    let delta = Vector3::new(4., 5., 6.);
    let mut target = parts[BodyId::Deck.index()];
    target.translation = delta;
    board.set_transform(target);
    for (i, body) in board.bodies().iter().enumerate() {
        close(body.rates.linear_velocity, before[i].rates.linear_velocity);
        close(
            body.rates.angular_velocity,
            before[i].rates.angular_velocity,
        );
        close(
            body.rates.force_acceleration,
            before[i].rates.force_acceleration,
        );
        close(
            body.rates.torque_acceleration,
            before[i].rates.torque_acceleration,
        );
        let p = parts[i].translation;
        close(
            board.part_transforms()[i].translation,
            Vector3::new(p.x + delta.x, p.y + delta.y, p.z + delta.z),
        );
    }
}

#[test]
fn spawn_preserves_each_mass_frame_and_keeps_the_hook_separate() {
    let mut data = masses();
    data[BodyId::Deck.index()].local_mass_frame = RetailLocalMassFrame {
        basis: Basis3 {
            columns: [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        },
        translation: Vector3::new(0.2, -0.03, 0.01),
    };
    let spawn = RetailAffineTransform {
        basis: Basis3 {
            columns: [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
        },
        translation: Vector3::new(3.0, 4.0, 5.0),
    };
    let gravity = Vector3::new(0.0, -9.81, 0.0);
    let board = BoardRuntime::new(
        data,
        authored_body_transforms(AuthoredTransformInputs::STOCK),
        spawn,
        simulation(gravity),
        BoardMotion::Active,
    );
    close(
        board.body_transform(BodyId::Deck).translation,
        Vector3::new(2.99, 4.2, 4.97),
    );
    let deck_part = board.part_transforms()[BodyId::Deck.index()];
    close(deck_part.translation, spawn.translation);
    for i in 0..3 {
        close(
            Vector3::new(
                deck_part.basis.columns[i][0],
                deck_part.basis.columns[i][1],
                deck_part.basis.columns[i][2],
            ),
            Vector3::new(
                spawn.basis.columns[i][0],
                spawn.basis.columns[i][1],
                spawn.basis.columns[i][2],
            ),
        );
    }
    for (i, body) in board.bodies().iter().enumerate() {
        assert_eq!(body.inertia, data[i].dynamics);
        assert_eq!(body.state_flags, 4);
        assert_eq!(body.rates.force_acceleration, gravity);
        assert_eq!(body.rates.kinetic_energy, f32::MAX);
        assert_eq!(body.rates.cool_down, 0);
    }
    close(board.hook_transform().translation, spawn.translation);
    assert_ne!(
        board.hook_transform().translation,
        board.body_transform(BodyId::Deck).translation
    );
    assert_eq!(board.hook().body.state_flags, 1);
    assert_eq!(board.hook().body.inertia.inverse_mass, 0.0);
    assert_eq!(
        board.hook().body.rates.world_inverse_inertia.columns,
        [[0.0; 3]; 3]
    );
    assert_eq!(board.hook().body.rates.force_acceleration, Vector3::ZERO);
}

#[test]
fn integration_updates_the_same_state_read_by_presentation_and_force_producers() {
    let mut board = runtime(Vector3::ZERO, BoardMotion::Active);
    board.forces_mut().append(QueuedPointForce {
        tag: 7,
        force_world: Vector3::new(0.0, 0.0, 14.0),
        point_body: Vector3::ZERO,
    });
    let initial = board.body_transform(BodyId::Deck).translation;
    let config = settings(Vector3::ZERO, 0);
    let dt = config.simulation.time_step;
    board.advance(&[], [0.0; 2], config);
    let deck = board.bodies()[BodyId::Deck.index()];
    close(deck.rates.linear_velocity, Vector3::new(0.0, 0.0, 7.0 * dt));
    close(
        board.part_transforms()[BodyId::Deck.index()].translation,
        Vector3::new(initial.x, initial.y, initial.z + 7.0 * dt * dt),
    );
    assert_eq!(board.forces().entries().len(), 1);
    board.clear_forces();
    board.advance(&[], [0.0; 2], config);
    close(
        board.bodies()[BodyId::Deck.index()].rates.linear_velocity,
        deck.rates.linear_velocity,
    );
    close(
        board.part_transforms()[BodyId::Deck.index()].translation,
        Vector3::new(initial.x, initial.y, initial.z + 14.0 * dt * dt),
    );
}

#[test]
fn moving_the_static_hook_changes_constraint_response_without_moving_its_body() {
    let mut target = runtime(Vector3::ZERO, BoardMotion::Active);
    let mut baseline = runtime(Vector3::ZERO, BoardMotion::Active);
    let mut animated = 0;
    target.hook_mut().drive.enable_animation_soft(&mut animated);
    baseline
        .hook_mut()
        .drive
        .enable_animation_soft(&mut animated);
    let requested = RetailAffineTransform {
        translation: Vector3::new(0.0, 0.2, 0.0),
        ..RetailAffineTransform::IDENTITY
    };
    target.set_hook_transform(requested);
    let config = settings(Vector3::ZERO, 25);
    target.advance(&[], [0.0; 2], config);
    baseline.advance(&[], [0.0; 2], config);
    assert!(
        target.bodies()[BodyId::Deck.index()]
            .rates
            .linear_velocity
            .y
            > baseline.bodies()[BodyId::Deck.index()]
                .rates
                .linear_velocity
                .y
    );
    assert_eq!(target.hook_transform().translation, requested.translation);
    assert_eq!(target.hook().body.rates.linear_velocity, Vector3::ZERO);
    assert_eq!(target.hook().body.rates.angular_velocity, Vector3::ZERO);
    assert_eq!(target.hook().body.inertia, ZERO_INERTIA);
}

#[test]
fn explicit_inactive_modes_do_not_integrate_under_gravity() {
    let gravity = Vector3::new(0.0, -9.81, 0.0);
    for mode in [BoardMotion::Frozen, BoardMotion::Static] {
        let mut board = runtime(gravity, mode);
        let before = board.body_transform(BodyId::Deck);
        board.advance(&[], [0.0; 2], settings(gravity, 25));
        assert_eq!(board.body_transform(BodyId::Deck), before);
        assert_eq!(board.bodies()[6].state_flags, mode.flags());
        assert_eq!(board.bodies()[6].rates.cool_down, 30);
        if mode == BoardMotion::Static {
            assert_eq!(board.bodies()[6].inertia.inverse_mass, 0.0);
        } else {
            assert_eq!(board.bodies()[6].inertia, masses()[6].dynamics);
        }
    }
}
