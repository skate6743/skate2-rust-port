//! `Sk8::Challenge::cChallengeAssetManager`: the per-challenge load pipeline
//! (challenge hulls, skater models, AI assets and paths). A finished load
//! posts `cMsgChallengeAssetLoadComplete`.
//!
//! The manager's `cPathedHullLoader` (`kChallengeContentDirectory`, load
//! flags 7, file names clamped to 30 bytes) runs on the host:
//! [`Output::LoadHull`] and [`Output::UnloadHull`] reach it, and every hull
//! it finishes comes back through [`on_complete`], the manager's
//! `cHullLoadRequestCallback::OnComplete` (`0x82e0fed0`). An AI challenge
//! then selects its characters and loads their models, then its static AI
//! path package, through the `ChallengeAIManager` (`super::ai`).

use super::{
    ai,
    attrib_db::{AttribDb, Location},
    attrib_lua::attrib_location,
    keys::{class, hash_value32},
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    objectives,
    system::{Event, Field, Message, Output, World},
};
use std::collections::BTreeMap;

/// `Sk8::Challenge::kChallengeContentDirectory`.
pub const CONTENT_DIRECTORY: &str = "data\\content\\missions.big|data\\content\\missions";
/// The directory half of [`CONTENT_DIRECTORY`]; its archive is mounted.
pub const CONTENT_STREAM_DIRECTORY: &str = "data\\content\\missions";
/// The manager's `cHullLoader::FileNameClamp`.
const FILE_NAME_CLAMP: usize = 30;
/// `challenges.RequiredChallengeHull` (`tRequiredChallengeHull[]`).
const REQUIRED_CHALLENGE_HULL: u64 = 0x876b_f081_9aa5_8858;
/// `challenges.ChallengeAssetLoadType`.
const CHALLENGE_ASSET_LOAD_TYPE: u64 = 0xc410_489a_c117_b97e;
/// `eChallengeAssetLoadType_UninterruptedLoad`.
const UNINTERRUPTED_LOAD: u32 = 1;
/// `eChallengeAssetLoadType_NoLoad`.
const NO_LOAD: u32 = 2;
/// `challenge_local_data.UseDemoSponsorCompetitors`.
const USE_DEMO_SPONSOR_COMPETITORS: u64 = 0x85ab_e4c3_17f5_acad;
/// `challenges.DmoTreatAsStatic`, which loads an additional hull with asset flag 0x10.
const DMO_TREAT_AS_STATIC: u64 = 0x198a_0e27_6e56_a9d6;
/// `challenges.CAIAssetOverride`.
const CAI_ASSET_OVERRIDE: u64 = 0x0550_ce4f_a91d_53e8;
/// `eChallengeTypes` Jam, MegaRamp and StreetSkating, whose characters' models
/// `CAIAssetOverride` can leave unloaded.
const JAM: i32 = 15;
const MEGA_RAMP: i32 = 26;
const STREET_SKATING: i32 = 27;

/// `eChallengeLoadState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoadState {
    LoadNotStarted = 0,
    WaitingForChallengeHull = 1,
    AllChallengeHullsLoaded = 2,
    WaitingForAccept = 3,
    WaitingForSkaterModels = 4,
    WaitingForAiPaths = 5,
    LoadCompleteNotify = 6,
    LoadComplete = 7,
    UnloadNotStarted = 8,
    WaitingForChallengeHullUnload = 9,
    AllChallengeHullsUnloaded = 10,
    UnloadComplete = 11,
}

/// `tChallengeLoadInfo`.
#[derive(Clone, Debug)]
pub struct LoadInfo {
    pub activity: usize,
    pub key: u64,
    pub state: LoadState,
    pub num_hulls: i32,
    pub hull_index: i32,
    pub accepted: bool,
    pub requested_unload: bool,
    pub register_on_load: bool,
    pub high_priority: bool,
    pub asset_flags: u32,
    pub hull_ids: [u32; 4],
}

impl LoadInfo {
    fn hull(&self, index: i32) -> u32 {
        usize::try_from(index).ok().and_then(|i| self.hull_ids.get(i)).copied().unwrap_or(0)
    }
}

/// `tAdditionalHullLoadInfo` (16 bytes) under its activity's unique ID.
#[derive(Clone, Debug)]
pub struct AdditionalHull {
    pub activity_key: u64,
    /// `HullPointer`: the `tRequiredChallengeHull` the hull was loaded for.
    pub hull_pointer: Location,
    pub hull: u32,
    /// `LoadNotStarted` while loading, `LoadComplete`, `UnloadNotStarted` while unloading,
    /// `UnloadComplete`.
    pub state: LoadState,
    pub high_priority: bool,
}

#[derive(Debug, Default)]
pub struct AssetManager {
    /// `LoadInfoList`, keyed by `GetHashValue32(name)`.
    pub loads: BTreeMap<u32, LoadInfo>,
    /// `AdditionalHulls`: a multimap by activity unique ID, equal keys in insertion order.
    pub additional: Vec<AdditionalHull>,
    /// `ActiveLoadInfo`.
    active_load: Option<u32>,
    /// `ActiveUnloadInfo`.
    active_unload: Option<u32>,
}

/// `cHullLoader::LoadHull`'s `StringnCopy(name, fileName, FileNameClamp)`.
#[must_use]
pub fn clamp_file_name(file_name: &str) -> &str {
    let mut end = file_name.len().min(FILE_NAME_CLAMP);
    while !file_name.is_char_boundary(end) {
        end -= 1;
    }
    &file_name[..end]
}

/// The hull ID `cPathedHullLoader::LoadHull` returns: `GetHashValue32` of
/// `"<base>\<clamped name>"`.
#[must_use]
pub fn hull_id(file_name: &str) -> u32 {
    hash_value32(&format!("{CONTENT_DIRECTORY}\\{}", clamp_file_name(file_name)))
}

/// `LoadChallengeAssets` (`0x82e122d0`): queue the challenge once, its hull
/// count from `NumberOfRequiredChallengeHulls` (`0x82e0ed68`): the
/// `RequiredChallengeHull` entries, or the challenge itself.
pub fn load_challenge_assets(world: &mut World, activity: usize, register_on_load: bool, high_priority: bool, asset_flags: u32) {
    let a = &world.activities.activities[activity];
    let (key, hash) = (a.key, hash_value32(&a.name));
    if world.assets.loads.contains_key(&hash) {
        return;
    }
    let num_hulls = match world.db.attribute(class::CHALLENGES, key, REQUIRED_CHALLENGE_HULL).map_or(0, |hulls| hulls.len()) {
        0 => 1,
        count => i32::try_from(count).unwrap_or(i32::MAX),
    };
    world.assets.loads.insert(hash, LoadInfo {
        activity,
        key,
        state: LoadState::LoadNotStarted,
        num_hulls,
        hull_index: -1,
        accepted: false,
        requested_unload: false,
        register_on_load,
        high_priority,
        asset_flags: asset_flags | 4,
        hull_ids: [0; 4],
    });
}

/// `IsAssetLoadComplete` (`0x82e0f900`): no load, or one past its notify.
#[must_use]
pub fn is_load_complete(world: &World, activity: usize) -> bool {
    let name = &world.activities.activities[activity].name;
    world
        .assets
        .loads
        .get(&hash_value32(name))
        .is_none_or(|l| matches!(l.state, LoadState::LoadCompleteNotify | LoadState::LoadComplete))
}

/// `IsAllUnloadComplete` (`0x82e0f400`): no additional hull unloading and no load left.
#[must_use]
pub fn is_all_unload_complete(world: &World) -> bool {
    !world.assets.additional.iter().any(|a| a.state == LoadState::UnloadNotStarted) && world.assets.loads.is_empty()
}

/// `IsLoadingChallengeHulls` (`0x82e0f470`): a load short of its hulls, or an additional hull
/// still loading.
#[must_use]
pub fn is_loading_challenge_hulls(world: &World) -> bool {
    world.assets.loads.values().any(|l| l.state < LoadState::AllChallengeHullsLoaded)
        || world.assets.additional.iter().any(|a| a.state == LoadState::LoadNotStarted)
}

/// `AreChallengeHullsLoaded` (`0x82e0fa60`): no load, or its hulls are in.
#[must_use]
pub fn are_hulls_loaded(world: &World, activity: usize) -> bool {
    let name = &world.activities.activities[activity].name;
    world.assets.loads.get(&hash_value32(name)).is_none_or(|l| l.state >= LoadState::AllChallengeHullsLoaded)
}

/// `RunMainActivity` (`0x82e12790`): a high-priority load, accepted.
pub fn run_main_activity(world: &mut World, activity: usize) {
    load_challenge_assets(world, activity, true, true, 0);
    let name = &world.activities.activities[activity].name;
    if let Some(load) = world.assets.loads.get_mut(&hash_value32(name)) {
        load.accepted = true;
    }
}

/// `RunGlobalActivity` (`0x82e12838`): outside an online game (`mGameTypeFlags` 8) a global
/// activity whose `ChallengeAssetLoadType` is not `NoLoad` loads, registered on load at normal
/// priority.
pub(crate) fn run_global_activity(world: &mut World, activity: usize) {
    if world.activities.game_load_flags & 8 == 8 {
        return;
    }
    let key = world.activities.activities[activity].key;
    if world.db.attribute(class::CHALLENGES, key, CHALLENGE_ASSET_LOAD_TYPE).and_then(|a| a.u32(0)).unwrap_or(0) != NO_LOAD {
        load_challenge_assets(world, activity, true, false, 0);
    }
}

/// `UnloadChallengeAssets` (`0x82e0f250`): a load that has not started ends
/// at once; one between its hulls and the accept starts unloading; any
/// other unloads when it reaches a stage that checks the request.
pub fn unload(world: &mut World, activity: usize) {
    let hash = hash_value32(&world.activities.activities[activity].name);
    let Some(load) = world.assets.loads.get_mut(&hash) else { return };
    load.requested_unload = true;
    match load.state {
        LoadState::LoadNotStarted => load.state = LoadState::UnloadComplete,
        LoadState::AllChallengeHullsLoaded | LoadState::WaitingForAccept => {
            load.state = LoadState::UnloadNotStarted;
            if world.assets.active_load == Some(hash) {
                world.assets.active_load = None;
            }
        }
        _ => {}
    }
}

/// `GenerateHullName` (`0x82e0ec30`): the `RequiredChallengeHull` entry, or
/// the challenge's name when it lists none.
fn generate_hull_name(world: &World, activity: usize, index: i32) -> String {
    let a = &world.activities.activities[activity];
    match world.db.attribute(class::CHALLENGES, a.key, REQUIRED_CHALLENGE_HULL).filter(|hulls| !hulls.is_empty()) {
        Some(hulls) => usize::try_from(index).ok().and_then(|i| hulls.text(i)).unwrap_or_default().to_owned(),
        None => a.name.clone(),
    }
}

/// The hulls `LoadChallengeAssets` loads for the challenge `key` named `name`: each
/// `RequiredChallengeHull` entry (`GenerateHullName` `0x82e0ec30`), or the name when it lists
/// none, through `ResolveHullName`.
#[must_use]
pub fn challenge_hull_names(db: &AttribDb, key: u64, name: &str) -> Vec<String> {
    let names = match db.attribute(class::CHALLENGES, key, REQUIRED_CHALLENGE_HULL).filter(|hulls| !hulls.is_empty()) {
        Some(hulls) => (0..hulls.len()).map(|index| resolve_hull_name(hulls.text(index).unwrap_or_default())).collect(),
        None => vec![resolve_hull_name(name)],
    };
    names.into_iter().filter(|name| !name.is_empty()).collect()
}

/// `Sk8ChallengeSystem::ResolveHullName` (`0x825a2ef0`): the name through
/// `FormatString` with no arguments, a leading `#` dropped, in 256 bytes.
fn resolve_hull_name(name: &str) -> String {
    let name = name.strip_prefix('#').unwrap_or(name);
    let mut end = name.len().min(255);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    name[..end].to_owned()
}

/// `StartChallengeHullLoad` (`0x82e0eec8`): load the current hull.
fn start_challenge_hull_load(world: &mut World, hash: u32) {
    let Some(load) = world.assets.loads.get(&hash) else { return };
    let (activity, index) = (load.activity, load.hull_index);
    let (register_on_load, high_priority, asset_flags) = (load.register_on_load, load.high_priority, load.asset_flags);
    let file_name = resolve_hull_name(&generate_hull_name(world, activity, index));
    let hull = hull_id(&file_name);
    if let Some(slot) = world.assets.loads.get_mut(&hash).and_then(|l| usize::try_from(index).ok().and_then(|i| l.hull_ids.get_mut(i))) {
        *slot = hull;
    }
    world.output(Output::LoadHull { hull, file_name, register_on_load, high_priority, asset_flags });
}

/// `StartChallengeHullUnload` (`0x82e0ebc8`): unload the highest hull still
/// held, stepping the index past unset ones.
fn start_challenge_hull_unload(world: &mut World, hash: u32) {
    let Some(load) = world.assets.loads.get_mut(&hash) else { return };
    if load.hull_index < 0 {
        return;
    }
    while load.hull(load.hull_index) == 0 {
        load.hull_index -= 1;
        if load.hull_index < 0 {
            return;
        }
    }
    let hull = load.hull(load.hull_index);
    world.output(Output::UnloadHull { hull });
}

/// `cHullLoadRequestCallback::OnComplete` (`0x82e0fed0`): the active load's
/// current hull moves it to its next hull or `AllChallengeHullsLoaded` (an
/// unload request ends it there); the active unload's current hull moves it
/// to the hull below or `AllChallengeHullsUnloaded`.
pub fn on_complete(world: &mut World, hull: u32) {
    if let Some(hash) = world.assets.active_load
        && let Some(load) = world.assets.loads.get_mut(&hash)
        && load.state == LoadState::WaitingForChallengeHull
        && load.hull(load.hull_index) == hull
    {
        if load.requested_unload {
            load.num_hulls = load.hull_index;
        }
        if load.hull_index + 1 >= load.num_hulls {
            load.state = LoadState::AllChallengeHullsLoaded;
        } else {
            load.hull_index += 1;
            start_challenge_hull_load(world, hash);
        }
    }
    if let Some(hash) = world.assets.active_unload
        && let Some(load) = world.assets.loads.get_mut(&hash)
        && load.state == LoadState::WaitingForChallengeHullUnload
        && load.hull(load.hull_index) == hull
    {
        if let Some(slot) = usize::try_from(load.hull_index).ok().and_then(|i| load.hull_ids.get_mut(i)) {
            *slot = 0;
        }
        load.hull_index -= 1;
        start_challenge_hull_unload(world, hash);
        if let Some(load) = world.assets.loads.get_mut(&hash) {
            load.state = if load.hull_index >= 0 { LoadState::WaitingForChallengeHullUnload } else { LoadState::AllChallengeHullsUnloaded };
        }
    }
    // The first additional hull with the ID: a load completes with cMsgAdditionalHullLoadComplete,
    // an unload with cMsgAdditionalHullUnLoadComplete.
    let Some(entry) = world.assets.additional.iter_mut().find(|a| a.hull == hull) else { return };
    let message = match entry.state {
        LoadState::LoadNotStarted => {
            entry.state = LoadState::LoadComplete;
            "cMsgAdditionalHullLoadComplete"
        }
        LoadState::UnloadNotStarted => {
            entry.state = LoadState::UnloadComplete;
            "cMsgAdditionalHullUnLoadComplete"
        }
        _ => return,
    };
    let key = entry.activity_key;
    world.send(Event::Challenge(Message::new(message, vec![("ActivityKey", Field::Key(key))])));
}

/// `LoadAdditionalChallengeHull(activity, hull, highPriority)` (`0x82e128f8`) with the
/// activity's `DmoTreatAsStatic`, then `LoadAdditionalChallengeHull(activityKey, hull,
/// treatDMOAsStatic, highPriority)` (`0x82e12408`): a hull this activity already holds for the
/// `tRequiredChallengeHull` loads again only once unloaded (its entry reused); the resolved name
/// loads registered on load (asset flag 0x10 for a static-DMO challenge) and the entry waits
/// for `OnComplete`.
pub fn load_additional_challenge_hull(world: &mut World, activity: usize, hull_pointer: Location, high_priority: bool) {
    let activity_key = world.activities.activities[activity].key;
    let treat_dmo_as_static =
        world.db.attribute(class::CHALLENGES, activity_key, DMO_TREAT_AS_STATIC).and_then(|a| a.value_bytes(0)).is_some_and(|b| b.first() == Some(&1));
    let existing = world.assets.additional.iter().position(|a| a.activity_key == activity_key && a.hull_pointer == hull_pointer);
    if existing.is_some_and(|i| world.assets.additional[i].state != LoadState::UnloadComplete) {
        return;
    }
    let file_name = resolve_hull_name(world.db.text_at(hull_pointer).unwrap_or_default());
    let hull = hull_id(&file_name);
    let asset_flags = if treat_dmo_as_static { 0x10 } else { 0 };
    world.output(Output::LoadHull { hull, file_name, register_on_load: true, high_priority, asset_flags });
    match existing {
        Some(i) => {
            let entry = &mut world.assets.additional[i];
            entry.hull = hull;
            entry.high_priority = high_priority;
            entry.state = LoadState::LoadNotStarted;
        }
        None => {
            let at = world.assets.additional.iter().position(|a| a.activity_key > activity_key).unwrap_or(world.assets.additional.len());
            world.assets.additional.insert(at, AdditionalHull {
                activity_key,
                hull_pointer,
                hull,
                state: LoadState::LoadNotStarted,
                high_priority,
            });
        }
    }
}

/// `UnloadAdditionalChallengeHull(activityKey, hull)` (`0x82e10110`): the activity's entry for the
/// `tRequiredChallengeHull` unloads and waits for `OnComplete`.
pub fn unload_additional_challenge_hull(world: &mut World, activity_key: u64, hull_pointer: Location) {
    let Some(entry) = world.assets.additional.iter_mut().find(|a| a.activity_key == activity_key && a.hull_pointer == hull_pointer) else {
        return;
    };
    entry.state = LoadState::UnloadNotStarted;
    let hull = entry.hull;
    world.output(Output::UnloadHull { hull });
}

/// `ShutdownActivity` (`0x82e10c98`): each of the activity's additional hulls unloads without a
/// callback and its entry goes; outside an online game (`mGameTypeFlags` 8) the activity's
/// challenge assets unload (`UnloadChallengeAssets`).
pub fn shutdown_activity(world: &mut World, activity: usize) {
    let activity_key = world.activities.activities[activity].key;
    let hulls: Vec<u32> = world.assets.additional.iter().filter(|a| a.activity_key == activity_key).map(|a| a.hull).collect();
    world.assets.additional.retain(|a| a.activity_key != activity_key);
    for hull in hulls {
        world.output(Output::UnloadHull { hull });
    }
    if world.activities.game_load_flags & 8 == 0 {
        unload(world, activity);
    }
}

/// `LoadSkaterModels` (`0x82e0e990`): an AI challenge whose competitors are not the demo
/// sponsor's selects its characters, a jam, mega ramp or street skating challenge leaving
/// their models to `CAIAssetOverride`.
fn load_skater_models(world: &mut World, activity: usize) {
    if !ai::is_activity_an_ai_challenge(world, activity) {
        return;
    }
    let local = world.activities.activities[activity].local_data;
    let sponsor = local.and_then(|(class, collection)| world.db.attribute(class, collection, USE_DEMO_SPONSOR_COMPETITORS));
    if sponsor.and_then(|a| a.bool(0)).unwrap_or(false) {
        return;
    }
    let kind = super::signup::challenge_type(world, activity);
    let no_asset_load = matches!(kind, JAM | MEGA_RAMP | STREET_SKATING)
        && world
            .db
            .attribute(class::CHALLENGES, world.activities.activities[activity].key, CAI_ASSET_OVERRIDE)
            .and_then(|a| a.bool(0))
            .unwrap_or(false);
    if let Err(error) = ai::select_characters(world, activity, no_asset_load) {
        world.output(Output::ScriptError { context: "cChallengeAssetManager::LoadSkaterModels".to_owned(), error });
    }
}

/// `LoadAIPaths` (`0x82e0eaf8`): an AI challenge loads its static paths (binary loading is on),
/// any other is done.
fn load_ai_paths(world: &mut World, activity: usize) -> LoadState {
    if !ai::is_activity_an_ai_challenge(world, activity) {
        return LoadState::LoadCompleteNotify;
    }
    if let Err(error) = ai::load_paths(world, activity) {
        world.output(Output::ScriptError { context: "cChallengeAssetManager::LoadAIPaths".to_owned(), error });
    }
    LoadState::WaitingForAiPaths
}

/// `cChallengeAssetManager::Update` (`0x82e0fb38`): one step of every load;
/// the host's hull loader update follows it.
pub fn update(world: &mut World) {
    let hashes: Vec<u32> = world.assets.loads.keys().copied().collect();
    for hash in hashes {
        let Some(load) = world.assets.loads.get(&hash).cloned() else { continue };
        let next = match load.state {
            LoadState::LoadNotStarted => {
                if world.assets.active_load.is_some() {
                    continue;
                }
                world.assets.active_load = Some(hash);
                if let Some(l) = world.assets.loads.get_mut(&hash) {
                    l.hull_index += 1;
                    l.state = LoadState::WaitingForChallengeHull;
                }
                start_challenge_hull_load(world, hash);
                continue;
            }
            LoadState::AllChallengeHullsLoaded => {
                if world.assets.active_load == Some(hash) {
                    world.assets.active_load = None;
                }
                if load.requested_unload {
                    LoadState::UnloadNotStarted
                } else {
                    let load_type =
                        world.db.attribute(class::CHALLENGES, load.key, CHALLENGE_ASSET_LOAD_TYPE).and_then(|a| a.u32(0)).unwrap_or(0);
                    if load_type == UNINTERRUPTED_LOAD || world.game_mode_flags == 8 || world.game_mode_flags == 16 {
                        if let Some(l) = world.assets.loads.get_mut(&hash) {
                            l.accepted = true;
                        }
                    }
                    #[expect(clippy::cast_precision_loss, reason = "(float)_u64tod")]
                    let key = load.key as f64 as f32;
                    world.send(Event::Challenge(Message::new("cMsgChallengeHullsLoaded", vec![("ActivityKey", Field::Number(key))])));
                    LoadState::WaitingForAccept
                }
            }
            LoadState::WaitingForAccept if load.accepted => {
                load_skater_models(world, load.activity);
                LoadState::WaitingForSkaterModels
            }
            LoadState::WaitingForSkaterModels => {
                if !ai::are_all_assets_loaded(world) {
                    continue;
                }
                load_ai_paths(world, load.activity)
            }
            LoadState::WaitingForAiPaths => {
                if !ai::are_static_paths_loaded(world) {
                    continue;
                }
                LoadState::LoadCompleteNotify
            }
            LoadState::LoadCompleteNotify => {
                if world.assets.active_load == Some(hash) {
                    world.assets.active_load = None;
                }
                #[expect(clippy::cast_precision_loss, reason = "(float)_u64tod")]
                let key = load.key as f64 as f32;
                world.send(Event::Challenge(Message::new("cMsgChallengeAssetLoadComplete", vec![("ActivityKey", Field::Number(key))])));
                objectives::challenge_asset_load_complete(world, load.key);
                LoadState::LoadComplete
            }
            LoadState::LoadComplete => {
                if world.assets.active_load == Some(hash) {
                    world.assets.active_load = None;
                }
                if !load.requested_unload {
                    continue;
                }
                LoadState::UnloadNotStarted
            }
            LoadState::UnloadNotStarted => {
                if world.assets.active_unload.is_some() {
                    continue;
                }
                world.assets.active_unload = Some(hash);
                start_challenge_hull_unload(world, hash);
                match world.assets.loads.get(&hash) {
                    Some(l) if l.hull_index < 0 => LoadState::AllChallengeHullsUnloaded,
                    _ => LoadState::WaitingForChallengeHullUnload,
                }
            }
            LoadState::AllChallengeHullsUnloaded => LoadState::UnloadComplete,
            LoadState::UnloadComplete => {
                world.assets.active_unload = None;
                world.assets.loads.remove(&hash);
                continue;
            }
            LoadState::WaitingForAccept | LoadState::WaitingForChallengeHull | LoadState::WaitingForChallengeHullUnload => continue,
        };
        if let Some(l) = world.assets.loads.get_mut(&hash) {
            l.state = next;
        }
    }
}

/// The calling activity and the `tRequiredChallengeHull` argument of the additional-hull
/// natives (`lua_touserdata(1)`, which the native asserts is light userdata).
fn additional_hull_args(lua: &Lua<World>, world: &World, call: &NativeCall, native: &str) -> LuaResult<(usize, Location)> {
    let activity = world.closure.ok_or_else(|| lua.error(format!("ChallengeAsset.{native}: no calling activity")))?;
    let hull = attrib_location(&world.pointers, call.arg(0))
        .ok_or_else(|| lua.error(format!("ChallengeAsset.{native}: expected a tRequiredChallengeHull")))?;
    Ok((activity, hull))
}

/// `ChallengeAsset.LoadAdditionalChallengeHull(tRequiredChallengeHull)` (`0x82e12978`): at high
/// priority.
fn load_additional_challenge_hull_native(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let (activity, hull) = additional_hull_args(lua, world, call, "LoadAdditionalChallengeHull")?;
    load_additional_challenge_hull(world, activity, hull, true);
    Ok(Vec::new())
}

/// `ChallengeAsset.UnloadAdditionalChallengeHull(tRequiredChallengeHull)` (`0x82e10918`).
fn unload_additional_challenge_hull_native(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let (activity, hull) = additional_hull_args(lua, world, call, "UnloadAdditionalChallengeHull")?;
    let activity_key = world.activities.activities[activity].key;
    unload_additional_challenge_hull(world, activity_key, hull);
    Ok(Vec::new())
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("ChallengeAsset", "LoadAdditionalChallengeHull", load_additional_challenge_hull_native),
        ("ChallengeAsset", "UnloadAdditionalChallengeHull", unload_additional_challenge_hull_native),
    ]
}
