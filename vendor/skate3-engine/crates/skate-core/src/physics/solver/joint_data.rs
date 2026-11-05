//! Decoded joint/drive geometry and shared reaction application.
//!
//! Record boundaries are four-float columns because native solver stores also
//! update their fourth lanes. Physical vectors/impulses have three components;
//! the fourth values are packed carry fields, not extra spatial dimensions.

use crate::math::Vector3;

pub(in crate::physics::solver) fn read_column(words: &[u32], byte_offset: usize) -> [f32; 4] {
    core::array::from_fn(|component| f32::from_bits(words[byte_offset / 4 + component]))
}

pub(in crate::physics::solver) fn write_column(
    words: &mut [u32],
    byte_offset: usize,
    values: [f32; 4],
) {
    for (destination, value) in words[byte_offset / 4..byte_offset / 4 + 4]
        .iter_mut()
        .zip(values)
    {
        *destination = value.to_bits();
    }
}

pub(in crate::physics::solver) fn xyz(values: [f32; 4]) -> Vector3 {
    Vector3::new(values[0], values[1], values[2])
}

/// `82AE1AE8` and `82AE3BC8` write these same response fields. Columns at
/// 64/96/128 and 80/112/144 project relative corrections into constraint axes;
/// 192..224 and 240..272 convert impulse changes back into world space.
pub(in crate::physics::solver) struct ConstraintGeometry {
    pub arm_a: Vector3,
    pub arm_b: Vector3,
    pub linear_projection: [[f32; 4]; 3],
    pub angular_projection: [[f32; 4]; 3],
    linear_axes: [[f32; 4]; 3],
    angular_axes: [[f32; 4]; 3],
    inverse_inertia_a: [[f32; 4]; 3],
    inverse_inertia_b: [[f32; 4]; 3],
}

impl ConstraintGeometry {
    pub fn decode(words: &[u32]) -> Self {
        assert_eq!(words.len(), 96);
        Self {
            arm_a: xyz(read_column(words, 0)),
            arm_b: xyz(read_column(words, 16)),
            linear_projection: core::array::from_fn(|axis| read_column(words, 64 + axis * 32)),
            angular_projection: core::array::from_fn(|axis| read_column(words, 80 + axis * 32)),
            linear_axes: core::array::from_fn(|axis| read_column(words, 192 + axis * 16)),
            angular_axes: core::array::from_fn(|axis| read_column(words, 240 + axis * 16)),
            inverse_inertia_a: core::array::from_fn(|axis| read_column(words, 288 + axis * 16)),
            inverse_inertia_b: core::array::from_fn(|axis| read_column(words, 336 + axis * 16)),
        }
    }

    /// Both candidates must read this incoming snapshot before either impulse
    /// is applied: 82AE2C00..2CC4 (joint), 82AE2E90..2F80 (drive).
    pub fn relative_corrections(&self, a: &ReactionState, b: &ReactionState) -> (Vector3, Vector3) {
        let point_a = point_correction(a, self.arm_a);
        let point_b = point_correction(b, self.arm_b);
        (
            Vector3::new(
                point_b.x - point_a.x,
                point_b.y - point_a.y,
                point_b.z - point_a.z,
            ),
            Vector3::new(
                b.angular[0] - a.angular[0],
                b.angular[1] - a.angular[1],
                b.angular[2] - a.angular[2],
            ),
        )
    }

    /// 82AE2D24..2E40 / 82AE2FE0..30D4. One linear and one angular impulse
    /// change affect both bodies through their own arm and inverse inertia.
    pub fn apply_impulse_changes(
        &self,
        a: &mut ReactionState,
        b: &mut ReactionState,
        linear_change: Vector3,
        angular_change: Vector3,
    ) {
        let linear_impulse = project_correction(&self.linear_axes, linear_change, [0.0; 4]);
        let angular_impulse = xyz(project_correction(
            &self.angular_axes,
            angular_change,
            [0.0; 4],
        ));
        let torque_a = torque_at_offset(self.arm_a, xyz(linear_impulse), angular_impulse);
        let torque_b = torque_at_offset(self.arm_b, xyz(linear_impulse), angular_impulse);
        let inverse_mass_a = self.inverse_inertia_a[0][3];
        let inverse_mass_b = self.inverse_inertia_b[0][3];
        // Native publication order: linear A, linear B, angular A, angular B.
        for (component, impulse) in linear_impulse.into_iter().enumerate() {
            a.linear[component] = impulse.mul_add(inverse_mass_a, a.linear[component]);
            b.linear[component] = (-impulse).mul_add(inverse_mass_b, b.linear[component]);
        }
        a.angular = project_correction(&self.inverse_inertia_a, torque_a, a.angular);
        for component in 0..4 {
            let after_x =
                (-self.inverse_inertia_b[0][component]).mul_add(torque_b.x, b.angular[component]);
            let after_y = (-self.inverse_inertia_b[1][component]).mul_add(torque_b.y, after_x);
            b.angular[component] =
                (-self.inverse_inertia_b[2][component]).mul_add(torque_b.z, after_y);
        }
    }
}

/// Velocity-producing displacement corrections at reaction offsets 0/32.
/// Contact-only pose corrections at 16/48 are neither read nor overwritten.
pub(in crate::physics::solver) struct ReactionState {
    linear: [f32; 4],
    angular: [f32; 4],
}

impl ReactionState {
    pub fn decode(words: &[u32]) -> Self {
        assert_eq!(words.len(), 16);
        Self {
            linear: read_column(words, 0),
            angular: read_column(words, 32),
        }
    }
}

pub(in crate::physics::solver) fn publish_reactions(
    a: &ReactionState,
    b: &ReactionState,
    words_a: &mut [u32],
    words_b: &mut [u32],
) {
    write_column(words_a, 0, a.linear);
    write_column(words_b, 0, b.linear);
    write_column(words_a, 32, a.angular);
    write_column(words_b, 32, b.angular);
}

/// Matrix projection with the native X, Y, Z fused accumulation order. The
/// incoming accumulated impulse is the initial value for candidate projection.
pub(in crate::physics::solver) fn project_correction(
    columns: &[[f32; 4]; 3],
    correction: Vector3,
    initial: [f32; 4],
) -> [f32; 4] {
    core::array::from_fn(|component| {
        let after_x = columns[0][component].mul_add(correction.x, initial[component]);
        let after_y = columns[1][component].mul_add(correction.y, after_x);
        columns[2][component].mul_add(correction.z, after_y)
    })
}

fn point_correction(reaction: &ReactionState, arm: Vector3) -> Vector3 {
    let [x, y, z, _] = reaction.angular;
    Vector3::new(
        (-z).mul_add(arm.y, y.mul_add(arm.z, reaction.linear[0])),
        (-x).mul_add(arm.z, z.mul_add(arm.x, reaction.linear[1])),
        (-y).mul_add(arm.x, x.mul_add(arm.y, reaction.linear[2])),
    )
}

fn torque_at_offset(arm: Vector3, impulse: Vector3, angular_impulse: Vector3) -> Vector3 {
    Vector3::new(
        (-arm.z).mul_add(impulse.y, arm.y.mul_add(impulse.z, angular_impulse.x)),
        (-arm.x).mul_add(impulse.z, arm.z.mul_add(impulse.x, angular_impulse.y)),
        (-arm.y).mul_add(impulse.x, arm.x.mul_add(impulse.y, angular_impulse.z)),
    )
}
