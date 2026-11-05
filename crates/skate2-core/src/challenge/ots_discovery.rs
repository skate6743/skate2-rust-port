//! `Sk8::Challenge::cChallengeOtsDiscoveryManager`: the Own the Spot challenges a career skater
//! discovers. Each Own the Spot challenge with a discovery radius gets a box at its `Location`;
//! while the local tester is inside it, and a main challenge could start, the challenge's assets
//! load, so its `OTSTriggerBoundary` volume exists. Entering that volume activates the challenge
//! in the progression and sends `cMsgEnterOTSChallengeBoundary`, which the
//! `ots_discovery_sp_flow_manager` global answers; a spot not found before holds the sign-ups
//! back for four seconds. Leaving the box unloads the assets and sends
//! `cMsgExitDiscoveryVolume`. As a sign-up sub-controller the manager speaks for the spot film
//! challenges whose sign-up boxes the skater stands in.

use super::{
    activity::{self, ActivityTree},
    assets,
    attrib_lua::{ObjectKind, Pointee},
    keys::{class, hash64, hash_value32},
    signpost, signpost_progress,
    system::{Event, Field, Message, Output, World},
    timers::TimerKind,
};

/// `challenge_global_data.ChallengeType`.
const CHALLENGE_TYPE: u64 = 0x02b8_28cc_db47_85e4;
/// `challenge_global_data`: an activity only debug builds create.
const DEBUG_ONLY: u64 = 0xfd40_e4a3_1027_c984;
/// `challenge_global_data.Location` (`tLocationID`).
const LOCATION: u64 = 0xf1c6_d5fa_4a23_951e;
/// `challenges`: the discovery box's half size, which must be positive.
const DISCOVERY_RADIUS: u64 = 0x0f13_6b32_f79b_4a15;
/// `challenges.OTSTriggerBoundary`: the boundary volume, its GUID at +8.
const OTS_TRIGGER_BOUNDARY: u64 = 0xc2a0_54ab_f59e_414c;
/// `eChallengeTypes_OwnTheSpot` and `eChallengeTypes_SpotFilm`.
const OWN_THE_SPOT: i32 = 9;
const SPOT_FILM: i32 = 40;
/// `AddCountdownTimer(4.0)`: the sign-up hold after a new spot.
const DISALLOW_SIGN_UP_SECONDS: f32 = 4.0;

#[derive(Debug)]
pub struct OtsDiscoveryManager {
    game_loaded: bool,
    /// `AllDiscoveryChallenges`.
    all: Vec<usize>,
    /// `OTSActivitesTree`.
    tree: ActivityTree,
    /// `PreviousCollisionIndicies`.
    previous: Vec<u32>,
    /// `DiscoveryVolumeList`: the challenges whose boxes the tester is in, their assets loading.
    discovery: Vec<usize>,
    /// `AudioQueueVector`, ascending: the spot film challenges spoken for.
    spoken: Vec<usize>,
    /// `mAllowSignUp`.
    allow_sign_up: bool,
    /// `mDisallowSignUpTimerID`.
    disallow_timer: i32,
}

impl Default for OtsDiscoveryManager {
    /// The constructor (`0x825c1b80`): sign-ups allowed.
    fn default() -> Self {
        Self {
            game_loaded: false,
            all: Vec::new(),
            tree: ActivityTree::default(),
            previous: Vec::new(),
            discovery: Vec::new(),
            spoken: Vec::new(),
            allow_sign_up: true,
            disallow_timer: 0,
        }
    }
}

fn global_attribute(world: &World, activity: usize, key: u64) -> Option<super::attrib_db::Attribute<'_>> {
    let (class, collection) = world.activities.activities[activity].global_data?;
    world.db.attribute(class, collection, key)
}

fn challenge_type(world: &World, activity: usize) -> i32 {
    global_attribute(world, activity, CHALLENGE_TYPE).and_then(|a| a.i32(0)).unwrap_or(0)
}

fn discovery_radius(world: &World, activity: usize) -> f32 {
    let key = world.activities.activities[activity].key;
    world.db.attribute(class::CHALLENGES, key, DISCOVERY_RADIUS).and_then(|a| a.f32(0)).unwrap_or(0.0)
}

/// The activity as the light userdata the discovery messages carry.
fn activity_pointer(world: &mut World, activity: usize) -> Field {
    let id = u32::try_from(activity).expect("activity count");
    Field::Pointer(world.pointers.handle(Pointee::Object { kind: ObjectKind::Activity, id }))
}

/// `cActivityManager::CreateActivity` (`0x82cdbca8`) passes every activity that is not
/// debug-only to `AddActivity` (`0x825c0600`), which keeps the Own the Spot challenges with a
/// positive discovery radius.
pub(crate) fn init(world: &mut World) {
    for activity in 0..world.activities.activities.len() {
        let debug_only = global_attribute(world, activity, DEBUG_ONLY).and_then(|a| a.bool(0)).unwrap_or(false);
        if !debug_only && challenge_type(world, activity) == OWN_THE_SPOT && discovery_radius(world, activity) > 0.0 {
            world.ots_discovery.all.push(activity);
        }
    }
}

/// The challenge's value of a `challenges` attribute, its first `len` bytes.
fn challenge_bytes(world: &World, activity: usize, key: u64, len: usize) -> Option<Vec<u8>> {
    let challenge = world.activities.activities[activity].key;
    world.db.attribute(class::CHALLENGES, challenge, key)?.value_bytes(0).filter(|bytes| bytes.len() >= len)
}

/// `GameLoad(gameTypeFlags)` (`0x825b6a30`): in the career each discovery challenge with a
/// `Location` gets its box (`GenerateLocationActivityBBox` at `GetLocationPosition`, the default
/// position for an unknown name), the manager is loaded and sign-ups allowed.
pub(crate) fn game_load(world: &mut World, flags: u32) {
    if flags == 4 {
        for activity in world.ots_discovery.all.clone() {
            let Some(name) = global_attribute(world, activity, LOCATION).and_then(|a| a.text(0)) else { continue };
            let position = world.location(hash_value32(name)).map_or(world.default_location, |t| [t[12], t[13], t[14]]);
            let bounds = activity::generate_location_activity_bbox(discovery_radius(world, activity), position);
            world.ots_discovery.tree.insert(activity, bounds);
        }
        let manager = &mut world.ots_discovery;
        manager.game_loaded = true;
        manager.allow_sign_up = true;
        manager.disallow_timer = 0;
    }
}

/// `Update` (`0x825c3990`): `PerformBBoxTest` (`0x825c2c78`) while loaded and a main challenge
/// could start. A box the local tester left unloads its challenge's assets and sends
/// `cMsgExitDiscoveryVolume`; each box it is in, while a main challenge could start, loads its
/// challenge's assets once (registered on load, normal priority).
pub(crate) fn update(world: &mut World) {
    if !world.ots_discovery.game_loaded || !activity::can_start_main_challenge(world) {
        return;
    }
    let Some(base) = world.player.tester_base else { return };
    let current = world.ots_discovery.tree.collisions(super::characters::tester_bounds(base));
    let (_, exited) = super::trigger_tester::enter_exit(&world.ots_discovery.previous, &current);
    for index in exited {
        let Some(activity) = world.ots_discovery.tree.entry(index) else { continue };
        if let Some(at) = world.ots_discovery.discovery.iter().position(|&a| a == activity) {
            assets::unload(world, activity);
            world.ots_discovery.discovery.remove(at);
            let pointer = activity_pointer(world, activity);
            world.send(Event::Challenge(Message::new("cMsgExitDiscoveryVolume", vec![("Activity", pointer)])));
        }
    }
    for &index in &current {
        let Some(activity) = world.ots_discovery.tree.entry(index) else { continue };
        if activity::can_start_main_challenge(world) && !world.ots_discovery.discovery.contains(&activity) {
            world.ots_discovery.discovery.push(activity);
            assets::load_challenge_assets(world, activity, true, false, 0);
        }
    }
    world.ots_discovery.previous = current;
}

/// The manager's Hermes handlers. `ReceiveTriggerEnterCollisionMessage` (`0x825c0720`): the local
/// player entering a discovery challenge's `OTSTriggerBoundary` (its GUID) activates the challenge
/// (`ActivateChallenge`: new, activities evaluated) and sends `cMsgEnterOTSChallengeBoundary`; a
/// spot whose `Spot Found` save value is still 0 holds the sign-ups back for four seconds.
/// `cOnlineLeaderboardManager::RequestOTSInfo` only asks while connected online. `OnTimerExpired`
/// (`0x825b49f8`) ends the hold.
pub(crate) fn receive(world: &mut World, event: &Event) {
    match *event {
        Event::TriggerCollision { enter: true, volume, player } if player == world.local_player => {
            for activity in world.ots_discovery.discovery.clone() {
                let guid = challenge_bytes(world, activity, OTS_TRIGGER_BOUNDARY, 16)
                    .map_or(0, |bytes| u64::from_be_bytes(bytes[8..16].try_into().expect("16 bytes")));
                if guid != volume {
                    continue;
                }
                let key = world.activities.activities[activity].key;
                if let Some(signpost) = signpost::signpost_key_for_challenge(world, key) {
                    signpost::activate_challenge(world, signpost, true, false, true);
                }
                let pointer = activity_pointer(world, activity);
                world.send(Event::Challenge(Message::new("cMsgEnterOTSChallengeBoundary", vec![("Activity", pointer)])));
                if signpost_progress::challenge_save_data(world, key, "Spot Found") == 0 {
                    world.ots_discovery.allow_sign_up = false;
                    world.ots_discovery.disallow_timer = world.timers.add(TimerKind::CountDown, DISALLOW_SIGN_UP_SECONDS, None);
                }
            }
        }
        Event::TimerExpired { timer } if timer == world.ots_discovery.disallow_timer => {
            world.ots_discovery.allow_sign_up = true;
            world.ots_discovery.disallow_timer = 0;
        }
        _ => {}
    }
}

/// `IsSignupAvailable` (`0x825af878`): `mAllowSignUp`.
pub(crate) fn is_signup_available(world: &World) -> bool {
    world.ots_discovery.allow_sign_up
}

/// `CollidingSignupChallenges` (`0x825c09a8`), each update the sign-up manager waits for D-pad up:
/// a spot film challenge among the colliding sign-ups that has not been spoken for marks its
/// `Spot Found` and plays its `AudioSpotFound_spotfilm` speech, or once found its
/// `AudioSpotRemind_spotfilm` (`ChallengeAudioManager::PlaySpeech`, param -1), and is spoken for.
pub(crate) fn colliding_sign_up_challenges(world: &mut World, challenges: &[usize]) {
    for &activity in challenges {
        if challenge_type(world, activity) != SPOT_FILM || world.ots_discovery.spoken.binary_search(&activity).is_ok() {
            continue;
        }
        let key = world.activities.activities[activity].key;
        let speech = if signpost_progress::challenge_save_data(world, key, "Spot Found") == 0 {
            signpost_progress::set_challenge_save_data(world, key, "Spot Found", 1);
            "AudioSpotFound_spotfilm"
        } else {
            "AudioSpotRemind_spotfilm"
        };
        if let Some(bytes) = challenge_bytes(world, activity, hash64(speech), 8) {
            let character = i32::from_be_bytes(bytes[0..4].try_into().expect("8 bytes"));
            let id = i32::from(i16::from_be_bytes(bytes[4..6].try_into().expect("8 bytes")));
            world.output(Output::PlayNpcSpeech { character, id, param: -1 });
        }
        let at = world.ots_discovery.spoken.binary_search(&activity).unwrap_or_else(|at| at);
        world.ots_discovery.spoken.insert(at, activity);
    }
}
