//! TU3 ToggleBoard82BA8FD8/82BA8FE8, retrieval82BA9310/82BA9E20,
//! drop82BA9990. Instance and channel clocks remain owned by the graph.
use super::motion_animation::MotionAnimation;
use skate3_core::animation::{
    channel_playback::ChannelSettings, playback::TransitionSettings,
    playback_parameters::AttributeSink, skeleton_input::name::encode,
};
use skate3_core::graph::intents::IntentMap;
const CHANNEL: &str = "RetrieveBoard"; //82F87410
const FADE: f32 = f32::from_bits(0x3e2aaaab);
#[derive(Clone, Copy, Default)]
pub(crate) struct Physical {
    pub blocked: bool,
    pub held: bool,
    pub returning: bool,
    pub forbid_retrieve: bool,
    pub angle: f32,
    pub pitch: f32,
    pub mirrored: bool,
}
#[derive(Default)]
pub(crate) struct ToggleBoard {
    state: u32,
    angle: f32,
    pitch: f32,
    yaw: f32,
    flags: u8,
    back: bool,
}
impl ToggleBoard {
    fn reset(&mut self) {
        self.angle = 0.;
        self.pitch = 0.;
        self.yaw = 0.;
        self.flags &= 0x7f;
        self.back = false;
    }
    fn aim(&mut self, p: Physical) {
        let mut yaw = p.angle * f32::from_bits(0xc2652ee0);
        if p.mirrored {
            yaw = -yaw;
        }
        if yaw < -90. {
            yaw = if self.flags & 0x80 != 0 {
                if self.yaw >= 0. { 180. } else { -90. }
            } else if yaw < -135. {
                180.
            } else {
                -90.
            };
        }
        if p.mirrored {
            yaw = -yaw;
        }
        if self.flags & 0x80 != 0 {
            let delta = yaw - self.angle;
            let lower = if -5. - delta >= 0. { -5. } else { delta };
            let step = if 5. - lower >= 0. { lower } else { 5. };
            yaw = self.angle + step;
        }
        self.angle = yaw;
        self.yaw = if p.mirrored { -yaw } else { yaw };
        self.flags |= 0x80;
        self.pitch = p.pitch * f32::from_bits(0x42652ee0);
        self.back = self.yaw.abs() > 90.;
    }
    fn clip(&self, suffix: &str) -> String {
        format!(
            "OFB_RETRIEVE_HIGH_{}_{suffix}",
            if self.back { "BACK" } else { "FRONT" }
        )
    }
    fn channel(
        a: &mut MotionAnimation,
        clip: &str,
        hold: bool,
        attrs: bool,
        immediate: bool,
    ) -> Result<(), String> {
        let settings = ChannelSettings {
            priority: 0,
            keep_alive: false,
            mirrored: false,
            speed: 1.,
            blend_in: 0.25,
            hold_during_blend_in: hold,
            blend_out: 0.25,
            hold_during_blend_out: attrs,
            use_attributes: true,
        };
        a.transition_channel(
            CHANNEL,
            clip,
            settings,
            TransitionSettings {
                kind: 1,
                seconds: if immediate { 0. } else { 0.25 },
                under: 0,
                matching: 0,
                use_channels_from_weights: false,
            },
            true,
            true,
        )?;
        Ok(())
    }
    fn attribute(a: &mut MotionAnimation, name: &[u8], v: f32) {
        a.set_attribute(
            skate3_core::animation::playback_parameters::SettableAttribute {
                name: encode(name),
                value: v,
                normalized: false,
                sequence_id: -1,
            },
        );
    }
    fn request(a: &mut MotionAnimation, name: &str) {
        a.motion_intents.insert(name.into(), 1.);
        Self::attribute(a, name.as_bytes(), 1.);
    }
    pub(crate) fn execute(
        &mut self,
        phase: u8,
        p: Physical,
        intents: &IntentMap,
        a: &mut MotionAnimation,
    ) -> Result<(), String> {
        if phase == 0 {
            self.state = 0;
            self.reset();
            return Ok(());
        }
        if phase != 1 {
            return Ok(());
        }
        if !p.blocked && self.state == 0 {
            if intents.contains_key("OB_DropBoard") && p.held {
                self.state = 5;
                self.flags &= !0x40;
            } else if intents.contains_key("OB_ThrowBoard") && p.held {
                self.state = 5;
                self.flags |= 0x40;
            } else if intents.contains_key("OB_RetrieveBoard") && !p.held && !p.forbid_retrieve {
                self.state = 1;
            }
        }
        match self.state {
            0 => self.reset(),
            1..=4 => {
                Self::request(a, "OB_RetrievingBoard");
                match self.state {
                    1 => {
                        self.aim(p);
                        self.state = 2;
                        Self::channel(a, &self.clip("INTO"), false, false, false)?;
                    }
                    2 => {
                        self.aim(p);
                        if !a.channels.has(CHANNEL) || a.channels.remaining(CHANNEL) <= 0. {
                            self.state = 0;
                        } else if a.channels.remaining(CHANNEL) <= 0.25 {
                            self.state = 3;
                            Self::channel(a, &self.clip("CYC"), true, true, true)?;
                            Self::request(a, "OB_RetrieveBoard");
                        }
                    }
                    3 => {
                        self.aim(p);
                        if !p.returning {
                            self.state = 0;
                            a.channels.end_with(CHANNEL, FADE, false);
                        } else if !a.channels.has(CHANNEL) || a.channels.remaining(CHANNEL) <= 0. {
                            self.state = 0;
                        } else if p.held {
                            self.state = 4;
                            Self::channel(a, &self.clip("OUT"), true, true, false)?;
                        } else if a.channels.remaining(CHANNEL) <= 0.25 {
                            Self::channel(a, &self.clip("CYC"), true, true, true)?;
                            Self::request(a, "OB_RetrieveBoard");
                        }
                    }
                    4 => {
                        if !a.channels.has(CHANNEL)
                            || a.channels.remaining(CHANNEL) <= 0.
                            || a.channels.time(CHANNEL) > FADE
                        {
                            self.state = 0;
                            a.channels.end_with(CHANNEL, FADE, false);
                        }
                    }
                    _ => unreachable!(),
                }
                Self::attribute(a, b"yaw", self.yaw);
                Self::attribute(a, b"pitch", self.pitch);
            }
            5..=6 => {
                Self::request(a, "OB_DroppingBoard");
                if self.state == 5 {
                    self.state = 6;
                    Self::channel(
                        a,
                        if self.flags & 0x40 == 0 {
                            "OFB_THROW_90"
                        } else {
                            "OFB_THROW_0"
                        },
                        false,
                        false,
                        false,
                    )?;
                } else if !a.channels.has(CHANNEL)
                    || a.channels.remaining(CHANNEL) <= 0.
                    || a.channels.time(CHANNEL) > 0.5
                {
                    a.channels.end_with(CHANNEL, FADE, false);
                    self.state = 0;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
