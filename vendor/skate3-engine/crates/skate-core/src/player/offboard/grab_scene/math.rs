use super::{Frame, Vector};
pub(super) fn dot(a: Vector, b: Vector) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(super) fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(super) fn madd(a: Vector, k: f32, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i].mul_add(k, b[i]))
}
pub(super) fn length(v: Vector) -> f32 {
    let d = dot(v, v);
    let mut r = d.sqrt().recip();
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-d).mul_add(r * r, 1.0), r);
    }
    if d == 0.0 { 0.0 } else { d * r }
}
pub(super) fn reciprocal(v: f32) -> f32 {
    let mut r = v.recip();
    for _ in 0..2 {
        r = r.mul_add((-r).mul_add(v, 1.0), r);
    }
    r
}
pub(super) fn point(f: Frame, v: Vector) -> Vector {
    madd(f[2], v[2], madd(f[1], v[1], madd(f[0], v[0], f[3])))
}
pub(super) fn direction(f: Frame, v: Vector) -> Vector {
    madd(f[2], v[2], madd(f[1], v[1], f[0].map(|x| x * v[0])))
}
///82ADD910 ->82ADDB48 positive-result branch, fatness zero.
pub(super) fn sphere_segment(center: Vector, radius: f32, start: Vector, end: Vector) -> bool {
    let c = sub(center, start);
    let rr = radius * radius;
    if dot(c, c) < rr {
        return true;
    }
    let d = sub(end, start);
    let projection = dot(c, d);
    if !(projection > 0.0) {
        return false;
    }
    let dd = dot(d, d);
    let cross = [
        (-c[2]).mul_add(d[1], c[1] * d[2]),
        (-c[0]).mul_add(d[2], c[2] * d[0]),
        (-c[1]).mul_add(d[0], c[0] * d[1]),
        0.0,
    ];
    let discriminant = dd * rr - dot(cross, cross);
    let beyond = projection - dd;
    !(discriminant < 0.0 || (beyond > 0.0 && beyond * beyond > discriminant))
}
