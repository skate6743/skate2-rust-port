//! Original hudComponents/spectator_mode (asset 116, fingerprint a968491a; `OnlineSpectatorOverlay`,
//! screen 29) and its `spectator_mode` class (InitAction character 12, `0x128c`), the skate.TV
//! overlay of the debug build's observer, which a free skate's Spectate (PC port addition) puts up
//! while the console watches another player: `Mission.GetViewingPlayerName` is that player, and
//! the first control line, an observer's camera line, is B's Stop Spectating with a Button Item
//! where the observer's D-pad would be.
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::apt_timeline::IDENTITY;
use skate2_formats::apt::Matrix;

pub(super) const ASSET: usize = 116;
/// The control line's text (PC port addition).
const STOP: &str = "#Stop Spectating";
/// The Button Item in `mcTextLeftJustified`: its depth past the authored ones, its scale against
/// the 18-unit control line, and its centre left of the line's text, at its middle.
const BUTTON_DEPTH: i32 = 13;
const BUTTON_SCALE: f32 = 0.6;
const BUTTON_X: f32 = -185.;
const BUTTON_Y: f32 = 4.6;

pub(super) struct SpectatorMovie {
    pub root: Clip,
    /// The name shown while the overlay is up.
    shown: Option<String>,
}

impl SpectatorMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        Ok(Self { root: Clip::new(assets, ASSET, 0)?, shown: None })
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root.tick(assets)
    }

    /// `SetState(kStateActive)` as the console starts watching `name`, a new name as `Refresh`
    /// reads it every `kUpdateCycles` frames, and `SetState(kStateHide)` as it stops.
    pub fn update(&mut self, assets: &Assets, name: Option<&str>) -> RenderResult<()> {
        if name == self.shown.as_deref() {
            return Ok(());
        }
        match name {
            Some(name) => {
                if self.shown.is_none() {
                    self.show(assets)?;
                }
                let line = self.root.named(&["mcTextLeftJustified", "mcTextName", "labelText"])?;
                line.text = Some(assets.initial_text(&format!("#{name}")));
            }
            None => self.root.goto(assets, "outro")?,
        }
        self.shown = name.map(str::to_owned);
        Ok(())
    }

    /// `Show(true)`: `intro`, the title and `Watching:` labels, `Mission.GetIsLocalPlayerObserver`
    /// false (the D-pad and camera line off), the next two lines' players off as there is no other
    /// to step to, and the first line B's Stop Spectating.
    fn show(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root.goto(assets, "intro")?;
        let group = self.root.named(&["mcTextLeftJustified"])?;
        for (name, text) in [("mcTextSpecMode", "ID_SPECTATOR_MODE_TITLE"), ("mcTextViewing", "ID_SPECTATOR_MODE_VIEWING"), ("mcTextControls1", STOP)] {
            let line = group.named(&[name])?;
            line.visible = true;
            line.named(&["labelText"])?.text = Some(assets.initial_text(text));
        }
        for name in ["mcTextControls2", "mcTextControls3"] {
            group.named(&[name])?.visible = false;
        }
        if !group.children.contains_key(&BUTTON_DEPTH) {
            let character = assets.movies[1]
                .movie
                .exports
                .iter()
                .find(|export| export.name == "Button Item")
                .map(|export| export.character_id)
                .ok_or("Button Item export absent")?;
            let mut button = Clip::new(assets, 1, character)?;
            button.named(&["mButtonRender"])?.properties.insert("_test".into(), "B".into());
            let matrix = Matrix { a: BUTTON_SCALE, d: BUTTON_SCALE, tx: BUTTON_X, ty: BUTTON_Y, ..IDENTITY };
            group.children.insert(BUTTON_DEPTH, super::online_hub_movie::placed(button, character, "mcStopButton", matrix));
        }
        Ok(())
    }
}

/// First-frame DoInitAction defines the guarded `spectator_mode` class, whose methods
/// `SpectatorMovie` above supplies.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 12, 0x128c))
}

pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        // `screen = new spectator_mode(this)` (whose constructor registers the screen), then
        // `_visible = false` and `Stop`.
        0x1250 => {
            clip.visible = false;
            clip.playback.stop();
        }
        // `intro`'s first frame: `_visible = true`.
        0x1624 => clip.visible = true,
        // `outro`'s last: `Stop`, `_visible = false`.
        0x1644 => {
            clip.playback.stop();
            clip.visible = false;
        }
        _ => return Err(format!("untranslated spectator overlay action {offset:#x}").into()),
    }
    Ok(())
}
