//! Original unranked/freeskateactivity_signup (screen 108, asset 118, fingerprint 749ca799;
//! template entry `FreeskateActivity_SignUp` 0x82fdba18, placement 0) and its
//! `freeskateactivity_signup` class (InitAction character 13, `0xac4`), FE state 67's screen: its
//! `mPanel.mcChallengeInfo` (controls/challenge_info, asset 5) in `SetMode(5)`, the online
//! sign-up mode (`0x50c4`..`0x5280`), laid out by `UpdatePositions` (`0x3ec4`), and `SetState(0)`'s
//! `UpdateOnlineSigninTimer` (`0x5288`) of `Online_Signup_GetCurrentTime`. What the natives answer
//! comes from `skate2_core::online::activities::SignUpView`.
use super::{
    assets::Assets,
    clips::{Child, Clip},
    objective_movie::{set_x, set_y, text_field_width},
    pause_movie::{bounds, child, configure_left_buttons},
};
use crate::gpu_renderer::RenderResult;
use skate2_core::online::activities::SignUpView;
use skate2_formats::apt::Character;

pub(super) const ASSET: usize = 118;
/// `UpdatePositions`' button menu height in the online sign-up mode.
const VIEW_Y: f32 = 526.;

#[derive(Default)]
pub(super) struct ActivitySignUpMovie {
    pub root: Option<Clip>,
    /// FE state 67 is up: `OnEnter` opened screen 108 and nothing has closed it.
    open: bool,
    intro_complete: bool,
    /// What the screen shows.
    shown: Option<SignUpView>,
    /// The location's small image `mRenderImage0.LoadRX2Texture` names, for the owner to load.
    pub image_request: Option<String>,
}

impl ActivitySignUpMovie {
    /// `OpenScreen(108)` as FE state 67 enters: the screen anew, constructed, its intro playing;
    /// `SetState(0)` as the time left changes; the outro once the state leaves.
    pub fn update(&mut self, assets: &Assets, view: Option<&SignUpView>) -> RenderResult<()> {
        match view {
            Some(view) if !self.open => {
                self.open = true;
                self.intro_complete = false;
                let mut root = Clip::new(assets, ASSET, 0)?;
                let mut calls = Vec::new();
                root.take_calls(&mut calls);
                for call in calls {
                    match call {
                        "freeskateactivity_signup" => construct(&mut root, assets, view)?,
                        _ => return Err(format!("untranslated activity sign-up callback {call}").into()),
                    }
                }
                self.image_request = Some(format!("{}_sm", view.image));
                self.root = Some(root);
                self.shown = Some(view.clone());
            }
            Some(view) => {
                if self.shown.as_ref().is_some_and(|shown| shown.seconds != view.seconds)
                    && let Some(root) = self.root.as_mut()
                {
                    let timer = root.named(&["mPanel", "mcChallengeInfo", "mActivity", "mTimer", "timerText"])?;
                    timer.text = Some(view.seconds.to_string().encode_utf16().collect());
                }
                self.shown = Some(view.clone());
            }
            None if self.open => {
                self.open = false;
                self.shown = None;
                if let Some(root) = self.root.as_mut() {
                    root.goto(assets, "outro")?;
                }
            }
            None => {}
        }
        Ok(())
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        let Some(root) = self.root.as_mut() else { return Ok(()) };
        root.tick(assets)?;
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        for call in calls {
            match call {
                "IntroComplete" => self.intro_complete = true,
                "OutroComplete" => {
                    self.root = None;
                    break;
                }
                _ => return Err(format!("untranslated activity sign-up callback {call}").into()),
            }
        }
        Ok(())
    }

    /// The screen's `ScreenManager.IntroComplete` since the last call.
    pub fn take_intro_complete(&mut self) -> bool {
        std::mem::take(&mut self.intro_complete)
    }

    /// `mRenderImage0`'s texture once the owner has loaded it.
    pub fn set_image(&mut self, texture: Option<String>) -> RenderResult<()> {
        let Some(root) = self.root.as_mut() else { return Ok(()) };
        let image = root.named(&["mPanel", "mcChallengeInfo", "mRenderImage0"])?;
        match texture {
            Some(texture) => {
                image.properties.insert("cac_texture".into(), texture);
            }
            None => {
                image.properties.remove("cac_texture");
            }
        }
        Ok(())
    }
}

/// `new freeskateactivity_signup(this)`: `mcChallengeInfo.SetMode(mOnlineSignUp)` (5), the edge
/// offset (zero) added to the info's, the background's and the plastic's `_x`, then
/// `RegisterScreen`.
fn construct(root: &mut Clip, assets: &Assets, view: &SignUpView) -> RenderResult<()> {
    let panel = root.named(&["mPanel"])?;
    set_mode_online_signup(&mut child(panel, "mcChallengeInfo")?.clip, assets, view)
}

/// challenge_info `SetMode(5)`: the screen shown and every part disabled, then the sign-up's
/// data (`Online_Signup_GetData`): the activity's title over the location's small image, its
/// description and the type's description; while accepting, who proposed it and when it
/// expires; Accept or Propose with Cancel; then `UpdatePositions`.
fn set_mode_online_signup(info: &mut Clip, assets: &Assets, view: &SignUpView) -> RenderResult<()> {
    info.visible = true;
    for part in info.children.values_mut() {
        part.clip.visible = part.entry.name.as_deref().is_some_and(|name| name.eq_ignore_ascii_case("mHeader"));
    }
    let title = assets.initial_text(&view.title);
    info.named(&["mHeader", "labelText"])?.text = Some(title.clone());
    child(info, "mRenderImage0")?.clip.visible = true;
    info.named(&["mDescription", "labelText"])?.text = Some(assets.initial_text(&view.description));
    child(info, "mDescription")?.clip.visible = true;
    info.named(&["mPriceOrPurchased", "labelText"])?.text = Some(assets.initial_text(view.type_description));
    child(info, "mPriceOrPurchased")?.clip.visible = true;
    if view.accepting {
        let activity = info.named(&["mActivity"])?;
        activity.named(&["proposedText"])?.text = Some(assets.initial_text("ID_ONLINE_SIGNUP_PROPOSED_BY"));
        activity.named(&["proposedGamertag"])?.text = Some(view.gamer_tag.encode_utf16().collect());
        activity.named(&["expiresText"])?.text = Some(assets.initial_text("ID_ONLINE_SIGNUP_EXPIRES_IN"));
        activity.named(&["mTimer", "timerText"])?.text = Some(view.seconds.to_string().encode_utf16().collect());
        child(info, "mActivity")?.clip.visible = true;
    }
    let items: [(&str, &str); 2] = if view.accepting { [("A", "ID_COMMON_ACCEPT"), ("B", "ID_COMMON_CANCEL")] } else { [("A", "ID_ONLINE_SIGNUP_PROPOSE"), ("B", "ID_COMMON_CANCEL")] };
    configure_left_buttons(info.named(&["mView"])?, assets, &items)?;
    // mTitleOffsetX and mTitleOffsetY are the prototype's zeros; the mode's header sits 7 lower.
    let header = child(info, "mHeader")?;
    set_x(header, 0.);
    set_y(header, 7.);
    child(info, "mView")?.clip.visible = true;
    update_positions(info, assets, &title)
}

/// `UpdatePositions` (`0x3ec4`) for the parts the online sign-up mode shows: below the header
/// (10 apart), a header that wraps takes the `smalltext` frames and, fitting one line there,
/// `smalltextcenter`, the title set again each time; the image its height plus 10 lower; the
/// description 5 lower, 16 per line plus 15; the type's description 16 per line plus 5; the
/// proposal's lines auto-sized left with the time 10 past `Expires in:`; and the button menu at
/// 526.
fn update_positions(info: &mut Clip, assets: &Assets, title: &[u16]) -> RenderResult<()> {
    let header = child(info, "mHeader")?;
    let mut y = header.entry.matrix.ty + height(header, assets)? + 10.;
    let header = &mut header.clip;
    if lines(header.named(&["labelText"])?, assets)? > 1 {
        header.goto(assets, "smalltext")?;
        header.named(&["labelText"])?.text = Some(title.to_vec());
        if lines(header.named(&["labelText"])?, assets)? == 1 {
            header.goto(assets, "smalltextcenter")?;
            header.named(&["labelText"])?.text = Some(title.to_vec());
        }
    }
    let image = child(info, "mRenderImage0")?;
    set_y(image, y);
    y += height(image, assets)? + 10.;
    y += 5.;
    let description = child(info, "mDescription")?;
    set_y(description, y);
    #[expect(clippy::cast_precision_loss, reason = "a line count")]
    let description_lines = lines(description.clip.named(&["labelText"])?, assets)? as f32;
    y += (16. * description_lines).floor() + 15.;
    let kind = child(info, "mPriceOrPurchased")?;
    set_y(kind, y);
    #[expect(clippy::cast_precision_loss, reason = "a line count")]
    let kind_lines = lines(kind.clip.named(&["labelText"])?, assets)? as f32;
    y += (16. * kind_lines).floor() + 5.;
    if child(info, "mActivity")?.clip.visible {
        let activity = child(info, "mActivity")?;
        for field in ["proposedText", "proposedGamertag", "expiresText"] {
            child(&mut activity.clip, field)?.clip.properties.insert("text_autosize".into(), "left".into());
        }
        let expires = text_field_width(&child(&mut activity.clip, "expiresText")?.clip, assets)?;
        set_x(child(&mut activity.clip, "mTimer")?, expires + 10.);
        set_y(activity, y);
    }
    set_y(child(info, "mView")?, VIEW_Y);
    Ok(())
}

/// A text field's `maxscroll`: a multiline field's line count, 1 for a single line.
fn lines(field: &Clip, assets: &Assets) -> RenderResult<usize> {
    let Character::Text(text) = field.definition(assets)? else {
        return Err("activity sign-up maxscroll target is not text".into());
    };
    if !text.multiline {
        return Ok(1);
    }
    super::tutorial_movie::max_scroll(field, assets)
}

/// A piece's `_height`: its bounds through its own (unrotated) matrix.
fn height(piece: &Child, assets: &Assets) -> RenderResult<f32> {
    let r = bounds(&piece.clip, assets)?.ok_or("activity sign-up _height target has no bounds")?;
    Ok(((r.bottom - r.top) * piece.entry.matrix.d).abs())
}

/// The screen's InitAction: the `freeskateactivity_signup` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 13, 0xac4))
}

/// Timeline actions besides `Stop`: root frame 0 constructs the class
/// (`var screen = new freeskateactivity_signup(this)`), frame 19 stops on `IntroComplete` and
/// frame 34 on `OutroComplete`.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0xaa8 => clip.calls.push("freeskateactivity_signup"),
        0xcdc => {
            clip.playback.stop();
            clip.calls.push("IntroComplete");
        }
        0xd08 => {
            clip.playback.stop();
            clip.calls.push("OutroComplete");
        }
        _ => return Err(format!("untranslated activity sign-up APT action {offset:#x}").into()),
    }
    Ok(())
}
