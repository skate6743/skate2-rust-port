use crate::physics::world_contact::{FeaturePrism, MaximumFeature, prism_math::*};

/// TU3 82ACCEF0. The quad owns the outer edge-intersection loop even when it is
/// the second feature in the caller's pair. This order reaches contact reduction.
pub(super) fn quad_triangle(
    output: &mut FeaturePrism,
    quad: &MaximumFeature,
    triangle: &MaximumFeature,
    normal: [u32; 4],
    quad_is_a: bool,
) -> u32 {
    intersect(output, quad, triangle, normal, quad_is_a)
}

/// TU3 82ACD7D8. Its calculations match 82ACCEF0 with four inner edges and four
/// vertices in both containment passes; the source transposes these into lanes.
pub(super) fn quad_quad(
    output: &mut FeaturePrism,
    a: &MaximumFeature,
    b: &MaximumFeature,
    normal: [u32; 4],
) -> u32 {
    intersect(output, a, b, normal, true)
}

fn vertex(feature: &MaximumFeature, edge: usize) -> V {
    load(feature, 4 + edge * 16)
}

fn project_to_face(face: &MaximumFeature, point: V) -> V {
    let normal = load(face, 132);
    madd(normal, dot(normal, sub(vertex(face, 0), point)), point)
}

fn append(
    output: &mut FeaturePrism,
    count: &mut usize,
    a: &MaximumFeature,
    b: &MaximumFeature,
    point: V,
    a_is_first: bool,
) {
    assert!(*count < 16, "feature prism point capacity exceeded");
    let (ao, bo) = if a_is_first { (0, 64) } else { (64, 0) };
    let offset = *count * 4;
    output[ao + offset..ao + offset + 4]
        .copy_from_slice(&project_to_face(a, point).map(f32::to_bits));
    output[bo + offset..bo + offset + 4]
        .copy_from_slice(&project_to_face(b, point).map(f32::to_bits));
    *count += 1;
}

// The contained-vertex passes use lane-wise multiply/FMA operations, with a
// rounded Y product followed by X and Z FMAs, instead of the VMX three-lane dot.
fn containment_dot(delta: V, normal: V) -> f32 {
    delta[2].mul_add(normal[2], delta[0].mul_add(normal[0], delta[1] * normal[1]))
}

fn intersect(
    output: &mut FeaturePrism,
    a: &MaximumFeature,
    b: &MaximumFeature,
    normal: [u32; 4],
    a_is_first: bool,
) -> u32 {
    let mut count = 0;
    let b_count = b[140] as usize;
    // Full native edge-pair validity masks, including endpoint equality.
    // Only the first lane of each mask controls publication, so broadcast
    // feature lengths are read from their first lane here as in the source.
    for edge_a in 0..4 {
        let a_origin = vertex(a, edge_a);
        let a_direction = load(a, 8 + edge_a * 16);
        let a_plane = load(a, 12 + edge_a * 16);
        let a_length = f32::from_bits(a[16 + edge_a * 16]);
        for edge_b in 0..b_count {
            let b_origin = vertex(b, edge_b);
            let b_direction = load(b, 8 + edge_b * 16);
            let b_length = f32::from_bits(b[16 + edge_b * 16]);
            let denominator = dot(b_direction, a_plane);
            let offset = dot(sub(a_origin, b_origin), a_plane);
            let point = madd(b_direction, reciprocal(denominator) * offset, b_origin);
            let along_a = dot(sub(point, a_origin), a_direction);
            let valid = denominator.abs() > f32::from_bits(0x3400_0000)
                && !(offset.abs() > (b_length * denominator).abs())
                && !(0.0 > offset * denominator)
                && !(along_a > a_length)
                && !(0.0 > along_a);
            if valid {
                append(output, &mut count, a, b, point, a_is_first);
            }
        }
    }
    let normal = normal.map(f32::from_bits);
    for (source, target, source_count, target_count) in [(a, b, 4, b_count), (b, a, b_count, 4)] {
        for corner in 0..source_count {
            let origin = vertex(source, corner);
            let distance = containment_dot(sub(vertex(target, 0), origin), normal);
            // The source transposes XYZ back into points with a literal W=1.
            let mut point = madd(normal, distance, origin);
            point[3] = 1.0;
            let inside = (0..target_count).all(|edge| {
                let distance = containment_dot(
                    sub(point, vertex(target, edge)),
                    load(target, 12 + edge * 16),
                );
                0.0 > distance
            });
            if inside {
                append(output, &mut count, a, b, point, a_is_first);
            }
        }
    }
    output[132] = count as u32;
    u32::from(count != 0)
}
