use super::*;

fn settings() -> Settings {
    Settings {
        starting_value: 0.0,
        default_value: 0.0,
        scale: 1.0,
        filters: [0; 4],
        ramp_time: None,
        blend_rising: 0.5,
        blend_falling: 0.25,
        blend_out: Some(0.125),
        clamp_velocity: None,
        clamp_acceleration: None,
    }
}

#[test]
fn starting_value_bypasses_scale_and_filters_and_reentry_resets_history() {
    let mut s = State {
        elapsed: 4.0,
        previous_delta: 9.0,
        value: 7.0,
    };
    let mut p = settings();
    p.starting_value = 2.0;
    p.scale = 3.0;
    p.filters = [1; 4];
    assert_eq!(s.begin(&p), 2.0);
    assert_eq!(s.elapsed, 0.0);
    assert_eq!(s.previous_delta, 0.0);
}

#[test]
fn missing_input_uses_blend_out_without_ramp_and_zero_is_present() {
    let mut p = settings();
    p.ramp_time = Some(8.0);
    let mut s = State {
        value: 1.0,
        ..State::default()
    };
    assert_eq!(s.update(&p, None, 1.0, (false, false)), 0.875);
    s = State {
        value: 1.0,
        ..State::default()
    };
    assert_eq!(s.update(&p, Some(0.0), 1.0, (false, false)), 0.96875);
}

#[test]
fn acceleration_precedes_velocity_and_limits_are_not_scaled_by_dt() {
    let mut p = settings();
    p.blend_rising = 1.0;
    p.clamp_acceleration = Some(0.25);
    p.clamp_velocity = Some(0.125);
    let mut s = State::default();
    assert_eq!(s.update(&p, Some(4.0), 0.0, (false, false)), 0.125);
    assert_eq!(s.update(&p, Some(4.0), 3.0, (false, false)), 0.25);
    assert_eq!(s.update(&p, Some(-4.0), 0.0, (false, false)), 0.125);
}

#[test]
fn scale_and_ordered_filters_use_live_stance() {
    let mut p = settings();
    p.blend_rising = 1.0;
    p.blend_falling = 1.0;
    p.scale = 2.0;
    p.filters = [1, 2, 3, 1];
    let mut s = State::default();
    assert_eq!(s.update(&p, Some(0.125), 0.0, (false, false)), 0.25);
    assert_eq!(s.update(&p, Some(0.125), 0.0, (true, true)), -0.75);
}

#[test]
fn ramp_has_native_endpoint_and_signed_bit_ordering() {
    assert_eq!(ramp(0.0, 2.0), 0.0);
    assert_eq!(ramp(0.5, 2.0), 0.25);
    assert_eq!(ramp(2.0, 2.0), 1.0);
    assert_eq!(ramp(3.0, 2.0), 1.0);
    assert_eq!(ramp(-1.0, 2.0), 0.0);
    assert_eq!(ramp(0.0, 0.0), 0.0);
    assert_eq!(ramp(1.0, 0.0), 1.0);
}
