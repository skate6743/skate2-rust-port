//! `Sk8::Challenge::cChallengeDynamicHullManager`
//! (`packages/sk8/sk8challenge/dynamichull`): the world pieces a challenge's state chooses, such
//! as Fun Track's gates, Slappy's gate, the pools' water and the skate stoppers.
//!
//! An activity whose `challenge_global_data` lists `tDynamicHullMapping`s (a state name and the
//! mission hull it shows) is a dynamic-hull activity (`AddActivity` `0x82db2ae8`). Its state is
//! `LookupValues[key]`, the `GetHashValue32` of the state's name: `AddActivity` sets the default
//! state when the profile has none, and the `Sk8ChallengeSystem` savable saves the map. In a
//! career (game type 4) or free skate (2) `GameLoad` (`0x82da48e8`) puts each activity's box (its
//! location ± its radius) in `DynamicHullTree`, and every update (`0x82db4dd0`) tests the local
//! skater's tester box against it (`PerformBBoxTests` `0x82db2e38`): an activity the box is in
//! loads the hull its state maps to through the manager's own `cPathedHullLoader`
//! ([`Output::LoadDynamicHull`]); one the box left unloads it ([`Output::UnloadDynamicHull`]).
//! `SetDynamicHullState` (`0x82db2ca0`) unloads the loaded hull of another state, and the next box
//! test loads the new state's hull when it maps one. The session runs the loader and answers each
//! requested load callback through [`on_complete`].

use super::{
    assets::{CONTENT_DIRECTORY, hull_id},
    binding::get_string,
    keys::{class, hash64, hash_value32},
    lua::{Lua, LuaResult, NativeCall, Value},
    presentation, signpost,
    signpost_progress::{self, Leaf, has_board_sponsor, subtract_cash},
    system::{Output, World},
    unlockables,
};
use crate::grind_data_octree::OctreeBounds;
use std::collections::BTreeMap;

/// `kInvalidHashValue32`.
pub const INVALID_HASH: u32 = 0xffff_ffff;
/// The savable `Sk8ChallengeSystem::SaveData` (`0x825a2ec0`) writes the chunk
/// into.
pub const SAVABLE: &str = "Sk8ChallengeSystem";
/// The savables registered before it (`BackEndManager::Init`), in order.
pub const SAVABLES_BEFORE: [&str; 6] = ["UNLOCKABLES", "CAC", "SKATER_PROFILE", "online", "STATISTICS", "DYNAMICOBJECTS"];
/// `kChallengeDynamicHullManagerHeaderName`, written at version 3.
pub const DATA: &str = "cChallengeDynamicHullManager::Data";
pub const DATA_VERSION: u32 = 3;
/// `kChallengeDynamicHullManagerVersion`, the packed states' version.
const PACK_VERSION: u32 = 3;
/// `PackHullStates` pads the states to this many slots.
const SLOTS: usize = 256;
/// `Sk8::Service::kUndrainedState`, `kDrainedState`, `kCappedState` and `kUncappedState`.
const UNDRAINED: &str = "Undrained";
const DRAINED: &str = "Drained";
const CAPPED: &str = "Capped";
const UNCAPPED: &str = "Uncapped";
/// The state `FrontEndState_ChallengeMap::ConfirmPurchaseCallback` (`0x826751f8`) sets.
const UNLOCKED: &str = "Unlocked";
/// The dam a progression script drains (`SetupDynamicHullState(.., "Drained")`).
const DAM: &str = "ol_fa_svm_21_accumulation_megadam_locator";
/// The skate stoppers progression scripts remove (`SetupDynamicHullState(.., "Uncapped")`).
const SCRIPTED_UNCAPS: [&str; 2] = ["mikey_uncap", "dt_madridplaza_0"];

/// The `challenge_global_data` attributes the manager reads.
mod field {
    /// `tDynamicHullMapping[]`: a state's `LookupName` and its `RequiredHull.HullName`.
    pub(super) const MAPPINGS: u64 = 0x0f2f_e38e_547c_5960;
    /// `Text`: the state `AddActivity` gives an activity the profile has no state for.
    pub(super) const DEFAULT_STATE: u64 = 0x4c91_1139_47cb_bbed;
    /// `tLocationID` `Location`: the locator the activity's box is centred on.
    pub(super) const LOCATION: u64 = 0xf1c6_d5fa_4a23_951e;
    /// `Float`: the box's half extent.
    pub(super) const RADIUS: u64 = 0x2c5f_b57a_89e8_cde2;
    /// `ActivityUtils::tActivityLoadMask`: the game types that load the activity's hulls.
    pub(super) const LOAD_MASK: u64 = 0xb9e5_f541_4f74_8c65;
    /// `Bool`: the hull stays while non-essential hulls are filtered.
    pub(super) const ESSENTIAL: u64 = 0xe2a4_965b_a631_d2c3;
    /// `tDynamicHullUnlock`: the `fe_locations` entry whose free-skate unlock gives the state
    /// `LookupName` (`GetDynamicHullUnlockValue`).
    pub(super) const UNLOCK: u64 = 0x58e8_06f8_df17_d732;
    /// `DebugOnly`: `CreateActivity` gives the sub-controllers no debug-only activity.
    pub(super) const DEBUG_ONLY: u64 = 0xfd40_e4a3_1027_c984;
}

/// `challenges` `Bool` 0x198a0e276e56a9d6: the hull's DMOs are treated as static.
const DMO_TREAT_AS_STATIC: u64 = 0x198a_0e27_6e56_a9d6;
/// `challenge_local_data` `RefSpec[]` 0xe1a63bc6cb47832c: the `challenges` whose state
/// `SetCappingOnJacked` can set `Capped`.
pub(crate) const CAP_LIST: u64 = 0xe1a6_3bc6_cb47_832c;
/// `eAssetFlags_DMODisallowPersistance | eAssetFlags_DMODisallowPriorityTracking |
/// eAssetFlags_DMOForceCull`, which `PerformBBoxTests` loads every hull with.
const HULL_ASSET_FLAGS: u32 = 0xe0;
/// `eAssetFlags_DMOTreatAsStatic`.
const ASSET_FLAG_DMO_TREAT_AS_STATIC: u32 = 0x10;
/// `ActivityUtils::kDefaultLocationTrackingRadius` (cvar default 40), the radius of a box whose
/// activity has none.
const DEFAULT_LOCATION_TRACKING_RADIUS: f32 = 40.0;
/// The hull ID `Update` (`0x82db4dd0`) answers each dispatched callback with.
const DISPATCH_HULL_ID: u32 = 0x1234_abcd;

/// A `tDynamicHullMapping`: the state's name and the mission hull it shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mapping {
    pub state: String,
    pub hull: String,
}

/// The `cHullLoader::cHullLoadRequestCallback`s a state change can hand the manager.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadCallback {
    /// `cCPDSetStateLoadCallback` (`OnComplete` `0x82e08a30`): presentation event 33 waits for
    /// the new state's hull, then the director plays its active group's next event.
    PresentationDirector,
}

/// `cChallengeDynamicHullManager`'s state.
#[derive(Debug)]
pub struct DynamicHulls {
    /// `LookupValues`.
    pub lookup_values: BTreeMap<u64, u32>,
    /// `OverrideGameFlags`, as `UnpackHullStates` reads it.
    pub override_game_flags: u32,
    /// `GameTypeFlags`.
    pub game_type_flags: u32,
    /// `GameLoaded`.
    pub game_loaded: bool,
    /// `AllowNonEssentialHulls` (the constructor's 1).
    pub allow_non_essential: bool,
    /// `AllDynamicHullActivities`, as activity indices.
    all: Vec<usize>,
    /// `DynamicHullTree`: each inserted activity with its box.
    tree: Vec<(usize, OctreeBounds)>,
    /// `PreviousCollisionIndicies`.
    previous: Vec<u32>,
    /// `LoadedDynamicHulls`: the mapping each activity's loaded hull came from.
    pub loaded: BTreeMap<u64, Mapping>,
    /// `LoadCallbackMap`.
    load_callbacks: BTreeMap<u64, LoadCallback>,
    /// `DispatchCallbacks`.
    dispatch: Vec<LoadCallback>,
    /// The callbacks handed to the loader with a hull, answered by its `OnComplete`.
    pending: Vec<(u32, LoadCallback)>,
    /// A travel has not landed yet. The native loading flow holds the simulation
    /// (`kPAUSE_STATE_TELEPORT`) from `StateLoading::DynamicHullState` until the skater stands at
    /// the destination, so no update tests the place he left; the port's travel lands a few
    /// updates after its spot test.
    pub teleport_hold: bool,
}

impl Default for DynamicHulls {
    /// The constructor (`0x82dab850`).
    fn default() -> Self {
        Self {
            lookup_values: BTreeMap::new(),
            override_game_flags: 0,
            game_type_flags: 0,
            game_loaded: false,
            allow_non_essential: true,
            all: Vec::new(),
            tree: Vec::new(),
            previous: Vec::new(),
            loaded: BTreeMap::new(),
            load_callbacks: BTreeMap::new(),
            dispatch: Vec::new(),
            pending: Vec::new(),
            teleport_hold: false,
        }
    }
}

impl DynamicHulls {
    /// `PackHullStates(stream, false)` (`0x82da4aa8`): the version, a zero
    /// game-type word, the count, each state (key, value) in key order, then
    /// `(-1, kInvalidHashValue32)` up to 256 slots; all big-endian.
    #[must_use]
    pub fn pack(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(12 + SLOTS * 12);
        out.extend_from_slice(&PACK_VERSION.to_be_bytes());
        out.extend_from_slice(&0u32.to_be_bytes());
        out.extend_from_slice(&u32::try_from(self.lookup_values.len()).unwrap_or(u32::MAX).to_be_bytes());
        for (&key, &value) in &self.lookup_values {
            out.extend_from_slice(&key.to_be_bytes());
            out.extend_from_slice(&value.to_be_bytes());
        }
        for _ in self.lookup_values.len()..SLOTS {
            out.extend_from_slice(&u64::MAX.to_be_bytes());
            out.extend_from_slice(&INVALID_HASH.to_be_bytes());
        }
        out
    }

    /// `UnpackHullStates` (`0x82db3210`): a version-3 buffer's game flags and
    /// its counted states, each set in `LookupValues`.
    pub fn unpack(&mut self, bytes: &[u8]) {
        let word = |at: usize| bytes.get(at..at + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
        if word(0) != Some(PACK_VERSION) {
            return;
        }
        self.override_game_flags = word(4).unwrap_or(0);
        let count = word(8).unwrap_or(0) as usize;
        for i in 0..count {
            let at = 12 + i * 12;
            let Some(key) = bytes.get(at..at + 8).map(|b| u64::from_be_bytes(b.try_into().unwrap_or([0xff; 8]))) else {
                break;
            };
            let value = word(at + 8).unwrap_or(INVALID_HASH);
            self.lookup_values.insert(key, value);
        }
    }

    /// `cChallengeDynamicHullManager::SaveData` (`0x82da97f8`): the packed
    /// states' size (`WriteUInt`) and the states (`WriteData`).
    #[must_use]
    pub fn save(&self) -> [Leaf; 1] {
        let packed = self.pack();
        let size = u32::try_from(packed.len()).unwrap_or(u32::MAX);
        [(DATA, DATA_VERSION, vec![size.to_be_bytes().to_vec(), packed])]
    }

    /// `cChallengeDynamicHullManager::LoadData` (`0x82db4330`): a version-3
    /// chunk whose data matches its size.
    pub fn load(&mut self, saved: signpost_progress::Saved<'_>) {
        let Some((DATA_VERSION, [size, data, ..])) = saved else { return };
        let Ok(size) = <[u8; 4]>::try_from(size.as_slice()) else { return };
        if u32::from_be_bytes(size) as usize != data.len() || data.is_empty() {
            return;
        }
        self.unpack(data);
    }
}

/// The ID `cPathedHullLoader::UnloadHull(fileName, …)` (`0x82eeec78`) looks the hull up by:
/// `GetHashValue32` of `"<base>\<file name>"`, the name not clamped as `LoadHull` clamps it.
fn unload_id(file_name: &str) -> u32 {
    hash_value32(&format!("{CONTENT_DIRECTORY}\\{file_name}"))
}

/// The activity's `challenge_global_data` attribute.
fn global<'a>(world: &'a World, activity: usize, key: u64) -> Option<super::attrib_db::Attribute<'a>> {
    let (c, k) = world.activities.activities[activity].global_data?;
    world.db.attribute(c, k, key)
}

fn global_bool(world: &World, activity: usize, key: u64) -> bool {
    global(world, activity, key).and_then(|a| a.bool(0)).unwrap_or(false)
}

/// The activity's `tDynamicHullMapping` `index`.
fn mapping(world: &World, activity: usize, index: usize) -> Option<Mapping> {
    let at = global(world, activity, field::MAPPINGS)?.data(index)?;
    let state = world.db.text_at(at)?.to_owned();
    let hull = world.db.text_at(at.offset_by(4)).unwrap_or_default().to_owned();
    Some(Mapping { state, hull })
}

fn mapping_count(world: &World, activity: usize) -> usize {
    global(world, activity, field::MAPPINGS).map_or(0, |a| a.len())
}

/// `cChallengeDynamicHullManager::Init` (`0x82da47e0`) with `cActivityManager::CreateActivity`
/// (`0x82cdbca8`): every activity that is not debug-only (`kAllowDebugOnlyActivities` off) goes
/// to `AddActivity` (`0x82db2ae8`), which keeps those with mappings and gives each its default
/// state unless `LookupValues` has one.
pub(crate) fn init(world: &mut World) {
    world.dynamic_hulls.tree.clear();
    for activity in 0..world.activities.activities.len() {
        if global_bool(world, activity, field::DEBUG_ONLY) || mapping_count(world, activity) == 0 {
            continue;
        }
        if !world.dynamic_hulls.all.contains(&activity) {
            world.dynamic_hulls.all.push(activity);
        }
        let default = global(world, activity, field::DEFAULT_STATE).and_then(|a| a.text(0)).unwrap_or_default();
        if !default.is_empty() {
            let (key, value) = (world.activities.activities[activity].key, hash_value32(default));
            world.dynamic_hulls.lookup_values.entry(key).or_insert(value);
        }
    }
}

/// `ActivityUtils::GenerateLocationActivityBBox` (`0x82cd89c0`): the location's position ± the
/// radius (`kDefaultLocationTrackingRadius` when not above zero) in x, y and z.
fn location_bounds(position: [f32; 3], radius: f32) -> OctreeBounds {
    let r = if radius > 0.0 { radius } else { DEFAULT_LOCATION_TRACKING_RADIUS };
    OctreeBounds {
        minimum: [position[0] - r, position[1] - r, position[2] - r, 0.0],
        maximum: [position[0] + r, position[1] + r, position[2] + r, 0.0],
    }
}

/// `GameLoad(gameTypeFlags)` (`0x82da48e8`): the game type is kept; a free skate (2) or career
/// (4) game is loaded and each activity whose location is valid gets its box in the tree.
pub(crate) fn game_load(world: &mut World, game_type_flags: u32) {
    world.dynamic_hulls.game_type_flags = game_type_flags;
    if game_type_flags != 2 && game_type_flags != 4 {
        return;
    }
    world.dynamic_hulls.game_loaded = true;
    for activity in world.dynamic_hulls.all.clone() {
        let Some(location) = global(world, activity, field::LOCATION).and_then(|a| a.text(0)) else { continue };
        let radius = global(world, activity, field::RADIUS).and_then(|a| a.f32(0)).unwrap_or(0.0);
        // cLocationManager::IsValidLocation, then GetLocationPosition.
        let Some(transform) = world.location(hash_value32(location)) else { continue };
        let bounds = location_bounds([transform[12], transform[13], transform[14]], radius);
        world.dynamic_hulls.tree.push((activity, bounds));
    }
}

/// `GameUnload` (`0x82da6470`): a loaded game resets.
pub(crate) fn game_unload(world: &mut World) {
    if world.dynamic_hulls.game_loaded {
        world.dynamic_hulls.game_loaded = false;
        reset(world);
    }
}

/// `Reset` (`0x82da63d8`): the tree and the box test's collisions emptied, every loaded hull
/// unloaded, the game flags cleared. `LoadCallbackMap` and `DispatchCallbacks` keep theirs.
fn reset(world: &mut World) {
    let hulls = &mut world.dynamic_hulls;
    hulls.tree.clear();
    hulls.previous.clear();
    let loaded = std::mem::take(&mut hulls.loaded);
    hulls.override_game_flags = 0;
    hulls.game_type_flags = 0;
    for mapping in loaded.into_values() {
        world.output(Output::UnloadDynamicHull { hull: unload_id(&mapping.hull) });
    }
}

/// `Update(localPlayerInfo)` (`0x82db4dd0`) up to the loader's update, which the session runs
/// after the system's: in a loaded game, the box test at the local skater's tester base while
/// its tester is valid, then the dispatched callbacks. The native calls every
/// `DispatchCallbacks` entry on every update and never empties the vector, while each
/// `cCPDSetStateLoadCallback` deletes itself in `OnComplete`; here each is called once.
pub(crate) fn update(world: &mut World) {
    if !world.dynamic_hulls.game_loaded || world.dynamic_hulls.teleport_hold {
        return;
    }
    if let Some(base) = world.player.tester_base {
        perform_bbox_tests(world, base);
    }
    for callback in std::mem::take(&mut world.dynamic_hulls.dispatch) {
        complete(world, callback, DISPATCH_HULL_ID);
    }
}

/// `SpotTestAgainstPoint` (`0x82db4318`): `StateLoading::DynamicHullState` (`0x82763be8`) tests
/// the stream focus before the loading flow waits for the loader.
pub fn spot_test_against_point(world: &mut World, point: [f32; 3]) {
    if world.dynamic_hulls.game_loaded {
        perform_bbox_tests(world, point);
    }
}

/// `PerformBBoxTests` (`0x82db2e38`, `kEnableDynamicHullLoading` on): the tester box
/// (`TriggerVolumeUtils::GenerateTesterBBox` `0x82e20ee0`) against the tree. Every activity the
/// box holds whose hull `RequiresHullLoad` loads it (registered on load, normal priority, the
/// `LoadCallbackMap` callback taken out of the map), with its mapping in `LoadedDynamicHulls`;
/// every activity the box left unloads its loaded hull.
fn perform_bbox_tests(world: &mut World, position: [f32; 3]) {
    let tester = super::characters::tester_bounds(position);
    // GetCollisions sorts the indices; GenerateEnterExitData's exits keep the previous order.
    let colliding: Vec<u32> = world
        .dynamic_hulls
        .tree
        .iter()
        .enumerate()
        .filter(|(_, (_, bounds))| bounds.overlaps(tester))
        .filter_map(|(index, _)| u32::try_from(index).ok())
        .collect();
    let exited: Vec<u32> = world.dynamic_hulls.previous.iter().copied().filter(|index| !colliding.contains(index)).collect();
    for &index in &colliding {
        let Some(&(activity, _)) = world.dynamic_hulls.tree.get(index as usize) else { continue };
        let Some(mapping) = requires_hull_load(world, activity) else { continue };
        let key = world.activities.activities[activity].key;
        let treat_as_static =
            world.db.attribute(class::CHALLENGES, key, DMO_TREAT_AS_STATIC).and_then(|a| a.bool(0)).unwrap_or(false);
        let asset_flags = HULL_ASSET_FLAGS | if treat_as_static { ASSET_FLAG_DMO_TREAT_AS_STATIC } else { 0 };
        let callback = world.dynamic_hulls.load_callbacks.remove(&key);
        let hull = hull_id(&mapping.hull);
        if let Some(callback) = callback {
            world.dynamic_hulls.pending.push((hull, callback));
        }
        world.output(Output::LoadDynamicHull { hull, file_name: mapping.hull.clone(), callback: callback.is_some(), asset_flags });
        world.dynamic_hulls.loaded.insert(key, mapping);
    }
    for &index in &exited {
        let Some(&(activity, _)) = world.dynamic_hulls.tree.get(index as usize) else { continue };
        let key = world.activities.activities[activity].key;
        if let Some(mapping) = world.dynamic_hulls.loaded.remove(&key) {
            world.output(Output::UnloadDynamicHull { hull: unload_id(&mapping.hull) });
        }
    }
    world.dynamic_hulls.previous = colliding;
}

/// `RequiresHullLoad` (`0x82da96b0`): the mapping whose hull the activity needs now. The
/// activity's load mask must share a bit with `OverrideGameFlags` (`GameTypeFlags` when none);
/// a non-essential hull needs `AllowNonEssentialHulls`; no hull of it may be loaded; its state
/// must be valid and map a hull (`DetermineDynamicHull`).
fn requires_hull_load(world: &World, activity: usize) -> Option<Mapping> {
    let hulls = &world.dynamic_hulls;
    let flags = if hulls.override_game_flags == 0 { hulls.game_type_flags } else { hulls.override_game_flags };
    let mask = global(world, activity, field::LOAD_MASK).and_then(|a| a.u32(0)).unwrap_or(0);
    if flags & mask == 0 || (!global_bool(world, activity, field::ESSENTIAL) && !hulls.allow_non_essential) {
        return None;
    }
    let key = world.activities.activities[activity].key;
    if hulls.loaded.contains_key(&key) {
        return None;
    }
    let lookup = lookup_value(world, key)?;
    if lookup == INVALID_HASH {
        return None;
    }
    determine_dynamic_hull(world, activity, lookup)
}

/// `DetermineDynamicHull` (`0x82da16c0`): the first mapping whose state's `GetHashValue32` is
/// the lookup value.
fn determine_dynamic_hull(world: &World, activity: usize, lookup: u32) -> Option<Mapping> {
    (0..mapping_count(world, activity))
        .filter_map(|index| mapping(world, activity, index))
        .find(|mapping| hash_value32(&mapping.state) == lookup)
}

/// `GetLookupValue` (`0x82da6590`): the challenge's `GetDynamicHullUnlockValue`, else its
/// `LookupValues` entry, else `kInvalidHashValue32`. The native reads the activity without a
/// check; a key that names none is `None` here.
fn lookup_value(world: &World, key: u64) -> Option<u32> {
    let activity = world.activities.find(key)?;
    let unlocked = unlock_value(world, activity);
    if unlocked != INVALID_HASH {
        return Some(unlocked);
    }
    Some(world.dynamic_hulls.lookup_values.get(&key).copied().unwrap_or(INVALID_HASH))
}

/// `Sk8ChallengeSystem::GetDynamicHullUnlockValue` (`0x825a4560`, through the
/// `IDynamicHullUnlock` base): only in free skate (`GameModeFlags` 2) and only for an unlock whose
/// `LockEntry` names a collection, the `GetHashValue32` of its `LookupName` once the profile has
/// unlocked the entry's location (`UnlockablesBackEndCore::IsUnlocked(2, layout name, true)`) or
/// every location (`kAllLocationsUnlockID`); otherwise `kInvalidHashValue32`.
fn unlock_value(world: &World, activity: usize) -> u32 {
    if world.game_mode_flags != 2 {
        return INVALID_HASH;
    }
    let Some(unlock) = global(world, activity, field::UNLOCK) else { return INVALID_HASH };
    let Some((entry_class, entry)) = unlock.ref_spec(0).filter(|&(c, k)| world.db.has_collection(c, k)) else {
        return INVALID_HASH;
    };
    let unlocks = &world.unlockables;
    let location = unlockables::category::LOCATIONS;
    let all = unlocks.is_unlocked(location, unlockables::string_hash("all_locations_unlock"), true);
    let name = world.db.layout_text(entry_class, entry, 0).unwrap_or_default();
    if !unlocks.is_unlocked(location, unlockables::string_hash(name), true) && !all {
        return INVALID_HASH;
    }
    let lookup_name = unlock.data(0).and_then(|at| world.db.text_at(at.offset_by(24))).unwrap_or_default();
    hash_value32(lookup_name)
}

/// `TestForDynamicHullState(key, state)` (`0x82da9670`): `None` when the key names no activity.
pub(crate) fn test_state(world: &World, key: u64, state: &str) -> Option<bool> {
    lookup_value(world, key).map(|v| v == hash_value32(state))
}

/// `SetDynamicHullState(key, state, loadCallback)` (`0x82db2ca0`): a loaded hull whose state is
/// valid and another one is unloaded and leaves `LoadedDynamicHulls`, then the state is set. A
/// callback waits in `LoadCallbackMap` for the hull the new state needs, or goes to
/// `DispatchCallbacks` when none is needed or the map already holds one for the key.
pub(crate) fn set_state(world: &mut World, key: u64, state: &str, callback: Option<LoadCallback>) {
    let hash = hash_value32(state);
    if let Some(mapping) = world.dynamic_hulls.loaded.get(&key).cloned() {
        let lookup = lookup_value(world, key).unwrap_or(INVALID_HASH);
        if lookup != INVALID_HASH && lookup != hash {
            world.output(Output::UnloadDynamicHull { hull: unload_id(&mapping.hull) });
            world.dynamic_hulls.loaded.remove(&key);
        }
    }
    world.dynamic_hulls.lookup_values.insert(key, hash);
    let challenge = world.activities.find(key).map_or_else(|| format!("{key:#018x}"), |a| world.activities.activities[a].name.clone());
    world.output(Output::DynamicHullState { challenge, state: state.to_owned() });
    let Some(callback) = callback else { return };
    let Some(activity) = world.activities.find(key) else { return };
    if requires_hull_load(world, activity).is_some() && !world.dynamic_hulls.load_callbacks.contains_key(&key) {
        world.dynamic_hulls.load_callbacks.insert(key, callback);
    } else {
        world.dynamic_hulls.dispatch.push(callback);
    }
}

/// `SetDynamicHullState(activity, state, loadCallback)` (`0x82db2df8`): the activity's key.
pub(crate) fn set_activity_state(world: &mut World, activity: usize, state: &str, callback: Option<LoadCallback>) {
    let key = world.activities.activities[activity].key;
    set_state(world, key, state, callback);
}

/// `FilterNonEssentialDynamicHulls` (`0x82da6490`): every loaded hull of an existing activity that
/// is not essential is unloaded and leaves `LoadedDynamicHulls`; then non-essential hulls are no
/// longer allowed.
pub(crate) fn filter_non_essential(world: &mut World) {
    let loaded: Vec<u64> = world.dynamic_hulls.loaded.keys().copied().collect();
    for key in loaded {
        let Some(activity) = world.activities.find(key) else { continue };
        if global_bool(world, activity, field::ESSENTIAL) {
            continue;
        }
        if let Some(mapping) = world.dynamic_hulls.loaded.remove(&key) {
            world.output(Output::UnloadDynamicHull { hull: unload_id(&mapping.hull) });
        }
    }
    world.dynamic_hulls.allow_non_essential = false;
}

/// A loaded hull's `cHullLoadRequestCallback::OnComplete`, from the loader's update: the
/// callback handed to the loader with this hull.
pub fn on_complete(world: &mut World, hull: u32) {
    let pending = &mut world.dynamic_hulls.pending;
    if let Some(index) = pending.iter().position(|&(id, _)| id == hull) {
        let (_, callback) = pending.remove(index);
        complete(world, callback, hull);
    }
}

fn complete(world: &mut World, callback: LoadCallback, _hull: u32) {
    match callback {
        LoadCallback::PresentationDirector => presentation::set_state_load_complete(world),
    }
}

/// `FrontEndState_ChallengeMap::ConfirmPurchaseCallback` (`0x826751f8`) after a yes: the
/// property's `PropertyCost` economy data is subtracted (`SubtractCash`, no credit), its save
/// data `PlayerOwns` set to 1, its dynamic hull set `Unlocked` and the challenge completed
/// (`cSignPostManager::CompleteChallenge(key, 1)`). The Big Black dismissal popup and the
/// teleport fade that follow are the front end's.
pub fn purchase_property(world: &mut World, activity: usize) {
    let key = world.activities.activities[activity].key;
    let cost = signpost_progress::challenge_economy_data(world, key, "PropertyCost");
    subtract_cash(world, cost, 0);
    signpost_progress::set_challenge_save_data(world, key, "PlayerOwns", 1);
    set_activity_state(world, activity, UNLOCKED, None);
    signpost::complete_challenge_by_key(world, key);
}

/// PC port addition (Max Out Save, `max_out.rs`): each dynamic-hull activity in the state the
/// whole career leaves it in. Gates and the properties open: a `Locked` or `UnderConstruction`
/// one `Unlocked` (the state a bought property takes), Slappy's `Unlock` (its script's state, the
/// one the minimap tests), a `Closed` one `Open` and the GvR added (`AddedGvR`). The pools the
/// player can have drained (`service_draining`'s) and the dam are `Drained`; the skate stoppers
/// the player can have removed (`service_uncapping`'s) and the two the scripts remove are
/// `Uncapped`. Any other activity keeps its state. `SKATE2_TRACE_MAX_OUT` logs each activity.
pub(crate) fn max_out(world: &mut World) {
    let trace = std::env::var_os("SKATE2_TRACE_MAX_OUT").is_some();
    for activity in world.dynamic_hulls.all.clone() {
        let key = world.activities.activities[activity].key;
        let states: Vec<String> =
            (0..mapping_count(world, activity)).filter_map(|index| mapping(world, activity, index)).map(|m| m.state).collect();
        let has = |state: &str| states.iter().any(|known| known == state);
        let state = if key == hash64("slappys") {
            Some("Unlock")
        } else if has("NoGvR") {
            Some("AddedGvR")
        } else if has("Closed") {
            Some("Open")
        } else if has("Locked") || has("UnderConstruction") {
            Some(UNLOCKED)
        } else if has(UNDRAINED) && (serviceable(world, key, "service_draining") || key == hash64(DAM)) {
            Some(DRAINED)
        } else if has(CAPPED) && (serviceable(world, key, "service_uncapping") || SCRIPTED_UNCAPS.iter().any(|&name| key == hash64(name))) {
            Some(UNCAPPED)
        } else {
            None
        };
        if trace {
            let was = world.dynamic_hulls.lookup_values.get(&key).and_then(|&value| states.iter().find(|s| hash_value32(s) == value));
            eprintln!(
                "max out: hull {} {} -> {} of {states:?}",
                world.activities.activities[activity].name,
                was.map_or("-", String::as_str),
                state.unwrap_or("kept")
            );
        }
        if let Some(state) = state {
            world.dynamic_hulls.lookup_values.insert(key, hash_value32(state));
        }
    }
}

/// Whether the challenge is below `root` among the challenges and the player can pay for its
/// service (`IsServiceableByUser`), as `GetNumPoolsInWorld` (`0x8289c890`) counts the pools.
fn serviceable(world: &World, key: u64, root: &str) -> bool {
    let root = hash64(root);
    let mut parent = world.db.parent(class::CHALLENGES, key);
    while let Some(value) = parent.filter(|&p| p != 0 && p != root) {
        parent = world.db.parent(class::CHALLENGES, value);
    }
    parent == Some(root)
        && world.db.attribute(class::CHALLENGES, key, hash64("IsServiceableByUser")).and_then(|a| a.bool(0)).unwrap_or(false)
}

/// `DynamicHull.SetupDynamicHullState(challenge, state)` (`0x82db41f8`): `SetDynamicHullState`
/// of `Attrib::StringToKey(challenge)` without a callback.
fn setup_dynamic_hull_state(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let challenge = get_string(call, 0).unwrap_or_default();
    let state = get_string(call, 1).unwrap_or_default();
    set_state(world, hash64(&challenge), &state, None);
    Ok(Vec::new())
}

/// `DynamicHull.TestForDynamicHullState(challenge, state)` (`0x82dab7b0`).
fn test_for_dynamic_hull_state(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let challenge = get_string(call, 0).unwrap_or_default();
    let state = get_string(call, 1).unwrap_or_default();
    match test_state(world, hash64(&challenge), &state) {
        Some(set) => Ok(vec![Value::Boolean(set)]),
        None => Err(lua.error(format!("TestForDynamicHullState: no challenge {challenge}"))),
    }
}

/// `DynamicHull.FilterNonEssentialDynamicHulls()` (`0x82da9530`).
fn filter_non_essential_dynamic_hulls(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> LuaResult<Vec<Value>> {
    filter_non_essential(world);
    Ok(Vec::new())
}

/// `DynamicHull.AllowNonEssentialDynamicHulls()` (`0x82da4760`): `AllowNonEssentialHulls` set.
fn allow_non_essential_dynamic_hulls(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> LuaResult<Vec<Value>> {
    world.dynamic_hulls.allow_non_essential = true;
    Ok(Vec::new())
}

/// `Service.DrainPoolViaProgression(challenge)` (`0x8288f6a8` →
/// `cServiceManager::DrainPoolViaProgression` `0x8289cc90`): an undrained
/// pool costs $200 without a board sponsor (`SubtractCash`, credit 0), is set
/// drained and its challenge completed. The stats (24, 25 and 28) and the
/// `Pull_The_Plug` achievement for the last pool go to the stats back end,
/// which is not ported.
fn drain_pool_via_progression(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let challenge = get_string(call, 0).unwrap_or_default();
    match test_state(world, hash64(&challenge), UNDRAINED) {
        Some(true) => {
            if !has_board_sponsor(world) {
                subtract_cash(world, 200, 0);
            }
            super::services::record_service(world, super::services::service_type::DRAINING, !has_board_sponsor(world));
            set_state(world, hash64(&challenge), DRAINED, None);
            signpost::complete_challenge_by_name(world, &challenge);
            Ok(Vec::new())
        }
        Some(false) => Ok(Vec::new()),
        None => Err(lua.error(format!("DrainPoolViaProgression: no challenge {challenge}"))),
    }
}

/// `Service.UncapRailsViaProgression(challenge)` (`0x8288f750` →
/// `cServiceManager::UncapRailsViaProgression` `0x8289c730`): capped rails cost $100 without a
/// board sponsor (`SubtractCash`, credit 0), are set uncapped and their challenge completed. The
/// stats (26 and 27) and the `New_San_Van_Hero` achievement at ten go to the stats back end,
/// which is not ported.
fn uncap_rails_via_progression(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let challenge = get_string(call, 0).unwrap_or_default();
    match test_state(world, hash64(&challenge), CAPPED) {
        Some(true) => {
            if !has_board_sponsor(world) {
                subtract_cash(world, 100, 0);
            }
            super::services::record_service(world, super::services::service_type::UNCAPPING, !has_board_sponsor(world));
            set_state(world, hash64(&challenge), UNCAPPED, None);
            signpost::complete_challenge_by_name(world, &challenge);
            Ok(Vec::new())
        }
        Some(false) => Ok(Vec::new()),
        None => Err(lua.error(format!("UncapRailsViaProgression: no challenge {challenge}"))),
    }
}

/// `cServiceManager::SetCappingOnJacked` (`0x8289ca48`): of the activity's cap list, the
/// challenge whose global data `Location` is a valid location nearest the skater (squared
/// distance, the first of equals) is set `Capped`. `ChaseMan::JackSkater` (`0x8270fd08`) calls
/// it for a jack of the local skater with no main challenge, the activity being the chaser's
/// `Pedestrian::mChallengeOwner`, which only `cChallengeCharacterManager::TestBBoxAgainstTree`
/// (`0x825ab590`) sets, on the characters it spawns.
pub fn set_capping_on_jacked(world: &mut World, activity: usize, skater: [f32; 3]) {
    let Some((class, collection)) = world.activities.activities[activity].local_data else { return };
    let mut nearest: Option<(u64, f32)> = None;
    if let Some(list) = world.db.attribute(class, collection, CAP_LIST) {
        for index in 0..list.len() {
            let Some((_, challenge)) = list.ref_spec(index) else { continue };
            let Some(location) = world
                .db
                .attribute(class::CHALLENGES, challenge, super::runtime::field::GLOBAL_DATA)
                .and_then(|a| a.ref_spec(0))
                .and_then(|(c, k)| world.db.attribute(c, k, field::LOCATION))
                .and_then(|a| a.text(0))
            else {
                continue;
            };
            // cLocationManager::IsValidLocation, then GetLocationOrientation's translation.
            let hash = hash_value32(location);
            let Some(transform) = world.host.location_orientation(hash)
                .or_else(|| world.locations.get(&hash).copied()) else { continue };
            let d = [transform[12] - skater[0], transform[13] - skater[1], transform[14] - skater[2], 0.];
            // vsubfp/vmsum3fp1288289cc14..20, using the shared host VMX reduction.
            let distance = crate::reckoning_numeric::host_dot3(d, d);
            if nearest.is_none_or(|(_, best)| distance < best) {
                nearest = Some((challenge, distance));
            }
        }
    }
    if let Some((challenge, _)) = nearest {
        set_state(world, challenge, CAPPED, None);
    }
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, super::lua::NativeFn<World>)> {
    vec![
        ("DynamicHull", "SetupDynamicHullState", setup_dynamic_hull_state),
        ("DynamicHull", "TestForDynamicHullState", test_for_dynamic_hull_state),
        ("DynamicHull", "FilterNonEssentialDynamicHulls", filter_non_essential_dynamic_hulls),
        ("DynamicHull", "AllowNonEssentialDynamicHulls", allow_non_essential_dynamic_hulls),
        ("Service", "DrainPoolViaProgression", drain_pool_via_progression),
        ("Service", "UncapRailsViaProgression", uncap_rails_via_progression),
    ]
}

#[cfg(test)]
mod tests {
    use super::DynamicHulls;

    #[test]
    fn packed_states_round_trip_and_pad_to_256_slots() {
        let mut hulls = DynamicHulls::default();
        hulls.lookup_values.insert(0x39a2_2309_8c44_8b9a, 0x1234_5678);
        hulls.lookup_values.insert(0x0000_0000_0000_0001, 7);
        let packed = hulls.pack();
        assert_eq!(packed.len(), 12 + 256 * 12);
        assert_eq!(&packed[..12], &[0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 2]);
        assert_eq!(&packed[12..24], &[0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 7]);
        assert_eq!(&packed[36..48], &[0xff; 12]);
        let mut loaded = DynamicHulls::default();
        loaded.load(Some((3, &hulls.save()[0].2)));
        assert_eq!(loaded.lookup_values, hulls.lookup_values);
    }
}
