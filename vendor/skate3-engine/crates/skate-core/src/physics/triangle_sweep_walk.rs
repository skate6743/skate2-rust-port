//! Five-step vertex/edge region walk82ADF008..82ADF5BC.
use super::{
    native_arithmetic,
    triangle_closest::ClosestTrianglePoint,
    triangle_query::{TriangleLineHit, dot, madd, scale, sub},
    triangle_sweep::{MIN_RECIPROCAL, add},
    triangle_sweep_roots::{self as roots, Fraction, Root},
};
use crate::math::Vector3;

pub(super) fn walk(
    result: &mut TriangleLineHit,
    mut start: Vector3,
    mut delta: Vector3,
    vertices: [Vector3; 3],
    closest: ClosestTrianglePoint,
    radius: f32,
) -> bool {
    let mut region = closest.region;
    let mut vertex = closest.point;
    for _ in 0..5 {
        let fraction;
        if region <= 2 {
            let mut hit = match roots::sphere(start, delta, vertex, radius) {
                Root::Away => return false,
                Root::Miss => None,
                Root::Hit(fraction) => Some(fraction),
            };
            let adjacent = match region {
                0 => [sub(vertices[1], vertices[0]), sub(vertices[2], vertices[0])],
                1 => [sub(vertices[0], vertices[1]), sub(vertices[2], vertices[1])],
                _ => [sub(vertices[0], vertices[2]), sub(vertices[1], vertices[2])],
            };
            let mut next_region = region;
            for (index, edge) in adjacent.into_iter().enumerate() {
                let candidate = Fraction {
                    numerator: dot(sub(vertex, start), edge),
                    denominator: dot(delta, edge),
                };
                if replaces(hit, candidate) {
                    hit = Some(candidate);
                    next_region = (region + if index == 0 { 6 } else { 9 }) / 2;
                }
            }
            let Some(hit) = hit else { return false };
            fraction = hit.value();
            if next_region <= 2 {
                advance_fraction(result, fraction);
                result.position = madd(delta, fraction, start);
                result.normal = scale(sub(result.position, vertex), inverse_radius(radius));
                result.volume_parameter = [
                    if region == 1 { 1.0 } else { 0.0 },
                    if region == 2 { 1.0 } else { 0.0 },
                    0.0,
                ];
                return true;
            }
            region = next_region;
        } else {
            let (origin, edge) = match region {
                3 => (vertices[0], sub(vertices[1], vertices[0])),
                4 => (vertices[0], sub(vertices[2], vertices[0])),
                _ => (vertices[1], sub(vertices[2], vertices[1])),
            };
            vertex = origin;
            let mut hit = match roots::cylinder(start, delta, origin, edge, radius) {
                Root::Away => return false,
                Root::Miss => None,
                Root::Hit(fraction) => Some(fraction),
            };
            let end = add(origin, edge);
            let forward = Fraction {
                numerator: dot(sub(end, start), edge),
                denominator: dot(delta, edge),
            };
            let backward = Fraction {
                numerator: dot(sub(start, origin), edge),
                denominator: -forward.denominator,
            };
            if replaces(hit, forward) {
                hit = Some(forward);
                vertex = end;
                region /= 2;
            } else if replaces(hit, backward) {
                hit = Some(backward);
                region = (region - 3) / 2;
            }
            let Some(hit) = hit else { return false };
            fraction = hit.value();
            if region > 2 {
                advance_fraction(result, fraction);
                result.position = madd(delta, fraction, start);
                let along_edge = dot(sub(result.position, origin), edge) * (1.0 / dot(edge, edge));
                let nearest = madd(edge, along_edge, origin);
                result.normal = scale(sub(result.position, nearest), inverse_radius(radius));
                result.volume_parameter = match region {
                    3 => [along_edge, 0.0, 0.0],
                    4 => [0.0, along_edge, 0.0],
                    _ => [1.0 - along_edge, along_edge, 0.0],
                };
                return true;
            }
        }
        advance_fraction(result, fraction);
        start = madd(delta, fraction, start);
        delta = scale(delta, 1.0 - fraction);
        if fraction > 1.0 {
            return false;
        }
    }
    // The source returns success after its fifth transition. Position/normal/
    // parameters have not been written by the walk on this path; preserve them.
    true
}

fn replaces(hit: Option<Fraction>, candidate: Fraction) -> bool {
    candidate.denominator > MIN_RECIPROCAL
        && candidate.numerator > 0.0
        && hit.is_none_or(|hit| !hit.before(candidate))
}
fn advance_fraction(result: &mut TriangleLineHit, fraction: f32) {
    result.fraction = (1.0 - result.fraction).mul_add(fraction, result.fraction);
}
fn inverse_radius(radius: f32) -> f32 {
    let mut inverse = native_arithmetic::reciprocal_estimate(radius);
    for _ in 0..2 {
        inverse = inverse.mul_add((-inverse).mul_add(radius, 1.0), inverse);
    }
    inverse
}
