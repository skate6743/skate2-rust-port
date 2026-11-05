//! Mounted first-local-marquee completion into the SAME supplied player loader.
//! Synchronous host IO fulfills source asset stages; no native thread scheduling
//! or registration completion is implied. All partial owners survive errors.
use super::requests::{LoadTicket, RequestError, SimCharacterRequests};
use super::{
    DEFAULT_MARQUEE_ID, MarqueeDefinition, PlayerLoadAction, PlayerLoadPhase, PlayerLoaderError,
    PlayerLoaderSlot,
    attributes_runtime::{PlayerAttributeServices, PlayerAttributesError},
};
use crate::{
    attribute_runtime::AttributeRuntimeError,
    character::CharacterScene,
    character_face_manager::{CharacterFaceError, CharacterFaceManager},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
};
use skate2_formats::{
    mount::{MountError, MountSet},
    recipe::{CharacterRecipe, CharacterRecipeError},
};
use std::{collections::BTreeMap, sync::Arc};
#[path = "../assets/character_recipe_finalize.rs"]
pub(crate) mod finalize;
use finalize::{FinalizedRecipeScene, RecipeFinalizeError};

/// A selected load's work made ahead of it: on a loader thread for a marquee's spawn, or
/// before a created character's replacement releases the old load. The recipe, its finalized
/// scene, and the face (its name and `anim.big` bytes) when it could be read.
pub(crate) struct PreparedRecipe {
    pub(crate) recipe: Arc<CharacterRecipe>,
    pub(crate) scene: Arc<FinalizedRecipeScene>,
    pub(crate) face: Option<(String, Arc<[u8]>)>,
}

struct RegisteredRecipe {
    recipe: Arc<CharacterRecipe>,
    scene: Option<Arc<CharacterScene>>,
    references: u32,
    ambient_assets: Option<Arc<ambient::AmbientRecipeAssets>>,
}
#[path = "../living_world/assets/living_world_mounted_recipe.rs"]
pub(crate) mod ambient;
enum RegisteredAsset {
    Recipe(RegisteredRecipe),
    Unsupported(u32),
}
/// Selected recipe fields of the supplied presentation registry and loaded list.
/// `cAssetRegistry82d65428` constructs an empty map; `CharacterAssetManager82d14308`
/// constructs an empty tracking vector. Other presentation assets must be admitted
/// explicitly; this owner does not claim the complete native registry or manager.
pub(crate) struct MountedRecipeRegistry {
    assets: BTreeMap<u64, RegisteredAsset>,
    tracked: Vec<Arc<CharacterRecipe>>,
}
impl MountedRecipeRegistry {
    pub(crate) fn construct_fields() -> Self {
        Self {
            assets: BTreeMap::new(),
            tracked: Vec::new(),
        }
    }
    pub(crate) fn scene(&self, id: u64) -> Result<&Arc<CharacterScene>, MountedRecipeError> {
        match self.assets.get(&id) {
            Some(RegisteredAsset::Recipe(entry)) => entry
                .scene
                .as_ref()
                .ok_or(MountedRecipeError::Missing("registered scene")),
            Some(RegisteredAsset::Unsupported(asset_type)) => {
                Err(MountedRecipeError::RegistryAssetType {
                    id,
                    asset_type: *asset_type,
                })
            }
            None => Err(MountedRecipeError::RegistryIdentity(id)),
        }
    }
    /// Record a supplied non-recipe namespace occupant instead of fabricating a miss.
    pub(crate) fn retain_other_asset(
        &mut self,
        id: u64,
        asset_type: u32,
    ) -> Result<(), MountedRecipeError> {
        if self.assets.contains_key(&id) {
            return Err(MountedRecipeError::RegistryOccupied(id));
        }
        self.assets
            .insert(id, RegisteredAsset::Unsupported(asset_type));
        Ok(())
    }
    fn require_miss(&self, id: u64) -> Result<(), MountedRecipeError> {
        match self.assets.get(&id) {
            None => Ok(()),
            Some(RegisteredAsset::Recipe(_)) => Err(MountedRecipeError::RecipeCacheHitPending(id)),
            Some(RegisteredAsset::Unsupported(asset_type)) => {
                Err(MountedRecipeError::RegistryAssetType {
                    id,
                    asset_type: *asset_type,
                })
            }
        }
    }
    /// `UpdateRequest`'s `RequestState_LoadSchemaFromFile` (0x82d14b90): a schema the registry
    /// already holds (`GetRegAsset`) gains the request's reference (`++mRefCount`) and the
    /// request goes on from `RequestState_SchemaLoaded` with that entity. A skater's entry is
    /// shared; a LivingWorld one keeps its own completion and is not.
    fn try_share(
        &mut self,
        id: u64,
    ) -> Result<Option<(Arc<CharacterRecipe>, Arc<CharacterScene>)>, MountedRecipeError> {
        match self.assets.get_mut(&id) {
            None => Ok(None),
            Some(RegisteredAsset::Recipe(entry)) => {
                if entry.ambient_assets.is_some() {
                    return Err(MountedRecipeError::RecipeCacheHitPending(id));
                }
                let scene = entry
                    .scene
                    .clone()
                    .ok_or(MountedRecipeError::Missing("registered scene"))?;
                entry.references = entry.references.wrapping_add(1);
                Ok(Some((Arc::clone(&entry.recipe), scene)))
            }
            Some(RegisteredAsset::Unsupported(asset_type)) => {
                Err(MountedRecipeError::RegistryAssetType {
                    id,
                    asset_type: *asset_type,
                })
            }
        }
    }
    fn publish_parsed(
        &mut self,
        id: u64,
        recipe: Arc<CharacterRecipe>,
    ) -> Result<(), MountedRecipeError> {
        self.require_miss(id)?;
        self.assets.insert(
            id,
            RegisteredAsset::Recipe(RegisteredRecipe {
                recipe: Arc::clone(&recipe),
                scene: None,
                references: 1,
                ambient_assets: None,
            }),
        );
        // AddPresRegistryAsset precedes TrackLoadedRecipe; track by actual pointer.
        if !self.tracked.iter().any(|old| Arc::ptr_eq(old, &recipe)) {
            self.tracked.push(recipe);
        }
        Ok(())
    }
    fn retain_scene(
        &mut self,
        id: u64,
        recipe: &Arc<CharacterRecipe>,
        scene: Arc<CharacterScene>,
    ) -> Result<(), MountedRecipeError> {
        let Some(RegisteredAsset::Recipe(entry)) = self.assets.get_mut(&id) else {
            return Err(MountedRecipeError::RegistryIdentity(id));
        };
        if !Arc::ptr_eq(&entry.recipe, recipe) {
            return Err(MountedRecipeError::RegistryIdentity(id));
        }
        entry.scene = Some(scene);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MountedRecipePhase {
    New,
    Requested,
    SchemaLoaded,
    AssetsLoaded,
    Finalized,
    WaitingFace,
    Loaded,
    CallbackDelivered,
}
pub(crate) struct MountedPlayerRecipeLoad {
    requests: SimCharacterRequests,
    callback: Arc<()>,
    ticket: Option<LoadTicket>,
    phase: MountedRecipePhase,
    recipe: Option<Arc<CharacterRecipe>>,
    finalized: Option<Arc<FinalizedRecipeScene>>,
    asset_id: Option<u64>,
}
#[derive(Debug)]
pub(crate) enum MountedRecipeError {
    Phase(MountedRecipePhase),
    Protocol(PlayerLoaderError),
    Requests(RequestError),
    Attribute(AttributeRuntimeError),
    Attributes(PlayerAttributesError),
    Cvar(InputCvarError),
    Mount(MountError),
    Recipe(CharacterRecipeError),
    Scene(Box<dyn std::error::Error>),
    Finalize(RecipeFinalizeError),
    Face(CharacterFaceError),
    Utf8(std::str::Utf8Error),
    MissingName,
    NameMismatch,
    LocalMarqueeRequired,
    XmlLoadPending,
    FaceCallbackMismatch,
    Missing(&'static str),
    RegistryOccupied(u64),
    RecipeCacheHitPending(u64),
    RegistryAssetType { id: u64, asset_type: u32 },
    RegistryIdentity(u64),
    AmbientCategory(u32),
    ReferencedRecipe(u64),
}
impl std::fmt::Display for MountedRecipeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AmbientCategory(value) => write!(
                f,
                "LivingWorld loader cannot load character category {value}"
            ),
            Self::ReferencedRecipe(id) => write!(f, "recipe {id:016x} still has load references"),
            Self::Phase(v) => write!(f, "mounted recipe already attempted at {v:?}"),
            Self::Protocol(v) => write!(f, "mounted recipe loader: {v}"),
            Self::Requests(v) => write!(f, "mounted recipe request: {v}"),
            Self::Attribute(v) => write!(f, "mounted recipe definition: {v}"),
            Self::Attributes(v) => write!(f, "mounted recipe attributes: {v}"),
            Self::Cvar(v) => write!(f, "mounted recipe cvar: {v}"),
            Self::Mount(v) => write!(f, "mounted recipe IO: {v}"),
            Self::Recipe(v) => write!(f, "mounted recipe decode: {v}"),
            Self::Scene(v) => write!(f, "mounted recipe assets: {v}"),
            Self::Finalize(v) => write!(f, "mounted recipe finalization: {v}"),
            Self::Face(v) => write!(f, "mounted recipe face: {v:?}"),
            Self::Utf8(v) => write!(f, "mounted recipe text: {v}"),
            Self::MissingName => f.write_str("marquee cached schema name absent"),
            Self::NameMismatch => {
                f.write_str("supplied marquee definition disagrees with cached name")
            }
            Self::LocalMarqueeRequired => {
                f.write_str("mounted producer admits first local marquee only")
            }
            Self::XmlLoadPending => f.write_str("native XML/graveyard recipe load is not mounted"),
            Self::FaceCallbackMismatch => {
                f.write_str("face callback belongs to another retained recipe loader")
            }
            Self::Missing(v) => write!(f, "mounted recipe {v} unavailable"),
            Self::RegistryOccupied(id) => {
                write!(f, "presentation registry ID {id:016x} already occupied")
            }
            Self::RecipeCacheHitPending(id) => {
                write!(f, "shared recipe {id:016x} requires cache-hit continuation")
            }
            Self::RegistryAssetType { id, asset_type } => write!(
                f,
                "presentation registry ID {id:016x} has unsupported asset type {asset_type:08x}"
            ),
            Self::RegistryIdentity(id) => write!(
                f,
                "presentation recipe {id:016x} belongs to another allocation"
            ),
        }
    }
}
impl MountedPlayerRecipeLoad {
    /// PresCharacterManager::Update82818f2c: actual recipe entity membership
    /// and low32 peer equality precede the two registration callbacks.
    pub(crate) fn complete_presentation_registration(
        &mut self,
        loader: &mut PlayerLoaderSlot,
        actual_scene: &Arc<CharacterScene>,
        actual_peer: i64,
    ) -> Result<(), MountedRecipeError> {
        use MountedRecipeError as E;
        if let PlayerLoadPhase::Loaded { character_id } = loader.phase {
            return if self.asset_id == Some(character_id)
                && Arc::ptr_eq(self.scene()?, actual_scene)
            {
                Ok(())
            } else {
                Err(E::RegistryIdentity(character_id))
            };
        }
        let PlayerLoadPhase::Registering {
            request_id,
            character_id,
        } = loader.phase
        else {
            return Err(E::Protocol(PlayerLoaderError::UnexpectedRegistration));
        };
        if self.asset_id != Some(character_id) || !Arc::ptr_eq(self.scene()?, actual_scene) {
            return Err(E::RegistryIdentity(character_id));
        }
        let peer = match loader.current {
            Some(
                super::PlayerSelection::Marquee { peer_id, .. }
                | super::PlayerSelection::CreateACharacter { peer_id },
            ) => peer_id,
            _ => return Err(E::Protocol(PlayerLoaderError::MissingCurrentSelection)),
        };
        if actual_peer as u32 != peer as u32 {
            // Native keeps this request pending and retries next Update.
            return Ok(());
        }
        loader
            .character_registered(request_id, character_id)
            .map_err(E::Protocol)?;
        self.finish_registration(request_id, loader.player_index())
    }
    /// Complete the existing register callback without leaving its shared slot
    /// occupied for the next edit. Call after real registration has completed.
    pub(crate) fn finish_registration(
        &mut self,
        request: u32,
        player: u32,
    ) -> Result<(), MountedRecipeError> {
        self.requests
            .finish_registered_callback(request, &self.callback, player)
            .map_err(MountedRecipeError::Requests)
    }

    /// Host synchronous replacement after the caller detaches the old presentation
    /// observer. Existing Arc holders retain old resources until their release.
    /// This shares the actual request allocator, registry and face manager.
    pub(crate) fn try_replace_cac(
        &mut self,
        loader: &mut PlayerLoaderSlot,
        services: &PlayerAttributeServices,
        faces: &mut CharacterFaceManager,
        mounts: &MountSet,
        cvars: &InputCvarReferences,
        registry: &mut MountedRecipeRegistry,
        record: &skate2_formats::skater_profile::CacRecord,
    ) -> Result<PlayerLoadAction, MountedRecipeError> {
        use MountedRecipeError as E;
        if self.phase != MountedRecipePhase::CallbackDelivered {
            return Err(E::Phase(self.phase));
        }
        let PlayerLoadPhase::Loaded { character_id } = loader.phase else {
            return Err(E::Protocol(PlayerLoaderError::SlotBusy(loader.phase)));
        };
        self.require_sole_registration(registry, character_id)?;
        // Prepare actual replacement geometry before releasing registry ownership.
        // Transfer this same allocation to the new load, avoiding duplicate IO.
        let mut replacement = CharacterRecipe::parse(&record.data.recipe).map_err(E::Recipe)?;
        fill_missing(mounts, &mut replacement, record)?;
        replacement.category = 3;
        replacement.clone_type = 3;
        let mut scene = crate::mounted_character_scene::own_character_scene_category(
            mounts,
            &replacement,
            "createacharacter",
        )
        .map_err(E::Scene)?;
        scene.category = replacement.category;
        let mut prepared = FinalizedRecipeScene::new(Arc::new(scene));
        prepared
            .try_finalize(replacement.clone_type)
            .map_err(E::Finalize)?;
        self.unload_registered(loader, registry, character_id)?;
        self.try_load_selected_inner(
            loader,
            None,
            services,
            faces,
            mounts,
            cvars,
            registry,
            Some(record),
            Some(&PreparedRecipe { recipe: Arc::new(replacement), scene: Arc::new(prepared), face: None }),
        )
    }
    /// The marquee counterpart of `try_replace_cac`: `UpdateGameSettings` 0x8264b268's
    /// `SetMarqueeCharacter` and `Load` once the loaded recipe is released.
    pub(crate) fn try_replace_marquee(
        &mut self,
        loader: &mut PlayerLoaderSlot,
        definition: &MarqueeDefinition,
        services: &PlayerAttributeServices,
        faces: &mut CharacterFaceManager,
        mounts: &MountSet,
        cvars: &InputCvarReferences,
        registry: &mut MountedRecipeRegistry,
    ) -> Result<PlayerLoadAction, MountedRecipeError> {
        use MountedRecipeError as E;
        if self.phase != MountedRecipePhase::CallbackDelivered {
            return Err(E::Phase(self.phase));
        }
        let PlayerLoadPhase::Loaded { character_id } = loader.phase else {
            return Err(E::Protocol(PlayerLoaderError::SlotBusy(loader.phase)));
        };
        self.require_registration(registry, character_id)?;
        self.unload_registered(loader, registry, character_id)?;
        loader.set_marquee(definition.id, super::NO_PEER_ID);
        self.try_load_selected_inner(loader, Some(definition), services, faces, mounts, cvars, registry, None, None)
    }
    /// `PlayerLoaderImpl::Load` 0x8281aa80's CAS branch for a slot the caller set to the
    /// created skater (`SetCACCharacter`): the profile's parts record.
    pub(crate) fn try_load_cac(
        &mut self,
        loader: &mut PlayerLoaderSlot,
        services: &PlayerAttributeServices,
        faces: &mut CharacterFaceManager,
        mounts: &MountSet,
        cvars: &InputCvarReferences,
        registry: &mut MountedRecipeRegistry,
        record: &skate2_formats::skater_profile::CacRecord,
    ) -> Result<PlayerLoadAction, MountedRecipeError> {
        self.try_load_selected_inner(loader, None, services, faces, mounts, cvars, registry, Some(record), None)
    }
    /// `PlayerLoaderImpl::Unload` 0x82818170 (`PlayerLoader::UnloadPartyMember` 0x828182f0):
    /// `SimCharacterManager::Unload` of the slot's character, registered or still awaiting
    /// its registration, whose request then has no loader to answer.
    pub(crate) fn try_unload(
        &mut self,
        loader: &mut PlayerLoaderSlot,
        registry: &mut MountedRecipeRegistry,
    ) -> Result<(), MountedRecipeError> {
        use MountedRecipeError as E;
        if self.phase != MountedRecipePhase::CallbackDelivered {
            return Err(E::Phase(self.phase));
        }
        let (PlayerLoadPhase::Loaded { character_id } | PlayerLoadPhase::Registering { character_id, .. }) =
            loader.phase
        else {
            return Err(E::Protocol(PlayerLoaderError::SlotBusy(loader.phase)));
        };
        self.require_registration(registry, character_id)?;
        self.unload_registered(loader, registry, character_id)
    }
    /// The loaded character's registry entry, which this load holds with its other users'
    /// references (returned).
    fn require_registration(
        &self,
        registry: &MountedRecipeRegistry,
        character_id: u64,
    ) -> Result<u32, MountedRecipeError> {
        use MountedRecipeError as E;
        let old = self.recipe()?;
        let Some(RegisteredAsset::Recipe(entry)) = registry.assets.get(&character_id) else {
            return Err(E::RegistryIdentity(character_id));
        };
        if self.asset_id != Some(character_id) || !Arc::ptr_eq(old, &entry.recipe) {
            return Err(E::RegistryIdentity(character_id));
        }
        Ok(entry.references)
    }
    /// The loaded character's registry entry, which this load alone holds.
    fn require_sole_registration(
        &self,
        registry: &MountedRecipeRegistry,
        character_id: u64,
    ) -> Result<(), MountedRecipeError> {
        if self.require_registration(registry, character_id)? != 1 {
            return Err(MountedRecipeError::ReferencedRecipe(character_id));
        }
        Ok(())
    }
    /// The unload request, then `UpdateRequest`'s `RequestState_UnloadRequested` (0x82d15554):
    /// the entity loses this load's reference, and only the last one's `Unload` (0x82d155d8)
    /// removes the registry entry before `TrackUnloadedRecipe`. The slot is left unloaded.
    fn unload_registered(
        &mut self,
        loader: &mut PlayerLoaderSlot,
        registry: &mut MountedRecipeRegistry,
        character_id: u64,
    ) -> Result<(), MountedRecipeError> {
        use MountedRecipeError as E;
        let unload = self
            .requests
            .begin_unload(Arc::clone(&self.callback))
            .map_err(E::Requests)?;
        let Some(RegisteredAsset::Recipe(entry)) = registry.assets.get_mut(&character_id) else {
            unreachable!()
        };
        entry.references = entry.references.wrapping_sub(1);
        let old = if entry.references == 0 {
            let Some(RegisteredAsset::Recipe(old)) = registry.assets.remove(&character_id) else {
                unreachable!()
            };
            registry
                .tracked
                .retain(|value| !Arc::ptr_eq(value, &old.recipe));
            Some(old)
        } else {
            None
        };
        self.recipe = None;
        self.finalized = None;
        self.asset_id = None;
        self.ticket = None;
        self.phase = MountedRecipePhase::New;
        loader.phase = PlayerLoadPhase::Unloaded;
        loader.current = None;
        drop(old);
        self.requests
            .finish_unloaded_callback(&unload)
            .map_err(E::Requests)
    }
    /// Lend the retained SimCharacterManager request allocator to LivingWorld.
    pub(crate) fn requests_mut(&mut self) -> &mut SimCharacterRequests {
        &mut self.requests
    }
    pub(crate) fn new() -> Self {
        Self {
            requests: SimCharacterRequests::construct_fields(),
            callback: Arc::new(()),
            ticket: None,
            phase: MountedRecipePhase::New,
            recipe: None,
            finalized: None,
            asset_id: None,
        }
    }
    /// The loaded recipe's asset ID.
    pub(crate) fn asset_id(&self) -> Option<u64> {
        self.asset_id
    }
    pub(crate) fn recipe(&self) -> Result<&Arc<CharacterRecipe>, MountedRecipeError> {
        self.recipe
            .as_ref()
            .ok_or(MountedRecipeError::Missing("recipe"))
    }
    pub(crate) fn scene(&self) -> Result<&Arc<CharacterScene>, MountedRecipeError> {
        self.finalized
            .as_ref()
            .ok_or(MountedRecipeError::Missing("finalized scene"))?
            .scene()
            .map_err(MountedRecipeError::Finalize)
    }
    pub(crate) fn finalized(&self) -> Result<&Arc<FinalizedRecipeScene>, MountedRecipeError> {
        self.finalized
            .as_ref()
            .ok_or(MountedRecipeError::Missing("finalized scene"))
    }
    fn face_callback(&mut self, asset_id: u64, callback: u64) -> Result<(), MountedRecipeError> {
        let expected = u64::try_from(Arc::as_ptr(&self.callback) as usize)
            .expect("host pointer fits callback identity");
        if self.phase != MountedRecipePhase::WaitingFace
            || self.asset_id != Some(asset_id)
            || callback != expected
        {
            return Err(MountedRecipeError::FaceCallbackMismatch);
        }
        self.phase = MountedRecipePhase::Loaded;
        Ok(())
    }
    /// Caller already resolved the selected marquee collection, corresponding to
    /// the native factory entry after message name→collection lookup. No alternate
    /// name table is constructed here. This does not run outer manager updates.
    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        reason = "one retained native load/callback continuation over supplied owners"
    )]
    pub(crate) fn try_load_selected(
        &mut self,
        loader: &mut PlayerLoaderSlot,
        definition: &MarqueeDefinition,
        services: &PlayerAttributeServices,
        faces: &mut CharacterFaceManager,
        mounts: &MountSet,
        cvars: &InputCvarReferences,
        registry: &mut MountedRecipeRegistry,
        cac: Option<&skate2_formats::skater_profile::CacRecord>,
    ) -> Result<PlayerLoadAction, MountedRecipeError> {
        self.try_load_selected_inner(
            loader,
            Some(definition),
            services,
            faces,
            mounts,
            cvars,
            registry,
            cac,
            None,
        )
    }
    /// [`Self::try_load_selected`] for a marquee whose loader already streamed its recipe, scene
    /// and face ([`Self::try_prepare_marquee_scene`]).
    #[expect(
        clippy::too_many_arguments,
        reason = "the selected load's supplied owners and its streamed scene"
    )]
    pub(crate) fn try_load_selected_prepared(
        &mut self,
        loader: &mut PlayerLoaderSlot,
        definition: &MarqueeDefinition,
        services: &PlayerAttributeServices,
        faces: &mut CharacterFaceManager,
        mounts: &MountSet,
        cvars: &InputCvarReferences,
        registry: &mut MountedRecipeRegistry,
        prepared: &PreparedRecipe,
    ) -> Result<PlayerLoadAction, MountedRecipeError> {
        self.try_load_selected_inner(
            loader,
            Some(definition),
            services,
            faces,
            mounts,
            cvars,
            registry,
            None,
            Some(prepared),
        )
    }
    /// The definition a marquee load streams: the marquee itself when byte 8 of its
    /// `characters_marquee` layout allows it, else `DEFAULT_MARQUEE_ID`'s. A requested
    /// model name must be the marquee's own.
    fn try_actual_marquee(
        services: &PlayerAttributeServices,
        definition: &MarqueeDefinition,
        requested: Option<&[u8]>,
    ) -> Result<MarqueeDefinition, MountedRecipeError> {
        use MountedRecipeError as E;
        let allowed = {
            let instance = services
                .vault
                .instance(0x91bc_a669_3efc_9aa7, definition.id);
            let name = instance
                .try_cached_layout_text(0)
                .map_err(E::Attribute)?
                .ok_or(E::MissingName)?;
            if requested.is_some_and(|requested| name.bytes() != requested) {
                return Err(E::NameMismatch);
            }
            instance
                .try_cached_layout_field(8, 1)
                .map_err(E::Attribute)?
                .is_some_and(|v| v.bytes()[0] != 0)
        };
        let actual_id = if allowed {
            definition.id
        } else {
            DEFAULT_MARQUEE_ID
        };
        let instance = services.vault.instance(0x91bc_a669_3efc_9aa7, actual_id);
        let name = instance
            .try_cached_layout_text(0)
            .map_err(E::Attribute)?
            .ok_or(E::MissingName)?;
        Ok(MarqueeDefinition {
            id: actual_id,
            schema_name: std::str::from_utf8(name.bytes())
                .map_err(E::Utf8)?
                .to_owned(),
        })
    }
    /// The definition [`Self::try_load_selected`] streams for this marquee.
    pub(crate) fn try_actual_marquee_definition(
        services: &PlayerAttributeServices,
        definition: &MarqueeDefinition,
    ) -> Result<MarqueeDefinition, MountedRecipeError> {
        Self::try_actual_marquee(services, definition, None)
    }
    /// What a selected marquee load reads, made ahead of the load: `AI::SkaterAsset`'s
    /// `CharacterRecipeLoader` streams its recipe before any skater spawns with it. `actual` is
    /// [`Self::try_actual_marquee_definition`]'s.
    pub(crate) fn try_prepare_marquee_scene(
        mounts: &MountSet,
        actual: &MarqueeDefinition,
    ) -> Result<Arc<PreparedRecipe>, MountedRecipeError> {
        use MountedRecipeError as E;
        let bytes = mounts
            .read_path(&actual.binary_recipe_path())
            .map_err(E::Mount)?;
        let recipe = Self::retain_runtime_recipe(CharacterRecipe::parse(&bytes).map_err(E::Recipe)?);
        let mut scene =
            crate::mounted_character_scene::own_character_scene_category(mounts, &recipe, "marquee")
                .map_err(E::Scene)?;
        scene.category = recipe.category;
        let mut finalized = FinalizedRecipeScene::new(Arc::new(scene));
        finalized
            .try_finalize(recipe.clone_type)
            .map_err(E::Finalize)?;
        let face_name = Self::face_name(&recipe).to_owned();
        let face = Self::face_entry(face_name.as_bytes())
            .and_then(|entry| mounts.read_path(&entry).ok())
            .map(|bytes| (face_name, Arc::from(bytes)));
        Ok(Arc::new(PreparedRecipe { recipe, scene: Arc::new(finalized), face }))
    }
    /// `CharacterRecipeLoader::OnComplete`'s face (0x82d0a234..274): the CAC gender face, not
    /// the master recipe's schema (`cas_db`), for a created character (a missing customization
    /// block is male); the schema's own for a marquee.
    fn face_name(recipe: &CharacterRecipe) -> &str {
        if recipe.category == 3 {
            if recipe.customizations.as_ref().is_none_or(|c| c.is_male) {
                "CAC_male"
            } else {
                "CAC_female"
            }
        } else {
            &recipe.schema_name
        }
    }
    /// The `anim.big` entry `LoadFaceData` reads for the named face.
    fn face_entry(name: &[u8]) -> Option<String> {
        let path = String::from_utf8(CharacterFaceManager::archive_path(name)).ok()?;
        path.split_once('|').map(|(_, entry)| entry.to_owned())
    }
    fn try_load_selected_inner(
        &mut self,
        loader: &mut PlayerLoaderSlot,
        definition: Option<&MarqueeDefinition>,
        services: &PlayerAttributeServices,
        faces: &mut CharacterFaceManager,
        mounts: &MountSet,
        cvars: &InputCvarReferences,
        registry: &mut MountedRecipeRegistry,
        cac: Option<&skate2_formats::skater_profile::CacRecord>,
        prepared: Option<&PreparedRecipe>,
    ) -> Result<PlayerLoadAction, MountedRecipeError> {
        use MountedRecipeError as E;
        if self.phase != MountedRecipePhase::New {
            return Err(E::Phase(self.phase));
        }
        // Selected admission before source work. Category/locality are genuine loader
        // outputs, not a replacement player-index or backend selection.
        let next = self.requests.next_request_id();
        let mut saved_recipe = cac
            .map(|r| CharacterRecipe::parse(&r.data.recipe))
            .transpose()
            .map_err(E::Recipe)?;
        if let (Some(recipe), Some(record)) = (saved_recipe.as_mut(), cac) {
            fill_missing(mounts, recipe, record)?;
        }
        let action = if let Some(recipe) = &saved_recipe {
            // A party member's SetCACCharacter already carries its peer.
            if !matches!(loader.requested, super::PlayerSelection::CreateACharacter { .. }) {
                loader.set_create_a_character(super::NO_PEER_ID);
            }
            loader.begin_cac_load(next, &recipe.schema_name)
        } else {
            loader.begin_marquee_load(definition.ok_or(E::Missing("marquee definition"))?, next)
        }
        .map_err(E::Protocol)?;
        self.phase = MountedRecipePhase::Requested;
        let PlayerLoadAction::Load(request) = action else {
            return Err(E::LocalMarqueeRequired);
        };
        if !request.is_local_player {
            return Err(E::LocalMarqueeRequired);
        }
        self.ticket = Some(
            self.requests
                .begin_load(Arc::clone(&self.callback), loader.player_index())
                .map_err(E::Requests)?,
        );
        // Fresh factory wrapper, cached layout byte8, folded policy function returns0.
        // Null generated layout defaults byte8 to0; only then choose native default.
        // A schema the registry holds is shared (`RequestState_LoadSchemaFromFile` 0x82d14b90)
        // instead of read again: several players may load one skater.
        let (recipe, shared) = if let Some(mut recipe) = saved_recipe {
            recipe.category = 3;
            recipe.clone_type = 3;
            let id =
                skate2_formats::hash::rw_hash64_string(&recipe.schema_name.to_ascii_lowercase());
            match registry.try_share(id)? {
                Some((recipe, scene)) => (recipe, Some(scene)),
                None => (Arc::new(recipe), None),
            }
        } else {
            let definition = definition.ok_or(E::Missing("marquee definition"))?;
            let actual =
                Self::try_actual_marquee(services, definition, Some(request.model_name.as_bytes()))?;
            // Same session registry lookup precedes the live binary/XML choice.
            if let Some((recipe, scene)) = registry.try_share(actual.schema_asset_id())? {
                (recipe, Some(scene))
            } else {
                if !cvars.try_schema_load_from_binary().map_err(E::Cvar)? {
                    return Err(E::XmlLoadPending);
                }
                let recipe = if let Some(prepared) = prepared {
                    Arc::clone(&prepared.recipe)
                } else {
                    let bytes = mounts
                        .read_path(&actual.binary_recipe_path())
                        .map_err(E::Mount)?;
                    Self::retain_runtime_recipe(CharacterRecipe::parse(&bytes).map_err(E::Recipe)?)
                };
                (recipe, None)
            }
        };
        let asset_id =
            skate2_formats::hash::rw_hash64_string(&recipe.schema_name.to_ascii_lowercase());
        let finalized = if let Some(scene) = shared {
            self.recipe = Some(Arc::clone(&recipe));
            self.asset_id = Some(asset_id);
            self.phase = MountedRecipePhase::SchemaLoaded;
            // The entity's assets are loaded (`mState`), so `RequestState_AssetLoadRequested`
            // goes straight to `FinalizeStructure` (0x82d15318): `RebindToMaterials` and
            // `BuildTInstanceArray` again over the shared entity.
            let mut finalized = FinalizedRecipeScene::new(scene);
            finalized
                .try_finalize(recipe.clone_type)
                .map_err(E::Finalize)?;
            Arc::new(finalized)
        } else {
            registry.require_miss(asset_id)?;
            self.recipe = Some(Arc::clone(&recipe));
            self.asset_id = Some(asset_id);
            registry.publish_parsed(asset_id, Arc::clone(&recipe))?;
            self.phase = MountedRecipePhase::SchemaLoaded;
            let finalized = if let Some(prepared) = prepared {
                Arc::clone(&prepared.scene)
            } else {
                let mut scene = crate::mounted_character_scene::own_character_scene_category(
                    mounts,
                    &recipe,
                    if cac.is_some() {
                        "createacharacter"
                    } else {
                        "marquee"
                    },
                )
                .map_err(E::Scene)?;
                scene.category = recipe.category;
                let mut finalized = FinalizedRecipeScene::new(Arc::new(scene));
                finalized
                    .try_finalize(recipe.clone_type)
                    .map_err(E::Finalize)?;
                Arc::new(finalized)
            };
            let scene = Arc::clone(finalized.scene().map_err(E::Finalize)?);
            registry.retain_scene(asset_id, &recipe, scene)?;
            finalized
        };
        self.finalized = Some(finalized);
        self.phase = MountedRecipePhase::AssetsLoaded;
        self.phase = MountedRecipePhase::Finalized;
        let callback = u64::try_from(Arc::as_ptr(&self.callback) as usize)
            .expect("host pointer fits callback identity");
        let face_name = Self::face_name(&recipe);
        let admission = faces.begin_load(face_name.as_bytes(), asset_id, callback);
        if admission.already_loaded {
            self.phase = MountedRecipePhase::Loaded;
        } else {
            self.phase = MountedRecipePhase::WaitingFace;
            if let Some(ticket) = admission.load {
                let path = std::str::from_utf8(&ticket.archive_path).map_err(E::Utf8)?;
                let (_, entry) = path
                    .split_once('|')
                    .ok_or(E::Missing("face archive entry"))?;
                let streamed = prepared
                    .and_then(|prepared| prepared.face.as_ref())
                    .filter(|(name, _)| name == face_name)
                    .map(|(_, bytes)| Box::<[u8]>::from(&bytes[..]));
                let bytes = match streamed {
                    Some(bytes) => bytes,
                    None => mounts.read_path(entry).map_err(E::Mount)?.into_boxed_slice(),
                };
                faces
                    .complete_load(&ticket, true, bytes)
                    .map_err(E::Face)?;
            }
            faces.dispatch_finished(|request| {
                self.face_callback(request.asset_id, request.callback)
            })?;
            if self.phase != MountedRecipePhase::Loaded {
                return Err(E::Missing("face completion"));
            }
        }
        // Both marquee LoadCompleteImpl entries are the proven folded empty function.
        // CAC LoadCompleteImpl82819da0 calls ApplyAttributesImpl when set-as-skater.
        if let Some(record) = cac {
            services
                .apply_cac_loader_attributes(record, loader.player_index())
                .map_err(E::Attributes)?;
        }
        self.phase = MountedRecipePhase::Loaded;
        let ticket = self.ticket.as_ref().expect("retained load request");
        // Keep the native current load request until actual registration allocation.
        loader
            .character_loaded(ticket.id(), asset_id, ticket.id())
            .map_err(E::Protocol)?;
        let (ghost, apply) = if let Some(record) = cac {
            loader.try_prepare_cac_loaded_attributes(services, record)
        } else {
            loader.try_prepare_loaded_attributes(services)
        }
        .map_err(E::Attributes)?;
        let registration = self
            .requests
            .begin_registration(ticket)
            .map_err(E::Requests)?;
        loader.phase = PlayerLoadPhase::ApplyingAttributes {
            request_id: registration,
            character_id: asset_id,
        };
        let action = loader
            .finish_loaded_attributes(ghost, apply)
            .map_err(E::Protocol)?;
        self.requests
            .finish_loaded_callback(ticket)
            .map_err(E::Requests)?;
        self.phase = MountedRecipePhase::CallbackDelivered;
        Ok(action)
    }
    fn retain_runtime_recipe(mut recipe: CharacterRecipe) -> Arc<CharacterRecipe> {
        // Local factory constructor defines clone3, so Unpack cannot replace it.
        // UpdateRequest publishes the requested category before registry insertion.
        recipe.clone_type = 3;
        recipe.category = 0;
        Arc::new(recipe)
    }
}
#[cfg(test)]
#[path = "player_mounted_recipe_load_tests.rs"]
mod tests;

/// A saved skater's parts whose files are absent (Skate 3 clothes with the Skate 3 data
/// unlinked) load as their slots' defaults; see `fill_missing_parts`.
fn fill_missing(
    mounts: &MountSet,
    recipe: &mut CharacterRecipe,
    record: &skate2_formats::skater_profile::CacRecord,
) -> Result<(), MountedRecipeError> {
    // SKATE2_DIAGNOSTIC_MISSING_PART=<slot>: that slot's model is one no archive holds, as a
    // Skate 3 part is once the Skate 3 data is unlinked.
    if let Ok(slot) = std::env::var("SKATE2_DIAGNOSTIC_MISSING_PART") {
        for component in recipe.components.iter_mut().filter(|c| c.name.eq_ignore_ascii_case(&slot)) {
            for lod in component.models.iter_mut().flat_map(|m| m.lods.iter_mut()) {
                lod.arena_asset_id = 0x5ca7_e300_0000_0001;
            }
        }
    }
    let replaced = crate::edit_skater_character::fill_missing_parts(
        mounts,
        recipe,
        record.data.is_male,
        "createacharacter",
    )
    .map_err(MountedRecipeError::Scene)?;
    if !replaced.is_empty() {
        eprintln!("skater parts missing from the mounted data, defaults worn: {}", replaced.join(", "));
    }
    Ok(())
}
