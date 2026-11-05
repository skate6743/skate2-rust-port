use super::math_host::*;
use skate3_core::animation::output::NativeMatrix;
pub(crate) fn native_matrix(matrix: NativeMatrix) -> Mat4 {
    Mat4::from_cols(
        Vec3::from_array(matrix[0][..3].try_into().unwrap()).extend(0.0),
        Vec3::from_array(matrix[1][..3].try_into().unwrap()).extend(0.0),
        Vec3::from_array(matrix[2][..3].try_into().unwrap()).extend(0.0),
        Vec3::from_array(matrix[3][..3].try_into().unwrap()).extend(1.0),
    )
}