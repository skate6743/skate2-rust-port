//! The AEMS PLAYER node (`SNDAEMSI_updateplayer`, inlined in `InterpretModule`), `Snd9::PlayerPlay`
//! 0x82c19818, `Snd9::PlayerUpdate` 0x82c19948, `Snd9::PlayerSetInput` 0x82c17358, and Skate's
//! sample player (`Sk8::Audio::SkateSamplePlayerFactory::CreateInstance` 0x824f91d0,
//! `SkateSamplePlayer` 0x824e9f88..0x824ea5e0): one RWAC voice per started sample.
//!
//! `PLAYERSTATE`: `pModuleBank` +0, `psamplegroup` +4, `pSamplePlayer` +8, `prevplaycontrol[2]`
//! +0xC, `numinputs` +0xE, `updateoutputs` +0xF, `numuseroutputs` +0x11, `sampleselect` +0x14,
//! `playcontrol` +0x18, then `numinputs` `PLAYERINPUTSTATE`s {u8 type, prevvalue, value}, then
//! `PLAYEROUTPUTS` {timeleft, timecurr} when `updateoutputs`, then the user outputs.
//!
//! Not ported: the flange sends of patches 4096/0x2000/0x4000, the notch pair of patch 1024
//! (`PeakingIir2`) and the moveable object voices of patches 512/2048.

#![allow(clippy::float_cmp)]

use super::{rd16, rd32, rd8, wr32, wr8, Aems};
use crate::formats::snr::BitGetter;
use crate::rwac::core::{PlugInConfig, PlugInKind};
use crate::rwac::decoder::DataRef;
use crate::rwac::plugins::snd_player::PlayParams;
use crate::rwac::voice::VoiceState;
use crate::rwac::{AudioCore, PluginRef, VoiceId};

/// `__real_38000100`: 1 / 32767.
const Q15: f32 = f32::from_bits(0x3800_0100);
/// `__real_3bb40000`: 360 / 65536.
const AZIMUTH_DEGREES: f32 = f32::from_bits(0x3BB4_0000);
/// `Pan2D1::mDefaultConstructorFrontAngle` / `RearAngle`.
const FRONT_ANGLE: f32 = 45.0;
const REAR_ANGLE: f32 = 135.0;

/// `IAemsSamplePlayer::InputSelector`.
const INPUT_PITCHMULT: u8 = 0;
const INPUT_VOL: u8 = 2;
const INPUT_AZIMUTH: u8 = 3;
const INPUT_FXWET0: u8 = 5;
const INPUT_LOWPASS: u8 = 6;
const INPUT_HIGHPASS: u8 = 7;
const INPUT_DRYLEVEL: u8 = 8;

/// `Sk8::Audio::SkateSamplePlayer` (0x88 bytes).
#[derive(Debug, Clone)]
pub struct SamplePlayer {
    voice: Option<VoiceId>,
    snd: PluginRef,
    resample: PluginRef,
    high_pass: PluginRef,
    low_pass: PluginRef,
    /// +0x18 (the env-send wet send; absent without `GlobalEffects`).
    send_wet: Option<PluginRef>,
    /// +0x1C.
    gain: PluginRef,
    pan: PluginRef,
    /// +0x24 (the pitch `Unpause` restores), +0x28 volume, +0x2C dry, +0x30 wet.
    pitch: f32,
    volume: f32,
    dry: f32,
    wet: f32,
    /// +0x34: the play request handle.
    request_handle: f32,
    /// +0x38: `numSamples / sampleRate`.
    sample_length: f64,
    /// +0x40.
    paused: bool,
    /// +0x45: the default front and rear angles are still to be applied by `SetAzimuth`.
    angles_pending: bool,
}

/// `SndPlayer1::GetFileInfo` of an EAAC header: channels, rate, samples.
fn file_info(sample: &DataRef) -> Option<(u32, u32, u32)> {
    let mut bits = BitGetter::new(sample.bytes.get(sample.offset..)?);
    let _version = bits.get(4);
    let _codec = bits.get(4);
    let channels = bits.get(6) + 1;
    let rate = bits.get(18);
    let _play_type = bits.get(2);
    let _looping = bits.get(1);
    let samples = bits.get(29);
    Some((channels, rate, samples))
}

impl Aems {
    fn player(&self, id: u32) -> Option<&SamplePlayer> {
        self.players.get((id as usize).checked_sub(1)?)?.as_ref()
    }

    fn player_mut(&mut self, id: u32) -> Option<&mut SamplePlayer> {
        self.players.get_mut((id as usize).checked_sub(1)?)?.as_mut()
    }

    /// `AemsStandardSamplePlayer::Release` 0x82c1ade8 (vtable 0): `Voice::Release` and free.
    pub(super) fn release_player(&mut self, core: &mut AudioCore, id: u32) {
        if let Some(player) = self.players.get_mut((id as usize).wrapping_sub(1)).and_then(Option::take) {
            if let Some(voice) = player.voice {
                core.release_voice(voice);
            }
        }
    }

    /// `SkateSamplePlayer::Pause` 0x824ea4a0: the dry and wet gains and the pitch go to 0.
    fn pause_player(&mut self, core: &mut AudioCore, id: u32) {
        let Some(player) = self.player_mut(id) else { return };
        player.paused = true;
        let player = player.clone();
        core.set_attribute(player.gain, 0, 0.0);
        if let Some(wet) = player.send_wet {
            core.set_attribute(wet, 0, 0.0);
        }
        core.set_attribute(player.resample, 0, 0.0);
    }

    /// `SkateSamplePlayer::Unpause` 0x824ea540.
    fn unpause_player(&mut self, core: &mut AudioCore, id: u32) {
        let Some(player) = self.player_mut(id) else { return };
        player.paused = false;
        let player = player.clone();
        core.set_attribute(player.gain, 0, player.dry * player.volume);
        if let Some(wet) = player.send_wet {
            core.set_attribute(wet, 0, player.wet * player.volume);
        }
        core.set_attribute(player.resample, 0, player.pitch);
    }

    /// `Snd9::PlayerSetInput` 0x82c17358 (clamps) into `SkateSamplePlayer::SetInput`
    /// 0x824ea058 / `SetAzimuth` 0x824ea270.
    fn player_set_input(&mut self, core: &mut AudioCore, id: u32, input: u8, value: i32) {
        let Some(player) = self.player_mut(id) else { return };
        match input {
            INPUT_PITCHMULT => core.set_attribute(player.resample, 0, value.clamp(0, 0xFFFF) as f32 * f32::from_bits(0x3980_0000)),
            1 | 4 => {}
            INPUT_VOL => {
                player.volume = value.clamp(0, 0x7FFF) as f32 * Q15;
                if let Some(wet) = player.send_wet {
                    core.set_attribute(wet, 0, player.wet * player.volume);
                }
                core.set_attribute(player.gain, 0, player.dry * player.volume);
            }
            INPUT_AZIMUTH => {
                if player.angles_pending {
                    core.set_attribute(player.pan, 7, FRONT_ANGLE);
                    core.set_attribute(player.pan, 8, REAR_ANGLE);
                    player.angles_pending = false;
                }
                core.set_attribute(player.pan, 0, value as f32 * AZIMUTH_DEGREES);
            }
            INPUT_FXWET0 => {
                player.wet = value.clamp(0, 0x7FFF) as f32 * Q15;
                if let Some(wet) = player.send_wet {
                    core.set_attribute(wet, 0, player.volume * player.wet);
                }
            }
            INPUT_LOWPASS => core.set_attribute(player.low_pass, 0, value.clamp(0, 0xFFFF) as f32),
            INPUT_HIGHPASS => core.set_attribute(player.high_pass, 0, value.clamp(0, 0xFFFF) as f32),
            INPUT_DRYLEVEL => {
                player.dry = value.clamp(0, 0x7FFF) as f32 * Q15;
                core.set_attribute(player.gain, 0, player.volume * player.dry);
            }
            // User inputs 9..=136 reach SetInput, which only uses input 11 for flange patches.
            _ => {}
        }
    }

    /// `SkateSamplePlayer::GetOutputs` 0x824ea350 (11 outputs): playing, then the time left and
    /// the time played in ms once the request has started.
    fn player_outputs(&self, core: &AudioCore, id: u32) -> [i32; 11] {
        let mut outputs = [0; 11];
        let Some(player) = self.player(id) else { return outputs };
        if player.voice.and_then(|v| core.voice_state(v)).is_none_or(|state| state == VoiceState::Expelled) {
            return outputs;
        }
        outputs[0] = 1;
        let handle = core.attribute(player.snd, 0);
        if handle < player.request_handle {
            outputs[1] = (player.sample_length * 1000.0) as i32;
            return outputs;
        }
        let played = core.attribute_f64(player.snd, 1);
        outputs[2] = (played * 1000.0) as i32;
        outputs[1] = ((player.sample_length - played) * 1000.0) as i32;
        outputs
    }

    /// `SkateSamplePlayerFactory::CreateInstance` 0x824f91d0 for a sample without a patch:
    /// [SndPlayer1, Rechannel, Resample, HighPassIir2, LowPassIir2, Gain, Pan2D1 (6),
    /// Send (6) to the dry sub mix], the pan at the sample's azimuth, priority / 100. With
    /// `GlobalEffects`, a Send to its reverb sub mix (the wet send, silent until input 5) follows the
    /// low pass.
    fn create_sample_player(&mut self, core: &mut AudioCore, sample: DataRef, priority: i32, azimuths: [i32; 6]) -> Option<u32> {
        let (channels, rate, samples) = file_info(&sample)?;
        let azimuth_indices = match channels {
            1 => [0, 0, 0],
            2 => [0, 1, 0],
            4 => [0, 1, 3],
            _ => [1, 2, 4],
        };
        let channels = u8::try_from(channels).ok()?;
        let config = |kind: PlugInKind, output_channels: u8| PlugInConfig { kind, output_channels };
        let mut configs = vec![
            config(PlugInKind::SndPlayer1 { max_requests: 1.0 }, channels),
            config(PlugInKind::Rechannel, channels),
            config(PlugInKind::Resample, channels),
            config(PlugInKind::HighPassIir2, channels),
            config(PlugInKind::LowPassIir2, channels),
        ];
        if self.env_send_sub_mix.is_some() {
            configs.push(config(PlugInKind::Send, channels));
        }
        let gain = configs.len() as u8;
        configs.extend([
            config(PlugInKind::Gain, channels),
            config(PlugInKind::Pan2D1 { constructor: Some((FRONT_ANGLE, REAR_ANGLE, 0.0)) }, 6),
            config(PlugInKind::Send, 6),
        ]);
        let voice = core.create_voice(0, "sk8sampleplayerfactory.cpp", &configs)?;
        let plugin = |index: u8| PluginRef { voice, index };
        let mut player = SamplePlayer {
            voice: Some(voice),
            snd: plugin(0),
            resample: plugin(2),
            high_pass: plugin(3),
            low_pass: plugin(4),
            send_wet: self.env_send_sub_mix.map(|_| plugin(5)),
            gain: plugin(gain),
            pan: plugin(gain + 1),
            pitch: 1.0,
            volume: 1.0,
            dry: 1.0,
            wet: 0.0,
            request_handle: -1.0,
            sample_length: f64::from(samples) / f64::from(rate),
            paused: false,
            angles_pending: true,
        };
        core.set_priority(voice, priority as f32 * f32::from_bits(0x3C23_D70A));
        player.request_handle =
            core.play(player.snd, PlayParams { ram_sample: Some(sample), stream_pool: 0x4145_4D53, expel_mode: 1.0, ..PlayParams::default() });
        if let Some(wet) = player.send_wet {
            core.connect_send(wet, self.env_send_sub_mix);
            core.set_attribute(wet, 0, 0.0);
        }
        if channels <= 1 {
            core.set_attribute(player.pan, 0, azimuths[azimuth_indices[0]] as f32 * AZIMUTH_DEGREES);
            player.angles_pending = false;
        } else {
            core.set_attribute(player.pan, 1, 0.0);
            core.set_attribute(player.pan, 7, azimuths[azimuth_indices[1]] as f32 * AZIMUTH_DEGREES);
            if channels > 2 {
                core.set_attribute(player.pan, 8, azimuths[azimuth_indices[2]] as f32 * AZIMUTH_DEGREES);
            }
        }
        core.connect_send(plugin(gain + 2), self.dry_sub_mix);
        let slot = self.players.iter().position(Option::is_none).unwrap_or(self.players.len());
        if slot == self.players.len() {
            self.players.push(Some(player));
        } else {
            self.players[slot] = Some(player);
        }
        Some(slot as u32 + 1)
    }

    /// `Snd9::PlayerPlay` 0x82c19818: the factory's player for the group entry (a
    /// `Snd10SampleEntry` {s16 sampleNum, u8 priority, u8 azimuth[6]...}), then every input.
    fn player_play(&mut self, core: &mut AudioCore, slot: usize, at: usize, entry: usize) -> u32 {
        let Some(instance) = self.instances[slot].as_ref() else { return 0 };
        let bank_index = rd32(&instance.data, at) as usize;
        let Some(bank) = bank_index.checked_sub(1).and_then(|i| self.banks.get(i)?.as_ref()) else { return 0 };
        let resident = &bank.resident;
        let sample_number = rd16(resident, entry) as i16;
        let priority = i32::from(rd8(resident, entry + 2));
        let azimuths: [i32; 6] = std::array::from_fn(|i| i32::from(rd8(resident, entry + 3 + i)) << 8);
        let sample = bank.sample_bank.map(|header| {
            let offset = rd32(&bank.file, header + (sample_number as isize + 3) as usize * 4);
            DataRef { bytes: bank.file.clone(), offset: header.wrapping_add(offset as usize) }
        });
        if let Some(elapsed) = crate::splicer::trace_elapsed() {
            let module = self.banks.get(instance.bank).and_then(Option::as_ref).and_then(|own| own.modules.get(instance.module)).map_or(0, |m| m.id);
            // FNV-1a of the sample's first bytes: the same audio in two banks hashes alike.
            let hash = sample.as_ref().and_then(|s| s.bytes.get(s.offset..)).map_or(0, |bytes| {
                bytes.iter().take(512).fold(0x811c_9dc5_u32, |hash, &byte| (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193))
            });
            eprintln!("{elapsed:9.3} aems play {}#{module} sample {sample_number} data {hash:08x}", bank.name);
        }
        let inputs = usize::from(rd8(&instance.data, at + 0xE));
        let Some(id) = sample.and_then(|sample| self.create_sample_player(core, sample, priority, azimuths)) else {
            self.reset_player_outputs(core, slot, at);
            return 0;
        };
        for input in 0..inputs {
            let state = at + 0x1C + 12 * input;
            let data = self.data_of(slot);
            let (kind, value) = (rd8(data, state), rd32(data, state + 8));
            self.player_set_input(core, id, kind, value);
            if let Some(instance) = self.instances[slot].as_mut() {
                wr32(&mut instance.data, state + 4, value);
            }
        }
        id
    }

    fn data_of(&self, slot: usize) -> &[u8] {
        self.instances[slot].as_ref().map_or(&[], |i| i.data.as_slice())
    }

    /// `SNDAEMSI_playerresetoutputs` 0x82c18758: release the player, zero the outputs.
    fn reset_player_outputs(&mut self, core: &mut AudioCore, slot: usize, at: usize) {
        let id = rd32(self.data_of(slot), at + 8) as u32;
        if id != 0 {
            self.release_player(core, id);
            if let Some(instance) = self.instances[slot].as_mut() {
                wr32(&mut instance.data, at + 8, 0);
            }
        }
        if let Some(instance) = self.instances[slot].as_mut() {
            if rd8(&instance.data, at + 0xF) != 0 {
                let outputs = at + 0x1C + 12 * usize::from(rd8(&instance.data, at + 0xE));
                wr32(&mut instance.data, outputs + 4, 0);
                wr32(&mut instance.data, outputs, 0);
            }
        }
    }

    /// `Snd9::PlayerUpdate` 0x82c19948: changed inputs, then the outputs; a finished player is
    /// released (0), otherwise 1.
    fn player_update(&mut self, core: &mut AudioCore, slot: usize, at: usize) -> i32 {
        let id = rd32(self.data_of(slot), at + 8) as u32;
        let inputs = usize::from(rd8(self.data_of(slot), at + 0xE));
        for input in 0..inputs {
            let state = at + 0x1C + 12 * input;
            let data = self.data_of(slot);
            let (kind, previous, value) = (rd8(data, state), rd32(data, state + 4), rd32(data, state + 8));
            if value != previous {
                self.player_set_input(core, id, kind, value);
                if let Some(instance) = self.instances[slot].as_mut() {
                    wr32(&mut instance.data, state + 4, value);
                }
            }
        }
        let outputs = self.player_outputs(core, id);
        let outputs_at = at + 0x1C + 12 * inputs;
        if outputs[0] == 0 {
            self.reset_player_outputs(core, slot, at);
            return 0;
        }
        let Some(instance) = self.instances[slot].as_mut() else { return 1 };
        let data = &mut instance.data;
        let mut user = outputs_at;
        if rd8(data, at + 0xF) != 0 {
            wr32(data, outputs_at + 4, outputs[2]);
            wr32(data, outputs_at, outputs[1]);
            user = outputs_at + 8;
        }
        if rd8(data, at + 0x11) != 0 {
            for (i, &value) in outputs[3..11].iter().enumerate() {
                wr32(data, user + 4 * i, value);
            }
        }
        1
    }

    /// The PLAYER node: `playcontrol` clamped to 0 (stop), 1 (play) or 2 (pause); a change
    /// releases, pauses, resumes or starts the player (sample `sampleselect` of the group, clamped);
    /// a playing player updates. The value is the play state, 0 without a player.
    pub(super) fn player_node(&mut self, core: &mut AudioCore, slot: usize, at: usize) -> i32 {
        let data = self.data_of(slot);
        let control = rd32(data, at + 0x18).clamp(0, 2);
        let previous = rd8(data, at + 0xC) as i8 as i32;
        let mut state = control;
        if control != previous {
            let id = rd32(data, at + 8) as u32;
            match control {
                0 => {
                    if id != 0 {
                        self.reset_player_outputs(core, slot, at);
                    }
                }
                1 => {
                    if id != 0 {
                        self.unpause_player(core, id);
                    } else if !(previous == 2 && rd8(data, at + 0xD) == 1) {
                        let group = rd32(data, at + 4) as usize;
                        let select = rd32(data, at + 0x14);
                        let bank = self.bank_bytes_of(slot);
                        let count = rd32(bank, group);
                        let select = if select >= count {
                            count - 1
                        } else if select < 0 {
                            0
                        } else {
                            select
                        };
                        let entry = group + 4 + 12 * select.max(0) as usize;
                        if rd16(bank, entry) == 0xFFFF {
                            self.reset_player_outputs(core, slot, at);
                        } else {
                            let id = self.player_play(core, slot, at, entry);
                            if let Some(instance) = self.instances[slot].as_mut() {
                                wr32(&mut instance.data, at + 8, id as i32);
                            }
                        }
                    }
                }
                _ => {
                    if id != 0 {
                        self.pause_player(core, id);
                    }
                }
            }
            if let Some(instance) = self.instances[slot].as_mut() {
                let old = rd8(&instance.data, at + 0xC);
                wr8(&mut instance.data, at + 0xC, control as u8);
                wr8(&mut instance.data, at + 0xD, old);
            }
        }
        if state == 1 && rd32(self.data_of(slot), at + 8) != 0 {
            state = self.player_update(core, slot, at);
        }
        if rd32(self.data_of(slot), at + 8) == 0 {
            0
        } else {
            state
        }
    }

    fn bank_bytes_of(&self, slot: usize) -> &[u8] {
        self.instances[slot].as_ref().and_then(|i| self.banks.get(i.bank)?.as_ref()).map_or(&[], |b| b.resident.as_slice())
    }
}
