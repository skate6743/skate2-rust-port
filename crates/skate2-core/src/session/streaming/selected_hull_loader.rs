//! `Sk8::cHullLoader` (`Update` `0x82eedc40`) as the challenge asset
//! manager's and the dynamic hull manager's `cPathedHullLoader`s run it: load
//! flags 7 (simulation, presentation and texture streams,
//! `kEnableHullLoaderTextureStream` on), each stream's XSM/XST manifest
//! registered in the world stream catalog after its `.xsm` exists, then the
//! collections of each stream in turn. Simulation collections load and
//! register through the terrain's resident loader with the hull's asset flags
//! as arena userdata. Presentation collections are handed to the presentation
//! ([`SelectedHullLoader::presentations`]), which loads and draws them on its
//! own thread; the simulation counts them registered at once rather than
//! waiting for the presentation's `cHullCollectionRegistrationCallback`.
//! Texture collections are not registered by the port (the mission hulls'
//! texture streams hold none).
use crate::{
    challenge::assets::{CONTENT_STREAM_DIRECTORY, clamp_file_name, hull_id},
    game_stream::{GameStreamState, InnerStreamBackend, ManifestOnlyStream, StreamKind},
    mounted_stream_backend::MountedStreamBackend,
    selected_terrain_startup::SelectedTerrainStartup,
    world_stream_catalog::WorldStreamCatalog,
};
use skate2_formats::mount::MountSet;
use std::{collections::BTreeMap, sync::Arc};

type Backend = MountedStreamBackend<WorldStreamCatalog>;
type Manifest = ManifestOnlyStream<<Backend as InnerStreamBackend>::Handle, <Backend as InnerStreamBackend>::Error>;

/// `AssetPaths::tStreamType` in the loader's order.
const STREAMS: [StreamKind; 3] = [StreamKind::Simulation, StreamKind::Presentation, StreamKind::Texture];

/// `eHullLoadState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    LoadNotStarted,
    CheckExistsSim,
    LoadingSimToc,
    CheckExistsPres,
    LoadingPresToc,
    CheckExistsTexture,
    LoadingTextureToc,
    LoadingSimCollections,
    LoadingPresCollections,
    LoadingTextureCollections,
    LoadCompleteNotify,
    LoadComplete,
    UnloadNotStarted,
    UnloadingCollections,
    UnloadingToc,
    UnloadCompleteNotify,
}

/// One stream of a hull: the `StartExistsCheck` answer, the loaded manifest
/// (absent is `kInvalidHashID`) and the collections its load started.
#[derive(Default)]
struct HullStream {
    exists: bool,
    manifest: Option<Manifest>,
    collections: Vec<u64>,
}

/// `tHullLoadInfo`.
struct HullLoadInfo {
    state: State,
    request_unload: bool,
    file_name: String,
    /// `LoadCallback` and `UnloadCallback` are set.
    load_callback: bool,
    unload_callback: bool,
    high_priority: bool,
    asset_flags: u32,
    streams: [HullStream; 3],
}

pub(crate) struct SelectedHullLoader {
    mounts: Arc<MountSet>,
    backend: Backend,
    /// `HullLoadInfoList`, keyed by hull ID.
    infos: BTreeMap<u32, HullLoadInfo>,
    active_load: Option<u32>,
    active_unload: Option<u32>,
    /// `DuplicateRequests`: callbacks answered at the end of the next update.
    duplicates: Vec<u32>,
}

impl SelectedHullLoader {
    pub(crate) fn try_new(mounts: Arc<MountSet>, catalog: WorldStreamCatalog) -> Result<Self, String> {
        let backend = MountedStreamBackend::new(Arc::clone(&mounts), catalog).map_err(|e| format!("hull stream backend: {e:?}"))?;
        Ok(Self { mounts, backend, infos: BTreeMap::new(), active_load: None, active_unload: None, duplicates: Vec::new() })
    }

    /// `cHullLoader::LoadHull` (`0x82eeedc8`): a new hull is queued; one
    /// already loaded answers a callback at the next update.
    pub(crate) fn load_hull(&mut self, file_name: &str, high_priority: bool, asset_flags: u32, callback: bool) -> u32 {
        let id = hull_id(file_name);
        match self.infos.get(&id) {
            None => {
                self.infos.insert(id, HullLoadInfo {
                    state: State::LoadNotStarted,
                    request_unload: false,
                    file_name: clamp_file_name(file_name).to_owned(),
                    load_callback: callback,
                    unload_callback: false,
                    high_priority,
                    asset_flags: asset_flags | 2,
                    streams: Default::default(),
                });
            }
            // IsHullLoaded (0x82eed968) with a callback: a duplicate request.
            Some(info) if callback && matches!(info.state, State::LoadCompleteNotify | State::LoadComplete) => self.duplicates.push(id),
            Some(_) => {}
        }
        id
    }

    /// `cHullLoader::IsLoading` (`0x82eed6d8`): a hull short of `LoadCompleteNotify`.
    pub(crate) fn is_loading(&self) -> bool {
        self.infos.values().any(|info| (info.state as u32) < State::LoadCompleteNotify as u32)
    }

    /// `cChallengeDynamicHullManager::IsAllUnloaded` (`0x82da1680`): the loader tracks no hull.
    pub(crate) fn is_all_unloaded(&self) -> bool {
        self.infos.is_empty()
    }

    /// Whether the loader holds the hull: queued, loading or loaded, and not asked to unload.
    pub(crate) fn holds_hull(&self, id: u32) -> bool {
        self.infos.get(&id).is_some_and(|info| !info.request_unload)
    }

    /// The hulls whose collections the presentation holds (`NotifyCollectionReg` registers
    /// the simulation collections with it too): from `LoadingPresCollections` until
    /// `StartStreamCollectionUnload` (`0x82eed290`) deregisters them, in hull ID order.
    pub(crate) fn presentations(&self) -> Vec<crate::presentation::HullPresentation> {
        self.infos
            .iter()
            .filter(|(_, info)| {
                matches!(
                    info.state,
                    State::LoadingPresCollections
                        | State::LoadingTextureCollections
                        | State::LoadCompleteNotify
                        | State::LoadComplete
                        | State::UnloadNotStarted
                )
            })
            .filter(|(_, info)| !info.streams[1].collections.is_empty() || !info.streams[0].collections.is_empty())
            .map(|(&hull, info)| crate::presentation::HullPresentation {
                hull,
                file_name: info.file_name.clone(),
                collections: info.streams[1].collections.clone(),
                sim_collections: info.streams[0].collections.clone(),
            })
            .collect()
    }

    /// `cHullLoader::UnloadHull` (`0x82eee9c0`): an unknown or unloaded hull
    /// answers a callback at the next update; one not yet started ends at once.
    pub(crate) fn unload_hull(&mut self, id: u32, callback: bool) {
        match self.infos.get_mut(&id) {
            // IsHullUnloaded (0x82eed9f0).
            None => {
                if callback {
                    self.duplicates.push(id);
                }
            }
            Some(info) if info.state == State::UnloadCompleteNotify => {
                if callback {
                    self.duplicates.push(id);
                }
            }
            Some(info) => {
                info.unload_callback = callback;
                info.request_unload = true;
                if info.state == State::LoadNotStarted {
                    info.state = State::UnloadCompleteNotify;
                }
            }
        }
    }

    /// `StartExistsCheck` (`0x82eecc38`): whether the stream's `.xsm` exists.
    fn exists(&self, file_name: &str, stream: usize) -> Result<bool, String> {
        let paths = crate::game_stream::StreamPaths::derive(CONTENT_STREAM_DIRECTORY, file_name, STREAMS[stream])
            .map_err(|e| format!("hull {file_name} stream path: {e:?}"))?;
        Ok(self.mounts.contains_path(&format!("{}.xsm", paths.manifest_stem)))
    }

    /// One `Update`. The manifests advance first (the stream manager's
    /// update); the result is the hull IDs whose callbacks fire, in order.
    /// A loaded or failed manifest waits for nothing until its hull's
    /// `UnloadStream` (`cGameStream::Update` takes it from there to unloaded).
    pub(crate) fn try_update(&mut self, terrain: &mut SelectedTerrainStartup) -> Result<Vec<u32>, String> {
        for info in self.infos.values_mut() {
            let unloading = info.state == State::UnloadingToc;
            for stream in &mut info.streams {
                if let Some(manifest) = stream.manifest.as_mut() {
                    let state = manifest.state();
                    if (unloading || (state != GameStreamState::LOADED && state != GameStreamState::LOAD_FAILED)) && !manifest.is_released() {
                        manifest.poll(&mut self.backend).map_err(|e| format!("hull {} manifest: {e}", info.file_name))?;
                    }
                }
            }
        }
        let mut completed = Vec::new();
        let mut candidate: Option<u32> = None;
        let ids: Vec<u32> = self.infos.keys().copied().collect();
        for id in ids {
            let Some(state) = self.infos.get(&id).map(|info| info.state) else { continue };
            let next = match state {
                State::LoadNotStarted => {
                    let high_priority = self.infos[&id].high_priority;
                    if self.active_load.is_none() && (candidate.is_none() || high_priority) {
                        candidate = Some(id);
                    }
                    continue;
                }
                State::CheckExistsSim => self.start_stream(id, 0, State::LoadingSimToc)?,
                State::LoadingSimToc if self.toc_done(id, 0) => self.check_exists(id, 1, State::CheckExistsPres)?,
                State::CheckExistsPres => self.start_stream(id, 1, State::LoadingPresToc)?,
                State::LoadingPresToc if self.toc_done(id, 1) => self.check_exists(id, 2, State::CheckExistsTexture)?,
                State::CheckExistsTexture => self.start_stream(id, 2, State::LoadingTextureToc)?,
                State::LoadingTextureToc if self.toc_done(id, 2) => self.start_collections(id, 0, terrain, State::LoadingSimCollections)?,
                State::LoadingSimCollections if self.collections_registered(id, 0, terrain) => {
                    self.start_collections(id, 1, terrain, State::LoadingPresCollections)?
                }
                State::LoadingPresCollections if self.collections_registered(id, 1, terrain) => {
                    self.start_collections(id, 2, terrain, State::LoadingTextureCollections)?
                }
                State::LoadingTextureCollections if self.collections_registered(id, 2, terrain) => State::LoadCompleteNotify,
                State::LoadCompleteNotify => {
                    if self.infos[&id].load_callback {
                        completed.push(id);
                    }
                    State::LoadComplete
                }
                State::LoadComplete => {
                    if self.active_load == Some(id) {
                        self.active_load = None;
                    }
                    if !self.infos[&id].request_unload {
                        continue;
                    }
                    State::UnloadNotStarted
                }
                State::UnloadNotStarted => {
                    if self.active_unload.is_some() {
                        continue;
                    }
                    self.active_unload = Some(id);
                    // StartStreamCollectionUnload (0x82eed290) of every stream; the
                    // presentation drops the presentation collections with this state.
                    for collection in &self.infos[&id].streams[0].collections {
                        terrain.release_hull_collection(*collection);
                    }
                    State::UnloadingCollections
                }
                State::UnloadingCollections => {
                    let info = &self.infos[&id];
                    if !info.streams[0].collections.iter().all(|c| terrain.hull_collection_released(*c)) {
                        continue;
                    }
                    // UnloadStream of each loaded manifest.
                    let info = self.infos.get_mut(&id).ok_or("hull info")?;
                    for stream in &mut info.streams {
                        stream.collections.clear();
                        if let Some(manifest) = stream.manifest.as_mut() {
                            manifest.request_unload().map_err(|e| format!("hull {} unload: {e:?}", info.file_name))?;
                        }
                    }
                    State::UnloadingToc
                }
                State::UnloadingToc => {
                    let info = self.infos.get_mut(&id).ok_or("hull info")?;
                    if !info.streams.iter().all(|stream| stream.manifest.as_ref().is_none_or(Manifest::is_released)) {
                        continue;
                    }
                    for stream in &mut info.streams {
                        stream.manifest = None;
                    }
                    State::UnloadCompleteNotify
                }
                // UnloadCompleteNotify, then UnloadComplete's erase in the same update.
                State::UnloadCompleteNotify => {
                    if self.infos[&id].unload_callback {
                        completed.push(id);
                    }
                    self.active_unload = None;
                    self.infos.remove(&id);
                    continue;
                }
                State::LoadingSimToc
                | State::LoadingPresToc
                | State::LoadingTextureToc
                | State::LoadingSimCollections
                | State::LoadingPresCollections
                | State::LoadingTextureCollections => continue,
            };
            if let Some(info) = self.infos.get_mut(&id) {
                info.state = next;
            }
        }
        if let Some(id) = candidate {
            self.active_load = Some(id);
            let next = self.check_exists(id, 0, State::CheckExistsSim)?;
            if let Some(info) = self.infos.get_mut(&id) {
                info.state = next;
            }
        }
        completed.append(&mut self.duplicates);
        Ok(completed)
    }

    /// `StartExistsCheck` for a stream, answered in the given state.
    fn check_exists(&mut self, id: u32, stream: usize, next: State) -> Result<State, String> {
        let file_name = self.infos[&id].file_name.clone();
        let exists = self.exists(&file_name, stream)?;
        if let Some(info) = self.infos.get_mut(&id) {
            info.streams[stream].exists = exists;
        }
        Ok(next)
    }

    /// `ShouldStartStreamLoad`, then `cAssetStreamManager::LoadStream`.
    fn start_stream(&mut self, id: u32, stream: usize, next: State) -> Result<State, String> {
        let info = self.infos.get_mut(&id).ok_or("hull info")?;
        if info.streams[stream].exists {
            let manifest = ManifestOnlyStream::load(CONTENT_STREAM_DIRECTORY, &info.file_name, STREAMS[stream])
                .map_err(|e| format!("hull {} stream: {e:?}", info.file_name))?;
            info.streams[stream].manifest = Some(manifest);
        }
        Ok(next)
    }

    /// `IsStreamLoaded` or `IsStreamLoadFailed`, or no stream.
    fn toc_done(&self, id: u32, stream: usize) -> bool {
        self.infos[&id].streams[stream]
            .manifest
            .as_ref()
            .is_none_or(|manifest| manifest.state() == GameStreamState::LOADED || manifest.state() == GameStreamState::LOAD_FAILED)
    }

    /// `StartStreamCollectionLoad` (`0x82eed1b0`) of a loaded stream: every
    /// collection of its map. Simulation collections load through the
    /// terrain; presentation collections go to the presentation.
    fn start_collections(&mut self, id: u32, stream: usize, terrain: &mut SelectedTerrainStartup, next: State) -> Result<State, String> {
        let info = self.infos.get_mut(&id).ok_or("hull info")?;
        let Some(map) = info.streams[stream].manifest.as_ref().and_then(Manifest::collection_map) else { return Ok(next) };
        let collections: Vec<u64> = map.collections.iter().map(|collection| collection.id).collect();
        match STREAMS[stream] {
            StreamKind::Simulation => {
                for &collection in &collections {
                    terrain.request_hull_collection(collection, info.asset_flags).map_err(|e| format!("hull {}: {e}", info.file_name))?;
                }
            }
            StreamKind::Presentation => {}
            StreamKind::Texture => return Ok(next),
        }
        info.streams[stream].collections = collections;
        Ok(next)
    }

    /// The stream's collections are registered (`UnregisteredCount` zero); the
    /// presentation's count at once.
    fn collections_registered(&self, id: u32, stream: usize, terrain: &SelectedTerrainStartup) -> bool {
        STREAMS[stream] != StreamKind::Simulation
            || self.infos[&id].streams[stream].collections.iter().all(|c| terrain.hull_collection_registered(*c))
    }
}
