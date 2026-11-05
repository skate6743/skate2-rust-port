//! `Sk8::Challenge::cChallengeSignUpManager`: sign-up (`Global` 3) activities
//! whose prerequisites pass get a box around their `Location`; the local
//! tester's box meeting one shows the D-pad-up prompt (`SignUpHUD`), and a
//! press runs the high- then low-priority sub-controllers' select operations,
//! the poster, their accept (or decline) operations, then
//! `StartMainChallenge`.

use super::{
    activity, assets, dmo, effects,
    hud::VISIBILITY_CHALLENGE_EFFECTS,
    indicators,
    keys::{class, hash64, hash_value32},
    signpost, signup_hud,
    system::{Event, Output, World},
};
use crate::grind_data_octree::OctreeBounds;

/// `eSignUpState`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SignUpState {
    #[default]
    Disabled,
    WaitingForSignUpVolume,
    WaitingForDPadUp,
    RunHighPriorityDPadUpOperations,
    RunLowPriorityDPadUpOperations,
    WaitingForConfirmation,
    RunHighPriorityAcceptOperations,
    RunLowPriorityAcceptOperations,
    RunHighPriorityDeclineOperations,
    RunLowPriorityDeclineOperations,
    StartChallenge,
}

mod field {
    /// `challenge_global_data.Global`.
    pub const GLOBAL: u64 = 0x122d_b7b0_f026_dbec;
    /// `challenge_global_data.Location`.
    pub const LOCATION: u64 = 0xf1c6_d5fa_4a23_951e;
    /// `challenge_global_data.ChallengeType`.
    pub const CHALLENGE_TYPE: u64 = 0x02b8_28cc_db47_85e4;
    /// `challenge_global_data`: an activity only debug builds create.
    pub const DEBUG_ONLY: u64 = 0xfd40_e4a3_1027_c984;
    /// `challenge_global_data` flags read for game types 8 and 2.
    pub const GAME_TYPE_8: u64 = 0xf9a9_a1dd_ed84_a643;
    pub const GAME_TYPE_2: u64 = 0xa279_f685_d237_09f4;
    /// `challenges`: the sign-up box's half size.
    pub const RADIUS: u64 = 0x8022_ba64_cb8e_431d;
    /// `challenges.IsAThrowdownChallenge`.
    pub const IS_A_THROWDOWN: u64 = 0x4263_ed15_8a50_063e;
    /// `challenge_global_data.SignUpIndicator`: the sign-up location's ribbon.
    pub const SIGN_UP_INDICATOR: u64 = 0x284b_0732_5d58_7cc4;
}

/// `eChallengeTypes` the sign-up path tests.
mod challenge_type {
    pub const OWN_THE_SPOT: i32 = 9;
    pub const CREATE_A_SPOT: i32 = 35;
}

/// `kDefaultLocationTrackingRadius`.
const DEFAULT_LOCATION_TRACKING_RADIUS: f32 = 40.0;
/// `TriggerVolumeUtils::kTriggerAABBCharacterWidth`/`Height`.
const TRIGGER_AABB_CHARACTER_WIDTH: f32 = 0.58;
const TRIGGER_AABB_CHARACTER_HEIGHT: f32 = 1.74;
/// The slowed update's base sim time (`0.016666668`).
const SLOW_DOWN_SIM_TIME: f32 = f32::from_bits(0x3c88_8889);

/// `mIsSignUpSlowDown` and `mSignUpSlowDownRate` (`EnableSignUpSlowDown`
/// `0x82da1ad0`, `DisableSignUpSlowDown` `0x82da1ae0`); the constructor
/// sets the rate to 1.0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlowDown {
    pub enabled: bool,
    pub rate: f32,
}

impl Default for SlowDown {
    fn default() -> Self {
        Self { enabled: false, rate: 1.0 }
    }
}

#[derive(Debug, Default)]
pub struct SignUpManager {
    game_loaded: bool,
    pub state: SignUpState,
    /// `AllSignUpActivities`.
    all: Vec<usize>,
    /// `SignUpActivitiesPassedPrereq`, ascending.
    passed: Vec<usize>,
    /// `SignUpOctTree.OctTreeDataEntries` up to `EntryFillIndex`.
    entries: Vec<Option<(usize, OctreeBounds)>>,
    /// `FreeDataEntryIndexices`.
    free: Vec<u32>,
    /// `PreviousCollisionIndicies`.
    previous: Vec<u32>,
    /// `MissingGlobalLocator`.
    missing_locator: Vec<usize>,
    /// `SelectedChallengeData`.
    pub selected: Option<usize>,
    teleport_start: bool,
    /// `SignUpData::mChallenges`: the prompt's challenges.
    pub prompt: Vec<usize>,
    /// `SignUpData`'s dirty flag (`MakeDirty`).
    pub prompt_dirty: bool,
    /// The prompt (`SignUpHUD`) is shown.
    pub prompt_visible: bool,
    /// `VisualIndicatorGroup`: the passed sign-ups' location indicators, from `GameLoad`.
    pub indicator_group: Option<u32>,
    /// `cChallengeEffectsManager`'s sign-up flag (select until accept or
    /// decline).
    effects_signing_up: bool,
    pub slow_down: SlowDown,
    /// `mShowSignUpRenderEffect`; nothing in this build reads it.
    pub show_render_effect: bool,
    /// `FrontEndState_SignUpPoster::mKey` (0 from its constructor `0x82647e88`): the challenge
    /// `OpenPoster` (`0x82658240`) last showed, which stays after the poster closes.
    pub poster: u64,
}

fn global_data(world: &World, activity: usize) -> Option<(u64, u64)> {
    world.activities.activities[activity].global_data
}

/// The activity's sign-up location (its global data's `Location`).
pub(crate) fn location_name(world: &World, activity: usize) -> Option<String> {
    global_text(world, activity, field::LOCATION)
}

/// `VisualIndicatorGroup->mEnabled`.
fn set_indicators_enabled(world: &mut World, enabled: bool) {
    if let Some(group) = world.signup.indicator_group.and_then(|id| world.indicators.group_mut(id)) {
        group.enabled = enabled;
    }
}

fn global_i32(world: &World, activity: usize, key: u64) -> i32 {
    global_data(world, activity).and_then(|(c, k)| world.db.attribute(c, k, key)).and_then(|a| a.i32(0)).unwrap_or(0)
}

fn global_bool(world: &World, activity: usize, key: u64) -> bool {
    global_data(world, activity).and_then(|(c, k)| world.db.attribute(c, k, key)).and_then(|a| a.bool(0)).unwrap_or(false)
}

fn global_text(world: &World, activity: usize, key: u64) -> Option<String> {
    global_data(world, activity).and_then(|(c, k)| world.db.attribute(c, k, key)).and_then(|a| a.text(0)).map(str::to_owned)
}

fn challenge_bool(world: &World, activity: usize, key: u64) -> bool {
    let challenge = world.activities.activities[activity].key;
    world.db.attribute(class::CHALLENGES, challenge, key).and_then(|a| a.bool(0)).unwrap_or(false)
}

pub(crate) fn challenge_type(world: &World, activity: usize) -> i32 {
    global_i32(world, activity, field::CHALLENGE_TYPE)
}

/// `cLocationManager::IsValidLocation`.
fn location_position(world: &World, name: &str) -> Option<[f32; 3]> {
    world.location(hash_value32(name)).map(|t| [t[12], t[13], t[14]])
}

/// `cActivityManager::CreateActivity` (`0x82cdbca8`) passes every activity
/// that is not debug-only to `AddActivity` (`0x82daee90`), which keeps the
/// `Global` 3 ones.
pub(crate) fn init(world: &mut World) {
    let count = world.activities.activities.len();
    for activity in 0..count {
        if !global_bool(world, activity, field::DEBUG_ONLY) && global_i32(world, activity, field::GLOBAL) == 3 {
            world.signup.all.push(activity);
        }
    }
}

/// `cChallengeSignUpManager::Reset` (`0x82dabc80`).
fn reset(world: &mut World) {
    if let Some(group) = world.signup.indicator_group.take() {
        world.indicators.delete_group(group);
    }
    let s = &mut world.signup;
    s.game_loaded = false;
    s.prompt.clear();
    s.prompt_visible = false;
    s.passed.clear();
    s.entries.clear();
    s.free.clear();
    s.previous.clear();
    s.missing_locator.clear();
    s.state = SignUpState::Disabled;
    s.selected = None;
    s.teleport_start = false;
    s.slow_down.enabled = false;
    if world.hud.group_exists(signup_hud::GROUP) {
        world.hud.delete_group(signup_hud::GROUP);
    }
}

/// `cChallengeSignUpManager::GameLoad` (`0x82daec00`).
pub(crate) fn game_load(world: &mut World, game_type_flags: u32) {
    reset(world);
    if game_type_flags == 4 {
        world.signup.indicator_group = Some(world.indicators.create_group(0));
        world.signup.game_loaded = true;
        reevaluate_activity_prereqs(world, 4);
        world.hud.create_group(signup_hud::GROUP, &[]);
        world.hud.add_component(signup_hud::GROUP, signup_hud::COMPONENT);
    }
}

/// `cChallengeSignUpManager::LoadingReset` (`0x82da1a58`).
pub(crate) fn loading_reset(world: &mut World) {
    if world.signup.game_loaded && world.signup.state == SignUpState::Disabled {
        world.signup.state = SignUpState::WaitingForSignUpVolume;
    }
}

/// `ActivityUtils::EvaluateActivitiesForPrereqs` (`0x82cd9e88`): the
/// activities that pass for these game type flags, ascending.
pub(crate) fn evaluate_for_prereqs(world: &World, source: &[usize], flags: u32, ignore_prereq: &str) -> Vec<usize> {
    let ignore_key = (!ignore_prereq.is_empty()).then(||hash64(ignore_prereq));
    let mut passed = Vec::new();
    for &activity in source {
        let mut pass = false;
        if flags & 8 == 8 {
            if !global_bool(world, activity, field::GAME_TYPE_8) {
                continue;
            }
            pass = true;
        }
        if flags & 2 == 2 && !global_bool(world, activity, field::GAME_TYPE_2) {
            continue;
        }
        if !pass {
            pass = ignore_key.is_some_and(|key|challenge_bool(world, activity, key));
        }
        if !pass {
            pass = signpost::is_neutrino_challenge_active(world, world.activities.activities[activity].key);
        }
        if pass {
            passed.push(activity);
        }
    }
    passed.sort_unstable();
    passed
}

/// `cManagedOctTree::Insert`: the most recently freed slot, else the next.
fn tree_insert(world: &mut World, activity: usize, bounds: OctreeBounds) {
    let s = &mut world.signup;
    if let Some(index) = s.free.pop() {
        s.entries[index as usize] = Some((activity, bounds));
    } else {
        s.entries.push(Some((activity, bounds)));
    }
}

/// `cManagedOctTree::Remove(data)`: the first slot holding the activity,
/// also dropped from the previous collisions.
fn tree_remove(world: &mut World, activity: usize) {
    let s = &mut world.signup;
    let Some(index) = s.entries.iter().position(|e| e.is_some_and(|(a, _)| a == activity)) else { return };
    s.entries[index] = None;
    #[expect(clippy::cast_possible_truncation, reason = "512 entries")]
    let index = index as u32;
    s.free.push(index);
    if let Some(at) = s.previous.iter().position(|&i| i == index) {
        s.previous.remove(at);
    }
}

/// `cChallengeSignUpManager::ReevaluateActivityPrereqs` (`0x82db35a0`).
pub(crate) fn reevaluate_activity_prereqs(world: &mut World, flags: u32) {
    if !world.signup.game_loaded {
        return;
    }
    let all = world.signup.all.clone();
    let passed = evaluate_for_prereqs(world, &all, flags, "IgnorePrereqForSignup");
    let cached = std::mem::take(&mut world.signup.passed);
    for &failed in cached.iter().filter(|a| passed.binary_search(a).is_err()) {
        tree_remove(world, failed);
    }
    for &activity in passed.iter().filter(|a| cached.binary_search(a).is_err()) {
        if world.signup.entries.iter().any(|e| e.is_some_and(|(a, _)| a == activity)) {
            continue;
        }
        let Some(location) = location_name(world, activity).filter(|l| !l.is_empty()) else { continue };
        if let Some(position) = location_position(world, &location) {
            // ActivityUtils::GenerateLocationActivityBBox 0x82cd89c0.
            let key = world.activities.activities[activity].key;
            let radius = world.db.attribute(class::CHALLENGES, key, field::RADIUS).and_then(|a| a.f32(0)).unwrap_or(0.0);
            let r = if radius > 0.0 { radius } else { DEFAULT_LOCATION_TRACKING_RADIUS };
            let bounds = OctreeBounds {
                minimum: [position[0] - r, position[1] - r, position[2] - r, 0.0],
                maximum: [position[0] + r, position[1] + r, position[2] + r, 0.0],
            };
            tree_insert(world, activity, bounds);
        } else if !world.signup.missing_locator.contains(&activity) {
            world.signup.missing_locator.push(activity);
        }
    }
    if let Some(group) = world.signup.indicator_group {
        world.indicators.clear_all(group);
        for &activity in &passed {
            if let Some(location) = location_name(world, activity).filter(|l| !l.is_empty())
                && location_position(world, &location).is_some()
            {
                // The SignUpIndicator ribbon, in the default colour.
                let ribbon = global_data(world, activity)
                    .and_then(|(c, k)| world.db.attribute(c, k, field::SIGN_UP_INDICATOR))
                    .and_then(|a| a.ref_spec(0))
                    .map_or(0, |(_, collection)| collection);
                indicators::add_location_indicator(world, group, &location, ribbon, u64::MAX);
            }
        }
    }
    world.signup.passed = passed;
}

/// `PerformBBoxTests` (`0x82db33e8`) with `TriggerVolumeUtils::
/// GenerateTesterBBox` (`0x82e20ee0`): the boxes the tester's base box meets,
/// and whether that set changed.
fn perform_bbox_tests(world: &mut World) -> bool {
    let Some(base) = world.player.tester_base else { return false };
    let half = TRIGGER_AABB_CHARACTER_WIDTH * 0.5;
    let tester = OctreeBounds {
        minimum: [base[0] - half, base[1], base[2] - half, 0.0],
        maximum: [base[0] + half, base[1] + TRIGGER_AABB_CHARACTER_HEIGHT, base[2] + half, 0.0],
    };
    let s = &mut world.signup;
    #[expect(clippy::cast_possible_truncation, reason = "512 entries")]
    let colliding: Vec<u32> =
        s.entries.iter().enumerate().filter(|(_, e)| e.is_some_and(|(_, b)| b.overlaps(tester))).map(|(i, _)| i as u32).collect();
    let changed = colliding != s.previous;
    s.previous = colliding;
    changed
}

/// `IsAllowedToSignup` (`0x82da2d60`): `cActivityManager::CanStartMainChallenge`
/// (`0x82cd8798`), and every sign-up sub-controller's `IsSignupAvailable`: the challenge
/// system's own (`0x825a4488`: no memory card operation and no NIS, while which the port's
/// simulation does not update the challenges), the living world manager's (`0x825ad858`, not
/// during a chase), the BE manager's (`0x825ada38`: not while the flow's `STATE_Loading` is
/// active, and `mGotoAllowsSignup`), the HOM manager's (`0x825b2fc8`, the local skater's
/// `PhysOut_Scoring2` neither in nor just out of a Hall of Meat bail), the OTS discovery
/// manager's (`0x825af878`) and the service manager's (`0x8289c660`).
fn is_allowed_to_sign_up(world: &World) -> bool {
    activity::can_start_main_challenge(world)
        && !world.memcard_active
        && super::living_world::is_signup_available(world)
        && !world.flow_loading
        && world.goto_allows_signup
        && !(world.player.hom_bail_in_progress || world.player.hom_bail_completed)
        && super::ots_discovery::is_signup_available(world)
        && super::services::is_signup_available(world)
}

/// `SetupDPadUpHUDDataFromColliding` (`0x82da4c48`) with
/// `SignUpData::SortChallenges`.
fn setup_prompt(world: &mut World) {
    let s = &mut world.signup;
    let mut prompt: Vec<usize> = s.previous.iter().filter_map(|&i| s.entries.get(i as usize).copied().flatten()).map(|(a, _)| a).collect();
    sort_challenges(world, &mut prompt);
    world.signup.prompt = prompt;
    world.signup.prompt_dirty = true;
}

/// `SignUpData::SortChallenges` (`0x82d83618`): EASTL `quick_sort` with
/// `SignUpChallengeSorter`, which puts an entry before any other unless it
/// is Create-a-Spot. Up to 28 entries only its `insertion_sort`
/// (`0x82d82d30`) runs, moving each other entry to the front in turn.
fn sort_challenges(world: &World, challenges: &mut [usize]) {
    for i in 1..challenges.len() {
        if challenge_type(world, challenges[i]) != challenge_type::CREATE_A_SPOT {
            challenges[..=i].rotate_right(1);
        }
    }
}

/// `ShowDPadUpHUD` (`0x82da2cc0`): the sign-up group made active and its
/// `signup` instance shown; the HUD update then runs `SignUpHUD::Show(true)`
/// (`signup_hud`), which owns the press delay and held-button state.
fn show_prompt(world: &mut World) {
    world.hud.add_group_to_active(signup_hud::GROUP);
    world.hud.show(signup_hud::GROUP, signup_hud::COMPONENT, true);
    world.signup.prompt_visible = true;
    output_prompt(world);
}

/// A diagnostic prompt of the given challenges, shown as `ShowDPadUpHUD` shows the colliding ones.
pub(crate) fn diagnostic_prompt(world: &mut World, prompt: Vec<usize>) {
    world.signup.prompt = prompt;
    world.signup.prompt_dirty = true;
    show_prompt(world);
}

/// `HideDPadUpHUD` (`0x82da2d10`): the instance hidden and the group
/// removed from the active groups; the HUD update unbinds the component,
/// which `Show(false)` hides.
fn hide_prompt(world: &mut World) {
    world.hud.show(signup_hud::GROUP, signup_hud::COMPONENT, false);
    world.hud.remove_group_from_active(signup_hud::GROUP);
    world.signup.prompt_visible = false;
    output_prompt(world);
}

fn output_prompt(world: &mut World) {
    let challenges = if world.signup.prompt_visible {
        world.signup.prompt.iter().map(|&a| world.activities.activities[a].name.clone()).collect()
    } else {
        Vec::new()
    };
    world.output(Output::SignUpPrompt { challenges });
}

/// `SignUpHUD::UpdateInput` D-pad up (`UpdateDPadUp` `0x826bd450`) for a
/// sign-up: a press before the delay has run out waits for a fresh press;
/// one challenge signs up, several open the choice. True when the choice
/// opened, which also pushes `kPAUSE_STATE_FRONTEND` (`0x826bd738`).
pub fn dpad_up(world: &mut World) -> bool {
    signup_hud::dpad_up(world)
}

/// `SignUpHUD::UpdateInput(0x4000)`: the release.
pub fn dpad_released(world: &mut World) {
    signup_hud::dpad_released(world);
}

/// `SignUpHUD::MultipleSignupsCallback` (`0x826b5430`): a chosen option, or
/// `None` for a cancel (the prompt shows again).
pub fn choose(world: &mut World, option: Option<usize>) {
    match option.and_then(|i| world.signup.prompt.get(i).copied()) {
        Some(activity) => player_trigger_sign_up(world, activity, false),
        None => {
            // A cancel runs the component's Show(true) directly.
            world.signup.prompt_visible = true;
            output_prompt(world);
            signup_hud::show(world, true);
        }
    }
}

/// `PlayerTriggerSignUp` (`0x82da6920`) with the high-priority
/// sub-controllers' `OnChallengeSelect`.
pub fn player_trigger_sign_up(world: &mut World, activity: usize, teleport_start: bool) {
    world.signup.state = SignUpState::RunHighPriorityDPadUpOperations;
    world.signup.selected = Some(activity);
    world.signup.teleport_start = teleport_start;
    set_indicators_enabled(world, false);
    world.output(Output::PinPlayer { pin: true });
    // cSignPostManager::OnChallengeSelect 0x825d7e30.
    let key = world.activities.activities[activity].key;
    world.signpost.progress.cash_earned = [0; 7];
    let queued = signpost::check_state_triggers(world, 4, key);
    signpost::set_activated_machine_count(world, queued);
    // cChallengeBEManager::OnChallengeSelect 0x825b7a18.
    world.pause_menu_disabled = true;
    hide_prompt(world);
}

/// The low-priority sub-controllers' `OnChallengeSelect`.
fn low_priority_select(world: &mut World, activity: usize) {
    // ChallengeHOMManager.OnChallengeSelect825ae3f8.
    super::hom::enable_visuals(world, Some(activity), false);
    world.hom.signing_up = true;
    super::services::on_select(world);
    teleport_on_select(world, activity);
    // cChallengeAssetManager::OnChallengeSelect 0x82e12650.
    assets::load_challenge_assets(world, activity, true, false, 0);
    // cChallengeEffectsManager::OnChallengeSelect 0x82da2528.
    world.signup.effects_signing_up = true;
}

/// `cChallengeTeleportManager::OnChallengeSelect` (`0x82dadb20`): unless the
/// challenge is Own the Spot, Create-a-Spot or a throwdown, or Pick-a-Pro
/// set the skip (cleared here), the local player is moved to its
/// `MapStartLocation`, else its `Location`, taken from the global data before
/// the challenge data, behind a one-second transition.
fn teleport_on_select(world: &mut World, activity: usize) {
    let kind = challenge_type(world, activity);
    let skip = kind == challenge_type::OWN_THE_SPOT
        || kind == challenge_type::CREATE_A_SPOT
        || challenge_bool(world, activity, field::IS_A_THROWDOWN)
        || world.teleport.is_pick_a_pro;
    world.teleport.is_pick_a_pro = false;
    if skip {
        return;
    }
    let challenge = world.activities.activities[activity].key;
    for name in ["MapStartLocation", "Location"] {
        let key = hash64(name);
        let location = global_data(world, activity)
            .and_then(|(c, k)| world.db.attribute(c, k, key))
            .or_else(|| world.db.attribute(class::CHALLENGES, challenge, key))
            .and_then(|a| a.text(0))
            .map(str::to_owned);
        if let Some(location) = location {
            // OnChallengeSelect82dadce8 uses adjusted this+0x14 = the
            // enclosing manager's INetworkGameQuery (+0x1c), slot4.
            let player = super::players::OFFLINE_LOCAL_PLAYER_ID;
            super::teleport::teleport_player(world, player, &location, player, false, 1.0, false);
            return;
        }
    }
}

/// `PlayerAcceptChallenge` (`0x82da66e0`) with the high-priority
/// `OnChallengeAccept`.
pub fn player_accept(world: &mut World) {
    let Some(activity) = world.signup.selected else { return };
    if world.signup.state != SignUpState::WaitingForConfirmation {
        return;
    }
    world.signup.state = SignUpState::RunHighPriorityAcceptOperations;
    let key = world.activities.activities[activity].key;
    let queued = signpost::check_state_triggers(world, 5, key);
    signpost::set_activated_machine_count(world, queued);
    // cChallengeBEManager::OnChallengeAccept 0x825b7a40.
    world.pause_menu_disabled = false;
    world.signup.show_render_effect = false;
    world.signup.slow_down = SlowDown::default();
    world.output(Output::PinPlayer { pin: false });
    set_indicators_enabled(world, true);
}

/// `PlayerDeclineChallenge` (`0x82da6800`) with the high-priority
/// `OnChallengeDecline`.
pub fn player_decline(world: &mut World) {
    let Some(activity) = world.signup.selected else { return };
    if world.signup.state != SignUpState::WaitingForConfirmation {
        return;
    }
    world.signup.state = SignUpState::RunHighPriorityDeclineOperations;
    let key = world.activities.activities[activity].key;
    let queued = signpost::check_state_triggers(world, 6, key);
    signpost::set_activated_machine_count(world, queued);
    // cChallengeBEManager's OnChallengeDecline slot (vtable 0x820539b4) is OnChallengeAccept
    // 0x825b7a40: the pause menu and backpack open again.
    world.pause_menu_disabled = false;
    world.signup.show_render_effect = false;
    world.signup.slow_down = SlowDown::default();
    world.output(Output::PinPlayer { pin: false });
    set_indicators_enabled(world, true);
}

/// The low-priority `OnChallengeAccept`, in `RegisterSubController` order: the DMO manager's
/// (`0x825c3728`), then the effects manager's.
fn low_priority_accept(world: &mut World, activity: usize) {
    super::services::on_accept(world, activity);
    // OnChallengeAccept825ae440; decline shares the same vtable target.
    super::hom::enable_visuals(world, None, true);
    world.hom.signing_up = false;
    dmo::on_challenge_accept(world, activity);
    // cChallengeEffectsManager::OnChallengeAccept 0x82da83e8.
    if challenge_type(world, activity) == challenge_type::OWN_THE_SPOT {
        world.hud.set_visible(true, VISIBILITY_CHALLENGE_EFFECTS);
    } else {
        effects::start_effect(world, effects::FADE_OUT, 0.5);
    }
    world.signup.effects_signing_up = false;
}

/// The low-priority `OnChallengeDecline`.
fn low_priority_decline(world: &mut World, activity: usize) {
    super::services::on_decline(world);
    // OnChallengeAccept825ae440; decline shares the same vtable target.
    super::hom::enable_visuals(world, None, true);
    world.hom.signing_up = false;
    // cChallengeAssetManager::OnChallengeDecline 0x82e0f500.
    assets::unload(world, activity);
    // cChallengeEffectsManager::OnChallengeDecline 0x82da2538.
    world.signup.effects_signing_up = false;
    world.hud.set_visible(true, VISIBILITY_CHALLENGE_EFFECTS);
}

/// The low-priority `IsChallengeAcceptOperationDone`, each asked every update: the challenge's
/// hulls (`AreChallengeHullsLoaded` `0x82e0fa60`), the DMO manager's operations (`0x825c37a8`)
/// and the fade out.
fn low_priority_accept_done(world: &mut World, activity: usize) -> bool {
    let hulls = assets::are_hulls_loaded(world, activity);
    let dmos = dmo::is_challenge_accept_operation_done(world, activity);
    let fade = challenge_type(world, activity) == challenge_type::OWN_THE_SPOT
        || !world.effects.params[effects::FADE_OUT].active
        || world.effects.params[effects::FADE_OUT].stay_persistent;
    hulls && dmos && fade && world.services.allow_cic_accept_to_continue
}

/// `cChallengeSignUpManager::Update` (`0x82db44a0`).
pub(crate) fn update(world: &mut World) {
    if !world.signup.game_loaded {
        return;
    }
    let changed = perform_bbox_tests(world);
    match world.signup.state {
        SignUpState::WaitingForSignUpVolume => {
            world.signup.selected = None;
            world.signup.teleport_start = false;
            if is_allowed_to_sign_up(world) && !world.signup.previous.is_empty() {
                setup_prompt(world);
                show_prompt(world);
                world.signup.state = SignUpState::WaitingForDPadUp;
            }
        }
        SignUpState::WaitingForDPadUp => {
            if !is_allowed_to_sign_up(world) || (changed && world.signup.previous.is_empty()) {
                hide_prompt(world);
                world.signup.state = SignUpState::WaitingForSignUpVolume;
            } else if changed {
                setup_prompt(world);
                output_prompt(world);
            }
        }
        SignUpState::RunHighPriorityDPadUpOperations => {
            if signpost::activated_machine_count(world) == 0
                && let Some(activity) = world.signup.selected
            {
                low_priority_select(world, activity);
                world.signup.state = SignUpState::RunLowPriorityDPadUpOperations;
            }
        }
        SignUpState::RunLowPriorityDPadUpOperations => {
            if let Some(activity) = world.signup.selected {
                world.signup.state = SignUpState::WaitingForConfirmation;
                match challenge_type(world, activity) {
                    challenge_type::CREATE_A_SPOT => world.output(Output::OpenCreateASpotPoster),
                    challenge_type::OWN_THE_SPOT => player_accept(world),
                    _ => {
                        world.signup.show_render_effect = true;
                        let challenge = world.activities.activities[activity].name.clone();
                        world.output(Output::OpenSignUpPoster { challenge });
                    }
                }
            }
        }
        SignUpState::RunHighPriorityAcceptOperations => {
            if signpost::activated_machine_count(world) == 0
                && let Some(activity) = world.signup.selected
            {
                world.signup.state = SignUpState::RunLowPriorityAcceptOperations;
                low_priority_accept(world, activity);
            }
        }
        SignUpState::RunLowPriorityAcceptOperations => {
            if let Some(activity) = world.signup.selected
                && low_priority_accept_done(world, activity)
            {
                world.signup.state = SignUpState::StartChallenge;
            }
        }
        SignUpState::RunHighPriorityDeclineOperations => {
            if let Some(activity) = world.signup.selected {
                world.signup.state = SignUpState::RunLowPriorityDeclineOperations;
                low_priority_decline(world, activity);
            }
        }
        SignUpState::RunLowPriorityDeclineOperations => world.signup.state = SignUpState::WaitingForSignUpVolume,
        SignUpState::StartChallenge => {
            if let Some(activity) = world.signup.selected {
                let key = world.activities.activities[activity].key;
                if world.signup.teleport_start {
                    world.output(Output::TeleportStartChallenge { challenge: key });
                } else {
                    activity::start_main_challenge(world, key);
                }
            }
            world.signup.state = SignUpState::WaitingForSignUpVolume;
        }
        SignUpState::Disabled | SignUpState::WaitingForConfirmation => {}
    }
    // Every state ends at the slow-down: one tick of the slowed rate per
    // update (cMsgSetSimRate on the simulation broadcaster).
    if world.signup.slow_down.enabled {
        let sim_time = SLOW_DOWN_SIM_TIME / world.signup.slow_down.rate;
        world.output(Output::SetSimRate { sim_time, ticks: 1 });
    }
    // Waiting for D-pad up, the colliding sign-ups go to every sub-controller's
    // CollidingSignupChallenges; only the OTS discovery manager's (0x825c09a8) does anything.
    if world.signup.state == SignUpState::WaitingForDPadUp {
        let s = &world.signup;
        let colliding: Vec<usize> =
            s.previous.iter().filter_map(|&i| s.entries.get(i as usize).copied().flatten()).map(|(activity, _)| activity).collect();
        super::ots_discovery::colliding_sign_up_challenges(world, &colliding);
    }
}

/// The sign-up manager's message handler: `HUDIntroDoneMsg` (`0x82da2c98`,
/// `cMsgHUDIntroDone`) slows the world to half speed while the poster
/// waits for confirmation.
pub(crate) fn receive(world: &mut World, event: &Event) {
    if matches!(event, Event::HudIntroDone) && world.signup.state == SignUpState::WaitingForConfirmation {
        world.signup.slow_down = SlowDown { enabled: true, rate: 0.5 };
    }
}

/// `RunMainActivity` (`0x82da1a80`).
pub(crate) fn run_main_activity(world: &mut World) {
    set_indicators_enabled(world, false);
}

/// `CompleteChallenge` (`0x82da1a98`).
pub(crate) fn complete_challenge(world: &mut World) {
    set_indicators_enabled(world, true);
}

/// `IsSigningUp` (`0x82da1ab0`).
#[must_use]
pub fn is_signing_up(world: &World) -> bool {
    !matches!(world.signup.state, SignUpState::Disabled | SignUpState::WaitingForSignUpVolume | SignUpState::WaitingForDPadUp)
}

