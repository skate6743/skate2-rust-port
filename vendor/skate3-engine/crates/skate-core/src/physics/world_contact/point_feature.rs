use super::{FeaturePrism, MaximumFeature, arithmetic::dot};

fn load(words: &[u32], offset: usize) -> [f32; 4] {
    std::array::from_fn(|i| f32::from_bits(words[offset + i]))
}

fn dot3(a: [f32; 4], b: [f32; 4]) -> f32 {
    dot(a[..3].try_into().unwrap(), b[..3].try_into().unwrap())
}

/// Complete TU3 82AC6EE8. Region codes are before=1, interior=2, after=3.
/// The upper-end comparison tests all four length lanes; equality is interior.
pub fn closest_feature_segment(segment: &[u32; 16], point: &mut [u32; 4]) -> u32 {
    let origin = load(segment, 0);
    let direction = load(segment, 4);
    let p = point.map(f32::from_bits);
    let along = dot3(std::array::from_fn(|i| p[i] - origin[i]), direction);
    if 0.0 > along {
        point.copy_from_slice(&segment[..4]);
        return 1;
    }
    let length = load(segment, 12);
    let after = length.iter().all(|&v| along > v);
    *point = std::array::from_fn(|i| {
        direction[i]
            .mul_add(if after { length[i] } else { along }, origin[i])
            .to_bits()
    });
    if after { 3 } else { 2 }
}

/// Complete TU3 82ACC160 point/face helper. `face_is_a` selects its two native
/// destination pointers within the prism; normal and untouched records survive.
/// The face header receives the selected edge index and segment-region code.
pub fn intersect_point_face(
    output: &mut FeaturePrism,
    face: &mut MaximumFeature,
    point: &MaximumFeature,
    normal: [u32; 4],
    face_is_a: bool,
) -> u32 {
    output[132] = 1;
    let p = load(point, 136);
    let normal = normal.map(f32::from_bits);
    let origin = load(face, 4);
    let distance = dot3(normal, std::array::from_fn(|i| origin[i] - p[i]));
    let mut projected = std::array::from_fn(|i| normal[i].mul_add(distance, p[i]));
    if (face[140] as i32) > 0 {
        let count = face[140] as usize;
        assert!(count <= 8, "native maximum-feature storage exceeded");
        let mut selected = 0;
        let mut worst = 0.0;
        for edge in 0..count {
            let start = load(face, 4 + edge * 16);
            let plane = load(face, 12 + edge * 16);
            let value = dot3(plane, std::array::from_fn(|i| projected[i] - start[i]));
            if edge == 0 || value > worst {
                worst = value;
                selected = edge;
            }
        }
        if worst > 0.0 {
            let plane = load(face, 12 + selected * 16);
            let mut projected_bits =
                std::array::from_fn(|i| (projected[i] - plane[i] * worst).to_bits());
            let segment = face[4 + selected * 16..20 + selected * 16]
                .try_into()
                .unwrap();
            let region = closest_feature_segment(segment, &mut projected_bits);
            face[0] = face[0].wrapping_add((selected as u32) * 2 + region);
            projected = projected_bits.map(f32::from_bits);
        }
    }
    let (face_offset, point_offset) = if face_is_a { (0, 64) } else { (64, 0) };
    output[face_offset..face_offset + 4].copy_from_slice(&projected.map(f32::to_bits));
    output[point_offset..point_offset + 4].copy_from_slice(&point[136..140]);
    1
}
