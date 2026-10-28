//! Mounted outer stream composition over supplied owners and live settings.
use crate::{
    game::{
        WorldStreamBackend, WorldStreamControllerError, WorldStreamHandle,
        WorldStreamInvariantError, WorldStreamLoads, WorldStreamPoll, WorldStreamSpec,
    },
    input_cvar_runtime::InputCvarReferences,
    mounted_stream_backend::{MountedStreamBackend, MountedStreamBackendError},
    world_stream_adapter::{WorldStreamAdapter, WorldStreamAdapterError},
    world_stream_catalog::{CatalogError, WorldStreamCatalog},
    world_stream_runtime::{
        TextureStreamGate, TextureStreamGateError, WorldBoundsError, WorldBoundsPublication,
        WorldStreamEffects, WorldStreamRuntime, WorldStreamRuntimeError,
    },
};
use skate2_formats::mount::MountSet;
use std::sync::Arc;

pub(crate) struct LiveTextureGate(InputCvarReferences);
impl TextureStreamGate for LiveTextureGate {
    fn texture_stream_enabled(&mut self) -> Result<bool, TextureStreamGateError> {
        self.0.try_enable_texture_stream()
    }
}
type Adapter<E> = WorldStreamAdapter<
    MountedStreamBackend<WorldStreamCatalog>,
    WorldStreamRuntime<LiveTextureGate, E>,
>;
type StreamError =
    Arc<WorldStreamAdapterError<MountedStreamBackendError<CatalogError>, WorldStreamRuntimeError>>;
#[derive(Debug)]
pub(crate) enum Error {
    Mounted(MountedStreamBackendError<CatalogError>),
    Start(WorldStreamInvariantError),
    Poll(Arc<WorldStreamControllerError<StreamError>>),
    Bounds(WorldBoundsError),
    Unload(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mounted(e) => write!(f, "mounted world stream: {e:?}"),
            Self::Start(e) => write!(f, "world stream start: {e}"),
            Self::Poll(e) => write!(f, "world stream poll: {e}"),
            Self::Bounds(e) => write!(f, "world stream bounds: {e}"),
            Self::Unload(e) => write!(f, "world stream unload: {e}"),
        }
    }
}
/// A completed stream's files while `cGameAssetManager::UpdateGameStreamFileUnloaded`
/// (`0x824e2300`) takes them out: Pres, then Sim, then Tex (the texture stream enabled), each
/// `cStreamFile::Unload`ed once the one before is done.
struct Unloading<F> {
    files: Vec<F>,
    /// The file being unloaded.
    next: usize,
    /// Its `cStreamFile::Unload` was asked.
    requested: bool,
}
/// Owns the existing stream controller, files and publication for their consumers.
/// The caller supplies the actual high-collection effects; none are invented here.
pub(crate) struct SelectedWorldStream<E: WorldStreamEffects> {
    pub(crate) catalog: WorldStreamCatalog,
    pub(crate) bounds: WorldBoundsPublication,
    adapter: Adapter<E>,
    loads: WorldStreamLoads<<Adapter<E> as WorldStreamBackend>::StreamFile, StreamError>,
    handle: WorldStreamHandle,
    unloading: Option<Unloading<<Adapter<E> as WorldStreamBackend>::StreamFile>>,
}
impl<E: WorldStreamEffects> SelectedWorldStream<E> {
    pub(crate) fn try_new(
        mounts: Arc<MountSet>,
        catalog: WorldStreamCatalog,
        cvars: InputCvarReferences,
        spec: WorldStreamSpec,
        effects: E,
    ) -> Result<Self, Error> {
        let inner = MountedStreamBackend::new(mounts, catalog.clone()).map_err(Error::Mounted)?;
        let runtime = WorldStreamRuntime::new(catalog.clone(), LiveTextureGate(cvars), effects);
        let bounds = runtime.world_bounds();
        let adapter = WorldStreamAdapter::new(inner, runtime);
        let mut loads = WorldStreamLoads::new();
        let handle = loads.start(spec).map_err(Error::Start)?;
        Ok(Self {
            catalog,
            bounds,
            adapter,
            loads,
            handle,
            unloading: None,
        })
    }
    /// `cGameAssetManager::UnloadGameStreamFile` (`0x824da8f0`): the completed stream begins
    /// unloading.
    pub(crate) fn try_begin_unload(&mut self) -> Result<(), Error> {
        if self.unloading.is_some() {
            return Err(Error::Unload("already unloading".into()));
        }
        let (_, _, presentation, simulation, texture) =
            self.loads.take_completed(self.handle).map_err(|e| Error::Unload(format!("{e:?}")))?.into_parts();
        let mut files = vec![presentation, simulation];
        if let crate::game::CompletedTextureStream::Loaded(texture) = texture {
            files.push(texture);
        }
        self.unloading = Some(Unloading { files, next: 0, requested: false });
        Ok(())
    }
    /// One `UpdateGameStreamFileUnloaded` step: the first file's unload asked, else the file
    /// unloading advanced and, once it is out, the next one's asked. True once every file is out
    /// (the case that recalculates the world box, `CalcWorldBBox`).
    pub(crate) fn try_poll_unload(&mut self) -> Result<bool, Error> {
        let Some(unloading) = self.unloading.as_mut() else { return Err(Error::Unload("not unloading".into())) };
        let unload = |adapter: &mut Adapter<E>, file: &mut <Adapter<E> as WorldStreamBackend>::StreamFile| {
            adapter.request_stream_file_unload(file).map_err(|e| Error::Unload(format!("{e:?}")))
        };
        if !unloading.requested {
            unload(&mut self.adapter, &mut unloading.files[unloading.next])?;
            unloading.requested = true;
            return Ok(false);
        }
        if unloading.next == unloading.files.len() {
            self.unloading = None;
            return Ok(true);
        }
        let state = self
            .adapter
            .poll_stream_file(&mut unloading.files[unloading.next])
            .map_err(|e| Error::Unload(format!("{e:?}")))?;
        if state.raw() != crate::game_stream::StreamFileState::UNLOAD_DONE.raw() {
            return Ok(false);
        }
        unloading.next += 1;
        if unloading.next < unloading.files.len() {
            unload(&mut self.adapter, &mut unloading.files[unloading.next])?;
        }
        Ok(false)
    }
    pub(crate) fn state(&self) -> Result<u32, WorldStreamInvariantError> {
        self.loads
            .state(self.handle)
            .map(crate::game::WorldStreamOuterState::raw)
    }
    /// Performs exactly one existing outer-state step. Successful completion
    /// additionally requires the actual bounds publication, not the void callback.
    pub(crate) fn try_poll(&mut self) -> Result<WorldStreamPoll, Error> {
        let result = self
            .loads
            .poll(self.handle, &mut self.adapter)
            .map_err(Error::Poll)?;
        if result == WorldStreamPoll::CompletedThisPoll {
            self.bounds.read().map_err(Error::Bounds)?;
        }
        Ok(result)
    }
}
#[cfg(test)]
#[path = "selected_world_stream_tests.rs"]
mod tests;
