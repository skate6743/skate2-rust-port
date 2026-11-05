use crate::physics::world_contact::{
    FeaturePrism, MaximumFeature, intersect_feature_corner_edge, prism_math::*,
};

const OUTSIDE_A: u32 = 0x4000_0000;
const OUTSIDE_B: u32 = 0x2000_0000;

fn vertex(feature: &MaximumFeature, index: usize) -> V {
    load(feature, 4 + index * 16)
}

fn plane_distance(feature: &MaximumFeature, edge: usize, point: V) -> f32 {
    dot(
        load(feature, 12 + edge * 16),
        sub(point, vertex(feature, edge)),
    )
}

fn project_to_face(face: &MaximumFeature, point: V) -> V {
    let normal = load(face, 132);
    madd(normal, dot(normal, sub(vertex(face, 0), point)), point)
}

fn append(output: &mut FeaturePrism, a: V, b: V) {
    let offset = output[132] as usize * 4;
    assert!(offset < 64, "feature prism point capacity exceeded");
    output[offset..offset + 4].copy_from_slice(&a.map(f32::to_bits));
    output[64 + offset..68 + offset].copy_from_slice(&b.map(f32::to_bits));
    output[132] += 1;
}

/// General convex-face branch at 82ACE3D8, after specialized paths decline.
/// Retains native candidate order: B vertices, edge crossings, then A vertices.
/// Plane-touching vertices/crossings are included without deduplication here.
pub(super) fn intersect(
    output: &mut FeaturePrism,
    a: &mut MaximumFeature,
    b: &mut MaximumFeature,
) -> u32 {
    let (a_count, b_count) = (a[140] as usize, b[140] as usize);
    let mut masks = [0u32; 8];
    let mut nearest_corner = 0;
    let mut nearest_edge = 0;
    let mut edge_is_a = false;
    let mut largest_minimum = -f32::MAX;
    output[132] = 0;

    for edge in 0..a_count {
        let mut previous = b_count - 1;
        let mut previous_distance = plane_distance(a, edge, vertex(b, previous));
        let mut minimum = previous_distance;
        let mut minimum_corner = previous;
        for corner in 0..b_count {
            let distance = plane_distance(a, edge, vertex(b, corner));
            if distance < minimum {
                minimum = distance;
                minimum_corner = corner;
            }
            if distance > 0.0 {
                masks[corner] |= OUTSIDE_B;
            }
            if !(distance * previous_distance > 0.0) {
                masks[previous] |= 1 << edge;
            }
            previous = corner;
            previous_distance = distance;
        }
        if minimum > largest_minimum {
            largest_minimum = minimum;
            nearest_corner = minimum_corner;
            nearest_edge = edge;
            edge_is_a = true;
        }
    }

    for corner in 0..b_count {
        if masks[corner] & OUTSIDE_B == 0 {
            let point = vertex(b, corner);
            append(output, project_to_face(a, point), point);
        }
    }
    // Containment is an early return; it must not duplicate all of A's vertices
    // when coincident faces have already contributed every B vertex.
    if output[132] as usize == b_count {
        return 1;
    }

    for edge in 0..b_count {
        let mut previous = a_count - 1;
        let mut previous_distance = plane_distance(b, edge, vertex(a, previous));
        let mut minimum = previous_distance;
        let mut minimum_corner = previous;
        for corner in 0..a_count {
            let distance = plane_distance(b, edge, vertex(a, corner));
            if distance < minimum {
                minimum = distance;
                minimum_corner = corner;
            }
            if distance > 0.0 {
                masks[corner] |= OUTSIDE_A;
            }
            let difference = previous_distance - distance;
            if !(distance * previous_distance > 0.0)
                && masks[edge] & (1 << previous) != 0
                && difference.abs() > f32::MIN_POSITIVE
            {
                let inverse_difference = 1.0 / difference;
                let origin = vertex(a, previous);
                let direction = load(a, 8 + previous * 16);
                let length = load(a, 16 + previous * 16);
                let point = std::array::from_fn(|lane| {
                    let numerator = length[lane] * previous_distance;
                    let along = inverse_difference * numerator;
                    direction[lane].mul_add(along, origin[lane])
                });
                append(output, point, project_to_face(b, point));
            }
            previous = corner;
            previous_distance = distance;
        }
        if minimum > largest_minimum {
            largest_minimum = minimum;
            nearest_corner = minimum_corner;
            nearest_edge = edge;
            edge_is_a = false;
        }
    }
    for corner in 0..a_count {
        if masks[corner] & OUTSIDE_A == 0 {
            let point = vertex(a, corner);
            append(output, point, project_to_face(b, point));
        }
    }
    if output[132] == 0 {
        output[132] = if edge_is_a {
            intersect_feature_corner_edge(output, b, a, nearest_corner, nearest_edge, false)
        } else {
            intersect_feature_corner_edge(output, a, b, nearest_corner, nearest_edge, true)
        };
        let delta = sub(load(output, 64), load(output, 0));
        let squared = dot(delta, delta);
        let length = if squared == 0.0 {
            0.0
        } else {
            squared * inverse_length(squared)
        };
        if length > f32::MIN_POSITIVE {
            output[128..132].copy_from_slice(&scale(delta, 1.0 / length).map(f32::to_bits));
            output[133] = 1;
        }
    }
    1
}
