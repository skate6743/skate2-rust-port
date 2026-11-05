//! Offboard animation lifecycle:82BB9F48/A070 and82BBACC0/ACE0/B418.
use crate::skate3_native::graph_host::motion_animation::MotionAnimation;
use skate3_core::{
    animation::{
        channel_playback::ChannelSettings,
        playback::TransitionSettings,
        playback_parameters::{AttributeSink, SettableAttribute},
        skeleton_input::name::encode,
    },
    graph::intents::IntentMap,
    player::input_phase::OffBoardOutputFields,
};
use skate_data::state_graph::attributes::Attributes;
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Operation {
    MatchTime,
    Tweak([String; 4]),
}
impl Operation {
    pub fn parse(a: &Attributes<'_>) -> Option<Self> {
        match a.text("name") {
            Some("MatchAirTime") => Some(Self::MatchTime),
            Some("OffboardBodyTweakBlend") => Some(Self::Tweak([
                a.text("NbCycAnim")
                    .unwrap_or("B_OBAIR_BODYTWEAK_NB_CYC")
                    .into(),
                a.text("NbIntoAnim")
                    .unwrap_or("B_OBAIR_BODYTWEAK_NB_INTO")
                    .into(),
                a.text("BrCycAnim")
                    .unwrap_or("B_OBAIR_BODYTWEAK_BR_CYC")
                    .into(),
                a.text("BrIntoAnim")
                    .unwrap_or("B_OBAIR_BODYTWEAK_BR_INTO")
                    .into(),
            ])),
            _ => None,
        }
    }
}
#[derive(Default)]
pub(crate) struct State {
    pub seed_write: Option<bool>,
    phase: f32,
    first: bool,
    ticks: u32,
    armed: bool,
    into: bool,
    cycle: bool,
    tweak: [f32; 2],
}
fn attribute(a: &mut MotionAnimation, name: &str, value: f32) {
    a.set_attribute(SettableAttribute {
        name: encode(name.as_bytes()),
        value,
        normalized: false,
        sequence_id: -1,
    });
}
impl State {
    pub fn execute(
        &mut self,
        operation: &Operation,
        phase: u8,
        p: OffBoardOutputFields,
        intents: &IntentMap,
        a: &mut MotionAnimation,
    ) -> Result<(), String> {
        self.seed_write = None;
        match operation {
            Operation::MatchTime => {
                if phase == 0 {
                    self.phase = p.phase_80;
                    self.first = true;
                }
                if phase != 1 {
                    return Ok(());
                }
                if !self.first && p.air_duration_92 > 0. {
                    a.seek_current_fraction((1. - p.scalar_32 / p.air_duration_92).clamp(0., 1.));
                }
                let mut direction = p.landing_direction_96;
                let length = direction[..3].iter().map(|v| v * v).sum::<f32>().sqrt();
                if length > 0.01 {
                    direction = direction.map(|v| v * length.min(2.) / length);
                }
                for (name, value) in [
                    ("CadenceStartPercent", self.phase),
                    ("AnimTime", p.air_duration_92),
                    ("AnimTransX", direction[0]),
                    ("AnimTransY", direction[1]),
                    ("AnimTransZ", direction[2]),
                ] {
                    attribute(a, name, value);
                }
                self.first = false;
            }
            Operation::Tweak(clips) => {
                const CHANNEL: &str = "AirBodyTweak"; //82F87578
                if phase == 0 {
                    self.ticks = 0;
                    self.into = false;
                    self.cycle = false;
                    self.armed = false;
                    return Ok(());
                }
                if phase != 1 {
                    if a.channels.has(CHANNEL) {
                        a.channels.end_with(CHANNEL, 0.1, true);
                    }
                    return Ok(());
                }
                let x = intents.get("OB_AirBodyTweakX").copied();
                let y = intents.get("OB_AirBodyTweakY").copied();
                if self.ticks > 80 || matches!((x,y),(Some(x),Some(y)) if x<0.1&&y<0.1) {
                    self.armed = true;
                }
                let (x, y) = (x.unwrap_or(0.), y.unwrap_or(0.));
                let active = self.armed && (x * x + y * y).sqrt() >= 0.1;
                let forced = intents.contains_key("OB_DoAirBodyTweak")
                    || (p.air_duration_92 > 4. && !active);
                if (forced || active) && p.scalar_32 >= 0.1 {
                    let remaining = a.channels.remaining(CHANNEL);
                    let held = if p.flag_311 != 0 { 2 } else { 0 };
                    let next = if !a.channels.has(CHANNEL) && !self.into {
                        self.into = true;
                        Some((held + 1, false, 0.15, 0., 0.15))
                    } else if remaining <= 0.01 && !self.cycle {
                        self.cycle = true;
                        Some((held, true, 0.5, 0.3, 0.1))
                    } else {
                        None
                    };
                    if let Some((index, keep_alive, blend_in, blend_out, seconds)) = next {
                        a.transition_channel(
                            CHANNEL,
                            &clips[index],
                            ChannelSettings {
                                priority: 0,
                                keep_alive,
                                mirrored: false,
                                speed: 1.,
                                blend_in,
                                hold_during_blend_in: false,
                                blend_out,
                                hold_during_blend_out: false,
                                use_attributes: false,
                            },
                            TransitionSettings {
                                kind: 1,
                                seconds,
                                under: 0,
                                matching: 0,
                                use_channels_from_weights: false,
                            },
                            true,
                            true,
                        )?;
                    }
                    self.tweak = [
                        self.tweak[0] * 0.85 + if forced { 0. } else { x * 0.15 },
                        self.tweak[1] * 0.85 + if forced { 0. } else { y * 0.15 },
                    ];
                    attribute(a, "bodytweakx", self.tweak[0]);
                    attribute(a, "bodytweaky", self.tweak[1]);
                    self.seed_write = Some(true);
                } else if !forced && !active && a.channels.has(CHANNEL) {
                    a.channels.end_with(CHANNEL, p.scalar_32.min(0.3), true);
                    self.seed_write = Some(false);
                }
                self.ticks = self.ticks.wrapping_add(1);
            }
        }
        Ok(())
    }
}
