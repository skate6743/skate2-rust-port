//! `Sk8::Challenge::cChallengePresentationDirector`: ordered presentation
//! events (fades, waits, speech, HUD changes, progression side effects) played
//! for a challenge, one group at a time. A group with no events left posts
//! `cMsgPresentationEventGroupFinished`.

use super::{
    attrib_db::{AttribDb, Location},
    attrib_lua::{ObjectKind, Pointee},
    binding_table::SK8_CHARACTERS,
    effects, hud,
    keys::hash_value32,
    system::{Event, Field, Message, Output, World},
    timers::TimerKind,
    tutorial_hud,
};

/// `eChallengePresentationEventType`.
pub mod event {
    pub const FADE_IN: u32 = 0;
    pub const FADE_OUT: u32 = 1;
    pub const SPEECH: u32 = 2;
    pub const SPEECH_NO_WAIT: u32 = 3;
    pub const WAIT: u32 = 4;
    pub const NIS: u32 = 5;
    pub const PIN_LOCAL_PLAYER: u32 = 6;
    pub const UNPIN_LOCAL_PLAYER: u32 = 7;
    pub const TELEPORT_LOCAL_PLAYER: u32 = 8;
    pub const SHOW_HUD: u32 = 9;
    pub const HIDE_HUD: u32 = 10;
    pub const ENABLE_SET_SESSION_MARKER: u32 = 11;
    pub const DISABLE_SET_SESSION_MARKER: u32 = 12;
    pub const ENABLE_USE_SESSION_MARKER: u32 = 13;
    pub const DISABLE_USE_SESSION_MARKER: u32 = 14;
    pub const SHOW_MINI_MAP: u32 = 15;
    pub const HIDE_MINI_MAP: u32 = 16;
    pub const REPLAY: u32 = 19;
    pub const SHOW_TUTORIAL_CALLOUT_HUD: u32 = 20;
    pub const HIDE_TUTORIAL_CALLOUT_HUD: u32 = 21;
    pub const CENTRAL_MESSAGE: u32 = 23;
    pub const COMPLETE_CHALLENGE: u32 = 26;
    pub const REVERT_CHALLENGE: u32 = 27;
    pub const ADD_CHALLENGE_MAP_ICON: u32 = 30;
    pub const REMOVE_CHALLENGE_MAP_ICON: u32 = 31;
    pub const SET_DYNAMIC_HULL_STATE: u32 = 33;
    pub const INCREMENT_POINTS_POOL_POINTS: u32 = 35;
}

/// `tChallengePresentationEvent` (12 bytes): type and two string arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationEvent {
    pub kind: u32,
    pub arg1: String,
    pub arg2: String,
}

impl PresentationEvent {
    #[must_use]
    pub fn read(db: &AttribDb, at: Location) -> Option<Self> {
        let bytes = db.bytes(at, 12)?;
        let kind = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let text = |offset: u32| db.text_at(at.offset_by(offset)).unwrap_or_default().to_owned();
        Some(Self { kind, arg1: text(4), arg2: text(8) })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupStatus {
    Idle,
    Running,
    Finished,
}

/// `cChallengePresentationGroup` (`0x84` bytes).
#[derive(Clone, Debug)]
pub struct PresentationGroup {
    pub id: u32,
    pub activity: u64,
    /// `mHUDGroupName`: the group's own HUD group.
    pub hud_group: String,
    pub handle: usize,
    pub events: Vec<PresentationEvent>,
    pub status: GroupStatus,
    pub counter: usize,
}

#[derive(Debug, Default)]
pub struct PresentationDirector {
    next_id: u32,
    pub groups: Vec<PresentationGroup>,
    active: Option<u32>,
    queued: Vec<u32>,
    timer: i32,
    watchdog_timer: i32,
    /// `mSpeechCharacter` and `mSpeechID` of the speech being waited for.
    speech: Option<(i32, i32)>,
    teleport_in_progress: bool,
    /// `mMapIcons`: the map icons the presentation events added, by `GetHashValue32` of the
    /// icon name.
    pub map_icons: std::collections::BTreeMap<u32, u32>,
}

impl PresentationDirector {
    fn group_mut(&mut self, id: u32) -> Option<&mut PresentationGroup> {
        self.groups.iter_mut().find(|g| g.id == id)
    }
}

/// `AddPresentationGroup` (`0x82e07ce8`): the group's HUD group name is
/// `ConvertIToA` of the challenge collection key's low 32 bits with its first
/// character replaced by `p`.
pub fn add_group(world: &mut World, activity: u64) -> usize {
    let director = &mut world.presentation;
    director.next_id += 1;
    let id = director.next_id;
    let handle = world.pointers.handle(Pointee::Object { kind: ObjectKind::PresentationGroup, id });
    #[expect(clippy::cast_possible_truncation, clippy::cast_possible_wrap, reason = "ConvertIToA(int)")]
    let mut hud_group = (activity as i32).to_string();
    hud_group.replace_range(..1, "p");
    world.presentation.groups.push(PresentationGroup {
        id,
        activity,
        hud_group,
        handle,
        events: Vec::new(),
        status: GroupStatus::Idle,
        counter: 0,
    });
    handle
}

/// `cChallengePresentationGroup::AddPresentationEvent` (`0x82e087e0`). PC port: a linked Skate 3
/// data folder's online games run Skate 2's online challenges over their own spots
/// (`crate::skate3`); Skate 2's NIS scenes are cut at San Vanelona's and would film it, so those
/// games play their presentation without them, and with Skate 3's own scenes.
pub fn add_event(world: &mut World, group: u32, event: PresentationEvent) {
    if let Some(g) = world.presentation.group_mut(group) {
        if crate::skate3::linked().is_some_and(|data| data.online_spot(g.activity).is_some()) {
            let skipped = event.kind == self::event::NIS && !crate::skate3::is_scene(&event.arg1);
            if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
                eprintln!("skate 3 game presentation event {} {:?} {:?}{}", event.kind, event.arg1, event.arg2, if skipped { " (Skate 2 scene, skipped)" } else { "" });
            }
            if skipped {
                return;
            }
        }
        g.events.push(event);
    }
}

/// `GetHUDComponentName` (`0x82e06ac0`): `mHUDComponentMap` holds the
/// `GetHashValue32` of each `eHUDComponent` name (ctor `0x82e08a88`).
fn hud_component(name: &str) -> Option<&'static str> {
    const NAMES: [&str; 25] = [
        "TimerHUD", "BigBlackTimerHUD", "CentralHUD", "DistanceMeter", "CountdownHUD", "ObjectiveHUD", "MiniMap",
        "SignUpHUD", "TrickDisplayHUD", "OnlineEventHUD", "HighlightReelHUD", "LeaderboardHUD", "OnlineVotingHUD",
        "OnlineObservingHUD", "OnlineOTSStatHUD", "HOMScoringHUD", "ABDHUD", "DPOHUD", "PhotoTreatmentHUD",
        "TutorialInstructionHUD", "PhonelistHUD", "PreFilmSetupHUD", "ThrowDownMessageHUD", "OnlineChyronHUD",
        "FreeskateActivityOverlayHUD",
    ];
    let hash = super::keys::hash_value32(name);
    let index = NAMES.iter().position(|n| super::keys::hash_value32(n) == hash)?;
    hud::component_name(i32::try_from(index).ok()?)
}

/// `CheckPresentationGroupForHUD` (`0x82e07d80`): the group's HUD group gets
/// the components its Show HUD and Show Tutorial Callout events need.
pub fn check_group_for_hud(world: &mut World, group: u32) {
    let Some(g) = world.presentation.group_mut(group) else { return };
    let name = g.hud_group.clone();
    let components: Vec<&'static str> = g
        .events
        .iter()
        .filter_map(|e| match e.kind {
            event::SHOW_HUD => hud_component(&e.arg1),
            event::SHOW_TUTORIAL_CALLOUT_HUD => Some(tutorial_hud::COMPONENT),
            _ => None,
        })
        .collect();
    for component in components {
        if !world.hud.group_exists(&name) {
            world.hud.create_group(&name, &[component]);
        } else if !world.hud.has_component(&name, component) {
            world.hud.add_component(&name, component);
        }
    }
}

/// `HUDEvent` (`0x82e06bc0`): Show HUD puts the group's HUD group in front.
fn hud_event(world: &mut World, show: bool, group: &str, e: &PresentationEvent) {
    if !world.hud.group_exists(group) {
        return;
    }
    if show {
        world.hud.add_group_to_active(group);
    }
    if let Some(component) = hud_component(&e.arg1) {
        world.hud.show(group, component, show);
    }
}

/// `MiniMapEvent` (`0x82e062c0`): in the career a shown minimap also sends
/// the progression message "Show MiniMap"; the `main` group's minimap
/// follows.
fn minimap_event(world: &mut World, show: bool) {
    // mGameTypeFlags is the director's GameLoad value (0x82e062b8).
    if show && world.activities.game_load_flags == 4 {
        super::signpost::send_message_by_name(world, "Show MiniMap");
    }
    world.hud.show("main", "minimap", show);
}

/// `TutorialCalloutHUDEvent` (`0x82e063c8`): show sets the instruction (arg1)
/// and its HAL id (arg2), puts the group's HUD group in front and shows the
/// component; hide resets the instruction to "none".
fn tutorial_callout_event(world: &mut World, show: bool, group: &str, e: &PresentationEvent) {
    let Some(g) = world.hud.group_mut(group) else { return };
    if let Some(data) = g.tutorial.as_mut() {
        if show {
            data.set_instruction(&e.arg1, &e.arg2);
        } else {
            data.set_instruction("none", "#");
        }
    }
    if show {
        world.hud.add_group_to_active(group);
    }
    world.hud.show(group, tutorial_hud::COMPONENT, show);
}

/// `RemovePresentationGroup` (`0x82e07af8`) / `ShutdownActivity` (`0x82e074f0`):
/// `GroupRemovalHelper` (`0x82e07348`) removes each activity group's HUD as well
/// as its presentation records, then `RemoveAllChallengeMapIcons` runs.
pub fn remove_groups(world: &mut World, activity: u64) {
    let director = &mut world.presentation;
    let mut removed_active = false;
    for group in director.groups.iter().filter(|g| g.activity == activity) {
        removed_active |= director.active == Some(group.id);
        director.queued.retain(|id| *id != group.id);
        // 82e0741c..7424: HUDManager2::DeleteGroup(mHUDGroupName), before
        // destroying the presentation group. Its removal from the active HUD
        // stack triggers FixupActiveComponents and the tutorial's Show(false).
        world.hud.delete_group(&group.hud_group);
    }
    director.groups.retain(|g| g.activity != activity);
    if removed_active {
        director.active = None;
        // 82e074cc..74e4: discard the removed active group's outstanding wait
        // identities. The timer manager owns deleting the timers themselves.
        director.speech = None;
        director.timer = 0;
        director.watchdog_timer = 0;
        director.teleport_in_progress = false;
    }
    super::map_icons::remove_all_presentation_icons(world);
}

/// `StartPresentation` (`0x82e07bc0`).
pub fn start(world: &mut World, group: u32) {
    let director = &mut world.presentation;
    if let Some(active) = director.active {
        if (!director.queued.is_empty() || active != group) && director.queued.last() != Some(&group) && director.queued.len() < 8 {
            director.queued.push(group);
        }
        return;
    }
    let Some(g) = director.group_mut(group) else { return };
    if g.status == GroupStatus::Running && g.counter != 0 {
        g.counter = 0;
    }
    g.status = GroupStatus::Running;
    director.active = Some(group);
    play_next(world, group);
}

/// `ResetPresentationGroup` (`0x82e06338`).
pub fn reset_group(world: &mut World, group: u32) {
    if let Some(g) = world.presentation.group_mut(group) {
        g.status = GroupStatus::Idle;
        g.counter = 0;
    }
}

/// `cCPDSetStateLoadCallback::OnComplete` (`0x82e08a30`): the active group, if any, plays its
/// next event.
pub(crate) fn set_state_load_complete(world: &mut World) {
    if let Some(active) = world.presentation.active {
        play_next(world, active);
    }
}

/// `PlayNextPresentationEvent` (`0x82e07f38`).
pub fn play_next(world: &mut World, group: u32) {
    loop {
        let Some(g) = world.presentation.group_mut(group) else { return };
        if g.status != GroupStatus::Running {
            return;
        }
        if g.counter >= g.events.len() {
            g.counter = 0;
            g.status = GroupStatus::Finished;
            let handle = g.handle;
            world.send(Event::Challenge(Message::new(
                "cMsgPresentationEventGroupFinished",
                vec![("ChallengePresentationGroup", Field::Pointer(handle))],
            )));
            world.presentation.active = None;
            if !world.presentation.queued.is_empty() {
                let next = world.presentation.queued.remove(0);
                start(world, next);
            }
            return;
        }
        let event = g.events[g.counter].clone();
        let hud_group = g.hud_group.clone();
        g.counter += 1;
        if !play_event(world, &event, &hud_group) {
            return;
        }
    }
}

/// One event of `PlayNextPresentationEvent`; `true` continues with the next
/// event at once (`LABEL_60`), `false` waits for its completion message.
fn play_event(world: &mut World, e: &PresentationEvent, hud_group: &str) -> bool {
    match e.kind {
        event::FADE_IN | event::FADE_OUT => {
            let effect = if e.kind == event::FADE_IN { effects::FADE_IN } else { effects::FADE_OUT };
            effects::start_effect(world, effect, parse_float(&e.arg1));
            false
        }
        event::SPEECH | event::SPEECH_NO_WAIT => {
            let Some(character) = speech_character(&e.arg1) else {
                // The native asserts (challengepresentationdirector.cpp:452);
                // every shipped speech event names a mapped character.
                world.output(Output::Unpresented { message: format!("speech character {}", e.arg1), kind: -1 });
                return true;
            };
            #[expect(clippy::cast_possible_truncation, reason = "atol")]
            let id = parse_float(&e.arg2) as i32;
            world.output(Output::PlayNpcSpeech { character, id, param: 0 });
            if e.kind == event::SPEECH {
                world.presentation.speech = Some((character, id));
                world.presentation.watchdog_timer = world.timers.add(TimerKind::CountDown, 10.0, None);
                false
            } else {
                world.presentation.speech = None;
                true
            }
        }
        event::WAIT => {
            world.presentation.timer = world.timers.add(TimerKind::CountDown, parse_float(&e.arg1), None);
            false
        }
        event::NIS => {
            // PlayNextPresentationEvent (0x82e07f38): "True" as the second
            // argument skips the NIS's breadcrumb teleport; then
            // RunBasicNIS(arg1, TRANSITION_TYPE_NONE, false, true). The group
            // waits for the NIS's cMsgNISChallengeNotify(1).
            if e.arg2.get(..4).is_some_and(|s| s.eq_ignore_ascii_case("true")) {
                world.output(Output::IgnoreTeleportToBreadcrumbOnNextNis);
            }
            world.output(Output::PlayNis { message: String::new(), scene: e.arg1.clone(), transition: 0 });
            false
        }
        event::PIN_LOCAL_PLAYER | event::UNPIN_LOCAL_PLAYER => {
            // PinEvent 0x82e06b70: cMsgPinPlayer.
            world.output(Output::PinPlayer { pin: e.kind == event::PIN_LOCAL_PLAYER });
            true
        }
        event::SHOW_HUD | event::HIDE_HUD => {
            hud_event(world, e.kind == event::SHOW_HUD, hud_group, e);
            true
        }
        event::SHOW_MINI_MAP | event::HIDE_MINI_MAP => {
            minimap_event(world, e.kind == event::SHOW_MINI_MAP);
            true
        }
        event::REPLAY => {
            // PlayNextPresentationEvent 0x82e08394..0x82e083e4: RunReplayNIS(arg2, the replay
            // recorder's last frame less 60 frames for each of atol(arg1) seconds, that frame,
            // TRANSITION_TYPE_NONE, no camera hold, skippable, the default playback overrides,
            // NIS_SPECIALFLAG_NONE).
            // The group waits for the scene's cMsgNISChallengeNotify(1). An online game plays
            // no replay (the port's choice, as with its highlight reels): the group goes on.
            if world.online.is_some() {
                return true;
            }
            let last = world.replay.last_frame_number_written();
            let start = last.wrapping_sub(atol(&e.arg1).wrapping_mul(60).cast_unsigned());
            world.output(Output::RunReplayNis {
                scene: e.arg2.clone(),
                start,
                end: last,
                hold_camera: false,
                skip_enabled: true,
                focus_player: -1,
                location: None,
                stream_position: None,
                special_flags: crate::nis::manager::SPECIAL_FLAG_NONE,
            });
            false
        }
        event::SHOW_TUTORIAL_CALLOUT_HUD | event::HIDE_TUTORIAL_CALLOUT_HUD => {
            tutorial_callout_event(world, e.kind == event::SHOW_TUTORIAL_CALLOUT_HUD, hud_group, e);
            true
        }
        event::TELEPORT_LOCAL_PLAYER => {
            let player = world.local_player;
            super::teleport::teleport_player(world, player, &e.arg1, 0, false, 0.5, false);
            world.presentation.teleport_in_progress = true;
            false
        }
        event::ENABLE_SET_SESSION_MARKER => {
            super::signpost::send_message_by_name(world, "Show Set Session Marker");
            true
        }
        event::DISABLE_SET_SESSION_MARKER => {
            super::signpost::send_message_by_name(world, "Hide Set Session Marker");
            true
        }
        event::ENABLE_USE_SESSION_MARKER => {
            super::signpost::send_message_by_name(world, "Show Use Session Marker");
            true
        }
        event::DISABLE_USE_SESSION_MARKER => {
            super::signpost::send_message_by_name(world, "Hide Use Session Marker");
            true
        }
        event::CENTRAL_MESSAGE => {
            // CentralHUDData::DataShowCentralMessage (0x82d82b30) on main's
            // central data, empty arguments; CentralHUD::Update (0x826bc398)
            // shows it with the default audio hook.
            if world.hud.group("main").is_some() {
                let message = hud::CentralMessage { hal_id: e.arg1.clone(), colour: 0, args: Default::default(), audio_hook: 0 };
                world.output(Output::CentralMessage(message.clone()));
                world.hud.central_messages.push(message);
            }
            true
        }
        event::COMPLETE_CHALLENGE => {
            super::signpost::complete_challenge_by_name(world, &e.arg1);
            true
        }
        event::REVERT_CHALLENGE => {
            super::signpost::revert_challenge_by_name(world, &e.arg1);
            true
        }
        event::ADD_CHALLENGE_MAP_ICON | event::REMOVE_CHALLENGE_MAP_ICON => {
            // RemoveChallengeMapIcon (0x82e06c38) drops the icon held under
            // GetHashValue32(arg1); an add then asks the map back end for
            // a new one at the location arg2.
            let location = (e.kind == event::ADD_CHALLENGE_MAP_ICON).then(|| e.arg2.clone());
            match &location {
                Some(location) => super::map_icons::add_presentation_icon(world, &e.arg1, location),
                None => super::map_icons::remove_presentation_icon(world, &e.arg1),
            }
            world.output(Output::ChallengeMapIcon { icon: e.arg1.clone(), location });
            true
        }
        event::SET_DYNAMIC_HULL_STATE => {
            // SetDynamicHullState(StringToKey(arg1), arg2, cCPDSetStateLoadCallback): the group
            // waits for the new state's hull, whose callback plays the next event.
            let key = super::keys::hash64(&e.arg1);
            super::dynamic_hull::set_state(world, key, &e.arg2, Some(super::dynamic_hull::LoadCallback::PresentationDirector));
            false
        }
        event::INCREMENT_POINTS_POOL_POINTS => {
            #[expect(clippy::cast_possible_truncation, reason = "atol")]
            let value = parse_float(&e.arg2) as i32;
            super::signpost_progress::increment_points_by_name(world, &e.arg1, value);
            true
        }
        _ => {
            world.output(Output::Unpresented { message: format!("presentation event {} {} {}", e.kind, e.arg1, e.arg2), kind: -1 });
            true
        }
    }
}

/// `eSk8Character_CameraMan`.
pub(crate) const CAMERA_MAN: i32 = 40;

/// `mSpeechCharacterMap`: the `eSk8Characters` value whose name has the
/// same `GetHashValue32`.
fn speech_character(name: &str) -> Option<i32> {
    let hash = hash_value32(name);
    #[expect(clippy::cast_possible_truncation, reason = "enum values")]
    SK8_CHARACTERS.iter().find(|(n, _)| hash_value32(n) == hash).map(|&(_, v)| v as i32)
}

fn parse_float(s: &str) -> f32 {
    s.trim().parse().unwrap_or(0.0)
}

/// `atol`: leading white space, a sign, then the decimal digits ("3.5" reads 3).
fn atol(s: &str) -> i32 {
    let s = s.trim_start();
    let (negative, digits) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let value = digits
        .bytes()
        .take_while(u8::is_ascii_digit)
        .fold(0i32, |value, digit| value.wrapping_mul(10).wrapping_add(i32::from(digit - b'0')));
    if negative { value.wrapping_neg() } else { value }
}

/// The director's Hermes handlers (`OnTimerExpired`, `OnEffectFinished`,
/// `OnNISChallengeNotify`, `OnSpeechEnded`, `OnTeleportTransitionFinished`).
pub fn receive(world: &mut World, event: &Event) {
    let Some(active) = world.presentation.active else { return };
    match event {
        Event::TimerExpired { timer } => {
            if *timer == world.presentation.timer {
                world.presentation.timer = 0;
                play_next(world, active);
            } else if *timer == world.presentation.watchdog_timer {
                world.presentation.speech = None;
                world.presentation.watchdog_timer = 0;
                play_next(world, active);
            }
        }
        Event::NisChallengeNotify { notification: 1 } => play_next(world, active),
        Event::SpeechEnded { character, id } => {
            // OnSpeechEnded 0x82e08870.
            if world.presentation.speech == Some((*character, *id)) {
                world.presentation.speech = None;
                let watchdog = std::mem::take(&mut world.presentation.watchdog_timer);
                world.timers.remove(watchdog);
                play_next(world, active);
            }
        }
        Event::Challenge(m) if m.kind == super::keys::hash32("cMsgEffectFinished") => {
            let effect = m.fields.as_ref().and_then(|f| f.first()).map(|(_, v)| v.clone());
            let Some(g) = world.presentation.groups.iter().find(|g| g.id == active) else { return };
            let Some(last) = g.counter.checked_sub(1).and_then(|i| g.events.get(i)) else { return };
            #[expect(clippy::cast_precision_loss, reason = "enum value")]
            let wanted = match last.kind {
                event::FADE_IN => Field::Number(effects::FADE_IN as f32),
                event::FADE_OUT => Field::Number(effects::FADE_OUT as f32),
                _ => return,
            };
            if effect == Some(wanted) {
                play_next(world, active);
            }
        }
        Event::Challenge(m) if m.kind == super::keys::hash32("cMsgTeleportTransitionFinished") => {
            // OnTeleportTransitionFinished 0x82e08a00.
            if world.presentation.teleport_in_progress {
                world.presentation.teleport_in_progress = false;
                play_next(world, active);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::atol;

    #[test]
    fn a_replays_seconds_read_as_atol() {
        assert_eq!(atol("3.5"), 3);
        assert_eq!(atol("4.5"), 4);
        assert_eq!(atol(" -12x"), -12);
        assert_eq!(atol(".5"), 0);
    }
}
