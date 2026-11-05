use super::*;
use crate::air::ground_normal::GroundNormalFilter;

fn orientation() -> GroundOrientation {
    let up = [0., 1., 0., 0.];
    GroundOrientation {
        dynamic_up: xyz(up),
        up: xyz(up),
        target: xyz(up),
        up_velocity: Vector3::new(7., 8., 9.),
        ground_normal: xyz(up),
        ground_blend: 0.75,
        ground_filter: GroundNormalFilter::initialized([1., 0., 0., 1.], up),
        slow_filter: GroundNormalFilter::initialized([0.2, 0.1, 0.05, 0.3], up),
        fast_filter: GroundNormalFilter::initialized([0.7, 0.2, 0.1, 0.8], up),
    }
}

fn run(orientation: &mut GroundOrientation, input: Input) -> (Output, BodySpinState, AirState) {
    let curve = PointGraph {
        x: [0., 1., 2., 3., 4., 5., 6., 7.],
        y: [0.; 8],
    };
    let mut frames = ReckoningFrames::new();
    let mut spin = BodySpinState::new();
    let mut air = AirState::new();
    air.spin_angle = 2.;
    air.secondary_lean_angle = 3.;
    let output = update(
        orientation,
        &mut frames,
        &mut spin,
        &mut air,
        Settings {
            ground_normal_smoothing: [1., 0., 0., 1.],
            tilt_vs_rotation: &curve,
            tilt_vs_slope: &curve,
        },
        input,
    );
    (output, spin, air)
}

fn input() -> Input {
    Input {
        previous_up: [0., 1., 0., 2.],
        requested_up: [0., 0., 1., 4.],
        requested_forward: [1., 0., 0., 0.],
        blend: 0.5,
        reverse_stance: false,
        enable_body_spin_input: true,
        physical_body_spin_2812: 0.25,
    }
}

#[test]
fn blend_preserves_fourth_lane_and_updates_history_without_resetting_other_state() {
    let mut state = orientation();
    let controls = state.slow_filter.words()[..4].to_vec();
    let (out, spin, air) = run(&mut state, input());
    assert!((out.up_1152[1] - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.00001);
    assert!((out.up_1152[3] - 3. * std::f32::consts::SQRT_2).abs() < 0.00001);
    assert_eq!(out.dynamic_up_1136, [0., 1., 0., 2.]);
    assert_eq!(out.velocity_1184, [0.; 4]);
    assert_eq!(state.up_velocity, Vector3::ZERO);
    assert_eq!(state.ground_blend, 0.75);
    assert_eq!(&state.slow_filter.words()[..4], controls.as_slice());
    assert_eq!(
        &state.slow_filter.words()[4..8],
        &out.up_1152.map(f32::to_bits)
    );
    assert_eq!(
        &state.fast_filter.words()[8..12],
        &out.up_1152.map(f32::to_bits)
    );
    assert!(
        state.slow_filter.words()[12..16]
            .iter()
            .any(|&word| word != 0)
    );
    assert_eq!(f32::from_bits(spin.words()[30]), 0.25);
    assert_eq!(air.secondary_lean_angle, 0.);
    assert_eq!(air.spin_angle, 2.);
}

#[test]
fn cancellation_retains_previous_up_and_disabled_spin_still_advances_history() {
    let mut state = orientation();
    let mut input = input();
    input.requested_up = [0., -1., 0., -2.];
    input.enable_body_spin_input = false;
    let previous = input.previous_up;
    let (out, spin, _) = run(&mut state, input);
    assert_eq!(out.up_1152, previous);
    assert_eq!(out.target_1168, previous);
    assert_eq!(f32::from_bits(spin.words()[30]), 0.);
    assert_eq!(spin.words()[42], 1);
}
