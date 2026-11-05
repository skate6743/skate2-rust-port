//! Board-owned fields consumed by PhysicalPlayerHiLOD::Input, from original
//! TU3 Skateboard::FillPhysOut82C02A80. Other physical components publish into
//! the same packet after its native per-frame reset; their fields are retained.
use super::{
    board::BodyId,
    board_ground::BoardGroundState,
    board_motion_output::{BoardMotionOutput, dot, scale, subtract},
};
use crate::{
    math::Vector3, player::input_phase::PhysicalPlayerInput,
    riding::collision_response::signed_angle,
};

/// Processed96 was captured by PrepareBoardToolkit before simulation. It is
/// deliberately separate from the current solved board transform. Ground64 is
/// the retained normal calculated by82C02388, not the wheel-contact normal.
pub struct BoardInputFrame {
    pub processed_forward: Vector3,
    pub reckoning_normal_1216: Vector3,
    pub reckoning_ground_up: Vector3,
    pub retained_ground_normal_112: Vector3,
    pub processed_flags_2476: u32,
}

pub fn publish_board_input(
    out: &mut PhysicalPlayerInput,
    motion: &BoardMotionOutput,
    contacts: &BoardGroundState,
    frame: BoardInputFrame,
) {
    let side = motion.effective_basis.columns[0];
    let sign = if frame.processed_flags_2476 & 4 != 0 {
        -1.0
    } else {
        1.0
    };
    out.board_reckoning_side_176 =
        [side[0] * sign, side[1] * sign, side[2] * sign, 0.0].map(f32::to_bits);
    out.skateboard.vector_64 = bits(motion.angular_velocity);
    out.skateboard.vector_80 = bits(motion.linear_velocity);
    out.skateboard.vector_96 = bits(motion.ground_velocity);
    out.skateboard.scalar_160 = motion.speed;
    out.skateboard.scalar_164 = motion.ground_speed;
    out.skateboard.scalar_168 = motion.forward_speed;
    out.skateboard.scalar_172 = travel_angle(
        frame.processed_forward,
        motion.ground_velocity,
        frame.reckoning_normal_1216,
        frame.reckoning_ground_up,
    );
    //82C02A80 never writes Motion128/144. Preserve the values from the
    //output reset and any other physical component, rather than inventing them.
    out.ground.vector_64 = bits(frame.retained_ground_normal_112);
    out.ground.vector_96 = bits(contacts.wheel_normal);
    out.collision.wheel_contact_3296_3299 =
        core::array::from_fn(|i| u8::from(contacts.parts[i].in_contact));
    out.collision.wheel_count_0 = u32::from(contacts.wheel_contact_count);
    out.collision.flag_3472 = u8::from(
        contacts.parts[BodyId::FrontTruck.index()].in_contact
            || contacts.parts[BodyId::BackTruck.index()].in_contact,
    );
    out.collision.flag_3475 = u8::from(contacts.parts[BodyId::Deck.index()].in_contact);
    out.collision.flag_3477 = u8::from(
        out.collision.wheel_count_0 > 0
            || out.collision.flag_3472 != 0
            || out.collision.flag_3475 != 0,
    );
}

///82C030B8..322C, including8286CD88's product gate and plane rejection.
fn travel_angle(forward: Vector3, velocity: Vector3, normal: Vector3, ground_up: Vector3) -> f32 {
    let mut projected_forward = reject(forward, normal);
    if !(dot(projected_forward, velocity) > 0.0) {
        projected_forward = scale(projected_forward, -1.0);
    }
    let angle = if dot(projected_forward, projected_forward) * dot(velocity, velocity)
        > f32::from_bits(0x3780_0000)
    {
        signed_angle(
            reject(projected_forward, ground_up),
            reject(velocity, ground_up),
            ground_up,
        )
    } else {
        0.0
    };
    let turns = angle * f32::from_bits(0x3e22_f983);
    let fraction = turns - turns.floor();
    (fraction - if fraction > 0.5 { 1.0 } else { 0.0 }) * f32::from_bits(0x40c9_0fdb)
}
fn reject(vector: Vector3, normal: Vector3) -> Vector3 {
    subtract(vector, scale(normal, dot(vector, normal)))
}
fn bits(vector: Vector3) -> [u32; 4] {
    [vector.x, vector.y, vector.z, 0.0].map(f32::to_bits)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forward_and_reverse_motion_use_the_same_travel_axis() {
        let forward = Vector3::new(0., 0., 1.);
        let normal = Vector3::new(0., 1., 0.);
        let a = travel_angle(forward, Vector3::new(1., 0., 4.), normal, normal);
        let b = travel_angle(forward, Vector3::new(-1., 0., -4.), normal, normal);
        assert!((a - b).abs() < 0.0001);
        assert!(a > 0. && a < 0.5);
        assert_eq!(travel_angle(forward, Vector3::ZERO, normal, normal), 0.);
    }
}
