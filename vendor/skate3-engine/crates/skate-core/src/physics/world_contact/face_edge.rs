use super::{FeaturePrism, MaximumFeature, prism_math::*};

fn store(output: &mut FeaturePrism, offset: usize, point: V) {
    output[offset..offset + 4].copy_from_slice(&point.map(f32::to_bits));
}

fn end(feature: &MaximumFeature, edge: usize) -> V {
    let origin = load(feature, 4 + edge * 16);
    let direction = load(feature, 8 + edge * 16);
    let length = load(feature, 16 + edge * 16);
    std::array::from_fn(|i| direction[i].mul_add(length[i], origin[i]))
}

/// Complete TU3 82ACCA28 corner/edge fallback. Returns one or two point pairs;
/// the caller writes the prism count. Both feature headers are replaced here.
pub fn intersect_feature_corner_edge(
    output: &mut FeaturePrism,
    a: &mut MaximumFeature,
    b: &mut MaximumFeature,
    corner: usize,
    edge: usize,
    a_is_first: bool,
) -> u32 {
    let (ao, bo) = if a_is_first { (0, 64) } else { (64, 0) };
    let previous = if corner != 0 {
        corner - 1
    } else {
        a[140] as usize - 1
    };
    assert!(corner < 8 && edge < 8 && previous < 8);
    let bd = load(b, 8 + edge * 16);
    let mut selected = previous;
    let nearly_parallel =
        |index| f32::from_bits(0x3d4c_cccd) > dot(load(a, 12 + index * 16), bd).abs();
    let parallel = if nearly_parallel(previous) {
        true
    } else {
        selected = corner;
        nearly_parallel(corner)
    };
    let b_origin = load(b, 4 + edge * 16);
    if !parallel {
        a[0] = (corner as u32) * 2 + 1;
        b[0] = (edge as u32 + 1) * 2;
        let origin = load(a, 4 + corner * 16);
        let mut position = dot(bd, sub(origin, b_origin));
        if position < f32::MIN_POSITIVE {
            b[0] = b[0].wrapping_sub(1);
            position = 0.0;
        } else {
            let length = load(b, 16 + edge * 16);
            if length.iter().all(|&v| position > v) {
                b[0] = b[0].wrapping_add(1);
                position = length[0];
            }
        }
        store(output, ao, origin);
        store(output, bo, madd(bd, position, b_origin));
        return 1;
    }
    a[0] = (selected as u32 + 1) * 2;
    b[0] = (edge as u32 + 1) * 2;
    let a_origin = load(a, 4 + selected * 16);
    let ad = load(a, 8 + selected * 16);
    let distance = dot(ad, sub(b_origin, a_origin));
    if distance < f32::MIN_POSITIVE {
        store(output, ao, a_origin);
        store(output, bo, b_origin);
        a[0] = a[0].wrapping_sub(1);
        b[0] = b[0].wrapping_sub(1);
        return 1;
    }
    let al = load(a, 16 + selected * 16);
    let bl = load(b, 16 + edge * 16);
    if (0..4).all(|i| f32::MIN_POSITIVE > (al[i] + bl[i]) - distance) {
        store(output, ao, end(a, selected));
        store(output, bo, end(b, edge));
        a[0] = a[0].wrapping_add(1);
        b[0] = b[0].wrapping_add(1);
        return 1;
    }
    if al.iter().all(|&v| f32::MIN_POSITIVE > v - distance) {
        store(output, ao, end(a, selected));
        store(
            output,
            bo,
            sub(
                b_origin,
                std::array::from_fn(|i| bd[i] * (al[i] - distance)),
            ),
        );
    } else {
        store(output, ao, madd(ad, distance, a_origin));
        store(output, bo, b_origin);
    }
    if bl.iter().all(|&v| f32::MIN_POSITIVE > v - distance) {
        store(
            output,
            ao + 4,
            sub(
                a_origin,
                std::array::from_fn(|i| ad[i] * (bl[i] - distance)),
            ),
        );
        store(output, bo + 4, end(b, edge));
    } else {
        store(output, ao + 4, a_origin);
        store(output, bo + 4, madd(bd, distance, b_origin));
    }
    2
}
