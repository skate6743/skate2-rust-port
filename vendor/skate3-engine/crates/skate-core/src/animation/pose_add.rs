//! ACS immediate AddSQT828CCC08, used by SkaterAnim::AddBindPose82B98118.
use super::{
    output::Sqt,
    pose_trajectory::{multiply, rotate},
};

/// Native operand B supplies the reference frame. The designated motion
/// operand supplies the channel weight, independently of operand ordering.
pub fn add(a: Sqt, b: Sqt, motion_is_a: bool) -> Sqt {
    let rotated = rotate(
        b.rotation,
        [a.translation[0], a.translation[1], a.translation[2]],
    );
    Sqt {
        scale: core::array::from_fn(|i| b.scale[i] * a.scale[i]),
        rotation: multiply(b.rotation, a.rotation),
        translation: [
            rotated[0] + b.translation[0],
            rotated[1] + b.translation[1],
            rotated[2] + b.translation[2],
            if motion_is_a {
                a.translation[3]
            } else {
                b.translation[3]
            },
        ],
    }
}
