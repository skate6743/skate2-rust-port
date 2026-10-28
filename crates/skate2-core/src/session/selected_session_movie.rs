//! The session's movie player: the front end manager's `FEMoviePlayer` with FE state 28
//! (`ui::movie_player`), pushed by the career's message type 15, and the new career's intro
//! states `StateMainMenu::NewCareerIntro` (0x82763790) and `RedaIntro` (0x8275f5f8), which
//! `InFEState` (0x82777dec) enters once `EnterGame`'s flow message 28 set
//! `mDoEnterCACFromMainMenu`, before `NewCAC`.
use super::{SelectedLiveError, SelectedLiveSession, SelectedLiveStage};
use crate::{
    challenge::attrib_db::AttribDb,
    nis,
    ui::{
        movie_player::{
            self, MovieContext, MovieEvent, MoviePlayer, MoviePresentation, MovieRequest,
            MovieSubtitle,
        },
        pause_menu::PauseMenuInput,
    },
};
use std::sync::Arc;

fn error(e: impl std::fmt::Display) -> SelectedLiveError {
    SelectedLiveError::failed(
        SelectedLiveStage::PreupdateInput,
        format!("movie player: {e}"),
    )
}

/// `StateMainMenu`'s intro states, each with its `mbMoviePlaying`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Intro {
    NewCareerIntro(bool),
    RedaIntro(bool),
}

#[derive(Default)]
pub(super) struct MovieSession {
    player: MoviePlayer,
    intro: Option<Intro>,
    /// `data\fe\subtitles\subtitles.big`, read for the first subtitled movie.
    subtitle_big: Option<Vec<u8>>,
    /// The SIM input's front-end pause the state holds.
    input_paused: bool,
}

impl<T, C> SelectedLiveSession<T, C> {
    /// `NewCareerIntro`'s `Event_Enter`: `god_intro1` under `ForceBlackOnClose`, then
    /// `SetState(28, -1)`.
    pub(super) fn begin_new_career_intro(&mut self) {
        self.movie.intro = Some(Intro::NewCareerIntro(false));
        self.movie
            .player
            .set_state_black_on_close(movie_player::GOD_INTRO1);
    }

    /// The new career's intro movies are running.
    #[must_use]
    pub(super) fn new_career_intro_running(&self) -> bool {
        self.movie.intro.is_some()
    }

    /// One front-end update of the movie player, after the career's pushes and the intro states'
    /// updates. True while FE state 28 holds the front end (its `gFEStateInfo` row pauses the game)
    /// or an intro state runs: the rest of the frame waits.
    pub(super) fn update_movie(
        &mut self,
        input: PauseMenuInput,
        has_focus: bool,
    ) -> Result<bool, SelectedLiveError> {
        if let Some(career) = self.career.as_mut() {
            for key in career.take_movie_pushes() {
                self.movie.player.push_state(key);
            }
        }
        let active = self.movie.player.holds_front_end() || self.movie.intro.is_some();
        if active && !self.movie.player.has_movies() {
            let db = self.movie_attrib_db()?;
            self.movie.player.load_movies(&db);
            self.movie.player.boot_flow_played();
        }
        let subtitles = self.audio.settings.subtitles;
        if active && subtitles && self.movie.subtitle_big.is_none() {
            let assets = self
                .inner
                .assets
                .as_ref()
                .ok_or_else(|| error("assets absent"))?;
            self.movie.subtitle_big = Some(
                assets
                    .mounts
                    .read_path(nis::subtitles::BIG_PATH)
                    .map_err(error)?,
            );
        }
        // Sk8AppMan::PreUpdate's Flow::Manager::Update (0x82767098) runs the intro states before
        // GameModule::Update's FrontEndManager::Update (0x82640130).
        self.update_new_career_intro()?;
        let context = MovieContext {
            subtitles,
            subtitle_big: self.movie.subtitle_big.as_deref(),
            has_focus,
        };
        let was_in_stack = self.movie.player.state_in_stack();
        self.movie.player.update(input, &context);
        if was_in_stack
            && !self.movie.player.state_in_stack()
            && let Some(career) = self.career.as_mut()
        {
            career.movie_state_left();
        }
        if !self.movie.player.holds_front_end()
            && let Some(extras) = self.main_menu.as_mut().and_then(|menu| menu.extras.as_mut())
            && matches!(extras.state.page, crate::ui::extras::Page::Movie | crate::ui::extras::Page::Credits)
        {
            extras.state.page = crate::ui::extras::Page::Menu;
            extras.state.selected = 0;
            extras.intro_complete = false;
        }
        for event in self.movie.player.take_events() {
            match event {
                MovieEvent::Finished => {
                    if let Some(career) = self.career.as_mut() {
                        career.fmv_finished_playing();
                    }
                }
                MovieEvent::Background {
                    enable: true,
                    snap: true,
                } => self.frontend_fade.cover_snap(),
                MovieEvent::Background {
                    enable: true,
                    snap: false,
                } => self.frontend_fade.cover(),
                MovieEvent::Background { enable: false, .. } => self.frontend_fade.reveal(),
            }
        }
        // StateManagerFrontEndBase::UpdateOnEnter/UpdateOnLeave push and pull the FRONTEND
        // pause of the state's gFEStateInfo row; any other front end holding it keeps it.
        let open = self.movie.player.holds_front_end();
        if open != self.movie.input_paused {
            self.movie.input_paused = open;
            let others = self
                .career
                .as_ref()
                .is_some_and(|career| career.overlay_open())
                || self.pause_menu.snapshot().visible || self.main_menu.is_some();
            if let Some(input) = self.inner.bound_input.as_mut() {
                input
                    .try_set_frontend_pause(open || others)
                    .map_err(|e| error(format!("{e:?}")))?;
            }
        }
        Ok(open || self.movie.intro.is_some())
    }

    /// The intro states' `Event_Update`: `NewCareerIntro` notes the movie playing, then moves to
    /// `RedaIntro` once FE state 28 left the stack; `RedaIntro` notes it only while the state is
    /// on the stack, then moves to `NewCAC` (Edit Skater in its new-career substate, opened still
    /// covered once the player is registered; its `Shutdown` enables progression).
    fn update_new_career_intro(&mut self) -> Result<(), SelectedLiveError> {
        let in_stack = self.movie.player.state_in_stack();
        let playing = self.movie.player.is_playing();
        match self.movie.intro {
            Some(Intro::NewCareerIntro(true)) if !in_stack => {
                self.movie.intro = Some(Intro::RedaIntro(false));
                self.movie
                    .player
                    .set_state_black_on_close(movie_player::REDA_CAR_RIDE);
            }
            Some(Intro::NewCareerIntro(false)) if playing => {
                self.movie.intro = Some(Intro::NewCareerIntro(true))
            }
            Some(Intro::RedaIntro(true)) if !in_stack => {
                self.movie.intro = None;
                self.enter_single_player()?;
                self.career_after_editor = true;
            }
            Some(Intro::RedaIntro(false)) if playing && in_stack => {
                self.movie.intro = Some(Intro::RedaIntro(true))
            }
            _ => {}
        }
        Ok(())
    }

    /// The Attrib database the `fe_movie` collections come from: the career's, the front end's,
    /// or the career vaults read now.
    pub(super) fn movie_attrib_db(&self) -> Result<Arc<AttribDb>, SelectedLiveError> {
        if let Some(career) = self.career.as_ref() {
            return Ok(career.attrib_db());
        }
        if let Some((db, _)) = self.freeskate.data.as_ref() {
            return Ok(Arc::clone(db));
        }
        let assets = self
            .inner
            .assets
            .as_ref()
            .ok_or_else(|| error("assets absent"))?;
        Ok(Arc::new(
            AttribDb::load_career(&assets.mounts).map_err(error)?,
        ))
    }

    /// `AptMovieIntegration`'s requests and the audio system's movie calls since the last call,
    /// for the presentation.
    pub fn take_movie_requests(&mut self) -> Vec<MovieRequest> {
        self.movie.player.take_requests()
    }

    /// The presentation's `AptMovieIntegration` after its render.
    pub fn set_movie_presentation(&mut self, presentation: MoviePresentation) {
        self.movie.player.set_presentation(presentation);
        for event in self.movie.player.take_events() {
            if let MovieEvent::Background {
                enable: true,
                snap: true,
            } = event
            {
                self.frontend_fade.cover_snap();
            }
        }
    }

    /// `mbForeground`: the movie draws after the front end's layers.
    #[must_use]
    pub fn movie_background_forced(&self) -> bool { self.movie.player.force_movie_background() }
    pub fn movie_foreground(&self) -> bool {
        self.movie.player.foreground()
    }

    /// The front end's subtitle line as the movie player sets it.
    #[must_use]
    pub fn movie_subtitle(&self) -> MovieSubtitle {
        self.movie.player.subtitle()
    }

    /// FE state 28 holds the front end.
    #[must_use]
    pub fn movie_state_open(&self) -> bool {
        self.movie.player.holds_front_end()
    }

    pub fn movie_screen(&self) -> Option<crate::ui::movie_player::MovieScreen> {
        self.movie.player.screen()
    }

    pub fn acknowledge_movie_screen_loaded(&mut self, generation: u64) {
        self.movie.player.acknowledge_screen_loaded(generation);
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    pub(super) fn start_extras_movie(&mut self, key: u64, credits: bool) {
        if credits { self.movie.player.push_credits(); }
        else { self.movie.player.push_state(key); }
    }
    pub(super) fn acknowledge_credits(&mut self, intro: bool, done: bool) {
        self.movie.player.acknowledge_credits(intro, done);
    }
}
