use super::*;

fn linear_settings() -> SpeedAndSlopeSettings {
    let keys = [0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 1.0];
    SpeedAndSlopeSettings {
        turn_torque_vs_speed: PointGraph { x: keys, y: keys },
        turn_torque_vs_slope: PointGraph { x: keys, y: keys },
        heading_adjust_max_speed: 8.0,
    }
}

#[test]
fn coefficient_uses_absolute_forward_speed_and_saturates_its_fraction() {
    let settings = linear_settings();
    let half_speed = settings.calculate(0.0, 4.0);
    assert!((half_speed - 0.5).abs() < 0.00001);
    assert_eq!(settings.calculate(0.0, -4.0), half_speed);
    assert!((settings.calculate(0.0, 80.0) - 1.0).abs() < 0.00001);
    assert_eq!(settings.calculate(0.0, 0.0), 0.0);
}

#[test]
fn over_vertical_slope_returns_to_the_first_curve_key() {
    let mut settings = linear_settings();
    settings.turn_torque_vs_slope.y[0] = 0.375;
    // acos(-1)/(pi/2) exceeds1.2. The native cutoff selects zero,
    // so an inverted board uses the first graph value, not its last value.
    assert_eq!(settings.calculate(-1.0, 8.0), 0.375);
    assert_eq!(settings.calculate(-2.0, 8.0), 0.375);
    assert!((settings.calculate(0.0, 8.0) - 1.0).abs() < 0.00001);
}
