use super::Family;
pub(super) type V = [f32; 4];
pub(super) use skate3_core::riding::ground_correction_math::dot_product as dot;

/// Consumed subset of the144-byte record copied by82DEEAF8. Ground80 and
/// Skeleton144/160 are copied by the original but not read by these producers.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ApproachPose {
    pub right: V,
    pub position: V,
    pub feet: [V; 2],
    pub fakie: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Input {
    pub category: u32,
    pub grinding_316: bool,
    pub air_event_439: bool,
    ///None represents the native reset/unknown family, not FiftyFifty.
    pub family: Option<Family>,
    pub basic_right_0: V,
    pub basic_forward_32: V,
    pub basic_position_48: V,
    pub basic_location_axis_96: V,
    pub basic_twist_axis_128: V,
    pub basic_location_position_144: V,
    pub feet_256_272: [V; 2],
    pub fakie_155: bool,
    pub point: V,
    pub direction: V,
    pub normal: V,
    pub across: V,
}

///82DEED98: twist uses Basic128, not Basic32 or a substituted deck-forward.
pub(super) fn straight(input: Input) -> bool {
    let projected = sub(
        input.basic_twist_axis_128,
        input
            .normal
            .map(|v| v * dot(input.basic_twist_axis_128, input.normal)),
    );
    let length = skate3_core::physics::board_motion_output::length(skate3_core::math::Vector3::new(
        projected[0],
        projected[1],
        projected[2],
    ));
    if length <= 0.01 {
        return true;
    }
    let mut inverse = length.recip();
    for _ in 0..2 {
        let correction = (-inverse).mul_add(length, 1.0);
        inverse = inverse.mul_add(correction, inverse);
    }
    let along = dot(projected.map(|v| v * inverse), input.direction);
    along.abs() > 0.9397
}

///82DEEEE0: sign the Basic32 axis from Basic48 minus contact.
pub(super) fn low(input: Input) -> bool {
    let axis = signed(
        input.basic_forward_32,
        dot(
            sub(input.basic_position_48, input.point),
            input.basic_forward_32,
        ) > 0.0,
    );
    dot(input.normal, axis) <= -0.1
}
pub(super) fn add(a: V, b: V) -> V {
    core::array::from_fn(|i| a[i] + b[i])
}
pub(super) fn sub(a: V, b: V) -> V {
    core::array::from_fn(|i| a[i] - b[i])
}
pub(super) fn signed(v: V, positive: bool) -> V {
    v.map(|x| if positive { x } else { -x })
}
