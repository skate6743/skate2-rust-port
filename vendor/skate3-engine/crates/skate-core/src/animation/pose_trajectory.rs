//! TU3 FetchSys trajectory delta82D178D8. This modifies only the trajectory SQT.
use super::output::Sqt;
use crate::physics::native_arithmetic::dot3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoopTransform {
    pub rotation: [f32; 4],
    pub translation: [f32; 3],
}

/// BatchFetch82D20A40 selects the authored transform when the loop argument is
/// nonzero, otherwise identity. It does not exponentiate by the loop count.
pub fn delta(current: Sqt, previous: Sqt, loop_transform: Option<LoopTransform>) -> Sqt {
    let loop_transform = loop_transform.unwrap_or(LoopTransform {
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [0.0; 3],
    });
    let inverse_previous = [
        -previous.rotation[0],
        -previous.rotation[1],
        -previous.rotation[2],
        previous.rotation[3],
    ];
    let loop_displacement = rotate(current.rotation, loop_transform.translation);
    let displacement = core::array::from_fn(|i| {
        (loop_displacement[i] + current.translation[i]) - previous.translation[i]
    });
    let translation = rotate(inverse_previous, displacement);
    let rotation = multiply(
        multiply(loop_transform.rotation, current.rotation),
        inverse_previous,
    );
    Sqt {
        scale: current.scale,
        rotation,
        // The extractor restores the channel weight separately for channel clips.
        translation: [
            translation[0],
            translation[1],
            translation[2],
            current.translation[3],
        ],
    }
}

/// Fused cross arithmetic from82D17908..17A24, without quaternion normalization.
fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 3] {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
    ]
}

pub(super) fn rotate(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let vector = [v[0], v[1], v[2], 0.0];
    let first = cross(q, vector);
    let intermediate = core::array::from_fn(|i| {
        if i < 3 {
            q[3].mul_add(v[i], first[i])
        } else {
            0.0
        }
    });
    let second = cross(q, intermediate);
    core::array::from_fn(|i| 2.0f32.mul_add(second[i], v[i]))
}

pub(super) fn multiply(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let vector = cross(a, b);
    [
        a[0].mul_add(b[3], b[0].mul_add(a[3], vector[0])),
        a[1].mul_add(b[3], b[1].mul_add(a[3], vector[1])),
        a[2].mul_add(b[3], b[2].mul_add(a[3], vector[2])),
        a[3] * b[3] - dot3(a, b),
    ]
}
