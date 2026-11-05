//! Ground-position observation from TU3 Skateboard::CalculateGroundPosition
//! 82C02840. The four wheel rigid-body centres are expressed in Reckoning752,
//! averaged laterally/longitudinally, and placed at the lowest wheel centre.
use skate3_core::physics::{
    board_runtime::BoardRuntime, skeleton_animation_record::AnimationPartTransform,
};

pub(crate) fn ground_position(board: &BoardRuntime, ground: &AnimationPartTransform) -> [u32; 4] {
    let wheels = std::array::from_fn(|i| {
        let p = board.bodies()[i].rates.position;
        [p.x, p.y, p.z, 0.0]
    });
    wheel_ground_position(wheels, ground).map(f32::to_bits)
}

fn wheel_ground_position(wheels: [[f32; 4]; 4], ground: &AnimationPartTransform) -> [f32; 4] {
    // Native transpose clears its fourth lane; the source does not subtract
    // or re-add the frame translation in this calculation.
    let transpose =
        std::array::from_fn(|axis| [ground[0][axis], ground[1][axis], ground[2][axis], 0.0]);
    let local = wheels.map(|position| rotate(&transpose, position));
    let mut mean =
        std::array::from_fn(|i| (((local[0][i] + local[1][i]) + local[2][i]) + local[3][i]) * 0.25);
    let first_minimum = if local[1][1] > local[0][1] {
        local[0][1]
    } else {
        local[1][1]
    };
    // fsel chooses its negative input on an unordered comparison as well.
    let next_minimum = if local[2][1] - first_minimum >= 0.0 {
        first_minimum
    } else {
        local[2][1]
    };
    mean[1] = if local[3][1] - next_minimum >= 0.0 {
        next_minimum
    } else {
        local[3][1]
    };
    rotate(&[ground[0], ground[1], ground[2]], mean)
}

fn rotate(basis: &[[f32; 4]; 3], point: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|i| {
        let x = basis[0][i] * point[0];
        let xy = basis[1][i].mul_add(point[1], x);
        basis[2][i].mul_add(point[2], xy)
    })
}

#[cfg(any())]
mod tests {
    use super::*;
    #[test]
    fn minimum_height_is_measured_in_the_ground_frame() {
        let ground = [
            [0.0, 1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [99.0; 4],
        ];
        let wheels = [
            [1.0, 2.0, 3.0, 0.0],
            [2.0, 4.0, 5.0, 0.0],
            [3.0, 6.0, 7.0, 0.0],
            [4.0, 8.0, 9.0, 0.0],
        ];
        assert_eq!(wheel_ground_position(wheels, &ground), [4.0, 5.0, 6.0, 0.0]);
    }
}
