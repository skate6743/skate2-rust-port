//! `Sk8::Audio::Sk8EmitterSystem` (`AudioSystem::mpEmitterSystem`, 0x220 bytes) over its base
//! `Sk8::Audio::EmitterSystem` (0x82a81100-0x82a83ae8): the sound volumes of the `.ems` data
//! files and the random location emitters.
//!
//! Each update (`EmitterSystem::Update` 0x82a83598, after the state managers' `ProcessUpdate`)
//! finds the volumes the camera is inside (`SearchOctTree`, `CheckVolume`) and keeps a status
//! per volume with its normalised distance. An AEMS volume loads its `emitters.big` bank into one
//! of five swap slots and plays it on a free EMITTER state (`PlayVolume`) whose level follows the
//! distance (`UpdateVolume`); music and reverb volumes are only tracked (`UpdateMusicVolumes`,
//! `GetCurrentReverbVolume` for `SFXObj_Reverb`). The location's `aud_wp_emitters` collection
//! (WorldPainter layer `eAudio_Emitters`) lists random one-shots, two of which stay loaded and
//! play at random intervals.
//!
//! The volumes' game states are only those `AudioSystem::UpdatePresentationData` 0x82504848 sets
//! from `GlobalData` (0x4000 for `damActive` in single player, 0x8000 for
//! `monsterConstructionActive`) and the challenge code; neither is ported, so only the volumes
//! of state 0 are active.

use super::emitter::{EmitterInfo, EmitterManager, AZIM_STATE_EXTERNAL, AZIM_STATE_NICOTINE, VOL_STATE_RANDOM, VOL_STATE_VOLUME};
use super::emitter_assets::{AssetStatus, AudioAssetManager, ASSET_AEMS_BANK, ASSET_DATA};
use crate::aems::{Aems, ClassObject};
use crate::nicotine::shape::fctiwz;
use crate::nicotine::MixMap;
use crate::random::RwRandom;
use crate::rwac::AudioCore;
use crate::vault::{Collection, Vault};
use crate::vmx::{self, Vec3};
use std::collections::VecDeque;
use std::sync::Arc;

/// `Attrib::Gen::aud_emitter`, `aud_wp_emitters` and `fe_locations` class keys.
const AUD_EMITTER: u64 = 0xF0CE_F367_088E_FFF8;
const AUD_WP_EMITTERS: u64 = 0x39A2_DE02_3291_2CE5;
const FE_LOCATIONS: u64 = 0xDDCD_6BC2_3255_E508;
/// `aud_emitter` attributes: the volume type (`GetVolumeType` 0x824f8248), the falloff
/// (`PlayVolume`), the alternative keys (`GetKeyFromVolume` 0x824f88e8), `reverbType`, the
/// music playlist, pan size and mid radius (`UpdateCurrentMusicVolume` 0x824f8cf0).
const VOLUME_TYPE: u64 = 0x6D18_B867_4D7E_5337;
const FALLOFF_TYPE: u64 = 0xF209_C093_F40A_4CCC;
const ALTERNATIVE_KEYS: u64 = 0xC48A_B73A_227B_3B00;
const REVERB_TYPE: u64 = 0x99FD_793B_C30C_F0FA;
const MUSIC_PLAYLIST: u64 = 0x442A_158E_27F2_B5AD;
const MUSIC_PAN_SIZE: u64 = 0x6391_FF22_F96A_4AD6;
const MUSIC_MID_RADIUS: u64 = 0x463B_21BB_4641_5616;
/// `aud_wp_emitters` attributes: the emitters (`RefSpec`s), their weights and the volume scale
/// `PlayEmitter` 0x824f85d8 draws in.
const WP_EMITTERS: u64 = 0x5CA0_6CA0_86D8_F953;
const WP_WEIGHTS: u64 = 0x07DD_ABF9_6E5F_2731;
const WP_VOLUME_SCALE: u64 = 0x7946_0EEF_1A82_53B0;
/// `fe_locations` attributes: whether the location has emitter data, and its file.
const DLC_HAS_EMITTERS: u64 = 0x0CC9_B87A_C0A4_3674;
const DLC_EMITTER_FILE: u64 = 0x9FD7_215F_00B0_81FB;

/// `Sk8::Audio::verbHash` 0x82fd8158: the reverb keys `VerifyReverbKey` 0x824f8ec0 accepts.
const VERB_HASH: [u64; 24] = [
    0xA278_2D75_A971_CC8C,
    0x407A_FA1D_6C7C_EAD8,
    0x6762_10D7_62D1_4388,
    0x9572_128E_FDA5_C188,
    0x68A9_E602_0DF2_076D,
    0x2C23_0A86_14C3_60EB,
    0xF84A_CA4E_713D_BCAE,
    0x4010_9DD4_2CA2_FF94,
    0x138D_A6EB_38A4_F705,
    0x51DE_CB09_7865_7BD2,
    0xBEEF_C8E3_DE04_FBAE,
    0xA40C_D7A7_7E6C_6572,
    0xD265_9BDE_9978_CC0B,
    0x47CA_9712_17C8_A2E5,
    0xD918_03A4_5ED1_1E5E,
    0xC1B2_248F_0522_A937,
    0x9339_F7B7_E0FE_2167,
    0x0EEF_F731_6BF2_88E8,
    0xA8A5_222E_8874_19FC,
    0x5F6F_5424_4BF0_57F2,
    0xDD0B_4DF2_D968_1029,
    0x8E7B_8D05_E04C_7BDF,
    0x5754_CD40_481A_8EB4,
    0xE025_5966_A05D_EED2,
];

/// `eVolumeType`.
pub const VOLUME_AEMS: u32 = 1;
pub const VOLUME_MUSIC: u32 = 4;
pub const VOLUME_REVERB: u32 = 5;
pub const VOLUME_SPEAKER: u32 = 6;
pub const VOLUME_CROWD: u32 = 7;

/// `eEmitterLoadStatus`.
const NOT_LOADED: u32 = 0;
const LOADED: u32 = 1;
const UNLOADING: u32 = 2;

/// `eEmitterSwapStatus`.
const SWAP_DISABLED: u32 = 0;
const SWAP_UNLOADING: u32 = 1;
const SWAP_UNRESOLVING: u32 = 2;
const SWAP_ENABLED: u32 = 3;

/// `eChallengeTypes_Point2PointRace`, `DeathRace` and `SpotRace`.
const RACES: [i32; 3] = [18, 19, 20];

/// The `EmitterSystem` constructor's arguments (`AudioSystem::AudioSystem` 0x8250dda0): five
/// swap slots of 0x64000 bytes; the DLC swap bank has one slot of 0x7800.
const NUM_SWAP_SLOTS: usize = 5;
/// `cManagedOctTree<tEmitterVolume*, 1500>`.
const MAX_VOLUMES: usize = 1500;
/// Freed statuses whose last values a stale `SFXObj_Reverb::mpevs` pointer still reads.
const FREED_STATUSES: usize = 16;

/// `Sk8::Audio::tEmitterVolume` (72 bytes in a data file).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmitterVolume {
    pub identifier: u32,
    /// The game states the volume needs (0: always).
    pub states: u32,
    pub pos: [f32; 3],
    pub size: [f32; 3],
    pub inner_radius: f32,
    pub orientation: [f32; 3],
    /// The `aud_emitter` collection.
    pub key: u64,
}

impl EmitterVolume {
    pub const SIZE: usize = 72;

    fn parse(bytes: &[u8]) -> Self {
        let word = |at: usize| u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
        let float = |at: usize| f32::from_bits(word(at));
        let triple = |at: usize| [float(at), float(at + 4), float(at + 8)];
        Self {
            identifier: word(0),
            states: word(4),
            pos: triple(0x08),
            size: triple(0x14),
            inner_radius: float(0x20),
            orientation: triple(0x24),
            key: (u64::from(word(0x30)) << 32) | u64::from(word(0x34)),
        }
    }
}

/// An emitter data file (`.ems`): a big-endian count, then the volumes.
#[must_use]
pub fn parse_data_file(bytes: &[u8]) -> Vec<EmitterVolume> {
    let Some(count) = bytes.get(..4).map(|b| i32::from_be_bytes([b[0], b[1], b[2], b[3]])) else { return Vec::new() };
    bytes[4..].chunks_exact(EmitterVolume::SIZE).take(count.max(0) as usize).map(EmitterVolume::parse).collect()
}

/// The emitter data `AudioSystem::Initialize` reads.
#[derive(Default)]
pub struct EmitterFiles {
    /// `Sk8EmitterSystem::LoadGameFiles` 0x824e9e00's files in its order: `data\audio\sfx.ems`,
    /// `reverb.ems`, `music.ems`, `speakers.ems`, `crowds.ems`.
    pub data_files: Vec<(String, Vec<u8>)>,
    /// `data\audio\emitters.big`'s banks by entry name.
    pub banks: Vec<(String, Arc<[u8]>)>,
}

/// The loader path `GetEmitterFilename` 0x824f8450 builds for a bank of `emitters.big`.
fn emitter_bank_path(name: &str) -> String {
    format!("data\\audio/emitters.big|{name}").replace('/', "\\").to_ascii_lowercase()
}

/// A volume of a loaded data file (a `tEmitterVolume*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VolumeRef {
    file: usize,
    index: usize,
}

/// `cManagedOctTree<tEmitterVolume*, 1500>`: `Insert` 0x82a81398 reuses the last freed entry,
/// else takes the next; `GetCollisions` 0x82a832e8 sorts the entries whose box overlaps the query
/// (`CollisionValid` accepts every one), so the octree's nodes only narrow the search.
#[derive(Debug, Default)]
struct VolumeTree {
    entries: Vec<Option<(VolumeRef, [f32; 3], [f32; 3])>>,
    free: Vec<usize>,
}

impl VolumeTree {
    fn insert(&mut self, volume: VolumeRef, min: [f32; 3], max: [f32; 3]) {
        let entry = Some((volume, min, max));
        if let Some(index) = self.free.pop() {
            self.entries[index] = entry;
        } else if self.entries.len() < MAX_VOLUMES {
            self.entries.push(entry);
        }
    }

    /// `Remove` 0x82a833c8: the first entry holding the volume.
    fn remove(&mut self, volume: VolumeRef) {
        if let Some(index) = self.entries.iter().position(|entry| entry.is_some_and(|(held, _, _)| held == volume)) {
            self.entries[index] = None;
            self.free.push(index);
        }
    }

    fn collisions(&self, min: [f32; 3], max: [f32; 3]) -> Vec<VolumeRef> {
        self.entries
            .iter()
            .flatten()
            .filter(|(_, low, high)| (0..3).all(|axis| low[axis] <= max[axis] && min[axis] <= high[axis]))
            .map(|&(volume, _, _)| volume)
            .collect()
    }
}

/// `Sk8::Audio::tEmitterVolumeStatus` (40 bytes); `id` stands for its address.
#[derive(Debug, Clone, Copy)]
struct VolumeStatus {
    id: u32,
    volume: VolumeRef,
    load_status: u32,
    slot: i32,
    distance: f32,
    touched: bool,
    /// `pState`: an EMITTER state.
    state: Option<usize>,
    key: u64,
    distance_no_inner: f32,
}

/// `Sk8::Audio::tEmitterStatus` (32 bytes): a random emitter.
#[derive(Debug, Clone, Copy)]
struct EmitterStatus {
    key: u64,
    weight: i32,
    load_status: u32,
    tick: i32,
    slot: i32,
    remove: bool,
    state: Option<usize>,
}

/// `Sk8::Audio::tMusicVolumeInfo`: the music volume the camera is in, for `SFXObj_Music`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MusicVolumeInfo {
    pub identifier: u32,
    pub playlist: u64,
    pub pan_size: f32,
    pub volume: f32,
    pub mid_radius: f32,
    pub inner_radius: f32,
    pub distance: f32,
    pub distance_no_inner: f32,
    pub pos: [f32; 3],
}

/// A `tEmitterVolumeStatus*` as `SFXObj_Reverb::mpevs` holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolumeStatusId(u32);

/// What `SFXObj_Reverb` reads through a status pointer: its key, distance and volume position;
/// and the volume's identifier and own key (`pVolume`, `None` without one).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeStatusView {
    pub key: u64,
    pub distance: f32,
    pub pos: [f32; 3],
    pub volume: Option<(u32, u64)>,
}

/// The audio core, AEMS, the mixer, the vault and `rw::math::Random` the system works with.
pub struct EmitterIo<'a> {
    pub core: &'a mut AudioCore,
    pub aems: &'a mut Aems,
    pub map: &'a mut MixMap,
    pub vault: &'a Vault,
    pub rng: &'a mut RwRandom,
}

/// `AudioSystem::GetEmitterKey` 0x824f1cf0: the presentation's `GlobalData.emitterKey`, none in a
/// race. No additional location is loaded (`mbUseAdditionalKeys`) and the music system, whose
/// chase mode (`MUSIC_CHASE`) also silences it, is not ported.
#[must_use]
pub fn audio_emitter_key(presentation_key: Option<u64>, challenge_type: i32) -> u64 {
    if RACES.contains(&challenge_type) {
        return 0;
    }
    presentation_key.unwrap_or(0)
}

/// The end of `AudioSystem::Update`: new emitter volumes are allowed outside races (and the
/// unported chase music).
#[must_use]
pub fn allow_new_emitter_volumes(challenge_type: i32) -> bool {
    !RACES.contains(&challenge_type)
}

/// `ConvertDistanceToVolume` 0x82a81298 by `eVolumeFalloffType`.
fn convert_distance_to_volume(distance: f32, kind: i32) -> f32 {
    match kind {
        0 => {
            let near = 1.0 - distance;
            near * near
        }
        1 => 1.0 - distance,
        _ => 1.0,
    }
}

/// The collection key of a `RefSpec` (bytes 8..16).
fn ref_spec_key(bytes: &[u8]) -> u64 {
    u64::from_be_bytes(bytes[8..16].try_into().expect("eight bytes"))
}

/// `aud_emitter::crowd_aems_id` 0x82550250: the crowd group a crowd volume plays.
const CROWD_AEMS_ID: u64 = 0x0B59_50C0_1840_14C2;

/// `aud_emitter::crowd_aems_id` of an emitter collection (0 from `DefaultDataArea`).
#[must_use]
pub fn crowd_aems_id(key: u64, vault: &Vault) -> i32 {
    vault.i32(Collection::new(AUD_EMITTER, key), CROWD_AEMS_ID)
}

/// `aud_emitter::reverbType` 0x824e7f10: the `aud_reverb` collection of an emitter collection.
#[must_use]
pub fn reverb_type(key: u64, vault: &Vault) -> u64 {
    ref_spec_key(&vault.bytes(Collection::new(AUD_EMITTER, key), REVERB_TYPE, 0, 24))
}

fn yzxw(v: Vec3) -> Vec3 {
    [v[1], v[2], v[0], v[3]]
}

fn mul(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2], a[3] * b[3]]
}

/// `vnmsubfp`: `b - a * c`, one rounding per lane.
fn nmsub(a: Vec3, c: Vec3, b: Vec3) -> Vec3 {
    std::array::from_fn(|lane| (-a[lane]).mul_add(c[lane], b[lane]))
}

/// The distance `CheckVolume` 0x82a82db0 computes for a camera in a volume, normalised so the
/// surface is 1: `None` outside. A sphere (equal sizes) divides the distance by its radius; any
/// other volume is an ellipsoid along `orientation`, its right axis `cross(up, orientation)` and
/// their cross product, the axes normalised without a zero guard.
#[must_use]
pub fn volume_distance(volume: &EmitterVolume, camera: Vec3) -> Option<f32> {
    let [s0, s1, s2] = volume.size;
    let pos = [volume.pos[0], volume.pos[1], volume.pos[2], 0.0];
    let d = vmx::sub(camera, pos);
    if s0 == s1 && s1 == s2 {
        let distance = vmx::length(d);
        if !(distance < volume.size[0]) {
            return None;
        }
        return Some(distance / volume.size[0]);
    }
    let o = [volume.orientation[0], volume.orientation[1], volume.orientation[2], 0.0];
    let up = [0.0, 1.0, 0.0, 0.0];
    let along = vmx::dot3(o, d);
    let right = yzxw(nmsub(yzxw(up), o, mul(up, yzxw(o))));
    let right = vmx::scale(right, vmx::rsqrt(vmx::dot3(right, right)));
    let across = vmx::dot3(right, d);
    let third = yzxw(nmsub(yzxw(o), right, mul(o, yzxw(right))));
    let third = vmx::scale(third, vmx::rsqrt(vmx::dot3(third, third)));
    let vertical = vmx::dot3(third, d);
    let term = |value: f32, size: f32| if size > 0.0 { (value * value) / (size * size) } else { 0.0 };
    let sum = (term(across, s2) + term(vertical, s1)) + term(along, s0);
    let distance = vmx::length_from_squared(sum);
    if !(distance < 1.0) {
        return None;
    }
    Some(distance)
}

/// The end of `CheckVolume`: inside the inner radius the distance is 0; between it and the
/// surface it rises linearly from 0 to 1.
#[must_use]
pub fn apply_inner_radius(distance: f32, inner_radius: f32) -> f32 {
    if !(inner_radius > 0.0) {
        return distance;
    }
    if distance < inner_radius {
        return 0.0;
    }
    if inner_radius < 1.0 {
        let scale = 1.0 / (1.0 - inner_radius);
        return scale.mul_add(distance, 1.0 - scale);
    }
    0.0
}

/// `Sk8::Audio::Sk8EmitterSystem` with the `AudioAssetManager` swap banks it loads into.
pub struct Sk8EmitterSystem {
    assets: AudioAssetManager,
    swap_bank: usize,
    /// `mGameState`, `mStatus`, `mEmitterKey`.
    game_state: u32,
    status: u32,
    emitter_key: u64,
    /// `mEmitterStatusList`, `maxRandomTriggers`, `numRamdomTriggersLoaded`, `mTimeSinceLast`,
    /// `mNextRandomTime`.
    emitter_statuses: Vec<EmitterStatus>,
    max_random_triggers: i32,
    random_triggers_loaded: i32,
    time_since_last: f32,
    next_random_time: f32,
    /// `mEmitterVolumeTree`, `mbInGamePlay`, `mpDataFile`/`mpFileNames`, `mNumFiles`.
    tree: VolumeTree,
    in_game_play: bool,
    data_files: [Option<(String, Vec<EmitterVolume>)>; 10],
    num_files: i32,
    /// `mEmitterVolumeList`, `mAttribKeyMusicVolume`, `mbAllowNewEmitterVolumes`.
    volume_statuses: Vec<VolumeStatus>,
    attrib_key_music_volume: u64,
    allow_new_emitter_volumes: bool,
    /// `pEmitterUtility`, `curMusicVolume`, `mLastMusicVolume`.
    emitter_utility: Option<ClassObject>,
    cur_music_volume: MusicVolumeInfo,
    last_music_volume: u32,
    /// `mDLCSwapBank`, `mDLCAsyncStatus`, `mDLCFileName`, `mDLCFileIndex`.
    dlc_swap_bank: usize,
    dlc_async_status: AssetStatus,
    dlc_file_name: String,
    dlc_file_index: i32,
    next_status_id: u32,
    freed: VecDeque<VolumeStatus>,
    /// PC port: the camera is in a linked Skate 3 district, which stands where the city's
    /// volumes were placed. TU3 loads only the district's own emitter files there
    /// (`reverb_downtown.ems` and the like, not ported), so none of the city's volumes holds the
    /// camera: its reverb is the district's painted one.
    skate3_district: bool,
}

impl Default for Sk8EmitterSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Sk8EmitterSystem {
    /// `EmitterSystem::EmitterSystem` 0x82a825d0 and the `Sk8EmitterSystem` fields
    /// `AudioSystem::AudioSystem` 0x8250dda0 sets: two random triggers, new volumes allowed, the
    /// emitter swap bank and the DLC swap bank.
    #[must_use]
    pub fn new() -> Self {
        let mut assets = AudioAssetManager::default();
        let swap_bank = assets.create_swap_bank(NUM_SWAP_SLOTS);
        let dlc_swap_bank = assets.create_swap_bank(1);
        Self {
            assets,
            swap_bank,
            game_state: 0,
            status: SWAP_DISABLED,
            emitter_key: 0,
            emitter_statuses: Vec::new(),
            max_random_triggers: 2,
            random_triggers_loaded: 0,
            time_since_last: 0.0,
            next_random_time: 0.0,
            tree: VolumeTree::default(),
            in_game_play: false,
            data_files: std::array::from_fn(|_| None),
            num_files: 0,
            volume_statuses: Vec::new(),
            attrib_key_music_volume: 0,
            allow_new_emitter_volumes: true,
            emitter_utility: None,
            cur_music_volume: MusicVolumeInfo::default(),
            last_music_volume: 0,
            dlc_swap_bank,
            dlc_async_status: AssetStatus::Empty,
            dlc_file_name: String::new(),
            dlc_file_index: -1,
            next_status_id: 1,
            freed: VecDeque::new(),
            skate3_district: false,
        }
    }

    /// PC port: whether the camera is in a linked Skate 3 district (`skate3_district`).
    pub fn set_skate3_district(&mut self, district: bool) {
        self.skate3_district = district;
    }

    /// The emitter steps of `AudioSystem::Initialize` 0x82509c18 after `emitter_utility.abk`
    /// loads: `Sk8EmitterSystem::EnterGamePlay` 0x824e9d80 (a `C_emitter_utility`, then
    /// `EmitterSystem::EnterGamePlay` 0x82a81498), `LoadGameFiles` 0x824e9e00 (`LoadDataFile`
    /// 0x82a811a0 each, `ResolveFiles` 0x82a81ae0) and `EnableEmitterSwap(true)`.
    pub fn initialize(&mut self, aems: &mut Aems, files: EmitterFiles) {
        self.emitter_utility = Some(aems.create_object("c_emitter_utility", Vec::new()));
        self.in_game_play = true;
        self.assets.add_files(files.banks.into_iter().map(|(name, bytes)| (emitter_bank_path(&name), bytes)));
        for (name, bytes) in files.data_files {
            let Ok(index) = usize::try_from(self.num_files) else { break };
            if index >= self.data_files.len() {
                break;
            }
            self.data_files[index] = Some((name, parse_data_file(&bytes)));
            self.num_files += 1;
        }
        for file in 0..self.data_files.len().min(self.num_files.max(0) as usize) {
            self.add_file_volumes(file);
        }
        self.status = SWAP_ENABLED;
    }

    /// `AddEmitterVolume` 0x82a81828 for each volume of a file: a cube of the largest size.
    fn add_file_volumes(&mut self, file: usize) {
        let Some((_, volumes)) = &self.data_files[file] else { return };
        for (index, volume) in volumes.iter().enumerate() {
            let mut extent = volume.size[0];
            if volume.size[1] > extent {
                extent = volume.size[1];
            }
            if volume.size[2] > extent {
                extent = volume.size[2];
            }
            let min = volume.pos.map(|p| p - extent);
            let max = volume.pos.map(|p| p + extent);
            self.tree.insert(VolumeRef { file, index }, min, max);
        }
    }

    fn volume(&self, volume: VolumeRef) -> Option<&EmitterVolume> {
        self.data_files.get(volume.file)?.as_ref()?.1.get(volume.index)
    }

    /// `AudioAssetManager::Update` 0x82a7a900, at the start of `AudioSystem::Update`.
    pub fn update_assets(&mut self, aems: &mut Aems, core: &mut AudioCore) {
        self.assets.update(aems, core);
    }

    /// The `AudioAssetManager` singleton, whose fixed asset banks also take the traffic and crowd
    /// banks.
    pub fn assets_mut(&mut self) -> &mut AudioAssetManager {
        &mut self.assets
    }

    #[must_use]
    pub fn assets(&self) -> &AudioAssetManager {
        &self.assets
    }

    /// `GetVolumeType` 0x824f8248.
    fn volume_type(key: u64, vault: &Vault) -> u32 {
        vault.i32(Collection::new(AUD_EMITTER, key), VOLUME_TYPE) as u32
    }

    /// `GetEmitterFilename` 0x824f8450: the bank in `emitters.big`, lower case.
    fn emitter_filename(key: u64, vault: &Vault) -> String {
        emitter_bank_path(&vault.layout_text(Collection::new(AUD_EMITTER, key), 0).unwrap_or_default())
    }

    /// `VerifyReverbKey` 0x824f8ec0: the `reverbType` collection is one of `verbHash`.
    fn verify_reverb_key(key: u64, vault: &Vault) -> bool {
        VERB_HASH.contains(&reverb_type(key, vault))
    }

    /// `ChooseTimeForNextRandomEmitter` 0x824f8538: `Random() % 1000` thousandths of the way
    /// from `min_time` to `max_time`.
    fn choose_time(key: u64, vault: &Vault, rng: &mut RwRandom) -> f32 {
        let fraction = (rng.next_u32() % 1000) as f32 * f32::from_bits(0x3A83_126F);
        let collection = Collection::new(AUD_WP_EMITTERS, key);
        let (min, max) = (vault.layout_f32(collection, 1), vault.layout_f32(collection, 3));
        (max - min).mul_add(fraction, min)
    }

    /// `Sk8EmitterSystem::GetEmitterKey` 0x824f81d0: `AudioSystem::GetEmitterKey` while the swap
    /// is not disabled (`IsSwapEnabled` 0x82a812d8).
    fn sk8_emitter_key(&self, audio_key: u64) -> u64 {
        if self.status == SWAP_DISABLED {
            return 0;
        }
        audio_key
    }

    /// `EmitterSystem::Update` 0x82a83598 at the camera position (`MicrophoneMan` microphone 0);
    /// `audio_key` is `AudioSystem::GetEmitterKey`.
    pub fn update(&mut self, dt: f32, camera: Vec3, audio_key: u64, states: &mut EmitterManager, io: &mut EmitterIo<'_>) {
        self.update_unload_random_emitter();
        if self.in_game_play {
            if self.status == SWAP_ENABLED {
                let key = self.sk8_emitter_key(audio_key);
                if key != self.emitter_key {
                    self.change_emitters(key, io);
                }
            }
            self.update_loaded_and_playing_emitters(states, io);
            if self.random_triggers_loaded < self.max_random_triggers {
                if let Some(index) = self.find_random_emitter(io.rng) {
                    self.load_random_emitter(index, io);
                }
            }
            self.try_to_play_random_trigger(dt, states, io);
        }
        self.update_volume_unloads();
        if self.in_game_play {
            for status in &mut self.volume_statuses {
                status.touched = false;
            }
            if !self.skate3_district {
                self.search_oct_tree(camera, io);
            }
            self.remove_untouched(states, io);
            self.update_volume_list(states, io);
            self.update_music_volumes(io.vault);
        }
        self.update_emitter_swap_status(io);
    }

    /// The end of `AudioSystem::Update` sets `mbAllowNewEmitterVolumes`.
    pub fn set_allow_new_emitter_volumes(&mut self, allow: bool) {
        self.allow_new_emitter_volumes = allow;
    }

    /// `EnableEmitterSwap` 0x82a82bd8: disabling detaches the random emitters, drops them
    /// (`ChangeEmitters(0)`) and starts unloading the swap slots.
    pub fn enable_emitter_swap(&mut self, enable: bool, states: &mut EmitterManager, io: &mut EmitterIo<'_>) {
        if enable {
            self.status = SWAP_ENABLED;
            return;
        }
        if self.status != SWAP_ENABLED {
            return;
        }
        for status in &mut self.emitter_statuses {
            if let Some(state) = status.state.take() {
                states.detach(state, io.aems, io.map);
            }
        }
        self.change_emitters(0, io);
        self.status = SWAP_UNLOADING;
    }

    /// `IsSwapEnabled` 0x82a812d8: the swap is not disabled.
    #[must_use]
    pub fn is_swap_enabled(&self) -> bool {
        self.status != SWAP_DISABLED
    }

    /// `UpdateEmitterSwapStatus` 0x82a812f0: once every slot is unloaded their banks leave AEMS,
    /// then the swap is disabled.
    fn update_emitter_swap_status(&mut self, io: &mut EmitterIo<'_>) {
        if self.status == SWAP_UNLOADING {
            if !self.assets.all_swap_slots_unloaded(self.swap_bank) {
                return;
            }
            self.assets.unresolve_all_swap_slots(self.swap_bank, io.aems, io.core);
            self.status = SWAP_UNRESOLVING;
        } else if self.status == SWAP_UNRESOLVING && self.assets.are_all_swap_slots_unresolved(self.swap_bank, io.aems) {
            self.status = SWAP_DISABLED;
        }
    }

    /// `ChangeEmitters` 0x82a82708: the old random emitters are dropped (unloading the loaded
    /// ones that are not playing), then the new key's `aud_wp_emitters` list is read.
    fn change_emitters(&mut self, key: u64, io: &mut EmitterIo<'_>) {
        self.emitter_key = key;
        let mut index = 0;
        while index < self.emitter_statuses.len() {
            let status = &mut self.emitter_statuses[index];
            if status.load_status == NOT_LOADED {
                self.emitter_statuses.remove(index);
                continue;
            }
            status.remove = true;
            if status.state.is_none() {
                if let Ok(slot) = usize::try_from(status.slot) {
                    self.assets.unload_from_swap_slot(self.swap_bank, slot, true, io.aems, io.core);
                }
                status.load_status = UNLOADING;
            }
            index += 1;
        }
        if key != 0 {
            let collection = Collection::new(AUD_WP_EMITTERS, key);
            let count = io.vault.length(collection, WP_EMITTERS) as i32;
            for index in 0..count.max(0) as u32 {
                let emitter = ref_spec_key(&io.vault.bytes(collection, WP_EMITTERS, index, 24));
                // `GetEmitterWeight` 0x824f83b0: 10 past the weights.
                let weight = if (index as i32) < io.vault.length(collection, WP_WEIGHTS) as i32 { io.vault.i32_at(collection, WP_WEIGHTS, index) } else { 10 };
                self.emitter_statuses.push(EmitterStatus { key: emitter, weight, load_status: NOT_LOADED, tick: 0, slot: -1, remove: false, state: None });
            }
        }
        self.time_since_last = 0.0;
        self.next_random_time = Self::choose_time(self.emitter_key, io.vault, io.rng);
    }

    /// `UpdateUnloadRandomEmitter` 0x82a81f10: an unloading emitter whose slot emptied is not
    /// loaded again (and leaves when marked).
    fn update_unload_random_emitter(&mut self) {
        let mut index = 0;
        while index < self.emitter_statuses.len() {
            let status = &mut self.emitter_statuses[index];
            if status.load_status != UNLOADING {
                index += 1;
                continue;
            }
            let empty = usize::try_from(status.slot).map_or(true, |slot| self.assets.async_to_swap_slot_status(self.swap_bank, slot) == AssetStatus::Empty);
            if empty {
                status.slot = -1;
                status.load_status = NOT_LOADED;
                self.random_triggers_loaded -= 1;
            }
            if status.remove && empty {
                self.emitter_statuses.remove(index);
            } else {
                index += 1;
            }
        }
    }

    /// `UpdateLoadedAndPlayingEmitters` 0x82a81650: a random emitter whose state detached
    /// unloads (resolved data included); a loaded idle one ages.
    fn update_loaded_and_playing_emitters(&mut self, states: &EmitterManager, io: &mut EmitterIo<'_>) {
        for status in &mut self.emitter_statuses {
            if let Some(state) = status.state {
                if !states.is_attached(state) {
                    status.state = None;
                    if let Ok(slot) = usize::try_from(status.slot) {
                        self.assets.unload_from_swap_slot(self.swap_bank, slot, true, io.aems, io.core);
                    }
                    status.load_status = UNLOADING;
                }
            } else if status.load_status == LOADED {
                status.tick += 1;
            }
        }
    }

    /// `FindRandomEmitter` 0x82a816e0: a weighted draw among the emitters not loaded.
    fn find_random_emitter(&self, rng: &mut RwRandom) -> Option<usize> {
        let total = self.emitter_statuses.iter().filter(|status| status.load_status == NOT_LOADED).fold(0u32, |sum, status| sum.wrapping_add(status.weight as u32));
        if total == 0 {
            return None;
        }
        let target = (rng.next_u32() % total) as i32;
        let mut sum = 0i32;
        for (index, status) in self.emitter_statuses.iter().enumerate() {
            if status.load_status == NOT_LOADED {
                sum = sum.wrapping_add(status.weight);
            }
            if sum > target {
                return Some(index);
            }
        }
        None
    }

    /// `LoadRandomEmitter` 0x82a81100: the first free slot.
    fn load_random_emitter(&mut self, index: usize, io: &mut EmitterIo<'_>) {
        let slot = self.assets.find_free_slot(self.swap_bank, false, None);
        let Ok(slot_index) = usize::try_from(slot) else { return };
        let name = Self::emitter_filename(self.emitter_statuses[index].key, io.vault);
        self.assets.async_load_to_swap_slot(&name, self.swap_bank, slot_index, ASSET_AEMS_BANK, io.aems, io.core);
        let status = &mut self.emitter_statuses[index];
        status.slot = slot;
        status.load_status = LOADED;
        status.tick = 0;
        self.random_triggers_loaded += 1;
    }

    /// `FindLoadedEmitterToPlay` 0x82a817a0: the idle loaded emitter loaded longest.
    fn find_loaded_emitter_to_play(&self) -> Option<usize> {
        let mut best = -1;
        let mut found = None;
        for (index, status) in self.emitter_statuses.iter().enumerate() {
            let loaded = status.load_status == LOADED && usize::try_from(status.slot).is_ok_and(|slot| self.assets.async_to_swap_slot_status(self.swap_bank, slot) == AssetStatus::Loaded);
            if loaded && status.state.is_none() && status.tick > best {
                best = status.tick;
                found = Some(index);
            }
        }
        found
    }

    /// `TryToPlayRandomTrigger` 0x82a81e70.
    fn try_to_play_random_trigger(&mut self, dt: f32, states: &mut EmitterManager, io: &mut EmitterIo<'_>) {
        if self.emitter_key == 0 {
            return;
        }
        let time = dt + self.time_since_last;
        self.time_since_last = time;
        if !(time > self.next_random_time) {
            return;
        }
        if let Some(index) = self.find_loaded_emitter_to_play() {
            self.play_emitter(index, self.emitter_key, states, io);
        }
        self.time_since_last = 0.0;
        self.next_random_time = Self::choose_time(self.emitter_key, io.vault, io.rng);
    }

    /// `PlayEmitter` 0x824f85d8: a random emitter at a random azimuth, its volume drawn in
    /// steps of 1/scale between the location's `min_volume` and `max_volume`, expiring after
    /// the emitter's `duration`.
    fn play_emitter(&mut self, index: usize, key: u64, states: &mut EmitterManager, io: &mut EmitterIo<'_>) {
        let emitter = Collection::new(AUD_EMITTER, self.emitter_statuses[index].key);
        let location = Collection::new(AUD_WP_EMITTERS, key);
        let vault = io.vault;
        let scale = vault.i32(location, WP_VOLUME_SCALE) as f32;
        let high = fctiwz(vault.layout_f32(location, 2) * scale);
        let low = fctiwz(scale * vault.layout_f32(location, 0));
        let mut level = high;
        if low != high {
            let range = high.wrapping_sub(low) as u32;
            level = (io.rng.next_u32() % range).wrapping_add(low as u32) as i32;
        }
        let info = EmitterInfo {
            aems_id: vault.layout_word(emitter, 3) as i32,
            vol_state: VOL_STATE_RANDOM,
            vol: (level as f32 / scale) * vault.layout_f32(emitter, 1),
            azim_state: AZIM_STATE_EXTERNAL,
            azimuth: (io.rng.next_u32() & 0xFFFF) as i32,
            expire: true,
            timeout: vault.layout_f32(emitter, 2),
            position: vmx::ZERO,
        };
        if let Some(state) = states.free_state() {
            states.attach(state, info, io.aems, io.map);
            self.emitter_statuses[index].state = Some(state);
        }
    }

    /// `UpdateVolumeUnloads` 0x82a82188: an unloading volume leaves once its slot is empty.
    fn update_volume_unloads(&mut self) {
        let mut index = 0;
        while index < self.volume_statuses.len() {
            let status = self.volume_statuses[index];
            let empty = status.load_status == UNLOADING && usize::try_from(status.slot).map_or(true, |slot| self.assets.async_to_swap_slot_status(self.swap_bank, slot) == AssetStatus::Empty);
            if empty {
                self.free_status(index);
            } else {
                index += 1;
            }
        }
    }

    /// Unlinks and frees a status, keeping its last values for stale reverb pointers.
    fn free_status(&mut self, index: usize) {
        let status = self.volume_statuses.remove(index);
        if self.freed.len() == FREED_STATUSES {
            self.freed.pop_front();
        }
        self.freed.push_back(status);
    }

    /// `SearchOctTree` 0x82a83448: `CheckVolume` for each volume whose box meets the camera's
    /// one-unit box, in entry order.
    fn search_oct_tree(&mut self, camera: Vec3, io: &mut EmitterIo<'_>) {
        let min = [camera[0] - 0.5, camera[1] - 0.5, camera[2] - 0.5];
        let max = [camera[0] + 0.5, camera[1] + 0.5, camera[2] + 0.5];
        for volume in self.tree.collisions(min, max) {
            self.check_volume(volume, camera, io);
        }
    }

    /// `CheckVolume` 0x82a82db0: a volume of an active game state that holds the camera.
    fn check_volume(&mut self, volume_ref: VolumeRef, camera: Vec3, io: &mut EmitterIo<'_>) {
        let Some(volume) = self.volume(volume_ref).copied() else { return };
        if volume.states != 0 && self.game_state & volume.states == 0 {
            return;
        }
        let Some(distance) = volume_distance(&volume, camera) else { return };
        let inner = apply_inner_radius(distance, volume.inner_radius);
        self.update_or_add_to_volume_list(volume_ref, &volume, inner, distance, io);
    }

    /// `GetKeyFromVolume` 0x824f88e8: one of the volume's alternative collections at random
    /// (draw 0, or a null entry, keeps the volume's own).
    fn key_from_volume(volume: &EmitterVolume, io: &mut EmitterIo<'_>) -> u64 {
        let collection = Collection::new(AUD_EMITTER, volume.key);
        let count = io.vault.length(collection, ALTERNATIVE_KEYS) as i32;
        if count <= 0 {
            return volume.key;
        }
        let pick = io.rng.next_u32() % (count as u32).wrapping_add(1);
        if pick == 0 {
            return volume.key;
        }
        match ref_spec_key(&io.vault.bytes(collection, ALTERNATIVE_KEYS, pick - 1, 24)) {
            0 => volume.key,
            key => key,
        }
    }

    /// `UpdateOrAddToVolumeList` 0x82a82910: a known volume is touched (an AEMS one only while
    /// the swap is enabled); a new AEMS volume shares a status's slot of the same key or loads
    /// its bank into a free slot. When no status is created, the touch lands on the last status
    /// the search read, as natively.
    fn update_or_add_to_volume_list(&mut self, volume_ref: VolumeRef, volume: &EmitterVolume, distance: f32, distance_no_inner: f32, io: &mut EmitterIo<'_>) {
        if volume.key == 0 {
            return;
        }
        let mut target = None;
        let mut found = false;
        for (index, status) in self.volume_statuses.iter().enumerate() {
            target = Some(index);
            if status.volume == volume_ref {
                found = true;
                break;
            }
        }
        if found {
            let status = self.volume_statuses[target.expect("found")];
            if status.load_status == UNLOADING {
                return;
            }
            if Self::volume_type(status.key, io.vault) == VOLUME_AEMS && self.status != SWAP_ENABLED {
                return;
            }
        } else {
            let key = Self::key_from_volume(volume, io);
            // The slot of a status to create: a non-AEMS volume has none.
            let mut create = Some(-1);
            if Self::volume_type(key, io.vault) == VOLUME_AEMS {
                if self.status != SWAP_ENABLED {
                    return;
                }
                let mut slot = -1;
                if let Some(status) = self.volume_statuses.iter().find(|status| status.key == key) {
                    slot = status.slot;
                } else if io.vault.has_collection(Collection::new(AUD_EMITTER, key)) && self.allow_new_emitter_volumes {
                    let name = Self::emitter_filename(key, io.vault);
                    slot = self.assets.find_free_slot(self.swap_bank, true, Some(&name));
                    if let Ok(free) = usize::try_from(slot) {
                        self.assets.async_load_to_swap_slot(&name, self.swap_bank, free, ASSET_AEMS_BANK, io.aems, io.core);
                    }
                }
                create = (slot != -1).then_some(slot);
            }
            if let Some(slot) = create {
                let id = self.next_status_id;
                self.next_status_id = self.next_status_id.wrapping_add(1);
                self.volume_statuses.push(VolumeStatus {
                    id,
                    volume: volume_ref,
                    load_status: LOADED,
                    slot,
                    distance: 0.0,
                    touched: false,
                    state: None,
                    key,
                    distance_no_inner: 0.0,
                });
                target = Some(self.volume_statuses.len() - 1);
            }
        }
        if let Some(index) = target {
            let status = &mut self.volume_statuses[index];
            status.touched = true;
            status.distance = distance;
            status.distance_no_inner = distance_no_inner;
        }
    }

    /// `RemoveUntouched` 0x82a81fd8: a volume the camera left detaches; a non-AEMS one leaves
    /// at once, an AEMS one leaves when another status shares its slot, else unloads its slot
    /// (the bank stays added).
    fn remove_untouched(&mut self, states: &mut EmitterManager, io: &mut EmitterIo<'_>) {
        let mut index = 0;
        while index < self.volume_statuses.len() {
            if self.volume_statuses[index].touched {
                index += 1;
                continue;
            }
            let key = self.volume_statuses[index].key;
            if Self::volume_type(key, io.vault) != VOLUME_AEMS {
                self.free_status(index);
                continue;
            }
            if let Some(state) = self.volume_statuses[index].state.take() {
                states.detach(state, io.aems, io.map);
            }
            let slot = self.volume_statuses[index].slot;
            let sharing = self.volume_statuses.iter().filter(|status| status.key == key && status.slot == slot).count();
            if sharing > 1 {
                self.free_status(index);
                continue;
            }
            if self.volume_statuses[index].load_status != UNLOADING {
                if let Ok(slot) = usize::try_from(slot) {
                    self.assets.unload_from_swap_slot(self.swap_bank, slot, false, io.aems, io.core);
                }
                self.volume_statuses[index].load_status = UNLOADING;
            }
            index += 1;
        }
    }

    /// `UpdateVolumeList` 0x82a818f0: each loaded AEMS volume plays (`PlayVolume` 0x824f89d0)
    /// or follows its distance (`UpdateVolume` 0x824f8ba8).
    fn update_volume_list(&mut self, states: &mut EmitterManager, io: &mut EmitterIo<'_>) {
        for index in 0..self.volume_statuses.len() {
            let status = self.volume_statuses[index];
            if Self::volume_type(status.key, io.vault) != VOLUME_AEMS || status.load_status != LOADED {
                continue;
            }
            let Ok(slot) = usize::try_from(status.slot) else { continue };
            if self.assets.async_to_swap_slot_status(self.swap_bank, slot) != AssetStatus::Loaded {
                continue;
            }
            let collection = Collection::new(AUD_EMITTER, status.key);
            let vol = io.vault.layout_f32(collection, 1) * convert_distance_to_volume(status.distance, io.vault.i32(collection, FALLOFF_TYPE));
            if let Some(state) = status.state {
                if let Some(info) = states.info_mut(state) {
                    info.vol = vol;
                }
                continue;
            }
            let Some(volume) = self.volume(status.volume) else { continue };
            let info = EmitterInfo {
                aems_id: io.vault.layout_word(collection, 3) as i32,
                vol_state: VOL_STATE_VOLUME,
                vol,
                azim_state: AZIM_STATE_NICOTINE,
                azimuth: 0,
                expire: false,
                timeout: 10.0,
                position: [volume.pos[0], volume.pos[1], volume.pos[2], 0.0],
            };
            if let Some(state) = states.free_state() {
                states.attach(state, info, io.aems, io.map);
                self.volume_statuses[index].state = Some(state);
            }
        }
    }

    /// `UpdateMusicVolumes` 0x82a81bd8 and `UpdateCurrentMusicVolume` 0x824f8cf0: the first music
    /// volume's attributes (read again only when it changes) and distances.
    fn update_music_volumes(&mut self, vault: &Vault) {
        self.attrib_key_music_volume = 0;
        if let Some(status) = self.volume_statuses.iter().find(|status| Self::volume_type(status.key, vault) == VOLUME_MUSIC).copied() {
            if let Some(volume) = self.volume(status.volume).copied() {
                if volume.identifier != self.last_music_volume {
                    let collection = Collection::new(AUD_EMITTER, status.key);
                    let music = &mut self.cur_music_volume;
                    music.identifier = volume.identifier;
                    music.playlist = ref_spec_key(&vault.bytes(collection, MUSIC_PLAYLIST, 0, 24));
                    music.pan_size = vault.f32(collection, MUSIC_PAN_SIZE);
                    music.volume = vault.layout_f32(collection, 1);
                    music.mid_radius = vault.f32(collection, MUSIC_MID_RADIUS);
                    music.inner_radius = volume.inner_radius;
                    music.pos = volume.pos;
                    self.last_music_volume = volume.identifier;
                }
            }
            self.cur_music_volume.distance = status.distance;
            self.cur_music_volume.distance_no_inner = status.distance_no_inner;
            self.attrib_key_music_volume = status.key;
        }
        if self.attrib_key_music_volume == 0 {
            self.last_music_volume = 0;
        }
    }

    /// `GetCurrentMusicVolume` 0x824e9e90: the music volume while one is current.
    #[must_use]
    pub fn current_music_volume(&self) -> Option<&MusicVolumeInfo> {
        (self.attrib_key_music_volume != 0).then_some(&self.cur_music_volume)
    }

    /// `mAttribKeyMusicVolume`: the current music volume's `aud_emitter` collection, 0 for none.
    #[must_use]
    pub fn music_volume_key(&self) -> u64 {
        self.attrib_key_music_volume
    }

    /// `AudioSystem::GetCurrentReverbVolume` 0x824e69c8 (`EmitterSystem::GetCurrentReverbVolume`
    /// 0x82a81c80): the first reverb volume other than `exclude`, if its reverb is known.
    #[must_use]
    pub fn current_reverb_volume(&self, exclude: Option<VolumeStatusId>, vault: &Vault) -> Option<VolumeStatusId> {
        let status = self.volume_statuses.iter().find(|status| Self::volume_type(status.key, vault) == VOLUME_REVERB && Some(VolumeStatusId(status.id)) != exclude)?;
        Self::verify_reverb_key(status.key, vault).then_some(VolumeStatusId(status.id))
    }

    /// The status behind a pointer: a live one, or the last values of one freed since.
    #[must_use]
    pub fn volume_status(&self, id: VolumeStatusId) -> Option<VolumeStatusView> {
        let status = self.volume_statuses.iter().chain(self.freed.iter().rev()).find(|status| status.id == id.0)?;
        let volume = self.volume(status.volume);
        let pos = volume.map_or([0.0; 3], |volume| volume.pos);
        Some(VolumeStatusView { key: status.key, distance: status.distance, pos, volume: volume.map(|volume| (volume.identifier, volume.key)) })
    }

    /// `GetVolumes` 0x82a81b58: up to `count` statuses of `kind`, in list order.
    #[must_use]
    pub fn volumes(&self, kind: u32, count: usize, vault: &Vault) -> Vec<VolumeStatusId> {
        self.volume_statuses.iter().filter(|status| Self::volume_type(status.key, vault) == kind).take(count).map(|status| VolumeStatusId(status.id)).collect()
    }

    /// `AddGameState` 0x82a81378 / `RemoveGameState` 0x82a81388.
    pub fn set_game_state(&mut self, state: u32, active: bool) {
        if active {
            self.game_state |= state;
        } else {
            self.game_state &= !state;
        }
    }

    /// `AsyncLoadDLCData` 0x824f8f68: the location's emitter data file into the DLC slot.
    pub fn async_load_dlc_data(&mut self, key: u64, io: &mut EmitterIo<'_>) {
        let collection = Collection::new(FE_LOCATIONS, key);
        if !io.vault.bool(collection, DLC_HAS_EMITTERS) {
            return;
        }
        self.dlc_file_name = io.vault.text(collection, DLC_EMITTER_FILE).unwrap_or_default().replace('/', "\\");
        let name = self.dlc_file_name.clone();
        self.assets.async_load_to_swap_slot(&name, self.dlc_swap_bank, 0, ASSET_DATA, io.aems, io.core);
        self.dlc_async_status = AssetStatus::Loading;
    }

    /// `UpdateDLCAsyncLoad` 0x824f9098: the loaded file's volumes join (`AddDataFile` 0x82a81998).
    pub fn update_dlc_async_load(&mut self) {
        if self.dlc_async_status != AssetStatus::Loading || self.assets.async_to_swap_slot_status(self.dlc_swap_bank, 0) != AssetStatus::Loaded {
            return;
        }
        self.dlc_async_status = AssetStatus::Loaded;
        let data = self.assets.swap_data(self.dlc_swap_bank, 0);
        self.dlc_file_index = -1;
        let Ok(index) = usize::try_from(self.num_files) else { return };
        if index >= self.data_files.len() {
            return;
        }
        let volumes = data.map(|data| parse_data_file(&data)).unwrap_or_default();
        self.data_files[index] = Some((self.dlc_file_name.clone(), volumes));
        self.add_file_volumes(index);
        self.dlc_file_index = self.num_files;
        self.num_files += 1;
    }

    /// `UnloadDLCData` 0x824f9148: `RemoveDataFile` 0x82a836d8, then the DLC slot unloads.
    pub fn unload_dlc_data(&mut self, io: &mut EmitterIo<'_>) {
        if self.dlc_async_status != AssetStatus::Loaded {
            return;
        }
        if let Ok(index) = usize::try_from(self.dlc_file_index) {
            if let Some(Some((_, volumes))) = self.data_files.get(index) {
                for volume in 0..volumes.len() {
                    self.tree.remove(VolumeRef { file: index, index: volume });
                }
                self.data_files[index] = None;
                self.num_files -= 1;
            }
        }
        self.dlc_file_index = -1;
        self.assets.unload_all_swap_slots(self.dlc_swap_bank, io.aems, io.core);
        self.dlc_async_status = AssetStatus::Empty;
    }
}

/// Shared by the emitter and reverb tests: a vault of mock collections and a world to update.
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::nicotine::MixerHost;
    use crate::vault::AttribSource;
    use std::collections::{HashMap, HashSet};

    pub(crate) struct NoHost;

    impl MixerHost for NoHost {
        fn state_count(&mut self, _state: usize) -> i32 {
            0
        }

        fn connect_dmixio(&mut self, _id: u32) -> bool {
            false
        }
    }

    /// Collections with attributes (element bytes), lengths, layout words and texts.
    #[derive(Default)]
    pub(crate) struct Mock {
        pub(crate) attributes: HashMap<(u64, u64, u64, u32), Vec<u8>>,
        pub(crate) lengths: HashMap<(u64, u64, u64), u32>,
        pub(crate) layouts: HashMap<(u64, u64), Vec<u32>>,
        pub(crate) texts: HashMap<(u64, u64), String>,
        pub(crate) collections: HashSet<(u64, u64)>,
    }

    impl Mock {
        pub(crate) fn emitter(&mut self, key: u64, kind: u32, file: &str, volume: f32, duration: f32, aems_id: i32) {
            self.collections.insert((AUD_EMITTER, key));
            self.attributes.insert((AUD_EMITTER, key, VOLUME_TYPE, 0), kind.to_be_bytes().to_vec());
            self.attributes.insert((AUD_EMITTER, key, FALLOFF_TYPE, 0), 1u32.to_be_bytes().to_vec());
            self.layouts.insert((AUD_EMITTER, key), vec![0, volume.to_bits(), duration.to_bits(), aems_id as u32]);
            self.texts.insert((AUD_EMITTER, key), file.to_owned());
        }

        /// A reverb volume's collection whose `reverbType` is `verbHash[verb]`.
        pub(crate) fn reverb(&mut self, key: u64, verb: usize) -> u64 {
            self.emitter(key, VOLUME_REVERB, "", 1.0, 0.0, 0);
            self.ref_spec(AUD_EMITTER, key, REVERB_TYPE, 0, VERB_HASH[verb]);
            VERB_HASH[verb]
        }

        pub(crate) fn ref_spec(&mut self, class: u64, key: u64, attribute: u64, index: u32, target: u64) {
            let mut bytes = vec![0; 24];
            bytes[8..16].copy_from_slice(&target.to_be_bytes());
            self.attributes.insert((class, key, attribute, index), bytes);
        }
    }

    impl AttribSource for Mock {
        fn attribute(&self, class: u64, collection: u64, key: u64, index: u32) -> Option<Vec<u8>> {
            self.attributes.get(&(class, collection, key, index)).cloned()
        }

        fn layout_word(&self, class: u64, collection: u64, index: u32) -> u32 {
            self.layouts.get(&(class, collection)).and_then(|words| words.get(index as usize)).copied().unwrap_or(0)
        }

        fn has_collection(&self, class: u64, collection: u64) -> bool {
            self.collections.contains(&(class, collection))
        }

        fn layout_text(&self, class: u64, collection: u64, index: u32) -> Option<String> {
            (index == 0).then(|| self.texts.get(&(class, collection)).cloned()).flatten()
        }

        fn length(&self, class: u64, collection: u64, key: u64) -> u32 {
            self.lengths.get(&(class, collection, key)).copied().unwrap_or(0)
        }
    }

    pub(crate) fn volume_bytes(volumes: &[EmitterVolume]) -> Vec<u8> {
        let mut bytes = (volumes.len() as i32).to_be_bytes().to_vec();
        for volume in volumes {
            let words = [
                volume.identifier,
                volume.states,
                volume.pos[0].to_bits(),
                volume.pos[1].to_bits(),
                volume.pos[2].to_bits(),
                volume.size[0].to_bits(),
                volume.size[1].to_bits(),
                volume.size[2].to_bits(),
                volume.inner_radius.to_bits(),
                volume.orientation[0].to_bits(),
                volume.orientation[1].to_bits(),
                volume.orientation[2].to_bits(),
                (volume.key >> 32) as u32,
                volume.key as u32,
                0,
                0,
                0,
                0,
            ];
            for word in words {
                bytes.extend_from_slice(&word.to_be_bytes());
            }
        }
        bytes
    }

    pub(crate) fn sphere(identifier: u32, pos: [f32; 3], radius: f32, key: u64) -> EmitterVolume {
        EmitterVolume { identifier, states: 0, pos, size: [radius; 3], inner_radius: 0.0, orientation: [1.0, 0.0, 0.0], key }
    }

    /// An `ABKC` bank without modules that `add_module_bank` accepts.
    pub(crate) fn empty_bank() -> Arc<[u8]> {
        let mut bytes = vec![0u8; 0x40];
        bytes[..4].copy_from_slice(b"ABKC");
        bytes[0x18..0x1C].copy_from_slice(&0x40u32.to_be_bytes());
        bytes[0x38..0x3C].copy_from_slice(&0x3Cu32.to_be_bytes());
        Arc::from(bytes)
    }

    pub(crate) struct World {
        pub(crate) system: Sk8EmitterSystem,
        pub(crate) states: EmitterManager,
        pub(crate) core: AudioCore,
        pub(crate) aems: Aems,
        pub(crate) map: MixMap,
        pub(crate) vault: Vault,
        pub(crate) rng: RwRandom,
    }

    impl World {
        pub(crate) fn new(mock: Mock, volumes: &[EmitterVolume], banks: &[&str]) -> Self {
            let vault = Vault::new(Some(Box::new(mock)));
            let mut aems = Aems::new(None);
            let mut system = Sk8EmitterSystem::new();
            let files = EmitterFiles {
                data_files: vec![("data\\audio\\sfx.ems".into(), volume_bytes(volumes))],
                banks: banks.iter().map(|name| ((*name).to_owned(), empty_bank())).collect(),
            };
            system.initialize(&mut aems, files);
            let map = MixMap::new(Vec::new(), 13, &mut NoHost);
            Self { system, states: EmitterManager::new(&vault, None), core: AudioCore::new(), aems, map, vault, rng: RwRandom::new(1) }
        }

        pub(crate) fn update(&mut self, camera: [f32; 3], key: u64) {
            self.system.update_assets(&mut self.aems, &mut self.core);
            let mut io = EmitterIo { core: &mut self.core, aems: &mut self.aems, map: &mut self.map, vault: &self.vault, rng: &mut self.rng };
            self.system.update(1.0 / 30.0, [camera[0], camera[1], camera[2], 1.0], key, &mut self.states, &mut io);
        }

        /// An audio frame's `RemoveModuleBankHandler` commands.
        pub(crate) fn audio_frame(&mut self) {
            self.aems.timer_update(&mut self.core, 256.0 / 48000.0);
        }

        pub(crate) fn bank_count(&self) -> usize {
            self.aems.banks.iter().flatten().count()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;

    #[test]
    fn data_file_round_trip() {
        let volume = EmitterVolume { identifier: 5, states: 0x4000, pos: [1.0, 2.0, 3.0], size: [4.0, 5.0, 6.0], inner_radius: 0.5, orientation: [0.0, 0.0, 1.0], key: 0x0123_4567_89AB_CDEF };
        assert_eq!(parse_data_file(&volume_bytes(&[volume, volume])), vec![volume, volume]);
        assert!(parse_data_file(&[0, 0, 0]).is_empty());
    }

    #[test]
    fn sphere_and_ellipsoid_distances() {
        let ball = sphere(1, [10.0, 0.0, 0.0], 8.0, 1);
        assert_eq!(volume_distance(&ball, [14.0, 0.0, 0.0, 1.0]), Some(0.5));
        assert_eq!(volume_distance(&ball, [18.0, 0.0, 0.0, 1.0]), None);
        assert_eq!(volume_distance(&ball, [f32::NAN, 0.0, 0.0, 1.0]), None);
        let ellipsoid = EmitterVolume { size: [10.0, 5.0, 2.0], ..ball };
        let at = |offset: [f32; 3]| volume_distance(&ellipsoid, [10.0 + offset[0], offset[1], offset[2], 1.0]);
        assert_eq!(at([5.0, 0.0, 0.0]), Some(0.5));
        assert_eq!(at([0.0, 2.5, 0.0]), Some(0.5));
        assert_eq!(at([0.0, 0.0, 1.0]), Some(0.5));
        assert_eq!(at([0.0, 0.0, 2.5]), None);
        // An upright orientation has no right axis: NaN, so never inside.
        let upright = EmitterVolume { orientation: [0.0, 1.0, 0.0], ..ellipsoid };
        assert_eq!(volume_distance(&upright, [10.0, 0.0, 0.0, 1.0]), None);
    }

    #[test]
    fn inner_radius_ramp() {
        assert_eq!(apply_inner_radius(0.3, 0.0), 0.3);
        assert_eq!(apply_inner_radius(0.3, 0.5), 0.0);
        assert_eq!(apply_inner_radius(0.75, 0.5), 0.5);
        assert_eq!(apply_inner_radius(0.75, 1.0), 0.0);
        assert_eq!(apply_inner_radius(0.75, f32::NAN), 0.75);
    }

    const KEY: u64 = 0x1111_0000_0000_0001;

    #[test]
    fn aems_volume_loads_plays_and_unloads() {
        let mut mock = Mock::default();
        mock.emitter(KEY, VOLUME_AEMS, "Test.abk", 0.5, 0.0, 700);
        let mut world = World::new(mock, &[sphere(1, [0.0; 3], 10.0, KEY)], &["test.abk"]);
        world.update([5.0, 0.0, 0.0], 0);
        assert_eq!(world.system.volume_statuses.len(), 1);
        assert_eq!(world.system.volume_statuses[0].slot, 4, "FindFreeSlot searches backwards");
        assert_eq!(world.system.volume_statuses[0].distance, 0.5);
        assert_eq!(world.states.free_state(), Some(0), "the bank is still loading");
        // The load starts on the next asset update and completes on the one after.
        world.update([5.0, 0.0, 0.0], 0);
        assert!(!world.states.is_attached(0));
        world.update([5.0, 0.0, 0.0], 0);
        assert!(world.states.is_attached(0));
        let info = *world.states.info_mut(0).expect("attached");
        assert_eq!((info.aems_id, info.vol_state, info.azim_state, info.expire), (700, VOL_STATE_VOLUME, AZIM_STATE_NICOTINE, false));
        assert_eq!(info.vol, 0.5 * 0.5);
        assert_eq!(world.bank_count(), 1);
        // Closer: the level follows the distance.
        world.update([2.0, 0.0, 0.0], 0);
        assert_eq!(world.states.info_mut(0).expect("attached").vol, 0.5 * 0.8);
        // Leaving detaches and empties the slot at once; the bank stays added.
        world.update([50.0, 0.0, 0.0], 0);
        assert!(!world.states.is_attached(0));
        assert_eq!(world.system.volume_statuses[0].load_status, UNLOADING);
        world.update([50.0, 0.0, 0.0], 0);
        assert!(world.system.volume_statuses.is_empty());
        assert_eq!(world.bank_count(), 1);
        // Coming back reuses the still-resolved slot without a load.
        world.update([5.0, 0.0, 0.0], 0);
        assert!(world.states.is_attached(0));
        assert_eq!(world.bank_count(), 1);
    }

    #[test]
    fn volumes_share_a_slot_and_a_failed_add_touches_the_last_status() {
        const OTHER: u64 = 0x2222_0000_0000_0002;
        let mut mock = Mock::default();
        mock.emitter(KEY, VOLUME_AEMS, "a.abk", 1.0, 0.0, 1);
        mock.emitter(OTHER, VOLUME_AEMS, "b.abk", 1.0, 0.0, 2);
        let volumes = [sphere(1, [0.0; 3], 10.0, KEY), sphere(2, [1.0, 0.0, 0.0], 10.0, KEY), sphere(3, [0.0, 0.0, 4.0], 8.0, OTHER)];
        let mut world = World::new(mock, &volumes, &["a.abk", "b.abk"]);
        world.system.set_allow_new_emitter_volumes(false);
        world.update([0.0, 0.0, 0.0], 0);
        // No new bank may load: nothing is created, and with no status nothing is touched.
        assert!(world.system.volume_statuses.is_empty());
        world.system.set_allow_new_emitter_volumes(true);
        world.update([0.0, 0.0, 0.0], 0);
        let slots: Vec<i32> = world.system.volume_statuses.iter().map(|status| status.slot).collect();
        assert_eq!(slots, vec![4, 4, 3], "the second volume of a key shares its slot");
        // Volume 1 no longer holds the camera: its status leaves without unloading the shared
        // slot; volume 3's unloads its queued bank.
        world.update([10.5, 0.0, 0.0], 0);
        let statuses: Vec<(usize, u32, i32)> = world.system.volume_statuses.iter().map(|status| (status.volume.index, status.load_status, status.slot)).collect();
        assert_eq!(statuses, vec![(1, LOADED, 4), (2, UNLOADING, 3)]);
        for _ in 0..8 {
            world.update([10.5, 0.0, 0.0], 0);
        }
        assert_eq!(world.system.volume_statuses.len(), 1, "the unloading status leaves once its slot empties");
        // Volume 1 shares the loaded slot again, but volume 3's key may not load: its distance
        // lands on the last status the search read, volume 1's.
        world.system.set_allow_new_emitter_volumes(false);
        world.update([2.0, 0.0, 0.0], 0);
        let expected = volume_distance(&volumes[2], [2.0, 0.0, 0.0, 1.0]).expect("inside");
        let statuses: Vec<(usize, f32)> = world.system.volume_statuses.iter().map(|status| (status.volume.index, status.distance)).collect();
        assert_eq!(statuses, vec![(1, 0.1), (0, expected)]);
    }

    #[test]
    fn random_emitters_load_play_and_unload() {
        const LOCATION: u64 = 0x3333_0000_0000_0003;
        const QUIET: u64 = 0x4444_0000_0000_0004;
        let mut mock = Mock::default();
        mock.emitter(KEY, VOLUME_AEMS, "dog.abk", 0.8, 2.0, 12);
        mock.emitter(QUIET, VOLUME_AEMS, "cat.abk", 0.8, 2.0, 13);
        mock.collections.insert((AUD_WP_EMITTERS, LOCATION));
        mock.lengths.insert((AUD_WP_EMITTERS, LOCATION, WP_EMITTERS), 2);
        mock.ref_spec(AUD_WP_EMITTERS, LOCATION, WP_EMITTERS, 0, QUIET);
        mock.ref_spec(AUD_WP_EMITTERS, LOCATION, WP_EMITTERS, 1, KEY);
        mock.lengths.insert((AUD_WP_EMITTERS, LOCATION, WP_WEIGHTS), 1);
        mock.attributes.insert((AUD_WP_EMITTERS, LOCATION, WP_WEIGHTS, 0), 0i32.to_be_bytes().to_vec());
        mock.attributes.insert((AUD_WP_EMITTERS, LOCATION, WP_VOLUME_SCALE, 0), 100i32.to_be_bytes().to_vec());
        mock.layouts.insert((AUD_WP_EMITTERS, LOCATION), vec![0.5f32.to_bits(), 0.0f32.to_bits(), 0.5f32.to_bits(), 0.0f32.to_bits()]);
        let mut world = World::new(mock, &[], &["dog.abk", "cat.abk"]);
        world.update([0.0; 3], LOCATION);
        // The first emitter weighs 0 and the second the default 10: the draw loads the second.
        assert_eq!(world.system.emitter_key, LOCATION);
        let loaded: Vec<(u64, u32, i32)> = world.system.emitter_statuses.iter().map(|status| (status.key, status.load_status, status.slot)).collect();
        assert_eq!(loaded, vec![(QUIET, NOT_LOADED, -1), (KEY, LOADED, 0)]);
        world.update([0.0; 3], LOCATION);
        world.update([0.0; 3], LOCATION);
        // A zero-length interval plays the loaded emitter at once, at half volume.
        let info = *world.states.info_mut(0).expect("playing");
        assert_eq!((info.aems_id, info.vol_state, info.azim_state, info.expire, info.timeout), (12, VOL_STATE_RANDOM, AZIM_STATE_EXTERNAL, true, 2.0));
        assert_eq!(info.vol, 0.5 * 0.8);
        assert_eq!(world.bank_count(), 1);
        // Once its state detaches the emitter unloads and its bank leaves AEMS.
        world.states.detach(0, &mut world.aems, &mut world.map);
        world.update([0.0; 3], LOCATION);
        assert_eq!(world.system.emitter_statuses[1].load_status, UNLOADING);
        world.audio_frame();
        assert_eq!(world.bank_count(), 0);
        // The slot empties and the emitter is drawn and loaded again.
        world.update([0.0; 3], LOCATION);
        assert_eq!((world.system.emitter_statuses[1].load_status, world.system.emitter_statuses[1].slot), (LOADED, 0));
        assert_eq!(world.system.random_triggers_loaded, 1);
    }

    #[test]
    fn reverb_volumes_need_a_known_reverb() {
        const ROOM: u64 = 0x5555_0000_0000_0005;
        const HALL: u64 = 0x6666_0000_0000_0006;
        let mut mock = Mock::default();
        mock.emitter(ROOM, VOLUME_REVERB, "", 1.0, 0.0, 0);
        mock.emitter(HALL, VOLUME_REVERB, "", 1.0, 0.0, 0);
        mock.ref_spec(AUD_EMITTER, ROOM, REVERB_TYPE, 0, VERB_HASH[3]);
        mock.ref_spec(AUD_EMITTER, HALL, REVERB_TYPE, 0, 0xDEAD);
        let volumes = [sphere(1, [0.0; 3], 10.0, ROOM), sphere(2, [0.0; 3], 20.0, HALL)];
        let mut world = World::new(mock, &volumes, &[]);
        world.update([1.0, 0.0, 0.0], 0);
        let first = world.system.current_reverb_volume(None, &world.vault).expect("room");
        assert_eq!(world.system.volume_status(first).map(|status| status.key), Some(ROOM));
        // The second volume's reverb is not in `verbHash`.
        assert_eq!(world.system.current_reverb_volume(Some(first), &world.vault), None);
        // The first reverb volume left decides: an unknown reverb gives none.
        world.update([15.0, 0.0, 0.0], 0);
        assert_eq!(world.system.current_reverb_volume(None, &world.vault), None);
        // A freed status keeps its last values.
        assert_eq!(world.system.volume_status(first).map(|status| (status.key, status.distance)), Some((ROOM, 0.1)));
    }
}
