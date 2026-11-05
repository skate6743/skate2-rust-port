//!82D80CE0: published frame copy and retained positional correction decay.
use super::math::*;
use super::{GroundMotionState, Vector};
pub(super) fn update(s: &mut GroundMotionState, displacement: Vector) {
    s.published_frame_64 = s.frame_0;
    if !s.correction_enabled_711 {
        return;
    }
    let magnitude = length(s.correction_576);
    if magnitude <= f32::from_bits(0x3c23_d70a) {
        s.correction_576 = ZERO;
        s.correction_enabled_711 = false;
    } else {
        let unit = scale(s.correction_576, 1.0 / magnitude);
        let proportional = magnitude * f32::from_bits(0xbd4c_cccd);
        let minimum = f32::from_bits(0xbba3_d70a);
        let decay = select(minimum - proportional, proportional, minimum);
        let projected = dot(displacement, unit);
        let amount = select(projected - decay, decay, projected);
        if amount > -magnitude {
            s.correction_576 = madd(unit, amount, s.correction_576);
        } else {
            s.correction_576 = ZERO;
            s.correction_enabled_711 = false;
        }
    }
    s.published_frame_64[3] = sub(s.published_frame_64[3], s.correction_576);
}
