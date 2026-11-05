//! Standard SQT blend in TU3 ACS828CB418..4A4 and worker828D6BA8..6C2C.
//! The caller owns tree order and coefficients; interpolation does not flatten
//! nested blends or use a renderer's quaternion blend implementation.
use super::output::{PoseBufferError, Sqt};
use crate::physics::native_arithmetic;

pub fn blend(
    first: &[Sqt],
    second: &[Sqt],
    weight: f32,
    output: &mut [Sqt],
) -> Result<(), PoseBufferError> {
    if first.len() != second.len() {
        return Err(PoseBufferError::ShortInput);
    }
    if output.len() < first.len() {
        return Err(PoseBufferError::ShortOutput);
    }
    for ((&a, &b), destination) in first.iter().zip(second).zip(output) {
        *destination = blend_sample(a, b, weight);
    }
    Ok(())
}

pub fn blend_sample(first: Sqt, second: Sqt, weight: f32) -> Sqt {
    let positive = native_arithmetic::dot4(first.rotation, second.rotation) > 0.0;
    let rotation = core::array::from_fn(|lane| {
        if positive {
            (second.rotation[lane] - first.rotation[lane]).mul_add(weight, first.rotation[lane])
        } else {
            (-(second.rotation[lane] + first.rotation[lane])).mul_add(weight, first.rotation[lane])
        }
    });
    let squared = native_arithmetic::dot4(rotation, rotation);
    let mut inverse = native_arithmetic::reciprocal_square_root_estimate(squared);
    for _ in 0..2 {
        inverse = (inverse * 0.5).mul_add((-squared).mul_add(inverse * inverse, 1.0), inverse);
    }
    Sqt {
        scale: interpolate(first.scale, second.scale, weight),
        rotation: rotation.map(|lane| lane * inverse),
        translation: interpolate(first.translation, second.translation, weight),
    }
}

/// ACSChannelBlend828CC91C..CA64 selects the first subtree's translation.W
/// when requested, otherwise the second's, and clamps the weighted coefficient.
pub fn channel_blend_sample(first: Sqt, second: Sqt, weight: f32, use_first_weights: bool) -> Sqt {
    let channel = if use_first_weights {
        first.translation[3]
    } else {
        second.translation[3]
    };
    let coefficient = weight * channel;
    let coefficient = if coefficient > 1.0 { 1.0 } else { coefficient };
    let coefficient = if 0.0 > coefficient { 0.0 } else { coefficient };
    blend_sample(first, second, coefficient)
}

fn interpolate(first: [f32; 4], second: [f32; 4], weight: f32) -> [f32; 4] {
    core::array::from_fn(|lane| (second[lane] - first[lane]).mul_add(weight, first[lane]))
}

///ACS828CBF68..828CC0B4: weighted SQT accumulation in authored order,
///then one quaternion normalization. This is distinct from repeated two-pose
///shortest-arc blending; the native multi-pose path does not flip hemispheres.
pub fn weighted(poses: &[Vec<Sqt>], weights: &[f32]) -> Result<Vec<Sqt>, PoseBufferError> {
    let Some(first) = poses.first() else {
        return Err(PoseBufferError::ShortInput);
    };
    if poses.len() != weights.len() || poses.iter().any(|p| p.len() != first.len()) {
        return Err(PoseBufferError::ShortInput);
    }
    if weights.iter().any(|weight| !weight.is_finite()) {
        return Err(PoseBufferError::NonFiniteWeight);
    }
    let mut output = first
        .iter()
        .map(|s| Sqt {
            scale: s.scale.map(|v| v * weights[0]),
            rotation: s.rotation.map(|v| v * weights[0]),
            translation: s.translation.map(|v| v * weights[0]),
        })
        .collect::<Vec<_>>();
    for (pose, &weight) in poses.iter().zip(weights).skip(1) {
        for (out, s) in output.iter_mut().zip(pose) {
            for lane in 0..4 {
                out.scale[lane] = s.scale[lane].mul_add(weight, out.scale[lane]);
                out.rotation[lane] = s.rotation[lane].mul_add(weight, out.rotation[lane]);
                out.translation[lane] = s.translation[lane].mul_add(weight, out.translation[lane]);
            }
        }
    }
    for out in &mut output {
        let squared = native_arithmetic::dot4(out.rotation, out.rotation);
        if !squared.is_finite() || squared <= 0.0 {
            return Err(PoseBufferError::InvalidQuaternion);
        }
        let mut inverse = native_arithmetic::reciprocal_square_root_estimate(squared);
        for _ in 0..2 {
            inverse = (inverse * 0.5).mul_add((-squared).mul_add(inverse * inverse, 1.), inverse);
        }
        out.rotation = out.rotation.map(|v| v * inverse);
        if out.rotation.iter().any(|v| !v.is_finite()) {
            return Err(PoseBufferError::InvalidQuaternion);
        }
    }
    Ok(output)
}

#[cfg(test)]
#[path = "tests/pose_blend.rs"]
mod tests;
