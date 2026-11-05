//! Native camera Euler conversions. These preserve the products and fused sums
//! from TU3 82E06CB8 and 82E045E0 rather than passing through renderer math.
use crate::{math::Basis3, trigonometry::sin_cos};

pub(super) fn quaternion_from_angles(pitch: f32, yaw: f32, roll: f32) -> [f32; 4] {
    let (sp, cp) = sin_cos(pitch * 0.5);
    let (sy, cy) = sin_cos(yaw * 0.5);
    let (sr, cr) = sin_cos(roll * 0.5);
    let cp_cr = cp * cr;
    let sp_cr = sp * cr;
    let sp_sr = sp * sr;
    let cp_sr = cp * sr;
    [
        cy * sp_cr - sy * cp_sr,
        cy.mul_add(sp_sr, sy * cp_cr),
        cy * cp_sr - sy * sp_cr,
        cy.mul_add(cp_cr, sy * sp_sr),
    ]
}

/// TU3 82E04AD4..82E04BC4. The native matrix's three spatial columns are
/// retained; its unused fourth lanes are not part of the host Basis3 type.
pub(super) fn basis_from_angles(pitch: f32, yaw: f32, roll: f32) -> Basis3 {
    let (sp, cp) = sin_cos(pitch);
    let (sy, cy) = sin_cos(yaw);
    let (sr, cr) = sin_cos(roll);
    let cp_sr = cp * sr;
    let cp_cr = cp * cr;
    let sp_sr = sp * sr;
    let sp_cr = sp * cr;
    Basis3 {
        columns: [
            [cy * cr, cy * sr, -sy],
            [sy * sp_cr - cp_sr, sy.mul_add(sp_sr, cp_cr), cy * sp],
            [sy.mul_add(cp_cr, sp_sr), sy * cp_sr - sp_cr, cy * cp],
        ],
    }
}

/// Complete axis rotation82BD35B8. Standalone sine/cosine and the matrix
/// product's X,Y,Z accumulation order are observable in the camera output.
pub(super) fn rotate_about_axis(basis: Basis3, axis: [f32; 3], angle: f32) -> Basis3 {
    let cosine = crate::trigonometry::cos(angle);
    let sine = crate::trigonometry::sin(angle);
    let [x, y, z] = axis;
    let sx = sine * x;
    let sy = sine * y;
    let sz = sine * z;
    let one_minus_cosine = 1.0 - cosine;
    let tx = one_minus_cosine * x;
    let ty = one_minus_cosine * y;
    let tz = one_minus_cosine * z;
    let rotation = [
        [x.mul_add(tx, cosine), tx.mul_add(y, sz), tx * z - sy],
        [ty * x - sz, ty.mul_add(y, cosine), ty.mul_add(z, sx)],
        [tz.mul_add(x, sy), tz * y - sx, z.mul_add(tz, cosine)],
    ];
    Basis3 {
        columns: basis.columns.map(|column| {
            core::array::from_fn(|i| {
                rotation[2][i].mul_add(
                    column[2],
                    rotation[1][i].mul_add(column[1], rotation[0][i] * column[0]),
                )
            })
        }),
    }
}

/// Look basis8296EF20: normalize world-up cross At, then normalize At cross
/// Right. The caller has already handled a zero direction and its pitch cap.
pub(super) fn look_basis(at: [f32; 3]) -> Basis3 {
    let right = normalize(cross([0.0, 1.0, 0.0], at));
    let up = normalize(cross(at, right));
    Basis3 {
        columns: [right, up, at],
    }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
    ]
}

pub(super) fn normalize<const N: usize>(v: [f32; N]) -> [f32; N] {
    let square =
        crate::physics::native_arithmetic::dot3([v[0], v[1], v[2], 0.0], [v[0], v[1], v[2], 0.0]);
    let mut inverse = crate::physics::reciprocal_sqrt::estimate(square);
    for _ in 0..2 {
        inverse = (inverse * 0.5).mul_add((-square).mul_add(inverse * inverse, 1.0), inverse);
    }
    v.map(|lane| lane * inverse)
}
