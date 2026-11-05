//! Original vector operation order from82D6FD60/82D6F5F0/82D70070.
//! Estimate seeds use independent PC math; Xenon bit equality is unverified.
use super::V;
pub(in crate::skate3_native::physics) const STEP: f32 = f32::from_bits(0x3c888889);
pub(in crate::skate3_native::physics) fn dot(a: V, b: V) -> f32 {
    (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]
}
pub(in crate::skate3_native::physics) fn cross(a: V, b: V) -> V {
    [a[1].mul_add(b[2], -a[2]*b[1]), a[2].mul_add(b[0], -a[0]*b[2]), a[0].mul_add(b[1], -a[1]*b[0]), 0.0]
}
pub(in crate::skate3_native::physics) fn add(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}
pub(in crate::skate3_native::physics) fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(in crate::skate3_native::physics) fn scale(a: V, s: f32) -> V {
    a.map(|v| v * s)
}
pub(in crate::skate3_native::physics) fn madd(a: V, s: f32, b: V) -> V {
    std::array::from_fn(|i| a[i].mul_add(s, b[i]))
}
pub(in crate::skate3_native::physics) fn reciprocal(s: f32) -> f32 {
    let mut r = s.recip();
    for _ in 0..2 {
        r = (-s).mul_add(r, 1.0).mul_add(r, r);
    }
    r
}
fn rsqrt(s: f32) -> f32 {
    let mut r = s.sqrt().recip();
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-s).mul_add(r * r, 1.0), r);
    }
    r
}
pub(in crate::skate3_native::physics) fn length(a: V) -> f32 {
    let s = dot(a, a);
    let l = s * rsqrt(s);
    if s == 0.0 { 0.0 } else { l }
}
pub(in crate::skate3_native::physics) fn normalize(a: V) -> V {
    let s = dot(a, a);
    let r = rsqrt(s);
    let l = if s == 0.0 { 0.0 } else { s * r };
    if l > f32::from_bits(0x358637bd) {
        scale(a, r)
    } else {
        [0.0; 4]
    }
}
pub(in crate::skate3_native::physics) fn point(m: &[V; 4], p: V) -> V {
    std::array::from_fn(|i| {
        let x = m[0][i].mul_add(p[0], m[3][i]);
        let y = m[1][i].mul_add(p[1], x);
        m[2][i].mul_add(p[2], y)
    })
}
pub(in crate::skate3_native::physics) fn rotate(m: &[V; 4], v: V) -> V {
    std::array::from_fn(|i| {
        let x = m[0][i] * v[0];
        let y = m[1][i].mul_add(v[1], x);
        m[2][i].mul_add(v[2], y)
    })
}
pub(in crate::skate3_native::physics) fn clamp_length(v: V, maximum: f32) -> V {
    let l = length(v);
    if !(l >= f32::from_bits(0x37800000)) {
        return v;
    }
    let capped = if maximum - l >= 0.0 { l } else { maximum };
    scale(scale(v, capped), reciprocal(l))
}
pub(in crate::skate3_native::physics) fn clamp01(v: f32) -> f32 {
    let a = if -v >= 0.0 { 0.0 } else { v };
    if 1.0 - a >= 0.0 { a } else { 1.0 }
}
