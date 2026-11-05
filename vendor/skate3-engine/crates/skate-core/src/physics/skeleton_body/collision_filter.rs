//! Original82BD5A00: select the angular interval of active contact planes.
use super::{SkeletonCollisionFeedback, collision_feedback::V, collision_vector::*};
use crate::{
    math::Vector3,
    riding::collision_response::signed_angle,
    trigonometry::{cos, sin},
};

impl SkeletonCollisionFeedback {
    pub fn filter_error(&self, error: V, axis: V) -> V {
        let projected = sub(error, scale(axis, dot(error, axis)));
        let direction = normalize(projected);
        if dot(direction, direction) < 0.1 {
            return projected;
        }
        let half_pi = f32::from_bits(0x3fc9_0fdb);
        let mut minimum = half_pi;
        let mut maximum = -half_pi;
        let mut found = false;
        for plane in &self.planes {
            let normal = normalize(sub(plane.normal, scale(axis, dot(plane.normal, axis))));
            if dot(normal, normal) < 0.1 {
                continue;
            }
            let angle = signed_angle(vector(direction), vector(normal), vector(axis));
            let turns = angle * f32::from_bits(0x3e22_f983);
            let fractional = turns - turns.floor();
            let centered = fractional - if fractional > 0.5 { 1.0 } else { 0.0 };
            let wrapped = centered * f32::from_bits(0x40c9_0fdb);
            if wrapped > -half_pi && wrapped < half_pi {
                if wrapped < minimum {
                    minimum = wrapped;
                }
                if wrapped > maximum {
                    maximum = wrapped;
                }
                found = true;
            }
        }
        let angle = (maximum + minimum) * 0.5;
        let magnitude = if found { cos(angle) } else { 0.0 };
        //82BD35B8 rotates identity around the supplied axis before applying
        //the projected error. Preserve its product/FMA and standalone trig.
        let c = cos(angle);
        let s = sin(angle);
        let [x, y, z, _] = axis;
        let sx = s * x;
        let sy = s * y;
        let sz = s * z;
        let t = 1.0 - c;
        let tx = t * x;
        let ty = t * y;
        let tz = t * z;
        let rotation = [
            [x.mul_add(tx, c), tx.mul_add(y, sz), tx * z - sy],
            [ty * x - sz, ty.mul_add(y, c), ty.mul_add(z, sx)],
            [tz.mul_add(x, sy), tz * y - sx, z.mul_add(tz, c)],
        ];
        let mut result = [0.0; 4];
        for i in 0..3 {
            result[i] = rotation[2][i].mul_add(
                projected[2],
                rotation[1][i].mul_add(projected[1], rotation[0][i] * projected[0]),
            ) * magnitude;
        }
        result
    }
}
fn vector(v: V) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
