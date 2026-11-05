//! Board portion of original TU3 physical pose worker82DB993C..A55C.
use crate::physics::{
    board_motion_output::inverse_length_squared,
    native_arithmetic::dot3,
    skeleton_animation_record::{AnimationPartTransform as Transform, compose_affine},
};
use crate::{math::Vector3, riding::collision_response::signed_angle, trigonometry::sin_cos};

#[derive(Clone, Copy)]
pub struct Settings {
    pub truck_tilt_scalar: f32,
    pub truck_tilt_max_angle: f32,
    pub truck_tilt_wobble_scalar: f32,
    pub truck_displacement_max: f32,
}
#[derive(Clone, Copy)]
pub struct BoneIndices {
    pub front_truck: usize,
    pub back_truck: usize,
    pub front_left_wheel: usize,
    pub front_right_wheel: usize,
    pub back_left_wheel: usize,
    pub back_right_wheel: usize,
}
impl BoneIndices {
    pub fn all(self) -> [usize; 6] {
        [
            self.front_truck,
            self.back_truck,
            self.front_left_wheel,
            self.front_right_wheel,
            self.back_left_wheel,
            self.back_right_wheel,
        ]
    }
}
pub struct Input<'a> {
    /// Raw dynamic-body matrices82585B58, native parts0..5; not82585CB0.
    pub bodies: &'a [Transform; 6],
    /// SkeletonPhysicalRecord.part0, captured separately by82DB670C.
    pub skeleton_board: &'a Transform,
    /// SkateboardBody7840/7904, not7712/7776 steering-drive base frames.
    pub truck_frames: &'a [Transform; 2],
    pub deck_wobble_tilt: f32,
    pub deck_wobble_squish: f32,
    /// CalculateAverageWheelCompressions82C08968, front8372/back8376.
    pub average_compression: [f32; 2],
}
pub fn publish(
    input: Input<'_>,
    settings: &Settings,
    bones: BoneIndices,
    locals: &mut [Transform],
) {
    let y = [0.0, 1.0, 0.0, 0.0];
    let z = [0.0, 0.0, 1.0, 0.0];
    let wheels: [Transform; 4] = core::array::from_fn(|wheel| {
        let up = inverse_direction(&input.bodies[4 + wheel / 2], input.bodies[wheel][1]);
        rotation_x(angle(up, y, z))
    });
    //82DB9C60/9E00 choose opposite axle directions at the two ends.
    let mut front = inverse_direction(
        input.skeleton_board,
        normalize_safe(subtract(input.bodies[0][3], input.bodies[1][3])),
    );
    let mut back = inverse_direction(
        input.skeleton_board,
        normalize_safe(subtract(input.bodies[3][3], input.bodies[2][3])),
    );
    if 0.5 > dot3(front, front) {
        front = input.truck_frames[0][2];
    }
    if 0.5 > dot3(back, back) {
        back = input.truck_frames[1][2].map(|v| -v);
    }
    let negative_x = [-1.0, -0.0, -0.0, -0.0];
    let back_angle = wrap(angle(back, negative_x, input.truck_frames[1][0]));
    let front_angle = wrap(angle(front, negative_x, input.truck_frames[0][0]));
    let wobble = input.deck_wobble_tilt * settings.truck_tilt_wobble_scalar;
    let back_tilt = clamp(
        (-settings.truck_tilt_scalar).mul_add(back_angle, -wobble),
        -settings.truck_tilt_max_angle,
        settings.truck_tilt_max_angle,
    );
    let front_tilt = clamp(
        (-settings.truck_tilt_scalar).mul_add(front_angle, wobble),
        -settings.truck_tilt_max_angle,
        settings.truck_tilt_max_angle,
    );
    for (side, bone, tilt) in [
        (0, bones.front_truck, front_tilt),
        (1, bones.back_truck, back_tilt),
    ] {
        let compression = if input.average_compression[side] < f32::from_bits(0x3A83_126F) {
            0.0
        } else {
            input.average_compression[side]
        };
        let displacement = clamp(
            compression - input.deck_wobble_squish,
            0.0,
            settings.truck_displacement_max,
        );
        let rotated = compose_affine(&locals[bone], &rotation_x(tilt));
        //82DBA130..A318 left-multiplies the rotated local by translation.
        let translation = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, displacement, 0.0, 0.0],
        ];
        locals[bone] = compose_affine(&translation, &rotated);
    }
    //Native physical-body order and animation-name order differ.
    for (wheel, bone) in [
        bones.front_left_wheel,
        bones.front_right_wheel,
        bones.back_left_wheel,
        bones.back_right_wheel,
    ]
    .into_iter()
    .enumerate()
    {
        locals[bone] = compose_affine(&locals[bone], &wheels[wheel]);
    }
}
fn angle(a: [f32; 4], b: [f32; 4], axis: [f32; 4]) -> f32 {
    let vector = |v: [f32; 4]| Vector3::new(v[0], v[1], v[2]);
    signed_angle(vector(a), vector(b), vector(axis))
}
fn inverse_direction(frame: &Transform, value: [f32; 4]) -> [f32; 4] {
    let mut output = [0.0; 4];
    for lane in 0..3 {
        let x = value[0] * frame[lane][0];
        let y = value[1].mul_add(frame[lane][1], x);
        output[lane] = value[2].mul_add(frame[lane][2], y);
    }
    output
}
fn normalize_safe(value: [f32; 4]) -> [f32; 4] {
    let squared = dot3(value, value);
    let inverse = inverse_length_squared(squared, 2);
    let length = if squared == 0.0 {
        0.0
    } else {
        squared * inverse
    };
    if length > f32::from_bits(0x3586_37BD) {
        value.map(|v| v * inverse)
    } else {
        [0.0; 4]
    }
}
fn subtract(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    core::array::from_fn(|i| a[i] - b[i])
}
fn wrap(angle: f32) -> f32 {
    let turns = angle * f32::from_bits(0x3E22_F983);
    let fraction = turns - turns.floor();
    (fraction - if fraction > 0.5 { 1.0 } else { 0.0 }) * f32::from_bits(0x40C9_0FDB)
}
fn clamp(value: f32, minimum: f32, maximum: f32) -> f32 {
    let low = if minimum - value >= 0.0 {
        minimum
    } else {
        value
    };
    if maximum - low >= 0.0 { low } else { maximum }
}
fn rotation_x(angle: f32) -> Transform {
    let (sine, cosine) = sin_cos(angle);
    //vperm822FB890 replicates X in W; vrlimi2 replaces only Z.
    [
        [1.0, 0.0, 0.0, 1.0],
        [0.0, cosine, sine, 0.0],
        [0.0, -sine, cosine, 0.0],
        [0.0; 4],
    ]
}
