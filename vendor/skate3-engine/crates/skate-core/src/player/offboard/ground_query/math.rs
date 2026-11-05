use crate::{math::Vector3, physics::native_arithmetic};
pub(super) fn add(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
pub(super) fn sub(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
pub(super) fn scale(a: Vector3, k: f32) -> Vector3 {
    Vector3::new(a.x * k, a.y * k, a.z * k)
}
pub(super) fn madd(a: Vector3, k: f32, b: Vector3) -> Vector3 {
    Vector3::new(
        a.x.mul_add(k, b.x),
        a.y.mul_add(k, b.y),
        a.z.mul_add(k, b.z),
    )
}
pub(super) fn dot(a: Vector3, b: Vector3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}
pub(super) fn cross(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(
        (-a.z).mul_add(b.y, a.y * b.z),
        (-a.x).mul_add(b.z, a.z * b.x),
        (-a.y).mul_add(b.x, a.x * b.y),
    )
}
pub(super) fn horizontal(a: Vector3) -> Vector3 {
    Vector3::new(a.x, 0., a.z)
}
pub(super) fn abs(a: Vector3) -> Vector3 {
    Vector3::new(a.x.abs(), a.y.abs(), a.z.abs())
}
// Independent PC estimate, retaining the two TU3 refinement steps; not bit-exact Xenon.
pub(super) fn inverse_root(q: f32) -> f32 {
    let mut r = native_arithmetic::reciprocal_square_root_estimate(q);
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-q).mul_add(r * r, 1.), r);
    }
    r
}
pub(super) fn length(a: Vector3) -> f32 {
    let q = dot(a, a);
    let l = q * inverse_root(q);
    if q == 0. { 0. } else { l }
}
pub(super) fn reciprocal(x: f32) -> f32 {
    let mut r = native_arithmetic::reciprocal_estimate(x);
    for _ in 0..2 {
        r = r.mul_add((-x).mul_add(r, 1.), r);
    }
    r
}
pub(super) fn safe_unit(a: Vector3, fallback: Vector3) -> Vector3 {
    let q = dot(a, a);
    let r = inverse_root(q);
    let l = if q == 0. { 0. } else { q * r };
    if l > f32::from_bits(0x358637bd) {
        scale(a, r)
    } else {
        fallback
    }
}
