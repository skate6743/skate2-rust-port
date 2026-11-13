//! `Sk8::Audio::SFXObj_Contacts` (488 bytes): pushes, ollies, landings, wheel hits, manuals,
//! foot catches and scrapes, hand grabs, kick-ups, air dismounts, board and body collisions.
//!
//! The speech calls (`CheckForImpactSpeech`, `PlayGrunt`, the speech of `CheckForCollisionSpeech`)
//! belong to a system that is not ported; the speech state the object keeps is. `CheckForScore`
//! makes a challenge's crowds cheer and gasp. The foot drag is a Csis class an AEMS module
//! plays, so its instance data is kept up to date.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use super::physics::{PlayerPhysics, COLLISIONS_DEFAULT as COLLISIONS, CONTACTS_DEFAULT as CONTACTS};
use super::skate3_mix::{follow, Skate3Splice};
use super::{CollisionInfo, SfxCtx, AZIMUTH_SCALE, PITCH_SCALE, VOLUME_SCALE};
use crate::collision::{self, LEVEL_L1, LEVEL_L3, LEVEL_L4, LEVEL_NUM};
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, PresetType};
use crate::presentation::{audio_material, SkaterData, MATERIAL_MAX};
use crate::rwac::{PluginRef, VoiceId};
use crate::splicer::{SpliceId, SpliceUpdateParams};
use crate::surface::{ollie_and_land_material, power_slide_material, stopping_material};
use crate::vault::{class, Collection, Vault};

/// `aud_rails` and `aud_speech` "default", `aud_splicer` "footcatch" and "handgrab",
/// `aud_general` "kickup" and "hom".
const RAILS: Collection = Collection::new(class::AUD_RAILS, 0xD7ED_BD36_2D7D_2152);
const SPEECH: Collection = Collection::new(class::AUD_SPEECH, 0xD7ED_BD36_2D7D_2152);
const FOOTCATCH: Collection = Collection::new(class::AUD_SPLICER, 0xF164_7BFB_782B_E97F);
const HANDGRAB: Collection = Collection::new(class::AUD_SPLICER, 0xE0C3_B44A_B44F_7B90);
const KICKUP: Collection = Collection::new(class::AUD_GENERAL, 0x109E_63A9_F198_B562);
/// TU3 `GlobalAttributeCollection` +0x8C: the effect sub mix each Skate 3 sound plays through.
pub(super) const SKATE3_EFFECTS: Collection = Collection::new(0x42AF_E160_E647_167C, 0xD7ED_BD36_2D7D_2152);
const HOM: Collection = Collection::new(class::AUD_GENERAL, 0x3404_9ABC_E23C_10AC);

/// `SPLICE_TYPE`.
const SPLICE_UNKNOWN: i32 = -1;
const SPLICE_LANDS_CONCRETE_HARD: i32 = 8;
const SPLICE_LANDS_WOOD_HARD: i32 = 9;
const SPLICE_LANDS_CONCRETE_SOFT: i32 = 10;
const SPLICE_LANDS_WOOD_SOFT: i32 = 11;
const SPLICE_OLLIES_CONCRETE: i32 = 12;
const SPLICE_OLLIES_WOOD: i32 = 13;
const SPLICE_OLLIES_SCRAPES: i32 = 14;
const SPLICE_FOOT_CATCH: i32 = 16;
/// PC port: the splicer slots of Skate 3's splice bank 0 (`Skate_Collisions.bnk`, which holds its
/// foot catches) and bank 7 (`sk8_foley.bnk`, its foot scrapes), loaded from the linked data on a
/// first darkcatch.
pub(super) const SPLICE_SKATE3_COLLISIONS: usize = 28;
const SPLICE_SKATE3_FOLEY: usize = 29;
const SPLICE_FOOT_SCRAPES: i32 = 17;
const SPLICE_GRABS: i32 = 19;
const SPLICE_L4: i32 = 20;
/// `eNumWheels`.
const WHEELS_FOUR: u32 = 0;
const WHEELS_TWO_FIRST: u32 = 1;
const WHEELS_TWO_LAST: u32 = 2;
const WHEELS_ONE: u32 = 3;
const WHEELS_TICK: u32 = 4;
const WHEELS_MAX: u32 = 5;
/// `eWheelHitLevel`.
const HIT_ONE: u32 = 0;
const HIT_TWO: u32 = 1;
const HIT_THREE: u32 = 2;
const HIT_MAX: u32 = 3;
/// `eOllieLandType`.
const LAND_CONCRETE_HARD: u32 = 0;
const LAND_CONCRETE_SOFT: u32 = 1;
const LAND_WOOD_HARD: u32 = 2;
const LAND_WOOD_SOFT: u32 = 3;
/// `eOLMaterial`.
const OL_CONCRETE: u32 = 0;
const OL_WOOD: u32 = 1;
/// `eSk8AudioTricks`.
const TRICK_INVALID: i32 = -1;
const TRICK_FOOT_PLANT: i32 = 31;
const TRICK_BACKSIDE_FOOT_PLANT: i32 = 32;
const TRICK_FS_BONELESS: i32 = 33;
const TRICK_NO_COMPLY: i32 = 34;
const TRICK_GRAB: i32 = 35;
const TRICK_COFFIN: i32 = 36;
/// `Sk8::Physics::eGrindType`.
const GRIND_BOARDSLIDE: i32 = 1;
const GRIND_TIPSLIDE: i32 = 2;
/// `eAudioMaterialTypes`.
const MATERIAL_METAL_SOLID_ROUND_1: u32 = 10;
const MATERIAL_BOARD: u32 = 93;
const MATERIAL_TRUCK: u32 = 94;
const MATERIAL_HEAD: u32 = 95;
const MATERIAL_TORSO: u32 = 96;
const MATERIAL_LEG: u32 = 97;
const MATERIAL_ARM: u32 = 98;
const MATERIAL_FOOT: u32 = 99;
const MATERIAL_HOM_HEAD: u32 = 100;
const MATERIAL_HOM_TORSO: u32 = 101;
const MATERIAL_HOM_LEG: u32 = 102;
const MATERIAL_HOM_ARM: u32 = 103;
const MATERIAL_HOM_FOOT: u32 = 104;
const MATERIAL_SKIN: u32 = 105;
const MATERIAL_DENIM: u32 = 106;
const MATERIAL_COTTON: u32 = 107;
const MATERIAL_BONE_CRACK: u32 = 108;
const MATERIAL_BONE_SNAP: u32 = 109;
const MATERIAL_FACE_HIT: u32 = 110;
const MATERIAL_BOARD_TUMBLE: u32 = 111;
/// `ePSMaterial`, `eFootDragSurface`.
const PS_GRASS: u32 = 3;
const PS_DIRT: u32 = 4;
const DRAG_RUFF_CNC: u32 = 0;
const DRAG_SMOOTH_CNC: u32 = 1;
/// `Splices::eSplice_hands_Type` and `eSK8_Foot_Catch_Type`.
const HANDS_RATTLE_GRAB: u32 = 0;
const HANDS_RELEASE: u32 = 1;
const HANDS_SOLID_GRAB: u32 = 2;
const CATCH_2ND_FOOT: u32 = 0;
const CATCH_BOTH_FEET: u32 = 1;
const CATCH_FIRST_FOOT: u32 = 2;
/// `tFootInfo::eFeet` and `tHandGrab::eHands`.
const FEET_RIGHT: usize = 0;
const FEET_LEFT: usize = 1;
const FEET_BOTH: usize = 2;
const HAND_LEFT: usize = 0;
const HAND_RIGHT: usize = 1;
/// `SKELETON_COLLISION_LEFTFOOT`: `CheckForBodyCollisions` hears the head, torso, arms and legs.
const SKELETON_COLLISION_LEFTFOOT: usize = 6;

/// `tWheelHitInfo`, and `tOllie` / `tOutofManual` with the fields they have: a splice playing into
/// its own `AudioSystem::CreateSplicerReverbVoice` voice.
#[derive(Debug, Clone, Copy)]
struct VoicedSplice {
    splice: Option<SpliceId>,
    num_wheels: u32,
    level: u32,
    /// `landtype` (`tOllie::material` for the ollie).
    land_type: u32,
    voice: Option<VoiceId>,
}

impl VoicedSplice {
    /// `tWheelHitInfo` as the constructor and every release leave it.
    const IDLE: Self = Self { splice: None, num_wheels: WHEELS_MAX, level: HIT_MAX, land_type: LAND_CONCRETE_HARD, voice: None };
    /// `tOllie` and `tOutofManual` as the constructor leaves them.
    const IDLE_OLLIE: Self = Self { splice: None, num_wheels: 0, level: 0, land_type: OL_CONCRETE, voice: None };
}

/// `tFootPush`.
#[derive(Debug, Clone, Copy, Default)]
struct FootPush {
    splice: Option<SpliceId>,
    surface: u32,
}

/// `tFootInfo`.
#[derive(Debug, Clone, Copy, Default)]
struct FootInfo {
    /// `bWasInAir` (the foot was on the deck).
    was_in_air: bool,
    time_in_air: f32,
}

/// `tHandGrab`.
#[derive(Debug, Clone, Copy, Default)]
struct HandGrab {
    splice: Option<SpliceId>,
    was_grabbing: bool,
}

/// `Csis::Class_foot_drag::mData`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FootDragData {
    pub speed: i32,
    pub volume: i32,
    pub dry: i32,
    pub wet: i32,
    pub low_pass: i32,
    pub high_pass: i32,
    pub azimuth: i32,
    pub pitch: i32,
    pub surface: i32,
    pub enter_volume: i32,
    pub exit_fast_volume: i32,
    pub exit_slow_volume: i32,
    pub loop_volume: i32,
    pub parts: i32,
}

/// `Sk8::Audio::SFXObj_Contacts`.
#[allow(clippy::struct_excessive_bools)]
pub struct Contacts {
    out_of_manual: VoicedSplice,
    ollie_l4: Option<SpliceId>,
    land_l4: Option<SpliceId>,
    ollie: VoicedSplice,
    push_on: FootPush,
    push_off: FootPush,
    scrape: Option<SpliceId>,
    foot_catch: [Option<SpliceId>; 3],
    /// TU3 `SFXObj_Contacts` +0x64: the catches with Skate 3 physics, each in a splice voice.
    skate3_foot_catch: [Option<Skate3Splice>; 3],
    foot_scrape: [Option<SpliceId>; 2],
    /// TU3 `SFXObj_Contacts` +0x1E4: a Skate 3 darkcatch holds the board inverted.
    skate3_inverted: bool,
    /// The skater runs Skate 3 physics, whose catches are TU3's.
    skate3_physics: bool,
    /// TU3 `SFXCTL_PlayerPhysics` +0x308/+0x304: a darkcatch now or within the last five frames.
    skate3_dark_catch_recent: bool,
    skate3_dark_catch_frames: u32,
    /// TU3 `SFXObj_Contacts` +0x1F0: the darkcatch landing, in a splice voice.
    skate3_dark_catch_land: Option<Skate3Splice>,
    /// PC port: the outputs TU3's darkcatch sounds read from its contacts mixer, here PLAYER
    /// object 13 (`skate3_mix::DARKCATCH_CHANNELS`) + the state's instance.
    pub skate3_io: Option<DMixIoId>,
    last_in_air: bool,
    last_in_manual: bool,
    last_in_grind: bool,
    last_was_pushing: bool,
    time_since_land_buffer: f32,
    /// `mpFootOrBoardDrag`.
    pub foot_drag: Option<FootDragData>,
    wheel_played: [bool; 4],
    l3_triggered: bool,
    land: [VoicedSplice; 4],
    sweetener: VoicedSplice,
    body_part_collision_counter: [f32; 8],
    board_collision_counter: f32,
    bone_crack_big: (f32, f32),
    bone_crack_small: (f32, f32),
    bone_snap: (f32, f32),
    face_hit_l1: (f32, f32),
    face_hit_l3: (f32, f32),
    max_cur_bail_average: f32,
    time_in_air: f32,
    was_wiping_out: bool,
    feet: [FootInfo; 2],
    num_feet_on_board: u32,
    /// Also `mFeet[eFeet_Both].mTimeInAir`: a both-feet catch stores its time here as float bits.
    num_feet_scraping: u32,
    dismount: Option<SpliceId>,
    hand_grabs_on: [HandGrab; 2],
    hand_grabs_off: [HandGrab; 2],
    check_for_wipe_out_over: bool,
    was_wipe_out: bool,
    was_head_hit_during_wipeout: bool,
    was_groin_hit_during_wipeout: bool,
    was_vehicle_hit_during_wipeout: bool,
    was_npc_hit_during_wipeout: bool,
    was_lw_hit_during_wipeout: bool,
    vehicle_collision: i32,
    npc_collision: i32,
    first_collision: bool,
    in_bail_last_update: bool,
    first_grunt: bool,
    land_tick: i32,
    /// `mbWasBail` (+0x1B0) and `mBailCount` (+0x1B4): the visual bail last update, and the
    /// updates until a bail without a wiped-out score gasps.
    was_bail: bool,
    bail_count: i32,
    /// `mCurrentPriority` (`tHOMPriority`): priority and `timeBeforeLesserOrEqual`.
    current_priority: (i32, i32),
    bail_level_count: [i32; 3],
    kickup: Option<SpliceId>,
    cur_bail_average: f32,
    cur_bail_average_max: f32,
    cur_max_bail: f32,
    num_bails: i32,
    was_hippy: bool,
    hippy_jump_strength: u32,
}

fn volume(ctx: &SfxCtx<'_>, io: Option<DMixIoId>, index: i32) -> i32 {
    ctx.output(io, index, PresetType::Volume)
}

/// The `mfSpread` of the updates that spread only a local skater's sound.
fn local_spread(ctx: &SfxCtx<'_>) -> f32 {
    if ctx.local { 1.0 } else { 0.0 }
}

/// `SFXCTL_PlayerPhysics::GetWheelType` 0x825199d0.
fn wheel_type(physics: &PlayerPhysics, ctx: &SfxCtx<'_>) -> u32 {
    if ctx.local { physics.wheel_hardness } else { ctx.npc_wheel_type }
}

/// The 0..1 `fsel` clamp.
fn clamp_unit(value: f32) -> f32 {
    let low = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - low >= 0.0 { low } else { 1.0 }
}

impl Contacts {
    /// `SFXObj_Contacts::SFXObj_Contacts` 0x8253ecb0.
    #[must_use]
    pub fn new(vault: &Vault) -> Self {
        let mut contacts = Self {
            out_of_manual: VoicedSplice::IDLE_OLLIE,
            ollie_l4: None,
            land_l4: None,
            ollie: VoicedSplice::IDLE_OLLIE,
            push_on: FootPush::default(),
            push_off: FootPush::default(),
            scrape: None,
            foot_catch: [None; 3],
            skate3_foot_catch: [None, None, None],
            foot_scrape: [None; 2],
            skate3_inverted: false,
            skate3_physics: false,
            skate3_dark_catch_recent: false,
            skate3_dark_catch_frames: 0,
            skate3_dark_catch_land: None,
            skate3_io: None,
            last_in_air: false,
            last_in_manual: false,
            last_in_grind: false,
            last_was_pushing: false,
            time_since_land_buffer: 0.0,
            foot_drag: None,
            wheel_played: [true; 4],
            l3_triggered: false,
            land: [VoicedSplice::IDLE; 4],
            sweetener: VoicedSplice::IDLE,
            body_part_collision_counter: [0.0; 8],
            board_collision_counter: 0.0,
            bone_crack_big: (0.0, 0.0),
            bone_crack_small: (0.0, 0.0),
            bone_snap: (0.0, 0.0),
            face_hit_l1: (0.0, 0.0),
            face_hit_l3: (0.0, 0.0),
            max_cur_bail_average: 0.0,
            time_in_air: 0.0,
            was_wiping_out: false,
            feet: [FootInfo::default(); 2],
            num_feet_on_board: 0,
            num_feet_scraping: 0,
            dismount: None,
            hand_grabs_on: [HandGrab::default(); 2],
            hand_grabs_off: [HandGrab::default(); 2],
            check_for_wipe_out_over: false,
            was_wipe_out: false,
            was_head_hit_during_wipeout: false,
            was_groin_hit_during_wipeout: false,
            was_vehicle_hit_during_wipeout: false,
            was_npc_hit_during_wipeout: false,
            was_lw_hit_during_wipeout: false,
            vehicle_collision: 0,
            npc_collision: 0,
            first_collision: false,
            in_bail_last_update: false,
            first_grunt: true,
            land_tick: 0,
            was_bail: false,
            bail_count: 0,
            current_priority: (0, 0),
            bail_level_count: [0; 3],
            kickup: None,
            cur_bail_average: 0.0,
            cur_bail_average_max: 0.0,
            cur_max_bail: 0.0,
            num_bails: 0,
            was_hippy: false,
            hippy_jump_strength: 0,
        };
        contacts.update_intensity_values(vault);
        contacts
    }

    /// `UpdateIntensityValues` 0x8251dd10.
    fn update_intensity_values(&mut self, vault: &Vault) {
        let value = |key| vault.f32(COLLISIONS, key);
        self.bone_crack_big = (value(0xAFA4_B509_0F1B_CF36), value(0x0096_0FED_B3EF_EE9C));
        self.bone_crack_small = (value(0xD120_03A6_0E98_7B9D), value(0xFA2A_A5A0_C048_1D00));
        self.bone_snap = (value(0x3695_327C_FB5E_1AC3), value(0x35FE_E8A9_5523_D812));
        self.face_hit_l1 = (value(0x076E_9081_CA17_59E9), value(0xDF53_9915_EB7E_883E));
        self.face_hit_l3 = (value(0x8E30_25BA_A686_F721), value(0x504D_3B73_5059_72D4));
        self.max_cur_bail_average = value(0x08B0_4D2D_8F89_B9A7);
    }

    /// `delete` of the splice and `AudioSystem::FreeSubmixVoice`.
    fn free_voiced(ctx: &mut SfxCtx<'_>, voiced: &mut VoicedSplice) {
        ctx.release(&mut voiced.splice);
        if let Some(voice) = voiced.voice.take() {
            ctx.core.release_voice(voice);
        }
    }

    /// `AudioSystem::CreateSplicerReverbVoice` 0x824f13c8 and the splice playing into it.
    fn play_voiced(ctx: &mut SfxCtx<'_>, voiced: &mut VoicedSplice, bank: i32, index: i32, wet: i32) {
        voiced.voice = collision::create_splicer_reverb_voice(ctx.core, ctx.master_sub_mix, ctx.global_effects.reverb_sub_mix(), wet as f32 * VOLUME_SCALE);
        voiced.splice = voiced.voice.and_then(|voice| ctx.new_splice(bank, index, PluginRef { voice, index: 0 }));
    }

    /// A new splice into the master mix.
    fn play_dry(ctx: &mut SfxCtx<'_>, bank: i32, index: i32) -> Option<SpliceId> {
        let master = ctx.master_sub_mix;
        ctx.new_splice(bank, index, master)
    }

    /// The `Splice::Update` of every `Update*`: `volume`, pitch output `pitch_index`, azimuth output
    /// 0 and the state's delta time while the splice plays; a finished splice is deleted (false).
    fn mix_update(ctx: &mut SfxCtx<'_>, io: Option<DMixIoId>, splice: &mut Option<SpliceId>, volume: i32, pitch_index: i32, spread: f32) -> bool {
        let Some(id) = *splice else { return false };
        if !ctx.is_playing(id) {
            ctx.release(splice);
            return false;
        }
        let update = SpliceUpdateParams {
            vol: volume as f32 * VOLUME_SCALE,
            pitch: ctx.output(io, pitch_index, PresetType::Pitch) as f32 * PITCH_SCALE,
            az: ctx.output(io, 0, PresetType::Azimuth) as f32 * AZIMUTH_SCALE,
            delta: ctx.delta_time,
            spread,
            time_stretch: 1.0,
        };
        ctx.update_splice(id, update);
        true
    }

    /// `SFXObj_Contacts::Destroy` 0x8251c108 (also `Detach`); the dismount splice is left playing.
    pub fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        if self.ollie.splice.is_some() {
            Self::free_voiced(ctx, &mut self.ollie);
        }
        if self.out_of_manual.splice.is_some() {
            Self::free_voiced(ctx, &mut self.out_of_manual);
            self.out_of_manual.land_type = LAND_CONCRETE_HARD;
        }
        ctx.release(&mut self.ollie_l4);
        ctx.release(&mut self.land_l4);
        ctx.release(&mut self.push_on.splice);
        ctx.release(&mut self.push_off.splice);
        ctx.release(&mut self.scrape);
        for splice in &mut self.foot_catch {
            ctx.release(splice);
        }
        for splice in &mut self.foot_scrape {
            ctx.release(splice);
        }
        for land in &mut self.land {
            if land.splice.is_some() {
                Self::free_voiced(ctx, land);
                *land = VoicedSplice::IDLE;
            }
        }
        if self.sweetener.splice.is_some() {
            Self::free_voiced(ctx, &mut self.sweetener);
            self.sweetener = VoicedSplice::IDLE;
        }
        for hand in 0..2 {
            ctx.release(&mut self.hand_grabs_on[hand].splice);
            ctx.release(&mut self.hand_grabs_off[hand].splice);
        }
        ctx.release(&mut self.kickup);
        for sound in &mut self.skate3_foot_catch {
            Skate3Splice::delete_slot(sound, ctx);
        }
        Skate3Splice::delete_slot(&mut self.skate3_dark_catch_land, ctx);
    }

    /// `SFXObj_Contacts::UpdateParams` 0x8254f8a0 (the state is attached), for the skater at
    /// `array_index` of the presentation.
    pub fn update_params(&mut self, t: f32, physics: &PlayerPhysics, skater: &SkaterData, array_index: i32, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        self.skate3_physics = physics.skate3_physics && ctx.skate3.is_some();
        // TU3's controller update (0x824b17fc..0x824b1834) keeps a darkcatch for five frames.
        if physics.skate3_dark_catch {
            self.skate3_dark_catch_recent = true;
            self.skate3_dark_catch_frames = 0;
        } else if self.skate3_dark_catch_frames < 5 {
            self.skate3_dark_catch_frames += 1;
        } else {
            self.skate3_dark_catch_recent = false;
        }
        self.check_for_foot_catches(physics, ctx);
        self.check_for_foot_scrapes(physics, ctx);
        self.check_for_skate3_inverted_board(physics, ctx);
        let manual = self.check_for_manual(physics, io, ctx);
        self.check_for_wheel_hits(manual, physics, io, ctx);
        self.check_for_jumps_and_lands(physics, io, ctx);
        self.check_for_stops(physics, ctx);
        self.check_for_pushes(physics, ctx);
        self.check_for_body_collisions(physics, io, ctx);
        self.check_for_board_collisions(physics, ctx);
        self.check_for_collision_speech(physics, ctx);
        self.check_for_hand_grabs(physics, ctx);
        self.check_for_score(skater, array_index, ctx);
        self.check_for_kick_up(physics, ctx);
        self.check_for_air_dismount(physics, ctx);
        let time = self.current_priority.1;
        let elapsed = fctiwz(t * 1000.0);
        self.current_priority.1 = time.wrapping_sub(elapsed);
        if time.wrapping_sub(elapsed) < 0 {
            self.current_priority.1 = 0;
        }
        self.update_bail_average(io, ctx);
    }

    /// `CheckForScore` 0x82520780: a skater's banked score (the local skater's rise in announcer
    /// score instead) cheers; the local skater's wiped-out trick score gasps, as does a visual
    /// bail with none, four updates after it began. Either goes to `AudioSystem::PlayCrowdSound`.
    fn check_for_score(&mut self, skater: &SkaterData, array_index: i32, ctx: &mut SfxCtx<'_>) {
        let local = skater.local;
        let mut score = skater.score as i32;
        let mut wiped_out = 0;
        if local {
            score = ctx.cond_score_delta.max(0);
            wiped_out = skater.wipedout_trick_score as i32;
            if ctx.bail {
                if !self.was_bail {
                    self.bail_count = 4;
                }
            } else {
                self.bail_count = 0;
            }
            self.was_bail = ctx.bail;
            if wiped_out > 0 {
                self.bail_count = 0;
            }
            if self.bail_count > 0 {
                self.bail_count -= 1;
                if self.bail_count == 0 {
                    wiped_out = 1;
                }
            }
        }
        if score != 0 || wiped_out > 0 {
            ctx.play_crowd_sound(array_index, score, wiped_out, local);
        }
    }

    /// `SFXObj_Contacts::ProcessUpdate` 0x8254c1c8 (the state is attached).
    pub fn process_update(&mut self, physics: &PlayerPhysics, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        self.update_skate3_foot_catches(ctx);
        self.update_foot_catches(io, ctx);
        self.update_foot_scrapes(io, ctx);
        self.update_jumps_and_lands(io, ctx);
        self.update_manual(io, ctx);
        self.update_stops(physics, io, ctx);
        self.update_pushes(io, ctx);
        self.update_hand_grabs(io, ctx);
        self.update_kick_up(physics, io, ctx);
        self.update_air_dismount(physics, io, ctx);
    }

    /// `GetOllieLandType` 0x8251cf00.
    fn ollie_land_type(physics: &PlayerPhysics, ctx: &SfxCtx<'_>) -> u32 {
        let forced = ctx.vault.i32(CONTACTS, 0xB5F9_1FF9_1AC9_E67E);
        if forced != -1 {
            return forced as u32;
        }
        let mut material = OL_CONCRETE;
        if physics.wheel_material[0] < MATERIAL_MAX {
            material = ollie_and_land_material(ctx.vault, physics.wheel_material[0]);
        }
        let wood = if material == OL_CONCRETE { 0 } else { 2 };
        if wheel_type(physics, ctx) != 0 { wood + 1 } else { wood }
    }

    /// `GetPushSurface` 0x8251cfe0.
    fn push_surface(physics: &PlayerPhysics, ctx: &SfxCtx<'_>) -> u32 {
        let forced = ctx.vault.i32(CONTACTS, 0xB280_CB10_9E9C_4BFE);
        if forced != -1 {
            return forced as u32;
        }
        let material = physics.wheel_material[0];
        if material >= MATERIAL_MAX { DRAG_RUFF_CNC } else { stopping_material(ctx.vault, material) }
    }

    /// `GetFootDragSurface` 0x8251d098.
    fn foot_drag_surface(physics: &PlayerPhysics, manual_brake: bool, ctx: &SfxCtx<'_>) -> u32 {
        let forced = ctx.vault.i32(CONTACTS, 0xA4E7_2653_01C0_8731);
        if forced != -1 {
            return forced as u32;
        }
        let material = if manual_brake { physics.wheel_material[2] } else { physics.wheel_material[0] };
        if material >= MATERIAL_MAX {
            return DRAG_RUFF_CNC;
        }
        let surface = stopping_material(ctx.vault, material);
        if manual_brake && surface == DRAG_SMOOTH_CNC { DRAG_RUFF_CNC } else { surface }
    }

    /// `GetSpliceIndexForLand` 0x8251d158: (bank, index, land type).
    fn splice_index_for_land(physics: &PlayerPhysics, num_wheels: u32, level: u32, ctx: &mut SfxCtx<'_>) -> (i32, i32, u32) {
        let element = 3 * num_wheels + level;
        let land_type = Self::ollie_land_type(physics, ctx);
        let kind = if num_wheels <= 2 && level >= 2 { LAND_CONCRETE_HARD } else { land_type };
        let (bank, start, count) = match kind {
            LAND_CONCRETE_HARD => (SPLICE_LANDS_CONCRETE_HARD, 0xACA9_CBE6_329E_9F18, 0x542A_ED42_8D1E_8B2C),
            LAND_CONCRETE_SOFT => (SPLICE_LANDS_CONCRETE_SOFT, 0x40A6_D5D9_9F67_0CA6, 0x5BE5_F4E4_D343_8174),
            LAND_WOOD_HARD => (SPLICE_LANDS_WOOD_HARD, 0x0B0C_35D6_E2D3_E23A, 0xE235_06E8_0C1B_D14E),
            LAND_WOOD_SOFT => (SPLICE_LANDS_WOOD_SOFT, 0x31FB_10F5_8114_CAEE, 0x35F1_F322_D970_272C),
            _ => return (SPLICE_UNKNOWN, ctx.rng.range(0, 0), land_type),
        };
        let start = ctx.vault.i32_at(CONTACTS, start, element);
        let count = ctx.vault.i32_at(CONTACTS, count, element) as u32;
        (bank, ctx.rng.range(start, count), land_type)
    }

    /// `GetSpliceIndexForOllie` 0x8253f160: (bank, index, the `eOLMaterial` it writes).
    fn splice_index_for_ollie(physics: &PlayerPhysics, level: u32, ctx: &mut SfxCtx<'_>) -> (i32, i32, Option<u32>) {
        let land_type = Self::ollie_land_type(physics, ctx);
        let (bank, material, start, count) = match land_type {
            LAND_CONCRETE_HARD | LAND_CONCRETE_SOFT => (SPLICE_OLLIES_CONCRETE, OL_CONCRETE, 0xED54_5C04_F769_119A, 0x7090_B84B_A88D_F603),
            LAND_WOOD_HARD | LAND_WOOD_SOFT => (SPLICE_OLLIES_WOOD, OL_WOOD, 0x69BB_3D4F_601B_3351, 0xB26E_6C62_342F_ABF1),
            _ => return (SPLICE_UNKNOWN, ctx.rng.range(0, 0), None),
        };
        let start = ctx.vault.i32_at(CONTACTS, start, level);
        let count = ctx.vault.i32_at(CONTACTS, count, level) as u32;
        let mut index = ctx.rng.range(start, count);
        if land_type <= LAND_CONCRETE_SOFT && index == 0 {
            index = 5;
        }
        (bank, index, Some(material))
    }

    /// `GetVolForWheelHit` 0x8251f368.
    fn vol_for_wheel_hit(io: Option<DMixIoId>, num_wheels: u32, level: u32, land_type: u32, ctx: &SfxCtx<'_>) -> i32 {
        let mixer = volume(ctx, io, 3);
        let element = 3 * num_wheels + level;
        let key = match land_type {
            LAND_CONCRETE_HARD => Some(0xCEA5_AFA8_BA17_0B07),
            LAND_CONCRETE_SOFT => Some(0x951A_D537_1803_0327),
            LAND_WOOD_HARD => Some(0x068C_8C5E_BEC1_B45F),
            LAND_WOOD_SOFT => Some(0x6823_F491_0A18_82AD),
            _ => None,
        };
        let scale = key.map_or(1.0, |key| ctx.vault.f32_at(CONTACTS, key, element));
        fctiwz(mixer as f32 * scale)
    }

    /// `GetOllieL4Index` 0x82520570 / `GetLandL4Index` 0x82520678.
    fn l4_index(ctx: &mut SfxCtx<'_>, start: u64, count: u64) -> i32 {
        let start = ctx.vault.i32(CONTACTS, start);
        let count = ctx.vault.i32(CONTACTS, count) as u32;
        ctx.rng.range(start, count)
    }

    /// `SpliceUtil::GetSK8_Foot_CatchIndex` 0x8251b7b8 / `GetSplice_handsIndex` 0x8251b900: the
    /// start (word 1 + 2 level) and count (word 2 + 2 level) of record `kind`.
    fn splice_util_index(ctx: &mut SfxCtx<'_>, collection: Collection, key: u64, kind: u32, level: usize) -> i32 {
        let start = ctx.vault.struct_word(collection, key, kind, 1 + 2 * level) as i32;
        let count = ctx.vault.struct_word(collection, key, kind, 2 + 2 * level);
        ctx.rng.range(start, count)
    }

    /// `PlayHandGrabOnSound` 0x8251c398.
    fn play_hand_grab_on(&mut self, hand: usize, in_air: bool, ctx: &mut SfxCtx<'_>) {
        ctx.release(&mut self.hand_grabs_on[hand].splice);
        let kind = if in_air { HANDS_RATTLE_GRAB } else { HANDS_SOLID_GRAB };
        let index = Self::splice_util_index(ctx, HANDGRAB, 0x8B58_8915_A2A9_6EA0, kind, 0);
        self.hand_grabs_on[hand].splice = Self::play_dry(ctx, SPLICE_GRABS, index);
    }

    /// `PlayHandGrabOffSound` 0x8251c4a0.
    fn play_hand_grab_off(&mut self, hand: usize, ctx: &mut SfxCtx<'_>) {
        ctx.release(&mut self.hand_grabs_off[hand].splice);
        let index = Self::splice_util_index(ctx, HANDGRAB, 0x8B58_8915_A2A9_6EA0, HANDS_RELEASE, 0);
        self.hand_grabs_off[hand].splice = Self::play_dry(ctx, SPLICE_GRABS, index);
    }

    /// `CheckForHandGrabs` 0x8251c598.
    fn check_for_hand_grabs(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let (right, left) = (physics.is_grabbing_right, physics.is_grabbing_left);
        let right_on = !self.hand_grabs_on[HAND_RIGHT].was_grabbing && right;
        let left_on = !self.hand_grabs_on[HAND_LEFT].was_grabbing && left;
        let right_off = self.hand_grabs_off[HAND_RIGHT].was_grabbing && !right;
        let left_off = self.hand_grabs_off[HAND_LEFT].was_grabbing && !left;
        if right_on {
            self.play_hand_grab_on(HAND_RIGHT, physics.in_air, ctx);
        }
        if left_on {
            self.play_hand_grab_on(HAND_LEFT, physics.in_air, ctx);
        }
        if right_off {
            self.play_hand_grab_off(HAND_RIGHT, ctx);
        }
        if left_off {
            self.play_hand_grab_off(HAND_LEFT, ctx);
        }
        self.hand_grabs_on[HAND_RIGHT].was_grabbing = right;
        self.hand_grabs_on[HAND_LEFT].was_grabbing = left;
        self.hand_grabs_off[HAND_RIGHT].was_grabbing = right;
        self.hand_grabs_off[HAND_LEFT].was_grabbing = left;
    }

    /// TU3 `PlayFootCatch` 0x824b9268 with Skate 3 physics: Skate 3's footcatch air time and speed
    /// thresholds (`GlobalAttributeCollection` +0x6C), its cue for the catch type and level
    /// (inverted with the board upside down, 0x824c0b90) from splice bank 0, through its effect, in
    /// a splice voice of its own (0x824b94dc).
    fn play_skate3_foot_catch(&mut self, kind: u32, speed: f32, foot: usize, ctx: &mut SfxCtx<'_>) {
        let Some(vault) = ctx.skate3 else { return };
        if !Self::skate3_bank(ctx, SPLICE_SKATE3_COLLISIONS, "Skate_Collisions.bnk") {
            return;
        }
        let wheels = FOOTCATCH;
        let time = if foot == FEET_BOTH { f32::from_bits(self.num_feet_scraping) } else { self.feet[foot].time_in_air };
        if !(time > vault.f32(wheels, 0x6BFB_6A3F_22AA_F797) * 0.001) || !(speed >= vault.f32(wheels, 0xB63C_C740_D29A_D118)) {
            return;
        }
        let level = if speed >= vault.f32(wheels, 0xE1F8_7C49_37E3_CCA2) || (self.was_hippy && self.hippy_jump_strength != 0) {
            2
        } else if speed >= vault.f32(wheels, 0x7D1C_A200_987A_A109) || (self.was_hippy && self.hippy_jump_strength == 0) {
            1
        } else {
            0
        };
        let key = match (self.skate3_inverted, kind) {
            (false, 0) => 0x6F13_3132_EF3F_8063,
            (false, 1) => 0xE518_088F_4E61_203A,
            (false, 3) => 0x3AC5_AA2D_2010_B49C,
            (false, _) => 0x4CFF_3685_5CA0_4B6F,
            (true, 0) => 0xDB56_9526_76D7_122E,
            (true, 1) => 0x0ACF_CB7E_532B_7A0B,
            (true, _) => 0x7EA3_7827_E422_8614,
        };
        let cue = vault.i32_at(wheels, key, level);
        trace_skate3_darkcatch(format_args!("catch kind {kind} level {level} inverted {}: cue {cue}", self.skate3_inverted));
        if self.skate3_foot_catch[foot].is_none() {
            self.skate3_foot_catch[foot] = Self::play_skate3_effect(ctx, cue, 0xF61F_2797_B248_68D9);
        }
    }

    /// `PlayFootCatch` 0x8251c6c8.
    fn play_foot_catch(&mut self, kind: u32, speed: f32, foot: usize, ctx: &mut SfxCtx<'_>) {
        if self.skate3_physics {
            self.play_skate3_foot_catch(kind, speed, foot, ctx);
            return;
        }
        let time = if foot == FEET_BOTH { f32::from_bits(self.num_feet_scraping) } else { self.feet[foot].time_in_air };
        if !(time > ctx.vault.f32(FOOTCATCH, 0x6BFB_6A3F_22AA_F797) * 0.001) {
            return;
        }
        if !(speed >= ctx.vault.f32(FOOTCATCH, 0xB63C_C740_D29A_D118)) {
            return;
        }
        let level = if speed >= ctx.vault.f32(FOOTCATCH, 0xE1F8_7C49_37E3_CCA2) || (self.was_hippy && self.hippy_jump_strength != 0) {
            2
        } else if speed >= ctx.vault.f32(FOOTCATCH, 0x7D1C_A200_987A_A109) || (self.was_hippy && self.hippy_jump_strength == 0) {
            1
        } else {
            0
        };
        let index = Self::splice_util_index(ctx, FOOTCATCH, 0x19FA_E352_5831_7C7C, kind, level);
        if self.foot_catch[foot].is_none() {
            self.foot_catch[foot] = Self::play_dry(ctx, SPLICE_FOOT_CATCH, index);
        }
    }

    /// A Skate 3 sound from its splice bank 0, through the effect sub mix its attribute `key` in
    /// `SKATE3_EFFECTS` names (TU3 0x824b9468 / 0x824bade4), in a splice voice of its own.
    fn play_skate3_effect(ctx: &mut SfxCtx<'_>, cue: i32, key: u64) -> Option<Skate3Splice> {
        let vault = ctx.skate3?;
        let index = vault.i32(SKATE3_EFFECTS, key);
        let target = ctx.global_effects.skate3_effect(ctx.core, index, ctx.local, vault, ctx.rng);
        Skate3Splice::new(ctx, SPLICE_SKATE3_COLLISIONS, cue, target)
    }

    /// TU3 `CheckForFootScrapes` 0x824b9a74..0x824b9ac0: a darkcatch (`SFXCTL_PlayerPhysics`
    /// +0x301) holds the board inverted; when it ends, the right foot scrapes the board back over
    /// unless its scrape already sounds. Nothing happens in Skate 2 physics.
    fn check_for_skate3_inverted_board(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if physics.skate3_dark_catch {
            self.skate3_inverted = true;
        } else if self.skate3_inverted {
            if self.foot_scrape[FEET_RIGHT].is_none() {
                self.foot_scrape[FEET_RIGHT] = Self::skate3_inverted_scrape(FEET_RIGHT, ctx);
            }
            self.skate3_inverted = false;
        }
    }

    /// TU3 `PlayFootScrapeSound` 0x824b97c8 with the board inverted: Skate 3's
    /// `aud_contacts/default` 0xF2866EE0540CDF05 element `foot`, in its splice bank 7.
    fn skate3_inverted_scrape(foot: usize, ctx: &mut SfxCtx<'_>) -> Option<SpliceId> {
        let vault = ctx.skate3?;
        if !Self::skate3_bank(ctx, SPLICE_SKATE3_FOLEY, "sk8_foley.bnk") {
            return None;
        }
        let cue = vault.i32_at(CONTACTS, 0xF286_6EE0_540C_DF05, foot as u32);
        trace_skate3_darkcatch(format_args!("scrape foot {foot}: cue {cue}"));
        Self::play_dry(ctx, SPLICE_SKATE3_FOLEY as i32, cue)
    }

    /// TU3 `LandGround` 0x824bac14..0x824baee8: landing in or just after a darkcatch plays Skate 3's
    /// darkcatch landing from its splice bank 0: `aud_contacts/default`'s cue by whether the deck's
    /// surface has a landing material (`SurfaceMapping` word 2) and whether the air time reached
    /// its threshold, through its effect, in a splice voice of its own (0x824baedc).
    fn land_skate3_dark_catch(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        Skate3Splice::delete_slot(&mut self.skate3_dark_catch_land, ctx);
        let Some(vault) = ctx.skate3 else { return };
        if !Self::skate3_bank(ctx, SPLICE_SKATE3_COLLISIONS, "Skate_Collisions.bnk") {
            return;
        }
        let material = if physics.deck_in_contact { physics.deck_material } else { 0 };
        let landing_surface = crate::surface::mapping(ctx.vault, material, 2) != 0;
        let short = self.time_in_air < vault.f32(CONTACTS, 0x224B_0656_2D9D_5E0E);
        let key = match (landing_surface, short) {
            (true, true) => 0x1E86_4695_56AC_D80A,
            (true, false) => 0xBF22_DD8B_C69D_DE53,
            (false, true) => 0xF262_042E_AA29_5711,
            (false, false) => 0x3A2F_1C78_8E21_D92D,
        };
        let cue = vault.i32(CONTACTS, key);
        trace_skate3_darkcatch(format_args!("land surface {landing_surface} short {short}: cue {cue}"));
        self.skate3_dark_catch_land = Self::play_skate3_effect(ctx, cue, 0x8B0E_0307_99CB_DD00);
    }

    /// One of Skate 3's splice banks, read from its `audiofiles.big` into `slot` the first time.
    pub(super) fn skate3_bank(ctx: &mut SfxCtx<'_>, slot: usize, name: &str) -> bool {
        if ctx.splicer.bank(slot).is_some() {
            return true;
        }
        let Some(files) = ctx.core.stream_pools.files() else { return false };
        let path = format!("{}data\\audio\\audiofiles.big|data\\audio\\{name}", crate::rwac::stream_pool::SKATE3_FILES);
        let Some(size) = files.size(&path) else { return false };
        let mut bytes = vec![0u8; size as usize];
        if !files.read_at(&path, 0, &mut bytes) {
            return false;
        }
        ctx.splicer.load_bank(std::sync::Arc::from(bytes), slot).is_ok()
    }

    /// `PlayFootCatchHelper` 0x8251c938; with Skate 3 physics TU3's 0x824b9508, which catches the
    /// first foot, then the second, then both.
    fn play_foot_catch_helper(&mut self, speed: f32, foot: usize, ctx: &mut SfxCtx<'_>) {
        if self.skate3_physics {
            match self.num_feet_on_board {
                0 => self.play_foot_catch(CATCH_FIRST_FOOT, speed, foot, ctx),
                1 => self.play_foot_catch(CATCH_2ND_FOOT, speed, foot, ctx),
                2 => {
                    self.play_foot_catch(CATCH_BOTH_FEET, speed, foot, ctx);
                    self.num_feet_on_board = 0;
                }
                _ => {}
            }
            return;
        }
        match self.num_feet_on_board {
            0 => self.play_foot_catch(CATCH_BOTH_FEET, speed, foot, ctx),
            1 => self.play_foot_catch(CATCH_FIRST_FOOT, speed, foot, ctx),
            2 => {
                self.play_foot_catch(CATCH_2ND_FOOT, speed, foot, ctx);
                self.num_feet_on_board = 0;
            }
            _ => {}
        }
    }

    /// `CheckForFootCatches` 0x8251c9d0.
    fn check_for_foot_catches(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let left = physics.left_foot_on_deck;
        let right = physics.right_foot_on_deck;
        if !left {
            self.feet[FEET_LEFT].time_in_air += ctx.delta_time;
        }
        if !right {
            self.feet[FEET_RIGHT].time_in_air += ctx.delta_time;
        }
        let right_lands = right && !self.feet[FEET_RIGHT].was_in_air;
        let left_lands = left && !self.feet[FEET_LEFT].was_in_air;
        if right_lands {
            if left_lands {
                self.num_feet_on_board = 0;
                let (right_time, left_time) = (self.feet[FEET_RIGHT].time_in_air, self.feet[FEET_LEFT].time_in_air);
                self.num_feet_scraping = (if right_time - left_time >= 0.0 { right_time } else { left_time }).to_bits();
                let (left_speed, right_speed) = (physics.left_foot_deck_y_speed, physics.right_foot_deck_y_speed);
                let speed = if left_speed - right_speed >= 0.0 { left_speed } else { right_speed };
                self.play_foot_catch_helper(speed, FEET_BOTH, ctx);
                self.feet[FEET_RIGHT].time_in_air = 0.0;
                self.num_feet_scraping = 0.0f32.to_bits();
                self.feet[FEET_LEFT].time_in_air = 0.0;
            } else {
                self.num_feet_on_board += 1;
                self.play_foot_catch_helper(physics.right_foot_deck_y_speed, FEET_RIGHT, ctx);
                self.feet[FEET_RIGHT].time_in_air = 0.0;
            }
        } else if left_lands {
            self.num_feet_on_board += 1;
            self.play_foot_catch_helper(physics.left_foot_deck_y_speed, FEET_LEFT, ctx);
            self.feet[FEET_LEFT].time_in_air = 0.0;
        }
        self.feet[FEET_RIGHT].was_in_air = right;
        self.feet[FEET_LEFT].was_in_air = left;
        if right || left {
            if self.was_hippy && right && left {
                self.was_hippy = false;
            }
        } else if physics.is_hippy_jump {
            if !self.was_hippy {
                self.hippy_jump_strength = physics.hippy_jump_strength;
            }
            self.was_hippy = true;
        }
    }

    /// `PlayFootScrapeSound` 0x8251cbb0 (TU3 0x824b97c8 plays the inverted cue during a Skate 3
    /// darkcatch).
    fn play_foot_scrape(&mut self, feet_scraping: u32, foot: usize, ctx: &mut SfxCtx<'_>) {
        if self.skate3_inverted
            && let Some(splice) = Self::skate3_inverted_scrape(foot, ctx)
        {
            self.foot_scrape[foot] = Some(splice);
            return;
        }
        let mut offset = 0;
        let mut start = 0;
        if feet_scraping > 1 {
            start = ctx.vault.i32_at(CONTACTS, 0x384B_CDC2_32D3_EE23, 0);
        }
        let count = ctx.vault.i32_at(CONTACTS, 0x384B_CDC2_32D3_EE23, feet_scraping.wrapping_sub(1)) as u32;
        if count != 0 {
            offset = (ctx.rng.next_u32() % count) as i32;
        }
        self.foot_scrape[foot] = Self::play_dry(ctx, SPLICE_FOOT_SCRAPES, start.wrapping_add(offset));
    }

    /// `CheckForFootScrapes` 0x8251cd60.
    fn check_for_foot_scrapes(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !ctx.vault.bool(CONTACTS, 0xC78B_480A_EEA6_47BA) || physics.num_wheels_on_ground <= 1 {
            return;
        }
        let threshold = |ctx: &SfxCtx<'_>| ctx.vault.f32(CONTACTS, 0x823D_59FB_4632_4175) * f32::from_bits(0x3C23_D70A);
        if physics.right_foot_on_deck && physics.right_foot_slide_deck_speed > threshold(ctx) {
            if self.num_feet_scraping > 1 {
                self.num_feet_scraping = 0;
            }
            if self.foot_scrape[FEET_RIGHT].is_none() {
                self.num_feet_scraping += 1;
                self.play_foot_scrape(self.num_feet_scraping, FEET_RIGHT, ctx);
            }
        }
        if physics.left_foot_on_deck && physics.left_foot_slide_deck_speed > threshold(ctx) {
            if self.num_feet_scraping > 1 {
                self.num_feet_scraping = 0;
            }
            if self.foot_scrape[FEET_LEFT].is_none() {
                self.num_feet_scraping += 1;
                self.play_foot_scrape(self.num_feet_scraping, FEET_LEFT, ctx);
            }
        }
        if self.num_feet_scraping > 2 {
            self.num_feet_scraping = 0;
        }
    }

    /// `CheckForManual` 0x8251d708: true when the manual ended on three or four wheels.
    fn check_for_manual(&mut self, physics: &PlayerPhysics, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) -> bool {
        let manual = physics.is_manual;
        let wheels = physics.num_wheels_on_ground;
        if manual || !self.last_in_manual {
            self.last_in_manual = manual;
        } else if wheels == 4 || wheels == 3 {
            if self.out_of_manual.splice.is_some() {
                Self::free_voiced(ctx, &mut self.out_of_manual);
                self.out_of_manual.land_type = LAND_CONCRETE_HARD;
            }
            let (bank, index, land_type) = Self::splice_index_for_land(physics, WHEELS_TICK, HIT_ONE, ctx);
            self.out_of_manual.land_type = land_type;
            let wet = volume(ctx, io, 17);
            Self::play_voiced(ctx, &mut self.out_of_manual, bank, index, wet);
            self.wheel_played = [true; 4];
            self.last_in_manual = false;
            return true;
        } else if wheels == 0 {
            self.last_in_manual = false;
        }
        false
    }

    /// `PlayDragForPowerSlide` 0x8251d8c0.
    fn play_drag_for_power_slide(physics: &PlayerPhysics, ctx: &SfxCtx<'_>) -> bool {
        if physics.wheel_slide > 0.0 && !physics.is_rail_grind && physics.wheel_material[0] < MATERIAL_MAX {
            let material = power_slide_material(ctx.vault, physics.wheel_material[0]);
            return material == PS_GRASS || material == PS_DIRT;
        }
        false
    }

    /// `CheckForPushes` 0x8251db18 with `NewPushOnSplice` 0x8251d938 / `NewPushOffSplice` 0x8251da28
    /// (the push surface is the splice bank).
    fn check_for_pushes(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if physics.is_pushing_left || physics.is_pushing_right {
            if !self.last_was_pushing {
                let surface = Self::push_surface(physics, ctx);
                let count = ctx.vault.i32_at(CONTACTS, 0x651A_76D6_6C0D_2D5E, surface) as u32;
                let index = ctx.rng.range(0, count);
                ctx.release(&mut self.push_on.splice);
                self.push_on.surface = surface;
                self.push_on.splice = Self::play_dry(ctx, surface as i32, index);
                self.last_was_pushing = true;
            }
        } else if self.last_was_pushing {
            let surface = Self::push_surface(physics, ctx);
            let start = ctx.vault.i32_at(CONTACTS, 0x651A_76D6_6C0D_2D5E, surface);
            let count = ctx.vault.i32_at(CONTACTS, 0x1002_8FB6_8931_4248, surface) as u32;
            let index = ctx.rng.range(start, count);
            ctx.release(&mut self.push_off.splice);
            self.push_off.surface = surface;
            self.push_off.splice = Self::play_dry(ctx, surface as i32, index);
            self.last_was_pushing = false;
        }
    }

    /// `PlayWheelHit` 0x8253eec0.
    fn play_wheel_hit(&mut self, physics: &PlayerPhysics, num_wheels: u32, level: u32, check: bool, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        let Some(slot) = self.land.iter().position(|land| land.splice.is_none()) else { return };
        let mut level = level;
        let mut sweeten = false;
        if level > 1 {
            level = HIT_TWO;
            sweeten = !check || !self.l3_triggered;
        }
        let mut sweetener = (SPLICE_UNKNOWN, 0, LAND_CONCRETE_HARD);
        if sweeten {
            sweetener = Self::splice_index_for_land(physics, num_wheels, HIT_THREE, ctx);
        }
        let (bank, index, land_type) = Self::splice_index_for_land(physics, num_wheels, level, ctx);
        trace_skate3_darkcatch(format_args!("wheel hit wheels {num_wheels} level {level} landed {:?}: bank {bank} index {index}", physics.landed));
        self.land[slot].num_wheels = num_wheels;
        self.land[slot].land_type = land_type;
        self.land[slot].level = level;
        let wet = volume(ctx, io, 17);
        Self::play_voiced(ctx, &mut self.land[slot], bank, index, wet);
        if sweeten && self.sweetener.splice.is_none() && ctx.cvars.enable_sweetener {
            self.sweetener.num_wheels = num_wheels;
            self.sweetener.land_type = sweetener.2;
            Self::play_voiced(ctx, &mut self.sweetener, sweetener.0, sweetener.1, wet);
        }
    }

    /// Marks the wheels in `landed` as played.
    fn mark_played(&mut self, landed: [bool; 4]) {
        for wheel in 0..4 {
            if landed[wheel] {
                self.wheel_played[wheel] = true;
            }
        }
    }

    /// `CheckForWheelHits` 0x8254af78.
    fn check_for_wheel_hits(&mut self, manual: bool, physics: &PlayerPhysics, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        ctx.set_input(io, 3, 0);
        ctx.set_input(io, 4, 0);
        let mut landed = physics.landed;
        let mut count = 0;
        let mut level = 0;
        for wheel in 0..4 {
            if landed[wheel] {
                if self.wheel_played[wheel] {
                    landed[wheel] = false;
                } else {
                    count += 1;
                    if physics.impact_level[wheel] > level {
                        level = physics.impact_level[wheel];
                    }
                }
            } else {
                self.wheel_played[wheel] = false;
            }
        }
        let played = self.wheel_played.iter().filter(|&&played| played).count();
        if !ctx.local && count > 0 && level == 2 {
            level = 1;
        }
        let level = level as u32;
        if count == 4 {
            if level == 1 || level == 2 {
                let speed = clamp_unit(physics.board_speed * f32::from_bits(0x3DA3_D70A));
                ctx.set_input(io, 6, fctiwz(speed * 32767.0).clamp(0, 0x7FFF));
                ctx.set_input(io, if level == 1 { 3 } else { 4 }, 0x7FFF);
            }
            self.play_wheel_hit(physics, WHEELS_FOUR, level, false, io, ctx);
            self.wheel_played = [true; 4];
            self.l3_triggered = false;
            return;
        }
        if count <= 0 {
            return;
        }
        if count == 2 {
            match played {
                0 => {
                    self.l3_triggered = level > 1;
                    self.play_wheel_hit(physics, WHEELS_TWO_FIRST, level, false, io, ctx);
                    self.mark_played(landed);
                }
                2 => {
                    if !manual && !self.last_in_manual {
                        self.play_wheel_hit(physics, WHEELS_TWO_LAST, level, true, io, ctx);
                    }
                    self.mark_played(landed);
                    self.l3_triggered = false;
                }
                _ => {
                    self.play_wheel_hit(physics, WHEELS_FOUR, level, true, io, ctx);
                    self.wheel_played = [true; 4];
                    self.l3_triggered = false;
                }
            }
        } else if count == 3 {
            self.play_wheel_hit(physics, WHEELS_FOUR, level, true, io, ctx);
            self.wheel_played = [true; 4];
            self.l3_triggered = false;
        } else if played == 3 {
            self.wheel_played = [true; 4];
        } else if played == 2 {
            if !manual && !self.last_in_manual {
                self.play_wheel_hit(physics, WHEELS_TWO_LAST, level, true, io, ctx);
            }
            self.wheel_played = [true; 4];
            self.l3_triggered = false;
        } else {
            if played == 1 {
                let triggered = self.l3_triggered;
                self.play_wheel_hit(physics, WHEELS_ONE, level, triggered, io, ctx);
                self.l3_triggered = level > 1;
            } else {
                self.l3_triggered = level > 1;
                self.play_wheel_hit(physics, WHEELS_ONE, level, false, io, ctx);
            }
            self.mark_played(landed);
        }
    }

    /// `TakeOffGround` 0x8253f350.
    fn take_off_ground(&mut self, trick: i32, physics: &PlayerPhysics, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        let early = self.land_tick < 2;
        if self.ollie.splice.is_some() {
            Self::free_voiced(ctx, &mut self.ollie);
            self.ollie.level = 0;
            self.ollie.land_type = OL_CONCRETE;
        }
        if self.ollie.splice.is_some() || early {
            return;
        }
        let fraction = physics.jump_fraction;
        let mut level = 0;
        if fraction <= ctx.vault.f32(CONTACTS, 0xF2A1_E273_ABB8_E9AB) {
            if fraction > ctx.vault.f32(CONTACTS, 0xD775_8385_CDB8_DC26) {
                level = 1;
            }
        } else {
            level = 2;
        }
        if trick == TRICK_FS_BONELESS || trick == TRICK_NO_COMPLY {
            level = 0;
        }
        let (bank, index, material) = Self::splice_index_for_ollie(physics, level, ctx);
        if let Some(material) = material {
            self.ollie.land_type = material;
        }
        self.ollie.level = level;
        let wet = volume(ctx, io, 16);
        Self::play_voiced(ctx, &mut self.ollie, bank, index, wet);
        ctx.set_input(io, 0, 0x7FFF);
        if self.scrape.is_none() {
            let speed = physics.board_speed;
            let scrape = if !(speed <= ctx.vault.f32(CONTACTS, 0x5852_3180_E1AD_61B4)) {
                2
            } else if !(speed <= ctx.vault.f32(CONTACTS, 0xA730_73D3_A33E_35AE)) {
                1
            } else {
                let mut scrape = -1;
                if speed > ctx.vault.f32(CONTACTS, 0xFB71_DF2C_8592_8859) {
                    scrape = 0;
                }
                if physics.is_trick_with_scrape {
                    scrape = 1;
                }
                scrape
            };
            if scrape != -1 {
                self.scrape = Self::play_dry(ctx, SPLICE_OLLIES_SCRAPES, scrape);
            }
        }
        if ctx.local {
            ctx.release(&mut self.ollie_l4);
            let index = Self::l4_index(ctx, 0x8218_702E_69B4_DD1E, 0x5092_E87C_BCF4_8E4B);
            self.ollie_l4 = Self::play_dry(ctx, SPLICE_L4, index);
        }
    }

    /// `LandGround` 0x8253f760.
    fn land_ground(&mut self, physics: &PlayerPhysics, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        self.land_tick = 0;
        ctx.set_input(io, 1, 0x7FFF);
        if ctx.local {
            ctx.release(&mut self.land_l4);
            let index = Self::l4_index(ctx, 0x7F43_C121_E500_4704, 0x0EF1_B78D_73B4_60F3);
            self.land_l4 = Self::play_dry(ctx, SPLICE_L4, index);
            ctx.set_input(io, 10, 0x7FFF);
        }
        let mut wheel_material = MATERIAL_MAX;
        for wheel in 0..4 {
            if physics.landed[wheel] {
                wheel_material = physics.wheel_material[wheel];
                if wheel_material != MATERIAL_MAX {
                    break;
                }
            }
        }
        let fraction = clamp_unit(self.time_in_air / ctx.vault.f32(CONTACTS, 0x6D68_BC2D_1A23_C29A));
        if collision::has_land_sound(ctx.vault, wheel_material) {
            let land_material = ctx.vault.i32(CONTACTS, 0x85FD_C8BF_696B_CA5C) as u32;
            let mut level = LEVEL_L1;
            let mut low = 0.0;
            let mut high = ctx.vault.f32(CONTACTS, 0x3462_CBB1_6DCA_696E);
            if fraction >= ctx.vault.f32(CONTACTS, 0x3462_CBB1_6DCA_696E) {
                level = LEVEL_L3;
                low = ctx.vault.f32(CONTACTS, 0x3462_CBB1_6DCA_696E);
                high = ctx.vault.f32(CONTACTS, 0x5904_95E4_20B3_99E5);
            }
            let wheel_volume = collision::collision_volume(ctx.vault, wheel_material, land_material, level, low, high, fraction, false);
            let land_volume = collision::collision_volume(ctx.vault, land_material, wheel_material, level, low, high, fraction, false);
            if wheel_volume != 0 && land_volume != 0 {
                let first = fctiwz(wheel_volume as f32 * ctx.vault.f32(CONTACTS, 0x31DE_EF8F_A219_950F));
                let second = fctiwz(land_volume as f32 * ctx.vault.f32(CONTACTS, 0x0EC6_EEF5_366F_EA85));
                ctx.collisions.push(CollisionInfo {
                    material: [land_material, wheel_material],
                    severity: [level, level],
                    position: physics.board_position,
                    volume: [first, second],
                    dmo_collision: false,
                });
            }
            if ctx.local {
                ctx.set_input(io, 7, 0x7FFF);
            }
        }
        if physics.skate3_dark_catch || self.skate3_dark_catch_recent {
            self.land_skate3_dark_catch(physics, ctx);
        }
        let mut count = 0;
        let mut impact = 0;
        for wheel in 0..4 {
            if physics.landed[wheel] {
                count += 1;
                if physics.impact_level[wheel] > impact {
                    impact = physics.impact_level[wheel];
                }
            }
        }
        let value = match impact {
            _ if count <= 0 => 0,
            1 => 16000,
            2 => 0x7FFF,
            _ => 0,
        };
        ctx.set_input(io, 2, value);
    }

    /// `LandRail` 0x8251d448.
    fn land_rail(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !(self.time_since_land_buffer <= 0.0) {
            return;
        }
        let mut material = ctx.cvars.debug_audio_material;
        if material == MATERIAL_MAX {
            material = MATERIAL_METAL_SOLID_ROUND_1;
            if physics.grind_material != MATERIAL_MAX {
                material = physics.grind_material;
            }
        }
        let impactor = if physics.grind_type == GRIND_BOARDSLIDE || physics.grind_type == GRIND_TIPSLIDE { MATERIAL_BOARD } else { MATERIAL_TRUCK };
        let impact = physics.grind_impact;
        let mut level = LEVEL_L1;
        let mut low = 0.0;
        let mut high = ctx.vault.f32(RAILS, 0x086B_66C3_D4FF_EE8F);
        if impact > high {
            level = LEVEL_L3;
            low = ctx.vault.f32(RAILS, 0x086B_66C3_D4FF_EE8F);
            high = ctx.vault.f32(RAILS, 0xB2AC_AFDB_CD96_3C93);
        }
        let first = collision::collision_volume(ctx.vault, impactor, material, level, low, high, impact, false);
        let second = collision::collision_volume(ctx.vault, material, impactor, level, low, high, impact, false);
        ctx.collisions.push(CollisionInfo { material: [impactor, material], severity: [level, level], position: physics.position, volume: [first, second], dmo_collision: false });
        self.time_since_land_buffer = 0.5;
    }

    /// `CheckForJumpsAndLands` 0x8254b4d0.
    fn check_for_jumps_and_lands(&mut self, physics: &PlayerPhysics, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        self.land_tick += 1;
        if physics.in_air != self.last_in_air {
            trace_skate3_darkcatch(format_args!("in air {} trick {} performing {} rail {}", physics.in_air, physics.trick_type,
                physics.is_performing_trick, physics.is_rail_grind));
        }
        ctx.set_input(io, 0, 0);
        ctx.set_input(io, 1, 0);
        ctx.set_input(io, 10, 0);
        ctx.set_input(io, 7, 0);
        let in_air = physics.in_air;
        let rail = physics.is_rail_grind;
        if !self.last_in_air && in_air {
            let trick = physics.trick_type;
            if physics.is_performing_trick {
                let launch = !matches!(trick, TRICK_INVALID | TRICK_BACKSIDE_FOOT_PLANT | TRICK_FOOT_PLANT | TRICK_GRAB | TRICK_COFFIN);
                if launch && !self.last_in_grind {
                    self.take_off_ground(trick, physics, io, ctx);
                }
            }
        } else if self.last_in_air && !in_air && !rail {
            self.land_ground(physics, io, ctx);
        }
        if in_air {
            self.time_in_air = physics.time_in_air;
        }
        self.last_in_air = in_air;
        self.was_wiping_out = physics.is_wiping_out;
        if rail && !self.last_in_grind {
            self.land_rail(physics, ctx);
        }
        self.last_in_grind = rail;
    }

    /// The foot drag's speed: `(speed - minimum) / range * 3.6` clamped to 0..1, in 1/10000s.
    fn foot_drag_speed(physics: &PlayerPhysics, minimum: f32, range: f32, dismount: bool) -> i32 {
        let speed = fctiwz(clamp_unit(((physics.board_speed - minimum) / range) * f32::from_bits(0x4066_6666)) * 10000.0);
        if dismount { 500 } else { speed }
    }

    /// `CheckForStops` 0x8253fea8 (creates the `Class_foot_drag` instance).
    fn check_for_stops(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let power_slide = Self::play_drag_for_power_slide(physics, ctx);
        let dismount = ctx.local && physics.is_dismount_contact;
        if !(physics.is_foot_braking || physics.is_manual_braking || power_slide || dismount) || self.foot_drag.is_some() {
            return;
        }
        let range = ctx.vault.f32(CONTACTS, 0xB2C8_1577_8204_08BE);
        let minimum = ctx.vault.f32(CONTACTS, 0xE5A6_D8AC_6EB9_B5AB);
        let speed = Self::foot_drag_speed(physics, minimum, range, dismount);
        let parts = i32::from(!(physics.is_foot_braking || power_slide));
        let loop_volume = ctx.vault.i32(CONTACTS, 0x9FF8_8541_CAB4_61C7);
        let exit_slow = ctx.vault.i32(CONTACTS, 0x6242_BC04_80B0_9A33);
        let exit_fast = ctx.vault.i32(CONTACTS, 0x194E_F41D_A225_4153);
        let enter = ctx.vault.i32(CONTACTS, 0x9717_1DE6_035D_6069);
        let surface = Self::foot_drag_surface(physics, physics.is_manual_braking, ctx);
        self.foot_drag = Some(FootDragData {
            speed: speed.clamp(0, 10000),
            volume: 0,
            dry: 0x7FFF,
            wet: 0,
            low_pass: 25000,
            high_pass: 0,
            azimuth: 0,
            pitch: 0,
            surface: (surface as i32).clamp(0, 10),
            enter_volume: enter.clamp(0, 0x7FFF),
            exit_fast_volume: exit_fast.clamp(0, 0x7FFF),
            exit_slow_volume: exit_slow.clamp(0, 0x7FFF),
            loop_volume: loop_volume.clamp(0, 0x7FFF),
            parts,
        });
    }

    /// `UpdateStops` 0x8251f7d0.
    fn update_stops(&mut self, physics: &PlayerPhysics, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        let Some(data) = self.foot_drag else { return };
        let power_slide = Self::play_drag_for_power_slide(physics, ctx);
        let dismount = ctx.local && physics.is_dismount_contact;
        if !(physics.is_foot_braking || physics.is_manual_braking || power_slide || dismount) {
            self.foot_drag = None;
            return;
        }
        let range = ctx.vault.f32(CONTACTS, 0xB2C8_1577_8204_08BE);
        let speed = Self::foot_drag_speed(physics, 0.5, range, dismount);
        let dry = if physics.is_foot_braking || power_slide { 4 } else { 5 };
        self.foot_drag = Some(FootDragData {
            speed: speed.clamp(0, 10000),
            volume: 0x7FFF,
            dry: volume(ctx, io, dry).clamp(0, 0x7FFF),
            wet: volume(ctx, io, 20).clamp(0, 0x7FFF),
            low_pass: ctx.output(io, 18, PresetType::Frequency).clamp(0, 25000),
            high_pass: ctx.output(io, 19, PresetType::Frequency).clamp(0, 25000),
            azimuth: ctx.output(io, 0, PresetType::Azimuth).clamp(0, 0x10000),
            pitch: ctx.output(io, 1, PresetType::Pitch).clamp(0, 0x2000),
            surface: (Self::foot_drag_surface(physics, physics.is_manual_braking, ctx) as i32).clamp(0, 10),
            ..data
        });
    }

    /// `UpdatePushes` 0x8251fb70.
    fn update_pushes(&mut self, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        for push in [&mut self.push_on, &mut self.push_off] {
            if push.splice.is_none() {
                continue;
            }
            let level = fctiwz(volume(ctx, io, 6) as f32 * ctx.vault.f32_at(CONTACTS, 0x4A3C_2146_8E49_D38F, push.surface));
            Self::mix_update(ctx, io, &mut push.splice, level, 1, 0.0);
        }
    }

    /// TU3 0x824bebd8, first in its `ProcessUpdate`: each Skate 3 catch follows Skate 3's contacts
    /// mixer (object 13 here) until it ends: volume output 8, pitch output 1, the reverb send at
    /// output 21.
    fn update_skate3_foot_catches(&mut self, ctx: &mut SfxCtx<'_>) {
        let io = self.skate3_io;
        for sound in &mut self.skate3_foot_catch {
            follow(sound, io, 8, 1, 21, ctx);
        }
    }

    /// `UpdateFootCatches` 0x8251f4c0.
    fn update_foot_catches(&mut self, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        for foot in 0..3 {
            if self.foot_catch[foot].is_some() {
                let level = volume(ctx, io, 8);
                Self::mix_update(ctx, io, &mut self.foot_catch[foot], level, 1, 0.0);
            }
        }
    }

    /// `UpdateFootScrapes` 0x8251ff00.
    fn update_foot_scrapes(&mut self, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        for foot in 0..2 {
            if self.foot_scrape[foot].is_some() {
                let level = volume(ctx, io, 9);
                Self::mix_update(ctx, io, &mut self.foot_scrape[foot], level, 1, 0.0);
            }
        }
    }

    /// `UpdateManual` 0x8251f640.
    fn update_manual(&mut self, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        if self.out_of_manual.splice.is_none() {
            return;
        }
        let level = Self::vol_for_wheel_hit(io, WHEELS_TICK, HIT_ONE, self.out_of_manual.land_type, ctx);
        if !Self::mix_update(ctx, io, &mut self.out_of_manual.splice, level, 1, 0.0) {
            Self::free_voiced(ctx, &mut self.out_of_manual);
            self.out_of_manual.land_type = LAND_CONCRETE_HARD;
        }
    }

    /// `UpdateJumpsAndLands` 0x825404d8.
    fn update_jumps_and_lands(&mut self, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        let spread = local_spread(ctx);
        if self.ollie.splice.is_some() {
            let key = if self.ollie.land_type == OL_CONCRETE { 0x8BF3_668C_F799_4CD4 } else { 0x212B_6F75_D11A_A39B };
            let level = fctiwz(volume(ctx, io, 2) as f32 * ctx.vault.f32_at(CONTACTS, key, self.ollie.level));
            if !Self::mix_update(ctx, io, &mut self.ollie.splice, level, 1, spread) {
                Self::free_voiced(ctx, &mut self.ollie);
                self.ollie.level = 0;
                self.ollie.land_type = OL_CONCRETE;
            }
        }
        if self.scrape.is_some() {
            let level = volume(ctx, io, 7);
            Self::mix_update(ctx, io, &mut self.scrape, level, 1, 0.0);
        }
        for slot in 0..4 {
            if self.land[slot].splice.is_none() {
                continue;
            }
            let land = self.land[slot];
            let level = Self::vol_for_wheel_hit(io, land.num_wheels, land.level, land.land_type, ctx);
            if !Self::mix_update(ctx, io, &mut self.land[slot].splice, level, 1, spread) {
                Self::free_voiced(ctx, &mut self.land[slot]);
                self.land[slot] = VoicedSplice::IDLE;
            }
        }
        if self.sweetener.splice.is_some() {
            let level = Self::vol_for_wheel_hit(io, self.sweetener.num_wheels, HIT_THREE, self.sweetener.land_type, ctx);
            if !Self::mix_update(ctx, io, &mut self.sweetener.splice, level, 1, spread) {
                Self::free_voiced(ctx, &mut self.sweetener);
                self.sweetener = VoicedSplice::IDLE;
            }
        }
        if self.ollie_l4.is_some() {
            let level = volume(ctx, io, 14);
            Self::mix_update(ctx, io, &mut self.ollie_l4, level, 1, spread);
        }
        if self.land_l4.is_some() {
            let level = volume(ctx, io, 15);
            Self::mix_update(ctx, io, &mut self.land_l4, level, 1, spread);
        }
        // TU3 0x824be900: the darkcatch landing follows Skate 3's contacts mixer (object 13 here)
        // until it ends: volume output 3, pitch output 1, the reverb send at output 15.
        follow(&mut self.skate3_dark_catch_land, self.skate3_io, 3, 1, 15, ctx);
        if self.time_since_land_buffer > 0.0 {
            self.time_since_land_buffer -= ctx.delta_time;
        }
    }

    /// `UpdateHandGrabs` 0x825201d8: element 0 of the grab volumes scales the grab, element 1 the
    /// release; the pitch is mixer output 13.
    fn update_hand_grabs(&mut self, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        for hand in 0..2 {
            for (grabs, element) in [(&mut self.hand_grabs_on, 0), (&mut self.hand_grabs_off, 1)] {
                if grabs[hand].splice.is_none() {
                    continue;
                }
                let level = fctiwz(volume(ctx, io, 12) as f32 * ctx.vault.f32_at(HANDGRAB, 0x25DC_B888_413D_570D, element));
                Self::mix_update(ctx, io, &mut grabs[hand].splice, level, 13, 0.0);
            }
        }
    }

    /// `CheckForKickUp` 0x82520cd8.
    fn check_for_kick_up(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if self.kickup.is_some() || !physics.is_dismount_contact {
            return;
        }
        let mut material = OL_CONCRETE;
        if physics.wheel_material[0] < MATERIAL_MAX {
            material = ollie_and_land_material(ctx.vault, physics.wheel_material[0]);
        }
        let bank = if material == OL_WOOD { SPLICE_OLLIES_WOOD } else { SPLICE_OLLIES_CONCRETE };
        let concrete = bank == SPLICE_OLLIES_CONCRETE;
        let speed = physics.board_speed * f32::from_bits(0x4066_6666);
        let value = |key| ctx.vault.i32(KICKUP, key);
        let mut index = value(if concrete { 0xC8AD_6743_FE48_41F3 } else { 0x102F_3F03_B6B8_F544 });
        if speed > ctx.vault.f32(KICKUP, 0xF60C_C341_DB01_452A) {
            index = value(if concrete { 0x47CC_07A8_59D6_A8C5 } else { 0x72B9_F4DE_2D0A_DC07 });
        } else if speed > ctx.vault.f32(KICKUP, 0x61AE_1666_FE7D_F49D) {
            index = value(if concrete { 0x0F52_513C_0394_2C24 } else { 0xF3C8_2D6F_8211_1158 });
        }
        self.kickup = Self::play_dry(ctx, bank, index);
    }

    /// `UpdateKickUp` 0x82520e98.
    fn update_kick_up(&mut self, physics: &PlayerPhysics, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        let Some(id) = self.kickup else { return };
        if ctx.is_playing(id) {
            let level = volume(ctx, io, 23);
            let spread = local_spread(ctx);
            Self::mix_update(ctx, io, &mut self.kickup, level, 1, spread);
        } else if !physics.is_dismount_contact {
            ctx.release(&mut self.kickup);
        }
    }

    /// `CheckForAirDismount` 0x82521048.
    fn check_for_air_dismount(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if self.dismount.is_some() || !physics.is_air_dismount {
            return;
        }
        let index = Self::splice_util_index(ctx, FOOTCATCH, 0x19FA_E352_5831_7C7C, CATCH_2ND_FOOT, 1);
        self.dismount = Self::play_dry(ctx, SPLICE_FOOT_CATCH, index);
    }

    /// `UpdateAirDismount` 0x82521108.
    fn update_air_dismount(&mut self, physics: &PlayerPhysics, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        let Some(id) = self.dismount else { return };
        if ctx.is_playing(id) {
            let level = volume(ctx, io, 24);
            let spread = local_spread(ctx);
            Self::mix_update(ctx, io, &mut self.dismount, level, 1, spread);
        } else if !physics.is_air_dismount {
            ctx.release(&mut self.dismount);
        }
    }

    /// `AddToBailAverage` 0x82514248.
    fn add_to_bail_average(&mut self, value: f32) {
        if self.num_bails != 0 {
            let count = self.num_bails;
            self.num_bails = count.wrapping_add(1);
            self.cur_bail_average = (count as f32).mul_add(self.cur_bail_average, value) / count.wrapping_add(1) as f32;
        } else {
            self.cur_bail_average = value;
            self.num_bails = 1;
        }
        if !(self.cur_bail_average <= self.cur_bail_average_max) {
            self.cur_bail_average_max = self.cur_bail_average;
        }
        if !(value <= self.cur_max_bail) {
            self.cur_max_bail = value;
        }
    }

    /// `UpdateBailAverage` 0x825212b8 (input 11).
    fn update_bail_average(&mut self, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        if !ctx.bail {
            self.cur_bail_average = 0.0;
            self.cur_bail_average_max = 0.0;
            self.cur_max_bail = 0.0;
            self.num_bails = 0;
        }
        let value = (self.cur_bail_average_max / self.max_cur_bail_average) * 32767.0;
        let low = if -value >= 0.0 { 0.0 } else { value };
        let clamped = if 32767.0 - low >= 0.0 { low } else { 32767.0 };
        ctx.set_input(io, 11, fctiwz(clamped).clamp(0, 0x7FFF));
    }

    /// `GetHOMMaterial` 0x82520920.
    fn hom_material(material: u32, ctx: &SfxCtx<'_>) -> u32 {
        if !ctx.hall_of_meat {
            return MATERIAL_MAX;
        }
        match material {
            MATERIAL_HEAD => MATERIAL_HOM_HEAD,
            MATERIAL_TORSO => MATERIAL_HOM_TORSO,
            MATERIAL_LEG => MATERIAL_HOM_LEG,
            MATERIAL_ARM => MATERIAL_HOM_ARM,
            MATERIAL_FOOT => MATERIAL_HOM_FOOT,
            _ => MATERIAL_MAX,
        }
    }

    /// `GetImpactorMaterials` 0x82540328: the part's (material, cloth, sweetener, Hall of Meat)
    /// materials, each `MATERIAL_MAX` unless set.
    fn impactor_materials(&mut self, part: usize, force: f32, physics: &PlayerPhysics, ctx: &SfxCtx<'_>) -> (u32, u32, u32, u32) {
        let (material, cloth, sweetener) = match part {
            0 => {
                if force > ctx.vault.f32(SPEECH, 0xFC2C_ACAA_802A_674F) && physics.is_wiping_out {
                    self.was_head_hit_during_wipeout = true;
                }
                let sweetener = if physics.skeleton_collision_info.is_face_contact { MATERIAL_FACE_HIT } else { MATERIAL_BONE_CRACK };
                (MATERIAL_HEAD, MATERIAL_MAX, sweetener)
            }
            1 => (MATERIAL_TORSO, MATERIAL_COTTON, MATERIAL_BONE_CRACK),
            2 | 3 => (MATERIAL_ARM, MATERIAL_SKIN, MATERIAL_BONE_SNAP),
            4 | 5 => (MATERIAL_LEG, MATERIAL_DENIM, MATERIAL_BONE_SNAP),
            _ => return (MATERIAL_MAX, MATERIAL_MAX, MATERIAL_MAX, MATERIAL_MAX),
        };
        (material, cloth, sweetener, Self::hom_material(material, ctx))
    }

    /// `GetSweetenerLevel` 0x8251e128: (level, low, high), left as given when the force is too low.
    fn sweetener_level(&self, material: u32, sweetener: u32, force: f32, ctx: &SfxCtx<'_>) -> (u32, f32, f32) {
        let (mut level, mut low, mut high) = (LEVEL_NUM, 0.0, 0.0);
        match sweetener {
            MATERIAL_HOM_HEAD..=MATERIAL_HOM_FOOT => {
                let (found, range) = collision::collision_level(ctx.vault, sweetener, force, false);
                level = found;
                if let Some(range) = range {
                    (low, high) = range;
                }
            }
            MATERIAL_BONE_CRACK => {
                if material == MATERIAL_HEAD {
                    if force > self.bone_crack_small.0 {
                        (level, low, high) = (LEVEL_L3, self.bone_crack_small.0, self.bone_crack_small.1);
                    }
                } else if material == MATERIAL_TORSO && force > self.bone_crack_big.0 {
                    (level, low, high) = (LEVEL_L3, self.bone_crack_big.0, self.bone_crack_big.1);
                }
            }
            MATERIAL_BONE_SNAP => {
                if force > self.bone_snap.0 {
                    (level, low, high) = (LEVEL_L3, self.bone_snap.0, self.bone_snap.1);
                }
            }
            MATERIAL_FACE_HIT => {
                if force <= self.face_hit_l3.0 {
                    if force > self.face_hit_l1.0 {
                        (level, low, high) = (LEVEL_L1, self.face_hit_l1.0, self.face_hit_l1.1);
                    }
                } else {
                    (level, low, high) = (LEVEL_L3, self.face_hit_l3.0, self.face_hit_l3.1);
                }
            }
            _ => {}
        }
        (level, low, high)
    }

    /// `CanPlayCollisionSound` 0x825209e8: the Hall of Meat priority gate (`aud_general` "hom").
    fn can_play_collision_sound(&mut self, material: u32, force: f32, ctx: &SfxCtx<'_>) -> bool {
        let (level, _) = collision::collision_level(ctx.vault, material, force, false);
        let keys: [u64; 3] = match material {
            MATERIAL_HOM_HEAD => [0xB4A6_D4A2_CC75_7376, 0x56AF_65D1_AE1D_909B, 0x912B_C24A_5B7F_7046],
            MATERIAL_HOM_TORSO => [0xECEF_CD21_30A6_592E, 0x2DA4_BE8F_EA60_1644, 0x76D3_7B4D_28BE_AC96],
            MATERIAL_HOM_LEG => [0xC684_7E75_A057_8858, 0xBC3D_3839_E518_3F2D, 0x6D61_1AE4_C850_69CE],
            MATERIAL_HOM_ARM => [0xF811_103F_09B2_7308, 0x57FA_FA6B_3007_2EFD, 0xFF95_1BDC_7A5B_74C5],
            MATERIAL_HOM_FOOT => [0xFD71_320D_C465_809D, 0x1A61_F1CC_F11D_BC89, 0xDD26_61F8_5B3F_F040],
            _ => return true,
        };
        if level == LEVEL_NUM {
            return false;
        }
        let key = keys[match level {
            LEVEL_L1 => 0,
            LEVEL_L3 => 1,
            _ => 2,
        }];
        let priority = (ctx.vault.struct_word(HOM, key, 0, 0) as i32, ctx.vault.struct_word(HOM, key, 0, 1) as i32);
        if priority.0 < 0 {
            return true;
        }
        let play = priority.0 > self.current_priority.0 || self.current_priority.1 <= 0;
        if play {
            self.current_priority = priority;
        }
        play
    }

    /// `CheckForFirstCollision` 0x8251e2d0: the first collision of a bail adds a torso hit.
    #[allow(clippy::too_many_arguments)]
    fn check_for_first_collision(&mut self, part: usize, force: f32, material: u32, level: u32, low: f32, high: f32, physics: &PlayerPhysics, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        if self.first_collision {
            ctx.set_input(io, 8, 0x7FFF);
        }
        if ctx.vault.bool(COLLISIONS, 0xC54A_6BF7_755A_A7D7) && self.first_collision && part != 1 && level as i32 >= 1 {
            let volume = collision::collision_volume(ctx.vault, MATERIAL_TORSO, material, level, low, high, force, false);
            ctx.collisions.push(CollisionInfo {
                material: [MATERIAL_TORSO, MATERIAL_MAX],
                severity: [LEVEL_L3, LEVEL_L3],
                position: physics.position,
                volume: [volume, 0],
                dmo_collision: false,
            });
        }
        self.first_collision = false;
    }

    /// The collision of `first` against `second` and, for an L4 side, its L3 copy ducked by
    /// `GetL3VolumeDuck` (`CheckForBodyCollisions`, `CheckForBoardCollisions`).
    fn push_collision_pair(ctx: &mut SfxCtx<'_>, materials: [u32; 2], levels: [u32; 2], ranges: [(f32, f32); 2], force: f32, position: crate::vmx::Vec3) {
        let first = collision::collision_volume(ctx.vault, materials[0], materials[1], levels[0], ranges[0].0, ranges[0].1, force, false);
        let second = collision::collision_volume(ctx.vault, materials[1], materials[0], levels[1], ranges[1].0, ranges[1].1, force, false);
        ctx.collisions.push(CollisionInfo { material: materials, severity: levels, position, volume: [first, second], dmo_collision: false });
        let ducked = levels.map(|level| if level == LEVEL_L4 { LEVEL_L3 } else { LEVEL_NUM });
        if ducked[0] != LEVEL_NUM || ducked[1] != LEVEL_NUM {
            let first = fctiwz(collision::l3_volume_duck(ctx.vault, materials[0], false) * first as f32);
            let second = fctiwz(collision::l3_volume_duck(ctx.vault, materials[1], false) * second as f32);
            ctx.collisions.push(CollisionInfo { material: materials, severity: ducked, position, volume: [first, second], dmo_collision: false });
        }
    }

    /// `CheckForBoardCollisions` 0x8251e448.
    fn check_for_board_collisions(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let force = physics.deck_contact_force;
        if force > 0.0 && self.board_collision_counter <= 0.0 {
            let impactor = if physics.is_off_board || physics.is_wiping_out { MATERIAL_BOARD_TUMBLE } else { MATERIAL_BOARD };
            let mut material = ctx.cvars.debug_audio_material;
            if material == MATERIAL_MAX {
                material = physics.deck_material;
            }
            let (impactor_level, impactor_range) = collision::collision_level(ctx.vault, impactor, force, false);
            let (material_level, material_range) = collision::collision_level(ctx.vault, material, force, false);
            if impactor_level != LEVEL_NUM || material_level != LEVEL_NUM {
                self.board_collision_counter = ctx.vault.i32(COLLISIONS, 0x27D3_C5DC_3282_B59D) as f32;
                if ctx.vault.bool(COLLISIONS, 0xD579_39CC_595F_80E6) {
                    let ranges = [impactor_range.unwrap_or((0.0, 0.0)), material_range.unwrap_or((0.0, 0.0))];
                    Self::push_collision_pair(ctx, [impactor, material], [impactor_level, material_level], ranges, force, physics.board_position);
                }
            }
        }
        if self.board_collision_counter > 0.0 {
            let speed = if physics.game_speed > 1.0 { 1.0 } else { physics.game_speed };
            self.board_collision_counter -= speed;
        }
    }

    /// `CheckForCollisionSpeech` 0x8251e788 without its speech: the wipe-out bookkeeping.
    fn check_for_collision_speech(&mut self, physics: &PlayerPhysics, ctx: &SfxCtx<'_>) {
        let wiping_out = physics.is_wiping_out;
        if wiping_out && !self.was_wipe_out {
            self.check_for_wipe_out_over = true;
            self.was_groin_hit_during_wipeout = false;
            self.was_vehicle_hit_during_wipeout = self.vehicle_collision > 0;
            self.was_head_hit_during_wipeout = false;
            self.was_lw_hit_during_wipeout = false;
            self.was_npc_hit_during_wipeout = self.npc_collision > 0;
        }
        if self.check_for_wipe_out_over && physics.is_wipe_out_over {
            if ctx.local {
                self.bail_level_count = [0; 3];
            }
            self.check_for_wipe_out_over = false;
        }
        self.was_wipe_out = wiping_out;
    }

    /// `CheckForBodyCollisions` 0x8254b680.
    fn check_for_body_collisions(&mut self, physics: &PlayerPhysics, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        ctx.set_input(io, 8, 0);
        let severity = if ctx.bail { fctiwz(ctx.bail_severity * 32767.0).clamp(0, 0x7FFF) } else { 0 };
        ctx.set_input(io, 9, severity);
        if ctx.bail && !self.in_bail_last_update {
            self.first_collision = true;
        }
        self.in_bail_last_update = ctx.bail;
        let wiping_out = physics.is_wiping_out;
        if wiping_out {
            if physics.is_wipe_out_over {
                return;
            }
        } else {
            self.first_grunt = true;
        }
        for part in 0..SKELETON_COLLISION_LEFTFOOT {
            let force = physics.skeleton_collision_info.contact_force[part];
            if force > 0.0 && self.body_part_collision_counter[part] <= 0.0 {
                self.body_part_collision(part, force, wiping_out, physics, io, ctx);
            }
            if self.body_part_collision_counter[part] > 0.0 {
                let speed = if physics.game_speed > 1.0 { 1.0 } else { physics.game_speed };
                self.body_part_collision_counter[part] -= speed;
            }
        }
        if self.vehicle_collision > 0 {
            self.vehicle_collision -= 1;
        }
        if self.npc_collision > 0 {
            self.npc_collision -= 1;
        }
    }

    /// One body part of `CheckForBodyCollisions` that touched something.
    fn body_part_collision(&mut self, part: usize, force: f32, wiping_out: bool, physics: &PlayerPhysics, io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) {
        let info = &physics.skeleton_collision_info;
        trace_skate3_darkcatch(format_args!("body part {part} force {force} surface {} wiping out {wiping_out} in air {}",
            info.audio_surface_type[part], physics.in_air));
        if info.is_groin_contact && wiping_out {
            self.was_groin_hit_during_wipeout = true;
        }
        if info.vehicle_impact > ctx.vault.f32(SPEECH, 0x7503_A1AD_356B_FCCD) {
            if wiping_out {
                self.was_vehicle_hit_during_wipeout = true;
            }
            self.vehicle_collision = ctx.cvars.collision_frame_decay;
        }
        if info.skater_impact > ctx.vault.f32(SPEECH, 0x359E_C8EF_5259_8AA8) {
            if wiping_out {
                self.was_npc_hit_during_wipeout = true;
            }
            self.npc_collision = ctx.cvars.collision_frame_decay;
        }
        let (material, cloth, sweetener, hom) = self.impactor_materials(part, force, physics, ctx);
        let mut other = ctx.cvars.debug_audio_material;
        if other == MATERIAL_MAX {
            other = audio_material(info.audio_surface_type[part]);
        }
        let (level, range) = collision::collision_level(ctx.vault, material, force, false);
        let range = range.unwrap_or((0.0, 0.0));
        if ctx.local && level < 3 {
            self.bail_level_count[level as usize] += 1;
        }
        let (other_level, other_range) = collision::collision_level(ctx.vault, other, force, false);
        let other_range = other_range.unwrap_or((0.0, 0.0));
        if ctx.local && other_level < 3 {
            self.bail_level_count[other_level as usize] += 1;
        }
        if level == LEVEL_NUM && other_level == LEVEL_NUM {
            return;
        }
        self.body_part_collision_counter[part] = ctx.vault.i32(COLLISIONS, 0x6DD8_5F43_C1B6_E6AA) as f32;
        let can_play = self.can_play_collision_sound(hom, force, ctx);
        if can_play || !ctx.cvars.prioritize_base_sounds {
            if ctx.vault.bool(COLLISIONS, 0xA3FC_4095_A221_1F9E) {
                Self::push_collision_pair(ctx, [material, other], [level, other_level], [range, other_range], force, physics.position);
                if level as i32 >= ctx.cvars.skater_grunt_impact_level && self.first_grunt && physics.is_wiping_out {
                    self.first_grunt = false;
                }
                self.add_to_bail_average(force);
            }
            if ctx.vault.bool(COLLISIONS, 0x290C_EF1F_5A45_2241) && cloth != MATERIAL_MAX {
                let volume = collision::collision_volume(ctx.vault, material, other, LEVEL_L1, range.0, range.1, force, false);
                ctx.collisions.push(CollisionInfo {
                    material: [cloth, MATERIAL_MAX],
                    severity: [LEVEL_L1, LEVEL_L1],
                    position: physics.position,
                    volume: [volume, 0],
                    dmo_collision: false,
                });
            }
        }
        if sweetener != MATERIAL_MAX && (can_play || !ctx.cvars.prioritize_base_sounds) {
            let (sweetener_level, low, high) = self.sweetener_level(material, sweetener, force, ctx);
            let past = sweetener_level as i32 >= 1 && force > high;
            if !past && ctx.vault.bool(COLLISIONS, 0xB90E_51F0_BD83_73FB) && sweetener_level != LEVEL_NUM {
                let volume = collision::collision_volume(ctx.vault, sweetener, other, sweetener_level, low, high, force, false);
                ctx.collisions.push(CollisionInfo {
                    material: [sweetener, MATERIAL_MAX],
                    severity: [sweetener_level, LEVEL_L1],
                    position: physics.position,
                    volume: [volume, 0],
                    dmo_collision: false,
                });
            }
        }
        if hom != MATERIAL_MAX && (can_play || !ctx.cvars.prioritize_hom_sounds) {
            let (hom_level, low, high) = self.sweetener_level(material, hom, force, ctx);
            if hom_level != LEVEL_NUM {
                let volume = collision::collision_volume(ctx.vault, hom, other, hom_level, low, high, force, false);
                ctx.collisions.push(CollisionInfo {
                    material: [hom, MATERIAL_MAX],
                    severity: [hom_level, LEVEL_L1],
                    position: physics.position,
                    volume: [volume, 0],
                    dmo_collision: false,
                });
            }
        }
        self.check_for_first_collision(part, force, other, level, range.0, range.1, physics, io, ctx);
    }
}

/// `SKATE2_TRACE_SKATE3_DARKCATCH`: the Skate 3 darkcatch cues as they play.
fn trace_skate3_darkcatch(message: std::fmt::Arguments<'_>) {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_DARKCATCH").is_some()) {
        eprintln!("skate 3 darkcatch audio: {message}");
    }
}
