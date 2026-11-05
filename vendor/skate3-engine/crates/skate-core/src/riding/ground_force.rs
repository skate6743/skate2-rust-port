//! Complete TU3 82D931E8, called twice by Ground Update for tags 4 and 5.
//! The research index has not recovered its original name. Keep its four
//! caller-supplied scalar arguments explicit instead of inventing units.
use super::vector::{clamp, dot3, normalize};
#[derive(Clone)]
pub struct GroundForceSettings {
    pub range_1220: f32,
    pub speed_scale_1224: f32,
    pub scale_1228: f32,
    pub normal_threshold: [f32; 4],
}
pub struct GroundForceInput {
    pub argument_1: f32,
    pub application_z: f32,
    pub argument_3: f32,
    pub argument_4: f32,
    pub balance: f32,       // Processed +2720
    pub surface_speed: f32, // +2656
    pub axis_384: [f32; 4],
    pub velocity_400: [f32; 4],
    pub axis_544: [f32; 4],
}
pub fn calculate(s: &GroundForceSettings, i: &GroundForceInput) -> [f32; 8] {
    let fraction = clamp(i.argument_1 / s.range_1220, 0.0, 1.0);
    // Native double literals at822F8700/+8 are0 and1, narrowed at zero arg1.
    let zero_scalar = if i.argument_1 != 0.0 || i.balance != 0.0 {
        1.0
    } else {
        0.0
    };
    let speed = (i.surface_speed * 0.125) * s.speed_scale_1224;
    let remainder = ((1.0 - fraction) * s.scale_1228) * zero_scalar;
    let magnitude = -(speed.mul_add(fraction, remainder) * i.argument_4);
    let force =
        core::array::from_fn(|n| i.axis_544[n].mul_add(magnitude, (-i.axis_384[n]) * i.argument_3));
    let direction = normalize(i.velocity_400, s.normal_threshold);
    let projected = dot3(direction, force);
    let mut result = [0.0; 8];
    for n in 0..4 {
        result[n] = force[n] - direction[n] * projected;
    }
    result[6] = i.application_z;
    result
}
