//! Original hud2/dpo (asset 69, fingerprint ad28a23d; template entry `DPOHUD` 0x82fdb888,
//! placement 6, g3dHUDRight) and its `dpo` class (InitAction character 22, `0x438c`), driven by
//! `DPOHUD` (`skate2_core::challenge::turn_hud`): a challenge's current and target values with
//! its title and icon. Spot Battle's cumulative locations show the turn score over the best
//! score. `Show` only sets the screen's `_visible`, so the root's intro and outro labels never
//! play; `mTitle` has no `titleGlow_txt`, so its sets do nothing.
use super::{
    assets::Assets,
    clips::Clip,
    hud_leaderboard_movie::{auto_size, find_child, width},
    localizer,
    objective_movie::set_x,
};
use crate::gpu_renderer::RenderResult;
use skate2_core::challenge::{
    dpo_hud::{DpoParam, format},
    turn_hud::{DpoView, TurnHudView},
};

pub(super) const ASSET: usize = 69;
/// `kLAST_HIGH` and `kLAST_LOW`.
const LAST_HIGH: i32 = 0;
const LAST_LOW: i32 = 1;
/// `Update`'s `dpoicons` for `eDPOIcon` 1 to 12; any later icon loads the last.
const ICONS: [&str; 13] = [
    "hud-killed",
    "hud-personalbest",
    "hud-friends",
    "hud_fa_accumulation",
    "hud_fa_gap",
    "hud_fa_list",
    "hud_fa_simultaneous",
    "hud_fa_survival",
    "hud_fa_tag",
    "challenge_createaspot",
    "location_megaramp",
    "challenge_spotphoto",
    "challenge_spotfilm",
];

pub(super) struct DpoMovie {
    pub root: Clip,
    /// `mLastPosition`.
    last_position: i32,
    /// The `dpoicons` texture the last `LoadRX2Texture` named, for the owner to load.
    pub icon_request: Option<String>,
}

impl DpoMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut movie = Self { root: Clip::new(assets, ASSET, 0)?, last_position: LAST_LOW, icon_request: None };
        movie.calls(assets)?;
        Ok(movie)
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root.tick(assets)?;
        self.calls(assets)
    }

    fn calls(&mut self, assets: &Assets) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "dpo" => self.construct(assets)?,
                "nextDPO" => self.root.named(&["mDPO", "mPosHolder"])?.goto(assets, "nextDPO")?,
                _ => return Err(format!("untranslated DPO HUD callback {call}").into()),
            }
        }
        Ok(())
    }

    /// `new dpo(this)`: `_x -= Screen_EdgeOffset` (zero), `Show(false)`, `RegisterScreen`, the
    /// `high` place over a leaderboard of more than four players (none while the HUD loads),
    /// else `normal`, the value fields auto-sized right, then `AlignText`.
    fn construct(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root.visible = false;
        self.root.named(&["mDPO"])?.goto(assets, "normal")?;
        self.last_position = LAST_LOW;
        for name in ["numerator", "denominator", "seperator"] {
            auto_size(self.root.named(&["mDPO", "mPosHolder", "mParams", name, "labeltext"])?, assets, "right")?;
        }
        self.align_text(assets)
    }

    /// `SetState`: 1 and 2 show and hide the screen, 3 runs `Update`, and 10 to 13 play a
    /// status (none, close, failed, success) on `mPosHolder` and `mParams`.
    pub fn set_state(&mut self, assets: &Assets, state: i32, view: &TurnHudView) -> RenderResult<()> {
        let (holder, params) = match state {
            1 | 2 => {
                self.root.visible = state == 1;
                return Ok(());
            }
            3 => {
                let TurnHudView::Dpo(view) = view else {
                    return Err("DPO HUD state 3 without DPO data".into());
                };
                return self.update(assets, view);
            }
            10 => ("idle", "none"),
            11 => ("nextDPO", "none"),
            12 => ("idle", "failed"),
            13 => ("idle", "success"),
            _ => return Ok(()),
        };
        let holder_clip = self.root.named(&["mDPO", "mPosHolder"])?;
        holder_clip.goto(assets, holder)?;
        holder_clip.named(&["mParams"])?.goto(assets, params)
    }

    /// `Update`: the place for the leaderboard's players, then `DPO_GetParams`' values, title
    /// and icon, then `AlignText`.
    fn update(&mut self, assets: &Assets, view: &DpoView) -> RenderResult<()> {
        let high = view.leaderboard_players > 4;
        if high && self.last_position == LAST_LOW {
            self.root.named(&["mDPO"])?.goto(assets, "high")?;
            self.last_position = LAST_HIGH;
        } else if !high && self.last_position == LAST_HIGH {
            self.root.named(&["mDPO"])?.goto(assets, "normal")?;
            self.last_position = LAST_LOW;
        }
        let data = &view.data;
        let params = self.root.named(&["mDPO", "mPosHolder", "mParams"])?;
        params.named(&["numerator", "labeltext"])?.text = Some(first_text(assets, data.formatted_first)?);
        params.named(&["denominator", "labeltext"])?.text = Some(second_text(assets, data.formatted_second)?);
        self.root.named(&["mDPO", "mPosHolder", "mTitle", "title_txt"])?.text = Some(assets.initial_text(&data.title));
        // PC port: a Skate 3 game's DPO shows no icon, as Skate 3's movie has none.
        self.root.named(&["mDPO", "mPosHolder", "icon_mc"])?.visible = !view.skate3;
        self.icon_request = if view.skate3 { None } else { icon_name(data.icon, &view.challenge_icon) };
        self.align_text(assets)
    }

    /// `AlignText`: the separator ends where the denominator's text starts, and the numerator
    /// where the separator's does.
    fn align_text(&mut self, assets: &Assets) -> RenderResult<()> {
        let params = self.root.named(&["mDPO", "mPosHolder", "mParams"])?;
        let denominator = find_child(params, "denominator").ok_or("DPO HUD denominator absent")?.entry.matrix.tx;
        let seperator = denominator - label_width(params, "denominator", assets)?;
        set_x(find_child(params, "seperator").ok_or("DPO HUD seperator absent")?, seperator);
        let numerator = seperator - label_width(params, "seperator", assets)?;
        set_x(find_child(params, "numerator").ok_or("DPO HUD numerator absent")?, numerator);
        Ok(())
    }

    /// `mImageComp0`'s `LoadRX2Texture` once the owner has loaded the named icon.
    pub fn set_icon(&mut self, texture: Option<String>) -> RenderResult<()> {
        let image = self.root.named(&["mDPO", "mPosHolder", "icon_mc", "mImageComp0"])?;
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

/// A value field's `labeltext._width`.
fn label_width(params: &mut Clip, name: &str, assets: &Assets) -> RenderResult<f32> {
    let field = find_child(params, name).ok_or("DPO HUD value field absent")?;
    let label = find_child(&mut field.clip, "labeltext").ok_or("DPO HUD labeltext absent")?;
    width(label, assets)
}

/// The icon `Update` loads: for `Challenge` (0) the main challenge's, a `mini` name read as its
/// `challenge` one, and none without a challenge; the fixed icons otherwise.
fn icon_name(icon: i32, challenge: &str) -> Option<String> {
    if icon < 1 {
        if challenge == "nochallenge" {
            return None;
        }
        if challenge.len() > 5 && challenge.get(..4) == Some("mini") {
            return Some(format!("challenge{}", &challenge[4..]));
        }
        return Some(challenge.to_owned());
    }
    let index = usize::try_from(icon - 1).map_or(0, |i| i.min(ICONS.len() - 1));
    Some(ICONS[index].to_owned())
}

/// `DPOHUDData::SetParams` (`0x826b3338`)'s first value: `FormatNumber` for counts and points,
/// `ID_COMMON_DISTANCE` for distances and `FormatTime` for times; the movie adds the `#`, so
/// nothing shows before a value is set.
#[expect(clippy::cast_possible_truncation, reason = "the native (int) conversion")]
fn first_text(assets: &Assets, param: Option<DpoParam>) -> RenderResult<Vec<u16>> {
    let Some(DpoParam { format, value }) = param else { return Ok(Vec::new()) };
    Ok(match format {
        format::DISTANCE => localizer::format_custom(assets, "ID_COMMON_DISTANCE", &(value as i32).to_string())?,
        format::TIME => time_text(assets, value)?.encode_utf16().collect(),
        _ => localizer::format_int(assets, value as i32).encode_utf16().collect(),
    })
}

/// Its second value, `#`-prefixed: `FormatString(ID_COMMON_NUMBER, int)` (`{0:d}`, grouped by
/// `FormatNumber`) for counts, `FormatStringCustom` of `ID_DPO_STR_POINTS` for points and of
/// `ID_COMMON_DISTANCE` for distances, and `FormatTime` for times. Before a value is set the
/// empty string names no string, which shows the missing-string space.
#[expect(clippy::cast_possible_truncation, reason = "the native (int) conversion")]
fn second_text(assets: &Assets, param: Option<DpoParam>) -> RenderResult<Vec<u16>> {
    let Some(DpoParam { format, value }) = param else { return Ok(assets.initial_text("")) };
    let whole = (value as i32).to_string();
    Ok(match format {
        format::POINTS => localizer::format_custom(assets, "ID_DPO_STR_POINTS", &whole)?,
        format::DISTANCE => localizer::format_custom(assets, "ID_COMMON_DISTANCE", &whole)?,
        format::TIME => time_text(assets, value)?.encode_utf16().collect(),
        _ => localizer::format_string_int(assets, "ID_COMMON_NUMBER", value as i32)?,
    })
}

/// `SetParams`' split of the seconds into `FormatTime(hours, minutes, seconds, hundredths, …,
/// 0)` (`0x829812f0`) through `BuildTimeFormatString` (`0x82980e30`) with the English
/// `hh:mm:ss.ff`: hours when there are any, minutes and seconds (two digits once a larger unit
/// shows), and two-digit hundredths without hours.
#[expect(clippy::cast_possible_truncation, clippy::cast_precision_loss, reason = "the native conversions")]
fn time_text(assets: &Assets, total: f32) -> RenderResult<String> {
    if assets.localize("ID_COMMON_TIME_FORMAT")? != "hh:mm:ss.ff" {
        return Err("unsupported DPO HUD time pattern".into());
    }
    let hours = (total * 0.000_277_777_78) as i32;
    let minutes = (total * 0.016_666_668 - (60 * hours) as f32) as i32;
    let seconds = ((total - (3600 * hours) as f32) - (60 * minutes) as f32) as i32;
    let hundredths = ((total - (total as i32) as f32) * 100.0) as i32;
    let mut text = String::new();
    let mut following = false;
    if hours != 0 {
        text.push_str(&format!("{hours:02}:"));
        following = true;
    }
    if minutes != 0 || following {
        text.push_str(&format!("{minutes:02}:"));
        following = true;
    }
    if seconds != 0 || hundredths != 0 || following {
        text.push_str(&format!("{seconds:02}"));
        following = true;
    }
    if hours == 0 && (hundredths != 0 || following) {
        text.push_str(&format!(".{hundredths:02}"));
    }
    Ok(text)
}

/// The movie's InitAction: the `dpo` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 22, 0x438c))
}

/// Timeline actions besides `Stop`: root frame 0 constructs the class (`var screen = new
/// dpo(this)`), and `mPosHolder`'s frame 69 plays `nextDPO` again.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x4370 => clip.calls.push("dpo"),
        0x4b10 => clip.calls.push("nextDPO"),
        _ => return Err(format!("untranslated DPO HUD APT action {offset:#x}").into()),
    }
    Ok(())
}
