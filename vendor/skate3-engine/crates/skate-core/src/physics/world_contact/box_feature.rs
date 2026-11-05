use super::{MaximumFeature, initialize_feature_segment};
use crate::physics::reciprocal_sqrt::estimate;

type V = [f32; 4];
const OTHER: [[usize; 2]; 3] = [[1, 2], [0, 2], [0, 1]]; // 83044248
const SIGNS: [[f32; 2]; 4] = [[1.0, 1.0], [1.0, -1.0], [-1.0, -1.0], [-1.0, 1.0]]; //83044260

/// Complete native box maximum-feature callback82AD7A98. Face winding uses the
/// full integer mode. `incoming_edge_plane` is VMX v60 at entry, copied only in
/// the edge branch without initialization by the native function.
pub fn box_maximum_feature(
    gp: &[u32; 48],
    mode: u32,
    direction: [u32; 4],
    output: &mut MaximumFeature,
    incoming_edge_plane: [u32; 4],
) {
    let center = load(gp, 0);
    let axes: [V; 3] = std::array::from_fn(|i| load(gp, 4 + i * 4));
    let half: [f32; 3] = std::array::from_fn(|i| f32::from_bits(gp[28 + i]));
    let direction = direction.map(f32::from_bits);
    let mut small = 0;
    let mut small_axis = 0;
    let mut large_axis = 0;
    for i in 0..3 {
        if dot(direction, axes[i]).abs() < f32::from_bits(0x3e4c_cccd) {
            small += 1;
            small_axis = i;
        } else {
            large_axis = i;
        }
    }
    if small == 2 {
        face(center, axes, half, large_axis, mode, direction, output);
    } else if small == 1 {
        let [u, v] = OTHER[small_axis];
        let mut best = [0.0; 4];
        let mut score = 0.0;
        for (i, sign) in SIGNS.into_iter().enumerate() {
            let point = madd(
                axes[v],
                half[v] * sign[1],
                madd(axes[u], half[u] * sign[0], center),
            );
            let projected = dot(point, direction);
            if i == 0 || projected > score {
                best = point;
                score = projected;
            }
        }
        let negative = sub(best, scale(axes[small_axis], half[small_axis]));
        let positive = madd(axes[small_axis], half[small_axis], best);
        let mut segment = [0; 16];
        segment[8..12].copy_from_slice(&incoming_edge_plane);
        initialize_feature_segment(
            &mut segment,
            negative.map(f32::to_bits),
            positive.map(f32::to_bits),
        );
        output[4..20].copy_from_slice(&segment);
        output[140] = 1;
    } else {
        let mut best = [0.0; 4];
        let mut score = 0.0;
        for i in 0..8 {
            let sign = if i > 3 { 1.0 } else { -1.0 };
            let pair = SIGNS[i % 4];
            let point = madd(
                axes[2],
                half[2] * pair[1],
                madd(
                    axes[1],
                    half[1] * pair[0],
                    madd(axes[0], half[0] * sign, center),
                ),
            );
            let projected = dot(point, direction);
            if i == 0 || projected > score {
                best = point;
                score = projected;
            }
        }
        store(output, 136, best);
        output[140] = 0;
    }
    output[0] = 0;
}

fn face(
    center: V,
    axes: [V; 3],
    half: [f32; 3],
    fixed: usize,
    mode: u32,
    direction: V,
    output: &mut MaximumFeature,
) {
    let sign = if dot(axes[fixed], direction) > 0.0 {
        1.0
    } else {
        -1.0
    };
    let face_center = madd(axes[fixed], half[fixed] * sign, center);
    let winding_sign = if fixed == 1 { -sign } else { sign };
    let [u, v] = OTHER[fixed];
    let positive_u = scale(axes[u], half[u]);
    let negative_u = scale(axes[u], -half[u]);
    let positive_v = scale(axes[v], half[v]);
    let negative_v = scale(axes[v], -half[v]);
    let p = plane(axes[v], direction, mode);
    let q = plane(axes[u], direction, mode);
    let len_u = half[u] * 2.0;
    let len_v = half[v] * 2.0;
    let points = [
        add(add(face_center, positive_u), positive_v),
        add(add(face_center, positive_u), negative_v),
        add(add(face_center, negative_u), negative_v),
        add(add(face_center, negative_u), positive_v),
    ];
    let negative_axis_u = axes[u].map(|v| -v);
    let negative_axis_v = axes[v].map(|v| -v);
    let (indices, directions, planes, lengths) = if u32::from(winding_sign < 0.0) == mode {
        (
            [0, 1, 2, 3],
            [negative_axis_v, negative_axis_u, axes[v], axes[u]],
            [neg(p), neg(q), p, q],
            [len_v, len_u, len_v, len_u],
        )
    } else {
        (
            [0, 3, 2, 1],
            [negative_axis_u, negative_axis_v, axes[u], axes[v]],
            [neg(q), neg(p), q, p],
            [len_u, len_v, len_u, len_v],
        )
    };
    for i in 0..4 {
        let offset = 4 + i * 16;
        store(output, offset, points[indices[i]]);
        store(output, offset + 4, directions[i]);
        store(output, offset + 8, planes[i]);
        output[offset + 12..offset + 16].fill(lengths[i].to_bits());
    }
    store(output, 132, scale(axes[fixed], -winding_sign));
    output[140] = 4;
}

fn plane(axis: V, direction: V, mode: u32) -> V {
    let mut result = if mode != 0 {
        cross(axis, direction)
    } else {
        cross(direction, axis)
    };
    let squared = dot(result, result);
    if squared > f32::from_bits(0x3400_0000) {
        let mut r = estimate(squared);
        for _ in 0..2 {
            r = (r * 0.5).mul_add((-squared).mul_add(r * r, 1.0), r);
        }
        result = scale(result, r);
    }
    result
}
fn load(words: &[u32], offset: usize) -> V {
    std::array::from_fn(|i| f32::from_bits(words[offset + i]))
}
fn store(words: &mut [u32], offset: usize, v: V) {
    words[offset..offset + 4].copy_from_slice(&v.map(f32::to_bits));
}
fn dot(a: V, b: V) -> f32 {
    super::arithmetic::dot(a[..3].try_into().unwrap(), b[..3].try_into().unwrap())
}
fn scale(a: V, b: f32) -> V {
    a.map(|v| v * b)
}
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn add(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}
fn madd(a: V, b: f32, c: V) -> V {
    std::array::from_fn(|i| a[i].mul_add(b, c[i]))
}
fn neg(a: V) -> V {
    a.map(|x| -x)
}
fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
