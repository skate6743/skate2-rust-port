//! Geometric helpers82E0A060/82E0A258/82D85C48 and tangent8252D980.
use super::{Vector, ZERO, cross, dot, length, scale, sub};
pub(super) fn madd(a: Vector, s: f32, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i].mul_add(s, b[i]))
}
pub(super) fn reciprocal(x: f32) -> f32 {
    let mut r = crate::physics::native_arithmetic::reciprocal_estimate(x);
    for _ in 0..2 {
        r = r.mul_add((-r).mul_add(x, 1.), r);
    }
    r
}
pub(super) fn normalize(v: Vector, fallback: Vector) -> Vector {
    let l = length(v);
    if l > f32::from_bits(0x358637bd) {
        scale(v, reciprocal(l))
    } else {
        fallback
    }
}
pub(super) fn plane_segment(p: Vector, n: Vector, a: Vector, b: Vector) -> Option<Vector> {
    let da = dot(n, sub(a, p));
    let db = dot(n, sub(b, p));
    if da * db >= 0. {
        return None;
    }
    let aa = da.abs();
    let bb = db.abs();
    let sum = aa + bb;
    Some(madd(a, bb / sum, scale(b, aa / sum)))
}
pub(super) fn clamp_normal(a: Vector, b: Vector, n: Vector) -> Vector {
    let a = normalize(a, ZERO);
    let b = normalize(b, ZERO);
    let axis = normalize(cross(a, b), ZERO);
    let projected = normalize(sub(n, scale(axis, dot(axis, n))), ZERO);
    if ((dot(axis, axis) * dot(projected, projected)) * dot(a, a)) * dot(b, b) < 0.1 {
        return n;
    }
    if dot(cross(a, projected), axis) > 0. && dot(cross(projected, b), axis) > 0. {
        projected
    } else if dot(projected, a) > dot(projected, b) {
        a
    } else {
        b
    }
}
pub(super) fn intersection(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2]) -> f32 {
    let u = [b[0] - a[0], b[1] - a[1]];
    let v = [d[0] - c[0], d[1] - c[1]];
    let denominator = v[1] * u[0] - v[0] * u[1];
    if denominator.abs() < 0.00001 {
        return 1e10;
    }
    (1. / denominator) * (v[0] * (a[1] - c[1]) - v[1] * (a[0] - c[0]))
}
pub(super) fn tangent(x: f32) -> f32 {
    if x == 0. {
        return 0.;
    }
    let c = |bits| f32::from_bits(bits);
    let turns = (x * c(0x3f22f983)).round_ties_even();
    let r = (-turns).mul_add(c(0x2e85a309), (-turns).mul_add(c(0x3fc90fdb), x));
    let q = r * r;
    // IDA's VMX operand display is A,B,C: vmaddfp computes A*C+B.
    // 8252DA20..38: denominator degree four, numerator correction degree two.
    let den = q.mul_add(
        q.mul_add(
            q.mul_add(q.mul_add(c(0x3505bba8), c(0xb9a37b25)), c(0x3cd23cf5)),
            c(0xbeeef582),
        ),
        1.,
    );
    let poly = q.mul_add(q.mul_add(c(0xb795d5b9), c(0x3b607415)), c(0xbe0895af));
    let num = r.mul_add(q * poly, r);
    let (num, den) = if r.abs() <= c(0x39800000) {
        (r, 1.)
    } else {
        (num, den)
    };
    if (turns.abs() as i32) & 1 == 0 {
        num * reciprocal(den)
    } else {
        den * reciprocal(-num)
    }
}
