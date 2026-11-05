//! Volume moments accumulated by TU3 82AE6CB8/82AE6D88/82AE6F88 and reduced
//! to a local principal frame by 82AE7508. This is the aggregate mass producer;
//! geometry construction and requested-mass scaling remain separate stages.

use super::{PrimitiveMass, principal_axes::diagonalize, refined_reciprocal};
use crate::math::{Basis3, Vector3};
use crate::physics::rigid_body::RetailLocalMassFrame;

/// Integrals of [x,y,z,1] outer products, in column order. The last column
/// holds first moments and volume. Keeping all lanes preserves native sums.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MassMoments {
    pub columns: [[f32; 4]; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AggregateMassProperties {
    pub volume: f32,
    pub local_mass_frame: RetailLocalMassFrame,
    pub moments_per_unit_mass: Vector3,
}

impl MassMoments {
    pub const ZERO: Self = Self {
        columns: [[0.0; 4]; 4],
    };

    /// 82AE6CB8 converts primitive principal inertia into volume moments.
    pub fn from_primitive(primitive: PrimitiveMass) -> Self {
        let inertia = primitive.moments_per_unit_mass;
        let volume = primitive.volume;
        let x = (((inertia.z + inertia.y) - inertia.x) * volume) * 0.5;
        let y = (((inertia.z + inertia.x) - inertia.y) * volume) * 0.5;
        let z = (((inertia.y + inertia.x) - inertia.z) * volume) * 0.5;
        Self {
            columns: [
                [x, 0.0, 0.0, 0.0],
                [0.0, y, 0.0, 0.0],
                [0.0, 0.0, z, 0.0],
                [0.0, 0.0, 0.0, volume],
            ],
        }
    }

    /// 82AE6D88: H * moments * transpose(H). The source explicitly sets the
    /// basis fourth lanes to zero and the translation fourth lane to one.
    pub fn transform(&mut self, basis: Basis3, translation: Vector3) {
        let mut affine = [[0.0; 4]; 4];
        for (column, axis) in affine[..3].iter_mut().zip(basis.columns) {
            column[..3].copy_from_slice(&axis);
        }
        affine[3] = [translation.x, translation.y, translation.z, 1.0];
        let left = self.columns.map(|column| product(affine, column));
        self.columns = core::array::from_fn(|column| {
            product(left, core::array::from_fn(|row| affine[row][column]))
        });
    }

    /// 82AE6F88 adds successful children in volume order, without weighting or
    /// normalizing them individually. Collision-enable flags are not read here.
    pub fn add(&mut self, child: Self) {
        for (output, value) in self.columns.iter_mut().zip(child.columns) {
            for (output, value) in output.iter_mut().zip(value) {
                *output += value;
            }
        }
    }

    /// 82AE7508 mutates the aggregate into its center-of-mass frame before
    /// diagonalizing its inertia. Preserve this mutation and native axis order.
    pub fn principal_properties(&mut self) -> AggregateMassProperties {
        let volume = self.columns[3][3];
        let inverse_volume = refined_reciprocal(volume);
        let center = Vector3::new(
            self.columns[3][0] * inverse_volume,
            self.columns[3][1] * inverse_volume,
            self.columns[3][2] * inverse_volume,
        );
        self.transform(
            RetailLocalMassFrame::IDENTITY.basis,
            Vector3::new(-center.x, -center.y, -center.z),
        );
        let m = self.columns;
        let xy = (m[1][0] + m[0][1]) * -0.5;
        let xz = (m[2][0] + m[0][2]) * -0.5;
        let yz = (m[2][1] + m[1][2]) * -0.5;
        let mut inertia = [
            [m[1][1] + m[2][2], xy, xz],
            [xy, m[0][0] + m[2][2], yz],
            [xz, yz, m[0][0] + m[1][1]],
        ];
        let axes = diagonalize(&mut inertia);
        let maximum_yz =
            select_nonnegative(inertia[1][1] - inertia[2][2], inertia[1][1], inertia[2][2]);
        let maximum = select_nonnegative(inertia[0][0] - maximum_yz, inertia[0][0], maximum_yz);
        let floor = maximum * 0.0050000004;
        let moments = core::array::from_fn::<_, 3, _>(|i| {
            select_nonnegative(inertia[i][i] - floor, inertia[i][i], floor) * inverse_volume
        });
        AggregateMassProperties {
            volume,
            local_mass_frame: RetailLocalMassFrame {
                basis: Basis3 {
                    columns: core::array::from_fn(|i| core::array::from_fn(|j| axes[j][i])),
                },
                translation: center,
            },
            moments_per_unit_mass: Vector3::new(moments[0], moments[1], moments[2]),
        }
    }
}

fn product(matrix: [[f32; 4]; 4], vector: [f32; 4]) -> [f32; 4] {
    core::array::from_fn(|i| {
        matrix[3][i].mul_add(
            vector[3],
            matrix[2][i].mul_add(
                vector[2],
                matrix[1][i].mul_add(vector[1], matrix[0][i] * vector[0]),
            ),
        )
    })
}

fn select_nonnegative(selector: f32, positive: f32, negative: f32) -> f32 {
    if selector >= 0.0 { positive } else { negative }
}
