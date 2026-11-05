//! The natives multi-skater challenges use to read and drive the skaters: the players back
//! end (`BackEndManager::mBEModules[2]`) as the [`PlayersBackEnd`] the session installs, the
//! skater manager's skaters (their completed outputs), turns (`cMsgSetPlayer`), the camera's
//! skater (`SkaterManager::SetLocalIndex`), pins (`cMsgPinPlayer`) and the press-A dialog.

use super::{
    attrib_lua::{ObjectKind, Pointee},
    binding::{get_bool, get_bool_or, get_float_or, get_int, get_int_or, int},
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    skate::skater,
    system::{Event, Field, Message, Output, World},
};
use crate::nis::manager::{SPECIAL_FLAG_FINAL_HIGHLIGHT, SPECIAL_FLAG_HALL_OF_MEAT, SPECIAL_FLAG_NONE};

type R = LuaResult<Vec<Value>>;

fn none() -> R {
    Ok(Vec::new())
}

/// `PT_Human`.
pub const PT_HUMAN: i32 = 1;
/// `GM_SinglePlayer`.
pub const GM_SINGLE_PLAYER: i32 = 1;
/// `GM_PartyPlay`.
pub const GM_PARTY_PLAY: i32 = 2;
/// `GM_ProfileFreeSkate`: free skate from the main menu.
pub const GM_PROFILE_FREE_SKATE: i32 = 3;
/// `GM_OnlineFreeSkate`.
pub const GM_ONLINE_FREE_SKATE: i32 = 4;
/// `GM_OnlineChallenge`.
pub const GM_ONLINE_CHALLENGE: i32 = 5;

/// `Simulation::GetLocalPlayerID` (`0x8289e458`) is zero outside a live
/// network game, including offline Party Play. Unlike Online.GetLocalPlayerID
/// (`0x8288a570`), it does not follow SkaterManager's spectator camera index.
pub(crate) const OFFLINE_LOCAL_PLAYER_ID: i32 = 0;

/// `ChallengeOnlineManager` and the network game's players while an online challenge runs
/// (`Simulation::IsNetworkGame`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OnlineChallenge {
    /// `ChallengeOnlineManager::GetLocalPlayerIndex` (`Simulation::GetLocalPlayerID`): this
    /// console's player.
    pub local_player: i32,
    /// The players with a peer id.
    pub players: i32,
    /// `PlayersBackEnd::GetPlayerIsDisconnected`, by player index.
    pub disconnected: [bool; 8],
    /// The player whose turn a turn-based challenge plays: the last one `AI.RepositionAI` moved
    /// with the camera (`setCamera`), its argument the script's turn player on every console.
    pub turn_player: Option<i32>,
    /// The players `AI.RepositionAI` teleported whose output has not yet finished the teleport.
    pub teleporting: [bool; 8],
    /// The round tick the challenge runs, the same on every console.
    pub tick: u32,
}

/// `Simulation::GetLocalPlayerID` (`0x8289e458`): the network game's local player, else
/// [`OFFLINE_LOCAL_PLAYER_ID`].
pub(crate) fn network_local_player(world: &World) -> i32 {
    world.online.as_ref().map_or(OFFLINE_LOCAL_PLAYER_ID, |online| online.local_player)
}

/// `Sk8::BE::PlayersBackEnd` as challenge natives use it; every index is a player index
/// below 9.
pub trait PlayersBackEnd: Send + Sync {
    /// `GetPlayerType` (`0x8255aa60`).
    fn player_type(&self, player: i32) -> i32;
    /// `GetPlayerName` (`0x8255a728`): `mName`.
    fn player_name(&self, player: i32) -> String;
    /// `IsPlayerStanceRegular` (`0x8255b500`).
    fn is_stance_regular(&self, player: i32) -> bool;
    /// `GetPlayerPeerID` (`0x8255aba8`).
    fn peer_id(&self, player: i32) -> i64;
    /// `SetManualsAreConnectors` (`0x8255ac18`).
    fn set_manuals_are_connectors(&self, player: i32, value: bool);
    /// `SetResetSequence` (`0x8255ace8`).
    fn set_reset_sequence(&self, player: i32, value: bool);
    /// `SetResetScoring` (`0x8255adc8`).
    fn set_reset_scoring(&self, player: i32, value: bool);
    /// `GetPlayerCharacter`: the player's character key, `u64::MAX` without one.
    fn character(&self, player: i32) -> u64;
    /// `GetNumPlayers` (`0x8255aa58`, through the `IPlayerInfo` base): `mTotalNumberOfPlayers`,
    /// which `CreatePlayer` (`0x82563d50`) counts.
    fn num_players(&self) -> i32;
    /// `SetPlayerRank` (`0x8255a538`).
    fn set_player_rank(&self, player: i32, rank: i32);
}

fn backend(world: &World) -> Option<&dyn PlayersBackEnd> {
    world.players.as_deref()
}

fn player_type(world: &World, player: i32) -> i32 {
    backend(world).map_or(0, |b| b.player_type(player))
}

// ---- ChallengeManager -----------------------------------------------------

/// `ChallengeManager.SetPlayer(player)` (`0x828954c8`): `cMsgSetPlayer` on the simulation
/// broadcaster.
fn set_player(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.output(Output::SetPlayer { player: get_int(call, 0) });
    none()
}

/// `ChallengeManager.GetNumHumanSkaters()` (`0x82882650`): the `PT_Human` rows of the first
/// eight players.
fn get_num_human_skaters(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let count = (0..8).filter(|&i| player_type(world, i) == PT_HUMAN).count();
    Ok(vec![int(i32::try_from(count).unwrap_or(0))])
}

/// `ChallengeManager.GetSkaterCount()` (`0x828825f8`): `PlayersBackEnd::GetNumPlayers`.
fn get_skater_count(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(backend(world).map_or(0, PlayersBackEnd::num_players))])
}

/// `ChallengeManager.SetManualsAreConnectors(value, player)` (`0x828824c0`).
fn set_manuals_are_connectors(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let (value, player) = (get_bool(call, 0), get_int(call, 1));
    if let Some(b) = backend(world) {
        b.set_manuals_are_connectors(player, value);
    }
    none()
}

/// `ChallengeManager.SetResetSequence(value, player)` (`0x82882528`).
fn set_reset_sequence(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let (value, player) = (get_bool(call, 0), get_int(call, 1));
    if let Some(b) = backend(world) {
        b.set_reset_sequence(player, value);
    }
    none()
}

/// `ChallengeManager.SetResetScoring(value, player)` (`0x82882590`).
fn set_reset_scoring(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let (value, player) = (get_bool(call, 0), get_int(call, 1));
    if let Some(b) = backend(world) {
        b.set_reset_scoring(player, value);
    }
    none()
}

// ---- Entity ---------------------------------------------------------------

/// `Entity.DoesSkaterExist(player)` (`0x82887068`): `SkaterManager::FindByIndex`.
fn does_skater_exist(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    Ok(vec![Value::Boolean(skater(world, get_int(call, 0)).is_some())])
}

/// `Entity.IsPlayerStanceRegular(player)` (`0x8287f2c0`).
fn is_player_stance_regular(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int(call, 0);
    let regular = backend(world).map_or(world.stance_regular, |b| b.is_stance_regular(player));
    Ok(vec![Value::Boolean(regular)])
}

// ---- Online ---------------------------------------------------------------

/// `Online.IsOnline()` (`0x8288a468`): `Simulation::IsNetworkGame`.
fn is_online(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Boolean(world.online.is_some())])
}

/// `Online.GetNumOnlineSkaters()` (`0x8288a6c0`): players with a peer, in a network game only.
fn get_num_online_skaters(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(world.online.as_ref().map_or(0, |online| online.players))])
}

/// `Online.IsPlayerDisconnected(player)` (`0x8288a768`): `PlayersBackEnd::
/// GetPlayerIsDisconnected` in a network game, false outside one.
fn is_player_disconnected(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = usize::try_from(get_int(call, 0)).ok();
    let disconnected = world.online.as_ref().zip(player).is_some_and(|(online, player)| online.disconnected.get(player).copied().unwrap_or(false));
    Ok(vec![Value::Boolean(disconnected)])
}

/// `Online.SwitchCamera(player)` (`0x8288a4e0`): `SkaterManager::SetLocalIndex`.
fn switch_camera(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int(call, 0);
    world.local_player = player;
    world.output(Output::SetLocalIndex { player });
    none()
}

// ---- AI -------------------------------------------------------------------

/// `AI.PinAI(player)`/`AI.UnpinAI(player)` (`0x82895d78`/`0x82895f38`): a player with a row
/// and a skater; an AI-driven skater's Actor flag `0x4000000` changes at once, a human's
/// through `cMsgPinPlayer` to `PlayerUI::ReceivePinMsg` (`0x828acc18`: command 7 on the
/// UI's current player). Online only the console's own player is pinned
/// (`ChallengeOnlineManager::GetLocalPlayerIndex`); its owner pins each other one.
fn pin(world: &mut World, player: i32, pin: bool) {
    if player_type(world, player) == 0 || skater(world, player).is_none() {
        return;
    }
    if player_type(world, player) == PT_HUMAN {
        if world.online.as_ref().is_none_or(|online| online.local_player == player) {
            world.output(Output::PinPlayer { pin });
        }
    } else {
        world.output(Output::PinActor { player, pin });
    }
}

fn pin_ai(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    pin(world, get_int(call, 0), true);
    none()
}

fn unpin_ai(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    pin(world, get_int(call, 0), false);
    none()
}

// ---- Challenge ------------------------------------------------------------

/// `Challenge.GetGameMode()` (`0x82888b28`): `sGameInfo.gameMode`, with no-profile free
/// skate and the debug front end read as single player.
fn get_game_mode(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let mode = if matches!(world.game_mode, 0 | 7) { GM_SINGLE_PLAYER } else { world.game_mode };
    Ok(vec![int(mode)])
}

/// `Challenge.GetPlayerName(player = 0)` (`0x828809e0`).
fn get_player_name(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int_or(call, 0, 0);
    let name = backend(world).map(|b| b.player_name(player)).unwrap_or_default();
    Ok(vec![Value::from(name.as_str())])
}

/// `Challenge.GetChallengeType(data)` (`0x82880890`): the activity's
/// `challenge_global_data.ChallengeType`.
fn get_challenge_type(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(activity) = super::attrib_lua::activity_of(lua, call.arg(0)) else { return Ok(vec![int(0)]) };
    Ok(vec![int(super::signup::challenge_type(world, activity))])
}

#[expect(clippy::cast_precision_loss, reason = "the messages' (float) fields")]
fn number(value: i32) -> Field {
    Field::Number(value as f32)
}

/// `Challenge.SendPlayerOverallResult(player, rank)` (`0x82894e68`): `cMsgPlayerOverallResult`.
fn send_player_overall_result(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let fields = vec![("PlayerID", number(get_int(call, 0))), ("OverallRank", number(get_int(call, 1)))];
    world.send(Event::Challenge(Message::new("cMsgPlayerOverallResult", fields)));
    none()
}

/// `Challenge.SendPlayerRoundResult(player, round, rank, score)` (`0x82894d80`):
/// `cMsgPlayerResult`.
fn send_player_round_result(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let fields = vec![
        ("PlayerID", number(get_int(call, 0))),
        ("Round", number(get_int(call, 1))),
        ("Rank", number(get_int(call, 2))),
        ("Score", number(get_int(call, 3))),
    ];
    world.send(Event::Challenge(Message::new("cMsgPlayerResult", fields)));
    none()
}

/// `Challenge.NotifyChallengeCompleteForObjectives()` (`0x82894f28`):
/// `cMsgChallengeCompleteForObjectives`.
fn notify_challenge_complete_for_objectives(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.send(Event::Challenge(Message::new("cMsgChallengeCompleteForObjectives", Vec::new())));
    none()
}

/// `Online.OnlineChallengeComplete(ranking, score, time, playersChoice = false, finishReason =
/// 1)` (`0x8289abc0`): `MESSAGE_ChallengeComplete` to the flow, in a network game only.
///
/// PC port: Skate 3's (`0x82846878`, its 1-Up's `show_results`) takes the final results instead,
/// a row per player: `{ player, rank, score, time, playersChoice, npc, finishReason, ... }`, and
/// sends every row; each console here reports its own, so the local player's row is the result.
fn online_challenge_complete(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if world.online.is_none() {
        return none();
    }
    if let Value::Table(results) = call.arg(0).clone() {
        let rows: Vec<Value> = (1..=lua.table(results).len()).map(|index| lua.table(results).get_int(index)).collect();
        let local = world.local_player;
        let row = rows.into_iter().find_map(|row| {
            let Value::Table(row) = row else { return None };
            let field = |index| lua.table(row).get_int(index);
            let number = |value: Value| if let Value::Number(n) = value { Some(n) } else { None };
            #[expect(clippy::cast_possible_truncation, reason = "the rows' integer fields")]
            let int = |index| number(field(index)).map(|n| n as i32);
            (int(1)? == local).then(|| (int(2), int(3), number(field(4)), matches!(field(5), Value::Boolean(true)), int(7)))
        });
        if let Some((ranking, score, time, players_choice, finish_reason)) = row {
            world.output(Output::OnlineChallengeComplete {
                ranking: ranking.unwrap_or(0),
                score: score.unwrap_or(0),
                time: time.unwrap_or(0.0),
                players_choice,
                finish_reason: finish_reason.unwrap_or(1),
            });
        }
        return none();
    }
    world.output(Output::OnlineChallengeComplete {
        ranking: get_int(call, 0),
        score: get_int(call, 1),
        time: get_float_or(call, 2, 0.0),
        players_choice: get_bool_or(call, 3, false),
        finish_reason: get_int_or(call, 4, 1),
    });
    none()
}

/// PC port: Skate 3's `HighlightReel.GetNumberOfHighlights(group)` (`0x82bcd1f0`): the group's
/// highlights.
pub(super) fn get_number_of_highlights(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let count = highlight_group(world, call.arg(0)).and_then(|group| world.highlight_groups.iter().find(|g| g.id == group)).map_or(0, |g| g.highlights.len());
    Ok(vec![int(i32::try_from(count).unwrap_or(i32::MAX))])
}

/// `Online.LeaveGame()` (`0x8289aaa0`): `MESSAGE_ChallengeLeaveGame` (18), which
/// `StateOnlineGame::FilterImpl` (`0x827711a0`) takes as the challenge's clean-up being done and
/// `StateMainGameFlow::FilterImpl` (`0x8277a608`) drops outside a network game.
fn online_leave_game(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    if world.online.is_some() {
        world.output(Output::OnlineChallengeCleanupDone);
    }
    none()
}

/// `Challenge.GetTeleportFadeInProgress()` (`0x82888b80`).
fn get_teleport_fade_in_progress(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Boolean(world.teleport.fade_in_progress)])
}

// ---- Transition, FrontEnd, Random, ChallengeReplay, HOM -------------------

/// `Transition.EndTransition()` (`0x8288e820`): `TransitionManager::EndTransition`.
fn end_transition(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.output(Output::EndTransition);
    none()
}

/// `FrontEnd.OpenPressADialog(playerTurn = 1, round = 1)` (`0x82896d80`): FE state 38
/// (`FrontEndState_PressA`) with them, pushed.
fn open_press_a_dialog(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player_turn = get_int_or(call, 0, 1);
    let round = get_int_or(call, 1, 1);
    world.output(Output::OpenPressADialog { player_turn, round });
    none()
}

/// `Random.GetRandomNumber(n)` (`0x8288adb8`): `(int)Simulation::GetRandom(1, n + 1)` with
/// `Sk8::Random::msLocked` cleared around it.
fn get_random_number(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    random_range(world, "Random.GetRandomNumber", 1, get_int(call, 0))
}

/// `Random.GetRandomNumberRange(min, max)` (`0x8288ae68`): `(int)Simulation::GetRandom(min,
/// max + 1)`, likewise.
fn get_random_number_range(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    random_range(world, "Random.GetRandomNumberRange", get_int(call, 0), get_int(call, 1))
}

#[expect(clippy::cast_precision_loss, reason = "the native (float) conversions")]
fn random_range(world: &mut World, context: &str, minimum: i32, maximum: i32) -> R {
    let Some(random) = world.random.as_mut() else {
        world.output(Output::ScriptError { context: context.to_owned(), error: "the Simulation RNG is not lent".to_owned() });
        return Ok(vec![int(0)]);
    };
    random.unlock_for_commands();
    let value = random.try_range(minimum as f32, (maximum + 1) as f32);
    random.lock_after_commands();
    match value {
        #[expect(clippy::cast_possible_truncation, reason = "the native (int) conversion")]
        Ok(value) => Ok(vec![int(value as i32)]),
        Err(e) => {
            world.output(Output::ScriptError { context: context.to_owned(), error: e.to_string() });
            Ok(vec![int(0)])
        }
    }
}

/// `ChallengeReplay.GetLastFrameNumberWritten()` (`0x82889ca0`): `DataMan::
/// GetLastFrameNumberWritten(SubmitRestoreFlag_Sim)`.
fn get_last_frame_number_written(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![super::binding::int_u32(world.replay.last_frame_number_written())])
}

/// `HOM.GetHOMScore()` (`0x8288edd8`): `ChallengeHOMManager::mWipeoutScore`.
fn get_hom_score(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(world.hom_wipeout_score)])
}

// ---- HighlightReel --------------------------------------------------------

/// `cHighlightGroup::tHighlight`: a replay pin and how to play it.
#[derive(Clone, Debug, PartialEq)]
pub struct Highlight {
    pub pin: i32,
    /// The `tNISPlaybackDefinition` the highlight plays with (`+0x10`, or `+0x50` for the
    /// alternate one).
    pub definition: Option<super::attrib_db::Location>,
    pub focus_player: i32,
}

/// `cHighlightGroup::eHighlightGroupPlaybackState`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlaybackState {
    #[default]
    NotPlaying,
    /// The first highlight's NIS was asked for; `HighlightUpdate` moves on.
    FrameWait,
    Playing,
}

/// `cHighlightGroup` (`0x160` bytes): one activity's replay pins.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HighlightGroup {
    pub id: u32,
    pub activity: usize,
    pub highlights: Vec<Highlight>,
    pub playback: PlaybackState,
    /// `CurrentlyPlayingList`: indices into `highlights`.
    pub playing: Vec<usize>,
    /// `CurrentlyPlaying`.
    pub current: usize,
    /// `UseHOMEffect`.
    pub use_hom_effect: bool,
    /// `RestoreStreamerOnComplete`.
    pub restore_streamer_on_complete: bool,
    /// `FocusRestore` (`+0x150`): where the last highlight returns the streamer, kept when a
    /// reel of more than one starts.
    pub focus_restore: [f32; 3],
}

/// `tNISPlaybackDefinition`: `UseLocationOverride` and `LocationOverride`.
const USE_LOCATION_OVERRIDE: u32 = 0x6;
const LOCATION_OVERRIDE: u32 = 0x10;

/// `cHighlightGroup::PlayHighlight(index, requiresStitch, focusRestore)` (`0x82cd75e0`):
/// the highlight's pinned frames played through its definition's scene
/// (`NISManager::RunReplayNIS`), skippable, the camera held at the end when more highlights
/// follow; the overrides keep the speed and track (1.0, mode 0) and carry the focus player,
/// the definition's location override (a zero direction faces +z) and, when a component
/// exceeds `FLT_EPSILON` (`0x82cd7794`), the focus restore as the stream position on return.
/// The special flag (`0x82cd77c8`) is `NIS_SPECIALFLAG_HALLOFMEAT` for a HoM group, otherwise
/// `NIS_SPECIALFLAG_FINALHIGHLIGHT` for a highlight not stitched to the next.
fn play_highlight(world: &mut World, group: usize, index: usize, stitch: bool, focus_restore: [f32; 3]) {
    let Some(highlight) = world.highlight_groups.get(group).and_then(|g| g.playing.get(index).and_then(|&h| g.highlights.get(h))).cloned()
    else {
        return;
    };
    let start = world.replay.pin_start_frame(highlight.pin).unwrap_or(0);
    let end = world.replay.pin_end_frame(highlight.pin).unwrap_or(0);
    let definition = highlight.definition;
    let scene = definition.and_then(|d| world.db.text_at(d)).unwrap_or_default().to_owned();
    let location = definition
        .filter(|d| world.db.bytes(d.offset_by(USE_LOCATION_OVERRIDE), 1).is_some_and(|b| b[0] != 0))
        .and_then(|d| world.db.bytes(d.offset_by(LOCATION_OVERRIDE), 32))
        .map(|b| {
            let lane = |i: usize| f32::from_be_bytes([b[i * 4], b[i * 4 + 1], b[i * 4 + 2], b[i * 4 + 3]]);
            let position = [lane(0), lane(1), lane(2)];
            let mut direction = [lane(4), lane(5), lane(6)];
            if direction == [0.0; 3] {
                direction = [0.0, 0.0, 1.0];
            }
            [position, direction]
        });
    let stream_position = focus_restore.iter().any(|v| v.abs() > f32::EPSILON).then_some(focus_restore);
    let special_flags = if world.highlight_groups[group].use_hom_effect {
        SPECIAL_FLAG_HALL_OF_MEAT
    } else if stitch {
        SPECIAL_FLAG_NONE
    } else {
        SPECIAL_FLAG_FINAL_HIGHLIGHT
    };
    world.output(Output::RunReplayNis {
        scene,
        start,
        end,
        hold_camera: stitch,
        skip_enabled: true,
        focus_player: highlight.focus_player,
        location,
        stream_position,
        special_flags,
    });
    if let Some(g) = world.highlight_groups.get_mut(group) {
        g.current = index;
    }
}

/// `PlayHighlightGroup` (`0x82cd7b68`) and `PlayHighlightGroupWithSpecifiedOrder`
/// (`0x82cd7d30`) once their non-empty list is made: its first highlight plays with the focus
/// restore, stitched to the next when there are more, and the group waits a frame. An online
/// game plays no reel (the port's choice, not the original's): every list answers
/// `cMsgHighlightReelComplete` at once, as an empty one does, so the results follow the end of
/// the challenge or round.
fn play_list(world: &mut World, group: usize, playing: Vec<usize>, focus_restore: [f32; 3]) {
    if world.online.is_some() {
        world.highlight_groups[group].restore_streamer_on_complete = false;
        world.highlight_groups[group].focus_restore = [0.0; 3];
        world.send(Event::Challenge(Message::new("cMsgHighlightReelComplete", Vec::new())));
        return;
    }
    let stitch = playing.len() > 1;
    world.highlight_groups[group].playing = playing;
    play_highlight(world, group, 0, stitch, focus_restore);
    world.highlight_groups[group].playback = PlaybackState::FrameWait;
}

/// `cHighlightReelManager::HighlightUpdate` (`0x82cd6be0`): a group waiting a frame plays.
pub(crate) fn highlight_update(world: &mut World) {
    for g in &mut world.highlight_groups {
        if g.playback == PlaybackState::FrameWait {
            g.playback = PlaybackState::Playing;
        }
    }
}

/// `cHighlightReelManager::OnNisCompleteMsg` (`0x82cd7af8`) to each group, then
/// `cHighlightGroup::OnNisCompleteMsg` (`0x82cd79f0`): a playing group plays its next
/// highlight (held at the end unless it is the last, which takes the kept focus restore when
/// the group restores the streamer), or after the last answers `cMsgHighlightReelComplete` and
/// stops.
pub(crate) fn receive(world: &mut World, event: &Event) {
    if !matches!(event, Event::NisComplete) {
        return;
    }
    for group in 0..world.highlight_groups.len() {
        let g = &mut world.highlight_groups[group];
        if g.playback != PlaybackState::Playing {
            continue;
        }
        let next = g.current + 1;
        g.current = next;
        if next == g.playing.len() {
            g.restore_streamer_on_complete = false;
            g.focus_restore = [0.0; 3];
            g.playback = PlaybackState::NotPlaying;
            world.send(Event::Challenge(Message::new("cMsgHighlightReelComplete", Vec::new())));
        } else {
            let stitch = next < g.playing.len() - 1;
            let focus_restore = if !stitch && g.restore_streamer_on_complete { g.focus_restore } else { [0.0; 3] };
            play_highlight(world, group, next, stitch, focus_restore);
        }
    }
}

fn highlight_group(world: &World, value: &Value) -> Option<u32> {
    world.pointers.object(value, ObjectKind::HighlightGroup)
}

/// `HighlightReel.CreateHighlightGroup()` (`0x82cd82c8`): `CreatesHighlightGroup`
/// (`0x82cd81a8`), the closure activity's group, made once.
fn create_highlight_group(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let Some(activity) = world.closure else { return Ok(vec![Value::Nil]) };
    let id = if let Some(group) = world.highlight_groups.iter().find(|g| g.activity == activity) {
        group.id
    } else {
        world.highlight_group_ids += 1;
        let id = world.highlight_group_ids;
        world.highlight_groups.push(HighlightGroup { id, activity, ..HighlightGroup::default() });
        id
    };
    let handle = world.pointers.handle(Pointee::Object { kind: ObjectKind::HighlightGroup, id });
    Ok(vec![Value::LightUserData(handle)])
}

/// `HighlightReel.AddHighlightUsingDefinition(group, definition, alternate, endFrame,
/// startFrame = endFrame, releaseOld = false, focusPlayer = -1, preRoll = 0, future = 0)`
/// (`0x82cd7828`): the definition's pre-roll (`+0`) and future (`+4`) seconds at 60 frames
/// a second unless given, then `cHighlightGroup::AddHighlight` (`0x82cd7428`).
fn add_highlight_using_definition(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(group) = highlight_group(world, call.arg(0)) else { return Ok(vec![int(-1)]) };
    let Some(definition) = super::attrib_lua::attrib_location(&world.pointers, call.arg(1)) else { return Ok(vec![int(-1)]) };
    let alternate = get_bool(call, 2);
    let end = get_int(call, 3);
    let start = get_int_or(call, 4, end);
    let release_old = super::binding::get_bool_or(call, 5, false);
    let focus = get_int_or(call, 6, -1);
    let pre_roll = super::binding::get_float_or(call, 7, 0.0);
    let future_seconds = super::binding::get_float_or(call, 8, 0.0);
    let float = |offset: u32| {
        world.db.bytes(definition.offset_by(offset), 4).map_or(0.0, |b| f32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    };
    #[expect(clippy::cast_possible_truncation, reason = "the native (int) conversions")]
    let (start_frame, future) = {
        let start_frame = if pre_roll > 0.0 { start - (pre_roll * 60.0) as i32 } else { start - (float(0) * 60.0) as i32 };
        let future = if future_seconds > 0.0 { (future_seconds * 60.0) as i32 } else { (float(4) * 60.0) as i32 };
        (start_frame, future)
    };
    let definition = definition.offset_by(if alternate { 0x50 } else { 0x10 });
    let pin = add_highlight(world, group, definition, start_frame.cast_unsigned(), end.cast_unsigned(), future, release_old, focus);
    Ok(vec![int(pin)])
}

/// `cHighlightGroup::AddHighlight` (`0x82cd7428`): an empty or reversed range is -1;
/// otherwise the replay system pins the frames (moving the start forward ten frames at a
/// time while it cannot, after releasing the oldest pin when asked to).
#[expect(clippy::too_many_arguments, reason = "the native signature")]
fn add_highlight(
    world: &mut World,
    group: u32,
    definition: super::attrib_db::Location,
    start: u32,
    end: u32,
    future: i32,
    release_old: bool,
    focus: i32,
) -> i32 {
    if start >= end {
        return -1;
    }
    let mut pin = world.replay.request_pin(start, end, future);
    if pin == -1 {
        if release_old
            && let Some(oldest) = world.highlight_groups.iter_mut().find(|g| g.id == group).filter(|g| !g.highlights.is_empty()).map(|g| g.highlights.remove(0))
        {
            world.replay.release_pin(oldest.pin);
        }
        let mut start = start;
        while pin == -1 {
            start = start.saturating_add(10);
            if start >= end {
                return pin;
            }
            pin = world.replay.request_pin(start, end, future);
        }
    }
    if let Some(g) = world.highlight_groups.iter_mut().find(|g| g.id == group) {
        g.highlights.push(Highlight { pin, definition: Some(definition), focus_player: focus });
    }
    pin
}

/// `HighlightReel.RemoveHighlight(group, id)` (`0x82cd7310`): `cHighlightGroup::
/// RemoveHighlight` (`0x82cd7220`).
fn remove_highlight(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(group) = highlight_group(world, call.arg(0)) else { return none() };
    let id = get_int(call, 1);
    let released = world.highlight_groups.iter_mut().find(|g| g.id == group).and_then(|g| {
        let at = g.highlights.iter().position(|h| h.pin == id)?;
        Some(g.highlights.remove(at).pin)
    });
    if let Some(pin) = released {
        world.replay.release_pin(pin);
    }
    none()
}

/// `HighlightReel.RemoveAllHighlights(group)` (`0x82cd6d68`): `ReleaseAllHighlights`
/// (`0x82cd6c58`).
fn remove_all_highlights(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(group) = highlight_group(world, call.arg(0)) else { return none() };
    let pins: Vec<i32> =
        world.highlight_groups.iter_mut().find(|g| g.id == group).map(|g| g.highlights.drain(..).map(|h| h.pin).collect()).unwrap_or_default();
    for pin in pins {
        world.replay.release_pin(pin);
    }
    none()
}

/// `HighlightReel.DestroyHighlightGroup(group)` (`0x82cd7180`): `ReleaseHighlightGroup`
/// (`0x82cd6e40`), the group's pins released with it.
fn destroy_highlight_group(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(group) = highlight_group(world, call.arg(0)) else { return none() };
    release_group(world, |g| g.id == group);
    none()
}

/// `HighlightReel.PlayHighlightReelWithSpecifiedOrder(group, count, ids)` (`0x82cd7f80`): the
/// table's first `count` IDs, then `cHighlightGroup::PlayHighlightGroupWithSpecifiedOrder`
/// (`0x82cd7d30`): the group's highlights whose pin is among the IDs, in the IDs' order, without
/// restoring the streamer; a group holding none of the IDs' pins answers
/// `cMsgHighlightReelComplete` at once.
fn play_highlight_reel_with_specified_order(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(group) = highlight_group(world, call.arg(0)) else { return none() };
    let count = get_int(call, 1);
    let mut ids = Vec::new();
    if let Value::Table(table) = call.arg(2) {
        let mut key = Value::Nil;
        while let Ok(Some((k, v))) = lua.table(*table).next(&key) {
            if i32::try_from(ids.len()).unwrap_or(i32::MAX) >= count {
                break;
            }
            #[expect(clippy::cast_possible_truncation, reason = "GetInt")]
            ids.push(match v {
                Value::Number(n) => (n + 0.5) as i32,
                _ => 0,
            });
            key = k;
        }
    }
    let Some(gi) = world.highlight_groups.iter().position(|g| g.id == group) else { return none() };
    let g = &mut world.highlight_groups[gi];
    let playing: Vec<usize> = ids.iter().filter_map(|id| g.highlights.iter().position(|h| h.pin == *id)).collect();
    if playing.is_empty() {
        world.send(Event::Challenge(Message::new("cMsgHighlightReelComplete", Vec::new())));
        return none();
    }
    g.restore_streamer_on_complete = false;
    g.focus_restore = [0.0; 3];
    play_list(world, gi, playing, [0.0; 3]);
    none()
}

/// `HighlightReel.PlayHighlightReel(group, useHOMEffect = false, restoreStreamerOnComplete =
/// false)` (`0x82cd7ef8`): `cHighlightGroup::PlayHighlightGroup` (`0x82cd7b68`): every
/// highlight in order; a group holding no pins answers `cMsgHighlightReelComplete` at once.
/// Restoring the streamer takes `NISManager::GetRestorePosition`: the only highlight returns
/// the streamer there, and a longer reel keeps it (`FocusRestore`) for its last.
fn play_highlight_reel(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(group) = highlight_group(world, call.arg(0)) else { return none() };
    let use_hom_effect = super::binding::get_bool_or(call, 1, false);
    let restore_streamer = super::binding::get_bool_or(call, 2, false);
    let Some(gi) = world.highlight_groups.iter().position(|g| g.id == group) else { return none() };
    let position = restore_position(world);
    let g = &mut world.highlight_groups[gi];
    g.use_hom_effect = use_hom_effect;
    let count = g.highlights.len();
    if count == 0 {
        g.restore_streamer_on_complete = false;
        g.focus_restore = [0.0; 3];
        world.send(Event::Challenge(Message::new("cMsgHighlightReelComplete", Vec::new())));
        return none();
    }
    let mut focus_restore = [0.0; 3];
    if restore_streamer {
        g.restore_streamer_on_complete = count > 1;
        if count > 1 {
            g.focus_restore = position;
        } else {
            focus_restore = position;
        }
    }
    play_list(world, gi, (0..count).collect(), focus_restore);
    none()
}

/// `NISManager::GetRestorePosition` (`0x828a1418`): the local skater's position, zero without
/// one.
fn restore_position(world: &World) -> [f32; 3] {
    if world.player.exists { world.player.position } else { [0.0; 3] }
}

fn release_group(world: &mut World, matches: impl Fn(&HighlightGroup) -> bool) {
    let mut pins = Vec::new();
    world.highlight_groups.retain(|g| {
        if matches(g) {
            pins.extend(g.highlights.iter().map(|h| h.pin));
            false
        } else {
            true
        }
    });
    for pin in pins {
        world.replay.release_pin(pin);
    }
}

/// `cHighlightReelManager::ShutdownActivity` (`0x82cd6ec0`): the activity's group goes.
pub(crate) fn shutdown_activity(world: &mut World, activity: usize) {
    release_group(world, |g| g.activity == activity);
}

/// `ChallengeReplay.ResetReplayBuffer()` (`0x82889e30`): `cChallengeSimManager::
/// ResetReplaySystemToOneFrame` (`0x82da1e80`), `EraseAllButLastNFrames(2)`.
fn reset_replay_buffer(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.replay.erase_all_but_last_n_frames(2);
    Ok(Vec::new())
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("ChallengeReplay", "ResetReplayBuffer", reset_replay_buffer),
        ("ChallengeManager", "SetPlayer", set_player),
        ("ChallengeManager", "GetNumHumanSkaters", get_num_human_skaters),
        ("ChallengeManager", "GetSkaterCount", get_skater_count),
        ("ChallengeManager", "SetManualsAreConnectors", set_manuals_are_connectors),
        ("ChallengeManager", "SetResetSequence", set_reset_sequence),
        ("ChallengeManager", "SetResetScoring", set_reset_scoring),
        ("Entity", "DoesSkaterExist", does_skater_exist),
        ("Entity", "IsPlayerStanceRegular", is_player_stance_regular),
        ("Online", "IsOnline", is_online),
        ("Online", "GetNumOnlineSkaters", get_num_online_skaters),
        ("Online", "IsPlayerDisconnected", is_player_disconnected),
        ("Online", "SwitchCamera", switch_camera),
        ("Online", "OnlineChallengeComplete", online_challenge_complete),
        ("Online", "LeaveGame", online_leave_game),
        // The same native (0x8288a4e0) under both names.
        ("ChallengeManager", "SwitchCamera", switch_camera),
        ("AI", "PinAI", pin_ai),
        ("AI", "UnpinAI", unpin_ai),
        ("Challenge", "GetGameMode", get_game_mode),
        ("Challenge", "GetPlayerName", get_player_name),
        ("Challenge", "GetChallengeType", get_challenge_type),
        ("Challenge", "SendPlayerOverallResult", send_player_overall_result),
        ("Challenge", "SendPlayerRoundResult", send_player_round_result),
        ("Challenge", "NotifyChallengeCompleteForObjectives", notify_challenge_complete_for_objectives),
        ("Challenge", "GetTeleportFadeInProgress", get_teleport_fade_in_progress),
        ("Transition", "EndTransition", end_transition),
        ("FrontEnd", "OpenPressADialog", open_press_a_dialog),
        ("Random", "GetRandomNumber", get_random_number),
        ("Random", "GetRandomNumberRange", get_random_number_range),
        ("ChallengeReplay", "GetLastFrameNumberWritten", get_last_frame_number_written),
        ("HOM", "GetHOMScore", get_hom_score),
        ("HighlightReel", "CreateHighlightGroup", create_highlight_group),
        ("HighlightReel", "AddHighlightUsingDefinition", add_highlight_using_definition),
        ("HighlightReel", "RemoveHighlight", remove_highlight),
        ("HighlightReel", "RemoveAllHighlights", remove_all_highlights),
        ("HighlightReel", "DestroyHighlightGroup", destroy_highlight_group),
        ("HighlightReel", "PlayHighlightReelWithSpecifiedOrder", play_highlight_reel_with_specified_order),
        ("HighlightReel", "PlayHighlightReel", play_highlight_reel),
    ]
}
