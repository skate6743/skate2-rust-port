//! TU3 inertia diagonalization: 82AE8388 and its three rotation helpers.
//! Rows here are mathematical rows. The caller transposes the resulting axes
//! into RenderWare's Ri/Up/At storage. No eigenvalue sorting is performed.

use super::refined_reciprocal;

pub(super) type Matrix3 = [[f32; 3]; 3];

pub(super) fn diagonalize(inertia: &mut Matrix3) -> Matrix3 {
    let mut axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mut remaining = 10;
    loop {
        // 82AE84A4..8680: squared lower-triangle residual, relative to the
        // smallest squared diagonal. Preserve comparison and FMA ordering.
        let diagonal = [inertia[0][0], inertia[1][1], inertia[2][2]].map(|x| x * x);
        let smallest = if diagonal[1] > diagonal[0] {
            if diagonal[2] > diagonal[0] {
                diagonal[0]
            } else {
                diagonal[2]
            }
        } else if diagonal[2] > diagonal[1] {
            diagonal[1]
        } else {
            diagonal[2]
        };
        let residual = inertia[2][1].mul_add(
            inertia[2][1],
            inertia[1][0].mul_add(inertia[1][0], inertia[2][0] * inertia[2][0]),
        );
        if !(residual > smallest * 1.0e-24) || remaining == 0 {
            break;
        }
        for row in 1..3 {
            for column in 0..row {
                let (sine, cosine) = rotation(inertia, row, column);
                rotate_columns(&mut axes, row, column, sine, cosine);
                rotate_columns(inertia, row, column, sine, cosine);
                rotate_rows(inertia, row, column, sine, cosine);
            }
        }
        remaining -= 1;
    }
    axes
}

/// 82AE7AD8. The zero off-diagonal path is exactly the identity rotation.
fn rotation(matrix: &Matrix3, row: usize, column: usize) -> (f32, f32) {
    let off_diagonal = matrix[row][column];
    if off_diagonal == 0.0 {
        return (0.0, 1.0);
    }
    let difference = matrix[column][column] - matrix[row][row];
    let theta = refined_reciprocal(off_diagonal) * (difference * 0.5);
    let length = theta.mul_add(theta, 1.0).sqrt();
    let tangent = if theta > 0.0 {
        1.0 / (length + theta)
    } else {
        -1.0 / (length - theta)
    };
    let cosine = 1.0 / tangent.mul_add(tangent, 1.0).sqrt();
    (cosine * tangent, cosine)
}

/// 82AE7C30: independently rounded products for the subtraction and one FMA
/// for the addition. Combining the subtraction into an FMA changes the port.
fn rotate_columns(matrix: &mut Matrix3, a: usize, b: usize, sine: f32, cosine: f32) {
    for row in matrix {
        let left = row[a];
        let right = row[b];
        row[a] = left * cosine - right * sine;
        row[b] = left.mul_add(sine, right * cosine);
    }
}

/// 82AE7FD0 applies the matching rotation across the two matrix rows.
fn rotate_rows(matrix: &mut Matrix3, a: usize, b: usize, sine: f32, cosine: f32) {
    let left = matrix[a];
    let right = matrix[b];
    for column in 0..3 {
        matrix[a][column] = left[column] * cosine - right[column] * sine;
        matrix[b][column] = left[column].mul_add(sine, right[column] * cosine);
    }
}
