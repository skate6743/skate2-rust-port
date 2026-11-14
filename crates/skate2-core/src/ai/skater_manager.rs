//! `AI::AISkaterManager` (0x1b8 bytes) and its `SkaterAssetManager`: the ambient skaters around
//! the camera. Every 60 updates it unspawns far skaters (offset 0), swaps or loads skater models
//! (15) and spawns one skater on the best path start in range (30); every update gives each new
//! ambient skater the Wander behaviour.
//!
//! The manager owns the backend's view of the nine player slots it changes and returns the
//! `SkaterManager` work (spawn, unspawn, behaviour, wipeout request) as [`AmbientCommand`]s.
//! It is one object the simulation and the challenge system share (`AISkaterManager` is a
//! singleton): a challenge's skaters are requested, spawned and released through it too, and
//! its `cMsgSpawnAi`/`cMsgUnspawnAi` messages wait in [`AiSkaterManager::pending`] for the
//! `SkaterManager` after the frame.
use super::{
    actor::AiClientType,
    path,
    path_manager::{LockedPathTest, PathManager, PathManagerError, PathQueryResult, PathQueryType},
    profile::SkaterProfile,
    vmx::{self, V4},
};
use crate::attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault};
use crate::players_backend_settings::PlayersBackendSettings;
use crate::challenge::unlockables::{self, Unlockables};
use std::sync::Arc;

/// `characters_marquee`.
pub(crate) const CHARACTERS_MARQUEE: u64 = 0x91bc_a669_3efc_9aa7;
/// `characters_marquee.aiprofile` and `.ChallengeSignupEquivalent`.
const AI_PROFILE: u64 = 0xcdc3_4d02_45e7_7199;
const CHALLENGE_SIGNUP_EQUIVALENT: u64 = 0x4faf_f7d8_0eaf_9779;
/// The `default` collection key's low word, which a `ChallengeSignupEquivalent` names to mean
/// none.
const DEFAULT_COLLECTION_LOW: u32 = 0x2d7d_2152;
/// The `ai_skater_profiles` allowed-skater flag `PickBestSkaterToLoad` favours (0..=42).
const ALLOWED_SKATER_FLAG: u64 = 0xa390_be5f_c0dd_a71c;

/// `GetPeerIDFromSkaterID` 0x824b2bc8: an AI skater's peer is its slot plus ten.
const AI_PEER_OFFSET: i64 = 10;

/// The `AI_Skater_Ambient` cvars `ConvertToAmbientSkater` 0x824cea88 reads, at their defaults,
/// and its path query's result count.
pub(crate) mod conversion {
    pub(crate) const MAX_PATH_DISTANCE: f32 = 20.0;
    pub(crate) const PATH_DISTANCE_WEIGHT: f32 = 500.0;
    pub(crate) const SKATER_DISTANCE_WEIGHT: f32 = 200.0;
    pub(crate) const MIN_DIST_BETWEEN_ACTORS: f32 = 2.0;
    pub(crate) const MAX_DIST_BETWEEN_ACTORS: f32 = 6.0;
    pub(crate) const MAX_PATHS: usize = 32;
}

/// `CalculateConvertToAmbientPathScore` 0x824b92f8: the path's first node's distance from the
/// skater over the maximum, weighted; another skater with a player row within the minimum
/// distance of the node rules the path out (the maximum score), one within the maximum
/// distance adds its weighted nearness.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "the native fctiwz")]
pub(crate) fn convert_to_ambient_path_score(start: V4, skater: V4, others: &[V4]) -> u32 {
    let distance = vmx::length3(vmx::sub(start, skater));
    let mut score = ((distance / conversion::MAX_PATH_DISTANCE) * conversion::PATH_DISTANCE_WEIGHT) as i32 as u32;
    for &other in others {
        let distance = vmx::length3(vmx::sub(start, other));
        if distance < conversion::MIN_DIST_BETWEEN_ACTORS {
            return u32::MAX;
        }
        if distance < conversion::MAX_DIST_BETWEEN_ACTORS {
            let span = conversion::MAX_DIST_BETWEEN_ACTORS - conversion::MIN_DIST_BETWEEN_ACTORS;
            let near = ((conversion::MAX_DIST_BETWEEN_ACTORS - distance) / span) * conversion::SKATER_DISTANCE_WEIGHT;
            score = score.wrapping_add(near as i32 as u32);
        }
    }
    score
}

/// `PickBestConvertToAmbientPath` 0x824c2e78: the lowest score, the first on ties; none when
/// every path scores the maximum.
pub(crate) fn pick_best_convert_to_ambient_path(scores: &[u32]) -> Option<usize> {
    let mut best = None;
    let mut lowest = u32::MAX;
    for (index, &score) in scores.iter().enumerate() {
        if score < lowest {
            lowest = score;
            best = Some(index);
        }
    }
    best
}

/// `AI` cvars at their defaults.
mod cvars {
    pub(super) const ENABLE_AMBIENT_SKATERS: bool = true;
    pub(super) const BAN_AMBIENT_SKATERS_IN_PARTY_PLAY: bool = true;
    pub(super) const MAX_AMBIENT_SKATERS: u32 = 3;
    pub(super) const MIN_AMBIENT_SKATERS_WHEN_SIM_SLOW: u32 = 3;
    pub(super) const MAX_AMBIENT_SKATER_MODELS_MINIMAL_SET: u32 = 2;
    pub(super) const MAX_AMBIENT_SKATER_MODELS: u32 = 3;
    pub(super) const PREVENT_MODEL_SWAP_DURING_MINIMAL_SET: bool = true;
    pub(super) const PREVENT_MODEL_SWAP_IN_FRONTEND: bool = true;
    pub(super) const ARE_AMBIENT_ONLY_SKATERS_ENABLED: bool = true;
    pub(super) const SKIP_IS_RECIPE_UNLOCKED_TEST: bool = false;
    pub(super) const AMBIENT_SPAWN_MIN_DISTANCE_IN_GAME: f32 = 60.0;
    pub(super) const AMBIENT_SPAWN_MIN_DISTANCE_IN_FE: f32 = 4.0;
    pub(super) const AMBIENT_SPAWN_MAX_DISTANCE_IN_GAME: f32 = 90.0;
    pub(super) const AMBIENT_SPAWN_MAX_DISTANCE_IN_FE: f32 = 30.0;
    pub(super) const AMBIENT_UNSPAWN_DISTANCE_IN_GAME: f32 = 120.0;
    pub(super) const AMBIENT_UNSPAWN_DISTANCE_IN_FE: f32 = 45.0;
    pub(super) const AMBIENT_UNSPAWN_Y_DIFFERENCE_IN_GAME: f32 = 1000.0;
    pub(super) const AMBIENT_UNSPAWN_Y_DIFFERENCE_IN_FE: f32 = 20.0;
    pub(super) const SPAWN_MIN_DIST_BETWEEN_ACTORS: f32 = 5.0;
}

/// `BE::PlayerType`.
pub(crate) mod player_type {
    pub(crate) const NONE: u32 = 0;
    pub(crate) const HUMAN: u32 = 1;
    pub(crate) const AI_AMBIENT: u32 = 2;
    pub(crate) const AI_CHALLENGE: u32 = 3;
    pub(crate) const AI_TEST: u32 = 4;

    pub(crate) const fn is_ai(kind: u32) -> bool {
        kind >= AI_AMBIENT && kind <= AI_TEST
    }
}

/// `EAmbientSkaterModelLoadingMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModelLoadingMode {
    LoadNothing,
    /// The constructor's mode, the front end's until a game loads.
    MinimalSet,
    /// `GameModule::GameLoad` 0x8273c8f0's mode.
    LoadAll,
}

/// `SkaterAsset::eSkaterAssetLoadStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssetStatus {
    Unloaded,
    Loading,
    LoadingRequiresUnload,
    LoadingRequiresReplace,
    Loaded,
    Unloading,
}

/// `AI::SkaterAsset` (0x38): one marquee recipe's `CharacterRecipeLoader`. The port's loader
/// finishes its request at an update once the session has streamed the recipe's scene
/// ([`SkaterAssetManager::streamed`]), or at the next update without session loads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SkaterAsset {
    pub(crate) recipe_id: Option<u64>,
    loader: bool,
    loader_loaded: bool,
    pub(crate) status: AssetStatus,
    pub(crate) current_ai_usage_count: u32,
    pub(crate) total_ai_usage_count: u32,
}

impl SkaterAsset {
    const fn new() -> Self {
        Self {
            recipe_id: None,
            loader: false,
            loader_loaded: false,
            status: AssetStatus::Unloaded,
            current_ai_usage_count: 0,
            total_ai_usage_count: 0,
        }
    }

    /// `LoadRecipe` 0x824b2fc0.
    fn load_recipe(&mut self, recipe: u64) {
        self.total_ai_usage_count = 0;
        if self.loader {
            match self.status {
                AssetStatus::LoadingRequiresUnload => {
                    self.recipe_id = Some(recipe);
                    self.status = AssetStatus::LoadingRequiresReplace;
                }
                _ => self.recipe_id = Some(recipe),
            }
        } else {
            self.recipe_id = Some(recipe);
            self.status = AssetStatus::Loading;
            self.loader = true;
            self.loader_loaded = false;
        }
    }

    /// `FreeRecipe` 0x824b30d8.
    fn free_recipe(&mut self) {
        match self.status {
            AssetStatus::Loading | AssetStatus::LoadingRequiresUnload | AssetStatus::LoadingRequiresReplace => {
                self.status = AssetStatus::LoadingRequiresUnload;
            }
            AssetStatus::Unloading => {}
            _ => self.unload_character_loader(),
        }
        self.recipe_id = None;
        self.current_ai_usage_count = 0;
        self.total_ai_usage_count = 0;
    }

    /// `UnloadCharacterLoader` 0x824b2f10.
    fn unload_character_loader(&mut self) {
        self.loader_loaded = false;
        self.status = AssetStatus::Unloading;
    }

    /// `ReplaceRecipe` 0x824ba9e8.
    fn replace_recipe(&mut self, recipe: u64) {
        match self.status {
            AssetStatus::Loading | AssetStatus::LoadingRequiresUnload | AssetStatus::LoadingRequiresReplace => {
                self.status = AssetStatus::LoadingRequiresReplace;
            }
            AssetStatus::Unloading => {}
            _ => self.free_recipe(),
        }
        self.recipe_id = Some(recipe);
    }

    /// `Update` 0x824baba0 with `UpdateLoading` 0x824baa90.
    fn update(&mut self, streamed: Option<&std::collections::BTreeSet<u64>>) {
        match self.status {
            AssetStatus::Unloaded | AssetStatus::Loaded => {}
            AssetStatus::Loading | AssetStatus::LoadingRequiresUnload | AssetStatus::LoadingRequiresReplace => {
                if !self.loader {
                    return;
                }
                if !self.loader_loaded {
                    self.loader_loaded = match (streamed, self.recipe_id) {
                        (Some(streamed), Some(recipe)) => streamed.contains(&recipe),
                        _ => true,
                    };
                    return;
                }
                let previous = self.status;
                self.status = AssetStatus::Loaded;
                match previous {
                    AssetStatus::LoadingRequiresUnload => self.free_recipe(),
                    AssetStatus::LoadingRequiresReplace => {
                        let recipe = self.recipe_id;
                        self.free_recipe();
                        self.recipe_id = recipe;
                    }
                    _ => {}
                }
            }
            AssetStatus::Unloading => {
                self.loader = false;
                match self.recipe_id {
                    None => self.status = AssetStatus::Unloaded,
                    Some(recipe) => self.load_recipe(recipe),
                }
            }
        }
    }

    /// `Register` 0x824c3930 / `DoRegister` 0x824b31b0 once loaded.
    fn register(&mut self) {
        self.current_ai_usage_count += 1;
        if self.total_ai_usage_count != u32::MAX {
            self.total_ai_usage_count += 1;
        }
    }

    /// `Unregister` 0x824c8740.
    fn unregister(&mut self) {
        self.current_ai_usage_count = self.current_ai_usage_count.saturating_sub(1);
    }
}

/// `AI::SkaterAssetManager` (0x120): five assets, the first `mNumActiveAssets` in use.
#[derive(Clone, Debug)]
pub(crate) struct SkaterAssetManager {
    assets: [SkaterAsset; 5],
    num_active_assets: usize,
    /// The recipes whose character loads have finished, while the session loads them.
    pub(crate) streamed: Option<std::collections::BTreeSet<u64>>,
}

impl Default for SkaterAssetManager {
    fn default() -> Self {
        Self { assets: [SkaterAsset::new(); 5], num_active_assets: 0, streamed: None }
    }
}

impl SkaterAssetManager {
    fn active(&self) -> &[SkaterAsset] {
        &self.assets[..self.num_active_assets]
    }

    fn find(&self, recipe: u64) -> Option<usize> {
        self.active().iter().position(|asset| asset.recipe_id.is_some_and(|id| id as u32 == recipe as u32))
    }

    /// The recipe's asset once registered, for its usage counts.
    pub(crate) fn asset(&self, recipe: u64) -> Option<&SkaterAsset> {
        self.find(recipe).map(|index| &self.assets[index])
    }

    pub(crate) fn update(&mut self) {
        for asset in &mut self.assets {
            asset.update(self.streamed.as_ref());
        }
    }

    /// `LoadSkaterRecipe` 0x824b32b0.
    fn load_skater_recipe(&mut self, recipe: u64) -> bool {
        if self.find(recipe).is_some() {
            return true;
        }
        if self.num_active_assets < 5 {
            self.assets[self.num_active_assets].load_recipe(recipe);
            self.num_active_assets += 1;
            return true;
        }
        false
    }

    /// `ReplaceSkaterRecipe` 0x824bae48.
    fn replace_skater_recipe(&mut self, old: u64, new: u64) -> bool {
        let Some(index) = self.find(old) else { return false };
        self.assets[index].replace_recipe(new);
        true
    }

    /// `FreeSkaterRecipe` 0x824baf58: freed, then swapped with the last active asset.
    fn free_skater_recipe(&mut self, recipe: u64) -> bool {
        let Some(index) = self.find(recipe) else { return false };
        self.assets[index].free_recipe();
        let last = self.num_active_assets - 1;
        self.assets.swap(index, last);
        self.num_active_assets -= 1;
        true
    }

    /// `HasRecipeBeenRequested` 0x824b33d0.
    fn has_recipe_been_requested(&self, recipe: u64) -> bool {
        self.find(recipe).is_some()
    }

    /// `ReplaceAnUnrequiredSkaterRecipe` 0x824bb030: an active recipe stays; otherwise the first
    /// active asset whose recipe the list does not hold takes it.
    fn replace_an_unrequired_skater_recipe(&mut self, required: &[u64], recipe: u64) -> bool {
        if self.active().iter().any(|asset| asset.recipe_id == Some(recipe)) {
            return true;
        }
        let unrequired = self.active().iter().position(|asset| asset.recipe_id.is_none_or(|id| !required.contains(&id)));
        let Some(index) = unrequired else { return false };
        self.assets[index].replace_recipe(recipe);
        true
    }

    /// `LoadSkaterRecipes` 0x824c8830: each required recipe replaces an unrequired one or takes
    /// a free asset.
    fn load_skater_recipes(&mut self, required: &[u64]) -> Result<()> {
        for &recipe in required {
            if !self.replace_an_unrequired_skater_recipe(required, recipe) && !self.load_skater_recipe(recipe) {
                return Err(AiSkaterManagerError::NoAssetFor(recipe));
            }
        }
        Ok(())
    }

    /// `RegisterSkaterRecipe` 0x824c39b8.
    fn register_skater_recipe(&mut self, recipe: u64) {
        if let Some(index) = self.find(recipe) {
            self.assets[index].register();
        }
    }

    /// `UnregisterSkaterRecipe` 0x824c8918.
    fn unregister_skater_recipe(&mut self, recipe: u64) {
        if let Some(index) = self.find(recipe) {
            self.assets[index].unregister();
        }
    }

    fn all_loaded(&self) -> bool {
        self.active().iter().all(|asset| asset.status == AssetStatus::Loaded)
    }

    /// The recipes the active assets hold, loading or loaded.
    pub(crate) fn recipe_ids(&self) -> Vec<u64> {
        self.active().iter().filter_map(|asset| asset.recipe_id).collect()
    }

    /// `GetCurrentlyUnusedRecipeIds` 0x824bb5b8: loaded, unused recipes (usable ones unless
    /// `can_be_unusable`), up to `limit`.
    fn currently_unused_recipe_ids(&self, catalog: &RecipeCatalog, limit: usize, can_be_unusable: bool) -> Vec<u64> {
        let mut recipes = Vec::new();
        for asset in self.active() {
            if asset.status != AssetStatus::Loaded || asset.current_ai_usage_count != 0 {
                continue;
            }
            let Some(recipe) = asset.recipe_id else { continue };
            if can_be_unusable || catalog.is_recipe_usable(recipe) {
                if recipes.len() >= limit {
                    break;
                }
                recipes.push(recipe);
            }
        }
        recipes
    }

    /// `GetCurrentlyUnusedRecipeId` 0x824bb528.
    fn currently_unused_recipe_id(&self, catalog: &RecipeCatalog, can_be_unusable: bool) -> Option<u64> {
        self.currently_unused_recipe_ids(catalog, 1, can_be_unusable).first().copied()
    }

    /// `GetUnusedUnloadableRecipeId` 0x824bb498: a loaded, unused recipe used twice or no
    /// longer usable.
    fn unused_unloadable_recipe_id(&self, catalog: &RecipeCatalog) -> Option<u64> {
        self.active().iter().find_map(|asset| {
            let recipe = asset.recipe_id?;
            (asset.status == AssetStatus::Loaded
                && asset.current_ai_usage_count == 0
                && (asset.total_ai_usage_count >= 2 || !catalog.is_recipe_usable(recipe)))
            .then_some(recipe)
        })
    }

    /// `GetUnlockedUsableSkaterRecipeIds` 0x824bb308: the marquee list's usable recipes, not
    /// already loaded unless `can_be_already_loaded`.
    fn unlocked_usable_skater_recipe_ids(&self, catalog: &RecipeCatalog, can_be_already_loaded: bool) -> Vec<u64> {
        catalog
            .recipes
            .iter()
            .map(|recipe| recipe.id)
            .filter(|&recipe| catalog.is_recipe_usable(recipe))
            .filter(|&recipe| can_be_already_loaded || self.find(recipe).is_none())
            .collect()
    }

    /// `GetAnUnlockedUsableSkater(false)` 0x824bb3e0: an `rw::math::Random` pick of the unlocked
    /// usable recipes not loaded.
    fn an_unlocked_usable_skater(&self, catalog: &RecipeCatalog) -> Result<Option<u64>> {
        let recipes = self.unlocked_usable_skater_recipe_ids(catalog, false);
        if recipes.is_empty() {
            return Ok(None);
        }
        Ok(Some(recipes[rw_random()? as usize % recipes.len()]))
    }
}

/// One `characters_marquee` recipe as the asset manager reads it.
#[derive(Clone, Debug)]
pub(crate) struct MarqueeRecipe {
    pub(crate) id: u64,
    pub(crate) name: String,
    /// Layout byte 9.
    pub(crate) usable: bool,
    /// Layout byte 17.
    pub(crate) ambient_only: bool,
    /// Layout word 3: the character `GetCharacterFromObservedIndex` reports.
    pub(crate) observed_character: i32,
    /// `GetProfileNameFromRecipeId` 0x824b9860: the `aiprofile` record's name.
    pub(crate) profile_name: String,
    /// `ChallengeSignupEquivalent`, unless it names `default`.
    pub(crate) equivalent: Option<u64>,
}

/// `Character::MarqueeDefs::mCharacterMarqueeList` (`Initialise` 0x82cfdbc8: the class's leaf
/// collections) with what `IsRecipeUsable` 0x824bb268 checks.
#[derive(Clone, Debug, Default)]
pub(crate) struct RecipeCatalog {
    pub(crate) recipes: Vec<MarqueeRecipe>,
    unlockables: Option<Unlockables>,
    /// Recipes the player's skater or the living world use (`IsRecipeUsedByPlayer`,
    /// `IsRecipeUsedByLivingWorld`).
    pub(crate) in_use: Vec<u64>,
}

impl RecipeCatalog {
    pub(crate) fn try_load(
        vault: &Arc<AttributeRuntimeVault>,
        unlockables: Option<Unlockables>,
    ) -> std::result::Result<Self, AttributeRuntimeError> {
        let mut recipes = Vec::new();
        for id in vault.leaf_collections_sorted(CHARACTERS_MARQUEE)? {
            let instance = vault.instance(CHARACTERS_MARQUEE, id);
            let name = instance
                .try_cached_layout_text(0)?
                .map(|text| String::from_utf8_lossy(text.bytes().strip_suffix(&[0]).unwrap_or(text.bytes())).into_owned())
                .unwrap_or_default();
            let flags = instance.try_cached_layout_field(8, 10)?.map(|data| data.bytes().to_vec()).unwrap_or_else(|| vec![0; 10]);
            let profile_name = match instance.try_ref_spec_instance(AI_PROFILE)? {
                Some(profile) => profile
                    .try_cached_layout_text(0)?
                    .map(|text| String::from_utf8_lossy(text.bytes().strip_suffix(&[0]).unwrap_or(text.bytes())).into_owned())
                    .unwrap_or_else(|| "default".to_owned()),
                None => "default".to_owned(),
            };
            let equivalent = instance
                .try_ref_spec_collection(CHALLENGE_SIGNUP_EQUIVALENT)?
                .filter(|key| *key as u32 != DEFAULT_COLLECTION_LOW);
            recipes.push(MarqueeRecipe {
                id,
                name,
                usable: flags[1] != 0,
                ambient_only: flags[9] != 0,
                observed_character: i32::from_be_bytes([flags[4], flags[5], flags[6], flags[7]]),
                profile_name,
                equivalent,
            });
        }
        Ok(Self { recipes, unlockables, in_use: Vec::new() })
    }

    fn recipe(&self, id: u64) -> Option<&MarqueeRecipe> {
        self.recipes.iter().find(|recipe| recipe.id as u32 == id as u32)
    }

    /// The character an observed AI skater reports (`GetCharacterFromObservedIndex` 0x824ba0f8):
    /// its `characters_marquee` layout word 3, 97 without one.
    pub(crate) fn observed_character(&self, id: u64) -> i32 {
        self.recipe(id).map_or(97, |recipe| recipe.observed_character)
    }

    /// `GetProfileNameFromRecipeId`.
    /// `IsRecipeUnlocked` 0x824b3420 by recipe; an unknown recipe is not.
    pub(crate) fn is_recipe_id_unlocked(&self, id: u64) -> bool {
        self.recipe(id).is_some_and(|recipe| self.is_recipe_unlocked(recipe))
    }

    pub(crate) fn profile_name(&self, id: u64) -> &str {
        self.recipe(id).map_or("default", |recipe| recipe.profile_name.as_str())
    }

    /// `IsRecipeUnlocked` 0x824b3420.
    fn is_recipe_unlocked(&self, recipe: &MarqueeRecipe) -> bool {
        if !recipe.usable {
            return false;
        }
        if recipe.ambient_only {
            return cvars::ARE_AMBIENT_ONLY_SKATERS_ENABLED;
        }
        if cvars::SKIP_IS_RECIPE_UNLOCKED_TEST {
            return true;
        }
        self.unlockables.as_ref().is_some_and(|unlockables| {
            unlockables.is_unlocked(unlockables::category::CHARACTERS, unlockables::string_hash(&recipe.name), true)
        })
    }

    /// `IsRecipeUsable` 0x824bb268.
    pub(crate) fn is_recipe_usable(&self, id: u64) -> bool {
        let Some(recipe) = self.recipe(id) else { return false };
        let used = |id: u64| self.in_use.iter().any(|used| *used as u32 == id as u32);
        self.is_recipe_unlocked(recipe) && !used(recipe.id) && !recipe.equivalent.is_some_and(used)
    }
}

/// A player slot in `PlayersBackEnd`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PlayerSlot {
    pub(crate) player_type: u32,
    pub(crate) character: Option<u64>,
}

/// `AISkaterManager::mSkaterDistanceCallbacks` entry (12 bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
struct DistanceCallback {
    skater_id: i32,
    dist_sqr: f32,
    dist_sqr_previous: f32,
}

impl DistanceCallback {
    const UNUSED: Self = Self { skater_id: -1, dist_sqr: 0.0, dist_sqr_previous: -1.0 };
}

/// A callback's skater crossing its radius around the local skater: `cMsgPlayerRadiusEnter`
/// coming inside, `cMsgPlayerRadiusExit` going out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RadiusCrossing {
    pub(crate) enter: bool,
    /// `fDistance`: the callback's squared distance.
    pub(crate) distance: f32,
    pub(crate) player_id: i32,
}

/// One Actor of `SkaterManager::mSkaterList` as the manager reads it.
#[derive(Clone, Debug)]
pub(crate) struct ManagedSkater {
    pub(crate) player_id: i32,
    /// `Actor::GetPosition` 0x825f9df0: the skeleton root.
    pub(crate) position: V4,
    pub(crate) ai: Option<ManagedAi>,
}

#[derive(Clone, Debug)]
pub(crate) struct ManagedAi {
    pub(crate) client_type: AiClientType,
    pub(crate) recipe_id: u64,
    pub(crate) has_behavior: bool,
    pub(crate) fading_out: bool,
    /// `Actor::GetOpacity`.
    pub(crate) opacity: f32,
}

/// What the manager asks of the `SkaterManager`, in call order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum AmbientCommand {
    /// `cMsgSpawnAi` → `SkaterManager::SpawnAi` 0x82600488.
    Spawn { slot: usize, client_type: AiClientType, recipe: u64, transform: [V4; 4], profile_name: String },
    /// `cMsgUnspawnAi`.
    Unspawn { slot: usize },
    /// `AIController::SetBehavior<WanderBehavior>`.
    SetWander { slot: usize },
    /// `MotionController::ExternalWipeoutRequest`.
    WipeoutRequest { slot: usize },
}

/// The world a manager update reads.
pub(crate) struct AmbientFrame<'a> {
    /// `FEBackEnd::Data.mFEState == eFE_STATE_IN_FE`.
    pub(crate) in_front_end: bool,
    /// `sGameInfo.gameMode == GM_PartyPlay`.
    pub(crate) party_play: bool,
    /// PC port: a profile free skate whose AI Skaters setting is off
    /// (`ui::freeskate_option::Settings::ai_skaters`).
    pub(crate) ambient_off: bool,
    /// `GetCameraReferenceMatrix` 0x824ba008: the presented camera in the front end, the game
    /// camera otherwise.
    pub(crate) camera: [V4; 4],
    pub(crate) skaters: &'a [ManagedSkater],
    pub(crate) paths: &'a PathManager,
    pub(crate) vault: &'a Arc<AttributeRuntimeVault>,
    pub(crate) catalog: &'a RecipeCatalog,
    pub(crate) locked: &'a mut LockedPathTest<'a>,
}

#[derive(Debug)]
pub(crate) enum AiSkaterManagerError {
    Paths(PathManagerError),
    Attributes(AttributeRuntimeError),
    Random(crate::native_skater_random::rw_math::RwRandomLockError),
    /// `LoadSkaterRecipes` asserted: no asset could take the recipe.
    NoAssetFor(u64),
    /// `SetupAssetForSkater` asserted: the challenge character has no recipe, or it was never
    /// requested.
    UnrequestedRecipe(Option<u64>),
}

impl std::fmt::Display for AiSkaterManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Paths(error) => write!(f, "AI skater manager paths: {error}"),
            Self::Attributes(error) => write!(f, "AI skater manager attributes: {error:?}"),
            Self::Random(error) => write!(f, "AI skater manager random: {error:?}"),
            Self::NoAssetFor(recipe) => write!(f, "AI skater manager: no asset for recipe {recipe:#018x}"),
            Self::UnrequestedRecipe(recipe) => write!(f, "AI skater manager: challenge recipe {recipe:?} was not requested"),
        }
    }
}

impl std::error::Error for AiSkaterManagerError {}

impl From<PathManagerError> for AiSkaterManagerError {
    fn from(error: PathManagerError) -> Self {
        Self::Paths(error)
    }
}

impl From<AttributeRuntimeError> for AiSkaterManagerError {
    fn from(error: AttributeRuntimeError) -> Self {
        Self::Attributes(error)
    }
}

type Result<T> = std::result::Result<T, AiSkaterManagerError>;

fn rw_random() -> Result<u32> {
    crate::native_skater_random::rw_math::global_next().map_err(AiSkaterManagerError::Random)
}

/// `AI::AISkaterManager`.
#[derive(Clone, Debug)]
pub(crate) struct AiSkaterManager {
    pub(crate) assets: SkaterAssetManager,
    pub(crate) players: [PlayerSlot; 9],
    update_count: u32,
    min_update_count_for_next_model_swap: u32,
    max_ambient_skaters_based_on_framerate: u32,
    pub(crate) model_loading_mode: ModelLoadingMode,
    in_an_ai_based_challenge: bool,
    in_an_ambient_skater_challenge: bool,
    max_ambient_skaters_in_ambient_challenge: u32,
    in_an_online_game: bool,
    has_challenge_asset_request_occurred: bool,
    pub(crate) allow_ambient_skater_spawn: bool,
    /// The `cMsgSpawnAi`/`cMsgUnspawnAi` messages the `SkaterManager` takes after the frame.
    pub(crate) pending: Vec<AmbientCommand>,
    /// `mSkaterDistanceCallbacks`.
    distance_callbacks: [DistanceCallback; 5],
}

impl Default for AiSkaterManager {
    /// `AISkaterManager::AISkaterManager` 0x824c2dc8.
    fn default() -> Self {
        Self {
            assets: SkaterAssetManager::default(),
            players: [PlayerSlot::default(); 9],
            update_count: 0,
            min_update_count_for_next_model_swap: 0,
            max_ambient_skaters_based_on_framerate: 0,
            model_loading_mode: ModelLoadingMode::MinimalSet,
            in_an_ai_based_challenge: false,
            in_an_ambient_skater_challenge: false,
            max_ambient_skaters_in_ambient_challenge: 0,
            in_an_online_game: false,
            has_challenge_asset_request_occurred: false,
            allow_ambient_skater_spawn: true,
            pending: Vec::new(),
            distance_callbacks: [DistanceCallback::UNUSED; 5],
        }
    }
}

impl AiSkaterManager {
    /// `RegisterSkaterDistanceCallbackMessage` 0x824b2d08: the first free slot for the skater at
    /// the distance, or -1.
    pub(crate) fn register_skater_distance_callback(&mut self, skater: i32, distance: f32) -> i32 {
        let Some(slot) = self.distance_callbacks.iter().position(|callback| callback.skater_id == -1) else { return -1 };
        self.distance_callbacks[slot] = DistanceCallback { skater_id: skater, dist_sqr: distance * distance, dist_sqr_previous: -1.0 };
        i32::try_from(slot).unwrap_or(-1)
    }

    /// `UnregisterSkaterDistanceCallbackMessage` 0x824b2da8.
    pub(crate) fn unregister_skater_distance_callback(&mut self, id: u32) {
        if let Some(callback) = usize::try_from(id).ok().and_then(|id| self.distance_callbacks.get_mut(id)) {
            callback.skater_id = -1;
        }
    }

    /// `ResetDistanceCallbacks` 0x824b2940.
    pub(crate) fn reset_distance_callbacks(&mut self) {
        for callback in &mut self.distance_callbacks {
            callback.dist_sqr_previous = -1.0;
            callback.skater_id = -1;
        }
    }

    /// `UpdateSkaterDistanceCallbackMessages` 0x824c8470: each callback skater's squared distance
    /// to the local skater (`Actor::GetPosition`), and the crossings of its radius since the
    /// callback's last measure. `skaters` is the skater manager's list.
    pub(crate) fn update_skater_distance_callbacks(&mut self, local_index: i32, skaters: &[ManagedSkater]) -> Vec<RadiusCrossing> {
        let mut crossings = Vec::new();
        let find = |player: i32| skaters.iter().find(|skater| skater.player_id == player).map(|skater| skater.position);
        if u32::try_from(local_index).is_ok_and(|local| local < 8)
            && let Some(local) = find(local_index)
        {
            let count = i32::try_from(skaters.len()).unwrap_or(i32::MAX);
            for callback in &mut self.distance_callbacks {
                let player_id = callback.skater_id;
                if player_id >= count || player_id == -1 {
                    continue;
                }
                let Some(position) = find(player_id) else { continue };
                let offset = vmx::sub(position, local);
                let current = vmx::dot3(offset, offset);
                let distance = callback.dist_sqr;
                if callback.dist_sqr_previous != -1.0 {
                    let inside = current <= distance;
                    let was_outside = callback.dist_sqr_previous > distance;
                    if !(inside || was_outside) {
                        crossings.push(RadiusCrossing { enter: false, distance, player_id });
                    } else if inside && was_outside {
                        crossings.push(RadiusCrossing { enter: true, distance, player_id });
                    }
                }
                callback.dist_sqr_previous = current;
            }
        }
        crossings
    }

    /// `LoadGame` 0x824b2b20.
    pub(crate) fn load_game(&mut self, online: bool) {
        self.update_count = 0;
        self.in_an_online_game = online;
        self.min_update_count_for_next_model_swap = 0;
        self.in_an_ai_based_challenge = false;
        self.in_an_ambient_skater_challenge = false;
        self.has_challenge_asset_request_occurred = false;
    }

    /// `IsInAnOnlineGame` 0x824b2a00.
    pub(crate) const fn is_in_an_online_game(&self) -> bool {
        self.in_an_online_game
    }

    /// `HasChallengeAssetRequestOccured` 0x824b2b18.
    pub(crate) const fn has_challenge_asset_request_occurred(&self) -> bool {
        self.has_challenge_asset_request_occurred
    }

    /// `CanAmbientSkatersBeSpawned` 0x824b96b0.
    pub(crate) fn can_ambient_skaters_be_spawned(&self, party_play: bool) -> bool {
        self.allow_ambient_skater_spawn
            && cvars::ENABLE_AMBIENT_SKATERS
            && (!party_play || !cvars::BAN_AMBIENT_SKATERS_IN_PARTY_PLAY)
    }

    /// `EnterAiBasedChallenge` 0x824ce660: entering clears the asset request, and the ambient
    /// skaters go.
    pub(crate) fn enter_ai_based_challenge(&mut self, party_play: bool) -> Vec<AmbientCommand> {
        let mut commands = Vec::new();
        if !self.in_an_online_game && self.can_ambient_skaters_be_spawned(party_play) {
            if !self.in_an_ai_based_challenge {
                self.has_challenge_asset_request_occurred = false;
                self.in_an_ai_based_challenge = true;
            }
            self.unspawn_all_ambient_skaters(&mut commands);
        }
        commands
    }

    /// `ExitAiBasedChallenge` 0x824cff60 after its `ConvertAllChallangeSkatersToAmbient` and
    /// `UnloadExcessSkaterModels`, which the session runs on the Actors: the update count goes
    /// back to its last 60-frame boundary.
    pub(crate) fn exit_ai_based_challenge(&mut self) {
        if self.in_an_ai_based_challenge {
            self.update_count -= self.update_count % 60;
        }
        self.in_an_ai_based_challenge = false;
        self.has_challenge_asset_request_occurred = false;
    }

    /// Whether `ExitAiBasedChallenge` converts and unloads: in an AI-based challenge.
    pub(crate) const fn in_an_ai_based_challenge(&self) -> bool {
        self.in_an_ai_based_challenge
    }

    /// `ConvertToAmbientSkater` 0x824cea88's room test: fewer ambient skaters than the maximum.
    pub(crate) fn has_room_for_ambient_skater(&self) -> bool {
        self.player_type_count(player_type::AI_AMBIENT) < self.max_ambient_skaters()
    }

    /// `ConvertSkaterToAmbient` 0x824b9c40's `ConvertAiPlayerType(slot, PT_AI_Ambient)` in the
    /// manager's player view.
    pub(crate) fn convert_to_ambient_player(&mut self, slot: usize) {
        self.players[slot].player_type = player_type::AI_AMBIENT;
    }

    /// `GetFurthestAmbientSkaterId` 0x824b9a00: the ambient-controlled skater with an ambient
    /// player row furthest from the position.
    fn furthest_ambient_skater_id(&self, position: V4, skaters: &[ManagedSkater]) -> Option<usize> {
        let mut furthest = None;
        let mut largest = 0.0;
        for skater in skaters {
            if !skater.ai.as_ref().is_some_and(|ai| ai.client_type == AiClientType::Ambient) {
                continue;
            }
            let Ok(slot) = usize::try_from(skater.player_id) else { continue };
            if self.players.get(slot).is_none_or(|row| row.player_type != player_type::AI_AMBIENT) {
                continue;
            }
            let offset = vmx::sub(position, skater.position);
            let squared = vmx::dot3(offset, offset);
            if squared > largest {
                largest = squared;
                furthest = Some(slot);
            }
        }
        furthest
    }

    /// `UnloadExcessSkaterModels` 0x824ce6d0: past the ambient model count, unused models go
    /// first (any unused one, then the furthest ambient skater's, unspawning it); each unused
    /// model the ambient set no longer wants swaps for an unlocked usable skater or goes; below
    /// the count, unlocked usable skaters load.
    pub(crate) fn unload_excess_skater_models(
        &mut self,
        catalog: &RecipeCatalog,
        camera: V4,
        skaters: &[ManagedSkater],
    ) -> Result<Vec<AmbientCommand>> {
        let mut commands = Vec::new();
        let max = self.max_ambient_skater_models() as usize;
        while self.assets.num_active_assets > max {
            let recipe = match self.assets.unused_unloadable_recipe_id(catalog).or_else(|| self.assets.currently_unused_recipe_id(catalog, true)) {
                Some(recipe) => Some(recipe),
                None => {
                    let Some(slot) = self.furthest_ambient_skater_id(camera, skaters) else { break };
                    let recipe = self.players[slot].character;
                    self.unspawn_ai_skater(slot, &mut commands);
                    recipe
                }
            };
            let Some(recipe) = recipe else { break };
            self.assets.free_skater_recipe(recipe);
        }
        while let Some(recipe) = self.assets.unused_unloadable_recipe_id(catalog) {
            match self.assets.an_unlocked_usable_skater(catalog)? {
                Some(new) => {
                    self.assets.replace_skater_recipe(recipe, new);
                }
                None => {
                    self.assets.free_skater_recipe(recipe);
                }
            }
        }
        let active = self.assets.num_active_assets;
        if active < max {
            for _ in 0..max - active {
                if let Some(new) = self.assets.an_unlocked_usable_skater(catalog)? {
                    self.assets.load_skater_recipe(new);
                }
            }
        }
        Ok(commands)
    }

    /// `LoadAIChallengeAssets` 0x824cb810.
    pub(crate) fn load_ai_challenge_assets(&mut self, recipes: &[u64]) -> Result<()> {
        self.assets.load_skater_recipes(recipes)?;
        self.has_challenge_asset_request_occurred = true;
        Ok(())
    }

    /// `AreAllAssetsLoaded` 0x824c31e8: the assets update, then every active one is loaded.
    pub(crate) fn are_all_assets_loaded(&mut self) -> bool {
        self.assets.update();
        self.assets.all_loaded()
    }

    /// `SpawnAISkater` 0x824cba18 for a challenge skater (`AC_CHALLENGE`, no recipe given):
    /// `SetupAssetForSkater` 0x824c8260 registers the recipe `ChallengeAIManager::
    /// GetCharacterRecipe` names, which must have been requested. False when a human holds the
    /// slot.
    pub(crate) fn spawn_challenge_skater(
        &mut self,
        slot: usize,
        recipe: Option<u64>,
        transform: [V4; 4],
        profile_name: String,
    ) -> Result<Option<AmbientCommand>> {
        if player_type::is_ai(self.players[slot].player_type) {
            if let Some(character) = self.players[slot].character {
                self.assets.unregister_skater_recipe(character);
            }
            self.players[slot] = PlayerSlot::default();
        }
        if self.players[slot].player_type != player_type::NONE {
            return Ok(None);
        }
        self.players[slot] = PlayerSlot { player_type: AiClientType::Challenge.player_type(), character: None };
        let recipe = recipe
            .filter(|&recipe| self.assets.has_recipe_been_requested(recipe))
            .ok_or(AiSkaterManagerError::UnrequestedRecipe(recipe))?;
        self.assets.register_skater_recipe(recipe);
        self.players[slot].character = Some(recipe);
        Ok(Some(AmbientCommand::Spawn { slot, client_type: AiClientType::Challenge, recipe, transform, profile_name }))
    }

    /// `UnregisterAllSkaters` 0x824cb868: each AI player's character recipe is unregistered and
    /// cleared, then the player is removed, and the manager's view of it goes too.
    pub(crate) fn unregister_all_skaters(&mut self, settings: &PlayersBackendSettings) -> std::result::Result<(), String> {
        let error = |stage: &str, cause: &dyn std::fmt::Debug| format!("AI {stage}: {cause:?}");
        for slot in 0..9 {
            let player = u32::try_from(slot).map_err(|e| error("player", &e))?;
            if !player_type::is_ai(settings.try_player_type(player).map_err(|e| error("player type", &e))?) {
                continue;
            }
            let character = settings.try_character(player).map_err(|e| error("character", &e))?;
            if character != u64::MAX {
                self.assets.unregister_skater_recipe(character);
                settings.try_set_character(player, u64::MAX).map_err(|e| error("character", &e))?;
            }
            settings.try_remove_player(player).map_err(|e| error("remove player", &e))?;
            self.players[slot] = PlayerSlot::default();
        }
        Ok(())
    }

    /// `UnspawnAISkater` 0x824cbc10 from the challenge system.
    pub(crate) fn unspawn_ai_skater_now(&mut self, slot: usize) -> Vec<AmbientCommand> {
        self.request_unspawn(slot)
    }

    /// The `PlayersBackEnd` half of `SpawnAISkater` 0x824cba18 (the player at
    /// `GetPeerIDFromSkaterID` 0x824b2bc8, its character), which the manager's own player view
    /// already took; the actor message then waits for the frame's end. Unspawn removes the
    /// backend row immediately (`0x824cbd08`), before a replacement can reuse the slot.
    pub(crate) fn post(&mut self, command: AmbientCommand, settings: &PlayersBackendSettings) -> std::result::Result<(), String> {
        if let AmbientCommand::Spawn { slot, client_type, recipe, .. } = &command {
            let error = |stage: &str, cause: &dyn std::fmt::Debug| format!("AI {stage}: {cause:?}");
            let player = u32::try_from(*slot).map_err(|e| error("spawn slot", &e))?;
            if settings.try_player_type(player).map_err(|e| error("player type", &e))? != player_type::NONE {
                settings.try_remove_player(player).map_err(|e| error("remove player", &e))?;
            }
            settings
                .try_create_ai_player(player, client_type.player_type(), i64::from(player) + AI_PEER_OFFSET)
                .map_err(|e| error("create player", &e))?;
            settings.try_set_character(player, *recipe).map_err(|e| error("character", &e))?;
        } else if let AmbientCommand::Unspawn { slot } = &command {
            let error = |stage: &str, cause: &dyn std::fmt::Debug| format!("AI {stage}: {cause:?}");
            let player = u32::try_from(*slot).map_err(|e| error("unspawn slot", &e))?;
            if player_type::is_ai(settings.try_player_type(player).map_err(|e| error("player type", &e))?) {
                settings.try_set_character(player, u64::MAX).map_err(|e| error("character", &e))?;
                settings.try_remove_player(player).map_err(|e| error("remove player", &e))?;
            }
        }
        self.pending.push(command);
        Ok(())
    }

    /// The messages for the `SkaterManager`, in order.
    pub(crate) fn take_pending(&mut self) -> Vec<AmbientCommand> {
        std::mem::take(&mut self.pending)
    }

    /// `EnableAmbientSkaterSpawn` 0x824b2b00.
    pub(crate) fn enable_ambient_skater_spawn(&mut self, enable: bool) {
        self.allow_ambient_skater_spawn = enable;
    }

    fn player_type_count(&self, kind: u32) -> u32 {
        self.players.iter().filter(|slot| slot.player_type == kind).count() as u32
    }

    /// `GetMaxAmbientSkaterModels` 0x824b28b0.
    fn max_ambient_skater_models(&self) -> u32 {
        match self.model_loading_mode {
            ModelLoadingMode::LoadNothing => 0,
            ModelLoadingMode::MinimalSet => cvars::MAX_AMBIENT_SKATER_MODELS_MINIMAL_SET,
            ModelLoadingMode::LoadAll => cvars::MAX_AMBIENT_SKATER_MODELS,
        }
    }

    fn max_ambient_skaters(&self) -> u32 {
        if self.in_an_ambient_skater_challenge {
            self.max_ambient_skaters_in_ambient_challenge
        } else {
            self.max_ambient_skaters_based_on_framerate
        }
    }

    /// `Update` 0x824d0188 without the test skaters, distance callbacks and audio events.
    pub(crate) fn update(&mut self, frame: &mut AmbientFrame<'_>) -> Result<Vec<AmbientCommand>> {
        let mut commands = Vec::new();
        if self.in_an_online_game {
            return Ok(commands);
        }
        self.update_count = self.update_count.wrapping_add(1);
        self.assets.update();
        self.update_ambient_skaters(frame, &mut commands)?;
        Ok(commands)
    }

    /// `ShouldPreventAmbientSkatersCurrently` 0x824b97b8. PC port: also a free skate whose AI
    /// Skaters setting is off, whose ambient skaters then neither load nor spawn.
    fn should_prevent_ambient_skaters_currently(&self, frame: &AmbientFrame<'_>) -> bool {
        !self.allow_ambient_skater_spawn
            || !cvars::ENABLE_AMBIENT_SKATERS
            || (frame.party_play && cvars::BAN_AMBIENT_SKATERS_IN_PARTY_PLAY)
            || frame.ambient_off
            || self.in_an_ai_based_challenge
            || self.player_type_count(player_type::AI_CHALLENGE) != 0
    }

    /// `UpdateAmbientSkaters` 0x824cfe08.
    fn update_ambient_skaters(&mut self, frame: &mut AmbientFrame<'_>, commands: &mut Vec<AmbientCommand>) -> Result<()> {
        if self.should_prevent_ambient_skaters_currently(frame) {
            self.unspawn_all_ambient_skaters(commands);
            return Ok(());
        }
        let phase = self.update_count % 60;
        self.evaluate_max_ambient_skaters();
        if phase == 0 {
            self.update_ambient_skaters_unspawning(frame, commands);
        }
        if cvars::ENABLE_AMBIENT_SKATERS {
            if phase == 15 {
                self.update_ambient_model_loading(frame)?;
            } else if phase == 30 {
                self.update_ambient_skaters_spawning(frame, commands)?;
            } else if (3..13).contains(&phase) || (18..28).contains(&phase) || (33..58).contains(&phase) {
                self.check_for_banned_ambient_skater_usage(frame, commands);
            }
            self.update_ambient_skaters_behaviors(frame, commands);
        }
        Ok(())
    }

    /// `EvaluateMaxAmbientSkaters` 0x824b9238: with equal maximum and slow-sim minimum the
    /// maximum stands.
    fn evaluate_max_ambient_skaters(&mut self) {
        self.max_ambient_skaters_based_on_framerate = cvars::MAX_AMBIENT_SKATERS;
        if cvars::MAX_AMBIENT_SKATERS > cvars::MIN_AMBIENT_SKATERS_WHEN_SIM_SLOW {
            let count = self.player_type_count(player_type::AI_AMBIENT);
            self.max_ambient_skaters_based_on_framerate = count.max(cvars::MIN_AMBIENT_SKATERS_WHEN_SIM_SLOW);
        }
    }

    /// `UpdateAmbientSkatersBehaviors` 0x824b9928.
    fn update_ambient_skaters_behaviors(&self, frame: &AmbientFrame<'_>, commands: &mut Vec<AmbientCommand>) {
        for skater in frame.skaters {
            if let Some(ai) = &skater.ai
                && !ai.has_behavior
                && ai.client_type == AiClientType::Ambient
                && let Ok(slot) = usize::try_from(skater.player_id)
            {
                commands.push(AmbientCommand::SetWander { slot });
            }
        }
    }

    /// `UnspawnAISkater` 0x824cbc10 from outside the update (`Actor::TeleportSomewhereAppropriate`).
    pub(crate) fn request_unspawn(&mut self, slot: usize) -> Vec<AmbientCommand> {
        let mut commands = Vec::new();
        self.unspawn_ai_skater(slot, &mut commands);
        commands
    }

    /// `UnspawnAllAmbientSkaters` 0x824ce550 from outside the update.
    pub(crate) fn request_unspawn_all_ambient(&mut self) -> Vec<AmbientCommand> {
        let mut commands = Vec::new();
        self.unspawn_all_ambient_skaters(&mut commands);
        commands
    }

    /// `UnspawnAISkater` 0x824cbc10.
    fn unspawn_ai_skater(&mut self, slot: usize, commands: &mut Vec<AmbientCommand>) {
        if !player_type::is_ai(self.players[slot].player_type) {
            return;
        }
        commands.push(AmbientCommand::Unspawn { slot });
        if let Some(recipe) = self.players[slot].character {
            self.assets.unregister_skater_recipe(recipe);
        }
        self.players[slot] = PlayerSlot::default();
    }

    /// `UnspawnAllAmbientSkaters` 0x824ce550.
    fn unspawn_all_ambient_skaters(&mut self, commands: &mut Vec<AmbientCommand>) {
        for slot in 0..9 {
            if self.players[slot].player_type == player_type::AI_AMBIENT {
                self.unspawn_ai_skater(slot, commands);
            }
        }
    }

    /// `UpdateAmbientSkatersUnspawning` 0x824ce858: skaters past the unspawn distance or height
    /// from the camera, and any beyond the maximum.
    fn update_ambient_skaters_unspawning(&mut self, frame: &AmbientFrame<'_>, commands: &mut Vec<AmbientCommand>) {
        let mut excess = self.player_type_count(player_type::AI_AMBIENT) as i32 - self.max_ambient_skaters() as i32;
        let camera = frame.camera[3];
        let (distance, height) = if frame.in_front_end {
            (cvars::AMBIENT_UNSPAWN_DISTANCE_IN_FE, cvars::AMBIENT_UNSPAWN_Y_DIFFERENCE_IN_FE)
        } else {
            (cvars::AMBIENT_UNSPAWN_DISTANCE_IN_GAME, cvars::AMBIENT_UNSPAWN_Y_DIFFERENCE_IN_GAME)
        };
        let distance_sqr = distance * distance;
        for skater in frame.skaters {
            if !skater.ai.as_ref().is_some_and(|ai| ai.client_type == AiClientType::Ambient) {
                continue;
            }
            let offset = vmx::sub(camera, skater.position);
            let squared = vmx::dot3(offset, offset);
            let rise = (camera[1] - skater.position[1]).abs();
            if !cvars::ENABLE_AMBIENT_SKATERS || squared > distance_sqr || rise > height || excess > 0 {
                if let Ok(slot) = usize::try_from(skater.player_id) {
                    self.unspawn_ai_skater(slot, commands);
                }
                excess -= 1;
            }
        }
    }

    /// `CheckForBannedAmbientSkaterUsage` 0x824ce0f0: an ambient skater whose recipe became
    /// unusable wipes out, and goes once faded.
    fn check_for_banned_ambient_skater_usage(&mut self, frame: &AmbientFrame<'_>, commands: &mut Vec<AmbientCommand>) {
        for skater in frame.skaters {
            let Some(ai) = &skater.ai else { continue };
            let Ok(slot) = usize::try_from(skater.player_id) else { continue };
            if ai.client_type != AiClientType::Ambient || self.players[slot].player_type != player_type::AI_AMBIENT {
                continue;
            }
            if !frame.catalog.is_recipe_usable(ai.recipe_id) {
                if ai.fading_out && ai.opacity < 0.2 {
                    self.unspawn_ai_skater(slot, commands);
                } else {
                    commands.push(AmbientCommand::WipeoutRequest { slot });
                }
            }
        }
    }

    /// `GetCameraReferenceMatrix`'s position and the spawn distance window.
    fn spawn_window(frame: &AmbientFrame<'_>) -> (f32, f32) {
        if frame.in_front_end {
            (cvars::AMBIENT_SPAWN_MIN_DISTANCE_IN_FE, cvars::AMBIENT_SPAWN_MAX_DISTANCE_IN_FE)
        } else {
            (cvars::AMBIENT_SPAWN_MIN_DISTANCE_IN_GAME, cvars::AMBIENT_SPAWN_MAX_DISTANCE_IN_GAME)
        }
    }

    /// `UpdateAmbientSkatersSpawning` 0x824ce270: the unused loaded recipe and path start in
    /// range with the lowest score, spawned into the first free slot from 1.
    fn update_ambient_skaters_spawning(&mut self, frame: &mut AmbientFrame<'_>, commands: &mut Vec<AmbientCommand>) -> Result<()> {
        let total = frame
            .skaters
            .iter()
            .filter(|skater| skater.ai.as_ref().is_some_and(|ai| ai.client_type != AiClientType::Player))
            .count();
        if total == 7 {
            return Ok(());
        }
        if self.player_type_count(player_type::AI_AMBIENT) >= self.max_ambient_skaters() {
            return Ok(());
        }
        let recipes = self.assets.currently_unused_recipe_ids(frame.catalog, 5, false);
        if recipes.is_empty() {
            return Ok(());
        }
        let camera = frame.camera[3];
        let (min, max) = Self::spawn_window(frame);
        let results = frame.paths.try_get_paths_in_distance_range(
            vmx::xyz(camera),
            min,
            max,
            32,
            PathQueryType::Start,
            Some(frame.paths.ambient_filter()),
            None,
            None,
            frame.locked,
        )?;
        if results.is_empty() {
            return Ok(());
        }
        let mut best_recipe = 0;
        let mut best_score = u32::MAX;
        let mut best_path = results.len();
        for (index, &recipe) in recipes.iter().enumerate() {
            let profile = SkaterProfile::try_new(frame.vault, frame.catalog.profile_name(recipe))?;
            let (path, score) = self.pick_best_ambient_spawn_path(frame, &results, &profile)?;
            if score < best_score {
                best_recipe = index;
                best_score = score;
                best_path = path;
            }
        }
        if best_path >= results.len() {
            return Ok(());
        }
        let result = &results[best_path];
        let transform = path::node_matrix(path::node(result.path.path(), result.node_index));
        let Some(slot) = (1..9).find(|&slot| self.players[slot].player_type == player_type::NONE) else {
            return Ok(());
        };
        let recipe = recipes[best_recipe];
        self.spawn_ai_skater(slot, recipe, transform, frame.catalog.profile_name(recipe).to_owned(), commands);
        Ok(())
    }

    /// `PickBestAmbientSpawnPath` 0x824c81e0: the lowest score's index, the result count when
    /// every path is unusable.
    fn pick_best_ambient_spawn_path(
        &self,
        frame: &AmbientFrame<'_>,
        results: &[PathQueryResult],
        profile: &SkaterProfile,
    ) -> Result<(usize, u32)> {
        let mut best = usize::MAX;
        let mut lowest = u32::MAX;
        for (index, result) in results.iter().enumerate() {
            let score = Self::calculate_ambient_path_score(frame, result, profile)?;
            if score < lowest {
                best = index;
                lowest = score;
            }
        }
        Ok((best, lowest))
    }

    /// `CalculateAmbientPathScore` 0x824c2ee8: unusable paths, front-end paths entirely out of
    /// view and paths starting within 5 m of a skater score the maximum; a skater within 10 m
    /// adds up to 600; then a random 0..400.
    fn calculate_ambient_path_score(frame: &AmbientFrame<'_>, result: &PathQueryResult, profile: &SkaterProfile) -> Result<u32> {
        let path = result.path.path();
        let abilities = profile.try_abilities()?;
        if !super::path_manager::is_usable_by_skater(path, Some(abilities), None)
            || (frame.in_front_end && path::is_entirely_out_of_camera_view(path, &frame.camera))
        {
            return Ok(u32::MAX);
        }
        let start = vmx::v3(path::node(path, 0).position);
        let limit = cvars::SPAWN_MIN_DIST_BETWEEN_ACTORS * cvars::SPAWN_MIN_DIST_BETWEEN_ACTORS;
        let mut nearest = f32::MAX;
        for skater in frame.skaters {
            let offset = vmx::sub(start, skater.position);
            let squared = vmx::dot3(offset, offset);
            if squared < limit {
                return Ok(u32::MAX);
            }
            if squared < nearest {
                nearest = squared;
            }
        }
        let mut crowding = 0;
        if nearest < 100.0 {
            let distance = vmx::sqrt_guarded(nearest);
            crowding = if distance >= 0.5 {
                (((10.0 - distance) * f32::from_bits(0x3dd7_9436)) * 600.0) as i64 as u32
            } else {
                600
            };
        }
        Ok((rw_random()? % 400).wrapping_add(crowding))
    }

    /// `SpawnAISkater` 0x824cba18 for an ambient skater: the backend's player, the recipe's
    /// registration and the `SkaterManager`'s spawn.
    fn spawn_ai_skater(
        &mut self,
        slot: usize,
        recipe: u64,
        transform: [V4; 4],
        profile_name: String,
        commands: &mut Vec<AmbientCommand>,
    ) {
        if player_type::is_ai(self.players[slot].player_type) {
            if let Some(character) = self.players[slot].character {
                self.assets.unregister_skater_recipe(character);
            }
            self.players[slot] = PlayerSlot::default();
        }
        if self.players[slot].player_type != player_type::NONE {
            return;
        }
        self.players[slot] = PlayerSlot { player_type: AiClientType::Ambient.player_type(), character: None };
        self.assets.register_skater_recipe(recipe);
        self.players[slot].character = Some(recipe);
        commands.push(AmbientCommand::Spawn { slot, client_type: AiClientType::Ambient, recipe, transform, profile_name });
    }

    /// `UpdateAmbientModelLoading` 0x824cb4e8.
    fn update_ambient_model_loading(&mut self, frame: &mut AmbientFrame<'_>) -> Result<()> {
        if !self.assets.all_loaded() || self.update_count < self.min_update_count_for_next_model_swap {
            return Ok(());
        }
        let full = self.assets.num_active_assets as u32 >= self.max_ambient_skater_models();
        let swap_allowed = !(cvars::PREVENT_MODEL_SWAP_DURING_MINIMAL_SET && self.model_loading_mode == ModelLoadingMode::MinimalSet)
            && !(cvars::PREVENT_MODEL_SWAP_IN_FRONTEND && frame.in_front_end);
        if full && !swap_allowed {
            return Ok(());
        }
        let unused = self.assets.currently_unused_recipe_ids(frame.catalog, 5, true);
        if unused.is_empty() && full {
            return Ok(());
        }
        let camera = frame.camera[3];
        let (min, max) = Self::spawn_window(frame);
        let near = min - 10.0;
        let near = if 4.0 - near >= 0.0 { 4.0 } else { near };
        let results = frame.paths.try_get_paths_in_distance_range(
            vmx::xyz(camera),
            near,
            max + 10.0,
            32,
            PathQueryType::Start,
            Some(frame.paths.ambient_filter()),
            None,
            None,
            frame.locked,
        )?;
        let unload = self.pick_best_skater_to_unload(frame, &results, &unused)?;
        if unload.is_none() && full {
            return Ok(());
        }
        let valid = self.assets.unlocked_usable_skater_recipe_ids(frame.catalog, false);
        let load = Self::pick_best_skater_to_load(frame, &results, &valid)?;
        if let Some(load) = load
            && unload.is_none_or(|unload| unload as u32 != load as u32)
        {
            match unload {
                None => {
                    self.assets.load_skater_recipe(load);
                }
                Some(unload) => {
                    self.assets.replace_skater_recipe(unload, load);
                    self.min_update_count_for_next_model_swap = self.update_count.wrapping_add(1800);
                }
            }
        }
        Ok(())
    }

    /// `PickBestSkaterToUnload` 0x824cb0d0: of the unused recipes, the most used one no path in
    /// range suits; else the first used twice or no longer usable.
    fn pick_best_skater_to_unload(
        &self,
        frame: &AmbientFrame<'_>,
        results: &[PathQueryResult],
        unused: &[u64],
    ) -> Result<Option<u64>> {
        let mut best: Option<u64> = None;
        let mut most = 0;
        if !results.is_empty() && !unused.is_empty() {
            for &recipe in unused {
                let Some(asset) = self.assets.asset(recipe) else { continue };
                let profile = SkaterProfile::try_new(frame.vault, frame.catalog.profile_name(recipe))?;
                let abilities = profile.try_abilities()?;
                let usable = results
                    .iter()
                    .filter(|result| super::path_manager::is_usable_by_skater(result.path.path(), Some(abilities), None))
                    .count();
                if usable == 0 && (best.is_none() || asset.total_ai_usage_count > most) {
                    best = Some(recipe);
                    most = asset.total_ai_usage_count;
                }
            }
            if best.is_some() {
                return Ok(best);
            }
        }
        for &recipe in unused {
            let Some(asset) = self.assets.asset(recipe) else { continue };
            if asset.total_ai_usage_count >= 2 || !frame.catalog.is_recipe_usable(recipe) {
                return Ok(Some(recipe));
            }
        }
        Ok(best)
    }

    /// `PickBestSkaterToLoad` 0x824cb358: the highest of a random 0..160, 500 when any path in
    /// range suits, 5 per suitable path and 40 for a flagged skater.
    fn pick_best_skater_to_load(frame: &AmbientFrame<'_>, results: &[PathQueryResult], valid: &[u64]) -> Result<Option<u64>> {
        let mut best = None;
        let mut highest = 0u32;
        for &recipe in valid {
            let profile = SkaterProfile::try_new(frame.vault, frame.catalog.profile_name(recipe))?;
            let abilities = profile.try_abilities()?;
            let usable = results
                .iter()
                .filter(|result| super::path_manager::is_usable_by_skater(result.path.path(), Some(abilities), None))
                .count() as u32;
            let flagged = (0..=42).contains(&abilities.allowed_skater_flag);
            let mut score = rw_random()? % 160 + if usable == 0 { 0 } else { 500 } + 5 * usable;
            if flagged {
                score += 40;
            }
            if score > highest {
                best = Some(recipe);
                highest = score;
            }
        }
        Ok(best)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_asset_loads_on_the_second_update_after_its_request() {
        let mut assets = SkaterAssetManager::default();
        assert!(assets.load_skater_recipe(7));
        assets.update();
        assert_eq!(assets.assets[0].status, AssetStatus::Loading);
        assets.update();
        assert_eq!(assets.assets[0].status, AssetStatus::Loaded);
        assets.register_skater_recipe(7);
        assert_eq!(assets.asset(7).map(|asset| asset.current_ai_usage_count), Some(1));
    }

    #[test]
    fn freeing_swaps_the_last_asset_into_place() {
        let mut assets = SkaterAssetManager::default();
        assets.load_skater_recipe(1);
        assets.load_skater_recipe(2);
        assert!(assets.free_skater_recipe(1));
        assert_eq!(assets.num_active_assets, 1);
        assert_eq!(assets.assets[0].recipe_id, Some(2));
    }
}
