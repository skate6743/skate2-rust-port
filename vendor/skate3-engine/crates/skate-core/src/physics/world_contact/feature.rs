use super::MaximumFeature;
use crate::physics::reciprocal_sqrt::estimate;

fn load(words: &[u32], offset: usize) -> [f32; 4] {
    std::array::from_fn(|i| f32::from_bits(words[offset + i]))
}

/// Complete 82AC6E50 segment initialization. Writes origin, normalized direction
/// and broadcast length; preserves the unwritten plane vector at words8..12.
pub fn initialize_feature_segment(output: &mut [u32; 16], origin: [u32; 4], end: [u32; 4]) {
    output[..4].copy_from_slice(&origin);
    let origin = origin.map(f32::from_bits);
    let end = end.map(f32::from_bits);
    let delta: [f32; 4] = std::array::from_fn(|i| end[i] - origin[i]);
    let squared = super::arithmetic::dot(
        delta[..3].try_into().unwrap(),
        delta[..3].try_into().unwrap(),
    );
    // One rsqrt refinement, then one reciprocal refinement. Runtime initializer
    // 82F837D8 broadcasts 8212BF78 (34000000) to threshold vector830BDD40.
    let mut reciprocal = estimate(squared);
    reciprocal =
        (reciprocal * 0.5).mul_add((-squared).mul_add(reciprocal * reciprocal, 1.0), reciprocal);
    let length = if squared > f32::from_bits(0x3400_0000) {
        if squared == 0.0 {
            0.0
        } else {
            squared * reciprocal
        }
    } else {
        0.0
    };
    output[12..16].fill(length.to_bits());
    let r = super::prism_math::reciprocal_estimate(length);
    let r = r.mul_add((-r).mul_add(length, 1.0), r);
    let direction = if squared > f32::from_bits(0x3400_0000) {
        delta.map(|x| x * r)
    } else {
        [0.0; 4]
    };
    output[4..8].copy_from_slice(&direction.map(f32::to_bits));
}

/// Complete capsule maximum-feature callback82AD98A0. `segment_scratch` carries
/// the native local64-byte segment, including its unwritten plane vector.
/// No fabricated value is assigned to those copied scratch words. Native r4
/// mode is unused by this capsule callback.
pub fn capsule_maximum_feature(
    gp: &[u32; 48],
    direction: [u32; 4],
    output: &mut MaximumFeature,
    segment_scratch: &mut [u32; 16],
) {
    let axis = load(gp, 16);
    let normal = direction.map(f32::from_bits);
    let projection = super::arithmetic::dot(
        axis[..3].try_into().unwrap(),
        normal[..3].try_into().unwrap(),
    );
    let half = f32::from_bits(gp[28]);
    let center = load(gp, 0);
    if projection.abs() < f32::from_bits(0x3d4c_cccd) {
        let scaled = axis.map(|v| v * half);
        let plus = std::array::from_fn(|i| (center[i] + scaled[i]).to_bits());
        let minus = std::array::from_fn(|i| (center[i] - scaled[i]).to_bits());
        initialize_feature_segment(segment_scratch, plus, minus);
        output[4..20].copy_from_slice(segment_scratch);
        output[140] = 1;
    } else {
        let point: [f32; 4] = if projection > 0.0 {
            std::array::from_fn(|i| axis[i].mul_add(half, center[i]))
        } else {
            std::array::from_fn(|i| center[i] - axis[i] * half)
        };
        output[136..140].copy_from_slice(&point.map(f32::to_bits));
        output[140] = 0;
    }
    output[0] = 0;
}
