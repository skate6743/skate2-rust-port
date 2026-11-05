//! PC port: the natives Skate 3's own challenge graphs call that Skate 2's binding tables lack,
//! by their Skate 3 (TU3) addresses, for a linked Skate 3 data folder's games that run Skate 3's
//! graph (`crate::skate3`); bound into the modules after Skate 2's own. Teams, AI competitors,
//! telemetry and the single player results are Skate 3's alone and play no part here.

use super::{
    binding::{get_int_or, get_string, get_string_or},
    hud::{CentralMessage, ModifiedCentral},
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    system::{Output, World},
};

type R = LuaResult<Vec<Value>>;

fn none(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(Vec::new())
}

fn no(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Boolean(false)])
}

/// `HUD2.eMessageColour.Green` and `HUD2.eTeamMessageType.Winning`.
const GREEN: i32 = 2;
const WINNING: i32 = 2;

/// The central message, as `HUD2.ShowCentralMessage` shows it.
fn central_message(world: &mut World, message: CentralMessage) {
    world.output(Output::CentralMessage(message.clone()));
    world.hud.central_messages.push(message);
}

/// The parameters a custom message's string gives `FormatStringCustom`, comma separated.
fn custom_args(custom: &str) -> [String; 4] {
    let mut parts = custom.split(',');
    std::array::from_fn(|_| parts.next().unwrap_or_default().to_owned())
}

/// `HUD2.ShowCustomCentralMessage(halId, colour = 0, custom = "", audio = 0)` (`0x82850968`):
/// `CentralHUD::ShowCustomCentralMessage` (`0x82664038`) formats the message with the custom
/// string's parameters.
fn show_custom_central_message(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let message = CentralMessage {
        hal_id: get_string(call, 0).unwrap_or_default(),
        colour: get_int_or(call, 1, 0),
        args: custom_args(&get_string_or(call, 2, "")),
        audio_hook: get_int_or(call, 3, 0),
    };
    central_message(world, message);
    Ok(Vec::new())
}

/// `HUD2.ShowTeamCentralMessage(halId, team, type = 0, s1..s4, audio = 0)` (`0x82850af0`): a
/// team's message, which without teams is the central message, a winning one in green.
fn show_team_central_message(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let message = CentralMessage {
        hal_id: get_string(call, 0).unwrap_or_default(),
        colour: if get_int_or(call, 2, 0) == WINNING { GREEN } else { 0 },
        args: [get_string_or(call, 3, ""), get_string_or(call, 4, ""), get_string_or(call, 5, ""), get_string_or(call, 6, "")],
        audio_hook: get_int_or(call, 7, 0),
    };
    central_message(world, message);
    Ok(Vec::new())
}

/// `HUD2.ShowCustomTeamCentralMessage(halId, team, type = 0, custom = "")` (`0x82850c58`): as
/// [`show_team_central_message`] with a custom string's parameters.
fn show_custom_team_central_message(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let message = CentralMessage {
        hal_id: get_string(call, 0).unwrap_or_default(),
        colour: if get_int_or(call, 2, 0) == WINNING { GREEN } else { 0 },
        args: custom_args(&get_string_or(call, 3, "")),
        audio_hook: 0,
    };
    central_message(world, message);
    Ok(Vec::new())
}

/// `HUD2.ClearModifiedCentralMessage()` (`0x828508e0`): the central HUD's modified line is no
/// longer held up, and screen state 88 plays its outro.
fn clear_modified_central_message(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.output(Output::ModifiedCentralMessage(ModifiedCentral::Clear));
    Ok(Vec::new())
}

/// `Online.GetTeam(player)` (`0x828463d8`): the player's team plus one; every player is on the
/// first here.
fn get_team(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::Number(1.0)])
}

/// `Teams.GetName(team)` (`0x828383b8`), read only by a team game.
fn team_name(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(vec![Value::string("")])
}

/// Skate 3's challenge setting `Physics::IsScoringMultiplierActive` reads (key 0xCB837EEF50958FBB):
/// true in its challenges' `main` local data, false in its 1-Ups' `1ups`.
const MULTIPLIER_ACTIVE: u64 = 0xcb83_7eef_5095_8fbb;

/// TU3 `Physics::IsScoringMultiplierActive` (`0x82761548`) for a challenge outside a contest
/// (Demo, Best Trick and Jam read their `ContestRounds` instead, and run Skate 2's graph here):
/// the setting, where the challenge's data providers hold it (the challenge, its global data,
/// then its local data, where the graft put Skate 3's), and off where none holds it
/// (`Attrib::DefaultDataArea`).
pub(super) fn multiplier_active(world: &World, activity: &super::activity::Activity) -> bool {
    std::iter::once((super::keys::class::CHALLENGES, activity.key))
        .chain(activity.global_data)
        .chain(activity.local_data)
        .find_map(|(class, collection)| world.db.attribute(class, collection, MULTIPLIER_ACTIVE))
        .and_then(|value| value.bool(0))
        .unwrap_or(false)
}

/// The natives, by module and name.
const NATIVES: [(&str, &str, NativeFn<World>); 20] = [
    ("HUD2", "ShowCustomCentralMessage", show_custom_central_message),
    ("HUD2", "ShowTeamCentralMessage", show_team_central_message),
    ("HUD2", "ShowCustomTeamCentralMessage", show_custom_team_central_message),
    ("HUD2", "ClearModifiedCentralMessage", clear_modified_central_message),
    // `0x828531f0`, `0x82851330`, `0x82851230`: the teams' colour, rank and score.
    ("HUD2", "DPOSetTeamColour", none),
    ("HUD2", "LeaderboardHUDSetTeamRank", none),
    ("HUD2", "LeaderboardHUDSetTeamScore", none),
    ("HighlightReel", "GetNumberOfHighlights", super::players::get_number_of_highlights),
    ("Online", "GetTeam", get_team),
    // `0x82846340`: a network game whose attributes play teams; the port's games play none.
    ("Online", "IsTeamChallenge", no),
    ("Teams", "GetName", team_name),
    // `0x828572b0`: the winning team for the outro.
    ("Results", "SetWinningTeam", none),
    // `0x82838708`, `0x82838760`: the single player results' progression lock.
    ("Progression", "LockForPresentation", none),
    ("Progression", "UnlockFromPresentation", none),
    // `0x828570e8`, `0x82857138`, `0x82857068`: telemetry.
    ("Telemetry", "BeginNewRound", none),
    ("Telemetry", "RoundFinished", none),
    ("Telemetry", "SendAttempt", none),
    // `0x82840460`: a flow message that closes Skate 3's open loading screen; `0x82840600`: the
    // visual transition manager's notice of the running challenge and its mission sound.
    ("Challenge", "CloseLoadingScreen", none),
    ("Challenge", "NotifyChallengeRunning", none),
    // `0x82839358` for both: never.
    ("GameMode", "OneUpSpeedRounds", no),
];

/// The natives bound as never true, by module and name, besides [`NATIVES`].
const NEVER: [(&str, &str); 1] = [("Debug", "IsDebugEnabled")];

/// The AI competitors' natives (`0x8284a5c0`, `0x8284a658`), which only a single player game's
/// competitors call.
const AI_NATIVES: [&str; 2] = ["SetPreferredAIPathSkillLevel", "SetScoreBiasMultiplier"];

/// Skate 3's enums: `AI.eSkillLevel` (`0x8284a030`), `Challenge.EActivityCompletionType`
/// (`0x8283e078`) and `HUD2.eTeamMessageType` (`0x8284fb70`).
const ENUMS: [(&str, &str, &[(&str, f32)]); 3] = [
    ("AI", "eSkillLevel", &[("Easy", 0.0), ("Medium", 1.0), ("Hard", 2.0), ("VeryHard", 3.0), ("Extreme", 4.0)]),
    ("Challenge", "EActivityCompletionType", &[("Abandoned", 0.0), ("Owned", 1.0), ("Killed", 2.0)]),
    ("HUD2", "eTeamMessageType", &[("Normal", 0.0), ("Assignment", 1.0), ("Winning", 2.0)]),
];

/// The module's table, made when Skate 2 binds none of that name.
fn module(lua: &mut Lua<World>, name: &str) -> Value {
    let found = lua.get_global(name);
    if matches!(found, Value::Table(_)) {
        return found;
    }
    let table = Value::Table(lua.create_table());
    lua.set_global(name, table.clone());
    table
}

/// Binds the natives and enums into their modules, over any of Skate 2's of the same name.
pub(super) fn bind(lua: &mut Lua<World>) {
    let functions = NATIVES
        .iter()
        .map(|&(module, name, function)| (module, name, function))
        .chain(NEVER.iter().map(|&(module, name)| (module, name, no as NativeFn<World>)))
        .chain(AI_NATIVES.iter().map(|&name| ("AI", name, none as NativeFn<World>)));
    for (module_name, name, function) in functions {
        let native = lua.create_native(name, function, Vec::new());
        if let Value::Table(target) = module(lua, module_name) {
            lua.table_mut(target).set_str(name, native);
        }
    }
    for (module_name, name, values) in ENUMS {
        let table = lua.create_table();
        for &(member, value) in values {
            lua.table_mut(table).set_str(member, Value::Number(value));
        }
        if let Value::Table(target) = module(lua, module_name) {
            lua.table_mut(target).set_str(name, Value::Table(table));
        }
    }
}
