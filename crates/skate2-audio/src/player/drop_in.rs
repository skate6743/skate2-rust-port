//! `Sk8::Audio::SFXObj_DropIn` (TU3 only: 48 bytes, PLAYER object 12, TypeInfo 0x000100c0,
//! `CreateObject` 0x824f03b8): the local skater's drop-in. The tail locking on the edge
//! (`SFXCTL_PlayerPhysics` +0x329, from the animation's DropInLock attribute) and letting go of it
//! (+0x32A, DropInRelease) each start their `aud_splicer/dropin` sound from Skate 3's splice bank 0,
//! through the Skate 3 effect the sound's attribute names, in a splice voice of its own
//! ([`Skate3Splice`]) that also sends to the environment reverb. Its mixer channels are TU3's
//! ([`super::skate3_mix::DROP_IN_CHANNELS`]).

use super::contacts::{Contacts, SKATE3_EFFECTS, SPLICE_SKATE3_COLLISIONS};
use super::physics::PlayerPhysics;
use super::skate3_mix::{follow, Skate3Splice};
use super::SfxCtx;
use crate::nicotine::DMixIoId;
use crate::vault::{class, Collection};

/// `GlobalAttributeCollection` +0x80: `aud_splicer` "dropin", each sound's cue in Skate 3's splice
/// bank 0. +0x8C ([`SKATE3_EFFECTS`]) names its effect under the same key.
const DROP_IN: Collection = Collection::new(class::AUD_SPLICER, 0x6E4F_7FF8_3CB9_886C);
/// The lock's and the release's attribute keys (0x824f0644, 0x824f078c).
const LOCK: u64 = 0x428F_DFC2_E99F_FA65;
const RELEASE: u64 = 0x2F88_0F05_9AB9_B151;
/// Skate 3's splice bank 0.
const SKATE3_COLLISIONS_BANK: &str = "Skate_Collisions.bnk";

/// `SFXObj_DropIn`; the constructor 0x824f03b8 clears everything.
#[derive(Default)]
pub struct DropIn {
    /// `m_pDMixIO` (objectID 0x400100c0 + the state's instance).
    pub io: Option<DMixIoId>,
    /// +0x24, +0x25: the lock and release flags as last seen.
    lock_seen: bool,
    release_seen: bool,
    /// +0x28, +0x2C.
    lock_sound: Option<Skate3Splice>,
    release_sound: Option<Skate3Splice>,
}

impl DropIn {
    /// The shared `InitSFX` 0x824b81f0: the mixer output on.
    pub fn init_sfx(&mut self, ctx: &mut SfxCtx<'_>) {
        if let Some(io) = self.io {
            ctx.map.set_output_enabled(io, true);
        }
    }

    /// `UpdateParams` 0x824f0578 (the local skater of an attached state): the lock (0x824f05d0),
    /// then the release (0x824f0718).
    pub fn update_params(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !ctx.local {
            return;
        }
        Self::trigger(&mut self.lock_sound, &mut self.lock_seen, physics.skate3_drop_in_lock, LOCK, ctx);
        Self::trigger(&mut self.release_sound, &mut self.release_seen, physics.skate3_drop_in_release, RELEASE, ctx);
    }

    /// `ProcessUpdate` 0x824effe0 (the local skater of an attached state): each sound follows the
    /// mixer until it ends, the lock (0x824f0038) at volume output 1 and pitch output 2, the release
    /// (0x824f01e8) at volume output 3 and pitch output 4, both with the reverb send at output 5.
    pub fn process_update(&mut self, ctx: &mut SfxCtx<'_>) {
        if !ctx.local {
            return;
        }
        follow(&mut self.lock_sound, self.io, 1, 2, 5, ctx);
        follow(&mut self.release_sound, self.io, 3, 4, 5, ctx);
    }

    /// `Destroy` 0x824ef7f0, which `Detach` runs: both sounds deleted (the flags stay).
    pub fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        Skate3Splice::delete_slot(&mut self.lock_sound, ctx);
        Skate3Splice::delete_slot(&mut self.release_sound, ctx);
    }

    /// The flag rising starts its sound anew, deleting the one before.
    fn trigger(sound: &mut Option<Skate3Splice>, seen: &mut bool, flag: bool, key: u64, ctx: &mut SfxCtx<'_>) {
        if flag && !*seen {
            Skate3Splice::delete_slot(sound, ctx);
            *sound = Self::play(key, ctx);
        }
        *seen = flag;
    }

    /// The sound `key` names: its cue, then its Skate 3 effect (`sub_82491108`, which switches the
    /// effect on for the local skater), then the splice voice into that effect. Without linked
    /// Skate 3 data there is nothing to play.
    fn play(key: u64, ctx: &mut SfxCtx<'_>) -> Option<Skate3Splice> {
        let vault = ctx.skate3?;
        let cue = vault.i32(DROP_IN, key);
        let effect = vault.i32(SKATE3_EFFECTS, key);
        let target = ctx.global_effects.skate3_effect(ctx.core, effect, ctx.local, vault, ctx.rng);
        trace_drop_in(format_args!("sound {key:016x}: cue {cue} effect {effect}"));
        if !Contacts::skate3_bank(ctx, SPLICE_SKATE3_COLLISIONS, SKATE3_COLLISIONS_BANK) {
            return None;
        }
        Skate3Splice::new(ctx, SPLICE_SKATE3_COLLISIONS, cue, target)
    }
}

/// `SKATE2_TRACE_SKATE3_DROP_IN`: each drop-in sound as it starts.
fn trace_drop_in(message: std::fmt::Arguments<'_>) {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_DROP_IN").is_some()) {
        eprintln!("skate 3 drop-in audio: {message}");
    }
}
