//! Original82BD3E78/8296EBB0 and inlined VMX arithmetic.
use super::Vector;
pub(super) fn select(test: f32, yes: f32, no: f32) -> f32 {
    if test >= 0.0 { yes } else { no }
}
pub(super) fn clamp(v: f32, low: f32, high: f32) -> f32 {
    let v = select(low - v, low, v);
    select(high - v, v, high)
}
pub(super) fn dot(a: Vector, b: Vector) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(super) fn scale(v: Vector, s: f32) -> Vector {
    v.map(|x| x * s)
}
pub(super) fn add(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] + b[i])
}
pub(super) fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(super) fn madd(a: Vector, s: f32, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i].mul_add(s, b[i]))
}
pub(super) fn cross(a: Vector, b: Vector) -> Vector {
    //vpermwi0x63 preserves lane w; its fused subtraction is retained too.
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
fn inverse(value: f32, steps: usize) -> f32 {
    //Ordinary host seed, not a claim of bit-exact Xenon estimate emulation.
    let mut r = crate::physics::reciprocal_sqrt::estimate(value);
    for _ in 0..steps {
        let e = (-value).mul_add(r * r, 1.0);
        r = (r * 0.5).mul_add(e, r);
    }
    r
}
pub(super) fn square_root(value: f32) -> f32 {
    let result = value * inverse(value, 2);
    if value == 0.0 { 0.0 } else { result }
}
pub(super) fn length(v: Vector) -> f32 {
    square_root(dot(v, v))
}
pub(super) fn normalize_or(v: Vector, fallback: Vector) -> Vector {
    let sq = dot(v, v);
    let r = inverse(sq, 2);
    let len = if sq == 0.0 { 0.0 } else { sq * r };
    if len > f32::from_bits(0x3586_37bd) {
        scale(v, r)
    } else {
        fallback
    }
}
pub(super) fn unsigned_angle(a: Vector, b: Vector) -> f32 {
    let aa = dot(a, a);
    let bb = dot(b, b);
    let gate = f32::from_bits(0x38d1_b717);
    if !(aa > gate && bb > gate) {
        return 0.0;
    }
    crate::trigonometry::acos(
        dot(scale(a, inverse(aa, 1)), scale(b, inverse(bb, 1)))
            .max(-1.0)
            .min(1.0),
    )
}
pub(super) fn wrap_angle(a: f32) -> f32 {
    let t = a * f32::from_bits(0x3e22_f983);
    let f = t - t.floor();
    (f - if f > 0.5 { 1.0 } else { 0.0 }) * f32::from_bits(0x40c9_0fdb)
}
pub(super) fn clamp_axis(value: Vector, axis: Vector, low: f32, high: f32) -> Vector {
    let projection = dot(value, axis);
    madd(
        axis,
        clamp(projection, low, high),
        sub(value, scale(axis, projection)),
    )
}
///82BD3E78 returns original target when close or cross-axis degenerate.
pub(super) fn clamp_angle(target: Vector, reference: Vector, limit: f32) -> Vector {
    let a = normalize_or(target, [0.0; 4]);
    let b = normalize_or(reference, [0.0; 4]);
    let axis = cross(a, b);
    let angle = crate::trigonometry::acos(dot(a, b).max(-1.0).min(1.0));
    let turns = angle.mul_add(f32::from_bits(0x3e22_f983), 0.5).floor();
    let wrapped = (-turns).mul_add(f32::from_bits(0x40c9_0fdb), angle);
    if wrapped.abs() < limit || f32::from_bits(0x3780_0000) > dot(axis, axis) {
        return target;
    }
    let unit = scale(axis, inverse(dot(axis, axis), 2));
    let (sin, cos) = crate::trigonometry::sin_cos((-limit) * 0.5);
    let mut q = scale(unit, sin);
    q[3] = cos;
    let first = cross(q, reference);
    let middle = madd(reference, cos, first);
    let rotated = madd(cross(q, middle), 2.0, reference);
    scale(rotated, length(target))
}
