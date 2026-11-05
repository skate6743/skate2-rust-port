//! Retained PROXY collections for the live city renderer: every texture
//! uploaded once, and each spatial collection's geometry arenas owned as a
//! Presentation collection whose registration `proxy_world` drives.
//! Reuses native metadata/collection decoding and the shared processed cache.
use crate::{
    gpu_renderer::{GpuTexture, GpuTextureUploader, RenderResult},
    proxy_kind0_resident_builder::{Kind0ProcessedArena, process_decoded_kind0},
    proxy_shared_loaded_arena::{
        SharedLoadedArenaAsset, SharedLoadedArenaCache, SharedLoadedArenaSessions,
    },
};
use skate2_core::{
    arena_registration_ref_counter::{ArenaRegistrationAddTransition, ArenaRegistrationRefCounter},
    catalog_resident_load_factory::{
        CatalogCollectionAssetMetadata, CollectionAssetBuilderSessionFactory,
    },
    collection_load_pump::CollectionStreamFormat,
    collection_parent_manager::{CollectionParentEffects, CollectionParentManager},
    collection_resident_streamer::{CollectionId, ResidentCollectionStorage},
    decoded_arena_asset::{DecodedArenaAsset, DecodedArenaAssetBuilder},
    game_stream::{StreamFile, StreamFileState, StreamKind},
    mounted_character_scene::own_presentation_arena,
    mounted_stream_backend::{MountedReadHandle, MountedStreamBackend, MountedStreamBackendError},
    owned_resident_collection::{
        OwnedResidentCollection, OwnedResidentCollectionError, OwnedResidentCollectionStorage,
        ResidentAssetIdentity,
    },
    presentation::PresentationCollection,
    resident_collection_load_driver::{
        CollectionAssetBuildInput, CollectionAssetBuilder, CollectionDecompressionWake,
        MountedResidentCollectionLoadDriver, MountedResidentCollectionLoadSpec,
    },
    world_stream_catalog::{
        AssetRegistrationSnapshot, CatalogError, CollectionRegistrationSnapshot, WorldStreamCatalog,
    },
};
use skate2_formats::{
    mount::MountSet,
    pegasus::{TABLE_OF_CONTENTS_TYPE_ID, TableOfContents},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::Infallible,
    io,
    sync::Arc,
};

enum Product {
    Texture(Kind0ProcessedArena),
    Other(DecodedArenaAsset),
}
impl ResidentAssetIdentity for Product {
    fn resident_asset_id(&self) -> CollectionId {
        match self {
            Self::Texture(a) => a.resident_asset_id(),
            Self::Other(a) => CollectionId(a.asset_id),
        }
    }
}
struct Builder(GpuTextureUploader);
impl CollectionAssetBuilderSessionFactory for Builder {
    type Asset = Product;
    type Builder = Self;
    type Error = io::Error;
    fn begin_session(&mut self, _: &CollectionRegistrationSnapshot) -> Result<Self, io::Error> {
        Ok(Self(self.0.clone()))
    }
}
impl CollectionAssetBuilder<AssetRegistrationSnapshot> for Builder {
    type Asset = Product;
    type Error = io::Error;
    fn build(
        &mut self,
        input: CollectionAssetBuildInput<'_, AssetRegistrationSnapshot>,
    ) -> Result<Option<Product>, io::Error> {
        let Some(decoded) = DecodedArenaAssetBuilder
            .build(input)
            .map_err(|e| io::Error::other(e.to_string()))?
        else {
            return Ok(None);
        };
        let mut texture = false;
        for entry in &decoded.arena.dictionary {
            if entry.type_id == TABLE_OF_CONTENTS_TYPE_ID {
                let bytes = decoded
                    .arena
                    .object_bytes(&decoded.resources.resources, entry)
                    .ok_or_else(|| io::Error::other("PROXY TOC range"))?;
                let toc =
                    TableOfContents::parse(bytes).map_err(|e| io::Error::other(e.to_string()))?;
                texture = toc.entries.len() == 1 && toc.entries[0].object_type == 0xac46_2e4a;
                break;
            }
        }
        if texture {
            process_decoded_kind0(decoded, &self.0)
                .map(Product::Texture)
                .map(Some)
                .map_err(|e| io::Error::other(e.to_string()))
        } else {
            Ok(Some(Product::Other(decoded)))
        }
    }
}

// Only the existing ancestor planner is used here. No synthetic callbacks.
pub(crate) struct Planning;
impl CollectionParentEffects for Planning {
    type Error = Infallible;
    fn simulation_register(&mut self, _: CollectionId) -> Result<(), Infallible> {
        unreachable!()
    }
    fn presentation_register(&mut self, _: CollectionId) -> Result<(), Infallible> {
        unreachable!()
    }
    fn simulation_defer_update(&mut self) -> Result<(), Infallible> {
        unreachable!()
    }
    fn simulation_deregister(&mut self, _: CollectionId) -> Result<(), Infallible> {
        unreachable!()
    }
    fn presentation_deregister(&mut self, _: CollectionId) -> Result<(), Infallible> {
        unreachable!()
    }
}

/// One spatial PROXY collection: its non-texture arenas as a Presentation
/// collection, and those arenas' asset IDs in the same order.
pub(crate) struct ProxyCollection {
    pub(crate) presentation: Arc<PresentationCollection>,
    pub(crate) drawn_arenas: Vec<u64>,
}

/// Every spatial PROXY collection's owned geometry, and every arena of its
/// container in load order (the `PresThreadMsgMan` ref-count walk).
#[derive(Default)]
pub(crate) struct ProxyGeometry {
    pub(crate) collections: BTreeMap<u64, ProxyCollection>,
    pub(crate) containers: BTreeMap<u64, Vec<u64>>,
}

/// A loaded proxy stream file, with what unloads it from the catalog.
type LoadedStream = (StreamFile<MountedReadHandle, MountedStreamBackendError<CatalogError>>, Arc<MountSet>, WorldStreamCatalog);

/// How many polls the stream's unload gets before it is left registered.
const UNLOAD_POLLS: usize = 4096;

pub(crate) struct ProxyTextures {
    textures: BTreeMap<u64, Arc<Product>>,
    collections: Vec<OwnedResidentCollection<SharedLoadedArenaAsset<Product>>>,
    registration_ids: Vec<CollectionId>,
    registrations: ArenaRegistrationRefCounter,
    stream: Option<LoadedStream>,
}
impl ProxyTextures {
    /// `order` is `mProxyCollectionID` (`proxy_world_manager`), the order
    /// `SyncLoadProxyWorldStream` 0x824df648 requests the collections in.
    pub(crate) fn load(
        mounts: Arc<MountSet>,
        catalog: WorldStreamCatalog,
        uploader: GpuTextureUploader,
        order: &[u64],
        stream_name: &str,
    ) -> RenderResult<(Self, ProxyGeometry)> {
        let mut backend = MountedStreamBackend::new(Arc::clone(&mounts), catalog.clone())?;
        let mut stream = StreamFile::load(
            "data\\content\\world\\stream",
            stream_name,
            StreamKind::Presentation,
        )?;
        while stream.state() != StreamFileState::LOAD_DONE {
            stream
                .poll(&mut backend)
                .map_err(|e| format!("PROXY metadata: {e:?}"))?;
            if let Some(stage) = stream.reference_read_failure() {
                return Err(format!("PROXY metadata read failed: {stage:?}").into());
            }
            std::thread::yield_now();
        }
        let file = catalog
            .file(stream.registration_id())?
            .ok_or("PROXY catalog owner absent")?;
        let map = file.collection_map.ok_or("PROXY XSM absent")?;
        let space = file.collection_space.ok_or("PROXY XSS absent")?;
        let mut parents = CollectionParentManager::new(Planning);
        for row in &map.collections {
            parents.add_parent(CollectionId(row.id), CollectionId(row.parent_id));
        }
        // The manager read this same XSS before the world stream started.
        let rows = space
            .collections
            .iter()
            .map(|row| (row.id, row))
            .collect::<BTreeMap<_, _>>();
        if rows.len() != order.len() || order.iter().any(|id| !rows.contains_key(id)) {
            return Err("PROXY XSS differs from the proxy manager's collections".into());
        }
        let mut sessions =
            SharedLoadedArenaSessions::new(Builder(uploader), SharedLoadedArenaCache::new());
        let mut storage = OwnedResidentCollectionStorage::new(
            std::convert::identity::<OwnedResidentCollectionError>,
        );
        let mut loaded = BTreeMap::new();
        let mut result = Self {
            textures: BTreeMap::new(),
            collections: Vec::new(),
            registration_ids: Vec::new(),
            registrations: ArenaRegistrationRefCounter::default(),
            stream: None,
        };
        for &spatial in order {
            for request in parents.plan_load(CollectionId(spatial))? {
                if loaded.contains_key(&request.id.0) {
                    continue;
                }
                let row = catalog
                    .collection(request.id.0)?
                    .ok_or("PROXY planned collection absent")?;
                let count = i32::try_from(row.asset_count)?;
                let collection = storage.create_empty(count)?;
                let writer = storage.writer(&collection)?;
                let wake = CollectionDecompressionWake::default();
                let builder = sessions.begin_session(&row)?;
                let mut driver = MountedResidentCollectionLoadDriver::new(
                    Arc::clone(&mounts),
                    MountedResidentCollectionLoadSpec {
                        path: Arc::clone(&row.path),
                        payload_offset: row.offset,
                        request_id: 0,
                        target_assets: count,
                        format: CollectionStreamFormat(i32::try_from(row.stream_format)?),
                    },
                    CatalogCollectionAssetMetadata::new(catalog.clone()),
                    writer,
                    builder,
                    wake.clone(),
                )
                .map_err(|e| format!("PROXY collection start: {e}"))?;
                while !driver.pump().is_complete() {
                    driver
                        .update_one_tick()
                        .map_err(|e| format!("PROXY collection update: {e}"))?;
                    let _ = wake.request();
                    std::thread::yield_now();
                }
                drop(driver);
                loaded.insert(request.id.0, result.collections.len());
                result.collections.push(collection);
            }
        }
        // After all loads, publish each texture on its first Presentation use.
        for collection in &result.collections {
            collection
                .resident_assets_view()
                .try_for_each::<io::Error>(|asset| {
                    let id = asset.resident_asset_id();
                    result.registration_ids.push(id);
                    if result.registrations.add_asset(id)
                        == ArenaRegistrationAddTransition::FirstUse
                    {
                        if let Product::Texture(texture) = asset.product() {
                            let guid = texture.alias.texture_asset_id;
                            if result
                                .textures
                                .insert(guid, asset.retained_product())
                                .is_some()
                            {
                                return Err(io::Error::other(
                                    "distinct PROXY arenas export the same Texture GUID",
                                ));
                            }
                        }
                    }
                    Ok(())
                })
                .map_err(|e| format!("PROXY texture registration: {e:?}"))?;
        }
        // Each spatial collection's non-texture arenas become its
        // Presentation collection, with the materials' Attribulator bindings.
        let mut owned = Vec::with_capacity(order.len());
        let mut keys = BTreeSet::new();
        for &spatial in order {
            let collection = &result.collections[loaded[&spatial]];
            let mut container = Vec::new();
            let mut drawn = Vec::new();
            let mut arenas = Vec::new();
            collection
                .resident_assets_view()
                .try_for_each::<io::Error>(|asset| {
                    let id = asset.resident_asset_id().0;
                    container.push(id);
                    if let Product::Other(decoded) = asset.product() {
                        arenas.push(
                            own_presentation_arena(&decoded.arena, &decoded.resources.resources)
                                .map_err(|e| io::Error::other(e.to_string()))?,
                        );
                        drawn.push(id);
                    }
                    Ok(())
                })
                .map_err(|e| format!("PROXY collection {spatial:016x} geometry: {e:?}"))?;
            keys.extend(
                arenas
                    .iter()
                    .flat_map(|arena| &arena.materials)
                    .filter_map(|material| material.effect.as_ref())
                    .filter_map(|effect| effect.attribute_class_id.zip(effect.material_variant_id)),
            );
            let name = map
                .collections
                .iter()
                .find(|row| row.id == spatial)
                .ok_or("PROXY spatial collection absent from its XSM")?
                .filename
                .clone();
            let row = rows[&spatial];
            let position = std::array::from_fn(|axis| {
                (row.hull_max[axis] - row.hull_min[axis]).mul_add(0.5, row.hull_min[axis])
            });
            let presentation = PresentationCollection {
                light_probes: Vec::new(),
        navigation: Vec::new(),
                id: spatial,
                name,
                position,
                distance_xz: 0.0,
                arenas,
            };
            owned.push((spatial, container, drawn, presentation));
        }
        let bindings = crate::load_material_attribute_bindings(&mounts, &keys)?;
        let mut geometry = ProxyGeometry::default();
        for (spatial, container, drawn_arenas, mut presentation) in owned {
            for effect in presentation
                .arenas
                .iter_mut()
                .flat_map(|arena| &mut arena.materials)
                .filter_map(|material| material.effect.as_mut())
            {
                let Some(key) = effect.attribute_class_id.zip(effect.material_variant_id) else {
                    continue;
                };
                effect.binding = Some(
                    bindings
                        .get(&key)
                        .ok_or("PROXY material has no attribute binding")?
                        .clone(),
                );
            }
            geometry.containers.insert(spatial, container);
            geometry.collections.insert(
                spatial,
                ProxyCollection {
                    presentation: Arc::new(presentation),
                    drawn_arenas,
                },
            );
        }
        eprintln!(
            "PROXY retained collections={} unique texture uploads={} spatial geometry={}",
            result.collections.len(),
            result.textures.len(),
            geometry.collections.len()
        );
        drop(backend);
        result.stream = Some((stream, mounts, catalog));
        Ok((result, geometry))
    }
    pub(crate) fn entries(&self) -> impl Iterator<Item = (u64, &GpuTexture)> {
        self.textures
            .iter()
            .filter_map(|(&guid, product)| match product.as_ref() {
                Product::Texture(texture) => Some((guid, &texture.alias.gpu)),
                _ => None,
            })
    }
    pub(crate) fn texture(&self, guid: u64) -> Option<&GpuTexture> {
        match self.textures.get(&guid)?.as_ref() {
            Product::Texture(a) => Some(&a.alias.gpu),
            _ => None,
        }
    }
}
impl Drop for ProxyTextures {
    fn drop(&mut self) {
        for id in self.registration_ids.drain(..) {
            let _ = self.registrations.remove_asset(id);
        }
        self.textures.clear();
        self.collections.clear();
        // `cStreamFile::Unload`: the stream's collections, asset TOC and file leave the catalog,
        // so another stream naming the same collections (a linked Skate 3 data folder's other
        // districts' proxies) can register them.
        if let Some((mut stream, mounts, catalog)) = self.stream.take()
            && stream.request_unload().is_ok()
            && let Ok(mut backend) = MountedStreamBackend::new(mounts, catalog)
        {
            for _ in 0..UNLOAD_POLLS {
                if stream.state() == StreamFileState::UNLOAD_DONE || stream.poll(&mut backend).is_err() {
                    break;
                }
                std::thread::yield_now();
            }
        }
    }
}
