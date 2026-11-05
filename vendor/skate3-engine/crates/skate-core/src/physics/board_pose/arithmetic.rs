//! Arithmetic order from the retained TU3 pose functions; guest lane order.
use super::PoseMatrix;
pub(super) type V = [f32; 4];
pub(super) type M = [V; 4];
pub(super) fn matrix(w: PoseMatrix) -> M {
    core::array::from_fn(|i| core::array::from_fn(|j| f32::from_bits(w[i * 4 + j])))
}
pub(super) fn words(m: M) -> PoseMatrix {
    core::array::from_fn(|i| m[i / 4][i % 4].to_bits())
}
pub(super) fn add(a: V, b: V) -> V {
    core::array::from_fn(|i| a[i] + b[i])
}
pub(super) fn sub(a: V, b: V) -> V {
    core::array::from_fn(|i| a[i] - b[i])
}
pub(super) fn mul(a: V, b: V) -> V {
    core::array::from_fn(|i| a[i] * b[i])
}
pub(super) fn madd(a: V, b: V, c: V) -> V {
    core::array::from_fn(|i| a[i].mul_add(b[i], c[i]))
}
pub(super) fn nmsub(a: V, b: V, c: V) -> V {
    core::array::from_fn(|i| (-a[i]).mul_add(b[i], c[i]))
}
pub(super) fn perm(a: V, p: [usize; 4]) -> V {
    p.map(|i| a[i])
}
pub(super) fn dot(a: V, b: V, four: bool) -> f32 {
    if four {
        crate::physics::native_arithmetic::dot4(a, b)
    } else {
        crate::physics::native_arithmetic::dot3(a, b)
    }
}
pub(super) fn rsqrt(v: f32) -> f32 {
    let mut e = super::super::reciprocal_sqrt::estimate(v);
    for _ in 0..2 {
        let s = e * e;
        let h = e * 0.5;
        e = h.mul_add((-v).mul_add(s, 1.0), e);
    }
    e
}
pub(super) fn normalize(v: V, four: bool) -> V {
    mul(v, [rsqrt(dot(v, v, four)); 4])
}
pub(super) fn cross(a: V, b: V) -> V {
    perm(
        nmsub(perm(a, [1, 2, 0, 3]), b, mul(a, perm(b, [1, 2, 0, 3]))),
        [1, 2, 0, 3],
    )
}
pub(super) fn compose(a: M, b: M) -> M {
    core::array::from_fn(|i| {
        let first = if i == 3 {
            madd([b[i][0]; 4], a[0], a[3])
        } else {
            mul([b[i][0]; 4], a[0])
        };
        madd([b[i][2]; 4], a[2], madd([b[i][1]; 4], a[1], first))
    })
}
pub(super) fn inverse_rigid(m: M) -> M {
    let mut out: [[f32; 4]; 4] = core::array::from_fn(|i| {
        if i < 3 {
            [m[0][i], m[1][i], m[2][i], 0.0]
        } else {
            [0.0; 4]
        }
    });
    let neg = sub([0.0; 4], m[3]);
    out[3] = madd(
        [neg[0]; 4],
        out[0],
        madd([neg[1]; 4], out[1], mul([neg[2]; 4], out[2])),
    );
    out
}
