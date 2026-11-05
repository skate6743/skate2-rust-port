//! Final CameraMan frame arithmetic82DFF88C..82DFFF28, Euler82DFCD00.
use crate::math::Basis3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraFrame {
    pub basis: Basis3,
    pub position: [f32; 4],
    pub previous_basis: Basis3,
    pub previous_position: [f32; 4],
    pub linear_velocity: [f32; 4],
    pub angular_velocity: [f32; 4],
    pub shake_translation: [f32; 4],
    pub discontinuity: bool,
    pub field_of_view_degrees: f32,
    pub opacity: f32,
    pub blur: f32,
}
impl CameraFrame {
    pub(super) fn new() -> Self {
        let basis = Basis3 {
            columns: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        };
        Self {
            basis,
            position: [0.0; 4],
            previous_basis: basis,
            previous_position: [0.0; 4],
            linear_velocity: [0.0; 4],
            angular_velocity: [0.0; 4],
            shake_translation: [0.0; 4],
            discontinuity: false,
            field_of_view_degrees: 0.0,
            opacity: 1.0,
            blur: 1.0,
        }
    }
    pub(super) fn motion(&mut self, dt: f32, basis: Basis3, position: [f32; 4]) {
        let inverse = if dt != 0.0 { 1.0 / dt } else { 1.0 };
        self.linear_velocity = core::array::from_fn(|i| (position[i] - self.position[i]) * inverse);
        let old = angles(self.basis);
        let new = angles(basis);
        self.angular_velocity = core::array::from_fn(|i| {
            if i == 3 {
                0.0
            } else {
                super::normalize_angle(new[i] - old[i]) * inverse
            }
        });
    }
}

pub(super) fn lens_field_of_view(lens: f32, aspect: f32) -> f32 {
    let angle = crate::input::angle::atan(35.0 / (lens * 2.0));
    let degrees = angle * f32::from_bits(0x42e52ee1);
    square_root((degrees * degrees) / (aspect + 1.0))
}

fn angles(basis: Basis3) -> [f32; 3] {
    let [right, up, at] = basis.columns;
    let horizontal = square_root(right[0].mul_add(right[0], right[1] * right[1]));
    let yaw = super::manager_incline::atan_ratio(-right[2], horizontal);
    if horizontal > f32::from_bits(0x3a83126f) {
        [
            super::manager_incline::atan_ratio(up[2], at[2]),
            yaw,
            super::manager_incline::atan_ratio(right[1], right[0]),
        ]
    } else {
        [super::manager_incline::atan_ratio(-at[1], up[1]), yaw, 0.0]
    }
}

fn square_root(value: f32) -> f32 {
    let mut inverse = crate::physics::reciprocal_sqrt::estimate(value);
    for _ in 0..2 {
        inverse = (inverse * 0.5).mul_add((-value).mul_add(inverse * inverse, 1.0), inverse);
    }
    if value == 0.0 { 0.0 } else { value * inverse }
}
