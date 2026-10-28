//! Original onlinehub/onlineHubMenu (screen 92, asset 108, fingerprint 66f8fc9d; template entry
//! `OnlineHubMenu` 0x82fdb918, placement 2; class `onlineHubMenu`, InitAction character 53
//! `0x266c`) with its `mHubTree` (controls/hub_tree, asset 109, fingerprint b55d1026, class
//! `hub_tree`) of three `hub_menu`s (controls/hub_menu, asset 110, fingerprint a135e0bf), each a
//! `menu_picker` in the `HUB_MENU` skin over a `background_scaler` nine-slice
//! (controls/background_scaler, asset 111, fingerprint 4cfe92e1), driven by
//! `skate2_core::ui::online_hub`: `GetHubMenuStack`, `GetHubMenuTitle`, `GetHubMenuDataForID`,
//! `GetSelectedItemForHubMenuID` and `OnMenuGetItemEnabled` read the hub's menus. Also the
//! online popups (sk8popup): the kick confirmation, the searching and signing-in popups and the
//! notices, over the hub, a screen over it or the game.
use super::{
    assets::Assets,
    clips::{Child, Clip},
    edit_skater_movie, localizer, map_movie,
    objective_movie::text_field_width,
    pause_movie::{bounds, child},
};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::{
    apt_timeline::{DisplayEntry, IDENTITY, PlacementIdentity},
    online_hub::{MenuView, Popup, ScreenCall, Snapshot, Title},
};

pub(super) const HUB: usize = 108;
pub(super) const TREE: usize = 109;
pub(super) const MENU: usize = 110;
pub(super) const SCALER: usize = 111;
/// `mMaxMenuDepth`.
const DEPTH: usize = 3;
/// `hub_menu.mNumOptions`, `mItemOffset` and `mMinInnerMenuWidth`.
const OPTIONS: usize = 8;
const ITEM_OFFSET: f32 = 34.0;
const MIN_INNER_WIDTH: f32 = 160.0;
/// `UpdateMenuWindow`'s `ScaleDimensions` height.
const INNER_HEIGHT: f32 = 128.0;
/// background_scaler's `mcHubMenu` export: the hub menu's nine-slice frame.
pub(super) const NINE_SLICE: u32 = 28;
/// menu_picker's `ITEMLABEL_HubMenu` export.
const ITEM_LABEL: &str = "ITEMLABEL_HubMenu";
const PICKER_ASSET: usize = 6;
/// sk8popup.
const POPUP_ASSET: usize = 25;

/// One `hub_menu` as its ActionScript holds it.
#[derive(Clone, Debug, Default)]
struct HubMenu {
    /// `eMenuID`.
    menu: Option<u32>,
    /// The `mItemTextArray` and enabled items the picker shows.
    shown: Option<MenuView>,
    /// `mHighlightEnabled`.
    enabled: bool,
}

pub(super) struct OnlineHubMovie {
    pub root: Clip,
    open: bool,
    menus: [HubMenu; DEPTH],
    /// `hub_tree.mCurrentMenuIndex`.
    current: Option<usize>,
    intro_complete: bool,
    pub popup: Option<Clip>,
    popup_shown: Option<Popup>,
}

impl OnlineHubMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, HUB, 0)?;
        root.playback.stop();
        root.visible = false;
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        Ok(Self { root, open: false, menus: Default::default(), current: None, intro_complete: false, popup: None, popup_shown: None })
    }

    pub fn open(&self) -> bool {
        self.open
    }

    pub fn take_intro_complete(&mut self) -> bool {
        std::mem::take(&mut self.intro_complete)
    }

    /// `OpenScreen(92)` when the hub opens, its `SetScreenState` calls, the picker's `Update`
    /// as the selection moves, and `CloseScreen` (the outro) when it closes; and the popup.
    pub fn update(&mut self, assets: &Assets, snapshot: Option<&Snapshot>, opened: bool, calls: &[ScreenCall], popup: Option<&Popup>) -> RenderResult<()> {
        if opened && snapshot.is_some() {
            self.construct(assets)?;
        }
        let Some(snapshot) = snapshot else {
            if self.open {
                self.open = false;
                self.root.goto(assets, "outro")?;
            }
            return self.update_popup(assets, popup);
        };
        for call in calls {
            match call {
                ScreenCall::Initialize => self.initialize(assets, &snapshot.stack)?,
                ScreenCall::Push => {
                    self.push(assets, &snapshot.stack)?;
                    self.root.goto(assets, "0-1")?;
                }
                ScreenCall::Pop => {
                    self.pop(assets)?;
                    self.root.goto(assets, "1-0")?;
                }
            }
        }
        // menu_picker NextValue and PrevValue: SetValue, then Update.
        if let Some(current) = self.current
            && let Some(view) = snapshot.stack.get(current)
            && self.menus[current].menu == Some(view.id)
            && self.menus[current].shown.as_ref().is_some_and(|shown| shown.selected != view.selected)
        {
            self.render(assets, current, view)?;
        }
        self.update_popup(assets, popup)
    }

    /// The screen loads: the constructor (`mState` -1, `stop`, `RegisterScreen`), each
    /// `hub_menu`'s constructor, then the screen manager's intro.
    fn construct(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root = Clip::new(assets, HUB, 0)?;
        self.root.visible = true;
        self.open = true;
        self.current = None;
        self.menus = Default::default();
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for index in 0..DEPTH {
            self.construct_menu(assets, index)?;
        }
        self.root.goto(assets, "intro")
    }

    fn menu(&mut self, index: usize) -> RenderResult<&mut Clip> {
        self.root.named(&["mHubTree", &format!("mHubMenu{index}")])
    }

    /// `hub_menu`'s constructor: the title shown, the loading wheel hidden, the frame's
    /// `HUB_MENU` skin, the picker's skin with eight items 34 apart, B Back and A Next, and the
    /// menu hidden and disabled until `Init`.
    fn construct_menu(&mut self, assets: &Assets, index: usize) -> RenderResult<()> {
        let menu = self.menu(index)?;
        menu.named(&["mMenu", "mTitle"])?.visible = true;
        menu.named(&["mMenu", "mLoading"])?.visible = false;
        menu.named(&["mMenu", "mButtonHelp"])?.visible = true;
        // background_scaler: the constructor removes mHubMenu and mShading; SetSkin("HUB_MENU")
        // attaches mcHubMenu as mNineSlice at depth 3.
        let background = menu.named(&["mMenu", "mBackground"])?;
        background.children.retain(|_, child| !matches!(child.entry.name.as_deref(), Some("mHubMenu" | "mShading")));
        background.children.insert(3, placed(Clip::new(assets, SCALER, NINE_SLICE)?, NINE_SLICE, "mNineSlice", IDENTITY));
        let picker = menu.named(&["mMenu", "mMenuPicker", "mMenuPicker"])?;
        for dummy in 0..8 {
            picker.named(&[&format!("dummy{dummy}")])?.visible = false;
        }
        let export = assets.movies[PICKER_ASSET].movie.exports.iter().find(|export| export.name == ITEM_LABEL).ok_or("missing hub menu item export")?.character_id;
        let group = picker.named(&["mItemGroup"])?;
        group.children.clear();
        for item in 0..OPTIONS {
            let mut clip = Clip::new(assets, PICKER_ASSET, export)?;
            clip.goto(assets, "deselected")?;
            clip.visible = false;
            let mut matrix = IDENTITY;
            matrix.ty = ITEM_OFFSET * item as f32;
            group.children.insert(i32::try_from(item).unwrap_or(0), placed(clip, export, &format!("item{item}"), matrix));
        }
        map_movie::styled_button_items(
            menu.named(&["mMenu", "mButtonHelp"])?,
            assets,
            &[("B", "ID_COMMON_BACK"), ("A", "ID_COMMON_NEXT")],
            0xffff_feec,
            false,
            "Marker_streak",
            20.0,
        )?;
        menu.visible = false;
        set_enabled(menu, false)?;
        tool_tip(menu, assets, false)
    }

    /// `SetState(0)`: `SetMenuOffset(5)` and `hub_tree.Initialize`: the menus under the top one
    /// set up disabled, the top one enabled, the ones above cleared.
    fn initialize(&mut self, assets: &Assets, stack: &[MenuView]) -> RenderResult<()> {
        let current = stack.len().checked_sub(1);
        self.current = current;
        for index in 0..DEPTH {
            match (stack.get(index), current) {
                (Some(view), Some(current)) if index <= current => self.set_menu(assets, index, view, index == current)?,
                _ => self.clear(assets, index)?,
            }
        }
        Ok(())
    }

    /// `OnMenuPush`: the top menu minimizes, the pushed one is set up enabled.
    fn push(&mut self, assets: &Assets, stack: &[MenuView]) -> RenderResult<()> {
        if let Some(current) = self.current {
            self.minimize(assets, current)?;
        }
        let Some(current) = stack.len().checked_sub(1).filter(|&current| current < DEPTH) else { return Ok(()) };
        self.current = Some(current);
        self.set_menu(assets, current, &stack[current], true)
    }

    /// `OnMenuPop`: the top menu clears and the one under it restores.
    fn pop(&mut self, assets: &Assets) -> RenderResult<()> {
        let Some(current) = self.current else { return Ok(()) };
        self.clear(assets, current)?;
        let Some(below) = current.checked_sub(1) else { return Ok(()) };
        self.current = Some(below);
        self.menus[below].enabled = true;
        let menu = self.menu(below)?;
        set_enabled(menu, true)?;
        menu.goto(assets, "restore")
    }

    /// `SetMenu`: `Enable`, then `Init`, which sets a new menu's data up (`UpdateMenuData`) and
    /// shows it, and plays `start`.
    fn set_menu(&mut self, assets: &Assets, index: usize, view: &MenuView, enabled: bool) -> RenderResult<()> {
        self.menus[index].enabled = enabled;
        set_enabled(self.menu(index)?, enabled)?;
        if self.menus[index].menu != Some(view.id) {
            self.menus[index].menu = Some(view.id);
            self.update_menu_data(assets, index, view)?;
            self.menu(index)?.visible = true;
        }
        let menu = self.menu(index)?;
        menu.goto(assets, "start")?;
        tool_tip(menu, assets, false)
    }

    /// `Clear`: `eMenuID` -1, `clear`, disabled.
    fn clear(&mut self, assets: &Assets, index: usize) -> RenderResult<()> {
        self.menus[index].menu = None;
        self.menus[index].enabled = false;
        let menu = self.menu(index)?;
        menu.goto(assets, "clear")?;
        set_enabled(menu, false)?;
        tool_tip(menu, assets, false)
    }

    /// `Minimize`: disabled, `minimize`.
    fn minimize(&mut self, assets: &Assets, index: usize) -> RenderResult<()> {
        self.menus[index].enabled = false;
        let menu = self.menu(index)?;
        set_enabled(menu, false)?;
        tool_tip(menu, assets, false)?;
        menu.goto(assets, "minimize")
    }

    /// `UpdateMenuData`: the title and its minimized copy, the picker cleared and refilled with
    /// the items, the selection set, the picker's `Update`, then `UpdateMenuWindow`.
    fn update_menu_data(&mut self, assets: &Assets, index: usize, view: &MenuView) -> RenderResult<()> {
        let title = title_text(assets, &view.title)?;
        let menu = self.menu(index)?;
        let units: Vec<u16> = title.encode_utf16().collect();
        for field in ["mText", "mGlow"] {
            let label = menu.named(&["mMenu", "mTitle", field])?;
            label.text = Some(units.clone());
            edit_skater_movie::autosize_text(label, assets, &title)?;
        }
        menu.named(&["mMinimized", "txtTitle"])?.text = Some(units);
        self.render(assets, index, view)?;
        self.update_menu_window(assets, index)
    }

    /// menu_picker `SetValue` and `Update`: the items in the window disabled or deselected, then
    /// the selected one, its indicator beside it; `UpdateText` fills and auto-sizes them.
    fn render(&mut self, assets: &Assets, index: usize, view: &MenuView) -> RenderResult<()> {
        let texts = view.items.iter().map(|(title, _)| title_text(assets, title)).collect::<RenderResult<Vec<_>>>()?;
        let any_enabled = view.items.iter().any(|(_, enabled)| *enabled);
        let menu = self.menu(index)?;
        let picker = menu.named(&["mMenu", "mMenuPicker", "mMenuPicker"])?;
        for item in 0..OPTIONS {
            let clip = picker.named(&["mItemGroup", &format!("item{item}")])?;
            let Some(text) = texts.get(item) else {
                clip.visible = false;
                continue;
            };
            clip.visible = true;
            let enabled = view.items[item].1;
            clip.goto(assets, if item == view.selected { "selected" } else if enabled { "deselected" } else { "disabled" })?;
            let units: Vec<u16> = text.encode_utf16().collect();
            for path in [&["itemText"][..], &["mTextglow", "itemText"][..]] {
                let field = clip.named(path)?;
                field.text = Some(units.clone());
                edit_skater_movie::autosize_text(field, assets, text)?;
            }
        }
        // Update's Enable(any enabled item), then the indicator at the selected item.
        let indicator = child(picker, "mIndicator")?;
        indicator.clip.visible = any_enabled && view.selected < view.items.len();
        indicator.entry.matrix.ty = ITEM_OFFSET * view.selected as f32;
        self.menus[index].enabled = any_enabled;
        self.menus[index].shown = Some(view.clone());
        Ok(())
    }

    /// `UpdateMenuWindow`: the frame scaled about the widest of 160, the items and the title by
    /// 128, the plastic 20 larger than it with its smudges at its right, and the button help
    /// and loading wheel at its bottom right.
    fn update_menu_window(&mut self, assets: &Assets, index: usize) -> RenderResult<()> {
        let menu = self.menu(index)?;
        let picker = menu.named(&["mMenu", "mMenuPicker", "mMenuPicker"])?;
        let mut inner = MIN_INNER_WIDTH;
        for item in 0..OPTIONS {
            let entry = child(picker.named(&["mItemGroup"])?, &format!("item{item}"))?;
            if !entry.clip.visible {
                continue;
            }
            let field = child(&mut entry.clip, "itemText")?;
            inner = inner.max(field.entry.matrix.tx + text_field_width(&field.clip, assets)?);
        }
        let title = child(menu.named(&["mMenu", "mTitle"])?, "mText")?;
        inner = inner.max(text_field_width(&title.clip, assets)?);
        let background = menu.named(&["mMenu", "mBackground"])?;
        scale_dimensions(background.named(&["mNineSlice"])?, assets, inner, INNER_HEIGHT)?;
        let outer = bounds(background.named(&["mNineSlice"])?, assets)?.ok_or("hub menu frame has no bounds")?;
        let (outer_width, outer_height) = (outer.right - outer.left, outer.bottom - outer.top);
        let frame = child(menu.named(&["mMenu"])?, "mBackground")?;
        let scaled = bounds(&frame.clip, assets)?.ok_or("hub menu background has no bounds")?;
        let plastic_width = 20.0 + (scaled.right - scaled.left) * frame.entry.matrix.a;
        let plastic_height = 20.0 + (scaled.bottom - scaled.top) * frame.entry.matrix.d;
        let art = menu.named(&["mMenu", "mScalingArt"])?;
        let plastic = child(art, "mPlastic")?;
        let natural = bounds(&plastic.clip, assets)?.ok_or("hub menu plastic has no bounds")?;
        plastic.entry.matrix.a = plastic_width / (natural.right - natural.left);
        plastic.entry.matrix.d = plastic_height / (natural.bottom - natural.top);
        plastic.clip.script_transform = true;
        for smudge in ["mSmudge0", "mSmudge1"] {
            let smudge = child(art, smudge)?;
            smudge.entry.matrix.tx = plastic_width;
            smudge.clip.script_transform = true;
        }
        let parent = menu.named(&["mMenu"])?;
        let help = child(parent, "mButtonHelp")?;
        help.entry.matrix.tx = outer_width - 10.0;
        help.entry.matrix.ty = outer_height - 12.0;
        help.clip.script_transform = true;
        let (help_x, help_y) = (help.entry.matrix.tx, help.entry.matrix.ty);
        let loading = child(parent, "mLoading")?;
        loading.entry.matrix.tx = help_x - 11.0;
        loading.entry.matrix.ty = help_y - 45.0;
        loading.clip.script_transform = true;
        Ok(())
    }

    /// The popup: `GoToKickPlayer`'s (the warning title, `ID_MESSENGER_KICK_CONFIRM` of the
    /// player's name, Yes and No), the searching popup 38, the signing-in popup 33 (B cancels) or
    /// a notice, under `ID_EANATION` as their `gPopupTemplate` (0x82fdbc58) rows have them.
    fn update_popup(&mut self, assets: &Assets, popup: Option<&Popup>) -> RenderResult<()> {
        if popup == self.popup_shown.as_ref() {
            return Ok(());
        }
        match popup {
            Some(shown) => {
                let (title, description, options, selected): (&str, String, &[&str], usize) = match shown {
                    Popup::Kick(kick) => {
                        let text = String::from_utf16(&localizer::format_string_text(assets, "ID_MESSENGER_KICK_CONFIRM", std::slice::from_ref(&kick.name))?)?;
                        ("ID_COMMON_WARNING_TITLE", format!("#{text}"), &["ID_COMMON_YES", "ID_COMMON_NO"], kick.selected)
                    }
                    Popup::Matching => ("ID_EANATION", "ID_ONLINE_MATCHING_SKATERS".to_owned(), &[], 0),
                    Popup::SigningIn => ("ID_EANATION", "ID_ONLINE_SIGNING_IN".to_owned(), &[], 0),
                    Popup::Notice(text) => ("ID_EANATION", (*text).to_owned(), &["ID_COMMON_OK"], 0),
                };
                match self.popup.as_mut() {
                    // One closing opens again.
                    Some(clip) if self.popup_shown.is_none() => clip.goto(assets, "intro")?,
                    Some(_) => {}
                    None => {
                        let mut clip = Clip::new(assets, POPUP_ASSET, 0)?;
                        clip.goto(assets, "intro")?;
                        self.popup = Some(clip);
                    }
                }
                let clip = self.popup.as_mut().ok_or("hub popup absent")?;
                super::edit_skater_movie::configure_popup(clip, assets, selected, title, &description, options, matches!(shown, Popup::SigningIn))?;
            }
            None => {
                if let Some(clip) = self.popup.as_mut() {
                    clip.goto(assets, "outro")?;
                }
            }
        }
        self.popup_shown = popup.cloned();
        Ok(())
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if let Some(popup) = self.popup.as_mut() {
            popup.tick(assets)?;
            let mut calls = Vec::new();
            popup.take_calls(&mut calls);
            for call in calls {
                match call {
                    "CacGenderPopupIntroComplete" => {}
                    "CacGenderPopupOutroComplete" => self.popup = None,
                    _ => return Err(format!("untranslated kick popup callback {call}").into()),
                }
            }
        }
        if !self.root.visible {
            return Ok(());
        }
        self.root.tick(assets)?;
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "OnlineHubScreen" => {}
                "IntroComplete" => self.intro_complete = true,
                "OutroComplete" => self.root.visible = false,
                _ => return Err(format!("untranslated online hub callback {call}").into()),
            }
        }
        Ok(())
    }
}

pub(super) fn placed(clip: Clip, character: u32, name: &str, matrix: skate2_formats::apt::Matrix) -> Child {
    Child {
        entry: DisplayEntry {
            identity: PlacementIdentity { character, frame: 0 },
            matrix,
            colour_scale: u32::MAX,
            colour_bias: 0,
            ratio: 0.,
            name: Some(name.to_owned()),
            clip_depth: -1,
            events: Vec::new(),
            blend_mode: None,
        },
        clip,
    }
}

/// `hub_menu.Enable` (the picker's `Enable`): its highlight and indicator.
fn set_enabled(menu: &mut Clip, enabled: bool) -> RenderResult<()> {
    let picker = menu.named(&["mMenu", "mMenuPicker", "mMenuPicker"])?;
    child(picker, "mIndicator")?.clip.visible = enabled;
    Ok(())
}

/// `ShowToolTip`: `mToolTip` at `on` or `off`, shown or hidden.
fn tool_tip(menu: &mut Clip, assets: &Assets, shown: bool) -> RenderResult<()> {
    let tip = menu.named(&["mToolTip"])?;
    tip.goto(assets, if shown { "on" } else { "off" })?;
    tip.visible = shown;
    Ok(())
}

/// A hub string as the screen shows it: a HAL id localized, a `#` name, `"#%s (%d/%d)"` of a
/// packed string, or `"#\xBB %s"` of one, the fonts mapping 0xBB to the flash character.
fn title_text(assets: &Assets, title: &Title) -> RenderResult<String> {
    Ok(match title {
        Title::Hal(label) => assets.localize(label)?,
        // The data's own ids, as a text field shows one the strings lack.
        Title::Text(label) => String::from_utf16(&assets.initial_text(label))?,
        Title::Literal(text) => text.clone(),
        Title::Count { label, count, total } => format!("{} ({count}/{total})", assets.localize(label)?),
        Title::Completed(label) => format!("\u{bb} {}", String::from_utf16(&assets.initial_text(label))?),
    })
}

/// background_scaler `ScaleDimensions(width, height, true)`: the centre column and row scaled
/// to the inner size and every piece laid after its neighbour.
pub(super) fn scale_dimensions(slice: &mut Clip, assets: &Assets, width: f32, height: f32) -> RenderResult<()> {
    let natural = |slice: &mut Clip, name: &str| -> RenderResult<(f32, f32)> {
        let piece = child(slice, name)?;
        let r = bounds(&piece.clip, assets)?.ok_or("hub frame piece has no bounds")?;
        Ok((r.right - r.left, r.bottom - r.top))
    };
    let (width, height) = (width.max(0.0), height.max(0.0));
    for name in ["mTopcenter", "mCore", "mBottomcenter"] {
        let (w, _) = natural(slice, name)?;
        let piece = child(slice, name)?;
        piece.entry.matrix.a = width / w;
        piece.clip.script_transform = true;
    }
    for name in ["mCenterleft", "mCore", "mCenterright"] {
        let (_, h) = natural(slice, name)?;
        let piece = child(slice, name)?;
        piece.entry.matrix.d = height / h;
        piece.clip.script_transform = true;
    }
    let size = |slice: &mut Clip, name: &str| -> RenderResult<(f32, f32, f32, f32)> {
        let (w, h) = natural(slice, name)?;
        let m = child(slice, name)?.entry.matrix;
        Ok((m.tx, m.ty, w * m.a.abs(), h * m.d.abs()))
    };
    let place = |slice: &mut Clip, name: &str, x: Option<f32>, y: Option<f32>| -> RenderResult<()> {
        let piece = child(slice, name)?;
        if let Some(x) = x {
            piece.entry.matrix.tx = x;
        }
        if let Some(y) = y {
            piece.entry.matrix.ty = y;
        }
        piece.clip.script_transform = true;
        Ok(())
    };
    let (tl_x, tl_y, tl_w, tl_h) = size(slice, "mTopleft")?;
    place(slice, "mTopcenter", Some(tl_x + tl_w), None)?;
    let (tc_x, _, tc_w, _) = size(slice, "mTopcenter")?;
    place(slice, "mTopright", Some(tc_x + tc_w), None)?;
    place(slice, "mCenterleft", None, Some(tl_y + tl_h))?;
    let (_, cl_y, _, cl_h) = size(slice, "mCenterleft")?;
    place(slice, "mBottomleft", None, Some(cl_y + cl_h))?;
    place(slice, "mCore", Some(tl_x + tl_w), Some(tl_y + tl_h))?;
    let (core_x, core_y, core_w, core_h) = size(slice, "mCore")?;
    place(slice, "mCenterright", Some(core_x + core_w), Some(tl_y + tl_h))?;
    let (bl_x, _, bl_w, _) = size(slice, "mBottomleft")?;
    place(slice, "mBottomcenter", Some(bl_x + bl_w), Some(core_y + core_h))?;
    let (bc_x, _, bc_w, _) = size(slice, "mBottomcenter")?;
    let (_, cr_y, _, cr_h) = size(slice, "mCenterright")?;
    place(slice, "mBottomright", Some(bc_x + bc_w), Some(cr_y + cr_h))
}

/// The movies' InitActions: the screen's, hub_tree's and hub_menu's classes and registrations,
/// and background_scaler's.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!(
        (asset, character, offset),
        (HUB, 53, 0x266c)
            | (HUB, 54, 0x293c)
            | (HUB, 55, 0x2c5c)
            | (TREE, 54, 0x13a8)
            | (TREE, 55, 0x16c8)
            | (TREE, 53, 0x1fbc)
            | (MENU, 106, 0x63b0)
            | (MENU, 105, 0x6d40)
            | (SCALER, 53, 0x1300)
            | (SCALER, 52, 0x1dbc)
    )
}

/// The screen's frame actions besides `Stop`: its constructor, `IntroComplete` and
/// `OutroComplete`.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match (clip.asset, offset) {
        (HUB, 0x2644) => {
            clip.playback.stop();
            clip.calls.push("OnlineHubScreen");
        }
        (HUB, 0x35f4) => {
            clip.playback.stop();
            clip.calls.push("IntroComplete");
        }
        (HUB, 0x3648) => {
            clip.playback.stop();
            clip.calls.push("OutroComplete");
        }
        _ => return Err(format!("untranslated online hub APT action {}:{offset:#x}", clip.asset).into()),
    }
    Ok(())
}
