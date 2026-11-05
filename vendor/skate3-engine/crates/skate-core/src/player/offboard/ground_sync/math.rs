use super::Vector;
pub(super) fn dot(a: Vector, b: Vector) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(super) fn scale(v: Vector, s: f32) -> Vector {
    v.map(|x| x * s)
}
pub(super) fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(super) fn madd(a: Vector, s: f32, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i].mul_add(s, b[i]))
}
pub(super) fn cross(a: Vector, b: Vector) -> Vector {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
fn inverse(sq: f32) -> f32 {
    // PC reciprocal-square-root seed; two native Newton refinements retained.
    let mut r = crate::physics::reciprocal_sqrt::estimate(sq);
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-sq).mul_add(r * r, 1.0), r);
    }
    r
}
pub(super) fn normalize_unchecked(v: Vector) -> Vector {
    scale(v, inverse(dot(v, v)))
}
pub(super) fn normalize_or(v: Vector, fallback: Vector) -> Vector {
    let sq = dot(v, v);
    let r = inverse(sq);
    let length = if sq == 0.0 { 0.0 } else { sq * r };
    if length > f32::from_bits(0x3586_37bd) {
        scale(v, r)
    } else {
        fallback
    }
}
