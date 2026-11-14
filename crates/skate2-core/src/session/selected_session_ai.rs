//! The session's `AI::AISkaterManager`, each AI skater's `AIController`, and the AI skaters'
//! owners. `SkaterManager::SpawnAi` 0x82600488 builds an AI skater as `SpawnPlayer` builds a
//! later player, then gives its Actor the AI controller; `UnspawnAi` destroys it again.
use super::{SelectedSession, party_skaters::PartyCharacter};
use crate::{
    actor_construction::selected_tick::{SelectedTickAi, SelectedTickSkater},
    actor_graph_construction::ActorGraphIntents,
    ai::{
        actor::{AiClientType, can_see_skater},
        attributes::AiSkaterAttributes,
        controller::AiController,
        living_world_obstacles::{self, ObstacleQuery},
        obstacle_avoider::ObstacleSkater,
        path,
        path_controller::{PathController, PathControllerContext, RespawnContext, SkaterPathUsage, challenge_type},
        path_manager::{LockedPathTest, PathManager, PathQueryType},
        profile::SkaterProfile,
        skater_manager::{
            self, AiSkaterManager, AmbientCommand, AmbientFrame, ManagedAi, ManagedSkater, RecipeCatalog,
            conversion, player_type,
        },
        tricks::TrickTable,
        vmx::{self, V4},
    },
    attribute_runtime::AttributeRuntimeVault,
    challenge::unlockables::Unlockables,
    physical_player_assembly::PhysicalAssemblyWorld,
    player::mounted_recipe::{MountedPlayerRecipeLoad, PreparedRecipe},
    players_backend_settings::PlayersBackendSettings,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

/// `scoring2_general`, its `default` collection, and the banked points above which
/// `UpdateAudioEvents` calls a trick big (its key 0x4beedca4_ad799796).
const SCORING2_GENERAL_CLASS: u64 = 0x3492_15e2_e817_703c;
const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;
const BIG_TRICK_SCORE: u64 = 0x4bee_dca4_ad79_9796;
/// `physics_skeleton` `SmallObjectMassThreshold`, under which `AddDynamicObstacles` passes an
/// object by.
const SMALL_OBJECT_MASS_THRESHOLD: u64 = 0x9ab5_7b38_d5bb_9449;


fn error(stage: &str, cause: impl std::fmt::Debug) -> String {
    format!("AI {stage}: {cause:?}")
}

/// `PathManager::IsPathLocked`'s volumes: no challenge locks a path here.
fn no_locked_volumes(_: [f32; 3], _: f32) -> Vec<([f32; 4], [f32; 4])> {
    Vec::new()
}

pub(crate) struct SessionAi {
    pub(crate) paths: Arc<Mutex<PathManager>>,
    /// The `AISkaterManager` singleton, which the challenge system shares.
    pub(crate) manager: Arc<Mutex<AiSkaterManager>>,
    tricks: TrickTable,
    attributes: AiSkaterAttributes,
    catalog: RecipeCatalog,
    big_trick_score: i32,
    small_object_mass_threshold: f32,
    pub(crate) controllers: BTreeMap<i32, AiController>,
    /// `GetCameraReferenceMatrix` 0x824ba008: the presented camera in the front end, the game
    /// camera otherwise.
    pub(crate) camera: [V4; 4],
    pub(crate) in_front_end: bool,
    pub(crate) party_play: bool,
    /// PC port: the profile free skate's AI Skaters setting is off (`AmbientFrame::ambient_off`).
    pub(crate) ambient_off: bool,
    /// `SkaterManager::GetLocalSimIndex` 0x825eddb8: the local player the AI reads.
    pub(crate) local_player: Option<i32>,
    /// `AI::GetChallengeType` 0x824bf008: the main challenge's `ChallengeType`, 52 without one.
    pub(crate) challenge_type: i32,
    /// `ChallengeAIManager::GetRespawnMatrix` by index for its respawn location.
    pub(crate) respawn_locations: Vec<[V4; 4]>,
    /// `ChallengeAIManager::mRespawnLocation` is set.
    pub(crate) respawn_location_set: bool,
    /// The race manager's racers and their estimated times to finish.
    pub(crate) race_estimated_times: Vec<(i32, f32)>,
    /// The `SkaterAssetManager` recipes' streamed scenes.
    pub(crate) scenes: AiSceneLoads,
    /// Spawns whose character is still loading: `SkaterManager::SpawnAi`'s PlayerLoader
    /// builds the skater once it has loaded.
    deferred: Vec<AmbientCommand>,
    /// The path packages being read, oldest first.
    pub(crate) path_packages: Vec<PathPackageLoad>,
}

/// One `LoadBinaryPathPackage` (`0x824b8f28`) arena on the stream reader: as the native's
/// `cAssetActivationManager::LoadArenaAsset`, it arrives frames after it was asked for.
pub(crate) struct PathPackageLoad {
    pub(crate) file: String,
    pub(crate) bytes: Arc<std::sync::OnceLock<Result<Vec<u8>, String>>>,
}

/// `AI::SkaterAsset`'s `CharacterRecipeLoader`s: each recipe the asset manager holds has its
/// marquee scene made on a loader thread, resident while the asset keeps the recipe, so a
/// spawn finds its character loaded.
#[derive(Default)]
pub(crate) struct AiSceneLoads {
    loads: BTreeMap<u64, AiSceneLoad>,
}

struct AiSceneLoad {
    /// The actual marquee's schema, which the selected load streams.
    schema_name: String,
    scene: AiSceneState,
}

enum AiSceneState {
    Loading(JoinHandle<Option<Arc<PreparedRecipe>>>),
    Loaded(Option<Arc<PreparedRecipe>>),
}

impl AiSceneLoads {
    /// Whether the recipe's scene is still streaming.
    fn loading(&self, recipe: u64) -> bool {
        self.loads
            .get(&recipe)
            .is_some_and(|load| matches!(&load.scene, AiSceneState::Loading(job) if !job.is_finished()))
    }

    /// The recipes whose scene loads have finished.
    fn streamed(&self) -> std::collections::BTreeSet<u64> {
        self.loads
            .iter()
            .filter(|(_, load)| !matches!(&load.scene, AiSceneState::Loading(job) if !job.is_finished()))
            .map(|(&recipe, _)| recipe)
            .collect()
    }

    /// Starts each newly held recipe's load and releases the recipes the assets let go.
    fn update(&mut self, recipes: &[u64], mut start: impl FnMut(u64) -> Option<AiSceneLoad>) {
        self.loads.retain(|recipe, _| recipes.contains(recipe));
        for &recipe in recipes {
            if !self.loads.contains_key(&recipe) {
                // A load that cannot start leaves the spawn its own synchronous load.
                let load = start(recipe)
                    .unwrap_or(AiSceneLoad { schema_name: String::new(), scene: AiSceneState::Loaded(None) });
                self.loads.insert(recipe, load);
            }
        }
    }

    /// The recipe's scene with its schema, waiting for a load still streaming.
    fn scene(&mut self, recipe: u64) -> Option<(String, Arc<PreparedRecipe>)> {
        let load = self.loads.get_mut(&recipe)?;
        if matches!(load.scene, AiSceneState::Loading(_)) {
            let AiSceneState::Loading(job) = std::mem::replace(&mut load.scene, AiSceneState::Loaded(None)) else {
                unreachable!("checked loading state");
            };
            load.scene = AiSceneState::Loaded(job.join().ok().flatten());
        }
        let AiSceneState::Loaded(scene) = &load.scene else {
            return None;
        };
        scene.as_ref().map(|scene| (load.schema_name.clone(), Arc::clone(scene)))
    }
}

impl SessionAi {
    pub(crate) fn try_new(
        paths: Arc<Mutex<PathManager>>,
        vault: &Arc<AttributeRuntimeVault>,
        unlockables: Option<Unlockables>,
    ) -> Result<Self, String> {
        Ok(Self {
            paths,
            manager: Arc::new(Mutex::new(AiSkaterManager::default())),
            tricks: TrickTable::try_load(vault).map_err(|e| error("tricks", e))?,
            attributes: AiSkaterAttributes::try_load(vault).map_err(|e| error("attributes", e))?,
            catalog: RecipeCatalog::try_load(vault, unlockables).map_err(|e| error("recipes", e))?,
            big_trick_score: vault
                .instance(SCORING2_GENERAL_CLASS, DEFAULT_COLLECTION)
                .try_scalar_word(BIG_TRICK_SCORE, 0)
                .map_err(|e| error("big trick score", e))?
                .map_or(0, u32::cast_signed),
            small_object_mass_threshold: vault
                .instance(
                    skate2_formats::attrib::string_hash64("physics_skeleton"),
                    skate2_formats::attrib::string_hash64("default"),
                )
                .try_scalar_word(SMALL_OBJECT_MASS_THRESHOLD, 0)
                .map_err(|e| error("small object mass threshold", e))?
                .map_or(0.0, f32::from_bits),
            controllers: BTreeMap::new(),
            camera: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
            in_front_end: false,
            party_play: false,
            ambient_off: false,
            local_player: None,
            challenge_type: challenge_type::NONE,
            respawn_locations: Vec::new(),
            respawn_location_set: false,
            race_estimated_times: Vec::new(),
            scenes: AiSceneLoads::default(),
            deferred: Vec::new(),
            path_packages: Vec::new(),
        })
    }

    pub(crate) fn manager(&self) -> std::sync::MutexGuard<'_, AiSkaterManager> {
        self.manager.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Recipes the player's skater or the living world use.
    pub(crate) fn set_recipes_in_use(&mut self, recipes: Vec<u64>) {
        self.catalog.in_use = recipes;
    }

    /// The marquee recipes the living world uses (`IsRecipeUsedByLivingWorld` 0x824bb120:
    /// `RecipeMan::IsLivingWorldModelTracked` of the marquee's name).
    pub(crate) fn recipes_used_by_living_world(&self, tracked: impl Fn(&str) -> bool) -> Vec<u64> {
        self.catalog.recipes.iter().filter(|recipe| tracked(&recipe.name)).map(|recipe| recipe.id).collect()
    }

    pub(crate) fn has_controller(&self, player: i32) -> bool {
        self.controllers.contains_key(&player)
    }

    /// `Actor::GetAIOut` 0x825ed0e8: the AI controller's `AIOut_Audio` words.
    pub(crate) fn controller_audio(&self, player: i32) -> Option<[u32; 10]> {
        self.controllers.get(&player).map(|controller| controller.audio.words)
    }

    /// `UnspawnAllAmbientSkaters` 0x824ce550, as `GameModule::GameUnload` leaves the front end.
    /// Unspawn removes its backend row immediately; ShouldSubmit 0x825f0008
    /// excludes that Actor from presentation until the queued destruction runs.
    pub(crate) fn try_unspawn_all_ambient(&mut self, settings: &PlayersBackendSettings) -> Result<(), String> {
        let mut manager = self.manager();
        for command in manager.request_unspawn_all_ambient() {
            manager.post(command, settings)?;
        }
        Ok(())
    }

    fn take_pending(&mut self) -> Vec<AmbientCommand> {
        self.manager().take_pending()
    }
}

/// `UnspawnAISkater`'s `SetPlayerCharacter(-1)` and `RemovePlayer` for an AI row.
fn release_player(settings: &PlayersBackendSettings, slot: usize) -> Result<(), String> {
    let player = u32::try_from(slot).map_err(|e| error("unspawn slot", e))?;
    if player_type::is_ai(settings.try_player_type(player).map_err(|e| error("player type", e))?) {
        settings.try_set_character(player, u64::MAX).map_err(|e| error("character", e))?;
        settings.try_remove_player(player).map_err(|e| error("remove player", e))?;
    }
    Ok(())
}

impl SessionAiFrame<'_> {
    /// `AISkaterManager::GetCharacterFromObservedIndex` 0x824ba0f8: 0 for a human, else its
    /// character's reported character.
    fn observed_character(&self, player: i32) -> Result<i32, String> {
        let slot = u32::try_from(player).map_err(|e| error("observed player", e))?;
        if self.settings.try_player_type(slot).map_err(|e| error("player type", e))? == player_type::HUMAN {
            return Ok(0);
        }
        let character = self.settings.try_character(slot).map_err(|e| error("character", e))?;
        Ok(if character == u64::MAX { 97 } else { self.ai.catalog.observed_character(character) })
    }

    /// `AISkaterManager::UpdateAudioEvents` 0x824c3668: a skater's wipeout, respawn or big trick
    /// this frame reaches each AI skater that sees it (`CheckForAiObservingEvent` 0x824ba350), and
    /// a big trick also marks the trickster's own output.
    fn try_update_audio_events(&mut self, skaters: &[SelectedTickSkater<'_>]) -> Result<(), String> {
        let mut seen = Vec::with_capacity(skaters.len());
        for skater in skaters {
            let player = skater.actor.try_ai_player_id().map_err(|e| error("player", e))?;
            let view = skater.actor.try_ai_actor_view(self.fade_out_time).map_err(|e| error("view", e))?;
            seen.push((player, view.skeleton_root[3], view.just_wiped_out, view.just_respawned, view.banked_point_change_this_frame));
        }
        for (index, &(player, position, wiped_out, respawned, score)) in seen.iter().enumerate() {
            let big_trick = score > self.ai.big_trick_score;
            if !(wiped_out || respawned || big_trick) {
                continue;
            }
            let observed = u32::try_from(index).map_err(|e| error("observed index", e))?;
            let character = self.observed_character(player)?;
            for (other, &(observer, at, ..)) in seen.iter().enumerate() {
                if other == index {
                    continue;
                }
                let Some(controller) = self.ai.controllers.get_mut(&observer) else { continue };
                if !can_see_skater(at, position) {
                    continue;
                }
                if wiped_out {
                    controller.audio.wipeout_seen(observed, character);
                }
                if respawned {
                    controller.audio.respawn_seen(observed, character);
                }
                if big_trick {
                    controller.audio.big_trick_seen(observed, score, character);
                }
            }
            if big_trick && let Some(controller) = self.ai.controllers.get_mut(&player) {
                controller.audio.big_trick_completed(score);
            }
        }
        Ok(())
    }
}

/// One frame's borrow of the session AI for the skater tick.
pub(crate) struct SessionAiFrame<'a> {
    pub(crate) ai: &'a mut SessionAi,
    pub(crate) settings: &'a PlayersBackendSettings,
    pub(crate) vault: &'a Arc<AttributeRuntimeVault>,
    /// `kSkaterFadeOutTime`.
    pub(crate) fade_out_time: f32,
}

impl SelectedTickAi for SessionAiFrame<'_> {
    /// Actor.TeleportSomewhereAppropriate825f0e18 -> AIController824cc208 ->
    /// PathController824ca430, invoked at the reached physics/security query.
    fn try_appropriate_respawn(
        &mut self,
        actor: &crate::actor_construction::ActorConstruction,
        skaters: &[SelectedTickSkater<'_>],
    ) -> Result<Option<crate::skeleton_root_transforms::SkeletonRootMatrix>, String> {
        let player = actor.try_ai_player_id().map_err(|e| error("respawn player", e))?;
        let client = self.ai.controllers.get(&player).ok_or_else(|| format!("AI respawn controller absent for {player}"))?.client_type;
        // Raw825f0e38..74: BOTH AC_AMBIENT and PT_AI_Ambient are required.
        // UnspawnAISkater releases the backend row and queues destruction at
        // this point; the native then still takes the breadcrumb branch.
        if client == AiClientType::Ambient
            && self.settings.try_player_type(u32::try_from(player).map_err(|e| error("respawn slot", e))?)
                .map_err(|e| error("respawn player type", e))? == player_type::AI_AMBIENT
        {
            let mut manager = self.ai.manager();
            for command in manager.request_unspawn(usize::try_from(player).map_err(|e| error("respawn slot", e))?) {
                manager.post(command, self.settings)?;
            }
            if std::env::var_os("SKATE2_TRACE_AI").is_some() {
                eprintln!("ai: appropriate respawn player={player} ambient unspawn; breadcrumbs");
            }
            return Ok(None);
        }
        let cvars = actor.cached_physical_allocation().map_err(|e| error("respawn physical", e))?.owner().inputs().cvars.clone();
        let mut positions = Vec::new();
        if !PathController::respawns_on_path(client, self.ai.respawn_location_set, &cvars).map_err(|e| error("respawn policy", e))? {
            for other in skaters {
                let id = other.actor.try_ai_player_id().map_err(|e| error("respawn obstacle player", e))?;
                if id == player { continue; }
                let view = other.actor.try_ai_actor_view(self.fade_out_time).map_err(|e| error("respawn obstacle", e))?;
                positions.push((id, view.teleport_destination.unwrap_or_else(|| view.path_relative_position())));
            }
        }
        let paths = self.ai.paths.lock().map_err(|_| "AI respawn path manager lock poisoned".to_owned())?;
        let context = RespawnContext {
            cvars: &cvars,
            paths: &paths,
            client_type: client,
            challenge_type: self.ai.challenge_type,
            respawn_location_set: self.ai.respawn_location_set,
            respawn_locations: &self.ai.respawn_locations,
            skater_positions: &positions,
            player,
        };
        self.ai.controllers.get_mut(&player).expect("checked respawn controller").path_controller
            .appropriate_respawn_transform(&context)
            .map(|matrix| matrix.map(|matrix| crate::skeleton_root_transforms::SkeletonRootMatrix::known(
                std::array::from_fn(|lane| matrix[lane / 4][lane % 4]))))
            .map_err(|e| error("appropriate respawn", e))
    }

    /// `AISkaterManager::Update` 0x824d0188: `PreUpdate` 0x824c80e0 clears every controller's
    /// audio output, then the ambient skaters update over the skater list.
    fn try_update_manager(&mut self, world: &mut PhysicalAssemblyWorld, skaters: &[SelectedTickSkater<'_>]) -> Result<(), String> {
        for controller in self.ai.controllers.values_mut() {
            controller.clear_audio();
        }
        let mut managed = Vec::with_capacity(skaters.len());
        for skater in skaters {
            let player = skater.actor.try_ai_player_id().map_err(|e| error("player", e))?;
            let ai = match self.ai.controllers.get(&player) {
                Some(controller) => Some(ManagedAi {
                    client_type: controller.client_type,
                    recipe_id: controller.recipe_id,
                    has_behavior: controller.behavior().is_some(),
                    fading_out: controller.path_controller.is_fading_out,
                    opacity: skater.actor.try_ai_opacity().map_err(|e| error("opacity", e))?,
                }),
                None => None,
            };
            managed.push(ManagedSkater {
                player_id: player,
                position: skater.actor.try_ai_position().map_err(|e| error("position", e))?,
                ai,
            });
        }
        let commands = {
            let paths = self.ai.paths.lock().map_err(|_| "AI path manager lock poisoned".to_owned())?;
            let mut gather = no_locked_volumes;
            let mut locked = LockedPathTest::new(vmx::xyz(self.ai.camera[3]), &mut gather);
            let mut frame = AmbientFrame {
                in_front_end: self.ai.in_front_end,
                party_play: self.ai.party_play,
                ambient_off: self.ai.ambient_off,
                camera: self.ai.camera,
                skaters: &managed,
                paths: &paths,
                vault: self.vault,
                catalog: &self.ai.catalog,
                locked: &mut locked,
            };
            self.ai.manager().update(&mut frame).map_err(|e| error("manager", e))?
        };
        for command in commands {
            match command {
                AmbientCommand::SetWander { slot } => {
                    if let Some(controller) = i32::try_from(slot).ok().and_then(|slot| self.ai.controllers.get_mut(&slot)) {
                        controller.set_wander_behavior();
                    }
                }
                AmbientCommand::WipeoutRequest { slot } => {
                    if let Some(controller) = i32::try_from(slot).ok().and_then(|slot| self.ai.controllers.get_mut(&slot)) {
                        controller.path_controller.external_wipeout_request();
                    }
                }
                command => self.ai.manager().post(command, self.settings)?,
            }
        }
        if let Some(identity) = world.local_skater_identity.clone() {
            let local_index = identity.try_local_index().map_err(|e| error("local index", e))?;
            let crossings = self.ai.manager().update_skater_distance_callbacks(local_index, &managed);
            if !crossings.is_empty() {
                let broadcaster = identity.try_challenge_broadcaster().map_err(|e| error("challenge broadcaster", e))?;
                for crossing in crossings {
                    broadcaster
                        .send_player_radius(crossing.enter, crossing.distance, crossing.player_id)
                        .map_err(|e| error("radius message", e))?;
                }
            }
        }
        self.try_update_audio_events(skaters)
    }

    /// `AIController::GenerateActionGraphIntents` 0x824bd950 inside the Actor's
    /// `GenerateActionGraphIntents`: its Actor calls in order, then its map for the Actor.
    fn try_generate_intents(
        &mut self,
        index: usize,
        world: &mut PhysicalAssemblyWorld,
        skaters: &[SelectedTickSkater<'_>],
    ) -> Result<(), String> {
        let skater = &skaters[index];
        let player = skater.actor.try_ai_player_id().map_err(|e| error("player", e))?;
        if !self.ai.controllers.contains_key(&player) {
            return Ok(());
        }
        let mut usage = Vec::with_capacity(skaters.len());
        let mut obstacles = Vec::with_capacity(skaters.len());
        let mut local_player_position = None;
        let mut local_player_wiping_out = false;
        let mut skater_positions = Vec::with_capacity(skaters.len());
        for other in skaters {
            let id = other.actor.try_ai_player_id().map_err(|e| error("player", e))?;
            let active = self.ai.controllers.get(&id).filter(|controller| controller.path_controller.is_active);
            let path = active.map(|controller| (controller.path_controller.path_id, controller.path_controller.current_node_index));
            usage.push(SkaterPathUsage { player_id: id, path });
            let other_view = other.actor.try_ai_actor_view(self.fade_out_time).map_err(|e| error("obstacle", e))?;
            obstacles.push(ObstacleSkater {
                player_id: id,
                position: other_view.skeleton_root[3],
                velocity: active.map_or(other_view.velocity, |controller| controller.path_controller.velocity()),
                effective_deck_transform: other_view.effective_deck_transform,
                collide_with_world_only: other_view.collide_with_world_only,
                is_wiping_out: other_view.is_wiping_out,
                hips_position: other_view.hips_position,
                hips_velocity: other_view.hips_velocity,
                hips_look: other_view.hips_look,
            });
            if Some(id) == self.ai.local_player && id != player {
                local_player_position = Some(other_view.path_relative_position());
                local_player_wiping_out = other_view.is_wiping_out;
            }
            skater_positions.push((id, other_view.teleport_destination.unwrap_or_else(|| other_view.path_relative_position())));
        }
        let mut view = skater.actor.try_ai_actor_view(self.fade_out_time).map_err(|e| error("view", e))?;
        let (ambient, challenge) = match self.ai.controllers[&player].client_type {
            AiClientType::Ambient => (true, false),
            AiClientType::Challenge => (false, true),
            other => return Err(format!("AI obstacles: AreWeInAnAIChallenge for a {other:?} skater is not ported")),
        };
        let query = ObstacleQuery {
            position: view.skeleton_root[3],
            radius: self.ai.attributes.obstacle_radius,
            ambient,
            challenge,
            small_object_mass_threshold: self.ai.small_object_mass_threshold,
        };
        let entities = {
            let living = world
                .grind_services
                .living_world()
                .lock()
                .map_err(|_| "AI obstacles: Living World lock poisoned".to_owned())?;
            living_world_obstacles::try_gather(&living, &query)?
        };
        let mut intents = ActorGraphIntents::try_new::<std::convert::Infallible>().map_err(|e| error("intents", e))?;
        let cvars = skater.actor.cached_physical_allocation().map_err(|e| error("physical cvars", e))?.owner().inputs().cvars.clone();
        let mut commands = Vec::new();
        {
            let SessionAi {
                paths,
                tricks,
                attributes,
                controllers,
                in_front_end,
                challenge_type,
                respawn_locations,
                respawn_location_set,
                race_estimated_times,
                local_player,
                ..
            } = &mut *self.ai;
            let controller = controllers.get_mut(&player).expect("checked AI controller");
            let profile = controller.profile.clone();
            let paths = paths.lock().map_err(|_| "AI path manager lock poisoned".to_owned())?;
            let mut gather = no_locked_volumes;
            let mut locked = LockedPathTest::new(vmx::xyz(view.path_relative_position()), &mut gather);
            let mut context = PathControllerContext {
                cvars: &cvars,
                paths: &paths,
                tricks,
                attributes,
                profile: &profile,
                abilities: controller.abilities,
                client_type: controller.client_type,
                challenge_round: controller.challenge_round,
                challenge_type: *challenge_type,
                in_front_end: *in_front_end,
                skaters: &usage,
                obstacle_skaters: &obstacles,
                obstacle_entities: &entities,
                local_player_position,
                local_player_wiping_out,
                local_player: *local_player,
                race_estimated_times,
                respawn_locations,
                respawn_location_set: *respawn_location_set,
                skater_positions: &skater_positions,
                locked: &mut locked,
            };
            controller
                .generate_action_graph_intents(&mut context, &mut view, &mut intents, &mut commands)
                .map_err(|e| error("controller", e))?;
        }
        for command in &commands {
            skater
                .actor
                .try_apply_ai_command(command, world, skater.board, skater.skeleton, self.fade_out_time)
                .map_err(|e| error("Actor call", e))?;
        }
        skater.actor.try_publish_ai_intents(intents).map_err(|e| error("intents", e))
    }

    /// `AIController::Update` 0x824c8be0 at the top of the Actor's `Update`.
    fn try_update_controller(&mut self, skater: &SelectedTickSkater<'_>) -> Result<(), String> {
        let player = skater.actor.try_ai_player_id().map_err(|e| error("player", e))?;
        if !self.ai.controllers.contains_key(&player) {
            return Ok(());
        }
        let view = skater.actor.try_ai_actor_view(self.fade_out_time).map_err(|e| error("view", e))?;
        self.ai.controllers.get_mut(&player).expect("checked AI controller").update(&view);
        Ok(())
    }
}

impl<T, C> SelectedSession<T, C> {
    /// The AI skaters' presentation entities, as the later players'.
    pub(crate) fn ai_skater_visuals(&self) -> Result<Vec<crate::SelectedSkaterVisual<'_>>, String> {
        let player = self.player_recipe_id();
        let mut visuals = Vec::with_capacity(self.ai_skaters.len());
        for skater in &self.ai_skaters {
            let runtime = &skater.presentation_runtime;
            let borrowed = runtime.borrowed();
            let Some(render) = runtime.render_opacity() else { continue };
            visuals.push(crate::SelectedSkaterVisual {
                name_tag: None,
                palette: runtime.palette().map_err(|e| error("palette", e))?,
                materials: Arc::clone(runtime.materials().map_err(|e| error("materials", e))?),
                hips_position: (if borrowed { runtime.borrowed_hips_position() } else { runtime.published_hips_position() }).ok_or("AI skater hips unpublished")?,
                world_bounds: runtime.published_bounds().ok_or("AI skater bounds unpublished")?,
                opacity: if borrowed { 1.0 } else { render.opacity },
                ghosted: render.ghosted && !borrowed,
                entity_type: if borrowed { 0x822 } else { 0x122 },
                players_recipe: skater.recipe_load.asset_id() == player,
                lobby_ghost: false,
            });
        }
        Ok(visuals)
    }

    /// The `SkaterManager`'s `cMsgSpawnAi`/`cMsgUnspawnAi` handling after the frame:
    /// `SpawnAi` 0x82600488 builds the skater and attaches its controller (`CreateAiController`
    /// 0x825f40b8); `UnspawnAi` destroys the skater and its controller.
    pub(crate) fn try_dispatch_ai_messages<E: std::fmt::Debug>(&mut self) -> Result<(), String>
    where
        T: FnMut() -> Result<u64, E>,
    {
        let Some(ai) = self.ai.as_mut() else { return Ok(()) };
        let _profile = crate::diagnostics::frame_profile::span(crate::diagnostics::frame_profile::AI_DISPATCH);
        if let Some(player) = self.player.as_ref() {
            let owners = &player.selected.owners;
            let recipes = ai.manager().assets.recipe_ids();
            ai.scenes.update(&recipes, |recipe| {
                let definition = super::party_skaters::marquee_definition(&owners.services, recipe).ok()?;
                let actual = MountedPlayerRecipeLoad::try_actual_marquee_definition(&owners.services, &definition).ok()?;
                let mounts = Arc::clone(&owners.mounts);
                let schema_name = actual.schema_name.clone();
                if std::env::var_os("SKATE2_TRACE_AI").is_some() {
                    eprintln!("ai: scene load {recipe:#018x} {schema_name}");
                }
                let job = std::thread::Builder::new()
                    .name("sk8 AI load".to_owned())
                    .spawn(move || MountedPlayerRecipeLoad::try_prepare_marquee_scene(&mounts, &actual).ok())
                    .ok()?;
                Some(AiSceneLoad { schema_name, scene: AiSceneState::Loading(job) })
            });
            ai.manager().assets.streamed = Some(ai.scenes.streamed());
        }
        let mut commands = std::mem::take(&mut ai.deferred);
        commands.extend(ai.take_pending());
        for command in commands {
            match command {
                AmbientCommand::Spawn { slot, client_type, recipe, transform, profile_name } => {
                    if let Some(ai) = self.ai.as_mut()
                        && ai.scenes.loading(recipe)
                    {
                        ai.deferred.push(AmbientCommand::Spawn { slot, client_type, recipe, transform, profile_name });
                        continue;
                    }
                    if std::env::var_os("SKATE2_TRACE_AI").is_some() {
                        eprintln!("ai: spawn {client_type:?} slot {slot} recipe {recipe:#018x} profile {profile_name} at {:?}", transform[3]);
                    }
                    let player = u32::try_from(slot).map_err(|e| error("spawn slot", e))?;
                    let spawn = std::array::from_fn(|index| transform[index / 4][index % 4]);
                    let prepared = self.ai.as_mut().and_then(|ai| ai.scenes.scene(recipe));
                    if std::env::var_os("SKATE2_TRACE_AI").is_some() {
                        eprintln!("ai: spawn scene {}", prepared.as_ref().map_or("not prepared", |(schema, _)| schema.as_str()));
                    }
                    // OnCommand 0x82600920 destroys the slot's player before SpawnAi: a challenge
                    // that spawns its skater again (a restart) replaces it without an unspawn
                    // (SpawnAISkater 0x824cba18 frees the slot itself).
                    if let Some(at) = self.ai_skaters.iter().position(|skater| skater.player_index() == player) {
                        let skater = self.ai_skaters.remove(at);
                        self.try_destroy_later_skaters(vec![skater])?;
                    }
                    let skater = self.try_build_later_skater(player, PartyCharacter::Marquee(recipe), spawn, prepared)?;
                    skater
                        .actor
                        .try_attach_ai_controller(client_type)
                        .map_err(|e| error("attach", e))?;
                    let vault = Arc::clone(&self.assets.as_ref().ok_or("AI spawn: assets absent")?.vault);
                    let profile = SkaterProfile::try_new(&vault, &profile_name).map_err(|e| error("profile", e))?;
                    let abilities = profile.try_abilities().map_err(|e| error("abilities", e))?;
                    let controller = AiController::new(client_type, &profile_name, recipe, profile, abilities);
                    self.ai
                        .as_mut()
                        .expect("checked session AI")
                        .controllers
                        .insert(i32::try_from(player).map_err(|e| error("spawn slot", e))?, controller);
                    let at = self.ai_skaters.partition_point(|other| other.player_index() < player);
                    self.ai_skaters.insert(at, skater);
                }
                AmbientCommand::Unspawn { slot } => {
                    if std::env::var_os("SKATE2_TRACE_AI").is_some() {
                        eprintln!("ai: unspawn slot {slot}");
                    }
                    if let Some(ai) = self.ai.as_mut() {
                        ai.deferred.retain(|command| !matches!(command, AmbientCommand::Spawn { slot: spawn, .. } if *spawn == slot));
                    }
                    let player = u32::try_from(slot).map_err(|e| error("unspawn slot", e))?;
                    // UnspawnAISkater already removed the backend row when it posted this
                    // message. A replacement Spawn may now own that slot and its name.
                    if let Some(controllers) = self.ai.as_mut().map(|ai| &mut ai.controllers) {
                        controllers.remove(&i32::try_from(player).map_err(|e| error("unspawn slot", e))?);
                    }
                    if let Some(at) = self.ai_skaters.iter().position(|skater| skater.player_index() == player) {
                        let skater = self.ai_skaters.remove(at);
                        self.try_destroy_later_skaters(vec![skater])?;
                    }
                }
                AmbientCommand::SetWander { .. } | AmbientCommand::WipeoutRequest { .. } => {}
            }
        }
        Ok(())
    }

    /// The `SkaterManager`'s skaters (the first player, the party, the AI skaters) as the AI
    /// manager reads them.
    fn try_managed_skaters(&self) -> Result<Vec<ManagedSkater>, String> {
        let actors = self
            .player
            .as_ref()
            .and_then(|player| player.selected.actor.as_ref())
            .into_iter()
            .chain(self.party.iter().map(|skater| &skater.actor))
            .chain(self.ai_skaters.iter().map(|skater| &skater.actor));
        let mut skaters = Vec::new();
        for actor in actors {
            let player = actor.try_ai_player_id().map_err(|e| error("player", e))?;
            let ai = match self.ai.as_ref().and_then(|ai| ai.controllers.get(&player)) {
                Some(controller) => Some(ManagedAi {
                    client_type: controller.client_type,
                    recipe_id: controller.recipe_id,
                    has_behavior: controller.behavior().is_some(),
                    fading_out: controller.path_controller.is_fading_out,
                    opacity: actor.try_ai_opacity().map_err(|e| error("opacity", e))?,
                }),
                None => None,
            };
            skaters.push(ManagedSkater { player_id: player, position: actor.try_ai_position().map_err(|e| error("position", e))?, ai });
        }
        Ok(skaters)
    }

    /// `AISkaterManager::ExitAiBasedChallenge(false)` 0x824cff60: in an AI-based challenge each
    /// challenge skater becomes an ambient one where it can (`ConvertAllChallangeSkatersToAmbient`
    /// 0x824cff08) and the models past the ambient set unload (`UnloadExcessSkaterModels`
    /// 0x824ce6d0); then the manager leaves the challenge.
    pub(crate) fn try_exit_ai_based_challenge(&mut self) -> Result<(), String> {
        let Some(ai) = self.ai.as_ref() else { return Ok(()) };
        if ai.manager().in_an_ai_based_challenge() {
            let settings = Arc::clone(&self.player.as_ref().ok_or("AI: first player absent")?.selected.owners.services.settings);
            for slot in 0..9_u32 {
                if settings.try_player_type(slot).map_err(|e| error("player type", e))? == player_type::AI_CHALLENGE {
                    self.try_convert_to_ambient_skater(slot as usize, &settings)?;
                }
            }
            let skaters = self.try_managed_skaters()?;
            let ai = self.ai.as_ref().expect("checked session AI");
            let mut manager = ai.manager();
            let commands = manager.unload_excess_skater_models(&ai.catalog, ai.camera[3], &skaters).map_err(|e| error("unload models", e))?;
            for command in commands {
                manager.post(command, &settings)?;
            }
        }
        self.ai.as_ref().expect("checked session AI").manager().exit_ai_based_challenge();
        Ok(())
    }

    /// `ConvertToAmbientSkater` 0x824cea88: a challenge skater whose recipe is unlocked, while
    /// the ambient skaters have room, moves to the best ambient path start within reach
    /// (`ConvertSkaterToAmbient` 0x824b9c40); otherwise it unspawns.
    fn try_convert_to_ambient_skater(&mut self, slot: usize, settings: &PlayersBackendSettings) -> Result<(), String> {
        let player = i32::try_from(slot).map_err(|e| error("conversion slot", e))?;
        let skaters = self.try_managed_skaters()?;
        let actor = self.ai_skaters.iter().find(|skater| skater.player_index() as usize == slot).map(|skater| Arc::clone(&skater.actor));
        let ai = self.ai.as_ref().ok_or("AI conversion: session AI absent")?;
        let mut target = None;
        if let (Some(actor), Some(controller)) = (actor.as_ref(), ai.controllers.get(&player))
            && ai.catalog.is_recipe_id_unlocked(controller.recipe_id)
            && ai.manager().has_room_for_ambient_skater()
        {
            let position = actor.try_ai_position().map_err(|e| error("position", e))?;
            let mut others = Vec::new();
            for skater in &skaters {
                let has_row = u32::try_from(skater.player_id)
                    .ok()
                    .and_then(|p| settings.try_player_type(p).ok())
                    .is_some_and(|t| t != player_type::NONE);
                if skater.player_id == player || !has_row {
                    continue;
                }
                let destination = self
                    .player
                    .as_ref()
                    .and_then(|first| first.selected.actor.as_ref())
                    .into_iter()
                    .chain(self.party.iter().map(|skater| &skater.actor))
                    .chain(self.ai_skaters.iter().map(|skater| &skater.actor))
                    .find(|other| other.try_ai_player_id().is_ok_and(|id| id == skater.player_id))
                    .map(|other| other.try_ai_teleport_destination())
                    .transpose()
                    .map_err(|e| error("teleport destination", e))?
                    .flatten();
                others.push(destination.unwrap_or(skater.position));
            }
            let paths = ai.paths.lock().map_err(|_| "AI path manager lock poisoned".to_owned())?;
            let mut gather = no_locked_volumes;
            let mut locked = LockedPathTest::new(vmx::xyz(position), &mut gather);
            let results = paths
                .try_get_paths_in_radius(
                    vmx::xyz(position),
                    conversion::MAX_PATH_DISTANCE,
                    conversion::MAX_PATHS,
                    PathQueryType::Start,
                    Some(paths.ambient_filter()),
                    Some(controller.abilities),
                    None,
                    &mut locked,
                )
                .map_err(|e| error("conversion paths", e))?;
            let scores: Vec<u32> = results
                .iter()
                .map(|result| {
                    let start = vmx::v3(path::node(result.path.path(), 0).position);
                    skater_manager::convert_to_ambient_path_score(start, position, &others)
                })
                .collect();
            target = skater_manager::pick_best_convert_to_ambient_path(&scores).map(|best| {
                let result = &results[best];
                (path::node_matrix(path::node(result.path.path(), result.node_index)), controller.recipe_id)
            });
        }
        let Some(((matrix, recipe), actor)) = target.zip(actor) else {
            let ai = self.ai.as_ref().expect("checked session AI");
            let mut manager = ai.manager();
            for command in manager.request_unspawn(slot) {
                manager.post(command, settings)?;
            }
            return Ok(());
        };
        let vault = Arc::clone(&self.assets.as_ref().ok_or("AI conversion: assets absent")?.vault);
        let ai = self.ai.as_mut().expect("checked session AI");
        let profile_name = ai.catalog.profile_name(recipe).to_owned();
        settings.try_convert_ai_player_type(slot as u32, player_type::AI_AMBIENT).map_err(|e| error("player type", e))?;
        ai.manager().convert_to_ambient_player(slot);
        actor.try_attach_ai_controller(AiClientType::Ambient).map_err(|e| error("attach", e))?;
        let profile = SkaterProfile::try_new(&vault, &profile_name).map_err(|e| error("profile", e))?;
        let abilities = profile.try_abilities().map_err(|e| error("abilities", e))?;
        ai.controllers.insert(player, AiController::new(AiClientType::Ambient, &profile_name, recipe, profile, abilities));
        let teleport = crate::skeleton_root_transforms::SkeletonRootMatrix::known(std::array::from_fn(|index| matrix[index / 4][index % 4]));
        actor.try_publish_teleport(&teleport, true).map_err(|e| error("teleport", e))?;
        actor.try_set_pinned(false).map_err(|e| error("unpin", e))
    }

    /// `cTeleportManager::TeleportPlayer(player, transform)` 0x828a9888 of a skater with an
    /// `AIController`: `Actor::SetTransform`. False when the player is not one.
    pub(crate) fn try_teleport_ai_player(&mut self, player: i32, transform: [f32; 16]) -> Result<bool, String> {
        if !self.ai.as_ref().is_some_and(|ai| ai.controllers.contains_key(&player)) {
            return Ok(false);
        }
        let Some(actor) = self
            .ai_skaters
            .iter()
            .find(|skater| u32::try_from(player).is_ok_and(|player| skater.player_index() == player))
            .map(|skater| Arc::clone(&skater.actor))
        else {
            return Ok(false);
        };
        let transform = crate::skeleton_root_transforms::SkeletonRootMatrix::known(transform);
        actor.try_set_transform(&transform).map_err(|e| error("set transform", e))?;
        Ok(true)
    }

    /// The Challenge queue's messages since the last call (`EventSys` queue 7, which the
    /// challenge system's `cLuaPostOffice` routes).
    pub(crate) fn try_take_challenge_messages(&self) -> Result<Vec<crate::hermes_challenge_queue::ChallengeMessage>, String> {
        let Some(owner) = self.world.as_ref() else { return Ok(Vec::new()) };
        let local = owner.world.lock().map_err(|_| "challenge queue: world lock poisoned".to_owned())?.local_skater_identity.clone();
        let Some(queue) = local.map(|local| local.try_challenge_queue()).transpose().map_err(|e| error("challenge queue", e))?.flatten() else {
            return Ok(Vec::new());
        };
        queue.try_take_all().map_err(|e| error("challenge queue", e))
    }

    /// Every AI skater goes with the game (`SkaterManager::Reset` 0x82600410 destroys them);
    /// the manager's ambient players leave the backend first.
    pub(crate) fn try_destroy_ai_skaters(&mut self) -> Result<(), String> {
        if let Some(ai) = self.ai.as_mut() {
            let settings = &self.player.as_ref().ok_or("AI: first player absent")?.selected.owners.services.settings;
            ai.try_unspawn_all_ambient(settings)?;
            for command in ai.take_pending() {
                if let AmbientCommand::Unspawn { slot } = command {
                    release_player(settings, slot)?;
                }
            }
            for command in std::mem::take(&mut ai.deferred) {
                if let AmbientCommand::Spawn { slot, .. } = command {
                    release_player(settings, slot)?;
                }
            }
            ai.controllers.clear();
        }
        let skaters = std::mem::take(&mut self.ai_skaters);
        self.try_destroy_later_skaters(skaters)
    }

    /// `AISkaterManager::UnregisterAllSkaters` 0x824cb868 on the session's players.
    pub(crate) fn try_unregister_all_ai_players(&mut self) -> Result<(), String> {
        let settings = &self.player.as_ref().ok_or("AI: first player absent")?.selected.owners.services.settings;
        match self.ai.as_ref() {
            Some(ai) => ai.manager().unregister_all_skaters(settings),
            None => Ok(()),
        }
    }
}
