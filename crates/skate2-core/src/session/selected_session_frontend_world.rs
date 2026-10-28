//! The world behind the main menu. `GameModule::OneOffLoad` 0x8273b1a0 boots at the non-streamed
//! `Z_TrainingStart_Fixed` location, and the simulation runs there for the ambient AI skaters
//! the front end camera watches (`AISkaterManager` in its minimal model set, front-end
//! distances). The first player is there too: `ResetGame` 0x8273a680's `PlayerUI::OnReset`
//! 0x828aadd0 asks for it and `PlayerUI::Update` 0x828af9f8, run by `DoGameUpdate` 0x82730418
//! every frame, spawns it at the location (`PlayerUI::ResetPlayer` 0x828ad280); it is
//! `GetLocalSimIndex`, the skater the audio's microphones follow. Unfocused, its input unpacks
//! with Brake held (`PlayerInputData::Unpack` 0x82a8c330), so it stands braking; the port parks
//! it there pinned (`ForceBrake`) and draws it in neither scene.
use super::{SelectedLiveError, SelectedLiveSession, SelectedLiveStage as Stage, SelectedQueuedFramePolicy};
use crate::{
    actor_construction::selected_tick::{SelectedQueuedTickOutcome, SelectedTickNumerics, SelectedTickRequest},
    challenge::{
        attrib_db::AttribDb,
        unlockables::{self, Unlockables},
    },
    edit_skater_profile::EditSkaterProfileStore,
    game_input_recognizer_runtime_services::GameInputRuntimeServices,
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    selected_session_boot_owner::ai_skaters::SessionAi,
    simulation_assembly_backend::constraint_session::{
        ground_pre_solve::{GroundPreparationGrindPolicy, HostGroundPreparationNumeric},
        skateboard_ground::{SkateboardGroundFrameInput, SkateboardGroundFrameSchedule},
        update5_managers::GrindManagerFillPolicy,
    },
    skeleton_root_transforms::SkeletonRootMatrix,
};
use skate2_formats::location::LocationTable;
use std::{fmt::Debug, sync::Arc};

/// `Z_TrainingStart_Fixed` (`AddNonStreamedLocation`): identity axes at the boot position.
const TRAINING_START: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    f32::from_bits(0xc525_6ddb), f32::from_bits(0x4301_b88c), f32::from_bits(0xc2cb_1ef8), 1.0,
];
const SIM_LOCATIONS: &str = "data\\content\\world\\stream\\BAM\\BAM_Sim.loc";
/// The first player's index, the front end's `GetLocalSimIndex` (`ResetGame`'s `SetLocalIndex`).
const FIRST_PLAYER: i32 = 0;

fn error(stage: Stage, cause: impl Debug) -> SelectedLiveError {
    SelectedLiveError::failed(stage, format!("front end world: {cause:?}"))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    /// The game owns the world.
    #[default]
    Off,
    /// The location streams in before the first player moves there.
    Streaming,
    Running,
}

#[derive(Default)]
pub(super) struct FrontendWorld {
    phase: Phase,
}

impl<T, C> SelectedLiveSession<T, C> {
    /// The session AI, made with the profile's unlockables on first use.
    fn try_ensure_session_ai(&mut self) -> Result<(), SelectedLiveError> {
        if self.inner.ai.is_some() {
            return Ok(());
        }
        let assets = self.inner.assets.as_ref().ok_or_else(|| error(Stage::Visual, "assets absent"))?;
        let mounts = Arc::clone(&assets.mounts);
        let vault = Arc::clone(&assets.vault);
        if self.freeskate.data.is_none() {
            let db = Arc::new(AttribDb::load_career(&mounts).map_err(|e| error(Stage::Visual, e))?);
            let table = LocationTable::parse(&mounts.read_path(SIM_LOCATIONS).map_err(|e| error(Stage::Visual, e))?)
                .map_err(|e| error(Stage::Visual, e))?;
            self.freeskate.data = Some((db, table));
        }
        let (db, _) = self.freeskate.data.as_ref().expect("free skate data loaded");
        // UnlockablesBackEnd::LoadData 0x8255cd08 over the defaults Init added.
        let store = EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(|e| error(Stage::Visual, e))?)
            .map_err(|e| error(Stage::Visual, e))?;
        let mut unlocks = Unlockables::new(db);
        unlocks.load(super::career::leaf_tuples(
            &store.load_leaf(unlockables::SAVABLE, unlockables::DATA).map_err(|e| error(Stage::Visual, e))?,
        ));
        let world = self.inner.world.as_ref().ok_or_else(|| error(Stage::Visual, "world absent"))?;
        let paths = Arc::clone(
            &self
                .inner
                .terrain
                .as_ref()
                .ok_or_else(|| error(Stage::Visual, "terrain absent"))?
                .terrain_for(&world.world)
                .map_err(|e| error(Stage::Visual, e))?
                .ai_paths,
        );
        self.inner.ai = Some(SessionAi::try_new(paths, &vault, Some(unlocks)).map_err(|e| error(Stage::Visual, e))?);
        Ok(())
    }

    /// One main-menu frame of the front end's world: the location streams in, the first player
    /// parks there pinned, then each frame queues a sync command without player input and runs
    /// the simulation frame with the AI, as the front end's `SimController` does.
    pub(super) fn try_tick_frontend_world<CE, TE>(
        &mut self,
        policy: SelectedQueuedFramePolicy,
        clamp: &mut impl FnMut() -> Result<i32, CE>,
    ) -> Result<(), SelectedLiveError>
    where
        CE: Debug,
        T: FnMut() -> Result<u64, TE>,
        TE: Debug,
    {
        self.try_ensure_session_ai()?;
        self.try_unload_frontend_additional_world()?;
        let position = [TRAINING_START[12], TRAINING_START[13], TRAINING_START[14]];
        let terrain = self.inner.terrain.as_mut().ok_or_else(|| error(Stage::TerrainResident, "terrain absent"))?;
        terrain.try_advance_asset_release().map_err(|e| error(Stage::TerrainResident, e))?;
        if self.frontend_world.phase != Phase::Running {
            self.frontend_world.phase = Phase::Streaming;
            if !terrain.try_prepare_teleport(position).map_err(|e| error(Stage::TerrainResident, e))? {
                return Ok(());
            }
            let actor = self
                .inner
                .player
                .as_ref()
                .and_then(|player| player.selected.actor.as_ref())
                .ok_or_else(|| error(Stage::SessionMarker, "first Actor absent"))?;
            actor.try_set_pinned(true).map_err(|e| error(Stage::SessionMarker, e))?;
            actor
                .try_publish_teleport(&SkeletonRootMatrix::known(TRAINING_START), false)
                .map_err(|e| error(Stage::SessionMarker, e))?;
            self.frontend_world.phase = Phase::Running;
        } else {
            terrain.try_update_focus(position).map_err(|e| error(Stage::TerrainResident, e))?;
        }
        let camera = self
            .main_menu_render_frame(0.0)?
            .ok_or_else(|| error(Stage::Visual, "main menu frame absent"))?
            .camera;
        let ai = self.inner.ai.as_mut().expect("session AI made");
        ai.camera = std::array::from_fn(|row| std::array::from_fn(|lane| camera[row * 4 + lane]));
        ai.in_front_end = true;
        ai.party_play = false;
        ai.local_player = Some(FIRST_PLAYER);
        ai.set_recipes_in_use(Vec::new());
        self.inner
            .bound_input
            .as_mut()
            .ok_or_else(|| error(Stage::QueuedFrame, "input absent"))?
            .try_generate_frontend_sync_command()
            .map_err(|e| error(Stage::QueuedFrame, e))?;
        let world = self.inner.world.as_ref().ok_or_else(|| error(Stage::QueuedFrame, "world absent"))?;
        let assets = self.inner.assets.as_ref().ok_or_else(|| error(Stage::QueuedFrame, "assets absent"))?;
        let cvars = assets.cvars.clone();
        let vault = Arc::clone(&assets.vault);
        let (mut sync, mut feedback, overlay) = {
            let world = world.world.lock().map_err(|_| error(Stage::QueuedFrame, "world lock poisoned"))?;
            let services = world
                .physical_world_services()
                .ok_or_else(|| error(Stage::QueuedFrame, "physical world services absent"))?;
            (
                Arc::clone(services.game_info()),
                world.try_live_contact_feedback(&cvars).map_err(|e| error(Stage::QueuedFrame, e))?,
                world.debug_overlay.clone(),
            )
        };
        let mut recognizer = GameInputRuntimeServices::new(&cvars, &vault, &overlay, None);
        let mut numeric = SelectedTickNumerics {
            input: HostReckoningNumeric,
            actor: HostReckoningNumeric,
            pre: HostGroundPreparationNumeric::default(),
            line: crate::collision_line_numeric::HostCollisionLineNumeric,
            vector: HostReckoningNumeric,
            rounding: HostReckoningSinCosNumeric,
            truck: crate::skateboard_truck_drives::HostSkateboardTruckDriveNumeric,
            collision: crate::skeleton_collision_numeric::HostSkeletonCollisionNumeric,
        };
        let outcome = self
            .inner
            .try_with_selected_queued_frame(|mut frame, ground, queued| {
                // The front end runs no LivingWorld manager; RecipeMan keeps the GameSim queue.
                frame.living_world = None;
                frame.try_step_queued::<_, _, _, _, _, _, std::convert::Infallible>(
                    SelectedTickRequest {
                        input: queued,
                        dt: policy.dt,
                        lod: policy.lod,
                        wait_for_bs_job: policy.wait_for_bs_job,
                        physics: SkateboardGroundFrameInput {
                            schedule: SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                            ground_pool: ground,
                            ground_group: policy.ground_group,
                            pipeline_culling: None,
                            allow_multiple_writes: policy.allow_multiple_writes,
                            spy_flags: policy.spy_flags,
                        },
                        pre_grind: GroundPreparationGrindPolicy::RunRecoveredPrefix,
                        post_grind: GrindManagerFillPolicy::RequireNative,
                    },
                    &mut numeric,
                    &mut sync,
                    &mut recognizer,
                    &mut feedback,
                    clamp,
                    &mut HostReckoningNumeric,
                    None,
                    None,
                )
            })
            .map_err(|e| error(Stage::QueuedFrame, e))?
            .map_err(|e| error(Stage::QueuedFrame, e))?;
        if matches!(outcome, SelectedQueuedTickOutcome::Completed(_)) {
            self.inner.try_complete_party_registration().map_err(|e| error(Stage::Visual, e))?;
            self.inner.try_dispatch_ai_messages().map_err(|e| error(Stage::ControlledPlayer, e))?;
            // No challenge runs behind the front end: its Challenge queue messages reach no handler.
            self.inner.try_take_challenge_messages().map_err(|e| error(Stage::ControlledPlayer, e))?;
        }
        Ok(())
    }

    /// Leaving the front end for a game: `GameModule::GameLoad` 0x8273c8f0 loads every AI model
    /// set and the front end's skaters go with the reset; the first player is unpinned for the
    /// game's own start. `AISkaterManager::LoadGame` notes a network game, in which no ambient
    /// skater updates.
    pub(super) fn try_leave_frontend_world(&mut self) -> Result<(), SelectedLiveError> {
        if self.frontend_world.phase == Phase::Off {
            return Ok(());
        }
        self.frontend_world.phase = Phase::Off;
        self.inner.try_destroy_ai_skaters().map_err(|e| error(Stage::ControlledPlayer, e))?;
        if let Some(ai) = self.inner.ai.as_mut() {
            ai.in_front_end = false;
            ai.local_player = Some(FIRST_PLAYER);
            let mut manager = ai.manager();
            manager.load_game(self.online.game.is_some());
            manager.model_loading_mode = crate::ai::skater_manager::ModelLoadingMode::LoadAll;
        }
        self.inner
            .player
            .as_ref()
            .and_then(|player| player.selected.actor.as_ref())
            .ok_or_else(|| error(Stage::SessionMarker, "first Actor absent"))?
            .try_set_pinned(false)
            .map_err(|e| error(Stage::SessionMarker, e))
    }

    /// Whether the front end's world runs, its first player parked at the location.
    pub(super) fn frontend_world_running(&self) -> bool {
        self.frontend_world.phase == Phase::Running
    }

    /// `AudioPresentation::DoSimSubmit` 0x827fb2f8's skaters behind the menu: the first player,
    /// the local one, and the ambient AI skaters.
    #[must_use]
    pub fn audio_front_end_skaters(&self) -> Vec<crate::AudioSkater> {
        if !self.frontend_world_running() {
            return Vec::new();
        }
        self.audio_roster().unwrap_or_default()
    }

    /// The front end's AI skaters, drawn in its scene.
    pub fn frontend_ai_skater_visuals(&self) -> Result<Vec<crate::SelectedSkaterVisual<'_>>, SelectedLiveError> {
        if self.frontend_world.phase != Phase::Running {
            return Ok(Vec::new());
        }
        self.inner.ai_skater_visuals().map_err(|e| error(Stage::Visual, e))
    }
}
