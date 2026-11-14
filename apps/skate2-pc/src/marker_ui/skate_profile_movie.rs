//! `fe\source\screens\onlinehub\skateProfile` (screen 93 `SkateProfile`, the flat template
//! `0x82fdb928`) as `FrontEndState_SkateProfile` (FE state 45) shows it offline, with the
//! `online_gamer_rank` control it imports. Root frame 0 constructs `skateProfile`: `mScreen.stop()`
//! and, `onlineprofile.IsOnlineMode()` being false, `SetState(k_iOFFLINE_MODE)`. `RegisterScreen`
//! reaches the state's `OnLoaded` (`0x82662000`), whose `SetState(10101)` populates the current tab
//! again; its `UpdateGamerComponents` and `InitMediaBrowsers` write the gamer tag, the rank and the
//! online thumbnails, which the offline layout keeps at alpha 0 (`mGamerTag`, `mRankInfo`) or
//! above the screen (`mContent.mDisplay`), as `UpdateInput` hides those thumbnails.
use super::{
    assets::Assets,
    clips::Clip,
    hud_leaderboard_movie::width,
    localizer,
    map_movie::{styled_button_items, update_scrollbar},
    objective_movie::{set_x, set_xscale},
    pause_movie::child,
    results_list::{ProfileList, ProfileListLayout, Value, auto_size, set_text_x},
};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::skate_profile::{STATS_TAB, StatValue, View};
use skate2_formats::localization::string_hash;

pub(super) const ASSET: usize = 114;
pub(super) const RANK: usize = 115;

/// `skateProfile.tabsAvailableWidth` and `tabsSpacing`.
const TABS_AVAILABLE_WIDTH: f32 = 694.;
const TABS_SPACING: f32 = 5.;
/// The offline `mFilterTabText`'s first two (`m_iMaxTabsOffline`); its third,
/// `ID_PROFILE_SINGLEPLAYER_TAB_AWARDS`, has no tab.
const TAB_TEXT: [&str; 2] = ["ID_PROFILE_SINGLEPLAYER_TAB_STATS", "ID_PROFILE_SINGLEPLAYER_TAB_PERSONALBESTS"];
/// `skateProfile.listRowHeight_SP` and `fontSize_SP`.
const LIST_ROW_HEIGHT: f32 = 24.;
const LIST_FONT_SIZE: &str = "small";
/// The rows `SetupOfflineDataTabListControl` shows at most.
const VISIBLE_ROWS: usize = 12;
/// `basic_slider`'s constructor `SetSliderLength(300)`, which the screen keeps.
const SLIDER_LENGTH: f32 = 300.;

/// `basic_slider` (`mScroller`, a `VerticalSliderComponent`): its value and window as its
/// `UpdateInput` steps them.
#[derive(Default)]
struct Slider {
    /// `mCurrentValue`, `mWindowedValue` and `mWindowLocalVal`.
    current: i32,
    windowed: i32,
    local: i32,
    /// `mNumItems` and `mNumVisibleRows`.
    items: i32,
    visible: i32,
    /// `mEnableGUI`.
    enabled: bool,
}

impl Slider {
    /// `SetNumberOfItems(numItems, numVisibleItems)`: the indicator's scale, which the next
    /// `UpdateDisplay` places.
    const fn set_number_of_items(&mut self, items: i32, visible: i32) {
        self.items = items;
        self.visible = visible;
    }

    /// `SetValue(n)`: the window moved to hold the value, then `UpdateDisplay`.
    const fn set_value(&mut self, value: i32) {
        self.current = value;
        if self.current < self.windowed {
            self.windowed = self.current;
        } else if self.current >= self.windowed + self.visible {
            self.windowed = self.current - self.visible + 1;
        }
        self.local = self.current - self.windowed;
        self.update_display();
    }

    /// `UpdateDisplay`: the arrows step while the items overflow the rows.
    const fn update_display(&mut self) {
        self.enabled = self.items > self.visible;
    }

    /// `NextValue`.
    const fn next_value(&mut self) {
        if !self.enabled {
            return;
        }
        self.current += 1;
        self.local += 1;
        if self.current > self.items - 1 {
            self.current = self.items - 1;
        }
        if self.local > self.visible - 1 {
            self.local = self.visible - 1;
            self.windowed += 1;
            if self.windowed > self.items - self.visible {
                self.windowed = self.items - self.visible;
            }
        }
        self.update_display();
    }

    /// `PrevValue`.
    const fn prev_value(&mut self) {
        if !self.enabled {
            return;
        }
        self.current -= 1;
        self.local -= 1;
        if self.current < 0 {
            self.current = 0;
        }
        if self.local < 0 {
            self.local = 0;
            self.windowed -= 1;
            if self.windowed < 0 {
                self.windowed = 0;
            }
        }
        self.update_display();
    }
}

#[derive(Default)]
pub(super) struct SkateProfileMovie {
    pub root: Option<Clip>,
    shown: Option<View>,
    pending: bool,
    /// The screen's `IntroComplete` since the last take.
    pub intro: bool,
    /// The screen's `OutroComplete`, which `DestroyScreen`s it.
    pub destroyed: bool,
    list: Option<ProfileList>,
    slider: Slider,
    /// `tabsVariableWidth`.
    tab_padding: f32,
}

impl SkateProfileMovie {
    pub const fn pending(&self) -> bool {
        self.pending
    }

    /// FE state 45 is on the stack: its `mbDimmer`, and the `StartHub`'s screen closed under it.
    pub const fn open(&self) -> bool {
        self.shown.is_some()
    }

    /// A push opens a new screen (`OpenScreen`), a pop plays the old one's outro (`CloseScreen`),
    /// and the state's input follows on the open screen.
    pub fn update(&mut self, assets: &Assets, state: Option<&View>) -> RenderResult<()> {
        let previous = std::mem::replace(&mut self.shown, state.cloned());
        match (previous, state) {
            (None, None) => {}
            (Some(old), Some(new)) if old.instance == new.instance => {
                if !self.pending && old != *new {
                    self.follow(assets, &old, new)?;
                }
            }
            (_, Some(_)) => {
                self.pending = true;
                self.intro = false;
            }
            (Some(_), None) => {
                if !std::mem::take(&mut self.pending)
                    && let Some(root) = self.root.as_mut()
                {
                    root.goto(assets, "outro")?;
                }
            }
        }
        Ok(())
    }

    /// The linked screen: root frame 0's `skateProfile`, `OnLoaded`'s `SetState(10101)`, then
    /// the screen's `intro`.
    pub fn link(&mut self, assets: &Assets) -> RenderResult<()> {
        if !std::mem::take(&mut self.pending) {
            return Ok(());
        }
        let view = self.shown.clone().ok_or("skate.Profile screen without its state")?;
        let mut root = Clip::new(assets, ASSET, 0)?;
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        if !calls.contains(&"skateProfile") {
            return Err("skateProfile root constructed no screen".into());
        }
        // The list's and the slider's constructors run as their clips are placed.
        let display = root.named(&["mProfile", "mContent", "mDisplay_sp"])?;
        self.list = Some(ProfileList::new(display.named(&["mList"])?, assets)?);
        construct_slider(display.named(&["mScroller"])?, assets)?;
        self.slider = Slider::default();
        // skateProfile: m_iCurrentTab = 0 and mScreen.stop().
        root.playback.stop();
        self.root = Some(root);
        self.destroyed = false;
        self.set_offline_state(assets, &view)?;
        self.populate_screen(assets, &view)?;
        self.root_mut()?.goto(assets, "intro")
    }

    /// One front-end update: the screen's `IntroComplete`, and its `OutroComplete`.
    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        let Some(root) = self.root.as_mut() else { return Ok(()) };
        root.tick(assets)?;
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        for call in calls {
            match call {
                "IntroComplete" => self.intro = true,
                "OutroComplete" => {
                    self.root = None;
                    self.destroyed = true;
                }
                _ => return Err(format!("untranslated skate.Profile callback {call}").into()),
            }
        }
        Ok(())
    }

    fn root_mut(&mut self) -> RenderResult<&mut Clip> {
        self.root.as_mut().ok_or_else(|| "skate.Profile screen absent".into())
    }

    /// `SetState(k_iOFFLINE_MODE)`: the third and fourth tabs hidden, `mProfile` and `mContent` on
    /// `sp`, the list's style, the title, the tabs spaced over the width left by their labels,
    /// then `SetFilterTab(0)`.
    fn set_offline_state(&mut self, assets: &Assets, view: &View) -> RenderResult<()> {
        let Self { root: Some(root), list: Some(list), .. } = self else {
            return Err("skate.Profile screen absent".into());
        };
        let profile = root.named(&["mProfile"])?;
        let tabs = profile.named(&["mFilterTabs"])?;
        for name in ["mFilterTab2", "mFilterTab3"] {
            tabs.named(&[name])?.visible = false;
        }
        profile.goto(assets, "sp")?;
        let content = profile.named(&["mContent"])?;
        content.goto(assets, "sp")?;
        // mDisplay_sp.gotoAndPlay(0): `_gotoAndX` ignores the number 0.
        list.set_style(content.named(&["mDisplay_sp", "mList"])?, assets, LIST_ROW_HEIGHT, LIST_FONT_SIZE)?;
        let title = profile.named(&["mTitle"])?;
        for name in ["txtTitle", "txtGlow"] {
            title.named(&[name])?.text = Some(assets.initial_text("ID_ONLINE_SKATEPROFILE_CAREER"));
        }
        let tabs = profile.named(&["mFilterTabs"])?;
        let mut label_widths = 0.;
        for (index, text) in TAB_TEXT.iter().enumerate() {
            let label = child(&mut child(tabs, &format!("mFilterTab{index}"))?.clip, "labelText")?;
            label.clip.text = Some(assets.initial_text(text));
            auto_size(label, assets, "left")?;
            label_widths += width(label, assets)?.ceil();
        }
        #[expect(clippy::cast_precision_loss, reason = "ActionScript numbers")]
        let count = TAB_TEXT.len() as f32;
        let mut padding = (TABS_AVAILABLE_WIDTH - 16. * count - TABS_SPACING * (count - 1.) - label_widths).floor() / (2. * count);
        if padding < 0. {
            padding = 0.;
        }
        for index in 1..TAB_TEXT.len() {
            let previous = child(tabs, &format!("mFilterTab{}", index - 1))?;
            let x = previous.entry.matrix.tx;
            let label_width = width(child(&mut previous.clip, "labelText")?, assets)?;
            set_x(child(tabs, &format!("mFilterTab{index}"))?, x + (2. * padding + label_width) + TABS_SPACING + 16.);
        }
        self.tab_padding = padding;
        self.set_filter_tab(assets, view)
    }

    /// `SetFilterTab(index)`: the tab on `selected_sp` with `mDisplay_sp` on its frame and
    /// `SetButtonHelp(false)`, the other on `unselected_sp`, each through `SizeFilterTab`, then
    /// `PopulateScreen(index)`.
    fn set_filter_tab(&mut self, assets: &Assets, view: &View) -> RenderResult<()> {
        let tab = usize::try_from(view.tab)?;
        let padding = self.tab_padding;
        let profile = self.root_mut()?.named(&["mProfile"])?;
        for (index, text) in TAB_TEXT.iter().enumerate() {
            if index == tab {
                profile.named(&["mContent", "mDisplay_sp"])?.goto(assets, &index.to_string())?;
                profile.named(&["mFilterTabs", &format!("mFilterTab{index}")])?.goto(assets, "selected_sp")?;
                // SetButtonHelp(false): SetInfo("Marker_streak", 20, 0xfffeec) and B. The Xbox 360's
                // bumper items open Xbox LIVE's gamercard and achievements, which the PC lacks.
                styled_button_items(profile.named(&["mButtonHelp"])?, assets, &[("B", "ID_COMMON_BACK")], 0xffff_feec, false, "Marker_streak", 20.)?;
            } else {
                profile.named(&["mFilterTabs", &format!("mFilterTab{index}")])?.goto(assets, "unselected_sp")?;
            }
            size_filter_tab(profile.named(&["mFilterTabs", &format!("mFilterTab{index}")])?, assets, text, padding)?;
        }
        self.populate_screen(assets, view)
    }

    /// `PopulateScreen(index)`: `PopulateUntabbedData`, then the Stats tab's
    /// `PopulateOfflineStatsTab` or the Personal Bests tab's `PopulateOfflinePersonalTab`, each
    /// `SetupOfflineDataTabListControl`: the titles and values, two text boxes 598 and 90 wide
    /// (left and right), at most twelve rows from the first, and the slider at the top.
    fn populate_screen(&mut self, assets: &Assets, view: &View) -> RenderResult<()> {
        let Self { root: Some(root), list: Some(list), slider, .. } = self else {
            return Err("skate.Profile screen absent".into());
        };
        let profile = root.named(&["mProfile"])?;
        // PopulateUntabbedData: mc_gamer_stats on sp, which keeps both panels at alpha 0, and
        // Stats.GetTotalCashEarned. Its visits text counts the online free-skate activities.
        let stats = profile.named(&["mc_gamer_stats"])?;
        stats.goto(assets, "sp")?;
        let cash = value_text(assets, view.lists.total_cash)?;
        stats.named(&["mc_gamer_stats_online", "txt_total_cash"])?.text = Some(assets.initial_text(&cash));
        let rows = if view.tab == STATS_TAB { &view.lists.stats } else { &view.lists.personal_bests };
        let data = rows.iter()
            .map(|row| Ok(vec![Value::Text(row.title.to_owned()), Value::Text(value_text(assets, row.value)?)]))
            .collect::<RenderResult<Vec<_>>>()?;
        let visible = rows.len().min(VISIBLE_ROWS);
        let layout = ProfileListLayout {
            definition: vec!["LIST_TEXTBOX", "LIST_TEXTBOX"],
            alignments: vec!["left", "right"],
            widths: vec![598., 90.],
            rows: i32::try_from(visible)?,
        };
        // SetColumnGapPixels(10) names the online display's list: this one keeps its gap.
        let display = profile.named(&["mContent", "mDisplay_sp"])?;
        let list_clip = display.named(&["mList"])?;
        list.set_data(list_clip, assets, layout, data)?;
        list_clip.visible = true;
        slider.set_number_of_items(i32::try_from(rows.len())?, i32::try_from(visible)?);
        slider.set_value(0);
        update_scrollbar(display.named(&["mScroller"])?, assets, rows.len(), visible, usize::try_from(slider.windowed)?, SLIDER_LENGTH)
    }

    /// `UpdateInput`: a released trigger's `NextTab` or `PreviousTab` (`RefreshTabs`), else the
    /// list's and the slider's `UpdateInput` a row on.
    fn follow(&mut self, assets: &Assets, old: &View, new: &View) -> RenderResult<()> {
        if new.tab != old.tab {
            return self.set_filter_tab(assets, new);
        }
        let Self { root: Some(root), list: Some(list), slider, .. } = self else {
            return Err("skate.Profile screen absent".into());
        };
        let display = root.named(&["mProfile", "mContent", "mDisplay_sp"])?;
        let rows = new.rows().len();
        while list.current() != new.selected {
            let (from, forward) = (list.current(), list.current() < new.selected);
            list.step(display.named(&["mList"])?, assets, forward)?;
            if forward { slider.next_value() } else { slider.prev_value() }
            if list.current() == from {
                return Err("skate.Profile row past its list".into());
            }
        }
        update_scrollbar(display.named(&["mScroller"])?, assets, rows, rows.min(VISIBLE_ROWS), usize::try_from(slider.windowed)?, SLIDER_LENGTH)
    }
}

/// `SizeFilterTab(index)`: the label's text auto-sized from the left, the backer as wide as the
/// label and twice the tab padding, the label the padding in.
fn size_filter_tab(tab: &mut Clip, assets: &Assets, text: &str, padding: f32) -> RenderResult<()> {
    let label = child(tab, "labelText")?;
    label.clip.text = Some(assets.initial_text(text));
    auto_size(label, assets, "left")?;
    let backer = 2. * padding + width(label, assets)?;
    set_text_x(label, padding + 8.);
    set_xscale(child(tab, "backer_fill")?, 100. * backer / 32.);
    set_x(child(tab, "backer_corner_right")?, backer + 16.);
    Ok(())
}

/// `basic_slider`'s constructor: `SetSliderLength(300)`, whose geometry `update_scrollbar`
/// lays out, and `SetSkin(mSkin)`.
pub(super) fn construct_slider(slider: &mut Clip, assets: &Assets) -> RenderResult<()> {
    let Some(skin) = slider.properties.get("mSkin").cloned() else { return Ok(()) };
    let bar = slider.named(&["mcScrollbar"])?;
    for path in [
        &["mArrowTop"][..],
        &["mArrowBottom"],
        &["mcEdge"],
        &["mGak"],
        &["mSliderIndicator", "mIndSolid", "mBarTop"],
        &["mSliderIndicator", "mIndSolid", "mBarMiddle"],
        &["mSliderIndicator", "mIndSolid", "mBarBottom"],
        &["mSliderIndicator", "mIndGlow", "mGlowTop"],
        &["mSliderIndicator", "mIndGlow", "mGlowMiddle"],
        &["mSliderIndicator", "mIndGlow", "mGlowBottom"],
    ] {
        bar.named(path)?.goto_and_stop(assets, &skin)?;
    }
    Ok(())
}

/// `Stats.GetStatValueText` and `GetPersonalBestValueText` (`GetStatsValue` `0x82568270`,
/// `GetPersonalBestsValue` `0x8255f1d8`) as the text boxes take them: the backend's
/// `#`-prefixed formatting, or `ID_COMMON_HYPHEN` itself, which the text box localizes.
fn value_text(assets: &Assets, value: StatValue) -> RenderResult<String> {
    let literal = |units: Vec<u16>| -> RenderResult<String> { Ok(format!("#{}", String::from_utf16(&units)?)) };
    match value {
        // FormatTime(.., true, 0), a space and the packed ID_COMMON_METRIC_TIME.
        StatValue::Time { hours, minutes, seconds, hundredths } => {
            let unit = String::from_utf16(&localizer::packed_string(assets, b"ID_COMMON_METRIC_TIME"))?;
            Ok(format!("#{} {unit}", localizer::format_time(assets, hours, minutes, seconds, hundredths, true, 0)))
        }
        StatValue::Number(value) => Ok(format!("#{}", localizer::format_float(assets, value, 0))),
        StatValue::Currency(value) => Ok(format!("#{}", localizer::format_currency(assets, value))),
        StatValue::Percent(value) => literal(localizer::format_string_float(assets, "ID_LOGBOOK_STATS_PERCENT_INT", value)?),
        // GetMetricText: Snprintf(16) of %d or %.1f, then FormatStringCustom(ID_COMMON_DISTANCE).
        StatValue::DistanceInt(value) => literal(localizer::format_custom(assets, "ID_COMMON_DISTANCE", &value.to_string())?),
        StatValue::DistanceFloat(value) => {
            let mut text = format!("{:.1}", f64::from(value));
            text.truncate(15);
            literal(localizer::format_custom(assets, "ID_COMMON_DISTANCE", &text)?)
        }
        StatValue::Label(label) => literal(localizer::format_hal(assets, string_hash(label.as_bytes()))?),
        StatValue::Hal(label) => Ok(label.to_owned()),
    }
}

/// The root's timeline actions besides `Stop`: frame 0 constructs the class; the intro's first
/// frame and the outro's ask `ScreenManager.EnableDimmer`, which screenmanager.apt never defines;
/// the intro's last frame reports `IntroComplete` and the outro's `OutroComplete`.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match (clip.asset, clip.character, offset) {
        (ASSET, 0, 0x85d4) => clip.calls.push("skateProfile"),
        (ASSET, 0, 0xa78c | 0xa7cc) => {}
        (ASSET, 0, 0xa7a8) => {
            clip.playback.stop();
            clip.calls.push("IntroComplete");
        }
        (ASSET, 0, 0xa7e8) => {
            clip.playback.stop();
            clip.calls.push("OutroComplete");
        }
        _ => return Err(format!("untranslated skate.Profile APT action {}:{} {offset:#x}", clip.asset, clip.character).into()),
    }
    Ok(())
}

/// The screen's `InitAction` defines the guarded global class translated above; the rank
/// control's define and register `online_gamer_rank` on its own root, which no screen places.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 155, 0x85f4))
}
