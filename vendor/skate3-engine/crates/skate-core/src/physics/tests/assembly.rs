use super::*;
use crate::{
    math::Vector3,
    physics::{
        drive_frames::{
            default_live_body_orientations, default_live_body_transforms, default_truck_transforms,
        },
        drive_parameters::{RetailTruckDriveSettings, retail_truck_drive_dynamics},
        mass::default_skateboard_mass_properties,
        rigid_body::{RetailReactionCorrections, basis_from_quaternion, world_inverse_inertia},
        solver::solve_constraints,
    },
};

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

fn inactive_hook() -> BoardHook {
    let mut body = bodies()[6];
    body.state_flags = 0;
    BoardHook {
        body,
        drive: super::super::hook_drive::HookDriveState::initial(),
    }
}

fn solve(targets: [f32; 2]) -> [RetailReactionCorrections; BODY_COUNT] {
    let mut hook = inactive_hook();
    let frames = prepare_drive_frames(default_truck_transforms(), targets, &mut hook);
    let mut constraints = BoardConstraints::build(
        &bodies(),
        &hook,
        frames,
        retail_truck_drive_dynamics(RetailTruckDriveSettings::STOCK),
        f32::from_bits(0x3C88_8889),
    );
    let mut reactions = [RetailReactionCorrections::default(); BODY_COUNT + 1];
    solve_constraints(
        &mut [],
        &mut constraints.joints,
        &mut constraints.drives,
        &mut reactions,
        25,
    );
    reactions[..BODY_COUNT].try_into().unwrap()
}

#[test]
fn native_pair_order_and_stock_constraint_counts_are_preserved() {
    let mut hook = inactive_hook();
    let frames = prepare_drive_frames(default_truck_transforms(), [0.0; 2], &mut hook);
    let constraints = BoardConstraints::build(
        &bodies(),
        &hook,
        frames,
        retail_truck_drive_dynamics(RetailTruckDriveSettings::STOCK),
        f32::from_bits(0x3C88_8889),
    );
    let pairs: Vec<_> = constraints
        .joints
        .into_iter()
        .map(|j| (j.reaction_a, j.reaction_b))
        .collect();
    assert_eq!(pairs, [(4, 6), (5, 6), (0, 4), (1, 4), (2, 5), (3, 5)]);
    assert_eq!(
        constraints
            .drives
            .into_iter()
            .map(|d| (d.frame_a_body.reaction_index, d.frame_b_body.reaction_index))
            .collect::<Vec<_>>(),
        [(4, 6), (5, 6), (7, 6)]
    );
}

#[test]
fn steering_reaches_wheels_through_shared_joint_and_drive_reactions() {
    let rest = solve([0.0; 2]);
    let turn = solve([0.3; 2]);
    for body in BodyId::ORDER {
        let r = turn[body.index()];
        assert_ne!(
            r.angular_displacement,
            rest[body.index()].angular_displacement,
            "{body:?}"
        );
        for v in [r.linear_displacement, r.angular_displacement] {
            assert!(
                v.x.is_finite() && v.y.is_finite() && v.z.is_finite(),
                "{body:?}"
            );
        }
    }
    // Internal constraints exchange momentum; they do not propel the board.
    let snapshots = bodies();
    let momentum = turn
        .iter()
        .zip(snapshots)
        .fold(Vector3::ZERO, |sum, (r, b)| {
            Vector3::new(
                sum.x + r.linear_displacement.x / b.inertia.inverse_mass,
                sum.y + r.linear_displacement.y / b.inertia.inverse_mass,
                sum.z + r.linear_displacement.z / b.inertia.inverse_mass,
            )
        });
    assert!(
        momentum.x.abs() < 1e-5 && momentum.y.abs() < 1e-5 && momentum.z.abs() < 1e-5,
        "{momentum:?}"
    );
}
