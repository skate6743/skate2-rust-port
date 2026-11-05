use super::{
    FeaturePrism, MaximumFeature, clamp_point_to_feature, clip_segment_to_feature,
    closest_feature_segment, prism_math::*,
};

fn store(output: &mut FeaturePrism, offset: usize, point: V) {
    output[offset..offset + 4].copy_from_slice(&point.map(f32::to_bits));
}

/// Complete TU3 82ACC5F0 segment/face prism and its interval/edge-clamp callees.
/// Keeps the normal correction flag, feature tags and both output-pointer orders.
pub fn intersect_segment_face(
    output: &mut FeaturePrism,
    face: &mut MaximumFeature,
    segment: &mut MaximumFeature,
    normal_bits: [u32; 4],
    face_is_a: bool,
) -> u32 {
    let (fo, so) = if face_is_a { (0, 64) } else { (64, 0) };
    let mut interval = [0; 2];
    // The native caller explicitly zeroes both scratch vectors before clipping.
    let mut outside = [0; 8];
    let inside = clip_segment_to_feature(face, segment, normal_bits, &mut interval, &mut outside);
    let normal = normal_bits.map(f32::from_bits);
    let origin = load(segment, 4);
    let direction = load(segment, 8);
    let face_origin = load(face, 4);
    let low = f32::from_bits(interval[0]);
    let high = f32::from_bits(interval[1]);
    if inside != 0 {
        output[132] = 2;
        let face_normal = load(face, 132);
        for (i, distance) in [low, high].into_iter().enumerate() {
            let p = madd(direction, distance, origin);
            store(output, so + i * 4, p);
            let t = dot(sub(face_origin, p), face_normal) / dot(normal, face_normal);
            store(output, fo + i * 4, madd(normal, t, p));
        }
        return 1;
    }
    let span = high - low;
    if span > f32::from_bits(0x3a83_126f) {
        output[132] = 2;
        let outside_origin = load(&outside, 0);
        let outside_plane = load(&outside, 4);
        let projected = madd(normal, dot(normal, sub(face_origin, origin)), origin);
        let projected = madd(
            outside_plane,
            dot(outside_plane, sub(outside_origin, projected)),
            projected,
        );
        let first = madd(direction, low, projected);
        let second = madd(direction, span, first);
        let mut first = first.map(f32::to_bits);
        let mut second = second.map(f32::to_bits);
        let first_code = clamp_point_to_feature(face, normal_bits, &mut first);
        let second_code = clamp_point_to_feature(face, normal_bits, &mut second);
        face[0] = face[0].wrapping_add(first_code.max(second_code) & !1);
        output[fo..fo + 4].copy_from_slice(&first);
        output[fo + 4..fo + 8].copy_from_slice(&second);
        closest_feature_segment(segment[4..20].try_into().unwrap(), &mut first);
        closest_feature_segment(segment[4..20].try_into().unwrap(), &mut second);
        output[so..so + 4].copy_from_slice(&first);
        output[so + 4..so + 8].copy_from_slice(&second);
        let delta = sub(load(output, fo), first.map(f32::from_bits));
        let squared = dot(delta, delta);
        let length = if squared == 0.0 {
            0.0
        } else {
            squared * inverse_length(squared)
        };
        if length > f32::MIN_POSITIVE {
            output[133] = 1;
            store(
                output,
                128,
                normalized(cross(direction, cross(delta, direction))),
            );
        }
    } else {
        output[132] = 1;
        let p = madd(direction, low, origin);
        let mut p = madd(normal, dot(normal, sub(face_origin, p)), p).map(f32::to_bits);
        let code = clamp_point_to_feature(face, normal_bits, &mut p);
        face[0] = face[0].wrapping_add(code);
        output[fo..fo + 4].copy_from_slice(&p);
        let region = closest_feature_segment(segment[4..20].try_into().unwrap(), &mut p);
        segment[0] = segment[0].wrapping_sub(region).wrapping_add(2);
        output[so..so + 4].copy_from_slice(&p);
    }
    1
}
