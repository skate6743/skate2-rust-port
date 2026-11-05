use crate::physics::reciprocal_sqrt::estimate;

pub(super) type V = [f32; 4];

pub(super) fn load(words: &[u32], offset: usize) -> V {
    std::array::from_fn(|i| f32::from_bits(words[offset + i]))
}

pub(super) fn dot(a: V, b: V) -> f32 {
    super::arithmetic::dot(a[..3].try_into().unwrap(), b[..3].try_into().unwrap())
}

pub(super) fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}

pub(super) fn scale(a: V, s: f32) -> V {
    a.map(|v| v * s)
}

pub(super) fn madd(a: V, s: f32, b: V) -> V {
    std::array::from_fn(|i| a[i].mul_add(s, b[i]))
}

pub(super) fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}

pub(super) fn normalized(a: V) -> V {
    let squared = dot(a, a);
    scale(a, inverse_length(squared))
}

pub(super) fn inverse_length(squared: f32) -> f32 {
    let mut r = estimate(squared);
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-squared).mul_add(r * r, 1.0), r);
    }
    r
}

pub(super) fn reciprocal(value: f32) -> f32 {
    let mut r = reciprocal_estimate(value);
    for _ in 0..2 {
        r = r.mul_add((-r).mul_add(value, 1.0), r);
    }
    r
}

pub(super) fn reciprocal_estimate(value: f32) -> f32 {
    crate::physics::native_arithmetic::reciprocal_estimate(value)
}
