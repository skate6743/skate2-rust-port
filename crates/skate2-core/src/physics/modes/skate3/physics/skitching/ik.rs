//! UpdateHandIK82d9dc58. Uses existing Skeleton hand-target services at the caller.
use super::{
    Matrix, Skitching, Vector, read_floats,
    skitch_geometry::compose,
    skitch_state::{ENTERING, HAND_OFF, HAND_ON, SkitchError, clamp, need},
};
use crate::{
    physical_player_grab_spline::geometry::transform_point,
    physical_player_grind_manager::math::{GrindNum, add, madd, sub},
};

pub(crate) struct SkitchIkInput {
    pub(crate) anim_to_world: Matrix,
    pub(crate) left_hand: Vector,
    pub(crate) right_hand: Vector,
    pub(crate) left_arm: Vector,
    pub(crate) right_arm: Vector,
    pub(crate) world_grab_y: f32,
    pub(crate) world_grab_z: f32,
    pub(crate) mirrored: bool,
    pub(crate) authored: bool,
    pub(crate) force_left: bool,
    pub(crate) force_right: bool,
    pub(crate) crouch_spring: f32,
}
pub(crate) struct SkitchHandTarget {
    pub(crate) position: Vector,
    pub(crate) maximum: f32,
    pub(crate) blend: f32,
}
impl Skitching {
    pub(crate) fn update_hand_ik(
        &mut self,
        input: SkitchIkInput,
        n: &mut GrindNum<'_>,
    ) -> Result<[Option<SkitchHandTarget>; 2], SkitchError> {
        let left_min = self.attr(0xe2dc_234c_a3a5_88e9)?;
        let right_min = self.attr(0xc6c3_d74a_4a96_4b3d)?;
        let left_max = self.attr(0x11b7_a22c_ee19_8ed8)?;
        let right_max = self.attr(0xc505_25fe_8e95_d076)?;
        let degrees = need(self.deck_angle, "deck angle")? * f32::from_bits(0x4265_2ee1);
        let maximum = self.attr(0x2c75_33be_2cfa_d9b0)?;
        let release_length = maximum + self.attr(0xeff7_30f7_546c_6422)?;
        if need(self.skitch_state, "skitch state")? == ENTERING {
            self.left_hand_state = Some(HAND_ON);
            self.right_hand_state = Some(HAND_ON);
        }
        if degrees > left_max || degrees < left_min {
            self.left_hand_state = Some(HAND_OFF);
        }
        if degrees > right_max || degrees < right_min {
            self.right_hand_state = Some(HAND_OFF);
        }
        if input.force_left {
            self.left_hand_state = Some(HAND_ON);
            self.right_hand_state = Some(HAND_OFF);
        }
        if input.force_right {
            self.left_hand_state = Some(HAND_OFF);
            self.right_hand_state = Some(HAND_ON);
        }
        let hook = [0.0, input.world_grab_y, input.world_grab_z, 0.0];
        let hands_distance = need(self.grab_spline_hands_dist, "hands distance")?;
        let physical = self.space(self.grab_spline_physical_hand_space)?;
        let to_ik = need(self.grab_spline_to_ik_hand_space_transform, "spline to IK")?;
        let centre = if input.authored {
            let point = self
                .grab_spline
                .as_ref()
                .ok_or(SkitchError::Missing("grab spline"))?
                .position_at_straight_length(hands_distance, n)?;
            transform_point(&to_ik, point)
        } else {
            madd(physical[0], hands_distance, physical[3])
        };
        let hook_world = transform_point(&input.anim_to_world, hook);
        self.anim_crouch = Some(clamp(
            need(self.anim_crouch, "crouch")? + (hook_world[1] - centre[1]) * input.crouch_spring,
            0.0,
            0.8,
        ));
        let offset = read_floats::<4>(
            need(self.attribs.as_ref(), "skitch attributes")?,
            0x858d_6102_90de_4f96,
        )
        .map_err(SkitchError::Attribute)?;
        let anchor = add(hook, offset);
        let mirror = |mut value: Vector| {
            if input.mirrored {
                value[0] *= -1.0;
                value[2] *= -1.0;
            }
            value
        };
        let lh = mirror(input.left_hand);
        let rh = mirror(input.right_hand);
        let la = mirror(input.left_arm);
        let ra = mirror(input.right_arm);
        let left_delta = sub(lh, anchor);
        let right_delta = sub(rh, anchor);
        let (left, right) = if input.authored {
            let half = need(self.grab_spline_half_length_mag, "half length")?;
            let l = clamp(left_delta[0] + hands_distance, -half, half);
            let r = clamp(right_delta[0] + hands_distance, -half, half);
            let spline = self
                .grab_spline
                .as_ref()
                .ok_or(SkitchError::Missing("grab spline"))?;
            (
                transform_point(&to_ik, spline.position_at_straight_length(l, n)?),
                transform_point(&to_ik, spline.position_at_straight_length(r, n)?),
            )
        } else {
            let mut hand_frame = physical;
            hand_frame[3] = centre;
            (
                transform_point(&to_ik, transform_point(&hand_frame, left_delta)),
                transform_point(&to_ik, transform_point(&hand_frame, right_delta)),
            )
        };
        let anim_to_ik = compose(
            need(
                self.grab_spline_anim_space_to_ik_hand_space_transform,
                "anim to IK",
            )?,
            input.anim_to_world,
        );
        let left_length = n.length(sub(transform_point(&anim_to_ik, la), left))?;
        let right_length = n.length(sub(transform_point(&anim_to_ik, ra), right))?;
        self.predicted_left_hand_extension = Some(left_length);
        self.predicted_right_hand_extension = Some(right_length);
        if left_length > release_length {
            self.left_hand_state = Some(HAND_OFF);
        }
        if right_length > release_length {
            self.right_hand_state = Some(HAND_OFF);
        }
        if self.left_hand_state != self.last_left_hand_state {
            self.last_left_hand_state = self.left_hand_state;
            self.left_blend_target = Some(if self.left_hand_state == Some(HAND_OFF) {
                0.0
            } else {
                1.0
            });
        }
        if self.right_hand_state != self.last_right_hand_state {
            self.last_right_hand_state = self.right_hand_state;
            self.right_blend_target = Some(if self.right_hand_state == Some(HAND_OFF) {
                0.0
            } else {
                1.0
            });
        }
        let old_left = need(self.left_blend, "left blend")?;
        let old_right = need(self.right_blend, "right blend")?;
        let left_blend = clamp(
            need(self.left_blend_target, "left blend target")?,
            old_left - 0.2,
            old_left + 0.03,
        );
        let right_blend = clamp(
            need(self.right_blend_target, "right blend target")?,
            old_right - 0.2,
            old_right + 0.03,
        );
        self.left_blend = Some(left_blend);
        self.right_blend = Some(right_blend);
        self.grip_hand = Some(0);
        let mut targets = [None, None];
        if left_blend > 0.0 {
            self.grab_spline_lh_pos = Some(left);
            self.grip_hand = Some(1);
            targets[0] = Some(SkitchHandTarget {
                position: left,
                maximum,
                blend: left_blend,
            });
        }
        if right_blend > 0.0 {
            self.grab_spline_rh_pos = Some(right);
            self.grip_hand = Some(need(self.grip_hand, "grip hand")? | 2);
            targets[1] = Some(SkitchHandTarget {
                position: right,
                maximum,
                blend: right_blend,
            });
        }
        Ok(targets)
    }
}
