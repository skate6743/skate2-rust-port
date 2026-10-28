//! Bounded retained terrain/input continuation of the selected boot owner.
//! Camera, queued frame scheduling and the app host remain separate.
use super::SelectedSession;
use crate::{
    collection_resident_streamer::CollectionId,
    game::{WorldStreamPoll, WorldStreamSpec},
    native_skater_random::SimulationCommandRandom,
    physical_player_assembly::PhysicalAssemblyWorld,
    selected_production_input_owner::{ProductionInputBindError, ProductionInputBindFailure},
    selected_session_terrain::{
        SelectedSessionTerrain, SelectedSessionTerrainError, SelectedTerrainSource,
    },
    selected_sim_command_queue::SelectedSimCommandQueue,
    selected_terrain_startup::SelectedTerrainStartup,
    simulation_environment_unlock_router::EnvironmentUnlockSeed,
    static_ground_pool::StaticGroundPool,
    world_stream_catalog::WorldStreamCatalog,
    world_stream_runtime::{WorldBoundsPublication, WorldStreamEffects},
};
use std::sync::{Arc, Mutex};

/// Type erasure retains the exact selected terrain occurrence while keeping
/// `SelectedSession<T, C>` usable before a host chooses stream effects. Every
/// operation delegates to the existing owner; no second stream is constructed.
pub(crate) trait SelectedSessionTerrainSlot {
    fn try_poll_stream(&mut self) -> Result<WorldStreamPoll, SelectedSessionTerrainError>;
    fn try_request(&mut self) -> Result<(), SelectedSessionTerrainError>;
    fn try_poll(&mut self) -> Result<bool, SelectedSessionTerrainError>;
    fn try_poll_with_props(
        &mut self,
        props: Option<&mut crate::selected_living_world_recipes::props::PropRecipeFrame<'_>>,
    ) -> Result<bool, SelectedSessionTerrainError>;
    fn try_update_focus(&mut self, position: [f32; 3]) -> Result<(), SelectedSessionTerrainError>;
    fn try_prepare_teleport(&mut self, position: [f32; 3]) -> Result<bool, SelectedSessionTerrainError>;
    fn landable(&self, position: [f32; 3]) -> Result<bool, SelectedSessionTerrainError>;
    fn focus_position(&self) -> Option<[f32; 3]>;
    fn try_set_online_area(&mut self, area: Option<(&str, [f32; 3])>) -> Result<(), SelectedSessionTerrainError>;
    fn try_focus_loaded(&self) -> Result<bool, SelectedSessionTerrainError>;
    fn replay_focus_status(&self)->Result<crate::streaming::replay_focus::Status,SelectedSessionTerrainError>;
    fn try_area_loaded(&self, position: [f32; 3], radius: f32) -> Result<bool, SelectedSessionTerrainError>;
    fn try_advance_asset_release(&mut self) -> Result<(), SelectedSessionTerrainError>;
    fn load_hull(&mut self, file_name: &str, high_priority: bool, asset_flags: u32) -> Result<u32, SelectedSessionTerrainError>;
    fn unload_hull(&mut self, hull: u32) -> Result<(), SelectedSessionTerrainError>;
    fn try_update_hulls(&mut self) -> Result<Vec<u32>, SelectedSessionTerrainError>;
    fn hulls_unloaded(&self) -> bool;
    fn holds_hull(&self, hull: u32) -> bool;
    fn load_dynamic_hull(&mut self, file_name: &str, asset_flags: u32, callback: bool) -> Result<u32, SelectedSessionTerrainError>;
    fn unload_dynamic_hull(&mut self, hull: u32) -> Result<(), SelectedSessionTerrainError>;
    fn try_update_dynamic_hulls(&mut self) -> Result<Vec<u32>, SelectedSessionTerrainError>;
    fn dynamic_hulls_loading(&self) -> bool;
    fn dynamic_hulls_unloaded(&self) -> bool;
    fn hull_presentations(&self) -> Vec<crate::presentation::HullPresentation>;
    /// An additional world's stream (`selected_session_terrain_additional`).
    fn try_start_additional_world(&mut self, file: &str) -> Result<(), SelectedSessionTerrainError>;
    fn try_poll_additional_world(&mut self) -> Result<bool, SelectedSessionTerrainError>;
    fn additional_world(&self) -> Option<&str>;
    fn additional_world_loaded(&self) -> bool;
    fn request_additional_world_unload(&mut self);
    fn cancel_additional_world_unload(&mut self);
    fn additional_world_leaving(&self) -> bool;
    fn try_update_additional_world_unload(&mut self, presentation_holds: bool) -> Result<bool, SelectedSessionTerrainError>;
    fn bounds(&self) -> &WorldBoundsPublication;
    fn terrain_for(
        &self,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
    ) -> Result<&SelectedTerrainStartup, SelectedSessionTerrainError>;
    fn ground_pool_for(
        &self,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
    ) -> Result<&StaticGroundPool, SelectedSessionTerrainError>;
}
impl<E: WorldStreamEffects> SelectedSessionTerrainSlot for SelectedSessionTerrain<E> {
    fn replay_focus_status(&self)->Result<crate::streaming::replay_focus::Status,SelectedSessionTerrainError> {
        SelectedSessionTerrain::replay_focus_status(self)
    }
    fn try_prepare_teleport(&mut self, position: [f32; 3]) -> Result<bool, SelectedSessionTerrainError> {
        SelectedSessionTerrain::try_prepare_teleport(self, position)
    }
    fn landable(&self, position: [f32; 3]) -> Result<bool, SelectedSessionTerrainError> {
        SelectedSessionTerrain::landable(self, position)
    }
    fn focus_position(&self) -> Option<[f32; 3]> {
        SelectedSessionTerrain::focus_position(self)
    }
    fn try_set_online_area(&mut self, area: Option<(&str, [f32; 3])>) -> Result<(), SelectedSessionTerrainError> {
        SelectedSessionTerrain::try_set_online_area(self, area)
    }
    fn try_focus_loaded(&self) -> Result<bool, SelectedSessionTerrainError> {
        SelectedSessionTerrain::try_focus_loaded(self)
    }
    fn try_area_loaded(&self, position: [f32; 3], radius: f32) -> Result<bool, SelectedSessionTerrainError> {
        SelectedSessionTerrain::try_area_loaded(self, position, radius)
    }
    fn try_poll_stream(&mut self) -> Result<WorldStreamPoll, SelectedSessionTerrainError> {
        SelectedSessionTerrain::try_poll_stream(self)
    }
    fn try_request(&mut self) -> Result<(), SelectedSessionTerrainError> {
        SelectedSessionTerrain::try_request(self)
    }
    fn try_poll(&mut self) -> Result<bool, SelectedSessionTerrainError> {
        SelectedSessionTerrain::try_poll(self)
    }
    fn try_poll_with_props(
        &mut self,
        props: Option<&mut crate::selected_living_world_recipes::props::PropRecipeFrame<'_>>,
    ) -> Result<bool, SelectedSessionTerrainError> {
        SelectedSessionTerrain::try_poll_with_props(self, props)
    }
    fn try_update_focus(&mut self, position: [f32; 3]) -> Result<(), SelectedSessionTerrainError> {
        SelectedSessionTerrain::try_update_focus(self, position)
    }
    fn try_advance_asset_release(&mut self) -> Result<(), SelectedSessionTerrainError> {
        SelectedSessionTerrain::try_advance_asset_release(self)
    }
    fn load_hull(&mut self, file_name: &str, high_priority: bool, asset_flags: u32) -> Result<u32, SelectedSessionTerrainError> {
        SelectedSessionTerrain::load_hull(self, file_name, high_priority, asset_flags)
    }
    fn unload_hull(&mut self, hull: u32) -> Result<(), SelectedSessionTerrainError> {
        SelectedSessionTerrain::unload_hull(self, hull)
    }
    fn try_update_hulls(&mut self) -> Result<Vec<u32>, SelectedSessionTerrainError> {
        SelectedSessionTerrain::try_update_hulls(self)
    }
    fn hulls_unloaded(&self) -> bool {
        SelectedSessionTerrain::hulls_unloaded(self)
    }
    fn holds_hull(&self, hull: u32) -> bool {
        SelectedSessionTerrain::holds_hull(self, hull)
    }
    fn load_dynamic_hull(&mut self, file_name: &str, asset_flags: u32, callback: bool) -> Result<u32, SelectedSessionTerrainError> {
        SelectedSessionTerrain::load_dynamic_hull(self, file_name, asset_flags, callback)
    }
    fn unload_dynamic_hull(&mut self, hull: u32) -> Result<(), SelectedSessionTerrainError> {
        SelectedSessionTerrain::unload_dynamic_hull(self, hull)
    }
    fn try_update_dynamic_hulls(&mut self) -> Result<Vec<u32>, SelectedSessionTerrainError> {
        SelectedSessionTerrain::try_update_dynamic_hulls(self)
    }
    fn dynamic_hulls_loading(&self) -> bool {
        SelectedSessionTerrain::dynamic_hulls_loading(self)
    }
    fn dynamic_hulls_unloaded(&self) -> bool {
        SelectedSessionTerrain::dynamic_hulls_unloaded(self)
    }
    fn hull_presentations(&self) -> Vec<crate::presentation::HullPresentation> {
        SelectedSessionTerrain::hull_presentations(self)
    }
    fn try_start_additional_world(&mut self, file: &str) -> Result<(), SelectedSessionTerrainError> {
        SelectedSessionTerrain::try_start_additional_world(self, file)
    }
    fn try_poll_additional_world(&mut self) -> Result<bool, SelectedSessionTerrainError> {
        SelectedSessionTerrain::try_poll_additional_world(self)
    }
    fn additional_world(&self) -> Option<&str> {
        SelectedSessionTerrain::additional_world(self)
    }
    fn additional_world_loaded(&self) -> bool {
        SelectedSessionTerrain::additional_world_loaded(self)
    }
    fn request_additional_world_unload(&mut self) {
        SelectedSessionTerrain::request_additional_world_unload(self);
    }
    fn cancel_additional_world_unload(&mut self) {
        SelectedSessionTerrain::cancel_additional_world_unload(self);
    }
    fn additional_world_leaving(&self) -> bool {
        SelectedSessionTerrain::additional_world_leaving(self)
    }
    fn try_update_additional_world_unload(&mut self, presentation_holds: bool) -> Result<bool, SelectedSessionTerrainError> {
        SelectedSessionTerrain::try_update_additional_world_unload(self, presentation_holds)
    }
    fn bounds(&self) -> &WorldBoundsPublication {
        &self.stream().bounds
    }
    fn terrain_for(
        &self,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
    ) -> Result<&SelectedTerrainStartup, SelectedSessionTerrainError> {
        SelectedSessionTerrain::terrain_for(self, world)
    }
    fn ground_pool_for(
        &self,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
    ) -> Result<&StaticGroundPool, SelectedSessionTerrainError> {
        SelectedSessionTerrain::ground_pool_for(self, world)
    }
}

#[derive(Debug)]
pub(crate) enum SelectedSessionTerrainStartError<'a> {
    AlreadyAttempted,
    MissingAssets,
    MissingWorld,
    Source(&'a SelectedSessionTerrainError),
}

/// A precondition failure returns the caller's untouched command owners. A
/// failed actual bind is retained in `SelectedSession::failed_input_bind`; the
/// returned reference names that retained cause without moving it again.
pub(crate) struct UnboundSelectedCommands {
    pub(crate) queue: SelectedSimCommandQueue,
    pub(crate) random: SimulationCommandRandom,
}
pub(crate) enum SelectedSessionInputBindError<'a> {
    AlreadyAttempted(UnboundSelectedCommands),
    MissingPlayer(UnboundSelectedCommands),
    MissingBootstrap(UnboundSelectedCommands),
    Bind(&'a ProductionInputBindError),
}

impl<T, C> SelectedSession<T, C> {
    /// Host supplies its selected stream, collection and raw policies once.
    /// The source receives this session's exact world, mount and cvar owners.
    /// A source-constructor failure retains its typed error here and ends this
    /// attempt; the lower constructor does not return consumed effects or seed.
    /// This does not infer native spatial focus or order terrain before the pair.
    #[expect(
        clippy::too_many_arguments,
        reason = "caller keeps distinct stream, unlock, collection and registration policies explicit"
    )]
    pub(crate) fn try_select_terrain<E: WorldStreamEffects + Send + 'static>(
        &mut self,
        catalog: WorldStreamCatalog,
        spec: WorldStreamSpec,
        effects: E,
        unlock_seed: EnvironmentUnlockSeed,
        collection: CollectionId,
        dmo_update_in_parallel: bool,
        arena_userdata: u32,
    ) -> Result<(), SelectedSessionTerrainStartError<'_>> {
        use SelectedSessionTerrainStartError as Error;
        if self.terrain_attempted {
            return Err(Error::AlreadyAttempted);
        }
        let assets = self.assets.as_ref().ok_or(Error::MissingAssets)?;
        let world = self.world.as_ref().ok_or(Error::MissingWorld)?;
        self.terrain_attempted = true;
        let source = match SelectedTerrainSource::try_new(
            Arc::clone(&world.world),
            Arc::clone(&assets.mounts),
            catalog,
            assets.cvars.clone(),
            spec,
            effects,
            unlock_seed,
        ) {
            Ok(source) => source,
            Err(error) => {
                self.terrain_start_failure = Some(error);
                return Err(Error::Source(
                    self.terrain_start_failure
                        .as_ref()
                        .expect("retained terrain constructor error"),
                ));
            }
        };
        self.terrain = Some(Box::new(SelectedSessionTerrain::new(
            source,
            collection,
            dmo_update_in_parallel,
            arena_userdata,
        )));
        Ok(())
    }

    /// Consume the early boot occurrence only after the selected player exists.
    /// A failed bind keeps bootstrap, queue and RNG together in this session.
    #[expect(
        clippy::result_large_err,
        reason = "precondition errors return the supplied queue and random owner without loss"
    )]
    pub(crate) fn try_bind_selected_input(
        &mut self,
        queue: SelectedSimCommandQueue,
        random: SimulationCommandRandom,
    ) -> Result<(), SelectedSessionInputBindError<'_>> {
        use SelectedSessionInputBindError as Error;
        let commands = UnboundSelectedCommands { queue, random };
        if self.input_bind_attempted {
            return Err(Error::AlreadyAttempted(commands));
        }
        let Some(player) = self.player.as_ref() else {
            return Err(Error::MissingPlayer(commands));
        };
        let Some(input) = self.input.take() else {
            return Err(Error::MissingBootstrap(commands));
        };
        self.input_bind_attempted = true;
        match input.try_bind_player(&player.selected.owners, commands.queue, commands.random) {
            Ok(bound) => {
                self.bound_input = Some(bound);
                Ok(())
            }
            Err(failure) => {
                self.failed_input_bind = Some(failure);
                Err(Error::Bind(
                    &self
                        .failed_input_bind
                        .as_ref()
                        .expect("retained bind failure")
                        .error,
                ))
            }
        }
    }

    pub(crate) fn failed_input_bind(&self) -> Option<&ProductionInputBindFailure<C>> {
        self.failed_input_bind.as_ref()
    }

    pub(crate) fn terrain_start_failure(&self) -> Option<&SelectedSessionTerrainError> {
        self.terrain_start_failure.as_ref()
    }
}
