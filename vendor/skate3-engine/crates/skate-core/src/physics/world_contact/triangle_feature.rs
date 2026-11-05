use super::MaximumFeature;
use crate::physics::reciprocal_sqrt::estimate;

type V = [f32; 4];

/// Full triangle maximum-feature callback82ADDD68, including near-tangent
/// edge/vertex branches. Header word0 is preserved outside the face branch.
pub fn triangle_maximum_feature(
    gp: &[u32; 48],
    mode: u32,
    direction: [u32; 4],
    out: &mut MaximumFeature,
) {
    let normal = load(gp, 4);
    let query = direction.map(f32::from_bits);
    let projected = dot(query, normal);
    let points = [load(gp, 0), load(gp, 8), load(gp, 12)];
    let edges = [load(gp, 16), load(gp, 20), load(gp, 24)];
    if projected.abs() > f32::from_bits(0x3f73_3333) {
        if u32::from(projected < 0.0) == mode {
            for (i, point) in [0, 2, 1].into_iter().enumerate() {
                segment(out, i, points[point], edges[i], gp[28 + i]);
            }
            out[0] = 8;
        } else {
            for (i, point) in [0, 1, 2].into_iter().enumerate() {
                segment(out, i, points[point], edges[2 - i].map(|x| -x), gp[30 - i]);
            }
            out[0] = 0;
        }
        store(out, 132, normal);
        out[140] = 3;
    } else {
        let p = points.map(|x| dot(query, x));
        let e = edges.map(|x| dot(query, x).abs());
        let threshold = f32::from_bits(0x3d4c_cccd);
        let (point, edge) = if p[0] > p[1] && p[0] > p[2] {
            (
                0,
                if threshold > e[0] && e[2] > e[0] {
                    Some(0)
                } else if threshold > e[2] {
                    Some(2)
                } else {
                    None
                },
            )
        } else if p[1] > p[2] {
            (
                1,
                if threshold > e[1] && e[2] > e[1] {
                    Some(1)
                } else if threshold > e[2] {
                    Some(2)
                } else {
                    None
                },
            )
        } else {
            (
                2,
                if threshold > e[0] && e[1] > e[0] {
                    Some(0)
                } else if threshold > e[1] {
                    Some(1)
                } else {
                    None
                },
            )
        };
        if let Some(edge) = edge {
            segment(out, 0, points[[0, 2, 1][edge]], edges[edge], gp[28 + edge]);
            out[140] = 1;
        } else {
            store(out, 136, points[point]);
            out[140] = 0;
        }
    }
    build_feature_edge_planes(out, mode, direction);
}

/// Complete82AC6F88. Only edge plane vectors are changed. Mode0 negates the
/// incoming direction before crossing, preserving the native arithmetic order.
pub fn build_feature_edge_planes(feature: &mut MaximumFeature, mode: u32, direction: [u32; 4]) {
    let count = feature[140] as i32;
    if count <= 0 {
        return;
    }
    assert!(
        count <= 8,
        "native feature storage holds at most eight segments"
    );
    let normal = direction
        .map(f32::from_bits)
        .map(|x| if mode == 0 { -x } else { x });
    for i in 0..count as usize {
        let offset = 4 + i * 16;
        let edge = load(feature, offset + 4);
        let plane = cross(edge, normal);
        let squared = dot(plane, plane);
        let mut r = estimate(squared);
        r = (r * 0.5).mul_add((-squared).mul_add(r * r, 1.0), r);
        let scale = if squared > f32::from_bits(0x3400_0000) {
            r
        } else {
            0.0
        };
        store(feature, offset + 8, plane.map(|x| x * scale));
    }
}

fn segment(out: &mut MaximumFeature, index: usize, point: V, edge: V, length: u32) {
    let offset = 4 + index * 16;
    store(out, offset, point);
    store(out, offset + 4, edge);
    out[offset + 12..offset + 16].fill(length);
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
fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
