//! Original alternate air-control82D3DD40, including its contact early return.
use super::{
    State,
    math::{self, V},
    torque,
};
use crate::physics::skeleton_body::SkeletonBody;
const UP: V = [0.0, 1.0, 0.0, 0.0];
pub fn update(
    state: &mut State,
    body: &mut SkeletonBody,
    physical_com: V,
    input: [f32; 2],
    contact: bool,
) -> bool {
    let spine = math::normalize_or(
        math::sub(body.record.pose[11][3], body.record.pose[23][3]),
        [0.0; 4],
    );
    state.angular_velocity =
        torque::angular_velocity(&body.record, &body.definition.animation_masses.fractional);
    if contact {
        return false;
    }
    let direction = math::normalize_or(state.velocity, [0.0; 4]);
    let mut sideways = math::normalize_or(math::cross(UP, spine), [0.0; 4]);
    sideways[1] = 0.0;
    sideways = math::normalize_or(sideways, [0.0; 4]);
    let velocity_side = math::cross(UP, direction);
    if math::dot(velocity_side, sideways) < 0.0 {
        sideways = math::scale(sideways, -1.0);
    }
    let plane_normal = math::normalize_or(velocity_side, [0.0; 4]);
    let projected = math::sub(
        spine,
        math::scale(plane_normal, math::dot(plane_normal, spine)),
    );
    let projected = math::normalize_or(projected, [0.0; 4]);
    let mut automatic = math::scale(math::cross(spine, projected), 0.5);
    let hips_up = body.record.pose[23][1];
    if input[0].abs() < 0.5 && hips_up[1] < 0.0 {
        let projected = math::sub(
            hips_up,
            math::scale(plane_normal, math::dot(plane_normal, hips_up)),
        );
        let projected = math::normalize_or(projected, [0.0; 4]);
        automatic = math::madd(math::cross(hips_up, projected), 0.5, automatic);
    }
    //Both original scalar limits820993E8/EC are8 degrees.
    let limit = 8.0 * f32::from_bits(0x3C8E_FA35);
    let side = math::scale(sideways, limit * input[1]);
    let negative_spine = spine.map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000));
    let target = math::add(
        math::madd(negative_spine, limit * input[0], side),
        automatic,
    );
    let target = math::scale(target, f32::from_bits(0x426F_FFFF));
    let delta = math::scale(
        math::scale(
            math::sub(target, state.angular_velocity),
            f32::from_bits(0x4019_999A),
        ),
        f32::from_bits(0x3C88_8889),
    );
    torque::apply(body, physical_com, math::limit_length(delta, 0.5));
    true
}
