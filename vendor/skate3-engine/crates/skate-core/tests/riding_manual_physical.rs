//! Independent physical-boundary checks, not a stock gameplay/reference replay.
//! Unlike the scalar fixtures, these use production signed-angle/projection,
//! real BoardRuntime bodies and the deck's actual force/torque accumulators.
use skate_core::{
    math::{Basis3, Vector3},
    physics::{
        board_runtime::{BoardMotion, BoardRuntime},
        deck_angular_correction::apply_angular_displacement,
        drive_frames::RetailAffineTransform,
        force_queue::{BoardForceQueue, QueuedPointForce},
        manual::{
            controller::{self, ManualAngleMeasurement, ManualInput},
            settings::{ManualGains, ManualMode, ManualSettings},
            state::ManualState,
        },
        point_force::RetailForceAccumulator,
        rigid_body::{
            RetailBodyMassProperties, RetailInertiaDynamics, RetailLocalMassFrame,
            RetailSimulationStep,
        },
    },
    point_graph::PointGraph,
    riding::{
        collision_response::signed_angle,
        ground_correction_math::dot_product,
        grounded::{
            manual_entry::{self, ManualGroundBodies, ManualGroundInput, ManualGroundProjection},
            propulsion::{GroundPropulsion, PropulsionSubmission},
        },
        push::PushAcceleration,
    },
};
use std::convert::Infallible;

fn xyz(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn lanes(v: Vector3) -> [f32; 4] {
    [v.x, v.y, v.z, 0.0]
}
fn state() -> ManualState {
    ManualState {
        filtered_angle_error: 0.0,
        target_angle: 0.0,
        measured_angle: 0.0,
        angular_correction: 0.0,
        elapsed: 1.0,
    }
}
fn settings() -> ManualSettings {
    ManualSettings {
        noise_vs_speed: PointGraph {
            x: [0., 1., 2., 3., 4., 5., 6., 7.],
            y: [0.; 8],
        },
        torque_scale_without_contact: 0.25,
        torque_bleed_without_contact: 0.5,
        start_torque_scale: 0.0,
        procedural_noise_scale: 0.0,
        procedural_noise_frequency: 0.0,
        powerslide: ManualGains {
            proportional: 1.0,
            integral: 0.0,
            derivative: 0.0,
        },
        manual: ManualGains {
            proportional: 1.0,
            integral: 0.0,
            derivative: 0.0,
        },
        maximum_tilt_degrees: 10.0,
        maximum_angle_error: 1.0,
        derivative_limit: 1.0,
        brake_tilt_degrees: 0.0,
        animation_noise_scale: 0.0,
    }
}
fn mode(enabled: bool) -> ManualMode {
    ManualMode {
        correction_angular_speed_threshold: 2.0,
        corrective_force_enabled: enabled,
    }
}
fn input() -> ManualInput {
    ManualInput {
        balance: 0.4,
        flipped_controls: 1.0,
        procedural_noise_time: 0.0,
        absolute_speed: 3.0,
        animation_noise: 0.0,
        timestep: 1.0 / 60.0,
        powersliding: false,
        braking: false,
        positive_balance_contact: true,
        negative_balance_contact: true,
        reversed_point_selection: false,
        //A yawed reference ensures no assertion relies on world X/Z aliases.
        reference_x: [0., 0., -1., 0.],
        reference_z: [1., 0., 0., 0.],
        deck_z: [1., 0., 0., 0.],
        velocity_frame_z: [1., 0., 0., 0.],
        angular_velocity_world: [-3., 0., 0., 0.],
        correction_point_7888: [0., -0.1, 0.3, 0.],
        correction_point_7952: [0., -0.1, -0.3, 0.],
    }
}
struct Geometry;
impl ManualAngleMeasurement for Geometry {
    type Error = Infallible;
    fn angle_between(
        &mut self,
        a: [f32; 4],
        b: [f32; 4],
        axis: [f32; 4],
    ) -> Result<f32, Infallible> {
        Ok(signed_angle(xyz(a), xyz(b), xyz(axis)))
    }
}
fn board() -> BoardRuntime {
    //Deliberate unit-mass/inertia test inputs; not stock construction claims.
    let mass = RetailBodyMassProperties {
        local_mass_frame: RetailLocalMassFrame::IDENTITY,
        dynamics: RetailInertiaDynamics {
            inverse_tensor: Vector3::new(1., 1., 1.),
            inverse_mass: 1.0,
            spherical: 0.0,
            maximum_linear_velocity: 100.0,
            maximum_angular_velocity: 100.0,
            linear_drag: 0.0,
            angular_drag: 0.0,
        },
    };
    BoardRuntime::new(
        [mass; 7],
        [RetailAffineTransform::IDENTITY; 7],
        RetailAffineTransform::IDENTITY,
        RetailSimulationStep::fixed_60_hz(30, 0.001, Vector3::ZERO),
        BoardMotion::Active,
    )
}

#[test]
fn measured_deck_error_produces_restoring_torque_on_both_sides_of_both_targets() {
    for balance in [-0.4_f32, 0.4] {
        for control_sign in [-1.0_f32, 1.0] {
            for offset in [-0.08_f32, 0.08] {
                let mut i = input();
                i.balance = balance;
                i.flipped_controls = control_sign;
                let target =
                    (balance * control_sign).signum() * 0.55 * 10.0 * f32::from_bits(0x3c8e_fa35);
                let measured = target + offset;
                i.deck_z = [measured.cos(), measured.sin(), 0.0, 0.0];
                let mut controller = state();
                let effect = controller::calculate(
                    &mut controller,
                    &settings(),
                    mode(true),
                    &i,
                    &mut Geometry,
                )
                .unwrap();
                assert!((controller.measured_angle - measured).abs() < 0.001);
                assert!((controller.target_angle - target).abs() < 0.00001);
                assert!(dot_product(effect.angular_displacement, i.reference_x) * offset > 0.0);
                let mut physical = board();
                let before = physical.bodies()[6].rates;
                apply_angular_displacement(
                    &mut physical.bodies_mut()[6].rates,
                    xyz(effect.angular_displacement),
                );
                let after = physical.bodies()[6].rates;
                assert!(
                    dot_product(lanes(after.torque_acceleration), i.reference_x) * offset > 0.0
                );
                assert_eq!(after.orientation, before.orientation);
                assert_eq!(after.position, before.position);
                assert_eq!(after.angular_velocity, before.angular_velocity);
                assert!(!effect.correction_active);
            }
        }
    }
}

struct Bodies<'a>(&'a mut BoardRuntime);
impl ManualGroundBodies for Bodies<'_> {
    fn linear_velocity(&mut self, part: usize) -> [f32; 4] {
        lanes(self.0.bodies()[part].rates.linear_velocity)
    }
    fn set_linear_velocity(&mut self, part: usize, v: [f32; 4]) {
        self.0.bodies_mut()[part].rates.linear_velocity = xyz(v);
    }
}
struct Projection;
impl ManualGroundProjection for Projection {
    type Error = Infallible;
    fn normal_speed(&mut self, n: [f32; 4], v: [f32; 4]) -> Result<f32, Infallible> {
        Ok(dot_product(n, v))
    }
}

#[test]
fn landing_entry_removes_real_normal_velocity_only_from_native_selected_parts() {
    for reversed in [false, true] {
        for speed in [-3.0_f32, 3.0] {
            let mut physical = board();
            let normal = [0.0, 0.6, 0.8, 0.0];
            for body in physical.bodies_mut() {
                body.rates.linear_velocity = Vector3::new(2.0, speed * 0.6, speed * 0.8);
                body.rates.angular_velocity = Vector3::new(1.0, 2.0, 3.0);
            }
            let before = *physical.bodies();
            let mut manual = state();
            manual_entry::enter_ground(
                &mut manual,
                200,
                0.25,
                ManualGroundInput {
                    balance: 0.4,
                    ground_normal: normal,
                    flags_2468: if reversed { 1 << 20 } else { 0 },
                    flags_2472: 1 << 27,
                },
                &mut Bodies(&mut physical),
                &mut Projection,
            )
            .unwrap();
            let selected = if reversed { [2, 3, 5, 6] } else { [0, 1, 4, 6] };
            for part in 0..7 {
                let after = physical.bodies()[part].rates;
                if selected.contains(&part) {
                    assert!(dot_product(normal, lanes(after.linear_velocity)).abs() < 0.000001);
                    assert_eq!(after.linear_velocity.x, 2.0);
                } else {
                    assert_eq!(after.linear_velocity, before[part].rates.linear_velocity);
                }
                assert_eq!(after.angular_velocity, before[part].rates.angular_velocity);
                assert_eq!(after.orientation, before[part].rates.orientation);
            }
            assert_eq!(manual.angular_correction, 0.0);
            assert_eq!(manual.elapsed, 0.0);
        }
    }
}

#[test]
fn opposing_ground_velocity_selects_tag7_and_reaches_linear_and_torque_accumulators() {
    let mut i = input();
    i.braking = true;
    let propulsion = GroundPropulsion {
        braking: QueuedPointForce::default(),
        push: PushAcceleration::default(),
    };
    let effect =
        controller::calculate(&mut state(), &settings(), mode(true), &i, &mut Geometry).unwrap();
    assert!(effect.correction_active);
    assert_eq!(effect.corrective_force_world, [1200.0, -0.0, -0.0, -0.0]);
    let mut queue = BoardForceQueue::default();
    assert_eq!(
        propulsion.submit(&effect, &mut queue),
        PropulsionSubmission::ManualCorrection(true)
    );
    assert_eq!(queue.entries().len(), 1);
    assert_eq!(queue.entries()[0].tag, 7);
    let identity = Basis3 {
        columns: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
    };
    let output = queue.apply_to_deck(
        RetailForceAccumulator {
            force_acceleration: Vector3::ZERO,
            torque_acceleration: Vector3::ZERO,
            cool_down: 7,
        },
        identity,
        1.0,
        identity,
        0.0,
    );
    assert_eq!(output.force_acceleration, Vector3::new(1200.0, 0.0, 0.0));
    assert!(output.torque_acceleration.y > 350.0);
    assert!(output.torque_acceleration.z > 110.0);
    assert_eq!(output.cool_down, 0);

    let disabled =
        controller::calculate(&mut state(), &settings(), mode(false), &i, &mut Geometry).unwrap();
    assert!(!disabled.correction_active);
    assert!(disabled.opposing_motion_without_correction);
    let mut ordinary = BoardForceQueue::default();
    assert!(matches!(
        propulsion.submit(&disabled, &mut ordinary),
        PropulsionSubmission::BrakingAndPush(_)
    ));
    assert!(!ordinary.entries().iter().any(|force| force.tag == 7));
}
