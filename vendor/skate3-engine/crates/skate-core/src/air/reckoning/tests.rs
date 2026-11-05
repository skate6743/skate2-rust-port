use super::*;
use crate::{
    air::{body_flip::BodyFlipSettings, body_spin::BodySpinSettings},
    physics::skeleton_animation_record::IDENTITY,
    point_graph::PointGraph,
    riding::ground_orientation::GroundOrientationSettings,
};
fn curve(value: f32) -> PointGraph<8> {
    PointGraph {
        x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
        y: [value; 8],
    }
}
fn orientation() -> GroundOrientation {
    GroundOrientation::new(&GroundOrientationSettings {
        ground_normal_smoothing: [1.0, 0.0, 0.0, 1.0],
        up_vector_smoothing_slow: [0.2, 0.1, 0.05, 0.3],
        up_vector_smoothing_fast: [0.7, 0.2, 0.1, 0.8],
        dynamic_up_vs_ground_y: curve(0.0),
        ground_vector_blend: curve(0.0),
        deck_angle_usage_vs_speed: curve(0.0),
        up_vector_smoothing_vs_speed: curve(0.0),
        up_vector_max_delta_vs_speed: curve(1.0),
        ground_blend_max_delta: 0.0,
        up_vector_max_acceleration: 1.0,
        anti_wobble_damping: 0.5,
        extra_side_damping: 0.0,
        minimum_wheels_for_ground_blend: 1,
    })
}
fn settings() -> Settings {
    Settings {
        ground_normal_smoothing: [1.0, 0.0, 0.0, 1.0],
        max_up_angle_delta: curve(0.5),
        tilt_vs_rotation: curve(0.0),
        tilt_vs_slope: curve(0.0),
        body_spin: BodySpinSettings {
            derivative_floor: 0.4,
            acceleration_limit: 0.2,
            curves: [curve(1.0); 7],
            input_fade_threshold: [f32::from_bits(0x37800000); 4],
        },
        body_flip: BodyFlipSettings {
            smoothing: Some(0.1),
            maximum_speed: Some(5.0),
            spin_scale: Some(0.5),
            missing_attribute_value: 0.0,
        },
    }
}
fn input() -> Input {
    Input {
        landing_normal: [0.0, 1.0, 0.0, 0.0],
        normal_blend: 0.1,
        target_spin: 0.0,
        flip_request: 0.0,
        com_to_deck: [0.0, 1.0, 0.0, 0.0],
        timestep: 1.0 / 60.0,
        physical_body_spin: 0.0,
        grind_adjusted_body_spin: 0.0,
        additive_spin: false,
        direct_spin: true,
        reverse_stance: false,
        easy_body_spins: true,
        perfect_body_flips: false,
    }
}

#[test]
fn angle_limit_rotates_from_toward_target_and_preserves_parallel_branch() {
    let target = [0.0, 0.0, 1.0, 0.0];
    let from = [1.0, 0.0, 0.0, 0.0];
    let result = math::limit_angle(target, from, std::f32::consts::FRAC_PI_4);
    assert!((result[0] - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.0001);
    assert!((result[2] - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.0001);
    assert_eq!(
        math::limit_angle([-2.0, 0.0, 0.0, 0.0], from, 0.1),
        [-2.0, 0.0, 0.0, 0.0]
    );
}
#[test]
fn direct_and_additive_spin_use_fixed_angle_step_and_keep_body_spin_history() {
    let mut orientation = orientation();
    let mut frames = ReckoningFrames::new();
    let mut spin = BodySpinState::new();
    let mut state = AirState::new();
    let mut input = input();
    input.target_spin = 2.0;
    input.timestep = 0.1;
    let old_history = *spin.words();
    update(
        &mut orientation,
        &mut frames,
        &mut spin,
        &mut state,
        &settings(),
        &input,
    );
    assert_eq!(state.spin_angle, 2.0 * f32::from_bits(0x3c888889));
    assert_eq!(spin.words(), &old_history);
    //Heading uses caller dt even though accumulated spin angle uses1/60.
    assert!((frames.heading[0] - 0.2_f32.cos()).abs() < 0.0001);
    assert!((frames.heading[2] + 0.2_f32.sin()).abs() < 0.0001);
    input.additive_spin = true;
    input.grind_adjusted_body_spin = 3.0;
    update(
        &mut orientation,
        &mut frames,
        &mut spin,
        &mut state,
        &settings(),
        &input,
    );
    assert_eq!(state.spin_speed, 5.0);
    assert_eq!(spin.words(), &old_history);
}
#[test]
fn airborne_filter_update_retains_controls_and_publishes_current_normal() {
    let mut orientation = orientation();
    let mut frames = ReckoningFrames::new();
    let mut spin = BodySpinState::new();
    let mut state = AirState::new();
    let mut input = input();
    let slow_control: Vec<_> = orientation.slow_filter.words()[..4].to_vec();
    let fast_control: Vec<_> = orientation.fast_filter.words()[..4].to_vec();
    input.landing_normal = [0.0, 0.8, 0.6, 0.0];
    update(
        &mut orientation,
        &mut frames,
        &mut spin,
        &mut state,
        &settings(),
        &input,
    );
    assert!(orientation.ground_normal.z > 0.59);
    assert!(orientation.up.z > 0.0 && orientation.up.z < orientation.ground_normal.z);
    assert_eq!(
        &orientation.slow_filter.words()[..4],
        slow_control.as_slice()
    );
    assert_eq!(
        &orientation.fast_filter.words()[..4],
        fast_control.as_slice()
    );
    assert_eq!(
        &orientation.slow_filter.words()[4..8],
        &lanes(orientation.up).map(f32::to_bits)
    );
    assert_eq!(frames.ground[1], lanes(orientation.ground_normal));
}
#[test]
fn active_zero_flip_preserves_combined_frame_and_spin_reset_preserves_flip() {
    let mut orientation = orientation();
    let mut frames = ReckoningFrames::new();
    let mut spin = BodySpinState::new();
    let mut state = AirState::new();
    let mut input = input();
    state.flip_active = true;
    input.target_spin = 1.0;
    update(
        &mut orientation,
        &mut frames,
        &mut spin,
        &mut state,
        &settings(),
        &input,
    );
    assert_eq!(frames.body_flip, IDENTITY);
    assert_eq!(frames.heading, [1.0, 0.0, 0.0, 0.0]);
    state.flip_angle = 0.3;
    state.flip_speed = 0.2;
    state.reset_spin();
    assert_eq!([state.spin_angle, state.spin_speed], [0.0; 2]);
    assert_eq!([state.flip_angle, state.flip_speed], [0.3, 0.2]);
    assert!(state.flip_active);
}
