//! Edit Skater frontend lifecycle on the retained local session.
use super::{SelectedLiveError, SelectedLiveSession, SelectedLiveStage};
use crate::{
    edit_skater_character::EditSkaterCharacter,
    ui::{
        edit_skater::{EditSkaterInput, EditSkaterView},
        pause_menu::PauseMenuInput,
    },
};
use std::sync::Arc;
fn error(e: impl std::fmt::Display) -> SelectedLiveError {
    SelectedLiveError::failed(SelectedLiveStage::Visual, e.to_string())
}
impl<T, C> SelectedLiveSession<T, C> {
    pub fn options_snapshot(&self) -> crate::ui::options::Snapshot { self.options_menu.snapshot() }
    /// The pad slot of the controller that plays the game (`PlayerUI::mControllerIndex`), whose
    /// sticks the Stick Settings pages show.
    #[must_use]
    pub fn controller_slot(&self) -> Option<usize> {
        self.inner.bound_input.as_ref().map(|input| input.controller())
    }
    /// Alt+Enter: Display Settings' Fullscreen flips and its chunk saves at once.
    ///
    /// # Errors
    /// The profile cannot be opened or written.
    pub fn toggle_fullscreen(&mut self) -> Result<(), SelectedLiveError> {
        self.options_menu.toggle_fullscreen();
        let path = crate::edit_skater_profile::EditSkaterProfileStore::default_path().map_err(error)?;
        crate::edit_skater_profile::EditSkaterProfileStore::open(path)
            .and_then(|mut store| store.save_display_settings(self.options_menu.snapshot().values))
            .map_err(error)?;
        self.options_menu.display_saved();
        Ok(())
    }
    pub fn start_main_menu(&mut self,mounts:&skate2_formats::mount::MountSet)->Result<(),SelectedLiveError>{
        self.main_menu_assets=Some(crate::ui::main_menu::Presentation::load(mounts).map_err(error)?);
        let mut menu=crate::ui::main_menu::Controller::new(true);
        menu.new_profile=Self::profile_is_new()?;
        let path=crate::edit_skater_profile::EditSkaterProfileStore::default_path().map_err(error)?;
        if !path.try_exists().map_err(error)? {
            menu.name_prompt=Some(Default::default());
        }
        self.main_menu=Some(menu);
        Ok(())
    }
    /// PC: the release's changes over the main menu once it stands idle, at a player's first
    /// launch of the release.
    pub fn show_changelog(&mut self, changelog: crate::ui::update_prompt::Changelog) {
        if let Some(menu) = self.main_menu.as_mut() {
            menu.changelog = Some(changelog);
        }
    }
    /// `ProfileBackEnd.mNewProfile` from the PC SKATER.P (see EditSkaterProfileStore).
    pub(super) fn profile_is_new()->Result<bool,SelectedLiveError>{
        let path=crate::edit_skater_profile::EditSkaterProfileStore::default_path().map_err(error)?;
        crate::edit_skater_profile::EditSkaterProfileStore::open(path).and_then(|s|s.is_new_profile()).map_err(error)
    }
    pub fn main_menu_frame(&self)->Result<Option<crate::ui::main_menu::Frame>,SelectedLiveError>{
        self.main_menu_render_frame(0.0)
    }
    /// Sample the same authored curve at the render fraction; no SIM state advance.
    pub fn main_menu_render_frame(&self,fraction:f32)->Result<Option<crate::ui::main_menu::Frame>,SelectedLiveError>{
        if self.replay_editor.is_some() {return Ok(None);}
        let Some(menu)=&self.main_menu else{return Ok(None);};
        let Some(assets)=&self.inner.assets else{return Ok(None);};
        let near=assets.cvars.try_camera_near_clip_distance().map_err(error)?;
        let mut frame=self.main_menu_assets.as_ref().ok_or_else(||error("main menu assets absent"))?
            .frame(menu,near,fraction).map_err(error)?;
        if frame.skate_reel.is_some() {
            frame.online_match=self.online_match_snapshot();
            frame.lobby_browser=self.lobby_browser_snapshot();
            frame.online_nickname=self.online_nickname();
        }
        Ok(Some(frame))
    }
    pub fn main_menu_focus(&self)->Result<Option<[f32;3]>,SelectedLiveError>{
        self.main_menu_assets.as_ref().map(|a|a.focus().map_err(error)).transpose()
    }
    pub fn acknowledge_main_menu(&mut self,intro_complete:bool){
        if let Some(menu)=self.main_menu.as_mut(){menu.ready=true;menu.intro_complete=intro_complete;}
    }
    pub fn frontend_fade_alpha(&self) -> f32 { self.frontend_fade.alpha() }
    pub fn frontend_scene_reveal_pending(&self) -> bool { self.frontend_reveal_pending }
    pub fn acknowledge_frontend_frame_presented(&mut self, alpha: f32) {
        self.frontend_fade.presented(alpha);
        if let Some(career) = self.career.as_mut() {
            career.map_challenge_frame_presented();
        }
    }
    /// The new room or completed gameplay pose has been prepared for drawing.
    pub fn acknowledge_frontend_scene_ready(&mut self) {
        if self.frontend_reveal_pending {
            self.frontend_reveal_pending = false;
            self.frontend_fade.reveal();
        }
    }
    pub fn edit_skater_view(&self) -> Result<Option<EditSkaterView>, SelectedLiveError> {
        self.editor
            .as_ref()
            .map(|e| e.frontend.view().map_err(error))
            .transpose()
    }
    pub fn edit_skater_character(&mut self) -> Option<(&mut EditSkaterCharacter, bool, u64, u64)> {
        self.editor.as_mut().map(|e| {
            let male = e.frontend.preview_record().data.is_male;
            let body = e.frontend.body_key();
            let camera = e.frontend.camera_key();
            (&mut e.character, male, body, camera)
        })
    }
    /// Original APT intro/outro completion messages gate frontend input/exit.
    pub fn acknowledge_edit_skater_movie(&mut self, intro: bool, outro: bool) {
        if let Some(editor) = self.editor.as_mut() {
            editor.frontend.controller.busy = !intro;
            self.editor_outro_complete = editor.finishing && outro;
        }
    }
    pub(super) fn complete_character_registration(&mut self) -> Result<(), SelectedLiveError> {
        let player = self
            .inner
            .player
            .as_mut()
            .ok_or_else(|| error("CAC player absent"))?;
        let owners = &mut player.selected.owners;
        if !matches!(
            owners.loader.phase(),
            crate::player::PlayerLoadPhase::Registering { .. }
        ) {
            return Ok(());
        }
        let runtime = self
            .inner
            .presentation_runtime
            .as_ref()
            .ok_or_else(|| error("CAC presentation absent"))?;
        let Some(peer) = runtime.published_peer() else {
            return Ok(());
        };
        let scene = Arc::clone(runtime.palette().map_err(error)?.scene());
        owners
            .recipe_load
            .as_mut()
            .ok_or_else(|| error("CAC recipe load absent"))?
            .complete_presentation_registration(&mut owners.loader, &scene, peer)
            .map_err(error)?;
        if matches!(owners.loader.phase(),crate::player::PlayerLoadPhase::Loaded{..}){
            let regular=owners.services.settings.try_stance_byte(owners.loader.player_index()).map_err(error)?!=0;
            if let Some(marker)=self.inner.session_marker.as_mut(){marker.teleport.reload_stance(regular);}
        }
        Ok(())
    }
    pub(super) fn open_edit_skater(&mut self) -> Result<(), SelectedLiveError> {
        self.complete_character_registration()?;
        let player = self
            .inner
            .player
            .as_ref()
            .ok_or_else(|| error("CAC player absent"))?;
        if !matches!(
            player.selected.owners.loader.phase(),
            crate::player::PlayerLoadPhase::Loaded { .. }
        ) {
            return Err(error("Edit Skater requires completed player registration"));
        }
        // The receipt's GetCash and LockedFilter's unlockables: the career's, online the
        // profile's (PC port addition), else (the new career's skater, made before its career
        // starts) a fresh SignPost's starting cash and the default unlocks.
        let funds = match self.career_shop_funds() {
            Some(funds) => Some(funds),
            None if self.online.edit_skater => Some(self.online_shop_funds()?),
            None => None,
        };
        let assets = self
            .inner
            .assets
            .as_ref()
            .ok_or_else(|| error("CAC assets absent"))?;
        let (cash, unlockables) = match funds {
            Some(funds) => funds,
            None => {
                use crate::challenge::attrib_db::AttribDb;
                let db = AttribDb::load_career(&assets.mounts).map_err(error)?;
                (crate::challenge::signpost_progress::starting_cash(&db), crate::challenge::unlockables::Unlockables::new(&db))
            }
        };
        let unlocked = |category, name: &str| {
            unlockables.is_unlocked(category, crate::challenge::unlockables::string_hash(name), true)
        };
        let new_unlock = |category, name: &str| {
            unlockables.is_new_unlock(category, crate::challenge::unlockables::string_hash(name))
        };
        let mut editor = crate::edit_skater_session::EditSkaterSession::open(
            &assets.mounts,
            Arc::clone(&assets.vault),
            assets.cvars.clone(),
            cash,
            &unlocked,
            &new_unlock,
            self.career_after_editor,
        )
        .map_err(error)?;
        editor.frontend.controller.busy = true;
        self.editor = Some(editor);
        self.editor_outro_complete = false;
        Ok(())
    }
    pub(super) fn update_edit_skater(
        &mut self,
        input: PauseMenuInput,
        focus: bool,
        dt: f32,
    ) -> Result<(), SelectedLiveError> {
        if self.editor_outro_complete {
            if self.frontend_fade.covered() { self.finish_edit_skater()?; }
            return Ok(());
        }
        let editor = self
            .editor
            .as_mut()
            .ok_or_else(|| error("CAC owner absent"))?;
        let assets = self
            .inner
            .assets
            .as_ref()
            .ok_or_else(|| error("CAC assets absent"))?;
        if focus {
            let was_finishing = editor.finishing;
            editor
                .update(
                    &assets.mounts,
                    EditSkaterInput {
                        start: input.start,
                        back: input.cancel,
                        select: input.select,
                        misc: input.misc,
                        up: input.up,
                        down: input.down,
                        left: input.left,
                        right: input.right,
                        morph_axis: input.morph_axis,
                        morph_axis_secondary: input.morph_axis_secondary,
                    },
                    dt,
                )
                .map_err(error)?;
            if editor.finishing && !was_finishing {
                // Shutdown82659de8 enters STATE_FADE_CAS_EDITOR and hides
                // the screen. The fade does not wait for the APT outro.
                self.frontend_fade.cover();
            }
        }
        Ok(())
    }
    fn finish_edit_skater(&mut self) -> Result<(), SelectedLiveError> {
        let editor = self
            .editor
            .as_mut()
            .ok_or_else(|| error("CAC owner absent"))?;
        let record = editor.gameplay_record().map_err(error)?;
        let gender_changed = record.data.is_male != editor.originally_male;
        let player = self
            .inner
            .player
            .as_mut()
            .ok_or_else(|| error("CAC player absent"))?;
        let owners = &mut player.selected.owners;
        // LeavingCACConfirmationCallback 0x82689a68: UpdateStatsAndAchievements 0x82680b20 counts
        // a changed gender in statistic 2 (forced) and grants Gender Bender.
        if gender_changed {
            let backend = owners.services.skaters.backend();
            backend.update_additive_statistic(2, 1.0, true, &owners.cvars).map_err(error)?;
            backend.grant_achievement(40, 0).map_err(error)?;
        }
        let action = owners
            .recipe_load
            .as_mut()
            .ok_or_else(|| error("CAC recipe load absent"))?
            .try_replace_cac(
                &mut owners.loader,
                &owners.services,
                &mut owners.faces,
                &owners.mounts,
                &owners.cvars,
                owners
                    .recipe_registry
                    .as_mut()
                    .ok_or_else(|| error("CAC registry absent"))?,
                &record,
            )
            .map_err(error)?;
        let crate::player::PlayerLoadAction::Register(registration) = action else {
            return Err(error(
                "CAC replacement did not request presentation registration",
            ));
        };
        player.registration_request = registration;
        owners.try_restore_replaced_recipe().map_err(error)?;
        let scene = Arc::clone(
            owners
                .recipe_load
                .as_ref()
                .ok_or_else(|| error("CAC loaded recipe absent"))?
                .scene()
                .map_err(error)?,
        );
        self.inner
            .presentation_runtime
            .as_mut()
            .ok_or_else(|| error("CAC runtime absent"))?
            .replace_recipe(
                owners
                    .presentation
                    .as_ref()
                    .ok_or_else(|| error("CAC presentation absent"))?,
                scene,
            )
            .map_err(error)?;
        // Resolve material/texture admission before committing the disk profile.
        // The regular frame observes this same bound owner on resume.
        let finalized = owners
            .recipe_load
            .as_ref()
            .ok_or_else(|| error("CAC recipe missing"))?
            .finalized()
            .map_err(error)?;
        self.inner
            .presentation_runtime
            .as_mut()
            .ok_or_else(|| error("CAC runtime absent"))?
            .try_bind_materials(
                finalized,
                Arc::clone(&owners.services.vault),
                owners.cvars.clone(),
            )
            .map_err(error)?;
        // Commit the native profile only after its return-to-game assets are ready.
        editor.store.save_record(&record).map_err(error)?;
        let purchases = editor.frontend.shop.purchases().to_vec();
        self.editor_sounds.extend(editor.frontend.leave());
        self.editor = None;
        self.editor_outro_complete = false;
        self.pause_menu.close();
        self.inner
            .bound_input
            .as_ref()
            .ok_or_else(|| error("CAC input absent"))?
            .try_set_frontend_pause(false)
            .map_err(error)?;
        if std::mem::take(&mut self.career_after_editor) {
            self.start_career(false)?;
        }
        // A new career's start travels under the fade and reveals once its skater has landed.
        self.frontend_reveal_pending = self.map_travel.is_none();
        // PC port: the online game's editor sold against the profile's career, and its look goes
        // to the other players once the local skater has it.
        if std::mem::take(&mut self.online.edit_skater) {
            self.online_close_editor_shop(&purchases)?;
            self.online_character_changed();
        } else {
            self.close_editor_shop(&purchases)?;
        }
        Ok(())
    }

    /// PC port: the online game closes the editor for the player (a round loading with it, a
    /// change of location, a travel, the game ending), without the outro or the fade. The
    /// receipt's lines leave the skater as Edit Cart's would with none kept, so it keeps only
    /// what the player owns or bought, and the rest saves as Finish's would.
    pub(super) fn close_edit_skater_for_player(&mut self) -> Result<(), SelectedLiveError> {
        self.editor
            .as_mut()
            .ok_or_else(|| error("CAC owner absent"))?
            .frontend
            .drop_receipt()
            .map_err(error)?;
        self.finish_edit_skater()
    }
}
