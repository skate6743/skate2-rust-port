//! `Sk8::Audio::GlobalEffects` (0x43C bytes, constructor 0x82505c10): the two environment
//! reverbs every wet send feeds, the two flanges, and the LFO task that sweeps them.
//!
//! `CreateReverbVoice` 0x824f3618 (one channel up to the six-channel pans):
//! - "EnvSendSub" (stage 199): SubMix, Send 1 and Send 2 at each instance's `fxlevel`. Its
//!   SubMix is `GetReverbSubMix`, the bus of every wet send.
//! - "EnvSub" per instance (200): SubMix, Gain (`fxgain`), PeakingIir2 (the notch), three Sends
//!   into the instance's reverb and its two delays.
//! - "RvrbSub" per instance (201): SubMix, Delay (pre-delay), ReverbModel1, Gain, two Sends into
//!   the instance's left and right "rvrbfiltsub".
//! - "rvrbfiltsub" (202) and "rvrbdelaysub" (201), two each per instance: SubMix, (Delay,)
//!   HighPassIir2, LowPassIir2, Gain, Pan2D1 (6), Send (6) to the master mix.
//!
//! `CreateFlangeVoice` 0x824f44a0: "FlangeSub" (150) twice, index 0 for patches 0x2000/0x3000
//! and 1 for 0x4000: SubMix, Delay, PeakingIir2, Send (wet, into "EnvSendSub"), Gain (dry),
//! Pan2D1 (6), Send (6) to the master mix.
//!
//! TU3 adds eight effect sub mixes (0x82490CA0, after `CreateLFOs`), stage 253, six channels:
//! SubMix, DistortionClip, two PeakingIir2, Send to the master mix. Its darkcatch sounds play into
//! the one `GlobalAttributeCollection` +0x8C names, and the local player's sound sets its filters from that
//! effect's attributes (0x824916E8); every audio update switches them all off again
//! (0x82491180). Built only when a Skate 3 sound first asks for one.
//!
//! Every Delay reads its constructor parameter one word past the float the creators store (the
//! debug `Param` is {tag, value}): the assert path pointer there is a tiny negative number, so
//! every line starts at its 256-sample minimum and its timer grows it to the delay asked for.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::math::{fmadd, fnmsub};
use crate::rwac::core::{PlugInConfig, PlugInKind};
use crate::random::RwRandom;
use crate::rwac::{AudioCore, PluginRef, VoiceId};
use crate::vault::{Collection, Vault};
use crate::vmx;

/// TU3's effect class and its eight collections (`qword_8224DC78`).
const SKATE3_EFFECT_CLASS: u64 = 0xAA80_1D9F_C0AD_BBBF;
const SKATE3_EFFECTS: [u64; 8] = [
    0x42DD_FA3F_5011_BAB4, 0xDA24_A51D_DC51_EDF3, 0x201C_8871_4085_E3C3, 0x8AF2_3914_D89E_4AF0,
    0xF7CF_A0A8_D2F9_8BDE, 0x8AA9_3567_C056_736B, 0x87D5_F21D_9A71_736A, 0xBFA2_D9D8_C138_F925,
];
/// Whether the effect takes its own attributes (otherwise its jitter values, not ported).
const SKATE3_USE_ATTRIBUTES: u64 = 0xC6DA_68C1_2A38_22D2;
/// Each PeakingIir2's frequency, gain and Q: (high, low) attribute pairs.
const SKATE3_FILTERS: [[(u64, u64); 3]; 2] = [
    [
        (0x5B48_CBEB_3EB4_40D2, 0x2B77_D4F5_8AF7_1CCF),
        (0x7317_CC02_098B_DFF6, 0x90A9_4DDD_A5F8_04CB),
        (0xF30B_7500_D67E_206B, 0xB419_0B5C_4243_2FA2),
    ],
    [
        (0x3264_F689_300A_522A, 0x1DA6_11DA_4DE5_A67A),
        (0x5486_8EDE_952E_F581, 0x2585_3859_640A_3796),
        (0x594D_91B3_0B95_483A, 0xCDF5_99CD_EC9C_7C64),
    ],
];
/// The DistortionClip's level.
const SKATE3_CLIP_LEVEL: u64 = 0x2086_A0CE_99C3_9A86;

struct Skate3Effects {
    voices: [VoiceId; 8],
    /// `+0x47C`: switched on this update.
    active: [bool; 8],
}

/// The id `RWAudioLFOTask::AddTimer` 0x824f4920 registers its `TIMERSTAGE_BEFORECOMMANDS` timer
/// under.
pub const LFO_TIMER_ID: u32 = 0x4C46_4F20;
/// `TWO_PI_3`.
const TWO_PI: f32 = f32::from_bits(0x40C9_0FDB);
/// `kEnableReverb` (`Audio_Debug` "Enable Reverb", default on).
const ENABLE_REVERB: bool = true;
/// The patch of flange 1 (`patchID == 0x4000` picks it).
pub const FLANGE_PATCH_1: u32 = 0x4000;

/// `Sk8::Audio::LFOParms` (0x20 bytes): `out = maxrange * depth * sin(2 pi t / cycletime) + base`.
#[derive(Debug, Clone, Copy)]
pub struct LfoParms {
    pub cycle_time: f32,
    pub cur_time: f32,
    pub depth: f32,
    pub base: f32,
    pub max_range: f32,
    pub out: f32,
    pub plugin: PluginRef,
    /// Never set by `SetFlangeParms` (taken as 0).
    pub attribute: usize,
}

/// The LFO fields of `tFlangePresetParms` `SetFlangeParms` copies (the plug-in is the flange's).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LfoShape {
    pub cycle_time: f32,
    pub depth: f32,
    pub base: f32,
    pub max_range: f32,
}

/// `Sk8::Audio::tEnvRvrbPresetParms` (0xD4 bytes).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EnvRvrbPresetParms {
    pub inst: usize,
    pub fxlevel: f32,
    pub fxgain: f32,
    pub notch_freq: f32,
    pub notch_gain: f32,
    pub notch_q: f32,
    pub notch_freq_lfodepth: f32,
    pub notch_freq_lfofreq: f32,
    pub send_reverb: f32,
    pub send_delay: [f32; 2],
    pub predelay_time: f32,
    pub predelay_fbgain: f32,
    pub reverb_time: f32,
    pub reverb_size: f32,
    pub reverb_brightness: f32,
    pub reverb_gain: f32,
    /// Left then right.
    pub filter: [FilterParms; 2],
    pub delay: [DelayParms; 2],
}

/// The `left_*` / `right_*` fields of `tEnvRvrbPresetParms`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FilterParms {
    pub lpf_freq: f32,
    pub hpf_freq: f32,
    pub gain: f32,
    pub panazimuth: f32,
    pub panradius: f32,
    pub lfe_level: f32,
}

/// The `delay_0_*` / `delay_1_*` fields of `tEnvRvrbPresetParms`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DelayParms {
    pub time: f32,
    pub fbgain: f32,
    pub time_lfodepth: f32,
    pub time_lfofreq: f32,
    pub lpf_freq: f32,
    pub hpf_freq: f32,
    pub gain: f32,
    pub panazimuth: f32,
    pub panradius: f32,
    pub lfe_level: f32,
    pub pan_lfodepth: f32,
    pub pan_lfofreq: f32,
}

/// `Sk8::Audio::tEnvRvrbRuntimeParms` (0x54 bytes) as `UpdateReverb` reads it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EnvRvrbRuntimeParms {
    pub inst: usize,
    pub fxlevel: f32,
    pub notch_freq: f32,
    pub notch_freqlfodepth: f32,
    pub notch_freqlfofreq: f32,
    pub left_gain: f32,
    pub left_panazimuth: f32,
    pub right_gain: f32,
    pub right_panazimuth: f32,
    pub delay_time: [f32; 2],
    pub delay_panazimuth: [f32; 2],
    pub delay_timelfodepth: [f32; 2],
    pub delay_timelfofreq: [f32; 2],
    pub delay_panlfodepth: [f32; 2],
    pub delay_panlfofreq: [f32; 2],
}

/// `Sk8::Audio::tFlangePresetParms` (0x60 bytes). Only the LFOs reach the plug-ins.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FlangePresetParms {
    pub patch_id: u32,
    pub delay_time: f32,
    pub delay_feedback: f32,
    pub notch_freq: f32,
    pub notch_gain: f32,
    pub notch_q: f32,
    pub wet_depth: f32,
    pub dry_depth: f32,
    pub delay_lfo: LfoShape,
    pub notch_lfo: LfoShape,
}

/// `Sk8::Audio::tFlangeRuntimeParms` (0x24 bytes); the attribute indices are never set (0).
#[derive(Debug, Clone, Copy)]
struct FlangeRuntimeParms {
    pan: f32,
    wet_depth: f32,
    dry_depth: f32,
    pan_plugin: PluginRef,
    wet_send: PluginRef,
    dry_gain: PluginRef,
}

pub struct GlobalEffects {
    /// +0x000..+0x024: the voices.
    env_send_sub: VoiceId,
    env_sub: [VoiceId; 2],
    rvrb: [VoiceId; 2],
    rvrb_filt: [VoiceId; 4],
    rvrb_delay: [VoiceId; 4],
    /// +0x068.
    global_reverb_volume: f32,
    /// +0x06C.
    flange_sub: [VoiceId; 2],
    /// +0x07C, +0x0C4.
    flange_runtime: [FlangeRuntimeParms; 2],
    flange_preset: [FlangePresetParms; 2],
    /// +0x18C: `mLFOTaskObject.mLFOParms`.
    lfos: [LfoParms; 14],
    /// +0x364, +0x438: the last `SetupReverb` parameters.
    tpp: EnvRvrbPresetParms,
    tpp_set: bool,
    master: PluginRef,
    skate3_effects: Option<Skate3Effects>,
}

fn plugin(voice: VoiceId, index: u8) -> PluginRef {
    PluginRef { voice, index }
}

fn create_voice(core: &mut AudioCore, stage: u8, name: &'static str, configs: &[PlugInConfig]) -> VoiceId {
    core.create_voice(stage, name, configs).expect("GlobalEffects voice")
}

impl GlobalEffects {
    /// The constructor: `CreateReverbVoice`, `CreateFlangeVoice`, then `CreateLFOs` 0x825009c8
    /// (cycle 1 s, depth 0.5, each sweeping one attribute 0 around its base) with the timer.
    ///
    /// # Panics
    /// Never: the core builds these plug-ins.
    pub fn new(core: &mut AudioCore, master: PluginRef) -> Self {
        let mono = |kind: PlugInKind| PlugInConfig { kind, output_channels: 1 };
        let six = |kind: PlugInKind| PlugInConfig { kind, output_channels: 6 };
        let sub_mix = |name: &str| mono(PlugInKind::SubMix { name: name.into() });
        let env_send_sub = create_voice(core, 199, "EnvSendSub", &[sub_mix("EnvSendSub"), mono(PlugInKind::Send), mono(PlugInKind::Send)]);
        let env_sub_configs = [
            sub_mix("EnvSub"),
            mono(PlugInKind::Gain),
            mono(PlugInKind::PeakingIir2),
            mono(PlugInKind::Send),
            mono(PlugInKind::Send),
            mono(PlugInKind::Send),
        ];
        let env_sub = [create_voice(core, 200, "EnvSub", &env_sub_configs), create_voice(core, 200, "EnvSub", &env_sub_configs)];
        let rvrb_configs = [
            sub_mix("RvrbSub"),
            mono(PlugInKind::Delay { max_time: None }),
            mono(PlugInKind::ReverbModel1 { space_size: None }),
            mono(PlugInKind::Gain),
            mono(PlugInKind::Send),
            mono(PlugInKind::Send),
        ];
        let rvrb = [create_voice(core, 201, "RvrbSub", &rvrb_configs), create_voice(core, 201, "RvrbSub", &rvrb_configs)];
        let filt_configs = [
            sub_mix("rvrbfiltsub"),
            mono(PlugInKind::HighPassIir2),
            mono(PlugInKind::LowPassIir2),
            mono(PlugInKind::Gain),
            six(PlugInKind::Pan2D1 { constructor: None }),
            six(PlugInKind::Send),
        ];
        let rvrb_filt = [(); 4].map(|()| create_voice(core, 202, "rvrbfiltsub", &filt_configs));
        let delay_configs = [
            sub_mix("rvrbdelaysub"),
            mono(PlugInKind::Delay { max_time: None }),
            mono(PlugInKind::HighPassIir2),
            mono(PlugInKind::LowPassIir2),
            mono(PlugInKind::Gain),
            six(PlugInKind::Pan2D1 { constructor: None }),
            six(PlugInKind::Send),
        ];
        let rvrb_delay = [(); 4].map(|()| create_voice(core, 201, "rvrbdelaysub", &delay_configs));
        core.connect_send(plugin(env_send_sub, 1), Some(plugin(env_sub[0], 0)));
        core.connect_send(plugin(env_send_sub, 2), Some(plugin(env_sub[1], 0)));
        for inst in 0..2 {
            let (filt, delay) = (2 * inst, 2 * inst);
            core.connect_send(plugin(env_sub[inst], 3), Some(plugin(rvrb[inst], 0)));
            core.connect_send(plugin(env_sub[inst], 4), Some(plugin(rvrb_delay[delay], 0)));
            core.connect_send(plugin(env_sub[inst], 5), Some(plugin(rvrb_delay[delay + 1], 0)));
            core.connect_send(plugin(rvrb[inst], 4), Some(plugin(rvrb_filt[filt], 0)));
            core.connect_send(plugin(rvrb[inst], 5), Some(plugin(rvrb_filt[filt + 1], 0)));
            core.connect_send(plugin(rvrb_filt[filt], 5), Some(master));
            core.connect_send(plugin(rvrb_filt[filt + 1], 5), Some(master));
            core.connect_send(plugin(rvrb_delay[delay], 6), Some(master));
            core.connect_send(plugin(rvrb_delay[delay + 1], 6), Some(master));
        }
        let flange_configs = [
            sub_mix("FlangeSub"),
            mono(PlugInKind::Delay { max_time: None }),
            mono(PlugInKind::PeakingIir2),
            mono(PlugInKind::Send),
            mono(PlugInKind::Gain),
            six(PlugInKind::Pan2D1 { constructor: None }),
            six(PlugInKind::Send),
        ];
        let flange_sub = [create_voice(core, 150, "FlangeSub", &flange_configs), create_voice(core, 150, "FlangeSub", &flange_configs)];
        for &flange in &flange_sub {
            core.connect_send(plugin(flange, 3), Some(plugin(env_send_sub, 0)));
        }
        for &flange in &flange_sub {
            core.connect_send(plugin(flange, 6), Some(master));
        }
        let flange_runtime = flange_sub.map(|flange| FlangeRuntimeParms {
            pan: 0.0,
            wet_depth: 0.0,
            dry_depth: 0.0,
            pan_plugin: plugin(flange, 5),
            wet_send: plugin(flange, 3),
            dry_gain: plugin(flange, 4),
        });
        let lfo = |plugin: PluginRef, base: f32| LfoParms { cycle_time: 1.0, cur_time: 0.0, depth: 0.5, base, max_range: base, out: 0.0, plugin, attribute: 0 };
        let lfos = [
            lfo(plugin(env_sub[0], 2), 1.0),
            lfo(plugin(rvrb_delay[0], 1), 0.3),
            lfo(plugin(rvrb_delay[1], 1), 0.3),
            lfo(plugin(rvrb_delay[0], 5), 0.3),
            lfo(plugin(rvrb_delay[1], 5), 0.3),
            lfo(plugin(env_sub[1], 2), 0.3),
            lfo(plugin(rvrb_delay[2], 1), 0.3),
            lfo(plugin(rvrb_delay[3], 1), 0.3),
            lfo(plugin(rvrb_delay[2], 5), 0.3),
            lfo(plugin(rvrb_delay[3], 5), 0.3),
            lfo(plugin(flange_sub[0], 1), f32::from_bits(0x3BC4_9BA6)),
            lfo(plugin(flange_sub[0], 2), 1.0),
            lfo(plugin(flange_sub[1], 1), f32::from_bits(0x3BC4_9BA6)),
            lfo(plugin(flange_sub[1], 2), 1.0),
        ];
        core.add_before_commands_timer(LFO_TIMER_ID);
        Self {
            env_send_sub,
            env_sub,
            rvrb,
            rvrb_filt,
            rvrb_delay,
            global_reverb_volume: 1.0,
            flange_sub,
            flange_runtime,
            flange_preset: [FlangePresetParms::default(); 2],
            lfos,
            tpp: EnvRvrbPresetParms::default(),
            tpp_set: false,
            master,
            skate3_effects: None,
        }
    }

    /// TU3 `sub_82491108`: the sub mix a sound routed to effect `index` plays into (the master mix
    /// for 8 and out of range), and for the local player `sub_824916E8` switching the effect on:
    /// with its attributes, each PeakingIir2's frequency, gain and Q (a pair's low end plus a
    /// random tenth step of 0..10 toward its high end when they differ) and the clip level.
    pub fn skate3_effect(&mut self, core: &mut AudioCore, index: i32, local: bool, vault: &Vault, rng: &mut RwRandom) -> PluginRef {
        let Some(index) = usize::try_from(index).ok().filter(|&index| index < 8) else { return self.master };
        let master = self.master;
        let effects = self.skate3_effects.get_or_insert_with(|| {
            let six = |kind: PlugInKind| PlugInConfig { kind, output_channels: 6 };
            let configs = [
                six(PlugInKind::SubMix { name: "Skate3FxSub".into() }),
                six(PlugInKind::DistortionClip),
                six(PlugInKind::PeakingIir2),
                six(PlugInKind::PeakingIir2),
                six(PlugInKind::Send),
            ];
            let voices = [(); 8].map(|()| create_voice(core, 253, "Skate3FxSub", &configs));
            for &voice in &voices {
                core.connect_send(plugin(voice, 4), Some(master));
            }
            Skate3Effects { voices, active: [false; 8] }
        });
        let voice = effects.voices[index];
        if local && !effects.active[index] {
            effects.active[index] = true;
            let collection = Collection::new(SKATE3_EFFECT_CLASS, SKATE3_EFFECTS[index]);
            let attributes = vault.bool(collection, SKATE3_USE_ATTRIBUTES);
            let mut set = [[0.0; 3]; 2];
            let mut clip = 100.0;
            if attributes {
                for (filter, keys) in SKATE3_FILTERS.iter().enumerate() {
                    for (attribute, &(high, low)) in keys.iter().enumerate() {
                        let (high, low) = (vault.f32(collection, high), vault.f32(collection, low));
                        let value = if low == high {
                            low
                        } else {
                            let step = (rng.next_u32() % 11) as f32;
                            fmadd(step * (high - low), 0.1, low)
                        };
                        core.set_attribute(plugin(voice, 2 + filter as u8), attribute, value);
                        set[filter][attribute] = value;
                    }
                }
                clip = vault.f32(collection, SKATE3_CLIP_LEVEL);
                core.set_attribute(plugin(voice, 1), 0, clip);
            }
            trace_skate3_effect(index, attributes, clip, set);
        }
        plugin(voice, 0)
    }

    /// TU3 `sub_82491180`, each audio update: every effect off again.
    pub fn reset_skate3_effects(&mut self) {
        if let Some(effects) = self.skate3_effects.as_mut() {
            effects.active = [false; 8];
        }
    }

    /// `GetReverbSubMix` 0x824e8668: the "EnvSendSub" SubMix.
    #[must_use]
    pub fn reverb_sub_mix(&self) -> PluginRef {
        plugin(self.env_send_sub, 0)
    }

    /// `GetFlangeSubMix` 0x824e8678: flange 1's SubMix for patch 0x4000, flange 0's otherwise.
    #[must_use]
    pub fn flange_sub_mix(&self, patch_id: u32) -> PluginRef {
        plugin(self.flange_sub[usize::from(patch_id == FLANGE_PATCH_1)], 0)
    }

    /// `SetGlobalReverbVolume` 0x824e8660.
    pub fn set_global_reverb_volume(&mut self, volume: f32) {
        self.global_reverb_volume = volume;
    }

    /// `SetFlangeAzimuth` 0x824e86a0: both flanges, whatever the patch.
    pub fn set_flange_azimuth(&mut self, _patch_id: u32, pan: f32) {
        for runtime in &mut self.flange_runtime {
            runtime.pan = pan;
        }
    }

    /// `SetFlangeWet` 0x824e86b0.
    pub fn set_flange_wet(&mut self, patch_id: u32, wet: f32) {
        self.flange_runtime[usize::from(patch_id == FLANGE_PATCH_1)].wet_depth = wet;
    }

    /// `SetFlangeDry` 0x824e86c8.
    pub fn set_flange_dry(&mut self, patch_id: u32, dry: f32) {
        self.flange_runtime[usize::from(patch_id == FLANGE_PATCH_1)].dry_depth = dry;
    }

    /// The reverb sends of instance `inst`, scaled by the global volume (all off without
    /// `kEnableReverb`).
    fn set_sends(&self, core: &mut AudioCore, inst: usize, tpp: &EnvRvrbPresetParms) {
        let env_sub = self.env_sub[inst];
        let levels = if ENABLE_REVERB {
            [tpp.send_reverb * self.global_reverb_volume, tpp.send_delay[0] * self.global_reverb_volume, tpp.send_delay[1] * self.global_reverb_volume]
        } else {
            [0.0; 3]
        };
        for (index, level) in (3..).zip(levels) {
            core.set_attribute(plugin(env_sub, index), 0, level);
        }
    }

    /// `SetupReverb` 0x824f2ef8: instance `inst` takes the preset (the notch gain 0.1..20, else
    /// 0.1; Q 0.2..20, else 0.2), and its notch, delay time and delay pan LFOs restart (a rate
    /// under 0.1 Hz for the notch, 0.01 Hz for the delays, gives a 10 s cycle; the pans swing
    /// 90 degrees around 270 and 90, the second half a cycle on).
    pub fn setup_reverb(&mut self, core: &mut AudioCore, tpp: &EnvRvrbPresetParms) {
        self.tpp = *tpp;
        self.tpp_set = true;
        let inst = tpp.inst;
        core.set_attribute(plugin(self.env_send_sub, if inst != 0 { 2 } else { 1 }), 0, tpp.fxlevel);
        let env_sub = self.env_sub[inst];
        core.set_attribute(plugin(env_sub, 1), 0, tpp.fxgain);
        core.set_attribute(plugin(env_sub, 2), 0, tpp.notch_freq);
        let gain = if tpp.notch_gain < 0.1 || !(tpp.notch_gain <= 20.0) { 0.1 } else { tpp.notch_gain };
        core.set_attribute(plugin(env_sub, 2), 1, gain);
        let q = if tpp.notch_q < 0.2 || tpp.notch_q > 20.0 { 0.2 } else { tpp.notch_q };
        core.set_attribute(plugin(env_sub, 2), 2, q);
        let cycle = |frequency: f32, minimum: f32| if frequency < minimum { 10.0 } else { 1.0 / frequency };
        let lfo = |plugin: PluginRef, cycle_time: f32, cur_time: f32, depth: f32, base: f32, max_range: f32| LfoParms {
            cycle_time,
            cur_time,
            depth,
            base,
            max_range,
            out: 0.0,
            plugin,
            attribute: 0,
        };
        let first = 5 * inst;
        self.lfos[first] = lfo(
            plugin(env_sub, 2),
            cycle(tpp.notch_freq_lfofreq, 0.1),
            0.0,
            tpp.notch_freq_lfodepth,
            tpp.notch_freq,
            tpp.notch_freq,
        );
        self.set_sends(core, inst, tpp);
        let rvrb = self.rvrb[inst];
        core.set_attribute(plugin(rvrb, 1), 0, tpp.predelay_time);
        core.set_attribute(plugin(rvrb, 1), 1, tpp.predelay_fbgain);
        core.set_attribute(plugin(rvrb, 2), 0, tpp.reverb_time);
        core.set_attribute(plugin(rvrb, 2), 1, tpp.reverb_size);
        core.set_attribute(plugin(rvrb, 2), 2, tpp.reverb_brightness);
        core.set_attribute(plugin(rvrb, 3), 0, tpp.reverb_gain);
        for (side, filter) in tpp.filter.iter().enumerate() {
            let filt = self.rvrb_filt[2 * inst + side];
            core.set_attribute(plugin(filt, 1), 0, filter.hpf_freq);
            core.set_attribute(plugin(filt, 2), 0, filter.lpf_freq);
            core.set_attribute(plugin(filt, 3), 0, filter.gain);
            core.set_attribute(plugin(filt, 4), 0, filter.panazimuth);
            core.set_attribute(plugin(filt, 4), 1, filter.panradius);
            core.set_attribute(plugin(filt, 4), 6, filter.lfe_level);
        }
        for (side, parms) in tpp.delay.iter().enumerate() {
            let delay = self.rvrb_delay[2 * inst + side];
            core.set_attribute(plugin(delay, 1), 0, parms.time);
            core.set_attribute(plugin(delay, 1), 1, parms.fbgain);
            core.set_attribute(plugin(delay, 2), 0, parms.hpf_freq);
            core.set_attribute(plugin(delay, 3), 0, parms.lpf_freq);
            core.set_attribute(plugin(delay, 4), 0, parms.gain);
            core.set_attribute(plugin(delay, 5), 0, parms.panazimuth);
            core.set_attribute(plugin(delay, 5), 1, parms.panradius);
            core.set_attribute(plugin(delay, 5), 6, parms.lfe_level);
            self.lfos[first + 1 + side] = lfo(plugin(delay, 1), cycle(parms.time_lfofreq, 0.01), 0.0, parms.time_lfodepth, parms.time, parms.time);
            let pan_cycle = cycle(parms.pan_lfofreq, 0.01);
            let (pan_start, pan_base) = if side == 0 { (0.0, 270.0) } else { (pan_cycle * 0.5, 90.0) };
            self.lfos[first + 3 + side] = lfo(plugin(delay, 5), pan_cycle, pan_start, parms.pan_lfodepth, pan_base, 90.0);
        }
    }

    /// `UpdateReverb` 0x824f2c20: the level, notch, filter gains and pans, and delay times and
    /// pans of instance `inst` (the LFOs take the delays and the notch over again).
    pub fn update_reverb(&self, core: &mut AudioCore, parms: &EnvRvrbRuntimeParms) {
        let inst = parms.inst;
        core.set_attribute(plugin(self.env_send_sub, if inst != 0 { 2 } else { 1 }), 0, parms.fxlevel);
        core.set_attribute(plugin(self.env_sub[inst], 2), 0, parms.notch_freq);
        let (left, right) = (self.rvrb_filt[2 * inst], self.rvrb_filt[2 * inst + 1]);
        core.set_attribute(plugin(left, 4), 0, parms.left_panazimuth);
        core.set_attribute(plugin(left, 3), 0, parms.left_gain);
        core.set_attribute(plugin(right, 4), 0, parms.right_panazimuth);
        core.set_attribute(plugin(right, 3), 0, parms.right_gain);
        for side in 0..2 {
            let delay = self.rvrb_delay[2 * inst + side];
            core.set_attribute(plugin(delay, 1), 0, parms.delay_time[side]);
            core.set_attribute(plugin(delay, 5), 0, parms.delay_panazimuth[side]);
        }
    }

    /// `UpdateGain` 0x824f2db0: instance `index`'s sends from the last `SetupReverb` parameters
    /// and the global volume.
    pub fn update_gain(&self, core: &mut AudioCore, index: usize) {
        if self.tpp_set {
            self.set_sends(core, index, &self.tpp);
        }
    }

    /// `SetFlangeParms` 0x824f4220: flange 1 for patch 0x4000, flange 0 otherwise; its delay time
    /// and notch frequency LFOs restart from the preset. The other fields reach no plug-in.
    pub fn set_flange_parms(&mut self, parms: &FlangePresetParms) {
        let index = usize::from(parms.patch_id == FLANGE_PATCH_1);
        self.flange_preset[index] = *parms;
        let flange = self.flange_sub[index];
        let lfo = |shape: LfoShape, plugin: PluginRef| LfoParms {
            cycle_time: shape.cycle_time,
            cur_time: 0.0,
            depth: shape.depth,
            base: shape.base,
            max_range: shape.max_range,
            out: 0.0,
            plugin,
            attribute: 0,
        };
        self.lfos[10 + 2 * index] = lfo(parms.delay_lfo, plugin(flange, 1));
        self.lfos[11 + 2 * index] = lfo(parms.notch_lfo, plugin(flange, 2));
    }

    /// `UpdateFlange` 0x824f43b0: each flange's pan, wet send and dry gain.
    pub fn update_flange(&self, core: &mut AudioCore) {
        for runtime in &self.flange_runtime {
            core.set_attribute(runtime.pan_plugin, 0, runtime.pan);
            core.set_attribute(runtime.wet_send, 0, runtime.wet_depth);
            core.set_attribute(runtime.dry_gain, 0, runtime.dry_depth);
        }
    }

    /// `RWAudioLFOTask::RwAudioCoreTimer` 0x824e86e0, each frame: the time wraps by the cycle
    /// (VMX reciprocal, truncation and a fused subtract), then the sine sets the attribute.
    pub fn lfo_timer(&mut self, core: &mut AudioCore, period: f32) {
        for lfo in &mut self.lfos {
            let time = lfo.cur_time + period;
            let cycles = (vmx::recip(lfo.cycle_time) * time).trunc();
            lfo.cur_time = fnmsub(cycles, lfo.cycle_time, time);
            let phase = (lfo.cur_time / lfo.cycle_time) * TWO_PI;
            let sine = f64::from(phase).sin() as f32;
            lfo.out = fmadd(lfo.max_range * lfo.depth, sine, lfo.base);
            core.set_attribute(lfo.plugin, lfo.attribute, lfo.out);
        }
    }
}

/// `SKATE2_TRACE_SKATE3_DARKCATCH`: an effect switched on, with the values it was given.
fn trace_skate3_effect(index: usize, attributes: bool, clip: f32, filters: [[f32; 3]; 2]) {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_DARKCATCH").is_some()) {
        eprintln!("skate 3 darkcatch audio: effect {index} attributes {attributes}: clip {clip} filters {:?} {:?}",
            filters[0], filters[1]);
    }
}
