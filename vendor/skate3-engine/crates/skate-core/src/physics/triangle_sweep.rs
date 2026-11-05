//! Rounded triangle query82ADEB70: bounds, face, overlap, then feature walk.
use super::{
    native_arithmetic,
    triangle_closest::closest,
    triangle_query::{TriangleLineHit, cross, dot, madd, scale, sub},
    triangle_sweep_walk,
};
use crate::math::Vector3;

/// Literal8212BF74, also used by closest-feature and walk denominator gates.
pub(super) const MIN_RECIPROCAL: f32 = f32::from_bits(0x0020_0000);

pub(super) fn sweep(
    result: &mut TriangleLineHit,
    start: Vector3,
    mut delta: Vector3,
    vertices: [Vector3; 3],
    radius: f32,
) -> bool {
    result.fraction = 0.0;
    let [a, b, c] = vertices;
    let ab = sub(b, a);
    let ac = sub(c, a);
    let mut from_a = sub(start, a);
    let cross_ac = cross(delta, ac);
    let mut determinant = dot(ab, cross_ac);
    let mut side = 1.0;
    if determinant < 0.0 {
        side = -1.0;
        determinant = -determinant;
    }
    if dot(result.normal, from_a) * side < -radius {
        return false;
    }
    let u = dot(from_a, cross_ac) * side;
    // This TU3 body uses the L1 sum, not the SDK's vector magnitude bound.
    let u_margin = l1(cross_ac) * radius;
    if u < -u_margin || u > u_margin + determinant {
        return false;
    }
    let cross_ab = cross(ab, delta);
    let v = dot(from_a, cross_ab) * side;
    let v_margin = l1(cross_ab) * radius;
    if v < -v_margin || v > v_margin + determinant || v + u > (u_margin + v_margin) + determinant {
        return false;
    }
    if determinant > MIN_RECIPROCAL {
        let offset = sub(from_a, scale(result.normal, side * radius));
        let distance = -(dot(offset, cross(ac, ab)) * side);
        if distance > determinant {
            return false;
        }
        if !(distance < 0.0) {
            let inverse = 1.0 / determinant;
            result.fraction = inverse * distance;
            let u = dot(offset, cross_ac) * side;
            if !(u < 0.0) && !(u > determinant) {
                let v = dot(offset, cross_ab) * side;
                if !(v < 0.0) && !(v + u > determinant) {
                    result.position = madd(delta, result.fraction, start);
                    result.normal = scale(result.normal, side);
                    result.volume_parameter = [inverse * u, inverse * v, 0.0];
                    return true;
                }
            }
            from_a = madd(delta, result.fraction, from_a);
            delta = scale(delta, 1.0 - result.fraction);
        }
    }
    let start = add(from_a, a);
    let feature = closest(start, vertices);
    let separation = sub(start, feature.point);
    let squared = dot(separation, separation);
    let penetration = radius.mul_add(radius, -squared);
    if penetration > 0.0 {
        result.position = start;
        if squared > 0.0 {
            let inverse = native_arithmetic::reciprocal_square_root_estimate(squared);
            let inverse =
                (inverse * 0.5).mul_add((-squared).mul_add(inverse * inverse, 1.0), inverse);
            result.normal = scale(separation, inverse);
        }
        result.volume_parameter = [feature.u, feature.v, penetration];
        return true;
    }
    if feature.region == 6 {
        result.position = start;
        //82ADEFA0 compares the absolute determinant retained in f1; it has
        // already been negated by82ADEC30 for a reverse-facing query.
        if determinant < 0.0 {
            result.normal = scale(result.normal, -1.0);
        }
        result.volume_parameter = [feature.u, feature.v, 0.0];
        return true;
    }
    triangle_sweep_walk::walk(result, start, delta, vertices, feature, radius)
}

pub(super) fn add(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
fn l1(v: Vector3) -> f32 {
    (v.y.abs() + v.z.abs()) + v.x.abs()
}
