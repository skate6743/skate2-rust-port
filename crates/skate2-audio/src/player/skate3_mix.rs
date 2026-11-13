//! What Skate 3 physics sounds need beyond Skate 2's audio (PC port): TU3's splice voice, which
//! its drop-in and darkcatch sounds play through, and the TU3 mixer channels those sounds read.
//!
//! The port mixes with Skate 2's `MixMapSK8.mxb`. [`add_channels`] gives its PLAYER state two more
//! SFX objects, each a set of master channels copied from Skate 3's map:
//! - object 12, TU3's `SFXObj_DropIn` ([`DROP_IN_CHANNELS`]);
//! - object 13, a port id for the outputs TU3's darkcatch catches and landing read from its
//!   `SFXObj_Contacts` (object 1), whose outputs in Skate 2's map are other channels
//!   ([`DARKCATCH_CHANNELS`]).
//!
//! Their references name the player's 3D controls, player sub channels and MAIN mix control 1,
//! which both maps hold at the same places; events Skate 3's map numbers differently are moved to
//! Skate 2's event with the same definition. The player sub channels are Skate 2's: Skate 3's sum
//! a few more events (its duck, sub channel 0, also MAIN events 139, 197, 219 and 220).

use super::{SfxCtx, AZIMUTH_SCALE, PITCH_SCALE, VOLUME_SCALE};
use crate::nicotine::map::{master_objects, with_master_channels};
use crate::nicotine::{DMixIoId, PresetType};
use crate::rwac::core::{PlugInConfig, PlugInKind};
use crate::rwac::{PluginRef, VoiceId};
use crate::splicer::{SpliceId, SplicePlayParams, SpliceUpdateParams};

/// The PLAYER mixer state (`eMAINMAPSTATES`).
const PLAYER_STATE: usize = 1;
/// TU3's `SFXObj_DropIn`: PLAYER object 12 (`objectID` 0x400100c0 + the state's instance).
pub const DROP_IN_SFX: u32 = 12;
const DROP_IN_OBJECT: u32 = 0x4001_00C0;
/// PC port: the darkcatch sounds' outputs of TU3's `SFXObj_Contacts`, as PLAYER object 13
/// (`objectID` 0x400100d0 + the state's instance).
pub const DARKCATCH_SFX: u32 = 13;
const DARKCATCH_OBJECT: u32 = 0x4001_00D0;

/// TU3's master channels for `SFXObj_DropIn`, as its `MixMapSK8.mxb` holds them in state 1
/// (masters 89 to 91: id, swing, `objectID`, inputs):
/// - volume over player 3D control 0, player sub channel 0 and MAIN mix control 1: the azimuth
///   into output 0, the lock at -10 dB into output 1, the release at -6 dB into output 3;
/// - pitch over player 3D control 1 and player sub channel 1, into outputs 2 and 4;
/// - the reverb send at -5 dB over the board's 3D control 4 and player sub channel 4, a further
///   -17.11 dB into output 5.
pub const DROP_IN_CHANNELS: [&[u32]; 3] = [
    &[0xC003_0159, 0x0000_D8F0, DROP_IN_OBJECT, 0x9001_0000, 0x3001_0000, 0x0800_0001],
    &[0xC102_015A, 0x0000_D8F0, DROP_IN_OBJECT, 0x9001_0001, 0x3001_0001],
    &[0xC002_015B, 0xFE0C_D8F0, DROP_IN_OBJECT, 0x9001_0004, 0x3001_0004],
];
/// Their preset groups (head, entries).
pub const DROP_IN_PRESETS: [&[u32]; 3] = [
    &[0xE001_5903, 0x8000_0000, 0x0401_FC18, 0x0C03_FDA8],
    &[0xE101_5A02, 0x0802_0000, 0x1004_0000],
    &[0xE001_5B01, 0x1405_F951],
];

/// TU3's `SFXObj_Contacts` master channels for the outputs its darkcatch sounds read (state 1
/// masters 27, 28, 33, 37, 41 and 44), as object 13:
/// - the azimuth (from player 3D control 2) into output 0, and -6 dB over 3D control 0 and
///   player sub channel 8 into output 6;
/// - the pitch over 3D control 1, player sub channel 9 and MAIN event 41 into output 1;
/// - the landing's reverb send at -5 dB over the board's 3D control 4, player sub channel 4, player
///   events 11 and 26 and mix controls MAIN 1 and PLAYER 13, -11 dB into output 15;
/// - the landing at -5 dB over 3D control 2, player sub channel 8, player events 5, 10 and 26,
///   MAIN events 29 and 9 and PLAYER mix control 13, -8.88 dB into output 3;
/// - the catches at -5 dB over 3D control 2, player sub channel 8 and player event 22, -4.88 dB
///   into output 8 (and output 20);
/// - the catches' reverb send at -5 dB over the board's 3D control 4 and player sub channel 4,
///   -12 dB into output 21.
///
/// Skate 3's map numbers those events MAIN 39 and 28, PLAYER 21, 4 and 25; its PLAYER event 9
/// fires on the contacts' input 6 where Skate 2's event 10 fires on input 7, the input both games
/// set for the local skater after a landing's collision sound (TU3 LandGround 0x824bac04), and is
/// otherwise the same. Two of Skate 3's references are richer: its player sub channel 8 also sums
/// MAIN events 42 and 51, and its PLAYER mix control 13 is also scaled by mix control 95 (a curve
/// of the physics controller's input 9).
pub const DARKCATCH_CHANNELS: [&[u32]; 6] = [
    &[0xC003_011B, 0xFDA8_D8F0, DARKCATCH_OBJECT, 0x9001_0002, 0x9001_0000, 0x3001_0008],
    &[0xC103_011C, 0x0000_D8F0, DARKCATCH_OBJECT, 0x9001_0001, 0x3001_0009, 0xB100_3029],
    &[0xC006_0121, 0xFE0C_D8F0, DARKCATCH_OBJECT, 0x9001_0004, 0x3001_0004, 0xB101_300B, 0x0800_0001, 0x0901_400D, 0xB101_301A],
    &[
        0xC008_0125, 0xFE0C_D8F0, DARKCATCH_OBJECT, 0x9001_0002, 0x3001_0008, 0xB101_3005, 0xB101_300A, 0xB100_301D, 0x0901_400D,
        0xB101_301A, 0xB100_3009,
    ],
    &[0xC003_0129, 0xFE0C_D8F0, DARKCATCH_OBJECT, 0x9001_0002, 0x3001_0008, 0xB101_3016],
    &[0xC002_012C, 0xFE0C_D8F0, DARKCATCH_OBJECT, 0x9001_0004, 0x3001_0004],
];
/// Their preset groups (head, entries).
pub const DARKCATCH_PRESETS: [&[u32]; 6] = [
    &[0xE001_1B02, 0x8000_0000, 0x1826_FDA8],
    &[0xE101_1C01, 0x0401_0000],
    &[0xE001_2101, 0x3C0F_FBB4],
    &[0xE001_2501, 0x0C03_FC88],
    &[0xE001_2902, 0x2008_FE18, 0x5014_0000],
    &[0xE001_2C01, 0x5415_FB50],
];

/// PC port: Skate 2's mix map with [`DROP_IN_CHANNELS`] and [`DARKCATCH_CHANNELS`] added to the
/// PLAYER state; a map whose PLAYER state already names either object, or that does not read
/// whole, stays as it is.
#[must_use]
pub fn add_channels(data: Vec<u8>) -> Vec<u8> {
    let named = master_objects(&data, PLAYER_STATE)
        .is_none_or(|objects| objects.iter().any(|&object| object == DROP_IN_OBJECT || object == DARKCATCH_OBJECT));
    if named {
        return data;
    }
    let records: Vec<&[u32]> = DROP_IN_CHANNELS.iter().chain(&DARKCATCH_CHANNELS).copied().collect();
    let presets: Vec<&[u32]> = DROP_IN_PRESETS.iter().chain(&DARKCATCH_PRESETS).copied().collect();
    with_master_channels(&data, PLAYER_STATE, &records, &presets).unwrap_or(data)
}

/// TU3's splice voice (16 bytes, vtable 0x822fbf3c, constructor 0x82497f48): a splice playing into
/// a voice of its own (0x82498248): processing stage 1, a stereo SubMix and two stereo Sends, the
/// first into `GlobalEffects`' environment send at the level each update sets (0 until then), the
/// second into the sub mix the sound was made for.
pub struct Skate3Splice {
    /// +4.
    splice: Option<SpliceId>,
    /// +0xC (+8 holds its plug-ins).
    voice: VoiceId,
}

impl Skate3Splice {
    /// The constructor 0x82497f48: the voice, then `new Splice(bank, index, the voice's SubMix)` and
    /// its `Play` (no time stretch) with a silent update of zero delta.
    pub(super) fn new(ctx: &mut SfxCtx<'_>, bank: usize, index: i32, target: PluginRef) -> Option<Self> {
        let stereo = |kind: PlugInKind| PlugInConfig { kind, output_channels: 2 };
        let configs = [stereo(PlugInKind::SubMix { name: String::new() }), stereo(PlugInKind::Send), stereo(PlugInKind::Send)];
        let voice = ctx.core.create_voice(1, "Skate3Splice", &configs)?;
        ctx.core.connect_send(PluginRef { voice, index: 1 }, Some(ctx.global_effects.reverb_sub_mix()));
        ctx.core.set_attribute(PluginRef { voice, index: 1 }, 0, 0.0);
        ctx.core.connect_send(PluginRef { voice, index: 2 }, Some(target));
        let splice = ctx.splicer.create_splice(ctx.core, bank, index, PluginRef { voice, index: 0 });
        if let Some(splice) = splice {
            let update = SpliceUpdateParams { vol: 0.0, pitch: 1.0, az: 0.0, delta: 0.0, spread: 1.0, time_stretch: 1.0 };
            ctx.splicer.play_splice(ctx.core, splice, SplicePlayParams::default(), &update);
        }
        Some(Self { splice, voice })
    }

    /// `Splicer::Splice::IsPlaying` of the splice.
    fn is_playing(&self, ctx: &SfxCtx<'_>) -> bool {
        self.splice.is_some_and(|splice| ctx.is_playing(splice))
    }

    /// 0x82498140: `Splice::Update` with `update`, then the environment send's level (the first
    /// Send's attribute 0) to `wet`.
    fn update(&self, ctx: &mut SfxCtx<'_>, update: SpliceUpdateParams, wet: f32) {
        if let Some(splice) = self.splice {
            ctx.update_splice(splice, update);
        }
        ctx.core.set_attribute(PluginRef { voice: self.voice, index: 1 }, 0, wet);
    }

    /// The destructor 0x82498068: the splice deleted, then the voice released.
    fn delete(mut self, ctx: &mut SfxCtx<'_>) {
        ctx.release(&mut self.splice);
        ctx.core.release_voice(self.voice);
    }

    /// Deletes the sound a slot holds (`(*p)->~` with the slot cleared).
    pub(super) fn delete_slot(sound: &mut Option<Self>, ctx: &mut SfxCtx<'_>) {
        if let Some(sound) = sound.take() {
            sound.delete(ctx);
        }
    }
}

/// One update of a sound in a splice voice (TU3 0x824f0038, 0x824bebd8 and UpdateJumpsAndLands
/// 0x824be900): deleted once it stops playing, else the mixer's volume output `volume`, pitch
/// output `pitch`, azimuth output 0, the state's delta time, spread 0 and time stretch 1, with the
/// reverb send at volume output `wet`.
pub(super) fn follow(sound: &mut Option<Skate3Splice>, io: Option<DMixIoId>, volume: i32, pitch: i32, wet: i32, ctx: &mut SfxCtx<'_>) {
    let Some(playing) = sound.as_ref() else { return };
    if !playing.is_playing(ctx) {
        Skate3Splice::delete_slot(sound, ctx);
        return;
    }
    let update = SpliceUpdateParams {
        vol: ctx.output(io, volume, PresetType::Volume) as f32 * VOLUME_SCALE,
        pitch: ctx.output(io, pitch, PresetType::Pitch) as f32 * PITCH_SCALE,
        az: ctx.output(io, 0, PresetType::Azimuth) as f32 * AZIMUTH_SCALE,
        delta: ctx.delta_time,
        spread: 0.0,
        time_stretch: 1.0,
    };
    let wet = ctx.output(io, wet, PresetType::Volume) as f32 * VOLUME_SCALE;
    playing.update(ctx, update, wet);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A two-state map whose PLAYER state (1) holds one master channel for object 11, followed at
    /// once by its presets and by another state's block.
    fn map() -> Vec<u8> {
        let words: Vec<u32> = vec![
            0, 2, 0x0C, // type, states, the state table at 0x0C
            0x14, 0x14, // both states' headers at 0x14
            // the header at 0x14: masters at +0x20 (0x34), presets at +0x40 (0x54)
            0xFFFF_FFFF, 0xFFFF_FFFF, 0xFFFF_FFFF, 0xFFFF_FFFF, 0x20, 0x40, 0xFFFF_FFFF, 0xFFFF_FFFF,
            // the masters at 0x34: one channel, one object, then its record
            1, 1, 0, 0, 0xC001_0100, 0x0000_D8F0, 0x4001_00B0, 0x3001_0000,
            // the presets at 0x54: one group
            0xE001_0001, 0x0401_0000,
            // the next block, at 0x5C
            0xDEAD_BEEF,
        ];
        words.iter().flat_map(|word| word.to_be_bytes()).collect()
    }

    #[test]
    fn the_skate3_channels_join_the_player_state_once() {
        let original = map();
        assert_eq!(master_objects(&original, PLAYER_STATE), Some(vec![0x4001_00B0]));
        let patched = add_channels(original.clone());
        assert_eq!(patched[..0x24], original[..0x24]);
        assert_eq!(patched[0x2C..original.len()], original[0x2C..], "only the header's two offsets change");
        let mut objects = vec![0x4001_00B0];
        objects.extend([DROP_IN_OBJECT; 3]);
        objects.extend([DARKCATCH_OBJECT; 6]);
        assert_eq!(master_objects(&patched, PLAYER_STATE), Some(objects));
        let word = |at: usize| u32::from_be_bytes(patched[at..at + 4].try_into().unwrap());
        let masters = 0x14 + word(0x14 + 0x10) as usize;
        assert_eq!(masters, original.len(), "rebuilt at the end of the map");
        assert_eq!((word(masters), word(masters + 4)), (10, 3), "ten channels, three objects with output arrays");
        assert_eq!(word(masters + 16 + 16 + 8), DROP_IN_OBJECT, "the first new record follows the old one");
        let presets = 0x14 + word(0x14 + 0x14) as usize;
        assert_eq!((word(presets), word(presets + 8)), (0xE001_0001, DROP_IN_PRESETS[0][0]), "the old presets come first");
        let added: usize = DROP_IN_PRESETS.iter().chain(&DARKCATCH_PRESETS).map(|group| 4 * group.len()).sum();
        assert_eq!(presets + 8 + added, patched.len());
        assert_eq!(add_channels(patched.clone()), patched, "a map that has them is left alone");
        assert_eq!(add_channels(vec![0u8; 8]), vec![0u8; 8], "a map that does not read is left alone");
    }

    #[test]
    fn every_channel_reads_whole() {
        for (record, group) in DROP_IN_CHANNELS.iter().zip(&DROP_IN_PRESETS).chain(DARKCATCH_CHANNELS.iter().zip(&DARKCATCH_PRESETS)) {
            assert_eq!(record.len(), ((record[0] >> 16) & 0xFF) as usize + 3, "{record:08x?}");
            assert_eq!(group.len(), (group[0] & 0xFF) as usize + 1, "{group:08x?}");
        }
    }
}
