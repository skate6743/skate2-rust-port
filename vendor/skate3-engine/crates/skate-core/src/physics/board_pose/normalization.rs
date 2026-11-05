use super::{PoseMatrix, arithmetic::*};

/// 825C5710: z/y/x projection against ORIGINAL input axes, then normalization.
/// Translation is copied verbatim. Degenerate inputs receive no invented repair.
pub fn orthonormalize_rotation(input: PoseMatrix) -> PoseMatrix {
    let original = matrix(input);
    let mut out = original;
    for axis in (0..3).rev() {
        let mut residual = original[axis];
        for other in (axis + 1..3).rev() {
            residual = sub(
                residual,
                mul(out[other], [dot(out[other], original[axis], false); 4]),
            );
        }
        out[axis] = normalize(residual, false);
    }
    words(out)
}

/// 82AE0A40: native magnitude gates and least-parallel pair selection followed
/// by two normalized cross products. This differs from 825C5710 above.
pub fn orthonormalize_part_basis(input: PoseMatrix) -> PoseMatrix {
    let mut m = matrix(input);
    let squares: [f32; 3] = core::array::from_fn(|i| dot(m[i], m[i], false));
    let magnitudes = squares.map(|v| {
        let root = v * rsqrt(v);
        if v == 0.0 { 0.0 } else { root }
    });
    for axis in 0..3 {
        m[axis] = mul(m[axis], [rsqrt(squares[axis]); 4]);
    }
    let (u, v, w) = if !(magnitudes[0] > 0.0) {
        (1, 2, 0)
    } else if !(magnitudes[1] > 0.0) {
        (2, 0, 1)
    } else if !(magnitudes[2] > 0.0) {
        (0, 1, 2)
    } else {
        let ca = dot(m[2], m[0], false).abs();
        let bc = dot(m[1], m[2], false).abs();
        let ab = dot(m[0], m[1], false).abs();
        if ca > bc {
            if ab > bc { (1, 2, 0) } else { (0, 1, 2) }
        } else if ab > ca {
            (2, 0, 1)
        } else {
            (0, 1, 2)
        }
    };
    m[w] = normalize(cross(m[u], m[v]), false);
    m[v] = normalize(cross(m[w], m[u]), false);
    words(m)
}
