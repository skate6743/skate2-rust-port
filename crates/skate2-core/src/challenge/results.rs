//! The front end's end of a challenge: `FrontEndState_LeaderBoard` (FE state 27) as the
//! scripts fill it (`FrontEnd.Results_*`, `SetEndChallengeOption`), and
//! `FrontEndState_EndChallengeOptions` (FE state 8), which `FrontEnd.OpenContinueRetryDialog`
//! pushes and whose A, B, X and Y pick the current leaderboard's options (`ReceiveInput`
//! `0x8267c798`, `OptionSelect` `0x82676da0`).
//!
//! The state keeps three leaderboards: the round results, the round results' second board and
//! the final results. The natives write the current one (`mCurrentLeaderboard`) or the second
//! current one (`mSecondCurrentLeaderboard`); `Results_SetActiveResultsMenu` points them at the
//! round or the final results.

use super::{
    binding::{get_bool_or, get_float, get_float_or, get_int, get_int_or, get_string},
    keys::class,
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    signup,
    system::{Output, World},
};

type R = LuaResult<Vec<Value>>;

fn none() -> R {
    Ok(Vec::new())
}

/// `eSk8BEEndMissionOptions`.
pub mod option {
    pub const CONTINUE: i32 = 0;
    pub const RETRY: i32 = 1;
    pub const REMATCH: i32 = 2;
    pub const VIEW_PHOTOS: i32 = 3;
    pub const VIEW_REPLAY: i32 = 4;
    pub const SAVE_REPLAY: i32 = 5;
    pub const ONLINE_SCORES: i32 = 6;
    pub const VIEW_DEMO_PHOTOS: i32 = 7;
    pub const QUIT: i32 = 8;
    pub const HIGHLIGHT_REEL: i32 = 9;
    pub const CONTINUE_RETRY: i32 = 10;
    pub const RETRY_QUIT: i32 = 11;
    pub const UNKNOWN: i32 = 12;
    pub const NEXT: i32 = 13;
    pub const PREVIOUS: i32 = 14;
    pub const SHOW_GAMER_CARD: i32 = 15;
    pub const EXIT_PARTY_PLAY: i32 = 16;
    pub const ADJUST_SETTINGS: i32 = 17;
}

/// `gEndMissionObjectives` (`0x8206b590`): each option's `mText`.
pub const OPTION_TEXT: [&str; 18] = [
    "ID_COMMON_CONTINUE",
    "ID_COMMON_RETRY",
    "ID_PARTYPLAY_REMATCH",
    "ID_MEDIA_VIEW_PHOTO_CAPS",
    "ID_REPLAY_EDITREPLAY",
    "ID_COMMON_SAVE_REPLAY_CAPS",
    "ID_ONLINE_SCORES_CAPS",
    "ID_DEMO_PHOTOCHALLENGE_VIEWPHOTO",
    "ID_COMMON_QUIT",
    "ID_COMMON_HIGHLIGHTREEL_CAPS",
    "ID_COMMON_UNKNOWN",
    "ID_COMMON_UNKNOWN",
    "ID_COMMON_UNKNOWN",
    "ID_COMMON_NEXT",
    "ID_COMMON_PREVIOUS",
    "ID_LOGBOOK_NOT_COMPLETED",
    "ID_PARTYPLAY_EXIT_GAME",
    "ID_PARTYPLAY_CHANGE_SETTINGS",
];

/// `eThrowDownType`.
pub mod throwdown {
    pub const NONE: i32 = 0;
    pub const ONE_UP: i32 = 1;
    pub const SKATE: i32 = 2;
}

/// `eResultGroupType`.
pub mod result_group {
    pub const FINAL_RESULTS: i32 = 0;
    pub const ROUND_RESULTS: i32 = 1;
}

/// `eResultsLBType_PlaceNameInt`.
const PLACE_NAME_INT: i32 = 0;
/// `eResultsLBType_GoofyVsRegularTeamResults`.
const GOOFY_VS_REGULAR_TEAM_RESULTS: i32 = 8;
/// `eChallengeTypes_BestTrick`.
const BEST_TRICK: i32 = 8;
/// `challenges.ContestTeams`.
const CONTEST_TEAMS: u64 = 0x0e22_fd36_2785_9240;

/// The screens `FrontEndState_EndChallengeOptions::OnEnter` opens (`gScreenTemplate` ids).
pub mod screen {
    /// `EndChallengeOptions`, hudComponents/endchallengeoptions.
    pub const RESULTS: i32 = 41;
    /// `EndChallengeOptions_Small`, hudComponents/endchallenge_small.
    pub const RESULTS_SMALL: i32 = 42;
    /// `EndChallengeOptions_GVR`, hudComponents/endchallenge_gvr.
    pub const RESULTS_GVR: i32 = 43;
    /// `EndChallengeOptionsFail`, hudComponents/challengefailed.
    pub const FAILED: i32 = 44;
    /// `EndChallengeOptionsFail_Small`, hudComponents/challengefailed_small.
    pub const FAILED_SMALL: i32 = 45;
    /// `ThrowDownEndChallengeOptions`, challenge/throwdownresults.
    pub const THROWDOWN: i32 = 47;
    /// The online end of a challenge (`mIsOnlineDisplay`).
    pub const ONLINE: i32 = 117;
}
/// The rows `tLeaderboardData` holds.
pub const RESULTS_PLAYERS: usize = 12;
/// `mPlayerName`'s `Snprintf` size.
const NAME_SIZE: usize = 0x14;
/// `mGoofyTeamHAL` (`0x82fdf3e8`), which `SetPlayerData` writes with `Snprintf` size `0x11`.
const GOOFY_TEAM_HAL: &str = "ID_GVR_TEAM_GOOFY";
/// `mRegularTeamHAL` (`0x82fdf3ec`), which `SetPlayerData` writes with `Snprintf` size `0x13`.
const REGULAR_TEAM_HAL: &str = "ID_GVR_TEAM_REGULAR";

/// `tResultsPlayerData`, as the `tLeaderboardData` constructor (`0x82647b68`) and `Reset` leave
/// it: `#EMPTY` with `#NOSTATUS`.
#[derive(Clone, Debug, PartialEq)]
pub struct ResultsPlayer {
    pub position: i32,
    pub score: i32,
    pub float_score: f32,
    /// `mPlayerName`: `#` and the player's name, or a Goofy vs Regular team's HAL id.
    pub name: String,
    /// `mPlayerStatus`: `#NOSTATUS`, or the HAL id `SetPlayerStatusHAL` (`0x82641b80`) formats
    /// (every argument a string) into 19 bytes.
    pub status: String,
    /// The time `Results_GetLeaderboardDataBlock` formats; a negative one is `ID_RACE_DNF`.
    pub time: f32,
    pub bonus: i32,
    pub is_goofy: bool,
}

impl Default for ResultsPlayer {
    fn default() -> Self {
        Self {
            position: 0,
            score: 0,
            float_score: 0.0,
            name: "#EMPTY".to_owned(),
            status: "#NOSTATUS".to_owned(),
            time: 0.0,
            bonus: 0,
            is_goofy: false,
        }
    }
}

/// `FrontEndState_LeaderBoard::tLeaderboardData`. Its constructor sets only the players.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LeaderboardData {
    pub players: Vec<ResultsPlayer>,
    /// `mColTitles`: each column's HAL id, which `SetColTitle` (`0x82641cb0`) formats, or
    /// `Reset`'s `#COL1` to `#COL9`.
    pub col_titles: [String; 9],
    pub num_players: i32,
    pub results_type: i32,
    pub player_to_highlight: i32,
    /// `mResultsMainTitle`: the HAL id `SetResultsMainTitle` (`0x82641da0`) formats, or
    /// `Reset`'s unformatted `ID_COMMON_UNKNOWN`.
    pub main_title: String,
    pub options: [i32; 4],
    pub self_rank: i32,
}

impl LeaderboardData {
    fn constructed() -> Self {
        Self { players: vec![ResultsPlayer::default(); RESULTS_PLAYERS], ..Self::default() }
    }

    /// `GetNumOptions` (`0x82641e58`): the options before the first `Unknown`.
    #[must_use]
    pub fn num_options(&self) -> usize {
        self.options.iter().take_while(|&&o| o != option::UNKNOWN).count()
    }

    /// `GetOption` (`0x82657b40`): Continue_Retry is Continue after a win and Retry after a
    /// loss, Retry_Quit Retry after a win and Quit after a loss.
    #[must_use]
    pub fn option(&self, index: usize, win: bool) -> i32 {
        match self.options.get(index).copied().unwrap_or(option::UNKNOWN) {
            option::CONTINUE_RETRY => {
                if win {
                    option::CONTINUE
                } else {
                    option::RETRY
                }
            }
            option::RETRY_QUIT => {
                if win {
                    option::RETRY
                } else {
                    option::QUIT
                }
            }
            other => other,
        }
    }

    fn player_mut(&mut self, index: i32) -> Option<&mut ResultsPlayer> {
        usize::try_from(index).ok().and_then(|i| self.players.get_mut(i))
    }
}

/// One player's line of `Message_GameFinalResults` as `StateOnlineGame::OnMessage(MT_GameFinalResults)`
/// (0x8276b060) writes it into FE state 27's final results.
#[derive(Clone, Debug, PartialEq)]
pub struct OnlineResultRow {
    /// `GetPlayerIndexByPeerID`: the player's index in the challenge.
    pub player: usize,
    /// `mPlayerName`.
    pub name: String,
    /// The row's place, score, experience and event time (`SetPlayerData`, `time`).
    pub position: i32,
    pub score: i32,
    pub experience: f32,
    pub time: f32,
    pub cash: i32,
    pub players_choice: bool,
}

/// FE state 8's online display (`mIsOnlineDisplay`, screen 117 `ranked/ranked_results_overall`):
/// `Mission.GetOnlineChallengeInstanceData` (0x82625148), `GetOnlineChallengeTypeName`
/// (0x8260dea8), the players' cash and choice (`Results_GetOnlineLeaderboardCashAndEXPDataBlock`
/// 0x826b98d0), `GetEnableOnlineQuitButton` (0x82642340) and the post-event timer
/// (`OnlineBackEnd::GetPostEventDisplayTime` 0x8255d110).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OnlineResults {
    /// The challenge's `Title`.
    pub title: String,
    /// The `fe_map_icons` name of the challenge's icon.
    pub icon: String,
    pub free_skate: bool,
    /// FE state 27's freeskate activity won.
    pub activity_won: bool,
    pub ranked: bool,
    /// `GetChallengeColumnHeader` 0x82605a50.
    pub column_header: String,
    pub skate: bool,
    pub last_round: bool,
    /// A round's board (`kScreenMode_Round`): FE state 27's active leaderboard
    /// (`Results_GetLeaderboardType`) was the round results as the display opened.
    pub round: bool,
    /// `gChallengeTemplate[type].mTitle`.
    pub type_name: String,
    /// `mOnlinePlayersChoice`, -1 for none.
    pub players_choice: i32,
    /// Each player's cash and experience by player index.
    pub cash: Vec<i32>,
    pub experience: Vec<f32>,
    pub quit_enabled: bool,
    /// `mOnlineResultsTimer`: seconds left of the board.
    pub timer: i32,
    /// `mOnlineRoundResultsTimer`: FE state 27's timer (`mTimerID`) in seconds, rounded up.
    pub round_timer: i32,
    /// Which opening of the display this is: each `OnEnter` opens screen 117 anew.
    pub instance: u32,
    /// The list's `CurrentValue`.
    pub selected: usize,
    /// `mPlayerIndexByRank`.
    pub player_by_rank: Vec<usize>,
}

impl OnlineResults {
    /// The final results board of the rows (`SetActiveResultsLeaderboard(FinalResults)` with the
    /// layout `SetResultsMenuType` gives the challenge: Death Race's time, a freeskate activity's
    /// by its type, otherwise place and score).
    #[must_use]
    pub fn leaderboard(rows: &[OnlineResultRow], results_type: i32) -> LeaderboardData {
        let mut board = LeaderboardData::constructed();
        board.num_players = i32::try_from(rows.len()).unwrap_or(0);
        board.results_type = results_type;
        for row in rows {
            if let Some(player) = board.players.get_mut(row.player) {
                player.position = row.position;
                player.score = row.score;
                player.float_score = row.experience;
                player.time = row.time;
                player.name = format!("#{}", row.name);
            }
        }
        board
    }

    /// `mPlayerIndexByRank`, `NextValue` and `PrevValue` of the list: a step within the rows,
    /// sounded even at either end.
    pub fn step(&mut self, forward: bool) -> &'static str {
        let count = self.player_by_rank.len();
        if forward {
            if self.selected + 1 < count {
                self.selected += 1;
            }
            "FE_SOUND_CORE_NAV_DOWN"
        } else {
            self.selected = self.selected.saturating_sub(1);
            "FE_SOUND_CORE_NAV_UP"
        }
    }
}

/// The leaderboard `mCurrentLeaderboard` or `mSecondCurrentLeaderboard` points at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Board {
    /// `mRoundResults`.
    Round,
    /// `mRoundResultsSecond`.
    RoundSecond,
    /// `mFinalResults`.
    Final,
}

/// `FrontEndState_LeaderBoard`, constructed in `StateManagerFrontEnd::Initialize`
/// (`0x8268c330`) and `Reset`. Of its online fields only those the natives write are modelled.
#[derive(Clone, Debug, PartialEq)]
pub struct LeaderboardState {
    pub round: LeaderboardData,
    pub round_second: LeaderboardData,
    pub final_results: LeaderboardData,
    pub current: Board,
    pub second: Board,
    /// `mCurrentResultType`.
    pub current_result_type: i32,
    /// `mRoundUpdateRedraw`.
    pub round_update_redraw: bool,
    /// `mResultsNavigatable`.
    pub results_navigatable: bool,
    /// `mTimerID`: the online results' countdown timer.
    pub timer_id: i32,
    /// `mOnlineIsResultsForLastRound`.
    pub online_is_results_for_last_round: bool,
}

impl Default for LeaderboardState {
    fn default() -> Self {
        let mut state = Self {
            round: LeaderboardData::constructed(),
            round_second: LeaderboardData::constructed(),
            final_results: LeaderboardData::constructed(),
            current: Board::Final,
            second: Board::Final,
            current_result_type: result_group::FINAL_RESULTS,
            round_update_redraw: false,
            results_navigatable: false,
            timer_id: -1,
            online_is_results_for_last_round: false,
        };
        state.reset();
        state
    }
}

impl LeaderboardState {
    fn board_mut(&mut self, board: Board) -> &mut LeaderboardData {
        match board {
            Board::Round => &mut self.round,
            Board::RoundSecond => &mut self.round_second,
            Board::Final => &mut self.final_results,
        }
    }

    /// `mCurrentLeaderboard`.
    #[must_use]
    pub fn current(&self) -> &LeaderboardData {
        match self.current {
            Board::Round => &self.round,
            Board::RoundSecond => &self.round_second,
            Board::Final => &self.final_results,
        }
    }

    fn current_mut(&mut self) -> &mut LeaderboardData {
        self.board_mut(self.current)
    }

    /// `mSecondCurrentLeaderboard`.
    #[must_use]
    pub fn second(&self) -> &LeaderboardData {
        match self.second {
            Board::Round => &self.round,
            Board::RoundSecond => &self.round_second,
            Board::Final => &self.final_results,
        }
    }

    fn second_mut(&mut self) -> &mut LeaderboardData {
        self.board_mut(self.second)
    }

    /// `Reset` (`0x82647c10`): the final results current; every board's players, counts,
    /// rank, highlight, layout and main title cleared; the round and final results' options
    /// Continue_Retry then Retry_Quit and their column titles `#COL1` to `#COL9`. The second
    /// title write repeats the round results' pointer (`0x82647e08`), so the round results'
    /// second board keeps its titles; its options are not written either.
    pub fn reset(&mut self) {
        self.current = Board::Final;
        self.second = Board::Final;
        self.current_result_type = result_group::FINAL_RESULTS;
        self.round_update_redraw = false;
        self.results_navigatable = false;
        self.timer_id = -1;
        self.online_is_results_for_last_round = false;
        for board in [&mut self.round, &mut self.final_results] {
            board.options = [option::CONTINUE_RETRY, option::RETRY_QUIT, option::UNKNOWN, option::UNKNOWN];
            for (column, title) in (1..).zip(board.col_titles.iter_mut()) {
                *title = format!("#COL{column}");
            }
        }
        for board in [&mut self.round, &mut self.round_second, &mut self.final_results] {
            board.num_players = 0;
            board.self_rank = 1;
            board.player_to_highlight = 0;
            board.results_type = PLACE_NAME_INT;
            board.players.iter_mut().for_each(|player| *player = ResultsPlayer::default());
            board.main_title = "ID_COMMON_UNKNOWN".to_owned();
        }
    }

    /// `SetActiveResultsLeaderboard(type, false)` (`0x82657960`): the final results make both
    /// pointers the final results; the round results make them the round results and its second
    /// board and set `mRoundUpdateRedraw`; any other type is only recorded.
    pub fn set_active_results_leaderboard(&mut self, result_type: i32) {
        match result_type {
            result_group::FINAL_RESULTS => {
                self.current = Board::Final;
                self.second = Board::Final;
                self.round_update_redraw = false;
            }
            result_group::ROUND_RESULTS => {
                self.current = Board::Round;
                self.second = Board::RoundSecond;
                self.round_update_redraw = true;
            }
            _ => {}
        }
        self.current_result_type = result_type;
    }
}

/// What `OptionSelect` does with an option, besides its button sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionAction {
    /// Continue and Quit: `PopState`, and `cMsgHUDContinue` once the outro is done
    /// (`OnOutroDone` `0x826590c8`).
    Leave,
    /// Retry and Rematch: `eSk8FEPopup_RematchConfirmation` (157); its yes runs
    /// `RestartMainChallenge(0, true)`, sends `cMsgHUDRetry` and pops the state.
    Rematch,
    /// View Photos and View Demo Photos: `PopState`, then HUD state 5.
    Photos,
    /// Online Scores: the terms-of-service flow message (79).
    OnlineScores,
    /// Highlight Reel: `cMsgHUDGotoHighlightReel` and `PopState`.
    HighlightReel,
    /// Next, Previous and Show Gamer Card: the results pages.
    Page(i32),
    /// Exit Party Play: `eSk8FEPopup_QuitGameConfirmationNoDataLoss` (153); its yes sends
    /// `cMsgHUDContinue`, pops the state and quits to the front end.
    ExitPartyPlay,
    /// Adjust Settings: the leaderboard is kept and FE state 39 is pushed.
    AdjustSettings,
    /// View Replay, Save Replay, Unknown and Max.
    Nothing,
}

/// `OptionSelect`: the option (the Continue_Retry and Retry_Quit pairs by the win state) and
/// whether it plays the button sound first.
#[must_use]
pub fn option_action(option: i32, win: bool) -> (OptionAction, bool) {
    let option = match option {
        option::CONTINUE_RETRY => i32::from(!win),
        option::RETRY_QUIT => {
            if win {
                option::RETRY
            } else {
                option::QUIT
            }
        }
        other => other,
    };
    match option {
        option::CONTINUE | option::QUIT => (OptionAction::Leave, true),
        option::RETRY | option::REMATCH => (OptionAction::Rematch, true),
        option::VIEW_PHOTOS | option::VIEW_DEMO_PHOTOS => (OptionAction::Photos, true),
        option::ONLINE_SCORES => (OptionAction::OnlineScores, true),
        option::HIGHLIGHT_REEL => (OptionAction::HighlightReel, true),
        option::NEXT | option::PREVIOUS | option::SHOW_GAMER_CARD => (OptionAction::Page(option), true),
        option::EXIT_PARTY_PLAY => (OptionAction::ExitPartyPlay, true),
        option::ADJUST_SETTINGS => (OptionAction::AdjustSettings, true),
        _ => (OptionAction::Nothing, false),
    }
}

/// `ReceiveInput`'s option slot for a key: A the first, B the second, Y the fourth and X the
/// third.
#[must_use]
pub const fn option_slot(keycode: i32) -> Option<usize> {
    match keycode {
        2 => Some(0),
        3 => Some(1),
        4 => Some(3),
        5 => Some(2),
        _ => None,
    }
}

/// `PlayCorrectButtonSound` (`0x82642298`): the `fe` sound of the key that chose.
#[must_use]
pub const fn button_sound(keycode: i32) -> Option<&'static str> {
    match keycode {
        2 => Some("FE_SOUND_CORE_A_BUTTON"),
        3 => Some("FE_SOUND_CORE_B_BUTTON"),
        4 => Some("FE_SOUND_CORE_Y_BUTTON"),
        5 => Some("FE_SOUND_CORE_X_BUTTON"),
        _ => None,
    }
}

/// `OnIntroDone` (`0x82658fe8`): the final results' success or failure sound.
#[must_use]
pub const fn intro_sound(win: bool) -> &'static str {
    if win { "FE_SOUND_CHALLENGE_END_SUCCESS" } else { "FE_SOUND_CHALLENGE_END_FAIL" }
}

/// Round hashes d8f50a95/735f373f in OnIntroDone82658fe8, resolved through
/// the original fe collections challenge_round_success/challenge_round_fail.
pub const fn round_intro_sound(win: bool) -> &'static str {
    if win { "FE_SOUND_CHALLENGE_ROUND_SUCCESS" } else { "FE_SOUND_CHALLENGE_ROUND_FAIL" }
}

/// SetActiveResultsLeaderboard82657960's optional final-results audio.
pub(crate) fn play_final_results_audio(world: &mut World) {
    let board = &world.results.final_results;
    let leader = board.players.iter().take(board.num_players.max(0) as usize)
        .position(|row| row.position == 1).unwrap_or(board.num_players.max(0) as usize);
    if board.results_type == GOOFY_VS_REGULAR_TEAM_RESULTS {
        let goofy = board.players.get(leader).is_some_and(|row| row.is_goofy);
        // Raw82657a18/24: cMsgPlayMissionSpeech9703d2a4, Cyrus36, 466/467.
        world.output(Output::PlayNpcSpeech { character: 36, id: 466 + i32::from(!goofy), param: 0 });
    } else {
        let character = world.players.as_deref().map_or(u64::MAX, |players| players.character(leader as i32));
        let character = if character as u32 == u32::MAX { 0 } else {
            world.db.layout_word(crate::ai::skater_manager::CHARACTERS_MARQUEE, character, 3) as i32
        };
        world.output(Output::MissionAudio(super::mission_audio::MissionAudio::LeaderboardOverall { character }));
    }
}

/// `FrontEndState_EndChallengeOptions` (FE state 8) as `OpenContinueRetryDialog` leaves it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndChallengeOptions {
    /// `mWinState`.
    pub win: bool,
    /// `mThrowDownType`.
    pub throwdown: i32,
    /// `mChallengeKey`: the calling challenge.
    pub challenge: u64,
    /// The screen `OnEnter` opens ([`screen`]).
    pub screen: i32,
}

fn truncated(text: &str, size: usize) -> String {
    let mut end = text.len().min(size - 1);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

fn player_name(world: &World, index: i32) -> String {
    let name = world.players.as_deref().map(|players| players.player_name(index)).unwrap_or_default();
    truncated(&format!("#{name}"), NAME_SIZE)
}

/// `FrontEnd.Results_SetNumPlayers(n)` (`0x82896e60`).
fn results_set_num_players(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.results.current_mut().num_players = get_int(call, 0);
    none()
}

/// `FrontEnd.Results_SetPlayerData(index, position, score = 0, floatScore = 0, bonus = 0)`
/// (`0x828970f8`): `SetPlayerData` (`0x82641a28`) of the current leaderboard; the row is named
/// `#` and the players back end's `GetPlayerName`, or in the Goofy vs Regular team layout the
/// regular team's HAL id (any index but 0) or the goofy team's (index 0).
fn results_set_player_data(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let index = get_int(call, 0);
    let teams = world.results.current().results_type == GOOFY_VS_REGULAR_TEAM_RESULTS;
    let name = (!teams).then(|| player_name(world, index));
    let Some(row) = world.results.current_mut().player_mut(index) else { return none() };
    row.position = get_int(call, 1);
    row.score = get_int_or(call, 2, 0);
    row.float_score = get_float_or(call, 3, 0.0);
    row.bonus = get_int_or(call, 4, 0);
    match name {
        Some(name) => row.name = name,
        None if index != 0 => {
            row.name = truncated(REGULAR_TEAM_HAL, 0x13);
            row.is_goofy = false;
        }
        None => {
            row.name = truncated(GOOFY_TEAM_HAL, 0x11);
            row.is_goofy = true;
        }
    }
    none()
}

/// `FrontEnd.Results_SetColTitle(column, title)` (`0x82896f30`).
fn results_set_col_title(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let title = get_string(call, 1).unwrap_or_default();
    let column = get_int(call, 0);
    if let Some(slot) = usize::try_from(column).ok().and_then(|i| world.results.current_mut().col_titles.get_mut(i)) {
        *slot = title;
    }
    none()
}

/// `FrontEnd.Results_SetResultsMenuType(type)` (`0x82896fb8`): `SetResultsMenuType`
/// (`0x82641e90`).
fn results_set_results_menu_type(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.results.current_mut().results_type = get_int(call, 0);
    none()
}

/// `FrontEnd.Results_SetActiveResultsMenu(type)` (`0x82897088`).
fn results_set_active_results_menu(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.results.set_active_results_leaderboard(get_int_or(call, 0, 0));
    none()
}

/// `FrontEnd.Results_SetMainTitle(title)` (`0x82896ec8`).
fn results_set_main_title(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.results.current_mut().main_title = get_string(call, 0).unwrap_or_default();
    none()
}

/// `FrontEnd.Results_SetPlayerStatus(index, status)` (`0x82897230`).
fn results_set_player_status(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let status = get_string(call, 1).unwrap_or_default();
    if let Some(row) = world.results.current_mut().player_mut(get_int(call, 0)) {
        row.status = status;
    }
    none()
}

/// `FrontEnd.Results_SetPlayerTime(index, time)` (`0x828972b8`).
fn results_set_player_time(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let time = get_float(call, 1);
    if let Some(row) = world.results.current_mut().player_mut(get_int(call, 0)) {
        row.time = time;
    }
    none()
}

/// `FrontEnd.Results_SetSecondLBNumPlayers(n)` (`0x82897350`).
fn results_set_second_lb_num_players(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.results.second_mut().num_players = get_int(call, 0);
    none()
}

/// `FrontEnd.Results_SetSecondLBPlayerData(backendIndex, index, position, score = 0,
/// floatScore = 0)` (`0x82897440`): `SetPlayerDataSecondLB` (`0x82641b18`), the row named `#`
/// and the back end player's name.
fn results_set_second_lb_player_data(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let name = player_name(world, get_int(call, 0));
    let Some(row) = world.results.second_mut().player_mut(get_int(call, 1)) else { return none() };
    row.position = get_int(call, 2);
    row.score = get_int_or(call, 3, 0);
    row.float_score = get_float_or(call, 4, 0.0);
    row.name = name;
    none()
}

/// `FrontEnd.Results_SetSecondLBColTitle(column, title)` (`0x828973b8`): `SetColTitleSecondLB`
/// (`0x82641d28`).
fn results_set_second_lb_col_title(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let title = get_string(call, 1).unwrap_or_default();
    let column = get_int(call, 0);
    if let Some(slot) = usize::try_from(column).ok().and_then(|i| world.results.second_mut().col_titles.get_mut(i)) {
        *slot = title;
    }
    none()
}

/// `FrontEnd.SetEndChallengeOption(index, option)` (`0x82897630`): `SetOption`
/// (`0x82641e00`) of the first four.
fn set_end_challenge_option(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let option = get_int(call, 1);
    let index = get_int(call, 0);
    if let Some(slot) = usize::try_from(index).ok().and_then(|i| world.results.current_mut().options.get_mut(i)) {
        *slot = option;
    }
    none()
}

/// `FrontEnd.OpenContinueRetryDialog(win, oneUp, skate)` (`0x82896a58`): FE state 8's win
/// state, its throwdown type (one-up before S.K.A.T.E.) and the calling challenge, then
/// `PushState(8)`.
fn open_continue_retry_dialog(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let win = get_bool_or(call, 0, false);
    let throwdown = if get_bool_or(call, 1, false) {
        throwdown::ONE_UP
    } else if get_bool_or(call, 2, false) {
        throwdown::SKATE
    } else {
        throwdown::NONE
    };
    let challenge = world.closure.map_or(0, |activity| world.activities.activities[activity].key);
    let screen = end_screen(world, win, throwdown);
    world.output(Output::OpenEndChallengeOptions(EndChallengeOptions { win, throwdown, challenge, screen }));
    none()
}

/// `FrontEnd.OpenOnlineContinueRetryDialog(win, oneUp, skate)` (`0x82896b40`): FE state 27's
/// `mResultsNavigatable`; FE state 8's win state, throwdown type, the calling challenge and
/// `mIsOnlineDisplay` (its `OnEnter` then opens the online screen 117); `PopToState(0)` and
/// `PushState(8)`. Scripts call it only in network games.
fn open_online_continue_retry_dialog(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let win = get_bool_or(call, 0, false);
    let throwdown = if get_bool_or(call, 1, false) {
        throwdown::ONE_UP
    } else if get_bool_or(call, 2, false) {
        throwdown::SKATE
    } else {
        throwdown::NONE
    };
    world.results.results_navigatable = true;
    let challenge = world.closure.map_or(0, |activity| world.activities.activities[activity].key);
    world.output(Output::OpenEndChallengeOptions(EndChallengeOptions { win, throwdown, challenge, screen: screen::ONLINE }));
    none()
}

/// `FrontEnd.CloseOnlineContinueRetryDialog()` (`0x82896c58`): `PopState` of the online dialog.
fn close_online_continue_retry_dialog(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.output(Output::CloseOnlineContinueRetryDialog);
    none()
}

/// `FrontEnd.Results_SetOnlineIsResultsForLastRound(last = false)` (`0x828971c8`).
fn results_set_online_is_results_for_last_round(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.results.online_is_results_for_last_round = get_bool_or(call, 0, false);
    none()
}

/// `FrontEnd.Results_SetOnlineTimerID(timer)` (`0x828976b8`): FE state 27's `mTimerID`.
fn results_set_online_timer_id(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.results.timer_id = get_int(call, 0);
    none()
}

/// FE state 8 as `FrontEndState_PhotoChallenge::PhotoDone` (`0x82660320`) enters it: offline,
/// with the win state `OpenPhotoSelection` set, no throwdown (each `OnLeave` resets it) and the
/// main challenge.
#[must_use]
pub fn photo_end_options(world: &World, win: bool) -> EndChallengeOptions {
    let challenge = world.activities.main.map_or(0, |main| world.activities.activities[main].key);
    EndChallengeOptions { win, throwdown: throwdown::NONE, challenge, screen: end_screen(world, win, throwdown::NONE) }
}

/// `FrontEndState_EndChallengeOptions::OnEnter` (`0x82658880`)'s screen offline (online it opens
/// 117): a throwdown's results; a win by the current leaderboard's player count, the GVR layout
/// for a Best Trick team contest of several players; a loss by the same count.
fn end_screen(world: &World, win: bool, throwdown: i32) -> i32 {
    if throwdown != throwdown::NONE {
        return screen::THROWDOWN;
    }
    let several = world.results.current().num_players > 1;
    if !win {
        return if several { screen::FAILED } else { screen::FAILED_SMALL };
    }
    let teams = world.activities.main.is_some_and(|main| {
        signup::challenge_type(world, main) == BEST_TRICK
            && world
                .db
                .attribute(class::CHALLENGES, world.activities.activities[main].key, CONTEST_TEAMS)
                .and_then(|a| a.bool(0))
                .unwrap_or(false)
    });
    match (teams && several, several) {
        (true, _) => screen::RESULTS_GVR,
        (false, true) => screen::RESULTS,
        (false, false) => screen::RESULTS_SMALL,
    }
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("FrontEnd", "Results_SetNumPlayers", results_set_num_players),
        ("FrontEnd", "Results_SetPlayerData", results_set_player_data),
        ("FrontEnd", "Results_SetColTitle", results_set_col_title),
        ("FrontEnd", "Results_SetResultsMenuType", results_set_results_menu_type),
        ("FrontEnd", "Results_SetActiveResultsMenu", results_set_active_results_menu),
        ("FrontEnd", "Results_SetMainTitle", results_set_main_title),
        ("FrontEnd", "Results_SetPlayerStatus", results_set_player_status),
        ("FrontEnd", "Results_SetPlayerTime", results_set_player_time),
        ("FrontEnd", "Results_SetSecondLBNumPlayers", results_set_second_lb_num_players),
        ("FrontEnd", "Results_SetSecondLBPlayerData", results_set_second_lb_player_data),
        ("FrontEnd", "Results_SetSecondLBColTitle", results_set_second_lb_col_title),
        ("FrontEnd", "SetEndChallengeOption", set_end_challenge_option),
        ("FrontEnd", "OpenContinueRetryDialog", open_continue_retry_dialog),
        ("FrontEnd", "OpenOnlineContinueRetryDialog", open_online_continue_retry_dialog),
        ("FrontEnd", "CloseOnlineContinueRetryDialog", close_online_continue_retry_dialog),
        ("FrontEnd", "Results_SetOnlineIsResultsForLastRound", results_set_online_is_results_for_last_round),
        ("FrontEnd", "Results_SetOnlineTimerID", results_set_online_timer_id),
    ]
}

#[cfg(test)]
mod tests {
    use super::{Board, LeaderboardState, ResultsPlayer, option, result_group};

    #[test]
    fn reset_leaves_the_round_results_second_titles() {
        let mut state = LeaderboardState::default();
        assert_eq!(state.round.col_titles[0], "#COL1");
        assert_eq!(state.final_results.col_titles[8], "#COL9");
        assert_eq!(state.round_second.col_titles[0], "");
        state.round_second.col_titles[0] = "ID_X".to_owned();
        state.round_second.options[0] = option::NEXT;
        state.final_results.players[3].name = "#A".to_owned();
        state.reset();
        assert_eq!(state.round_second.col_titles[0], "ID_X");
        assert_eq!(state.round_second.options[0], option::NEXT);
        assert_eq!(state.final_results.players[3], ResultsPlayer::default());
        assert_eq!(state.round.main_title, "ID_COMMON_UNKNOWN");
    }

    #[test]
    fn active_results_switch_both_pointers() {
        let mut state = LeaderboardState::default();
        state.set_active_results_leaderboard(result_group::ROUND_RESULTS);
        assert_eq!((state.current, state.second, state.round_update_redraw), (Board::Round, Board::RoundSecond, true));
        state.current_mut().num_players = 3;
        state.set_active_results_leaderboard(result_group::FINAL_RESULTS);
        assert_eq!((state.current, state.second, state.round_update_redraw), (Board::Final, Board::Final, false));
        assert_eq!((state.round.num_players, state.current().num_players), (3, 0));
    }
}
