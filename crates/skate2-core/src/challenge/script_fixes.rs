//! Corrections to the original challenge scripts, made to a message handler's chunk before
//! `cMessageHandler::Attach` (`0x82ce38c8`) loads it or by a wrapper the handler runs through,
//! and Skate 3's own rules over Skate 2's handlers for a linked Skate 3 data folder's games.

use std::borrow::Cow;

/// Skate 2's Spot Battle (`skate2_ots_online`) `challenge_running` handlers that end a run: the
/// timer's expiry and a finished line.
pub(crate) const SPOT_BATTLE_TIMER_EXPIRED: u64 = 0x6C94_E9C3_7482_DE81;
pub(crate) const SPOT_BATTLE_SEQUENCE_COMPLETE: u64 = 0x01A1_0F67_9F02_A963;
/// Its `countdown` `stateenter`, which starts the next turn.
pub(crate) const SPOT_BATTLE_COUNTDOWN_ENTER: u64 = 0x5170_0D28_7965_7162;
/// Skate 2's Death Race (`r_2_2_dr_ur_02_ol`) `run_heat` trigger entry, which Skate 3's race
/// graph (`race_shared`) keeps under the same key.
pub(crate) const DEATH_RACE_TRIGGER_ENTER: u64 = 0x23FF_AF72_EBB4_C8F0;

/// PC port: the handlers a linked Skate 3 data folder's games of a network type run through
/// `h_skate3_<key>` of [`SKATE3_SOURCE`].
pub(crate) const SKATE3_HANDLERS: [(u64, u8); 4] = [
    (SPOT_BATTLE_TIMER_EXPIRED, skate2_net::protocol::challenge::OWN_THE_SPOT),
    (SPOT_BATTLE_SEQUENCE_COMPLETE, skate2_net::protocol::challenge::OWN_THE_SPOT),
    (SPOT_BATTLE_COUNTDOWN_ENTER, skate2_net::protocol::challenge::OWN_THE_SPOT),
    (DEATH_RACE_TRIGGER_ENTER, skate2_net::protocol::challenge::DEATH_RACE),
];

/// PC port: Skate 3's rules over Skate 2's handlers.
///
/// A Spot Battle ends a run as Skate 3's own Spot Battle graph does (its `challenge_running`
/// timer and line handlers): where Skate 2 changes to `end_turn`, Skate 3 changes to it only on
/// the battle's last run (`LastRun`: the last round, no connected player still to go) and
/// otherwise starts a two-second `EndRunTimerID`, at whose expiry the screen fades out over half
/// a second (`FadeOut`, which hides the HUD) and the turn ends; lines finished meanwhile still
/// score. Skate 3 lifts the fade under its loading screen before the next turn; here the next
/// turn's teleport transition takes the faded screen over (`StartEffect` of a `FadeOut` that
/// already stays only reports it finished, and the teleport's `FadeIn` clears it), and a
/// `countdown` that ends the battle without one lifts it as Skate 3 does (`ResetEffect`).
///
/// A Death Race takes a racer's split trigger past the next one for every racer, as Skate 3's
/// race graph does (`LastLapGateIndex + 1 <= i`); Skate 2's takes it so only for the online local
/// racer, the others' only in order. Skate 3's races whose gates are their split triggers leave
/// a racer free to miss one.
pub(crate) const SKATE3_SOURCE: &str = r#"
local function IsLastRun(self, vars)
    if vars.CurrentRound ~= self.NumRounds then
        return false
    end
    for i = vars.CurrentPlayerIndex + 1, vars.NumPlayers do
        if not vars.PlayerData[vars.PlayerOrderMap[i]].IsDisconnected then
            return false
        end
    end
    return true
end

local function EndRun(handler, self, message, vars)
    local changeState = ChangeState
    ChangeState = function(state)
        if state == "end_turn" and not IsLastRun(self, vars) then
            vars.EndRunTimerID = ChallengeTimer.AddCountdownTimer(2.0)
        else
            changeState(state)
        end
    end
    handler(self, message, vars)
    ChangeState = changeState
end

function h_skate3_6C94E9C37482DE81(self, message, vars)
    if vars.EndRunTimerID ~= nil and message.TimerID == vars.EndRunTimerID then
        vars.EndRunTimerID = nil
        vars.EndRunFadeOut = true
        Effects.StartEffect(Effects.eChallengeEffect.FadeOut, 0.5)
        ChangeState("end_turn")
    else
        EndRun(h_6C94E9C37482DE81, self, message, vars)
    end
end

function h_skate3_01A10F679F02A963(self, message, vars)
    EndRun(h_01A10F679F02A963, self, message, vars)
end

function h_skate3_51700D2879657162(self, message, vars)
    if not vars.EndRunFadeOut then
        h_51700D2879657162(self, message, vars)
        return
    end
    vars.EndRunFadeOut = nil
    local finished = vars.LockOut
    local changeState = ChangeState
    ChangeState = function(state)
        if state == "challenge_finished" then
            finished = true
        end
        changeState(state)
    end
    h_51700D2879657162(self, message, vars)
    ChangeState = changeState
    if finished then
        Effects.ResetEffect(Effects.eChallengeEffect.FadeOut)
    end
end

function h_skate3_23FFAF72EBB4C8F0(self, message, vars)
    local index = vars.GetCompetitorIndex ~= nil and vars.GetCompetitorIndex(vars, message.PlayerID) or 0
    local competitor = index > 0 and vars.Competitor[index] or nil
    if competitor ~= nil and competitor.CompetitorActive and vars.CurrentHeat ~= nil then
        local heat = self.RaceHeats[vars.CurrentHeat]
        local leg = heat ~= nil and heat.Legs[competitor.CurrentLeg] or nil
        if leg ~= nil then
            local triggers = leg.SplitTimeTriggers
            for i = 1, triggers.Count do
                if message.VolumeID == triggers[i] then
                    if competitor.LastLapGateIndex + 1 < i then
                        competitor.LastLapGateIndex = i - 1
                    end
                    break
                end
            end
        end
    end
    h_23FFAF72EBB4C8F0(self, message, vars)
end
"#;

/// S.K.A.T.E.'s and 1-Up's `check_players` `stateenter`, which every challenge of the type
/// shares, each with its `global` `playerdisconnected`.
pub(crate) const SKATE_CHECK_PLAYERS: u64 = 0x330B_4986_37FF_1BE1;
pub(crate) const SKATE_PLAYER_DISCONNECTED: u64 = 0xB8C3_74BC_3202_9AF5;
pub(crate) const ONE_UP_CHECK_PLAYERS: u64 = 0x5D77_72E7_46ED_13EC;
pub(crate) const ONE_UP_PLAYER_DISCONNECTED: u64 = 0x784E_E510_5BA6_A161;

/// PC port: the handlers every game runs through `h_fix_<key>` of [`FIXES_SOURCE`], each with
/// the other shipped handler its wrapper calls.
pub(crate) const FIXED_HANDLERS: [(u64, u64); 2] = [
    (SKATE_CHECK_PLAYERS, SKATE_PLAYER_DISCONNECTED),
    (ONE_UP_CHECK_PLAYERS, ONE_UP_PLAYER_DISCONNECTED),
];

/// PC port: corrections that wrap a shipped handler.
///
/// Online, S.K.A.T.E.'s and 1-Up's `check_players` waits, a quarter second at a time (`wait`),
/// until every skater of `SkatersList` exists. `setup_skate` lists the players the challenge
/// started with (`Online.GetNumOnlineSkaters()`), and until the wait ends the backpack and
/// pause menu stay disabled (`make_hud`) and no one is pinned. A player who leaves in between
/// has no skater any more (`HandlePlayerLeave` 0x8276aa70 unspawns it: `SkaterManager::
/// OnCommand` 0x82600920 `DestroyPlayer`), so the original waits without end: the others skate
/// on with no menu, and no result ever ends the round. Corrected, a disconnected player counts
/// as there, as the reposition after the wait already skips them. A player whose
/// `cMsgPlayerDisconnected` came before `setup_skate` built the list, when the `global`
/// handler had no list to mark, is marked now through that handler, so the rounds leave them
/// out as after a later leave.
pub(crate) const FIXES_SOURCE: &str = r#"
local function CheckPlayers(handler, playerDisconnected, self, message, vars)
    local skaters = vars.SkatersList
    if skaters == nil then
        handler(self, message, vars)
        return
    end
    for i = 1, table.getn(skaters) do
        if playerDisconnected ~= nil and not skaters[i].disconnected and Online.IsPlayerDisconnected(skaters[i].id) then
            playerDisconnected(self, { PlayerIndex = skaters[i].id }, vars)
        end
    end
    local doesSkaterExist = Entity.DoesSkaterExist
    Entity.DoesSkaterExist = function(player)
        return Online.IsPlayerDisconnected(player) or doesSkaterExist(player)
    end
    local ok, problem = pcall(handler, self, message, vars)
    Entity.DoesSkaterExist = doesSkaterExist
    if not ok then
        error(problem)
    end
end

function h_fix_330B498637FF1BE1(self, message, vars)
    CheckPlayers(h_330B498637FF1BE1, h_B8C374BC32029AF5, self, message, vars)
end

function h_fix_5D7772E746ED13EC(self, message, vars)
    CheckPlayers(h_5D7772E746ED13EC, h_784EE5105BA6A161, self, message, vars)
end
"#;

/// S.K.A.T.E.'s `prepare_follower` `stateenter`, which every S.K.A.T.E. challenge shares.
pub(crate) const SKATE_PREPARE_FOLLOWER: u64 = 0x416F_B2B7_F3E1_08CD;

/// Its online search for the next connected follower, at the step past the end of
/// `FollowerOrder`: `CALL r9 2 2` (`table.getn(FollowerOrder)`), `LT 0 r9 r8`, `JMP 1`,
/// `SETTABLE r2 "FollowerIndex" 1.0`, `FORLOOP r4 -46`.
const FOLLOWER_SEARCH: [u32; 5] = [0x0100_825C, 0x0482_0018, 0x8000_0016, 0x90C2_C089, 0x7FF4_411F];
/// The `SETTABLE` of that search, which the correction replaces.
const FOLLOWER_SEARCH_RESTART: usize = 3;
/// `SETTABLE r2 "FollowerIndex" r9`: `FollowerIndex` stays at `table.getn(FollowerOrder)`.
const FOLLOWER_SEARCH_END: u32 = 0x9082_4089;

/// The handler's chunk with the corrections that apply to it.
///
/// `prepare_follower` online skips each disconnected follower, and past the end of
/// `FollowerOrder` the original starts again at the first one. With the last follower
/// disconnected before its turn, the followers before it take the same set trick again and
/// again: the round never reaches `process_round`, so no one else sets a trick. Corrected, the
/// search stops at the last follower, which ends the followers' turns as when every follower
/// left (`process_round`); the next round's `round_setup` leaves the disconnected player out.
pub(crate) fn corrected(handler: u64, chunk: &[u8]) -> Cow<'_, [u8]> {
    if handler != SKATE_PREPARE_FOLLOWER {
        return Cow::Borrowed(chunk);
    }
    let search: Vec<u8> = FOLLOWER_SEARCH.iter().flat_map(|word| word.to_le_bytes()).collect();
    let mut found = chunk.windows(search.len()).enumerate().filter(|(_, window)| *window == search.as_slice()).map(|(at, _)| at);
    let (Some(at), None) = (found.next(), found.next()) else {
        return Cow::Borrowed(chunk);
    };
    let mut fixed = chunk.to_vec();
    let restart = at + 4 * FOLLOWER_SEARCH_RESTART;
    fixed[restart..restart + 4].copy_from_slice(&FOLLOWER_SEARCH_END.to_le_bytes());
    Cow::Owned(fixed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk_with(words: &[u32]) -> Vec<u8> {
        let mut chunk = vec![0x1B, b'L', b'u', b'a', 0x51, 0, 1, 4, 4, 4, 4, 0];
        chunk.extend(words.iter().flat_map(|word| word.to_le_bytes()));
        chunk.extend([0xAA; 8]);
        chunk
    }

    #[test]
    fn the_follower_search_stops_at_the_last_follower() {
        let chunk = chunk_with(&FOLLOWER_SEARCH);
        let fixed = corrected(SKATE_PREPARE_FOLLOWER, &chunk);
        let mut expected = FOLLOWER_SEARCH;
        expected[FOLLOWER_SEARCH_RESTART] = FOLLOWER_SEARCH_END;
        assert_eq!(fixed.as_ref(), chunk_with(&expected).as_slice());
    }

    #[test]
    fn other_handlers_and_other_code_load_as_shipped() {
        let chunk = chunk_with(&FOLLOWER_SEARCH);
        assert!(matches!(corrected(0x1234, &chunk), Cow::Borrowed(_)));
        let mut changed = FOLLOWER_SEARCH;
        changed[0] ^= 1;
        assert!(matches!(corrected(SKATE_PREPARE_FOLLOWER, &chunk_with(&changed)), Cow::Borrowed(_)));
        let twice = [FOLLOWER_SEARCH, FOLLOWER_SEARCH].concat();
        assert!(matches!(corrected(SKATE_PREPARE_FOLLOWER, &chunk_with(&twice)), Cow::Borrowed(_)));
    }

    /// The shipped handler under stand-ins for its natives: an online round of four followers
    /// at the fourth's turn, the third having just tried, with the given followers disconnected.
    /// What `prepare_follower` does: the follower it moves to the attempt spawn, or the state
    /// it changes to, and `FollowerIndex` after it.
    fn prepare_follower(chunk: &[u8], disconnected: &[i32], index: i32, previous: i32) -> (Option<f32>, Option<String>, f32) {
        use super::super::lua::{Lua, Value};
        let mut lua = Lua::<()>::new();
        let entries: String = disconnected.iter().map(|player| format!("[{player}] = true, ")).collect();
        let stubs = format!(
            "disconnected = {{ {entries}}}
             Challenge = {{ GameMode = {{ PartyPlay = 1, SinglePlayer = 2, OnlineChallenge = 3 }} }}
             Skate = {{ ResetAttemptItem = function() end }}
             HUD2 = {{ ABDHUDRefreshCopyTrick = function() end }}
             ChallengeManager = {{ SetPlayer = function() end }}
             Online = {{ IsPlayerDisconnected = function(player) return disconnected[player] == true end }}
             AI = {{ RepositionAI = function(player) repositioned = player end, UnpinAI = function() end, PinAI = function() end }}
             function ChangeState(state) changed = state end
             function print() end
             data = {{ Name = 'skate', SpotVolumes = {{ Count = 0 }}, TurnBasedWaitingLocation = 0, TurnBasedAttemptSpawnPoint = 0 }}
             vars = {{ GameMode = 3, CachedTeleportID = -1, FollowerIndex = {index}, PreviousFollowerIndex = {previous}, FollowerOrder = {{ 1, 2, 3, 4 }}, SpotVolumeEntered = {{}} }}"
        );
        let setup = lua.load_source(stubs.as_bytes(), "stubs").expect("stubs compile");
        lua.call(&mut (), &setup, &[]).expect("stubs run");
        let main = lua.load(chunk, "prepare_follower").expect("handler loads");
        lua.call(&mut (), &main, &[]).expect("handler defines itself");
        let handler = lua.get_global(&format!("h_{SKATE_PREPARE_FOLLOWER:016X}"));
        let (data, vars) = (lua.get_global("data"), lua.get_global("vars"));
        let message = Value::Table(lua.create_table());
        lua.call(&mut (), &handler, &[data, message, vars.clone()]).expect("prepare_follower runs");
        let number = |value: Value| if let Value::Number(n) = value { Some(n) } else { None };
        let repositioned = number(lua.get_global("repositioned"));
        let changed = match lua.get_global("changed") {
            Value::Nil => None,
            state => Some(lua.display(&state)),
        };
        let Value::Table(vars) = vars else { unreachable!("vars is a table") };
        let index = number(lua.table(vars).get_str("FollowerIndex")).expect("FollowerIndex");
        (repositioned, changed, index)
    }

    /// Skate 3's end of a Spot Battle run over stand-ins for Skate 2's two handlers (each ends its
    /// run with `ChangeState("end_turn")`, the timer's when its `MainTimerID` expires): round
    /// `round` of two, player order 1..3 at `index`, with the given players disconnected. Returns
    /// the state changed to, the end-of-run timer and the fade-out's time, after `messages` timer
    /// IDs reach the timer handler (0 runs the line handler instead).
    fn skate3_end_run(round: i32, index: i32, disconnected: &[i32], messages: &[i32]) -> (Option<String>, Option<f32>, Option<f32>) {
        use super::super::lua::{Lua, Value};
        let mut lua = Lua::<()>::new();
        let players: String = (1..=3)
            .map(|player| format!("[{player}] = {{ IsDisconnected = {} }}, ", disconnected.contains(&player)))
            .collect();
        let stubs = format!(
            "ChallengeTimer = {{ AddCountdownTimer = function(seconds) timers = (timers or 40) + 1 return timers end }}
             Effects = {{ eChallengeEffect = {{ FadeOut = 2 }}, StartEffect = function(effect, time) if effect == 2 then fade = time end end }}
             function ChangeState(state) changed = state end
             function h_{SPOT_BATTLE_TIMER_EXPIRED:016X}(self, message, vars)
                 if message.TimerID == vars.MainTimerID then vars.MainTimerID = 0 ChangeState('end_turn') end
             end
             function h_{SPOT_BATTLE_SEQUENCE_COMPLETE:016X}(self, message, vars) ChangeState('end_turn') end
             data = {{ NumRounds = 2 }}
             vars = {{ MainTimerID = 7, CurrentRound = {round}, CurrentPlayerIndex = {index}, NumPlayers = 3,
                 PlayerOrderMap = {{ 1, 2, 3 }}, PlayerData = {{ {players}}} }}"
        );
        let setup = lua.load_source(stubs.as_bytes(), "stubs").expect("stubs compile");
        lua.call(&mut (), &setup, &[]).expect("stubs run");
        let main = lua.load_source(SKATE3_SOURCE.as_bytes(), "skate3_spot_battle").expect("source compiles");
        lua.call(&mut (), &main, &[]).expect("wrappers define themselves");
        let (data, vars) = (lua.get_global("data"), lua.get_global("vars"));
        for &timer in messages {
            let message = lua.create_table();
            #[expect(clippy::cast_precision_loss, reason = "a timer ID")]
            lua.table_mut(message).set_str("TimerID", Value::Number(timer as f32));
            let handler = if timer == 0 { SPOT_BATTLE_SEQUENCE_COMPLETE } else { SPOT_BATTLE_TIMER_EXPIRED };
            let wrapper = lua.get_global(&format!("h_skate3_{handler:016X}"));
            lua.call(&mut (), &wrapper, &[data.clone(), Value::Table(message), vars.clone()]).expect("wrapper runs");
        }
        let changed = match lua.get_global("changed") {
            Value::Nil => None,
            state => Some(lua.display(&state)),
        };
        let number = |value: Value| if let Value::Number(n) = value { Some(n) } else { None };
        let fade = number(lua.get_global("fade"));
        let Value::Table(vars) = vars else { unreachable!("vars is a table") };
        (changed, number(lua.table(vars).get_str("EndRunTimerID")), fade)
    }

    #[test]
    fn a_skate3_spot_battle_run_scores_on_for_two_seconds_but_the_last() {
        assert_eq!(skate3_end_run(1, 1, &[], &[7]), (None, Some(41.0), None), "the timer starts the end-of-run timer");
        assert_eq!(skate3_end_run(1, 1, &[], &[7, 41]), (Some("end_turn".into()), None, Some(0.5)), "which fades out and ends the turn");
        assert_eq!(skate3_end_run(1, 1, &[], &[7, 9]), (None, Some(41.0), None), "another timer leaves the run going");
        assert_eq!(skate3_end_run(2, 3, &[], &[7]), (Some("end_turn".into()), None, None), "the battle's last run ends at once");
        assert_eq!(skate3_end_run(2, 2, &[3], &[7]), (Some("end_turn".into()), None, None), "with every later player gone");
        assert_eq!(skate3_end_run(2, 2, &[], &[7]), (None, Some(41.0), None), "a player still to go keeps the grace");
        assert_eq!(skate3_end_run(1, 1, &[], &[0]), (None, Some(41.0), None), "the turn's last line starts it too");
        assert_eq!(skate3_end_run(1, 1, &[], &[0, 41]), (Some("end_turn".into()), None, Some(0.5)));
    }

    /// Skate 3's `countdown` entry over a stand-in for Skate 2's (which returns at once when
    /// locked out, ends the battle with `finishes`, and otherwise teleports the next player):
    /// after a run that `faded` out. Returns the state changed to, whether the stand-in
    /// teleported, and whether the fade-out was lifted.
    fn skate3_countdown(faded: bool, lock_out: bool, finishes: bool) -> (Option<String>, bool, bool) {
        use super::super::lua::{Lua, Value};
        let mut lua = Lua::<()>::new();
        let stubs = format!(
            "Effects = {{ eChallengeEffect = {{ FadeOut = 2 }}, ResetEffect = function(effect) if effect == 2 then reset = true end end }}
             function ChangeState(state) changed = state end
             function h_{SPOT_BATTLE_COUNTDOWN_ENTER:016X}(self, message, vars)
                 if vars.LockOut then return end
                 if {finishes} then ChangeState('challenge_finished') else teleported = true end
             end
             vars = {{ EndRunFadeOut = {faded}, LockOut = {lock_out} }}"
        );
        let setup = lua.load_source(stubs.as_bytes(), "stubs").expect("stubs compile");
        lua.call(&mut (), &setup, &[]).expect("stubs run");
        let main = lua.load_source(SKATE3_SOURCE.as_bytes(), "skate3_spot_battle").expect("source compiles");
        lua.call(&mut (), &main, &[]).expect("wrappers define themselves");
        let wrapper = lua.get_global(&format!("h_skate3_{SPOT_BATTLE_COUNTDOWN_ENTER:016X}"));
        let (message, vars) = (Value::Table(lua.create_table()), lua.get_global("vars"));
        lua.call(&mut (), &wrapper, &[Value::Nil, message, vars.clone()]).expect("wrapper runs");
        let changed = match lua.get_global("changed") {
            Value::Nil => None,
            state => Some(lua.display(&state)),
        };
        let set = |value: Value| !matches!(value, Value::Nil | Value::Boolean(false));
        let Value::Table(vars) = vars else { unreachable!("vars is a table") };
        assert!(!set(lua.table(vars).get_str("EndRunFadeOut")), "the fade is the next turn's to lift");
        (changed, set(lua.get_global("teleported")), set(lua.get_global("reset")))
    }

    #[test]
    fn a_skate3_spot_battle_fade_out_lasts_until_the_next_turn() {
        assert_eq!(skate3_countdown(true, false, false), (None, true, false), "the next turn's teleport takes the faded screen over");
        assert_eq!(skate3_countdown(true, false, true), (Some("challenge_finished".into()), false, true), "an end without a teleport lifts it");
        assert_eq!(skate3_countdown(true, true, false), (None, false, true), "and so does a locked-out countdown");
        assert_eq!(skate3_countdown(false, false, true), (Some("challenge_finished".into()), false, false), "a battle's last run never faded");
        assert_eq!(skate3_countdown(false, false, false), (None, true, false));
    }

    /// Skate 3's split trigger rule over a stand-in for Skate 2's trigger entry (which records
    /// the racer's `LastLapGateIndex` it is called with): a racer at `last` hits volume `volume`
    /// of a leg whose split triggers are volumes 101 to 103.
    fn skate3_split_trigger(last: i32, volume: i32) -> f32 {
        use super::super::lua::{Lua, Value};
        let mut lua = Lua::<()>::new();
        let stubs = format!(
            "function h_{DEATH_RACE_TRIGGER_ENTER:016X}(self, message, vars) seen = vars.Competitor[1].LastLapGateIndex end
             data = {{ RaceHeats = {{ {{ Legs = {{ {{ SplitTimeTriggers = {{ Count = 3, 101, 102, 103 }} }} }} }} }} }}
             vars = {{ CurrentHeat = 1, Competitor = {{ {{ CompetitorActive = true, CurrentLeg = 1, LastLapGateIndex = {last} }} }},
                 GetCompetitorIndex = function(vars, player) return player + 1 end }}
             message = {{ PlayerID = 0, VolumeID = {volume} }}"
        );
        let setup = lua.load_source(stubs.as_bytes(), "stubs").expect("stubs compile");
        lua.call(&mut (), &setup, &[]).expect("stubs run");
        let main = lua.load_source(SKATE3_SOURCE.as_bytes(), "skate3_handlers").expect("source compiles");
        lua.call(&mut (), &main, &[]).expect("wrappers define themselves");
        let wrapper = lua.get_global(&format!("h_skate3_{DEATH_RACE_TRIGGER_ENTER:016X}"));
        let args = [lua.get_global("data"), lua.get_global("message"), lua.get_global("vars")];
        lua.call(&mut (), &wrapper, &args).expect("wrapper runs");
        match lua.get_global("seen") {
            Value::Number(seen) => seen,
            _ => panic!("the shipped handler ran"),
        }
    }

    #[test]
    fn a_skate3_racer_may_pass_split_triggers_out_of_order() {
        assert_eq!(skate3_split_trigger(0, 103), 2.0, "past the next: Skate 2's in-order check then takes it");
        assert_eq!(skate3_split_trigger(0, 101), 0.0, "the next one as it is");
        assert_eq!(skate3_split_trigger(2, 101), 2.0, "one behind is not taken back");
        assert_eq!(skate3_split_trigger(1, 999), 1.0, "another volume");
    }

    /// What a `check_players` did ([`check_players`]).
    #[derive(Debug, PartialEq)]
    struct Checked {
        state: Option<String>,
        /// Each listed player's `disconnected` and `rank`.
        marks: Vec<(bool, f32)>,
        /// The stand-in `playerdisconnected`'s calls.
        replays: f32,
        /// `Entity.DoesSkaterExist` is the native again.
        restored: bool,
        ran: bool,
    }

    /// `handler`'s `check_players` as the challenge runs it (`call`: the port's wrapper, else the
    /// shipped handler) over the shipped `chunks` and `stubs`, in an online round that lists
    /// players 0 to 2: the given players disconnected, without a skater, and already marked.
    fn check_players(handler: u64, call: &str, chunks: &[&[u8]], stubs: &str, disconnected: &[i32], absent: &[i32], marked: &[i32]) -> Checked {
        use super::super::lua::{Lua, Value};
        let mut lua = Lua::<()>::new();
        let set = |players: &[i32]| players.iter().map(|player| format!("[{player}] = true, ")).collect::<String>();
        let skaters: String = (0..3)
            .map(|player| format!("{{ id = {player}, rank = 0, letters = 0, npc = false, disconnected = {} }}, ", marked.contains(&player)))
            .collect();
        let world = format!(
            "disconnected = {{ {} }}
             absent = {{ {} }}
             Online = {{ IsPlayerDisconnected = function(player) return disconnected[player] == true end }}
             native = function(player) return absent[player] ~= true end
             Entity = {{ DoesSkaterExist = native }}
             function ChangeState(state) changed = state end
             data = {{ Name = 'check', TurnBasedWaitingLocation = 0, VisualIndicators = {{ Count = 0 }} }}
             vars = {{ GameMode = 3, NumSkaters = 3, CurrentRank = 3, LocalPlayerID = 0, ChallengeRestarted = true, SkatersList = {{ {skaters}}} }}
             {stubs}",
            set(disconnected),
            set(absent)
        );
        let setup = lua.load_source(world.as_bytes(), "stubs").expect("stubs compile");
        lua.call(&mut (), &setup, &[]).expect("stubs run");
        for chunk in chunks {
            let main = lua.load(chunk, "shipped").expect("handler loads");
            lua.call(&mut (), &main, &[]).expect("handler defines itself");
        }
        let main = lua.load_source(FIXES_SOURCE.as_bytes(), "script_fixes").expect("source compiles");
        lua.call(&mut (), &main, &[]).expect("wrappers define themselves");
        let function = lua.get_global(&format!("{call}{handler:016X}"));
        let (data, vars) = (lua.get_global("data"), lua.get_global("vars"));
        let message = Value::Table(lua.create_table());
        let ran = lua.call(&mut (), &function, &[data, message, vars.clone()]).is_ok();
        let state = match lua.get_global("changed") {
            Value::Nil => None,
            state => Some(lua.display(&state)),
        };
        let number = |value: Value| if let Value::Number(n) = value { n } else { 0.0 };
        let Value::Table(vars) = vars else { unreachable!("vars is a table") };
        let Value::Table(list) = lua.table(vars).get_str("SkatersList") else { unreachable!("the list is a table") };
        let marks = (1..=3)
            .map(|i| {
                let Value::Table(skater) = lua.table(list).get_int(i) else { unreachable!("a listed skater") };
                (matches!(lua.table(skater).get_str("disconnected"), Value::Boolean(true)), number(lua.table(skater).get_str("rank")))
            })
            .collect();
        let restored = lua.load_source(b"restored = Entity.DoesSkaterExist == native", "check").and_then(|check| lua.call(&mut (), &check, &[])).is_ok()
            && matches!(lua.get_global("restored"), Value::Boolean(true));
        Checked { state, marks, replays: number(lua.get_global("replays")), restored, ran }
    }

    /// Stand-ins for S.K.A.T.E.'s `check_players` (waits unless every listed skater exists, fails
    /// with `fail`) and `global` `playerdisconnected` (marks the player at rank 9, counted).
    fn stand_ins() -> String {
        format!(
            "function h_{SKATE_CHECK_PLAYERS:016X}(self, message, vars)
                 if fail then error('fails') end
                 for i = 1, table.getn(vars.SkatersList) do
                     if Entity.DoesSkaterExist(vars.SkatersList[i].id) == false then ChangeState('wait') return end
                 end
                 ChangeState('round_setup')
             end
             function h_{SKATE_PLAYER_DISCONNECTED:016X}(self, message, vars)
                 replays = (replays or 0) + 1
                 for i = 1, table.getn(vars.SkatersList) do
                     if vars.SkatersList[i].id == message.PlayerIndex then vars.SkatersList[i].disconnected = true vars.SkatersList[i].rank = 9 end
                 end
             end"
        )
    }

    #[test]
    fn a_player_who_left_before_the_first_turn_holds_no_round_at_check_players() {
        let check = |disconnected: &[i32], absent: &[i32], marked: &[i32], stubs: &str| {
            check_players(SKATE_CHECK_PLAYERS, "h_fix_", &[], &format!("{}\n{stubs}", stand_ins()), disconnected, absent, marked)
        };
        let all = vec![(false, 0.0); 3];
        let gone = vec![(false, 0.0), (true, 9.0), (false, 0.0)];
        assert_eq!(check(&[], &[], &[], ""), Checked { state: Some("round_setup".into()), marks: all.clone(), replays: 0.0, restored: true, ran: true });
        assert_eq!(
            check(&[1], &[1], &[], ""),
            Checked { state: Some("round_setup".into()), marks: gone.clone(), replays: 1.0, restored: true, ran: true },
            "a player gone before the list was built is marked through the global handler"
        );
        assert_eq!(
            check(&[1], &[1], &[1], ""),
            Checked { state: Some("round_setup".into()), marks: vec![(false, 0.0), (true, 0.0), (false, 0.0)], replays: 0.0, restored: true, ran: true },
            "one the global handler marked is not marked again"
        );
        assert_eq!(check(&[], &[2], &[], ""), Checked { state: Some("wait".into()), marks: all, replays: 0.0, restored: true, ran: true }, "a skater still to spawn is waited for");
        assert_eq!(check(&[1], &[1, 2], &[], ""), Checked { state: Some("wait".into()), marks: gone.clone(), replays: 1.0, restored: true, ran: true });
        assert_eq!(
            check(&[1], &[1], &[], "fail = true"),
            Checked { state: None, marks: gone, replays: 1.0, restored: true, ran: false },
            "a failing handler's error still comes out, with the native back"
        );
    }

    /// The shipped handlers of a correction from the game's vaults.
    fn shipped_handlers(keys: &[u64]) -> Vec<Vec<u8>> {
        use super::super::attrib_db::{AttribDb, CAREER_VAULTS};
        use super::super::keys::class;
        use super::super::runtime::field;
        use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
        let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
        let archive = SKATE2_ARCHIVES.iter().find(|archive| archive.relative_path == "data\\db\\db.big").expect("db.big");
        let mut mounts = MountSet::new();
        mounts.push(MountedBig::open(archive.path_under(std::path::Path::new(&root)), archive.use_external_header).expect("db.big opens"));
        let db = AttribDb::load(&mounts, &CAREER_VAULTS).expect("vaults load");
        keys.iter()
            .map(|&key| db.attribute(class::CHALLENGE_MESSAGE_HANDLERS, key, field::HANDLER_BLOB).and_then(|blob| blob.blob()).expect("handler chunk").to_vec())
            .collect()
    }

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn the_shipped_check_players_go_on_without_a_player_who_left() {
        const NATIVES: &str = "Challenge = { GameMode = { PartyPlay = 1, SinglePlayer = 2, OnlineChallenge = 3 } }
             HUD2 = { eLeaderboardHUDMenuType = { Skate = 1, ThrowDown = 2 }, LeaderboardHUDInit = function() end, LeaderboardHUDSetRank = function() end,
                 LeaderboardHUDSetInt = function() end, LeaderboardHUDSetFloat = function() end, LeaderboardHUDSetPlayerHighlight = function() end }
             ChallengeManager = { SetPlayer = function() end, SetManualsAreConnectors = function() end }
             AI = { RepositionAI = function() end, PinAI = function() end, GetNumCompetitors = function() return 0 end }
             ChallengeSpeech = { StartThrowdownChallenge = function() end }
             VisualIndicators = { AddIndicator = function() end }
             Pause = { EnablePauseMenuAndBackpack = function() end }";
        for (check, disconnect) in FIXED_HANDLERS {
            let chunks = shipped_handlers(&[check, disconnect]);
            let chunks: Vec<&[u8]> = chunks.iter().map(Vec::as_slice).collect();
            let run = |call: &str, disconnected: &[i32], absent: &[i32]| {
                let checked = check_players(check, call, &chunks, NATIVES, disconnected, absent, &[]);
                assert!(checked.ran && checked.restored, "{check:016X}: {checked:?}");
                (checked.state, checked.marks.iter().map(|&(mark, _)| mark).collect::<Vec<_>>(), checked.marks[1].1)
            };
            assert_eq!(run("h_", &[1], &[1]).0.as_deref(), Some("wait"), "{check:016X} shipped: waits for the player who left");
            assert_eq!(run("h_fix_", &[1], &[1]), (Some("round_setup".into()), vec![false, true, false], 3.0), "{check:016X}: goes on, the player marked last");
            assert_eq!(run("h_fix_", &[], &[]).0.as_deref(), Some("round_setup"), "{check:016X}");
            assert_eq!(run("h_fix_", &[], &[2]).0.as_deref(), Some("wait"), "{check:016X}: a skater still to spawn");
        }
    }

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn a_disconnected_last_follower_ends_the_followers_turns() {
        let chunks = shipped_handlers(&[SKATE_PREPARE_FOLLOWER]);
        let shipped = chunks[0].as_slice();
        let fixed = corrected(SKATE_PREPARE_FOLLOWER, shipped);
        assert!(matches!(fixed, Cow::Owned(_)), "the shipped chunk holds the follower search once");

        assert_eq!(prepare_follower(shipped, &[4], 4, 3), (Some(1.0), None, 1.0), "shipped: the first follower takes the set trick again");
        assert_eq!(prepare_follower(&fixed, &[4], 4, 3), (None, Some("process_round".into()), 4.0));
        assert_eq!(prepare_follower(&fixed, &[3, 4], 3, 2), (None, Some("process_round".into()), 4.0));
        assert_eq!(prepare_follower(&fixed, &[2], 2, 1), (Some(3.0), None, 3.0), "a disconnected follower before the last is skipped as shipped");
        assert_eq!(prepare_follower(&fixed, &[], 4, 3), (Some(4.0), None, 4.0));
    }
}
