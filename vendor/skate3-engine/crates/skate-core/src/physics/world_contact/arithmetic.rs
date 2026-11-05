pub(super) type V = [f32; 3];

pub(super) fn vector(record: &[u32], offset: usize) -> V {
    std::array::from_fn(|i| f32::from_bits(record[offset + i]))
}

pub(super) fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}

pub(super) fn dot(a: V, b: V) -> f32 {
    crate::physics::native_arithmetic::dot3([a[0], a[1], a[2], 0.0], [b[0], b[1], b[2], 0.0])
}

pub(super) fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
    ]
}

pub(super) fn scale(a: V, b: f32) -> V {
    a.map(|v| v * b)
}
