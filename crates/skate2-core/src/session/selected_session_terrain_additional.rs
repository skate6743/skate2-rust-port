//! An additional world: a downloadable content location's, or a sponsor's training facility.
//! `GameModule::StartAdditionalWorldStreamLoad` (`0x8272d1c8`) loads its stream as BAM's is
//! (`StartWorldStreamLoad` `0x8272d0c0`) under a handle of its own, its Pres, Sim and Tex files
//! joining the one `cAssetStreamSystem` (here, the same catalog), whose `FindCollections`
//! (`0x824dfea8`) then selects from every Sim file.
//!
//! Unloading (`PurgeAdditionalWorldFile` `0x8272eb38`, `UpdateAdditionalWorldStreamUnload`
//! `0x8272d2d0`): the original removes every stream focus, waits for no pending request,
//! unloads the stream file and makes the focus again. The port takes the world out of the focus
//! selection, so its collections leave as any departing collection does, and unloads the stream
//! file once neither the simulation nor the presentation holds one of them, then lets the focus
//! carry on. The worlds stand in cells BAM's grid leaves empty, so BAM's collections are not
//! affected either way.
use super::{SelectedSessionTerrain, SelectedSessionTerrainError};
use crate::{
    game::{WORLD_STREAM_PATH, WorldStreamPoll, WorldStreamSpec},
    game_stream::{StreamKind, StreamPaths},
    selected_world_stream::SelectedWorldStream,
    world_stream_runtime::WorldStreamEffects,
};
use skate2_formats::stream::CollectionSpace;
use std::sync::Arc;

/// The additional worlds' `AddHighCollection`s: no PROXY collection stands at any of their
/// cells, so `ProxyWorldManager::AddHighCollection` maps none of them.
pub(crate) struct NoProxies;

impl WorldStreamEffects for NoProxies {
    fn add_high_collection(&mut self, _: u64, _: u32, _: u32) {}
    fn finalize_world_bounds(&mut self) {}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// `mIsLoadingAdditionalWorldStream`.
    Loading,
    Loaded,
    /// Out of the selection; its collections are leaving.
    Purging,
    /// `eUnloadAdditionalStreamState_UnloadingStreamFile`.
    Unloading,
}

pub(crate) struct AdditionalWorld {
    /// `GetAdditionalLocationWorldFile`: the stream's basename.
    file: String,
    stream: SelectedWorldStream<NoProxies>,
    /// Its Sim file's collections once the stream is in: the spatial ones, and every one its
    /// map lists (the global collection the cells stand under as well).
    space: Option<Arc<CollectionSpace>>,
    collections: Arc<[u64]>,
    phase: Phase,
}

impl AdditionalWorld {
    /// The Sim collections the focus selection takes from it.
    pub(super) fn selectable(&self) -> Option<&CollectionSpace> {
        self.space.as_deref().filter(|_| self.phase == Phase::Loaded)
    }
}

impl<Effects: WorldStreamEffects> SelectedSessionTerrain<Effects> {
    /// `StartAdditionalWorldStreamLoad`: the world's stream begins, one additional world at a
    /// time (the native asserts none is loading or unloading).
    pub(crate) fn try_start_additional_world(&mut self, file: &str) -> Result<(), SelectedSessionTerrainError> {
        use SelectedSessionTerrainError as E;
        if self.additional.is_some() {
            return Err(E::Focus("an additional world is already held"));
        }
        let spec = WorldStreamSpec::derive(file, None).map_err(|_| E::Focus("additional world file name"))?;
        let stream = SelectedWorldStream::try_new(
            Arc::clone(&self.source.mounts),
            self.source.stream.catalog.clone(),
            self.source.cvars.clone(),
            spec,
            NoProxies,
        )
        .map_err(E::Stream)?;
        self.additional = Some(AdditionalWorld { file: file.to_owned(), stream, space: None, collections: Arc::from([]), phase: Phase::Loading });
        Ok(())
    }

    /// The additional world held, loading, loaded or leaving.
    pub(crate) fn additional_world(&self) -> Option<&str> {
        self.additional.as_ref().map(|world| world.file.as_str())
    }

    /// Whether the held additional world is in: its stream loaded and selectable.
    pub(crate) fn additional_world_loaded(&self) -> bool {
        self.additional.as_ref().is_some_and(|world| world.phase == Phase::Loaded)
    }

    /// `UpdateAdditionalWorldStreamLoad` (`0x8272d248`): one step of the stream; true once it is
    /// in (or none loads).
    pub(crate) fn try_poll_additional_world(&mut self) -> Result<bool, SelectedSessionTerrainError> {
        let Some(phase) = self.additional.as_ref().map(|world| world.phase) else { return Ok(true) };
        match phase {
            Phase::Loading => {
                if self.try_finish_additional_stream()? {
                    self.additional.as_mut().expect("held above").phase = Phase::Loaded;
                    return Ok(true);
                }
                Ok(false)
            }
            phase => Ok(phase == Phase::Loaded),
        }
    }

    /// One step of the held world's stream load; true once it completed and its Sim file's
    /// collection space is held.
    fn try_finish_additional_stream(&mut self) -> Result<bool, SelectedSessionTerrainError> {
        use SelectedSessionTerrainError as E;
        let world = self.additional.as_mut().ok_or(E::Pending)?;
        if world.space.is_some() {
            return Ok(true);
        }
        if world.stream.try_poll().map_err(E::Stream)? != WorldStreamPoll::CompletedThisPoll {
            return Ok(false);
        }
        let paths = StreamPaths::derive(WORLD_STREAM_PATH, &world.file, StreamKind::Simulation)
            .map_err(|_| E::Focus("additional world stream path"))?;
        let file = self
            .source
            .stream
            .catalog
            .file_with_paths(StreamKind::Simulation, &paths)
            .map_err(E::Catalog)?
            .ok_or(E::Focus("additional world simulation stream missing"))?;
        let space = file.collection_space.ok_or(E::Focus("additional world simulation XSS missing"))?;
        let world = self.additional.as_mut().expect("held above");
        world.space = Some(space);
        world.collections = file.collection_ids;
        Ok(true)
    }

    /// `PurgeAdditionalWorldFile`: the world leaves the focus selection; its stream unloads in
    /// [`Self::try_update_additional_world_unload`]. Nothing happens without one.
    pub(crate) fn request_additional_world_unload(&mut self) {
        if let Some(world) = self.additional.as_mut()
            && matches!(world.phase, Phase::Loading | Phase::Loaded)
        {
            world.phase = Phase::Purging;
        }
    }

    /// A purge the next travel no longer wants, as its world is this one: the world stays, its
    /// collections selectable again. An unload already begun runs to its end.
    pub(crate) fn cancel_additional_world_unload(&mut self) {
        if let Some(world) = self.additional.as_mut()
            && world.phase == Phase::Purging
        {
            world.phase = if world.space.is_some() { Phase::Loaded } else { Phase::Loading };
        }
    }

    /// Whether the held world is leaving: purging or unloading.
    pub(crate) fn additional_world_leaving(&self) -> bool {
        self.additional.as_ref().is_some_and(|world| matches!(world.phase, Phase::Purging | Phase::Unloading))
    }

    /// `UpdateAdditionalWorldStreamUnload`: once none of its Sim collections is registered and
    /// `presentation_holds` (whether the presentation still holds any of its collections) is
    /// false, the stream file unloads (`UnloadGameStreamFile`, `UpdateGameStreamFileUnloaded`).
    /// True once no additional world is held.
    pub(crate) fn try_update_additional_world_unload(&mut self, presentation_holds: bool) -> Result<bool, SelectedSessionTerrainError> {
        use SelectedSessionTerrainError as E;
        let Some(phase) = self.additional.as_ref().map(|world| world.phase) else { return Ok(true) };
        match phase {
            Phase::Loading | Phase::Loaded => Ok(false),
            Phase::Purging => {
                // A stream still loading finishes first (the native asserts it is not loading);
                // none of its collections was selectable meanwhile.
                if !self.try_finish_additional_stream()? {
                    return Ok(false);
                }
                let world = self.additional.as_ref().expect("held above");
                let registered = self
                    .terrain
                    .as_ref()
                    .is_some_and(|terrain| world.collections.iter().any(|&id| terrain.collection_registered(id)));
                if registered || presentation_holds {
                    return Ok(false);
                }
                let world = self.additional.as_mut().expect("held above");
                world.stream.try_begin_unload().map_err(E::Stream)?;
                world.phase = Phase::Unloading;
                Ok(false)
            }
            Phase::Unloading => {
                let world = self.additional.as_mut().expect("held above");
                if world.stream.try_poll_unload().map_err(E::Stream)? {
                    self.additional = None;
                    return Ok(true);
                }
                Ok(false)
            }
        }
    }
}
