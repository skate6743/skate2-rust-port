//! TU3 triangle closest-feature calculation82ADE8B8.
use super::triangle_query::{dot, madd, sub};
use crate::math::Vector3;

pub(super) struct ClosestTrianglePoint {
    pub point: Vector3,
    /// Vertex0..2, edge AB/AC/BC3..5, face6, as used by82ADEB70.
    pub region: u32,
    pub u: f32,
    pub v: f32,
}

pub(super) fn closest(point: Vector3, vertices: [Vector3; 3]) -> ClosestTrianglePoint {
    let [origin, b, c] = vertices;
    let edge_b = sub(b, origin);
    let edge_c = sub(c, origin);
    let toward_origin = sub(origin, point);
    let bb = dot(edge_b, edge_b);
    let cc = dot(edge_c, edge_c);
    let bc = dot(edge_b, edge_c);
    let bp = dot(edge_b, toward_origin);
    let cp = dot(edge_c, toward_origin);
    let determinant = cc.mul_add(bb, -(bc * bc));
    let u_numerator = cp.mul_add(bc, -(bp * cc));
    let v_numerator = bp.mul_add(bc, -(cp * bb));
    let opposite_length_squared = (-bc).mul_add(2.0, bb) + cc;
    let feature = if determinant < f32::from_bits(0x0020_0000) {
        if bb > cc {
            if bb > opposite_length_squared { 0 } else { 2 }
        } else if cc > opposite_length_squared {
            1
        } else {
            2
        }
    } else if u_numerator + v_numerator > determinant {
        if u_numerator < 0.0 {
            if cp + cc < bp + bc { 1 } else { 2 }
        } else if v_numerator < 0.0 {
            if bp + bb < cp + bc { 0 } else { 2 }
        } else {
            2
        }
    } else if u_numerator < 0.0 {
        if -cp > 0.0 { 1 } else { 0 }
    } else if v_numerator < 0.0 {
        if -bp > 0.0 { 0 } else { 1 }
    } else {
        3
    };

    let (u, v, region) = match feature {
        0 => {
            if !(bp < 0.0) {
                (0.0, 0.0, 0)
            } else if !(-bp < bb) {
                (1.0, 0.0, 1)
            } else {
                (-(bp / bb), 0.0, 3)
            }
        }
        1 => {
            if !(cp < 0.0) {
                (0.0, 0.0, 0)
            } else if !(-cp < cc) {
                (0.0, 1.0, 2)
            } else {
                (0.0, -(cp / cc), 4)
            }
        }
        2 => {
            let numerator = ((cp + cc) - bc) - bp;
            if !(numerator > 0.0) {
                (0.0, 1.0, 2)
            } else if !(numerator < opposite_length_squared) {
                (1.0, 0.0, 1)
            } else {
                let u = numerator / opposite_length_squared;
                (u, 1.0 - u, 5)
            }
        }
        _ => {
            let inverse = 1.0 / determinant;
            (inverse * u_numerator, inverse * v_numerator, 6)
        }
    };
    ClosestTrianglePoint {
        point: madd(edge_c, v, madd(edge_b, u, origin)),
        region,
        u,
        v,
    }
}
