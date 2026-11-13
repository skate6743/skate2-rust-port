//! Moving visual inventory using the session's registered stream metadata.
//! Native focus selection is reused. Each generation's high collection loads
//! and unloads drive the PROXY registrations (`proxy_world`); full processed
//! Presentation registration remains separate work. The mission hulls the
//! simulation's hull loaders registered join each generation
//! (`hull_presentation`). A loaded additional world's Pres and Tex files join
//! the selection, as `cAssetStreamSystem::FindCollections` (`0x824dfea8`)
//! selects from every file of a type.
use crate::{
    gpu_renderer::{GpuTextureUploader, RenderResult},
    proxy_texture_world::ProxyTextures,
    proxy_world::{ProxyWorldOwner, SharedProxyWorld},
    render_streams::{RenderStreamFile, RenderStreams},
};
use skate2_core::{
    game::WORLD_STREAM_PATH,
    game_stream::{StreamKind, StreamPaths},
    presentation::{HullPresentation, PresentationScene},
    streaming::{CollectionRequestKind, StreamFocusTransition},
    texture::TextureScene,
    world_stream_catalog::{FileRegistrationSnapshot, WorldStreamCatalog},
};
use skate2_formats::{
    mount::MountSet,
    stream::{
        CollectionSpace, DEFAULT_PRESENTATION_FOCUS_COLLECTIONS, FocusCollection, MAX_FOCUS_ENTRIES,
        TEXTURE_FOCUS_RADIUS, WORLD_FOCUS_RADIUS, merge_focus_collections,
    },
    stream_xml::{StreamXml, TileRecords},
};
use std::{
    collections::BTreeSet,
    sync::{Arc, mpsc},
    thread,
};

struct FocusSource {
    file: RenderStreamFile,
    space: Arc<CollectionSpace>,
    xml: Option<Arc<StreamXml>>,
    /// PC port: a linked Skate 3 world's records, which its selection takes from instead.
    records: Option<Arc<TileRecords>>,
    radius: f32,
}
impl FocusSource {
    fn from_registered(
        catalog: &WorldStreamCatalog,
        seed: u64,
        kind: StreamKind,
        radius: f32,
    ) -> RenderResult<Self> {
        let collection = catalog
            .collection(seed)?
            .ok_or("visual seed collection not registered")?;
        let file = catalog
            .file(&collection.owner)?
            .ok_or("visual seed stream not registered")?;
        Self::from_file(file, kind, radius)
    }
    fn from_file(file: FileRegistrationSnapshot, kind: StreamKind, radius: f32) -> RenderResult<Self> {
        if file.kind != kind {
            return Err("visual seed stream kind mismatch".into());
        }
        let map = file.collection_map.ok_or("visual stream XSM absent")?;
        let space = file.collection_space.ok_or("visual stream XSS absent")?;
        let xml = file
            .stream_xml
            .as_ref()
            .filter(|bytes| !bytes.is_empty())
            .map(|bytes| StreamXml::parse(bytes, kind.suffix(), &space).map(Arc::new))
            .transpose()?;
        Ok(Self {
            file: RenderStreamFile::from_paths(map, &file.paths),
            space,
            xml,
            records: None,
            radius,
        })
    }
    fn select(&self, position: [f32; 3], xml_enabled: bool) -> Vec<FocusCollection> {
        // A Skate 3 world: Skate 3's `FindCollections` (0x8247b3a0 in its title update 3), which
        // no cvar gates.
        if let Some(records) = &self.records {
            return records.focus_collections(&self.space, position);
        }
        // FindCollections824df068 uses a fixed capacity of16 for Pres/Tex.
        // The simulation-specific cvar is not a presentation cap.
        if xml_enabled {
            if let Some(xml) = &self.xml {
                return xml.focus_collections(&self.space, position, self.radius, 16);
            }
        }
        self.space.focus_collections(position, self.radius, 16)
    }
}

/// A loaded additional world's Pres and Tex files (`StartAdditionalWorldStreamLoad`), which the
/// selection takes from with BAM's while the session holds the world loaded. Once it no longer
/// does, they leave the selection and go when the installed generation holds none of their
/// collections, which the session's unload waits for.
struct AdditionalSources {
    world: String,
    pres: FocusSource,
    /// None while texture streaming is off (`kEnableTextureStream`), as for BAM.
    tex: Option<FocusSource>,
    leaving: bool,
}
impl AdditionalSources {
    fn registered(catalog: &WorldStreamCatalog, world: &str) -> RenderResult<Self> {
        let source = |kind: StreamKind, radius: f32| -> RenderResult<Option<FocusSource>> {
            let paths = StreamPaths::derive(WORLD_STREAM_PATH, world, kind)?;
            catalog.file_with_paths(kind, &paths)?.map(|file| FocusSource::from_file(file, kind, radius)).transpose()
        };
        let mut pres = source(StreamKind::Presentation, WORLD_FOCUS_RADIUS)?
            .ok_or_else(|| format!("additional world {world} presentation stream not registered"))?;
        let mut tex = source(StreamKind::Texture, TEXTURE_FOCUS_RADIUS)?;
        // PC port: a linked Skate 3 world selects as Skate 3 does, from its files' records.
        if skate2_core::skate3::is_district_world(world) {
            let records = |source: &FocusSource, kind: StreamKind| Some(Arc::new(skate2_core::skate3::world_records(world, &source.space, kind.suffix())));
            pres.records = records(&pres, StreamKind::Presentation);
            if let Some(tex) = tex.as_mut() {
                tex.records = records(tex, StreamKind::Texture);
            }
        }
        Ok(Self { world: world.to_owned(), pres, tex, leaving: false })
    }
    /// Whether the installed generation holds any of its collections.
    fn held_by(&self, scene: &PresentationScene, textures: &TextureScene) -> bool {
        let listed = |file: &RenderStreamFile, id: u64| file.map.collections.iter().any(|entry| entry.id == id);
        scene.collections.iter().chain(&scene.dependencies).any(|collection| listed(&self.pres.file, collection.id))
            || self.tex.as_ref().is_some_and(|tex| textures.collections.iter().any(|collection| listed(&tex.file, collection.id)))
    }
}

/// `cAssetStreamSystem::FindCollections` (`0x824dfea8`) over a type's files: the first file's
/// selection with the next file's merged in, nearest first, at most `cap`
/// (`kPresMaxFocusCollections` for Pres).
fn merged_selection(first: Vec<FocusCollection>, more: Option<Vec<FocusCollection>>, cap: usize) -> Vec<FocusCollection> {
    let Some(more) = more else { return first };
    let mut selected = merge_focus_collections([first, more]);
    selected.truncate(cap);
    selected
}

pub(crate) type PreparedRenderState = (
    skate2_core::render::NativeRenderQueue,
    crate::render_inspection::DiagnosticTechniques,
);
/// A generation's owners, render state and the proxy revision its scene holds
/// (none before activation).
type Loaded = Result<
    (
        PresentationScene,
        TextureScene,
        PreparedRenderState,
        Option<u64>,
    ),
    String,
>;

/// One generation's high collection registrations and their proxies
/// (`NotifyCollectionPresDReg` 0x82d580d0 / `NotifyCollectionPresReg`
/// 0x82d5c680): the executed unloads re-register their proxies, then the
/// executed loads deregister theirs, in the request queue's order
/// (`move_runtime_presentation_selection` drains every unload before any
/// load). The scene then holds the registered proxies; returns their revision.
fn register_proxies(
    proxy: &SharedProxyWorld,
    active: bool,
    transition: Option<&StreamFocusTransition>,
    scene: &mut PresentationScene,
) -> RenderResult<Option<u64>> {
    if !active {
        return Ok(None);
    }
    let mut owner = proxy.lock().map_err(|_| "PROXY world lock poisoned")?;
    if let Some(transition) = transition {
        for request in &transition.requests {
            if matches!(request.kind, CollectionRequestKind::Unload) {
                owner.high_deregistered(request.id)?;
            }
        }
        for request in &transition.requests {
            if matches!(request.kind, CollectionRequestKind::Load) {
                owner.high_registered(request.id)?;
            }
        }
    }
    scene.proxies = owner.registered_collections();
    Ok(Some(owner.revision()?))
}

/// PC port: a linked Skate 3 data folder's district's own proxy world (`<District>_100_Proxy`,
/// `skate2_core::skate3`), drawn in `PROXY`'s place while the district is selected. The districts'
/// proxy streams name the same tiles' proxies (each its own low-detail view of the city), so one
/// is registered at a time: it stays through San Vanelona until another district's replaces it.
/// It follows every high collection the scene registers from the scene it started with, as
/// `PROXY` does.
struct DistrictProxy {
    stream: &'static str,
    proxy: SharedProxyWorld,
    /// Its revision when the installed scene took its proxies; none before.
    revision: Option<u64>,
}

/// A Skate 3 high collection's tile (`cPres_<x>_<z>_high`), which names its proxy.
fn high_collection_tile(name: &str) -> Option<(i32, i32)> {
    let name = name.rsplit(['\\', '/']).next()?.trim_end_matches(".xsf");
    let (x, z) = name.strip_prefix("cPres_")?.strip_suffix("_high")?.split_once('_')?;
    Some((x.parse().ok()?, z.parse().ok()?))
}

pub(crate) struct CityStreaming {
    pub(crate) mounts: Arc<MountSet>,
    pub(crate) catalog: WorldStreamCatalog,
    proxy: SharedProxyWorld,
    /// Proxy revision of the installed scene; none before activation.
    proxy_revision: Option<u64>,
    sources: Option<(FocusSource, FocusSource)>,
    additional: Option<AdditionalSources>,
    district_proxy: Option<DistrictProxy>,
    district_proxy_load: Option<DistrictProxyLoad>,
    pending: Option<mpsc::Receiver<Loaded>>,
}

/// A district proxy world loaded off the render thread: the stream file, its high collections'
/// tiles and its geometry and textures. Activation waits for the installed scene.
type DistrictProxyLoaded = (SharedProxyWorld, ProxyTextures, crate::proxy_texture_world::ProxyGeometry);
enum DistrictProxyLoad {
    Loading(&'static str, mpsc::Receiver<Result<DistrictProxyLoaded, String>>),
    Ready(&'static str, DistrictProxyLoaded),
}

impl CityStreaming {
    pub(crate) fn is_pending(&self) -> bool { self.pending.is_some() }
    pub(crate) fn new(
        mounts: Arc<MountSet>,
        catalog: WorldStreamCatalog,
        proxy: SharedProxyWorld,
    ) -> Self {
        Self {
            mounts,
            catalog,
            proxy,
            proxy_revision: None,
            sources: None,
            additional: None,
            district_proxy: None,
            district_proxy_load: None,
            pending: None,
        }
    }

    /// The selected Skate 3 district's loaded proxy world.
    fn selected_district_proxy(&self) -> Option<&DistrictProxy> {
        let wanted = self.exclusive().and_then(|world| skate2_core::skate3::district_spec(&world.world))?.proxy;
        self.district_proxy.as_ref().filter(|district| district.stream == wanted)
    }

    /// PC port: the selected Skate 3 district's proxy world, loaded when the district is selected
    /// and another's (or none) is: the other's world and `textures` go first, its stream unloaded
    /// from the catalog, then its `<District>_100_Proxy` loads (geometry and textures, under the
    /// travel's fade), the district's high collections mapped onto it by tile and the installed
    /// scene's registered. Nothing while a generation is loading.
    pub(crate) fn sync_district_proxy(
        &mut self,
        uploader: GpuTextureUploader,
        scene: Option<&PresentationScene>,
        textures: &mut Option<ProxyTextures>,
    ) -> RenderResult<()> {
        // A downloadable content map has no proxy world: no proxy draws there.
        let Some(stream) =
            self.exclusive().and_then(|world| skate2_core::skate3::district_spec(&world.world)).map(|spec| spec.proxy).filter(|proxy| !proxy.is_empty())
        else {
            return Ok(());
        };
        if self.district_proxy.as_ref().is_some_and(|district| district.stream == stream) {
            return Ok(());
        }
        if let Some(DistrictProxyLoad::Loading(loading, receive)) = &self.district_proxy_load {
            let loading = *loading;
            match receive.try_recv() {
                Ok(result) => {
                    self.district_proxy_load = (loading == stream).then(|| result.map(|loaded| DistrictProxyLoad::Ready(loading, loaded))).transpose()?;
                }
                Err(mpsc::TryRecvError::Empty) => return Ok(()),
                Err(mpsc::TryRecvError::Disconnected) => return Err("district proxy loader stopped without completion".into()),
            }
        }
        let Some(scene) = scene.filter(|_| self.pending.is_none()) else { return Ok(()) };
        if !matches!(&self.district_proxy_load, Some(DistrictProxyLoad::Ready(ready, _)) if *ready == stream) {
            // The geometry and textures load on a worker (under the travel's fade); the render
            // thread keeps presenting meanwhile.
            self.district_proxy = None;
            *textures = None;
            let tiles: Vec<(u64, i32, i32)> = self.exclusive().map_or_else(Vec::new, |world| {
                world.pres.file.map.collections.iter()
                    .filter_map(|entry| high_collection_tile(&entry.filename).map(|(x, z)| (entry.id, x, z)))
                    .collect()
            });
            let mounts = Arc::clone(&self.mounts);
            // The district's proxy registers in a catalog of its own. The Skate 2 PROXY world stays
            // registered in the shared one for the whole session, and an asset ID both games'
            // proxies carry keeps its first (Skate 2) metadata there, so a district proxy loaded
            // after starting on the Skate 2 map read Skate 2's resource sizes for Skate 3's data.
            let catalog = WorldStreamCatalog::new();
            let (send, receive) = mpsc::sync_channel(1);
            thread::Builder::new()
                .name("district-proxy-loader".into())
                .spawn(move || {
                    let result = (|| -> RenderResult<DistrictProxyLoaded> {
                        let shared = ProxyWorldOwner::load_shared_stream(&mounts, stream)?;
                        let order = {
                            let mut owner = shared.lock().map_err(|_| "district proxy world lock poisoned")?;
                            // SKATE2_DIAGNOSTIC_DISTRICT_PROXIES_SHOWN: no high tile hides a proxy, so
                            // every district proxy stays drawn (a comparison for holes in the distance).
                            let shown = std::env::var_os("SKATE2_DIAGNOSTIC_DISTRICT_PROXIES_SHOWN").is_some();
                            let trace = std::env::var_os("SKATE2_TRACE_DISTRICT_PROXY").is_some();
                            for &(id, x, z) in &tiles {
                                if !shown {
                                    owner.add_high_collection(id, x, z)?;
                                }
                                if trace {
                                    let proxy = owner.proxy_of(id).map_or_else(|| "none".to_owned(), |proxy| format!("{proxy:016x}"));
                                    eprintln!("{stream} tile ({x},{z}) high {id:016x} hides proxy {proxy}");
                                }
                            }
                            owner.collection_ids()
                        };
                        let (loaded, geometry) = ProxyTextures::load(mounts, catalog, uploader, &order, stream)?;
                        Ok((shared, loaded, geometry))
                    })()
                    .map_err(|error| error.to_string());
                    let _ = send.send(result);
                })?;
            self.district_proxy_load = Some(DistrictProxyLoad::Loading(stream, receive));
            return Ok(());
        }
        let Some(DistrictProxyLoad::Ready(_, (shared, loaded, geometry))) = self.district_proxy_load.take() else {
            unreachable!("checked ready above")
        };
        {
            let mut owner = shared.lock().map_err(|_| "district proxy world lock poisoned")?;
            owner.activate(geometry)?;
            for collection in &scene.collections {
                owner.high_registered(collection.id)?;
            }
            eprintln!("{stream} activated: {} registered proxies", owner.registered_collections().len());
        }
        self.district_proxy = Some(DistrictProxy { stream, proxy: shared, revision: None });
        *textures = Some(loaded);
        Ok(())
    }

    /// The additional world whose collections the presentation may hold: the one it selects
    /// from, or one leaving until the installed generation holds none of its collections.
    pub(crate) fn additional_world(&self) -> Option<&str> {
        self.additional.as_ref().map(|world| world.world.as_str())
    }

    /// The additional world the selection takes from.
    pub(crate) fn selected_world(&self) -> Option<&str> {
        self.selectable().map(|world| world.world.as_str())
    }

    fn selectable(&self) -> Option<&AdditionalSources> {
        self.additional.as_ref().filter(|world| !world.leaving)
    }

    /// PC port: the selected additional world when it is a linked Skate 3 district, which stands
    /// where San Vanelona does: the selections take from it alone and BAM's proxies are not drawn.
    fn exclusive(&self) -> Option<&AdditionalSources> {
        self.selectable().filter(|world| skate2_core::skate3::is_district_world(&world.world))
    }

    /// PC port: the Pres and Tex collections of a linked Skate 3 district streaming in or still
    /// leaving, and its proxy world's, which draw Skate 3's world programs with Skate 3's texture
    /// IDs whatever world the session stands in this frame.
    pub(crate) fn skate3_collections(&self) -> BTreeSet<u64> {
        let mut collections: BTreeSet<u64> = self
            .additional
            .iter()
            .filter(|world| skate2_core::skate3::is_district_world(&world.world))
            .flat_map(|world| {
                let tex = world.tex.iter().flat_map(|tex| tex.file.map.collections.iter());
                world.pres.file.map.collections.iter().chain(tex).map(|entry| entry.id)
            })
            .collect();
        if let Some(district) = &self.district_proxy
            && let Ok(proxy) = district.proxy.lock()
        {
            collections.extend(proxy.collection_ids());
        }
        // And its loose sky and water (`crate::sync_loose_world`).
        if let Some(spec) = self.exclusive().and_then(|world| skate2_core::skate3::district_spec(&world.world)) {
            collections.insert(crate::loose_world_id(spec.loose));
        }
        collections
    }

    /// The Pres and Tex selections at `position` over every file.
    fn select(&self, position: [f32; 3], xml_enabled: bool) -> Option<(Vec<FocusCollection>, Vec<FocusCollection>)> {
        let (pres, tex) = self.sources.as_ref()?;
        if let Some(district) = self.exclusive() {
            let mut selected = district.pres.select(position, xml_enabled);
            // Skate 3's records hold at most 64, which it keeps whole.
            if district.pres.records.is_none() {
                selected.truncate(DEFAULT_PRESENTATION_FOCUS_COLLECTIONS);
            }
            let textures = district.tex.as_ref().map_or_else(Vec::new, |tex| tex.select(position, xml_enabled));
            return Some((selected, textures));
        }
        let more = self.selectable();
        Some((
            merged_selection(
                pres.select(position, xml_enabled),
                more.map(|world| world.pres.select(position, xml_enabled)),
                DEFAULT_PRESENTATION_FOCUS_COLLECTIONS,
            ),
            merged_selection(
                tex.select(position, xml_enabled),
                more.and_then(|world| world.tex.as_ref()).map(|tex| tex.select(position, xml_enabled)),
                MAX_FOCUS_ENTRIES,
            ),
        ))
    }

    /// Whether the texture selection at `position` is empty, as it can be at an additional
    /// world's location, whose textures its Pres files carry.
    pub(crate) fn texture_selection_empty(&self, position: [f32; 3], xml_enabled: bool) -> bool {
        self.select(position, xml_enabled).is_some_and(|(_, tex)| tex.is_empty())
    }

    /// Whether the presentation selection at `position` is empty, as it is in a Skate 3 world
    /// where the tile holds no collection.
    pub(crate) fn presentation_selection_empty(&self, position: [f32; 3], xml_enabled: bool) -> bool {
        self.select(position, xml_enabled).is_some_and(|(pres, _)| pres.is_empty())
    }

    /// Follows the session's loaded additional world (`world`).
    fn sync_additional(
        &mut self,
        world: Option<&str>,
        scene: Option<&PresentationScene>,
        textures: Option<&TextureScene>,
    ) -> RenderResult<()> {
        if let Some(held) = self.additional.as_mut()
            && world != Some(held.world.as_str())
        {
            held.leaving = true;
        }
        if let Some(held) = self.additional.as_ref().filter(|held| held.leaving)
            && let (Some(scene), Some(textures)) = (scene, textures)
            && !held.held_by(scene, textures)
        {
            eprintln!("city streaming: additional world {} released", held.world);
            self.additional = None;
        }
        if let Some(world) = world
            && self.additional.is_none()
        {
            self.additional = Some(AdditionalSources::registered(&self.catalog, world)?);
            eprintln!("city streaming: additional world {world} joins the selection");
        }
        Ok(())
    }

    /// Loads the PROXY collections in `mProxyCollectionID` order (texture
    /// uploads need the renderer), activates them (`SyncActivateProxyWorld`
    /// 0x824d9ef8), then registers `scene`'s resident high collections, whose
    /// proxies deregister, and hands `scene` the registered proxies.
    /// The presentation stream for the session's `IsAreaLoaded`, with the high collections
    /// `scene` registered; none before the first focus move picked the stream.
    pub(crate) fn presentation_area(
        &self,
        scene: &PresentationScene,
    ) -> Option<skate2_core::streaming::area_loaded::PresentationStreamArea> {
        let (pres, _) = self.sources.as_ref()?;
        Some(skate2_core::streaming::area_loaded::PresentationStreamArea {
            space: Arc::clone(&pres.space),
            xml: pres.xml.clone(),
            additional: self.selectable().map(|world| Arc::clone(&world.pres.space)),
            radius: pres.radius,
            resident: scene.collections.iter().map(|collection| collection.id).collect(),
            exclusive: self.exclusive().is_some(),
            records: self.exclusive().and_then(|world| world.pres.records.clone()),
        })
    }

    pub(crate) fn replay_focuses(&self,scene:&PresentationScene,textures:&TextureScene)
        ->Option<[skate2_core::streaming::replay_focus::VisualFocus;2]> {
        use skate2_core::streaming::replay_focus::VisualFocus;
        let (pres,tex)=self.sources.as_ref()?;
        // A Skate 3 district stands where San Vanelona does: as `select`, the focuses take from it
        // alone, and without its Tex file the texture focus is empty.
        let (pres,tex,more)=match self.exclusive() {
            Some(district)=>(&district.pres,district.tex.as_ref(),None),
            None=>(pres,Some(tex),self.selectable()),
        };
        let additional=|source:&FocusSource|(Arc::clone(&source.space),Arc::clone(&source.file.map));
        let texture_focus=match tex {
            Some(tex)=>VisualFocus {space:Arc::clone(&tex.space),map:Arc::clone(&tex.file.map),xml:tex.xml.clone(),
                additional:more.and_then(|world|world.tex.as_ref()).map(additional),records:tex.records.clone(),radius:tex.radius,
                resident:textures.collections.iter().map(|c|c.id).collect(),texture:true},
            None=>VisualFocus {space:Arc::default(),map:Arc::default(),xml:None,additional:None,records:None,radius:0.,resident:Vec::new(),texture:true},
        };
        Some([
            VisualFocus {space:Arc::clone(&pres.space),map:Arc::clone(&pres.file.map),xml:pres.xml.clone(),
                additional:more.map(|world|additional(&world.pres)),records:pres.records.clone(),radius:pres.radius,
                resident:scene.collections.iter().map(|c|c.id).collect(),texture:false},
            texture_focus,
        ])
    }

    pub(crate) fn activate_proxy_world(
        &mut self,
        uploader: GpuTextureUploader,
        scene: &mut PresentationScene,
    ) -> RenderResult<ProxyTextures> {
        let order = self
            .proxy
            .lock()
            .map_err(|_| "PROXY world lock poisoned")?
            .collection_ids();
        let (textures, geometry) = ProxyTextures::load(
            Arc::clone(&self.mounts),
            self.catalog.clone(),
            uploader,
            &order,
            "PROXY",
        )?;
        let mut owner = self.proxy.lock().map_err(|_| "PROXY world lock poisoned")?;
        owner.activate(geometry)?;
        for collection in &scene.collections {
            owner.high_registered(collection.id)?;
        }
        scene.proxies = owner.registered_collections();
        self.proxy_revision = Some(owner.revision()?);
        eprintln!(
            "PROXY world activated: {} registered proxies beside {} high collections",
            scene.proxies.len(),
            scene.collections.len()
        );
        Ok(textures)
    }

    /// Transfer the existing CPU owners to one loader while the immutable GPU
    /// draw generation stays live. No scene cloning or concurrent second loader.
    /// Return prepared render state only when a complete replacement is available.
    pub(crate) fn update(
        &mut self,
        scene: &mut Option<PresentationScene>,
        textures: &mut Option<TextureScene>,
        position: [f32; 3],
        disabled: bool,
        xml_enabled: bool,
        hulls: &[HullPresentation],
        additional_world: Option<&str>,
    ) -> RenderResult<Option<PreparedRenderState>> {
        if let Some(pending) = &self.pending {
            match pending.try_recv() {
                Ok(result) => {
                    self.pending = None;
                    let (loaded_scene, loaded_textures, render_state, proxy_revision) = result?;
                    *scene = Some(loaded_scene);
                    *textures = Some(loaded_textures);
                    self.proxy_revision = proxy_revision;
                    for district in &mut self.district_proxy {
                        district.revision = Some(district.proxy.lock().map_err(|_| "district proxy world lock poisoned")?.revision()?);
                    }
                    return Ok(Some(render_state));
                }
                Err(mpsc::TryRecvError::Empty) => return Ok(None),
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err("city loader stopped without completion".into());
                }
            }
        }
        self.sync_additional(additional_world, scene.as_ref(), textures.as_ref())?;
        // A proxy registration change without a focus change (a mapping that
        // arrived after its high collection registered) still needs a new
        // draw list.
        let proxy_changed = match self.proxy_revision {
            Some(built) => {
                self.proxy
                    .lock()
                    .map_err(|_| "PROXY world lock poisoned")?
                    .revision()?
                    != built
            }
            None => false,
        };
        let mut district_changed = false;
        for district in &self.district_proxy {
            district_changed |= Some(district.proxy.lock().map_err(|_| "district proxy world lock poisoned")?.revision()?) != district.revision;
        }
        let proxy_changed = proxy_changed || district_changed;
        let focus_enabled = !disabled && !position.iter().any(|x| x.is_nan());
        // A hull registration change also needs a new draw list. The scene holds each hull's
        // presentation collections, then its simulation collections' visual indicators
        // (`hull_presentation::sync`).
        let hulls_changed = scene.as_ref().is_some_and(|scene| {
            !scene.hulls.iter().map(|c| c.id).eq(hulls.iter().flat_map(|hull| hull.collections.iter().chain(&hull.sim_collections).copied()))
        });
        if !focus_enabled && !proxy_changed && !hulls_changed {
            return Ok(None);
        }
        let current_scene = scene.as_mut().ok_or("city scene owner absent")?;
        let current_textures = textures.as_mut().ok_or("city texture owner absent")?;
        let mut moved = None;
        if focus_enabled {
            if self.sources.is_none() {
                self.sources = Some((
                    FocusSource::from_registered(
                        &self.catalog,
                        current_scene
                            .collections
                            .first()
                            .ok_or("city has no presentation seed")?
                            .id,
                        StreamKind::Presentation,
                        WORLD_FOCUS_RADIUS,
                    )?,
                    FocusSource::from_registered(
                        &self.catalog,
                        current_textures
                            .collections
                            .first()
                            .ok_or("city has no texture seed")?
                            .id,
                        StreamKind::Texture,
                        TEXTURE_FOCUS_RADIUS,
                    )?,
                ));
            }
            let (selected_pres, selected_tex) = self.select(position, xml_enabled).expect("sources initialized above");
            let ids = |selected: &[FocusCollection]| {
                selected.iter().map(|c| c.id).collect::<BTreeSet<_>>()
            };
            let changed = ids(&selected_pres)
                != current_scene.collections.iter().map(|c| c.id).collect()
                || ids(&selected_tex)
                    != current_textures.collections.iter().map(|c| c.id).collect();
            if changed {
                let (pres, tex) = self.sources.as_ref().expect("sources initialized above");
                let more = self.selectable();
                let streams = |first: &FocusSource, more: Option<&FocusSource>| RenderStreams {
                    files: std::iter::once(first.file.clone()).chain(more.map(|source| source.file.clone())).collect(),
                };
                moved = Some((
                    selected_pres,
                    selected_tex,
                    streams(pres, more.map(|world| &world.pres)),
                    streams(tex, more.and_then(|world| world.tex.as_ref())),
                ));
            } else {
                // Do not reorder index-based handles just because distances change.
                current_scene.stream_focus.move_to(position, false);
                current_scene.stream_focus.reconcile(&selected_pres);
                current_scene.focus = position;
                current_textures.stream_focus.move_to(position, false);
                current_textures.stream_focus.reconcile(&selected_tex);
                current_textures.focus = position;
                if !proxy_changed && !hulls_changed {
                    return Ok(None);
                }
            }
        }
        let mut scene_owner = scene.take().expect("scene checked above");
        let mut texture_owner = textures.take().expect("textures checked above");
        let mounts = Arc::clone(&self.mounts);
        let proxy = Arc::clone(&self.proxy);
        let proxy_active = self.proxy_revision.is_some();
        let exclusive = self.exclusive().is_some();
        let loose_world = self.exclusive().and_then(|world| skate2_core::skate3::district_spec(&world.world));
        let district_proxies: Vec<SharedProxyWorld> = self.district_proxy.iter().map(|district| Arc::clone(&district.proxy)).collect();
        let selected_proxy = self.selected_district_proxy().map(|district| Arc::clone(&district.proxy));
        let hulls = hulls.to_vec();
        let (send, receive) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("city-resource-loader".into())
            .spawn(move || {
                let started = std::time::Instant::now();
                let mut steps: Vec<(&str, f64)> = Vec::new();
                let mut mark = |name: &'static str, from: &mut std::time::Instant| {
                    steps.push((name, from.elapsed().as_secs_f64() * 1000.));
                    *from = std::time::Instant::now();
                };
                let result = (|| -> RenderResult<_> {
                    let mut step = std::time::Instant::now();
                    let mut transition = None;
                    if let Some((selected_pres, selected_tex, pres_streams, tex_streams)) = moved {
                        transition = Some(crate::move_runtime_presentation_selection(
                            &mut scene_owner,
                            &mounts,
                            position,
                            &pres_streams,
                            &selected_pres,
                        )?);
                        mark("presentation", &mut step);
                        crate::move_runtime_texture_selection(
                            &mut texture_owner,
                            &mounts,
                            position,
                            &tex_streams,
                            &selected_tex,
                        )?;
                        scene_owner.bind_resident_textures(&texture_owner);
                        mark("textures", &mut step);
                    }
                    if crate::hull_presentation::sync(&mut scene_owner, &mounts, &hulls)? {
                        crate::bind_runtime_material_attributes(&mounts, &mut scene_owner)?;
                        scene_owner.bind_resident_textures(&texture_owner);
                    }
                    mark("hulls", &mut step);
                    // PC port: a Skate 3 district's own sky and water stand for San Vanelona's.
                    if crate::sync_loose_world(&mut scene_owner, &mounts, loose_world) {
                        crate::bind_runtime_material_attributes(&mounts, &mut scene_owner)?;
                        scene_owner.bind_resident_textures(&texture_owner);
                    }
                    mark("loose world", &mut step);
                    let proxy_revision = register_proxies(
                        &proxy,
                        proxy_active,
                        transition.as_ref(),
                        &mut scene_owner,
                    )?;
                    // A Skate 3 district stands where San Vanelona does: its own proxies, none of
                    // BAM's. The loaded district's proxy world follows every registration as
                    // `PROXY` does; its registered proxies are drawn while it is selected.
                    for district in &district_proxies {
                        register_proxies(district, true, transition.as_ref(), &mut scene_owner)?;
                    }
                    if exclusive {
                        scene_owner.proxies = match &selected_proxy {
                            Some(district) => district.lock().map_err(|_| "district proxy world lock poisoned")?.registered_collections(),
                            None => Vec::new(),
                        };
                    } else if !district_proxies.is_empty() {
                        scene_owner.proxies = proxy.lock().map_err(|_| "PROXY world lock poisoned")?.registered_collections();
                    }
                    mark("proxies", &mut step);
                    let render_state =
                        crate::render_inspection::build_render_state(&scene_owner, &mounts)?;
                    mark("render state", &mut step);
                    Ok((scene_owner, texture_owner, render_state, proxy_revision))
                })()
                .map_err(|error| error.to_string());
                if std::env::var_os("SKATE2_TRACE_CITY_STREAM").is_some() {
                    let steps: Vec<String> = steps.iter().map(|(name, ms)| format!("{name} {ms:.1}")).collect();
                    let held = result.as_ref().map_or_else(
                        |_| String::new(),
                        |(scene, textures, ..)| format!(" ({} Pres, {} Tex collections)", scene.collections.len(), textures.collections.len()),
                    );
                    eprintln!("city streaming: loader {:.1} ms{held}: {}", started.elapsed().as_secs_f64() * 1000., steps.join(", "));
                }
                let _ = send.send(result);
            })?;
        self.pending = Some(receive);
        Ok(None)
    }
}
