//! Skeleton::UpdateOffboardAdjust82BD8F10 and SkeletonIK82BF1A78.
//! Both transforms are current animation globals, before physical bone offsets.
use crate::physics::skeleton_animation_record::AnimationPartTransform as Transform;
/// Flags2476 bit2 selects left when set, right when clear.
pub fn selected_hand(flags_2476: u32) -> usize {
    usize::from(flags_2476 & 4 == 0)
}
/// Actual hand composed with the rigid inverse of the board-parented hand.
/// Native inverse is a transpose, not a general scale/shear matrix inverse.
pub fn adjustment(actual: &Transform, reparented: &Transform) -> Transform {
    let inverse: [[f32; 4]; 3] = std::array::from_fn(|column| {
        [
            reparented[0][column],
            reparented[1][column],
            reparented[2][column],
            0.,
        ]
    });
    //82BF1B18/40/50: inverse translation accumulates Z, then Y, then X.
    let translation: [f32; 4] = std::array::from_fn(|lane| {
        let z = (0. - reparented[3][2]) * inverse[2][lane];
        let yz = (0. - reparented[3][1]).mul_add(inverse[1][lane], z);
        (0. - reparented[3][0]).mul_add(inverse[0][lane], yz)
    });
    std::array::from_fn(|column| {
        std::array::from_fn(|lane| {
            if column < 3 {
                let x = inverse[column][0] * actual[0][lane];
                let y = inverse[column][1].mul_add(actual[1][lane], x);
                inverse[column][2].mul_add(actual[2][lane], y)
            } else {
                let x = translation[0].mul_add(actual[0][lane], actual[3][lane]);
                let y = translation[1].mul_add(actual[1][lane], x);
                translation[2].mul_add(actual[2][lane], y)
            }
        })
    })
}
#[cfg(test)]
mod tests;
