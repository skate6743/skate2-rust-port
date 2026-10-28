//! Original hud2/hud_freeskateactivity_overlay (screen 89, asset 117, fingerprint abb9425f;
//! template entry `FreeskateActivityOverlayHUD` 0x82fdb8e8, placement 5) and its
//! `hud_freeskateactivity_overlay` class (InitAction character 17, `0x123c`), the overlay
//! `FreeskateActivityOverlayHUD` (0x826beea8, 0x826bef58) drives while a freeskate activity is
//! proposed: `Activity Proposed!`, the type's `dpoicons` icon, the proposer's name and the seconds
//! left, with D-pad up while this console may open the sign-up to accept. What its
//! `HUDComponents` calls answer comes from `skate2_core::online::activities::OverlayView`.
use super::{assets::Assets, clips::Clip, objective_movie::set_x, pause_movie::child};
use crate::gpu_renderer::RenderResult;
use skate2_core::online::activities::OverlayView;

pub(super) const ASSET: usize = 117;
/// `SetActivityIcon`'s icons by the type's HAL id.
const ICONS: [(&str, &str); 6] = [
    ("ID_CHALLENGE_TYPE_ACCUMULATION", "hud_fa_accumulation"),
    ("ID_CHALLENGE_TYPE_GAP", "hud_fa_gap"),
    ("ID_CHALLENGE_TYPE_TAG", "hud_fa_tag"),
    ("ID_CHALLENGE_TYPE_SIMULTANEOUSTRICK", "hud_fa_simultaneous"),
    ("ID_CHALLENGE_TYPE_SURVIVAL", "hud_fa_survival"),
    ("ID_CHALLENGE_TYPE_GROUPTRICKLIST", "hud_fa_list"),
];

pub(super) struct ActivityOverlayMovie {
    pub root: Clip,
    /// What the overlay shows while it is up.
    shown: Option<OverlayView>,
    /// The type and proposer `UpdateText` last set (`mIsActivityTypeAndGamerTagSet`).
    text: Option<(&'static str, String)>,
    /// The `dpoicons` icon `SetActivityIcon` names, for the owner to load.
    pub icon_request: Option<String>,
}

impl ActivityOverlayMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut movie = Self { root: Clip::new(assets, ASSET, 0)?, shown: None, text: None, icon_request: None };
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
                "hud_freeskateactivity_overlay" => self.construct()?,
                _ => return Err(format!("untranslated activity overlay callback {call}").into()),
            }
        }
        Ok(())
    }

    /// `new hud_freeskateactivity_overlay(this)`: the edge offset (zero) added to `_x`,
    /// `RegisterScreen`, then `Initialise`: the screen hidden, no type or proposer set, and the
    /// name and time fields empty.
    fn construct(&mut self) -> RenderResult<()> {
        self.root.visible = false;
        self.text = None;
        let content = self.root.named(&["mContent"])?;
        content.named(&["mcGamertag", "txtGamertag"])?.text = Some(Vec::new());
        for field in ["txtTimer", "txtGlow"] {
            content.named(&["mcTimer", field])?.text = Some(Vec::new());
        }
        Ok(())
    }

    /// The overlay as `FreeskateActivityOverlayHUD` shows it: `Show(true)` (`SetState(1)` with
    /// the D-pad while accepting, else 5) and the next `Update`'s `SetState(3)`, a time that
    /// changes `SetState(4)`, a mode that changes `Show(true)` again, and `Show(false)`
    /// (`SetState(2)`) as it goes.
    pub fn update(&mut self, assets: &Assets, view: Option<&OverlayView>) -> RenderResult<()> {
        let shown = self.shown.as_ref().map(|shown| (shown.accepting, shown.seconds));
        match (view, shown) {
            (Some(view), None) => {
                self.show(assets, view.accepting)?;
                if self.text.as_ref().is_none_or(|(kind, name)| *kind != view.type_title || *name != view.proposer) {
                    self.update_text(view)?;
                }
                self.update_timer(view)?;
            }
            (Some(view), Some((accepting, seconds))) => {
                if view.accepting != accepting {
                    self.show(assets, view.accepting)?;
                }
                if view.seconds != seconds {
                    self.update_timer(view)?;
                }
            }
            (None, Some(_)) => self.root.goto(assets, "fa_outro")?,
            (None, None) => {}
        }
        self.shown = view.cloned();
        Ok(())
    }

    /// `SetState(1)` and `SetState(5)`: `UpdateHiddenMovieClips` (with the D-pad, right of it, the
    /// title and time at 70 and the name at 102 as authored; without, 38 and 70), then
    /// `Show(true)`: the screen shown and `fa_intro`.
    fn show(&mut self, assets: &Assets, dpad: bool) -> RenderResult<()> {
        let content = self.root.named(&["mContent"])?;
        let (left, name) = if dpad { (70., 102.) } else { (38., 70.) };
        set_x(child(content, "mTitle")?, left);
        set_x(child(content, "mcTimer")?, left);
        set_x(child(content, "mcGamertag")?, name);
        content.named(&["mcDpadUp"])?.visible = dpad;
        self.root.visible = true;
        self.root.goto(assets, "fa_intro")
    }

    /// `UpdateText`: `FreeskateActivityOverlay_GetData`'s type picks the icon `SetActivityIcon`
    /// loads (`LoadRX2Texture("fe\source\images\dpoicons\" + icon)`, none for an unknown type),
    /// and its proposer is the name.
    fn update_text(&mut self, view: &OverlayView) -> RenderResult<()> {
        self.text = Some((view.type_title, view.proposer.clone()));
        self.icon_request = Some(ICONS.iter().find(|(kind, _)| *kind == view.type_title).map_or("", |(_, icon)| icon).to_owned());
        let name = self.root.named(&["mContent", "mcGamertag", "txtGamertag"])?;
        name.text = Some(view.proposer.encode_utf16().collect());
        Ok(())
    }

    /// `UpdateCurrentTimer`: `"#:"` and `FreeskateActivityOverlay_GetTime`'s seconds.
    fn update_timer(&mut self, view: &OverlayView) -> RenderResult<()> {
        let timer = self.root.named(&["mContent", "mcTimer", "txtTimer"])?;
        timer.text = Some(format!(":{}", view.seconds).encode_utf16().collect());
        Ok(())
    }

    /// `mIcon`'s `LoadRX2Texture` once the owner has loaded the named icon.
    pub fn set_icon(&mut self, texture: Option<String>) -> RenderResult<()> {
        let icon = self.root.named(&["mIcon"])?;
        match texture {
            Some(texture) => {
                icon.properties.insert("cac_texture".into(), texture);
            }
            None => {
                icon.properties.remove("cac_texture");
            }
        }
        Ok(())
    }
}

/// The movie's InitAction: the `hud_freeskateactivity_overlay` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 17, 0x123c))
}

/// Timeline actions besides `Stop`: root frame 0 constructs the class
/// (`var screen = new hud_freeskateactivity_overlay(this)`).
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x1214 => clip.calls.push("hud_freeskateactivity_overlay"),
        _ => return Err(format!("untranslated activity overlay APT action {offset:#x}").into()),
    }
    Ok(())
}
