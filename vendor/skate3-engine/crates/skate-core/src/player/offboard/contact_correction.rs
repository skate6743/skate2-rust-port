//! Biped contact correction82D7C9E0, called before slide/movement/cadence.
//! Input displacement vectors come from actual skeleton collision output.
use crate::physics::{native_arithmetic, reciprocal_sqrt::estimate};
type Vector = [f32; 4];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactCorrection {
    /// Biped708, copied by common ProcessOutput into OffBoard333.
    pub active: bool,
    /// Biped400, copied into OffBoard288 even when inactive; retain old value.
    pub direction: Vector,
    /// Biped384, recomputed every ground update.
    pub displacement: Vector,
}

#[derive(Clone, Copy, Debug)]
pub struct Input {
    /// Ground job192/208 from skeleton16288/16304.
    pub collision_displacements: [Vector; 2],
    pub projection_axis_416: Vector,
    pub up_axis_16: Vector,
}

impl ContactCorrection {
    pub fn update(&mut self, input: Input) {
        self.displacement = [0.0; 4];
        let [a, b] = input.collision_displacements;
        let square_a = dot(a, a);
        let square_b = dot(b, b);
        //The second vector wins equality and unordered comparison.
        let selected = if square_a > square_b { a } else { b };
        let maximum = select(square_a - square_b, square_a, square_b);
        self.active = false;
        //82D7CA7C ble skips the branch only for ordered <=.
        if !(maximum <= f32::from_bits(0x38d1_b717)) {
            let inverse = inverse_length(dot(selected, selected));
            self.direction = selected.map(|value| value * inverse);
            self.active = true;
            let length = magnitude(maximum);
            let excess = length - f32::from_bits(0x3d4c_cccd);
            let excess = select(excess, excess, 0.0);
            let displacement = self.direction.map(|value| (value * excess) * 0.5);
            self.displacement = clamp_length(
                reject(displacement, input.projection_axis_416),
                f32::from_bits(0x3dcc_cccd),
            );
        }
        self.displacement = reject(self.displacement, input.up_axis_16);
    }
}

fn dot(a: Vector, b: Vector) -> f32 {
    native_arithmetic::dot3(a, b)
}
fn select(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}
fn reject(vector: Vector, axis: Vector) -> Vector {
    let projection = dot(axis, vector);
    std::array::from_fn(|i| vector[i] - axis[i] * projection)
}
pub(super) fn inverse_length(squared: f32) -> f32 {
    let mut inverse = estimate(squared);
    for _ in 0..2 {
        inverse = (inverse * 0.5).mul_add((-squared).mul_add(inverse * inverse, 1.0), inverse);
    }
    inverse
}
pub(super) fn magnitude(squared: f32) -> f32 {
    let length = squared * inverse_length(squared);
    if squared == 0.0 { 0.0 } else { length }
}

/// Original82BD3D90, preserving four lanes and two reciprocal refinements.
pub(super) fn clamp_length(vector: Vector, maximum: f32) -> Vector {
    let length = magnitude(dot(vector, vector));
    if !(length >= f32::from_bits(0x3780_0000)) {
        return vector;
    }
    let bounded = select(maximum - length, length, maximum);
    let mut inverse = native_arithmetic::reciprocal_estimate(length);
    for _ in 0..2 {
        inverse = inverse.mul_add((-inverse).mul_add(length, 1.0), inverse);
    }
    vector.map(|value| (value * bounded) * inverse)
}

#[cfg(test)]
#[path = "contact_correction/tests.rs"]
mod tests;
