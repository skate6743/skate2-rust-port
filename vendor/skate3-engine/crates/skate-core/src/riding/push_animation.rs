//! TU3 push animation calculations, upstream of the contact-driven push force.
//! `0x82BAD658` maps held time to strength; `0x82BACF50` updates the three
//! Andale parameters. These do not select an animation or synthesize root motion.
use crate::point_graph::PointGraph;

#[derive(Clone, Copy, Debug)]
pub struct PushAnimationCurves {
    /// anim_motion/pushing, collection +640 (x +656, y +688).
    pub button_time_max: PointGraph<8>,
    /// Collection +560 (x +576, y +608).
    pub button_time_to_dv: PointGraph<8>,
    /// Collection +720 (x +736, y +768).
    pub blend_speed_over_frames: PointGraph<8>,
    /// Collection +800 (x +816, y +848).
    pub blend_acc_over_frames: PointGraph<8>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PushBlendParameters {
    pub hstr_vel_b: f32,
    pub lstr_vel_b: f32,
    /// A negative current value is the native uninitialized sentinel.
    pub vel_e: f32,
}

impl PushAnimationCurves {
    pub fn strength(&self, held_seconds: f32, speed: f32) -> f32 {
        let duration = self.button_time_max.evaluate(speed);
        self.button_time_to_dv
            .evaluate(unit_interval(held_seconds / duration))
    }

    /// Returns the values written to shared properties +8/+12/+16 and then
    /// published as HSTR_VEL_B, LSTR_VEL_B, VEL_E, in that order.
    pub fn update_blend(
        &self,
        current: PushBlendParameters,
        target: PushBlendParameters,
        forward_speed: f32,
        delta_seconds: f32,
    ) -> PushBlendParameters {
        let current = if current.vel_e < 0.0 { target } else { current };
        // This factor is a native constant at 0x820849C8, not the graph dt.
        let frame_seconds = f32::from_bits(0x3c88_8889);
        let velocity_rate =
            1.0 / (self.blend_speed_over_frames.evaluate(current.vel_e) * frame_seconds);
        let speed = if forward_speed >= 0.0 {
            forward_speed
        } else {
            0.0
        };
        let strength_rate = 1.0 / (self.blend_acc_over_frames.evaluate(speed) * frame_seconds);
        PushBlendParameters {
            hstr_vel_b: approach(
                current.hstr_vel_b,
                target.hstr_vel_b,
                velocity_rate,
                delta_seconds,
            ),
            lstr_vel_b: approach(
                current.lstr_vel_b,
                target.lstr_vel_b,
                velocity_rate,
                delta_seconds,
            ),
            vel_e: approach(current.vel_e, target.vel_e, strength_rate, delta_seconds),
        }
    }
}

fn approach(current: f32, target: f32, rate: f32, dt: f32) -> f32 {
    let difference = target - current;
    let result = if difference.abs() > 0.001 {
        if difference > 0.0 {
            let candidate = dt.mul_add(rate, current);
            if candidate - target >= 0.0 {
                target
            } else {
                candidate
            }
        } else {
            // Native fnmsubs, followed by fsel; preserve the fused operation.
            let candidate = -dt.mul_add(rate, -current);
            if candidate - target >= 0.0 {
                candidate
            } else {
                target
            }
        }
    } else {
        target
    };
    unit_interval(result)
}

fn unit_interval(value: f32) -> f32 {
    // Native fsel sequence also canonicalizes negative zero to positive zero.
    let lower = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}
