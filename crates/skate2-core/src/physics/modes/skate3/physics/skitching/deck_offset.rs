//! UpdateDeckAnimOffset, Skate 3 0x82d4a0c0 (Skate 2 0x82d96430). At or below the threshold no
//! setter is called.
use super::{
    Matrix, Skitching,
    skitch_state::{SkitchError, clamp, need},
};
use crate::physical_player_grind_manager::math::{DEG_TO_RAD, GrindNum};
impl Skitching {
    pub(crate) fn deck_animation_offset(
        &mut self,
        blend: f32,
        max_delta: f32,
        min_angle: f32,
        push_speed: bool,
        n: &mut GrindNum<'_>,
    ) -> Result<Option<Matrix>, SkitchError> {
        let angle = need(self.deck_angle, "deck angle")?;
        let sign = if angle >= 0.0 { 1.0 } else { -1.0 };
        let target = self.graph(
            0x30fd_fce1_85cd_3d4f,
            (angle * sign) * f32::from_bits(0x4265_2ee1),
        )? * DEG_TO_RAD;
        // 0x82d4a1d4..f4: the PushSpeed record (Processed+0x9B8 0x00800000) aims at zero.
        let aim = if push_speed { 0.0 } else { target * sign };
        let old = need(self.anim_deck_angle, "animation deck angle")?;
        let maximum = max_delta * DEG_TO_RAD;
        let next = clamp((aim - old) * blend, -maximum, maximum) + old;
        self.anim_deck_angle = Some(next);
        // 0x82d4a2b4 ble: an unordered angle still sets the offset.
        if next.abs() <= min_angle * DEG_TO_RAD {
            return Ok(None);
        }
        let (s, c) = n.sin_cos(next)?;
        // 0x82d4a310..4c: the X and Z rows keep the splatted cos/sin in W.
        Ok(Some([
            [c, 0., -s, c],
            [0., 1., 0., 0.],
            [s, 0., c, s],
            [0.; 4],
        ]))
    }
}
