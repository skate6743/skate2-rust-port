//! TU3 OffboardBodyTweakBlend: ctor82BBAB98, Begin82BBACC0,
//! Update82BBACE0, End82BBB418, instance82BBB4D0. Original IDA only.
use super::MotionAnimation;
use crate::skate3_native::graph_host::motion_wipeout::Controls;
use skate3_core::animation::{
    channel_playback::ChannelSettings,
    playback::TransitionSettings,
    playback_parameters::{AttributeSink, ParameterInputs, SettableAttribute},
    skeleton_input::name::encode,
};
use skate_data::state_graph::attributes::Attributes;

const CHANNEL: &str = "AirBodyTweak"; // initializer82F87578 ->830BE028

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Settings {
    pub nb_cycle: String,
    pub nb_into: String,
    pub br_cycle: String,
    pub br_into: String,
}
impl Settings {
    pub fn parse(a: &Attributes<'_>) -> Self {
        Self {
            nb_cycle: a.text("NbCycAnim").unwrap_or("B_OBAIR_BODYTWEAK_NB_CYC").into(),
            nb_into: a.text("NbIntoAnim").unwrap_or("B_OBAIR_BODYTWEAK_NB_INTO").into(),
            br_cycle: a.text("BrCycAnim").unwrap_or("B_OBAIR_BODYTWEAK_BR_CYC").into(),
            br_into: a.text("BrIntoAnim").unwrap_or("B_OBAIR_BODYTWEAK_BR_INTO").into(),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Physical {
    pub time_to_land: f32,
    pub scalar_92: f32,
    pub board_held: bool,
}

#[derive(Default)]
pub(crate) struct State {
    ticks: u32,
    axes: [f32; 2],
    into_started: bool,
    cycle_started: bool,
    released: bool,
}
impl State {
    pub fn begin(&mut self) {
        // Raw82BBACC0: zero8,20,21,22. Axes12/16 are only zeroed at allocation.
        self.ticks = 0;
        self.into_started = false;
        self.cycle_started = false;
        self.released = false;
    }

    pub fn update(
        &mut self,
        settings: &Settings,
        animation: &mut MotionAnimation,
        controls: &mut Controls,
        physical: Physical,
    ) -> Result<(), String> {
        let x = animation.motion_intent("OB_AirBodyTweakX");
        let y = animation.motion_intent("OB_AirBodyTweakY");
        let request = animation.motion_intent("OB_DoAirBodyTweak").is_some();
        let (directed, centered) = self.gates(x, y, request, physical.scalar_92);
        let active = directed || centered;
        //82BBB030 is blt: unordered does not take the near-landing branch.
        if active && !(physical.time_to_land < 0.1) {
            let remaining = animation.channels.remaining(CHANNEL);
            if animation.channels.has(CHANNEL) || self.into_started {
                if !(remaining > 0.01) && !self.cycle_started {
                    transition(animation, settings, physical.board_held, true)?;
                    self.cycle_started = true;
                }
            } else {
                transition(animation, settings, physical.board_held, false)?;
                self.into_started = true;
            }
            self.filter(x.unwrap_or(0.0), y.unwrap_or(0.0), centered);
            for (name, value) in [b"bodytweakx", b"bodytweaky"].into_iter().zip(self.axes) {
                animation.set_attribute(SettableAttribute {
                    name: encode(name), value, normalized: false, sequence_id: -1,
                });
            }
            //82BBB328 ->MGv200/8258FBF0;82BBB33C ->ISkaterAnimv116/82B97228.
            controls.seed_from_air_tweak = true;
            set_flag(animation, 0x8000, true)?;
        } else if active && !(physical.time_to_land >= 0.1) {
            //82BBB36C ->ISkaterAnimv112/82B97218: one-frame request bit16.
            set_flag(animation, 0x10000, true)?;
        } else if animation.channels.has(CHANNEL) {
            let duration = if physical.time_to_land - 0.3 >= 0.0 {
                0.3
            } else {
                physical.time_to_land
            };
            animation.channels.end_with(CHANNEL, duration, true);
            controls.seed_from_air_tweak = false;
            set_flag(animation, 0x8000, false)?;
        }
        self.ticks = self.ticks.wrapping_add(1);
        Ok(())
    }

    fn gates(&mut self, x: Option<f32>, y: Option<f32>, request: bool, scalar_92: f32) -> (bool, bool) {
        if (self.ticks as i32) > 80 {
            self.released = true;
        }
        //82BBAED8..EE8 uses signed comparisons, not abs(axis).
        if !self.released && x.is_some_and(|x| !(x >= 0.1))
            && y.is_some_and(|y| !(y >= 0.1)) {
            self.released = true;
        }
        let x = x.unwrap_or(0.0);
        let y = y.unwrap_or(0.0);
        let squared = y.mul_add(y, x * x);
        //Same independent PC seed as native_arithmetic; both native refinements
        //remain below. This is not a claim of bit-exact Xenon rsqrt emulation.
        let mut inverse = squared.sqrt().recip();
        for _ in 0..2 {
            let correction = (-squared).mul_add(inverse * inverse, 1.0);
            inverse = (inverse * 0.5).mul_add(correction, inverse);
        }
        let magnitude = if squared == 0.0 { 0.0 } else { squared * inverse };
        let directed = self.released && magnitude >= 0.1;
        //82BBAFD4 is ble. This preserves its unordered path as well.
        (directed, request || (!(scalar_92 <= 4.0) && !directed))
    }

    fn filter(&mut self, x: f32, y: f32, centered: bool) {
        //Raw82BBB290..2C8: preserve fmuls/fmadds order and axis identity.
        self.axes = if centered {
            self.axes.map(|value| value * 0.85)
        } else {
            [x.mul_add(0.15, self.axes[0] * 0.85), self.axes[1].mul_add(0.85, y * 0.15)]
        };
    }

    pub fn end(&mut self, animation: &mut MotionAnimation) {
        //82BBB418 ends the channel from its last frame, but does not clear MGbit20.
        if animation.channels.has(CHANNEL) {
            animation.channels.end_with(CHANNEL, 0.1, true);
        }
    }
}

fn set_flag(animation: &mut MotionAnimation, bit: u32, value: bool) -> Result<(), String> {
    let flags = animation.skater_animation_flags.as_mut()
        .ok_or("OffboardBodyTweakBlend requires SkaterAnim flag owner")?;
    *flags = (*flags & !bit) | if value { bit } else { 0 };
    Ok(())
}

fn transition(animation: &mut MotionAnimation, settings: &Settings, held: bool, cycle: bool) -> Result<(), String> {
    let name = match (held, cycle) {
        (false, false) => &settings.nb_into,
        (false, true) => &settings.nb_cycle,
        (true, false) => &settings.br_into,
        (true, true) => &settings.br_cycle,
    };
    //82BBB164/270 call IChannelAnimatablev16=82D1D090. Stack57=false
    //resurrection,5F=true create-if-missing,67=true use animation attributes.
    animation.transition_channel(CHANNEL, name, ChannelSettings {
        priority: 0, keep_alive: cycle, mirrored: false, speed: 1.0,
        blend_in: if cycle { 0.5 } else { 0.15 }, hold_during_blend_in: false,
        blend_out: if cycle { 0.3 } else { 0.0 }, hold_during_blend_out: false,
        use_attributes: true,
    }, TransitionSettings {
        kind: 1, seconds: if cycle { 0.1 } else { 0.15 }, under: 0,
        matching: 0, use_channels_from_weights: false,
    }, false, true)?;
    Ok(())
}

#[cfg(any())]
#[path = "body_tweak/tests.rs"]
mod tests;
