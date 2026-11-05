//! Free-look compass82DF4100. Retains native orbit position, angular dead-zone,
//! thirty-degree limit relative to the last camera, and two-unit arm length.
use super::compass::{heading, horizontal, sub, wrap};
use super::vector_tracker::{dot, length, refined_reciprocal};
use super::{Compass, CompassInputs};

impl Compass {
    pub(super) fn update_orbit(&mut self, dt: f32, input: CompassInputs) {
        let position = input.transform[3];
        if input.selected_compass != 6 || dt == 0.0 {
            self.orbit_position = input.camera_position;
            self.headings[6] = heading(horizontal(sub(position, self.orbit_position)));
            self.orbit_delta = 0.0;
            return;
        }
        let stick = -input.look[0];
        let mut step = 0.0;
        let mut retained: f32 = 0.95;
        if stick.abs() > 0.3 {
            retained = 0.9;
            step =
                ((stick - if stick >= 0.0 { 0.3 } else { -0.3 }) * dt) * f32::from_bits(0x408f9d9c);
        }
        self.orbit_delta = wrap((1.0 - retained).mul_add(step, self.orbit_delta * retained));
        let half = self.orbit_delta * 0.5;
        let axis = [
            0.0,
            crate::trigonometry::sin(half),
            0.0,
            crate::trigonometry::cos(half),
        ];
        self.orbit_position = add(position, rotate(axis, sub(self.orbit_position, position)));
        let from = horizontal(sub(self.orbit_position, position));
        let previous = horizontal(sub(input.camera_position, position));
        let from_direction = normalize_safe(from);
        let previous_direction = normalize_safe(previous);
        let normal = cross(from_direction, previous_direction);
        if dot(from_direction, previous_direction) < f32::from_bits(0x3f5db22d) {
            let magnitude = length(normal);
            if magnitude >= 0.001 {
                let inverse = refined_reciprocal(magnitude);
                let half = -f32::from_bits(0x3f060a92) * 0.5;
                let sine = crate::trigonometry::sin(half);
                let axis = [
                    normal[0] * inverse * sine,
                    normal[1] * inverse * sine,
                    normal[2] * inverse * sine,
                    crate::trigonometry::cos(half),
                ];
                self.orbit_position = add(position, rotate(axis, previous));
            } else {
                self.orbit_position = input.camera_position;
            }
        }
        let relative = sub(position, self.orbit_position);
        let distance = length(relative);
        if distance > 0.001 {
            self.orbit_position = sub(position, relative.map(|v| v * (2.0 / distance)));
        }
        self.headings[6] = heading(horizontal(sub(position, self.orbit_position)));
    }
}
fn normalize_safe(v: [f32; 4]) -> [f32; 4] {
    if length(v) > f32::from_bits(0x358637bd) {
        super::orientation_math::normalize(v)
    } else {
        [0.0; 4]
    }
}
fn add(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    core::array::from_fn(|i| a[i] + b[i])
}
fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        0.0,
    ]
}
fn rotate(q: [f32; 4], v: [f32; 4]) -> [f32; 4] {
    let first = cross(q, v);
    let intermediate = core::array::from_fn(|i| q[3].mul_add(v[i], first[i]));
    let second = cross(q, intermediate);
    core::array::from_fn(|i| 2.0_f32.mul_add(second[i], v[i]))
}
