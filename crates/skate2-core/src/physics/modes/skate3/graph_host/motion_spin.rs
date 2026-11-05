//! BodySpin82BAB640, including its native five-state channel controller.
use super::motion_animation::MotionAnimation;
use skate3_core::{
    animation::{channel_playback::ChannelSettings, playback_parameters::ParameterInputs},
    point_graph::PointGraph,
};
use skate_data::collections::Collections;

const FRONT: &str = "IA_BODYSPIN_OLLIE_FS_0_N";
const BACK: &str = "IA_BODYSPIN_OLLIE_BS_0_N";

pub struct Settings {
    map: PointGraph<8>,
    blend_out: f32,
    blend_in: f32,
    maximum_acceleration: f32,
    maximum_delta: f32,
    final_height: f32,
    height_velocity: f32,
    on_deck_height: f32,
    override_x: f32,
    override_velocity_y: f32,
    landing_distance: PointGraph<8>,
}
impl Settings {
    pub fn load(data: &Collections) -> Result<Self, String> {
        //Global304 writer8289F9C0..F9EC:DCBADE97CA665643=body_spin.
        let float = |name| data.float("anim_motion", "body_spin", name);
        let w = data.words::<16>("anim_motion", "body_spin", "spin_map")?;
        let d = data.words::<16>("animation", "default", "LandingDistanceScalarVsNormalY")?;
        Ok(Self {
            map: PointGraph {
                x: std::array::from_fn(|i| f32::from_bits(w[i])),
                y: std::array::from_fn(|i| f32::from_bits(w[8 + i])),
            },
            blend_out: float("spin_influence_blendout")?,
            blend_in: float("spin_influence_blendin")?,
            maximum_acceleration: float("spin_clamp_influence_deltadelta")?,
            maximum_delta: float("spin_clamp_influence_delta")?,
            //Global336=anim_motion/inair_disttocog;328=body_tilt.
            final_height: data.float(
                "anim_motion",
                "inair_disttocog",
                "preland_final_disttocom",
            )?,
            height_velocity: data.float(
                "anim_motion",
                "inair_disttocog",
                "preland_disttocom_vel",
            )?,
            on_deck_height: data.float(
                "anim_motion",
                "inair_disttocog",
                "landingondeck_disttocog",
            )?,
            override_x: data.float("anim_motion", "body_tilt", "overide_preland_x")?,
            override_velocity_y: data.float("anim_motion", "body_tilt", "overide_preland_velY")?,
            landing_distance: PointGraph {
                x: std::array::from_fn(|i| f32::from_bits(d[i])),
                y: std::array::from_fn(|i| f32::from_bits(d[8 + i])),
            },
        })
    }
}

///Actual publication fields used only by the native airborne prelanding query.
#[derive(Clone, Copy, Debug)]
pub struct PrelandingPhysical {
    pub air_444: bool,
    pub air_normal_144_y: f32,
    pub animation_16_x: f32,
    pub com_velocity_y: f32,
    pub offboard_316: bool,
    pub offboard_319: bool,
    pub offboard_time_32: f32,
    pub air_437: bool,
    pub air_normal_36: f32,
    pub air_remaining_184: f32,
    pub animation_height_72: f32,
}
impl PrelandingPhysical {
    ///825954C0 ->82595670, original source guards and strict comparisons.
    pub(crate) fn override_prelanding(self, s: &Settings) -> bool {
        self.air_444
            || (self.air_normal_144_y <= 0.5
                && self.animation_16_x.abs() > s.override_x
                && self.com_velocity_y > s.override_velocity_y)
    }
    pub(crate) fn near_landing(self, s: &Settings) -> bool {
        if self.override_prelanding(s) || self.com_velocity_y >= 0.0 {
            return false;
        }
        if self.offboard_316 {
            return self.offboard_time_32 < (s.final_height - s.on_deck_height) / s.height_velocity;
        }
        if self.air_437 {
            let mut time = (s.final_height - self.animation_height_72) / s.height_velocity;
            time *= s
                .landing_distance
                .evaluate(clamp(self.air_normal_36, 0.0, 1.0));
            return self.air_remaining_184 < time;
        }
        self.offboard_319 && self.offboard_time_32 < f32::from_bits(0x3f99999a)
    }
}

///82BABE20 initializes every represented scalar and the state enum to zero.
#[derive(Default)]
pub struct State {
    mode: u32,
    previous_spin: f32,
    back: f32,
    back_delta: f32,
    front: f32,
    front_delta: f32,
}
impl State {
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        animation: &mut MotionAnimation,
        category: u32,
        physical_state: u32,
        doing_trick: bool,
        busy_hands: [u32; 2],
        mirrored: bool,
        prelanding: Option<PrelandingPhysical>,
        settings: &Settings,
    ) -> Result<(), String> {
        let biped_air = physical_state == 503;
        let air = category == 2 || biped_air;
        let active = air || doing_trick;
        let busy = busy_hands[0] != 0 || busy_hands[1] != 0;
        let mut spin = animation.motion_intent("BodySpin").unwrap_or(0.0);
        if mirrored {
            spin = -spin;
        }
        let mapped = settings.map.evaluate(spin.abs());
        let influence = if spin >= 0.0 { mapped } else { -mapped };
        let near = if air {
            prelanding
                .ok_or("BodySpin airborne branch requires actual prelanding physical outputs")?
                .near_landing(settings)
        } else {
            false
        };
        let mut mode = self.mode;
        match mode {
            0 => {
                if category == 3 {
                    mode = 4;
                }
                if active {
                    mode = if busy { 4 } else { 1 };
                }
            }
            1 => {
                if !active {
                    mode = 0;
                }
                if busy {
                    mode = 4;
                }
                if spin.abs() > f32::from_bits(0x3e99999a) {
                    mode = 2;
                }
            }
            2 => {
                if !active {
                    mode = 0;
                }
                if busy {
                    mode = 4;
                }
                if air {
                    mode = 3;
                }
            }
            3 => {
                if busy || (near && biped_air) {
                    mode = 4;
                }
                if !air {
                    mode = 0;
                }
            }
            4 => {
                if !active && category == 1 {
                    mode = 0;
                }
            }
            _ => {}
        }
        match mode {
            0 | 4 => {
                for name in [FRONT, BACK] {
                    animation
                        .channels
                        .end_with(name, f32::from_bits(0x3e4ccccd), false);
                }
            }
            1 => {
                self.front = 0.0;
                self.front_delta = 0.0;
                self.back = 0.0;
                self.back_delta = 0.0;
            }
            2 | 3 => {
                if mode == 2 {
                    let channel = ChannelSettings {
                        priority: 0,
                        keep_alive: false,
                        mirrored: false,
                        speed: 1.0,
                        blend_in: f32::from_bits(0x3e4ccccd),
                        hold_during_blend_in: false,
                        blend_out: f32::from_bits(0x3e99999a),
                        hold_during_blend_out: false,
                        use_attributes: false,
                    };
                    if spin > f32::from_bits(0x3e99999a) {
                        if !animation.channels.has(FRONT) && !near {
                            animation.new_channel(FRONT, FRONT, channel)?;
                        }
                        animation
                            .channels
                            .end_with(BACK, f32::from_bits(0x3e19999a), false);
                    }
                    if spin < f32::from_bits(0xbe99999a) {
                        if !animation.channels.has(BACK) && !near {
                            animation.new_channel(BACK, BACK, channel)?;
                        }
                        animation
                            .channels
                            .end_with(FRONT, f32::from_bits(0x3e19999a), false);
                    }
                }
                if animation.channels.has(FRONT) {
                    let target = clamp(influence, 0.0, 1.0);
                    let blend = if target < self.front {
                        settings.blend_out
                    } else {
                        settings.blend_in
                    };
                    update_influence(
                        &mut self.front,
                        &mut self.front_delta,
                        target,
                        blend,
                        settings,
                    );
                    animation.channels.influence(FRONT, self.front);
                }
                if animation.channels.has(BACK) {
                    let target = -clamp(influence, -1.0, 0.0);
                    //The original backside branch intentionally reverses this comparison.
                    let blend = if target > self.back {
                        settings.blend_out
                    } else {
                        settings.blend_in
                    };
                    update_influence(
                        &mut self.back,
                        &mut self.back_delta,
                        target,
                        blend,
                        settings,
                    );
                    animation.channels.influence(BACK, self.back);
                }
            }
            _ => {}
        }
        self.mode = mode;
        self.previous_spin = spin;
        Ok(())
    }
}
fn update_influence(value: &mut f32, delta: &mut f32, target: f32, blend: f32, s: &Settings) {
    //82BABC68..BD9C: first clamp acceleration, then change, then [0,1].
    let desired = blend.mul_add(target, (1.0 - blend) * *value) - *value;
    *delta = clamp(
        desired,
        *delta - s.maximum_acceleration,
        *delta + s.maximum_acceleration,
    );
    let next = clamp(
        *value + *delta,
        *value - s.maximum_delta,
        *value + s.maximum_delta,
    );
    *value = clamp(next, 0.0, 1.0);
}
fn clamp(v: f32, low: f32, high: f32) -> f32 {
    let v = if low - v >= 0.0 { low } else { v };
    if high - v >= 0.0 { v } else { high }
}
