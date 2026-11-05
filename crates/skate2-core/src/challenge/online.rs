//! `Online` natives. The port's online games run on its own server rather than EA's, so
//! `Fesl::HubSingle::ConnInstance()->GetStatus()` is never `CONNECTION_READY` (there is no local
//! `Fesl::User`) and `cOnlineLeaderboardManager::mChallengeInfo`, which only online leaderboard
//! replies fill, stays empty. A network game (`CommonBackEnd::sGameInfo.isNetworkGame`) is one
//! with [`World::online`].

use super::{
    attrib_lua::attrib_location,
    binding::{get_bool_or, get_int, get_int_or, get_string, int},
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    system::{Event, Output, World},
};

type R = LuaResult<Vec<Value>>;

/// `Online.GetSkaterName(player)` (`0x82882f88`): `PlayersBackEnd::GetPlayerName` (`0x8255a728`).
fn get_skater_name(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int(call, 0);
    let name = world.players.as_deref().map(|players| players.player_name(player)).unwrap_or_default();
    Ok(vec![Value::string(&name)])
}

/// `Online.GetLocalPlayerGamerTag()` (`0x82882fe0`): `Sk8::Net::GetLocalUser` (`0x824b27a0`) has
/// no user, so the empty string.
fn get_local_player_gamer_tag(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::string("")])
}

/// `Online.IsConnectedReady()` (`0x82882f30`): the connection's status is not 5 (`CONNECTION_READY`).
fn is_connected_ready(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Boolean(false)])
}

/// `Online.GetOTSScore(data)` (`0x8288aab8`): `cOnlineLeaderboardManager::GetScore` (`0x825b6838`)
/// of the activity, 0 without leaderboard data, pushed as an int.
fn get_ots_score(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(0)])
}

/// `Online.GetTopFriendScore(data)` (`0x8288ab80`): `GetTopFriendScore` (`0x825b6758`), 0 without
/// leaderboard data.
fn get_top_friend_score(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(0)])
}

/// `Online.GetTopFriendName(data)` (`0x8288ac48`): `GetTopFriendName` (`0x825b67c8`), the empty
/// string without leaderboard data.
fn get_top_friend_name(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::string("")])
}

/// `Online.UploadOTSScore(data, score)` (`0x82883040`): `Sk8::Net::UpdateChallengeBestScore`
/// (`0x82799610`) sends the `SetStat` transaction only while connected.
fn upload_ots_score(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(Vec::new())
}

/// `Online.OnlineChallengeRoundSyncPoint(round, tLocationID)` (`0x8289adb8`): only in a network
/// game, which pauses (`kPAUSE_STATE_ONLINE`, `Net::PauseGame`) and sends the flow manager
/// `MESSAGE_ChallengeSyncPoint` with the location's spawn point for the local player. The round's
/// NIS, which the flow preloads before it reports the sync point, needs no preload here.
fn online_challenge_round_sync_point(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if world.online.is_none() {
        return Ok(Vec::new());
    }
    let round = get_int(call, 0);
    let location = attrib_location(&world.pointers, call.arg(1)).and_then(|at| world.db.text_at(at)).unwrap_or_default().to_owned();
    world.output(Output::OnlineRoundSyncPoint { round, location });
    Ok(Vec::new())
}

/// `NIS.EnterOnlineNISUsingDefinition(definition, fallback, scene = nil, transition = 0,
/// holdCamera = false, enableSkip = true)` (`0x828958b0`): the first `tNISPlaybackDefinition`, or
/// the second when the first names no scene, with its scene replaced by a given one. A scene runs
/// (`NISManager::RunBasicNISDefinition` without a streamer restore; transitions 3 and 4 complete
/// with 4, the others with none); without one `cMsgNISChallengeNotify` carries notification 1.
fn enter_online_nis_using_definition(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(first) = attrib_location(&world.pointers, call.arg(0)) else { return Ok(Vec::new()) };
    let Some(second) = attrib_location(&world.pointers, call.arg(1)) else { return Ok(Vec::new()) };
    let scene_of = |definition| world.db.text_at(definition).unwrap_or_default().to_owned();
    let definition = if scene_of(first).is_empty() { second } else { first };
    let scene = get_string(call, 2).unwrap_or_else(|| scene_of(definition));
    let transition = if (3..=4).contains(&get_int_or(call, 3, 0)) { 4 } else { 0 };
    let hold_camera = get_bool_or(call, 4, false);
    let skip_enabled = get_bool_or(call, 5, true);
    if scene.is_empty() || super::natives::is_skate2_scene_at_skate3_spot(world, &scene) {
        world.send(Event::NisChallengeNotify { notification: 1 });
        return Ok(Vec::new());
    }
    let (ignore_breadcrumb, location) = super::natives::nis_definition_overrides(world, definition);
    let restore_streamer = false;
    world.output(Output::RunNisDefinition { scene, ignore_breadcrumb, transition, hold_camera, skip_enabled, location, restore_streamer });
    Ok(Vec::new())
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("NIS", "EnterOnlineNISUsingDefinition", enter_online_nis_using_definition),
        ("Online", "GetSkaterName", get_skater_name),
        ("Online", "GetLocalPlayerGamerTag", get_local_player_gamer_tag),
        ("Online", "IsConnectedReady", is_connected_ready),
        ("Online", "GetOTSScore", get_ots_score),
        ("Online", "GetTopFriendScore", get_top_friend_score),
        ("Online", "GetTopFriendName", get_top_friend_name),
        ("Online", "UploadOTSScore", upload_ots_score),
        ("Online", "OnlineChallengeRoundSyncPoint", online_challenge_round_sync_point),
    ]
}
