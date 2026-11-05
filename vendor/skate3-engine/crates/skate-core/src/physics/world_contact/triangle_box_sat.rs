use crate::physics::{reciprocal_sqrt::estimate, world_contact::prism_math::*};

/// TU3 82ACF950. Triangle normal first, then box Z/Y/X and all nine edge
/// crosses. Cross axes receive exactly one reciprocal-square-root refinement;
/// there is no small-cross rejection or normalization fallback in this entry.
pub(in super::super) fn triangle_box(
    triangle: &[u32; 48],
    box_shape: &[u32; 48],
) -> ([u32; 4], [u32; 4]) {
    let mut axes = [[0.0; 4]; 13];
    axes[0] = load(triangle, 4);
    for i in 0..3 {
        axes[1 + i] = load(box_shape, 12 - i * 4);
    }
    for edge in 0..3 {
        for box_edge in 0..3 {
            let axis = cross(
                load(triangle, 16 + edge * 4),
                load(box_shape, 16 + box_edge * 4),
            );
            let squared = dot(axis, axis);
            let seed = estimate(squared);
            let inverse = (seed * 0.5).mul_add((-squared).mul_add(seed * seed, 1.0), seed);
            axes[4 + edge * 3 + box_edge] = scale(axis, inverse);
        }
    }
    let scaled_box: [V; 3] = std::array::from_fn(|i| {
        scale(
            load(box_shape, 4 + i * 4),
            f32::from_bits(box_shape[28 + i]),
        )
    });
    let mut best = 0.0;
    let mut selected = axes[0];
    let mut selected_sign = 1.0;
    for (index, axis) in axes.into_iter().enumerate() {
        let p0 = dot(load(triangle, 0), axis);
        let p1 = dot(load(triangle, 8), axis);
        let p2 = dot(load(triangle, 12), axis);
        // The source compares vertices 1 and 2, then combines vertex 0.
        let minimum = minimum(p0, minimum(p1, p2));
        let maximum = maximum(p0, maximum(p1, p2));
        let center = dot(load(box_shape, 0), axis);
        let extents = scaled_box.map(|extent| dot(extent, axis).abs());
        let radius = (extents[0] + extents[1]) + extents[2];
        let forward = minimum - (center + radius);
        let reverse = (center - radius) - maximum;
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

fn minimum(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}
fn maximum(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}
