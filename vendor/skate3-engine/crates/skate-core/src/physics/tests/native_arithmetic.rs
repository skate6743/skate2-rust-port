use super::*;

#[test]
fn host_math_normalizes_a_direction_without_using_its_homogeneous_lane() {
    let direction = [3.0, 4.0, 0.0, 100.0];
    let inverse_length = reciprocal_square_root_estimate(dot3(direction, direction));
    let unit = direction.map(|v| v * inverse_length);
    assert!((dot3(unit, unit) - 1.0).abs() < 1.0e-6);
    assert_eq!(dot3(direction, [4.0, -3.0, 0.0, 50.0]), 0.0);
    assert_eq!(dot4(direction, [0.0, 0.0, 0.0, 1.0]), 100.0);
}

#[test]
fn host_math_handles_scalar_geometry_and_ordinary_float_boundaries() {
    assert_eq!(reciprocal_estimate(4.0), 0.25);
    assert_eq!(reciprocal_square_root_estimate(16.0), 0.25);
    assert_eq!(reciprocal_estimate(-0.0), f32::NEG_INFINITY);
    assert_eq!(reciprocal_square_root_estimate(f32::INFINITY), 0.0);
    assert!(reciprocal_square_root_estimate(-1.0).is_nan());
    assert_eq!(vector_min(-2.0, 3.0), -2.0);
    assert_eq!(vector_max(-2.0, 3.0), 3.0);
}
