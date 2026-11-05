use crate::physics::{reciprocal_sqrt::estimate, world_contact::prism_math::*};

/// TU3 82ACF448. Unlike the triangle/box entry, edge crosses use the raw
/// reciprocal-square-root estimate without refinement. The last cross is
/// repeated to fill its final four-candidate batch; equal scores retain the
/// earlier axis. Shape fatness is excluded from these intervals.
pub(in super::super) fn box_box(a: &[u32; 48], b: &[u32; 48]) -> ([u32; 4], [u32; 4]) {
    let mut axes = [[0.0; 4]; 16];
    for i in 0..3 {
        axes[i] = load(a, 12 - i * 4);
        axes[3 + i] = load(b, 12 - i * 4);
    }
    for edge_a in 0..3 {
        for edge_b in 0..3 {
            let axis = cross(load(a, 16 + edge_a * 4), load(b, 16 + edge_b * 4));
            axes[6 + edge_a * 3 + edge_b] = scale(axis, estimate(dot(axis, axis)));
        }
    }
    axes[15] = axes[14];
    let scaled_a: [V; 3] =
        std::array::from_fn(|i| scale(load(a, 4 + i * 4), f32::from_bits(a[28 + i])));
    let scaled_b: [V; 3] =
        std::array::from_fn(|i| scale(load(b, 4 + i * 4), f32::from_bits(b[28 + i])));
    let mut best = 0.0;
    let mut selected = axes[0];
    let mut selected_sign = 1.0;
    for (index, axis) in axes.into_iter().enumerate() {
        let a_center = dot(load(a, 0), axis);
        let b_center = dot(load(b, 0), axis);
        let a_extents = scaled_a.map(|extent| dot(extent, axis).abs());
        let b_extents = scaled_b.map(|extent| dot(extent, axis).abs());
        let a_radius = (a_extents[0] + a_extents[1]) + a_extents[2];
        let b_radius = (b_extents[0] + b_extents[1]) + b_extents[2];
        let forward = (a_center - a_radius) - (b_center + b_radius);
        let reverse = (b_center - b_radius) - (a_center + a_radius);
        let (separation, sign) = if forward > reverse {
            (forward, -1.0)
        } else {
            (reverse, 1.0)
        };
        if index == 0 || separation > best {
            best = separation;
            selected = axis;
            selected_sign = sign;
        }
    }
    (
        [best.to_bits(); 4],
        scale(selected, selected_sign).map(f32::to_bits),
    )
}
