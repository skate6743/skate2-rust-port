use super::{MaximumFeature, closest_feature_segment, prism_math::*};

/// Complete TU3 82ACC298. Projects an exterior point onto the selected edge;
/// returns its edge/region code without modifying the feature header itself.
pub fn clamp_point_to_feature(
    face: &MaximumFeature,
    normal: [u32; 4],
    point: &mut [u32; 4],
) -> u32 {
    let normal = normal.map(f32::from_bits);
    let p = point.map(f32::from_bits);
    let count = (face[140] as i32).max(0) as usize;
    assert!(count <= 8, "native maximum-feature storage exceeded");
    let mut selected = 0;
    let mut minimum = 0.0;
    let mut selected_plane = [0.0; 4];
    for edge in 0..count {
        let mut plane = cross(normal, load(face, 8 + edge * 16));
        if dot(plane, plane) > f32::from_bits(0x3400_0000) {
            plane = normalized(plane);
        }
        let value = dot(plane, sub(p, load(face, 4 + edge * 16)));
        if edge == 0 || minimum > value {
            minimum = value;
            selected = edge;
            selected_plane = plane;
        }
    }
    if 0.0 > minimum {
        *point = sub(p, scale(selected_plane, minimum)).map(f32::to_bits);
        closest_feature_segment(
            face[4 + selected * 16..20 + selected * 16]
                .try_into()
                .unwrap(),
            point,
        ) + selected as u32 * 2
    } else {
        0
    }
}

/// Complete TU3 82ACC400 interval clipping. `interval` stores the two scalar
/// limits; `outside` stores the last rejecting edge origin and normalized plane.
/// Those vectors are preserved unless a near-parallel exterior edge writes them.
pub fn clip_segment_to_feature(
    face: &MaximumFeature,
    segment: &MaximumFeature,
    normal: [u32; 4],
    interval: &mut [u32; 2],
    outside: &mut [u32; 8],
) -> u32 {
    interval[0] = 0;
    // lfs/stfs quiet an input signalling NaN in the scalar length field.
    interval[1] = if f32::from_bits(segment[16]).is_nan() {
        segment[16] | 0x0040_0000
    } else {
        segment[16]
    };
    let normal = normal.map(f32::from_bits);
    let count = (face[140] as i32).max(0) as usize;
    assert!(count <= 8, "native maximum-feature storage exceeded");
    let mut rejected = false;
    for edge in 0..count {
        let plane = cross(normal, load(face, 8 + edge * 16));
        let squared = dot(plane, plane);
        let length = if squared == 0.0 {
            0.0
        } else {
            squared * super::prism_math::inverse_length(squared)
        };
        if length < f32::from_bits(0x3a83_126f) {
            continue;
        }
        let plane = normalized(plane);
        let origin = load(face, 4 + edge * 16);
        let denominator = dot(load(segment, 8), plane);
        let numerator = dot(sub(origin, load(segment, 4)), plane);
        if denominator.abs() < f32::from_bits(0x3d4c_cccd) {
            if numerator > 0.0 {
                rejected = true;
                outside[..4].copy_from_slice(&face[4 + edge * 16..8 + edge * 16]);
                outside[4..].copy_from_slice(&plane.map(f32::to_bits));
            }
        } else {
            let position = numerator / denominator;
            if denominator > 0.0 {
                if position > f32::from_bits(interval[0]) {
                    interval[0] = position.to_bits();
                }
                if f32::from_bits(interval[0]) > f32::from_bits(interval[1]) {
                    interval[0] = interval[1];
                    return 0;
                }
            } else {
                if position < f32::from_bits(interval[1]) {
                    interval[1] = position.to_bits();
                }
                if f32::from_bits(interval[1]) < f32::from_bits(interval[0]) {
                    interval[1] = interval[0];
                    return 0;
                }
            }
        }
    }
    u32::from(!rejected)
}
