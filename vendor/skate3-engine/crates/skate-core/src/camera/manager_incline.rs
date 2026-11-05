//! Ground incline and frontside state, complete TU3 82E002A8.
use super::manager_state::{HALF_PI, UP};
use super::vector_tracker::{dot, length};
use super::{ManagerState, ManagerSubject, direction_to_angles};

impl ManagerState {
    pub(super) fn update_incline(&mut self, subject: &ManagerSubject) {
        self.ground_normal = subject.ground_normal;
        let normal = subject.rig.last_valid_ground_up;
        if normal[1] > f32::from_bits(0x3f7fff58) && subject.rig.grinding == 0 {
            self.velocity_incline = 0.0;
            self.absolute_velocity_incline = 0.0;
            self.frontside_angle = 0.0;
            self.ground_heading = 0.0;
            self.direction_incline = 0.0;
            self.flags &= !0x20;
            return;
        }
        let direction = if length(self.anchor_velocity) > f32::from_bits(0x358637bd) {
            super::orientation_math::normalize(self.anchor_velocity)
        } else {
            subject.rig.transform[2]
        };
        self.velocity_incline = HALF_PI - angle_between(UP, direction);
        self.absolute_velocity_incline = if self.velocity_incline < 0.0 {
            -self.velocity_incline
        } else {
            self.velocity_incline
        };
        self.ground_heading = direction_to_angles(normal)[1];
        // Projection and subtraction are separate VMX operations, followed
        // by the two-refinement normalization used inline at82E00554.
        let projected = normal.map(|v| v * dot(UP, normal));
        let tangent = super::orientation_math::normalize(core::array::from_fn::<_, 4, _>(|i| {
            UP[i] - projected[i]
        }));
        let cross = [
            (-normal[2]).mul_add(tangent[1], normal[1] * tangent[2]),
            (-normal[0]).mul_add(tangent[2], normal[2] * tangent[0]),
            (-normal[1]).mul_add(tangent[0], normal[0] * tangent[1]),
            0.0,
        ];
        let denominator = dot(direction, tangent);
        let numerator = dot(direction, cross);
        self.frontside_angle = atan_ratio(numerator, denominator);
        self.flags = (self.flags & !0x20) | (u8::from(self.frontside_angle > 0.0) << 5);
        if self.mirrored() {
            self.frontside_angle *= -1.0;
        }
        // vrlimi mask1 replaces W with X; this condition tests XYZ only.
        if self.anchor_velocity[..3]
            .iter()
            .any(|v| v.abs() > f32::from_bits(0x34000000))
        {
            self.direction_incline =
                super::normalize_angle(HALF_PI - angle_between(subject.direction_424, UP));
        }
    }
}

pub(super) fn angle_between(a: [f32; 4], b: [f32; 4]) -> f32 {
    let to_vector = |v: [f32; 4]| crate::math::Vector3::new(v[0], v[1], v[2]);
    crate::physics::board_ground::angle_between(to_vector(a), to_vector(b))
}

/// Inline atan wrapper82E005A4..82E00628: one reciprocal Newton step,
/// multiply-add with positive zero, then signed quadrant selection.
pub(super) fn atan_ratio(numerator: f32, denominator: f32) -> f32 {
    let initial = crate::physics::native_arithmetic::reciprocal_estimate(denominator);
    let inverse = initial.mul_add((-initial).mul_add(denominator, 1.0), initial);
    let mut angle = crate::input::angle::atan(numerator.mul_add(inverse, 0.0));
    let sign = numerator.to_bits() & 0x80000000;
    if 0.0 > denominator {
        angle = f32::from_bits(0x40490fdb | sign) + angle;
    }
    if denominator == 0.0 {
        angle = f32::from_bits(0x3fc90fdb | sign);
    }
    angle
}
