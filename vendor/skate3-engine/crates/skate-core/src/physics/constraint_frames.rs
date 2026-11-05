//! Original TU3 constraint-frame arithmetic shared by joint and drive builders.
//! 82AE0DB0 constructs quaternion-component rows directly from bilinear products.
//! The builders compose frames and construct bases without normalizing inputs.
use super::{
    native_arithmetic,
    rigid_body::{RetailPackedWorldInverseInertia, RetailQuaternion},
};
use crate::math::{Basis3, Vector3};

#[derive(Clone, Copy, Debug)]
pub(crate) struct QuaternionRows {
    pub axes: [Vector3; 3],
    pub relative: RetailQuaternion,
}

/// 82AE0DB0..0F88. Keep each rounded product and the original addition order;
/// rotating a relative quaternion's axes is only algebraically equivalent for
/// exact unit quaternions and has different finite-precision behavior.
pub(crate) fn quaternion_rows(a: RetailQuaternion, b: RetailQuaternion) -> QuaternionRows {
    let xx = a.x * b.x;
    let xy = a.x * b.y;
    let xz = a.x * b.z;
    let xw = a.x * b.w;
    let yx = a.y * b.x;
    let yy = a.y * b.y;
    let yz = a.y * b.z;
    let yw = a.y * b.w;
    let zx = a.z * b.x;
    let zy = a.z * b.y;
    let zz = a.z * b.z;
    let zw = a.z * b.w;
    let wx = a.w * b.x;
    let wy = a.w * b.y;
    let wz = a.w * b.z;
    let ww = a.w * b.w;
    QuaternionRows {
        axes: [
            Vector3::new(
                ((ww + xx) - yy) - zz,
                ((yx + xy) + wz) + zw,
                ((xz + zx) - wy) - yw,
            ),
            Vector3::new(
                ((yx + xy) - wz) - zw,
                ((ww + yy) - zz) - xx,
                ((yz + zy) + wx) + xw,
            ),
            Vector3::new(
                ((xz + zx) + yw) + wy,
                ((zy + yz) - wx) - xw,
                ((ww + zz) - xx) - yy,
            ),
        ],
        relative: RetailQuaternion {
            x: ((wx - xw) + zy) - yz,
            y: ((wy - yw) + xz) - zx,
            z: ((wz - zw) + yx) - xy,
            w: ((ww + zz) + yy) + xx,
        },
    }
}

/// Original quaternion multiplication at 82AE3C10..3C60 / 82AE1B94..1C4C.
pub(crate) fn compose(a: RetailQuaternion, b: RetailQuaternion) -> RetailQuaternion {
    let cross = cross(Vector3::new(a.x, a.y, a.z), Vector3::new(b.x, b.y, b.z));
    RetailQuaternion {
        x: a.x.mul_add(b.w, b.x.mul_add(a.w, cross.x)),
        y: a.y.mul_add(b.w, b.y.mul_add(a.w, cross.y)),
        z: a.z.mul_add(b.w, b.z.mul_add(a.w, cross.z)),
        w: a.w * b.w - native_arithmetic::dot3([a.x, a.y, a.z, 0.0], [b.x, b.y, b.z, 0.0]),
    }
}

/// Independently verified at Drive Build 82AE1CC4..1D90: sqrt(2) scaling,
/// 0.5 - component squared, paired residuals, and masks 822FB8A0/B0/C0.
/// This does not depend on Joint Build's unresolved RQD-gather shuffle at
/// 82AE3D98; it accepts an already known quaternion.
pub(crate) fn basis(q: RetailQuaternion) -> Basis3 {
    let root_two = f32::from_bits(0x3fb5_04f3);
    let x = q.x * root_two;
    let y = q.y * root_two;
    let z = q.z * root_two;
    let w = q.w * root_two;
    let dx = (-x).mul_add(x, 0.5);
    let dy = (-y).mul_add(y, 0.5);
    let dz = (-z).mul_add(z, 0.5);
    let xy = x * y;
    let yz = y * z;
    let zx = z * x;
    let wz = w * z;
    let wx = w * x;
    let wy = w * y;
    Basis3 {
        columns: [
            [dy + dz, xy + wz, zx - wy],
            [xy - wz, dz + dx, yz + wx],
            [zx + wy, yz - wx, dx + dy],
        ],
    }
}

pub(crate) fn transform_direction(b: Basis3, v: Vector3) -> Vector3 {
    let c = b.columns;
    Vector3::new(
        c[2][0].mul_add(v.z, c[1][0].mul_add(v.y, c[0][0] * v.x)),
        c[2][1].mul_add(v.z, c[1][1].mul_add(v.y, c[0][1] * v.x)),
        c[2][2].mul_add(v.z, c[1][2].mul_add(v.y, c[0][2] * v.x)),
    )
}

pub(crate) fn cross(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(
        (-a.z).mul_add(b.y, a.y * b.z),
        (-a.x).mul_add(b.z, a.z * b.x),
        (-a.y).mul_add(b.x, a.x * b.y),
    )
}

/// Source adds both cross-product terms directly into the incoming rate.
pub(crate) fn point_rate(linear: Vector3, angular: Vector3, arm: Vector3) -> Vector3 {
    Vector3::new(
        arm.z
            .mul_add(angular.y, (-arm.y).mul_add(angular.z, linear.x)),
        arm.x
            .mul_add(angular.z, (-arm.z).mul_add(angular.x, linear.y)),
        arm.y
            .mul_add(angular.x, (-arm.x).mul_add(angular.y, linear.z)),
    )
}

pub(crate) fn multiply_add(v: Vector3, scalar: f32, addend: Vector3) -> Vector3 {
    Vector3::new(
        v.x.mul_add(scalar, addend.x),
        v.y.mul_add(scalar, addend.y),
        v.z.mul_add(scalar, addend.z),
    )
}

pub(crate) fn reciprocal(value: f32) -> f32 {
    let mut r = native_arithmetic::reciprocal_estimate(value);
    for _ in 0..2 {
        let residual = (-r).mul_add(value, 1.0);
        r = r.mul_add(residual, r);
    }
    r
}

pub(crate) fn reciprocal_sqrt(value: f32) -> f32 {
    let mut r = native_arithmetic::reciprocal_square_root_estimate(value);
    for _ in 0..2 {
        let squared = r * r;
        let half = r * 0.5;
        let residual = (-value).mul_add(squared, 1.0);
        r = half.mul_add(residual, r);
    }
    r
}

/// Drive 82AE2430..2490: X product, then the Y and Z fused additions.
pub(crate) fn multiply_inertia(t: RetailPackedWorldInverseInertia, v: Vector3) -> Vector3 {
    inertia_tail(
        t,
        v,
        Vector3::new(t.full.x * v.x, t.full.y * v.x, t.full.z * v.x),
    )
}

/// Joint 82AE470C..4758 starts the same contraction with a fused +zero.
pub(crate) fn multiply_inertia_from_zero(
    t: RetailPackedWorldInverseInertia,
    v: Vector3,
) -> Vector3 {
    inertia_tail(
        t,
        v,
        Vector3::new(
            t.full.x.mul_add(v.x, 0.0),
            t.full.y.mul_add(v.x, 0.0),
            t.full.z.mul_add(v.x, 0.0),
        ),
    )
}

fn inertia_tail(t: RetailPackedWorldInverseInertia, v: Vector3, first: Vector3) -> Vector3 {
    Vector3::new(
        t.full.z.mul_add(v.z, t.full.y.mul_add(v.y, first.x)),
        t.split.z.mul_add(v.z, t.split.y.mul_add(v.y, first.y)),
        t.split.x.mul_add(v.z, t.split.z.mul_add(v.y, first.z)),
    )
}
