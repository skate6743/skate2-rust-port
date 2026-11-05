//! Fingerprinted StartHub/challenge_info/menu_picker pause presentation.
//! Native input and option ownership live in ui::pause_menu; destinations are
//! deliberately outside this retained movie translation.
use super::{
    assets::Assets,
    clips::{Child, Clip},
    localizer,
    objective_movie::{set_x, set_y, text_field_width},
    tutorial_movie::max_scroll,
};
use crate::gpu_renderer::RenderResult;
use skate2_core::{
    challenge::start_hub::{ChallengeInfoView, StartHubView},
    ui::{
        apt_timeline::{DisplayEntry, PlacementIdentity, IDENTITY},
        pause_menu::HubPopup,
    },
};
use skate2_formats::apt::{Character, Rect};

/// `UpdatePositions`' fixed row of the button menu in the pause and challenge modes.
const VIEW_Y: f32 = 577.;

/// A picker row as `SetItemText` and `IsItemEnabled` give it: the option's HAL id, the count
/// `GetOptionText` prints into it (a "new" label's `%d`), and whether it is enabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct HubRow {
    pub label: &'static str,
    pub count: Option<i32>,
    pub enabled: bool,
}

/// What `SetScreenState`'s challenge_info `SetMode` shows besides the rows, as its text fields
/// show it.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct HubLayout {
    /// The wallet (`EnableCash`): `FEUnlock.Cash_GetFormattedCashString` of the cash.
    cash: Option<Vec<u16>>,
    /// The challenge mode's texts, for a screen in state 1.
    challenge: Option<ChallengeLayout>,
}

impl HubLayout {
    /// The hub's layout from the career's view: the wallet where `shows_cash`, the challenge
    /// mode's texts for state 1.
    pub fn new(assets: &Assets, pause: &skate2_core::ui::pause_menu::PauseMenuSnapshot, view: Option<&StartHubView>) -> RenderResult<Self> {
        let cash = view.map_or(0, |view| view.cash);
        #[expect(clippy::cast_precision_loss, reason = "the native's (float) of the cash")]
        let cash = pause.shows_cash().then(|| localizer::format_currency(assets, cash as f32).encode_utf16().collect());
        let challenge = if pause.screen_state() == 1 {
            let info = view.and_then(|view| view.challenge.clone()).unwrap_or_default();
            Some(ChallengeLayout::new(assets, &info)?)
        } else {
            None
        };
        Ok(Self { cash, challenge })
    }
}

/// What `challenge_info.SetMode(1)` (`challenge_info.apt` `0x4a3c`) shows in the in-challenge hub
/// (`SetScreenState(1)`), as its text fields show it.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct ChallengeLayout {
    description: Vec<u16>,
    description_enabled: bool,
    /// The rows `InitObjectives` fills, when `Mission.IsObjectivesEnabled(false)`.
    objectives: Option<Vec<Vec<u16>>>,
}

impl ChallengeLayout {
    /// `Mission.GetDescription` through the localizer's `FormatStringCustom` (or the native's
    /// literal), and each `SignUp_GetObjectiveTitle` likewise.
    fn new(assets: &Assets, view: &ChallengeInfoView) -> RenderResult<Self> {
        let description = if view.description.formatted {
            localizer::format_custom(assets, &view.description.text, "")?
        } else {
            assets.initial_text(&view.description.text)
        };
        let objectives = if view.objectives_enabled {
            let rows = view.objectives.iter().map(|(hal, arguments)| localizer::format_custom(assets, hal, arguments));
            Some(rows.collect::<RenderResult<Vec<_>>>()?)
        } else {
            None
        };
        Ok(Self { description, description_enabled: view.description_enabled, objectives })
    }
}

pub(super) struct PauseMovie {
    pub root: Clip,
    pub popup: Option<Clip>,
    popup_selection: Option<(HubPopup, usize)>,
    requested_visible: bool,
    /// `OnEnter`'s `OpenScreen(11)` waits for the screen manager to link the screen.
    pending: bool,
    /// The hub's rows and its row, which a linked screen takes.
    rows: Vec<HubRow>,
    target: usize,
    selected: Option<usize>,
    /// `OnIntroDone`'s `SetScreenState` set the linked screen up.
    configured: bool,
    /// The rows the picker holds (`SetNumOptions`).
    configured_rows: usize,
    /// The intro's `IntroComplete` since the last take.
    intro_complete: bool,
    /// The outro's `OutroComplete` since the last take.
    outro_complete: bool,
    pub sounds: Vec<&'static str>,
    /// What `SetScreenState` sets up besides the rows.
    layout: HubLayout,
}

impl PauseMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, 4, 0)?;
        root.playback.stop();
        root.visible = false;
        Ok(Self { root, popup:None,popup_selection:None, requested_visible: false, pending: false,
            rows: Vec::new(), target: 0, selected: None, configured: false, configured_rows: 0,
            intro_complete: false, outro_complete: false, sounds: Vec::new(), layout: HubLayout::default() })
    }

    /// What the next `SetScreenState` sets up besides the rows.
    pub fn set_layout(&mut self, layout: HubLayout) {
        self.layout = layout;
    }

    /// FE state 7 on top or not: its `OnEnter` (0x8265e4d0) opens the screen, which the screen
    /// manager links later (`link`); `OnLeave` (0x8264c4d8) closes it, playing its outro.
    pub fn update(&mut self, assets: &Assets, visible: bool, selected: usize, rows: &[HubRow]) -> RenderResult<()> {
        self.sounds.clear();
        self.rows = rows.to_vec();
        self.target = selected;
        if visible != self.requested_visible {
            self.requested_visible = visible;
            if visible {
                self.pending = true;
            } else if !std::mem::take(&mut self.pending) {
                self.root.goto(assets, "outro")?;
            }
        }
        // The hub's options can change while it shows (a challenge ending into the lobby):
        // `SetScreenState` sets the picker's rows again.
        if visible && !self.pending && self.configured && self.rows.len() != self.configured_rows {
            self.configure(assets)?;
            self.selected = None;
        }
        if visible && !self.pending && self.configured && self.selected != Some(selected) {
            self.select(assets, selected)?;
            self.selected = Some(selected);
        }
        self.callbacks(assets)
    }

    /// The shown hub's picker holds a different number of rows than `count`: its next update
    /// sets the screen up again.
    pub fn rows_change(&self, count: usize) -> bool {
        self.configured && !self.pending && count != self.configured_rows
    }

    /// The state waits for its screen.
    pub fn pending(&self) -> bool {
        self.pending
    }

    /// The screen manager linked the StartHub's screen: a new component at its `intro`, its
    /// challenge_info hidden (the constructor's `mScreen._visible = false`) until `OnIntroDone`'s
    /// `SetScreenState` (`0x826536e0`) sets it up, when the intro completes.
    pub fn link(&mut self, assets: &Assets) -> RenderResult<()> {
        if !std::mem::take(&mut self.pending) {
            return Ok(());
        }
        self.root = Clip::new(assets, 4, 0)?;
        self.root.playback.stop();
        self.root.named(&["mHub", "mcChallengeInfo"])?.visible = false;
        self.configured = false;
        self.selected = None;
        self.root.goto(assets, "intro")?;
        self.callbacks(assets)
    }

    /// The screen's `IntroComplete` since the last call.
    pub fn take_intro_complete(&mut self) -> bool {
        std::mem::take(&mut self.intro_complete)
    }

    /// The screen's `OutroComplete` since the last call.
    pub fn take_outro_complete(&mut self) -> bool {
        std::mem::take(&mut self.outro_complete)
    }

    /// The hub's confirmation (sk8popup), each template's message under the warning title.
    pub fn update_quit(&mut self,assets:&Assets,selection:Option<(HubPopup, usize)>)->RenderResult<()> {
        if selection==self.popup_selection{return Ok(());}
        if let Some((popup, selected))=selection {
            if self.popup.is_none(){let mut popup=Clip::new(assets,25,0)?;popup.goto(assets,"intro")?;self.popup=Some(popup);}
            super::edit_skater_movie::configure_popup(self.popup.as_mut().unwrap(),assets,selected,
                "ID_COMMON_WARNING_TITLE",popup.message(),
                &["ID_COMMON_NO","ID_COMMON_YES"],false)?;
        } else if let Some(popup)=self.popup.as_mut(){popup.goto(assets,"outro")?;}
        self.popup_selection=selection;
        Ok(())
    }
    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if let Some(popup)=self.popup.as_mut(){popup.tick(assets)?;let mut calls=Vec::new();popup.take_calls(&mut calls);
            for call in calls {match call {
                "CacGenderPopupIntroComplete"=>{},
                "CacGenderPopupOutroComplete"=>{self.popup=None;},
                _=>return Err(format!("untranslated quit popup callback {call}").into()),
            }}
        }
        if self.root.visible { self.root.tick(assets)?; }
        self.callbacks(assets)
    }

    /// The screen's callbacks; its `IntroComplete` reaches `OnScreenReady`, whose `OnIntroDone`
    /// calls `SetScreenState` at once: the screen set up and the hub's row selected.
    fn callbacks(&mut self, assets: &Assets) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "PauseIntroComplete" => {
                    self.intro_complete = true;
                    if !self.configured {
                        self.configure(assets)?;
                        self.select(assets, self.target)?;
                        self.selected = Some(self.target);
                        self.configured = true;
                    }
                }
                "PauseOutroComplete" => {
                    self.outro_complete = true;
                    self.root.visible = false;
                }
                "PauseSound" => self.sounds.push("FE_SOUND_PAUSE_MENU"),
                _ => return Err(format!("untranslated pause callback {call}").into()),
            }
        }
        Ok(())
    }

    fn configure(&mut self, assets: &Assets) -> RenderResult<()> {
        let count = self.rows.len();
        self.configured_rows = count;
        if let Some(challenge) = &self.layout.challenge {
            return configure_challenge(&mut self.root, assets, count, challenge, self.layout.cash.as_deref());
        }
        // starthub SetState0 -> SetMode0 -> challenge_info SetMode_FreeSkate.
        self.root.named(&["mHub"])?.goto(assets, "pause")?;
        let info = self.root.named(&["mHub", "mcChallengeInfo"])?;
        info.visible = true;
        for child in info.children.values_mut() {
            child.clip.visible = matches!(child.entry.name.as_deref(),
                Some("mHeader" | "mMenuPicker" | "mView"));
        }
        let header_height = bounds(info.named(&["mHeader"])?, assets)?
            .map_or(0., |r| r.bottom - r.top);
        info.named(&["mHeader", "labelText"])?.text =
            Some(assets.localize("ID_PAUSEMENU_PAUSE")?.encode_utf16().collect());
        // challenge_info::UpdatePositions 0x3ee4, kPAUSE_OFFSET=168.
        child(info, "mHeader")?.entry.matrix.ty += 168.;
        child(info, "mMenuPicker")?.entry.matrix.ty = header_height + 10. + 168.;
        child(info, "mView")?.entry.matrix.ty = VIEW_Y;
        add_items(info, assets, count)?;
        // SetMode(0)'s wallet unless the game is Party Play or free skate.
        if let Some(cash) = &self.layout.cash {
            set_wallet(info, assets, cash)?;
        }
        configure_left_buttons(info.named(&["mView"])?, assets, &[("A", "ID_COMMON_SELECT"), ("B", "ID_COMMON_BACK")])?;
        Ok(())
    }

    fn select(&mut self, assets: &Assets, selected: usize) -> RenderResult<()> {
        if selected >= self.rows.len() { return Err("pause selection outside authored options".into()); }
        let picker = self.root.named(&["mHub", "mcChallengeInfo", "mMenuPicker", "mMenuPicker"])?;
        let selected_y;
        {
            let group = picker.named(&["mItemGroup"])?;
            for (i, row) in self.rows.iter().enumerate() {
                let item = group.named(&[&format!("item{i}")])?;
                item.goto(assets, if i == selected { "selected" }
                    else if row.enabled { "deselected" } else { "disabled" })?;
                let label = assets.localize(row.label)?;
                let label = row.count.map_or_else(|| label.clone(), |count| label.replace("%d", &count.to_string()));
                let text: Vec<u16> = label.encode_utf16().collect();
                item.named(&["itemText"])?.text = Some(text.clone());
                item.named(&["mTextglow", "itemText"])?.text = Some(text);
            }
            selected_y = child(group, &format!("item{selected}"))?.entry.matrix.ty;
        }
        child(picker, "mIndicator")?.entry.matrix.ty = selected_y;
        Ok(())
    }
}

/// The picker's rows (`SetNumOptions`): `dummy0`..`dummy7` hidden and an `ITEMLABEL_Dark` per
/// option in `mItemGroup`, `dummy0`'s height apart.
fn add_items(info: &mut Clip, assets: &Assets, count: usize) -> RenderResult<()> {
    let picker = info.named(&["mMenuPicker", "mMenuPicker"])?;
    let item_height = bounds(picker.named(&["dummy0"])?, assets)?
        .map_or(0., |r| r.bottom - r.top);
    for i in 0..8 { picker.named(&[&format!("dummy{i}")])?.visible = false; }
    let group = picker.named(&["mItemGroup"])?;
    // SetNumOptions removes the previous options before attaching the new count.
    group.children.retain(|_, child| !child.entry.name.as_deref().is_some_and(|name| {
        name.strip_prefix("item").and_then(|n| n.parse::<usize>().ok()).is_some_and(|n| n >= count)
    }));
    // SetSkin on mHub.mMenu addresses an absent instance in the shipped
    // movie. The real picker retains its LIGHT_ON_DARK default.
    for i in 0..count {
        let mut item = Clip::new(assets, 6, 31)?; // ITEMLABEL_Dark
        item.goto(assets, "deselected")?;
        let mut matrix = IDENTITY;
        matrix.ty = item_height * i as f32;
        group.children.insert(i as i32, Child {
            entry: DisplayEntry { identity: PlacementIdentity { character: 31, frame: 0 },
                matrix, colour_scale: u32::MAX, colour_bias: 0, ratio: 0.,
                name: Some(format!("item{i}")), clip_depth: -1,
                events: Vec::new(), blend_mode: None }, clip: item,
        });
    }
    Ok(())
}

/// starthub `SetState(1)` -> `SetMode(1)`: `mHub` at `tall` (the short backing removed, the tall one
/// shown) and challenge_info's challenge mode (`SetMode(1)`, `0x4a3c`): every part off, the header
/// `ID_PAUSEMENU_PAUSE` at (`mTitleOffsetX`, `mTitleOffsetY` + 7) (the prototype's zeros), the
/// description (shown with `InChallengeIsDescriptionEnabled`), the objectives (`InitObjectives`),
/// the picker's rows, the wallet (`totalbetText` right of the auto-sized `totalbetHALText`) and the
/// A/B button menu; then `UpdatePositions`.
fn configure_challenge(root: &mut Clip, assets: &Assets, count: usize, layout: &ChallengeLayout, cash: Option<&[u16]>) -> RenderResult<()> {
    root.named(&["mHub"])?.goto(assets, "tall")?;
    let info = root.named(&["mHub", "mcChallengeInfo"])?;
    info.visible = true;
    for part in info.children.values_mut() {
        part.clip.visible = matches!(part.entry.name.as_deref(), Some("mHeader" | "mMenuPicker" | "mCash" | "mView"));
    }
    info.named(&["mHeader", "labelText"])?.text = Some(assets.localize("ID_PAUSEMENU_PAUSE")?.encode_utf16().collect());
    info.named(&["mDescription", "labelText"])?.text = Some(layout.description.clone());
    child(info, "mDescription")?.clip.visible = layout.description_enabled;
    let header = child(info, "mHeader")?;
    set_x(header, 0.);
    set_y(header, 7.);
    let objectives = match &layout.objectives {
        Some(rows) => init_objectives(info, rows)?,
        None => 0,
    };
    add_items(info, assets, count)?;
    set_wallet(info, assets, cash.unwrap_or_default())?;
    configure_left_buttons(info.named(&["mView"])?, assets, &[("A", "ID_COMMON_SELECT"), ("B", "ID_COMMON_BACK")])?;
    update_challenge_positions(info, assets, objectives)
}

/// `EnableCash(true)` and the wallet's text: `totalbetText` right of the auto-sized
/// `totalbetHALText` (the constructor's `autoSize = "left"`); `UpdatePositions` puts the wallet 10
/// above the button menu's fixed row.
pub(super) fn set_wallet(info: &mut Clip, assets: &Assets, cash: &[u16]) -> RenderResult<()> {
    let wallet = child(info, "mCash")?;
    wallet.clip.visible = true;
    let label = child(&mut wallet.clip, "totalbetHALText")?;
    label.clip.properties.insert("text_autosize".into(), "left".into());
    let right = label.entry.matrix.tx + text_field_width(&label.clip, assets)?;
    let value = child(&mut wallet.clip, "totalbetText")?;
    value.clip.text = Some(cash.to_vec());
    set_x(value, right);
    let wallet_height = height(wallet, assets)?;
    set_y(wallet, VIEW_Y - wallet_height - 10.);
    Ok(())
}

/// `InitObjectives` (`0x5399`): with any objective, `EnableObjectives` (the objective text hidden),
/// the four rows hidden, then each objective's title in its row; the movie has no row past the
/// fourth. Returns `mNumOptions`.
fn init_objectives(info: &mut Clip, rows: &[Vec<u16>]) -> RenderResult<usize> {
    if rows.is_empty() {
        return Ok(0);
    }
    child(info, "mObjectiveText")?.clip.visible = false;
    child(info, "mObjectives")?.clip.visible = true;
    let list = info.named(&["mObjectives"])?;
    for i in 0..4 {
        child(list, &format!("mObjective{i}"))?.clip.visible = false;
    }
    for (i, text) in rows.iter().enumerate().take(4) {
        let row = child(list, &format!("mObjective{i}"))?;
        row.clip.named(&["labelText"])?.text = Some(text.clone());
        row.clip.visible = true;
    }
    Ok(rows.len())
}

/// `UpdatePositions` (`0x3ec4`) in the challenge mode: shown objectives hide the description; from
/// the header down, 10 apart, the description 5 lower (16 per line plus 15), the objective rows 26
/// per line under their title, and the picker; the button menu at its fixed row and the wallet 10
/// above it.
fn update_challenge_positions(info: &mut Clip, assets: &Assets, objectives: usize) -> RenderResult<()> {
    let header = child(info, "mHeader")?;
    let mut y = header.entry.matrix.ty + height(header, assets)? + 10.;
    if child(info, "mObjectives")?.clip.visible {
        child(info, "mDescription")?.clip.visible = false;
    }
    let description = child(info, "mDescription")?;
    if description.clip.visible {
        y += 5.;
        set_y(description, y);
        #[expect(clippy::cast_precision_loss, reason = "a line count")]
        let lines = max_scroll(description.clip.named(&["labelText"])?, assets)? as f32;
        y += (16. * lines).floor() + 15.;
    }
    if child(info, "mObjectives")?.clip.visible {
        set_y(child(info, "mObjectives")?, y);
        let list = info.named(&["mObjectives"])?;
        let mut row_y = height(child(list, "mObjectiveTitle")?, assets)?;
        for i in 0..objectives.min(4) {
            let row = child(list, &format!("mObjective{i}"))?;
            if row.clip.visible {
                set_y(row, row_y);
                #[expect(clippy::cast_precision_loss, reason = "a line count")]
                let lines = max_scroll(row.clip.named(&["labelText"])?, assets)? as f32;
                row_y += lines * 26.;
            }
        }
        y += row_y;
    }
    set_y(child(info, "mMenuPicker")?, y);
    set_y(child(info, "mView")?, VIEW_Y);
    Ok(())
}

/// A piece's `_height`: its bounds through its own (unrotated) matrix.
fn height(piece: &Child, assets: &Assets) -> RenderResult<f32> {
    let r = bounds(&piece.clip, assets)?.ok_or("pause _height target has no bounds")?;
    Ok(((r.bottom - r.top) * piece.entry.matrix.d).abs())
}

/// A named child, found case-insensitively as Apt does (`AptStricmp`).
pub(super) fn child<'a>(clip: &'a mut Clip, name: &str) -> RenderResult<&'a mut Child> {
    clip.children.values_mut().find(|c| c.entry.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(name)))
        .ok_or_else(|| format!("missing authored pause child {name}").into())
}

/// The button_menu constructor's `SetInfo(Marker_streak, 24, 0xffffec)` and
/// `ClearItems`, with the first `count` items shown by their `AddItem`s.
fn set_button_info(view: &mut Clip, font: &super::assets::Font, count: usize) -> RenderResult<()> {
    for i in 0..4 {
        let text = child(view, &format!("mText{i}"))?;
        text.clip.visible = i < count;
        text.clip.properties.insert("text_font".into(), "Marker_streak".into());
        text.clip.properties.insert("text_height".into(), "24".into());
        text.clip.properties.insert("text_colour".into(), 0xffff_ffecu32.to_string());
        // SkateAptString82a70d60: textHeight = GetMaxCharHeight()*size.
        // cFont::GetMaxCharHeight82849a80 multiplies Ascent by baseScale.Y.
        text.entry.matrix.ty = 0.5 - font.metrics.ascent * font.scale[1] * 24. / 2. - 4.;
        child(view, &format!("mButton{i}"))?.clip.visible = i < count;
    }
    Ok(())
}

/// challenge_info's `mView` button menu after the button_menu constructor's
/// `SetInfo(Marker_streak, 24, 0xffffec)`, `ClearItems`,
/// `SetJustify("left")`, the `AddItem`s and `RefreshItems`.
pub(super) fn configure_left_buttons(view: &mut Clip, assets: &Assets, items: &[(&str, &str)]) -> RenderResult<()> {
    let font = assets.fonts.get("Marker_streak").ok_or("pause button font absent")?;
    let mut x = child(view, "mText0")?.entry.matrix.tx;
    set_button_info(view, font, items.len())?;
    for (i, (button, label)) in items.iter().enumerate() {
        let entry = child(view, &format!("mButton{i}"))?;
        entry.entry.matrix.tx = x;
        entry.clip.named(&["mButtonRender"])?.properties.insert("_test".into(), (*button).into());
        // _width uses the APT symbol's authored32-unit bounds, independently
        // of the custom button texture's rendered pixel dimensions.
        let button_bounds = bounds(&entry.clip, assets)?.ok_or("pause button bounds absent")?;
        x += button_bounds.right - button_bounds.left - 10.;
        let text = child(view, &format!("mText{i}"))?;
        text.entry.matrix.tx = x;
        let units: Vec<u16> = assets.localize(label)?.encode_utf16().collect();
        x += units.iter().filter_map(|&c| font.metrics.mapped_glyph(c))
            .map(|g| g.advance * font.scale[0] * 24.).sum::<f32>() + 25.;
        text.clip.text = Some(units);
    }
    Ok(())
}

/// A button menu after its constructor, the `AddItem`s and `RefreshItems`
/// with the prototype's `mJustify` (`"right"`): from `mText0`'s authored x
/// leftwards, each item's auto-sized text, its button overlapping the text by
/// 10, and `mOffset` (25) to the next item.
pub(super) fn configure_right_buttons(view: &mut Clip, assets: &Assets, items: &[(&str, &str)]) -> RenderResult<()> {
    let font = assets.fonts.get("Marker_streak").ok_or("button menu font absent")?;
    let mut x = child(view, "mText0")?.entry.matrix.tx;
    set_button_info(view, font, items.len())?;
    for (i, (button, label)) in items.iter().enumerate() {
        let units: Vec<u16> = assets.localize(label)?.encode_utf16().collect();
        x -= units.iter().filter_map(|&c| font.metrics.mapped_glyph(c))
            .map(|g| g.advance * font.scale[0] * 24.).sum::<f32>();
        let text = child(view, &format!("mText{i}"))?;
        text.entry.matrix.tx = x;
        text.clip.text = Some(units);
        x += 10.;
        let entry = child(view, &format!("mButton{i}"))?;
        entry.clip.named(&["mButtonRender"])?.properties.insert("_test".into(), (*button).into());
        let button_bounds = bounds(&entry.clip, assets)?.ok_or("button menu bounds absent")?;
        x -= button_bounds.right - button_bounds.left;
        entry.entry.matrix.tx = x;
        x -= 25.;
    }
    Ok(())
}

/// Authored local bounds for constructor _height queries; these clips have
/// no rotations and no scripted autoSize yet when the queries run.
pub(super) fn bounds(clip: &Clip, assets: &Assets) -> RenderResult<Option<Rect>> {
    let mut result = match clip.definition(assets)? {
        Character::Shape { bounds } => Some(*bounds),
        Character::Text(text) => Some(text.bounds),
        _ => None,
    };
    for child in clip.children.values() {
        if let Some(r) = bounds(&child.clip, assets)? {
            let m = child.entry.matrix;
            let r = Rect { left: r.left*m.a + m.tx, right: r.right*m.a + m.tx,
                top: r.top*m.d + m.ty, bottom: r.bottom*m.d + m.ty };
            result = Some(match result {
                None => r,
                Some(old) => Rect { left: old.left.min(r.left), top: old.top.min(r.top),
                    right: old.right.max(r.right), bottom: old.bottom.max(r.bottom) },
            });
        }
    }
    Ok(result)
}

/// Only the verified class-registration streams; imported constructors have
/// their pause-mode effects applied by configure, not a general AVM.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (4,30,0xeb4) | (5,110,0x378c)
        | (5,109,0x5604) | (6,56,0x28e0) | (6,55,0x37f4)
        | (7,8,0x4dc) | (7,7,0xae4) | (8,4,0x284) | (8,3,0x574))
}

pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match (clip.asset, offset) {
        (4, 0xe98 | 0xeb4 | 0x1108) |
        (5, 0x378c | 0x5604) | (6, 0x28e0 | 0x37f4) |
        (7, 0x4dc | 0xae4) | (8, 0x284 | 0x574) => {},
        (4, 0x1114) => { clip.playback.stop(); clip.calls.push("PauseIntroComplete"); }
        (4, 0x1138) => clip.calls.push("PauseSound"),
        (4, 0x1158) => { clip.playback.stop(); clip.calls.push("PauseOutroComplete"); }
        _ => return Err(format!("untranslated pause APT action {}:{offset:#x}", clip.asset).into()),
    }
    Ok(())
}
