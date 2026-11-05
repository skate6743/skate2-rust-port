use super::{FeaturePrism, MaximumFeature, closest_feature_segment, prism_math::*};

fn store(output: &mut FeaturePrism, offset: usize, point: V) {
    output[offset..offset + 4].copy_from_slice(&point.map(f32::to_bits));
}

fn endpoint(feature: &mut MaximumFeature, start: bool) -> V {
    feature[0] = if start {
        feature[0].wrapping_sub(1)
    } else {
        feature[0].wrapping_add(1)
    };
    let origin = load(feature, 4);
    if start {
        origin
    } else {
        let direction = load(feature, 8);
        let length = load(feature, 16);
        std::array::from_fn(|i| direction[i].mul_add(length[i], origin[i]))
    }
}

/// Complete TU3 82ACBAD0 segment/segment prism, including the parallel overlap
/// and disjoint endpoint branches. Normal/flags and untouched point slots survive.
/// `a_is_first` represents the helper's two independently supplied output pointers.
pub fn intersect_feature_segments(
    output: &mut FeaturePrism,
    a: &mut MaximumFeature,
    b: &mut MaximumFeature,
    normal: [u32; 4],
    a_is_first: bool,
) -> u32 {
    let (ao, bo) = if a_is_first { (0, 64) } else { (64, 0) };
    let ad = load(a, 8);
    let bd = load(b, 8);
    let start_a = load(a, 4);
    let start_b = load(b, 4);
    let perpendicular = cross(ad, bd);
    if dot(perpendicular, perpendicular) > f32::from_bits(0x3400_0000) {
        let plane = cross(normalized(perpendicular), bd);
        let mut along = dot(sub(start_b, start_a), plane) / dot(ad, plane);
        if along < 0.0 {
            along = 0.0;
        }
        let length = load(a, 16);
        if length.iter().all(|&v| along > v) {
            along = length[0];
        }
        output[132] = 1;
        let mut point = madd(ad, along, start_a).map(f32::to_bits);
        let region = closest_feature_segment(b[4..20].try_into().unwrap(), &mut point);
        b[0] = b[0].wrapping_sub(region).wrapping_add(2);
        output[bo..bo + 4].copy_from_slice(&point);
        let region = closest_feature_segment(a[4..20].try_into().unwrap(), &mut point);
        a[0] = a[0].wrapping_sub(region).wrapping_add(2);
        output[ao..ao + 4].copy_from_slice(&point);
        return 1;
    }
    let normal = normal.map(f32::from_bits);
    let axis = if dot(normal, ad).abs() < dot(normal, bd).abs() {
        ad
    } else {
        bd
    };
    let al = load(a, 16);
    let bl = load(b, 16);
    let end_a = std::array::from_fn(|i| ad[i].mul_add(al[i], start_a[i]));
    let end_b = std::array::from_fn(|i| bd[i].mul_add(bl[i], start_b[i]));
    let a0 = dot(axis, start_a);
    let b0 = dot(axis, start_b);
    let a1 = dot(axis, end_a);
    let b1 = dot(axis, end_b);
    // Native compare/select ordering also specifies unordered/equal operands.
    let (amin, amax) = if a1 > a0 { (a0, a1) } else { (a1, a0) };
    let (bmin, bmax) = if b1 > b0 { (b0, b1) } else { (b1, b0) };
    let low = if amin > bmin { amin } else { bmin };
    let high = if bmax > amax { amax } else { bmax };
    if high > low {
        let ap = normalized(cross(cross(normal, ad), ad));
        let bp = normalized(cross(cross(normal, bd), bd));
        for (index, along) in [low, high].into_iter().enumerate() {
            let anchor = scale(axis, along);
            for (offset, origin, direction, plane) in [(ao, start_a, ad, ap), (bo, start_b, bd, bp)]
            {
                let inverse = reciprocal(dot(normal, plane));
                let distance = inverse * dot(sub(origin, anchor), plane);
                let projected = madd(normal, distance, anchor);
                let projected = madd(direction, dot(sub(projected, origin), direction), origin);
                store(output, offset + index * 4, projected);
            }
        }
        output[132] = 2;
    } else {
        output[132] = 1;
        let (a_start, b_start) = if bmin > amin {
            (a0 > a1, b0 < b1)
        } else {
            (a0 < a1, b0 > b1)
        };
        store(output, ao, endpoint(a, a_start));
        store(output, bo, endpoint(b, b_start));
    }
    1
}
