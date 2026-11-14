//! `Sk8::Challenge::cSignPostManager` (`packages/sk8/signpost`): career
//! progression state machines, challenge availability/completion, and the
//! progression message system. See `docs/career/runtime.md`.

use super::{
    attrib_db::AttribDb,
    binder,
    keys::{class, hash32, hash64, hash_value32},
    lua::{Lua, LuaResult, NativeCall, Value, stdlib::check_string},
    signpost_progress::{self, Progress, blog_type},
    system::{Event, Output, Skate101View, World},
};
use std::collections::{BTreeMap, HashMap};

/// Handler message types in `RunStateMachine` order (`eSignPostScriptTypes`).
pub const SCRIPT_TYPES: [&str; 9] = [
    "cMsgStateEnter",
    "cMsgNISChallengeNotify",
    "cMsgHUDComplete",
    "cMsgHUDContinue",
    "cMsgHUDNo",
    "cMsgHUDYes",
    "cMsgSpeechEnded",
    "cMsgTimerExpired",
    "cMsgFMVFinishedPlaying",
];
const STATE_ENTER: usize = 0;

mod field {
    pub const NAME: u64 = 0x942a_b8ae_e4b4_14ed;
    pub const TITLE: u64 = 0xe2a5_0788_b6c2_0c92;
    pub const STARTING: u64 = 0xbdcb_66ee_70f1_ced8;
    pub const STATES: u64 = 0x8efb_bc13_e812_6ed3;
    pub const INITIAL_STATE: u64 = 0xa1cd_8e48_3c70_678c;
    pub const STATE_HANDLERS: u64 = 0xdeb2_096f_e58b_1805;
    /// `tLuaScript` (layout +12): the handler's Lua source.
    pub const HANDLER_SOURCE: u64 = 0x6391_acea_b5a0_3cd2;
    /// `progression_challenges` byte at layout +40: not a progression challenge.
    pub const CHALLENGE_EXCLUDED: u64 = 0x9990_9de7_1b39_bdf4;
    /// `progression_challenges`/`progression_messages`: machines to trigger.
    pub const TRIGGERS: u64 = 0xd102_e741_1533_faa8;
    pub const MESSAGE_TYPE_ID: u64 = 0x1cc1_319b_adba_0454;
    pub const PROPERTY_DEFINITIONS: u64 = 0x69bd_c77c_3309_e1d1;
    pub const PROPERTIES: u64 = 0xdeb4_4f85_2063_0628;
    pub const SETTING_DEFINITIONS: u64 = 0x6bb7_3eeb_05a9_0592;
    pub const SETTINGS: u64 = 0xc113_f817_3b56_267b;
    /// Definition/value fields shared by properties and settings.
    pub const DEFAULT_STRING: u64 = 0xf3b2_94ca_1fee_c1ae;
    pub const DEFAULT_INT: u64 = 0x1704_df87_cc24_1fff;
    pub const DEFAULT_BOOL: u64 = 0x8666_37cd_fa26_077c;
    pub const VALUE_TYPE: u64 = 0x3c32_f163_e200_08cd;
    pub const VALUE_STRING: u64 = 0x54a3_90b7_ed8d_cf1d;
    pub const VALUE_INT: u64 = 0x0e9c_e6a6_267b_a806;
    pub const VALUE_BOOL: u64 = 0x9530_0b5d_da3a_0a01;
}

/// Setting definitions (`MessageMan` keys); property definitions.
const SETTING_DEFINITION_CLASS: u64 = 0x0af5_806a_faa5_083e;

/// `challenge_local_data.IgnoreNewChallengeCount`.
const IGNORE_NEW_CHALLENGE_COUNT: u64 = 0x133a_ad27_f053_43f8;

#[derive(Debug)]
pub(crate) struct MachineState {
    key: u64,
    name: String,
    /// Registry references to the compiled handler functions.
    handlers: [Option<usize>; 9],
}

/// `cSignPostStateMachine` (`0x878` bytes).
#[derive(Debug)]
pub(crate) struct Machine {
    key: u64,
    states: BTreeMap<u64, MachineState>,
    current: Option<u64>,
    next: Option<u64>,
    terminate: bool,
    autosave: bool,
    running: bool,
    running_handler: bool,
    change_count: i32,
}

/// `cSignPostChallengeHelper`: title/name → `progression_challenges` key.
#[derive(Default)]
struct ChallengeHelper {
    titles: HashMap<u32, u64>,
    names: HashMap<u32, u64>,
}

/// `cSignPostMessageMan`: per-setting values keyed by definition key.
#[derive(Default)]
struct MessageMan {
    names: HashMap<u32, u64>,
    data: BTreeMap<u64, u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompletedChallenge {
    pub order: i32,
    /// `intPhotoCaption`.
    pub photo_caption: i32,
}

/// `Sk8::Progression::tCompletionResults` (`mCompletionResults`): what the
/// unlock overlay (`FrontEndState_Unlock`) shows. Built once by the
/// `cSignPostManager` constructor; the overlay's `OnLeave` clears it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionResults {
    pub num_phone_numbers_unlocked: i32,
    pub num_characters_unlocked: i32,
    pub award_unlocked: bool,
    /// `eAwards`; `eAwards_Total` (15) when none.
    pub award: i32,
    pub num_paths_unlocked: i32,
    pub coverage_unlocked: bool,
    /// `eCompletionMedal`; -1 is `eCompletionMedal_None`.
    pub medal_type: i32,
    pub unlocked_challenges: Vec<u64>,
    pub unlocked_characters: Vec<u64>,
    pub unlocked_phone_numbers: Vec<u64>,
    pub unlocked_locations: Vec<u64>,
    pub unlocked_property: Vec<u64>,
    pub unlocked_cac_items: Vec<u64>,
    pub unlocked_sponsor_cac_items: Vec<u64>,
    pub sponsor_me_created: bool,
    pub sponsor_me_brand: u64,
    pub photo_article: u64,
}

impl Default for CompletionResults {
    /// The constructor (`0x825cb070`).
    fn default() -> Self {
        Self {
            num_phone_numbers_unlocked: 0,
            num_characters_unlocked: 0,
            award_unlocked: false,
            award: 15,
            num_paths_unlocked: 0,
            coverage_unlocked: false,
            medal_type: -1,
            unlocked_challenges: Vec::new(),
            unlocked_characters: Vec::new(),
            unlocked_phone_numbers: Vec::new(),
            unlocked_locations: Vec::new(),
            unlocked_property: Vec::new(),
            unlocked_cac_items: Vec::new(),
            unlocked_sponsor_cac_items: Vec::new(),
            sponsor_me_created: false,
            sponsor_me_brand: 0,
            photo_article: 0,
        }
    }
}

impl CompletionResults {
    /// `FrontEndState_Unlock::OnLeave` (`0x82652990`): every count, flag and
    /// list cleared; `photoArticle` is kept.
    pub fn clear(&mut self) {
        let photo_article = self.photo_article;
        *self = Self { photo_article, ..Self::default() };
    }
}

/// The scenes `HandleCaughtMessage` (`0x825de4e0`) holds back 12 updates.
const DELAYED_NIS: [&str; 4] = ["04_Arrival_of_Mikey", "18_mag_cover", "17_Thrasher_Cover", "22_SlappyEnder"];

pub struct SignPost {
    pub(crate) game_type_flags: u32,
    initialized: bool,
    during_init: bool,
    state_machine_override: i32,
    helper: ChallengeHelper,
    messages: MessageMan,
    /// `mAvailableChallengeData`: signpost key → "new" flag.
    pub available: BTreeMap<u64, bool>,
    /// `mCompletedChallengeData`.
    pub completed: BTreeMap<u64, CompletedChallenge>,
    progression_challenges: Vec<u64>,
    machines: BTreeMap<u64, Machine>,
    triggered: Vec<u64>,
    /// `SetClosureData`: the machine whose handler is running.
    current_machine: Option<u64>,
    activated_machine_count: i32,
    /// `mChallengeResults`: challenge (neutrino) key → result (0 won, 1/2 lost).
    pub results: BTreeMap<u64, i32>,
    /// `mStateMachineTriggered`.
    state_machine_triggered: bool,
    /// `intCompletedChallengeCount`.
    completed_count: i32,
    /// Groups, point pools, rankings, BLOG, news and the saved orientation.
    pub progress: Progress,
    /// `mCompletionResults`.
    pub completion: CompletionResults,
    /// `cSignPostUtil::mNISWaitTimerTicks` and `mCachedNISMessageKey`: a
    /// delayed NIS message counting down its updates.
    nis_wait_ticks: i32,
    cached_nis_message: Option<u64>,
    /// The player's bet on a betting challenge.
    pub(crate) betting: signpost_progress::Betting,
}

fn text(db: &AttribDb, class: u64, collection: u64, key: u64) -> Option<String> {
    db.attribute(class, collection, key)?.text(0).map(str::to_owned)
}

fn int(db: &AttribDb, class: u64, collection: u64, key: u64) -> Option<i32> {
    db.attribute(class, collection, key)?.i32(0)
}

fn flag(db: &AttribDb, class: u64, collection: u64, key: u64) -> Option<bool> {
    db.attribute(class, collection, key)?.bool(0)
}

fn refs(db: &AttribDb, class: u64, collection: u64, key: u64) -> Vec<(u64, u64)> {
    db.attribute(class, collection, key)
        .map(|a| (0..a.len()).filter_map(|i| a.ref_spec(i)).collect())
        .unwrap_or_default()
}

impl SignPost {
    pub(crate) fn new(db: &AttribDb) -> Self {
        let mut helper = ChallengeHelper::default();
        // cSignPostChallengeHelper::Init 0x825d81f8.
        for &key in db.collections(class::PROGRESSION_CHALLENGES) {
            if let Some(title) = text(db, class::PROGRESSION_CHALLENGES, key, field::TITLE) {
                helper.titles.insert(hash_value32(&title), key);
            }
            if let Some(name) = text(db, class::PROGRESSION_CHALLENGES, key, field::NAME) {
                helper.names.insert(hash_value32(&name), key);
            }
        }
        Self {
            game_type_flags: 0,
            initialized: false,
            during_init: false,
            state_machine_override: -1,
            helper,
            messages: MessageMan::default(),
            available: BTreeMap::new(),
            completed: BTreeMap::new(),
            progression_challenges: Vec::new(),
            machines: BTreeMap::new(),
            triggered: Vec::new(),
            current_machine: None,
            activated_machine_count: 0,
            results: BTreeMap::new(),
            state_machine_triggered: false,
            completed_count: 0,
            progress: Progress::default(),
            completion: CompletionResults::default(),
            nis_wait_ticks: 0,
            cached_nis_message: None,
            betting: signpost_progress::Betting::default(),
        }
    }

    pub(crate) fn running_machine_names(&self, db: &AttribDb) -> Vec<String> {
        self.machines
            .keys()
            .map(|&k| text(db, class::PROGRESSION_STATE_MACHINES, k, field::NAME).unwrap_or_else(|| format!("{k:016x}")))
            .collect()
    }

    /// `GetChallengeKeyFromTitle` (`0x825cc880`).
    pub(crate) fn key_from_title(&self, title: &str) -> Option<u64> {
        self.helper.titles.get(&hash_value32(title)).copied()
    }

    /// `ConvertSignpostKeyToChallengeKey` (`0x825c4910`): `StringToKey(Name)`
    /// when that `challenges` collection exists.
    fn challenge_key(db: &AttribDb, signpost: u64) -> Option<u64> {
        let name = text(db, class::PROGRESSION_CHALLENGES, signpost, field::NAME)?;
        let key = hash64(&name);
        db.has_collection(class::CHALLENGES, key).then_some(key)
    }

    #[must_use]
    pub fn is_challenge_active(&self, signpost: u64) -> bool {
        self.available.contains_key(&signpost)
    }
    #[must_use]
    pub fn is_challenge_complete(&self, signpost: u64) -> bool {
        self.completed.contains_key(&signpost)
    }

    /// `mProgressionChallenges`.
    pub(crate) fn progression_challenge_keys(&self) -> &[u64] {
        &self.progression_challenges
    }
    pub(crate) const fn during_init(&self) -> bool {
        self.during_init
    }
    pub(crate) const fn set_during_init(&mut self, during_init: bool) {
        self.during_init = during_init;
    }

    /// `GetCompletedChallengeInOrder(mnSize)` (`0x825c7a48`): the completed
    /// challenge whose order is the completed-challenge count.
    #[must_use]
    pub fn last_completed_challenge(&self) -> Option<u64> {
        let count = i32::try_from(self.completed.len()).ok().filter(|&c| c > 0)?;
        self.completed.iter().find(|(_, c)| c.order == count).map(|(&k, _)| k)
    }

    /// `cSignPostMessageMan`'s stored word for a setting name.
    pub(crate) fn message_setting_value(&self, name: &str) -> Option<u32> {
        let man = &self.messages;
        man.names.get(&hash_value32(name)).and_then(|k| man.data.get(k)).copied()
    }
    /// `MessageDataMap.find(GetMessageKey(name))` (`0x825cb150`): a present
    /// setting's stored word replaced.
    pub(crate) fn set_message_setting_value(&mut self, name: &str, value: u32) {
        let man = &mut self.messages;
        if let Some(word) = man.names.get(&hash_value32(name)).and_then(|k| man.data.get_mut(k)) {
            *word = value;
        }
    }
    /// `MessageDataMap`.
    pub(crate) const fn message_data(&self) -> &BTreeMap<u64, u32> {
        &self.messages.data
    }
    pub(crate) fn set_message_data(&mut self, key: u64, value: u32) {
        self.messages.data.insert(key, value);
    }
}

/// `cSignPostMessageMan::Init` (`0x825d84e0`): every setting definition with
/// its authored default (0 bool, 1 int, 2 string hash).
fn init_message_settings(world: &mut World) {
    let db = &world.db;
    let man = &mut world.signpost.messages;
    for &key in db.collections(SETTING_DEFINITION_CLASS) {
        let Some(name) = text(db, SETTING_DEFINITION_CLASS, key, field::NAME) else { continue };
        man.names.insert(hash_value32(&name), key);
        let value = match int(db, SETTING_DEFINITION_CLASS, key, field::VALUE_TYPE).unwrap_or(0) {
            0 => u32::from(flag(db, SETTING_DEFINITION_CLASS, key, field::DEFAULT_BOOL).unwrap_or(false)),
            1 => int(db, SETTING_DEFINITION_CLASS, key, field::DEFAULT_INT).unwrap_or(0).cast_unsigned(),
            2 => text(db, SETTING_DEFINITION_CLASS, key, field::DEFAULT_STRING).map_or(0, |s| hash_value32(&s)),
            _ => 0,
        };
        man.data.insert(key, value);
    }
}

/// `cSignPostManager::Init` (`0x825e3b98`): message settings, the counters,
/// `InitChallengeProgressionData`, `InitGroupProgressionData`,
/// `InitPointPoolData` and `InitRankingGroupData`. The sponsor and pinch
/// point managers are not ported.
pub(crate) fn init(world: &mut World) {
    world.signpost.initialized = true;
    world.signpost.during_init = true;
    init_message_settings(world);
    world.signpost.progress = Progress::default();
    world.signpost.completed_count = 0;
    // InitChallengeProgressionData 0x825e28a8: included challenges, and the
    // ones flagged Starting are activated (not new, no evaluation).
    let keys: Vec<u64> = world.db.collections(class::PROGRESSION_CHALLENGES).to_vec();
    for key in keys {
        if flag(&world.db, class::PROGRESSION_CHALLENGES, key, field::CHALLENGE_EXCLUDED).unwrap_or(false) {
            continue;
        }
        world.signpost.progression_challenges.push(key);
        if flag(&world.db, class::PROGRESSION_CHALLENGES, key, field::STARTING) == Some(true) {
            activate_challenge(world, key, false, false, false);
        }
    }
    signpost_progress::init_challenge_save_data(world);
    signpost_progress::init_groups(world);
    signpost_progress::init_point_pools(world);
    signpost_progress::init_rankings(world);
    super::map_icons::check_pinch_points(world);
    world.signpost.during_init = false;
}

/// `LoadProgressionData`'s `DeInit` (`0x825d2398`) and `Init`: every map and
/// message setting cleared, then initialised afresh.
pub(crate) fn reinit(world: &mut World) {
    let sp = &mut world.signpost;
    sp.available.clear();
    sp.completed.clear();
    sp.progression_challenges.clear();
    sp.machines.clear();
    sp.messages.names.clear();
    sp.messages.data.clear();
    sp.initialized = false;
    init(world);
}

/// `EvaluateActivities` (`0x825e0030`): `CheckPinchPoints` and the activity
/// manager's `ReevaluatePrereqActivities`.
fn evaluate_activities(world: &mut World) {
    super::map_icons::check_pinch_points(world);
    super::activity::reevaluate_prereq_activities(world);
}

/// `intActivatedStateMachineCount`: set by the sign-up hooks to the machines
/// `CheckStateTriggers` queued, lowered as machines terminate.
pub(crate) const fn activated_machine_count(world: &World) -> i32 {
    world.signpost.activated_machine_count
}
pub(crate) const fn set_activated_machine_count(world: &mut World, count: i32) {
    world.signpost.activated_machine_count = count;
}

/// `Sk8ChallengeSystem::IsChallengeActive` (`0x825a2c78`), the activities'
/// progression interface: every challenge without progression, else the
/// signpost's availability of the activity's challenge.
pub(crate) fn is_neutrino_challenge_active(world: &World, challenge: u64) -> bool {
    if !world.enable_progression && world.game_mode_flags != 2 {
        return true;
    }
    signpost_key_for_challenge(world, challenge).is_some_and(|k| world.signpost.is_challenge_active(k))
}

/// `cSignPostManager::ActivateChallenge` (`0x825e02c8`), signpost-key form:
/// the challenge becomes available when its activity exists (new unless the
/// activity's `IgnoreNewChallengeCount`), outside the initialisation it joins
/// the completion results (`AddCompletedChallengeToResults`), and its
/// activation BLOG entry is added.
pub(crate) fn activate_challenge(world: &mut World, signpost: u64, evaluate: bool, force: bool, is_new: bool) {
    if signpost == 0 {
        return;
    }
    let sp = &world.signpost;
    if (sp.is_challenge_complete(signpost) && !force) || sp.is_challenge_active(signpost) {
        return;
    }
    let challenge = SignPost::challenge_key(&world.db, signpost);
    let activity = challenge.and_then(|k| world.activities.find(k));
    if let Some(activity) = activity {
        let ignore = world.activities.activities[activity]
            .local_data
            .and_then(|(c, k)| world.db.attribute(c, k, IGNORE_NEW_CHALLENGE_COUNT))
            .and_then(|a| a.bool(0))
            .unwrap_or(false);
        world.signpost.available.insert(signpost, is_new && !ignore);
    }
    if !world.signpost.during_init
        && let Some(challenge) = challenge
    {
        add_completed_challenge_to_results(world, challenge);
    }
    signpost_progress::add_challenge_blog(world, signpost, blog_type::CHALLENGE_ACTIVATE);
    if evaluate {
        evaluate_activities(world);
    }
}

/// `AddCompletedChallengeToResults(neutrinoKey)` (`0x825d6fe0`): the
/// challenge's activity (`cActivityManager::GetChallenge`) puts its key once
/// into the completion results, `unlockedProperty` for `ChallengeType` 45
/// and `unlockedChallenges` otherwise.
fn add_completed_challenge_to_results(world: &mut World, challenge: u64) {
    let Some(activity) = world.activities.find(challenge) else { return };
    let property = super::signup::challenge_type(world, activity) == PROPERTY_CHALLENGE_TYPE;
    let completion = &mut world.signpost.completion;
    let list = if property { &mut completion.unlocked_property } else { &mut completion.unlocked_challenges };
    if !list.contains(&challenge) {
        list.push(challenge);
    }
}

/// The `ChallengeType` of property challenges.
const PROPERTY_CHALLENGE_TYPE: i32 = 45;

/// `cSignPostUtil::GetListKeyFromName` (`0x825c8c30`): the first
/// `progression_lists` collection whose name (layout word 0) matches, or 0.
fn list_key_from_name(db: &AttribDb, name: &str) -> u64 {
    db.collections(class::PROGRESSION_LISTS)
        .iter()
        .copied()
        .find(|&k| db.layout_text(class::PROGRESSION_LISTS, k, 0) == Some(name))
        .unwrap_or(0)
}

/// `cSignPostUtil::IsChallengeInList(listKey, signpostKey)` (`0x825c6068`):
/// one of the list's `Challenges` refers to the progression challenge.
fn is_challenge_in_list(db: &AttribDb, list: u64, signpost: u64) -> bool {
    db.attribute(class::PROGRESSION_LISTS, list, LIST_CHALLENGES)
        .is_some_and(|a| (0..a.len()).any(|i| a.ref_spec(i).is_some_and(|(_, k)| k == signpost)))
}

/// `progression_lists.Challenges` (`RefSpec[]`).
const LIST_CHALLENGES: u64 = 0xec1c_213b_bdbf_1a68;

/// `cSignPostUtil::CheckChallengeResults(listName)` (`0x825d0968`): a
/// challenge the completion results unlocked
/// (`ConvertChallengeKeyToSignpostKey`) is in the named list.
pub(crate) fn check_challenge_results(world: &World, list_name: &str) -> bool {
    let list = list_key_from_name(&world.db, list_name);
    world.signpost.completion.unlocked_challenges.iter().any(|&challenge| {
        let signpost = signpost_key_for_challenge(world, challenge).unwrap_or(0);
        is_challenge_in_list(&world.db, list, signpost)
    })
}

/// `DeactivateChallenge` (`0x825e05b8`): only an incomplete challenge.
pub(crate) fn deactivate_challenge(world: &mut World, signpost: u64) {
    if signpost & 0xffff_ffff == 0 || world.signpost.is_challenge_complete(signpost) {
        return;
    }
    if world.signpost.available.remove(&signpost).is_some() {
        evaluate_activities(world);
    }
}

/// `QueueTriggeredStateMachine` (`0x825d6c20`).
pub(crate) fn queue_triggered_state_machine(world: &mut World, machine: u64) {
    if !world.signpost.during_init && world.signpost.game_type_flags == 4 {
        world.signpost.state_machine_triggered = true;
        world.signpost.triggered.push(machine);
    }
}

/// `StartGameSession` (`0x825e4b48`): queue every `Starting` machine.
pub(crate) fn start_game_session(world: &mut World) {
    let machines: Vec<u64> = world.db.collections(class::PROGRESSION_STATE_MACHINES).to_vec();
    for key in machines {
        if flag(&world.db, class::PROGRESSION_STATE_MACHINES, key, field::STARTING) == Some(true)
            && world.signpost.state_machine_override == -1
        {
            queue_triggered_state_machine(world, key);
        }
    }
}

/// `cSignPostManager::Update` (`0x825e2ce8`): the delayed NIS countdown,
/// then, while not loading, one triggered machine per frame. Map challenge loading keeps
/// the retry-triggered machines queued through `WorldPrepareState`.
pub(crate) fn update(lua: &mut Lua<World>, world: &mut World, loading: bool) {
    util_update(world);
    if loading || world.signpost.triggered.is_empty() {
        return;
    }
    let key = world.signpost.triggered.remove(0);
    run_state_machine(lua, world, key);
}

/// `cSignPostUtil::Update` (`0x825c61d8`): at the end of its 12 updates the
/// delayed message's scene runs (`RunBasicNIS(scene, "Transition Type",
/// false, true)`).
fn util_update(world: &mut World) {
    if world.signpost.nis_wait_ticks <= 0 {
        return;
    }
    world.signpost.nis_wait_ticks -= 1;
    if world.signpost.nis_wait_ticks == 0
        && let Some(message) = world.signpost.cached_nis_message.take()
    {
        let db = std::sync::Arc::clone(&world.db);
        let name = text(&db, class::PROGRESSION_MESSAGES, message, field::NAME).unwrap_or_default();
        let scene = message_string_property(&db, message, "Scene Name").unwrap_or_default();
        let transition = message_int_property(&db, message, "Transition Type").unwrap_or(0);
        world.output(Output::PlayNis { message: name, scene, transition });
    }
}

fn script_error(world: &mut World, context: String, error: &dyn std::fmt::Display) {
    eprintln!("career script error in {context}: {error}");
    world.output(Output::ScriptError { context, error: error.to_string() });
}

/// `cSignPostScriptHandler::Attach` (`0x825c6320`): compile the handler's
/// `tLuaScript` source wrapped as `function %s() \n%s\n end`
/// (`GenerateAttachedBuffer` `0x825c4750`) with `luaL_loadstring`, run the
/// chunk, and capture the function. The native names it from an
/// uninitialised read (docs/career/runtime.md); the handler key keeps each
/// handler distinct.
fn load_handler(lua: &mut Lua<World>, world: &mut World, handler: u64) -> Option<usize> {
    let db = std::sync::Arc::clone(&world.db);
    let source = db.attribute(class::PROGRESSION_HANDLERS, handler, field::HANDLER_SOURCE)?.text(0)?;
    if source.is_empty() {
        return None;
    }
    let function_name = format!("h_{handler:016X}");
    let wrapped = format!("function {function_name}() \n{source}\n end");
    let context = format!("progression handler {handler:016x}");
    let main = match lua.load_source(wrapped.as_bytes(), &wrapped) {
        Ok(main) => main,
        Err(e) => {
            script_error(world, context, &e);
            return None;
        }
    };
    if let Err(e) = lua.call(world, &main, &[]) {
        script_error(world, context, &e);
        return None;
    }
    let function = lua.get_global(&function_name);
    matches!(function, Value::Function(_)).then(|| lua.reference(function))
}

/// `RunStateMachine` (`0x825e15c0`).
fn run_state_machine(lua: &mut Lua<World>, world: &mut World, key: u64) {
    if world.signpost.machines.contains_key(&key) {
        terminate_state_machine_now(lua, world, key);
    }
    let db = std::sync::Arc::clone(&world.db);
    let initial = refs(&db, class::PROGRESSION_STATE_MACHINES, key, field::INITIAL_STATE).first().map(|&(_, c)| c);
    let mut machine = Machine {
        key,
        states: BTreeMap::new(),
        current: None,
        next: None,
        terminate: false,
        autosave: false,
        running: false,
        running_handler: false,
        change_count: 0,
    };
    for (_, state) in refs(&db, class::PROGRESSION_STATE_MACHINES, key, field::STATES) {
        let mut handlers = [None; 9];
        let mut any = false;
        let handler_refs = refs(&db, class::PROGRESSION_STATES, state, field::STATE_HANDLERS);
        for (i, message) in SCRIPT_TYPES.iter().enumerate() {
            // GetMessageHandlerKeyFromName 0x825c97e8: first handler whose
            // Name is this message class.
            let Some(&(_, handler)) = handler_refs
                .iter()
                .find(|&&(_, h)| text(&db, class::PROGRESSION_HANDLERS, h, field::NAME).as_deref() == Some(message))
            else {
                continue;
            };
            any = true;
            handlers[i] = load_handler(lua, world, handler);
        }
        if any {
            let name = text(&db, class::PROGRESSION_STATES, state, field::NAME).unwrap_or_default();
            machine.states.insert(state, MachineState { key: state, name, handlers });
            if Some(state) == initial {
                machine.next = Some(state);
            }
        }
    }
    world.signpost.machines.insert(key, machine);
    execute(lua, world, key, STATE_ENTER);
}

/// `cSignPostStateMachine::Execute` (`0x825d0b78`).
fn execute(lua: &mut Lua<World>, world: &mut World, key: u64, mut script: usize) {
    let previous = world.signpost.current_machine.replace(key);
    {
        let Some(m) = world.signpost.machines.get_mut(&key) else { return };
        m.running = true;
    }
    loop {
        let (state, handler) = {
            let Some(m) = world.signpost.machines.get_mut(&key) else { break };
            if m.next.is_none() || m.terminate {
                break;
            }
            m.running_handler = true;
            if m.next != m.current {
                m.current = m.next;
                script = STATE_ENTER;
            }
            m.next = None;
            let state = m.current.and_then(|s| m.states.get(&s));
            (state.map(|s| s.name.clone()), state.and_then(|s| s.handlers[script]))
        };
        if let Some(reference) = handler {
            let function = lua.referenced(reference);
            if let Err(e) = lua.call(world, &function, &[]) {
                let machine = text(&world.db, class::PROGRESSION_STATE_MACHINES, key, field::NAME).unwrap_or_default();
                script_error(world, format!("{machine}/{}/{}", state.unwrap_or_default(), SCRIPT_TYPES[script]), &e);
            }
        }
        let Some(m) = world.signpost.machines.get(&key) else { break };
        if m.next.is_none() {
            break;
        }
    }
    world.signpost.current_machine = previous;
    let Some(m) = world.signpost.machines.get_mut(&key) else { return };
    m.running_handler = false;
    if m.terminate {
        let autosave = m.autosave;
        m.running = false;
        m.autosave = false;
        world.send(Event::TerminateStateMachine { machine: key, autosave });
    }
}

/// `SetMachineTerminateFlag` (`0x825cd038`).
fn set_terminate_flag(world: &mut World, key: u64, terminate: bool, autosave: bool) {
    let Some(m) = world.signpost.machines.get_mut(&key) else { return };
    if m.running || !terminate {
        if terminate {
            m.running = false;
        }
        m.terminate = terminate;
        m.autosave = autosave;
        if terminate && !m.running_handler {
            world.send(Event::TerminateStateMachine { machine: key, autosave });
        }
    }
}

/// `TerminateStateMachineNOW` (`0x825cc068`).
fn terminate_state_machine_now(lua: &mut Lua<World>, world: &mut World, key: u64) {
    if let Some(machine) = world.signpost.machines.remove(&key) {
        for state in machine.states.values() {
            for reference in state.handlers.iter().flatten() {
                lua.unreference(*reference);
            }
        }
        if world.signpost.activated_machine_count > 0 {
            world.signpost.activated_machine_count -= 1;
        }
    }
}

fn script_type(event: &Event) -> Option<usize> {
    Some(match event {
        Event::NisChallengeNotify { .. } => 1,
        Event::HudComplete => 2,
        Event::HudContinue => 3,
        Event::HudNo => 4,
        Event::HudYes => 5,
        Event::SpeechEnded { .. } => 6,
        Event::TimerExpired { .. } => 7,
        Event::FmvFinishedPlaying => 8,
        _ => return None,
    })
}

/// The manager's Hermes receivers on the Challenge port.
pub(crate) fn receive(lua: &mut Lua<World>, world: &mut World, event: &Event) {
    match event {
        Event::MessageSent { message } => receive_message_sent(world, *message),
        Event::TerminateStateMachine { machine, autosave } => {
            // ReceiveTerminateStateMachineMessage 0x825cf3b0.
            terminate_state_machine_now(lua, world, *machine);
            if *autosave {
                initiate_auto_save(world);
            }
        }
        other => {
            // Receive{NISChallengeNotify,HUD*,SpeechEnded,TimerExpired,FMV}Message.
            let Some(script) = script_type(other) else { return };
            let keys: Vec<u64> = world.signpost.machines.keys().copied().collect();
            for key in keys {
                let Some(m) = world.signpost.machines.get_mut(&key) else { continue };
                if !m.running {
                    continue;
                }
                if m.next.is_none() {
                    m.next = m.current;
                }
                execute(lua, world, key, script);
            }
        }
    }
}

/// `progression_messages` key from its `Name` (`GetMessageKeyFromName`).
fn message_key(world: &mut World, name: &str) -> Option<u64> {
    let key = hash64(name);
    if world.db.has_collection(class::PROGRESSION_MESSAGES, key)
        && text(&world.db, class::PROGRESSION_MESSAGES, key, field::NAME).as_deref() == Some(name)
    {
        return Some(key);
    }
    world
        .db
        .collections(class::PROGRESSION_MESSAGES)
        .iter()
        .copied()
        .find(|&k| text(&world.db, class::PROGRESSION_MESSAGES, k, field::NAME).as_deref() == Some(name))
}

/// `GetMessageStringProperty` (`0x825c57e0`) and friends: the property whose
/// definition is named `property`, read from the parallel `Properties` array.
fn message_property(db: &AttribDb, message: u64, property: &str) -> Option<(u64, u64)> {
    let definitions = refs(db, class::PROGRESSION_MESSAGES, message, field::PROPERTY_DEFINITIONS);
    let values = refs(db, class::PROGRESSION_MESSAGES, message, field::PROPERTIES);
    let index = definitions.iter().position(|&(c, k)| text(db, c, k, field::NAME).as_deref() == Some(property))?;
    values.get(index).copied()
}

pub(crate) fn message_string_property(db: &AttribDb, message: u64, property: &str) -> Option<String> {
    let (c, k) = message_property(db, message, property)?;
    text(db, c, k, field::VALUE_STRING)
}

fn message_int_property(db: &AttribDb, message: u64, property: &str) -> Option<i32> {
    let (c, k) = message_property(db, message, property)?;
    int(db, c, k, field::VALUE_INT)
}

/// `GetMessageBoolProperty` (`0x825c5360`): the value's byte at layout
/// offset 16.
fn message_bool_property(db: &AttribDb, message: u64, property: &str) -> Option<bool> {
    let (c, k) = message_property(db, message, property)?;
    flag(db, c, k, field::VALUE_BOOL)
}

/// `ReceiveMessageSentMessage` (`0x825e13e8`).
fn receive_message_sent(world: &mut World, message: u64) {
    update_message_settings(world, message);
    handle_caught_message(world, message);
    for (_, machine) in refs(&world.db, class::PROGRESSION_MESSAGES, message, field::TRIGGERS) {
        queue_triggered_state_machine(world, machine);
    }
}

/// `UpdateMessageSettings` (`0x825cc1b8`).
fn update_message_settings(world: &mut World, message: u64) {
    let db = std::sync::Arc::clone(&world.db);
    let values = refs(&db, class::PROGRESSION_MESSAGES, message, field::SETTINGS);
    let definitions = refs(&db, class::PROGRESSION_MESSAGES, message, field::SETTING_DEFINITIONS);
    for (i, &(vc, vk)) in values.iter().enumerate() {
        let Some(&(dc, dk)) = definitions.get(i) else { continue };
        let Some(slot) = world.signpost.messages.data.get_mut(&dk) else { continue };
        match int(&db, dc, dk, field::VALUE_TYPE).unwrap_or(0) {
            0 => *slot = u32::from(flag(&db, vc, vk, field::VALUE_BOOL) == Some(true)),
            1 => *slot = int(&db, vc, vk, field::VALUE_INT).unwrap_or(0).cast_unsigned(),
            2 => *slot = text(&db, vc, vk, field::VALUE_STRING).map_or(0, |s| hash_value32(&s)),
            _ => {}
        }
    }
}

/// `cSignPostUtil::HandleCaughtMessage` (`0x825de4e0`), by message type id;
/// types 8, 13 and 17 have no case. `kEnableSignPostMedia` is on in the
/// shipped game; media the port cannot present yet (NIS) takes the native's
/// media-off branch, which posts the completion message at once.
/// The unlock screens, NPC speech, Skate 101 and the cell-phone moment take
/// the media-on branch. `HandleCaughtMessageScriptOverride` (`0x825cc918`)
/// only answers while the Juice RPC runs a progression.
fn handle_caught_message(world: &mut World, message: u64) {
    let db = std::sync::Arc::clone(&world.db);
    let name = text(&db, class::PROGRESSION_MESSAGES, message, field::NAME).unwrap_or_default();
    let type_id = int(&db, class::PROGRESSION_MESSAGES, message, field::MESSAGE_TYPE_ID).unwrap_or(0);
    match type_id {
        1 => show_unlocks_screen(world, message),
        6 => handle_throwdown_events(world, message),
        14 => handle_skater_events(world, message),
        9 => {
            // TeleportPlayerMoment 0x825c5db0: cChallengeTeleportManager::
            // TeleportPlayer of the local player to "Location Name", no
            // transition.
            let location = message_string_property(&db, message, "Location Name").unwrap_or_default();
            let player = world.local_player;
            super::teleport::teleport_player(world, player, &location, player, false, 0.0, false);
        }
        4 => {
            // ChallengeAudioManager::PlayNPCSpeech (0x82da1bb0) of
            // "CharacterID" and "FileID".
            let character = message_int_property(&db, message, "CharacterID").unwrap_or(0);
            let id = message_int_property(&db, message, "FileID").unwrap_or(0);
            world.output(Output::PlayNpcSpeech { character, id, param: 0 });
        }
        5 => {
            // cMsgSkate101Show (0xF9141E2A) to the front end:
            // ReceiveSkate101HUDShowMsg (0x826285e8) reads the message's
            // "TitleHALID" and "TextHALID" and pushes FE state 79.
            let title = message_string_property(&db, message, "TitleHALID").unwrap_or_default();
            let text = message_string_property(&db, message, "TextHALID").unwrap_or_default();
            world.output(Output::ShowSkate101(Skate101View { title: hal_buffer(title), text: hal_buffer(text) }));
        }
        7 => cell_phone_moment(world, message),
        // ShowDebugText (0x825c5e88): cChallengeDebugManager::
        // DrawText2DPersistent, empty in this build.
        11 => {}
        16 => handle_blog_news_event(world, message),
        2 => {
            // kEnableSignPostMedia is on (dynamic initializer 0x82f55b60). Four
            // scenes wait 12 updates (cSignPostUtil::Update); the names are
            // compared case-sensitively (rw::core::stdc::StringCompare), so the
            // shipped "18_Mag_Cover" never matches "18_mag_cover".
            let scene = message_string_property(&db, message, "Scene Name").unwrap_or_default();
            let transition = message_int_property(&db, message, "Transition Type").unwrap_or(0);
            if DELAYED_NIS.contains(&scene.as_str()) {
                world.signpost.nis_wait_ticks = 12;
                world.signpost.cached_nis_message = Some(message);
            } else {
                world.output(Output::PlayNis { message: name, scene, transition });
            }
        }
        3 => {
            if let Some(challenge) = message_string_property(&db, message, "Challenge Name")
                && let Some(key) = world.signpost.helper.names.get(&hash_value32(&challenge)).copied()
            {
                execute_challenge(world, key);
            }
        }
        10 => {
            // ShowHUDComponent 0x825d0880: only "MiniMap", shown per the
            // "Minimap Visible" setting this message just updated.
            if message_string_property(&db, message, "ComponentName").as_deref() == Some("MiniMap") {
                let show = message_setting(world, "Minimap Visible") != 0;
                world.output(Output::ShowHudComponent { component: "minimap".into(), show });
            }
        }
        12 => {
            // Message id (layout word 2) 256 spawns Big Black, 257 removes
            // him; the other feature messages only update their settings.
            match db.layout_word(class::PROGRESSION_MESSAGES, message, 2) {
                256 => {
                    world.output(Output::BigBlack { spawn: true });
                    super::map_icons::activate_big_black_icon(world, true);
                }
                257 => {
                    world.output(Output::BigBlack { spawn: false });
                    super::map_icons::activate_big_black_icon(world, false);
                }
                _ => {}
            }
        }
        15 => {
            // kEnableProgressionMovies is on (dynamic initializer 0x82f55d90): the
            // FMV_ID's fe_movie key goes to FE state 28, whose movie sends
            // cMsgFMVFinishedPlaying when it ends; without a key it is sent at once.
            let movie = message_string_property(&db, message, "FMV_ID").unwrap_or_default();
            let key = crate::ui::movie_player::fmv_movie_key_from_name(&db, &movie);
            if key == 0 {
                world.send(Event::FmvFinishedPlaying);
            } else {
                world.output(Output::PlayMovie { message: name, movie, key });
            }
        }
        18 => {
            // HandleShowDialogEvent 0x825ccf78: cMsgProgPopupOpener to the FE.
            let title = message_string_property(&db, message, "Title").unwrap_or_default();
            let text_id = message_string_property(&db, message, "Text").unwrap_or_default();
            world.output(Output::ShowDialog { message: name, title, text: text_id, yes_no: true });
        }
        _ => {}
    }
}

/// `cSignPostUtil::ShowUnlocksScreen` (`0x825cca80`, message type 1 with
/// `kEnableSignPostMedia` on), by message id (layout word 2): 10 opens the
/// unlock overlay (`cMsgOpenUnlockOverlay`, FE state 10), 57 sends the front
/// end `cMsgInitAutosave`, 180 counts an unlocked path, and any other with a
/// positive `SponsorID` opens the sponsor overlay (`cMsgOpenSponsorOverlay`,
/// FE state 11).
fn show_unlocks_screen(world: &mut World, message: u64) {
    let db = std::sync::Arc::clone(&world.db);
    match db.layout_word(class::PROGRESSION_MESSAGES, message, 2) {
        10 => world.output(Output::OpenUnlockOverlay),
        57 => initiate_auto_save(world),
        180 => world.signpost.completion.num_paths_unlocked += 1,
        _ => {
            let sponsor = message_int_property(&db, message, "SponsorID").unwrap_or(0);
            if sponsor > 0 {
                world.output(Output::OpenSponsorOverlay { sponsor });
            }
        }
    }
}

/// `throwdown_competitors` and its layout fields `HandleThrowdownEvents`
/// reads (`_LayoutStruct`).
mod competitor {
    pub const CLASS: u64 = 0x9c86_9217_c5ae_e9b4;
    /// `BoardSponsor` (`ClassRefSpec_sponsor`, collection key at +0x30).
    pub const BOARD_SPONSOR: u32 = 0x30;
    /// `DemoPro` (bool, +0x4c).
    pub const DEMO_PRO: u32 = 0x4c;
    /// `PhoneNumberMessageID` (short, +0x56).
    pub const PHONE_NUMBER_MESSAGE_ID: u32 = 0x56;
    /// `CompletedMessageID` (short, +0x5e).
    pub const COMPLETED_MESSAGE_ID: u32 = 0x5e;
}

/// The message id that creates the sponsor-me tape.
const SPONSOR_ME_TAPE_MESSAGE_ID: u32 = 180;

/// A signed short of a collection's layout.
fn layout_short(db: &AttribDb, class: u64, collection: u64, offset: u32) -> i32 {
    let word = db.layout_word(class, collection, offset / 4);
    let half = if offset % 4 == 0 { word >> 16 } else { word & 0xffff };
    i32::from(u16::try_from(half).unwrap_or(0).cast_signed())
}

/// `cSignPostUtil::HandleThrowdownEvents` (`0x825d72e0`, message type 6), by
/// message id (layout word 2), over the leaf `throwdown_competitors`
/// collections (`AttribSysUtils::GetLeafCollections`): a competitor whose
/// `PhoneNumberMessageID` is the id adds a phone number to the completion
/// results; a demo pro whose `CompletedMessageID` is the id names the
/// sponsor-me brand (its board sponsor); id 180 creates the sponsor-me tape.
fn handle_throwdown_events(world: &mut World, message: u64) {
    let db = std::sync::Arc::clone(&world.db);
    let id = db.layout_word(class::PROGRESSION_MESSAGES, message, 2);
    let signed_id = id.cast_signed();
    for competitor in db.leaf_collections(competitor::CLASS) {
        let completion = &mut world.signpost.completion;
        if layout_short(&db, competitor::CLASS, competitor, competitor::PHONE_NUMBER_MESSAGE_ID) == signed_id {
            completion.num_phone_numbers_unlocked += 1;
            completion.unlocked_phone_numbers.push(competitor);
        }
        if layout_short(&db, competitor::CLASS, competitor, competitor::COMPLETED_MESSAGE_ID) == signed_id
            && db.layout_byte(competitor::CLASS, competitor, competitor::DEMO_PRO) != 0
        {
            let index = competitor::BOARD_SPONSOR / 4;
            let sponsor = (u64::from(db.layout_word(competitor::CLASS, competitor, index)) << 32)
                | u64::from(db.layout_word(competitor::CLASS, competitor, index + 1));
            completion.sponsor_me_brand = if db.has_collection(signpost_progress::classes::SPONSORS, sponsor) { sponsor } else { 0 };
        }
    }
    if id == SPONSOR_ME_TAPE_MESSAGE_ID {
        world.signpost.completion.sponsor_me_created = true;
    }
}

/// `cSignPostUtil::HandleSkaterEvents` (`0x825ccc10`, message type 14): the
/// message named (layout word 0) "Pin Player" or "Unpin Player" sends
/// `cMsgPinPlayer` for the local human skater; other skater events do
/// nothing. The native skips the pin, clearing a skater flag instead, when the
/// skater's word at +0x214 is set; that skater state is not ported.
fn handle_skater_events(world: &mut World, message: u64) {
    match world.db.layout_text(class::PROGRESSION_MESSAGES, message, 0) {
        Some("Pin Player") => world.output(Output::PinPlayer { pin: true }),
        Some("Unpin Player") => world.output(Output::PinPlayer { pin: false }),
        _ => {}
    }
}

/// `cSignPostManager::ExecuteChallenge` (`0x825d27f0`): a main challenge
/// (`Global` 3 or 4) starts as the main activity; a location challenge
/// teleports the local player to its `MapStartLocation` with no transition.
fn execute_challenge(world: &mut World, signpost: u64) {
    let Some(challenge) = SignPost::challenge_key(&world.db, signpost) else { return };
    let Some(activity) = world.activities.find(challenge) else { return };
    match world.activities.global_type(&world.db, activity) {
        3 | 4 => super::activity::start_main_challenge(world, challenge),
        // eGlobalType_Location / eGlobalType_Full.
        1 | 2 => {
            let location = world
                .db
                .attribute(class::CHALLENGES, challenge, hash64("MapStartLocation"))
                .and_then(|a| a.text(0))
                .unwrap_or_default()
                .to_owned();
            let player = world.local_player;
            super::teleport::teleport_player(world, player, &location, player, false, 0.0, false);
        }
        _ => {}
    }
}

/// `cSignPostChallengeHelper::ConvertChallengeKeyToSignpostKey`
/// (`0x825cafb0`): the challenge's `Name` through the name map.
pub(crate) fn signpost_key_for_challenge(world: &World, challenge: u64) -> Option<u64> {
    let name = text(&world.db, class::CHALLENGES, challenge, field::NAME)?;
    world.signpost.helper.names.get(&hash_value32(&name)).copied()
}

/// The challenge-state-machine trigger class (`0xf5c8efa024ee3925`):
/// `StateMachineID` (+4), `WinLose` (+28) and `ChallengeID` (+32).
const TRIGGER_CLASS: u64 = 0xf5c8_efa0_24ee_3925;
mod trigger_field {
    pub const STATE_MACHINE_ID: u64 = 0x30fe_f85a_4e39_b759;
    pub const WIN_LOSE: u64 = 0x20c0_a525_c304_bb07;
    pub const CHALLENGE_ID: u64 = 0x2aeb_dda5_5a47_9fde;
}

/// `progression_challenges` `ChallengeID` (+48).
fn challenge_id(world: &World, signpost: u64) -> i32 {
    int(&world.db, class::PROGRESSION_CHALLENGES, signpost, trigger_field::CHALLENGE_ID).unwrap_or(0)
}

/// `cSignPostUtil::GetAllChallengeStateMachineTriggers` (`0x825d7180`):
/// triggers for one challenge id, or every trigger with an id when `None`.
fn challenge_triggers(world: &World, search: Option<i32>) -> Vec<u64> {
    let db = &world.db;
    db.collections(TRIGGER_CLASS)
        .iter()
        .copied()
        .filter(|&t| {
            let id = int(db, TRIGGER_CLASS, t, trigger_field::CHALLENGE_ID).unwrap_or(0);
            match search {
                Some(s) => s > 0 && id == s,
                None => id > 0,
            }
        })
        .collect()
}

/// `cSignPostUtil::GetStateMachineKeyFromID` (`0x825c96d0`).
fn state_machine_key_from_id(world: &World, id: i32) -> Option<u64> {
    let db = &world.db;
    db.collections(class::PROGRESSION_STATE_MACHINES)
        .iter()
        .copied()
        .find(|&k| int(db, class::PROGRESSION_STATE_MACHINES, k, trigger_field::STATE_MACHINE_ID) == Some(id))
}

/// `CheckResultLogic` (`0x825c4520`).
const fn check_result_logic(result: i32, win_lose: i32) -> bool {
    match result {
        0 => matches!(win_lose, 0 | 3 | 5),
        1 => matches!(win_lose, 1 | 3 | 4 | 5),
        2 => matches!(win_lose, 2 | 4 | 5),
        3 => win_lose == 6,
        4 => win_lose == 7,
        5 => matches!(win_lose, 8 | 10 | 11),
        6 => matches!(win_lose, 9 | 10),
        7 => win_lose == 11,
        _ => false,
    }
}

/// `CheckStateTriggers` (`0x825d7c90`): queue the machines whose trigger
/// matches this result for the activity's progression challenge.
pub(crate) fn check_state_triggers(world: &mut World, result: i32, challenge: u64) -> i32 {
    let Some(signpost) = signpost_key_for_challenge(world, challenge) else { return 0 };
    let id = challenge_id(world, signpost);
    let mut queued = 0;
    for trigger in challenge_triggers(world, Some(id)) {
        let win_lose = int(&world.db, TRIGGER_CLASS, trigger, trigger_field::WIN_LOSE).unwrap_or(0);
        if check_result_logic(result, win_lose) {
            let machine_id = int(&world.db, TRIGGER_CLASS, trigger, trigger_field::STATE_MACHINE_ID).unwrap_or(0);
            if let Some(machine) = state_machine_key_from_id(world, machine_id) {
                queue_triggered_state_machine(world, machine);
            }
            queued += 1;
        }
    }
    queued
}

/// `cSignPostManager::WinChallenge` (`0x825e0ea0`): record a win. The key is
/// the activity's (neutrino) key; it is deactivated as a signpost key, which
/// only matches a challenge whose keys coincide.
pub(crate) fn win_challenge(world: &mut World, challenge: u64) {
    if world.replaying_challenge {
        return;
    }
    let inserted = !world.signpost.results.contains_key(&challenge);
    world.signpost.results.entry(challenge).or_insert(0);
    if SignPost::challenge_key(&world.db, challenge).is_some() {
        // DeactivateChallenge(key, isNeutrinoKey = true).
        if let Some(signpost) = signpost_key_for_challenge(world, challenge) {
            deactivate_challenge(world, signpost);
        }
        if inserted {
            #[expect(clippy::cast_precision_loss, reason = "(float)_u64tod")]
            let key = challenge as f64 as f32;
            world.send(Event::Challenge(super::system::Message::new(
                "cMsgChallengeSuccess",
                vec![("ActivityKey", super::system::Field::Number(key))],
            )));
        }
    } else {
        // DeactivateChallenge(key, isNeutrinoKey = false).
        deactivate_challenge(world, challenge);
    }
}

/// `cSignPostManager::LoseChallenge` (`0x825e0f90`, result 1) and `AbandonChallenge`
/// (`0x825e1028`, result 2): outside a replay the challenge's result, `DeductChallengeLosses`
/// (`0x825de1e0`, a lost bet), then `cMsgChallengeFailed` of its key and the result (kind
/// `0xCFCC4109` at `0x825e0ff4` and `0x825e108c`, through the folded
/// `SendMsg<cMsgTerminateStateMachine>`).
pub(crate) fn end_challenge_unwon(world: &mut World, challenge: u64, result: i32) {
    if world.replaying_challenge {
        return;
    }
    world.signpost.results.insert(challenge, result);
    signpost_progress::deduct_challenge_losses(world, challenge);
    world.send(Event::ChallengeFailed { activity: challenge, result });
}

/// `UpdateCompletedChallengeMap(key, order, photo)` (`0x825d8e18`): -1
/// takes the next order.
pub(crate) fn update_completed_challenge_map(world: &mut World, signpost: u64, order: i32, photo_caption: i32) {
    let sp = &mut world.signpost;
    if sp.is_challenge_complete(signpost) {
        return;
    }
    sp.completed_count += 1;
    let order = if order == -1 { sp.completed_count } else { order };
    sp.completed.insert(signpost, CompletedChallenge { order, photo_caption });
    sp.available.remove(&signpost);
}

/// `cSignPostManager::ChallengeComplete` (`0x825e00f8`) for a signpost key:
/// the completion BLOG entry, awards, the completed map, the photo articles and the group exits.
fn challenge_complete(world: &mut World, signpost: u64) {
    if !world.enable_progression {
        evaluate_activities(world);
        return;
    }
    if signpost & 0xffff_ffff == 0 || world.signpost.is_challenge_complete(signpost) {
        return;
    }
    signpost_progress::add_challenge_blog(world, signpost, blog_type::CHALLENGE_COMPLETE);
    signpost_progress::issue_challenge_awards(world, signpost);
    update_completed_challenge_map(world, signpost, -1, 0);
    super::photo_articles::update_photo_articles_blogs(world, signpost);
    signpost_progress::group_exit_logic(world, signpost);
    evaluate_activities(world);
}

/// `cSignPostManager::CompleteChallenge(challengeKey, true)` (`0x825e1550`) of a challenge
/// (neutrino) key: a nonzero key, outside a replay, is won and the results processed.
pub(crate) fn complete_challenge_by_key(world: &mut World, challenge: u64) {
    if challenge == 0 || world.replaying_challenge {
        return;
    }
    win_challenge(world, challenge);
    process_challenge_results(world);
}

/// `cSignPostManager::CompleteChallenge(name)` (`0x825e3098`): the name's
/// signpost key (`GetChallengeKeyFromName`), its challenge key
/// (`ConvertSignpostChallengeKeyToNeutrinoKey`), then outside a replay
/// `WinChallenge` and `ProcessChallengeResults` (`CompleteChallenge`
/// `0x825e1550`).
pub(crate) fn complete_challenge_by_name(world: &mut World, name: &str) {
    let Some(signpost) = world.signpost.helper.names.get(&hash_value32(name)).copied() else { return };
    let Some(challenge) = SignPost::challenge_key(&world.db, signpost) else { return };
    complete_challenge_by_key(world, challenge);
}

/// PC port addition (Max Out Save, `max_out.rs`): every progression challenge completed as
/// `ChallengeComplete` (`0x825e00f8`) completes one, in key order and with progression enabled:
/// its completion BLOG entry, its awards, its place in the completed map, its photo articles and
/// its groups' exits. The state machines a completion would trigger are not run.
/// `SKATE2_TRACE_MAX_OUT` logs the challenges left incomplete.
pub(crate) fn complete_every_challenge(world: &mut World) {
    let enabled = std::mem::replace(&mut world.enable_progression, true);
    for signpost in world.signpost.progression_challenges.clone() {
        challenge_complete(world, signpost);
    }
    world.enable_progression = enabled;
    world.signpost.triggered.clear();
    world.signpost.state_machine_triggered = false;
    if std::env::var_os("SKATE2_TRACE_MAX_OUT").is_some() {
        let sp = &world.signpost;
        let left: Vec<&str> = sp
            .progression_challenges
            .iter()
            .filter(|&&key| !sp.is_challenge_complete(key))
            .map(|&key| world.db.layout_text(class::PROGRESSION_CHALLENGES, key, 0).unwrap_or("?"))
            .collect();
        eprintln!(
            "max out: {} of {} progression challenges complete ({} completed in all), left {left:?}",
            sp.progression_challenges.len() - left.len(),
            sp.progression_challenges.len(),
            sp.completed.len()
        );
    }
}

/// PC port addition (Max Out Save, `max_out.rs`): the names the progression handlers' scripts
/// pass to the native `call` (`Namespace.Function`) as a literal first argument outside a
/// comment, in handler order, each once.
pub(crate) fn scripted_names(db: &AttribDb, call: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for &handler in db.collections(class::PROGRESSION_HANDLERS) {
        let Some(source) = db.attribute(class::PROGRESSION_HANDLERS, handler, field::HANDLER_SOURCE).and_then(|a| a.text(0)) else {
            continue;
        };
        for name in literal_arguments(source, call) {
            if !names.iter().any(|known| known == name) {
                names.push(name.to_owned());
            }
        }
    }
    names
}

/// The literal first arguments of `call` in Lua `source`, each line read up to its `--` comment.
fn literal_arguments<'a>(source: &'a str, call: &str) -> Vec<&'a str> {
    let mut names = Vec::new();
    for line in source.lines() {
        let mut rest = line.split("--").next().unwrap_or_default();
        while let Some(at) = rest.find(call) {
            rest = &rest[at + call.len()..];
            let name = rest
                .trim_start()
                .strip_prefix('(')
                .and_then(|arguments| arguments.trim_start().strip_prefix('"'))
                .and_then(|literal| literal.split_once('"'))
                .map(|(name, _)| name);
            if let Some(name) = name.filter(|name| !name.is_empty()) {
                names.push(name);
            }
        }
    }
    names
}

#[cfg(test)]
mod max_out_tests {
    /// A call's literal first arguments count only outside comments, each where it is written.
    #[test]
    fn literal_arguments_skip_comments_and_other_calls() {
        let source = "\tUnlock.UnlockLocationForFreeskate( \"svpeak\" );\n\t-- Unlock.UnlockLocationForFreeskate( \"gvr\" );\n\
            Progression.SendMessage( \"x\" ); Unlock.UnlockLocationForFreeskate(\"megapark\") -- Unlock.UnlockLocationForFreeskate(\"dam\")\n\
            Unlock.UnlockLocationForFreeskate( name );";
        assert_eq!(super::literal_arguments(source, "Unlock.UnlockLocationForFreeskate"), ["svpeak", "megapark"]);
    }
}

/// `cSignPostManager::ProcessChallengeResults` (`0x825e10c0`): trigger the
/// state machines matching each result, complete the won challenges, and
/// autosave when no machine was triggered (a triggered machine saves when it
/// terminates); after a result, unless the start hub's quit overrode the
/// orientation, `TeleportLocalPlayerToChallengeEndPoint` (`0x82da30b8`) moves
/// the local skater to the main challenge's end point, off the board.
pub(crate) fn process_challenge_results(world: &mut World) {
    // 825e10d8..e0: leave pending career results, triggers and end-point teleport untouched.
    if world.replaying_challenge {
        return;
    }
    world.signpost.state_machine_triggered = false;
    let triggers = challenge_triggers(world, None);
    let results: Vec<(u64, i32)> = std::mem::take(&mut world.signpost.results).into_iter().collect();
    if results.is_empty() {
        world.signpost.state_machine_triggered = true;
    }
    for &(challenge, result) in &results {
        let signpost = signpost_key_for_challenge(world, challenge);
        let id = challenge_id(world, signpost.unwrap_or(challenge));
        for &trigger in &triggers {
            let trigger_id = int(&world.db, TRIGGER_CLASS, trigger, trigger_field::CHALLENGE_ID).unwrap_or(0);
            let win_lose = int(&world.db, TRIGGER_CLASS, trigger, trigger_field::WIN_LOSE).unwrap_or(0);
            if trigger_id == id && check_result_logic(result, win_lose) {
                let machine_id = int(&world.db, TRIGGER_CLASS, trigger, trigger_field::STATE_MACHINE_ID).unwrap_or(0);
                if let Some(machine) = state_machine_key_from_id(world, machine_id) {
                    queue_triggered_state_machine(world, machine);
                }
            }
        }
        if result == 0
            && let Some(signpost) = signpost
        {
            challenge_complete(world, signpost);
        }
    }
    if !world.signpost.state_machine_triggered {
        initiate_auto_save(world);
        evaluate_activities(world);
    }
    if !results.is_empty()
        && !world.signpost.progress.use_override_because_quitting
        && let Some(location) = super::start_hub::challenge_end_point(world)
    {
        world.output(Output::TeleportToChallengeEndPoint { location });
    }
}

/// `Sk8ChallengeSystem::InitiateAutoSave` (`0x825a66c0`), the progression's `mSignPostSave`:
/// `cMsgInitAutosave` in a career game (game mode flags 4) only.
fn initiate_auto_save(world: &mut World) {
    if world.game_mode_flags == 4 {
        world.output(Output::AutoSave);
    }
}

/// `cSignPostManager::SendMessage(name)` from native code.
pub(crate) fn send_message_by_name(world: &mut World, name: &str) {
    if let Some(message) = message_key(world, name) {
        world.send(Event::MessageSent { message });
    }
}

// ---------------------------------------------------------------------------
// Lua natives bound into the progression state (`PerformLuaCBinding`
// `0x825e4d58`).

fn string_arg(lua: &Lua<World>, call: &NativeCall, n: usize, function: &str) -> LuaResult<String> {
    Ok(String::from_utf8_lossy(&check_string(lua, call, n, function)?).into_owned())
}

fn title_key(lua: &Lua<World>, world: &World, call: &NativeCall, function: &str) -> LuaResult<Option<u64>> {
    Ok(world.signpost.key_from_title(&string_arg(lua, call, 1, function)?))
}

fn is_challenge_complete_by_title(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let key = title_key(lua, world, call, "IsChallengeCompleteByTitle")?;
    Ok(vec![Value::Boolean(key.is_some_and(|k| world.signpost.is_challenge_complete(k)))])
}

fn is_challenge_active_by_title(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let key = title_key(lua, world, call, "IsChallengeActiveByTitle")?;
    Ok(vec![Value::Boolean(key.is_some_and(|k| world.signpost.is_challenge_active(k)))])
}

fn activate_challenge_by_title(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    if let Some(key) = title_key(lua, world, call, "ActivateChallengeByTitle")? {
        activate_challenge(world, key, true, false, true);
    }
    Ok(Vec::new())
}

/// `Progression.ActivateChallenge(name)`: looked up by `Name`.
fn activate_challenge_by_name(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let name = string_arg(lua, call, 1, "ActivateChallenge")?;
    if let Some(key) = world.signpost.helper.names.get(&hash_value32(&name)).copied() {
        activate_challenge(world, key, true, false, true);
    }
    Ok(Vec::new())
}

fn deactivate_challenge_by_title(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    if let Some(key) = title_key(lua, world, call, "DeactivateChallengeByTitle")? {
        deactivate_challenge(world, key);
    }
    Ok(Vec::new())
}

/// `Progression.RevertChallengeByTitle` (`0x825e40f0`).
fn revert_challenge_by_title(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    if let Some(key) = title_key(lua, world, call, "RevertChallengeByTitle")? {
        revert_completed_challenge_map(world, key);
    }
    Ok(Vec::new())
}

/// `RevertCompletedChallengeMap` (`0x825e2ef8`): a completed challenge
/// leaves the completed map; the order of the last entry (in key order)
/// with its `ChallengeID`, 2049 when none, bounds the shuffle: every
/// remaining entry at or above it moves down a place, the count drops, and
/// the challenge is activated again (`ActivateChallenge(key, true, false,
/// true)`).
pub(crate) fn revert_completed_challenge_map(world: &mut World, signpost: u64) {
    if !world.signpost.is_challenge_complete(signpost) {
        return;
    }
    let id = challenge_id(world, signpost);
    let order = world
        .signpost
        .completed
        .iter()
        .filter(|&(&key, _)| challenge_id(world, key) == id)
        .map(|(_, c)| c.order)
        .last()
        .unwrap_or(2049);
    let sp = &mut world.signpost;
    sp.completed.remove(&signpost);
    sp.completed_count -= 1;
    for c in sp.completed.values_mut() {
        if c.order >= order {
            c.order -= 1;
        }
    }
    activate_challenge(world, signpost, true, false, true);
}

/// `cSignPostManager::RevertChallenge(name)` (`0x825e4050`): the name's
/// signpost key (`GetChallengeKeyFromName`), reverted.
pub(crate) fn revert_challenge_by_name(world: &mut World, name: &str) {
    if let Some(signpost) = world.signpost.helper.names.get(&hash_value32(name)).copied() {
        revert_completed_challenge_map(world, signpost);
    }
}

fn execute_challenge_by_title(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    if let Some(key) = title_key(lua, world, call, "ExecuteChallengeByTitle")? {
        execute_challenge(world, key);
    }
    Ok(Vec::new())
}

/// `Progression.SendMessage` and the challenge state's `ChallengeProgression.SendMessage`
/// (`0x82890af0`, through `ISignPostManager`) → `cSignPostManager::SendMessage` (`0x825cf190`).
fn send_message(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let name = string_arg(lua, call, 1, "SendMessage")?;
    match message_key(world, &name) {
        Some(message) => world.send(Event::MessageSent { message }),
        None => return Err(lua.error(format!("SendMessage: unknown progression message '{name}'"))),
    }
    Ok(Vec::new())
}

/// `cSignPostMessageMan::GetMessageSetting(name)`: the stored word.
fn message_setting(world: &World, name: &str) -> u32 {
    let man = &world.signpost.messages;
    man.names.get(&hash_value32(name)).and_then(|k| man.data.get(k)).copied().unwrap_or(0)
}

/// `Progression.DidPlayerBetMax()` (`0x825c7320`): `cSignPostManager::playerBetMax`, which only
/// `SetPlayersBet` (`0x825cfaf8`: a bet of at least the `MaximumBet` setting) sets.
fn did_player_bet_max(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> LuaResult<Vec<Value>> {
    Ok(vec![Value::Boolean(world.signpost.betting.bet_max)])
}

/// `cSignPostMessageMan::GetMessageSetting(name)` as a bool.
fn get_message_setting_bool(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let name = string_arg(lua, call, 1, "GetMessageSettingBool")?;
    let man = &world.signpost.messages;
    let value = man.names.get(&hash_value32(&name)).and_then(|k| man.data.get(k)).copied().unwrap_or(0);
    Ok(vec![Value::Boolean(value != 0)])
}

fn get_message_setting_int(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let name = string_arg(lua, call, 1, "GetMessageSettingInt")?;
    let man = &world.signpost.messages;
    let value = man.names.get(&hash_value32(&name)).and_then(|k| man.data.get(k)).copied().unwrap_or(0);
    #[expect(clippy::cast_precision_loss, reason = "lua_Number")]
    Ok(vec![Value::Number(value.cast_signed() as f32)])
}

/// `ChangeState` (`ProgressionChangeState` `0x825d2700` → `SetNextState`).
fn change_state(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let name = string_arg(lua, call, 1, "ChangeState")?;
    let Some(key) = world.signpost.current_machine else {
        return Err(lua.error("ChangeState outside a progression handler"));
    };
    let mut terminate = false;
    if let Some(m) = world.signpost.machines.get_mut(&key)
        && let Some(state) = m.states.values().find(|s| s.name == name).map(|s| s.key)
    {
        m.next = Some(state);
        m.change_count += 1;
        terminate = m.change_count > 500;
    }
    if terminate {
        set_terminate_flag(world, key, true, false);
    }
    Ok(Vec::new())
}

fn terminate(lua: &mut Lua<World>, world: &mut World, _: &NativeCall) -> LuaResult<Vec<Value>> {
    let key = world.signpost.current_machine.ok_or_else(|| lua.error("Terminate outside a handler"))?;
    set_terminate_flag(world, key, true, true);
    Ok(Vec::new())
}

fn terminate_without_save(lua: &mut Lua<World>, world: &mut World, _: &NativeCall) -> LuaResult<Vec<Value>> {
    let key = world.signpost.current_machine.ok_or_else(|| lua.error("TerminateWithoutSave outside a handler"))?;
    set_terminate_flag(world, key, true, false);
    Ok(Vec::new())
}

/// `SessionMarker.SetSessionMarker()` (`0x82da92d0`).
fn set_session_marker(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> LuaResult<Vec<Value>> {
    world.output(Output::SetSessionMarker);
    Ok(Vec::new())
}

/// `SessionMarker.ClearSessionMarker()` (`0x82da9350`): `cMsgClearSessionMarker`, which
/// `PlayerUI::ReceiveClearSessionMarkerMsg` (`0x828aa3f0`) turns into
/// `SessionMarker::ClearSessionMarker` (`0x828aa550`).
fn clear_session_marker(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> LuaResult<Vec<Value>> {
    world.output(Output::ClearSessionMarker);
    Ok(Vec::new())
}

/// `Achievement.GrantAchievement(achievement, delay = 0)` (`0x82885fe8`):
/// the stats back end's `GrantAchievement` (module 14).
fn grant_achievement(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let achievement = super::binding::get_int(call, 0);
    let time_delay = super::binding::get_int_or(call, 1, 0);
    world.output(Output::GrantAchievement { achievement, time_delay });
    Ok(Vec::new())
}

/// `NIS.IgnoreTeleportToBreadcrumbOnNextNIS()` (`0x8288bf80`).
fn ignore_teleport_to_breadcrumb_on_next_nis(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> LuaResult<Vec<Value>> {
    world.output(Output::IgnoreTeleportToBreadcrumbOnNextNis);
    Ok(Vec::new())
}

/// `FrontEndState_Skate101::SetTitleHAL`/`SetDescriptionHAL`:
/// `Snprintf(buffer, 0x80, "%s", hal)`, at most 127 bytes.
fn hal_buffer(mut hal: String) -> String {
    let mut end = hal.len().min(127);
    while !hal.is_char_boundary(end) {
        end -= 1;
    }
    hal.truncate(end);
    hal
}

/// `CellPhoneMoment` (`0x825d0798`): the message setting named by "Cell
/// Phone ID" is set true unless it already is; a missing setting reads true
/// (`GetMessageSetting(name, 1)` `0x825cd6a8`).
fn cell_phone_moment(world: &mut World, message: u64) {
    let Some(name) = message_string_property(&world.db, message, "Cell Phone ID") else { return };
    if world.signpost.message_setting_value(&name) == Some(0) {
        world.signpost.set_message_setting_value(&name, 1);
    }
}

/// `HandleBLOGNewsEvent` (`0x825d9410`): by the message's "Type", 0 a BLOG
/// message entry and 1 user news. "RemoveEntry" takes away what the adding
/// message with the same "HALText" added; otherwise the message's own id
/// (layout word 2) is added.
fn handle_blog_news_event(world: &mut World, message: u64) {
    let db = std::sync::Arc::clone(&world.db);
    let id = db.layout_word(class::PROGRESSION_MESSAGES, message, 2);
    let Some(text) = message_string_property(&db, message, "HALText") else { return };
    let kind = message_int_property(&db, message, "Type").unwrap_or(0);
    let remove = message_bool_property(&db, message, "RemoveEntry").unwrap_or(false);
    let added = |kind| add_blog_message(&db, kind, &text).map(|m| db.layout_word(class::PROGRESSION_MESSAGES, m, 2));
    match (kind, remove) {
        (0, false) => signpost_progress::add_message_blog(world, id),
        (0, true) => {
            if let Some(added) = added(0) {
                signpost_progress::remove_blog_entry(world, blog_type::MESSAGE, added);
            }
        }
        (1, false) => signpost_progress::add_user_news(world, id),
        (1, true) => {
            if let Some(added) = added(1) {
                signpost_progress::remove_user_news(world, added);
            }
        }
        _ => {}
    }
}

/// `GetAddBLOGMessage(type, text)` (`0x825ca1b8`): the first message of type
/// 16 (layout word 1) with that "Type" and "HALText" whose "RemoveEntry"
/// is false.
fn add_blog_message(db: &AttribDb, kind: i32, text: &str) -> Option<u64> {
    let messages = class::PROGRESSION_MESSAGES;
    db.collections(messages).iter().copied().find(|&m| {
        db.layout_word(messages, m, 1) == 16
            && message_int_property(db, m, "Type") == Some(kind)
            && message_string_property(db, m, "HALText").as_deref() == Some(text)
            && message_bool_property(db, m, "RemoveEntry") == Some(false)
    })
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, super::lua::NativeFn<World>)> {
    let mut natives: Vec<(&'static str, &'static str, super::lua::NativeFn<World>)> = vec![
        ("Progression", "IsChallengeCompleteByTitle", is_challenge_complete_by_title),
        ("Progression", "IsChallengeActiveByTitle", is_challenge_active_by_title),
        ("Progression", "ActivateChallengeByTitle", activate_challenge_by_title),
        ("Progression", "ActivateChallenge", activate_challenge_by_name),
        ("Progression", "DeactivateChallengeByTitle", deactivate_challenge_by_title),
        ("Progression", "RevertChallengeByTitle", revert_challenge_by_title),
        ("Progression", "ExecuteChallengeByTitle", execute_challenge_by_title),
        ("Progression", "SendMessage", send_message),
        ("ChallengeProgression", "SendMessage", send_message),
        ("Progression", "GetMessageSettingBool", get_message_setting_bool),
        ("Progression", "DidPlayerBetMax", did_player_bet_max),
        ("Progression", "GetMessageSettingInt", get_message_setting_int),
        ("ChallengeAsset", "PreloadChallengeAssetsByName", super::natives::asset_preload_by_name),
        ("ChallengeSpeech", "PlayCameramanSpeech", super::natives::speech_play_cameraman),
        ("ChallengeSpeech", "PlayProgressionReminder", super::natives::speech_play_cameraman),
        ("SessionMarker", "SetSessionMarker", set_session_marker),
        ("SessionMarker", "ClearSessionMarker", clear_session_marker),
        ("Teleport", "TeleportLocalPlayerToLocationName", super::natives::teleport_local_player_to_location_name),
        ("Achievement", "GrantAchievement", grant_achievement),
        ("NIS", "IgnoreTeleportToBreadcrumbOnNextNIS", ignore_teleport_to_breadcrumb_on_next_nis),
    ];
    natives.extend(signpost_progress::natives());
    natives.extend(super::dynamic_hull::natives());
    natives.extend(super::unlockables::natives());
    natives
}

/// `cSignPostManager::PerformLuaCBinding` (`0x825e4d58`).
pub(crate) fn bind(lua: &mut Lua<World>) {
    let modules = [
        "Progression",
        "Achievement",
        "Award",
        "Unlock",
        "ChallengeAsset",
        "GameMode",
        "DynamicHull",
        "ChallengeSpeech",
        "NIS",
        "Service",
        "Teleport",
        "SessionMarker",
    ];
    binder::bind_modules(lua, &modules, &natives());
    for (name, f) in [
        ("ChangeState", change_state as super::lua::NativeFn<World>),
        ("Terminate", terminate),
        ("TerminateWithoutSave", terminate_without_save),
    ] {
        let native = lua.create_native(name, f, Vec::new());
        lua.set_global(name, native);
    }
    binder::bind_message_exporters(lua, &super::natives::message_exporters());
    let _ = hash32;
}
