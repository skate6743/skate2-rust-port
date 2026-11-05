//! `Sk8::Audio::SFXObj_OffBoard` (592 bytes, PLAYER object 10, `CreateObject` 0x82549858) and the
//! `FootStepUtils` it drives. Off the board each foot has a `playercharacter_footstep` AEMS
//! instance (`SK8_AEMS_Foley.csi`); for the local skater a foot touching down also plays its heel
//! (walk collision, or the grass/metal table) and foot-push splices through four "FootStep
//! SubMix" voices per foot, filtered by the splice's `aud_cluster` tuning and a random notch pair.
//! Every step plays the cloth foley, with the board's rattle while the local skater carries it;
//! off-board jumps, hippy jumps and leaving a foot plant play the jump cloth.
//!
//! Not ported: `CheckForOffBoardSpeech` 0x8253a4a0 (a non-local skater stepping back on the board
//! plays living-world speech through `Sk8SpeechSystem`; `mbWasInOffBoardState` only feeds it).

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use super::physics::{point_graph, PlayerPhysics};
use super::{SfxCtx, AZIMUTH_SCALE, PITCH_SCALE, VOLUME_SCALE};
use crate::aems::ClassObject;
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, PresetType};
use crate::presentation::MATERIAL_MAX;
use crate::random::RwRandom;
use crate::rwac::core::{PlugInConfig, PlugInKind};
use crate::rwac::plugins::pan2d::DEFAULT_CONSTRUCTOR;
use crate::rwac::{AudioCore, PluginRef, VoiceId};
use crate::splicer::{SpliceId, SplicePlayParams, SpliceUpdateParams};
use crate::surface::{foot_step_material, material_collection, material_splice};
use crate::vault::{class, Collection, Vault};

/// `GlobalAttributeCollection`'s `aud_general` footstep (+0x78) and clothing (+0x58) collections.
const FOOTSTEP: Collection = Collection::new(class::AUD_GENERAL, 0x1ABD_2984_D724_8589);
const CLOTHING: Collection = Collection::new(class::AUD_GENERAL, 0x3982_76A6_3764_95EE);

/// Footstep attributes: the notch ranges and Q (integer pairs), the 16-point speed graphs, the
/// walk/run threshold, the six AEMS volumes and the splice tuning tables (the collisions bank's
/// and every other bank's).
const NOTCH_FREQ_MAX: u64 = 0x43AA_0442_D256_3D17;
const NOTCH_FREQ_MIN: u64 = 0x859D_597E_1862_647E;
const NOTCH_GAIN_MAX: u64 = 0xB795_9E9B_E344_BA44;
const NOTCH_GAIN_MIN: u64 = 0x69F6_08EE_8A18_8CCB;
const NOTCH_Q: u64 = 0x1C23_7A44_FB97_22C8;
const SPEED_GRAPH: u64 = 0xC3CD_069B_B1B1_6B58;
const LINEAR_GRAPH: u64 = 0xCF84_4597_AB96_EAF8;
const ANGULAR_GRAPH: u64 = 0x2363_1160_4A3C_1FB5;
const RUN_THRESHOLD: u64 = 0x729D_6290_FB6A_8E3B;
const VOLUMES: u64 = 0x6364_64FB_AD0D_71A3;
const COLLISION_TUNING: u64 = 0x60B0_43CC_A6F2_11C3;
const TUNING: u64 = 0xCA94_5189_6540_51D3;
/// The clothing collection's sprint and run speeds.
const CLOTH_SPEEDS: u64 = 0xE12A_F885_D3C3_A168;

/// `aud_cluster` (layout: volume factor, notch Q, notch gain, notch frequency, low-pass and
/// high-pass frequencies) and the "default" key `GetSpliceTuningKey` falls back to.
const AUD_CLUSTER: u64 = 0x370A_F270_4BFA_6866;
const DEFAULT_KEY: u64 = 0xD7ED_BD36_2D7D_2152;
/// `aud_material` `HOM_Soft_L1`, `HOM_Soft_L3` and `HOM_Medium_L3`.
const HOM_SOFT_L1: u64 = 0x137C_683B_FB50_6ECA;
const HOM_SOFT_L3: u64 = 0x5432_B352_24E4_B1C1;
const HOM_MEDIUM_L3: u64 = 0x3FCB_E040_7F83_3721;

/// `SPLICE_TYPE`: the collisions, metal and Hall of Meat banks, `sk82_cloth` and
/// `sk8_foley_rattle`.
const SPLICE_COLLISIONS: i32 = 5;
const SPLICE_METAL: i32 = 6;
const SPLICE_HOM: i32 = 7;
const SPLICE_CLOTH: i32 = 0x17;
const SPLICE_RATTLE: i32 = 0x18;
/// `sk82_cloth_foley` `clth_wlk`, `clth_run` and `clth_sprnt`.
const CLOTH_WALK: i32 = 0x15;
const CLOTH_RUN: i32 = 0x16;
const CLOTH_SPRINT: i32 = 0x17;

/// `eFootStepSurface` metal and grass.
const SURFACE_METAL: u32 = 5;
const SURFACE_GRASS: u32 = 7;
/// `eFootStepWalkRun`, `eFootContact` (heel) and `FootStepSound`.
const WALK: usize = 0;
const RUN: usize = 1;
const HEEL: usize = 0;
const PRIMARY: usize = 0;
const SECONDARY: usize = 1;
/// `eAudioMaterialType_Concrete_Polished` and `eAudioMaterialType_Concrete_Rough`.
const MATERIAL_CONCRETE_POLISHED: u32 = 2;
const MATERIAL_CONCRETE_ROUGH: u32 = 3;
/// `eHippyJumpStrength`.
const SHORT_HIPPY: u32 = 0;
const NORMAL_HIPPY: u32 = 1;
const HIGH_HIPPY: u32 = 2;
/// `eYDirection`.
const GOING_UP: u32 = 0;
const GOING_DOWN: u32 = 1;
const DIRECTIONS_MAX: u32 = 2;

/// `mRightFoot`/`mLeftFoot` and the right/left pairs of the other fields.
const RIGHT: usize = 0;
const LEFT: usize = 1;
/// The frames `UpdateParams` flags a jump for after leaving a foot plant.
const FOOT_PLANT_JUMP_FRAMES: i32 = 10;
/// `UpdateSplicerVoice`'s degrees per azimuth unit (`__real_3bb40000`, 360/65536).
const VOICE_AZIMUTH_SCALE: f32 = f32::from_bits(0x3BB4_0000);

/// `types` 0x82fd8f68: the grass/metal heel splices by `((speed * 2 + contact) * 2 + sound) * 2`
/// plus 0 on grass, 1 on metal.
const TYPES: [i32; 16] = [764, 439, 766, 440, -1, 441, -1, 442, 765, 443, 767, 444, -1, -1, -1, -1];
/// `spliceTypeMap` 0x82fd8e88: per terrain row (flat, ramp and stair terrains first) and surface
/// 1..=7, the primary and secondary foot-push banks, then their splices.
const SPLICE_TYPE_MAP: [i32; 56] = [
    1, -1, 11, -1, 0, -1, 10, -1, 5, 2, 768, 12, 5, 2, 769, 12, 2, -1, 12, -1, 3, -1, 12, -1, 4, -1, 11, -1, //
    1, -1, 12, -1, 0, -1, 11, -1, 5, 2, 768, 13, 5, 2, 769, 13, 2, -1, 13, -1, 3, -1, 13, -1, 4, -1, 11, -1,
];

/// `Csis::Playercharacter_footstep` members.
const VOLUME: usize = 0;
const AZIMUTH: usize = 1;
const PITCH: usize = 2;
const LOW_PASS: usize = 4;
const HIGH_PASS: usize = 5;
const WET_FX: usize = 6;
const DRY_FX: usize = 7;
const FOOT_HEEL: usize = 8;
const FOOT_TOE: usize = 9;
const FOOT_ANGLE_VEL: usize = 10;
const FOOT_SLIDE_VEL: usize = 11;
const INITIATE_JUMP: usize = 15;
const JUMP_LANDED: usize = 16;
const PLAYER_SPEED: usize = 18;
const SHOE_TYPE: usize = 19;
const FOOT_SURFACE: usize = 20;
const TERRAIN: usize = 21;
const STEP_VOL: usize = 22;
const LOCAL_SKATER: usize = 28;
const GAIN_0: usize = 29;
const FREQ_0: usize = 30;
const Q_0: usize = 31;
const GAIN_1: usize = 32;
const FREQ_1: usize = 33;
const Q_1: usize = 34;
/// The member ranges the constructor 0x8250fe58 and the inline setters clamp to.
const RANGES: [(i32, i32); 35] = [
    (0, 0x7FFF),
    (0, 0xFFFF),
    (0, 0x2000),
    (0, 1000),
    (0, 25001),
    (0, 25001),
    (0, 0x7FFF),
    (0, 0x7FFF),
    (0, 1),
    (0, 1),
    (0, 1000),
    (0, 1000),
    (0, 1),
    (0, 1),
    (0, 1),
    (0, 1),
    (1, 4),
    (0, 1000),
    (0, 1000),
    (0, 5),
    (1, 7),
    (1, 5),
    (0, 0x7FFF),
    (0, 0x7FFF),
    (0, 0x7FFF),
    (0, 0x7FFF),
    (0, 0x7FFF),
    (0, 0x7FFF),
    (0, 1),
    (0, 0x7FFF),
    (0, 24000),
    (0, 20),
    (0, 0x7FFF),
    (0, 24000),
    (0, 20),
];

fn set(data: &mut [i32], member: usize, value: i32) {
    let (min, max) = RANGES[member];
    data[member] = value.clamp(min, max);
}

fn layout_byte(vault: &Vault, collection: Collection, offset: u32) -> u8 {
    (vault.layout_word(collection, offset / 4) >> (24 - 8 * (offset % 4))) as u8
}

/// `AudioSystem::GetWalkCollision` 0x824effb0: `(bank, splice)`, the material's `Soft_L1` walking,
/// `Soft_L3` running, else `Medium_L3` splice of its bank. Out of range or without `Add_In_Walk`
/// (layout byte 0x7D) the splice is -1 and the bank stays 0; banks other than collisions, metal and
/// Hall of Meat leave the splice 0.
fn walk_collision(vault: &Vault, material: u32, speed: usize) -> (i32, i32) {
    let collection = match material_collection(material) {
        Some(collection) if material < MATERIAL_MAX => collection,
        _ => return (0, -1),
    };
    if layout_byte(vault, collection, 0x7D) == 0 {
        return (0, -1);
    }
    let bank = material_splice(material);
    let word = |offset: u32| vault.layout_word(collection, offset / 4) as i32;
    let splice = match (bank, speed) {
        (SPLICE_COLLISIONS, WALK) => word(0x40),
        (SPLICE_COLLISIONS, RUN) => word(0x3C),
        (SPLICE_COLLISIONS, _) => word(0x68),
        (SPLICE_METAL, WALK) => word(0x50),
        (SPLICE_METAL, RUN) => word(0x4C),
        (SPLICE_METAL, _) => word(0x54),
        (SPLICE_HOM, WALK) => vault.i32(collection, HOM_SOFT_L1),
        (SPLICE_HOM, RUN) => vault.i32(collection, HOM_SOFT_L3),
        (SPLICE_HOM, _) => vault.i32(collection, HOM_MEDIUM_L3),
        _ => 0,
    };
    (bank, splice)
}

/// `AudioSystem::GetWalkVolume` 0x824f01b8: `Walk_Volume` (layout word 12) over 32767.
fn walk_volume(vault: &Vault, material: u32) -> f32 {
    match material_collection(material) {
        Some(collection) if material < MATERIAL_MAX => vault.layout_word(collection, 12) as i32 as f32 * VOLUME_SCALE,
        _ => 0.0,
    }
}

/// `FootStepUtils::GetMaterialVolume` 0x824f5670: the walk volume, `None` on grass and metal
/// surfaces and materials without a walk collision.
fn material_volume(vault: &Vault, material: u32) -> Option<f32> {
    let surface = foot_step_material(vault, material);
    if surface == SURFACE_GRASS || surface == SURFACE_METAL || walk_collision(vault, material, WALK).1 == -1 {
        return None;
    }
    Some(walk_volume(vault, material))
}

/// `FootStepUtils::GetSpliceInfoForFootStep` 0x824f5588: `(bank, splice)`. Grass and metal
/// surfaces read `types` (collisions and metal banks); others play the material's walk collision
/// as the primary sound and have no secondary.
fn splice_info(vault: &Vault, speed: usize, contact: usize, sound: usize, material: u32) -> Option<(i32, i32)> {
    let surface = foot_step_material(vault, material);
    let (bank, splice) = if surface == SURFACE_GRASS || surface == SURFACE_METAL {
        let (bank, metal) = if surface == SURFACE_GRASS { (SPLICE_COLLISIONS, 0) } else { (SPLICE_METAL, 1) };
        (bank, TYPES[((speed * 2 + contact) * 2 + sound) * 2 + metal])
    } else if sound != PRIMARY {
        return None;
    } else {
        walk_collision(vault, material, speed)
    };
    (splice != -1).then_some((bank, splice))
}

/// `FootStepUtils::GetFootPushSpliceInfoForFootStep` 0x824f5510: the `spliceTypeMap` entry for
/// the terrain, surface and sound (a surface outside 1..=7 indexes outside the table natively).
fn foot_push_splice_info(vault: &Vault, material: u32, terrain: u32, sound: usize) -> Option<(i32, i32)> {
    let row = if matches!(terrain, 1 | 3 | 5) { 0 } else { 1 };
    let index = usize::try_from(4 * (7 * row + i64::from(foot_step_material(vault, material))) + sound as i64 - 4).ok()?;
    let (&bank, &splice) = (SPLICE_TYPE_MAP.get(index)?, SPLICE_TYPE_MAP.get(index + 2)?);
    (splice != -1).then_some((bank, splice))
}

/// `FootStepUtils::GetSpliceTuningKey` 0x824f5268: the `aud_cluster` collection of the first
/// tuning entry naming `splice` (32 bytes: the splice, then an `aud_cluster` ref spec whose
/// collection key is at byte 16), else "default".
fn splice_tuning_key(vault: &Vault, splice: i32, bank: i32) -> u64 {
    let table = if bank == SPLICE_COLLISIONS { COLLISION_TUNING } else { TUNING };
    for index in 0..vault.length(FOOTSTEP, table) {
        let entry = vault.bytes(FOOTSTEP, table, index, 0x20);
        if i32::from_be_bytes([entry[0], entry[1], entry[2], entry[3]]) == splice {
            return u64::from_be_bytes(entry[16..24].try_into().expect("eight bytes"));
        }
    }
    DEFAULT_KEY
}

/// `Attrib::Gen::aud_cluster::Change`: a found collection's layout replaces the instance's; a
/// missing one keeps the previous layout (`Attrib::Instance::Change` leaves `mLayoutPtr`).
fn change_cluster(cluster: &mut [f32; 6], vault: &Vault, key: u64) {
    let collection = Collection::new(AUD_CLUSTER, key);
    if vault.has_collection(collection) {
        *cluster = std::array::from_fn(|word| vault.layout_f32(collection, word as u32));
    }
}

/// `FootStepUtils::GetFootStepClothSplice` 0x824f5c70.
fn foot_step_cloth_splice(vault: &Vault, speed: f32) -> i32 {
    // `ble` and `bgt` after `fcmpu`: an unordered speed reaches the walk splice.
    if speed > vault.f32_at(CLOTHING, CLOTH_SPEEDS, 0) {
        CLOTH_SPRINT
    } else if speed > vault.f32_at(CLOTHING, CLOTH_SPEEDS, 1) {
        CLOTH_RUN
    } else {
        CLOTH_WALK
    }
}

/// `Sk8::PointGraphEval::Evaluate(16, x, y, value)` over a 0x90-byte footstep graph (x at byte
/// 0x10, y at byte 0x50), truncated.
fn speed_graph(vault: &Vault, key: u64, value: f32) -> i32 {
    let bytes = vault.bytes(FOOTSTEP, key, 0, 0x90);
    let at = |offset: usize| f32::from_be_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]);
    let x: [f32; 16] = std::array::from_fn(|i| at(0x10 + 4 * i));
    let y: [f32; 16] = std::array::from_fn(|i| at(0x50 + 4 * i));
    fctiwz(point_graph(&x, &y, value))
}

/// `new Splicer::Splice(bank, index, master)` and `Play` at zero volume.
fn play(ctx: &mut SfxCtx<'_>, bank: i32, index: i32, delta: f32, spread: f32) -> Option<SpliceId> {
    let master = ctx.master_sub_mix;
    let splice = ctx.splicer.create_splice(ctx.core, usize::from(bank as i8 as u8), index, master)?;
    let update = SpliceUpdateParams { vol: 0.0, pitch: 1.0, az: 0.0, delta, spread, time_stretch: 1.0 };
    ctx.splicer.play_splice(ctx.core, splice, SplicePlayParams::default(), &update);
    Some(splice)
}

/// The `Splice::Update` of `UpdateCloth`, `UpdateRattle` and `UpdateJumps`: volume output
/// `volume`, pitch output 1, azimuth output 0 and the state's delta time.
fn mixer_update(io: Option<DMixIoId>, volume: i32, ctx: &SfxCtx<'_>) -> SpliceUpdateParams {
    SpliceUpdateParams {
        vol: ctx.output(io, volume, PresetType::Volume) as f32 * VOLUME_SCALE,
        pitch: ctx.output(io, 1, PresetType::Pitch) as f32 * PITCH_SCALE,
        az: ctx.output(io, 0, PresetType::Azimuth) as f32 * AZIMUTH_SCALE,
        delta: ctx.delta_time,
        spread: if ctx.local { 1.0 } else { 0.0 },
        time_stretch: 1.0,
    }
}

/// Updates a playing splice and deletes a finished one.
fn update_or_release(io: Option<DMixIoId>, splice: &mut Option<SpliceId>, volume: i32, ctx: &mut SfxCtx<'_>) {
    let Some(id) = *splice else { return };
    if !ctx.is_playing(id) {
        ctx.release(splice);
        return;
    }
    let update = mixer_update(io, volume, ctx);
    ctx.update_splice(id, update);
}

/// `tSplicerFootStep` (40 bytes): a foot's splice and the "FootStep SubMix" voice it plays into.
#[derive(Debug, Clone, Copy)]
struct SplicerFootStep {
    splice: Option<SpliceId>,
    hipass_freq: f32,
    lowpass_freq: f32,
    notch_freq: f32,
    notch_gain: f32,
    notch_q: f32,
    volume_factor: f32,
    output_channels: u8,
    use_random_eq: bool,
    /// `pVoice` (its plug-ins are `pVoicePlugInPtrs`).
    voice: Option<VoiceId>,
}

impl Default for SplicerFootStep {
    /// The `tFootStep` constructor's values 0x82516570.
    fn default() -> Self {
        Self {
            splice: None,
            hipass_freq: 0.0,
            lowpass_freq: 96000.0,
            notch_freq: 96000.0,
            notch_gain: 1.0,
            notch_q: 3.0,
            volume_factor: 1.0,
            output_channels: 6,
            use_random_eq: false,
            voice: None,
        }
    }
}

impl SplicerFootStep {
    /// `FootStepUtils::CreateFootStepSubmixVoice` 0x824f5718: stage 4, `SubMix` "FootStep SubMix",
    /// `HighPassIir2`, `LowPassIir2`, three `PeakingIir2`s and a `Send` in one channel, then
    /// `Pan2D1` (the default angles, norm mode 0) and a `Send` in `output_channels`. Send 6 goes
    /// to `GlobalEffects`' environment send (not ported), send 8 to the master mix.
    fn create_voice(&mut self, ctx: &mut SfxCtx<'_>) {
        let config = |kind, output_channels| PlugInConfig { kind, output_channels };
        let (front, rear, _) = DEFAULT_CONSTRUCTOR;
        let channels = self.output_channels;
        self.voice = ctx.core.create_voice(
            4,
            "FootStep SubMix",
            &[
                config(PlugInKind::SubMix { name: "FootStep SubMix".into() }, 1),
                config(PlugInKind::HighPassIir2, 1),
                config(PlugInKind::LowPassIir2, 1),
                config(PlugInKind::PeakingIir2, 1),
                config(PlugInKind::PeakingIir2, 1),
                config(PlugInKind::PeakingIir2, 1),
                config(PlugInKind::Send, 1),
                config(PlugInKind::Pan2D1 { constructor: Some((front, rear, 0.0)) }, channels),
                config(PlugInKind::Send, channels),
            ],
        );
        if let Some(voice) = self.voice {
            ctx.core.connect_send(PluginRef { voice, index: 6 }, Some(ctx.global_effects.reverb_sub_mix()));
            ctx.core.connect_send(PluginRef { voice, index: 8 }, Some(ctx.master_sub_mix));
        }
    }

    /// The `aud_cluster` layout's filters and `volume`.
    fn tune(&mut self, cluster: &[f32; 6], volume: f32) {
        self.hipass_freq = cluster[5];
        self.lowpass_freq = cluster[4];
        self.notch_freq = cluster[3];
        self.notch_gain = cluster[2];
        self.notch_q = cluster[1];
        self.volume_factor = volume;
    }

    /// `new Splicer::Splice(bank, splice, voice plug-in 0)` and `Play`.
    fn play(&mut self, bank: i32, splice: i32, ctx: &mut SfxCtx<'_>) {
        self.splice = self.voice.and_then(|voice| ctx.new_splice(bank, splice, PluginRef { voice, index: 0 }));
    }
}

/// `tFootStep` (240 bytes).
struct FootStep {
    /// `mpFootStep`.
    foot_step: Option<ClassObject>,
    notch_gain: [f32; 2],
    notch_gain_min: [f32; 2],
    notch_gain_max: [f32; 2],
    notch_freq: [f32; 2],
    notch_freq_min: [f32; 2],
    notch_freq_max: [f32; 2],
    notch_q: [f32; 2],
    wet_level: f32,
    azimuth: i32,
    /// Heel and toe (the toe stays false).
    is_touching: [bool; 2],
    was_touching: [bool; 2],
    audio_material: u32,
    collision_splice: [SplicerFootStep; 2],
    footpush_splice: [SplicerFootStep; 2],
}

impl FootStep {
    /// The `tFootStep` constructor 0x82516570 and the `SFXObj_OffBoard` constructor's footstep
    /// reads, which replace its notch ranges and Q with the integer attributes.
    fn new(vault: &Vault) -> Self {
        let read = |key: u64| [0, 1].map(|index| vault.i32_at(FOOTSTEP, key, index) as f32);
        Self {
            foot_step: None,
            notch_gain: [1.0; 2],
            notch_gain_min: read(NOTCH_GAIN_MIN),
            notch_gain_max: read(NOTCH_GAIN_MAX),
            notch_freq: [96000.0; 2],
            notch_freq_min: read(NOTCH_FREQ_MIN),
            notch_freq_max: read(NOTCH_FREQ_MAX),
            notch_q: read(NOTCH_Q),
            wet_level: 0.0,
            azimuth: 0,
            is_touching: [false; 2],
            was_touching: [false; 2],
            audio_material: MATERIAL_CONCRETE_POLISHED,
            collision_splice: [SplicerFootStep::default(); 2],
            footpush_splice: [SplicerFootStep::default(); 2],
        }
    }

    /// The heel touched down this update.
    fn new_step(&self) -> bool {
        !self.was_touching[0] && self.is_touching[0]
    }

    /// `collisionSplice` then `footpushSplice`.
    fn splicers_mut(&mut self) -> [&mut SplicerFootStep; 4] {
        let [collision0, collision1] = &mut self.collision_splice;
        let [footpush0, footpush1] = &mut self.footpush_splice;
        [collision0, collision1, footpush0, footpush1]
    }

    /// `FootStepUtils::AdjustFilterParameters` 0x824e87e0: each notch frequency and gain becomes
    /// `rw::math::Random() % trunc(max - min) + min`, drawn as frequency 0, gain 0, frequency 1,
    /// gain 1. A zero range traps natively; here it gives the minimum.
    fn adjust_filter_parameters(&mut self, rng: &mut RwRandom) {
        let mut random = |min: f32, max: f32| rng.next_u32().checked_rem(fctiwz(max - min) as u32).unwrap_or(0) as f32 + min;
        for index in 0..2 {
            self.notch_freq[index] = random(self.notch_freq_min[index], self.notch_freq_max[index]);
            self.notch_gain[index] = random(self.notch_gain_min[index], self.notch_gain_max[index]);
        }
    }

    /// `FootStepUtils::SetupSpliceVoices` 0x824f5d88: the missing voices; the foot-push splices
    /// take the random EQ.
    fn setup_splice_voices(&mut self, ctx: &mut SfxCtx<'_>) {
        for (index, splicer) in self.splicers_mut().into_iter().enumerate() {
            if splicer.voice.is_none() {
                splicer.create_voice(ctx);
                splicer.use_random_eq = index >= 2;
            }
        }
    }

    /// `FootStepUtils::CleanUpSplices` 0x824e8940.
    fn clean_up_splices(&mut self, ctx: &mut SfxCtx<'_>) {
        for splicer in self.splicers_mut() {
            ctx.release(&mut splicer.splice);
        }
    }

    /// `FootStepUtils::CleanUpSpliceVoices` 0x824f5e20 (`AudioSystem::FreeSubmixVoice`).
    fn clean_up_splice_voices(&mut self, ctx: &mut SfxCtx<'_>) {
        for splicer in self.splicers_mut() {
            if let Some(voice) = splicer.voice.take() {
                ctx.core.release_voice(voice);
            }
        }
    }

    /// `FootStepUtils::UpdateSplicerVoice` 0x824f5a38: the splice's filters, the random notches
    /// when it takes them, the foot's wet level and azimuth.
    fn update_splicer_voice(&self, splicer: &SplicerFootStep, core: &mut AudioCore) {
        let Some(voice) = splicer.voice else { return };
        let plugin = |index: u8| PluginRef { voice, index };
        core.set_attribute(plugin(1), 0, splicer.hipass_freq);
        core.set_attribute(plugin(2), 0, splicer.lowpass_freq);
        core.set_attribute(plugin(3), 0, splicer.notch_freq);
        core.set_attribute(plugin(3), 1, splicer.notch_gain);
        core.set_attribute(plugin(3), 2, splicer.notch_q);
        if splicer.use_random_eq {
            for (index, peaking) in [(0, 4), (1, 5)] {
                core.set_attribute(plugin(peaking), 0, self.notch_freq[index]);
                core.set_attribute(plugin(peaking), 1, self.notch_gain[index] * VOLUME_SCALE);
                core.set_attribute(plugin(peaking), 2, self.notch_q[index]);
            }
        }
        core.set_attribute(plugin(6), 0, self.wet_level);
        core.set_attribute(plugin(7), 0, self.azimuth as f32 * VOICE_AZIMUTH_SCALE);
    }

    /// `FootStepUtils::UpdateFootSplices` 0x824e89e0: collision then foot-push splice of each
    /// index at its volume factor times `vol`; a finished one is deleted. `Splice::Update` scales
    /// the shared parameters' pitch in place, so it compounds from splice to splice.
    fn update_foot_splices(&mut self, update: &mut SpliceUpdateParams, vol: f32, ctx: &mut SfxCtx<'_>) {
        for index in 0..2 {
            for splicer in [&mut self.collision_splice[index], &mut self.footpush_splice[index]] {
                let Some(id) = splicer.splice else { continue };
                if !ctx.is_playing(id) {
                    ctx.release(&mut splicer.splice);
                    continue;
                }
                update.vol = splicer.volume_factor * vol;
                ctx.splicer.update_splice(ctx.core, id, update);
            }
        }
    }

    /// One step of the local skater in `CheckForFootSteps`: the heel's primary and secondary
    /// splices with their `aud_cluster` tuning (the primary's volume factor is the walk volume of
    /// `volume_material` when it has one), then the primary and secondary foot pushes.
    fn play_step(&mut self, speed: usize, volume_material: u32, terrain: u32, cluster: &mut [f32; 6], ctx: &mut SfxCtx<'_>) {
        let vault = ctx.vault;
        self.clean_up_splices(ctx);
        self.play_heel(speed, volume_material, cluster, ctx);
        for sound in [PRIMARY, SECONDARY] {
            if let Some((bank, splice)) = foot_push_splice_info(vault, self.audio_material, terrain, sound) {
                self.update_splicer_voice(&self.footpush_splice[sound], ctx.core);
                self.footpush_splice[sound].play(bank, splice, ctx);
            }
        }
    }

    fn play_heel(&mut self, speed: usize, volume_material: u32, cluster: &mut [f32; 6], ctx: &mut SfxCtx<'_>) {
        let vault = ctx.vault;
        for sound in [PRIMARY, SECONDARY] {
            let Some((bank, splice)) = splice_info(vault, speed, HEEL, sound, self.audio_material) else { return };
            change_cluster(cluster, vault, splice_tuning_key(vault, splice, bank));
            let volume = match sound {
                PRIMARY => material_volume(vault, volume_material).unwrap_or(cluster[0]),
                _ => cluster[0],
            };
            self.collision_splice[sound].tune(cluster, volume);
            self.update_splicer_voice(&self.collision_splice[sound], ctx.core);
            self.collision_splice[sound].play(bank, splice, ctx);
        }
    }
}

/// `new Csis::Playercharacter_footstep` as `CheckForFootSteps` creates it (constructor
/// 0x8250fe58).
fn new_foot_step_object(q: [i32; 2], ctx: &mut SfxCtx<'_>) -> ClassObject {
    let mut data = [0; 35];
    data[PITCH] = 0x1000;
    data[LOW_PASS] = 25000;
    data[DRY_FX] = 0x7FFF;
    data[JUMP_LANDED] = 1;
    data[FOOT_SURFACE] = 1;
    data[TERRAIN] = 1;
    data[LOCAL_SKATER] = i32::from(ctx.local);
    data[GAIN_0] = 1;
    data[FREQ_0] = 24000;
    data[Q_0] = q[0];
    data[GAIN_1] = 1;
    data[FREQ_1] = 24000;
    data[Q_1] = q[1];
    let data = data.iter().zip(RANGES).map(|(&value, (min, max))| value.clamp(min, max)).collect();
    ctx.aems.create_object("playercharacter_footstep", data)
}

pub struct OffBoard {
    /// `m_pDMixIO` (objectID 0x400100a0 + the state's instance).
    pub io: Option<DMixIoId>,
    /// `mRightFoot`, `mLeftFoot`.
    feet: [FootStep; 2],
    was_jump: bool,
    aems_speed: i32,
    /// `mRightLinearAemsSpeed`, `mLeftLinearAemsSpeed`.
    linear_aems_speed: [i32; 2],
    /// `mRightAngularAemsSpeed`, `mLeftAngularAemsSpeed`.
    angular_aems_speed: [i32; 2],
    /// `mpRightFootCloth`, `mpLeftFootCloth`.
    foot_cloth: [Option<SpliceId>; 2],
    /// `mpRightFootRattle`, `mpLeftFootRattle`.
    foot_rattle: [Option<SpliceId>; 2],
    cloth_jump_up: Option<SpliceId>,
    cloth_jump_down: Option<SpliceId>,
    was_hippy: bool,
    last_direction: u32,
    was_foot_planting: bool,
    /// `mRightFootPlantJumpTimer`, `mLeftFootPlantJumpTimer`.
    foot_plant_jump_timer: [i32; 2],
    hippy_jump_strength: u32,
    just_jumped: bool,
}

impl OffBoard {
    /// The constructor 0x82537038.
    #[must_use]
    pub fn new(vault: &Vault) -> Self {
        Self {
            io: None,
            feet: [FootStep::new(vault), FootStep::new(vault)],
            was_jump: false,
            aems_speed: 0,
            linear_aems_speed: [0; 2],
            angular_aems_speed: [0; 2],
            foot_cloth: [None; 2],
            foot_rattle: [None; 2],
            cloth_jump_up: None,
            cloth_jump_down: None,
            was_hippy: false,
            last_direction: DIRECTIONS_MAX,
            was_foot_planting: false,
            foot_plant_jump_timer: [0; 2],
            hippy_jump_strength: SHORT_HIPPY,
            just_jumped: false,
        }
    }

    /// `InitSFX` (`SFXObjBase` 0x82515d80): the mixer outputs on.
    pub fn init_sfx(&mut self, _physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if let Some(io) = self.io {
            ctx.map.set_output_enabled(io, true);
        }
    }

    /// `UpdateParams` 0x82549910.
    pub fn update_params(&mut self, t: f32, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let offboard = physics.is_off_board;
        let foot_planting = physics.is_foot_planting;
        let just_left_plant = !foot_planting && self.was_foot_planting;
        ctx.set_input(self.io, 0, if offboard { 0x7FFF } else { 0 });
        let vault = ctx.vault;
        self.aems_speed = speed_graph(vault, SPEED_GRAPH, physics.player_speed);
        self.linear_aems_speed[LEFT] = speed_graph(vault, LINEAR_GRAPH, physics.left_foot_slide_world_speed);
        self.linear_aems_speed[RIGHT] = speed_graph(vault, LINEAR_GRAPH, physics.right_foot_slide_world_speed);
        self.angular_aems_speed[LEFT] = speed_graph(vault, ANGULAR_GRAPH, physics.left_foot_angular_y_world_speed);
        self.angular_aems_speed[RIGHT] = speed_graph(vault, ANGULAR_GRAPH, physics.right_foot_angular_y_world_speed);
        self.feet[RIGHT].is_touching = [physics.right_foot_touching_off_board, false];
        self.feet[LEFT].is_touching = [physics.left_foot_touching_off_board, false];
        if just_left_plant {
            if self.feet[RIGHT].was_touching[0] {
                self.foot_plant_jump_timer[RIGHT] = FOOT_PLANT_JUMP_FRAMES;
            } else if self.feet[LEFT].was_touching[0] {
                self.foot_plant_jump_timer[LEFT] = FOOT_PLANT_JUMP_FRAMES;
            }
        }
        for timer in &mut self.foot_plant_jump_timer {
            if *timer > 0 {
                *timer -= 1;
            }
        }
        self.check_for_foot_steps(physics, ctx);
        self.check_for_cloth(t, offboard, physics, ctx);
        self.check_for_jumps(t, offboard, just_left_plant, physics, ctx);
        self.was_foot_planting = foot_planting;
        for foot in &mut self.feet {
            foot.was_touching = foot.is_touching;
        }
    }

    /// `CheckForFootSteps` 0x82538400: the feet's surfaces and AEMS instances, the local skater's
    /// voices, then each new step's random notches and (local) splices, right foot first.
    fn check_for_foot_steps(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !ctx.cvars.enable_pc_foot_steps {
            return;
        }
        let new = [self.feet[RIGHT].new_step(), self.feet[LEFT].new_step()];
        let surface = |material: u32| if material == MATERIAL_MAX { MATERIAL_CONCRETE_ROUGH } else { material };
        self.feet[RIGHT].audio_material = surface(physics.right_foot_off_board_surface);
        self.feet[LEFT].audio_material = surface(physics.left_foot_off_board_surface);
        if ctx.cvars.foot_step_override {
            self.feet[LEFT].audio_material = ctx.cvars.foot_step_override_type;
            self.feet[RIGHT].audio_material = ctx.cvars.foot_step_override_type;
        }
        if self.feet[LEFT].foot_step.is_none() || self.feet[RIGHT].foot_step.is_none() {
            let q = [0, 1].map(|index| ctx.vault.i32_at(FOOTSTEP, NOTCH_Q, index));
            for side in [LEFT, RIGHT] {
                if self.feet[side].foot_step.is_none() {
                    self.feet[side].foot_step = Some(new_foot_step_object(q, ctx));
                }
            }
        }
        if ctx.local {
            self.feet[RIGHT].setup_splice_voices(ctx);
            self.feet[LEFT].setup_splice_voices(ctx);
        }
        let speed = if self.aems_speed > ctx.vault.i32(FOOTSTEP, RUN_THRESHOLD) { RUN } else { WALK };
        // The `aud_cluster` instance: `DefaultDataArea` until a `Change` finds a collection.
        let mut cluster = [0.0; 6];
        // The left foot's primary volume factor reads `mRightFoot.audioMaterial`.
        let volume_material = self.feet[RIGHT].audio_material;
        for side in [RIGHT, LEFT] {
            if !new[side] {
                continue;
            }
            trace_foot_step(format_args!("foot step side {side} off board {} foot planting {} material {}",
                physics.is_off_board, physics.is_foot_planting, self.feet[side].audio_material));
            self.feet[side].adjust_filter_parameters(ctx.rng);
            if ctx.local {
                self.feet[side].play_step(speed, volume_material, physics.terrain, &mut cluster, ctx);
            }
        }
    }

    /// `CheckForCloth` 0x825380e8: off the board each new step replaces that foot's cloth splice
    /// and, while the local skater carries the board, its rattle.
    fn check_for_cloth(&mut self, t: f32, offboard: bool, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !offboard {
            return;
        }
        let new = [self.feet[RIGHT].new_step(), self.feet[LEFT].new_step()];
        let rattle = ctx.local && physics.is_off_board && physics.is_holding_board;
        for side in [RIGHT, LEFT] {
            if !new[side] {
                continue;
            }
            ctx.release(&mut self.foot_cloth[side]);
            let cloth = foot_step_cloth_splice(ctx.vault, physics.player_speed);
            self.foot_cloth[side] = play(ctx, SPLICE_CLOTH, cloth, t, 1.0);
            if rattle {
                ctx.release(&mut self.foot_rattle[side]);
                let index = match cloth {
                    CLOTH_SPRINT => 5,
                    CLOTH_RUN => 4,
                    _ => 3,
                };
                self.foot_rattle[side] = play(ctx, SPLICE_RATTLE, index, t, 1.0);
            }
        }
    }

    /// `CheckForJumps` 0x82537af8: an off-board jump, a hippy jump or leaving a foot plant
    /// replaces the jump-up cloth; starting to fall while it exists plays the jump-down cloth.
    fn check_for_jumps(&mut self, t: f32, offboard: bool, just_left_plant: bool, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let jump_start = offboard && physics.is_jump && !self.was_jump;
        let (direction, falling) =
            if !(physics.velocity[1] <= 0.0) { (GOING_UP, false) } else { (GOING_DOWN, self.last_direction == GOING_UP) };
        self.last_direction = direction;
        let start_down = self.cloth_jump_up.is_some() && falling;
        let hippy_start = !self.was_hippy && physics.is_hippy_jump;
        let spread = if ctx.local { 1.0 } else { 0.0 };
        if jump_start || hippy_start || just_left_plant {
            ctx.release(&mut self.cloth_jump_up);
            self.hippy_jump_strength = physics.hippy_jump_strength;
            let index = match self.hippy_jump_strength {
                _ if jump_start => 0x18,
                NORMAL_HIPPY => 0x1C,
                HIGH_HIPPY => 0x1E,
                _ => 0x1A,
            };
            self.cloth_jump_up = play(ctx, SPLICE_CLOTH, index, t, spread);
            self.just_jumped = jump_start;
        }
        if start_down {
            ctx.release(&mut self.cloth_jump_up);
            ctx.release(&mut self.cloth_jump_down);
            let index = match self.hippy_jump_strength {
                _ if self.just_jumped => 0x19,
                NORMAL_HIPPY => 0x1D,
                HIGH_HIPPY => 0x1F,
                _ => 0x1B,
            };
            self.cloth_jump_down = play(ctx, SPLICE_CLOTH, index, t, spread);
        }
        self.was_hippy = physics.is_hippy_jump;
        self.was_jump = physics.is_jump;
    }

    /// `ProcessUpdate` 0x82549d20.
    pub fn process_update(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        self.update_foot_steps(physics, ctx);
        self.update_cloth(ctx);
        self.update_rattle(ctx);
        self.update_jumps(ctx);
    }

    /// `UpdateCloth` 0x82538eb0: volume output 3, right foot first.
    fn update_cloth(&mut self, ctx: &mut SfxCtx<'_>) {
        for side in [RIGHT, LEFT] {
            update_or_release(self.io, &mut self.foot_cloth[side], 3, ctx);
        }
    }

    /// `UpdateRattle` 0x82539168: volume output 12, right foot first.
    fn update_rattle(&mut self, ctx: &mut SfxCtx<'_>) {
        for side in [RIGHT, LEFT] {
            update_or_release(self.io, &mut self.foot_rattle[side], 12, ctx);
        }
    }

    /// `UpdateFootSteps` 0x82539420: both feet's `playercharacter_footstep` members, then the
    /// local skater's splices (volume output 7, or 11 when the right foot's material has a walk
    /// volume; wet output 8).
    fn update_foot_steps(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let io = self.io;
        if self.feet[LEFT].foot_step.is_some() && self.feet[RIGHT].foot_step.is_some() {
            let shoe = if ctx.cvars.foot_step_shoe_type == 0 { 1 } else { ctx.cvars.foot_step_shoe_type };
            let terrain = physics.terrain as i32;
            if ctx.cvars.foot_step_override {
                self.feet[LEFT].audio_material = ctx.cvars.foot_step_override_type;
                self.feet[RIGHT].audio_material = ctx.cvars.foot_step_override_type;
            }
            for side in [RIGHT, LEFT] {
                let jump = self.foot_plant_jump_timer[side] > 0 || self.cloth_jump_up.is_some();
                let foot = &mut self.feet[side];
                let Some(object) = foot.foot_step.as_mut() else { continue };
                let data = &mut object.data;
                set(data, VOLUME, 0x7FFF);
                set(data, AZIMUTH, ctx.output(io, 0, PresetType::Azimuth));
                set(data, PITCH, ctx.output(io, 1, PresetType::Pitch));
                set(data, LOW_PASS, ctx.output(io, 4, PresetType::Frequency));
                set(data, HIGH_PASS, ctx.output(io, 5, PresetType::Frequency));
                set(data, WET_FX, ctx.output(io, 6, PresetType::Volume));
                set(data, DRY_FX, ctx.output(io, 2, PresetType::Volume));
                set(data, FOOT_HEEL, i32::from(foot.is_touching[0]));
                set(data, FOOT_TOE, i32::from(foot.is_touching[1]));
                set(data, FOOT_ANGLE_VEL, self.angular_aems_speed[side]);
                set(data, FOOT_SLIDE_VEL, self.linear_aems_speed[side]);
                set(data, INITIATE_JUMP, i32::from(jump));
                set(data, JUMP_LANDED, physics.off_board_landing as i32);
                set(data, PLAYER_SPEED, self.aems_speed);
                set(data, SHOE_TYPE, shoe);
                set(data, FOOT_SURFACE, foot_step_material(ctx.vault, foot.audio_material) as i32);
                set(data, TERRAIN, terrain);
                for index in 0..6 {
                    set(data, STEP_VOL + index, ctx.vault.i32_at(FOOTSTEP, VOLUMES, index as u32));
                }
                set(data, LOCAL_SKATER, i32::from(ctx.local));
                set(data, GAIN_0, fctiwz(foot.notch_gain[0]));
                set(data, FREQ_0, fctiwz(foot.notch_freq[0]));
                set(data, GAIN_1, fctiwz(foot.notch_gain[1]));
                set(data, FREQ_1, fctiwz(foot.notch_freq[1]));
                ctx.aems.set_member_data(object);
            }
        }
        if !ctx.local {
            return;
        }
        let output = if material_volume(ctx.vault, self.feet[RIGHT].audio_material).is_some() { 11 } else { 7 };
        let vol = ctx.output(io, output, PresetType::Volume) as f32 * VOLUME_SCALE;
        let pitch = ctx.output(io, 1, PresetType::Pitch) as f32 * PITCH_SCALE;
        for foot in &mut self.feet {
            foot.azimuth = ctx.output(io, 0, PresetType::Azimuth);
            foot.wet_level = ctx.output(io, 8, PresetType::Volume) as f32 * VOLUME_SCALE;
        }
        let mut update = SpliceUpdateParams { vol: 0.0, pitch, az: 0.0, delta: ctx.delta_time, spread: 1.0, time_stretch: 1.0 };
        self.feet[RIGHT].update_foot_splices(&mut update, vol, ctx);
        self.feet[LEFT].update_foot_splices(&mut update, vol, ctx);
    }

    /// `UpdateJumps` 0x82537e48: the jump-up cloth only updates while playing (it stays allocated
    /// until replaced); the jump-down cloth is deleted once finished.
    fn update_jumps(&mut self, ctx: &mut SfxCtx<'_>) {
        if let Some(id) = self.cloth_jump_up {
            if ctx.is_playing(id) {
                let update = mixer_update(self.io, 9, ctx);
                ctx.update_splice(id, update);
            }
        }
        update_or_release(self.io, &mut self.cloth_jump_down, 10, ctx);
    }

    /// `Destroy` 0x825379a0.
    pub fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        for side in [RIGHT, LEFT] {
            if let Some(object) = self.feet[side].foot_step.take() {
                ctx.aems.release_object(object);
            }
        }
        ctx.release(&mut self.foot_cloth[LEFT]);
        ctx.release(&mut self.foot_cloth[RIGHT]);
        ctx.release(&mut self.foot_rattle[LEFT]);
        ctx.release(&mut self.foot_rattle[RIGHT]);
        ctx.release(&mut self.cloth_jump_up);
        ctx.release(&mut self.cloth_jump_down);
        for side in [RIGHT, LEFT] {
            self.feet[side].clean_up_splices(ctx);
            self.feet[side].clean_up_splice_voices(ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::AttribSource;

    const MAPPING: u64 = 0xC489_459A_0C07_D154;
    const MAPPING_KEY: u64 = 0x4CA6_0755_8B1C_F440;
    const CLUSTER_A: u64 = 0x1111;
    const CLUSTER_B: u64 = 0x2222;

    /// Footstep surface = material for materials 1..=7, two tuning tables, and two `aud_cluster`
    /// collections whose layout words are `key + word`.
    struct Footsteps;

    fn tuning_entry(splice: i32, key: u64) -> Vec<u8> {
        let mut entry = splice.to_be_bytes().to_vec();
        entry.resize(16, 0);
        entry.extend(key.to_be_bytes());
        entry.resize(0x20, 0);
        entry
    }

    impl AttribSource for Footsteps {
        fn attribute(&self, _class: u64, collection: u64, key: u64, index: u32) -> Option<Vec<u8>> {
            match (collection, key, index) {
                (MAPPING, MAPPING_KEY, material) => {
                    let mut record = vec![0; 48];
                    record[24..28].copy_from_slice(&material.to_be_bytes());
                    Some(record)
                }
                (_, COLLISION_TUNING, 0) => Some(tuning_entry(40, CLUSTER_B)),
                (_, COLLISION_TUNING, 1) => Some(tuning_entry(41, CLUSTER_A)),
                (_, COLLISION_TUNING, 2) => Some(tuning_entry(41, CLUSTER_B)),
                (_, TUNING, 0) => Some(tuning_entry(41, CLUSTER_B)),
                _ => None,
            }
        }

        fn layout_word(&self, _class: u64, collection: u64, index: u32) -> u32 {
            (collection as u32) + index
        }

        fn has_collection(&self, class: u64, collection: u64) -> bool {
            class == AUD_CLUSTER && (collection == CLUSTER_A || collection == CLUSTER_B)
        }

        fn length(&self, _class: u64, _collection: u64, key: u64) -> u32 {
            match key {
                COLLISION_TUNING => 3,
                TUNING => 1,
                _ => 0,
            }
        }
    }

    #[test]
    fn tuning_keys_take_the_first_match_of_the_bank_table() {
        let vault = Vault::new(Some(Box::new(Footsteps)));
        assert_eq!(splice_tuning_key(&vault, 41, SPLICE_COLLISIONS), CLUSTER_A);
        assert_eq!(splice_tuning_key(&vault, 41, SPLICE_METAL), CLUSTER_B);
        assert_eq!(splice_tuning_key(&vault, 40, SPLICE_METAL), DEFAULT_KEY);
        assert_eq!(splice_tuning_key(&vault, 7, SPLICE_COLLISIONS), DEFAULT_KEY);
    }

    #[test]
    fn a_missing_cluster_keeps_the_previous_layout() {
        let vault = Vault::new(Some(Box::new(Footsteps)));
        let mut cluster = [0.0; 6];
        change_cluster(&mut cluster, &vault, DEFAULT_KEY);
        assert_eq!(cluster, [0.0; 6]);
        change_cluster(&mut cluster, &vault, CLUSTER_A);
        let a = cluster;
        assert_eq!(a[5].to_bits(), 0x1111 + 5);
        change_cluster(&mut cluster, &vault, DEFAULT_KEY);
        assert_eq!(cluster, a);
    }

    #[test]
    fn grass_and_metal_read_the_types_table() {
        let vault = Vault::new(Some(Box::new(Footsteps)));
        assert_eq!(splice_info(&vault, WALK, HEEL, PRIMARY, SURFACE_GRASS), Some((SPLICE_COLLISIONS, 764)));
        assert_eq!(splice_info(&vault, WALK, HEEL, SECONDARY, SURFACE_METAL), Some((SPLICE_METAL, 440)));
        assert_eq!(splice_info(&vault, RUN, HEEL, PRIMARY, SURFACE_METAL), Some((SPLICE_METAL, 443)));
        assert_eq!(splice_info(&vault, WALK, 1, PRIMARY, SURFACE_GRASS), None);
        assert_eq!(splice_info(&vault, WALK, HEEL, SECONDARY, 2), None);
    }

    #[test]
    fn foot_pushes_follow_the_terrain_rows() {
        let vault = Vault::new(Some(Box::new(Footsteps)));
        assert_eq!(foot_push_splice_info(&vault, 1, 1, PRIMARY), Some((1, 11)));
        assert_eq!(foot_push_splice_info(&vault, 1, 2, PRIMARY), Some((1, 12)));
        assert_eq!(foot_push_splice_info(&vault, 1, 1, SECONDARY), None);
        assert_eq!(foot_push_splice_info(&vault, 3, 4, SECONDARY), Some((2, 13)));
        assert_eq!(foot_push_splice_info(&vault, 4, 5, PRIMARY), Some((5, 769)));
        assert_eq!(foot_push_splice_info(&vault, 0, 1, PRIMARY), None);
    }

    #[test]
    fn notches_stay_in_their_range() {
        let mut foot = FootStep::new(&Vault::default());
        foot.notch_freq_min = [100.0, 200.0];
        foot.notch_freq_max = [150.0, 200.0];
        foot.notch_gain_min = [1000.0, 0.5];
        foot.notch_gain_max = [9000.0, 3.0];
        let mut rng = RwRandom::new(7);
        for _ in 0..64 {
            foot.adjust_filter_parameters(&mut rng);
            assert!((100.0..150.0).contains(&foot.notch_freq[0]));
            assert_eq!(foot.notch_freq[1], 200.0);
            assert!((1000.0..9000.0).contains(&foot.notch_gain[0]));
            assert!(foot.notch_gain[1] == 0.5 || foot.notch_gain[1] == 1.5);
            assert_eq!(foot.notch_freq[0].fract(), 0.0);
        }
    }
}

/// `SKATE2_TRACE_SKATE3_DARKCATCH`: the foot steps, with what raised them.
fn trace_foot_step(message: std::fmt::Arguments<'_>) {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_DARKCATCH").is_some()) {
        eprintln!("skate 3 darkcatch audio: {message}");
    }
}
