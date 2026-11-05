//! Wheel/ground response through the production shared solver. These tests
//! query the production sphere/triangle path before compiling the contact.
//! Wheel mass/radius/inertia come from the recovered stock assembly.
use skate_core::{
    math::Vector3,
    physics::{
        board::RETAIL_WHEEL_RADIUS,
        collision::{
            Sphere, Triangle, TriangleFeature, WorldContactSettings, sphere_triangle_world_contact,
        },
        contact::{RetailContactBodyState, RetailContactInput, generate_contact},
        contact_solver::{RetailContactJacobian, build_contact_jacobian},
        mass::retail_wheel_mass_properties,
        rigid_body::RetailReactionCorrections,
        solver::solve_constraints,
    },
};

const DT: f32 = 1.0 / 60.0;
const GRAVITY: f32 = 9.8;

fn contact(speed: f32, spin: f32, force: f32, friction: (f32, f32)) -> RetailContactJacobian {
    let inertia = retail_wheel_mass_properties().dynamics;
    let world = RetailContactBodyState {
        contact_body_id: u32::MAX,
        reaction_id: 1,
        state: 0,
        center_of_mass: Vector3::ZERO,
        inverse_inertia_full: Vector3::ZERO,
        inverse_inertia_split: Vector3::ZERO,
        inverse_mass: 0.0,
        force_acceleration: Vector3::ZERO,
        torque_acceleration: Vector3::ZERO,
        linear_velocity: Vector3::ZERO,
        angular_velocity: Vector3::ZERO,
        kinetic_energy: 0.0,
        cool_down: 0,
    };
    let wheel = RetailContactBodyState {
        contact_body_id: 0,
        reaction_id: 0,
        state: 4,
        center_of_mass: Vector3::new(0.0, RETAIL_WHEEL_RADIUS, 0.0),
        inverse_inertia_full: Vector3::new(inertia.inverse_tensor.x, 0.0, 0.0),
        inverse_inertia_split: Vector3::new(
            inertia.inverse_tensor.z,
            inertia.inverse_tensor.y,
            0.0,
        ),
        inverse_mass: inertia.inverse_mass,
        force_acceleration: Vector3::new(0.0, -GRAVITY, force * inertia.inverse_mass),
        linear_velocity: Vector3::new(0.0, 0.0, speed),
        angular_velocity: Vector3::new(spin, 0.0, 0.0),
        ..world
    };
    let pair = sphere_triangle_world_contact(
        Sphere {
            center: wheel.center_of_mass,
            radius: RETAIL_WHEEL_RADIUS,
        },
        Triangle {
            vertices: [
                Vector3::new(-2.0, 0.0, -2.0),
                Vector3::new(2.0, 0.0, -2.0),
                Vector3::new(-2.0, 0.0, 2.0),
            ],
            feature: TriangleFeature {
                normal: Vector3::new(0.0, 1.0, 0.0),
                edges: [
                    Vector3::new(0.0, 0.0, 1.0),
                    Vector3::new(
                        std::f32::consts::FRAC_1_SQRT_2,
                        0.0,
                        -std::f32::consts::FRAC_1_SQRT_2,
                    ),
                    Vector3::new(-1.0, 0.0, 0.0),
                ],
                flags: 0x1F0,
                edge_cosines: [1.0; 3],
            },
            edge_lengths: [4.0, 4.0 * 2.0_f32.sqrt(), 4.0],
            fatness: 0.0,
        },
        wheel.linear_velocity,
        WorldContactSettings {
            volume_padding: 0.0,
            maximum_separating_distance: 0.0,
            edge_cos_bend_normal_threshold: -1.0,
            convexity_epsilon: 0.0,
            is_object: false,
        },
    )
    .expect("wheel sphere reaches the triangle");
    build_contact_jacobian(
        generate_contact(
            RetailContactInput {
                position_on_a: pair.points.a,
                position_on_b: pair.points.b,
                normal: pair.normal,
                static_friction: friction.0,
                dynamic_friction: friction.1,
                restitution: 0.0,
                tag: 0,
            },
            wheel,
            world,
        ),
        DT,
    )
}

fn solve(
    speed: f32,
    spin: f32,
    force: f32,
    friction: (f32, f32),
    iterations: u32,
) -> (f32, f32, RetailContactJacobian) {
    let mut contacts = [contact(speed, spin, force, friction)];
    let mut reactions = [RetailReactionCorrections::default(); 2];
    solve_constraints(&mut contacts, &mut [], &mut [], &mut reactions, iterations);
    let r = reactions[0];
    let inv_mass = retail_wheel_mass_properties().dynamics.inverse_mass;
    let velocity = speed + force * inv_mass * DT + r.linear_displacement.z / DT;
    let angular = spin + r.angular_displacement.x / DT;
    assert_eq!(
        reactions[1],
        RetailReactionCorrections::default(),
        "static world must not move"
    );
    if iterations > 0 {
        assert!(
            (-GRAVITY * DT + r.linear_displacement.y / DT).abs() < 1e-6,
            "ground supports the wheel"
        );
    }
    (velocity, angular, contacts[0])
}

#[test]
fn ground_friction_spins_a_slipping_wheel_in_both_directions() {
    for speed in [-1.0, 1.0] {
        let (velocity, angular, _) = solve(speed, 0.0, 0.0, (0.8, 0.7), 25);
        assert!(velocity.abs() < speed.abs());
        assert!(angular * speed > 0.0);
        assert!((velocity - angular * RETAIL_WHEEL_RADIUS).abs() < speed.abs());
        let inertia = retail_wheel_mass_properties().dynamics;
        let linear_impulse = (velocity - speed) / inertia.inverse_mass;
        let angular_impulse = angular / inertia.inverse_tensor.x;
        assert!((angular_impulse + RETAIL_WHEEL_RADIUS * linear_impulse).abs() < 1e-7);
    }
}

#[test]
fn a_rolling_wheel_does_not_receive_artificial_drag() {
    for speed in [-3.0, 3.0] {
        let spin = speed / RETAIL_WHEEL_RADIUS;
        let (velocity, angular, _) = solve(speed, spin, 0.0, (0.8, 0.7), 25);
        assert!((velocity - speed).abs() < 1e-6);
        assert!((angular - spin).abs() < 1e-5);
    }
}

#[test]
fn frictionless_ground_supports_without_spinning_or_slowing_the_wheel() {
    let (velocity, angular, _) = solve(2.0, 0.0, 0.0, (0.0, 0.0), 25);
    assert_eq!(velocity, 2.0);
    assert_eq!(angular, 0.0);
}

#[test]
fn static_grip_transitions_to_dynamic_friction_under_larger_load() {
    let (velocity, angular, _) = solve(0.0, 0.0, 0.1, (0.8, 0.7), 25);
    assert!(velocity > 0.0 && angular > 0.0);
    assert!((velocity - angular * RETAIL_WHEEL_RADIUS).abs() < 1e-6);
    let (velocity, angular, contact) = solve(0.0, 0.0, 30.0, (0.8, 0.7), 25);
    assert!(velocity - angular * RETAIL_WHEEL_RADIUS > 0.1);
    let tangential = contact.accumulated_impulse()[1]
        .abs()
        .max(contact.accumulated_impulse()[2].abs());
    assert!((tangential - 0.7 * contact.accumulated_impulse()[0]).abs() < 1e-8);
}

#[test]
fn first_contact_pass_uses_previous_normal_impulse_for_friction() {
    let (_, first_spin, _) = solve(1.0, 0.0, 0.0, (0.8, 0.7), 1);
    let (_, second_spin, _) = solve(1.0, 0.0, 0.0, (0.8, 0.7), 2);
    assert_eq!(first_spin, 0.0);
    assert!(second_spin > 0.0);
}
