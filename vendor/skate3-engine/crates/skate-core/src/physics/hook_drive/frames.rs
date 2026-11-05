//! Full 82C00940/82C00AF0 matrix33 conversion. Unlike Part::SetTransform,
//! these functions do not normalize the selected quaternion afterward.
type V = [f32; 4];
fn add(a: V, b: V) -> V {
    core::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: V, b: V) -> V {
    core::array::from_fn(|i| a[i] - b[i])
}
fn mul(a: V, b: V) -> V {
    core::array::from_fn(|i| a[i] * b[i])
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
/// 82C00940 preserves both translations and the parent quaternion.
pub fn set_child_angular_frame(frames: &mut [u32; 16], basis: [u32; 12]) {
    frames[..4].copy_from_slice(&convert(basis));
}
/// 82C00AF0 preserves both translations and the child quaternion.
pub fn set_parent_angular_frame(frames: &mut [u32; 16], basis: [u32; 12]) {
    frames[8..12].copy_from_slice(&convert(basis));
}
fn convert(basis: [u32; 12]) -> [u32; 4] {
    quaternion(core::array::from_fn(|i| {
        core::array::from_fn(|j| f32::from_bits(basis[i * 4 + j]))
    }))
    .map(f32::to_bits)
}
fn quaternion(m: [V; 3]) -> V {
    let [ri, up, at] = m;
    // Sign masks are initialized by TU3 82F83360/90/C0, not image defaults.
    let rx = [ri[0], ri[0], -ri[0], -ri[0]];
    let uy = [up[1], -up[1], up[1], -up[1]];
    let az = [at[2], -at[2], -at[2], at[2]];
    let first = add(rx, uy);
    let d = add(first, add(az, [1.0; 4]));
    let inv = d.map(rsqrt);
    let roots = mul(d, inv);
    let halves = mul([0.5; 4], inv);
    let a = [up[2], at[0], ri[1], 0.5];
    let b = [at[1], ri[2], up[0], 0.0];
    let minus = sub(a, b);
    let plus = add(a, b);
    let y = mul(
        [plus[2], minus[3], plus[0], minus[1]],
        [halves[2], roots[2], halves[2], halves[2]],
    );
    let z = mul(
        [plus[1], plus[0], minus[3], minus[2]],
        [halves[3], halves[3], roots[3], halves[3]],
    );
    let x = mul(
        [minus[3], plus[2], plus[1], minus[0]],
        [roots[1], halves[1], halves[1], halves[1]],
    );
    let w = mul(minus, [halves[0], halves[0], halves[0], roots[0]]);
    let yz = if up[1] > at[2] { y } else { z };
    let xyz = if ri[0] > up[1] && ri[0] > at[2] {
        x
    } else {
        yz
    };
    if first[0] + at[2] > 0.0 { w } else { xyz }
}
