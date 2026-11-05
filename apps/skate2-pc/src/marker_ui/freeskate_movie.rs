//! Original main/freeskate_options APT (`FrontEndState_FreeSkateOption`, screen 66), fingerprint
//! e7de5095: highlight_selector rows for the character, location and security and
//! highlight_slider rows for traffic and pedestrians. It drives the front end's letterbox.
use super::{assets::Assets, clips::{Child, Clip}, edit_skater_movie, letterbox::LetterBox, map_movie};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::{
    edit_skater::EditSkaterRow,
    freeskate_option::{Row, Snapshot, AREA_STATE, SELECTION_STATE},
};

pub(super) const ASSET: usize = 48;
/// PC port: the Area row a linked Skate 3 data folder adds (`freeskate_option::Area`), a copy of
/// the character selector standing where the timeline places that row; the native rows move one
/// row down.
const AREA_SELECT: &str = "mAreaSelect";
const AREA_TITLE: &str = "#Area";
/// The placed controls in `kState_*` order, their titles and kinds.
const ROWS: [(&str, &str, &str); 5] = [
    ("mCharacterSelect", "ID_FREESKATEOPTION_CHARACTER_TITLE", "selector"),
    ("mLocationSelect", "ID_FREESKATEOPTION_LOCATION_TITLE", "selector"),
    ("mTraffic", "ID_FREESKATEOPTION_TRAFFIC", "slider"),
    ("mPedestrians", "ID_FREESKATEOPTION_PEDESTRIAN", "slider"),
    ("mSecurity", "ID_FREESKATEOPTION_SECURITY", "selector"),
];

pub(super) struct FreeskateMovie {
    pub root: Clip,
    /// The refreshes' `Audio.PlaySound` requests.
    pub sounds: Vec<&'static str>,
    /// `mbAudioAllowed`.
    audio: bool,
    /// The screen is open (between the constructor and the outro).
    open: bool,
    /// The traffic and pedestrian sliders' `mCurrentValue`.
    values: [i32; 2],
    intro_complete: bool,
}

impl FreeskateMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, ASSET, 0)?;
        root.playback.stop();
        root.visible = false;
        Ok(Self { root, sounds: Vec::new(), audio: false, open: false, values: [0; 2], intro_complete: false })
    }

    /// The intro's `ScreenManager.IntroComplete` since the last call.
    pub fn take_intro_complete(&mut self) -> bool {
        std::mem::take(&mut self.intro_complete)
    }

    /// Opens with the screen's first snapshot, plays the outro when the screen closes, and
    /// runs each `SetState` the state sent.
    pub fn update(&mut self, assets: &Assets, snapshot: Option<&Snapshot>, states: &[u32], letterbox: &mut LetterBox) -> RenderResult<()> {
        match snapshot {
            Some(snapshot) if !self.open => self.construct(assets, snapshot, letterbox)?,
            None if self.open => {
                self.open = false;
                self.root.goto(assets, "outro")?;
            }
            _ => {}
        }
        let Some(snapshot) = snapshot.filter(|_| self.open) else { return Ok(()) };
        for &state in states {
            self.set_state(assets, snapshot, state)?;
        }
        Ok(())
    }

    /// The constructor 0004d3: the letterbox title and B/A help, the row titles, ten slider
    /// bars, every row refreshed without sound, then `UpdateSelection` and `mbAudioAllowed`.
    fn construct(&mut self, assets: &Assets, snapshot: &Snapshot, letterbox: &mut LetterBox) -> RenderResult<()> {
        self.root = Clip::new(assets, ASSET, 0)?;
        self.root.visible = true;
        self.root.goto(assets, "intro")?;
        self.open = true;
        letterbox.set_title(&assets.localize("ID_SCREEN_GAME_MODE_FREESKATE")?)?;
        map_movie::configure_button_items(
            letterbox.clip.named(&["mcLetterbox", "mInfo", "mcBottomMenu", "mButtonMenu"])?,
            assets,
            &[("B", "ID_COMMON_BACK"), ("A", "ID_FREESKATEOPTION_START")],
            0xffff_ffec,
        )?;
        self.audio = false;
        self.values = [snapshot.traffic, snapshot.pedestrians];
        if snapshot.area.is_some() {
            self.add_area_row()?;
        }
        self.update_selection(assets, snapshot)?;
        self.audio = true;
        Ok(())
    }

    /// PC port: the Area row, a copy of the character selector, placed by [`Self::follow_area_row`].
    fn add_area_row(&mut self) -> RenderResult<()> {
        let named = |child: &Child, name: &str| child.entry.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(name));
        if self.root.children.values().any(|child| named(child, AREA_SELECT)) {
            return Ok(());
        }
        let mut area = self.root.children.values().find(|child| named(child, ROWS[0].0)).cloned().ok_or("mCharacterSelect absent")?;
        area.entry.name = Some(AREA_SELECT.to_owned());
        area.clip.layout_offset = [0.0; 2];
        let depth = self.root.children.keys().next_back().copied().unwrap_or(0) + 1;
        self.root.children.insert(depth, area);
        self.follow_area_row()
    }

    /// PC port: the Area row stands where the timeline places the character selector, with its
    /// fade (it is no timeline's own), and each native row one row below its own placement: the
    /// spacing between the timeline's character and location rows.
    fn follow_area_row(&mut self) -> RenderResult<()> {
        let named = |child: &Child, name: &str| child.entry.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(name));
        if !self.root.children.values().any(|child| named(child, AREA_SELECT)) {
            return Ok(());
        }
        let placed = |name: &str| {
            self.root.children.values().find(|child| named(child, name)).map(|child| {
                let mut entry = child.entry.clone();
                entry.matrix.ty -= child.clip.layout_offset[1];
                entry
            })
        };
        let (Some(character), Some(location)) = (placed(ROWS[0].0), placed(ROWS[1].0)) else { return Ok(()) };
        let row = location.matrix.ty - character.matrix.ty;
        for child in self.root.children.values_mut() {
            if ROWS.iter().any(|&(name, _, _)| named(child, name)) {
                child.entry.matrix.ty += row - child.clip.layout_offset[1];
                child.clip.layout_offset[1] = row;
            } else if named(child, AREA_SELECT) {
                let name = child.entry.name.take();
                child.entry = character.clone();
                child.entry.name = name;
            }
        }
        Ok(())
    }

    fn row(assets: &Assets, snapshot: &Snapshot, index: usize) -> RenderResult<EditSkaterRow> {
        let (_, title, kind) = ROWS[index];
        let (subtitle, value) = match index {
            0 => (assets.localize(&snapshot.character)?, 0),
            1 => (assets.localize(&snapshot.location)?, 0),
            2 => (assets.localize(density_label(snapshot.traffic, 5, "TRAFFIC"))?, snapshot.traffic),
            3 => (assets.localize(density_label(snapshot.pedestrians, 6, "PEDS"))?, snapshot.pedestrians),
            _ => (assets.localize(if snapshot.security { "ID_COMMON_ON" } else { "ID_COMMON_OFF" })?, 0),
        };
        Ok(EditSkaterRow { kind: kind.into(), title: assets.localize(title)?, subtitle, value, ..Default::default() })
    }

    /// PC port: the Area row's title and choice.
    fn area_row(assets: &Assets, snapshot: &Snapshot) -> RenderResult<Option<EditSkaterRow>> {
        let Some(area) = snapshot.area.as_deref() else { return Ok(None) };
        Ok(Some(EditSkaterRow { kind: "selector".into(), title: assets.localize(AREA_TITLE)?, subtitle: assets.localize(area)?, ..Default::default() }))
    }

    /// `UpdateSelection` 000615: every row `unselected`, then `GetSelection`'s `selected` (the
    /// Area row's, when it is selected).
    fn update_selection(&mut self, assets: &Assets, snapshot: &Snapshot) -> RenderResult<()> {
        for index in 0..ROWS.len() {
            let row = Self::row(assets, snapshot, index)?;
            let control = self.root.named(&[ROWS[index].0])?;
            edit_skater_movie::highlight(control, assets, &row, !snapshot.area_selected && snapshot.selection as usize == index)?;
        }
        if let Some(row) = Self::area_row(assets, snapshot)? {
            edit_skater_movie::highlight(self.root.named(&[AREA_SELECT])?, assets, &row, snapshot.area_selected)?;
        }
        Ok(())
    }

    /// `SetState` 000a84: the row's `Refresh*`, or with 999 `UpdateSelection`.
    fn set_state(&mut self, assets: &Assets, snapshot: &Snapshot, state: u32) -> RenderResult<()> {
        if state == SELECTION_STATE {
            return self.update_selection(assets, snapshot);
        }
        // PC port: the Area row's choice; the Location row's refresh that follows plays the sound.
        if state == AREA_STATE {
            if let Some(row) = Self::area_row(assets, snapshot)? {
                edit_skater_movie::selector_subtitle(self.root.named(&[AREA_SELECT])?, assets, &row.subtitle)?;
            }
            return Ok(());
        }
        let Some(index) = [Row::Character, Row::Location, Row::Traffic, Row::Pedestrians, Row::Security]
            .iter()
            .position(|&row| row as u32 == state)
        else {
            return Ok(());
        };
        let row = Self::row(assets, snapshot, index)?;
        let selected = snapshot.selection as usize == index;
        match index {
            // RefreshCharacter 000745 / RefreshLocation 000794: the name, then nav right.
            0 | 1 => {
                edit_skater_movie::selector_subtitle(self.root.named(&[ROWS[index].0])?, assets, &row.subtitle)?;
                if self.audio {
                    self.sounds.push("FE_SOUND_CORE_NAV_RIGHT");
                }
            }
            // RefreshTraffic 0007e4 / RefreshPedestrians 00097b: nav right when the slider's
            // value is above the setting, nav left when below; then SetValue and the label.
            2 | 3 => {
                let current = &mut self.values[index - 2];
                if self.audio {
                    if *current > row.value {
                        self.sounds.push("FE_SOUND_CORE_NAV_RIGHT");
                    } else if *current < row.value {
                        self.sounds.push("FE_SOUND_CORE_NAV_LEFT");
                    }
                }
                *current = row.value;
                let control = self.root.named(&[ROWS[index].0])?;
                edit_skater_movie::slider_bars(control, assets, row.value, selected)?;
                control.named(&["mSubtitle", "TEXTLABEL"])?.text = Some(row.subtitle.encode_utf16().collect());
            }
            // RefreshSecurity 0008f0: nav right and On, or nav left and Off.
            _ => {
                if self.audio {
                    self.sounds.push(if snapshot.security { "FE_SOUND_CORE_NAV_RIGHT" } else { "FE_SOUND_CORE_NAV_LEFT" });
                }
                edit_skater_movie::selector_subtitle(self.root.named(&[ROWS[index].0])?, assets, &row.subtitle)?;
            }
        }
        Ok(())
    }

    pub fn tick(&mut self, assets: &Assets, letterbox: &mut LetterBox) -> RenderResult<()> {
        if !self.root.visible {
            return Ok(());
        }
        self.root.tick(assets)?;
        self.follow_area_row()?;
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                // Frame 22: ScreenManager.IntroComplete, EnableLetterBoxInfo(true).
                "FreeskateIntroComplete" => {
                    self.intro_complete = true;
                    letterbox.enable_info(assets, true)?;
                }
                // Frame 23: EnableLetterBoxInfo(false), CycleLetterBox.
                "FreeskateOutroBegin" => {
                    letterbox.enable_info(assets, false)?;
                    letterbox.cycle(assets)?;
                }
                // Frame 40: ScreenManager.OutroComplete.
                "FreeskateOutroComplete" => self.root.visible = false,
                _ => return Err(format!("untranslated free skate callback {call}").into()),
            }
        }
        Ok(())
    }
}

/// The traffic and pedestrian labels: under 1 none, under 3 low, up to `medium` medium, above
/// it high.
fn density_label(value: i32, medium: i32, kind: &str) -> &'static str {
    match (kind, value) {
        ("TRAFFIC", v) if v < 1 => "ID_FREESKATEOPTION_NO_TRAFFIC",
        ("TRAFFIC", v) if v < 3 => "ID_FREESKATEOPTION_LOW_TRAFFIC",
        ("TRAFFIC", v) if v <= medium => "ID_FREESKATEOPTION_MEDIUM_TRAFFIC",
        ("TRAFFIC", _) => "ID_FREESKATEOPTION_HIGH_TRAFFIC",
        (_, v) if v < 1 => "ID_FREESKATEOPTION_NO_PEDS",
        (_, v) if v < 3 => "ID_FREESKATEOPTION_LOW_PEDS",
        (_, v) if v <= medium => "ID_FREESKATEOPTION_MEDIUM_PEDS",
        _ => "ID_FREESKATEOPTION_HIGH_PEDS",
    }
}

/// The InitAction defining `_global.freeskate_options`, translated by `FreeskateMovie`.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 3, 0x4b8))
}

pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        // `var screen = new freeskate_options(this)`; FreeskateMovie owns the constructor.
        0x458 => {}
        0xc0c => {
            clip.playback.stop();
            clip.calls.push("FreeskateIntroComplete");
        }
        0xc2c => clip.calls.push("FreeskateOutroBegin"),
        0xc50 => {
            clip.playback.stop();
            clip.calls.push("FreeskateOutroComplete");
        }
        _ => return Err(format!("untranslated free skate APT action {offset:#x}").into()),
    }
    Ok(())
}
