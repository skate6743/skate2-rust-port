//! `Sk8::Audio::SFXObj_Tricks` (76 bytes, PLAYER object 5): the local skater's board flip whoosh
//! (`Class_Flips`, `Sk8_Air_Flip_Tricks.abk`) and the trick and trick-done cloth foley
//! (`cloth_trick`, `Foley_Cloth.abk`), with the fun meter the flip instance reads.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use super::physics::{PlayerPhysics, INVALID_AUDIO_TRICK};
use super::SfxCtx;
use crate::aems::ClassObject;
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, PresetType};
use crate::vault::{class, Collection, Vault};

/// `GlobalAttributeCollection`'s `aud_general` clothing (+0x58) and tricks (+0x64) collections, and
/// the `aud_general` collection `UpdateFunMeter` finds.
const CLOTHING: Collection = Collection::new(class::AUD_GENERAL, 0x3982_76A6_3764_95EE);
const TRICKS: Collection = Collection::new(class::AUD_GENERAL, 0x1FA8_AC00_6CAB_EF59);
const FUN_METER: Collection = Collection::new(class::AUD_GENERAL, 0x47EC_76B4_F9FC_79F6);

const PLAY_CLOTH_FOLEY: u64 = 0xF55E_AEC1_E5C8_4069;
const PLAY_BOARD_FLIPS: u64 = 0x5833_7856_1673_B34E;
/// `aud_general::MinInputX/Y/Z`, `MaxValueX/Y/Z`, `BoardFlipsVolX/Y/Z`, `TrickVol`.
const MIN_INPUT: [u64; 3] = [0x5C73_CF6A_0D50_C8D8, 0xEE15_7886_DE5D_3C97, 0x9A03_1662_5B63_CD99];
const MAX_VALUE: [u64; 3] = [0x1494_BB20_854C_155C, 0x02D3_9586_635F_B1A3, 0x8EDB_ACCB_A6FE_46AD];
const BOARD_FLIPS_VOL: [u64; 3] = [0x13C1_55A1_81A5_5BA4, 0xD2D0_EBAC_4384_2F6D, 0x99E6_FF02_4834_E4C7];
const TRICK_VOL: u64 = 0xB565_D4D7_6312_8252;
/// The fun meter targets while the momentum multiplier is 1.5, 2 (`Times_Two_Fun`) and 3
/// (`Times_Three_Fun`), and the per-second fall and rise rates.
const FUN: [u64; 3] = [0xB601_DFAB_3AF7_DE66, 0x0844_1EA8_E801_9665, 0x36F8_D124_86A9_29D1];
const FUN_FALL_RATE: u64 = 0x6B57_BD44_C0E0_B267;
const FUN_RISE_RATE: u64 = 0x57AE_D5FB_C374_D8F1;

/// The trick a dismount contact plays, and the two tricks without a flip.
const TRICK_DISMOUNT: i32 = 0x22;
const NO_FLIP_TRICKS: [i32; 2] = [0x23, 0x24];

/// `Csis::Class_Flips::Class_Flips` 0x825137a0: each member's clamp.
const FLIPS_RANGES: [i32; 27] = [
    0x7FFF, 0x7FFF, 0x7FFF, 0x1_0000, 0x2000, 25000, 25000, 1000, 1000, 1000, 1000, 40, 10000, 1000, 1, 1, 2, 1, 1, 0x7FFF,
    0x7FFF, 0x7FFF, 0x7FFF, 0x7FFF, 0x7FFF, 1, 0x7FFF,
];

/// PC port: a Skate 3-only trick (a darkcatch) flips with Skate 3's sounds, as TU3 plays its audio
/// trick type: `Sk8_Air_Flip_Tricks.abk` and its interface from the linked Skate 3 data, kept
/// under these names behind Skate 2's.
const SKATE3_INTERFACE: &str = "skate3:SK8_AEMS_skateboard.csi";
const SKATE3_BANK: &str = "skate3:Sk8_Air_Flip_Tricks.abk";
const SKATE3_FILES: [&str; 2] = ["SK8_AEMS_skateboard.csi", "Sk8_Air_Flip_Tricks.abk"];
/// TU3 `CheckForBoardFlips` 0x824cbfb8's last `Class_Flips` member: `aud_senseofspeed`'s
/// 0xD9BE1F2F1A72FEE8.
const SENSE_OF_SPEED: Collection = Collection::new(class::AUD_SENSEOFSPEED, 0xD7ED_BD36_2D7D_2152);
const FLIPS_SENSE_OF_SPEED: u64 = 0xD9BE_1F2F_1A72_FEE8;

pub struct Tricks {
    /// `m_pDMixIO` (objectID 0x40010050 + the state's instance).
    pub io: Option<DMixIoId>,
    board_flip: Option<ClassObject>,
    tricks_foley: Option<ClassObject>,
    tricks_done_foley: Option<ClassObject>,
    previous_trick: i32,
    cur_flip_trick: i32,
    cur_foley_trick: i32,
    done_trick_sound: i32,
    time_in_trick: f32,
    time_last_trick_held: f32,
    last_fun_meter: i32,
    /// PC port: `board_flip` is Skate 3's `Class_Flips`.
    board_flip_skate3: bool,
    /// PC port: Skate 3's flip sounds could not be read.
    skate3_bank_failed: bool,
}

impl Default for Tricks {
    /// The constructor 0x82514f70.
    fn default() -> Self {
        Self {
            io: None,
            board_flip: None,
            tricks_foley: None,
            tricks_done_foley: None,
            previous_trick: INVALID_AUDIO_TRICK,
            cur_flip_trick: INVALID_AUDIO_TRICK,
            cur_foley_trick: INVALID_AUDIO_TRICK,
            done_trick_sound: INVALID_AUDIO_TRICK,
            time_in_trick: 0.0,
            time_last_trick_held: 0.0,
            last_fun_meter: 0,
            board_flip_skate3: false,
            skate3_bank_failed: false,
        }
    }
}

/// One deck rotation axis in thousandths of `MaxValue`, 0 below `MinInput`.
fn axis_speed(vault: &Vault, speed: f32, axis: usize) -> i32 {
    let min_input = vault.i32(TRICKS, MIN_INPUT[axis]);
    let max_value = vault.f32(TRICKS, MAX_VALUE[axis]);
    let mut value = fctiwz(speed.abs() / max_value * 1000.0);
    if value > 1000 {
        value = 1000;
    }
    if value < min_input {
        value = 0;
    }
    value
}

impl Tricks {
    /// The `InitSFX` (0x82515d80, shared) `SetupLoadData` runs on attach: the mixer outputs on.
    pub fn init_sfx(&mut self, ctx: &mut SfxCtx<'_>) {
        if let Some(io) = self.io {
            ctx.map.set_output_enabled(io, true);
        }
    }

    /// `Destroy` 0x8252c000, which `Detach` runs: the flip and the trick foley (the trick-done
    /// foley stays).
    pub fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        for object in [self.board_flip.take(), self.tricks_foley.take()].into_iter().flatten() {
            ctx.aems.release_object(object);
        }
    }

    /// `UpdateParams` 0x82544060 (local skater only).
    pub fn update_params(&mut self, t: f32, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !ctx.local {
            return;
        }
        let trick = physics.trick_type;
        if trick != INVALID_AUDIO_TRICK {
            self.done_trick_sound = physics.trick_done_type;
        }
        self.check_for_board_flips(trick, physics, ctx);
        self.check_for_tricks(trick, ctx);
        self.check_for_tricks_done(trick, t, ctx);
        self.previous_trick = trick;
        self.time_last_trick_held = if self.time_last_trick_held <= 0.0 { 0.0 } else { self.time_last_trick_held - t };
        self.update_fun_meter(t, ctx);
    }

    /// `ProcessUpdate` 0x82544118.
    pub fn process_update(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        self.update_board_flips(physics, ctx);
        self.update_tricks(physics, ctx);
        self.update_tricks_done(physics, ctx);
    }

    /// `CheckForBoardFlips` 0x82526850: a flip instance for a trick in the air, or for a
    /// dismount contact on the ground.
    fn check_for_board_flips(&mut self, trick: i32, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !ctx.vault.bool(TRICKS, PLAY_BOARD_FLIPS) {
            return;
        }
        let dismount = !physics.in_air && ctx.local && physics.is_dismount_contact;
        let start = if physics.in_air { physics.is_performing_trick } else { dismount };
        if !start || self.board_flip.is_some() {
            return;
        }
        self.cur_flip_trick = if dismount { TRICK_DISMOUNT } else { trick };
        if self.cur_flip_trick == INVALID_AUDIO_TRICK || NO_FLIP_TRICKS.contains(&self.cur_flip_trick) {
            return;
        }
        let speed = physics.deck_local_xyz_angular_speed;
        let [x, y, z] = [0, 1, 2].map(|axis| axis_speed(ctx.vault, speed[axis], axis));
        let [volume_x, volume_y, volume_z] = BOARD_FLIPS_VOL.map(|key| ctx.vault.i32(TRICKS, key));
        let trick_volume = ctx.vault.i32(TRICKS, TRICK_VOL);
        let jump_height = fctiwz(physics.jump_fraction * 10000.0);
        let game_speed = fctiwz(physics.game_speed * 500.0);
        let new_trick = i32::from(ctx.challenge_type == 0);
        let own_the_spot = i32::from(ctx.in_own_the_spot);
        let (local, flange) = if ctx.local { (1, ctx.output(self.io, 6, PresetType::Volume)) } else { (0, 0) };
        let forward_replay = i32::from(!physics.is_rewinding);
        self.board_flip_skate3 = physics.skate3_only_trick && !dismount && self.skate3_bank(ctx);
        if self.board_flip_skate3 {
            // TU3 0x824cc3d0..c564: its constructor (0x824afad8) keeps the first twelve members,
            // then takes its own flags and volumes, each clamped. Member 16 is the physics' not
            // rewinding (+0xE0, Skate 2's mIsRewinding +0xDC), which the module needs to play.
            let sense = ctx.skate3.unwrap_or(ctx.vault).i32(SENSE_OF_SPEED, FLIPS_SENSE_OF_SPEED);
            let (unit, level, flag) = (|v: i32| v.clamp(0, 1000), |v: i32| v.clamp(0, 0x7FFF), |v: i32| v.clamp(0, 1));
            let data = vec![
                0,
                0x7FFF,
                0,
                0,
                0x1000,
                25000,
                0,
                unit(z),
                unit(y),
                unit(x),
                unit(game_speed),
                self.cur_flip_trick.clamp(0, 40),
                0,
                0,
                0,
                0,
                flag(forward_replay),
                level(volume_z),
                level(volume_y),
                level(volume_x),
                0,
                level(trick_volume),
                0,
                flag(own_the_spot),
                flag(local),
                flag(local),
                level(flange),
                level(sense),
            ];
            self.board_flip = Some(ctx.aems.create_object_in(SKATE3_INTERFACE, "Class_Flips", data));
            return;
        }
        let data = [
            0,
            0x7FFF,
            0,
            0,
            0x1000,
            25000,
            0,
            z,
            y,
            x,
            game_speed,
            self.cur_flip_trick,
            jump_height,
            0,
            0,
            new_trick,
            0,
            own_the_spot,
            forward_replay,
            volume_z,
            volume_y,
            volume_x,
            0,
            trick_volume,
            0,
            local,
            flange,
        ];
        let data = data.iter().zip(FLIPS_RANGES).map(|(&value, max)| value.clamp(0, max)).collect();
        self.board_flip = Some(ctx.aems.create_object("Class_Flips", data));
    }

    /// PC port: Skate 3's flip sounds ([`SKATE3_FILES`]), read from the linked Skate 3 data the
    /// first time a Skate 3-only trick flips.
    fn skate3_bank(&mut self, ctx: &mut SfxCtx<'_>) -> bool {
        if ctx.aems.has_interface(SKATE3_INTERFACE) {
            return true;
        }
        if self.skate3_bank_failed {
            return false;
        }
        let read = |ctx: &SfxCtx<'_>, name: &str| {
            let files = ctx.core.stream_pools.files()?;
            let path = format!("{}data\\audio\\audiofiles.big|data\\audio\\{name}", crate::rwac::stream_pool::SKATE3_FILES);
            let mut bytes = vec![0u8; usize::try_from(files.size(&path)?).ok()?];
            files.read_at(&path, 0, &mut bytes).then_some(bytes)
        };
        let loaded = match (read(ctx, SKATE3_FILES[0]), read(ctx, SKATE3_FILES[1])) {
            (Some(interface), Some(bank)) if bank.starts_with(b"ABKC") => {
                ctx.aems.subscribe_interface_last(SKATE3_INTERFACE, &interface)
                    && ctx.aems.add_module_bank(&mut *ctx.core, SKATE3_BANK, std::sync::Arc::from(bank)).is_some()
            }
            _ => false,
        };
        self.skate3_bank_failed = !loaded;
        loaded
    }

    /// `kClothingType`, 1 when unforced.
    fn cloth_type(ctx: &SfxCtx<'_>) -> i32 {
        match ctx.cvars.clothing_type {
            -1 => 1,
            cloth => cloth,
        }
    }

    /// The `Cloth_trick` the trick checks build.
    fn new_cloth_trick(ctx: &mut SfxCtx<'_>, trick: i32) -> ClassObject {
        let data = vec![0, 0, 0x1000, 0, 25000, 0, 0, 0, Self::cloth_type(ctx).clamp(0, 3), trick.clamp(0, 40)];
        ctx.aems.create_object("cloth_trick", data)
    }

    /// `CheckForTricks` 0x82526c60: a cloth instance per trick.
    fn check_for_tricks(&mut self, trick: i32, ctx: &mut SfxCtx<'_>) {
        if !ctx.vault.bool(CLOTHING, PLAY_CLOTH_FOLEY) || trick == INVALID_AUDIO_TRICK {
            return;
        }
        match self.tricks_foley.take() {
            None => {
                self.tricks_foley = Some(Self::new_cloth_trick(ctx, trick));
                self.cur_foley_trick = trick;
            }
            Some(foley) if trick == self.cur_foley_trick => self.tricks_foley = Some(foley),
            Some(foley) => ctx.aems.release_object(foley),
        }
    }

    /// `CheckForTricksDone` 0x82526e30: when a trick ends, its trick-done cloth instance, kept
    /// while `mTimeLastTrickHeld` (the trick's length) runs down.
    fn check_for_tricks_done(&mut self, trick: i32, t: f32, ctx: &mut SfxCtx<'_>) {
        if !ctx.vault.bool(CLOTHING, PLAY_CLOTH_FOLEY) {
            return;
        }
        let mut ended = false;
        if trick == INVALID_AUDIO_TRICK {
            if self.previous_trick != INVALID_AUDIO_TRICK {
                self.time_last_trick_held = self.time_in_trick;
                ended = true;
                self.time_in_trick = 0.0;
            }
        } else {
            self.time_in_trick += t;
        }
        if self.time_last_trick_held <= 0.0 {
            if let Some(foley) = self.tricks_done_foley.take() {
                ctx.aems.release_object(foley);
            }
            return;
        }
        if self.done_trick_sound == INVALID_AUDIO_TRICK || !ended {
            return;
        }
        if let Some(foley) = self.tricks_done_foley.take() {
            ctx.aems.release_object(foley);
        }
        self.tricks_done_foley = Some(Self::new_cloth_trick(ctx, self.done_trick_sound));
    }

    /// `UpdateFunMeter` 0x82527800: toward the multiplier's fun target at the fall and rise
    /// rates.
    fn update_fun_meter(&mut self, t: f32, ctx: &SfxCtx<'_>) {
        let mut fun = 0;
        if !(t <= 0.0) {
            let [one_five, two, three] = ctx.times;
            if one_five {
                fun = ctx.vault.i32(FUN_METER, FUN[0]);
            } else if two {
                fun = ctx.vault.i32(FUN_METER, FUN[1]);
            } else if three {
                fun = ctx.vault.i32(FUN_METER, FUN[2]);
            }
            let fall = fctiwz(ctx.vault.f32(FUN_METER, FUN_FALL_RATE) * t);
            let rise = fctiwz(ctx.vault.f32(FUN_METER, FUN_RISE_RATE) * t);
            let current = self.last_fun_meter;
            if fun < current {
                if current - fun > fall {
                    fun = current - fall;
                }
            } else if fun > current && fun - current > rise {
                fun = current + rise;
            }
        }
        self.last_fun_meter = fun;
    }

    /// `UpdateBoardFlips` 0x82527088: the flip instance follows the deck rotation and the mixer
    /// while the same trick stays in the air (or on its dismount contact).
    fn update_board_flips(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !ctx.vault.bool(TRICKS, PLAY_BOARD_FLIPS) {
            return;
        }
        let dismount = !physics.in_air && ctx.local && physics.is_dismount_contact;
        let trick = if dismount { TRICK_DISMOUNT } else { physics.trick_type };
        if !(physics.in_air || dismount) || trick != self.cur_flip_trick {
            if let Some(flip) = self.board_flip.take() {
                ctx.aems.release_object(flip);
            }
            return;
        }
        let Some(mut flip) = self.board_flip.take() else { return };
        let speed = physics.deck_local_xyz_angular_speed;
        let [x, y, z] = [0, 1, 2].map(|axis| axis_speed(ctx.vault, speed[axis], axis));
        let io = self.io;
        let data = &mut flip.data;
        data[9] = x.clamp(0, 1000);
        data[8] = y.clamp(0, 1000);
        data[7] = z.clamp(0, 1000);
        data[10] = fctiwz(physics.game_speed * 500.0).clamp(0, 1000);
        data[4] = ctx.output(io, 2, PresetType::Pitch).clamp(0, 0x2000);
        data[0] = ctx.output(io, 1, PresetType::Volume).clamp(0, 0x7FFF);
        data[3] = ctx.output(io, 0, PresetType::Azimuth).clamp(0, 0x1_0000);
        data[1] = 0x7FFF;
        data[2] = 0;
        data[5] = ctx.output(io, 3, PresetType::Frequency).clamp(0, 25000);
        data[6] = 0;
        data[26] = if ctx.local { ctx.output(io, 6, PresetType::Volume) } else { 0 }.clamp(0, 0x7FFF);
        if self.board_flip_skate3 {
            // TU3 UpdateBoardFlips 0x824cc7d8's members of its own Class_Flips.
            data[16] = i32::from(!physics.is_rewinding);
            data[20] = ctx.output(io, 8, PresetType::Volume).clamp(0, 0x7FFF);
            data[22] = ctx.output(io, 7, PresetType::Volume).clamp(0, 0x7FFF);
            data[12] = self.last_fun_meter.clamp(0, 1000);
        } else {
            data[18] = i32::from(!physics.is_rewinding);
            data[22] = ctx.output(io, 8, PresetType::Volume).clamp(0, 0x7FFF);
            data[24] = ctx.output(io, 7, PresetType::Volume).clamp(0, 0x7FFF);
            data[13] = self.last_fun_meter.clamp(0, 1000);
        }
        ctx.aems.set_member_data(&flip);
        self.board_flip = Some(flip);
    }

    /// The mixer values `UpdateTricks` and `UpdateTricksDone` give a cloth instance.
    fn update_cloth(io: Option<DMixIoId>, foley: &mut ClassObject, ctx: &SfxCtx<'_>) {
        let pitch = ctx.output(io, 5, PresetType::Pitch);
        let dry = ctx.output(io, 4, PresetType::Volume);
        let data = &mut foley.data;
        data[0] = 0x7FFF;
        data[4] = 25000;
        data[5] = 0;
        data[6] = 0;
        data[7] = dry.clamp(0, 0x7FFF);
        data[1] = ctx.output(io, 0, PresetType::Azimuth).clamp(0, 0xFFFF);
        data[2] = pitch.clamp(0, 0x2000);
    }

    /// `UpdateTricks` 0x82527610: the trick's cloth instance until a wipeout, or until the
    /// trick ends on the ground.
    fn update_tricks(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !ctx.vault.bool(CLOTHING, PLAY_CLOTH_FOLEY) {
            return;
        }
        let Some(mut foley) = self.tricks_foley.take() else { return };
        if physics.is_wiping_out || (physics.trick_type == INVALID_AUDIO_TRICK && !physics.in_air) {
            ctx.aems.release_object(foley);
            return;
        }
        Self::update_cloth(self.io, &mut foley, ctx);
        ctx.aems.set_member_data(&foley);
        self.tricks_foley = Some(foley);
    }

    /// `UpdateTricksDone` 0x82515028: the trick-done cloth instance until a wipeout.
    fn update_tricks_done(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let Some(mut foley) = self.tricks_done_foley.take() else { return };
        if physics.is_wiping_out {
            ctx.aems.release_object(foley);
            return;
        }
        Self::update_cloth(self.io, &mut foley, ctx);
        ctx.aems.set_member_data(&foley);
        self.tricks_done_foley = Some(foley);
    }
}
