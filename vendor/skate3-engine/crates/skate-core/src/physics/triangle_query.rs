//! Native triangle segment queries used by wheel probes and camera sweeps.
//! Thin test82AC7858 and dispatcher82ADF5D8, TU3. The world owns candidate
//! storage; these routines retain the source winding, tolerances and arithmetic.
use super::native_arithmetic;
use crate::math::Vector3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriangleLineHit {
    pub position: Vector3,
    pub normal: Vector3,
    pub fraction: f32,
    /// Native result48.xy: weights for vertices1 and2; z is penetration data.
    pub volume_parameter: [f32; 3],
}

/// TU3 dispatcher82ADF5D8. The rounded query uses the sum of the two radii,
/// then shifts its center hit back by the query radius. As in the native output
/// structure, fields not written by a branch retain the caller's values.
pub fn triangle_segment(
    result: &mut TriangleLineHit,
    start: Vector3,
    direction: Vector3,
    vertices: [Vector3; 3],
    line_radius: f32,
    triangle_fatness: f32,
) -> bool {
    let radius = line_radius + triangle_fatness;
    if radius == 0.0 {
        if let Some(hit) = thin_triangle(start, direction, vertices) {
            *result = hit;
            return true;
        }
        return false;
    }
    let normal = cross(sub(vertices[0], vertices[1]), sub(vertices[0], vertices[2]));
    result.normal = scale(normal, inverse_length(dot(normal, normal)));
    if super::triangle_sweep::sweep(result, start, direction, vertices, radius) {
        result.position = sub(result.position, scale(result.normal, line_radius));
        true
    } else {
        false
    }
}

/// Complete zero-total-radius branch. `direction` is end minus start, not a
/// normalized ray. TU3 accepts a small margin beyond both segment endpoints;
/// do not clamp its returned fraction or replace the determinant tolerance.
pub fn thin_triangle(
    start: Vector3,
    direction: Vector3,
    vertices: [Vector3; 3],
) -> Option<TriangleLineHit> {
    let [a, b, c] = vertices;
    let ac = sub(c, a);
    let ab = sub(b, a);
    let perpendicular = cross(direction, ac);
    let determinant = dot(ab, perpendicular);
    if !(determinant > f32::from_bits(0x322b_cc77)) {
        return None;
    }
    let from_a = sub(start, a);
    let lower = determinant * f32::from_bits(0xb727_c5ac);
    let upper = determinant - lower;
    let u = dot(from_a, perpendicular);
    if u < lower || u > upper {
        return None;
    }
    let perpendicular = cross(from_a, ab);
    let v = dot(direction, perpendicular);
    if v < lower || v + u > upper {
        return None;
    }
    let distance = dot(ac, perpendicular);
    if distance < lower || distance > upper {
        return None;
    }
    let inverse = 1.0 / determinant;
    let fraction = distance * inverse;
    // Dispatcher82ADF61C..688 recomputes and normalizes this winding with
    // two refinements; it does not use the Volume's cached contact normal.
    let normal = cross(sub(a, b), sub(a, c));
    Some(TriangleLineHit {
        position: madd(direction, fraction, start),
        normal: scale(normal, inverse_length(dot(normal, normal))),
        fraction,
        volume_parameter: [u * inverse, v * inverse, 0.0],
    })
}

pub(super) fn dot(a: Vector3, b: Vector3) -> f32 {
    native_arithmetic::dot3([a.x, a.y, a.z, 0.0], [b.x, b.y, b.z, 0.0])
}
pub(super) fn sub(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
pub(super) fn scale(v: Vector3, scale: f32) -> Vector3 {
    Vector3::new(v.x * scale, v.y * scale, v.z * scale)
}
pub(super) fn madd(a: Vector3, scale: f32, b: Vector3) -> Vector3 {
    Vector3::new(
        a.x.mul_add(scale, b.x),
        a.y.mul_add(scale, b.y),
        a.z.mul_add(scale, b.z),
    )
}
pub(super) fn cross(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(
        (-a.z).mul_add(b.y, a.y * b.z),
        (-a.x).mul_add(b.z, a.z * b.x),
        (-a.y).mul_add(b.x, a.x * b.y),
    )
}
pub(super) fn inverse_length(squared: f32) -> f32 {
    let mut inverse = native_arithmetic::reciprocal_square_root_estimate(squared);
    for _ in 0..2 {
        inverse = (inverse * 0.5).mul_add((-squared).mul_add(inverse * inverse, 1.0), inverse);
    }
    inverse
}

#[cfg(test)]
#[path = "tests/triangle_query.rs"]
mod tests;
