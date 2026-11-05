//! Persistent channel timing from TU3 NewChannel82D1CC20, AddTime82D266A8,
//! RemoveExpiredChannels82D1DA00 and EndChannelPrematurely82D1D6A8.
use super::{clip_clock::AdvanceResult, output::attributes::AnimationAttribute};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChannelSettings {
    pub priority: i32,
    pub keep_alive: bool,
    pub mirrored: bool,
    pub speed: f32,
    pub blend_in: f32,
    pub hold_during_blend_in: bool,
    pub blend_out: f32,
    pub hold_during_blend_out: bool,
    pub use_attributes: bool,
}

#[derive(Clone, Debug)]
pub struct ChannelPlayback {
    pub settings: ChannelSettings,
    pub weight: f32,
    pub influence: f32,
    blend_in_elapsed: f32,
    blend_out_elapsed: f32,
    resurrection_elapsed: f32,
    resurrection_duration: f32,
    flags: u32,
}
impl ChannelPlayback {
    pub fn new(settings: ChannelSettings) -> Self {
        let flags = (u32::from(settings.keep_alive) << 28)
            | (u32::from(settings.hold_during_blend_in) << 31)
            | (u32::from(settings.hold_during_blend_out) << 30)
            | (u32::from(settings.blend_out > 0.0) << 27)
            | (u32::from(settings.use_attributes) << 25);
        Self {
            settings,
            weight: 0.0,
            influence: 1.0,
            blend_in_elapsed: 0.0,
            blend_out_elapsed: 0.0,
            resurrection_elapsed: 0.0,
            resurrection_duration: 0.0,
            flags,
        }
    }
    ///Called before advancing any trees, in the native actor AddTime order.
    pub fn expired(&self) -> bool {
        if self.flags & 0x08000000 != 0 {
            self.blend_out_elapsed >= self.settings.blend_out
        } else {
            self.flags & 0x10000000 == 0 && self.flags & 0x04000000 != 0
        }
    }
    pub fn end(&mut self) {
        if self.blend_out_elapsed == 0.0 {
            self.flags |= 0x28000000;
        }
    }
    pub fn end_with(&mut self, seconds: f32, from_last_frame: bool) {
        if self.blend_out_elapsed == 0.0 {
            self.settings.blend_out = seconds;
            self.settings.hold_during_blend_out = from_last_frame;
            self.flags =
                (self.flags & !0x40000000) | (u32::from(from_last_frame) << 30) | 0x28000000;
        }
    }
    pub fn can_transition(&self, resurrect: bool) -> bool {
        !(self.blend_out_elapsed > 0.0) || resurrect
    }
    ///StartTransition82D1D398..D47C preserves fade-in time/influence and
    ///priority, replaces fade-out settings, and optionally restores a fading
    ///channel from its existing coefficient rather than restarting at zero.
    pub fn transition(&mut self, settings: ChannelSettings, resurrect: bool) {
        if self.blend_out_elapsed > 0.0 && resurrect {
            self.resurrection_duration = settings.blend_in;
            self.resurrection_elapsed =
                unit(1.0 - self.blend_out_elapsed / self.settings.blend_out) * settings.blend_in;
        }
        self.blend_out_elapsed = 0.0;
        self.settings.keep_alive = settings.keep_alive;
        self.settings.blend_out = settings.blend_out;
        self.settings.hold_during_blend_out = settings.hold_during_blend_out;
        self.settings.use_attributes = settings.use_attributes;
        self.flags = (self.flags & !0x7a000000)
            | (u32::from(settings.keep_alive) << 28)
            | (u32::from(settings.hold_during_blend_out) << 30)
            | (u32::from(settings.blend_out > 0.0) << 27)
            | (u32::from(settings.use_attributes) << 25);
    }
    ///Returns whether the channel child should advance. The base has already
    ///advanced; fade weights use PRE-update elapsed times, then increment them.
    pub fn advance(&mut self, dt: f32, length: f32, time: f32) -> bool {
        let mut fade_in = 1.0;
        if self.settings.blend_in > 0.0 && self.blend_in_elapsed < self.settings.blend_in {
            fade_in = unit(self.blend_in_elapsed / self.settings.blend_in);
            self.blend_in_elapsed += dt;
        }
        let remaining = length - time;
        let remaining = if remaining >= 0.0 { remaining } else { 0.0 };
        let hold_end = self.flags & 0x40000000 != 0;
        let ended = self.flags & 0x04000000 != 0;
        let fade_automatically = if hold_end {
            ended
        } else {
            remaining < self.settings.blend_out
        };
        let mut fade_out = 1.0;
        if self.settings.blend_out > 0.0
            && (self.flags & 0x20000000 != 0
                || (self.flags & 0x10000000 == 0 && fade_automatically))
        {
            fade_out = unit(1.0 - self.blend_out_elapsed / self.settings.blend_out);
            self.blend_out_elapsed += dt;
        }
        let mut resurrection = 1.0;
        if self.resurrection_duration > 0.0
            && self.resurrection_elapsed < self.resurrection_duration
        {
            resurrection = unit(self.resurrection_elapsed / self.resurrection_duration);
            self.resurrection_elapsed += dt;
        }
        let fade = if fade_in - fade_out >= 0.0 {
            fade_out
        } else {
            fade_in
        };
        let fade = if fade - resurrection >= 0.0 {
            resurrection
        } else {
            fade
        };
        self.weight = unit(fade * self.influence);
        !(self.flags & 0x80000000 != 0 && self.blend_in_elapsed < self.settings.blend_in)
            && !(hold_end && self.blend_out_elapsed > 0.0)
    }
    pub fn did_advance(&mut self, property: AdvanceResult) {
        self.flags = (self.flags & !0x04000000) | (u32::from(property.crossed_end) << 26);
    }
    pub fn merge_attributes(
        &self,
        base: Vec<AnimationAttribute>,
        channel: &[AnimationAttribute],
    ) -> Vec<AnimationAttribute> {
        if !self.settings.use_attributes {
            return base;
        }
        //82B966D0: ordered union, channel wins equal names without scaling.
        let mut output = Vec::with_capacity(base.len() + channel.len());
        let mut b = 0;
        let mut c = 0;
        while b < base.len() && c < channel.len() {
            match base[b].name.0.cmp(&channel[c].name.0) {
                core::cmp::Ordering::Less => {
                    output.push(base[b]);
                    b += 1;
                }
                core::cmp::Ordering::Greater => {
                    output.push(channel[c]);
                    c += 1;
                }
                core::cmp::Ordering::Equal => {
                    output.push(channel[c]);
                    b += 1;
                    c += 1;
                }
            }
        }
        output.extend_from_slice(&base[b..]);
        output.extend_from_slice(&channel[c..]);
        output
    }
}
fn unit(value: f32) -> f32 {
    let low = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - low >= 0.0 { low } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn settings() -> ChannelSettings {
        ChannelSettings {
            priority: 0,
            keep_alive: false,
            mirrored: false,
            speed: 1.0,
            blend_in: 0.5,
            hold_during_blend_in: true,
            blend_out: 0.5,
            hold_during_blend_out: false,
            use_attributes: false,
        }
    }
    #[test]
    fn preincrement_fade_and_expiry_order_preserve_native_first_frame() {
        let mut channel = ChannelPlayback::new(settings());
        assert!(!channel.advance(0.25, 2.0, 0.0));
        assert_eq!(channel.weight, 0.0);
        assert!(channel.advance(0.25, 2.0, 0.0));
        assert_eq!(channel.weight, 0.5);
        channel.end();
        assert!(channel.advance(0.25, 2.0, 0.25));
        assert_eq!(channel.weight, 1.0);
        assert!(!channel.expired());
        channel.advance(0.25, 2.0, 0.5);
        assert_eq!(channel.weight, 0.5);
        assert!(channel.expired());
    }
    #[test]
    fn returning_channel_restores_from_existing_fade_without_zero_restart() {
        let mut settings = settings();
        settings.hold_during_blend_in = false;
        let mut channel = ChannelPlayback::new(settings);
        channel.advance(0.5, 2.0, 0.0);
        channel.end();
        channel.advance(0.25, 2.0, 0.5);
        assert!(!channel.can_transition(false));
        assert!(channel.can_transition(true));
        channel.transition(settings, true);
        channel.advance(0.1, 2.0, 0.0);
        assert_eq!(channel.weight, 0.5);
        assert!(!channel.expired());
    }
}
