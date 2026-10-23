//! Profile free skate: `FrontEndState_FreeSkateOption` over the main menu, then
//! `FrontEndManager::StartGameModule` in `GM_ProfileFreeSkate` at the chosen location.
use super::{SelectedLiveError, SelectedLiveSession, SelectedLiveStage, SelectedSceneCharacterVisual, freeskate_viewer::Viewer};
use crate::{
    challenge::{
        attrib_db::{AttribDb, CAREER_VAULTS},
        unlockables::{self, Unlockables},
    },
    edit_skater_profile::EditSkaterProfileStore,
    player::{MarqueeDefinition, PlayerLoadAction, PlayerLoadPhase, PlayerSelection},
    ui::freeskate_option::{self, GameSettings, Settings},
};
use skate2_formats::location::LocationTable;
use std::sync::Arc;

/// `GameInfo.gameMode`'s `GM_SinglePlayer`, `GM_PartyPlay` and `GM_ProfileFreeSkate`.
const GM_SINGLE_PLAYER: u32 = 1;
const GM_PARTY_PLAY: u32 = 2;
const GM_PROFILE_FREE_SKATE: u32 = 3;
const SIM_LOCATIONS: &str = "data\\content\\world\\stream\\BAM\\BAM_Sim.loc";

fn error(e: impl std::fmt::Display) -> SelectedLiveError {
    SelectedLiveError::failed(SelectedLiveStage::Visual, e.to_string())
}

/// The free-skate front end's data and choices.
#[derive(Default)]
pub(super) struct FreeSkate {
    /// `CommonBackEnd::sGameInfo`'s traffic, pedestrian and security settings, which the
    /// screen reads on entry and writes on EnterGame.
    pub(super) settings: Settings,
    /// EnterGame, waiting for the fade.
    pub(super) start: Option<GameSettings>,
    /// The skater the front end loads (a marquee, or the created skater), waiting for the
    /// loader to finish its current request as `PlayerLoaderImpl::Load` queues behind it.
    pending_character: Option<Option<u64>>,
    /// The host's retained first Actor needs the new game's constructor
    /// attributes once the selected character and spawn area are ready.
    pub(super) spawn_attributes_pending: bool,
    /// `skater.vlt` and `BAM_Sim.loc`, loaded on the first visit (FreeSkate's or PartyPlay's).
    pub(super) data: Option<(Arc<AttribDb>, LocationTable)>,
    /// The screen's `SimCharacterViewer`, loaded on the first visit.
    viewer: Option<Viewer>,
    /// The free-skate game's challenge system (`GenerateGameModeSignature` 2), of which the port
    /// runs the dynamic hull manager only.
    pub(super) hulls: Option<crate::challenge::ChallengeSystem>,
}

impl<T, C> SelectedLiveSession<T, C> {
    /// `OnMenuSelectGame`82681338 row 2: the screen `UpdateFEState` pushes once the NIS ends, with
    /// `FrontEndState_FreeSkateOption::OnEnter`8265b098's character and location lists.
    pub(super) fn open_freeskate_option(&mut self) -> Result<(), SelectedLiveError> {
        let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("free skate assets absent"))?.mounts);
        if self.freeskate.data.is_none() {
            let db = Arc::new(AttribDb::load(&mounts, &CAREER_VAULTS).map_err(error)?);
            let table = LocationTable::parse(&mounts.read_path(SIM_LOCATIONS).map_err(error)?).map_err(error)?;
            self.freeskate.data = Some((db, table));
        }
        let (db, _) = self.freeskate.data.as_ref().expect("free skate data loaded");
        if self.freeskate.viewer.is_none() {
            self.freeskate.viewer = Some(Viewer::load(db, &mounts, super::freeskate_viewer::FREESKATE).map_err(error)?);
        }
        // UnlockablesBackEnd::LoadData 0x8255cd08 over the defaults Init added.
        let store = EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(error)?).map_err(error)?;
        let mut unlocks = Unlockables::new(db);
        unlocks.load(super::career::leaf_tuples(&store.load_leaf(unlockables::SAVABLE, unlockables::DATA).map_err(error)?));
        let name = self
            .inner
            .player
            .as_ref()
            .and_then(|player| player.selected.owners.services.profile.name())
            .unwrap_or_default();
        let new_profile = Self::profile_is_new()?;
        let characters = freeskate_option::characters(db, &unlocks, (!new_profile).then_some(name.as_str()));
        let locations = freeskate_option::locations(db, &unlocks, &mounts);
        // OnEnter asserts a location.
        if locations.is_empty() {
            return Err(error("free skate has no unlocked valid location"));
        }
        let menu = self.main_menu.as_mut().ok_or_else(|| error("main menu absent"))?;
        menu.freeskate = Some(freeskate_option::Controller::new(characters, locations, self.freeskate.settings));
        Ok(())
    }

    /// The FreeSkate screen's intro finished (`FrontEndStateBase::OnIntroDone`).
    pub fn acknowledge_freeskate_intro(&mut self) {
        if let Some(freeskate) = self.main_menu.as_mut().and_then(|menu| menu.freeskate.as_mut()) {
            freeskate.intro_done();
        }
    }

    /// The FreeSkate screen's `SetScreenState` calls since the last take.
    pub fn take_freeskate_screen_states(&mut self) -> Vec<u32> {
        self.main_menu.as_mut().map(crate::ui::main_menu::Controller::take_freeskate_screen_states).unwrap_or_default()
    }

    /// The viewer follows the open screen's last `UpdateCharacter`, and shuts down with the
    /// screen (`OnLeave` 0x82653240).
    pub(super) fn update_freeskate_viewer(&mut self, dt: f32) -> Result<(), SelectedLiveError> {
        let requested = self
            .main_menu
            .as_ref()
            .and_then(|menu| menu.freeskate.as_ref())
            .and_then(freeskate_option::Controller::viewer_character);
        let Some(viewer) = self.freeskate.viewer.as_mut() else { return Ok(()) };
        let (db, _) = self.freeskate.data.as_ref().ok_or_else(|| error("free skate data absent"))?;
        let assets = self.inner.assets.as_ref().ok_or_else(|| error("free skate assets absent"))?;
        viewer.update(requested, dt, db, &assets.mounts, &assets.vault, &assets.cvars).map_err(error)
    }

    /// The viewer's posed skater, for the front end's scene.
    #[must_use]
    pub fn freeskate_viewer_visuals(&self) -> Vec<SelectedSceneCharacterVisual<'_>> {
        self.freeskate.viewer.as_ref().map(Viewer::visuals).unwrap_or_default()
    }

    /// `EnterGame`8265b230 after `FadeComplete`: `UpdateGameSettings`8264b268 stores the densities,
    /// security and start location and loads the chosen skater; `StartGameModule`82633da0 then
    /// enables progression only for `GM_SinglePlayer`, so no career runs. The skater starts at
    /// the location once its streams are in, as the map's travel does.
    pub(super) fn start_freeskate(&mut self, game: GameSettings) -> Result<(), SelectedLiveError> {
        // EnterGame shuts the viewer down before UpdateGameSettings' load is awaited.
        if let Some(viewer) = self.freeskate.viewer.as_mut() {
            viewer.shutdown();
        }
        self.freeskate.settings = game.settings;
        self.set_game_mode(GM_PROFILE_FREE_SKATE, Some(game.settings))?;
        self.start_single_player_players()?;
        self.drop_career()?;
        self.freeroam_replay.reset();
        self.freeroam_replay_frame=0;
        self.replay_visual_focuses=None;
        self.replay_frames.clear();
        self.replay_clock=None;
        self.replay_restorer=crate::presentation_restore::PresentationRestorer::new();
        self.freeskate.hulls = Some(self.free_skate_dynamic_hulls()?);
        self.pause_menu.set_hub(crate::ui::pause_menu::StartHub::OfflineFreeSkate);
        // The load applies a marquee's character, stance and hat (`ApplyPlayerAttributes`
        // 0x828158f0) before StartGameModule, so the spawn reads them even while this loader
        // still finishes its current request.
        if let Some(marquee) = game.character.marquee {
            let owners = &self.inner.player.as_ref().ok_or_else(|| error("free skate player absent"))?.selected.owners;
            owners.services.apply_marquee(marquee, owners.loader.player_index()).map_err(error)?;
        }
        self.freeskate.pending_character = Some(game.character.marquee);
        self.apply_pending_character()?;
        let (_, table) = self.freeskate.data.as_ref().ok_or_else(|| error("free skate data absent"))?;
        let start = table
            .find(&game.location.spawn)
            .ok_or_else(|| error(format!("free skate start {} absent", game.location.spawn)))?;
        self.map_travel = Some(start.transform);
        self.map_travel_presentation_ready = false;
        self.map_travel_dispatched = false;
        self.map_travel_on_board = true;
        Ok(())
    }

    /// `FrontEndState_PartyPlay::EnterGame` 0x8264ad08 after `FadeComplete` 0x8265ae38:
    /// `GM_PartyPlay` and `SetLocationName` of the challenge's `Location`, where the first player's
    /// skater starts. The other party members, the turns and the challenge are not ported yet.
    pub(super) fn start_party_play(&mut self, game: crate::ui::party_play::PartyGame) -> Result<(), SelectedLiveError> {
        /// `challenge_global_data.Location` (`tLocationID`).
        const CHALLENGE_GLOBAL_DATA: u64 = 0xA4F3_5B15_9D74_32B5;
        const LOCATION: u64 = 0xF1C6_D5FA_4A23_951E;
        let (db, table) = self.freeskate.data.as_ref().ok_or_else(|| error("party play data absent"))?;
        let name = db
            .attribute(CHALLENGE_GLOBAL_DATA, game.location.key, LOCATION)
            .and_then(|location| location.text(0))
            .ok_or_else(|| error("party play challenge location absent"))?;
        let start = table.find(name).ok_or_else(|| error(format!("party play start {name} absent")))?.transform;
        self.set_game_mode(GM_PARTY_PLAY, None)?;
        self.create_party_players(&game)?;
        self.drop_career()?;
        self.pause_menu.set_hub(crate::ui::pause_menu::StartHub::PartyPlay);
        self.freeskate.pending_character = Some(game.players.first().and_then(|player| player.character.marquee));
        self.apply_pending_character()?;
        self.party_play.game = Some(game.clone());
        self.party_play.spawn = Some((game, start));
        self.map_travel = Some(start);
        self.map_travel_presentation_ready = false;
        self.map_travel_dispatched = false;
        self.map_travel_on_board = true;
        Ok(())
    }

    /// `StateGameUnload::UnloadGameState` 0x82757cc8 entering as the quit reaches the main menu.
    /// `ResetGame` 0x8273a680 sets an offline game's local index to 0 and gives player 0 a fresh
    /// `PlayerUI`; its `SimController::Reset` ends the career's challenges
    /// (`Sk8ChallengeSystem::GameUnload`) and destroys the party skaters, whose characters
    /// `UnloadPartyMember` then unloads. `PopAllHUDStates` and `HUDManager2::ExitToFE`
    /// 0x82cee828 unload every HUD component ([`Self::take_game_unloaded`]), and
    /// `MusicPlayerManager::RevertMusicMode` puts the music system back on the profile's mode.
    pub(super) fn game_unload(&mut self) -> Result<(), SelectedLiveError> {
        self.online_leave_game();
        self.unload_game()
    }

    /// The game's unload without leaving an online game: the single player game a game joined
    /// from the hub replaces.
    pub(super) fn unload_game(&mut self) -> Result<(), SelectedLiveError> {
        use crate::session_marker::teleport::PlayerUiTeleportReceiver as _;
        self.unload_players()?;
        let owner = self.inner.world.as_ref().ok_or_else(|| error("world absent"))?;
        let local = owner
            .world
            .lock()
            .map_err(|_| error("world lock poisoned"))?
            .local_skater_identity
            .clone()
            .ok_or_else(|| error("local skater identity absent"))?;
        local.try_set_local_index(0).map_err(|e| error(format!("{e:?}")))?;
        // SkaterManager::Reset destroys the first player's Actor too, and its next one starts
        // unpinned; the port keeps that Actor, so a pin from another player's turn goes.
        if let Some(actor) = self.inner.player.as_ref().and_then(|player| player.selected.actor.as_ref()) {
            actor.try_set_pinned(false).map_err(|e| error(format!("{e:?}")))?;
        }
        if let (Some(input), Some(marker)) = (self.inner.bound_input.as_mut(), self.inner.session_marker.as_mut()) {
            let foot_forward = input.set_player(0).map_err(|e| error(format!("{e:?}")))?;
            marker.teleport.reload_stance(foot_forward != 0);
        }
        self.unload_career_living_world()?;
        self.unload_career_dynamic_hulls()?;
        self.unload_career_activities()?;
        self.drop_career()?;
        self.freeskate.hulls = None;
        self.freeskate.pending_character = None;
        self.freeskate.spawn_attributes_pending = false;
        self.inner.try_destroy_party_skaters().map_err(error)?;
        self.inner.try_destroy_ai_skaters().map_err(error)?;
        // ResetGame8273d994 -> Simulation::Reset8289effc also resets the
        // living world. Challenge unload alone leaves ambient pedestrians
        // with command streams from a roster that the unload has changed.
        self.inner.try_with_selected_queued_frame(|frame, _, _| {
            let living = frame.living_world.ok_or("unload LivingWorld manager absent")?;
            let recipes = frame.living_world_recipes.as_ref().ok_or("unload RecipeMan absent")?;
            living.try_reset_game(frame.world, frame.actor, frame.cvars, recipes, false)
                .map_err(|e| format!("unload LivingWorld reset: {e:?}"))
        }).map_err(|e| error(format!("{e:?}")))?.map_err(error)?;
        self.party_play.spawn = None;
        self.party_play.game = None;
        self.audio.revert_music_mode();
        self.try_remove_all_chase_arrows().map_err(error)?;
        self.game_unloaded = true;
        Ok(())
    }

    /// Whether the game unloaded since the last call, for the presentation's HUD.
    pub fn take_game_unloaded(&mut self) -> bool {
        std::mem::take(&mut self.game_unloaded)
    }

    /// `UnloadGameState`'s players before its `ResetGame`: `AISkaterManager::UnregisterAllSkaters`,
    /// `ResetPlayers`, then player 0 again (`CreatePlayer(0, PT_Human, -1)`, named for the game
    /// that is still in its mode) on the controller it had.
    fn unload_players(&mut self) -> Result<(), SelectedLiveError> {
        let Some(player) = self.inner.player.as_ref() else { return Ok(()) };
        let settings = Arc::clone(&player.selected.owners.services.settings);
        self.inner.try_unregister_all_ai_players().map_err(error)?;
        let controller = settings.try_controller_index(0).map_err(error)?;
        settings.try_reset_players().map_err(error)?;
        self.create_first_human()?;
        settings.try_set_controller_index(0, controller).map_err(error)
    }

    /// `CreatePlayer(0, PT_Human, -1)` 0x82563d50, whose `UpdatePlayerName` 0x82563a88 names a
    /// human by the game mode: Party Play's player name, otherwise the profile's.
    fn create_first_human(&self) -> Result<(), SelectedLiveError> {
        let services = &self.inner.player.as_ref().ok_or_else(|| error("first player absent"))?.selected.owners.services;
        if services.skaters.backend().try_game_mode().map_err(error)? == GM_PARTY_PLAY {
            let name = self.party_player_name(0)?;
            services.settings.try_create_party_human(0, crate::player::NO_PEER_ID, name.as_bytes()).map_err(error)
        } else {
            services.settings.try_create_offline_human(0, &services.profile, services.skaters.backend()).map_err(error)
        }
    }

    /// `StateMainGameFlow::DoStartSinglePlayerGame` 0x8277b070 outside Party Play, as a game
    /// starts: the AI players go, `ResetPlayers`, player 0 again with the profile's name, the
    /// loaded skater's attributes again (`ApplyPlayerAttributes(0)`, and slot 8's with ghosts,
    /// once `IsLoaded`: a skater still registering counts),
    /// and the front end's controller, the PC pad 0, for player 0.
    pub(super) fn start_single_player_players(&mut self) -> Result<(), SelectedLiveError> {
        self.inner.try_unregister_all_ai_players().map_err(error)?;
        let services = &self.inner.player.as_ref().ok_or_else(|| error("single player absent"))?.selected.owners.services;
        services.settings.try_reset_players().map_err(error)?;
        self.create_first_human()?;
        let player = self.inner.player.as_ref().ok_or_else(|| error("single player absent"))?;
        let (loader, services) = (&player.selected.owners.loader, &player.selected.owners.services);
        let mut regular = None;
        if loader.is_loaded()
            && let Some(selection) = loader.current()
        {
            let record = if matches!(selection, PlayerSelection::CreateACharacter { .. }) {
                EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(error)?)
                    .map_err(error)?
                    .load_record()
                    .map_err(error)?
            } else {
                None
            };
            services.apply_player_attributes(selection, 0, record.as_ref()).map_err(error)?;
            if services.skaters.backend().try_ghost_enabled().map_err(error)? {
                services.apply_player_attributes(selection, 8, record.as_ref()).map_err(error)?;
            }
            regular = Some(services.settings.try_stance_byte(0).map_err(error)? != 0);
        }
        services.settings.try_set_controller_index(0, 0).map_err(error)?;
        // ApplyPlayerAttributes' ReloadLocalPlayerAttributeData: the PlayerUI's stance.
        if let (Some(regular), Some(marker)) = (regular, self.inner.session_marker.as_mut()) {
            marker.teleport.reload_stance(regular);
        }
        Ok(())
    }

    /// `FrontEndState_MainMenu::EnterGame`826788b0's single-player start: `GM_SinglePlayer` with
    /// the created skater.
    pub(super) fn enter_single_player(&mut self) -> Result<(), SelectedLiveError> {
        self.set_game_mode(GM_SINGLE_PLAYER, None)?;
        self.start_single_player_players()?;
        self.pause_menu.set_hub(crate::ui::pause_menu::StartHub::SinglePlayer);
        self.load_created_skater()
    }

    /// The profile's created skater as the game's skater (`PlayerLoader::SetCACCharacter`).
    pub(super) fn load_created_skater(&mut self) -> Result<(), SelectedLiveError> {
        self.freeskate.pending_character = Some(None);
        self.apply_pending_character().map(drop)
    }

    /// Whether the front end's chosen skater still waits for the loader.
    pub(super) const fn front_end_character_pending(&self) -> bool {
        self.freeskate.pending_character.is_some()
    }

    /// Loads the front end's skater once the loader holds a registered one: true when it did,
    /// leaving the new presentation unpublished until the frame draws.
    pub(super) fn apply_pending_character(&mut self) -> Result<bool, SelectedLiveError> {
        let Some(marquee) = self.freeskate.pending_character else { return Ok(false) };
        let loaded = self
            .inner
            .player
            .as_ref()
            .is_some_and(|player| matches!(player.selected.owners.loader.phase(), PlayerLoadPhase::Loaded { .. }));
        if !loaded {
            return Ok(false);
        }
        self.freeskate.pending_character = None;
        self.replace_local_character(marquee)?;
        self.freeskate.spawn_attributes_pending = true;
        Ok(true)
    }

    /// Game load normally destroys/spawns the Actor (SkaterManager::OnCommand
    /// 82600984..9bc), after applying the selected character. This host keeps
    /// player zero: resample its IsSkaterBouncy constructor byte and animation
    /// character data at that boundary. A map teleport does not resample them.
    pub(super) fn apply_spawn_character_attributes(&mut self) -> Result<(), SelectedLiveError> {
        if !self.freeskate.spawn_attributes_pending {
            return Ok(());
        }
        self.apply_retained_skater_spawn_attributes(0)?;
        self.freeskate.spawn_attributes_pending = false;
        Ok(())
    }

    /// Both the retained first player and Change Game's later party players
    /// replace native cMsgSpawnPlayer with host teleports. Refresh each actual
    /// Actor's constructor attributes before dispatching that spawn.
    pub(super) fn apply_retained_skater_spawn_attributes(&mut self, player: u32) -> Result<(), SelectedLiveError> {
        self.inner.try_with_selected_queued_frame(|mut frame, _, _| {
            let mut selected = None;
            if frame.actor.prefix().map_err(|e| e.to_string())?
                .fields().ok_or("spawn Actor fields absent")?.player_index == player
            {
                selected = Some((frame.actor, frame.animation, frame.board, frame.skeleton));
            } else {
                for skater in &mut frame.others {
                    if skater.actor.prefix().map_err(|e| e.to_string())?
                        .fields().ok_or("spawn Actor fields absent")?.player_index == player
                    {
                        selected = Some((skater.actor, &mut *skater.animation, skater.board, skater.skeleton));
                        break;
                    }
                }
            }
            let (actor, animation, board, skeleton) = selected.ok_or("spawn skater absent")?;
            let bouncy = crate::actor_construction::player_animation::is_skater_bouncy(
                frame.services, frame.cvars, player,
            ).map_err(|e| e.to_string())?;
            frame.world.try_pair_set_spawn_bouncy(board, skeleton, bouncy)
                .map_err(|e| e.to_string())?;
            animation.constructor_flags.bouncy_ragdoll = u8::from(bouncy);
            crate::actor_animation_construction::edit_skater_attributes::reload_character_data_for_spawn(
                actor, frame.settings,
            )
        }).map_err(|e| error(format!("{e:?}")))?.map_err(error)?;
        Ok(())
    }

    pub(super) fn set_game_mode(&mut self, mode: u32, settings: Option<Settings>) -> Result<(), SelectedLiveError> {
        {
            let owner = self.inner.world.as_ref().ok_or_else(|| error("world absent"))?;
            let world = owner.world.lock().map_err(|_| error("world lock poisoned"))?;
            let local = world.local_skater_identity.as_ref().ok_or_else(|| error("local skater identity absent"))?;
            let info = local.backend().game_info();
            info.try_set_game_mode(mode).map_err(|e| error(format!("{e:?}")))?;
            if let Some(settings) = settings {
                info.try_set_free_skate((settings.traffic, settings.pedestrians, settings.security))
                    .map_err(|e| error(format!("{e:?}")))?;
            }
        }
        // GameLoad8273c01c -> PreLoadStateGraphs8272550c -> InitPropData.
        // Graphs stay loaded in the host, but a prior game unload destroyed
        // the tracker Prop. Recreate it through the original spawn path.
        self.inner.try_with_selected_queued_frame(|frame, _, _| {
            let recipes = frame.living_world_recipes.as_ref().ok_or("load RecipeMan absent")?;
            let mut living = frame.world.grind_services.living_world().lock()
                .map_err(|_| "load LivingWorld lock")?;
            recipes.prop_recipe_frame().initialize_actor_tracker(&mut living, frame.cvars)
        }).map_err(|e| error(format!("{e:?}")))?.map_err(error)?;
        // The game's load then runs GameModule::ResetGame, which makes its session marker.
        if let Some(marker) = self.inner.session_marker.as_mut() {
            let owner = self.inner.world.as_ref().ok_or_else(|| error("marker reset world absent"))?;
            let world = owner.world.lock().map_err(|_| error("marker reset world lock"))?;
            let mut living = world.grind_services.living_world().lock().map_err(|_| error("marker reset living world lock"))?;
            if let Some(session) = marker.marker.as_mut() {
                // ClearDynamicObjectLocatorsInSessionMarker828ad6a0 clears membership.
                session.props.clear();
                session.props.flush_cleared(&mut living).map_err(error)?;
            }
            let net_challenge_type = self.online.game.as_ref().map(|game| game.attributes.challenge_type);
            marker.try_reset_game(mode, net_challenge_type).map_err(error)?;
            marker.teleport.set_party_play_ui(mode == GM_PARTY_PLAY);
            marker.prop_actions.clear();
            marker.restore_props=false;
        }
        Ok(())
    }

    /// `PlayerLoader::SetMarqueeCharacter` / `SetCACCharacter`, then `Load`: an unchanged
    /// selection stays loaded, and the created skater without a saved record keeps the
    /// loaded skater, as an invalid CAS selection falls back to it.
    fn replace_local_character(&mut self, marquee: Option<u64>) -> Result<(), SelectedLiveError> {
        let player = self.inner.player.as_mut().ok_or_else(|| error("player absent"))?;
        let owners = &mut player.selected.owners;
        let record = match (marquee, owners.loader.requested()) {
            (None, PlayerSelection::CreateACharacter { .. }) => return Ok(()),
            (Some(id), PlayerSelection::Marquee { definition_id, .. }) if id == definition_id => return Ok(()),
            (None, _) => {
                let store = EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(error)?).map_err(error)?;
                let Some(record) = store.load_record().map_err(error)?.filter(|record| !record.data.recipe.is_empty()) else {
                    return Ok(());
                };
                Some(record)
            }
            (Some(_), _) => None,
        };
        let registry = owners.recipe_registry.as_mut().ok_or_else(|| error("recipe registry absent"))?;
        let recipe_load = owners.recipe_load.as_mut().ok_or_else(|| error("recipe load absent"))?;
        let action = match (marquee, record) {
            (Some(id), _) => {
                let instance = owners.services.vault.instance(freeskate_option::CHARACTERS_MARQUEE, id);
                let name = instance.try_cached_layout_text(0).map_err(error)?.ok_or_else(|| error("marquee schema name absent"))?;
                let definition = MarqueeDefinition { id, schema_name: std::str::from_utf8(name.bytes()).map_err(error)?.to_owned() };
                recipe_load.try_replace_marquee(
                    &mut owners.loader,
                    &definition,
                    &owners.services,
                    &mut owners.faces,
                    &owners.mounts,
                    &owners.cvars,
                    registry,
                )
            }
            (None, Some(record)) => recipe_load.try_replace_cac(
                &mut owners.loader,
                &owners.services,
                &mut owners.faces,
                &owners.mounts,
                &owners.cvars,
                registry,
                &record,
            ),
            (None, None) => return Ok(()),
        }
        .map_err(error)?;
        let PlayerLoadAction::Register(registration) = action else {
            return Err(error("free skate skater did not request presentation registration"));
        };
        player.registration_request = registration;
        owners.try_restore_replaced_recipe().map_err(error)?;
        let scene = Arc::clone(owners.recipe_load.as_ref().ok_or_else(|| error("recipe load absent"))?.scene().map_err(error)?);
        let runtime = self.inner.presentation_runtime.as_mut().ok_or_else(|| error("presentation runtime absent"))?;
        runtime
            .replace_recipe(owners.presentation.as_ref().ok_or_else(|| error("player presentation absent"))?, scene)
            .map_err(error)?;
        let finalized = owners.recipe_load.as_ref().ok_or_else(|| error("recipe load absent"))?.finalized().map_err(error)?;
        runtime
            .try_bind_materials(finalized, Arc::clone(&owners.services.vault), owners.cvars.clone())
            .map_err(error)?;
        Ok(())
    }
}
