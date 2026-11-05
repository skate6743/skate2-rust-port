//! Independent manual control-flow checks; supplied angle measurements are not
//! a replacement implementation or numerical validation of 8296EC98.
use skate_core::{
    physics::manual::{
        angle::{AngleError, normalize},
        controller::{ManualAngleMeasurement, ManualError, ManualInput, calculate},
        settings::{ManualGains, ManualMode, ManualSettings},
        state::{ManualEntryContinuation, ManualState},
    },
    point_graph::PointGraph,
};

fn state() -> ManualState {
    ManualState {
        filtered_angle_error: 0.0,
        target_angle: 0.0,
        measured_angle: 0.0,
        angular_correction: 0.0,
        elapsed: 0.0,
    }
}
fn gains() -> ManualGains {
    ManualGains {
        proportional: 0.0,
        integral: 0.0,
        derivative: 0.0,
    }
}
fn settings() -> ManualSettings {
    ManualSettings {
        noise_vs_speed: PointGraph {
            x: [0., 1., 2., 3., 4., 5., 6., 7.],
            y: [1.0; 8],
        },
        torque_scale_without_contact: 0.25,
        torque_bleed_without_contact: 0.5,
        start_torque_scale: 0.0,
        procedural_noise_scale: 0.0,
        procedural_noise_frequency: 1.0,
        powerslide: gains(),
        manual: gains(),
        maximum_tilt_degrees: 0.0,
        maximum_angle_error: 1.0,
        derivative_limit: 0.25,
        brake_tilt_degrees: 0.0,
        animation_noise_scale: 0.0,
    }
}
fn mode() -> ManualMode {
    ManualMode {
        correction_angular_speed_threshold: 4.0,
        corrective_force_enabled: true,
    }
}
fn input() -> ManualInput {
    ManualInput {
        balance: 1.0,
        flipped_controls: 1.0,
        procedural_noise_time: 0.0,
        absolute_speed: 0.0,
        animation_noise: 0.0,
        timestep: 0.125,
        powersliding: false,
        braking: false,
        positive_balance_contact: true,
        negative_balance_contact: true,
        reversed_point_selection: false,
        reference_x: [1.0, 0.0, 0.0, 0.0],
        reference_z: [0.0, 0.0, 1.0, 0.0],
        deck_z: [0.0, 0.0, 1.0, 0.0],
        velocity_frame_z: [0.0, 0.0, 1.0, 0.0],
        angular_velocity_world: [0.0, 0.0, -5.0, 0.0],
        correction_point_7888: [1.0, 2.0, 3.0, 4.0],
        correction_point_7952: [5.0, 6.0, 7.0, 8.0],
    }
}
struct Angle {
    value: f32,
    calls: usize,
    fail: bool,
}
impl Angle {
    fn new(value: f32) -> Self {
        Self {
            value,
            calls: 0,
            fail: false,
        }
    }
}
impl ManualAngleMeasurement for Angle {
    type Error = &'static str;
    fn angle_between(
        &mut self,
        _deck: [f32; 4],
        _z: [f32; 4],
        _x: [f32; 4],
    ) -> Result<f32, Self::Error> {
        self.calls += 1;
        if self.fail {
            Err("angle measurement unavailable")
        } else {
            Ok(self.value)
        }
    }
}

#[test]
fn zero_balance_resets_five_fields_without_querying_geometry() {
    let mut state = ManualState {
        filtered_angle_error: 1.,
        target_angle: 2.,
        measured_angle: 3.,
        angular_correction: 4.,
        elapsed: 5.,
    };
    let mut input = input();
    input.balance = -0.0;
    let mut angle = Angle::new(0.0);
    angle.fail = true;
    let effect = calculate(&mut state, &settings(), mode(), &input, &mut angle).unwrap();
    assert_eq!(state, crate::state());
    assert_eq!(angle.calls, 0);
    assert_eq!(effect.angular_displacement, [0.0; 4]);
    assert!(!effect.correction_active);
    assert_eq!(effect.corrective_force_world, [0.0; 4]);
    assert!(!effect.opposing_motion_without_correction);
}

#[test]
fn ground_entry_preserves_only_the_native_ground_history_branch() {
    let original = ManualState {
        filtered_angle_error: 2.,
        target_angle: 3.,
        measured_angle: 4.,
        angular_correction: 6.,
        elapsed: 7.,
    };
    let mut state = original;
    assert_eq!(
        state.enter_ground(100, 0.25),
        ManualEntryContinuation::Continue
    );
    assert_eq!(state.filtered_angle_error, 0.5);
    assert_eq!(state.angular_correction, 1.5);
    assert_eq!(state.target_angle, 3.0);
    assert_eq!(state.measured_angle, 4.0);
    assert_eq!(state.elapsed, 7.0);
    assert_eq!(
        state.enter_ground(200, 0.25),
        ManualEntryContinuation::RemoveVelocityIntoGround
    );
    assert_eq!(state, crate::state());
}

#[test]
fn derivative_is_suppressed_only_on_first_active_tick_and_then_clamped() {
    let mut state = state();
    state.measured_angle = -2.0;
    let mut settings = settings();
    settings.manual.derivative = 2.0;
    calculate(
        &mut state,
        &settings,
        mode(),
        &input(),
        &mut Angle::new(0.5),
    )
    .unwrap();
    assert_eq!(state.angular_correction, 0.0);
    assert_eq!(state.elapsed, 0.125);
    let result = calculate(
        &mut state,
        &settings,
        mode(),
        &input(),
        &mut Angle::new(1.0),
    )
    .unwrap();
    assert_eq!(state.angular_correction, 0.25);
    assert_eq!(result.angular_displacement[0], -0.25);
    assert_eq!(state.elapsed, 0.25);
}

#[test]
fn wrong_contact_bleeds_state_and_separately_scales_output() {
    let mut state = state();
    state.elapsed = 1.0;
    state.angular_correction = 8.0;
    let mut input = input();
    input.positive_balance_contact = false;
    let result = calculate(
        &mut state,
        &settings(),
        mode(),
        &input,
        &mut Angle::new(0.0),
    )
    .unwrap();
    assert_eq!(state.angular_correction, 2.0); // bleed0.5 * one-axle factor0.5
    assert_eq!(result.angular_displacement[0], -0.25); // output0.25 * one-axle factor0.5
    input.balance = -1.0;
    calculate(
        &mut state,
        &settings(),
        mode(),
        &input,
        &mut Angle::new(0.0),
    )
    .unwrap();
    assert_eq!(state.angular_correction, 2.0); // negative balance contact exists
}

#[test]
fn powerslide_argument_selects_distinct_controller_gains() {
    let mut settings = settings();
    settings.manual.proportional = 2.0;
    settings.powerslide.proportional = 4.0;
    for (powersliding, expected) in [(false, -1.0), (true, -2.0)] {
        let mut input = input();
        input.powersliding = powersliding;
        let mut state = state();
        calculate(&mut state, &settings, mode(), &input, &mut Angle::new(0.5)).unwrap();
        assert_eq!(state.angular_correction, expected);
        assert_eq!(state.filtered_angle_error, -0.025);
    }
}

#[test]
fn brake_target_ignores_noise_but_retains_signed_target_mapping() {
    let mut settings = settings();
    settings.maximum_tilt_degrees = 24.0;
    settings.brake_tilt_degrees = 30.0;
    settings.procedural_noise_scale = 100.0;
    settings.animation_noise_scale = 100.0;
    let mut input = input();
    input.braking = true;
    input.balance = -0.5;
    input.flipped_controls = -1.0;
    input.animation_noise = 1.0;
    input.procedural_noise_time = 0.25;
    let mut state = state();
    calculate(&mut state, &settings, mode(), &input, &mut Angle::new(0.0)).unwrap();
    let expected = 18.75 * f32::from_bits(0x3c8efa35); // target magnitude .625 *30deg
    assert_eq!(state.target_angle, expected);
}

#[test]
fn procedural_noise_uses_processed_time_instead_of_elapsed_timer() {
    let mut settings = settings();
    settings.procedural_noise_scale = 0.25;
    let mut input = input();
    input.procedural_noise_time = 0.25;
    let mut state = state();
    state.elapsed = 20.0;
    calculate(&mut state, &settings, mode(), &input, &mut Angle::new(0.0)).unwrap();
    assert!((state.target_angle - 0.25).abs() < 0.000001);
    assert_eq!(state.elapsed, 20.125);
}

#[test]
fn correction_requires_opposing_motion_speed_and_alignment_and_selects_point() {
    let mut input = input();
    input.braking = true;
    for (reversed, point) in [
        (false, input.correction_point_7888),
        (true, input.correction_point_7952),
    ] {
        input.reversed_point_selection = reversed;
        let result = calculate(
            &mut state(),
            &settings(),
            mode(),
            &input,
            &mut Angle::new(0.0),
        )
        .unwrap();
        assert!(result.correction_active);
        assert_eq!(result.corrective_force_world, [0.0, 0.0, 2000.0, 0.0]);
        assert_eq!(result.corrective_point_body, point);
        assert!(!result.opposing_motion_without_correction);
    }
    input.angular_velocity_world[2] = -4.0; // threshold equality does not activate
    assert!(
        !calculate(
            &mut state(),
            &settings(),
            mode(),
            &input,
            &mut Angle::new(0.0)
        )
        .unwrap()
        .correction_active
    );
    input.angular_velocity_world[2] = -5.0;
    assert!(
        !calculate(
            &mut state(),
            &settings(),
            mode(),
            &input,
            &mut Angle::new(0.1)
        )
        .unwrap()
        .correction_active
    );
    input.angular_velocity_world[2] = 5.0;
    assert!(
        !calculate(
            &mut state(),
            &settings(),
            mode(),
            &input,
            &mut Angle::new(0.0)
        )
        .unwrap()
        .correction_active
    );
}

#[test]
fn disabled_mode_reports_opposing_motion_before_speed_and_alignment_gates() {
    let mut input = input();
    input.braking = true;
    input.angular_velocity_world[2] = -0.1;
    let mut mode = mode();
    mode.corrective_force_enabled = false;
    let result = calculate(
        &mut state(),
        &settings(),
        mode,
        &input,
        &mut Angle::new(0.5),
    )
    .unwrap();
    assert!(result.opposing_motion_without_correction);
    assert!(!result.correction_active);
    assert_eq!(result.corrective_force_world, [0.0; 4]);
}

#[test]
fn measurement_failure_does_not_return_partial_effect_or_advance_timer() {
    let mut state = state();
    let mut angle = Angle::new(0.0);
    angle.fail = true;
    assert_eq!(
        calculate(&mut state, &settings(), mode(), &input(), &mut angle),
        Err(ManualError::Measurement("angle measurement unavailable"))
    );
    assert_eq!(state.elapsed, 0.0);
}

#[test]
fn finite_angle_wrap_has_native_interval_and_explicit_exception_boundary() {
    let pi = f32::from_bits(0x40490fdb);
    let tau = f32::from_bits(0x40c90fdb);
    assert_eq!(normalize(pi), Ok(-pi));
    assert_eq!(normalize(-pi), Ok(-pi));
    assert_eq!(normalize(0.25), Ok(0.25));
    assert_eq!(normalize(tau).unwrap(), 0.0);
    assert_eq!(normalize(tau).unwrap().to_bits(), (-0.0_f32).to_bits());
    assert_eq!(
        normalize(f32::NAN),
        Err(AngleError::IntegerConversionUnavailable)
    );
    assert_eq!(
        normalize(f32::INFINITY),
        Err(AngleError::IntegerConversionUnavailable)
    );
    assert_eq!(
        normalize(f32::MAX),
        Err(AngleError::IntegerConversionUnavailable)
    );
}
