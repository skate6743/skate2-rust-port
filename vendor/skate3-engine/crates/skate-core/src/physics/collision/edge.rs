//! TU3 EdgeCosTest / EdgeCosTestFull, 82AD2860 / 82AD2928.
use super::arithmetic::*;
use crate::math::Vector3;

pub(super) fn edge_cos_test(
    edge: Vector3,
    face: Vector3,
    normal: Vector3,
    cosine: f32,
    convex: bool,
    two_sided: bool,
) -> bool {
    if dot(normal, cross(edge, face)) < 0.0 {
        return true;
    }
    if !convex && !two_sided {
        return false;
    }
    let projected = normalize(sub(normal, scale(edge, dot(normal, edge))), 2);
    dot(projected, if convex { face } else { neg(face) }) >= cosine
}
