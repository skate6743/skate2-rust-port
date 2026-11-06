//! Original hud2/leaderboardhud (asset 57, fingerprint c381485c; template entry
//! `LeaderboardHUD` 0x82fdb848, placement 6, g3dHUDRight) and its `leaderboardhud` class
//! (InitAction character 133, `0x70e8`), driven by `LeaderboardHUD`
//! (`skate2_core::challenge::turn_hud`): the S.K.A.T.E. and one-up letter board
//! (`mcThrowdownHUD`) and the ranked leaderboard (`mcLeaderboardHUD`). `SetScreenState` runs
//! the class's `SetState` at once, so each state is applied with the data it carries, which the
//! `HUDComponents.Leaderboard_*` getters read. In a network game (`Mission.GetIsOnlineMission`)
//! each ranked row is `kTURNIND_VOIPICON_SIZE` wider and a player's new row plays its `tintBox`
//! to the player's index, the box of the player's colour; offline the box stays on its empty
//! `grey` frame and the voice icons hide.
//! ActionScript ignores sets and calls on absent members, so lookups of rows the movie does
//! not author do nothing.
//!
//! PC port: a linked Skate 3 data folder's 1-Ups show Skate 3's own letters, as its
//! `leaderboardhud` does for its `eLeaderBoardHUD_1up` layout: each row's `mUp` (character 163
//! of `Assets::skate3_hud`'s leaderboard, labels `0` to `3`) in place of its `mSkate` (character
//! 154), here placed and sized as Skate 3 places it over the S.K.A.T.E. strip, over this
//! board's `mcSkate` rows.
use std::collections::BTreeMap;

use super::{
    assets::Assets,
    clips::{Child, Clip},
    localizer,
    objective_movie::{set_x, set_xscale, set_y, text_field_width},
    pause_movie::{bounds, child},
    tutorial_movie::set_height,
};
use crate::gpu_renderer::RenderResult;
use skate2_core::{
    challenge::{leaderboard_hud::LeaderboardHud, system::MissionTitle, turn_hud::TurnHudView},
    ui::apt_timeline::concatenate,
};
use skate2_formats::{
    apt::{Character, Matrix},
    localization::string_hash,
};

pub(super) const ASSET: usize = 57;

/// Skate 3's `leaderboardhud` rows' `mSkate` and `mUp` characters.
const SKATE3_SKATE_LETTERS: u32 = 154;
const SKATE3_ONE_UP_LETTERS: u32 = 163;
/// The depth of the letter board's first Skate 3 `mUp` row, past all it authors.
const SKATE3_LETTERS_DEPTH: i32 = 0x4000;
/// The space between a row's name panel and its Skate 3 1-Up letters, in board units.
const SKATE3_LETTERS_GAP: f32 = 4.0;

/// The `eLeaderBoardHUD` layouts the class tests.
const LAYOUT_ONE_UP: i32 = 1;
const LAYOUT_INT_STRING: i32 = 2;
const LAYOUT_SKATE: i32 = 7;
const LAYOUT_GVR: i32 = 8;
const LAYOUT_RANK_NAME: i32 = 11;
/// `kNORM`, `kHIGHLIGHT` and `kRED`.
const NORM: i32 = 0;
const HIGHLIGHT: i32 = 1;
const RED: i32 = 2;
/// The class's `textColor`s, which a set leaves opaque.
const FONT_COLOR_NORMAL: u32 = 0xffff_feec;
const FONT_RANK_COLOR_NORMAL: u32 = 0xffff_fcca;
const FONT_COLOR_LOGGEDOFF: u32 = 0xff00_0000;
const FONT_COLOR_HIGHLIGHTED: u32 = 0xffff_ff33;
const THROWDOWN_HIGHLIGHT_COLOR: u32 = 0xffff_ff99;
const THROWDOWN_NORMAL_COLOR: u32 = 0xff42_2104;
/// `kSKATERIGHTOFFSET`.
const SKATE_RIGHT_OFFSET: f32 = 105.;
/// The rows each board authors.
const ROWS: i32 = 8;
/// `kTURNIND_VOIPICON_SIZE` in a network game.
const VOIP_ICON_SIZE: f32 = 30.;
/// `eONLINE_VOIP_DISABLED`, the `FELeaderboard.GetVoipStatus` (`0x82605408`) of every player:
/// the port has no voice chat.
const VOIP_DISABLED: i32 = 0;

pub(super) struct LeaderboardMovie {
    pub root: Clip,
    num_players: i32,
    last_num_players: i32,
    last_layout_type: i32,
    layout_type: i32,
    was_hidden: bool,
    /// `mLastPlayerTurn`, `mLastPlayerPosition` and `mLastColor`: eight entries, and any index
    /// a refresh writes past them.
    last_player_turn: BTreeMap<i32, bool>,
    last_player_position: BTreeMap<i32, i32>,
    last_color: BTreeMap<i32, i32>,
    /// `mLastPlayerVoip`, by row.
    last_player_voip: BTreeMap<i32, i32>,
    /// `mIsOnlineMission` and the `kTURNIND_VOIPICON_SIZE` it sets.
    online: bool,
    voip_icon_size: f32,
    use_variant: bool,
    switch_goofy_regular: bool,
    /// `mNameArray`.
    names: Vec<String>,
    /// Whether the rows show Skate 3's letters, and the transform from Skate 3's letter strips
    /// into this board's `mcSkate` rows once made.
    skate3_letters: bool,
    skate3_letters_transform: Option<Matrix>,
    /// The authored placements [`Self::condense_for_skate3_letters`] changed, by piece.
    condensed: BTreeMap<String, Matrix>,
}

impl LeaderboardMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut movie = Self {
            root: Clip::new(assets, ASSET, 0)?,
            num_players: 0,
            last_num_players: 0,
            last_layout_type: -1,
            layout_type: 0,
            was_hidden: true,
            last_player_turn: BTreeMap::new(),
            last_player_position: BTreeMap::new(),
            last_color: BTreeMap::new(),
            last_player_voip: BTreeMap::new(),
            online: false,
            voip_icon_size: 0.,
            use_variant: false,
            switch_goofy_regular: false,
            names: Vec::new(),
            skate3_letters: false,
            skate3_letters_transform: None,
            condensed: BTreeMap::new(),
        };
        movie.calls()?;
        Ok(movie)
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root.tick(assets)?;
        self.calls()
    }

    fn calls(&mut self) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "leaderboardhud" => self.construct()?,
                _ => return Err(format!("untranslated leaderboard HUD callback {call}").into()),
            }
        }
        Ok(())
    }

    /// Root action `0x70a4`'s `new leaderboardhud(this)`; the root then plays its intro to frame
    /// 10. No players, the last layout -1, both boards hidden and `Show(false)`. The
    /// `mBackground` and `mLeaderboardBack` it hides on the screen are not root members, and
    /// `mScreen._x -= Screen_EdgeOffset` subtracts zero.
    fn construct(&mut self) -> RenderResult<()> {
        self.num_players = 0;
        self.last_num_players = 0;
        self.last_layout_type = -1;
        self.layout_type = 0;
        self.last_player_turn = (0..ROWS).map(|i| (i, false)).collect();
        self.last_player_position = (0..ROWS).map(|i| (i, -1)).collect();
        self.last_color = (0..ROWS).map(|i| (i, -1)).collect();
        self.last_player_voip = (0..ROWS).map(|i| (i, -1)).collect();
        self.root.named(&["mcThrowdownHUD"])?.visible = false;
        self.root.named(&["mcLeaderboardHUD"])?.visible = false;
        self.use_variant = false;
        self.switch_goofy_regular = false;
        self.show(false);
        Ok(())
    }

    /// `SetState`: 1 and 2 show and hide the screen, 3 sets it up, 4 and 5 refresh it and 6
    /// updates the voice icons.
    pub fn set_state(&mut self, assets: &Assets, state: i32, view: &TurnHudView, title: &MissionTitle) -> RenderResult<()> {
        let data = || match view {
            TurnHudView::Leaderboard { data, stance_regular, online, skate3_letters } => {
                Ok((data, *stance_regular, *online, *skate3_letters && assets.skate3_hud.is_some()))
            }
            _ => Err(format!("leaderboard HUD state {state} without leaderboard data")),
        };
        match state {
            1 => self.show(true),
            2 => self.show(false),
            3 => {
                let (data, regular, online, skate3_letters) = data()?;
                self.skate3_letters = skate3_letters;
                self.setup_screen(assets, data, regular, online, title)?;
            }
            4 | 5 => {
                let (data, regular, _, skate3_letters) = data()?;
                self.skate3_letters = skate3_letters;
                self.refresh_all(assets, data, regular)?;
            }
            6 => self.update_voip_icons(),
            _ => {}
        }
        Ok(())
    }

    /// `Show(bVisible)`: hiding marks the screen as hidden since its last refresh.
    fn show(&mut self, visible: bool) {
        self.root.visible = visible;
        if !visible {
            self.was_hidden = true;
        }
    }

    fn throwdown(&self) -> bool {
        matches!(self.layout_type, LAYOUT_ONE_UP | LAYOUT_SKATE)
    }

    /// `SetupScreen`: whether the game is a network game and so the voice icons' room, the
    /// names, count and layout (`Leaderboard_GetLeaderboardNameBlock` `0x826b93a8`,
    /// `Leaderboard_GetNumPlayers` `0x826b8e70`, `Leaderboard_GetLayoutType` `0x826b8ed0`), the
    /// board for the layout, its variant line, title and rows, then the background sized and
    /// placed for the widest of them.
    fn setup_screen(&mut self, assets: &Assets, data: &LeaderboardHud, regular: bool, online: bool, title: &MissionTitle) -> RenderResult<()> {
        self.restore_condensed()?;
        self.online = online;
        self.voip_icon_size = if online { VOIP_ICON_SIZE } else { 0. };
        self.num_players = data.num_players;
        self.names = data.players.iter().take(usize::try_from(data.num_players).unwrap_or(0)).map(|p| p.name.clone()).collect();
        self.layout_type = data.menu_type;
        let throwdown = self.throwdown();
        self.root.named(&["mcThrowdownHUD"])?.visible = throwdown;
        self.root.named(&["mcLeaderboardHUD"])?.visible = !throwdown;
        let variant = self.setup_contest_variant(assets, data, throwdown)?;
        let title = self.setup_title(assets, title, throwdown)?;
        self.refresh_visibility_of_players_list(throwdown)?;
        let rows = self.refresh_all(assets, data, regular)?;
        self.last_layout_type = self.layout_type;
        let widest = as_max(as_max(variant, title), rows);
        self.update_voip_icons();
        self.set_screen_size(assets, throwdown, widest)?;
        self.position_hud(assets, throwdown, widest)?;
        if throwdown {
            self.condense_for_skate3_letters(assets, widest, title)?;
        }
        Ok(())
    }

    /// PC port: Skate 3's `SizePlayerRow` sizes its row around the 1-Up strip, three letters to
    /// the S.K.A.T.E. strip's five. This board's backs come in fixed widths, so with Skate 3's
    /// letters the shown back narrows (its left edge kept, the title centred on it again) until
    /// it ends as far past the 1-Up strip as the strip starts past the name panel, the even
    /// spacing the S.K.A.T.E. letters keep; never below the title, which `SetScreenSize` keeps
    /// 4 narrower than its back. The board moves right as far, its right edge where the back's
    /// was. Each layout starts from the authored placements.
    fn condense_for_skate3_letters(&mut self, assets: &Assets, widest: f32, title: f32) -> RenderResult<()> {
        if !self.shows_skate3_letters() {
            return Ok(());
        }
        let Some(hud) = assets.skate3_hud else { return Ok(()) };
        let board = self.root.named(&["mcThrowdownHUD"])?;
        let Some(up) = board.children.get(&SKATE3_LETTERS_DEPTH).map(|up| up.entry.matrix) else { return Ok(()) };
        let Some(strip) = find_child(board, "mcSkateStrip0") else { return Ok(()) };
        let Some(panel) = bounds(&strip.clip, assets)? else { return Ok(()) };
        let s = strip.entry.matrix;
        let panel_right = panel.left.mul_add(s.a, s.tx).max(panel.right.mul_add(s.a, s.tx));
        let letters = bounds(&Clip::new(assets, hud.leaderboard, SKATE3_ONE_UP_LETTERS)?, assets)?.ok_or("Skate 3's 1-Up letters without bounds")?;
        let (up_left, up_right) = (letters.left.mul_add(up.a, up.tx), letters.right.mul_add(up.a, up.tx));
        let bg = find_child(board, "mBackground").ok_or("letter board without mBackground")?.entry.matrix;
        let back_name = size(self.root.named(&["mcThrowdownHUD", "mBackground"])?, assets, widest)?.1;
        let back_path = format!("back:{back_name}");
        let (r, m) = {
            let piece = condensed_piece(&mut self.root, &back_path)?;
            (bounds(&piece.clip, assets)?.ok_or("letter board back without bounds")?, piece.entry.matrix)
        };
        let local_left = r.left.mul_add(m.a, m.tx);
        let left = local_left.mul_add(bg.a, bg.tx);
        let right = r.right.mul_add(m.a, m.tx).mul_add(bg.a, bg.tx);
        let target = (up_right + (up_left - panel_right).max(0.)).max((title + 4.).mul_add(bg.a, left));
        let unused = right - target;
        if !(unused > 0.) || !(r.right > r.left) {
            return Ok(());
        }
        for path in ["board", "title", back_path.as_str()] {
            let matrix = condensed_piece(&mut self.root, path)?.entry.matrix;
            self.condensed.insert(path.to_owned(), matrix);
        }
        let a = (target - left) / bg.a / (r.right - r.left);
        let back = condensed_piece(&mut self.root, &back_path)?;
        back.clip.script_transform = true;
        back.entry.matrix.a = a;
        back.entry.matrix.tx = r.left.mul_add(-a, local_left);
        let title = condensed_piece(&mut self.root, "title")?;
        title.clip.script_transform = true;
        title.entry.matrix.tx -= unused / bg.a * 0.5;
        let board = condensed_piece(&mut self.root, "board")?;
        board.entry.matrix.tx = unused.mul_add(board.entry.matrix.a, board.entry.matrix.tx);
        Ok(())
    }

    /// The authored placements back, before the background is sized again from them.
    fn restore_condensed(&mut self) -> RenderResult<()> {
        for (path, matrix) in std::mem::take(&mut self.condensed) {
            let piece = condensed_piece(&mut self.root, &path)?;
            piece.entry.matrix = matrix;
            if path != "board" {
                piece.clip.script_transform = false;
            }
        }
        Ok(())
    }

    /// `SetupContestVariant`: a named variant (not `#Unknown`, `ID_COMMON_UNKNOWN` or its
    /// localized text) on the ranked board shows centre-sized in `mContestVariant`; otherwise
    /// it hides. Answers the label's `_width`; the backgrounds author no `mVariant`.
    fn setup_contest_variant(&mut self, assets: &Assets, data: &LeaderboardHud, throwdown: bool) -> RenderResult<f32> {
        let name = &data.contest_variant_name;
        let unknown = format!("#{}", String::from_utf16_lossy(&localizer::format_hal(assets, string_hash(b"ID_COMMON_UNKNOWN"))?));
        self.use_variant = name != "#Unknown" && name != "ID_COMMON_UNKNOWN" && *name != unknown && !throwdown;
        let variant = self.root.named(&[board(throwdown), "mBackground", "mContestVariant"])?;
        for field in ["label", "labelGlow"] {
            let label = variant.named(&[field])?;
            label.visible = self.use_variant;
            if self.use_variant {
                auto_size(label, assets, "center")?;
                label.text = Some(assets.initial_text(name));
            }
        }
        width(child(variant, "label")?, assets)
    }

    /// `SetupTitle`: `Mission.GetTitle()` centre-sized in the background's `mcTitle`; answers
    /// the title's `_width`.
    fn setup_title(&mut self, assets: &Assets, title: &MissionTitle, throwdown: bool) -> RenderResult<f32> {
        let text = localizer::mission_title(assets, title)?;
        let holder = self.root.named(&[board(throwdown), "mBackground", "mcTitle"])?;
        for field in ["labelTitle", "labelGlow"] {
            let label = holder.named(&[field])?;
            auto_size(label, assets, "center")?;
            label.text = Some(text.clone());
        }
        width(child(holder, "labelTitle")?, assets)
    }

    /// `RefreshVisibilityOfPlayersList(isThrowdown, mNumPlayers)`: with players, the rows
    /// below the count show and the rest hide; the ranked board then ends on `mcStrip7`, moved
    /// up to the last row. Its outer loop shares the row counter, so it runs once.
    fn refresh_visibility_of_players_list(&mut self, throwdown: bool) -> RenderResult<()> {
        let count = self.num_players;
        if count <= 0 {
            return Ok(());
        }
        let (hud, names) = if throwdown {
            (self.root.named(&["mcThrowdownHUD"])?, &["mcStrip", "mcSkateStrip", "playerName", "mcSkate"][..])
        } else {
            (self.root.named(&["mcLeaderboardHUD"])?, &["mcStrip", "playerName"][..])
        };
        for i in 0..ROWS {
            for name in names {
                if let Some(row) = find(hud, &[&format!("{name}{i}")]) {
                    row.visible = i < count;
                }
            }
        }
        if throwdown {
            return self.refresh_visibility_of_skate3_letters();
        }
        if count < ROWS {
            hud.named(&["mcStrip7"])?.visible = true;
            let last = child(hud, &format!("mcStrip{}", count - 1))?;
            last.clip.visible = false;
            let y = last.entry.matrix.ty;
            set_y(child(hud, "mcStrip7")?, y);
        }
        Ok(())
    }

    /// `RefreshAll`: each player's row from `Leaderboard_GetLeaderboardDataBlock`
    /// (`0x826b8f10`: rank, int, score text, highlight, turn). The ranked board fills the row
    /// at the player's rank and, when the count or layout changed, plays the player's own row
    /// to `lobby` (rank and name layouts) or `regular`; the letter board fills the player's row,
    /// its letters, highlight and turn. Answers the widest row.
    fn refresh_all(&mut self, assets: &Assets, data: &LeaderboardHud, regular: bool) -> RenderResult<f32> {
        let throwdown = self.throwdown();
        let mut widest = 0.0_f32;
        for (i, player) in (0..self.num_players).zip(&data.players) {
            let name = self.names.get(usize::try_from(i).unwrap_or(usize::MAX)).cloned().unwrap_or_default();
            let row_width = if throwdown {
                let row = find(&mut self.root, &["mcThrowdownHUD", &format!("playerName{i}")]);
                let name_width = match row.and_then(|row| find(row, &["name_txt"])) {
                    Some(field) => {
                        auto_size(field, assets, "right")?;
                        field.text = Some(assets.initial_text(&name));
                        text_field_width(field, assets)?
                    }
                    None => f32::NAN,
                };
                let letters = ["0", "1", "2", "3", "4", "5"][usize::try_from(player.int_var.clamp(0, 5)).unwrap_or(0)];
                if self.shows_skate3_letters() {
                    self.set_skate3_letters(assets, i, letters)?;
                } else if let Some(skate) = find(&mut self.root, &["mcThrowdownHUD", &format!("mcSkate{i}")]) {
                    skate.goto(assets, letters)?;
                }
                self.update_highlight(assets, true, i, player.highlighted, player.int_var);
                self.update_player_turn(assets, i, player.is_turn)?;
                as_max(name_width + SKATE_RIGHT_OFFSET + 15., 247.)
            } else {
                self.refresh_ranked_row(assets, data, i, &name)?
            };
            widest = as_max(row_width, widest);
        }
        if self.layout_type == LAYOUT_GVR {
            widest = 240.;
            self.update_gvr_score_special_case(assets, regular);
        }
        if self.root.visible {
            self.was_hidden = false;
        }
        self.last_num_players = self.num_players;
        Ok(widest)
    }

    /// One ranked row of `RefreshAll`: `#rank`, the name right-sized, the score text (`#--`
    /// below zero), and the row's width with the voice icon's room; then the highlight and the
    /// player's last position (a new one plays that row's `tintBox` to the player's index in a
    /// network game).
    fn refresh_ranked_row(&mut self, assets: &Assets, data: &LeaderboardHud, i: i32, name: &str) -> RenderResult<f32> {
        let player = &data.players[usize::try_from(i).unwrap_or(0)];
        let rank = player.rank;
        let rank_text = assets.initial_text(&format!("#{rank}"));
        let score = if player.int_var < 0 { assets.initial_text("#--") } else { score_text(assets, data, i)? };
        let (mut name_width, mut rank_width) = (f32::NAN, f32::NAN);
        if let Some(row) = find(&mut self.root, &["mcLeaderboardHUD", &format!("playerName{}", rank - 1), "mLeaderboardData"]) {
            for field in ["rank_txt", "rankGlow_txt"] {
                row.named(&[field])?.text = Some(rank_text.clone());
            }
            let name_field = row.named(&["name_txt"])?;
            auto_size(name_field, assets, "right")?;
            name_field.text = Some(assets.initial_text(name));
            row.named(&["score_txt"])?.text = Some(score);
            name_width = width(child(row, "name_txt")?, assets)?;
            rank_width = width(child(row, "rank_txt")?, assets)?;
        }
        let mut row_width = name_width + 114. + self.voip_icon_size;
        if self.last_num_players != self.num_players || self.last_layout_type != self.layout_type {
            let lobby = matches!(self.layout_type, LAYOUT_RANK_NAME | LAYOUT_INT_STRING);
            if lobby {
                row_width = name_width + rank_width + 15. + self.voip_icon_size;
            }
            if let Some(score) = find(&mut self.root, &["mcLeaderboardHUD", &format!("playerName{}", rank - 1), "mLeaderboardData", "score_txt"]) {
                score.visible = !lobby;
            }
            if let Some(own) = find(&mut self.root, &["mcLeaderboardHUD", &format!("playerName{i}"), "mLeaderboardData"]) {
                own.goto(assets, if lobby { "lobby" } else { "regular" })?;
            }
        }
        self.update_highlight(assets, false, rank, player.highlighted, player.int_var);
        if self.last_player_position.get(&i) != Some(&rank) {
            if self.online
                && let Some(tint) = find(&mut self.root, &["mcLeaderboardHUD", &format!("playerName{}", rank - 1), "tintBox"])
            {
                tint.goto(assets, &i.to_string())?;
            }
            self.last_player_position.insert(i, rank);
        }
        Ok(row_width)
    }

    /// `UpdateHighlight(isThrowdown, position, Highlight, playerScore)`: on the letter board the
    /// row's name turns the highlight or normal colour when its colour changes; on the ranked
    /// board the leader's highlight decides `mSwitchGoofyRegular`, and a changed colour (or one
    /// after the screen was hidden) recolours the row: highlighted, normal, or cut (below zero)
    /// with `ID_CHALLENGE_CUT_PLAYER_SCORE`.
    fn update_highlight(&mut self, assets: &Assets, throwdown: bool, position: i32, highlight: bool, score: i32) {
        if throwdown {
            let colour = if highlight { HIGHLIGHT } else { NORM };
            if self.last_color.get(&position) != Some(&colour)
                && let Some(name) = find(&mut self.root, &["mcThrowdownHUD", &format!("playerName{position}"), "name_txt"])
            {
                set_text_colour(name, if highlight { THROWDOWN_HIGHLIGHT_COLOR } else { THROWDOWN_NORMAL_COLOR });
            }
            self.last_color.insert(position, colour);
            return;
        }
        let row = position - 1;
        if row == 0 {
            self.switch_goofy_regular = !highlight;
        }
        let colour = if highlight {
            HIGHLIGHT
        } else if score < 0 {
            RED
        } else {
            NORM
        };
        if (self.last_color.get(&row) != Some(&colour) || self.was_hidden)
            && let Some(data) = find(&mut self.root, &["mcLeaderboardHUD", &format!("playerName{row}"), "mLeaderboardData"])
        {
            let (text, rank) = match colour {
                HIGHLIGHT => (FONT_COLOR_HIGHLIGHTED, FONT_COLOR_HIGHLIGHTED),
                NORM => (FONT_COLOR_NORMAL, FONT_RANK_COLOR_NORMAL),
                _ => {
                    if let Some(field) = find(data, &["score_txt"]) {
                        field.text = Some(assets.initial_text("ID_CHALLENGE_CUT_PLAYER_SCORE"));
                    }
                    (FONT_COLOR_LOGGEDOFF, FONT_COLOR_LOGGEDOFF)
                }
            };
            for (field, colour) in [("name_txt", text), ("score_txt", text), ("rank_txt", rank)] {
                if let Some(field) = find(data, &[field]) {
                    set_text_colour(field, colour);
                }
            }
        }
        self.last_color.insert(row, colour);
    }

    /// Whether the letter board's rows show Skate 3's 1-Up letters.
    fn shows_skate3_letters(&self) -> bool {
        self.skate3_letters && self.layout_type == LAYOUT_ONE_UP
    }

    /// The row's Skate 3 `mUp` at `letters`, made over its `mcSkate` row when absent; Skate 3's
    /// `SetupScreen` hides the S.K.A.T.E. letters it replaces.
    fn set_skate3_letters(&mut self, assets: &Assets, row: i32, letters: &str) -> RenderResult<()> {
        let Some(hud) = assets.skate3_hud else { return Ok(()) };
        let transform = match self.skate3_letters_transform {
            Some(transform) => transform,
            None => {
                let transform = skate3_letters_transform(assets, hud.leaderboard, &mut self.root)?;
                self.skate3_letters_transform = Some(transform);
                transform
            }
        };
        let depth = SKATE3_LETTERS_DEPTH + row;
        let board = self.root.named(&["mcThrowdownHUD"])?;
        let made = board.children.contains_key(&depth);
        let Some(skate) = find_child(board, &format!("mcSkate{row}")) else { return Ok(()) };
        skate.clip.visible = false;
        if !made {
            let mut entry = skate.entry.clone();
            entry.matrix = concatenate(entry.matrix, transform);
            entry.name = Some(format!("mcUp{row}"));
            let clip = Clip::new(assets, hud.leaderboard, SKATE3_ONE_UP_LETTERS)?;
            board.children.insert(depth, Child { entry, clip });
        }
        let Some(up) = board.children.get_mut(&depth) else { return Ok(()) };
        up.clip.visible = true;
        up.clip.goto(assets, letters)
    }

    /// The Skate 3 letters' rows follow the count as the `mcSkate` rows do, which they keep
    /// hidden; a board without them removes any.
    fn refresh_visibility_of_skate3_letters(&mut self) -> RenderResult<()> {
        let skate3 = self.shows_skate3_letters();
        let count = self.num_players;
        let board = self.root.named(&["mcThrowdownHUD"])?;
        if !skate3 {
            board.children.retain(|&depth, _| depth < SKATE3_LETTERS_DEPTH);
            return Ok(());
        }
        for i in 0..ROWS {
            if let Some(skate) = find_child(board, &format!("mcSkate{i}")) {
                skate.clip.visible = false;
            }
            if let Some(up) = board.children.get_mut(&(SKATE3_LETTERS_DEPTH + i)) {
                up.clip.visible = i < count;
            }
        }
        Ok(())
    }

    /// `UpdatePlayerTurn(true, position, playerTurn)`, which only the letter board calls: a
    /// changed turn plays the strip's `mLeft.mTurn` to `turn` or `noturn`.
    fn update_player_turn(&mut self, assets: &Assets, position: i32, turn: bool) -> RenderResult<()> {
        if self.last_player_turn.get(&position) != Some(&turn)
            && let Some(indicator) = find(&mut self.root, &["mcThrowdownHUD", &format!("mcSkateStrip{position}"), "mLeft", "mTurn"])
        {
            indicator.goto(assets, if turn { "turn" } else { "noturn" })?;
        }
        self.last_player_turn.insert(position, turn);
        Ok(())
    }

    /// `UpdateGVRScoreSpecialCase`: the first two ranked rows name the teams, regular first
    /// for a regular local skater (`PlayerStance_IsRegular` `0x826b4bd8`) unless
    /// `mSwitchGoofyRegular` swaps them.
    fn update_gvr_score_special_case(&mut self, assets: &Assets, regular: bool) {
        let teams = if regular != self.switch_goofy_regular {
            ["ID_GVR_TEAM_REGULAR", "ID_GVR_TEAM_GOOFY"]
        } else {
            ["ID_GVR_TEAM_GOOFY", "ID_GVR_TEAM_REGULAR"]
        };
        for (row, team) in teams.into_iter().enumerate() {
            if let Some(field) = find(&mut self.root, &["mcLeaderboardHUD", &format!("playerName{row}"), "mLeaderboardData", "name_txt"]) {
                field.text = Some(assets.initial_text(team));
            }
        }
    }

    /// `UpdateVOIPIcons`: each player's last ranked row's voice icon. In a network game a changed
    /// `GetVoipStatus` shows `micoff` (enabled) or `micon` (speaking) and hides it otherwise, here
    /// always disabled; offline it hides.
    fn update_voip_icons(&mut self) {
        for i in 0..self.num_players {
            let Some(position) = self.last_player_position.get(&i).copied() else { continue };
            let row = position - 1;
            let changed = !self.online || self.last_player_voip.get(&row) != Some(&VOIP_DISABLED);
            if changed && let Some(voip) = find(&mut self.root, &["mcLeaderboardHUD", &format!("playerName{row}"), "voip"]) {
                voip.visible = false;
            }
            if self.online {
                self.last_player_voip.insert(row, VOIP_DISABLED);
            }
        }
    }

    /// `SetScreenSize(isThrowdown, maxFromMaxesX, mBack)`: the background plays the smallest
    /// size (`var` prefixed with a variant) whose back is wider than the content, then each
    /// back stretches to `GetYScale`.
    fn set_screen_size(&mut self, assets: &Assets, throwdown: bool, widest: f32) -> RenderResult<()> {
        let back = self.root.named(&[board(throwdown), "mBackground"])?;
        let (label, _) = size(back, assets, widest)?;
        back.goto(assets, &if self.use_variant { format!("var{label}") } else { label.to_owned() })?;
        let y_scale = self.y_scale(assets, throwdown)?;
        for name in ["mLargeBack", "mMediumBack", "mSmallBack"] {
            scale_y_background(self.root.named(&[board(throwdown), "mBackground", name])?, assets, y_scale)?;
        }
        Ok(())
    }

    /// `GetYScale(isThrowdown)`: the title's height, a margin and a row height per player (and
    /// the variant line's height on the ranked board while it shows), at least 95.
    fn y_scale(&mut self, assets: &Assets, throwdown: bool) -> RenderResult<f32> {
        #[expect(clippy::cast_precision_loss, reason = "ActionScript numbers")]
        let count = self.num_players as f32;
        let back = self.root.named(&[board(throwdown), "mBackground"])?;
        let scale = if throwdown {
            height(child(back, "mcTitle")?, assets)? + 30. + 29. * count
        } else {
            let mut top = height(child(back, "mcTitle")?, assets)? + 20.;
            let variant = child(back, "mContestVariant")?;
            if variant.clip.named(&["label"])?.visible {
                top += height(variant, assets)?;
            }
            top + 24. * count
        };
        Ok(if scale < 95. { 95. } else { scale })
    }

    /// `PositionHUD(isThrowdown, maxScaleX, mBack)`: the board sits `GetYScale` above its base
    /// line (700 for letters, 710 for ranks); the letter strips play the background's size,
    /// and each ranked strip stretches left across the back with its voice and turn icons.
    fn position_hud(&mut self, assets: &Assets, throwdown: bool, widest: f32) -> RenderResult<()> {
        let back = self.root.named(&[board(throwdown), "mBackground"])?;
        let (label, back_name) = size(back, assets, widest)?;
        let back_width = width(child(back, back_name)?, assets)?;
        if throwdown {
            let y = 700. - self.y_scale(assets, true)?;
            set_y(child(&mut self.root, "mcThrowdownHUD")?, y);
            for i in 0..self.num_players {
                if let Some(strip) = find(&mut self.root, &["mcThrowdownHUD", &format!("mcSkateStrip{i}")]) {
                    strip.goto(assets, label)?;
                }
            }
            return Ok(());
        }
        for i in 0..self.num_players {
            let hud = self.root.named(&["mcLeaderboardHUD"])?;
            if let Some(strip) = find_child(hud, &format!("mcStrip{i}")) {
                set_xscale(strip, 100.);
                let strip_width = width(strip, assets)?;
                set_xscale(strip, 0. - 100. * (back_width / strip_width));
            }
            if let Some(row) = find(hud, &[&format!("playerName{i}")]) {
                set_x(child(row, "voip")?, 0. - back_width + 15.);
                set_x(child(row, "mTurn")?, 0. - back_width - 15.);
            }
        }
        let y = 710. - self.y_scale(assets, false)?;
        set_y(child(&mut self.root, "mcLeaderboardHUD")?, y);
        Ok(())
    }
}

/// The board a layout uses.
const fn board(throwdown: bool) -> &'static str {
    if throwdown { "mcThrowdownHUD" } else { "mcLeaderboardHUD" }
}

/// `SetScreenSize`'s and `PositionHUD`'s choice of size label and back: `small` while the
/// content is narrower than the small back less 4, else `medium` likewise, else `large`.
fn size(back: &mut Clip, assets: &Assets, widest: f32) -> RenderResult<(&'static str, &'static str)> {
    let small = width(child(back, "mSmallBack")?, assets)? - 4.;
    let medium = width(child(back, "mMediumBack")?, assets)? - 4.;
    Ok(if widest < small {
        ("small", "mSmallBack")
    } else if widest < medium {
        ("medium", "mMediumBack")
    } else {
        ("large", "mLargeBack")
    })
}

/// `ScaleYBackground(mBackgroundHolder, ywidth)`: the centre piece takes the height the top and
/// bottom pieces leave (1 when none, floored) and the pieces stack from the top.
fn scale_y_background(holder: &mut Clip, assets: &Assets, y_scale: f32) -> RenderResult<()> {
    let top = child(holder, "mTopleft")?;
    let (top_y, top_height) = (top.entry.matrix.ty, height(top, assets)?);
    let bottom_height = height(child(holder, "mBottomleft")?, assets)?;
    let mut centre = y_scale - top_height - bottom_height;
    if centre < 0. {
        centre = 1.;
    }
    let centre_piece = child(holder, "mCenterleft")?;
    set_height(centre_piece, assets, centre.floor())?;
    set_y(centre_piece, top_y + top_height);
    let bottom_y = centre_piece.entry.matrix.ty + height(centre_piece, assets)?;
    set_y(child(holder, "mBottomleft")?, bottom_y);
    Ok(())
}

/// The data block's score text by layout: the time layout (`RankNameTime`) formats
/// `GetPlayerTimeFloat` (`0x826b1eb0`) with `FormatTime`, the sign shown for every rank but the
/// first (`GetPlayerRank` `0x826b1e40`), and a time within `EPSILON` of zero reads
/// `ID_RACE_LEADER`; distance layouts format `mFloatVar` (at least zero, one decimal) through
/// `ID_COMMON_DISTANCE`; others `FormatNumber(mIntVar)`.
fn score_text(assets: &Assets, data: &LeaderboardHud, i: i32) -> RenderResult<Vec<u16>> {
    let player = &data.players[usize::try_from(i).unwrap_or(0)];
    match data.menu_type - 4 {
        0 if player.time.abs() > f32::EPSILON => {
            let (hours, minutes, seconds, hundredths) = split_time(player.time);
            let flags = if player.rank == 1 { 0 } else { 2 };
            Ok(localizer::format_time(assets, hours, minutes, seconds, hundredths, true, flags).encode_utf16().collect())
        }
        0 => Ok(assets.initial_text("ID_RACE_LEADER")),
        2 | 5 | 6 => {
            let distance = if player.float_var <= 0. { 0. } else { player.float_var };
            localizer::format_custom(assets, "ID_COMMON_DISTANCE", &format!("{:.1}", f64::from(distance)))
        }
        _ => Ok(localizer::format_int(assets, player.int_var).encode_utf16().collect()),
    }
}

/// `Leaderboard_GetLeaderboardDataBlock`'s split of a time (`0x826b9124`): whole hours, minutes
/// and seconds and the hundredths of the fraction, in single precision as the native rounds them
/// (the minutes from one fused `fmsubs`).
#[expect(clippy::cast_possible_truncation, clippy::cast_precision_loss, reason = "the native fctiwz and integer conversions")]
pub(super) fn split_time(time: f32) -> (i32, i32, i32, i32) {
    let hours = (time * f32::from_bits(0x3991_a2b4)) as i32;
    let hundredths = ((time - (time as i32) as f32) * 100.0) as i32;
    let minutes = time.mul_add(f32::from_bits(0x3c88_8889), -((60 * hours) as f32)) as i32;
    let seconds = ((time - (3600 * hours) as f32) - (60 * minutes) as f32) as i32;
    (hours, minutes, seconds, hundredths)
}

/// `autoSize` on a single-line field aligned the same way: it keeps that edge (or centre) and
/// shrinks to its text, so it draws unchanged and its `_width` becomes `textWidth + 4`.
pub(super) fn auto_size(field: &mut Clip, assets: &Assets, mode: &str) -> RenderResult<()> {
    let Character::Text(text) = field.definition(assets)? else {
        return Err("leaderboard HUD auto-size target is not text".into());
    };
    let alignment = if mode == "right" { 1 } else { 2 };
    if text.multiline || text.alignment != alignment {
        return Err("leaderboard HUD auto-size field differs from the verified layout".into());
    }
    field.properties.insert("text_autosize".into(), mode.into());
    Ok(())
}

fn set_text_colour(field: &mut Clip, colour: u32) {
    field.properties.insert("text_colour".into(), colour.to_string());
}

/// A member's `_width` (`objectMemberLookup` `0x82a1ecd0`): an auto-sized text field's
/// `textWidth + 4`, otherwise its bounds through its own matrix (`GetBoundingRect`).
pub(super) fn width(piece: &Child, assets: &Assets) -> RenderResult<f32> {
    if piece.clip.properties.contains_key("text_autosize") {
        return text_field_width(&piece.clip, assets);
    }
    let r = bounds(&piece.clip, assets)?.ok_or("leaderboard HUD _width target has no bounds")?;
    Ok(((r.right - r.left) * piece.entry.matrix.a).abs())
}

/// A member's `_height`: its bounds through its own matrix; auto-sizing keeps a single line's
/// authored height.
fn height(piece: &Child, assets: &Assets) -> RenderResult<f32> {
    let r = bounds(&piece.clip, assets)?.ok_or("leaderboard HUD _height target has no bounds")?;
    Ok(((r.bottom - r.top) * piece.entry.matrix.d).abs())
}

/// A member, `None` (`undefined`) when absent.
/// The transform from Skate 3's letter strips into this board's `mcSkate` rows: Skate 3's
/// `mSkate` S.K.A.T.E. letters (where its `mUp` sits) scaled and moved onto this board's, at
/// their first frames.
fn skate3_letters_transform(assets: &Assets, skate3: usize, root: &mut Clip) -> RenderResult<Matrix> {
    let board = root.named(&["mcThrowdownHUD"])?;
    let skate = find_child(board, "mcSkate0").ok_or("letter board without mcSkate0")?;
    let (asset, character, at) = (skate.clip.asset, skate.clip.character, skate.entry.matrix);
    let ours = bounds(&Clip::new(assets, asset, character)?, assets)?.ok_or("letter board's letters without bounds")?;
    // The row's name panel (`mcSkateStrip0` at its current size): its right edge on the board.
    let panel = match find_child(board, "mcSkateStrip0") {
        Some(strip) => bounds(&strip.clip, assets)?.map(|r| r.right.mul_add(strip.entry.matrix.a, strip.entry.matrix.tx)),
        None => None,
    };
    let theirs = bounds(&Clip::new(assets, skate3, SKATE3_SKATE_LETTERS)?, assets)?.ok_or("Skate 3's letters without bounds")?;
    let up = bounds(&Clip::new(assets, skate3, SKATE3_ONE_UP_LETTERS)?, assets)?.ok_or("Skate 3's 1-Up letters without bounds")?;
    let scale = (ours.bottom - ours.top) / (theirs.bottom - theirs.top);
    if !scale.is_finite() || scale <= 0. {
        return Err("Skate 3's letters without height".into());
    }
    // Skate 3's SizePlayerRow puts both strips at the row's right end (`_x` = rowWidth - 6), right
    // aligned, and sizes the row around the shorter 1-Up strip; this board keeps its S.K.A.T.E.
    // width, so the 1-Up letters start just past the name panel (where the S.K.A.T.E. strip, which
    // starts under the panel's end, would leave them overlapping it).
    let left = panel
        .map(|right| (right + SKATE3_LETTERS_GAP - at.tx) / at.a)
        .filter(|left| left.is_finite())
        .unwrap_or(ours.left);
    Ok(Matrix { a: scale, b: 0., c: 0., d: scale, tx: up.left.mul_add(-scale, left), ty: theirs.top.mul_add(-scale, ours.top) })
}

/// A piece [`LeaderboardMovie::condense_for_skate3_letters`] moves: the letter board itself, its
/// title, or a back (`back:<name>`) of its background.
fn condensed_piece<'a>(root: &'a mut Clip, path: &str) -> RenderResult<&'a mut Child> {
    let board = child(root, "mcThrowdownHUD")?;
    if path == "board" {
        return Ok(board);
    }
    let background = child(&mut board.clip, "mBackground")?;
    match path.strip_prefix("back:") {
        Some(name) => child(&mut background.clip, name),
        None => child(&mut background.clip, "mcTitle"),
    }
}

pub(super) fn find_child<'a>(clip: &'a mut Clip, name: &str) -> Option<&'a mut Child> {
    clip.children.values_mut().find(|c| c.entry.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(name)))
}

/// A member path, `None` (`undefined`) where a member is absent.
pub(super) fn find<'a>(clip: &'a mut Clip, path: &[&str]) -> Option<&'a mut Clip> {
    let Some((first, rest)) = path.split_first() else { return Some(clip) };
    find(&mut find_child(clip, first)?.clip, rest)
}

/// `Math.max`: NaN when either is.
fn as_max(a: f32, b: f32) -> f32 {
    if a.is_nan() || b.is_nan() { f32::NAN } else { a.max(b) }
}

/// The movie's InitAction: the `leaderboardhud` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 133, 0x70e8))
}

/// Timeline actions besides `Stop`: the root constructs the class (and plays on into its
/// intro).
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x70a4 => clip.calls.push("leaderboardhud"),
        _ => return Err(format!("untranslated leaderboard HUD APT action {offset:#x}").into()),
    }
    Ok(())
}
