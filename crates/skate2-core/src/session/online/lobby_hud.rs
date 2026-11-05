//! The lobby skate HUD: `FrontEndManager::OpenOnlineLobbyHuds` (0x82634ee0) creates the
//! "lobbyhud" group (`leaderboard` and `timer`) when `StateOnlineGame::LobbySkate` (0x82777820)
//! enters, and `CloseOnlineLobbyHuds` (0x826065a8) deletes it when it exits. `SetupLobbyHUD`
//! (0x82762858) fills the leaderboard on entering and as players join and leave (`OnEvent`
//! 0x8276ac88): `eLeaderBoardHUD_RankName`, each player's index as the rank, the local player
//! highlighted, every player connected, the variant line, then the group shown. The leaderboard
//! component runs as `HUDManager2` runs a challenge group's (`LeaderboardHUD::Show` 0x826bdcc0,
//! `Refresh` 0x826bdd58, `Update` 0x826bddc0), and its screen states go to the same movie. The
//! timer shows only in the online timer's states (`OnlineTimer::UpdateHUD` 0x8263a2b0).

use crate::challenge::leaderboard_hud::LeaderboardHud;
use crate::challenge::turn_hud::TurnHudView;

/// `eLeaderBoardHUD_RankName`.
const RANK_NAME: i32 = 11;
/// `LeaderboardHUD::Update` refreshes at most every 170 ms.
const UPDATE_MS: u32 = 170;

/// One player of `SetupLobbyHUD`, in `PlayersBackEnd` order.
pub(super) struct LobbyRow {
    pub(super) name: String,
    pub(super) local: bool,
}

/// `SetupLobbyHUD`'s variant line: with the game in progress during online free skate,
/// `ID_ONLINE_OTHERSKATERS_INACTIVITY` in an online free skate game and
/// `ID_ONLINE_INVITE_PLAYERSINCHALLENGE` in a challenge's; in a ranked game the starting line
/// once two are connected, else the waiting one; otherwise none.
pub(super) const fn variant(in_progress: bool, freeskate: bool, ranked: bool, connected: usize) -> &'static str {
    if in_progress {
        if freeskate { "ID_ONLINE_OTHERSKATERS_INACTIVITY" } else { "ID_ONLINE_INVITE_PLAYERSINCHALLENGE" }
    } else if !ranked {
        "#Unknown"
    } else if connected > 1 {
        "ID_ONLINE_CHALLENGE_STARTING_IN"
    } else {
        "ID_ONLINE_WAITING_FOR_SKATERS"
    }
}

/// The group's leaderboard and its component.
#[derive(Default)]
pub(super) struct LobbyHud {
    /// The group's `LeaderboardHUDData` while "lobbyhud" exists.
    data: Option<LeaderboardHud>,
    /// `HUDGroup::Show(1)`: the group's instances are shown.
    shown: bool,
    /// The component's `mVisible`.
    visible: bool,
    time_since_update: u32,
    /// `PlayersBackEnd::GetPlayerName` of each row.
    names: Vec<String>,
    stance_regular: bool,
    screen_states: Vec<(i32, TurnHudView)>,
}

impl LobbyHud {
    pub(super) const fn is_open(&self) -> bool {
        self.data.is_some()
    }

    /// `OpenOnlineLobbyHuds`: the group, its instances not yet shown.
    pub(super) fn open(&mut self) {
        if self.data.is_none() {
            self.data = Some(LeaderboardHud::default());
            self.shown = false;
        }
    }

    /// `CloseOnlineLobbyHuds`: `DeleteGroup` unbinds the component, which hides.
    pub(super) fn close(&mut self) {
        if self.data.take().is_some() {
            self.shown = false;
            self.show(false);
        }
    }

    /// `SetupLobbyHUD` on an open group.
    pub(super) fn setup(&mut self, rows: &[LobbyRow], variant: &str) {
        let Some(data) = self.data.as_mut() else { return };
        data.menu_type = RANK_NAME;
        data.dirty = true;
        data.set_num_players(i32::try_from(rows.len()).unwrap_or(0));
        for (player, row) in data.players.iter_mut().zip(rows) {
            player.highlighted = row.local;
            player.int_var = 1;
        }
        for (rank, player) in (1..).zip(data.players.iter_mut().take(rows.len())) {
            player.rank = rank;
        }
        data.non_order_data_dirty = true;
        data.set_contest_variant_name_string(variant);
        self.names = rows.iter().map(|row| row.name.clone()).collect();
        self.shown = true;
    }

    /// `HUDManager2::Update` for the component, `dt` the front end's update time: it shows with
    /// its group, then `LeaderboardHUD::Update` refreshes dirty data at most every 170 ms.
    pub(super) fn update(&mut self, dt: u32, stance_regular: bool) {
        self.stance_regular = stance_regular;
        let shown = self.data.is_some() && self.shown;
        if shown != self.visible {
            self.show(shown);
        }
        if !self.visible {
            return;
        }
        if self.time_since_update < UPDATE_MS {
            self.time_since_update += dt;
            return;
        }
        self.time_since_update = 0;
        let Some(data) = self.data.as_mut() else { return };
        if data.dirty {
            self.refresh();
            if let Some(data) = self.data.as_mut() {
                data.dirty = false;
                data.non_order_data_dirty = false;
            }
        } else if data.non_order_data_dirty {
            data.non_order_data_dirty = false;
            self.state(5);
        }
    }

    /// `LeaderboardHUD::Show`: a refresh when shown, then states 1 and 3, or 2.
    fn show(&mut self, show: bool) {
        if show {
            self.refresh();
        }
        if self.visible != show {
            if show {
                self.state(1);
                self.state(3);
            } else {
                self.state(2);
            }
        }
        self.visible = show;
    }

    /// `LeaderboardHUD::Refresh`: the players' names, then states 3 and 4.
    fn refresh(&mut self) {
        let names = &self.names;
        if let Some(data) = self.data.as_mut() {
            data.refresh_player_names(|player| usize::try_from(player).ok().and_then(|player| names.get(player)).cloned().unwrap_or_default());
        }
        self.state(3);
        self.state(4);
    }

    fn state(&mut self, state: i32) {
        let view = self.data.clone().map_or(TurnHudView::None, |data| TurnHudView::Leaderboard { data, stance_regular: self.stance_regular, online: true, skate3_letters: false });
        self.screen_states.push((state, view));
    }

    /// The screen states the leaderboard movie has not run.
    pub(super) fn take_screen_states(&mut self) -> Vec<(i32, TurnHudView)> {
        std::mem::take(&mut self.screen_states)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<LobbyRow> {
        vec![LobbyRow { name: "Alpha".into(), local: false }, LobbyRow { name: "Bravo".into(), local: true }]
    }

    #[test]
    fn the_board_shows_each_player_by_index_with_the_local_one_highlighted() {
        let mut hud = LobbyHud::default();
        hud.open();
        hud.setup(&rows(), variant(false, false, false, 2));
        hud.update(0, true);
        let states: Vec<i32> = hud.screen_states.iter().map(|(state, _)| *state).collect();
        assert_eq!(states, [3, 4, 1, 3]);
        let Some((_, TurnHudView::Leaderboard { data, .. })) = hud.screen_states.last() else { panic!("leaderboard view") };
        assert_eq!(data.menu_type, RANK_NAME);
        assert_eq!(data.num_players, 2);
        assert_eq!([data.players[0].rank, data.players[1].rank], [1, 2]);
        assert_eq!([data.players[0].name.as_str(), data.players[1].name.as_str()], ["#Alpha", "#Bravo"]);
        assert!(!data.players[0].highlighted && data.players[1].highlighted);
        assert_eq!(data.contest_variant_name, "#Unknown");
    }

    #[test]
    fn closing_hides_the_board_and_only_a_ranked_or_running_game_has_a_variant() {
        let mut hud = LobbyHud::default();
        hud.open();
        hud.setup(&rows(), "#Unknown");
        hud.update(0, true);
        hud.take_screen_states();
        hud.close();
        assert_eq!(hud.take_screen_states().iter().map(|(state, _)| *state).collect::<Vec<_>>(), [2]);
        assert_eq!(variant(true, false, false, 2), "ID_ONLINE_INVITE_PLAYERSINCHALLENGE");
        assert_eq!(variant(true, true, false, 2), "ID_ONLINE_OTHERSKATERS_INACTIVITY");
        assert_eq!(variant(false, false, true, 1), "ID_ONLINE_WAITING_FOR_SKATERS");
        assert_eq!(variant(false, false, true, 2), "ID_ONLINE_CHALLENGE_STARTING_IN");
    }
}
