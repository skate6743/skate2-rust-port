//! `Sk8::Audio::SFXObj_Reverb` (MAIN SFX 5, 0x350 bytes, DMixIO 0x40000050): sets up
//! `GlobalEffects`' two reverb instances from `aud_reverb` collections, cross-fades from one to
//! the other over a second when the location's reverb key changes, and feeds the flange levels and
//! the global reverb volume from its mixer outputs.
//!
//! Inside the emitter system's reverb volumes (`AudioSystem::GetCurrentReverbVolume`) the
//! transitions follow the volume: a distance transition pans and fades the volume's reverb in
//! towards its centre (`ProcDistTrans` 0x82546c68, `PanTransition` 0x82531ca0), and a second
//! volume starts a sphere-to-sphere one (`ProcSprhSphrTrans` 0x82546e18). The slot LFO
//! parameters `VerbInit` also fills are not ported.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::global_effects::{DelayParms, EnvRvrbPresetParms, EnvRvrbRuntimeParms, FilterParms, FlangePresetParms, GlobalEffects, LfoShape};
use crate::nicotine::{DMixIoId, MixMap, PresetType};
use crate::rwac::AudioCore;
use crate::vault::{Collection, Vault};
use crate::vmx::{self, Vec3};
use crate::world::emitter_system::{reverb_type, Sk8EmitterSystem, VolumeStatusId};

/// `Attrib::Gen::aud_reverb` and `aud_flange` class keys.
const AUD_REVERB: u64 = 0x204C_AC1F_D770_88B8;
const AUD_FLANGE: u64 = 0x4CDD_7CDC_1A95_5D5C;
/// `Attrib::Class::GetFirstCollection` 0x8297a508 of `aud_reverb` in `skater.vlt`: the class's
/// collection table is a `VecHashMap` of `collection_reserve` (33) slots, keyed by the low word
/// modulo 33 with linear probing, walked in slot order; 0x9572128E_FDA5C188 is the only key at
/// slot 0.
const FIRST_REVERB: u64 = 0x9572_128E_FDA5_C188;
/// The same walk of `aud_flange` (2 slots): the even low word, then the odd one
/// (`GetNextCollection` 0x8297a518).
const FLANGES: [u64; 2] = [0xDFDF_FAD6_7CCB_A322, 0xC629_0FFC_B9E8_4CD9];
/// The patches `InitializeFlange` gives the first and next collection.
const FLANGE_PATCHES: [u32; 2] = [0x3000, 0x4000];
/// `__real_38000100` (1/32767).
const Q15_SCALE: f32 = f32::from_bits(0x3800_0100);
const LEFT: f32 = 270.0;
const RIGHT: f32 = 90.0;
/// `__real_3ea2f983` (1/pi).
const INV_PI: f32 = f32::from_bits(0x3EA2_F983);

/// `SFXObj_Reverb::eType`: how a transition advances.
const TRANS_TIME: u32 = 0;
const TRANS_DIST: u32 = 1;
const TRANS_SPHERE: u32 = 2;

/// `SFXObj_Reverb::RvrbParms` (0xB8 bytes) as the time transitions use it.
#[derive(Debug, Clone, Copy, Default)]
struct RvrbParms {
    /// +0x00: the `aud_reverb` instance's collection.
    reverb: u64,
    /// +0x10.
    errp: EnvRvrbRuntimeParms,
    /// +0x64: the transition type `SetupWPVerb` gave the slot.
    kind: u32,
    /// +0xB0, +0xB4: the transition's elapsed and total seconds (a distance transition's
    /// `current` is the volume distance).
    current: f32,
    length: f32,
    /// PC port: `reverb` is a linked Skate 3 district's `aud_reverb` collection, read from Skate
    /// 3's attributes (TU3 `VerbInit` 0x824de140 reads the same offsets).
    skate3: bool,
}

pub struct Reverb {
    pub io: Option<DMixIoId>,
    /// +0x1C: `mrvrb[2]`.
    slots: [RvrbParms; 2],
    /// +0x190, +0x198, +0x1A0, +0x1A4.
    active_key: u64,
    target_key: u64,
    active_index: usize,
    target_index: usize,
    /// +0x1B4: `tpp`, the parameters of the last `VerbInit`.
    tpp: EnvRvrbPresetParms,
    /// +0x1A8, +0x1AC: `mSphereIndex` and `mpevs`, the reverb volumes a transition follows.
    sphere_index: usize,
    pevs: [Option<VolumeStatusId>; 2],
    /// PC port: a linked Skate 3 data folder's attributes; none unlinked.
    skate3_vault: Option<std::sync::Arc<Vault>>,
}

impl Reverb {
    #[must_use]
    pub fn new(io: Option<DMixIoId>) -> Self {
        Self {
            io,
            slots: [RvrbParms::default(); 2],
            active_key: 0,
            target_key: 0,
            active_index: 0,
            target_index: 0,
            tpp: EnvRvrbPresetParms::default(),
            sphere_index: 0,
            pevs: [None; 2],
            skate3_vault: None,
        }
    }

    /// PC port: the linked Skate 3 attributes whose `aud_reverb` collections a district's keys name.
    pub fn set_skate3(&mut self, vault: Option<std::sync::Arc<Vault>>) {
        self.skate3_vault = vault;
    }

    /// The attributes holding slot `n`'s collection.
    fn slot_vault<'v>(&self, n: usize, skate3: &'v Option<std::sync::Arc<Vault>>, vault: &'v Vault) -> &'v Vault {
        match skate3.as_deref() {
            Some(skate3) if self.slots[n].skate3 => skate3,
            _ => vault,
        }
    }

    /// `InitSFX` 0x82546b68: the first `aud_reverb` collection in both slots, instance 0 at full
    /// level and instance 1 silent, then `InitializeFlange` 0x82531330.
    pub fn init_sfx(&mut self, core: &mut AudioCore, map: &mut MixMap, vault: &Vault, effects: &mut GlobalEffects) {
        if let Some(io) = self.io {
            map.set_output_enabled(io, true);
        }
        self.active_key = FIRST_REVERB;
        self.slots[0].reverb = self.active_key;
        self.slots[1].reverb = self.active_key;
        self.pevs = [None; 2];
        self.active_index = 0;
        self.target_index = 0;
        self.sphere_index = 0;
        self.target_key = self.active_key;
        self.verb_init(0, 1.0, core, vault, effects);
        self.verb_init(1, 0.0, core, vault, effects);
        self.initialize_flange(vault, effects);
    }

    /// `InitializeFlange`: the first `aud_flange` collection is patch 0x3000 (flange 0), the next
    /// patch 0x4000, both dry and wet silent until `ProcessUpdate`.
    fn initialize_flange(&self, vault: &Vault, effects: &mut GlobalEffects) {
        for (key, patch_id) in FLANGES.into_iter().zip(FLANGE_PATCHES) {
            let layout = |offset: u32| vault.layout_f32(Collection::new(AUD_FLANGE, key), offset / 4);
            let delay_time = layout(0x14);
            let notch_freq = layout(0x10);
            effects.set_flange_parms(&FlangePresetParms {
                patch_id,
                delay_time,
                delay_feedback: layout(0x20),
                notch_freq,
                notch_gain: layout(0xC),
                notch_q: layout(0),
                wet_depth: 0.0,
                dry_depth: 0.0,
                delay_lfo: LfoShape { cycle_time: 1.0 / layout(0x18), depth: layout(0x1C), base: delay_time, max_range: delay_time },
                notch_lfo: LfoShape { cycle_time: 1.0 / layout(4), depth: layout(8), base: notch_freq, max_range: notch_freq },
            });
        }
    }

    /// `VerbInit` 0x82531560: slot `n`'s collection into `tpp` (filters panned 270 and 90
    /// degrees, radius 1, LFE 0.5) and its runtime pans and delays, then
    /// `GlobalEffects::SetupReverb`.
    fn verb_init(&mut self, n: usize, fxlevel: f32, core: &mut AudioCore, vault: &Vault, effects: &mut GlobalEffects) {
        let skate3 = self.skate3_vault.clone();
        let vault = self.slot_vault(n, &skate3, vault);
        let collection = Collection::new(AUD_REVERB, self.slots[n].reverb);
        let layout = |offset: u32| vault.layout_f32(collection, offset / 4);
        let filter = |lpf: u32, hpf: u32, gain: u32, panazimuth: f32| FilterParms {
            lpf_freq: layout(lpf),
            hpf_freq: layout(hpf),
            gain: layout(gain),
            panazimuth,
            panradius: 1.0,
            lfe_level: 0.5,
        };
        let delay = |[time, fbgain, lfofreq, lfodepth, lpf, hpf, gain]: [u32; 7], panazimuth: f32| DelayParms {
            time: layout(time),
            fbgain: layout(fbgain),
            time_lfodepth: layout(lfodepth),
            time_lfofreq: layout(lfofreq),
            lpf_freq: layout(lpf),
            hpf_freq: layout(hpf),
            gain: layout(gain),
            panazimuth,
            panradius: 1.0,
            lfe_level: 0.5,
            pan_lfodepth: layout(0x5C),
            pan_lfofreq: layout(0x58),
        };
        self.tpp = EnvRvrbPresetParms {
            inst: n,
            fxlevel,
            fxgain: layout(0x54),
            notch_freq: layout(0x50),
            notch_gain: layout(0x4C),
            notch_q: layout(0x40),
            notch_freq_lfodepth: layout(0x48),
            notch_freq_lfofreq: layout(0x44),
            send_reverb: layout(0x34),
            send_delay: [layout(0x3C), layout(0x38)],
            predelay_time: layout(0xC),
            predelay_fbgain: layout(0x10),
            reverb_time: layout(0x14),
            reverb_size: layout(0x18),
            reverb_brightness: layout(0x20),
            reverb_gain: layout(0x1C),
            filter: [filter(0x24, 0x28, 0x2C, LEFT), filter(0, 4, 8, RIGHT)],
            delay: [delay([0xA8, 0xAC, 0x98, 0x9C, 0x94, 0xA0, 0xA4], LEFT), delay([0x80, 0x84, 0x70, 0x74, 0x6C, 0x78, 0x7C], RIGHT)],
        };
        let tpp = self.tpp;
        let errp = &mut self.slots[n].errp;
        errp.notch_freq = tpp.notch_freq;
        errp.left_gain = 1.0;
        errp.left_panazimuth = tpp.filter[0].panazimuth;
        errp.right_gain = 1.0;
        errp.right_panazimuth = tpp.filter[1].panazimuth;
        for side in 0..2 {
            errp.delay_time[side] = tpp.delay[side].time;
            errp.delay_panazimuth[side] = tpp.delay[side].panazimuth;
        }
        effects.setup_reverb(core, &tpp);
    }

    /// `VerbUpdate` 0x82531870: the instance's level from its slot, gains, delays and notch from
    /// its collection (the pans stay the caller's), then `GlobalEffects::UpdateReverb`.
    fn verb_update(&self, parms: &mut EnvRvrbRuntimeParms, core: &mut AudioCore, vault: &Vault, effects: &GlobalEffects) {
        let slot = &self.slots[parms.inst];
        let vault = self.slot_vault(parms.inst, &self.skate3_vault, vault);
        let layout = |offset: u32| vault.layout_f32(Collection::new(AUD_REVERB, slot.reverb), offset / 4);
        parms.fxlevel = slot.errp.fxlevel;
        parms.left_gain = layout(0x2C);
        parms.right_gain = layout(8);
        parms.delay_panazimuth[0] = LEFT;
        parms.delay_time[0] = layout(0xA8);
        parms.delay_panazimuth[1] = RIGHT;
        parms.delay_time[1] = layout(0x80);
        parms.notch_freq = layout(0x50);
        effects.update_reverb(core, parms);
    }

    /// `SetupWPVerb` 0x82531988: the other slot takes the collection at level 0 and a one-second
    /// transition of `kind`.
    fn setup_wp_verb(&mut self, key: u64, skate3: bool, kind: u32, core: &mut AudioCore, vault: &Vault, effects: &mut GlobalEffects) {
        self.target_key = key;
        self.target_index = self.active_index.wrapping_sub(1) & 1;
        let target = self.target_index;
        self.slots[target].reverb = key;
        self.slots[target].skate3 = skate3;
        self.slots[target].kind = kind;
        self.verb_init(target, 0.0, core, vault, effects);
        let slot = &mut self.slots[target];
        slot.current = 0.0;
        slot.length = 1.0;
        slot.errp.inst = target;
    }

    /// `ProcTimeTrans` 0x82531a68: the target fades in linearly and the active slot out; at full
    /// level the target becomes active.
    fn proc_time_trans(&mut self, t: f32, core: &mut AudioCore, vault: &Vault, effects: &GlobalEffects) {
        let (target, active) = (self.target_index, self.active_index);
        let slot = &mut self.slots[target];
        slot.current += t;
        slot.errp.fxlevel = slot.current / slot.length;
        let level = slot.errp.fxlevel;
        if !(level < 1.0) {
            self.slots[target].errp.fxlevel = 1.0;
            self.slots[active].errp.fxlevel = 0.0;
        } else {
            self.slots[active].errp.fxlevel = 1.0 - level;
        }
        let mut parms = EnvRvrbRuntimeParms { inst: target, left_panazimuth: LEFT, right_panazimuth: RIGHT, ..EnvRvrbRuntimeParms::default() };
        self.verb_update(&mut parms, core, vault, effects);
        parms.inst = active;
        self.verb_update(&mut parms, core, vault, effects);
        if !(self.slots[target].errp.fxlevel < 1.0) {
            self.active_index = target;
            self.active_key = self.target_key;
        }
    }

    /// `GetReverbCategory` 0x82517018: the active collection's type word (+0x30): types 6-8 and
    /// 22 are category 1; 9, 13-15, 17 and 18 category 2; 10 category 3; 21 category 4; the
    /// rest 0.
    fn reverb_category(&self, vault: &Vault) -> i32 {
        let vault = self.slot_vault(self.active_index, &self.skate3_vault, vault);
        let kind = vault.layout_word(Collection::new(AUD_REVERB, self.slots[self.active_index].reverb), 12) as i32;
        match kind {
            6..=8 | 22 => 1,
            9 | 13..=15 | 17 | 18 => 2,
            10 => 3,
            21 => 4,
            _ => 0,
        }
    }

    /// `UpdateNicotine` 0x825170c0: inputs 0-3 are the categories 1-4.
    fn update_nicotine(&self, map: &mut MixMap, vault: &Vault) {
        let Some(io) = self.io else { return };
        for index in 0..4 {
            map.set_input(io, index, 0);
        }
        let category = self.reverb_category(vault);
        if (1..=4).contains(&category) {
            map.set_input(io, category as usize - 1, 0x7FFF);
        }
    }

    /// `UpdateParams` 0x8254e1e0 with a `Presentation` (`AudioSystem::GetReverbKey` 0x824f1df8
    /// as `reverb_key`, the local skater's board position and look as `AudioPresentation::mLocal`
    /// +0x30/+0x40). With no transition running: outside the reverb volumes a new non-zero key
    /// starts a time transition; entering one starts a distance transition to its `reverbType`;
    /// a second volume starts a sphere-to-sphere one; leaving the centre of the only one starts a
    /// distance transition back to the key. A running transition advances by its type.
    #[allow(clippy::too_many_arguments)]
    pub fn update_params(
        &mut self,
        t: f32,
        presentation: bool,
        reverb_key: u64,
        skate3_key: bool,
        local_board: Option<(Vec3, Vec3)>,
        volumes: &Sk8EmitterSystem,
        core: &mut AudioCore,
        map: &mut MixMap,
        vault: &Vault,
        effects: &mut GlobalEffects,
    ) {
        if !presentation {
            return;
        }
        self.update_nicotine(map, vault);
        if self.active_key != self.target_key {
            match self.slots[self.target_index].kind {
                TRANS_TIME => self.proc_time_trans(t, core, vault, effects),
                TRANS_DIST => self.proc_dist_trans(local_board, volumes, core, map, vault, effects),
                TRANS_SPHERE => self.proc_sphere_trans(local_board, volumes, core, map, vault, effects),
                _ => self.target_index = self.active_index,
            }
            return;
        }
        let key_of = |id: VolumeStatusId| volumes.volume_status(id).map_or(0, |status| status.key);
        let Some(first) = volumes.current_reverb_volume(None, vault) else {
            self.pevs = [None; 2];
            if reverb_key != 0 && self.active_key != reverb_key {
                self.setup_wp_verb(reverb_key, skate3_key, TRANS_TIME, core, vault, effects);
            }
            return;
        };
        let Some(held) = self.pevs[0] else {
            self.pevs[0] = Some(first);
            self.setup_wp_verb(reverb_type(key_of(first), vault), false, TRANS_DIST, core, vault, effects);
            self.sphere_index = self.target_index;
            return;
        };
        if let Some(second) = volumes.current_reverb_volume(Some(first), vault) {
            let first_key = reverb_type(key_of(first), vault);
            let second_key = reverb_type(key_of(second), vault);
            if first_key == second_key {
                if self.active_key == first_key {
                    self.target_index = self.active_index;
                    return;
                }
            } else if self.active_key == first_key {
                self.setup_wp_verb(second_key, false, TRANS_SPHERE, core, vault, effects);
                self.pevs[1] = Some(second);
                return;
            }
            self.setup_wp_verb(first_key, false, TRANS_SPHERE, core, vault, effects);
            self.pevs[1] = Some(first);
            return;
        }
        // `mpevs[0]` may be a status the emitter system has freed; its last values are read.
        let distance = volumes.volume_status(held).map_or(0.0, |status| status.distance);
        if distance > 0.0 && self.sphere_index == self.active_index && reverb_key != 0 {
            self.setup_wp_verb(reverb_key, skate3_key, TRANS_DIST, core, vault, effects);
            self.sphere_index = self.active_index;
        }
    }

    /// `VerbUpdate` of `first` then `second` with the default pans.
    fn verb_update_pair(&self, first: usize, second: usize, core: &mut AudioCore, vault: &Vault, effects: &GlobalEffects) {
        let mut parms = EnvRvrbRuntimeParms { inst: first, left_panazimuth: LEFT, right_panazimuth: RIGHT, ..EnvRvrbRuntimeParms::default() };
        self.verb_update(&mut parms, core, vault, effects);
        parms.inst = second;
        self.verb_update(&mut parms, core, vault, effects);
    }

    /// `ProcDistTrans` 0x82546c68: while the followed volume stays current (or is replaced by one
    /// of the same key) its distance drives `PanTransition`; at distance 0 the sphere slot wins.
    /// Otherwise `ResetDistTrans`.
    #[allow(clippy::too_many_arguments)]
    fn proc_dist_trans(&mut self, local_board: Option<(Vec3, Vec3)>, volumes: &Sk8EmitterSystem, core: &mut AudioCore, map: &mut MixMap, vault: &Vault, effects: &mut GlobalEffects) {
        let first = volumes.current_reverb_volume(None, vault);
        let second = first.and_then(|first| volumes.current_reverb_volume(Some(first), vault));
        let Some(first) = first else {
            self.reset_dist_trans(core, vault, effects);
            return;
        };
        if self.pevs[0] != Some(first) {
            match second {
                Some(second) if self.pevs[0] != Some(second) => {
                    self.reset_dist_trans(core, vault, effects);
                    return;
                }
                Some(_) => {}
                None => {
                    let key = |id: Option<VolumeStatusId>| id.and_then(|id| volumes.volume_status(id)).map(|status| status.key);
                    if key(self.pevs[0]) != key(Some(first)) {
                        self.reset_dist_trans(core, vault, effects);
                        return;
                    }
                    self.pevs[0] = Some(first);
                }
            }
        }
        let sphere = self.sphere_index;
        self.slots[sphere].current = self.pevs[0].and_then(|id| volumes.volume_status(id)).map_or(0.0, |status| status.distance);
        if self.slots[sphere].current != 0.0 {
            self.pan_transition(sphere, self.pevs[0], local_board, volumes, core, map, vault, effects);
            return;
        }
        if self.target_index == sphere {
            self.active_index = self.target_index;
            self.active_key = self.target_key;
        } else {
            self.target_key = self.active_key;
            self.target_index = self.active_index;
        }
        let active = self.active_index;
        let other = active.wrapping_sub(1) & 1;
        self.slots[active].errp.fxlevel = 1.0;
        self.slots[other].errp.fxlevel = 0.0;
        self.verb_update_pair(active, other, core, vault, effects);
    }

    /// `ResetDistTrans` 0x82531b88: the sphere slot's side wins at once and the volumes are
    /// dropped.
    fn reset_dist_trans(&mut self, core: &mut AudioCore, vault: &Vault, effects: &GlobalEffects) {
        let (active, target) = (self.active_index, self.target_index);
        if self.sphere_index == active {
            self.slots[active].errp.fxlevel = 0.0;
            self.slots[target].errp.fxlevel = 1.0;
            self.verb_update_pair(active, target, core, vault, effects);
            self.active_index = target;
            self.active_key = self.target_key;
        } else {
            self.slots[target].errp.fxlevel = 0.0;
            self.slots[active].errp.fxlevel = 1.0;
            self.verb_update_pair(target, active, core, vault, effects);
            self.target_index = active;
            self.target_key = self.active_key;
        }
        self.pevs = [None; 2];
    }

    /// `ProcSprhSphrTrans` 0x82546e18: with one volume left its reverb wins; with both followed
    /// volumes current, equal keys keep the active slot, the second volume's centre makes it
    /// active, and otherwise its distance drives `PanTransition`. Any other change restarts
    /// (`InitSFX`).
    #[allow(clippy::too_many_arguments)]
    fn proc_sphere_trans(&mut self, local_board: Option<(Vec3, Vec3)>, volumes: &Sk8EmitterSystem, core: &mut AudioCore, map: &mut MixMap, vault: &Vault, effects: &mut GlobalEffects) {
        let first = volumes.current_reverb_volume(None, vault);
        let second = first.and_then(|first| volumes.current_reverb_volume(Some(first), vault));
        let Some(first) = first else {
            self.init_sfx(core, map, vault, effects);
            return;
        };
        let view = |id: Option<VolumeStatusId>| id.and_then(|id| volumes.volume_status(id));
        let (active, target) = (self.active_index, self.target_index);
        let Some(second) = second else {
            if self.pevs[0] != Some(first) {
                self.pevs[0] = self.pevs[1];
            }
            let reverb = reverb_type(view(self.pevs[0]).map_or(0, |status| status.key), vault);
            let (on, off) = if self.active_key == reverb { (active, target) } else { (target, active) };
            self.slots[on].errp.fxlevel = 1.0;
            self.slots[off].errp.fxlevel = 0.0;
            self.verb_update_pair(target, active, core, vault, effects);
            self.pevs[1] = None;
            self.target_index = active;
            self.target_key = self.active_key;
            self.sphere_index = active;
            return;
        };
        let current = |id: Option<VolumeStatusId>| id == Some(first) || id == Some(second);
        if !current(self.pevs[0]) || !current(self.pevs[1]) {
            self.init_sfx(core, map, vault, effects);
            return;
        }
        let (held, next) = (view(self.pevs[0]), view(self.pevs[1]));
        if held.map(|status| status.key) == next.map(|status| status.key) {
            self.slots[active].errp.fxlevel = 1.0;
            self.slots[target].errp.fxlevel = 0.0;
            self.verb_update_pair(target, active, core, vault, effects);
            self.sphere_index = active;
            self.target_index = active;
            self.target_key = self.active_key;
            return;
        }
        let distance = next.map_or(0.0, |status| status.distance);
        if distance != 0.0 {
            self.slots[target].current = distance;
            self.pan_transition(target, self.pevs[1], local_board, volumes, core, map, vault, effects);
            return;
        }
        self.slots[active].errp.fxlevel = 0.0;
        self.slots[target].errp.fxlevel = 1.0;
        self.verb_update_pair(target, active, core, vault, effects);
        let followed = self.pevs[1];
        self.pevs[1] = None;
        self.active_index = target;
        self.sphere_index = target;
        self.active_key = self.target_key;
        self.pevs[0] = followed;
    }

    /// `PanTransition` 0x82531ca0: slot `target` fades to `1 - current` and the other to
    /// `current`, the target's filters panned by the volume's bearing from the local board
    /// (`acos` of the normalised x-z vectors in degrees, signed by their cross product). Without
    /// the volume or a local skater, `InitSFX`.
    #[allow(clippy::too_many_arguments)]
    fn pan_transition(
        &mut self,
        target: usize,
        source: Option<VolumeStatusId>,
        local_board: Option<(Vec3, Vec3)>,
        volumes: &Sk8EmitterSystem,
        core: &mut AudioCore,
        map: &mut MixMap,
        vault: &Vault,
        effects: &mut GlobalEffects,
    ) {
        let (Some(source), Some((board_pos, board_look))) = (source.and_then(|id| volumes.volume_status(id)), local_board) else {
            self.init_sfx(core, map, vault, effects);
            return;
        };
        let other = target.wrapping_sub(1) & 1;
        let look = [board_look[0], board_look[2]];
        let delta = [source.pos[0] - board_pos[0], source.pos[2] - board_pos[2]];
        let look_scale = vmx::rsqrt(look[0] * look[0] + look[1] * look[1]);
        let delta_scale = vmx::rsqrt(delta[0] * delta[0] + delta[1] * delta[1]);
        let look = [look[0] * look_scale, look[1] * look_scale];
        let delta = [delta[0] * delta_scale, delta[1] * delta_scale];
        let dot = vmx::vmin(1.0, vmx::vmax(-1.0, delta[0] * look[0] + delta[1] * look[1]));
        let angle = INV_PI * (vmx::xm_acos(dot) * 180.0);
        let cross = delta[0] * look[1] - delta[1] * look[0];
        let mut left = angle - 270.0;
        let mut right = 90.0 - angle;
        if cross > 0.0 {
            if angle > 270.0 {
                right = (360.0 - angle) + 90.0;
            }
        } else if angle < 90.0 {
            left = angle + 90.0;
        }
        let current = self.slots[target].current;
        let mut right = (-current).mul_add(right, 90.0);
        let mut left = current.mul_add(left, 270.0);
        if right > 360.0 {
            right -= 360.0;
        }
        if right < 0.0 {
            right += 360.0;
        }
        if left > 360.0 {
            left -= 360.0;
        }
        if left < 0.0 {
            left += 360.0;
        }
        self.slots[target].errp.fxlevel = 1.0 - current;
        self.slots[other].errp.fxlevel = current;
        let mut parms = EnvRvrbRuntimeParms { inst: target, left_panazimuth: left, right_panazimuth: right, ..EnvRvrbRuntimeParms::default() };
        self.verb_update(&mut parms, core, vault, effects);
        parms.left_panazimuth = LEFT;
        parms.right_panazimuth = RIGHT;
        parms.inst = other;
        self.verb_update(&mut parms, core, vault, effects);
    }

    /// `ProcessUpdate` 0x82532000: outputs 0-3 are flange 0's dry and wet and flange 1's, output
    /// 4 the global reverb volume; then the sends of the target and active instances.
    pub fn process_update(&self, core: &mut AudioCore, map: &MixMap, effects: &mut GlobalEffects) {
        let volume = |index: i32| self.io.map_or(0, |io| map.output(io, index, PresetType::Volume)) as f32 * Q15_SCALE;
        let (dry_0, wet_0, dry_1, wet_1) = (volume(0), volume(1), volume(2), volume(3));
        effects.set_flange_dry(0x2000, dry_0);
        effects.set_flange_wet(0x2000, wet_0);
        effects.set_flange_dry(0x4000, dry_1);
        effects.set_flange_wet(0x4000, wet_1);
        effects.update_flange(core);
        effects.set_global_reverb_volume(volume(4));
        effects.update_gain(core, self.target_index);
        if self.target_index != self.active_index {
            effects.update_gain(core, self.active_index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rwac::core::{PlugInConfig, PlugInKind};
    use crate::rwac::PluginRef;
    use crate::world::emitter_system::test_support::{sphere, Mock, NoHost, World};

    const ROOM: u64 = 0x5555_0000_0000_0005;

    struct Rig {
        world: World,
        reverb: Reverb,
        core: AudioCore,
        map: MixMap,
        effects: GlobalEffects,
    }

    impl Rig {
        fn new() -> (Self, u64) {
            let mut mock = Mock::default();
            // `verbHash[3]` is the first `aud_reverb` collection, which `InitSFX` makes active.
            let verb = mock.reverb(ROOM, 5);
            let world = World::new(mock, &[sphere(1, [0.0; 3], 10.0, ROOM)], &[]);
            let mut core = AudioCore::new();
            let configs = [
                PlugInConfig { kind: PlugInKind::SubMix { name: "Master".into() }, output_channels: 6 },
                PlugInConfig { kind: PlugInKind::SampleCapture, output_channels: 6 },
                PlugInConfig { kind: PlugInKind::Dac, output_channels: 6 },
            ];
            let master = core.create_voice(255, "Mastering", &configs).expect("master");
            let mut effects = GlobalEffects::new(&mut core, PluginRef { voice: master, index: 0 });
            let mut map = MixMap::new(Vec::new(), 13, &mut NoHost);
            let mut reverb = Reverb::new(None);
            reverb.init_sfx(&mut core, &mut map, &world.vault, &mut effects);
            (Self { world, reverb, core, map, effects }, verb)
        }

        /// The emitter system's update, then `SFXObj_Reverb::UpdateParams` (the next frame's) with
        /// the local board 5 units behind the volume's centre, looking at it.
        fn step(&mut self, camera: [f32; 3]) {
            self.world.update(camera, 0);
            let board = Some(([0.0, 0.0, -5.0, 1.0], [0.0, 0.0, 1.0, 0.0]));
            self.reverb.update_params(1.0 / 30.0, true, 0, false, board, &self.world.system, &mut self.core, &mut self.map, &self.world.vault, &mut self.effects);
        }
    }

    #[test]
    fn distance_transition_into_a_reverb_volume() {
        let (mut rig, verb) = Rig::new();
        rig.step([5.0, 0.0, 0.0]);
        let reverb = &rig.reverb;
        assert_eq!((reverb.active_key, reverb.target_key, reverb.active_index, reverb.target_index, reverb.sphere_index), (FIRST_REVERB, verb, 0, 1, 1));
        assert_eq!(reverb.slots[1].kind, TRANS_DIST);
        assert!(reverb.pevs[0].is_some());
        // Halfway in: `PanTransition` fades the two slots evenly, the target panned towards the
        // volume ahead of the board (left 315, right 45).
        rig.step([5.0, 0.0, 0.0]);
        let reverb = &rig.reverb;
        assert_eq!((reverb.slots[0].errp.fxlevel, reverb.slots[1].errp.fxlevel), (0.5, 0.5));
        assert_eq!(reverb.slots[1].current, 0.5);
        // At the centre the volume's reverb becomes active.
        rig.step([0.0, 0.0, 0.0]);
        let reverb = &rig.reverb;
        assert_eq!((reverb.active_key, reverb.active_index), (verb, 1));
        assert_eq!((reverb.slots[0].errp.fxlevel, reverb.slots[1].errp.fxlevel), (0.0, 1.0));
        // Leaving drops the volume and keeps the reverb.
        rig.step([50.0, 0.0, 0.0]);
        let reverb = &rig.reverb;
        assert_eq!(reverb.pevs, [None, None]);
        assert_eq!((reverb.active_key, reverb.target_key), (verb, verb));
    }

    #[test]
    fn leaving_mid_transition_resets() {
        let (mut rig, _) = Rig::new();
        rig.step([5.0, 0.0, 0.0]);
        rig.step([5.0, 0.0, 0.0]);
        rig.step([50.0, 0.0, 0.0]);
        let reverb = &rig.reverb;
        // `ResetDistTrans`: the sphere slot (1) is not the active one, so the active side wins.
        assert_eq!((reverb.active_key, reverb.target_key, reverb.active_index, reverb.target_index), (FIRST_REVERB, FIRST_REVERB, 0, 0));
        assert_eq!((reverb.slots[0].errp.fxlevel, reverb.slots[1].errp.fxlevel), (1.0, 0.0));
        assert_eq!(reverb.pevs, [None, None]);
    }
}
