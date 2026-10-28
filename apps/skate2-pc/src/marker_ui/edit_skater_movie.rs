//! Original createskater APT and imported highlight controls, fingerprinted in Assets.
//! Game.CAC_* values arrive from the native controller/catalog; no generated menu art.
use super::{
    assets::Assets,
    clips::{Child, Clip},
    localizer,
    pause_movie::{bounds, child},
};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::{
    apt_timeline::{DisplayEntry, IDENTITY, PlacementIdentity},
    edit_skater::{EditSkaterRow, EditSkaterView},
};

pub(super) struct EditSkaterMovie {
    pub root: Clip,
    pub letterbox: Clip,
    /// ScreenManager mounts sk8popup independently of createskater. It must
    /// not inherit the card's safe-area translation or its timeline fades.
    pub popup: Option<Clip>,
    pub intro_complete: bool,
    pub outro_complete: bool,
    pub sounds: Vec<&'static str>,
    previous: Option<EditSkaterView>,
}
impl EditSkaterMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, 15, 0)?;
        root.visible = false;
        root.playback.stop();
        let mut letterbox = Clip::new(assets, 11, 0)?;
        letterbox.visible = false;
        letterbox.playback.stop();
        Ok(Self {
            root,
            letterbox,
            popup: None,
            intro_complete: false,
            outro_complete: true,
            sounds: Vec::new(),
            previous: None,
        })
    }
    pub fn update(&mut self, assets: &Assets, view: &EditSkaterView) -> RenderResult<()> {
        self.sounds.clear();
        let was_visible = self.previous.as_ref().is_some_and(|p| p.visible);
        if was_visible != view.visible {
            self.intro_complete = false;
            self.outro_complete = false;
            if view.visible {
                self.root = Clip::new(assets, 15, 0)?;
                self.root.goto(assets, "intro")?;
                // createskater ctor00e5c0..627, Screen_EdgeOffset==0.
                // FrontEndManager::SetScreenSize82614300 leaves it zero on
                // widescreen; the PC UI retains the same 1280x720 canvas.
                let done = self.root.named(&["mDone"])?;
                for name in ["mBack", "mButtonhelp", "mStartbutton"] {
                    child(done, name)?.entry.matrix.tx += 272.;
                }
                // createskater ctor00e590..596: LetterBox.EnableLetterBox(false).
                // The filmstrip belongs to other frontend screens; drawing it
                // here covers the lower card and the Select/Back button help.
                self.letterbox.visible = false;
            } else {
                self.root.goto(assets, "outro")?;
            }
        }
        if view.visible && self.previous.as_ref() != Some(view) {
            self.configure(assets, view)?;
        }
        self.popups(assets, view)?;
        self.previous = Some(view.clone());
        self.callbacks()
    }
    pub fn screen_transform(&self) -> skate2_formats::apt::Matrix {
        let mut matrix = IDENTITY;
        matrix.tx = -136.;
        matrix
    }
    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if self.root.visible {
            self.root.tick(assets)?;
        }
        if self.letterbox.visible {
            self.letterbox.tick(assets)?;
        }
        if let Some(popup) = &mut self.popup { popup.tick(assets)?; }
        self.callbacks()
    }
    fn callbacks(&mut self) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        self.letterbox.take_calls(&mut calls);
        if let Some(popup) = &mut self.popup { popup.take_calls(&mut calls); }
        for c in calls {
            match c {
                "CacIntroComplete" => self.intro_complete = true,
                "CacOutroComplete" => {
                    self.outro_complete = true;
                    self.root.visible = false;
                    self.letterbox.visible = false;
                }
                "LetterIntroComplete" | "LetterOutroComplete" | "CacGenderPopupIntroComplete" => {}
                "CacGenderPopupOutroComplete" => {
                    self.popup = None;
                }
                "FE_SOUND_PAGETITLE_FLICKER"
                | "FE_SOUND_LETTERBOX_IN"
                | "FE_SOUND_LETTERBOX_OUT" => self.sounds.push(c),
                _ => return Err(format!("untranslated CAC callback {c}").into()),
            }
        }
        Ok(())
    }
    fn configure(&mut self, assets: &Assets, view: &EditSkaterView) -> RenderResult<()> {
        let mode = view.menu.movie_mode();
        self.root.named(&["mWallet"])?.visible = view.menu.banner() == "slappys";
        let changed = self
            .previous
            .as_ref()
            .is_none_or(|p| !p.visible || p.menu != view.menu);
        if changed {
            let banner = view.menu.banner();
            self.root.named(&["mLogo"])?.visible = banner != "none";
            self.root.named(&["mTitlepanel"])?.goto(
                assets,
                if banner == "petes" {
                    "petes"
                } else {
                    "slappys"
                },
            )?;
            if banner != "none" {
                self.root.named(&["mLogo"])?.goto(assets, banner)?;
            }
            self.root.named(&["mSelection"])?.goto(assets, mode)?;
            self.root.named(&["mSelection", "mScrollbar"])?.properties
                .insert("cac_window_start".into(), "0".into());
        }
        // GetFilterText82642c88 returns '#'. UpdateFilterText00f044..f097
        // clears the label and only plays 'off' when transitioning from on.
        // Its initial state is already off; replaying that fade exposes the
        // authoring placeholder every time a slider or recipe value changes.
        text(self.root.named(&["mPanel", "mFilterTab", "mFiltertext", "labeltext"])?, "");
        for name in ["labeltext", "labeltext2"] {
            text(
                self.root.named(&["mTitlepanel", "mTitletext", name])?,
                &view.title,
            );
        }
        // UpdateWallet: CAC_GetReceiptBalance's FormatCurrency.
        text(
            self.root.named(&["mWallet", "mCash", "LABELTEXT"])?,
            &localizer::format_currency(assets, view.wallet as f32),
        );
        text(
            self.root
                .named(&["mWallet", "mBalanceTitle", "LABELTEXT"])?,
            &assets.localize("ID_CAC_WALLET_TITLE")?,
        );
        // mBack is the authored animated Finished chevrons, not a Back label.
        // createskater never hides it; retain its native timeline visibility.
        buttons(
            self.root.named(&["mDone", "mButtonhelp", "mButtonmenu"])?,
            assets,
            &[("B", &view.help_b), ("A", &view.help_a)],
            "Marker_streak", 24., 0xffff_ffec,
        )?;
        buttons(
            self.root.named(&["mDone", "mStartbutton"])?,
            assets,
            &[("Start", &view.help_start)],
            "HelveticaPresto", 28., 0xff00_0000 | 12047872,
        )?;
        match mode {
            "thumbs" => self.thumbnails(assets, view)?,
            "list" => self.list(assets, view)?,
            "morph" => self.morph(assets, view)?,
            "style" => self.style(assets, view)?,
            _ => return Err("unknown original CAC movie mode".into()),
        }
        // UpdateInfoPanel: rows expose the native description; no synthetic labels.
        let panel = self
            .root
            .named(&["mSelection", "mInfopanelAnim", "mInfopanel"])?;
        let panel_type = if view.menu.banner() == "petes" {
            "petes"
        } else if view.menu.banner() == "none" {
            "blurb"
        } else if matches!(view.menu, skate2_core::ui::edit_skater::EditSkaterMenu::TattooThumbnails
            | skate2_core::ui::edit_skater::EditSkaterMenu::GraphicThumbnails) {
            "blurb"
        } else if mode == "thumbs" {
            view.rows
                .get(view.selected)
                .map_or("blurb", |r| r.status.as_str())
        } else {
            "noblurb"
        };
        panel.goto(assets, panel_type)?;
        if let Some(row) = view.rows.get(view.selected) {
            if panel_type == "blurb" {
                text(panel.named(&["mBlurb", "LABELTEXT"])?, &row.description);
            } else if !matches!(panel_type, "petes" | "noblurb") {
                // UpdateInfoPanel00f6b4..f847 tries big, small, then wrap.
                // SkateAptString82a70b28 publishes its wrapped line count as
                // nMaxScroll; the APT tests >1, not the field's pixel height.
                for size in ["big", "small", "wrap"] {
                    panel.named(&["mSubtitle"])?.goto(assets, size)?;
                    let field = panel.named(&["mSubtitle", "LABELTEXT"])?;
                    text(field, &row.description);
                    if text_line_count(field, assets)? <= 1 { break; }
                }
                // 00f70c..f7ce: a locked part reads ID_CAC_FILTER_LOCKED, an owned, DLC or
                // receipt part nothing, any other its CAC_GetItemCost.
                let info = match panel_type {
                    "locked" => assets.localize("ID_CAC_FILTER_LOCKED")?,
                    "owned" | "dlc" | "purchased" => String::new(),
                    _ => localizer::format_currency(assets, row.cost as f32),
                };
                for name in ["LABELTEXT", "LABELTEXT_GLOW"] {
                    text(panel.named(&["mInfo", name])?, &info);
                }
            }
        }
        // The blurb, noblurb and petes panels DestroySponsorLogo; a part's panel ends in
        // UpdateSponsorLogo, the logo of CAC_GetSponsorLogoFileName beside the part's name.
        let logo = view.rows.get(view.selected).and_then(|row| row.sponsor_logo.clone())
            .filter(|_| !matches!(panel_type, "blurb" | "noblurb" | "petes"));
        let image = panel.named(&["mImage"])?;
        image.visible = logo.is_some();
        match logo {
            Some(key) => {
                image.properties.insert("cac_texture".into(), key);
            }
            None => {
                image.properties.remove("cac_texture");
            }
        }
        Ok(())
    }
    /// The editor's sk8popups, none of them cancellable (`sPopupInfo` `0x82d67a80`): the
    /// GenderSwitch confirmation, `ID_CAC_NOT_ENOUGH_CASH` (`OnThumbnailSelect`
    /// `0x8268b0d4..0x8268b130`: one option, OK) and `Finish`'s exit confirmation.
    fn popups(&mut self, assets: &Assets, view: &EditSkaterView) -> RenderResult<()> {
        let popup = |v: &EditSkaterView| {
            v.gender_confirmation
                .map(|s| (s, 0))
                .or(v.cash_popup.then_some((0, 1)))
                .or(v.exit_confirmation.map(|s| (s, 2)))
        };
        let selection = popup(view);
        let old = self.previous.as_ref().and_then(popup);
        // Refresh is an event, not a per-render-frame rebuild of text/items.
        if selection == old { return Ok(()); }
        if selection.is_none() {
            if old.is_some() {
                if let Some(popup) = &mut self.popup {
                    popup.goto(assets, "outro")?;
                }
            }
            return Ok(());
        }
        if old.is_none() {
            let mut popup = Clip::new(assets, 25, 0)?;
            popup.goto(assets, "intro")?;
            self.popup = Some(popup);
        }
        let popup = self.popup.as_mut().ok_or("CAC popup missing")?;
        match selection {
            Some((_, 1)) => configure_popup(popup,assets,0,"ID_CAC_NOT_ENOUGH_CASH_TITLE",
                "ID_CAC_NOT_ENOUGH_CASH_DESC",&["ID_COMMON_OK"],false),
            Some((selected, 2)) => configure_popup(popup,assets,selected,"ID_COMMON_WARNING_TITLE",
                "ID_CAC_EXIT_CAC_CONFIRMATION",&["ID_COMMON_YES","ID_COMMON_NO"],false),
            selection => configure_popup(popup,assets,selection.map_or(0, |(s, _)| s),"ID_COMMON_WARNING_TITLE",
                "ID_CAC_GENDERSWITCH_CONFIRMATION",&["ID_COMMON_YES","ID_COMMON_NO"],false),
        }
    }

    fn list(&mut self, assets: &Assets, view: &EditSkaterView) -> RenderResult<()> {
        let first = self.scrollbar(assets, view.rows.len(), 11, view.selected)?;
        let picker = self
            .root
            .named(&["mSelection", "mMenuPicker", "mMenuPicker"])?;
        let height = bounds(picker.named(&["dummy0"])?, assets)?.ok_or("CAC picker bounds")?;
        let height = height.bottom - height.top;
        for i in 0..8 {
            picker.named(&[&format!("dummy{i}")])?.visible = false;
        }
        let group = picker.named(&["mItemGroup"])?;
        group.children.clear();
        for (i, row) in view.rows.iter().enumerate().skip(first).take(11) {
            let mut clip = Clip::new(assets, 6, 31)?;
            clip.goto(
                assets,
                if i == view.selected {
                    "selected"
                } else {
                    "deselected"
                },
            )?;
            text(clip.named(&["itemText"])?, &row.title);
            text(clip.named(&["mTextglow", "itemText"])?, &row.title);
            attach(group, i, clip, 0., (i - first) as f32 * height);
        }
        child(picker, "mIndicator")?.entry.matrix.ty = (view.selected - first) as f32 * height;
        Ok(())
    }
    fn thumbnails(&mut self, assets: &Assets, view: &EditSkaterView) -> RenderResult<()> {
        // InitThumbBrowser00f242..f255 computes floor((items-1)/3)+1
        // into register3 BEFORE SetNumberOfItems00f2b1. Both the total
        // and selected value are rows, not individual thumbnails.
        self.scrollbar(assets, view.rows.len().div_ceil(3), 3, view.selected / 3)?;
        let thumbs = self.root.named(&["mSelection", "mThumbnails"])?;
        for i in 0..9 {
            let thumb = thumbs.named(&[&format!("mThumb{i}")])?;
            let index = view.window_start + i;
            thumb.visible = index < view.rows.len();
            if let Some(row) = view.rows.get(index) {
                thumb.goto(
                    assets,
                    if index == view.selected {
                        "selected"
                    } else {
                        "unselected"
                    },
                )?;
                thumb.named(&["mIcon"])?.goto(
                    assets,
                    if row.status.is_empty() {
                        "none"
                    } else {
                        &row.status
                    },
                )?;
                thumb.named(&["mNewIcon"])?.visible = row.is_new;
                let image = thumb.named(&["mImageComponent"])?;
                image.visible = row.texture.is_some();
                if let Some(texture) = &row.texture {
                    image
                        .properties
                        .insert("cac_texture".into(), texture.clone());
                } else {
                    image.properties.remove("cac_texture");
                }
            }
        }
        Ok(())
    }
    fn scrollbar(&mut self, assets: &Assets, count: usize, visible: usize,
        selected: usize) -> RenderResult<usize> {
        let slider = self.root.named(&["mSelection", "mScrollbar"])?;
        let mut start = slider.properties.get("cac_window_start")
            .map(|s|s.parse::<usize>()).transpose()?.unwrap_or(0);
        // basic_slider.SetValue001840..18a1 retains its window while moving
        // inside it; only a selection outside the window scrolls the bar.
        if selected < start { start = selected; }
        else if selected >= start + visible { start = selected + 1 - visible; }
        slider.properties.insert("cac_window_start".into(), start.to_string());
        super::map_movie::update_scrollbar(slider, assets, count, visible, start, 320.)?;
        Ok(start)
    }
    fn morph(&mut self, assets: &Assets, view: &EditSkaterView) -> RenderResult<()> {
        let menu = self.root.named(&["mSelection", "mMorphmenu"])?;
        let start = child(menu, "dummy0")?.entry.matrix;
        for i in 0..3 {
            menu.named(&[&format!("dummy{i}")])?.visible = false;
        }
        menu.children.retain(|_, c| {
            c.entry
                .name
                .as_deref()
                .is_some_and(|n| n.starts_with("dummy"))
        });
        let mut y = start.ty;
        for (i, row) in view.rows.iter().enumerate() {
            let (asset, id, height) = match row.kind.as_str() {
                "option" => (20, 36, 36.),
                "category" => (19, 18, 36.),
                "slider" => (21, 22, 72.),
                "selector" => (18, 21, 58.),
                "morph" => (16, MORPH, 45.),
                "color" => (17, 33, 58.),
                _ => return Err(format!("unknown CAC highlight type {}", row.kind).into()),
            };
            let mut clip = Clip::new(assets, asset, id)?;
            highlight(&mut clip, assets, row, i == view.selected)?;
            attach(menu, i + 100, clip, start.tx, y);
            y += height + 6.;
        }
        Ok(())
    }
    fn style(&mut self, assets: &Assets, view: &EditSkaterView) -> RenderResult<()> {
        let menu = self.root.named(&["mSelection", "mStylemenu"])?;
        text(
            menu.named(&["mGestureTitle", "LABELTEXT"])?,
            &assets.localize("ID_CAC_GESTURES_TITLE")?,
        );
        for i in 0..6 {
            let item = menu.named(&[&format!("mItem{i}")])?;
            item.visible = i < view.rows.len();
            if let Some(row) = view.rows.get(i) {
                highlight(item, assets, row, i == view.selected)?;
            }
        }
        for i in 0..4 {
            child(menu, &format!("mButton{i}"))?.entry.colour_scale = if i == view.selected {
                u32::MAX
            } else {
                0x7fffffff
            };
        }
        Ok(())
    }
}
fn text(clip: &mut Clip, s: &str) {
    clip.text = Some(s.encode_utf16().collect());
}
fn text_line_count(clip: &Clip, assets: &Assets) -> RenderResult<usize> {
    let skate2_formats::apt::Character::Text(def) = clip.definition(assets)? else {
        return Err("CAC subtitle is not text".into());
    };
    let Some(skate2_formats::apt::Character::Font { name, .. }) =
        assets.movies[clip.asset].movie.characters.get(def.font_id as usize).and_then(Option::as_ref)
    else { return Err("CAC subtitle font absent".into()); };
    let font = assets.apt_font(name)?;
    Ok(super::native_text_lines(clip.text.as_deref().unwrap_or_default(), def.multiline,
        def.bounds.right - def.bounds.left, |c|font.metrics.mapped_glyph(c)
            .map_or(0., |g|g.advance * font.scale[0] * def.height)).len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};

    #[test]
    #[ignore = "requires external SKATE2_GAME_ROOT"]
    fn original_cac_style_title_arrows_and_button_layout() {
        let game = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
        let mut mounts = MountSet::new();
        for spec in SKATE2_ARCHIVES.iter() {
            mounts.push(MountedBig::open(spec.path_under(&game), spec.use_external_header).unwrap());
        }
        let assets = Assets::load(&mounts).unwrap();
        // Exercise every authored editor/control/popup frame, including class
        // registration delivered through InitAction rather than Action.
        for asset in 15..=25 {
            for (id, character) in assets.movies[asset].movie.characters.iter().enumerate() {
                let Some(timeline) = character.as_ref().and_then(|c| c.movie()) else { continue; };
                // Linkage-only class symbols have no display timeline.
                if timeline.frames.is_empty() { continue; }
                let mut clip = Clip::new(&assets, asset, id as u32)
                    .unwrap_or_else(|e| panic!("editor asset {asset}:{id}: {e}"));
                for frame in 1..=timeline.frames.len() {
                    clip.goto_number(&assets, frame)
                        .unwrap_or_else(|e| panic!("editor frame {asset}:{id}:{frame}: {e}"));
                }
            }
        }
        assert_eq!(assets.localize("ID_CAC_HAIRSTYLE_MOOSED_UP").unwrap(), "Moussed Up");
        let mut selector = Clip::new(&assets, 18, 21).unwrap();
        for value in ["Regular", "Goofy"] {
            let row = EditSkaterRow { kind: "selector".into(), title: "Stance".into(),
                subtitle: value.into(), ..Default::default() };
            highlight(&mut selector, &assets, &row, true).unwrap();
            let subtitle_x = child(&mut selector, "mSubtitle").unwrap().entry.matrix.tx;
            let width: f32 = selector.named(&["mSubtitle", "TEXTLABEL"]).unwrap()
                .properties["text_width"].parse().unwrap();
            assert_eq!(child(&mut selector, "mArrowright").unwrap().entry.matrix.tx, subtitle_x + width);
        }
        let mut clip = Clip::new(&assets, 19, 18).unwrap();
        let mut row = EditSkaterRow { kind: "category".into(), title: "Style".into(),
            subtitle: assets.localize("ID_CAC_WEAR_STYLE_OPTION_1").unwrap(), ..Default::default() };
        highlight(&mut clip, &assets, &row, true).unwrap();
        assert_eq!(String::from_utf16(clip.named(&["mTitle", "TEXTLABEL"]).unwrap().text.as_ref().unwrap()).unwrap(), "Option 1");
        assert!(clip.named(&["mArrowright"]).unwrap().visible);
        row.subtitle = assets.localize("ID_CAC_WEAR_STYLE_OPTION_2").unwrap();
        highlight(&mut clip, &assets, &row, false).unwrap();
        assert_eq!(String::from_utf16(clip.named(&["mTitle", "TEXTLABEL"]).unwrap().text.as_ref().unwrap()).unwrap(), "Option 2");
        assert!(!clip.named(&["mArrowright"]).unwrap().visible);
        let mut slider = Clip::new(&assets, 21, 22).unwrap();
        // Each body/face morph has an independent knob timeline. Changing
        // the row timeline alone leaves every knob on its default lit frame.
        let mut morph = Clip::new(&assets, 16, 36).unwrap();
        for selected in [false, true, false, true, false] {
            highlight(&mut morph, &assets, &EditSkaterRow {
                kind: "morph".into(), title: "Width".into(), value: 50,
                ..Default::default()
            }, selected).unwrap();
            for _ in 0..120 {
                morph.tick(&assets).unwrap();
                let knob = morph.named(&["mKnob"]).unwrap();
                // Original Notch33: on holds at frame8; off at frame17.
                assert_eq!(knob.children.contains_key(&2), selected);
                assert_eq!(knob.children.contains_key(&1), !selected);
            }
        }
        // Shared skin, eyes, hair and clothing palette control. Its arrow
        // must follow the admitted swatches, never the ten-item placeholder.
        let mut palette = Clip::new(&assets, 17, 33).unwrap();
        for count in [1, 3, 6, 10, 2] {
            for selected in [true, false, true] {
                let row = EditSkaterRow { kind: "color".into(), title: "Color".into(),
                    colours: vec![[80,120,160]; count], value: (count - 1) as i32,
                    ..Default::default() };
                highlight(&mut palette, &assets, &row, selected).unwrap();
                let x = child(&mut palette, "mArrowright").unwrap().entry.matrix.tx;
                let last = child(&mut palette, &format!("item{}", count-1)).unwrap().entry.matrix.tx;
                let next = if count < 10 {
                    child(&mut palette, &format!("item{count}")).unwrap().entry.matrix.tx
                } else { 328. };
                assert!(x > last && x <= next, "palette {count}: arrow {x}, last {last}, next {next}");
                for _ in 0..60 {
                    palette.tick(&assets).unwrap();
                    assert_eq!(child(&mut palette, "mArrowright").unwrap().entry.matrix.tx, x);
                    assert_eq!(palette.named(&["mArrowright"]).unwrap().visible, selected);
                }
                for i in 0..10 {
                    assert_eq!(palette.named(&[&format!("item{i}")]).unwrap().visible, i < count);
                }
            }
        }
        let slider_row = EditSkaterRow { kind: "slider".into(), title: "Trucks".into(),
            subtitle: "Medium".into(), value: 5, ..Default::default() };
        for selected in [false, true, false] {
            highlight(&mut slider, &assets, &slider_row, selected).unwrap();
            for _ in 0..120 {
                slider.tick(&assets).unwrap();
                for name in ["mArrowleft", "mArrowright"] {
                    assert_eq!(slider.named(&[name]).unwrap().visible, selected);
                }
            }
        }
        let mut root = Clip::new(&assets, 15, 0).unwrap();
        let help = root.named(&["mDone", "mStartbutton"]).unwrap();
        let label = "Continue".to_owned();
        let anchor = child(help, "mText0").unwrap().entry.matrix.tx;
        buttons(help, &assets, &[("Start", &label)], "HelveticaPresto", 28., 0xffb7d600).unwrap();
        let x = child(help, "mText0").unwrap().entry.matrix.tx;
        assert!(x < anchor);
        assert!(child(help, "mButton0").unwrap().entry.matrix.tx < x);
        for _ in 0..10 {
            buttons(help, &assets, &[("Start", &label)], "HelveticaPresto", 28., 0xffb7d600).unwrap();
            assert_eq!(child(help, "mText0").unwrap().entry.matrix.tx, x);
        }
        let help = root.named(&["mDone", "mButtonhelp", "mButtonmenu"]).unwrap();
        buttons(help, &assets, &[("B", &"Back".into()), ("A", &"Select".into())],
            "Marker_streak", 24., 0xffff_ffec).unwrap();
        assert!(child(help, "mText1").unwrap().entry.matrix.tx
            < child(help, "mButton0").unwrap().entry.matrix.tx);

        let mut movie = EditSkaterMovie::new(&assets).unwrap();
        let mut view = EditSkaterView { gender_confirmation: None, exit_confirmation: None, visible: true,
            menu: skate2_core::ui::edit_skater::EditSkaterMenu::EditSkaterMain,
            title: "Edit Skater".into(), rows: vec![EditSkaterRow { kind: "option".into(),
                title: "Body Mods".into(), ..Default::default() }, slider_row],
            selected: 0, window_start: 0, wallet: 0, help_a: "Select".into(),
            help_b: "Back".into(), help_start: "Finished".into(), cash_popup: false, receipt: None };
        for visible in [true, false, true] {
            view.visible = visible;
            movie.update(&assets, &view).unwrap();
            for _ in 0..120 {
                movie.tick(&assets).unwrap();
                assert!(!movie.letterbox.visible);
            }
            if visible {
                assert!(movie.intro_complete);
                assert!(movie.root.named(&["mDone", "mBack"]).unwrap().visible);
            } else {
                assert!(movie.outro_complete);
            }
        }
        for value in [2, 7, 3] {
            view.rows[1].value = value;
            movie.update(&assets, &view).unwrap();
            for _ in 0..60 {
                movie.tick(&assets).unwrap();
                let filter = movie.root.named(&["mPanel", "mFilterTab"]).unwrap();
                assert_eq!(filter.playback.timeline().frame(), Some(0));
                assert!(filter.named(&["mFiltertext", "labeltext"]).unwrap().text.as_ref().unwrap().is_empty());
            }
        }
        for selection in [Some(0), Some(1), None] {
            view.gender_confirmation = selection;
            movie.update(&assets, &view).unwrap();
            for _ in 0..90 { movie.tick(&assets).unwrap(); }
            assert_eq!(movie.popup.is_some(), selection.is_some());
            if let Some(popup) = &movie.popup {
                assert!(!movie.root.children.values().any(|c| c.clip.asset == 25));
                // Independent screen coordinates: the modal's dimmer covers
                // ordinary and ultrawide windows without stretching the card.
                let backdrop = &popup.children[&1];
                for size in [(1280,720), (2560,720)] {
                    let mut batches = Vec::new();
                    super::super::flatten_projected(&assets, &backdrop.clip,
                        backdrop.entry.matrix, super::super::Colour::default(),
                        size, &mut batches, true, None).unwrap();
                    let left = batches.iter().flat_map(|b| &b.vertices)
                        .map(|v|v[0]).fold(f32::INFINITY, f32::min);
                    let right = batches.iter().flat_map(|b| &b.vertices)
                        .map(|v|v[0]).fold(f32::NEG_INFINITY, f32::max);
                    assert!(left <= -1. && right >= 1., "popup cover: {left}..{right}");
                }
            }
        }
        let start = movie.scrollbar(&assets, 40, 11, 0).unwrap();
        assert_eq!(start, 0);
        assert_eq!(movie.scrollbar(&assets, 40, 11, 15).unwrap(), 5);
        assert_eq!(movie.scrollbar(&assets, 40, 11, 14).unwrap(), 5);
        assert_eq!(movie.scrollbar(&assets, 40, 11, 3).unwrap(), 3);
        // Exercise the actual thumbnail caller, including partial last rows.
        // At the final row the thumb must reach the same endpoint as a list
        // of that many rows; item-count/row-index mixing left it near the top.
        for count in [8usize, 9, 10, 20, 21, 22, 40] {
            let mut grid = view.clone();
            grid.rows = vec![EditSkaterRow::default(); count];
            grid.selected = count - 1;
            grid.window_start = (count.div_ceil(3).saturating_sub(3)) * 3;
            movie.root.named(&["mSelection", "mScrollbar"]).unwrap()
                .properties.insert("cac_window_start".into(), "0".into());
            movie.thumbnails(&assets, &grid).unwrap();
            let bar = movie.root.named(&["mSelection", "mScrollbar", "mcScrollbar"]).unwrap();
            assert_eq!(bar.visible, count > 9);
            if count > 9 {
                let actual = child(bar, "mSliderIndicator").unwrap().entry.matrix.ty;
                let rows = count.div_ceil(3);
                super::super::map_movie::update_scrollbar(
                    movie.root.named(&["mSelection", "mScrollbar"]).unwrap(),
                    &assets, rows, 3, rows - 3, 320.).unwrap();
                let expected = child(movie.root.named(&["mSelection", "mScrollbar", "mcScrollbar"]).unwrap(),
                    "mSliderIndicator").unwrap().entry.matrix.ty;
                assert_eq!(actual, expected, "grid count {count}");
            }
        }
    }
}
pub(super) fn attach(parent: &mut Clip, depth: usize, clip: Clip, x: f32, y: f32) {
    let mut matrix = IDENTITY;
    matrix.tx = x;
    matrix.ty = y;
    parent.children.insert(
        depth as i32,
        Child {
            entry: DisplayEntry {
                identity: PlacementIdentity {
                    character: clip.character,
                    frame: 0,
                },
                matrix,
                colour_scale: u32::MAX,
                colour_bias: 0,
                ratio: 0.,
                name: Some(format!("item{depth}")),
                clip_depth: -1,
                events: Vec::new(),
                blend_mode: None,
            },
            clip,
        },
    );
}
pub(super) fn highlight(
    clip: &mut Clip,
    assets: &Assets,
    row: &EditSkaterRow,
    selected: bool,
) -> RenderResult<()> {
    clip.goto(assets, if selected { "selected" } else { "unselected" })?;
    // SetState on the original highlight classes controls arrows separately
    // from their timelines; unselected sliders must not retain live arrows.
    for child in clip.children.values_mut() {
        if matches!(child.entry.name.as_deref(), Some("mArrowleft" | "mArrowright")) {
            child.clip.visible = selected;
        }
    }
    for label in ["mTitle", "mTitleglow"] {
        let field = clip.named(&[label, "TEXTLABEL"])?;
        // createskater.UpdateItemValue010030..046 calls SetTitle with
        // CAC_GetStringValue for category controls (e.g. Style 1/Style 2).
        text(field, if row.kind == "category" { &row.subtitle } else { &row.title });
        field.properties.insert(
            "text_colour".into(),
            if selected {
                0xffff_ffec_u32
            } else {
                0xff99_8b63_u32
            }
            .to_string(),
        );
    }
    match row.kind.as_str() {
        "selector" | "slider" => {
            let subtitle = clip.named(&["mSubtitle", "TEXTLABEL"])?;
            text(subtitle, &row.subtitle);
            subtitle.properties.insert("text_colour".into(),
                (0xff00_0000u32 | if selected { 16771497 } else { 6245684 }).to_string());
            if row.kind == "selector" {
                // highlight_selector.SetSubtitle0016a6..16d6: autoSize left,
                // arrow.x = subtitle.x + text.width + mArrowMargin (zero).
                selector_subtitle(clip, assets, &row.subtitle)?;
            }
            if row.kind == "slider" {
                slider_bars(clip, assets, row.value, selected)?;
            }
        }
        "morph" => {
            // highlight_morph.SetState0021bd..21f8 switches the knob's own
            // timeline as well as the row; its initial frame is the lit state.
            clip.named(&["mKnob"])?.goto(assets, if selected { "on" } else { "off" })?;
            morph_knob(clip, assets, row.value)?;
        }
        "color" => {
            let first = (row.value.max(0) as usize).saturating_sub(9);
            let mut arrow_x = None;
            for i in 0..10 {
                let item = child(clip, &format!("item{i}"))?;
                let swatch = &mut item.clip;
                swatch.visible = first + i < row.colours.len();
                if let Some(rgb) = row.colours.get(first + i) {
                    // highlight_color.SetNumSwatches002552..2566 sets the
                    // arrow after EACH admitted swatch, leaving it after the
                    // last one. _width includes the authored item transform;
                    // mArrowMargin is -8 (00276f..2775).
                    swatch.goto(assets, "off")?;
                    let r = bounds(swatch, assets)?.ok_or("CAC colour swatch bounds")?;
                    arrow_x = Some(item.entry.matrix.tx
                        + (r.right - r.left) * item.entry.matrix.a.abs() - 8.);
                    swatch.goto(
                        assets,
                        if first + i == row.value.max(0) as usize {
                            if selected { "on" } else { "on_noglow" }
                        } else {
                            "off"
                        },
                    )?;
                    swatch.named(&["mSwatch"])?.properties.insert(
                        "cac_colour".into(),
                        format!(
                            "{}",
                            0xff000000u32
                                | ((rgb[0] as u32) << 16)
                                | ((rgb[1] as u32) << 8)
                                | rgb[2] as u32
                        ),
                    );
                }
            }
            if let Some(x) = arrow_x {
                child(clip, "mArrowright")?.entry.matrix.tx = x;
            }
        }
        "category" => {
            // highlight_category.SetTitle001538..155d: autoSize left, then
            // put the right arrow immediately after the measured label.
            let field = clip.named(&["mTitle", "TEXTLABEL"])?;
            let width = autosize_text(field, assets, &row.subtitle)?;
            let title_x = child(clip, "mTitle")?.entry.matrix.tx;
            child(clip, "mArrowright")?.entry.matrix.tx = title_x + width;
        }
        "option" => {}
        _ => return Err(format!("unsupported CAC control {}", row.kind).into()),
    }
    Ok(())
}
/// `highlight_morph`'s row (asset 16): Edit Skater's body and face morph knob.
pub(super) const MORPH: u32 = 36;
/// The morph knob at `value` (0 to 100) along its line.
fn morph_knob(clip: &mut Clip, assets: &Assets, value: i32) -> RenderResult<()> {
    let length = bounds(clip.named(&["mMorphLine"])?, assets)?.ok_or("CAC morph bounds")?;
    let line = child(clip, "mMorphLine")?.entry.matrix;
    child(clip, "mKnob")?.entry.matrix.tx = line.tx + (length.right - length.left) * line.a * value as f32 / 100.;
    Ok(())
}
/// PC port: a morph row's title and knob refreshed without replaying its
/// selection, as a Stick Settings deadzone moves.
pub(super) fn morph_value(clip: &mut Clip, assets: &Assets, title: &str, value: i32) -> RenderResult<()> {
    for label in ["mTitle", "mTitleglow"] {
        text(clip.named(&[label, "TEXTLABEL"])?, title);
    }
    morph_knob(clip, assets, value)
}
/// highlight_category.SetTitle001538..155d alone: both title fields, then the
/// right arrow immediately after the measured label.
pub(super) fn category_title(clip: &mut Clip, assets: &Assets, row: &EditSkaterRow) -> RenderResult<()> {
    for label in ["mTitle", "mTitleglow"] {
        text(clip.named(&[label, "TEXTLABEL"])?, &row.subtitle);
    }
    let field = clip.named(&["mTitle", "TEXTLABEL"])?;
    let width = autosize_text(field, assets, &row.subtitle)?;
    let title_x = child(clip, "mTitle")?.entry.matrix.tx;
    child(clip, "mArrowright")?.entry.matrix.tx = title_x + width;
    Ok(())
}
/// highlight_slider CreateBars: ten bars, total width200, spacing5, the first
/// `value` lit.
pub(super) fn slider_bars(clip: &mut Clip, assets: &Assets, value: i32, selected: bool) -> RenderResult<()> {
    let asset = clip.asset;
    let holder = clip.named(&["mNotchHolder"])?;
    holder.children.clear();
    for i in 0..10 {
        let mut bar = Clip::new(assets, asset, 9)?;
        let original = bounds(bar.named(&["notchSize"])?, assets)?
            .ok_or("CAC slider bar bounds")?;
        let scale = 14.5 / (original.right - original.left);
        bar.goto(
            assets,
            if i < value.max(0) as usize {
                if selected { "on" } else { "on_noglow" }
            } else {
                "off_dark"
            },
        )?;
        attach(holder, i, bar, 5. + i as f32 * 19.5, 0.);
        holder.children.get_mut(&(i as i32)).unwrap().entry.matrix.a = scale;
    }
    Ok(())
}
/// highlight_slider SetSubtitle and SetValue: a refreshed value keeps the row's animation.
pub(super) fn slider_value(clip: &mut Clip, assets: &Assets, subtitle: &str, value: i32, selected: bool) -> RenderResult<()> {
    text(clip.named(&["mSubtitle", "TEXTLABEL"])?, subtitle);
    slider_bars(clip, assets, value, selected)
}
/// Shared highlight_selector.SetSubtitle0016a6..16d6. Refreshing a value
/// must not restart the selected/unselected animation.
pub(super) fn selector_subtitle(clip: &mut Clip, assets: &Assets, value: &str) -> RenderResult<()> {
    let subtitle = clip.named(&["mSubtitle", "TEXTLABEL"])?;
    text(subtitle, value);
    let width = autosize_text(subtitle, assets, value)?;
    let subtitle_x = child(clip, "mSubtitle")?.entry.matrix.tx;
    child(clip, "mArrowright")?.entry.matrix.tx = subtitle_x + width;
    Ok(())
}
pub(super) fn autosize_text(field: &mut Clip, assets: &Assets, label: &str) -> RenderResult<f32> {
    let skate2_formats::apt::Character::Text(def) = field.definition(assets)? else {
        return Err("CAC auto-sized field is not text".into());
    };
    let Some(skate2_formats::apt::Character::Font { name, .. }) =
        assets.movies[field.asset].movie.characters.get(def.font_id as usize).and_then(Option::as_ref)
    else { return Err("CAC text font absent".into()); };
    let font = assets.apt_font(name)?;
    let width: f32 = label.encode_utf16().filter_map(|c|font.metrics.mapped_glyph(c))
        .map(|g|g.advance * font.scale[0] * def.height).sum();
    field.properties.insert("text_width".into(), width.to_string());
    field.properties.insert("text_alignment".into(), "0".into());
    Ok(width)
}
fn buttons(clip: &mut Clip, assets: &Assets, items: &[(&str, &String)],
    font_name: &str, height: f32, colour: u32) -> RenderResult<()> {
    let font = assets
        .fonts
        .get(font_name)
        .ok_or("CAC button font")?;
    let items: Vec<_> = items.iter().filter(|(_, label)|!label.is_empty() && *label != "#").collect();
    let mut x = if let Some(x) = clip.properties.get("button_original_start_x") {
        x.parse::<f32>()?
    } else {
        let x = child(clip, "mText0")?.entry.matrix.tx;
        clip.properties.insert("button_original_start_x".into(), x.to_string());
        x
    };
    let button_y = child(clip, "mButton0")?.entry.matrix.ty;
    for i in 0..4 {
        child(clip, &format!("mButton{i}"))?.clip.visible =
            i < items.len() && !items[i].1.is_empty() && items[i].1 != "#";
        child(clip, &format!("mText{i}"))?.clip.visible =
            i < items.len() && !items[i].1.is_empty() && items[i].1 != "#";
    }
    for (i, (button, label)) in items.iter().enumerate() {
        // button_menu prototype000ad1..ad5 defaults to right justification.
        // RefreshItems0009c4..aa8 measures the auto-sized text, then places
        // its icon to the left. B is added first, so A appears left of B.
        let width = label.encode_utf16()
            .filter_map(|c| font.metrics.mapped_glyph(c))
            .map(|g| g.advance * font.scale[0] * height)
            .sum::<f32>();
        x -= width;
        let text_x = x;
        x += 10.;
        let entry = child(clip, &format!("mButton{i}"))?;
        entry
            .clip
            .named(&["mButtonRender"])?
            .properties
            .insert("_test".into(), (*button).into());
        let b = bounds(&entry.clip, assets)?.ok_or("CAC button bounds")?;
        x -= b.right - b.left;
        entry.entry.matrix.tx = x;
        x -= 25.;
        let entry = child(clip, &format!("mText{i}"))?;
        entry.entry.matrix.tx = text_x;
        entry.entry.matrix.ty = button_y - font.metrics.ascent * font.scale[1] * height / 2. - 4.;
        entry
            .clip
            .properties
            .insert("text_font".into(), font_name.into());
        entry
            .clip
            .properties
            .insert("text_height".into(), height.to_string());
        entry.clip.properties.insert("text_colour".into(), colour.to_string());
        text(&mut entry.clip, label);
    }
    Ok(())
}
/// Native class registration streams only; constructor display work is above.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!(
        (asset, character, offset),
        (15, 153, 0xe544)
            | (16, 37, 0x187c)
            | (16, 38, 0x2088)
            | (16, 36, 0x247c)
            | (17, 34, 0x1a44)
            | (17, 35, 0x2250)
            | (17, 33, 0x27a0)
            | (18, 22, 0xd88)
            | (18, 23, 0x1594)
            | (18, 21, 0x18a4)
            | (19, 19, 0xc50)
            | (19, 20, 0x145c)
            | (19, 18, 0x16fc)
            | (20, 37, 0x1844)
            | (20, 38, 0x2050)
            | (20, 36, 0x22a4)
            | (21, 23, 0xd80)
            | (21, 22, 0x158c)
            | (22, 14, 0x948)
            | (22, 13, 0x9ac)
            | (23, 14, 0x990)
            | (23, 13, 0x9f4)
            | (24, 4, 0x2d8)
            | (24, 5, 0x578)
            | (24, 3, 0x700)
            // sk8popup class registration. Constructor/Refresh are translated
            // by gender_popup; this is also emitted as an InitAction.
            | (25, 29, 0x1b00)
    )
}
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match (clip.asset, offset) {
        (25, 0x1adc) => clip.playback.stop(),
        (25, 0x1b00) => {}
        (25, 0x2120) => {
            clip.playback.stop();
            clip.calls.push("CacGenderPopupIntroComplete");
        }
        (25, 0x214c) => {
            clip.playback.stop();
            clip.calls.push("CacGenderPopupOutroComplete");
        }
        (15, 0xe4f8) => clip.playback.stop(),
        (15, 0x10250) => {
            clip.playback.stop();
            clip.calls.push("CacIntroComplete");
        }
        (15, 0x10280) => {}
        (15, 0x1028c) => {
            clip.playback.stop();
            clip.calls.push("CacOutroComplete");
        }
        _ => return Err(format!("untranslated CAC action {}:{offset:#x}", clip.asset).into()),
    }
    Ok(())
}

/// Shared original sk8popup Refresh; titles, options and cancellation come from
/// native gPopupTemplate. Reused by CAC and StartHub, never duplicate geometry.
pub(super) fn configure_popup(popup:&mut Clip,assets:&Assets,selection:usize,
    title:&str,description:&str,options:&[&str],cancellable:bool)->RenderResult<()> {
        let panel = popup.named(&["mcPanel"])?;
        text(
            panel.named(&["titleText"])?,
            &assets.localize(title)?,
        );
        let core = bounds(panel.named(&["mCore"])?, assets)?.ok_or("popup core bounds")?;
        let width = core.right - core.left;
        let original_height = core.bottom - core.top;
        let mut body = Clip::new(assets, 25, 21)?;
        for (k, v) in [
            ("text_font", "Helvetica LT Std"),
            ("text_height", "22"),
            ("text_alignment", "2"),
            ("text_left", "0"),
            ("text_top", "0"),
        ] {
            body.properties.insert(k.into(), v.into());
        }
        body.properties
            .insert("text_colour".into(), (0xff000000u32 | 12046808).to_string());
        body.properties
            .insert("text_width".into(), (width - 50.0).to_string());
        let font = assets
            .fonts
            .get("Helvetica LT Std")
            .ok_or("popup body font missing")?;
        // sk8popup::Refresh: a description of "#", or one that localizes to "#" or "# ", leaves
        // the body empty and puts mMenuPickerOnlySpacing (15) under the menu picker.
        let localized = assets.localize(description)?;
        let literal = description.starts_with('#') && description.len() > 2;
        let menu_only = description == "#" || (!literal && matches!(localized.as_str(), "#" | "# "));
        let body_text = if menu_only { String::new() } else { localized };
        // SkateAptString 0x82a70b28 breaks a multiline field at '^' as at a newline.
        let description = body_text.replace('^', "\n");
        let advance = |s: &str| {
            s.encode_utf16()
                .filter_map(|c| font.metrics.mapped_glyph(c))
                .map(|g| g.advance * font.scale[0] * 22.0)
                .sum::<f32>()
        };
        let mut lines: Vec<String> = Vec::new();
        let mut line = String::new();
        for paragraph in description.split('\n') {
            for word in paragraph.split_whitespace() {
                let candidate = if line.is_empty() {
                    word.to_owned()
                } else {
                    format!("{line} {word}")
                };
                if !line.is_empty() && advance(&candidate) > width - 50.0 {
                    lines.push(std::mem::take(&mut line));
                    line.push_str(word);
                } else {
                    line = candidate;
                }
            }
            lines.push(std::mem::take(&mut line));
        }
        text(&mut body, &lines.join("\n"));
        attach(panel, 20001, body, 0.0, 30.0);
        let labels = options.iter().map(|option| assets.localize(option)).collect::<Result<Vec<_>, _>>()?;
        let picker = panel.named(&["mMenuPicker", "mMenuPicker"])?;
        let height = bounds(picker.named(&["dummy0"])?, assets)?.ok_or("popup picker bounds")?;
        let height = height.bottom - height.top;
        for i in 0..8 {
            picker.named(&[&format!("dummy{i}")])?.visible = false;
        }
        let group = picker.named(&["mItemGroup"])?;
        group.children.clear();
        let mut maximum = 0.0f32;
        for (i, label) in labels.iter().enumerate() {
            let mut item = Clip::new(assets, 6, 31)?;
            item.goto(
                assets,
                if i == selection {
                    "selected"
                } else {
                    "deselected"
                },
            )?;
            for names in [&["itemText"][..], &["mTextglow", "itemText"][..]] {
                let t = item.named(names)?;
                text(t, label);
                t.properties
                    .insert("text_colour".into(), u32::MAX.to_string());
            }
            maximum = maximum.max(advance(label));
            attach(group, 100 + i, item, 0.0, i as f32 * height);
        }
        // menu_picker's Update hides the indicator without an item to stand on.
        let indicator = child(picker, "mIndicator")?;
        indicator.entry.matrix.ty = selection as f32 * height;
        indicator.clip.visible = !labels.is_empty();
        let menu_height = height * labels.len() as f32;
        let body_height = if menu_only { 0.0 } else { lines.len() as f32 * 22.0 };
        let scale = ((body_height + menu_height + 20.0) / original_height).max(1.0);
        child(panel, "mCore")?.entry.matrix.d = scale;
        let bottom_y = child(panel, "mCore")?.entry.matrix.ty + original_height * scale;
        let bottom_height = {
            let b = bounds(panel.named(&["mBottom"])?, assets)?.ok_or("popup bottom bounds")?;
            b.bottom - b.top
        };
        child(panel, "mBottom")?.entry.matrix.ty = bottom_y;
        child(panel, "mMenuPicker")?.entry.matrix.tx = width / 2.0 - maximum / 2.0 - 32.0;
        child(panel, "mMenuPicker")?.entry.matrix.ty =
            bottom_y + bottom_height - menu_height - 48.0 - if menu_only { 15.0 } else { 0.0 };
        // sk8popup::Refresh adds B for a cancellable popup and A only with options.
        let mut help=Vec::new();
        if cancellable {help.push(("B",assets.localize("ID_COMMON_CANCEL")?));}
        if !labels.is_empty() {help.push(("A",assets.localize("ID_COMMON_SELECT")?));}
        let help=help.iter().map(|(b,s)|(*b,s)).collect::<Vec<_>>();
        buttons(
            panel.named(&["mBottom", "mcButtonhelp", "mButtons"])?,
            assets,
            &help,
            "Marker_streak", 24., 0xffff_ffec,
        )?;
        child(popup, "mcPanel")?.entry.matrix.ty = 360.0 - original_height * scale / 2.0;
        Ok(())
}
