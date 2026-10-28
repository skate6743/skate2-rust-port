//! Live original marker, trick-score and Hall of Meat HUDs. Authored APT
//! timelines, translated screen controllers and completed facts have separate owners.
mod assets;
mod clips;
mod score_movie;
mod hom_movie;
mod apt_random;
mod pause_movie;
mod map_movie;
mod jacked_movie;
mod central_movie;
mod minimap_movie;
mod localizer;
mod objective_movie;
mod tutorial_movie;
mod action_overlay_movie;
mod progression_popup;
mod signup_poster_movie;
mod skate101_movie;
mod hom_results_movie;
mod hom_annals_movie;
mod hom_skeleton_movie;
mod photo_select_movie;
mod receipt_movie;
mod subtitle_movie;
mod gestollie_movie;
mod ownthespot_movie;
mod spotfound_movie;
mod killedit_movie;
mod phototreatment_movie;
mod pro_bio_movie;
mod gvr_movie;
mod sanvantv_movie;
mod unlock_overlay_movie;
mod sponsor_overlay_movie;
mod slappys_movie;
mod skate_profile_movie;
mod letterbox;
mod main_movie;
mod extras_movie;
mod dlc_movie;
mod replay_editor_movie;
mod trick_guide_movie;
mod contacts_movie;
mod replay_footage_movie;
mod skate_reel_movie;
mod credits_movie;
mod fmv_tutorial_movie;
mod freeskate_movie;
mod music_player_movie;
mod chyron_movie;
mod party_play_movie;
mod party_play_press_movie;
mod throwdown_results_movie;
mod hud_timer_movie;
mod big_black_timer_movie;
mod hud_throwmes_movie;
mod hud_abd_movie;
mod hud_leaderboard_movie;
mod hud_countdown_movie;
mod hud_dpo_movie;
mod hud_distance_movie;
mod hud_voting_movie;
mod online_eventlog_movie;
mod online_match_movie;
mod lobby_browser_movie;
mod online_hub_movie;
mod online_results_movie;
mod spectator_movie;
mod activity_overlay_movie;
mod activity_signup_movie;
pub(crate) mod online_names;
mod end_challenge_movie;
mod results_list;
mod challenge_failed_movie;
mod challenge_failed_small_movie;
mod end_challenge_small_movie;
mod party_play_change_movie;
mod settings_movie;
mod stick_preview;
mod screen_manager;
mod edit_skater_movie;
use crate::gpu_renderer::{
    GpuRenderer, GpuScene, RenderResult,
    marker_ui::{Batch, Textures},
};
use assets::Assets;
use clips::Clip;
pub(crate) use screen_manager::ScreenEvents;
use screen_manager::{Dimmer, Screen, ScreenManager};
use skate2_core::challenge::system::MissionTitle;
use skate2_core::nis::fe_overlays::{
    FIXED_PHOTO_TREATMENT, GVR_GOOFY, GVR_REGULAR, NON_FIXED_PHOTO_TREATMENT, PHOTO_TREATMENT, PRO_BIO_INFO, SAN_VAN_TV,
    SPOT_FOUND, SPOT_KILLED, SPOT_OWNED, ScreenCommand, TUTORIAL_GESTURE_OLLIE,
};
use skate2_core::ui::{
    apt_playback::MovieClock,
    apt_timeline::{IDENTITY, concatenate},
    phonelist::*,
};
use skate2_formats::{
    apt::{Character, GeometryKind, Matrix},
    mount::MountSet,
};
use std::{
    num::NonZeroU32,
    time::{Duration, Instant},
};

/// A `FrontEndState_NIS` overlay screen's movie.
enum NisScreen {
    GestureOllie(gestollie_movie::GestOllieMovie),
    SpotOwned(ownthespot_movie::OwnTheSpotMovie),
    SpotFound(spotfound_movie::SpotFoundMovie),
    KilledIt(killedit_movie::KilledItMovie),
    PhotoTreatment(phototreatment_movie::PhotoTreatmentMovie),
    ProBio(pro_bio_movie::ProBioMovie),
    Gvr(gvr_movie::GvrMovie),
    SanVanTv(sanvantv_movie::SanVanTvMovie),
    /// A template whose movie is not translated.
    Unported,
}

impl NisScreen {
    fn root(&self) -> Option<&Clip> {
        match self {
            Self::GestureOllie(movie) => Some(&movie.root),
            Self::SpotOwned(movie) => Some(&movie.root),
            Self::SpotFound(movie) => Some(&movie.root),
            Self::KilledIt(movie) => Some(&movie.root),
            Self::PhotoTreatment(movie) => Some(&movie.root),
            Self::ProBio(movie) => Some(&movie.root),
            Self::Gvr(movie) => Some(&movie.root),
            Self::SanVanTv(movie) => Some(&movie.root),
            Self::Unported => None,
        }
    }

    fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        match self {
            Self::GestureOllie(movie) => movie.tick(assets),
            Self::SpotOwned(movie) => movie.tick(assets),
            Self::SpotFound(movie) => movie.tick(assets),
            Self::KilledIt(movie) => movie.tick(assets),
            Self::PhotoTreatment(movie) => movie.tick(assets),
            Self::ProBio(movie) => movie.tick(assets),
            Self::Gvr(movie) => movie.tick(assets),
            Self::SanVanTv(movie) => movie.tick(assets),
            Self::Unported => Ok(()),
        }
    }

    /// The movie's `Audio.PlaySound` requests, for the screens that make any.
    fn sounds(&mut self) -> Option<&mut Vec<&'static str>> {
        match self {
            Self::SpotOwned(movie) => Some(&mut movie.sounds),
            Self::SpotFound(movie) => Some(&mut movie.sounds),
            Self::KilledIt(movie) => Some(&mut movie.sounds),
            Self::PhotoTreatment(movie) => Some(&mut movie.sounds),
            Self::ProBio(movie) => Some(&mut movie.sounds),
            Self::GestureOllie(_) | Self::Gvr(_) | Self::SanVanTv(_) | Self::Unported => None,
        }
    }

    fn set_state(&mut self, assets: &Assets, state: i32, title: &MissionTitle, pro: &[String; 3]) -> RenderResult<()> {
        match self {
            Self::GestureOllie(movie) => movie.set_state(assets, state),
            Self::SpotOwned(movie) => movie.set_state(assets, state),
            Self::SpotFound(movie) => movie.set_state(assets, state, title),
            Self::KilledIt(movie) => movie.set_state(assets, state),
            Self::PhotoTreatment(movie) => movie.set_state(assets, state),
            Self::ProBio(movie) => movie.set_state(assets, state, pro),
            Self::Gvr(movie) => movie.set_state(assets, state),
            Self::SanVanTv(movie) => movie.set_state(assets, state),
            Self::Unported => Ok(()),
        }
    }
}

/// `StringHash64("default")`, the default `worldpainter_fe_location_triggers` key.
const DEFAULT_ZONE: u64 = 0xd7ed_bd36_2d7d_2152;

pub(crate) struct MarkerUi {
    defer_geometry: bool,
    geometry_pending: bool,
    settings: settings_movie::SettingsMovie,
    /// The music player Options pushes (FE state 30).
    music_player: music_player_movie::MusicPlayerMovie,
    /// screenmanager.apt's queues, through which the StartHub's, Options' and the music player's
    /// screens open.
    screens: ScreenManager,
    /// mainmisc.swf's `gDimmer`, which the `mbDimmer` of the state on top drives.
    dimmer: Dimmer,
    main: Option<main_movie::MainMovie>,
    extras: extras_movie::ExtrasMovie,
    /// PC: FE state 25's screen, the linked downloadable content.
    dlc: dlc_movie::DlcMovie,
    replay_editor: replay_editor_movie::ReplayEditorMovie,
    trick_guide:trick_guide_movie::TrickGuideMovie,
    contacts:contacts_movie::ContactsMovie,
    can_show_phone_options:bool,
    trick_guide_letterbox:letterbox::LetterBox,
    replay_footage: replay_footage_movie::FootageMovie,
    skate_reel:[skate_reel_movie::Movie;3],
    fmv: fmv_tutorial_movie::FmvTutorialMovie,
    /// FreeSkateOption's movie on the front end's APT layer.
    freeskate: freeskate_movie::FreeskateMovie,
    /// PartyPlay's movie on the front end's APT layer.
    party_play: party_play_movie::PartyPlayMovie,
    assets: Assets,
    root: Clip,
    score: score_movie::ScoreMovie,
    hom: hom_movie::HomMovie,
    pause: pause_movie::PauseMovie,
    map: map_movie::MapMovie,
    jacked: jacked_movie::JackedMovie,
    central: central_movie::CentralMovie,
    minimap: minimap_movie::MiniMapMovie,
    objective: objective_movie::ObjectiveMovie,
    tutorial: tutorial_movie::TutorialMovie,
    action_overlay: action_overlay_movie::ActionOverlayMovie,
    progression_popup: progression_popup::ProgressionPopup,
    name_popup: progression_popup::ProgressionPopup,
    /// PC: the main menu's Downloadable Content notice.
    dlc_popup: progression_popup::ProgressionPopup,
    /// PC: the main menu's update popup (`skate2_core::ui::update_prompt`).
    update_popup: progression_popup::ProgressionPopup,
    /// The several-challenges popup (FE state 34 over the sign-up HUD).
    sign_up_choice: progression_popup::ProgressionPopup,
    /// The lobby chat's prompt (T, PC port addition), in the cheat code prompt's popup.
    chat_popup: progression_popup::ProgressionPopup,
    signup_poster: signup_poster_movie::SignUpPosterMovie,
    unlock_overlay: unlock_overlay_movie::UnlockOverlayMovie,
    sponsor_overlay: sponsor_overlay_movie::SponsorOverlayMovie,
    slappys: slappys_movie::SlappysMovie,
    /// FE state 45's screen 93, skate.Profile: Career over the StartHub.
    skate_profile: skate_profile_movie::SkateProfileMovie,
    skate101: skate101_movie::Skate101Movie,
    hom_results:hom_results_movie::HomResultsMovie,
    hom_annals:hom_annals_movie::AnnalsMovie,
    photo_select: photo_select_movie::PhotoSelectMovie,
    /// `FrontEndState_Receipt`'s screen 122 over the editor.
    receipt: receipt_movie::ReceiptMovie,
    /// `FrontEndState_PressA`'s screen 64.
    party_play_press: party_play_press_movie::PartyPlayPressMovie,
    /// `FrontEndState_EndChallengeOptions`' screen 47 for a throwdown.
    throwdown_results: throwdown_results_movie::ThrowdownResultsMovie,
    end_challenge: end_challenge_movie::EndChallengeMovie,
    /// `FrontEndState_EndChallengeOptions`' screen 42 for a win without several players.
    end_small: end_challenge_small_movie::EndChallengeSmallMovie,
    /// `FrontEndState_EndChallengeOptions`' screen 44 for a loss with several players.
    failed: challenge_failed_movie::ChallengeFailedMovie,
    /// `FrontEndState_EndChallengeOptions`' screen 45 for a loss without several players.
    failed_small: challenge_failed_small_movie::ChallengeFailedSmallMovie,
    /// APT's `Math.random` stream, which every movie's script shares.
    random: apt_random::AptRandom,
    change_game: party_play_change_movie::PartyPlayChangeMovie,
    /// The turn-based challenges' HUD components: `TimerHUD`, `ThrowDownMessageHUD`, `ABDHUD`,
    /// `LeaderboardHUD`, `CountdownHUD` and `OnlineVotingHUD`.
    timer: hud_timer_movie::TimerMovie,
    big_black_timer:big_black_timer_movie::BigBlackTimerMovie,
    /// `DistanceMeter`'s hud2/huddistance.
    distance: hud_distance_movie::DistanceMovie,
    throwdown_message: hud_throwmes_movie::ThrowDownMessageMovie,
    abd: hud_abd_movie::AbdMovie,
    leaderboard: hud_leaderboard_movie::LeaderboardMovie,
    dpo: hud_dpo_movie::DpoMovie,
    countdown: hud_countdown_movie::CountdownMovie,
    voting: hud_voting_movie::VotingMovie,
    /// `HUD2_OnlineEventLog`, loaded with the online HUD group while an online game runs.
    online_event_log: online_eventlog_movie::OnlineEventLogMovie,
    online_event_log_open: bool,
    /// The skate.TV overlay while a free skate's Spectate watches another player (PC port addition).
    spectator: spectator_movie::SpectatorMovie,
    /// `FreeskateActivityOverlayHUD` while a freeskate activity is proposed, and FE state 67's
    /// sign-up screen.
    activity_overlay: activity_overlay_movie::ActivityOverlayMovie,
    activity_signup: activity_signup_movie::ActivitySignUpMovie,
    /// `SetOnlineHudsVisible(false)`: the activity overlay hides the event log.
    online_event_log_hidden: bool,
    /// FE states 69 and 68's screens, which replace the Online options.
    online_custom: online_match_movie::OnlineMatchMovie,
    online_freeskate: online_match_movie::OnlineMatchMovie,
    /// The Lobby Browser, in place of the Online options.
    lobby_browser: lobby_browser_movie::LobbyBrowserMovie,
    online_match_intro: bool,
    /// FE state 68 is up in the main menu.
    online_freeskate_open: bool,
    /// The main menu's screens with `sFEStateInfo::mbDimmer`: the name prompts, FE states 68
    /// and 69 and the Lobby Browser on 69's screen.
    menu_dim: bool,
    /// FE state 44's screen, the online hub.
    online_hub: online_hub_movie::OnlineHubMovie,
    /// FE state 68 or 69 is up in the game (`mbDimmer`).
    online_hub_dim: bool,
    /// FE state 8's online display, screen 117.
    online_results: online_results_movie::OnlineResultsMovie,
    subtitle: subtitle_movie::SubtitleMovie,
    /// `FrontEndState_MoviePlayer` (FE state 28) is on the stack: `FEMoviePlayer` drives the
    /// subtitle line and the state's `gFEStateInfo` row hides the HUD.
    movie_open: bool,
    /// main.swf's `gChyron`: the music player's now-playing card.
    chyron: chyron_movie::ChyronMovie,
    /// `FrontEndState_NIS`'s overlay screens by screen manager id; the
    /// screen manager's layer draws them in that (depth) order.
    nis_screens: std::collections::BTreeMap<i32, NisScreen>,
    minimap_direction: skate2_core::ui::minimap::Direction,
    minimap_frame: skate2_core::ui::minimap::Frame,
    minimap_icons: Vec<skate2_core::ui::minimap::Icon>,
    minimap_heading: f32,
    district_key: Option<u64>,
    /// `FrontEndManager::mZoneKey` (+0x25E0): the default key from the constructor, then the
    /// location triggers `ReceiveMsg` keeps; kept across games like the front end manager.
    zone_key: u64,
    minimap_enabled: bool,
    /// A free skate game's profile `mHUD` and `mMiniMap`: `StartInGame` shows the HUD by it
    /// (`eHUDVisibilityFlag_GameSettings`, `HudManagerSetVisibleOverride` once toggled) and
    /// `MiniMap::Show` (0x826bcc50) the minimap.
    free_skate_settings: Option<(bool, bool)>,
    minimap_before_bail: bool,
    /// What the career's HUD manager bound the free-skate components to.
    hud_binding: skate2_core::challenge::hud::HudBinding,
    editor: edit_skater_movie::EditSkaterMovie,
    editor_view: Option<skate2_core::ui::edit_skater::EditSkaterView>,
    jacked_request: bool,
    map_facts: MapRenderFacts,
    pause_state: skate2_core::ui::pause_menu::PauseMenuSnapshot,
    hom_hud: skate2_core::ui::hom_scoring::HomScoringHud,
    /// The front end's one spin (`AptIntegration`), which turns every `g3dSpinContainer`
    /// screen: the unlock overlay and the end-of-challenge screens.
    spin: skate2_core::ui::apt_projection::Spin,
    jolt: skate2_core::ui::apt_jolt::HudJolt,
    textures: Textures,
    fps_font: String,
    fps_cached: Option<(u32, (u32, u32))>,
    /// The career's HUD state (objective, sign-up prompt, poster, choice).
    career: Option<skate2_core::CareerHud>,
    hud: PhoneListHud,
    data: PhoneListData,
    /// The game's side of the phone's alerts, as the career last gave it.
    alert_facts: AlertFacts,
    previous: MarkerHudSnapshot,
    /// The movies' `Audio.PlaySound` requests of the frames since `take_sounds`.
    pending_sounds: Vec<&'static str>,
    clock: MovieClock,
    last: Option<Instant>,
    trace: bool,
}
impl MarkerUi {
    pub(crate) fn set_free_skate_settings(&mut self, settings: Option<(bool, bool)>) {
        self.free_skate_settings = settings;
    }
    /// The Stick Settings pages' boxes: the playing controller's sticks as its pad reports them.
    pub(crate) fn set_stick_preview(&mut self, sticks: Option<[i16; 4]>) {
        self.settings.set_stick_preview(sticks);
    }
    pub(crate) fn set_options_menu(&mut self, state: skate2_core::ui::options::Snapshot) -> RenderResult<()> {
        use skate2_core::ui::options::Page;
        let closing = state.page == Page::Hidden && self.settings.state.page != Page::Hidden;
        let pending = self.settings.pending();
        self.settings.update(&self.assets, state, self.main.as_mut().map(|m| &mut m.letterbox))?;
        self.screens.track(Screen::Options, pending, self.settings.pending());
        // ExecutePopState82d672f8: the main menu's Show(true) -> OnGainFocus.
        if closing { if let Some(main)=self.main.as_mut() { main.gain_focus(&self.assets)?; } }
        self.sync_dimmer()?;
        self.collect_sounds();
        Ok(())
    }
    pub(crate) fn set_replay_editor(&mut self,state:Option<&skate2_core::ui::replay_editor::Snapshot>,states:&[u32],
        reel:Option<&skate2_core::ui::skate_reel::Snapshot>)->RenderResult<()> {
        let pending=self.replay_editor.pending();
        let footage=state.and_then(|s|s.footage.as_ref()).or_else(||reel.and_then(|s|s.footage.as_ref()));
        self.replay_editor.update(&self.assets,state.filter(|s|s.footage.is_none()),states)?;
        self.screens.track(Screen::ReplayEditor,pending,self.replay_editor.pending());
        let pending=self.replay_footage.pending();
        let popup=state.filter(|s|s.footage.is_some()).and_then(|s|s.popup.as_ref())
            .or_else(||reel.filter(|s|s.footage.is_some()).and_then(|s|s.popup.as_ref()));
        self.replay_footage.update(&self.assets,footage,popup)?;
        self.screens.track(Screen::ReplayFootage,pending,self.replay_footage.pending());
        self.sync_dimmer()
    }
    pub(crate) fn set_trick_guide(&mut self,state:Option<&skate2_core::ui::trick_guide::Snapshot>)->RenderResult<()> {
        let pending=self.trick_guide.pending();
        self.trick_guide.update(&self.assets,state,&mut self.trick_guide_letterbox)?;
        self.screens.track(Screen::TrickGuide,pending,self.trick_guide.pending());
        Ok(())
    }
    /// FE state 45, skate.Profile: Career over the StartHub: its screen and input.
    pub(crate) fn set_skate_profile(&mut self, state: Option<&skate2_core::ui::skate_profile::View>) -> RenderResult<()> {
        let pending = self.skate_profile.pending();
        self.skate_profile.update(&self.assets, state)?;
        self.screens.track(Screen::SkateProfile, pending, self.skate_profile.pending());
        self.sync_dimmer()
    }
    pub(crate) fn update_trick_guide(&mut self,renderer:&GpuRenderer,scene:&mut GpuScene,now:Instant,
        pause:&skate2_core::ui::pause_menu::PauseMenuSnapshot)->RenderResult<()> {
        // StartHub OnEnter8265e4d0 opens a fresh screen after TrickGuide leaves.
        // Retire its previous movie while this separate room owns rendering, so
        // the return produces a new IntroComplete and releases the input gate.
        self.apply_pause(pause,false)?;
        self.dimmer.enable(&self.assets,false)?;
        self.advance_movies(now)?;self.draw(renderer,scene)
    }
    pub(crate) fn set_music_player(&mut self, state: &skate2_core::ui::music_player::Snapshot) -> RenderResult<()> {
        let pending = self.music_player.pending();
        self.music_player.update(&self.assets, state)?;
        self.screens.track(Screen::MusicPlayer, pending, self.music_player.pending());
        self.sync_dimmer()
    }
    /// What the StartHub's, Options' and the music player's screens told their states since the
    /// last call.
    pub(crate) fn take_screen_events(&mut self) -> ScreenEvents {
        ScreenEvents {
            start_hub_intro: self.pause.take_intro_complete(),
            replay_editor_intro: self.replay_editor.take_intro_complete(),
            trick_guide_intro:std::mem::take(&mut self.trick_guide.intro),
            contacts_intro:std::mem::take(&mut self.contacts.intro),
            skate_profile_intro: std::mem::take(&mut self.skate_profile.intro),
            replay_editor_ready: self.replay_editor.take_editor_ready(),
            replay_footage_intro: self.replay_footage.take_intro(),
            skate_reel_intro:std::array::from_fn(|i|self.skate_reel[i].take_intro()),
            options_intro: self.settings.take_intro_complete(),
            music_player_loaded: self.music_player.take_loaded(),
            music_player_intro: self.music_player.take_intro_complete(),
        }
    }
    /// The front-end state on top, then the songs this frame shows on the chyron
    /// (`MusicPlayerManager::Play`'s restarts and the music system's `UpdateChyron`).
    /// `FrontEndManager::OpenOnlineHuds` (the online HUD group, with the event log) while an
    /// online game runs, and the log's `Refresh` (`eASCall_UpdateOnlineEventLog`) when it changed.
    pub(crate) fn update_online_event_log(&mut self, open: bool, log: Option<&skate2_core::online::event_log::EventLogSnapshot>) -> RenderResult<()> {
        if open && !self.online_event_log_open {
            self.online_event_log = online_eventlog_movie::OnlineEventLogMovie::new(&self.assets)?;
        }
        self.online_event_log_open = open;
        if let Some(log) = log.filter(|_| open) {
            self.online_event_log.refresh(&self.assets, log)?;
        }
        Ok(())
    }

    /// The skate.TV overlay naming the player a free skate's Spectate watches, while it does (PC
    /// port addition).
    pub(crate) fn update_online_spectator(&mut self, name: Option<&str>) -> RenderResult<()> {
        self.spectator.update(&self.assets, name)
    }

    /// The freeskate activity overlay while a proposal shows it, with its type's icon loaded; FE
    /// state 67's screen while it is up, with the location's small image; and whether the overlay
    /// hides the event log. Whether the screen's intro completed.
    pub(crate) fn update_online_activities(
        &mut self,
        renderer: &GpuRenderer,
        overlay: Option<&skate2_core::online::activities::OverlayView>,
        sign_up: Option<&skate2_core::online::activities::SignUpView>,
        log_hidden: bool,
    ) -> RenderResult<bool> {
        self.activity_overlay.update(&self.assets, overlay)?;
        if let Some(name) = self.activity_overlay.icon_request.take() {
            let texture = match self.assets.ensure_dpo_icon(&name) {
                Ok((key, loaded)) => {
                    if loaded {
                        self.textures.extend(renderer.upload_ui_textures(self.assets.textures.get(&key).into_iter())?);
                    }
                    Some(key)
                }
                Err(_) => None,
            };
            self.activity_overlay.set_icon(texture)?;
        }
        self.activity_signup.update(&self.assets, sign_up)?;
        if let Some(image) = self.activity_signup.image_request.take() {
            let texture = match self.assets.ensure_location_texture(&image) {
                Ok((key, loaded)) => {
                    if loaded {
                        self.textures.extend(renderer.upload_ui_textures(self.assets.textures.get(&key).into_iter())?);
                    }
                    Some(key)
                }
                Err(_) => None,
            };
            self.activity_signup.set_image(texture)?;
        }
        self.online_event_log_hidden = log_hidden;
        Ok(self.activity_signup.take_intro_complete())
    }

    /// The lobby skate HUD's leaderboard ("lobbyhud"): its screen states on the challenge HUD's
    /// leaderboard movie, titled by `Mission.GetTitle()` in lobby skate.
    pub(crate) fn update_online_lobby_hud(&mut self, states: &[(i32, skate2_core::challenge::turn_hud::TurnHudView)],
        title: &skate2_core::challenge::system::MissionTitle) -> RenderResult<()> {
        for (state, view) in states {
            self.leaderboard.set_state(&self.assets, *state, view, title)?;
        }
        Ok(())
    }

    /// `SceneRenderView::Render`'s `DrawName` (0x827d7fe0) for this frame's named skaters, drawn
    /// into the scene after its meshes.
    pub(crate) fn update_online_names(&self, renderer: &GpuRenderer, scene: &mut GpuScene,
        tags: &[online_names::NameTag<'_>], view: &online_names::NameView) -> RenderResult<()> {
        let batches = online_names::batches(&self.assets, tags, view, renderer.dimensions())?;
        scene.online_names = renderer.upload_ui_batches(&self.textures, batches)?;
        Ok(())
    }

    /// `FrontEndManager::Render2D` (0x82633be0) draws the chase arrows while no `HUDManager2`
    /// visibility flag hides the HUD: not in the front-end states that hide it, nor over the trick
    /// guide and the replay screens.
    pub(crate) fn chase_arrows_shown(&self) -> bool {
        !(self.trick_guide.open() || self.replay_editor.open() || self.replay_footage.open()
            || self.hud_hidden() || self.hud_binding.hidden)
    }

    /// `ChaseArrowMan::Render` (0x82631638): each arrow's quad white on the front buffer, which the
    /// window fits as it fits the 3D view, with its texture from the HUD's icons.
    pub(crate) fn update_chase_arrows(&self, renderer: &GpuRenderer, scene: &mut GpuScene,
        quads: &[skate2_core::ui::chase_arrows::Quad]) -> RenderResult<()> {
        let size = renderer.dimensions();
        let mut batches = Vec::new();
        for quad in quads {
            // A texture the pres asset registry lacks draws nothing.
            let Ok(texture) = self.assets.texture(quad.texture) else { continue };
            let vertices = [0, 1, 2, 0, 2, 3].map(|index| {
                let corner = quad.corners[index];
                vertex(IDENTITY, [corner[0], corner[1]], [corner[2], corner[3]], [1.0, 1.0, 1.0, quad.alpha], size, true)
            });
            batches.push(Batch { texture: Some(texture.name.clone()), wrapped: false, vertices: vertices.to_vec() });
        }
        scene.chase_arrows = renderer.upload_ui_batches(&self.textures, batches)?;
        Ok(())
    }

    /// The lobby chat's prompt while it is open (its line with the cursor and the characters
    /// left), or the single player hub's nickname prompt, as the Online options show theirs.
    pub(crate) fn update_online_chat(&mut self, prompt: Option<&skate2_core::online::chat::ChatPrompt>, nickname: Option<&skate2_core::ui::profile_name::Prompt>) -> RenderResult<()> {
        if let Some(prompt) = nickname {
            let mut field = prompt.text.clone();
            field.insert(prompt.cursor, '|');
            let error = prompt.error.as_deref().unwrap_or("");
            let dialog = ("#Online Nickname".to_owned(), format!("#Please enter a nickname for online play:\n\n{field}\n\n{error}"), 0);
            return self.chat_popup.update(&self.assets, Some(&dialog), Some(&["#Enter: Confirm".to_owned(), "#Esc: Cancel".to_owned()]));
        }
        let dialog = prompt.map(|prompt| {
            // Ctrl+A's selection shows the line on the selection's blue, with no cursor.
            let field = if prompt.selected {
                let mark = |unit: u16| char::from_u32(u32::from(unit)).unwrap_or(' ');
                format!("{}{}{}", mark(TEXT_HIGHLIGHT_START), prompt.text, mark(TEXT_HIGHLIGHT_END))
            } else {
                let mut field = prompt.text.clone();
                field.insert(prompt.cursor, '|');
                field
            };
            let limit = skate2_net::protocol::CHAT_CHARACTERS;
            ("#Lobby Chat".to_owned(), format!("#{field}\n\n{}/{limit}", prompt.text.len()), 0)
        });
        self.chat_popup.update(&self.assets, dialog.as_ref(), Some(&["#Enter: Send    Esc: Cancel".to_owned()]))
    }

    /// FE state 44 (the online hub) in the game: its screen and `SetScreenState` calls, the Host
    /// Options screen over it (FE state 68 or 69, dimmed), the popup over them or the game, and
    /// the hub's central messages.
    #[expect(clippy::too_many_arguments, reason = "the hub, its screen calls, the screen over it, the popup and its messages")]
    pub(crate) fn update_online_hub(&mut self, renderer: &GpuRenderer, hub: Option<&skate2_core::ui::online_hub::Snapshot>, opened: bool,
        calls: &[skate2_core::ui::online_hub::ScreenCall], screen: Option<&skate2_core::ui::online_match::Snapshot>,
        popup: Option<&skate2_core::ui::online_hub::Popup>, nickname: &str, central: &[skate2_core::challenge::hud::CentralMessage]) -> RenderResult<()> {
        self.online_hub.update(&self.assets, hub, opened, calls, popup)?;
        if self.main.is_none() {
            if update_online_match_movies(&mut self.assets, &mut self.textures, renderer, [&mut self.online_custom, &mut self.online_freeskate], screen, nickname)? {
                self.online_match_intro = true;
            }
            self.online_hub_dim = screen.is_some();
            self.sync_dimmer()?;
        }
        for message in central {
            self.central.show_formatted(&self.assets, &message.hal_id, &message.args, true)?;
        }
        Ok(())
    }

    /// Whether the online hub's intro completed since the last call.
    pub(crate) fn take_online_hub_intro(&mut self) -> bool {
        self.online_hub.take_intro_complete()
    }

    /// Whether an online matchmaking screen's or the Lobby Browser's intro completed since the
    /// last call.
    pub(crate) fn take_online_match_intro(&mut self) -> bool {
        std::mem::take(&mut self.online_match_intro)
    }

    pub(crate) fn update_chyron(&mut self, state: i32, songs: &[u32]) -> RenderResult<()> {
        self.chyron.enter(&self.assets, state)?;
        for &song in songs {
            self.chyron.display(&self.assets, song)?;
        }
        Ok(())
    }
    /// `StateGameUnload`'s `PopAllHUDStates` and `HUDManager2::ExitToFE` 0x82cee828: every HUD
    /// component unloads with the game's screens, so the next game loads them new
    /// (`HUDManager2::Load`) over a reset HUD state.
    pub(crate) fn exit_to_front_end(&mut self) -> RenderResult<()> {
        let assets = &self.assets;
        self.root = Clip::new(assets, 0, 0)?;
        self.score = score_movie::ScoreMovie::new(assets)?;
        self.hom = hom_movie::HomMovie::new(assets)?;
        self.pause = pause_movie::PauseMovie::new(assets)?;
        self.replay_editor = replay_editor_movie::ReplayEditorMovie::new(assets)?;
        self.trick_guide=trick_guide_movie::TrickGuideMovie::new(assets)?;
        self.contacts=contacts_movie::ContactsMovie::new(assets)?;self.can_show_phone_options=false;
        self.trick_guide_letterbox=letterbox::LetterBox::new(assets)?;
        self.replay_footage = replay_footage_movie::FootageMovie::new(assets)?;
        self.skate_reel = [skate_reel_movie::Movie::new(assets,97)?,skate_reel_movie::Movie::new(assets,98)?,skate_reel_movie::Movie::new(assets,99)?];
        self.map = map_movie::MapMovie::new(assets)?;
        self.jacked = jacked_movie::JackedMovie::new(assets)?;
        self.central = central_movie::CentralMovie::new(assets)?;
        self.minimap = minimap_movie::MiniMapMovie::new(assets)?;
        self.objective = objective_movie::ObjectiveMovie::new(assets)?;
        self.tutorial = tutorial_movie::TutorialMovie::new(assets)?;
        self.action_overlay = action_overlay_movie::ActionOverlayMovie::new(assets)?;
        self.subtitle = subtitle_movie::SubtitleMovie::new(assets)?;
        self.progression_popup = progression_popup::ProgressionPopup::default();
        self.name_popup = progression_popup::ProgressionPopup::default();
        self.dlc_popup = progression_popup::ProgressionPopup::default();
        self.update_popup = progression_popup::ProgressionPopup::default();
        self.sign_up_choice = progression_popup::ProgressionPopup::default();
        self.chat_popup = progression_popup::ProgressionPopup::default();
        self.signup_poster = signup_poster_movie::SignUpPosterMovie::default();
        self.unlock_overlay = unlock_overlay_movie::UnlockOverlayMovie::default();
        self.sponsor_overlay = sponsor_overlay_movie::SponsorOverlayMovie::default();
        self.slappys = slappys_movie::SlappysMovie::default();
        self.skate_profile = skate_profile_movie::SkateProfileMovie::default();
        self.spin = skate2_core::ui::apt_projection::Spin::default();
        self.skate101 = skate101_movie::Skate101Movie::default();
        self.hom_results=hom_results_movie::HomResultsMovie::default();
        self.hom_annals=hom_annals_movie::AnnalsMovie::default();
        self.photo_select = photo_select_movie::PhotoSelectMovie::default();
        self.party_play_press = party_play_press_movie::PartyPlayPressMovie::default();
        self.throwdown_results = throwdown_results_movie::ThrowdownResultsMovie::default();
        self.end_challenge = end_challenge_movie::EndChallengeMovie::default();
        self.end_small = end_challenge_small_movie::EndChallengeSmallMovie::default();
        self.failed = challenge_failed_movie::ChallengeFailedMovie::default();
        self.failed_small = challenge_failed_small_movie::ChallengeFailedSmallMovie::default();
        self.change_game = party_play_change_movie::PartyPlayChangeMovie::default();
        self.timer = hud_timer_movie::TimerMovie::new(assets)?;
        self.big_black_timer=big_black_timer_movie::BigBlackTimerMovie::new(assets)?;
        self.distance = hud_distance_movie::DistanceMovie::new(assets)?;
        self.throwdown_message = hud_throwmes_movie::ThrowDownMessageMovie::new(assets)?;
        self.abd = hud_abd_movie::AbdMovie::new(assets)?;
        self.leaderboard = hud_leaderboard_movie::LeaderboardMovie::new(assets)?;
        self.dpo = hud_dpo_movie::DpoMovie::new(assets)?;
        self.countdown = hud_countdown_movie::CountdownMovie::new(assets)?;
        self.voting = hud_voting_movie::VotingMovie::new(assets)?;
        self.online_event_log = online_eventlog_movie::OnlineEventLogMovie::new(assets)?;
        self.spectator = spectator_movie::SpectatorMovie::new(assets)?;
        self.activity_overlay = activity_overlay_movie::ActivityOverlayMovie::new(assets)?;
        self.activity_signup = activity_signup_movie::ActivitySignUpMovie::default();
        self.online_event_log_hidden = false;
        self.online_custom = online_match_movie::OnlineMatchMovie::new(assets, online_match_movie::CUSTOM)?;
        self.online_freeskate = online_match_movie::OnlineMatchMovie::new(assets, online_match_movie::FREESKATE)?;
        self.lobby_browser = lobby_browser_movie::LobbyBrowserMovie::new(assets)?;
        self.online_freeskate_open = false;
        self.menu_dim = false;
        self.online_hub = online_hub_movie::OnlineHubMovie::new(assets)?;
        self.online_hub_dim = false;
        self.online_results = online_results_movie::OnlineResultsMovie::default();
        self.nis_screens.clear();
        self.minimap_direction = Default::default();
        self.minimap_frame = Default::default();
        self.minimap_icons.clear();
        self.minimap_heading = 0.;
        self.district_key = None;
        self.minimap_enabled = true;
        self.minimap_before_bail = true;
        self.jacked_request = false;
        self.map_facts = MapRenderFacts { snapshot: Default::default(), locations: Vec::new(), player: None };
        self.pause_state = Default::default();
        self.hom_hud = Default::default();
        self.jolt = Default::default();
        self.career = None;
        self.hud = Default::default();
        self.data = Default::default();
        self.alert_facts = AlertFacts::default();
        self.previous = Default::default();
        Ok(())
    }
    pub(crate) fn close_main_menu(&mut self)->RenderResult<()>{
        let Some(mut main)=self.main.take() else { return Ok(()) };
        self.screens.cancel(Screen::MainMenu);
        self.screens.cancel(Screen::Extras);
        self.screens.cancel(Screen::Dlc);
        for screen in [Screen::OnlineOptions,Screen::SkateReelGeneral,Screen::SkateReelPhoto] {self.screens.cancel(screen);}
        self.pending_sounds.append(&mut main.letterbox.sounds);
        self.pending_sounds.append(&mut self.freeskate.sounds);
        self.freeskate=freeskate_movie::FreeskateMovie::new(&self.assets)?;
        self.party_play=party_play_movie::PartyPlayMovie::new(&self.assets)?;
        self.extras=extras_movie::ExtrasMovie::new(&self.assets)?;
        self.dlc=dlc_movie::DlcMovie::new(&self.assets)?;
        self.skate_reel=[skate_reel_movie::Movie::new(&self.assets,97)?,skate_reel_movie::Movie::new(&self.assets,98)?,skate_reel_movie::Movie::new(&self.assets,99)?];
        Ok(())
    }
    /// The main menu's frame: its intro state and whether FreeSkateOption's and PartyPlay's
    /// intros completed.
    pub(crate) fn update_main_menu(&mut self,renderer:&GpuRenderer,scene:&mut GpuScene,now:Instant,
        frame:&skate2_core::ui::main_menu::Frame,freeskate_states:&[u32],party_play_states:&[u32])->RenderResult<(bool,bool,bool)>{
        self.geometry_pending = false;
        if self.main.is_none(){self.main=Some(main_movie::MainMovie::new(&self.assets)?);}
        self.advance_movies(now)?;
        if frame.skate_reel.as_ref().is_some_and(|s|s.page==skate2_core::ui::skate_reel::Page::General) {
            for path in skate_reel_movie::IMAGES {
                let (key,loaded)=self.assets.ensure_rx2_image(path)?;
                if loaded {self.textures.extend(renderer.upload_ui_textures(self.assets.textures.get(&key).into_iter())?);}
            }
        }
        if let Some(skate2_core::ui::skate_reel::Page::Photo(slot))=frame.skate_reel.as_ref().map(|s|s.page) {
            let (key,loaded)=self.assets.ensure_replay_photo(slot)?;
            if loaded {self.textures.extend(renderer.upload_ui_textures(self.assets.textures.get(&key).into_iter())?);}
        }
        self.resolve_footage_textures(renderer)?;
        self.menu_dim=frame.name_prompt.is_some()||frame.nickname_prompt.is_some()||frame.online_match.is_some()||frame.lobby_browser.is_some();
        self.sync_dimmer()?;
        let main=self.main.as_mut().expect("main movie created");
        let pending=main.pending();
        main.update(&self.assets,frame.menu_visible,frame.selected,frame.new_profile)?;
        self.screens.track(Screen::MainMenu,pending,main.pending());
        let pending=self.extras.pending();
        self.extras.update(&self.assets,frame.extras.as_ref(),&mut main.letterbox)?;
        self.screens.track(Screen::Extras,pending,self.extras.pending());
        let pending=self.dlc.pending();
        self.dlc.update(&self.assets,frame.dlc.as_ref())?;
        self.screens.track(Screen::Dlc,pending,self.dlc.pending());
        // OnMenuSelect (0x82674b58) takes the letterbox away (EnableLetterBox(0)) before
        // GoToCustomFreeSkate pushes FE state 68; the Online options bring it back when entered.
        let freeskate=frame.online_match.as_ref().is_some_and(|screen|screen.freeskate);
        if freeskate&&!self.online_freeskate_open {main.letterbox.enable(&self.assets,false)?;}
        self.online_freeskate_open=freeskate;
        for (reel,screen) in self.skate_reel.iter_mut().zip([Screen::OnlineOptions,Screen::SkateReelGeneral,Screen::SkateReelPhoto]) {
            // Pushing FE state 68 or 69 leaves MainMenuOnlineOptions, whose OnLeave (0x8264b6d0)
            // closes screen 9; popping back enters it again, which opens it anew.
            let state=frame.skate_reel.as_ref().filter(|_|screen!=Screen::OnlineOptions||(frame.online_match.is_none()&&frame.lobby_browser.is_none()));
            let pending=reel.pending();reel.update(&self.assets,state,&mut main.letterbox)?;
            self.screens.track(screen,pending,reel.pending());
        }
        let name_dialog=frame.name_prompt.as_ref().map(|prompt| {
            let mut field=prompt.text.clone();field.insert(prompt.cursor,'|');
            let error=prompt.error.as_deref().unwrap_or("");
            ("ID_KEYBOARD_PROFILE_NAME_HEADER".to_owned(),
                format!("#It looks like you are on a fresh save, please enter a name for your skater:\n\n{field}\n\n{error}"),0)
        }).or_else(|| frame.nickname_prompt.as_ref().map(|prompt| {
            let mut field=prompt.text.clone();field.insert(prompt.cursor,'|');
            let error=prompt.error.as_deref().unwrap_or("");
            ("#Online Nickname".to_owned(),
                format!("#Please enter a nickname for online play:\n\n{field}\n\n{error}"),0)
        }));
        let name_buttons:Vec<String>=if frame.name_prompt.is_none()&&frame.nickname_prompt.is_some() {
            vec!["#Enter: Confirm".into(),"#Esc: Cancel".into()]
        } else {vec!["#Enter: Confirm".into()]};
        self.name_popup.update(&self.assets,name_dialog.as_ref(),Some(&name_buttons))?;
        let dlc_popup=dlc_movie::DlcMovie::popup(frame.dlc.as_ref());
        self.dlc_popup.update(&self.assets,dlc_popup.as_ref().map(|(dialog,_)|dialog),dlc_popup.as_ref().map(|(_,options)|options.as_slice()))?;
        self.update_popup.show(&self.assets,frame.update_prompt.as_ref().map(skate2_core::ui::update_prompt::UpdatePrompt::dialog))?;
        self.freeskate.update(&self.assets,frame.freeskate.as_ref(),freeskate_states,&mut main.letterbox)?;
        // LoadRX2Texture: the location's large image, once the screen asks for it.
        let image=match frame.party_play.as_ref().and_then(|party_play|party_play.image.as_deref()) {
            Some(image)=>match self.assets.ensure_location_texture(image) {
                Ok((key,loaded))=>{
                    if loaded {
                        let uploaded=renderer.upload_ui_textures(self.assets.textures.get(&key).into_iter())?;
                        self.textures.extend(uploaded);
                    }
                    Some(key)
                }
                Err(_)=>None,
            },
            None=>None,
        };
        self.party_play.update(&self.assets,frame.party_play.as_ref(),party_play_states,&mut main.letterbox,image.as_deref())?;
        if update_online_match_movies(&mut self.assets,&mut self.textures,renderer,[&mut self.online_custom,&mut self.online_freeskate],
            frame.online_match.as_ref(),&frame.online_nickname)? {self.online_match_intro=true;}
        if update_lobby_browser_movie(&mut self.assets,&mut self.textures,renderer,&mut self.lobby_browser,frame.lobby_browser.as_ref())? {
            self.online_match_intro=true;
        }
        let intro=main.intro_complete;
        let freeskate_intro=self.freeskate.take_intro_complete();
        let party_play_intro=self.party_play.take_intro_complete();
        let mut batches=Vec::new();
        // Options' push keeps the main menu (mbKeepPrevious) hidden: ExecutePushState's Show(false)
        // hides its screen, the pop's Show(true) shows it at once under Options' outro.
        if !self.settings.on_stack() {
            // ReadyScreen assigns increasing depths. A new main menu on Back
            // draws above the departing Extras screen, just as Extras draws
            // above the old main menu when entering it.
            let mut menus=[(Screen::MainMenu,&main.root),(Screen::Extras,&self.extras.root),(Screen::OnlineOptions,&self.skate_reel[0].root)];
            menus.sort_by(|(a,_),(b,_)|if self.screens.over(*a,*b) {std::cmp::Ordering::Greater} else if self.screens.over(*b,*a) {std::cmp::Ordering::Less} else {std::cmp::Ordering::Equal});
            for (_,clip) in menus {
                flatten_projected(&self.assets,clip,IDENTITY,Colour::default(),(1280,720),&mut batches,true,None)?;
            }
            flatten_projected(&self.assets,&self.freeskate.root,IDENTITY,Colour::default(),(1280,720),&mut batches,true,None)?;
            flatten_projected(&self.assets,&self.party_play.root,IDENTITY,Colour::default(),(1280,720),&mut batches,true,None)?;
        }
        let clip=crate::gpu_renderer::CameraRelativeClip::new(
            &crate::live_skating_view::window_clip_from_world(frame.clip,renderer.dimensions()),
            [frame.camera[12],frame.camera[13],frame.camera[14]]);
        // Render3D8262a550 draws the APT quad, then the secondary texture's at alpha .7.
        let tint=f32::from_bits(0x3f34fdf4);
        let quad=world_quad(&clip,&frame.wall,frame.scale,[tint,tint,tint,1.]);
        let secondary=match frame.freeskate.as_ref().and_then(|freeskate|freeskate.image.as_deref()) {
            Some(image)=>{
                let (key,loaded)=self.assets.ensure_location_texture(image)?;
                if loaded {
                    let uploaded=renderer.upload_ui_textures(self.assets.textures.get(&key).into_iter())?;
                    self.textures.extend(uploaded);
                }
                let (wall,scale)=&frame.secondary;
                Some((key,world_quad(&clip,wall,*scale,[tint,tint,tint,f32::from_bits(0x3f333333)])))
            }
            None=>None,
        };
        renderer.update_world_ui(scene,&self.textures,batches,quad,secondary)?;
        // The dimmer (mainmisc.swf's _level8) under Options in main.swf's gScreenManager (depth 12),
        // then gLetterBox (14) and gChyron (15). The DLC screen's template entry (0x82fdb3f8) is
        // placement 0 as Options' is, not the wall's 1.
        let mut overlay=Vec::new();
        if self.movie_open {
            // main.swf's gSubtitle precedes loaded screen layers.
            flatten_projected(&self.assets,&self.subtitle.root,IDENTITY,Colour::default(),renderer.dimensions(),&mut overlay,true,None)?;
        }
        for clip in [&self.dimmer.clip,&self.settings.root,&self.dlc.root] {
            flatten_projected(&self.assets,clip,IDENTITY,Colour::default(),renderer.dimensions(),&mut overlay,true,None)?;
        }
        let mut reel_screens=[(Screen::SkateReelGeneral,&self.skate_reel[1].root,&self.skate_reel[1].popup),
            (Screen::ReplayFootage,&self.replay_footage.root,&self.replay_footage.popup),
            (Screen::SkateReelPhoto,&self.skate_reel[2].root,&self.skate_reel[2].popup)];
        reel_screens.sort_by(|(a,_,_),(b,_,_)|if self.screens.over(*a,*b) {std::cmp::Ordering::Greater} else if self.screens.over(*b,*a) {std::cmp::Ordering::Less} else {std::cmp::Ordering::Equal});
        for (_,root,popup) in reel_screens {
            flatten_projected(&self.assets,root,IDENTITY,Colour::default(),renderer.dimensions(),&mut overlay,true,None)?;
            if let Some(popup)=&popup.popup {flatten_projected(&self.assets,popup,IDENTITY,Colour::default(),renderer.dimensions(),&mut overlay,true,None)?;}
        }
        for clip in [&self.online_custom.root,&self.online_freeskate.root,&self.lobby_browser.root] {
            flatten_projected(&self.assets,clip,IDENTITY,Colour::default(),renderer.dimensions(),&mut overlay,true,None)?;
        }
        if let Some(popup)=&self.lobby_browser.popup.popup {
            flatten_projected(&self.assets,popup,IDENTITY,Colour::default(),renderer.dimensions(),&mut overlay,true,None)?;
        }
        if let Some(popup)=&self.skate_reel[0].popup.popup {flatten_projected(&self.assets,popup,IDENTITY,Colour::default(),renderer.dimensions(),&mut overlay,true,None)?;}
        for clip in [&main.letterbox.clip,&self.chyron.root] {
            flatten_projected(&self.assets,clip,IDENTITY,Colour::default(),renderer.dimensions(),&mut overlay,true,None)?;
        }
        flatten_projected(&self.assets,&self.extras.credits.root,IDENTITY,Colour::default(),renderer.dimensions(),&mut overlay,true,None)?;
        if let Some(tutorial) = &self.fmv.tutorial {
            flatten_projected(&self.assets,tutorial,fmv_tutorial_movie::PLACEMENT,Colour::default(),renderer.dimensions(),&mut overlay,true,None)?;
        }
        if let Some(popup) = &self.extras.popup.popup {
            flatten_projected(&self.assets,popup,IDENTITY,Colour::default(),renderer.dimensions(),&mut overlay,true,None)?;
        }
        if let Some(popup) = &self.name_popup.popup {
            flatten_projected(&self.assets,popup,IDENTITY,Colour::default(),renderer.dimensions(),&mut overlay,true,None)?;
        }
        if let Some(popup) = &self.dlc_popup.popup {
            flatten_projected(&self.assets,popup,IDENTITY,Colour::default(),renderer.dimensions(),&mut overlay,true,None)?;
        }
        if let Some(popup) = &self.update_popup.popup {
            flatten_projected(&self.assets,popup,IDENTITY,Colour::default(),renderer.dimensions(),&mut overlay,true,None)?;
        }
        renderer.update_marker_ui(scene,&self.textures,overlay)?;
        self.finish_frame(renderer,scene)?;
        Ok((intro,freeskate_intro,party_play_intro))
    }
    pub fn take_extras_events(&mut self) -> (bool, bool, bool) { self.extras.take_events() }
    /// The Downloadable Content screen's `IntroComplete` since the last call.
    pub fn take_dlc_intro(&mut self) -> bool { self.dlc.take_intro() }
    /// The mounted downloadable content's strings join the screens' (the game's own first).
    pub fn add_dlc_strings(&mut self, mounts: &MountSet) { self.assets.add_dlc_strings(mounts); }
    pub fn take_jacked_request(&mut self)->bool{std::mem::take(&mut self.jacked_request)}
    /// The sign-up poster screen's `IntroComplete` since the last call.
    pub fn take_poster_intro_complete(&mut self) -> bool {
        self.signup_poster.take_intro_complete()
    }
    /// The unlock overlay screen's `IntroComplete` since the last call.
    pub fn take_unlock_intro_complete(&mut self) -> bool {
        self.unlock_overlay.take_intro_complete()
    }
    /// The unlock overlay's `Cash_SetCashAnimDone` since the last call.
    pub fn take_cash_anim_done(&mut self) -> bool {
        self.unlock_overlay.take_cash_anim_done()
    }
    /// Slappy's blog's top screen's `IntroComplete` since the last call.
    pub fn take_slappys_intro_complete(&mut self) -> bool {
        self.slappys.take_intro_complete()
    }
    /// The sponsor overlay screen's `IntroComplete` and `OutroComplete` since the last call.
    pub fn take_sponsor_screen(&mut self) -> (bool, bool) {
        self.sponsor_overlay.take_screen()
    }
    /// The Skate 101 screen's `IntroComplete` since the last call.
    pub fn take_skate101_intro_complete(&mut self) -> bool {
        self.skate101.take_intro_complete()
    }
    /// The change-game screen's `IntroComplete` since the last call.
    pub fn take_change_game_intro(&mut self) -> bool {
        self.change_game.take_intro_complete()
    }
    /// The end-of-challenge screen's `IntroComplete` and `OutroComplete` since the last call.
    pub fn take_end_options_screen(&mut self) -> (bool, bool) {
        let intro = self.throwdown_results.take_intro_complete()
            | self.online_results.take_intro_complete()
            | self.end_challenge.take_intro_complete()
            | self.end_small.take_intro_complete()
            | self.failed.take_intro_complete()
            | self.failed_small.take_intro_complete();
        let outro = self.throwdown_results.take_outro_complete()
            | self.online_results.take_outro_complete()
            | self.end_challenge.take_outro_complete()
            | self.end_small.take_outro_complete()
            | self.failed.take_outro_complete()
            | self.failed_small.take_outro_complete();
        (intro, outro)
    }
    /// The `Audio.PlaySound` arguments the movies produced since the last call.
    pub(crate) fn take_sounds(&mut self) -> Vec<skate2_core::ui::FeSoundRequest> {
        let mut sounds:Vec<_>=std::mem::take(&mut self.pending_sounds).into_iter().map(skate2_core::ui::FeSoundRequest::Play).collect();
        sounds.append(&mut self.hom_results.sounds);
        sounds.extend(std::mem::take(&mut self.hom_annals.sounds).into_iter().map(skate2_core::ui::FeSoundRequest::Play));sounds
    }
    pub fn take_hom_annals_intro_complete(&mut self)->bool {std::mem::take(&mut self.hom_annals.ready)}
    pub fn take_hom_results_intro_complete(&mut self)->bool {std::mem::take(&mut self.hom_results.ready)}
    /// Moves the requests out of the movies, which clear their lists on their next update.
    /// Frames can outnumber the ticks that take them.
    fn collect_sounds(&mut self) {
        let sounds = &mut self.pending_sounds;
        for list in [&mut self.score.sounds, &mut self.hom.sounds, &mut self.pause.sounds, &mut self.map.sounds,
            &mut self.central.sounds, &mut self.minimap.sounds, &mut self.objective.sounds, &mut self.editor.sounds,
            &mut self.dimmer.sounds, &mut self.settings.letterbox.sounds, &mut self.freeskate.sounds,
            &mut self.throwdown_message.sounds, &mut self.abd.sounds,
            &mut self.countdown.sounds, &mut self.sponsor_overlay.sounds, &mut self.slappys.sounds,
            &mut self.replay_footage.sounds,&mut self.trick_guide_letterbox.sounds] {
            sounds.append(list);
        }
        if let Some(main) = self.main.as_mut() {
            sounds.append(&mut main.letterbox.sounds);
        }
        for reel in &mut self.skate_reel {sounds.append(&mut reel.sounds);}
        for list in self.nis_screens.values_mut().filter_map(NisScreen::sounds) {
            sounds.append(list);
        }
    }
    /// The tail of every frame: its sound requests, then the FPS overlay.
    fn finish_frame(&mut self, renderer: &GpuRenderer, scene: &mut GpuScene) -> RenderResult<()> {
        self.collect_sounds();
        self.update_fps(renderer, scene)
    }
    pub fn load(mounts: &MountSet, renderer: &GpuRenderer) -> RenderResult<Self> {
        let assets = Assets::load(mounts)?;
        let root = Clip::new(&assets, 0, 0)?;
        let mut score = score_movie::ScoreMovie::new(&assets)?;
        let hom = hom_movie::HomMovie::new(&assets)?;
        let pause = pause_movie::PauseMovie::new(&assets)?;
        let settings = settings_movie::SettingsMovie::new(&assets)?;
        let map = map_movie::MapMovie::new(&assets)?;
        let jacked = jacked_movie::JackedMovie::new(&assets)?;
        let central = central_movie::CentralMovie::new(&assets)?;
        let minimap = minimap_movie::MiniMapMovie::new(&assets)?;
        let objective = objective_movie::ObjectiveMovie::new(&assets)?;
        let tutorial = tutorial_movie::TutorialMovie::new(&assets)?;
        let action_overlay = action_overlay_movie::ActionOverlayMovie::new(&assets)?;
        let editor = edit_skater_movie::EditSkaterMovie::new(&assets)?;
        let subtitle = subtitle_movie::SubtitleMovie::new(&assets)?;
        let chyron = chyron_movie::ChyronMovie::new(&assets)?;
        let timer = hud_timer_movie::TimerMovie::new(&assets)?;
        let big_black_timer=big_black_timer_movie::BigBlackTimerMovie::new(&assets)?;
        let distance = hud_distance_movie::DistanceMovie::new(&assets)?;
        let throwdown_message = hud_throwmes_movie::ThrowDownMessageMovie::new(&assets)?;
        let abd = hud_abd_movie::AbdMovie::new(&assets)?;
        let leaderboard = hud_leaderboard_movie::LeaderboardMovie::new(&assets)?;
        let countdown = hud_countdown_movie::CountdownMovie::new(&assets)?;
        let dpo = hud_dpo_movie::DpoMovie::new(&assets)?;
        let voting = hud_voting_movie::VotingMovie::new(&assets)?;
        let online_event_log = online_eventlog_movie::OnlineEventLogMovie::new(&assets)?;
        let spectator = spectator_movie::SpectatorMovie::new(&assets)?;
        let activity_overlay = activity_overlay_movie::ActivityOverlayMovie::new(&assets)?;
        let online_custom = online_match_movie::OnlineMatchMovie::new(&assets, online_match_movie::CUSTOM)?;
        let online_freeskate = online_match_movie::OnlineMatchMovie::new(&assets, online_match_movie::FREESKATE)?;
        let lobby_browser = lobby_browser_movie::LobbyBrowserMovie::new(&assets)?;
        let online_hub = online_hub_movie::OnlineHubMovie::new(&assets)?;
        let label = score.root.named(&["mcTrickAnim0", "mcTrickName", "mcText", "labelText"])?;
        let Character::Text(text) = label.definition(&assets)? else {
            return Err("trick name is not text".into());
        };
        let Some(Character::Font { name, .. }) = &assets.movies[label.asset].movie.characters[text.font_id as usize] else {
            return Err("trick name font absent".into());
        };
        let fps_font = name.clone();
        let textures = renderer.upload_ui_textures(assets.textures.values())?;
        Ok(Self {
            defer_geometry: false,
            geometry_pending: false,
            settings,
            music_player: music_player_movie::MusicPlayerMovie::new(&assets)?,
            screens: ScreenManager::default(),
            dimmer: Dimmer::new(&assets)?,
            main:None,
            extras: extras_movie::ExtrasMovie::new(&assets)?,
            dlc: dlc_movie::DlcMovie::new(&assets)?,
            replay_editor: replay_editor_movie::ReplayEditorMovie::new(&assets)?,
            trick_guide:trick_guide_movie::TrickGuideMovie::new(&assets)?,
            contacts:contacts_movie::ContactsMovie::new(&assets)?,can_show_phone_options:false,
            trick_guide_letterbox:letterbox::LetterBox::new(&assets)?,
            replay_footage: replay_footage_movie::FootageMovie::new(&assets)?,
            skate_reel:[skate_reel_movie::Movie::new(&assets,97)?,skate_reel_movie::Movie::new(&assets,98)?,skate_reel_movie::Movie::new(&assets,99)?],
            fmv: Default::default(),
            freeskate: freeskate_movie::FreeskateMovie::new(&assets)?,
            party_play: party_play_movie::PartyPlayMovie::new(&assets)?,
            assets,
            root,
            score,
            hom,
            pause,
            map,
            jacked,
            central,
            minimap,
            objective,
            tutorial,
            action_overlay,
            progression_popup: progression_popup::ProgressionPopup::default(),
            name_popup: progression_popup::ProgressionPopup::default(),
            dlc_popup: progression_popup::ProgressionPopup::default(),
            update_popup: progression_popup::ProgressionPopup::default(),
            sign_up_choice: progression_popup::ProgressionPopup::default(),
            chat_popup: progression_popup::ProgressionPopup::default(),
            signup_poster: signup_poster_movie::SignUpPosterMovie::default(),
            unlock_overlay: unlock_overlay_movie::UnlockOverlayMovie::default(),
            sponsor_overlay: sponsor_overlay_movie::SponsorOverlayMovie::default(),
            slappys: slappys_movie::SlappysMovie::default(),
            skate_profile: skate_profile_movie::SkateProfileMovie::default(),
            skate101: skate101_movie::Skate101Movie::default(),
            hom_results:hom_results_movie::HomResultsMovie::default(),
            hom_annals:hom_annals_movie::AnnalsMovie::default(),
            photo_select: photo_select_movie::PhotoSelectMovie::default(),
            receipt: receipt_movie::ReceiptMovie::default(),
            party_play_press: party_play_press_movie::PartyPlayPressMovie::default(),
            throwdown_results: throwdown_results_movie::ThrowdownResultsMovie::default(),
            end_challenge: end_challenge_movie::EndChallengeMovie::default(),
            end_small: end_challenge_small_movie::EndChallengeSmallMovie::default(),
            failed: challenge_failed_movie::ChallengeFailedMovie::default(),
            failed_small: challenge_failed_small_movie::ChallengeFailedSmallMovie::default(),
            random: apt_random::AptRandom::new(),
            change_game: party_play_change_movie::PartyPlayChangeMovie::default(),
            timer,
            big_black_timer,
            distance,
            throwdown_message,
            abd,
            leaderboard,
            dpo,
            countdown,
            voting,
            online_event_log,
            online_event_log_open: false,
            spectator,
            activity_overlay,
            activity_signup: activity_signup_movie::ActivitySignUpMovie::default(),
            online_event_log_hidden: false,
            online_custom,
            online_freeskate,
            lobby_browser,
            online_match_intro: false,
            online_freeskate_open: false,
            menu_dim: false,
            online_hub,
            online_hub_dim: false,
            online_results: online_results_movie::OnlineResultsMovie::default(),
            subtitle,
            movie_open: false,
            chyron,
            nis_screens: std::collections::BTreeMap::new(),
            minimap_direction: Default::default(),
            minimap_frame: Default::default(),
            minimap_icons: Vec::new(),
            minimap_heading: 0.,
            district_key: None,
            zone_key: DEFAULT_ZONE,
            minimap_enabled: true,
            free_skate_settings: None,
            minimap_before_bail: true,
            hud_binding: Default::default(),
            editor,
            editor_view: None,
            jacked_request:false,
            map_facts: MapRenderFacts { snapshot: Default::default(), locations: Vec::new(), player: None },
            pause_state: Default::default(),
            hom_hud: Default::default(),
            spin: Default::default(),
            jolt: Default::default(),
            textures,
            fps_font,
            fps_cached: None,
            career: None,
            hud: Default::default(),
            data: Default::default(),
            alert_facts: AlertFacts::default(),
            previous: Default::default(),
            pending_sounds: Vec::new(),
            clock: Default::default(),
            last: None,
            trace: std::env::var("SKATE2_TRACE_MARKER_UI").is_ok_and(|v| v == "1"),
        })
    }
    pub fn update(
        &mut self,
        renderer: &GpuRenderer,
        scene: &mut GpuScene,
        now: Instant,
        facts: MarkerHudSnapshot,
        scoring: &[skate2_core::ui::trick_backend::PhysicsFrame],
        scoring_tuning: skate2_core::ui::trick_display::DisplayTuning,
        pause: &skate2_core::ui::pause_menu::PauseMenuSnapshot,
        map: &skate2_core::ui::map_menu::MapMenuSnapshot,
        locations: &[skate2_core::ui::map_menu::MapLocation],
        player_pose: Option<([f32; 2], [f32; 2])>,
        minimap_frame: &skate2_core::ui::minimap::Frame,
        minimap_icons: &[skate2_core::ui::minimap::Icon],
        career: Option<&skate2_core::CareerHud>,
    ) -> RenderResult<()> {
        self.editor_view = None;
        self.advance_movies(now)?;
        self.objective.sounds.clear();
        scene.career_photo = career.and_then(|career| career.photo_save_slot);
        self.hud_binding = career.map(|career| career.binding).unwrap_or_default();
        if career.is_some() {
            self.free_skate_settings = None;
        }
        if let Some(career) = career {
            self.assets.profile_name.clone_from(&career.profile_name);
            self.assets.sponsor_labels.clone_from(&career.sponsor_labels);
            self.apply_career_hud_screens(career, !pause.visible)?;
            self.career = Some(career.clone());
        }
        if career.is_none() && (facts.prop_tick!=self.previous.prop_tick || facts.prop!=self.previous.prop) {
            let view=skate2_core::challenge::signup_hud::SignUpHudView {dmo:facts.prop.map(|p|p.params),..Default::default()};
            self.action_overlay.set_state(&self.assets,if facts.prop.is_some(){4}else{0},&view)?;
        }
        self.progression_popup.update(&self.assets, career.and_then(|c| c.dialog.as_ref()), career.and_then(|c| c.dialog_options.as_deref()))?;
        self.sign_up_choice.update_sign_up_choice(&self.assets, career.and_then(|c| c.sign_up_choice.as_ref()))?;
        self.signup_poster.update(&self.assets, career.and_then(|c| c.poster.as_ref()), renderer.dimensions())?;
        self.update_unlock_overlay(career.and_then(|c| c.unlock_overlay.as_ref()))?;
        self.sponsor_overlay.update(&self.assets, career.and_then(|c| c.sponsor_overlay.as_ref()), &mut self.spin)?;
        self.slappys.update(&self.assets, career.and_then(|c| c.slappys.as_ref()), career.map_or(&[][..], |c| c.slappys_screen.as_slice()))?;
        self.skate101.update(&self.assets, career.and_then(|c| c.skate101.as_ref()))?;
        self.hom_results.update(&self.assets,career.and_then(|c|c.hom_results.as_ref()))?;
        self.hom_annals.update(&self.assets,career.and_then(|c|c.hom_annals.as_ref()))?;
        self.photo_select.update(&self.assets, career.and_then(|c| c.photo_select.as_ref()), career.map_or(&[][..], |c| c.photo_select_screen.as_slice()))?;
        self.party_play_press.update(&self.assets, career.and_then(|c| c.press_a.as_ref()))?;
        self.update_end_options(career)?;
        self.update_change_game(renderer, career)?;
        if let Some(career) = career {
            if !self.movie_open {
                self.subtitle.update(&self.assets, career.nis.subtitle, career.subtitles)?;
            }
            for command in &career.nis_screens {
                self.apply_nis_screen(*command, &career.mission_title, &career.pro_info)?;
            }
        }
        self.apply_pause(pause, map.visible)?;
        self.apply_map(renderer, map, locations, player_pose)?;
        self.sync_dimmer()?;
        // Original FrontEndManager.Update82640130 advances these counters
        // once per game update. Uncapped presentation frames must not shorten
        // the 140-update stance delay or accelerate the shared landing jolt.
        // Preserve intermediate trick notifications when rendering catches up.
        for frame in scoring {
            // StartWipeout827c7858 hides the minimap even if HoM effects are
            // disabled. ReactivateScoring827c7230 restores its saved state.
            if frame.hom.just_wiped_out {
                self.minimap_before_bail=self.minimap_enabled;
                self.minimap_enabled=false;
            }
            if frame.hom.just_reactivated {self.minimap_enabled=self.minimap_before_bail;}
            for command in &frame.backend_commands{
                if let skate2_core::ui::trick_backend::BackendCommand::ShowCentralMessage(key)=command{
                    self.central.show(&self.assets,key,!self.pause_state.visible)?;
                }
            }
            self.central.update(&self.assets)?;
            self.jolt.update(scoring_tuning.jolt_speed, scoring_tuning.jolt_distance,
                scoring_tuning.jolt_recovery);
            self.score.update(&self.assets, std::slice::from_ref(frame), scoring_tuning, self.hud_binding.trick_display)?;
            self.jacked_request|=self.score.take_jacked_request();
            if let Some(distance) = self.score.jolt.take() { self.jolt.trigger(distance); }
            // Free skate has no completed career HoM challenge records.
            let hom = self.hom_hud.advance(&frame.hom, career.map_or(0,|c|c.hom_completed as u32));
            self.hom.update(&self.assets, std::slice::from_ref(&hom), &mut self.random)?;
        }
        // Shown by the instance FixupActiveComponents bound: "main"'s or a challenge group's.
        let minimap_shown = match self.hud_binding.minimap {
            skate2_core::challenge::hud::Binding::Main => {
                self.minimap_enabled && self.free_skate_settings.is_none_or(|(_, mini_map)| mini_map)
            }
            skate2_core::challenge::hud::Binding::Group(shown) => shown,
            skate2_core::challenge::hud::Binding::Unbound => false,
        };
        self.minimap.show(&self.assets,facts.visible && !pause.visible && minimap_shown)?;
        self.minimap.no_skate_zone(&self.assets,minimap_frame.no_skate_zone)?;
        self.minimap_heading=self.minimap_direction.update(minimap_frame);
        self.minimap_frame=minimap_frame.clone();
        if self.minimap_icons.is_empty() {self.minimap_icons=minimap_icons.to_vec();}
        if self.district_key != Some(minimap_frame.district_key) {
            if self.trace {
                std::eprintln!("minimap district={:016x} name={:?} main_challenge={} position={:?} hud={} icons={} enabled_icons={}",
                    minimap_frame.district_key,minimap_frame.district_name,minimap_frame.main_challenge,minimap_frame.position,
                    facts.visible,minimap_icons.len(),minimap_icons.iter().filter(|icon|icon.visible).count());
            }
            self.district_key=Some(minimap_frame.district_key);
            // ReceiveMsg(cMsgWorldPainter_KeyChange)8263de80 keeps mZoneKey for keys 0 and -1;
            // UpdateDistrictZoneName82638820 shows nothing for the default key and, while a main
            // challenge runs (all of Party Play), sets mZoneKey to -1 instead of naming the area.
            let key=minimap_frame.district_key;
            if !pause.visible && key!=0 && key!=u64::MAX {
                self.zone_key=key;
                if key!=DEFAULT_ZONE {
                    if minimap_frame.main_challenge {self.zone_key=u64::MAX;}
                    else if let Some(name)=&minimap_frame.district_name {self.central.district(&self.assets,name)?;}
                }
            }
        }
        // An online game's B Spectate takes the options row the career's Contacts has (PC port
        // addition).
        let context = HudContext {
            can_show_marker: facts.permissions.is_some(),
            can_show_options: self.can_show_phone_options || facts.spectate,
            marker: facts.permissions,
        };
        let mut commands = Vec::new();
        let phone_visible=facts.visible && !facts.phone_hidden;
        if self.hud.state() == PhoneListState::Uninitialized || self.hud.visible() != phone_visible
        {
            self.hud.show(
                phone_visible,
                facts.bumper_held,
                true,
                context,
                &mut commands,
            );
        } else if facts.bumper_held != self.previous.bumper_held {
            self.hud.input(
                if facts.bumper_held {
                    INPUT_LEFT_BUMPER
                } else {
                    INPUT_LEFT_BUMPER_RELEASED
                },
                context,
                &mut commands,
            );
        }
        // The Spectate row comes or goes while the menu is up: the menu takes its layout anew.
        if self.hud.active() && facts.spectate != self.previous.spectate {
            commands.push(HudCommand::SetScreenState(if context.can_show_options {
                ScreenState::MaximizedWithOptions
            } else {
                ScreenState::MaximizedMarkerOnly
            }));
        }
        if facts.successful_sets != self.previous.successful_sets {
            self.data.marker_set_succeeded();
        }
        self.hud
            .update(Some(&mut self.data), context, &mut commands);
        if self.trace && !commands.is_empty() {
            std::eprintln!(
                "marker-ui held={} permissions={:?} successful_sets={} commands={:?}",
                facts.bumper_held,
                facts.permissions,
                facts.successful_sets,
                commands
            );
        }
        for command in commands {
            match command {
                HudCommand::SetScreenState(state) => {
                    self.set_state(state, facts.permissions.unwrap_or_default(), facts.spectate && !self.can_show_phone_options)?
                }
                HudCommand::CentralHudForceOutro=>self.central.force_outro(&self.assets)?,
                // Frontend audio is retained by its respective movie owner.
                // Retain native controller commands without inventing sounds.
                HudCommand::SetPhoneListGroupFilter
                | HudCommand::ClearGroupFilter
                | HudCommand::PlayOpenSound => {}
                // Y-release is consumed by the session's frontend input owner,
                // which opens Contacts and queues its sound before SIM pauses.
                HudCommand::PlayPhoneOptionsSound | HudCommand::PushPhoneOptionsState => {}
            }
        }
        self.previous = facts;
        self.draw(renderer, scene)
    }
    /// Frontend continues on wall time while the last completed world stays frozen.
    /// `FrontEndState_Unlock::OnEnter` (`0x82659370`) forces the central
    /// HUD's outro (`CentralHUD::ForceCentralHUDOutro` `0x826bc348`) before it
    /// opens the screen. Its `cMsgResultScreenEffect` to the presentation is
    /// not ported.
    fn update_unlock_overlay(&mut self, view: Option<&skate2_core::challenge::unlock_overlay::UnlockOverlayView>) -> RenderResult<()> {
        if view.is_some() && !self.unlock_overlay.open {
            self.central.force_outro(&self.assets)?;
        }
        self.unlock_overlay.update(&self.assets, view, &mut self.spin)
    }
    /// `FrontEndManager::Update` (`0x82640220..40`) runs the HUD while in
    /// game, including paused front-end screens. `take_career_hud` drains
    /// these calls, so every consumer must apply them even when drawing the
    /// HUD is suppressed; otherwise a hide/show transition is lost on resume.
    fn apply_career_hud_screens(&mut self, career: &skate2_core::CareerHud, central_visible: bool) -> RenderResult<()> {
        self.hud_binding = career.binding;
        self.assets.pro_film_pro_name.clone_from(&career.pro_film_pro_name);
        self.assets.frontend_arguments.clone_from(&career.frontend_arguments);
        self.can_show_phone_options=career.can_show_phone_options;
        self.alert_facts=career.phone_alert_facts;
        if career.phone_data_dirty || self.data.service_icons!=career.phone_service_icons {
            self.data.service_icons=career.phone_service_icons;
            self.data.dirty=true;
        }
        let pending=self.contacts.pending();self.contacts.update(&self.assets,career.contacts.as_ref())?;
        self.screens.track(Screen::Contacts,pending,self.contacts.pending());
        for message in &career.central_messages {
            self.central.show_formatted(&self.assets, &message.hal_id, &message.args, central_visible)?;
        }
        for (state, view) in &career.objective_screen {
            self.objective.set_state(&self.assets, *state, view)?;
        }
        for (state, view) in &career.tutorial_screen {
            self.tutorial.set_state(&self.assets, *state, view)?;
        }
        for (state, view) in &career.signup_screen {
            self.action_overlay.set_state(&self.assets, *state, view)?;
        }
        self.apply_turn_huds(career)
    }

    /// The challenge system went outside a game unload: its HUD's last screen states, and
    /// nothing of the challenge stays. `FrontEndState_NIS`'s screens and subtitle go with its NIS,
    /// and the HUD no longer reads the challenge's last update.
    pub(crate) fn end_career(&mut self, last: &skate2_core::CareerHud) -> RenderResult<()> {
        self.apply_career_hud_screens(last, !self.pause_state.visible)?;
        let screens: Vec<i32> = self.nis_screens.keys().copied().collect();
        for id in screens {
            self.apply_nis_screen(ScreenCommand::Destroy { id }, &last.mission_title, &last.pro_info)?;
        }
        self.subtitle.update(&self.assets, None, last.subtitles)?;
        self.career = None;
        Ok(())
    }

    /// The turn HUD components' screen states, each with the data its movie reads then.
    fn apply_turn_huds(&mut self, career: &skate2_core::CareerHud) -> RenderResult<()> {
        for (state,view) in &career.big_black_timer_screen{self.big_black_timer.set_state(&self.assets,*state,view)?;}
        for (state, view) in &career.timer_screen {
            self.timer.set_state(&self.assets, *state, view)?;
        }
        for (state, view) in &career.distance_screen {
            self.distance.set_state(&self.assets, *state, view)?;
        }
        for (state, _) in &career.throwdown_screen {
            self.throwdown_message.set_state(&self.assets, *state)?;
        }
        for (state, view) in &career.abd_screen {
            self.abd.set_state(&self.assets, *state, view)?;
        }
        for (state, view) in &career.leaderboard_screen {
            self.leaderboard.set_state(&self.assets, *state, view, &career.mission_title)?;
        }
        for (state, view) in &career.countdown_screen {
            self.countdown.set_state(&self.assets, *state, view, self.zone_key)?;
        }
        for (state, _) in &career.voting_screen {
            self.voting.set_state(&self.assets, *state)?;
        }
        for (state, view) in &career.dpo_screen {
            self.dpo.set_state(&self.assets, *state, view)?;
        }
        Ok(())
    }
    /// `FrontEndState_EndChallengeOptions::OnEnter` (`0x82658880`) forces the central HUD's
    /// outro before it opens its screen: 47 for a throwdown, 41 and 42 for a win with and
    /// without several players, 44 and 45 for a loss with and without.
    fn update_end_options(&mut self, career: Option<&skate2_core::CareerHud>) -> RenderResult<()> {
        use skate2_core::challenge::results::screen;
        let view = career.and_then(|c| c.end_options.as_ref());
        if view.is_some()
            && !self.throwdown_results.open
            && !self.end_challenge.open
            && !self.end_small.open
            && !self.failed.open
            && !self.failed_small.open
        {
            self.central.force_outro(&self.assets)?;
        }
        let online = view.filter(|v| v.options.screen == screen::ONLINE);
        self.online_results.update(&self.assets, online)?;
        self.pending_sounds.append(&mut self.online_results.sounds);
        if let Some(view) = view
            && !matches!(
                view.options.screen,
                screen::THROWDOWN | screen::RESULTS | screen::RESULTS_GVR | screen::RESULTS_SMALL | screen::FAILED | screen::FAILED_SMALL | screen::ONLINE
            )
        {
            return Err(format!("end challenge screen {} is not translated", view.options.screen).into());
        }
        let failed = view.filter(|v| v.options.screen == screen::FAILED);
        self.failed.update(&self.assets, failed, &mut self.random)?;
        let failed_small = view.filter(|v| v.options.screen == screen::FAILED_SMALL);
        self.failed_small.update(&self.assets, failed_small, &mut self.random)?;
        let lobby = skate2_core::challenge::system::MissionTitle::Lobby;
        let title = career.map_or(&lobby, |c| &c.mission_title);
        let throwdown = view.filter(|v| v.options.screen == screen::THROWDOWN);
        self.throwdown_results.update(&self.assets, throwdown, title, &mut self.spin)?;
        let small = view.filter(|v| v.options.screen == screen::RESULTS_SMALL);
        self.end_small.update(&self.assets, small, title, &mut self.spin, &mut self.random)?;
        let results = view.filter(|v| matches!(v.options.screen, screen::RESULTS | screen::RESULTS_GVR));
        self.end_challenge.update(&self.assets, results, title, &mut self.spin)
    }
    /// FE state 39's screen 65: opened and closed with the state, given its `SetState`s and the
    /// location image `LoadRX2Texture` names. Entering the state turns the dimmer on and leaving it
    /// for FE state 8 turns it off (`sFEStateInfo::mbDimmer`); the screen's own calls do nothing.
    fn update_change_game(&mut self, renderer: &GpuRenderer, career: Option<&skate2_core::CareerHud>) -> RenderResult<()> {
        let view = career.and_then(|c| c.change_game.as_ref());
        let states = career.map_or(&[][..], |c| c.change_game_screen.as_slice());
        let image = match view.and_then(|view| view.image.as_deref()) {
            Some(image) => match self.assets.ensure_location_texture(image) {
                Ok((key, loaded)) => {
                    if loaded {
                        let uploaded = renderer.upload_ui_textures(self.assets.textures.get(&key).into_iter())?;
                        self.textures.extend(uploaded);
                    }
                    Some(key)
                }
                Err(_) => None,
            },
            None => None,
        };
        self.change_game.update(&self.assets, view, states, image.as_deref())?;
        self.sync_dimmer()
    }
    /// `StateManagerFrontEnd::EnableDimmer` (0x82656a30) with the `gFEStateInfo` (0x82fded80)
    /// `mbDimmer` of the state on top, which `ExecutePushState` (0x82d694e8) and `ExecutePopState`
    /// (0x82d672f8) send: Options (73), the music player (30), the challenge map (21), Party Play's
    /// change game (39) and the jacked overlay (42) dim; the StartHub (7), the main menu (22), the
    /// end-of-challenge options (8) and the game do not. The screens' own
    /// `ScreenManager.EnableDimmer` calls reach a method screenmanager.apt never defines.
    /// A popup over the map is FE state 34, whose `mbDimmer` is off. Every caller asks this one
    /// condition: two that disagreed would restart the dimmer's intro and outro each frame.
    fn sync_dimmer(&mut self) -> RenderResult<()> {
        let map_popup = self.map_facts.snapshot.career.as_ref().is_some_and(|career| career.popup.is_some());
        // Slappy's blog, its articles and covers (61 to 63) and skate.Profile (45) have mbDimmer.
        let on = self.settings.on_stack() || (self.map_facts.snapshot.visible && !map_popup) || self.change_game.open
            || self.extras.text_entry() || (self.pause_state.visible && self.pause_state.jacked) || self.slappys.open() || self.hom_annals.open
            || self.skate_profile.open()
            || self.replay_footage.dimming() || self.skate_reel[1].dimming()
            || if self.main.is_some() { self.menu_dim } else { self.online_hub_dim };
        self.dimmer.enable(&self.assets, on)
    }
    /// A frame while the unlock overlay or Skate 101 pauses the game: the
    /// front end keeps updating (`FrontEndManager::Update`) over the retained
    /// world.
    pub fn update_career_frontend(&mut self, renderer: &GpuRenderer, scene: &mut GpuScene,
        now: Instant, career: &skate2_core::CareerHud) -> RenderResult<()> {
        scene.career_photo = career.photo_save_slot;
        self.assets.profile_name.clone_from(&career.profile_name);
        self.assets.sponsor_labels.clone_from(&career.sponsor_labels);
        self.advance_movies(now)?;
        self.apply_career_hud_screens(career, !self.pause_state.visible)?;
        self.update_unlock_overlay(career.unlock_overlay.as_ref())?;
        self.sponsor_overlay.update(&self.assets, career.sponsor_overlay.as_ref(), &mut self.spin)?;
        self.slappys.update(&self.assets, career.slappys.as_ref(), &career.slappys_screen)?;
        self.sync_dimmer()?;
        self.skate101.update(&self.assets, career.skate101.as_ref())?;
        self.hom_results.update(&self.assets,career.hom_results.as_ref())?;
        self.hom_annals.update(&self.assets,career.hom_annals.as_ref())?;
        // ExecutePushState82d69604 -> StartHub::OnLeave8264c4d8 closes the
        // parent screen while Slappy's state owns the paused front end.
        let pause = self.pause_state;
        self.apply_pause(&pause, self.map_facts.snapshot.visible)?;
        self.photo_select.update(&self.assets, career.photo_select.as_ref(), &career.photo_select_screen)?;
        self.update_end_options(Some(career))?;
        self.update_change_game(renderer, Some(career))?;
        self.progression_popup.update(&self.assets, career.dialog.as_ref(), career.dialog_options.as_deref())?;
        self.sign_up_choice.update_sign_up_choice(&self.assets, career.sign_up_choice.as_ref())?;
        if !self.movie_open {
            self.subtitle.update(&self.assets, career.nis.subtitle, career.subtitles)?;
        }
        self.career = Some(career.clone());
        self.draw(renderer, scene)
    }
    /// `FrontEndState_MoviePlayer` on the stack or not, with `FEMoviePlayer`'s subtitle line
    /// and the profile's subtitles option. `RefreshSubtitle` (0x82614c80) runs the screen's
    /// refresh with the option or `mbOverrideProfileSetting`; `Stop` (0x82639380) clears and
    /// refreshes the line as the state leaves.
    pub(crate) fn set_movie_state(&mut self, open: bool, subtitle: skate2_core::ui::movie_player::MovieSubtitle,
        subtitles: bool, screen: Option<skate2_core::ui::movie_player::MovieScreen>) -> RenderResult<Option<u64>> {
        let was_open = std::mem::replace(&mut self.movie_open, open);
        if open || was_open {
            self.subtitle.update(&self.assets, subtitle.text, subtitles || subtitle.override_profile)?;
        }
        self.fmv.update(&self.assets, screen)
    }
    /// A frame while `FrontEndState_MoviePlayer` pauses the game: the front end keeps updating
    /// (`FrontEndManager::Update`) with the HUD hidden. A subtitled movie with the subtitles
    /// option on plays under the front end (`FEMoviePlayer::Play` 0x82638fb8 clears
    /// `mbForeground`), so the subtitle layer shows over it.
    pub fn update_movie_frontend(&mut self, renderer: &GpuRenderer, scene: &mut GpuScene, now: Instant) -> RenderResult<()> {
        self.advance_movies(now)?;
        self.draw(renderer, scene)
    }
    /// The NIS's line in the last career update (`FESubtitle::mSubtitleText`), shown or not.
    pub(crate) fn nis_subtitle_line(&self) -> Option<u32> {
        self.career.as_ref().and_then(|career| career.nis.subtitle)
    }
    /// The line the subtitle layer shows.
    pub(crate) fn subtitle_shown(&self) -> Option<u32> {
        self.subtitle.shown()
    }
    pub fn update_pause(&mut self, renderer: &GpuRenderer, scene: &mut GpuScene,
        now: Instant, pause: &skate2_core::ui::pause_menu::PauseMenuSnapshot,
        map: &skate2_core::ui::map_menu::MapMenuSnapshot,
        locations: &[skate2_core::ui::map_menu::MapLocation],
        player_pose: Option<([f32; 2], [f32; 2])>,
        career: Option<&skate2_core::CareerHud>,
    ) -> RenderResult<()> {
        // ExecutePopState82d672f8 closes the child screen and re-enters the
        // hub even though the simulation remains paused. Consume the fresh
        // career view here so Slappy's outro also runs on this return path.
        self.slappys.update(&self.assets, career.and_then(|c| c.slappys.as_ref()),
            career.map_or(&[][..], |c| c.slappys_screen.as_slice()))?;
        self.hom_annals.update(&self.assets, career.and_then(|c| c.hom_annals.as_ref()))?;
        self.career = career.cloned();
        // ExecuteActions' pushes and pops come before the next APT tick.
        self.apply_pause(pause, map.visible)?;
        self.apply_map(renderer, map, locations, player_pose)?;
        self.sync_dimmer()?;
        self.advance_movies(now)?;
        if let Some(career) = career {
            self.apply_career_hud_screens(career, !pause.visible)?;
        }
        self.draw(renderer, scene)
    }
    pub fn update_editor(&mut self,renderer:&GpuRenderer,scene:&mut GpuScene,
        now:Instant,view:&skate2_core::ui::edit_skater::EditSkaterView,mounts:&MountSet)
        ->RenderResult<(bool,bool)> {
        self.geometry_pending = false;
        self.advance_movies(now)?;
        if self.editor_view.as_ref() != Some(view) {
        let names:Vec<_>=view.rows.iter().skip(view.window_start).take(9).filter_map(|r|r.texture.clone()).collect();
        if self.assets.ensure_cac_textures(mounts,&names)? {
        let uploaded = renderer.upload_ui_textures(self.assets.textures.iter()
            .filter(|(key,_)|!self.textures.contains_key(*key)).map(|(_,texture)|texture))?;
        self.textures.extend(uploaded);
        }
        let mut localized=view.clone();
        let localize=|value:&str|->RenderResult<String>{
            if value=="0" || value=="#" {Ok(String::new())}
            else if value.starts_with("ID_") || value.starts_with('#') {self.assets.localize(value)}
            else {Ok(value.into())}
        };
        localized.title=localize(&view.title)?;
        localized.help_a=localize(&view.help_a)?;
        localized.help_b=localize(&view.help_b)?;
        localized.help_start=localize(&view.help_start)?;
        for row in &mut localized.rows {
            let selected=row.description_candidates.iter().find_map(|key|self.assets.localize(key).ok());
            row.title=if let Some(title)=selected.as_ref().filter(|_|row.description_candidates.contains(&row.title)){title.clone()}else{localize(&row.title)?};
            row.subtitle=localize(&row.subtitle)?;
            row.description=if let Some(description)=selected {description}else{localize(&row.description)?};
        }
        // UpdateSponsorLogo's LoadRX2Texture of the selected part's brand logo (screens.big);
        // FETextureData's load of a logo the archive lacks shows nothing.
        if let Some(row)=localized.rows.get_mut(view.selected) {
            row.sponsor_logo=match row.sponsor_logo.take() {
                Some(path)=>match self.assets.ensure_rx2_image(&path) {
                    Ok((key,loaded))=>{
                        if loaded {self.textures.extend(renderer.upload_ui_textures(self.assets.textures.get(&key).into_iter())?);}
                        Some(key)
                    }
                    Err(_)=>None,
                },
                None=>None,
            };
        }
        self.editor.update(&self.assets,&localized)?;
        self.receipt.update(&self.assets,view.receipt.as_ref())?;
        self.editor_view = Some(view.clone());
        }
        let mut batches=Vec::new();
        // CaC::Shutdown82659dfc uses ShowScreen(false) before the room fades.
        // Keep ticking the retained outro for cleanup, but hide its drawing.
        if view.visible {
        flatten_projected(&self.assets,&self.editor.root,self.editor.screen_transform(),Colour::default(),
            renderer.dimensions(),&mut batches,true,None)?;
        flatten_projected(&self.assets,&self.editor.letterbox,IDENTITY,Colour::default(),
            renderer.dimensions(),&mut batches,true,None)?;
        flatten_projected(&self.assets,&self.chyron.root,IDENTITY,Colour::default(),
            renderer.dimensions(),&mut batches,true,None)?;
        if let Some(receipt) = &self.receipt.root {
            flatten_projected(&self.assets,receipt,IDENTITY,Colour::default(),
                renderer.dimensions(),&mut batches,true,None)?;
        }
        if let Some(popup) = &self.editor.popup {
            flatten_projected(&self.assets,popup,IDENTITY,Colour::default(),
                renderer.dimensions(),&mut batches,true,None)?;
        }
        }
        renderer.update_marker_ui(scene,&self.textures,batches)?;
        self.finish_frame(renderer, scene)?;
        Ok((self.editor.intro_complete,self.editor.outro_complete))
    }
    /// FE state 7 is on top while the pause is up as the StartHub (not the jacked overlay, 42) with
    /// neither Options (73, or the music player 30 over it), the challenge map (21), nor
    /// Slappy's blog/coverage/Annals (61-63/77) pushed over it: each push's `UpdateOnLeave` closes the hub's screen, each pop's `UpdateOnEnter` opens a
    /// new one.
    fn apply_pause(&mut self, pause: &skate2_core::ui::pause_menu::PauseMenuSnapshot, map: bool) -> RenderResult<()> {
        if pause.visible&&!self.pause_state.visible{self.central.clear(&self.assets)?;}
        let view = self.career.as_ref().map(|career| &career.start_hub);
        let (new_challenges, new_blogs) = view.map_or((0, 0), |view| (view.new_challenges, view.new_blogs));
        let rows = pause.options().iter()
            .map(|&option| {
                let (label, count) = pause.option_label(option, new_challenges, new_blogs);
                pause_movie::HubRow { label, count, enabled: pause.item_enabled(option) }
            })
            .collect::<Vec<_>>();
        let hub = pause.visible && !pause.jacked && !self.settings.on_stack() && !map
            && !self.slappys.open() && !self.hom_annals.open && !self.replay_editor.open() && !self.replay_footage.open()
            && !self.trick_guide.open() && !self.skate_profile.open();
        let pending = self.pause.pending();
        self.pause.update(&self.assets, hub, pause.selected, &rows)?;
        if self.pause.pending() {
            let layout = pause_movie::HubLayout::new(&self.assets, pause, view)?;
            self.pause.set_layout(layout);
        }
        self.screens.track(Screen::StartHub, pending, self.pause.pending());
        self.jacked.update(&self.assets,pause.visible&&pause.jacked)?;
        self.pause.update_quit(&self.assets,pause.popup)?;
        self.pause_state = *pause;
        Ok(())
    }
    fn apply_map(&mut self, renderer: &GpuRenderer, snapshot: &skate2_core::ui::map_menu::MapMenuSnapshot,
        locations: &[skate2_core::ui::map_menu::MapLocation],
        player_pose: Option<([f32; 2], [f32; 2])>,
    ) -> RenderResult<()> {
        // GetCategoryHAL8261cb48 formats the authored category count after
        // localization; destination names are direct original HAL strings.
        if !snapshot.visible && !self.map_facts.snapshot.visible { return Ok(()); }
        if !snapshot.visible {
            self.map.update(&self.assets, false, &[], 0, "", None)?;
            self.map.update_popup(&self.assets, None)?;
            self.map_facts.snapshot.visible = false;
            self.map_facts.snapshot.career = None;
            return Ok(());
        }
        if let Some(career) = snapshot.career.as_deref() {
            self.apply_challenge_map(renderer, career)?;
            self.map_facts.snapshot = snapshot.clone();
            self.map_facts.player =
                player_pose.map(|(position, forward)| (position, forward[0].atan2(forward[1])));
            return Ok(());
        }
        let rows = snapshot.rows(locations);
        let labels = rows.iter().map(|row| {
            self.assets.localize(row.localization_key).map(|label| {
                if row.location_key.is_none() { label.replace("%d", &row.count.to_string()) }
                else { label }
            })
        }).collect::<RenderResult<Vec<_>>>()?;
        let count = locations.iter().filter(|location| {
            matches!(snapshot.category, skate2_core::ui::map_menu::MapCategory::Root |
                skate2_core::ui::map_menu::MapCategory::Locations) || location.category == snapshot.category
        }).count();
        let title = self.assets.localize(snapshot.category.localization_key())?
            .replace("%d", &count.to_string());
        let info_title = snapshot.info_location.map(|key| {
            let location = locations.iter().find(|location| location.key == key)
                .ok_or("Map selected location is absent")?;
            self.assets.localize(&location.localization_key)
        }).transpose()?;
        let rows: Vec<_> = labels.iter().map(|label| (label.as_str(), true)).collect();
        let info = info_title.map(|title| map_movie::InfoPanel { title, option: "ID_COMMON_TELEPORT", property: None });
        self.map.update(&self.assets, snapshot.visible, &rows, snapshot.selected, &title, info.as_ref())?;
        self.map_facts.snapshot = snapshot.clone();
        if self.map_facts.locations != locations { self.map_facts.locations = locations.to_vec(); }
        self.map_facts.player = player_pose.map(|(position, forward)|
            (position, forward[0].atan2(forward[1])));
        Ok(())
    }
    /// The career's FrontEndState_ChallengeMap on screen 14: the Stuff panel's title and rows,
    /// the info component and the letterbox's buttons (UpdateLetterBox 5e64).
    fn apply_challenge_map(
        &mut self,
        renderer: &GpuRenderer,
        career: &skate2_core::challenge::stuff_menu::ChallengeMapView,
    ) -> RenderResult<()> {
        use skate2_core::challenge::stuff_menu::Letterbox;
        let rows = career
            .rows
            .iter()
            .map(|text| self.menu_text(text))
            .collect::<RenderResult<Vec<_>>>()?;
        let title = self.menu_text(&career.title)?;
        let info = career
            .info
            .as_ref()
            .map(|info| -> RenderResult<map_movie::InfoPanel> {
                let property = info
                    .property
                    .as_ref()
                    .map(|property| self.property_panel(renderer, property))
                    .transpose()?;
                Ok(map_movie::InfoPanel { title: self.menu_text(&info.title)?, option: info.option, property })
            })
            .transpose()?;
        let rows: Vec<_> = rows.iter().map(|label| (label.as_str(), true)).collect();
        self.map.update(&self.assets, true, &rows, career.selected, &title, info.as_ref())?;
        let hom = career.hom.as_ref().map(|paths| -> RenderResult<[map_movie::HomPathPanel; 4]> {
            let mut localized = Vec::with_capacity(4);
            for path in paths {
                let objective = match &path.objective {
                    Some((hal, args)) => String::from_utf16(&localizer::format_custom(&self.assets, hal, args)?)?,
                    None => String::new(),
                };
                localized.push(map_movie::HomPathPanel { reached: path.reached, total: path.total, objective });
            }
            localized.try_into().map_err(|_| "Hall of Meat requires four authored paths".into())
        }).transpose()?;
        self.map.update_hom(hom.as_ref())?;
        self.map.update_popup(&self.assets, career.popup)?;
        // kLBConfig_Stuff's X names the other view: mSTUFF_COMPLETED_CHALLENGE_TEXT among the
        // available challenges, mSTUFF_ACTIVE_CHALLENGE_TEXT among the completed ones.
        let buttons: &[(&'static str, &'static str)] = match career.letterbox {
            Letterbox::Basic => &[("B", "ID_COMMON_BACK"), ("A", "ID_COMMON_SELECT")],
            Letterbox::Stuff {
                populate_active: true,
            } => &[
                ("B", "ID_COMMON_BACK"),
                ("A", "ID_COMMON_SELECT"),
                ("X", "ID_CHALLENGE_MAP_COMPLETED_CHALLENGES"),
            ],
            Letterbox::Stuff {
                populate_active: false,
            } => &[
                ("B", "ID_COMMON_BACK"),
                ("A", "ID_COMMON_SELECT"),
                ("X", "ID_CHALLENGE_MAP_ACTIVE_CHALLENGES"),
            ],
            Letterbox::Back => &[("B", "ID_COMMON_BACK")],
        };
        self.map.set_buttons(&self.assets, buttons)
    }
    /// `challenge_info`'s purchase texts: the description through `FormatStringCustom`, the
    /// location image `LoadRX2Texture` loads (none when the archive lacks it), the price as
    /// `GetPropertyPriceOrPaid` formats it (`FormatCurrency` with the literal prefix) or
    /// `ID_CIC_PURCHASED`, and `Cash_GetFormattedCashString` of the wallet.
    fn property_panel(
        &mut self,
        renderer: &GpuRenderer,
        property: &skate2_core::challenge::stuff_menu::PropertyView,
    ) -> RenderResult<map_movie::PropertyPanel> {
        let image = match self.assets.ensure_location_texture(&property.image) {
            Ok((key, loaded)) => {
                if loaded {
                    let uploaded = renderer.upload_ui_textures(self.assets.textures.get(&key).into_iter())?;
                    self.textures.extend(uploaded);
                }
                Some(key)
            }
            Err(_) => None,
        };
        let price = match property.price {
            Some(cost) => localizer::format_currency(&self.assets, cost as f32),
            None => self.assets.localize("ID_CIC_PURCHASED")?,
        };
        Ok(map_movie::PropertyPanel {
            description: self.menu_text(&property.description)?,
            image,
            price,
            cash: property.cash.map(|cash| localizer::format_currency(&self.assets, cash as f32)),
        })
    }
    /// A Stuff panel text as the screen shows it: a copied HAL id as a text field shows one
    /// (the constructor sets ID_CHALLENGE_MAP_STUFF so), a category's label through FormatString
    /// with its count printed in (GetCategoryHAL 8261cb48), an item's HAL id through
    /// FormatStringCustom (GetChallengeHAL 82639770).
    fn menu_text(
        &self,
        text: &skate2_core::challenge::stuff_menu::MenuText,
    ) -> RenderResult<String> {
        use skate2_core::challenge::stuff_menu::MenuText;
        Ok(match text {
            MenuText::Empty => String::new(),
            MenuText::Copied(label) if label.starts_with("ID_") || label.starts_with('#') => {
                String::from_utf16(&self.assets.initial_text(label))?
            }
            MenuText::Copied(label) => label.clone(),
            MenuText::Category { hal, count } => {
                let label = skate2_formats::localization::string_hash(hal.as_bytes());
                String::from_utf16(&localizer::format_hal(&self.assets, label)?)?
                    .replace("%d", &count.to_string())
            }
            MenuText::Formatted(hal) => {
                String::from_utf16(&localizer::format_custom(&self.assets, hal, "")?)?
            }
        })
    }
    fn advance_movies(&mut self, now: Instant) -> RenderResult<()> {
        let elapsed = self
            .last
            .map_or(0, |last| now.duration_since(last).as_millis() as u32);
        self.last = Some(
            self.last
                .map_or(now, |last| last + Duration::from_millis(u64::from(elapsed))),
        );
        let steps = self.clock.advance(
            elapsed,
            NonZeroU32::new(self.assets.movies[0].movie.milliseconds_per_frame)
                .ok_or("APT zero frame duration")?,
            false,
        );
        for _ in 0..steps {
            // The screen manager's onEnterFrame runs before its screens' frame actions.
            let linked = self.screens.tick();
            // AptIntegration::Update (0x826014c0) advances the spin before the movies.
            self.spin.update();
            self.fmv.tick(&self.assets)?;
            self.replay_editor.tick(&self.assets)?;
            self.trick_guide.tick(&self.assets,&mut self.trick_guide_letterbox)?;
            self.contacts.tick(&self.assets)?;
            if std::mem::take(&mut self.contacts.destroyed){self.screens.destroy();}
            self.trick_guide_letterbox.tick(&self.assets)?;
            if std::mem::take(&mut self.trick_guide.destroyed){self.screens.destroy();}
            self.skate_profile.tick(&self.assets)?;
            if std::mem::take(&mut self.skate_profile.destroyed) {self.screens.destroy();}
            if self.replay_editor.take_outro_complete() {self.screens.destroy();}
            self.replay_footage.tick(&self.assets)?;
            if self.replay_footage.take_outro() {self.screens.destroy();}
            if let Some(main)=self.main.as_mut(){
                main.tick(&self.assets)?;
                if main.take_outro_complete() {
                    self.screens.destroy();
                }
                self.extras.tick(&self.assets, &mut main.letterbox)?;
                self.dlc.tick(&self.assets, &mut main.letterbox)?;
                if self.dlc.take_outro_complete() {
                    self.screens.destroy();
                }
                for reel in &mut self.skate_reel {
                    reel.tick(&self.assets,&mut main.letterbox)?;
                    if reel.take_outro() {self.screens.destroy();}
                }
                if self.extras.take_outro_complete() {
                    self.screens.destroy();
                }
                self.freeskate.tick(&self.assets, &mut main.letterbox)?;
                self.party_play.tick(&self.assets, &mut main.letterbox)?;
            }
            self.online_custom.tick(&self.assets)?;
            self.online_freeskate.tick(&self.assets)?;
            self.lobby_browser.tick(&self.assets)?;
            self.online_hub.tick(&self.assets)?;
            self.root.tick(&self.assets)?;
            self.score.tick(&self.assets)?;
            self.hom.tick(&self.assets)?;
            self.central.tick(&self.assets)?;
            self.minimap.tick(&self.assets)?;
            self.objective.tick(&self.assets)?;
            self.tutorial.tick(&self.assets)?;
            self.action_overlay.tick(&self.assets)?;
            self.progression_popup.tick(&self.assets)?;
            self.name_popup.tick(&self.assets)?;
            self.dlc_popup.tick(&self.assets)?;
            self.update_popup.tick(&self.assets)?;
            self.sign_up_choice.tick(&self.assets)?;
            self.chat_popup.tick(&self.assets)?;
            self.signup_poster.tick(&self.assets)?;
            self.unlock_overlay.tick(&self.assets)?;
            self.sponsor_overlay.tick(&self.assets)?;
            self.slappys.tick(&self.assets)?;
            self.skate101.tick(&self.assets)?;
            self.hom_results.tick(&self.assets)?;
            self.hom_annals.tick(&self.assets)?;
            self.photo_select.tick(&self.assets)?;
            self.party_play_press.tick(&self.assets)?;
            self.throwdown_results.tick(&self.assets)?;
            self.end_challenge.tick(&self.assets)?;
            self.online_results.tick(&self.assets)?;
            self.end_small.tick(&self.assets)?;
            self.failed.tick(&self.assets)?;
            self.failed_small.tick(&self.assets)?;
            self.change_game.tick(&self.assets)?;
            self.timer.tick(&self.assets)?;
            self.big_black_timer.tick(&self.assets)?;
            self.distance.tick(&self.assets)?;
            self.throwdown_message.tick(&self.assets)?;
            self.abd.tick(&self.assets)?;
            self.leaderboard.tick(&self.assets)?;
            self.dpo.tick(&self.assets)?;
            self.countdown.tick(&self.assets)?;
            self.voting.tick(&self.assets)?;
            if self.online_event_log_open {
                self.online_event_log.tick(&self.assets)?;
            }
            self.spectator.tick(&self.assets)?;
            self.activity_overlay.tick(&self.assets)?;
            self.activity_signup.tick(&self.assets)?;
            self.subtitle.root.tick(&self.assets)?;
            self.chyron.tick(&self.assets)?;
            for screen in self.nis_screens.values_mut() {
                screen.tick(&self.assets)?;
            }
            self.pause.tick(&self.assets)?;
            if self.pause.take_outro_complete() {
                self.screens.destroy();
            }
            self.settings.tick(&self.assets, self.main.as_mut().map(|m| &mut m.letterbox))?;
            if self.settings.take_outro_complete() {
                self.screens.destroy();
            }
            self.music_player.tick(&self.assets)?;
            if self.music_player.take_outro_complete() {
                self.screens.destroy();
            }
            self.map.tick(&self.assets)?;
            self.editor.tick(&self.assets)?;
            self.receipt.tick(&self.assets)?;
            self.jacked.tick(&self.assets)?;
            self.dimmer.tick(&self.assets)?;
            // AptLinker::Update links the loaded screens after the tick's actions.
            for screen in linked {
                match screen {
                    Screen::StartHub => self.pause.link(&self.assets)?,
                    Screen::Contacts=>self.contacts.link(&self.assets)?,
                    Screen::ReplayEditor => self.replay_editor.link(&self.assets)?,
                    Screen::TrickGuide=>self.trick_guide.link(&self.assets,&mut self.trick_guide_letterbox)?,
                    Screen::SkateProfile => self.skate_profile.link(&self.assets)?,
                    Screen::ReplayFootage => self.replay_footage.link(&self.assets)?,
                    Screen::OnlineOptions|Screen::SkateReelGeneral|Screen::SkateReelPhoto=>{
                        let i=match screen {Screen::OnlineOptions=>0,Screen::SkateReelGeneral=>1,_=>2};
                        if let Some(main)=self.main.as_mut() {self.skate_reel[i].link(&self.assets,&mut main.letterbox)?;}
                    }
                    Screen::Options => self.settings.link(&self.assets, self.main.as_mut().map(|m| &mut m.letterbox))?,
                    Screen::MusicPlayer => self.music_player.link(&self.assets)?,
                    Screen::MainMenu => {
                        if let Some(main)=self.main.as_mut() {
                            main.link(&self.assets)?;
                        }
                    }
                    Screen::Extras => {
                        if let Some(main)=self.main.as_mut() {
                            self.extras.link(&self.assets,&mut main.letterbox)?;
                        }
                    }
                    Screen::Dlc => {
                        if let Some(main)=self.main.as_mut() {
                            self.dlc.link(&self.assets,&mut main.letterbox)?;
                        }
                    }
                }
            }
            if self.music_player.take_letterbox_off() {
                self.settings.enable_letterbox(&self.assets, false, self.main.as_mut().map(|m| &mut m.letterbox))?;
            }
            // StateManagerFrontEndBase::Update asks for the screens of the states that entered.
            self.screens.update_states();
        }
        Ok(())
    }
    /// The DPO HUD's `LoadRX2Texture`: the named icon, loaded and uploaded once.
    fn resolve_dpo_icon(&mut self, renderer: &GpuRenderer) -> RenderResult<()> {
        let Some(name) = self.dpo.icon_request.take() else { return Ok(()) };
        let texture = match self.assets.ensure_dpo_icon(&name) {
            Ok((key, loaded)) => {
                if loaded {
                    let uploaded = renderer.upload_ui_textures(self.assets.textures.get(&key).into_iter())?;
                    self.textures.extend(uploaded);
                }
                Some(key)
            }
            Err(_) => None,
        };
        self.dpo.set_icon(texture)
    }
    /// The sponsor overlay's `LoadRX2Texture` logos, each loaded and uploaded once.
    fn resolve_sponsor_logos(&mut self, renderer: &GpuRenderer) -> RenderResult<()> {
        for (index, path) in self.sponsor_overlay.image_requests() {
            let texture = match self.assets.ensure_rx2_image(&path) {
                Ok((key, loaded)) => {
                    if loaded {
                        let uploaded = renderer.upload_ui_textures(self.assets.textures.get(&key).into_iter())?;
                        self.textures.extend(uploaded);
                    }
                    Some(key)
                }
                Err(_) => None,
            };
            self.sponsor_overlay.set_image(index, texture);
        }
        Ok(())
    }
    /// Keep Update (FrontEndManager 0x82640130) separate from the APT render
    /// list consumer (AptRender 0x829f6b20 / Render2D 0x82631a98). Intermediate
    /// catch-up ticks still execute all scripts, resource requests and sounds.
    pub(crate) fn begin_geometry_batch(&mut self) {
        self.defer_geometry = true;
    }
    pub(crate) fn finish_geometry_batch(&mut self, renderer: &GpuRenderer, scene: &mut GpuScene) -> RenderResult<()> {
        self.defer_geometry = false;
        if std::mem::take(&mut self.geometry_pending) {
            self.draw_geometry(renderer, scene)?;
        }
        Ok(())
    }
    fn resolve_footage_textures(&mut self,renderer:&GpuRenderer)->RenderResult<()> {
        for (replay,slot) in self.replay_footage.texture_requests() {
            let (key,loaded)=self.assets.ensure_replay_thumbnail(replay,slot)?;
            if loaded {self.textures.extend(renderer.upload_ui_textures(self.assets.textures.get(&key).into_iter())?);}
        }
        Ok(())
    }
    fn draw(&mut self, renderer: &GpuRenderer, scene: &mut GpuScene) -> RenderResult<()> {
        self.resolve_dpo_icon(renderer)?;
        self.resolve_sponsor_logos(renderer)?;
        self.resolve_footage_textures(renderer)?;
        for (slot, thumbnail) in self.slappys.photo_requests() {
            let texture = match self.assets.ensure_career_photo(slot, thumbnail) {
                Ok(Some((key, loaded))) => {
                    if loaded { self.textures.extend(renderer.upload_ui_textures(self.assets.textures.get(&key).into_iter())?); }
                    Some(key)
                }
                Ok(None) => None,
                Err(error) => {
                    eprintln!("career: could not load photo slot {slot}: {error}");
                    None
                }
            };
            self.slappys.set_photo(slot, thumbnail, texture.as_deref());
        }
        if self.defer_geometry {
            self.geometry_pending = true;
        } else {
            self.geometry_pending = false;
            self.draw_geometry(renderer, scene)?;
        }
        // Drain sounds every update: several movie owners clear their lists
        // on the next tick. Batching geometry must not lose these events.
        self.finish_frame(renderer, scene)
    }
    /// Whether the front-end state on top, a NIS or the free-skate settings hide the HUD.
    fn hud_hidden(&self) -> bool {
        // StartHub gFEStateInfo.mShowHUD=HideExceptOnline; no online HUD here.
        // The sign-up poster's (32), unlock overlay's (10), end-of-challenge (8), change-game
        // (39) and movie player's (28) states hide the HUD outright, as FrontEndState_NIS and the
        // NIS HUD flag do.
        // PhoneList FE state74 has mShowHUD=Hide (82fdf348).
        self.contacts.open() || self.pause_state.visible || self.signup_poster.open || self.unlock_overlay.open || self.sponsor_overlay.open
            || self.slappys.open()
            || self.throwdown_results.open || self.end_challenge.open || self.end_small.open || self.failed.open || self.online_results.open
            || self.failed_small.open
            || self.hom_results.open || self.hom_annals.open || self.change_game.open || self.photo_select.open || self.movie_open
            || self.career.as_ref().is_some_and(|career| career.nis.hud_hidden)
            || self.free_skate_settings.is_some_and(|(hud, _)| !hud)
            // gFEStateInfo[44], [45], [68] and [69]: mShowHUD Hide.
            || self.online_hub.open() || self.skate_profile.open() || (self.main.is_none() && self.online_hub_dim)
    }
    fn draw_geometry(&self, renderer: &GpuRenderer, scene: &mut GpuScene) -> RenderResult<()> {
        let mut batches = Vec::new();
        if self.trick_guide.open() {
            for clip in [&self.trick_guide.root,&self.trick_guide_letterbox.clip] {
                flatten_projected(&self.assets,clip,IDENTITY,Colour::default(),renderer.dimensions(),&mut batches,true,None)?;
            }
            return renderer.update_marker_ui(scene,&self.textures,batches);
        }
        if self.replay_editor.open() || self.replay_footage.open() {
            flatten_projected(&self.assets,&self.dimmer.clip,IDENTITY,Colour::default(),renderer.dimensions(),&mut batches,true,None)?;
            let editor=(&self.replay_editor.root,&self.replay_editor.popup);
            let footage=(&self.replay_footage.root,&self.replay_footage.popup);
            let screens=if self.screens.over(Screen::ReplayEditor,Screen::ReplayFootage) {[footage,editor]} else {[editor,footage]};
            for (root,popup) in screens {
                flatten_projected(&self.assets,root,IDENTITY,Colour::default(),renderer.dimensions(),&mut batches,true,None)?;
                if let Some(popup)=&popup.popup {
                    flatten_projected(&self.assets,popup,IDENTITY,Colour::default(),renderer.dimensions(),&mut batches,true,None)?;
                }
            }
            return renderer.update_marker_ui(scene,&self.textures,batches);
        }
        // gDimmer: mainmisc.swf's _level8, which AptRenderingIntegration::Render2D (0x82631a98)
        // draws first, under the 3D levels and main.swf's screens.
        flatten_projected(&self.assets, &self.dimmer.clip, IDENTITY, Colour::default(),
            renderer.dimensions(), &mut batches, true, None)?;
        // main.swf's gSubtitle: a _level0 ScreenLayer clip, under the loaded
        // levels the HUD and the screens use.
        flatten(&self.assets, &self.subtitle.root, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches)?;
        // Focus_Left826082b0 adds the same landing offset to every 3D HUD
        // matrix's ty before projecting, including the marker phone.
        let hud_matrix = Matrix { ty: self.jolt.offset(), ..IDENTITY };
        let hud_hidden = self.hud_hidden();
        if !hud_hidden {
        self.draw_minimap(renderer.dimensions(),hud_matrix,&mut batches)?;
        // ObjectiveHUD's template places hudobjective2 in g3dHUDRight.
        let start = batches.len();
        flatten_projected(&self.assets, &self.objective.root, hud_matrix, Colour::default(),
            (1280, 720), &mut batches, true, None)?;
        project_focus_right(renderer.dimensions(), &mut batches[start..]);
        // TutorialInstructionHUD's template places hudtutorial there too.
        let start = batches.len();
        flatten_projected(&self.assets, &self.tutorial.root, hud_matrix, Colour::default(),
            (1280, 720), &mut batches, true, None)?;
        project_focus_right(renderer.dimensions(), &mut batches[start..]);
        // LeaderboardHUD's (0x82fdb848), OnlineVotingHUD's (0x82fdb8c8) and DPOHUD's
        // (0x82fdb888) templates place theirs there too.
        for root in [&self.leaderboard.root, &self.voting.root, &self.dpo.root] {
            let start = batches.len();
            flatten_projected(&self.assets, root, hud_matrix, Colour::default(), (1280, 720), &mut batches, true, None)?;
            project_focus_right(renderer.dimensions(), &mut batches[start..]);
        }
        if self.hud_binding.trick_display == Some(true) {
            flatten(&self.assets, &self.score.root, hud_matrix, Colour::default(),
                renderer.dimensions(), &mut batches)?;
        }
        // ABDHUD's template (0x82fdb878) places hudabd in HUD layer 5, as the score.
        flatten(&self.assets, &self.abd.root, hud_matrix, Colour::default(), renderer.dimensions(), &mut batches)?;
        // HUD2_OnlineEventLog's template (0x82fdb8b8) is placement 5 too.
        if self.online_event_log_open && !self.online_event_log_hidden {
            flatten(&self.assets, &self.online_event_log.root, hud_matrix, Colour::default(), renderer.dimensions(), &mut batches)?;
        }
        // OnlineSpectatorOverlay's skate.TV overlay, in HUD layer 5 as the event log.
        flatten(&self.assets, &self.spectator.root, hud_matrix, Colour::default(), renderer.dimensions(), &mut batches)?;
        // FreeskateActivityOverlayHUD's template (0x82fdb8e8) is placement 5 as well.
        flatten(&self.assets, &self.activity_overlay.root, hud_matrix, Colour::default(), renderer.dimensions(), &mut batches)?;
        flatten(&self.assets,&self.central.root,IDENTITY,Colour::default(),renderer.dimensions(),&mut batches)?;
        // SignUpHUD's template places actionoverlay2 at placement 0, as hudcentral.
        flatten(&self.assets,&self.action_overlay.root,IDENTITY,Colour::default(),renderer.dimensions(),&mut batches)?;
        // TimerHUD's (0x82fdb7d8) and ThrowDownMessageHUD's (0x82fdb7f8) templates are
        // placement 0 as well.
        flatten(&self.assets, &self.timer.root, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches)?;
        // BigBlackTimerHUD template73 (82fdb7e8) uses g3dHUDRight/type6.
        // Keep bbtimer.apt's authored transform/size; only its native layer projects it.
        let start = batches.len();
        flatten_projected(&self.assets, &self.big_black_timer.root, hud_matrix, Colour::default(),
            (1280, 720), &mut batches, true, None)?;
        project_focus_right(renderer.dimensions(), &mut batches[start..]);
        flatten(&self.assets, &self.throwdown_message.root, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches)?;
        // So are DistanceMeter's (0x82fdb808) and CountdownHUD's (0x82fdb828).
        flatten(&self.assets, &self.distance.root, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches)?;
        flatten(&self.assets, &self.countdown.root, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches)?;
        flatten(
            &self.assets,
            &self.root,
            hud_matrix,
            Colour::default(),
            renderer.dimensions(),
            &mut batches,
        )?;
        }
        // StartHub layer2 uses the same FocusLeft matrix as HUD layer5; main.swf's _level0
        // screens draw over it.
        // FrontEndManager::ShowScreen(false) hides the hub's screen during its quit's save.
        if !self.pause_state.screen_hidden {
            flatten(&self.assets, &self.pause.root, hud_matrix, Colour::default(),
                renderer.dimensions(), &mut batches)?;
        }
        // OnlineHubMenu's template entry (0x82fdb918) is placement 2, as the StartHub.
        flatten(&self.assets, &self.online_hub.root, hud_matrix, Colour::default(), renderer.dimensions(), &mut batches)?;
        // UnrankedFreeskate's and UnrankedCustomMatch's (0x82fdba28, 0x82fdba38) are placement 0.
        if self.main.is_none() {
            for clip in [&self.online_custom.root, &self.online_freeskate.root] {
                flatten_projected(&self.assets, clip, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches, true, None)?;
            }
        }
        // FreeskateActivity_SignUp's template entry (0x82fdba18) is placement 0.
        if let Some(root) = &self.activity_signup.root {
            flatten_projected(&self.assets, root, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches, true, None)?;
        }
        // PhoneList template13 (82fdb428) is type2/g3dFocusLeft, just like
        // StartHub. The authored menu and its intro/outro retain their APT matrices.
        flatten_projected(&self.assets, &self.contacts.root, hud_matrix, Colour::default(),
            renderer.dimensions(), &mut batches, false, None)?;
        // PartyPlayChangeGame's template entry (0x82fdb768) is placement 0.
        if let Some(root) = &self.change_game.root {
            flatten(&self.assets, root, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches)?;
        }
        flatten_projected(&self.assets, &self.map.root, IDENTITY, Colour::default(),
            renderer.dimensions(), &mut batches, true, Some(&self.map_facts))?;
        // Options' screen 120, the music player's 35 and skate.Profile's 93 are flat screens in
        // gScreenManager (depth 12), each at the depth its ReadyScreen gave it, so the one opened
        // later draws over the others; gLetterBox (14) and gChyron (15) are above.

        if let Some(popup)=&self.contacts.popup.popup {flatten_projected(&self.assets,popup,IDENTITY,Colour::default(),renderer.dimensions(),&mut batches,true,None)?;}
        let mut screens = vec![(Screen::Options, Some(&self.settings.root)), (Screen::MusicPlayer, Some(&self.music_player.root)),
            (Screen::SkateProfile, self.skate_profile.root.as_ref())];
        screens.sort_by(|(a, _), (b, _)| if self.screens.over(*a, *b) {std::cmp::Ordering::Greater}
            else if self.screens.over(*b, *a) {std::cmp::Ordering::Less} else {std::cmp::Ordering::Equal});
        for clip in screens.into_iter().filter_map(|(_, clip)| clip) {
            flatten_projected(&self.assets, clip, IDENTITY, Colour::default(),
                renderer.dimensions(), &mut batches, true, None)?;
        }
        flatten(&self.assets, &self.map.letterbox, IDENTITY, Colour::default(),
            renderer.dimensions(), &mut batches)?;
        if !self.settings.in_front_end() {
            flatten_projected(&self.assets, &self.settings.letterbox.clip, IDENTITY, Colour::default(),
                renderer.dimensions(), &mut batches, true, None)?;
        }
        flatten_projected(&self.assets, &self.chyron.root, IDENTITY, Colour::default(),
            renderer.dimensions(), &mut batches, true, None)?;
        flatten(&self.assets,&self.jacked.root,IDENTITY,Colour::default(),
            renderer.dimensions(),&mut batches)?;
        // Native layer0 follows layer2, including the menu's closing animation. A challenge
        // group's hidden homscoring instance keeps "main"'s from being bound.
        if !hud_hidden && matches!(self.hud_binding.hom_scoring,
            skate2_core::challenge::hud::Binding::Main | skate2_core::challenge::hud::Binding::Group(true)) {
            flatten(&self.assets, &self.hom.root, IDENTITY, Colour::default(),
                renderer.dimensions(), &mut batches)?;
        }
        // SignUp_Poster's template entry places challenge_signup at
        // placement 0, drawn flat; its outro plays after the HUD returns.
        if let Some(root) = &self.signup_poster.root {
            flatten(&self.assets, root, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches)?;
        }
        // Skate101's template entry places skate101 at placement 0, drawn
        // flat; its state leaves the HUD as it is.
        if let Some(root) = &self.skate101.root {
            flatten(&self.assets, root, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches)?;
        }
        // PhotoSelect's template entry (0x82fdba98) places photoselect in g3dHUDRight; its
        // state hides the HUD (gFEStateInfo[5], HudManagerShow(0)).
        if let Some(root)=&self.hom_results.root {
            flatten(&self.assets,root,IDENTITY,Colour::default(),renderer.dimensions(),&mut batches)?;
        }
        if let Some(root) = &self.photo_select.root {
            let start = batches.len();
            flatten_projected(&self.assets, root, IDENTITY, Colour::default(), (1280, 720), &mut batches, true, None)?;
            project_focus_right(renderer.dimensions(), &mut batches[start..]);
        }
        // PartyPlayPressA's template entry (0x82fdb758) is eFEScreenType_2D as well.
        if let Some(root) = &self.party_play_press.root {
            flatten(&self.assets, root, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches)?;
        }
        // ThrowDownEndChallengeOptions' (0x82fdb648), EndChallengeOptions' (0x82fdb5e8) and
        // EndChallengeOptions_Small's (0x82fdb5f8) template entries place their screens at
        // placement 4, the g3dSpinContainer, turned by AptIntegration's one spin.
        for root in [&self.throwdown_results.root, &self.end_challenge.root, &self.end_small.root].into_iter().flatten() {
            let start = batches.len();
            flatten_projected(&self.assets, root, IDENTITY, Colour::default(), (1280, 720), &mut batches, true, None)?;
            project_spin(renderer.dimensions(), self.spin.angle, &mut batches[start..]);
        }
        // RankedOnlineEndChallengeOptions' template entry (0x82fdbaa8) is placement 0.
        if let Some(root) = &self.online_results.root {
            flatten(&self.assets, root, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches)?;
        }
        // EndChallengeOptionsFail's (0x82fdb618) and EndChallengeOptionsFail_Small's (0x82fdb628)
        // template entries are placement 0.
        for root in [&self.failed.root, &self.failed_small.root].into_iter().flatten() {
            flatten(&self.assets, root, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches)?;
        }
        // The NIS overlay templates are placement 0 as well, each above the
        // ones opened before it (mScreenArrayDepth).
        for root in self.nis_screens.values().filter_map(NisScreen::root) {
            flatten(&self.assets, root, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches)?;
        }
        // UnlockOverlay's template places unlockoverlay at placement 4, the
        // g3dSpinContainer: flattened on the stage, then turned by the spin.
        if let Some(root) = &self.unlock_overlay.root {
            let start = batches.len();
            flatten_projected(&self.assets, root, IDENTITY, Colour::default(), (1280, 720), &mut batches, true, None)?;
            project_spin(renderer.dimensions(), self.spin.angle, &mut batches[start..]);
        }
        // Sponsor's template entry places sponsoroverlay at placement 4 as well.
        if let Some(root) = &self.sponsor_overlay.root {
            let start = batches.len();
            flatten_projected(&self.assets, root, IDENTITY, Colour::default(), (1280, 720), &mut batches, true, None)?;
            project_spin(renderer.dimensions(), self.spin.angle, &mut batches[start..]);
        }
        // SlappysBlog's, SlappysArticles' and SlappysCovers' template entries are placement 0.
        for root in self.slappys.roots() {
            flatten(&self.assets, root, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches)?;
        }
        // Annals screen128: template82fdbb58 placement0, above its parent screen.
        if let Some(root)=&self.hom_annals.root {
            flatten(&self.assets,root,IDENTITY,Colour::default(),renderer.dimensions(),&mut batches)?;
        }
        if let Some(popup)=&self.pause.popup {
            flatten_projected(&self.assets,popup,IDENTITY,Colour::default(),renderer.dimensions(),&mut batches,true,None)?;
        }
        if let Some(popup)=&self.online_hub.popup {
            flatten_projected(&self.assets,popup,IDENTITY,Colour::default(),renderer.dimensions(),&mut batches,true,None)?;
        }
        if let Some(popup)=&self.progression_popup.popup {
            flatten_projected(&self.assets,popup,IDENTITY,Colour::default(),renderer.dimensions(),&mut batches,true,None)?;
        }
        if let Some(popup) = &self.sign_up_choice.popup {
            flatten_projected(&self.assets, popup, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches, true, None)?;
        }
        if let Some(popup) = &self.map.popup {
            flatten_projected(&self.assets, popup, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches, true, None)?;
        }
        if let Some(popup) = &self.chat_popup.popup {
            flatten_projected(&self.assets, popup, IDENTITY, Colour::default(), renderer.dimensions(), &mut batches, true, None)?;
        }
        if let Some(tutorial) = &self.fmv.tutorial {
            flatten_projected(&self.assets,tutorial,fmv_tutorial_movie::PLACEMENT,Colour::default(),renderer.dimensions(),&mut batches,true,None)?;
        }
        renderer.update_marker_ui(scene, &self.textures, batches)
    }
    /// The front end screen manager's work on a NIS overlay screen.
    fn apply_nis_screen(&mut self, command: ScreenCommand, title: &MissionTitle, pro: &[String; 3]) -> RenderResult<()> {
        match command {
            ScreenCommand::Create { id, template } => {
                let screen = match template {
                    TUTORIAL_GESTURE_OLLIE => NisScreen::GestureOllie(gestollie_movie::GestOllieMovie::new(&self.assets)?),
                    SPOT_OWNED => NisScreen::SpotOwned(ownthespot_movie::OwnTheSpotMovie::new(&self.assets)?),
                    SPOT_FOUND => NisScreen::SpotFound(spotfound_movie::SpotFoundMovie::new(&self.assets)?),
                    SPOT_KILLED => NisScreen::KilledIt(killedit_movie::KilledItMovie::new(&self.assets)?),
                    PHOTO_TREATMENT => NisScreen::PhotoTreatment(phototreatment_movie::PhotoTreatmentMovie::new(&self.assets, phototreatment_movie::ASSET)?),
                    FIXED_PHOTO_TREATMENT => NisScreen::PhotoTreatment(phototreatment_movie::PhotoTreatmentMovie::new(&self.assets, phototreatment_movie::FIXED)?),
                    NON_FIXED_PHOTO_TREATMENT => NisScreen::PhotoTreatment(phototreatment_movie::PhotoTreatmentMovie::new(&self.assets, phototreatment_movie::NON_FIXED)?),
                    PRO_BIO_INFO => NisScreen::ProBio(pro_bio_movie::ProBioMovie::new(&self.assets)?),
                    GVR_REGULAR => NisScreen::Gvr(gvr_movie::GvrMovie::new(&self.assets, gvr_movie::REGULAR)?),
                    GVR_GOOFY => NisScreen::Gvr(gvr_movie::GvrMovie::new(&self.assets, gvr_movie::GOOFY)?),
                    SAN_VAN_TV => NisScreen::SanVanTv(sanvantv_movie::SanVanTvMovie::new(&self.assets)?),
                    _ => {
                        eprintln!("marker UI: NIS overlay screen template {template} not ported");
                        NisScreen::Unported
                    }
                };
                self.nis_screens.insert(id, screen);
            }
            ScreenCommand::SetState { id, state } => {
                if let Some(screen) = self.nis_screens.get_mut(&id) {
                    screen.set_state(&self.assets, state, title, pro)?;
                }
            }
            ScreenCommand::Destroy { id } => {
                if let Some(list) = self.nis_screens.get_mut(&id).and_then(NisScreen::sounds) {
                    self.pending_sounds.append(list);
                }
                self.nis_screens.remove(&id);
            }
        }
        Ok(())
    }
    fn update_fps(&mut self, renderer: &GpuRenderer, scene: &mut GpuScene) -> RenderResult<()> {
        let fps_key = (renderer.render_fps(), renderer.dimensions());
        if self.fps_cached != Some(fps_key) || scene.fps_ui.is_none() {
            let font = &self.assets.fonts[&self.fps_font];
            let atlas = &self.assets.textures[&font.texture];
            scene.fps_ui = renderer.upload_ui_batches(&self.textures,
                vec![fps_batch(font, atlas, fps_key.0, fps_key.1)?])?;
            self.fps_cached = Some(fps_key);
        }
        Ok(())
    }
    fn set_state(
        &mut self,
        state: ScreenState,
        permissions: MarkerPermissions,
        spectate: bool,
    ) -> RenderResult<()> {
        let minimized = self.data.minimized_menu(self.alert_facts);
        let quick = self.root.named(&["mcQuickMenu"])?;
        match state {
            ScreenState::Hidden => quick.goto(&self.assets, "hidden")?,
            ScreenState::Minimized => update_minimized_menu(quick, &self.assets, minimized)?,
            ScreenState::MaximizedWithOptions | ScreenState::MaximizedMarkerOnly => {
                quick.goto(&self.assets, "maximized")?;
                quick.named(&["mQuickOpen"])?.goto(
                    &self.assets,
                    if state == ScreenState::MaximizedWithOptions {
                        "main"
                    } else {
                        "main2"
                    },
                )?;
                self.refresh(permissions, spectate)?;
            }
            ScreenState::RefreshMarkerPermissions => self.refresh(permissions, spectate)?,
            ScreenState::TrySetMarker => quick
                .named(&["mQuickOpen", "mTextHelp1"])?
                .goto(&self.assets, "pulse")?,
            ScreenState::TryUseMarker => {} // Original SetState52 only traces.
        }
        Ok(())
    }
    /// The quick menu's rows; with `spectate`, the options row is an online game's B Spectate
    /// (PC port addition): its text, and its button (the unnamed one at depth 28 of `main`) B in
    /// place of the placement's own.
    fn refresh(&mut self, p: MarkerPermissions, spectate: bool) -> RenderResult<()> {
        /// `main`'s button beside the options row.
        const OPTIONS_BUTTON_DEPTH: i32 = 28;
        let menu = self.root.named(&["mcQuickMenu", "mQuickOpen"])?;
        for (help, allowed, label) in [
            ("mTextHelp0", p.use_allowed, "ID_PHONELIST_GOTO_MARKER"),
            ("mTextHelp1", p.set_allowed, "ID_PHONELIST_PLACE_MARKER"),
            ("mTextHelp2", true, "ID_PHONELIST_OPTIONS_TITLE"),
        ] {
            let text = menu.named(&[help, "mText"])?;
            if help != "mTextHelp2" {
                text.goto(&self.assets, if allowed { "enabled" } else { "disabled" })?;
            }
            text.named(&["labelText"])?.text = Some(if help == "mTextHelp2" && spectate {
                "Spectate".encode_utf16().collect()
            } else {
                self.assets.strings[label].clone()
            });
        }
        if let Some(button) = menu.children.get_mut(&OPTIONS_BUTTON_DEPTH) {
            let image = if spectate { Some("B".to_owned()) } else { button.clip.properties.get("mButtonImage").cloned() };
            if let Some(image) = image {
                button.clip.named(&["mButtonRender"])?.properties.insert("_test".into(), image);
            }
        }
        for name in ["labelText", "labelText_glow"] {
            menu.named(&["mcTextHeader", name])?.text =
                Some(self.assets.strings["ID_PHONELIST_TITLE"].clone());
        }
        Ok(())
    }
}

/// `hudphonelist.UpdateMinimizedMenu`: the quick menu plays `minimized` under
/// ID_PHONELIST_USE_PHONE, its alert line shown only for an alert, and each icon slot plays its
/// label; with nothing to show it plays `hidden`.
fn update_minimized_menu(quick: &mut Clip, assets: &Assets, menu: MinimizedMenu) -> RenderResult<()> {
    let (icons, alert) = match menu {
        MinimizedMenu::Hidden => return quick.goto(assets, "hidden"),
        MinimizedMenu::Services(icons) => (icons, None),
        MinimizedMenu::Alert(text) => (["off"; 2], Some(text)),
    };
    quick.goto(assets, "minimized")?;
    for name in ["labelText", "labelText_glow"] {
        quick.named(&["mcTextHeader", name])?.text = Some(assets.strings["ID_PHONELIST_USE_PHONE"].clone());
    }
    let line = quick.named(&["mAlert"])?;
    line.visible = alert.is_some();
    if let Some(text) = alert {
        for name in ["labelText", "labelTextGlow"] {
            line.named(&["mTextAlert", name])?.text = Some(assets.strings[text].clone());
        }
    }
    for (slot, label) in icons.into_iter().enumerate() {
        quick.named(&[&format!("mIconServices{slot}")])?.goto(assets, label)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Colour {
    scale: [u8; 4],
    bias: [u8; 4],
}
impl Default for Colour {
    fn default() -> Self {
        Self {
            scale: [255; 4],
            bias: [0; 4],
        }
    }
}
fn rgba(value: u32) -> [u8; 4] {
    [
        (value >> 16) as u8,
        (value >> 8) as u8,
        value as u8,
        (value >> 24) as u8,
    ]
}
impl Colour {
    fn append(self, scale: u32, bias: u32) -> Self {
        let s = rgba(scale);
        let b = rgba(bias);
        Self {
            scale: std::array::from_fn(|i| {
                ((f32::from(self.scale[i]) * f32::from(s[i])) * f32::from_bits(0x3b808081)) as u8
            }),
            bias: std::array::from_fn(|i| self.bias[i].saturating_add(b[i])),
        }
    }
    fn apply(self, colour: [f32; 4]) -> [f32; 4] {
        std::array::from_fn(|i| {
            colour[i].mul_add(
                f32::from(self.scale[i]) * f32::from_bits(0x3b808081),
                f32::from(self.bias[i]) * f32::from_bits(0x3b808081),
            )
        })
    }
}
fn point(m: Matrix, p: [f32; 2]) -> [f32; 2] {
    [
        m.c.mul_add(p[1], m.a * p[0]) + m.tx,
        m.d.mul_add(p[1], m.b * p[0]) + m.ty,
    ]
}
/// The PC port's `shape_bands` property: a shape drawn in horizontal bands, each
/// `[source_top, source_bottom, top, bottom]` drawing the shape between its first two heights
/// stretched between the last two, and the rest of the shape left out.
fn shape_bands(bands: &[[f32; 4]]) -> String {
    bands.iter().map(|band| band.map(|height| height.to_string()).join(",")).collect::<Vec<_>>().join(";")
}
fn parse_shape_bands(value: &str) -> RenderResult<Vec<[f32; 4]>> {
    value.split(';').map(|band| -> RenderResult<[f32; 4]> {
        let heights = band.split(',').map(str::parse).collect::<Result<Vec<f32>, _>>()?;
        heights.try_into().map_err(|_| "shape band is not four heights".into())
    }).collect()
}
/// A triangle list drawn in `bands` (`shape_bands`): each point to draw, with the shape's point
/// it shows.
fn band_triangles(triangles: &[[f32; 2]], bands: &[[f32; 4]]) -> Vec<([f32; 2], [f32; 2])> {
    let mut out = Vec::new();
    for triangle in triangles.chunks_exact(3) {
        for &[source_top, source_bottom, top, bottom] in bands {
            let polygon = clip_at(&clip_at(triangle, source_top, true), source_bottom, false);
            let scale = (bottom - top) / (source_bottom - source_top);
            for i in 1..polygon.len().saturating_sub(1) {
                for p in [polygon[0], polygon[i], polygon[i + 1]] {
                    out.push(([p[0], (p[1] - source_top).mul_add(scale, top)], p));
                }
            }
        }
    }
    out
}
/// The part of a convex polygon below the height `at` (at greater heights) or, without `below`,
/// above it.
fn clip_at(polygon: &[[f32; 2]], at: f32, below: bool) -> Vec<[f32; 2]> {
    let inside = |p: [f32; 2]| if below { p[1] >= at } else { p[1] <= at };
    let mut out = Vec::with_capacity(polygon.len() + 1);
    for (i, &p) in polygon.iter().enumerate() {
        let q = polygon[(i + 1) % polygon.len()];
        if inside(p) {
            out.push(p);
        }
        if inside(p) != inside(q) {
            let t = (at - p[1]) / (q[1] - p[1]);
            out.push([(q[0] - p[0]).mul_add(t, p[0]), at]);
        }
    }
    out
}

/// SetApt3DFocusLeftMatrix826082b0, default angle11.25/distance624.
/// Native inverse-view cancels the same APT camera's view. ViewMan::Init
/// 8284911c copies perspective60deg,near1,far10000 into camera7. Native
/// projection82840770 uses radians*2/pi (not a conventional tan-half-FOV).
fn project(p: [f32; 2], (width, height): (u32, u32)) -> [f32; 4] {
    static PROJECTION: std::sync::OnceLock<skate2_core::ui::apt_projection::FocusLeftProjection> =
        std::sync::OnceLock::new();
    let mut clip = PROJECTION
        .get_or_init(|| skate2_core::ui::apt_projection::FocusLeftProjection::new(11.25, 624.))
        .project(p);
    let aspect = width as f32 / height.max(1) as f32;
    clip[0] *= ((16. / 9.) / aspect).min(1.);
    clip[1] *= (aspect / (16. / 9.)).min(1.);
    clip
}
fn vertex(m: Matrix, p: [f32; 2], uv: [f32; 2], colour: [f32; 4], size: (u32, u32), flat: bool) -> [f32; 10] {
    let p = point(m, p);
    let c = if flat {
        // HOMScoringHUD template type0: original 1280x720 2D stage.
        let aspect = size.0 as f32 / size.1.max(1) as f32;
        [(p[0] / 640. - 1.) * ((16. / 9.) / aspect).min(1.),
         (1. - p[1] / 360.) * (aspect / (16. / 9.)).min(1.), 0., 1.]
    } else { project(p, size) };
    [
        c[0], c[1], c[2], c[3], uv[0], uv[1], colour[0], colour[1], colour[2], colour[3],
    ]
}
fn quad(
    m: Matrix,
    bounds: [f32; 4],
    uv: [f32; 4],
    colour: [f32; 4],
    size: (u32, u32),
    flat: bool,
) -> Vec<[f32; 10]> {
    let p = [
        [bounds[0], bounds[1]],
        [bounds[2], bounds[1]],
        [bounds[2], bounds[3]],
        [bounds[0], bounds[3]],
    ];
    let t = [
        [uv[0], uv[1]],
        [uv[2], uv[1]],
        [uv[2], uv[3]],
        [uv[0], uv[3]],
    ];
    [0, 1, 2, 0, 2, 3]
        .map(|i| vertex(m, p[i], t[i], colour, size, flat))
        .to_vec()
}
fn flatten(
    assets: &Assets,
    clip: &Clip,
    m: Matrix,
    colour: Colour,
    size: (u32, u32),
    out: &mut Vec<Batch>,
) -> RenderResult<()> {
    // Projection belongs to the screen, not an imported control's asset.
    // eFEScreenType_2D (0) templates draw on camera 8's flat stage, as do
    // main.swf's own layers (letterbox, subtitle): SignUpHUD 0x82fdb8f8,
    // SignUp_Poster 0x82fdb488, Skate101 0x82fdbbc8, the NIS overlays
    // 0x82fdb458..0x82fdb688, PhotoTreatment 0x82fdb728, Pro_Bio_Info
    // 0x82fdb5b8, gestollie 0x82fdbb88, the GVR badges 0x82fdbb98/0x82fdbba8
    // NISSanVanTV 0x82fdbbd8 and the fixed and non-fixed photo treatments
    // 0x82fdb738/0x82fdb748, PartyPlayPressA 0x82fdb758, PartyPlayChangeGame 0x82fdb768,
    // TimerHUD 0x82fdb7d8, ThrowDownMessageHUD 0x82fdb7f8, CountdownHUD 0x82fdb828,
    // EndChallengeOptionsFail 0x82fdb618, EndChallengeOptionsFail_Small 0x82fdb628 and
    // RankedOnlineEndChallengeOptions 0x82fdbaa8 among them.
    let flat = matches!(clip.asset, 3 | 9 | 11 | 13 | 14 | 32 | 33 | 35 | 36 | 37..=47 | 52 | 54 | 55 | 58 | 68 | 70 | 80 | 81 | 83 | 112);
    flatten_projected(assets, clip, m, colour, size, out, flat, None)
}
/// PC tall-window adaptation, as the backgrounds fill wide windows: in a window narrower than
/// 16:9 the letterbox's bars keep to the window's top and bottom edges rather than the
/// stage's. `mcLetterbox` (letterbox character 24) places the top bar and its film strip at
/// depths 20 and 23 and the bottom ones at 1 and 4; its `mInfo` (character 11) holds the page
/// title (`mTitle`, depth 3) in the top bar and the buttons (`mcBottomMenu`, depth 1) in the
/// bottom one. Answers the stage y shift of the child at `depth`.
fn letterbox_edge_shift(clip: &Clip, depth: i32, (width, height): (u32, u32)) -> f32 {
    if clip.asset != 11 {
        return 0.;
    }
    let fit = (width as f32 / height.max(1) as f32 / (16. / 9.)).min(1.);
    let shift = 360. * (1. - fit) / fit;
    match (clip.character, depth) {
        (24, 20 | 23) | (11, 3) => -shift,
        (24, 1 | 4) | (11, 1) => shift,
        _ => 0.,
    }
}
#[derive(Clone, Debug, Default)]
struct MapRenderFacts {
    snapshot: skate2_core::ui::map_menu::MapMenuSnapshot,
    locations: Vec<skate2_core::ui::map_menu::MapLocation>,
    /// World X/Z and Actor heading; retained while FRONTEND owns pause.
    player: Option<([f32; 2], f32)>,
}
fn flatten_projected(
    assets: &Assets, clip: &Clip, m: Matrix, colour: Colour,
    size: (u32, u32), out: &mut Vec<Batch>, flat: bool, map: Option<&MapRenderFacts>,
) -> RenderResult<()> {
    flatten_masked(assets, clip, m, colour, size, out, flat, map, &mut MaskDepth::default())
}
/// One clip and its display list while a movie draws; what the clip draws itself is hidden
/// where the movie's masks hold depth 0 at that moment.
fn flatten_masked(
    assets: &Assets, clip: &Clip, m: Matrix, colour: Colour, size: (u32, u32),
    out: &mut Vec<Batch>, flat: bool, map: Option<&MapRenderFacts>, mask: &mut MaskDepth,
) -> RenderResult<()> {
    if !clip.visible || (colour.scale[3] == 0 && colour.bias[3] == 0) {
        return Ok(());
    }
    let start = out.len();
    if let Some(texture)=clip.properties.get("cac_texture") {
        let texture=assets.texture(texture)?;
        let bounds=pause_movie::bounds(clip,assets)?.ok_or("CAC image component has no authored bounds")?;
        out.push(Batch {texture:Some(texture.name.clone()),wrapped:false,
            vertices:quad(m,[bounds.left,bounds.top,bounds.right,bounds.bottom],
                [0.,0.,1.,1.],colour.apply([1.;4]),size,flat)});
        mask.occlude(&mut out[start..], size);
        return Ok(());
    }
    let colour=if let Some(value)=clip.properties.get("cac_colour") {
        // swatch_renderer sets RGB transform while retaining inherited alpha.
        let rgb=rgba(value.parse::<u32>()?);
        let painted=colour.apply(rgb.map(|v|f32::from(v)/255.));
        Colour {scale:[0,0,0,colour.scale[3]],bias:[
            (painted[0]*255.) as u8,(painted[1]*255.) as u8,(painted[2]*255.) as u8,colour.bias[3]]}
    } else {colour};
    if clip.properties.get("_type").map(String::as_str)==Some("CustomTimeline") {
        replay_editor_movie::draw_timeline(assets,clip,m,colour,size,out,flat)?;
        mask.occlude(&mut out[start..],size);
        return Ok(());
    }
    if clip.properties.get("_type").map(String::as_str) == Some(stick_preview::TYPE) {
        stick_preview::draw(clip, m, colour, size, out, flat)?;
        mask.occlude(&mut out[start..], size);
        return Ok(());
    }
    if clip.properties.get("_type").map(String::as_str) == Some("MapRenderComponent") {
        render_map_component(assets, clip, m, size, out, flat, map)?;
        mask.occlude(&mut out[start..], size);
        return Ok(());
    }
    if clip.properties.get("_type").map(String::as_str) == Some("MiniMapComponent") {
        return Ok(()); // Custom drawing at depth3 belongs to draw_minimap.
    }
    if clip.properties.get("_type").map(String::as_str) == Some("CustomButton") {
        let button = clip
            .properties
            .get("_test")
            .ok_or("CustomButton has no texture name")?;
        let t = assets.texture(&format!("button_{button}.Texture"))?;
        out.push(Batch {
            texture: Some(t.name.clone()),
            wrapped: false,
            vertices: quad(
                m,
                [-24., -24., t.width as f32 - 24., t.height as f32 - 24.],
                [0., 0., 1., 1.],
                colour.apply([1.; 4]),
                size,
                flat,
            ),
        });
        mask.occlude(&mut out[start..], size);
        return Ok(());
    }
    match clip.definition(assets)? {
        Character::Shape { .. } => {
            // PC window adaptation: stretch background artwork only.
            // Original APT: map19 is the backdrop, letterbox3 the 1280-wide
            // bars, dimmer2 the 1280x720 textured screen, subtitle1 the
            // 1280-wide band behind a subtitle, and challenge_signup's
            // intro/outro transition: field30 and film37 (1280x720), the
            // strip42/44/47/51, its dust33 and light58/64, which no other
            // part of the poster draws. sk8popup2, receipt1, challengefailed2
            // and challengefailed_small2 (each failure screen's 1280x720 dim),
            // Skate101's shape2 and ranked_results_overall's shape8 are the
            // screens' full-stage backgrounds, separate from their panels.
            // Keep their authored matrices/UVs/fades and all foreground
            // geometry unchanged. Every one fills a wider window's width, and
            // one as tall as the stage a narrower window's height.
            let background = flat && matches!((clip.asset, clip.character),
                (9, 19) | (11, 3) | (12, 2) | (25, 2) | (36, 1)
                | (33, 30 | 33 | 37 | 42 | 44 | 47 | 51 | 58 | 64) | (74, 1) | (81, 1)
                | (challenge_failed_movie::ASSET | challenge_failed_small_movie::ASSET, 2)
                | (skate101_movie::ASSET, 2) | (online_results_movie::ASSET, 8));
            let background_width = if background {
                (size.0 as f32 / size.1.max(1) as f32 / (16. / 9.)).max(1.)
            } else { 1. };
            let bands = clip.properties.get("shape_bands").map(|s| parse_shape_bands(s)).transpose()?;
            if let Some(shape) = assets.movies[clip.asset]
                .geometry
                .shape(clip.character as i32)
            {
                let (top, bottom) = shape.elements.iter().flat_map(|e| &e.positions)
                    .map(|&p| point(m, p)[1])
                    .fold((f32::INFINITY, f32::NEG_INFINITY), |(top, bottom), y| (top.min(y), bottom.max(y)));
                let background_height = if background && bottom - top >= 719. {
                    (size.1 as f32 / size.0.max(1) as f32 * (16. / 9.)).max(1.)
                } else { 1. };
                for e in &shape.elements {
                    if e.kind == GeometryKind::Line {
                        continue;
                    }
                    let texture =
                        if e.kind == GeometryKind::Solid {
                            None
                        } else {
                            Some(assets.texture(&format!(
                                "{}_textures\\{}.Texture",
                                assets.movies[clip.asset].texture_owner,
                                e.texture_id
                            ))?)
                        };
                    let c = colour.apply(e.colour);
                    if c[3] <= 0. {
                        continue;
                    }
                    let to_vertex = |(p, source): ([f32; 2], [f32; 2])| {
                        let uv = texture.map_or([0.; 2], |t| {
                            let q = point(e.texture_matrix, source);
                            [q[0] / t.width as f32, q[1] / t.height as f32]
                        });
                        let mut v = vertex(m, p, uv, c, size, flat);
                        v[0] *= background_width;
                        v[1] *= background_height;
                        v
                    };
                    let vertices = if let Some(bands) = &bands {
                        band_triangles(&e.positions, bands).into_iter().map(to_vertex).collect()
                    } else {
                        e.positions.iter().map(|&p| to_vertex((p, p))).collect()
                    };
                    out.push(Batch {
                        texture: texture.map(|t| t.name.clone()),
                        wrapped: e.kind == GeometryKind::TextureWrapped,
                        vertices,
                    });
                }
            }
        }
        Character::Text(text) => {
            let definition = assets.movies[clip.asset]
                .movie
                .characters
                .get(text.font_id as usize)
                .and_then(Option::as_ref)
                .ok_or("APT text font absent")?;
            let Character::Font { name, .. } = definition else {
                return Err("APT font ID not a font".into());
            };
            let name = clip.properties.get("text_font").unwrap_or(name);
            let font = assets.apt_font(name)?;
            let t = assets
                .textures
                .get(&font.texture)
                .ok_or("font atlas missing")?;
            let units = clip.text.as_deref().ok_or("APT text was not initialized")?;
            // Original button_menu.SetInfo changes its TextFormat at runtime.
            let height = clip.properties.get("text_height")
                .map(|s| s.parse::<f32>()).transpose()?.unwrap_or(text.height);
            let text_colour = clip.properties.get("text_colour")
                .map(|s| s.parse::<u32>()).transpose()?.unwrap_or(text.colour);
            let scale = font.scale.map(|s| s * height);
            let c = colour.apply(rgba(text_colour).map(|v| f32::from(v) * f32::from_bits(0x3b808081)));
            let mut vertices = Vec::new();
            let width = clip.properties.get("text_width").map(|s|s.parse::<f32>()).transpose()?.unwrap_or(text.bounds.right-text.bounds.left);
            let multiline = clip.properties.get("text_multiline").map_or(text.multiline, |s|s=="true");
            let lines = native_text_lines(units, multiline, width, |c| {
                font.metrics.mapped_glyph(c).map_or(0., |g| g.advance * scale[0])
            });
            // PC: the text between the highlight marks (a prompt's Ctrl+A) sits on a selection's
            // blue, in white; the marks draw nothing.
            let mut highlighting = false;
            let mut highlights = Vec::new();
            for (line_index, line) in lines.into_iter().enumerate() {
                let extent = line
                    .iter()
                    .filter_map(|&c| font.metrics.mapped_glyph(c))
                    .map(|g| g.advance * scale[0])
                    .sum::<f32>();
                let left=clip.properties.get("text_left").map(|s|s.parse::<f32>()).transpose()?.unwrap_or(text.bounds.left);
                let top=clip.properties.get("text_top").map(|s|s.parse::<f32>()).transpose()?.unwrap_or(text.bounds.top);
                let alignment=clip.properties.get("text_alignment").map(|s|s.parse::<u32>()).transpose()?.unwrap_or(text.alignment as u32);
                let align = match alignment {
                    0 => 0.,
                    1 => width - extent,
                    2 => (width - extent) * 0.5,
                    _ => return Err("unhandled APT text alignment".into()),
                };
                // AllocateString82601fe0 supplies line offsets0,0 and scale1.
                let y = top + line_index as f32 * font.metrics.line_height * scale[1];
                let mut pen = left + align;
                let mut span: Option<[f32; 2]> = None;
                let mut lit = Vec::with_capacity(line.len());
                for &unit in line {
                    match unit {
                        TEXT_HIGHLIGHT_START => highlighting = true,
                        TEXT_HIGHLIGHT_END => highlighting = false,
                        _ => {
                            let Some(glyph) = font.metrics.mapped_glyph(unit) else { continue };
                            let advance = glyph.advance * scale[0];
                            if highlighting {
                                span = Some(span.map_or([pen, pen + advance], |[from, _]| [from, pen + advance]));
                            }
                            pen += advance;
                            lit.push(highlighting);
                        }
                    }
                }
                if let Some([from, to]) = span {
                    highlights.push([from, y, to, font.metrics.line_height.mul_add(scale[1], y)]);
                }
                let white = colour.apply([1.0; 4]);
                for ((bounds, uv), lit) in glyph_quads(font, t, line, scale, [left + align, y]).zip(lit.into_iter().chain(std::iter::repeat(false))) {
                    vertices.extend(quad(m, bounds, uv, if lit { white } else { c }, size, flat));
                }
            }
            if !highlights.is_empty() {
                let blue = colour.apply(TEXT_HIGHLIGHT_COLOUR);
                out.push(Batch {
                    texture: None,
                    wrapped: false,
                    vertices: highlights.into_iter().flat_map(|bounds| quad(m, bounds, [0.0; 4], blue, size, flat)).collect(),
                });
            }
            out.push(Batch {
                texture: Some(t.name.clone()),
                wrapped: false,
                vertices,
            });
        }
        Character::Bitmap { .. } => {
            return Err("standalone marker bitmap requires placement implementation".into());
        }
        Character::Sprite(_) | Character::Animation(_) | Character::Font { .. } => {}
    }
    mask.occlude(&mut out[start..], size);
    // AptDecoupleTreeTraversal (0x82a003d0) walks the display list in depth order. A
    // placement whose clip depth is below 0x8000 as u16 is a mask (0x82a004c8):
    // AptDecoupleClippedTreeTraversal (0x829f8bf8) Adds its geometry and that of each
    // following sibling past its clip depth. The siblings up to the clip depth draw under it,
    // a mask among them as content, and the first sibling past it or the list's end Subtracts
    // the same geometry (AptDecoupleTreeTraversalClipper 0x829f8a88).
    let children: Vec<_> = clip.children.iter().collect();
    let mut active: Option<(i16, std::ops::Range<usize>)> = None;
    let mut index = 0;
    while let Some(&(&depth, child)) = children.get(index) {
        if !active.as_ref().is_some_and(|(clip_depth, _)| *clip_depth >= depth as i16) {
            if let Some((_, geometry)) = active.take() {
                mask_siblings(assets, &children[geometry], m, colour, mask, MaskOperation::Subtract)?;
            }
            if (child.entry.clip_depth as u32 & 0xffff) < 0x8000 {
                if !flat {
                    return Err("Apt mask on a perspective level not ported".into());
                }
                let clip_depth = child.entry.clip_depth as i16;
                let mut end = index + 1;
                while children.get(end).is_some_and(|&(&next, _)| next as i16 > clip_depth) {
                    end += 1;
                }
                mask_siblings(assets, &children[index..end], m, colour, mask, MaskOperation::Add)?;
                active = Some((clip_depth, index..end));
                index = end;
                continue;
            }
        }
        let mut m = concatenate(m, child.entry.matrix);
        if flat {
            m.ty += letterbox_edge_shift(clip, depth, size);
        }
        let colour = colour.append(child.entry.colour_scale, child.entry.colour_bias);
        flatten_masked(assets, &child.clip, m, colour, size, out, flat, map, mask)?;
        index += 1;
    }
    if let Some((_, geometry)) = active {
        mask_siblings(assets, &children[geometry], m, colour, mask, MaskOperation::Subtract)?;
    }
    Ok(())
}

/// The `AptMaskRenderOperation` a mask's geometry draws with.
#[derive(Clone, Copy)]
enum MaskOperation {
    Add,
    Subtract,
}

/// The UI depth buffer's mask rectangles while one movie draws
/// (`AptRenderingIntegration::DrawRenderingUnit` 0x8261da48): an Add writes its rectangle at
/// depth 0 and keeps it in `m_maskX..m_maskY1`, a Subtract writes depth 1 back over the kept
/// rectangle, and content drawn at z 0.5 with LESSEQUAL (`ZTestEnable_ZWriteDisable`) is
/// hidden where depth 0 remains.
#[derive(Default)]
struct MaskDepth {
    hidden: Vec<[f32; 4]>,
    kept: Option<[f32; 4]>,
}

impl MaskDepth {
    fn add(&mut self, rect: [f32; 4]) {
        self.hidden.push(rect);
        self.kept = Some(rect);
    }

    fn subtract(&mut self) {
        let Some(kept) = self.kept else { return };
        self.hidden = self.hidden.iter().flat_map(|&rect| rect_minus(rect, kept)).collect();
    }

    fn occlude(&self, batches: &mut [Batch], size: (u32, u32)) {
        for &rect in &self.hidden {
            occlude(batches, rect, size);
        }
    }
}

/// The parts of rectangle `a` outside rectangle `b`.
fn rect_minus(a: [f32; 4], b: [f32; 4]) -> Vec<[f32; 4]> {
    let inner = [a[0].max(b[0]), a[1].max(b[1]), a[2].min(b[2]), a[3].min(b[3])];
    if !(inner[0] < inner[2] && inner[1] < inner[3]) {
        return vec![a];
    }
    let mut parts = Vec::new();
    if a[1] < inner[1] {
        parts.push([a[0], a[1], a[2], inner[1]]);
    }
    if inner[3] < a[3] {
        parts.push([a[0], inner[3], a[2], a[3]]);
    }
    if a[0] < inner[0] {
        parts.push([a[0], inner[1], inner[0], inner[3]]);
    }
    if inner[2] < a[2] {
        parts.push([inner[2], inner[1], a[2], inner[3]]);
    }
    parts
}

/// Each listed placement's mask geometry under its own matrix and colour.
fn mask_siblings(
    assets: &Assets, siblings: &[(&i32, &clips::Child)], m: Matrix, colour: Colour, mask: &mut MaskDepth,
    operation: MaskOperation,
) -> RenderResult<()> {
    for (_, child) in siblings {
        let m = concatenate(m, child.entry.matrix);
        let colour = colour.append(child.entry.colour_scale, child.entry.colour_bias);
        mask_geometry(assets, &child.clip, m, colour, mask, operation)?;
    }
    Ok(())
}

/// `AptDecoupleTreeTraversalClipper` (0x829f8a88) for one item: one with a display list draws
/// its children in depth order under its matrix and colour, any other its own rendering unit.
fn mask_geometry(
    assets: &Assets, clip: &Clip, m: Matrix, colour: Colour, mask: &mut MaskDepth,
    operation: MaskOperation,
) -> RenderResult<()> {
    if !clip.children.is_empty() {
        for child in clip.children.values() {
            let m = concatenate(m, child.entry.matrix);
            let colour = colour.append(child.entry.colour_scale, child.entry.colour_bias);
            mask_geometry(assets, &child.clip, m, colour, mask, operation)?;
        }
        return Ok(());
    }
    // DrawRenderingUnit draws nothing under an accumulated alpha scale of 0 (0x8261da98).
    if colour.scale[3] == 0 {
        return Ok(());
    }
    match clip.definition(assets)? {
        Character::Shape { .. } => {}
        Character::Sprite(_) | Character::Animation(_) => return Ok(()),
        _ => return Err("Apt mask geometry other than shapes not ported".into()),
    }
    match operation {
        MaskOperation::Add => mask.add(mask_rect(assets, clip, m)?),
        MaskOperation::Subtract => mask.subtract(),
    }
    Ok(())
}

/// DrawRenderingUnit's Add (0x8261db08..0x8261dc24): vertices 0 and 3 of the shape's first
/// rendering unit, moved by the accumulated translation alone, bound the rectangle written to
/// depth 0.
fn mask_rect(assets: &Assets, clip: &Clip, m: Matrix) -> RenderResult<[f32; 4]> {
    let shape = assets.movies[clip.asset]
        .geometry
        .shape(clip.character as i32)
        .ok_or("Apt mask geometry absent")?;
    let unit = shape.elements.first().ok_or("Apt mask shape has no rendering unit")?;
    if unit.kind != GeometryKind::Solid {
        return Err("Apt mask over a textured or line unit not ported".into());
    }
    let (Some(a), Some(b)) = (unit.positions.first(), unit.positions.get(3)) else {
        return Err("Apt mask unit has fewer than four vertices".into());
    };
    let span = |a: f32, b: f32| if a >= b { (b, a) } else { (a, b) };
    let (left, right) = span(a[0] + m.tx, b[0] + m.tx);
    let (top, bottom) = span(a[1] + m.ty, b[1] + m.ty);
    Ok([left, top, right, bottom])
}

/// What a mask rectangle at depth 0 hides from content drawn at 0.5 with
/// LESSEQUAL: each triangle keeps its parts in the four regions around the
/// rectangle, whose shared edges rasterise once. A rectangle without area
/// hides nothing.
fn occlude(batches: &mut [Batch], rect: [f32; 4], size: (u32, u32)) {
    let corner = |x, y| {
        let v = vertex(IDENTITY, [x, y], [0.; 2], [0.; 4], size, true);
        [v[0], v[1]]
    };
    let (a, b) = (corner(rect[0], rect[1]), corner(rect[2], rect[3]));
    let (x0, x1, y0, y1) = (a[0].min(b[0]), a[0].max(b[0]), a[1].min(b[1]), a[1].max(b[1]));
    if !(x0 < x1 && y0 < y1) {
        return;
    }
    // Half-planes as (axis, bound, keep the side below the bound).
    let regions: [&[(usize, f32, bool)]; 4] = [
        &[(0, x0, true)],
        &[(0, x1, false)],
        &[(0, x0, false), (0, x1, true), (1, y0, true)],
        &[(0, x0, false), (0, x1, true), (1, y1, false)],
    ];
    for batch in batches {
        let mut kept = Vec::with_capacity(batch.vertices.len());
        for triangle in batch.vertices.chunks_exact(3) {
            for planes in regions {
                let mut polygon = triangle.to_vec();
                for &(axis, bound, below) in planes {
                    polygon = clip_polygon(&polygon, axis, bound, below);
                }
                for i in 1..polygon.len().saturating_sub(1) {
                    kept.extend([polygon[0], polygon[i], polygon[i + 1]]);
                }
            }
        }
        batch.vertices = kept;
    }
}

/// One Sutherland-Hodgman step, every vertex attribute interpolated.
fn clip_polygon(polygon: &[[f32; 10]], axis: usize, bound: f32, below: bool) -> Vec<[f32; 10]> {
    let inside = |v: &[f32; 10]| if below { v[axis] <= bound } else { v[axis] >= bound };
    let mut out = Vec::with_capacity(polygon.len() + 2);
    for (i, p) in polygon.iter().enumerate() {
        let q = &polygon[(i + 1) % polygon.len()];
        if inside(p) {
            out.push(*p);
        }
        if inside(p) != inside(q) {
            let t = (bound - p[axis]) / (q[axis] - p[axis]);
            let mut v: [f32; 10] = std::array::from_fn(|k| (q[k] - p[k]).mul_add(t, p[k]));
            v[axis] = bound;
            out.push(v);
        }
    }
    out
}

/// SkateAptString82a70b28 wraps multiline fields at the last space and treats
/// '^' as an authored forced break. An unbroken word is never split. Explicit
/// newlines are retained for the existing host-generated multiline controls.
/// The marks a text puts around what it shows selected (PC: the lobby chat's Ctrl+A), private
/// use characters no font draws, and the selection's colour, a Windows selection's blue.
pub(super) const TEXT_HIGHLIGHT_START: u16 = 0xE000;
pub(super) const TEXT_HIGHLIGHT_END: u16 = 0xE001;
const TEXT_HIGHLIGHT_COLOUR: [f32; 4] = [0.0, 0.471, 0.843, 1.0];

fn native_text_lines<'a>(units: &'a [u16], multiline: bool, width: f32,
    advance: impl Fn(u16) -> f32) -> Vec<&'a [u16]> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut last_space = None;
    let mut extent = 0.;
    for (i, &c) in units.iter().enumerate() {
        if c == 10 || (multiline && c == 94) {
            lines.push(&units[start..i]);
            start = i + 1;
            extent = 0.;
            last_space = None;
            continue;
        }
        if multiline && c == 32 { last_space = Some(i); }
        extent += advance(c);
        if multiline && extent > width {
            if let Some(split) = last_space.take() {
                lines.push(&units[start..split]);
                start = split + 1;
                extent = units[start..=i].iter().map(|&c| advance(c)).sum();
            }
        }
    }
    lines.push(&units[start..]);
    lines
}

/// FE states 68 and 69's screens with the state they show, the location image each loads;
/// whether an intro completed.
fn update_online_match_movies(assets: &mut Assets, textures: &mut Textures, renderer: &GpuRenderer, movies: [&mut online_match_movie::OnlineMatchMovie; 2],
    snapshot: Option<&skate2_core::ui::online_match::Snapshot>, nickname: &str) -> RenderResult<bool> {
    let mut intro = false;
    for movie in movies {
        movie.update(assets, snapshot, nickname)?;
        // LoadRX2Texture of the location's small image.
        if let Some(image) = movie.image_request.take()
            && let Ok((key, loaded)) = assets.ensure_location_texture(&image)
        {
            if loaded {
                textures.extend(renderer.upload_ui_textures(assets.textures.get(&key).into_iter())?);
            }
            movie.set_image(&key)?;
        }
        intro |= movie.take_intro_complete();
    }
    Ok(intro)
}

/// The Lobby Browser's screen with the state it shows and the location image of each card in
/// view (`LoadRX2Texture`); whether its intro completed.
fn update_lobby_browser_movie(assets: &mut Assets, textures: &mut Textures, renderer: &GpuRenderer, movie: &mut lobby_browser_movie::LobbyBrowserMovie,
    snapshot: Option<&skate2_core::ui::lobby_browser::Snapshot>) -> RenderResult<bool> {
    let mut images = std::collections::BTreeMap::new();
    for lobby in snapshot.map_or(&[][..], |snapshot| snapshot.lobbies.as_slice()) {
        if let Ok((key, loaded)) = assets.ensure_location_texture(&lobby.image) {
            if loaded {
                textures.extend(renderer.upload_ui_textures(assets.textures.get(&key).into_iter())?);
            }
            images.insert(lobby.image.clone(), key);
        }
    }
    movie.update(assets, snapshot, &images)?;
    Ok(movie.take_intro_complete())
}

/// CustomControlRender826363xx -> FEMapManager::RenderMap82631f70.
/// Its original custom shape supplies pPositions[1] as symmetric half extents.
fn render_map_component(assets: &Assets, clip: &Clip, m: Matrix,
    size: (u32, u32), out: &mut Vec<Batch>, flat: bool,
    facts: Option<&MapRenderFacts>,
) -> RenderResult<()> {
    use skate2_core::ui::{map_menu::MapCategory, map_viewport::MapViewport};
    let shape_clip = clip.children.values().find(|child| {
        assets.movies[child.clip.asset].geometry.shape(child.clip.character as i32).is_some()
    }).ok_or("MapRenderComponent has no authored shape")?;
    let shape = assets.movies[shape_clip.clip.asset].geometry
        .shape(shape_clip.clip.character as i32).ok_or("map geometry absent")?;
    let element = shape.elements.first().ok_or("map shape has no render strip")?;
    let half = element.positions.get(1).ok_or("map strip has no second position")?;
    let bounds = [-half[0].abs(), -half[1].abs(), half[0].abs(), half[1].abs()];
    let m = concatenate(m, shape_clip.entry.matrix);
    let info = &assets.map_info;
    let texture = assets.texture(&info.texture)?;
    // Native RenderMap uses white, not inherited colour; caller tests visibility.
    out.push(Batch { texture: Some(texture.name.clone()), wrapped: false,
        vertices: quad(m, bounds, [0.,0.,1.,1.], [1.;4], size, flat) });
    let Some(facts) = facts else { return Ok(()); };
    let viewport = MapViewport { x_max: info.extents[0], x_min: info.extents[1],
        y_max: info.extents[2], y_min: info.extents[3],
        texture_width: info.pixels[0], texture_height: info.pixels[1] };
    let rows = facts.snapshot.rows(&facts.locations);
    let row = rows.get(facts.snapshot.selected);
    let category = if facts.snapshot.category == MapCategory::Root {
        MapCategory::Locations
    } else if facts.snapshot.category == MapCategory::Locations {
        row.map_or(MapCategory::Locations, |row| row.category)
    } else { facts.snapshot.category };
    let selected = facts.snapshot.info_location.or_else(|| row.and_then(|row| row.location_key));
    let center = point(m, [0., 0.]);
    // RenderIconOnScreenMap8262e350 uses identity matrix and adds map translation
    // to native texture-pixel offsets. It does not scale by the map quad matrix.
    let mut icon = |name: &str, world: [f32;2], width: f32, heading: Option<f32>| -> RenderResult<()> {
        let offset = viewport.screen_offset(world, MapViewport::DEFAULT_ICON_SCALE);
        let half = width * 0.5;
        if offset[0] < -(info.pixels[0] as f32 * 0.5)
            || offset[0] > info.pixels[0] as f32 * 0.5
            || offset[1] < -(info.pixels[1] as f32 * 0.5)
            || offset[1] > info.pixels[1] as f32 * 0.5 { return Ok(()); }
        let t = assets.texture(name)?;
        let mut vertices = Vec::with_capacity(6);
        let p = [[-half,-half],[half,-half],[half,half],[-half,half]];
        let uv = [[0.,0.],[1.,0.],[1.,1.],[0.,1.]];
        for i in [0,1,2,0,2,3] {
            let mut q = p[i];
            if let Some(heading) = heading {
                q[1] -= width * 0.25;
                let (sin,cos) = (heading + std::f32::consts::PI).sin_cos();
                q = [cos*q[0]+sin*q[1],cos*q[1]-sin*q[0]];
            }
            q[0] += center[0] + offset[0]; q[1] += center[1] + offset[1];
            vertices.push(vertex(IDENTITY, q, uv[i], [1.;4], size, flat));
        }
        out.push(Batch { texture: Some(t.name.clone()), wrapped:false, vertices });
        Ok(())
    };
    // Local skater is constructed before backend location icons.
    if let Some((position,heading)) = facts.player {
        icon(&assets.local_player_icon, position, 32., Some(heading))?;
    }
    // RenderMapIcons82631138: the career's map-visible mSimIcons. An unregistered texture draws
    // the native's "missingtexture", which none of the loaded arenas holds.
    if let Some(career) = facts.snapshot.career.as_deref() {
        for map_icon in &career.icons {
            if assets.texture(&map_icon.texture).is_ok() {
                icon(&map_icon.texture, map_icon.position, map_icon.size, None)?;
            }
        }
        return Ok(());
    }
    for location in &facts.locations {
        if category == MapCategory::Locations || location.category == category {
            icon(&location.icon, location.world_position,
                if selected == Some(location.key) {64.} else {32.}, None)?;
        }
    }
    Ok(())
}

/// Shared original bitmap bearings, advances and atlas coordinates.
fn glyph_quads<'a>(font: &'a assets::Font, atlas: &'a skate2_core::texture::NativeTexture,
    units: &'a [u16], scale: [f32; 2], origin: [f32; 2],
) -> impl Iterator<Item = ([f32; 4], [f32; 4])> + 'a {
    units.iter().filter_map(move |&unit| font.metrics.mapped_glyph(unit))
        .scan(origin[0], move |x, g| {
            let left = g.x_offset.mul_add(scale[0], *x);
            let top = (g.y_offset + font.metrics.ascent).mul_add(scale[1], origin[1]);
            *x += g.advance * scale[0];
            Some((
                [left, top, g.width.mul_add(scale[0], left), g.height.mul_add(scale[1], top)],
                [g.u / atlas.width as f32, g.v / atlas.height as f32,
                 (g.u + g.width) / atlas.width as f32, (g.v + g.height) / atlas.height as f32],
            ))
        })
}

/// Diagnostic FPS uses the trick-name font, kept white and flat in screen space.
/// Geometry changes only with the half-second FPS sample or window dimensions.
fn fps_batch(font: &assets::Font, atlas: &skate2_core::texture::NativeTexture,
    fps: u32, size: (u32, u32),
) -> RenderResult<Batch> {
    let cap = font.metrics.mapped_glyph(u16::from(b'F')).ok_or("FPS font has no F")?;
    let sy = 24. / cap.height;
    let scale = [sy * font.scale[0] / font.scale[1], sy];
    let origin = [10. - cap.x_offset * scale[0], 10. - (cap.y_offset + font.metrics.ascent) * sy];
    let units: Vec<u16> = format!("FPS: {fps}").encode_utf16().collect();
    let mut vertices = Vec::new();
    for (b, uv) in glyph_quads(font, atlas, &units, scale, origin) {
        let points = [[b[0],b[1],uv[0],uv[1]], [b[2],b[1],uv[2],uv[1]],
            [b[2],b[3],uv[2],uv[3]], [b[0],b[3],uv[0],uv[3]]];
        for index in [0,1,2,0,2,3] {
            let [x,y,u,v] = points[index];
            vertices.push([x * 2. / size.0.max(1) as f32 - 1.,
                1. - y * 2. / size.1.max(1) as f32, 0., 1., u, v, 1., 1., 1., 1.]);
        }
    }
    Ok(Batch { texture: Some(atlas.name.clone()), wrapped: false, vertices })
}

/// `SetApt3DFocusRightMatrix` (`0x82608748`) for the `g3dHUDRight` screens:
/// flattened on the 1280x720 stage, then every vertex is projected.
fn project_focus_right(size: (u32, u32), batches: &mut [Batch]) {
    static RIGHT: std::sync::OnceLock<skate2_core::ui::apt_projection::FocusLeftProjection> = std::sync::OnceLock::new();
    let projection = RIGHT.get_or_init(|| skate2_core::ui::apt_projection::FocusLeftProjection::focus_right(11.25, 624.));
    project_stage(size, projection, batches);
}

/// `SetApt3DSpinMatrix` (`0x82608be0`) for the `g3dSpinContainer` screens:
/// flattened on the 1280x720 stage, then every vertex is projected through
/// the spin's current angle (degrees).
fn project_spin(size: (u32, u32), angle: f32, batches: &mut [Batch]) {
    use skate2_core::ui::apt_projection::{DISTANCE_OF_3D, FocusLeftProjection};
    project_stage(size, &FocusLeftProjection::spin(angle, DISTANCE_OF_3D), batches);
}

/// Stage-flattened vertices through a 3D screen's projection.
fn project_stage(size: (u32, u32), projection: &skate2_core::ui::apt_projection::FocusLeftProjection, batches: &mut [Batch]) {
    let aspect = size.0 as f32 / size.1.max(1) as f32;
    for batch in batches {
        for vertex in &mut batch.vertices {
            let p = [(vertex[0] + 1.) * 640., (1. - vertex[1]) * 360.];
            let mut clip = projection.project(p);
            clip[0] *= ((16. / 9.) / aspect).min(1.);
            clip[1] *= (aspect / (16. / 9.)).min(1.);
            vertex[..4].copy_from_slice(&clip);
        }
    }
}

/// One `AptRenderingIntegration` quad: 1280x720 at `scale` about the wall's origin, projected.
/// The quad's corners rebased on the camera as `world_clip` rebases the world's vertices, so the
/// photo's depth against its wall 2.6 cm behind resolves as the world's own does.
#[expect(clippy::cast_possible_truncation, reason = "the f64 rebase narrows to the f32 GPU lanes")]
fn world_quad(clip: &crate::gpu_renderer::CameraRelativeClip, wall: &[[f32; 4]; 4], scale: f32, colour: [f32; 4]) -> Vec<[f32; 10]> {
    [(-640., 360., 0., 0.), (640., 360., 1., 0.), (-640., -360., 0., 1.),
        (-640., -360., 0., 1.), (640., 360., 1., 0.), (640., -360., 1., 1.)]
        .map(|(x, y, u, v)| {
            let p: [f32; 4] = std::array::from_fn(|i| {
                let origin = if i < 3 { f64::from(wall[3][i]) - f64::from(clip.camera[i]) } else { f64::from(wall[3][i]) };
                (f64::from(wall[0][i] * x * scale) + f64::from(wall[1][i] * y * scale) + origin) as f32
            });
            let q: [f32; 4] = std::array::from_fn(|i| (0..4).map(|j| clip.clip_from_camera[j * 4 + i] * p[j]).sum());
            [q[0], q[1], q[2], q[3], u, v, colour[0], colour[1], colour[2], colour[3]]
        })
        .to_vec()
}

#[cfg(test)]
mod tests;
