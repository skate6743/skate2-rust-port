//! VMX arithmetic shared by the possession stages. Four stored lanes, xyz dot.
use super::{Frame, Vector};
pub fn dot(a: Vector, b: Vector) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
pub fn scale(a: Vector, s: f32) -> Vector {
    a.map(|x| x * s)
}
pub fn madd(a: Vector, s: f32, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i].mul_add(s, b[i]))
}
pub fn cross(a: Vector, b: Vector) -> Vector {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
fn inverse_sqrt(s: f32) -> f32 {
    let mut r = crate::physics::reciprocal_sqrt::estimate(s);
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-s).mul_add(r * r, 1.), r);
    }
    r
}
pub fn length(a: Vector) -> f32 {
    let s = dot(a, a);
    let n = s * inverse_sqrt(s);
    if s == 0. { 0. } else { n }
}
pub fn unit(a: Vector, fallback: Vector) -> Vector {
    let s = dot(a, a);
    let r = inverse_sqrt(s);
    let n = if s == 0. { 0. } else { s * r };
    if n > f32::from_bits(0x358637bd) {
        scale(a, r)
    } else {
        fallback
    }
}
pub fn reciprocal(x: f32, steps: usize) -> f32 {
    let mut r = crate::physics::native_arithmetic::reciprocal_estimate(x);
    for _ in 0..steps {
        r = r.mul_add((-r).mul_add(x, 1.), r);
    }
    r
}
///76D20 differs from atan2 at zero, including its signed-zero half-pi result.
pub fn quadrant_angle(y: f32, x: f32) -> f32 {
    let basic = crate::input::angle::atan(y.mul_add(reciprocal(x, 1), 0.));
    let sign = y.to_bits() & 0x80000000;
    let result = if 0. > x {
        f32::from_bits(0x40490fdb | sign) + basic
    } else {
        basic
    };
    if x == 0. {
        f32::from_bits(0x3fc90fdb | sign)
    } else {
        result
    }
}
pub fn transform_direction(frame: Frame, v: Vector) -> Vector {
    madd(frame[2], v[2], madd(frame[1], v[1], scale(frame[0], v[0])))
}
pub fn projected_angle(a: Vector, b: Vector, axis: Vector) -> f32 {
    if !(dot(a, a) * dot(b, b) > f32::from_bits(0x37800000)) {
        return 0.;
    }
    let a = sub(a, scale(axis, dot(axis, a)));
    let b = sub(b, scale(axis, dot(axis, b)));
    let aa = dot(a, a);
    let bb = dot(b, b);
    let gate = f32::from_bits(0x38d1b717);
    if !(aa > gate && bb > gate) {
        return 0.;
    }
    let one = |x: f32| {
        let r = crate::physics::reciprocal_sqrt::estimate(x);
        (r * 0.5).mul_add((-x).mul_add(r * r, 1.), r)
    };
    let a = scale(a, one(aa));
    let b = scale(b, one(bb));
    let angle = crate::trigonometry::acos(dot(a, b).max(-1.).min(1.));
    if dot(cross(a, b), axis) < 0. {
        f32::from_bits(0x40c90fdb) - angle
    } else {
        angle
    }
}
pub fn wrap(angle: f32) -> f32 {
    let t = angle * f32::from_bits(0x3e22f983);
    let fraction = t - t.floor();
    (fraction - if fraction > 0.5 { 1. } else { 0. }) * f32::from_bits(0x40c90fdb)
}
