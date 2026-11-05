//! Original SkeletonState angular estimate82BEC0E8 and torque82BE3B58.
use super::math::{self, V};
use crate::physics::skeleton_body::{SkeletonBody, SkeletonPhysicalRecord};

///4656 contains the same fractional masses used by physical COM publication:
///original Init82BEBAA8 divides each4560 mass by total5216 at82BEBB80.
pub fn angular_velocity(record: &SkeletonPhysicalRecord, fractional: &[f32; 24]) -> V {
    let mut result = [0.0; 4];
    for part in 1..24 {
        let radial = math::sub(record.positions[part], record.centre_of_mass);
        let velocity = math::sub(record.velocities[part], record.centre_of_mass_velocity);
        let squared = math::dot(radial, radial);
        if squared > f32::from_bits(0x38D1_B717) {
            let term = math::scale(math::cross(radial, velocity), math::reciprocal(squared));
            result = math::madd(term, fractional[part], result);
        }
    }
    result
}

///82BE3B58 converts the control magnitude and direction into original
///per-part displacement accumulators. Actual solved velocities remain intact.
pub fn apply(body: &mut SkeletonBody, physical_com: V, control: V) {
    let magnitude = math::length(control);
    for part in 1..24 {
        let radial = math::sub(body.record.pose[part][3], physical_com);
        if math::dot(radial, radial) > 0.001 {
            let tangent = math::normalize_or(math::cross(control, radial), [0.0; 4]);
            body.apply_part_displacement(part, math::scale(tangent, magnitude));
        }
    }
}
