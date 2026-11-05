//! CreateMtxFromHeadingAndUp82D61B60 and axis-angle frame blend82BD3150.
use super::*;
use skate3_core::animation::foot_ik::transforms::inverse_rigid;
use skate3_core::math::Basis3;
use skate3_core::physics::{
    drive_frames::retail_quaternion_from_basis, skeleton_animation_record::compose_affine,
};
pub(super) fn frame(mut heading: V, up: V) -> [V; 4] {
    if dot(heading, up).abs() > f32::from_bits(0x3f7fff58) {
        heading = [1.0, 0.0, 0.0, 0.0];
        if dot(heading, up).abs() > f32::from_bits(0x3f7fff58) {
            heading = [0.0, 0.0, 1.0, 0.0];
        }
    }
    let right = normalize(cross(up, heading));
    let forward = normalize(cross(right, up));
    [right, normalize(cross(forward, right)), forward, [0.0; 4]]
}
pub(super) fn blend(a: [V; 4], b: [V; 4], weight: f32) -> [V; 4] {
    if weight >= 1.0 {
        return b;
    }
    if weight <= 0.0 {
        return a;
    }
    let relative = compose_affine(&b, &inverse_rigid(&a));
    let q = retail_quaternion_from_basis(Basis3 {
        columns: std::array::from_fn(|i| relative[i][..3].try_into().unwrap()),
    });
    let sign = if q.w < 0.0 { -1.0 } else { 1.0 };
    let angle = 2.0 * skate3_core::trigonometry::acos((q.w * sign).clamp(-1.0, 1.0));
    let mut result = a;
    if angle < f32::from_bits(0x3d0efa35) {
        for i in 0..3 {
            result[i] = normalize(madd(sub(b[i], a[i]), weight, a[i]));
        }
    } else {
        let axis = normalize([q.x * sign, q.y * sign, q.z * sign, 0.0]);
        let (sin, cos) = skate3_core::trigonometry::sin_cos(angle * weight);
        for i in 0..3 {
            result[i] = madd(
                axis,
                dot(axis, a[i]) * (1.0 - cos),
                madd(cross(axis, a[i]), sin, scale(a[i], cos)),
            );
        }
    }
    result[3] = madd(sub(b[3], a[3]), weight, a[3]);
    result
}
