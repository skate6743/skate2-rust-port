//! The contests' side of `Sk8::Audio::AudioSystem`: the mission sound effects
//! (`PlayMissionSFXImp` 0x824fecb8), the crowd banks of `data\audio\crowds.big` (`LoadCrowdsData`
//! 0x824fdcc8, `GetCrowdsLoadStatus` 0x824f0818, `UnloadCrowdData` 0x824f0918) and the traffic
//! banks they share their memory with (`LoadVehiclesData` 0x824fd868, `GetVehiclesLoadStatus`
//! 0x824f0718, `UnloadVehicleData` 0x824fdbc0), the contest's music, emitter states and crowds
//! (`TriggerMissionMusicImp` 0x824fee80, `CheckForRoundStart` 0x824f24b0, `RestartChallenge`
//! 0x824ff320) and the crowd reactions (`PlayCrowdSound` 0x824e69e0, `CacheCrowdPodInfos`
//! 0x824fd6c0, `GetNearestCrowdPodInfo` 0x824f0298).

use super::{AudioSystem, MissionMusic, SceneInfo, WHEEL_SOFT};
use crate::music::challenge_trigger;
use crate::player::SfxCtx;
use crate::vault::Collection;
use crate::vmx::{Vec3, ZERO};
use crate::world::crowd::{self, CrowdEnv, CrowdManager, CrowdPodInfo, CrowdSoundInputs, ARENA_CROWD, AUD_CROWDS, CHALLENGES, CROWD_TYPE};
use crate::world::emitter_assets::{AssetStatus, ASSET_AEMS_BANK};
use crate::world::emitter_system::EmitterIo;

/// `eModuleLoadStatus`.
pub mod module_status {
    pub const NO_HEADER: i32 = 0;
    pub const EMPTY: i32 = 1;
    pub const WAITING_TO_LOAD: i32 = 2;
    pub const LOADING: i32 = 3;
    pub const LOADED: i32 = 4;
    pub const UNLOADING: i32 = 5;
}

/// `eSk8MissionSFX`.
pub mod mission_sfx {
    pub const CAMERA_SHUTTER: i32 = 0;
    pub const OWN_SPOT: i32 = 1;
    pub const CONTEST_LOAD: i32 = 2;
    pub const CONTEST_START: i32 = 3;
    pub const CONTEST_END: i32 = 4;
    pub const CONTEST_UNLOAD: i32 = 5;
}

/// The crowd banks' asset bank and its slots (`sCrowdTypes`), one per entry of a challenge's
/// crowd list.
const CROWD_BANK: usize = 14;
const CROWD_SLOTS: [usize; 3] = [0, 1, 2];
/// The `challenges` attribute `LoadCrowdsData` reads: the crowd banks' `eCrowdSFX` sizes, 9 for
/// none.
const CROWD_SFX_TYPES: u64 = 0xFAF1_8C29_FE10_A934;
const CROWD_SFX_NONE: i32 = 9;
const CROWD_SFX_ARENA_AMBIENCE: usize = 6;
/// `sCrowdNamesThree`, `Five`, `Ten`, `LargeA`, `LargeB`, `LargeRand`, `ArenaAmbience`,
/// `ArenaCheers`, `ArenaOhhs`: two banks each, `rw::math::Random() & 1` choosing.
const CROWD_NAMES: [[&str; 2]; 9] = [
    ["crowds_threes_a.abk", "crowds_threes_b.abk"],
    ["crowds_fives_a.abk", "crowds_fives_b.abk"],
    ["crowds_tens_a.abk", "crowds_tens_b.abk"],
    ["crowds_hundreds_a.abk", "crowds_hundreds_a.abk"],
    ["crowds_hundreds_b.abk", "crowds_hundreds_b.abk"],
    ["crowds_hundreds_a.abk", "crowds_hundreds_b.abk"],
    ["arena_ambience.abk", "arena_ambience.abk"],
    ["arena_cheers.abk", "arena_cheers.abk"],
    ["arena_ohhs.abk", "arena_ohhs.abk"],
];
/// The banks' share of `mpBigFileSwapBlock`.
const CROWD_SWAP_SIZE: usize = 0x48_0000;

/// The traffic banks: the engines (`sEngineNames`, asset bank 10, slots `sTrafficCarEngineTypes`)
/// and the horn, car alarms and skids (asset bank 11).
const ENGINE_BANK: usize = 10;
const TRAFFIC_BANK: usize = 11;
const ENGINE_NAMES: [&str; 8] = [
    "C00_heavy01.abk",
    "C01_family01.abk",
    "C03_sports01.abk",
    "C04_taxi01.abk",
    "C05_truck01.abk",
    "C06_sports02.abk",
    "C07_family02.abk",
    "C08_family03.abk",
];
const HORN_BANK: &str = "Traffic_Horn.abk";
const SKID_BANK: &str = "Traffic_Skid.abk";
const ALARM_BANK: &str = "car_alarms.abk";

/// `challenges` attributes: `ContestSpeaker` and `NISSpeaker` (`eEmitterStates`),
/// `MusicPlaylist` and `Name`.
const CONTEST_SPEAKER: u64 = 0x232D_684A_3922_08D4;
const NIS_SPEAKER: u64 = 0x16BD_53AB_B0F7_D19D;
const MUSIC_PLAYLIST: u64 = 0x874E_C914_B4D3_4708;
const NAME: u64 = 0x942A_B8AE_E4B4_14ED;
/// `TriggerMissionMusicImp`'s playlist when the challenge has no `MusicPlaylist`.
const DEFAULT_CONTEST_PLAYLIST: u64 = 0x614B_4D4D_8251_0720;
/// `eEmitterStates`: the first contest state, and the one past the last.
const EMITTER_STATE_FIRST: u32 = 4;
const EMITTER_STATE_END: u32 = 0x8_0000;
/// The menu sound `PlayMissionSFXImp` plays for the camera's shutter.
const CAMERA_SHUTTER_SOUND: u64 = 0x77B0_44DC_217D_905E;
/// The jam whose contest has crowds and speakers (`LoadCrowdsAndSpeakerSystem` 0x824e7d58).
const DANNY_WAY_JAM: &str = "m_3a_03_besttrick_danny";
/// `eChallengeTypes`: Demo, Best Trick, Jam, Point-to-Point, Death and Spot races, Mega Ramp
/// and Street Skating.
const DEMO: i32 = 2;
const BEST_TRICK: i32 = 8;
const JAM: i32 = 15;
const RACES: [i32; 3] = [18, 19, 20];
const MEGA_RAMP: i32 = 26;
const STREET: i32 = 27;

/// The maximum of the slots' `eAssetStatus` (Empty < Loaded < Loading < Unloading), as
/// `GetCrowdsLoadStatus` and `GetVehiclesLoadStatus` fold them.
fn status_rank(status: AssetStatus) -> u8 {
    match status {
        AssetStatus::Empty => 0,
        AssetStatus::Loaded => 1,
        AssetStatus::Loading => 2,
        AssetStatus::Unloading => 3,
    }
}

/// `GetCrowdsLoadStatus`/`GetVehiclesLoadStatus`'s update of the module status from its banks'.
fn module_status(current: &mut i32, banks: AssetStatus) -> i32 {
    match banks {
        AssetStatus::Empty if *current == module_status::UNLOADING => *current = module_status::EMPTY,
        AssetStatus::Empty => {}
        AssetStatus::Loaded => *current = module_status::LOADED,
        AssetStatus::Loading => *current = module_status::LOADING,
        AssetStatus::Unloading => *current = module_status::UNLOADING,
    }
    *current
}

/// `LoadCrowdsAndSpeakerSystem` 0x824e7d58: Demos and Best Tricks, and the Danny Way jam.
fn loads_crowds_and_speakers(challenge_type: i32, name: Option<&str>) -> bool {
    match challenge_type {
        DEMO | BEST_TRICK => true,
        JAM => name == Some(DANNY_WAY_JAM),
        _ => false,
    }
}

/// The collection of a `RefSpec` (bytes 8..16).
fn ref_spec_collection(bytes: &[u8]) -> u64 {
    bytes.get(8..16).map_or(0, |key| u64::from_be_bytes(key.try_into().expect("eight bytes")))
}

impl AudioSystem {
    /// `PlayMissionSFX` 0x82504e70 (+0x38, `ChallengeAudioManager::PlayMissionSFX`): posts
    /// `cMsgPlayMissionSFX`, which the next update's group A hands `PlayMissionSFXImp`
    /// (`PlayMissionSFXMsg` 0x82504ec0). `mission` is what the handler reads of the challenge.
    pub fn play_mission_sfx(&mut self, sfx: i32, mission: MissionMusic) {
        self.messages.push_back(super::AudioMessage::PlayMissionSfx { sfx, mission });
    }

    /// `CacheCrowdPodInfos` 0x824fd6c0, from `TriggerMissionMusic` 0x82504f28 at a countdown on the
    /// game's side: the spectator groups (`waypoint_spectator` props with at least two spectators)
    /// within 1000 of the local skater, at most thirty, as the host found them.
    pub fn cache_crowd_pod_infos(&mut self, pods: &[CrowdPodInfo]) {
        self.crowd_pod_infos = [CrowdPodInfo::default(); 30];
        for (slot, pod) in self.crowd_pod_infos.iter_mut().zip(pods) {
            *slot = *pod;
        }
    }

    /// `mbIsOnlineGame`.
    fn is_online_game(&self) -> bool {
        self.state.online_game
    }

    /// The `challenges` collection of a key.
    fn challenge(key: u64) -> Collection {
        Collection::new(CHALLENGES, key)
    }

    /// `GetCrowdType` 0x824f2398: none on line; else the `crowd_type` of the challenge's
    /// `CrowdTuning` (0 without one).
    pub(super) fn crowd_type(&self, challenge: u64) -> i32 {
        if self.is_online_game() {
            return 0;
        }
        let tuning = ref_spec_collection(&self.vault.bytes(Self::challenge(challenge), crowd::CROWD_TUNING, 0, 24));
        self.vault.i32(Collection::new(AUD_CROWDS, tuning), CROWD_TYPE)
    }

    /// `PlayMissionSFXImp` 0x824fecb8: the camera's shutter (a menu sound), Own the Spot
    /// (`mbInOwnTheSpot`, never cleared), a contest's load (with crowds and speakers: the contest
    /// id, its crowd banks when it has a crowd, the speakers' multi-speaker mode off line) and
    /// unload (the crowds stop and the traffic banks return, the music's `CHALLENGE_EXIT`, the
    /// speakers and the emitter state off).
    pub(super) fn play_mission_sfx_imp(&mut self, sfx: i32, mission: MissionMusic) {
        let crowd_type = self.crowd_type(mission.challenge);
        match sfx {
            mission_sfx::CAMERA_SHUTTER => self.play_menu_sound(CAMERA_SHUTTER_SOUND, false),
            mission_sfx::OWN_SPOT => self.in_own_the_spot = true,
            mission_sfx::CONTEST_LOAD => {
                let name = self.vault.text(Self::challenge(mission.challenge), NAME);
                if let Some(elapsed) = crate::splicer::trace_elapsed() {
                    eprintln!("{elapsed:9.3} contest load {name:?}: type {}, crowd type {crowd_type}, speakers {}", mission.challenge_type, loads_crowds_and_speakers(mission.challenge_type, name.as_deref()));
                }
                if loads_crowds_and_speakers(mission.challenge_type, name.as_deref()) {
                    self.contest_id = mission.challenge;
                    self.contest_type = mission.challenge_type;
                    if crowd_type != 0 {
                        self.load_crowds_data(mission.challenge);
                    }
                    if !self.is_online_game() {
                        if let Some(speakers) = self.speakers.as_mut() {
                            speakers.set_multi_speaker_mode(true);
                        }
                    }
                }
            }
            mission_sfx::CONTEST_UNLOAD => {
                if let Some(elapsed) = crate::splicer::trace_elapsed() {
                    eprintln!("{elapsed:9.3} contest unload: crowd type {crowd_type}");
                }
                if crowd_type != 0 {
                    self.stop_crowds();
                    self.load_vehicles_data(false);
                }
                let contest_loaded = self.contest_id != 0;
                let challenge_type = mission.challenge_type;
                self.with_music(|music, io| music.challenge_trigger(challenge_trigger::EXIT, 0, challenge_type, contest_loaded, io));
                if let Some(speakers) = self.speakers.as_mut() {
                    speakers.set_multi_speaker_mode(false);
                }
                self.emitter_system.set_game_state(self.current_emitter_state, false);
                self.current_emitter_state = 0;
                self.emitters_set_up = false;
                self.contest_id = 0;
            }
            _ => {}
        }
    }

    /// `EmitterSystem::EnableEmitterSwap` 0x82a82bd8 with the EMITTER states.
    fn enable_emitter_swap(&mut self, enable: bool) {
        let Self { core, mixer, vault, rng, aems, emitters, emitter_system, .. } = self;
        let (Some(map), Some(emitters)) = (mixer.map_mut(), emitters.as_mut()) else { return };
        let mut io = EmitterIo { core, aems, map, vault, rng };
        emitter_system.enable_emitter_swap(enable, emitters, &mut io);
    }

    /// The maximum status of a fixed asset bank's slots.
    fn bank_status(&self, bank: usize, slots: impl IntoIterator<Item = usize>) -> AssetStatus {
        let assets = self.emitter_system.assets();
        slots.into_iter().map(|slot| assets.async_status(bank, slot)).max_by_key(|&status| status_rank(status)).unwrap_or(AssetStatus::Empty)
    }

    /// `GetCrowdsLoadStatus` 0x824f0818.
    pub(super) fn crowds_load_status(&mut self) -> i32 {
        let banks = self.bank_status(CROWD_BANK, CROWD_SLOTS);
        module_status(&mut self.crowds_load, banks)
    }

    /// `GetVehiclesLoadStatus` 0x824f0718: the engines' slots.
    fn vehicles_load_status(&mut self) -> i32 {
        let banks = self.bank_status(ENGINE_BANK, 0..ENGINE_NAMES.len());
        module_status(&mut self.vehicles_load, banks)
    }

    /// `AsyncLoadAt` of a bank into a fixed asset bank's slot.
    fn async_load_at(&mut self, name: &str, bank: usize, slot: usize) {
        let Self { core, aems, emitter_system, .. } = self;
        emitter_system.assets_mut().async_load_at(name, bank, slot, ASSET_AEMS_BANK, aems, core);
    }

    /// `UnLoad` of a fixed asset bank's slot.
    fn unload_bank_slot(&mut self, bank: usize, slot: usize) {
        let Self { core, aems, emitter_system, .. } = self;
        emitter_system.assets_mut().unload_bank_slot(bank, slot, aems, core);
    }

    /// `LoadCrowdsData` 0x824fdcc8: unless loaded, the contest's banks wait for the emitter swap
    /// (turned off) and the traffic banks (unloaded) to leave their memory; then each entry of
    /// its crowd list other than none draws one of its two banks (an arena ambience playing on
    /// load) into the next crowd slot while the memory lasts.
    pub(super) fn load_crowds_data(&mut self, key: u64) {
        if self.crowds_load_status() == module_status::LOADED {
            return;
        }
        self.contest_key = key;
        self.enable_emitter_swap(false);
        self.unload_vehicle_data();
        self.crowds_load = module_status::WAITING_TO_LOAD;
        if self.emitter_system.is_swap_enabled() || self.vehicles_load_status() > module_status::EMPTY {
            return;
        }
        self.crowds_load = module_status::LOADING;
        self.crowd_banks_loaded = [false; 9];
        let challenge = Self::challenge(key);
        let length = self.vault.length(challenge, CROWD_SFX_TYPES);
        let mut offset = 0;
        for index in 0..length {
            let sfx = self.vault.i32_at(challenge, CROWD_SFX_TYPES, index);
            if sfx == CROWD_SFX_NONE {
                continue;
            }
            let pick = (self.rng.next_u32() & 1) as usize;
            let Some(names) = usize::try_from(sfx).ok().and_then(|sfx| CROWD_NAMES.get(sfx)) else { continue };
            let name = names[pick];
            if sfx as usize == CROWD_SFX_ARENA_AMBIENCE {
                self.play_arena_on_load = true;
            }
            let Some(size) = self.emitter_system.assets().file_size(name) else { continue };
            let rounded = ((size >> 7) + 1) << 7;
            if offset + rounded >= CROWD_SWAP_SIZE {
                break;
            }
            let Some(&slot) = CROWD_SLOTS.get(index as usize) else { break };
            self.async_load_at(name, CROWD_BANK, slot);
            self.crowd_banks_loaded[sfx as usize] = true;
            offset += rounded;
        }
    }

    /// `UnloadCrowdData` 0x824f0918: the crowd slots unload and the emitter swap turns back on.
    pub(super) fn unload_crowd_data(&mut self) {
        if self.crowds_load_status() == module_status::NO_HEADER {
            return;
        }
        self.crowds_load = module_status::UNLOADING;
        self.contest_key = 0;
        for slot in CROWD_SLOTS {
            self.unload_bank_slot(CROWD_BANK, slot);
        }
        self.enable_emitter_swap(true);
        self.crowd_banks_loaded = [false; 9];
    }

    /// `LoadVehiclesData` 0x824fd868: the crowd banks unload; the traffic banks wait for them
    /// (`bForceLoadOnUpdate` always waits for the next update), then the engines, the horn, the
    /// skids and the car alarms load.
    pub(super) fn load_vehicles_data(&mut self, force_load_on_update: bool) {
        self.unload_crowd_data();
        self.vehicles_load = module_status::WAITING_TO_LOAD;
        if force_load_on_update || self.crowds_load_status() > module_status::EMPTY {
            return;
        }
        self.vehicles_load = module_status::LOADING;
        for (slot, name) in ENGINE_NAMES.iter().enumerate() {
            if self.emitter_system.assets().file_size(name).is_some() {
                self.async_load_at(name, ENGINE_BANK, slot);
            }
        }
        self.async_load_at(HORN_BANK, TRAFFIC_BANK, 0);
        self.async_load_at(SKID_BANK, TRAFFIC_BANK, 2);
        self.async_load_at(ALARM_BANK, TRAFFIC_BANK, 1);
    }

    /// `UnloadVehicleData` 0x824fdbc0.
    fn unload_vehicle_data(&mut self) {
        if self.vehicles_load_status() == module_status::NO_HEADER {
            return;
        }
        self.vehicles_load = module_status::UNLOADING;
        for slot in 0..ENGINE_NAMES.len() {
            self.unload_bank_slot(ENGINE_BANK, slot);
        }
        for slot in 0..3 {
            self.unload_bank_slot(TRAFFIC_BANK, slot);
        }
    }

    /// `AudioSystem::Update`'s loads that waited (`mCrowdsLoadStatus`, `mVehiclesLoadStatus`
    /// `WaitingToLoad`).
    pub(super) fn update_waiting_loads(&mut self) {
        if self.crowds_load == module_status::WAITING_TO_LOAD {
            self.load_crowds_data(self.contest_key);
        }
        if self.vehicles_load == module_status::WAITING_TO_LOAD {
            self.load_vehicles_data(false);
        }
    }

    /// `AudioSystem::Update` after the HUD splices: an arena loaded with `mbPlayArenaOnLoad`
    /// starts its crowd.
    pub(super) fn update_arena_on_load(&mut self) {
        if !self.play_arena_on_load || self.crowds_load_status() != module_status::LOADED {
            return;
        }
        if self.crowds.is_some() {
            let challenge = self.contest_id;
            self.start_crowds(ARENA_CROWD, challenge);
        }
        self.play_arena_on_load = false;
    }

    /// The local skater's position (`AudioPresentation::mLocal->skaterPos`), zero without one.
    pub(super) fn local_skater_position(&self) -> Vec3 {
        self.world.as_ref().and_then(|world| world.player.skaters.iter().find(|skater| skater.local)).map_or(ZERO, |skater| skater.skater_pos)
    }

    /// The crowd manager with the context its state objects play in, outside the update passes.
    fn with_crowds<R>(&mut self, f: impl FnOnce(&mut CrowdManager, &[bool; 9], &mut SfxCtx<'_>) -> R) -> Option<R> {
        let in_own_the_spot = self.in_own_the_spot || self.state.in_own_the_spot;
        let cond_score_delta = self.cond_score - self.cond_prev_score;
        let Self { core, splicer, mixer, master_sub_mix, global_effects, vault, skate3_vault, rng, microphones, cvars, world, aems, state, times, crowds, collision_requests, crowd_banks_loaded, .. } = self;
        let crowds = crowds.as_mut()?;
        let map = mixer.map_mut()?;
        let (bail, bail_severity) = world.as_ref().map_or((false, 0.0), |world| (world.bail, world.bail_severity));
        let (out_of_bounds, out_of_bounds_amount) = world.as_ref().map_or((false, 0.0), |world| (world.out_of_bounds, world.out_of_bounds_amount));
        let mut ctx = SfxCtx {
            core,
            aems,
            splicer,
            map,
            vault,
            skate3: skate3_vault.as_deref(),
            rng,
            master_sub_mix: *master_sub_mix,
            global_effects,
            cvars: *cvars,
            delta_time: 0.0,
            local: false,
            bail,
            bail_severity,
            out_of_bounds,
            out_of_bounds_amount,
            hall_of_meat: false,
            times: *times,
            in_own_the_spot,
            challenge_type: state.challenge_type,
            camera_position: microphones.microphones[0].cur_position,
            npc_wheel_type: WHEEL_SOFT,
            collisions: collision_requests,
            crowds: None,
            crowd_env: None,
            cond_score_delta,
        };
        Some(f(crowds, crowd_banks_loaded, &mut ctx))
    }

    /// `CSTATEMGR_Crowd::StartCrowds` 0x82557810 with the crowd banks' status and the local skater.
    pub(super) fn start_crowds(&mut self, crowd_type: i32, challenge: u64) {
        let loaded = self.crowds_load_status() >= module_status::LOADED;
        let position = self.local_skater_position();
        self.with_crowds(|crowds, banks, ctx| crowds.start_crowds(crowd_type, challenge, loaded, position, banks, ctx));
    }

    /// `CSTATEMGR_Crowd::StopCrowds` 0x82554210, which ends with `UnloadCrowdData`.
    fn stop_crowds(&mut self) {
        self.with_crowds(|crowds, _, ctx| crowds.stop_crowds(ctx));
        self.unload_crowd_data();
    }

    /// What `CSTATEMGR_Crowd::PlayCrowdSound` reads for a skater: its position, the crowd
    /// volumes and the nearest pods.
    fn crowd_sound_inputs(&self, skater_id: i32) -> CrowdSoundInputs {
        let skaters = self.world.as_ref().map_or(&[][..], |world| &world.player.skaters[..]);
        let env = CrowdEnv { skaters, emitter_system: &self.emitter_system, pods: &self.crowd_pod_infos, banks_loaded: &self.crowd_banks_loaded };
        env.inputs(skater_id, &self.vault)
    }

    /// `AudioSystem::PlayCrowdSound` 0x824e69e0: `CSTATEMGR_Crowd::PlayCrowdSound`.
    fn play_crowd_sound(&mut self, skater_id: i32, score: i32, wiped_out_score: i32, local: bool) {
        let inputs = self.crowd_sound_inputs(skater_id);
        self.with_crowds(|crowds, banks, ctx| crowds.play_crowd_sound(skater_id, score, wiped_out_score, local, &inputs, banks, ctx));
    }

    /// `TriggerMissionMusicImp` 0x824fee80. Demos, Best Tricks, Mega Ramps, Street Skating and the
    /// Danny Way jam play the contest music (`MusicSystem::ChallengeTrigger` with the challenge's
    /// `MusicPlaylist`), the countdown first starting a crowd with its opening cheer and moving
    /// the emitters to the contest's speaker state; Point-to-Point, Death and Spot races hand
    /// the trigger to `RaceTrigger`.
    pub(super) fn trigger_mission_music_imp(&mut self, trigger: i32, mission: MissionMusic) {
        let challenge_type = mission.challenge_type;
        let plays = match challenge_type {
            DEMO | BEST_TRICK | MEGA_RAMP | STREET => true,
            JAM => self.vault.text(Self::challenge(mission.challenge), NAME).as_deref() == Some(DANNY_WAY_JAM),
            t => RACES.contains(&t),
        };
        if !plays {
            return;
        }
        let request = match trigger {
            0 => challenge_trigger::COUNTDOWN,
            2 => challenge_trigger::HALFWAY,
            3 => challenge_trigger::THREE_QUARTERS,
            4 => challenge_trigger::STRAIGHT_AWAY,
            5 => challenge_trigger::ABORT,
            6 => challenge_trigger::END_ROUND,
            7 => challenge_trigger::OVER,
            8 => challenge_trigger::WIN,
            9 => challenge_trigger::LOSE,
            10 => challenge_trigger::JUMP_SHORT,
            11 => challenge_trigger::MAX,
            _ => challenge_trigger::START,
        };
        if request == challenge_trigger::COUNTDOWN {
            let crowd_type = self.crowd_type(mission.challenge);
            if crowd_type != 0 {
                self.start_crowds(crowd_type, mission.challenge);
                self.play_crowd_sound(0, 500, 0, true);
            }
        }
        let challenge = Self::challenge(mission.challenge);
        let playlist = if self.vault.length(challenge, MUSIC_PLAYLIST) > 0 {
            ref_spec_collection(&self.vault.bytes(challenge, MUSIC_PLAYLIST, 0, 24))
        } else {
            DEFAULT_CONTEST_PLAYLIST
        };
        match challenge_type {
            DEMO | BEST_TRICK | JAM | MEGA_RAMP | STREET => {
                let contest_loaded = self.contest_id != 0;
                self.with_music(|music, io| music.challenge_trigger(request, playlist, challenge_type, contest_loaded, io));
                if trigger == 0 {
                    self.set_up_contest_emitters(mission.challenge);
                }
            }
            t if RACES.contains(&t) => self.music.race_trigger(request, mission.challenge, mission.race_tuning),
            _ => {}
        }
    }

    /// The countdown's emitter state: a contest not set up (or set up by its scene) takes its
    /// `ContestSpeaker` state (the first contest state without one, or for none or the end); one
    /// set up for the round steps to the next state (wrapping to the first); the state is added.
    fn set_up_contest_emitters(&mut self, challenge: u64) {
        let current = self.current_emitter_state;
        if current == 0 || self.nis_emitters_set_up {
            if self.nis_emitters_set_up {
                self.emitter_system.set_game_state(current, false);
                self.nis_emitters_set_up = false;
            }
            self.current_emitter_state = EMITTER_STATE_FIRST;
            let collection = Self::challenge(challenge);
            if self.vault.length(collection, CONTEST_SPEAKER) > 0 {
                let state = self.vault.i32(collection, CONTEST_SPEAKER) as u32;
                self.current_emitter_state = if state == 0 || state == EMITTER_STATE_END { EMITTER_STATE_FIRST } else { state };
            }
            self.emitters_set_up = false;
        } else if !self.emitters_set_up {
            self.emitter_system.set_game_state(current, false);
            let next = current.wrapping_mul(2);
            self.current_emitter_state = if next == EMITTER_STATE_END { EMITTER_STATE_FIRST } else { next };
        } else {
            self.emitters_set_up = false;
        }
        self.emitter_system.set_game_state(self.current_emitter_state, true);
    }

    /// `CheckForRoundStart` 0x824f24b0 (`NotifyNISPlaybackStart`): outside a replay, a loaded
    /// contest's emitters not yet set up step to the next state, or first take the scene's
    /// `NISSpeaker` state.
    pub(super) fn check_for_round_start(&mut self, scene: SceneInfo) {
        if scene.is_replay_nis || self.emitters_set_up || self.contest_id == 0 {
            return;
        }
        let current = self.current_emitter_state;
        if current != 0 {
            self.emitter_system.set_game_state(current, false);
            let next = current.wrapping_mul(2);
            self.current_emitter_state = if next == EMITTER_STATE_END { EMITTER_STATE_FIRST } else { next };
        } else {
            self.current_emitter_state = EMITTER_STATE_FIRST;
            let collection = Self::challenge(self.contest_id);
            if self.vault.length(collection, NIS_SPEAKER) > 0 {
                let state = self.vault.i32(collection, NIS_SPEAKER) as u32;
                self.current_emitter_state = if state == 0 || state == EMITTER_STATE_END { EMITTER_STATE_FIRST } else { state };
                self.nis_emitters_set_up = true;
            }
        }
        self.emitter_system.set_game_state(self.current_emitter_state, true);
        self.emitters_set_up = true;
    }

    /// `RestartChallenge` 0x824ff320 with a contest loaded: the music waits for the next start, the
    /// emitter state clears and the leaderboard leader resets.
    pub(super) fn restart_contest(&mut self) {
        self.music.restart_contest();
        if self.current_emitter_state != 0 {
            self.emitter_system.set_game_state(self.current_emitter_state, false);
        }
        self.current_emitter_state = 0;
        self.emitters_set_up = false;
        self.last_contest_leader = 100;
    }

    /// `mContestId` and its challenge type while a contest is loaded, for the music system.
    pub(super) fn loaded_contest(&self) -> Option<(u64, i32)> {
        (self.contest_id != 0).then_some((self.contest_id, self.contest_type))
    }
}
