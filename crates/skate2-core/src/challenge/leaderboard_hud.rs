//! The turn-based challenges' HUD data, created with a group whose components include theirs:
//! `LeaderboardHUDData`, `ABDHUDData` (S.K.A.T.E.'s set trick, copy trick and
//! already-been-done list), `TimerHUDData`, `CountdownHUDData` and `ThrowDownMessageHUDData`,
//! and the `HUD2` natives that write them.

use super::{
    binding::{get_bool_or, get_float, get_int, get_int_or, get_string, get_string_or},
    hud::CentralMessage,
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    skate::{self, AlreadyBeenDone},
    system::{Output, World},
};

type R = LuaResult<Vec<Value>>;

fn none() -> R {
    Ok(Vec::new())
}

/// `eLeaderBoardHUD_UNDEFINED`.
pub const MENU_UNDEFINED: i32 = 13;
/// `eContestType_Unknown`.
pub const CONTEST_UNKNOWN: i32 = 18;

/// `LeaderboardHUDData::mContestTypeHalNamesJam` (`eContestType` order).
pub const CONTEST_TYPE_HAL_NAMES_JAM: [&str; 19] = [
    "ID_CONTEST_VARIANT_SEQUENCE_JAM",
    "ID_CONTEST_VARIANT_SEQUENCE_GRAB_JAM",
    "ID_CONTEST_VARIANT_SEQUENCE_GRIND_JAM",
    "ID_CONTEST_VARIANT_SEQUENCE_FLIP_JAM",
    "ID_CONTEST_VARIANT_SEQUENCE_FOOTPLANT_JAM",
    "ID_CONTEST_VARIANT_SEQUENCE_HANDPLANT_JAM",
    "ID_CONTEST_VARIANT_SEQUENCE_HIPPYJUMP_JAM",
    "ID_CONTEST_VARIANT_GRIND_LENGTH_JAM",
    "ID_CONTEST_VARIANT_SPEED_JAM",
    "ID_CONTEST_VARIANT_AIR_DISTANCE_JAM",
    "ID_CONTEST_VARIANT_AIR_GRAB_TIME_JAM",
    "ID_CONTEST_VARIANT_AIR_HEIGHT_JAM",
    "ID_CONTEST_VARIANT_AIR_SPIN_JAM",
    "ID_CONTEST_VARIANT_AIR_TIME_JAM",
    "ID_COMMON_UNKNOWN",
    "ID_CONTEST_VARIANT_AIR_ONLY",
    "ID_CONTEST_VARIANT_GRIND_ONLY",
    "ID_COMMON_UNKNOWN",
    "ID_COMMON_UNKNOWN",
];

/// `LeaderboardHUDData::mContestTypeHalNamesBestTrick` (`eContestType` order).
pub const CONTEST_TYPE_HAL_NAMES_BEST_TRICK: [&str; 19] = [
    "ID_CONTEST_VARIANT_SEQUENCE",
    "ID_CONTEST_VARIANT_SEQUENCE_GRAB",
    "ID_CONTEST_VARIANT_SEQUENCE_GRIND",
    "ID_CONTEST_VARIANT_SEQUENCE_FLIP",
    "ID_CONTEST_VARIANT_SEQUENCE_FOOTPLANT",
    "ID_CONTEST_VARIANT_SEQUENCE_HANDPLANT",
    "ID_CONTEST_VARIANT_SEQUENCE_HIPPYJUMP",
    "ID_CONTEST_VARIANT_GRIND_LENGTH",
    "ID_CONTEST_VARIANT_SPEED",
    "ID_CONTEST_VARIANT_AIR_DISTANCE",
    "ID_CONTEST_VARIANT_AIR_GRAB_TIME",
    "ID_CONTEST_VARIANT_AIR_HEIGHT",
    "ID_CONTEST_VARIANT_AIR_SPIN",
    "ID_CONTEST_VARIANT_AIR_TIME",
    "ID_CONTEST_VARIANT_AIR_LINE",
    "ID_CONTEST_VARIANT_AIR_ONLY",
    "ID_CONTEST_VARIANT_GRIND_ONLY",
    "ID_CONTEST_VARIANT_TRICK_LIST",
    "ID_COMMON_UNKNOWN",
];

/// `Sk8::HUD::PlayerData` (`0x34` bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct LeaderboardPlayer {
    pub rank: i32,
    pub int_var: i32,
    pub float_var: f32,
    /// `mPlayerName` (32 bytes).
    pub name: String,
    pub highlighted: bool,
    pub is_turn: bool,
    pub time: f32,
}

impl LeaderboardPlayer {
    /// One `Reset` row: rank `n`, `"#Player n"`.
    fn reset(n: i32) -> Self {
        Self { rank: n, int_var: 0, float_var: 0.0, name: format!("#Player {n}"), highlighted: false, is_turn: false, time: 0.0 }
    }
}

/// `LeaderboardHUDData` (`0x590` bytes; constructor `0x826b6e80` runs `Reset`).
#[derive(Clone, Debug, PartialEq)]
pub struct LeaderboardHud {
    pub dirty: bool,
    pub non_order_data_dirty: bool,
    pub num_players: i32,
    pub players: Vec<LeaderboardPlayer>,
    pub num_players_second: i32,
    pub players_second: Vec<LeaderboardPlayer>,
    /// `mLeaderboardHUDMenuType` (`eLeaderboardHUDMenuType`).
    pub menu_type: i32,
    pub second_menu_type: i32,
    pub score_to_beat: i32,
    pub current_player: i32,
    pub local_got_bonus: bool,
    pub contest_variant: i32,
    /// `mContestVariantName`: the localised `FormatString` of this id (`#Unknown` as is).
    pub contest_variant_name: String,
}

impl Default for LeaderboardHud {
    /// `Reset` (`0x826b6788`).
    fn default() -> Self {
        Self {
            dirty: true,
            non_order_data_dirty: false,
            num_players: 0,
            players: (1..=12).map(LeaderboardPlayer::reset).collect(),
            num_players_second: 0,
            players_second: (1..=12).map(LeaderboardPlayer::reset).collect(),
            menu_type: MENU_UNDEFINED,
            second_menu_type: MENU_UNDEFINED,
            score_to_beat: 0,
            current_player: 0,
            local_got_bonus: false,
            contest_variant: CONTEST_UNKNOWN,
            contest_variant_name: CONTEST_TYPE_HAL_NAMES_BEST_TRICK[18].to_owned(),
        }
    }
}

impl LeaderboardHud {
    fn player_mut(&mut self, index: i32) -> Option<&mut LeaderboardPlayer> {
        if index < 0 || index >= self.num_players {
            return None;
        }
        self.players.get_mut(usize::try_from(index).ok()?)
    }

    /// `SetNumPlayers` (`0x82742358`): fewer than 12.
    pub fn set_num_players(&mut self, count: i32) {
        self.num_players = count.clamp(0, 11);
        self.dirty = true;
    }

    /// `SetContestVariantNameString` (`0x826b2fc8`): the HAL id, which the movie formats, or
    /// `#Unknown` for `ID_COMMON_UNKNOWN` and `#Unknown`; then `MakeDirty`.
    pub fn set_contest_variant_name_string(&mut self, name: &str) {
        self.contest_variant_name = if name == "ID_COMMON_UNKNOWN" || name == "#Unknown" { "#Unknown".to_owned() } else { name.to_owned() };
        self.dirty = true;
    }

    /// `RefreshPlayerNames` (`0x826b2eb8`): `"#"` and each `PlayersBackEnd` name.
    pub fn refresh_player_names(&mut self, name: impl Fn(i32) -> String) {
        for i in 0..self.num_players {
            if let Some(player) = self.players.get_mut(usize::try_from(i).unwrap_or(usize::MAX)) {
                player.name = format!("#{}", name(i)).chars().take(31).collect();
            }
        }
    }
}

/// `ABDHUDData::ABDListItem`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AbdListItem {
    /// `HalName` (256 bytes).
    pub hal_name: String,
    pub spin_degrees: i32,
}

/// `ABDHUDData` (`0x938` bytes; constructor `0x826b3d40`).
#[derive(Clone, Debug, PartialEq, Eq)]
#[expect(clippy::struct_excessive_bools, reason = "native layout")]
pub struct AbdHud {
    pub dirty: bool,
    pub num_tricks_in_list: i32,
    pub list_items_visible: bool,
    pub gesture_area_visible: bool,
    pub list: [AbdListItem; 5],
    pub set_hal_name: String,
    pub set_gesture: String,
    pub set_spin: i32,
    pub copy_hal_name: String,
    pub copy_gesture: String,
    pub copy_spin: i32,
    pub update_list: bool,
    pub update_set_trick: bool,
    pub update_copy_trick: bool,
    pub update_list_visibility: bool,
    pub update_gesture_visibility: bool,
}

impl Default for AbdHud {
    fn default() -> Self {
        Self {
            dirty: false,
            num_tricks_in_list: 0,
            list_items_visible: true,
            gesture_area_visible: true,
            list: Default::default(),
            set_hal_name: String::new(),
            set_gesture: String::new(),
            set_spin: 0,
            copy_hal_name: String::new(),
            copy_gesture: String::new(),
            copy_spin: 0,
            update_list: false,
            update_set_trick: false,
            update_copy_trick: false,
            update_list_visibility: false,
            update_gesture_visibility: false,
        }
    }
}

/// A trick's HUD view: `mTrickNameHal`, `GetGestureString` of its first input's gesture and
/// `180 * |mCompletedAir180Count|`; an empty pattern reads `"#"`, `"none"` and 0.
fn trick_view(world: &World, item: &AlreadyBeenDone) -> (String, String, i32) {
    if item.pattern.length <= 0 {
        return ("#".to_owned(), "none".to_owned(), 0);
    }
    let index = item.pattern.items[0].1;
    let hal = skate::trick_name_hal(&world.db, index).unwrap_or_default();
    let gesture = super::tutorial_hud::gesture_string(skate::trick_gesture(&world.db, index)).to_owned();
    (hal, gesture, 180 * item.completed_air_180_count.abs())
}

impl AbdHud {
    /// `SetListRefresh(index, tAlreadyBeenDone)` (`0x826b7720`): the entry's name and spin, an
    /// empty entry `"#"` and 0; an index below 5 sets the count to it plus one.
    fn set_list_item(&mut self, index: i32, hal_name: &str, spin: i32) {
        self.dirty = true;
        self.update_list = true;
        if let Some(item) = usize::try_from(index).ok().and_then(|i| self.list.get_mut(i)) {
            self.num_tricks_in_list = index + 1;
            item.hal_name = hal_name.to_owned();
            item.spin_degrees = spin;
        }
    }
}

/// `TimerHUDData` (`0x18` bytes; constructor `0x826b38f8`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimerHud {
    pub dirty: bool,
    pub timer_id: i32,
    /// `eTimerModes`: 0 challenge, 1 online.
    pub timer_mode: i32,
    pub current_online_time: i32,
}

impl Default for TimerHud {
    fn default() -> Self {
        Self { dirty: false, timer_id: -1, timer_mode: 0, current_online_time: 0 }
    }
}

/// `CountdownHUDData` (`0x10` bytes; constructor `0x826b3a80`): the timer whose seconds count
/// down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountdownHud {
    pub dirty: bool,
    pub timer_id: i32,
}

impl Default for CountdownHud {
    fn default() -> Self {
        Self { dirty: true, timer_id: 0 }
    }
}

/// `ThrowDownMessageHUDData` (`0x18` bytes; constructor `0x826b3fc8`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThrowDownMessageHud {
    pub skate_mode: bool,
    pub message_state: i32,
    pub message_dirty: bool,
}

impl Default for ThrowDownMessageHud {
    fn default() -> Self {
        Self { skate_mode: true, message_state: -1, message_dirty: false }
    }
}

fn leaderboard<'a>(world: &'a mut World, group: &str) -> Option<&'a mut LeaderboardHud> {
    world.hud.group_mut(group)?.leaderboard.as_mut()
}

/// `HUD2.LeaderboardHUDInit(group, menuType, numPlayers, highlighted = local player)`
/// (`0x82898a10`): only the highlighted row stays highlighted. Skate 3's (`0x82851018`) takes
/// whether teams play as its fourth argument instead and highlights the local player's row; the
/// port has no teams.
fn leaderboard_init(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let menu_type = get_int(call, 1);
    let count = get_int(call, 2);
    let highlighted = if matches!(call.arg(3), Value::Boolean(_)) { world.local_player } else { get_int_or(call, 3, world.local_player) };
    if let Some(data) = leaderboard(world, &group) {
        data.menu_type = menu_type;
        data.dirty = true;
        data.set_num_players(count);
        for i in 0..data.num_players {
            if let Some(player) = data.player_mut(i) {
                player.highlighted = i == highlighted;
            }
            data.non_order_data_dirty = true;
        }
    }
    none()
}

/// `HUD2.LeaderboardSetTurn(group, player)` (`0x82898d98`). Skate 3's (`0x828513c8`) adds
/// `(isTurn = true, exclusive = true)`: exclusive, every row's turn is the player's and
/// `isTurn`, as Skate 2's; otherwise only the player's row takes `isTurn` (a team's turn).
fn leaderboard_set_turn(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let turn = get_int(call, 1);
    let is_turn = get_bool_or(call, 2, true);
    let exclusive = get_bool_or(call, 3, true);
    if let Some(data) = leaderboard(world, &group) {
        if !exclusive {
            if let Some(player) = data.player_mut(turn) {
                player.is_turn = is_turn;
            }
            data.non_order_data_dirty = true;
            return none();
        }
        for i in 0..data.num_players {
            if let Some(player) = data.player_mut(i) {
                player.is_turn = i == turn && is_turn;
            }
            data.non_order_data_dirty = true;
        }
    }
    none()
}

/// `HUD2.LeaderboardHUDSetRank(group, player, rank)` (`0x828990b8`): `SetPlayerRank`
/// (`0x825ad6d8`).
fn leaderboard_set_rank(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let (index, rank) = (get_int(call, 1), get_int(call, 2));
    if let Some(data) = leaderboard(world, &group) {
        if let Some(player) = data.player_mut(index) {
            player.rank = rank;
        }
        data.dirty = true;
    }
    none()
}

/// `HUD2.LeaderboardHUDSetInt(group, player, value)` (`0x82899258`).
fn leaderboard_set_int(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let (index, value) = (get_int(call, 1), get_int(call, 2));
    if let Some(data) = leaderboard(world, &group) {
        if let Some(player) = data.player_mut(index) {
            player.int_var = value;
        }
        data.non_order_data_dirty = true;
    }
    none()
}

/// `HUD2.LeaderboardHUDSetFloat(group, player, value)` (`0x828993e8`): `SetPlayerFloatVar`
/// (`0x8288e8e0`).
fn leaderboard_set_float(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let (index, value) = (get_int(call, 1), get_float(call, 2));
    if let Some(data) = leaderboard(world, &group) {
        if let Some(player) = data.player_mut(index) {
            player.float_var = value;
        }
        data.non_order_data_dirty = true;
    }
    none()
}

/// `HUD2.LeaderboardHUDReset(group)` (`0x828995a0`).
fn leaderboard_reset(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    if let Some(data) = leaderboard(world, &group) {
        *data = LeaderboardHud::default();
    }
    none()
}

/// `HUD2.LeaderboardHUDSetVariantString(group, halId)` (`0x828996c0`):
/// `SetContestVariantNameString` (`0x826b2fc8`).
fn leaderboard_set_variant_string(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let name = get_string(call, 1).unwrap_or_default();
    if let Some(data) = leaderboard(world, &group) {
        data.set_contest_variant_name_string(&name);
    }
    none()
}

/// `HUD2.LeaderboardHUDSetPlayerHighlight(group, player = 0, highlighted = true)`
/// (`0x82899740`).
fn leaderboard_set_player_highlight(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let index = get_int_or(call, 1, 0);
    let highlighted = get_bool_or(call, 2, true);
    if let Some(data) = leaderboard(world, &group) {
        if let Some(player) = data.player_mut(index) {
            player.highlighted = highlighted;
        }
        data.non_order_data_dirty = true;
    }
    none()
}

/// `HUD2.StartTimer(group, timerId)` (`0x828988f8`): the timer data's id, and the group's
/// `timer` instance shows.
fn start_timer(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let timer = get_int(call, 1);
    if let Some(data) = world.hud.group_mut(&group).and_then(|g| g.timer.as_mut()) {
        data.timer_id = timer;
        data.dirty = true;
        world.hud.show(&group, "timer", true);
    }
    none()
}

/// `HUD2.StartCountdown(group, timer)` (`0x82898878`): the countdown data's timer, dirty, and
/// the group's `countdown` instance shown.
fn start_countdown(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let timer = get_int(call, 1);
    if let Some(data) = world.hud.group_mut(&group).and_then(|g| g.countdown.as_mut()) {
        data.timer_id = timer;
        data.dirty = true;
        world.hud.show(&group, "countdown", true);
    }
    none()
}

/// `HUD2.ShowSkateMessage(group, state, skateMode = true)` (`0x82898978`).
fn show_skate_message(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let state = get_int(call, 1);
    let skate_mode = get_bool_or(call, 2, true);
    if let Some(data) = world.hud.group_mut(&group).and_then(|g| g.throwdown.as_mut()) {
        data.skate_mode = skate_mode;
        data.message_state = state;
        data.message_dirty = true;
    }
    none()
}

/// `HUD2.ShowModifiedCentralMessage(halId, colour, s1..s4)` (`0x828964c0`):
/// `CentralHUD::ShowModifiedCentralMessage` (`0x826bc050`), the line under the central
/// message. Skate 3's (TU3 `0x82850770`) takes a seventh, `stayUp = false`, which Skate 2's
/// scripts never pass.
fn show_modified_central_message(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let message = CentralMessage {
        hal_id: get_string(call, 0).unwrap_or_default(),
        colour: get_int_or(call, 1, 0),
        args: [get_string_or(call, 2, ""), get_string_or(call, 3, ""), get_string_or(call, 4, ""), get_string_or(call, 5, "")],
        audio_hook: 0,
    };
    let stay_up = get_bool_or(call, 6, false);
    world.output(Output::ModifiedCentralMessage(super::hud::ModifiedCentral::Show { message, stay_up }));
    none()
}

/// `HUD2.ABDHUDRefreshTrickList(group)` (`0x82899f70`): `SetListRefresh` of every entry up
/// to the limit.
fn abd_refresh_trick_list(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    if world.hud.group(&group).and_then(|g| g.abd.as_ref()).is_none() {
        return none();
    }
    let views: Vec<(String, i32)> = (0..world.skate.limit)
        .map(|i| {
            let item = *world.skate.item(i);
            let (hal, _, spin) = trick_view(world, &item);
            (hal, spin)
        })
        .collect();
    if let Some(data) = world.hud.group_mut(&group).and_then(|g| g.abd.as_mut()) {
        for (i, (hal, spin)) in (0..).zip(views) {
            data.set_list_item(i, &hal, spin);
        }
    }
    none()
}

/// `HUD2.ABDHUDRefreshFilmTrickList(group, objectiveGroup)` (`0x8289a088`): the first five of the
/// objective group's completed unique tricks (`GetUniqueTrickData`) through `SetListRefresh(index,
/// scoring_trick)` (`0x826b68c8`): the trick's `mTrickNameHal` and no spin.
fn abd_refresh_film_trick_list(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let objective_group = world
        .pointers
        .object(call.arg(1), super::attrib_lua::ObjectKind::ObjectiveGroup)
        .ok_or_else(|| lua.error("HUD2.ABDHUDRefreshFilmTrickList: expected an objective group".to_owned()))?;
    if world.hud.group(&group).and_then(|g| g.abd.as_ref()).is_none() {
        return none();
    }
    let tricks = world
        .objectives
        .group_index(objective_group)
        .map_or_else(Vec::new, |g| world.objectives.groups[g].completed_unique_tricks.iter().take(5).copied().collect::<Vec<u64>>());
    let names: Vec<String> =
        tricks.iter().map(|&trick| world.db.layout_text(skate::TRICK_CLASS, trick, 0).unwrap_or_default().to_owned()).collect();
    if let Some(data) = world.hud.group_mut(&group).and_then(|g| g.abd.as_mut()) {
        for (index, name) in (0..).zip(names) {
            data.set_list_item(index, &name, 0);
        }
    }
    none()
}

/// `HUD2.ABDHUDRefreshSetTrick(group)` (`0x8289a1d8`): `SetTrickRefresh` (`0x826b7858`) of the
/// set trick.
fn abd_refresh_set_trick(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let set = world.skate.set;
    let (hal, gesture, spin) = trick_view(world, &set);
    if let Some(data) = world.hud.group_mut(&group).and_then(|g| g.abd.as_mut()) {
        data.dirty = true;
        data.update_set_trick = true;
        data.set_hal_name = hal;
        data.set_spin = spin;
        data.set_gesture = gesture;
    }
    none()
}

/// `HUD2.ABDHUDRefreshCopyTrick(group)` (`0x8289a2c0`): `CopyTrickRefresh` (`0x826b7978`) of
/// the attempt.
fn abd_refresh_copy_trick(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let attempt = world.skate.attempt;
    let (hal, gesture, spin) = trick_view(world, &attempt);
    if let Some(data) = world.hud.group_mut(&group).and_then(|g| g.abd.as_mut()) {
        data.dirty = true;
        data.update_copy_trick = true;
        data.copy_hal_name = hal;
        data.copy_spin = spin;
        data.copy_gesture = gesture;
    }
    none()
}

/// `HUD2.LeaderboardHUDSetVariantType(group, variant, bestTrick = true)` (`0x82899628`):
/// `SetContestTypeVariant` (`0x826b2f30`), the variant and its best trick or jam name.
fn leaderboard_set_variant_type(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let variant = get_int(call, 1);
    let best_trick = get_bool_or(call, 2, true);
    if let Some(data) = leaderboard(world, &group) {
        data.contest_variant = variant;
        let names = if best_trick { &CONTEST_TYPE_HAL_NAMES_BEST_TRICK } else { &CONTEST_TYPE_HAL_NAMES_JAM };
        if let Some(name) = usize::try_from(variant).ok().and_then(|index| names.get(index)) {
            (*name).clone_into(&mut data.contest_variant_name);
        }
        data.dirty = true;
    }
    none()
}

/// `HUD2.SetABDGestureVisible(group, visible = false)` (`0x82899ed8`).
fn abd_set_gesture_visible(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let visible = get_bool_or(call, 1, false);
    if let Some(data) = world.hud.group_mut(&group).and_then(|g| g.abd.as_mut()) {
        data.dirty = true;
        data.gesture_area_visible = visible;
        data.update_gesture_visibility = true;
    }
    none()
}

/// `HUD2.ABDHUDClearList(group)` (`0x8289a3a8`): `ABDHUDData::ClearList` (`0x826b6970`), whose
/// loop count follows the list size it rewrites, so only the first entry is cleared.
fn abd_clear_list(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    if let Some(data) = world.hud.group_mut(&group).and_then(|g| g.abd.as_mut()) {
        let mut index = 0;
        if data.num_tricks_in_list > 0 {
            loop {
                data.dirty = true;
                data.update_list = true;
                if let Some(item) = usize::try_from(index).ok().and_then(|i| data.list.get_mut(i)) {
                    data.num_tricks_in_list = index + 1;
                    "#".clone_into(&mut item.hal_name);
                    item.spin_degrees = 0;
                }
                index += 1;
                if index >= data.num_tricks_in_list {
                    break;
                }
            }
        }
        data.num_tricks_in_list = 0;
        data.dirty = true;
    }
    none()
}

/// `HUD2.ABDHUDSetListVisible(group, visible = true)` (`0x8289a480`).
fn abd_set_list_visible(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = get_string(call, 0).unwrap_or_default();
    let visible = get_bool_or(call, 1, true);
    if let Some(data) = world.hud.group_mut(&group).and_then(|g| g.abd.as_mut()) {
        data.list_items_visible = visible;
        data.update_list_visibility = true;
    }
    none()
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("HUD2", "LeaderboardHUDInit", leaderboard_init),
        ("HUD2", "LeaderboardSetTurn", leaderboard_set_turn),
        ("HUD2", "LeaderboardHUDSetRank", leaderboard_set_rank),
        ("HUD2", "LeaderboardHUDSetInt", leaderboard_set_int),
        ("HUD2", "LeaderboardHUDSetFloat", leaderboard_set_float),
        ("HUD2", "LeaderboardHUDReset", leaderboard_reset),
        ("HUD2", "LeaderboardHUDSetVariantString", leaderboard_set_variant_string),
        ("HUD2", "LeaderboardHUDSetPlayerHighlight", leaderboard_set_player_highlight),
        ("HUD2", "StartTimer", start_timer),
        ("HUD2", "StartCountdown", start_countdown),
        ("HUD2", "ShowSkateMessage", show_skate_message),
        ("HUD2", "ShowModifiedCentralMessage", show_modified_central_message),
        ("HUD2", "ABDHUDRefreshTrickList", abd_refresh_trick_list),
        ("HUD2", "ABDHUDRefreshFilmTrickList", abd_refresh_film_trick_list),
        ("HUD2", "ABDHUDRefreshSetTrick", abd_refresh_set_trick),
        ("HUD2", "ABDHUDRefreshCopyTrick", abd_refresh_copy_trick),
        ("HUD2", "ABDHUDSetListVisible", abd_set_list_visible),
        ("HUD2", "LeaderboardHUDSetVariantType", leaderboard_set_variant_type),
        ("HUD2", "SetABDGestureVisible", abd_set_gesture_visible),
        ("HUD2", "ABDHUDClearList", abd_clear_list),
    ]
}
