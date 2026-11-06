//! Live-player owner for the explicitly requested PC simulation mod.
//!
//! The actual Skate 2 physical assembly owns this selected solver and controller.
//! The original solver slots stay dormant while completed, read-only views
//! serve the character, world queries and replay. No second game runtime runs.
use skate3_core::{input::xbox::XboxState, physics::{board::BodyId, skeleton_animation_record::{AnimationPartTransform, compose_affine}}};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::PhysicalAssemblyWorld,
    simulation_rigid_body_pool::SimulationRigidBodyObservation,
    static_ground_pool::{ResidentGroundGeometry, StaticGroundPool},
};
use crate::skate3_native::{
    BodyOutput, BodyReaction, SkeletonPartOutput, FrameOutput, RawController, Scene, SceneTriangle, SceneTriangleFeatures,
};

#[path = "skate3_terrain.rs"]
pub(crate) mod terrain;
#[path = "skate3_grinds.rs"]
mod grinds;
#[path = "skate3_grabs.rs"]
mod grabs;
#[path = "skate3_pushing.rs"]
mod pushing;
#[path = "player/skate3_exit.rs"]
pub(crate) mod exit;
#[path = "player/skate3_actor_input.rs"]
pub(crate) mod actor_input;
#[path = "player/skate3_camera_rate.rs"]
mod camera_rate;
#[path = "player/skate3_spawn.rs"]
mod spawn;

/// The linked Skate 3 data as the source engine's asset root (`skate_data::asset_io::ROOT`): its
/// archives and loose files, linked again when the player links other data.
pub(crate) fn assets_root() -> Result<&'static std::path::Path, String> {
    static LINKED: std::sync::Mutex<usize> = std::sync::Mutex::new(0);
    let data = crate::skate3::linked().ok_or("Skate 3 data is not linked")?;
    let identity = std::sync::Arc::as_ptr(&data) as usize;
    let mut linked = LINKED.lock().map_err(|_| "Skate 3 asset link lock")?;
    if *linked != identity {
        let mounts = std::sync::Arc::clone(&data.mounts);
        skate_data::asset_io::link(std::sync::Arc::new(move |path: &str| {
            mounts.read_path(path).map_err(|error| std::io::Error::new(std::io::ErrorKind::NotFound, format!("{path}: {error}")))
        }));
        *linked = identity;
    }
    Ok(std::path::Path::new(skate_data::asset_io::ROOT))
}

/// A value read once from the linked Skate 3 data. A failure (no data linked yet, for example) is
/// returned and not kept, so the first call after the data is linked reads it.
pub(crate) fn linked_once<T>(
    cell: &'static std::sync::OnceLock<T>,
    load: impl FnOnce() -> Result<T, String>,
) -> Result<&'static T, String> {
    if let Some(value) = cell.get() {
        return Ok(value);
    }
    let _ = cell.set(load()?);
    Ok(cell.get().expect("linked Skate 3 value set"))
}

/// The skater database at `root`: the shared linked copy, or a parse of a test asset root.
pub(crate) fn collections_at(root:&std::path::Path)->Result<std::sync::Arc<skate_data::collections::Collections>,String> {
    if skate_data::asset_io::is_linked_root(root) {
        stock_collections()
    } else {
        Ok(std::sync::Arc::new(skate_data::collections::Collections::load(root)?))
    }
}

/// The linked game's skater database. Runtime profile overrides clone it; no mutable tuning or
/// query state is shared.
pub(crate) fn stock_collections()->Result<std::sync::Arc<skate_data::collections::Collections>,String> {
    static DATA:std::sync::OnceLock<std::sync::Arc<skate_data::collections::Collections>>=std::sync::OnceLock::new();
    linked_once(&DATA, || {
        let started=std::time::Instant::now();
        let data=std::sync::Arc::new(skate_data::collections::Collections::load(assets_root()?)?);
        crate::diagnostics::test_log::event("skate3_stock_collections",format_args!(
            "collections_ms={:.3}",started.elapsed().as_secs_f64()*1000.));
        Ok(data)
    }).map(std::sync::Arc::clone)
}

pub(crate) fn stock_animation_source()->Result<std::sync::Arc<crate::skate3_native::skater_animation::AnimationSource>,String> {
    Ok(std::sync::Arc::clone(&player_assets(assets_root()?)?.animation))
}

struct PreparedPlayerTick {
    actions:skate3_core::input::gameplay_map::GameplayActions,
    input_available:bool,
    frame:Option<super::frame::PreparedFrame>,
}

struct PlayerAssets {
    graphs: std::sync::Arc<crate::skate3_native::graph_runtime::StockGraphs>,
    animation: std::sync::Arc<crate::skate3_native::skater_animation::AnimationSource>,
}
fn player_assets(root: &std::path::Path) -> Result<&'static PlayerAssets, String> {
    // Only immutable graph/bank/pose data is shared. Every player still owns
    // its own graph state, physics, camera, input history and scoring runtime.
    static ASSETS: std::sync::OnceLock<PlayerAssets> = std::sync::OnceLock::new();
    linked_once(&ASSETS, || {
        let manifest = skate_data::GameAssets::load_physics(root).map_err(|error|error.to_string())?;
        Ok(PlayerAssets {
            graphs: std::sync::Arc::new(crate::skate3_native::graph_runtime::StockGraphs::load(root, &manifest)?),
            animation: crate::skate3_native::skater_animation::AnimationSource::load(root)?,
        })
    })
}

/// The two engines' authored mass frames can place their mirror COM at
/// different offsets. Transport the solved rigid velocity to that actual point.
fn velocity_at(source: &crate::skate3_native::BodyOutput, target: &[f32; 16]) -> [f32; 3] {
    let r: [f32; 3] = std::array::from_fn(|i| target[12 + i] - source.centre_of_mass[i]);
    let w = source.omega;
    [
        source.velocity[0] + w[1] * r[2] - w[2] * r[1],
        source.velocity[1] + w[2] * r[0] - w[0] * r[2],
        source.velocity[2] + w[0] * r[1] - w[1] * r[0],
    ]
}

/// A background terrain build's result: the decoded sources, the terrain cache and the new world.
type TerrainBuild = (Vec<ResidentGroundGeometry>, terrain::PreparedTerrain, Result<skate3_core::physics::board_world::BoardWorld, String>);

#[derive(Clone, Copy)]
struct HostTeleport {
    transform: [f32; 16],
    on_board: bool,
    generation: Option<u32>,
    foot_forward: Option<u32>,
}

/// One-time transfer at solver selection, using the current actor's published
/// physical root and velocity. Each subsequent tick has exactly one solver.
pub(crate) struct PlayerEntry {
    pub transform: [f32; 16],
    pub velocity: [f32; 3],
    pub on_board: bool,
    pub state: u32,
    pub holding_board: bool,
    pub filtered: Option<skate3_core::physics::filtered_state::FilteredStateOutput>,
    pub camera_basis: Option<[[f32; 3]; 3]>,
}


pub(crate) struct Skate3Player {
    root: std::path::PathBuf,
    graphs: std::sync::Arc<crate::skate3_native::graph_runtime::StockGraphs>,
    physics: super::Skate3PhysicsState,
    skater: super::Skate3ControllerState,
    controls: super::PlayerControls,
    input: crate::skate3_native::input::ControllerInput,
    entry_camera_basis: Option<[[f32; 3]; 3]>,
    camera: crate::skate3_native::camera::CameraRuntime,
    airtime_slow_motion: bool,
    hips_body: usize,
    grinds: grinds::HostGrinds,
    grabs: grabs::HostGrabs,
    /// TU3 PhysState_OffBoard_Pushing's object movement, run over Skate 2's.
    pushing: pushing::Pushing,
    host_cvars: Option<crate::input_cvar_runtime::InputCvarReferences>,
    /// The Skate 2 player's attribute vault (the pushing state's tuning for the world's props).
    host_vault: Option<std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>>,
    host_bodies: crate::simulation_assembly_backend::constraint_session::skate3_world::HostBodies,
    sources: Vec<ResidentGroundGeometry>,
    terrain: terrain::PreparedTerrain,
    /// Streamed ground decoded and built on another thread while the current world stays in use;
    /// it holds the sources and terrain cache until it returns them with the new world.
    terrain_job: Option<std::thread::JoinHandle<TerrainBuild>>,
    board: Vec<SimulationRigidBodyObservation>,
    board_frames: Vec<[f32; 16]>,
    skeleton_frames: Vec<[f32; 16]>,
    skeleton: Vec<SimulationRigidBodyObservation>,
    skeleton_bones: Vec<(String, [f32; 16])>,
    output: std::sync::Arc<FrameOutput>,
    queued_teleport: Option<HostTeleport>,
    awaiting_teleport: Option<HostTeleport>,
    completed_teleport: Option<HostTeleport>,
    advanced_this_tick: bool,
    prepared_tick: Option<PreparedPlayerTick>,
    waiting_for_terrain: bool,
    teleport_generation: Option<u32>,
    world_body_count: usize,
    actor_input: actor_input::HostActorInput,
    input_focused: bool,
    /// PC: bridge single-tick misses while holding the prop menu open.
    interactable_hold: Option<(u32, u32)>,
    /// The host Actor manager's spawn generation the breadcrumb history last restarted at.
    breadcrumb_generation: Option<u64>,
    /// The Skate 3 skater's truck tightness and wheel hardness, which ride in place of the Skate 2
    /// Actor's; `None` keeps the Skate 2 profile's.
    equipment: Option<[f32; 2]>,
}

pub(crate) use super::respawn::{LivingWorldRadii, RespawnHost};
use crate::actor_construction::breadcrumbs::{ActorBreadCrumbManager, SkaterBreadCrumb};
use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};

fn host_matrix(matrix: &SkeletonRootMatrix) -> AnimationPartTransform {
    let columns = matrix.columns();
    std::array::from_fn(|i| {
        let [x, y, z] = columns[i].xyz;
        [x, y, z, columns[i].w.unwrap_or(0.)]
    })
}
/// The Actor enum stores RIGHT as zero, as the checkpoint stance does.
fn host_crumb(candidate: skate3_core::player::respawn::Candidate) -> SkaterBreadCrumb {
    SkaterBreadCrumb {
        transform: SkeletonRootMatrix::known(flatten(candidate.transform)),
        foot_forward_right: candidate.stance == 0,
        offboard: candidate.offboard,
    }
}

const INTERACTABLE_HOLD_TICKS: u32 = 30;
fn hold_interactable(hold: &mut Option<(u32, u32)>, published: &mut (u32, bool), grab_world: bool) {
    if !grab_world {
        *hold = None;
    } else if published.1 {
        *hold = Some((published.0, 0));
    } else if let Some((object, missed)) = hold {
        *missed += 1;
        if *missed > INTERACTABLE_HOLD_TICKS { *hold = None; }
        else { *published = (*object, true); }
    }
}

// Diagnostic-only capture of the already decoded static collision triangles.
fn diagnostic_ground_export(sources: &[ResidentGroundGeometry]) {
    use std::io::Write;
    let Some(path)=std::env::var_os("SKATE2_DIAGNOSTIC_GROUND_EXPORT") else {return;};
    let result=(||->std::io::Result<()> {
        let path=std::path::PathBuf::from(path);
        if let Some(parent)=path.parent(){std::fs::create_dir_all(parent)?;}
        let mut out=std::io::BufWriter::new(std::fs::File::create(path)?);
        for source in sources {
            for triangle in &source.triangles {
                for point in triangle.vertices {for coordinate in point {out.write_all(&coordinate.to_le_bytes())?;}}
            }
        }
        out.flush()
    })();
    if let Err(error)=result {eprintln!("ground diagnostic export failed: {error}");}
}

fn scene(sources: &[ResidentGroundGeometry], spawn: [f32; 3]) -> Result<Scene, String> {
    let mut triangles = Vec::new();
    for source in sources {
        if source.triangles.len() != source.features.len() {
            return Err("Skate 2 resident ground has incomplete native features".into());
        }
        for (triangle, feature) in source.triangles.iter().zip(&source.features) {
            let surface = triangle.surface_id.unwrap_or(0);
            triangles.push(SceneTriangle {
                points: triangle.vertices,
                // S3 WorldLineHit.tag is the packed RW surface itself. S2's
                // GP user tag (surface << 16 | group) has a different contract.
                surface: u32::from(surface),
                packed_surface: surface,
                native_features: Some(SceneTriangleFeatures {
                    flags: feature.flags,
                    edge_cosines: feature.edge_cosines,
                }),
                native_edges: None,
            });
        }
    }
    if triangles.is_empty() {
        return Err("Skate 3 player requires actual Skate 2 terrain".into());
    }
    Ok(Scene {
        spawn,
        triangles,
        ..Default::default()
    })
}

impl Skate3Player {
    pub(crate) fn load(
        world: &mut PhysicalAssemblyWorld,
        ground: &StaticGroundPool,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        profile: &str,
        entry: Option<PlayerEntry>,
    ) -> Result<Self, String> {
        let board_bodies = world
            .board_publication(board)
            .map_err(|e| e.to_string())?
            .rigid_bodies()
            .to_vec();
        let skeleton_bodies = world
            .skeleton_publication(skeleton)
            .map_err(|e| e.to_string())?
            .rigid_bodies()
            .to_vec();
        if board_bodies.len() != 7 || skeleton_bodies.len() != 24 {
            return Err(
                "Skate 3 player requires the retained seven-part board and 24-part skeleton".into(),
            );
        }
        let deck = world
            .board(board)
            .map_err(|e| e.to_string())?
            .try_live_part_transform_host(world.backend(), 6)
            .map_err(|e| e.to_string())?;
        let identity = crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;
        let board_frames = (0..7)
            .map(|i| {
                world
                    .board(board)
                    .map_err(|e| e.to_string())?
                    .try_skate3_mirror_body_transform(world.backend(), i, identity)
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, String>>()?;
        let skeleton_frames = (0..24)
            .map(|i| {
                world
                    .skeleton(skeleton)
                    .map_err(|e| e.to_string())?
                    .session()
                    .try_skate3_mirror_body_transform(world.backend(), i, identity)
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, String>>()?;
        let velocity = world
            .backend()
            .try_external_body_snapshot(board_bodies[6])?
            .velocity;
        let sources = ground.resident_geometry().map_err(|e| e.to_string())?;
        diagnostic_ground_export(&sources);
        let root = assets_root()?;
        let scene = scene(&sources, [deck[12], deck[13], deck[14]])?;
        let _asset_load = crate::diagnostics::test_log::span("skate3_asset_graph_load");
        let difficulty = crate::skate3_native::difficulty::Difficulty::parse(profile)?;
        let assets = player_assets(root)?;
        let graphs = std::sync::Arc::clone(&assets.graphs);
        let mut physics = {
            let _stage = crate::diagnostics::test_log::timed_span("skate3_load_physics");
            super::Skate3PhysicsState::load_with_host_scene(root, &scene, difficulty)?
        };
        let terrain={
            let _stage = crate::diagnostics::test_log::timed_span("skate3_load_terrain_seed");
            terrain::PreparedTerrain::seed(&sources,&physics.world,physics.settings.floor_material)?
        };
        let mut skater = {
            let _stage = crate::diagnostics::test_log::timed_span("skate3_load_skater");
            super::Skate3ControllerState::load_for_world(root, &graphs, &physics, difficulty.profile_key(),
                Some(std::sync::Arc::clone(&assets.animation)))?
        };
        let controls = {
            let _stage = crate::diagnostics::test_log::timed_span("skate3_load_controls");
            super::PlayerControls::load(root)?
        };
        let camera = {
            let _stage = crate::diagnostics::test_log::timed_span("skate3_load_camera");
            crate::skate3_native::camera::CameraRuntime::load(root)?
        };
        let hips_bone = skater.animation.evaluator.frames.bone_names.iter()
            .position(|name| name.eq_ignore_ascii_case("Hips")).ok_or("Native movement rig lacks Hips")?;
        let hips_body = skater.animated_skeleton.bone_indices.iter().position(|&bone| bone == hips_bone)
            .ok_or("Native movement physical rig lacks Hips body")?;
        let entry = entry.unwrap_or(PlayerEntry { transform: deck, velocity, on_board: true, state: 100, holding_board: false, filtered: None, camera_basis: None });
        if !entry.on_board {
            // The first graph tick must see the outgoing solver's completed
            // offboard observations. Zero/ground constructor observations would
            // select an onboard graph and immediately undo the manual reset.
            // These facts seed graph input only; the source's ordinary reset,
            // state selection and solver still own the first physical tick.
            skater.player_input.physical.state.category_12 = 500;
            skater.player_input.physical.state.state_16 = entry.state;
            skater.player_input.physical.off_board.flag_311 = u8::from(entry.holding_board);
            let filtered = entry.filtered.ok_or("Offboard entry requires the outgoing filtered publication")?;
            skater.player_input.physical.filtered_state_0 = filtered.category as u32;
            skater.player_state.filtered_output = Some(filtered);
            skater.player_state.state_flags[75 - 52] = true;
        }
        let entry_target = spawn::target(&physics.world, unflatten(entry.transform), entry.on_board)?;
        skater.player_input.request_teleport_ex(entry_target, Some(entry.velocity))?;
        skater.teleport_state.request_manual(entry_target, entry.on_board);
        let mut grinds = grinds::HostGrinds::default();
        grinds.sync(world, &mut physics, &mut skater)?;
        let data = &world
            .skeleton(skeleton)
            .map_err(|e| e.to_string())?
            .state()
            .binding
            .body()
            .data;
        let skeleton_bones = data
            .bone_ids()
            .iter()
            .enumerate()
            .map(|(part, &bone)| {
                (
                    data.hierarchy().bone_names[bone].clone(),
                    data.bone_to_volume()[part],
                )
            })
            .collect();
        let bodies: Vec<_> = board_bodies
            .iter()
            .chain(&skeleton_bodies)
            .copied()
            .collect();
        let output = std::sync::Arc::new(export_frame(&physics, &skater, &camera, hips_body));
        // Validate names before taking solver ownership. Positional guesses
        // between the two rigs are deliberately not admitted.
        let result = Self {
            root: root.to_owned(), graphs, physics, skater, controls,
            input: Default::default(), entry_camera_basis: entry.camera_basis, camera, airtime_slow_motion: false, hips_body,
            grinds,
            grabs: Default::default(), pushing: Default::default(), host_cvars: None, host_vault: None,
            host_bodies: Default::default(),
            sources, terrain, terrain_job: None,
            board: board_bodies,
            board_frames,
            skeleton_frames,
            skeleton: skeleton_bodies,
            skeleton_bones,
            output,
            queued_teleport: None,
            awaiting_teleport: Some(HostTeleport {
                transform: entry.transform,
                on_board: entry.on_board,
                generation: None,
                foot_forward: None,
            }),
            completed_teleport: None,
            advanced_this_tick: false,
            prepared_tick: None,
            waiting_for_terrain: false,
            teleport_generation: None,
            world_body_count: 0,
            actor_input: Default::default(),
            interactable_hold: None, input_focused: true,
            breadcrumb_generation: None,
            equipment: None,
        };
        result.character_physical_pose()?;
        world.backend_mut().try_select_external_player(&bodies)?;
        Ok(result)
    }

    /// Moving bodies answered by their authored mesh, and grind primitives in total.
    #[cfg(test)]
    pub(crate) fn test_prop_world_counts(&self) -> (usize, usize) {
        (self.physics.world_proxies.meshes.len(), self.physics.grind_world.primitives().len())
    }

    /// Every Skate 3 grind primitive's start and end, in provider order.
    #[cfg(test)]
    pub(crate) fn test_grind_segments(&self) -> Vec<([f32; 4], [f32; 4])> {
        self.physics.grind_world.primitives().iter().map(|p| (p.start, p.end)).collect()
    }

    /// Host runs this after its world solve, before source-player stepping.
    pub(crate) fn sync_world_bodies(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        collide_other_skaters: bool,
    ) -> Result<(), String> {
        world.try_refresh_skate3_sources()?;
        let bodies = world.try_skate3_bodies(
            &mut self.host_bodies, collide_other_skaters, self.physics.settings.floor_material)?;
        self.world_body_count = bodies.solids.len();
        self.set_moving_mesh_bodies(bodies.solids, bodies.meshes)
    }



    pub(crate) fn prepare_animation_tick(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        ground: &StaticGroundPool,
        dt: f32,
        raw: Option<crate::input_pad::XboxPadSample>,
    ) -> Result<bool, String> {
        if self.prepared_tick.is_some(){return Err("Source animation prepared twice in one host tick".into());}
        if !dt.is_finite() || dt <= 0. {
            return Err("Invalid host simulation interval".into());
        }
        let _test_stage = crate::diagnostics::test_log::span("skate3_player_animation_prepare");
        let online = world.sim_rate.lock().map_err(|_| "simulation rate lock")?.online_game();
        self.physics.set_online(online);
        self.physics.turn_bail_limit = match world.physical_world_services() {
            Some(services) => services.game_info().try_turn_bail_limit().map_err(|e| format!("{e:?}"))?,
            None => None,
        }
        .or_else(diagnostic_turn_bail_limit);
        self.completed_teleport = None;
        self.advanced_this_tick = false;
        //A retained player can outlive the streamed collision during travel.
        //Do not decode/replace it with an empty world or consume a queued
        //marker request before the destination has resident collision.
        if ground.published_volume_count() == 0 {
            if !self.waiting_for_terrain {
                crate::diagnostics::test_log::event("skate3_scene_wait",format_args!(
                    "tick={} reason=no_resident_terrain phase=player_step queued_generation={:?} root={:?}",
                    self.output.tick,self.queued_teleport.and_then(|target|target.generation),self.output.root_transform[3]));
            }
            self.waiting_for_terrain = true;
            return Ok(false);
        }
        if self.waiting_for_terrain {
            crate::diagnostics::test_log::event("skate3_scene_resume",format_args!(
                "tick={} phase=player_step volumes={}",self.output.tick,ground.published_volume_count()));
            self.waiting_for_terrain = false;
        }
        // Dynamic props (Object Dropper and city DMOs) are moving bodies whose
        // authored meshes answer ground queries each tick (sync_world_bodies);
        // only the streamed static ground is rebuilt here.
        // PC port: streamed ground decodes and builds on another thread, so a refresh (tens of
        // milliseconds) never stalls a tick; the finished world replaces the old one at the start
        // of a tick. A travel, or a world without ground, still builds here at once.
        if self.terrain_job.as_ref().is_some_and(std::thread::JoinHandle::is_finished) {
            let job = self.terrain_job.take().expect("finished terrain job");
            let (sources, terrain, world) = job.join().map_err(|_| "Skate 3 terrain build panicked".to_owned())?;
            self.terrain = terrain;
            let world = world?;
            self.replace_collision_world(world)?;
            crate::diagnostics::test_log::event("skate3_terrain_refresh", format_args!(
                "tick={} sources={} triangles={} background=true", self.output.tick, sources.len(),
                sources.iter().map(|source|source.triangles.len()).sum::<usize>()));
            self.sources = sources;
        }
        let inline = self.queued_teleport.is_some() || self.awaiting_teleport.is_some() || self.sources.is_empty();
        if self.terrain_job.is_none() && !inline && !ground.matches_resident_geometry(&self.sources) {
            let jobs = ground.resident_geometry_jobs();
            let previous = std::mem::take(&mut self.sources);
            let mut terrain = std::mem::take(&mut self.terrain);
            let material = self.physics.settings.floor_material;
            self.terrain_job = Some(std::thread::Builder::new().name("skate 3 terrain".into()).spawn(move || {
                let built = crate::static_ground_pool::decode_resident_geometry(&jobs, previous)
                    .map_err(|e| e.to_string())
                    .and_then(|sources| terrain.world(&sources, material).map(|world| (sources, world)));
                match built {
                    Ok((sources, world)) => (sources, terrain, Ok(world)),
                    Err(error) => (Vec::new(), terrain, Err(error)),
                }
            }).map_err(|e| format!("Skate 3 terrain thread: {e}"))?);
        }
        if self.terrain_job.is_none() && !ground.matches_resident_geometry(&self.sources) {
            let _stage = crate::diagnostics::test_log::timed_span("skate3_terrain_refresh");
            let sources = {
                let _stage = crate::diagnostics::test_log::timed_span("skate3_terrain_decode");
                ground.resident_geometry_reusing(std::mem::take(&mut self.sources)).map_err(|e| e.to_string())?
            };
            let prepared_world={
                let _stage=crate::diagnostics::test_log::timed_span("skate3_terrain_world_build");
                self.terrain.world(&sources,self.physics.settings.floor_material)?
            };
            self.replace_collision_world(prepared_world)?;
            crate::diagnostics::test_log::event("skate3_terrain_refresh", format_args!(
                "tick={} sources={} triangles={}", self.output.tick, sources.len(),
                sources.iter().map(|source|source.triangles.len()).sum::<usize>()));
            diagnostic_ground_export(&sources);
            self.sources = sources;
        }
        {
            let _stage = crate::diagnostics::test_log::timed_span("skate3_grind_refresh");
            self.grinds.sync(world, &mut self.physics, &mut self.skater)?;
        }
        {
            let _stage = crate::diagnostics::test_log::timed_span("skate3_grab_refresh");
            self.physics.offboard_grab_scene=self.grabs.sync(world,&self.physics.world,self.host_cvars.as_ref())?;
        }
        let raw = raw.map(|pad| RawController {
            buttons: pad.buttons,
            triggers: [pad.left_trigger, pad.right_trigger],
            left: [pad.left_x, pad.left_y],
            right: [pad.right_x, pad.right_y],
        });
        self.collect_controller(raw);
        // The actual game's simulation timer owns this physical player, just
        // as it owns the other players, cars, score and career state. A second
        // accumulator here could repeat/skip publications or burst after a
        // slow-motion rate change. Each admitted world tick advances it once.
            // A newer explicit Actor reset supersedes an old manual board-family
            // continuation once its input matrix has been consumed. Waiting for
            // that continuation can deadlock online travel after an offboard reset.
            if self.skater.player_input.pending_teleport().is_none()
                // An onboard constructor/reset reaches Ground in two ticks.
                // Let its existing Teleport leaf exit before replacing it.
                // Offboard continuations remain supersedable (pinned online
                // actors can prevent that dismount from ever completing).
                && self.skater.teleport_state.manual_on_board() != Some(true)
                && let Some(teleport) = self.queued_teleport.take()
            {
                // Restore each native Actor reset's requested foot in the same
                // graph dispatch that consumes its transform and board state.
                if let Some(foot) = teleport.foot_forward {
                    self.skater.animation.restore_foot_forward(foot != 0);
                    // restore_foot_forward previews the native reset pose with
                    // a copied request; the graph still consumes its original.
                    // This also covers a new target during an active Teleport.
                }
                crate::diagnostics::test_log::event("skate3_spawn_apply", format_args!(
                    "tick={} generation={:?} foot={:?} natural={} on_board={} transform={:?}",
                    self.physics.ticks,teleport.generation,teleport.foot_forward,
                    self.skater.animation.natural_stance(),teleport.on_board,teleport.transform));
                self.host_teleport(unflatten(teleport.transform), teleport.on_board)?;
                self.awaiting_teleport = Some(teleport);
            }
        let (actions,input_available)=self.sample_solver_input()?;
        let frame=super::frame::prepare(&mut self.physics,&mut self.skater,&mut self.controls,&self.graphs,&mut self.camera)?;
        self.prepared_tick=Some(PreparedPlayerTick {actions,input_available,frame});
        Ok(true)
    }

    pub(crate) fn advance(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        ground: &StaticGroundPool,
        dt: f32,
        raw: Option<crate::input_pad::XboxPadSample>,
    ) -> Result<(), String> {
        let _test_stage=crate::diagnostics::test_log::span("skate3_player_solve");
        if self.prepared_tick.is_none() && !self.prepare_animation_tick(world,ground,dt,raw)? {return Ok(());}
        {
            let PreparedPlayerTick {mut actions,input_available,frame}=self.prepared_tick.take().ok_or("Missing prepared source tick")?;
            if let Some(prepared)=frame {
                let _stage=crate::diagnostics::test_log::timed_span("skate3_source_tick");
                super::frame::finish(&mut self.physics,&mut self.skater,&mut self.controls,&mut actions,
                    input_available,&mut self.camera,prepared)?;
            }
            let mut output=export_frame(&self.physics,&self.skater,&self.camera,self.hips_body);
            hold_interactable(&mut self.interactable_hold, &mut output.gameplay.interactable_object,
                self.skater.player_input.processed.flags_2476 & (1 << 22) != 0);
            self.output=std::sync::Arc::new(output);
            send_skitch_messages(world, &mut self.skater.skitch_messages)?;
            if let (Some(cvars), Some(vault)) = (self.host_cvars.as_ref(), self.host_vault.as_ref()) {
                let processed = &self.skater.player_input.processed;
                let record = processed.grab_records_1888_2176[0];
                // GetAnimPartTransformWorldRaw: the animation pose (not the physical one) in world
                // space, for the left hand, right hand and hips (parts 3, 7 and 23).
                let names = &self.skater.animation.evaluator.frames.bone_names;
                let part = |name: &str| names.iter().position(|bone| bone.eq_ignore_ascii_case(name))
                    .and_then(|bone| self.skater.animation.packet.hierarchy.get(bone))
                    .map_or([0.0; 4], |local| compose_affine(&self.skater.animated_skeleton.roots.animation_to_world, local)[3]);
                let root = self.output.effective_root_transform();
                let extra = &self.skater.animation_input.extra;
                let inputs = pushing::Inputs {
                    in_state: self.skater.player_state.current() == skate3_core::player::state::PhysicalStateId::OffBoardPushing,
                    world_grab: processed.flags_2476 & (1 << 22) != 0,
                    record: (record[49] != 0).then_some((record[47], record[48])),
                    parts: [part("LEFTHAND"), part("RIGHTHAND"), part("HIPS")],
                    effective_x: root[0],
                    controls: crate::physical_player_assembly::remaining_states::MoveControls {
                        z: extra.object_move_z,
                        x: extra.object_move_x,
                        rotation: extra.object_move_rotation,
                    },
                    ground_y: root[3][1],
                    effective: root,
                    state_timer: processed.state_timer_2664,
                    height_above_ground: self.skater.offboard_feet.scalars_288_to_296[2],
                    // Skeleton+0x3E00: Enter seeds the follow with it, and UpdateOffBoardPushing
                    // clamps the follow near it.
                    centre_of_mass: self.skater.animated_skeleton.board_frames.com_frame[3],
                    effective_animation: processed.effective_anim_transform_192.map(|row| row.map(f32::from_bits)),
                    pose_error: processed.collision_pose_error_736.map(f32::from_bits),
                    flags_2484: processed.flags_2484,
                    candidates: (self.skater.offboard_grab.flags_12836 & 0x40 != 0)
                        .then(|| self.skater.offboard_grab.validated().to_vec()),
                };
                let shoulders = [part("LEFTARM"), part("RIGHTARM")];
                self.pushing.tick_or_disable(world, vault, cvars, &inputs);
                self.skater.animation.motion.moving_objects.physical = self.pushing.output();
                // SKATE2_DIAGNOSTIC_PUSHING_FOLLOW=0 / _PUSHING_HAND_IK=0: either part of the
                // pushing state's skeleton output off, to isolate it.
                let enabled = |name: &str| std::env::var(name).map_or(true, |value| value != "0");
                self.skater.pushing_follow = self.pushing.follow().filter(|_| enabled("SKATE2_DIAGNOSTIC_PUSHING_FOLLOW"));
                // Skeleton::SetLeft/RightHandIKTargetPosition (0x82bd9728/0x82bd97d0): each hand's
                // target within kDistComToBoard (0.65) of its shoulder, with the pushing blend.
                if let Some(hands) = self.pushing.hand_targets().filter(|_| enabled("SKATE2_DIAGNOSTIC_PUSHING_HAND_IK")) {
                    let ik = &mut self.skater.foot_ik.state;
                    for (limb, target, shoulder) in [(2, hands.left, shoulders[0]), (3, hands.right, shoulders[1])] {
                        ik.external_targets[limb].world_position = pushing::clamp_within(target, shoulder, 0.65);
                        ik.limbs[limb].external_target_set = true;
                        ik.limbs[limb].target_blend = hands.blend;
                    }
                }
                // FillPhysOut of PhysState_OffBoard_Pushing (0x82d46fd0) publishes its own grab,
                // not BipedGround's per-tick decision; the next ProcessInput (2476 bit 21) and
                // the animation read it.
                if inputs.in_state {
                    self.skater.player_input.physical.off_board.flag_304 = u8::from(self.pushing.grabbing_object());
                }
            }
            self.advanced_this_tick = true;
            if !self.teleport_pending() {
                self.completed_teleport = self.awaiting_teleport.take().or(self.completed_teleport);
                if let Some(target) = self.completed_teleport {
                    let requested = unflatten(target.transform);
                    let bone = |name: &str| self.output.bone_names.iter().position(|n|n.eq_ignore_ascii_case(name))
                        .map(|i|self.output.world_bones[i][3]);
                    let feet = bone("LeftToeBase").zip(bone("RightToeBase")).map(|(left,right)| {
                        let sign=if target.foot_forward==Some(0) {-1.0} else {1.0};
                        sign*((left[0]-right[0])*requested[2][0]+(left[2]-right[2])*requested[2][2])
                    });
                    crate::diagnostics::test_log::event("skate3_spawn_complete",format_args!(
                        "tick={} generation={:?} foot={:?} actual_foot={} natural={} on_board={} requested_at={:?} root_at={:?} board_at={:?} leading_foot_projection={feet:?}",
                        self.output.tick,target.generation,target.foot_forward,self.skater.animation.checkpoint_stance(),
                        self.skater.animation.natural_stance(),target.on_board,requested[2],self.output.effective_root_transform()[2],self.output.board_transform[2]));
                }
            }
            let reactions = self.take_body_reactions();
            world.try_apply_skate3_reactions(&self.host_bodies, reactions)?;
            // Camera effects are a separate mod preference. Suppression leaves
            // native story/script rates alone; no baseline reset is sent.
            camera_rate::submit(&world.sim_rate, &self.camera.simulation_rate_requests,
                self.airtime_slow_motion, self.output.tick)?;
        }
        if crate::diagnostics::test_log::enabled() {
            let output = &self.output;
            let spawn_frames=self.skater.player_input.processed.frames_since_teleport_2584;
            if spawn_frames<12 && !output.offboard {
                let rendered=["Left_WheelFront","Right_WheelFront","Left_WheelBack","Right_WheelBack"].map(|name|
                    output.bone_names.iter().position(|bone|bone.eq_ignore_ascii_case(name)).map(|bone|output.world_bones[bone][3]));
                crate::diagnostics::test_log::event("skate3_spawn_wheels",format_args!(
                    "tick={} frames_since_reset={spawn_frames} state={} generation={:?} physical_gaps_m={:?} rendered_centres={rendered:?} contacts={:?}",
                    output.tick,output.state,self.awaiting_teleport.and_then(|target|target.generation),
                    self.test_spawn_wheel_gaps(),output.physical.collision.wheel_contact_3296_3299));
            }
            let finite_pose = output.world_bones.iter().flatten().flatten().all(|v| v.is_finite());
            crate::diagnostics::test_log::event("skate3_physics", format_args!("tick={} state={} category={} offboard={} wiping_out={} root={:?} root_velocity={:?} root_omega={:?} board={:?} board_velocity={:?} centre_of_mass={:?} collision={:?} ground={:?} bones={} finite_pose={} teleport_pending={} mirrored_world_bodies={} rails={}",
                output.tick, output.state, output.category, output.offboard, output.wiping_out,
                output.root_transform, output.root_velocity, output.root_omega, output.board_transform,
                output.board_velocity, output.centre_of_mass, output.physical.collision,
                output.physical.ground, output.world_bones.len(), finite_pose,
                self.teleport_pending(), self.world_body_count, self.grind_primitive_count()));
            if output.state==601 || (output.state==300 && output.tick%6==0) {
                crate::diagnostics::test_log::event("skate3_control_path",format_args!(
                    "tick={} state={} camera_axes={:?} derived_left={:?} state83={} body_adjust={:?} bail_control={:?}",
                    output.tick,output.state,self.controls.sampled_camera_axes(),
                    [self.controls.controller.words()[7],self.controls.controller.words()[8]].map(f32::from_bits),
                    output.gameplay.state_flags[83-52],self.skater.animation_input.extra.body_adjust,
                    self.skater.animation_input.extra.wipeout_control));
            }
            if output.state==300 && output.tick%6==0 {
                let w=&self.skater.wipeout_state.state;
                crate::diagnostics::test_log::event("skate3_hom_control",format_args!(
                    "tick={} focused={} profile={} gesture={:?} control={:?} response={} responses={} speed={} over={} flags2484={:08x}",
                    output.tick,self.input_focused,w.profile,
                    self.skater.animation_input.extra.wipeout_gesture,self.skater.animation_input.extra.wipeout_control,
                    w.response_strength(),w.response_count,w.maximum_speed,w.over,self.skater.player_input.processed.flags_2484));
            }
            if output.tick % 60 == 0 {
                crate::diagnostics::test_log::event("skate3_physics_full", format_args!("tick={} input={:?}", output.tick, output.physical));
            }
            if !finite_pose { crate::diagnostics::test_log::anomaly("nonfinite_animation", &format!("tick={} state={} category={}", output.tick, output.state, output.category)); }
        }
        self.publish_body_views(world)
    }

    fn character_physical_pose(&self) -> Result<Vec<([f32; 16], &crate::skate3_native::BodyOutput)>, String> {
        self.skeleton_bones
            .iter()
            .map(|(name, bone_to_volume)| {
                let part = self
                    .output
                    .skeleton_parts
                    .iter()
                    .find(|part| part.bone_name.eq_ignore_ascii_case(name))
                    .ok_or_else(|| format!("Skate 3 pose lacks Skate 2 physical bone {name}"))?;
                let bone = flatten(part.bone_transform);
                Ok((
                    crate::skateboard_spawn_math::skateboard_spawn_compose(&bone, bone_to_volume),
                    &part.body,
                ))
            })
            .collect()
    }

    fn publish_body_views(&self, world: &mut PhysicalAssemblyWorld) -> Result<(), String> {
        let skeleton = self.character_physical_pose()?;
        let mut views = Vec::with_capacity(self.board.len() + self.skeleton.len());
        // Board part matrices/rates are the solved seven native bodies, not a
        // rigid board transform applied to independently rotating trucks.
        for ((body, frame), solved) in self
            .board
            .iter()
            .zip(&self.board_frames)
            .zip(&self.output.board_parts)
        {
            let transform = crate::skateboard_spawn_math::skateboard_spawn_compose(
                &flatten(solved.transform),
                frame,
            );
            views.push((*body, transform, velocity_at(solved, &transform), solved.omega));
        }
        for ((body, frame), (transform, solved)) in self
            .skeleton
            .iter()
            .zip(&self.skeleton_frames)
            .zip(skeleton)
        {
            let transform =
                crate::skateboard_spawn_math::skateboard_spawn_compose(&transform, frame);
            views.push((*body, transform, velocity_at(solved, &transform), solved.omega));
        }
        world.backend_mut().try_publish_mode_body_views(&views)
    }

    pub(crate) fn grind_primitive_count(&self) -> usize {
        self.physics.grind_world.primitives().len()
    }
    pub(crate) fn queue_teleport(
        &mut self,
        transform: [f32; 16],
        on_board: bool,
        generation: u32,
        foot_forward: u32,
    ) -> Result<(), String> {
        if foot_forward > 1 { return Err("Invalid native teleport leading foot".into()); }
        if transform.iter().any(|v| !v.is_finite()) {
            return Err("Non-finite host teleport".into());
        }
        self.queued_teleport = Some(HostTeleport {
            transform,
            on_board,
            generation: Some(generation),
            foot_forward: Some(foot_forward),
        });
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn test_offboard_motion_trace(&self)->String {
        let s=&self.skater;
        let hands=["LEFTARM","LEFTFOREARM","LEFTHAND","RIGHTARM","RIGHTFOREARM","RIGHTHAND"].map(|name|{
            let bone=s.animation.evaluator.frames.bone_names.iter().position(|n|n.eq_ignore_ascii_case(name)).unwrap();
            let physical=s.animated_skeleton.bone_indices.iter().position(|b|*b==bone).unwrap();
            let world=compose_affine(&s.animated_skeleton.roots.animation_to_world,&s.animation.packet.hierarchy[bone]);
            let actual=self.output.skeleton_parts[physical].bone_transform;
            let error=(0..3).map(|i|(actual[3][i]-world[3][i]).powi(2)).sum::<f32>().sqrt();
            (name,world,actual,error)
        });
        let danger=s.animation.attributes.entries().iter().filter(|a|
            a.name==skate3_core::animation::skeleton_input::name::encode(b"DangerZone"))
            .copied().collect::<Vec<_>>();
        let ik=["LEFTARM","LEFTFOREARM","LEFTHAND","RIGHTARM","RIGHTFOREARM","RIGHTHAND"].map(|name| {
            let bone=s.animation.evaluator.frames.bone_names.iter().position(|n|n.eq_ignore_ascii_case(name)).unwrap();
            let part=s.animated_skeleton.bone_indices.iter().position(|b|*b==bone).unwrap();
            let target=compose_affine(&s.animated_skeleton.roots.animation_to_world,&s.skeleton_input.drive_frames[part]);
            let physical=s.skeleton.record.pose[part];
            let distance=(0..3).map(|i|(physical[3][i]-target[3][i]).powi(2)).sum::<f32>().sqrt();
            let max_basis=(0..3).flat_map(|i|(0..3).map(move |j|(physical[i][j]-target[i][j]).abs())).fold(0.0f32,f32::max);
            let drive=s.skeleton_drives.bones[part].as_ref().map(|d|(d.dynamics.mode,d.dynamics.strengths));
            let actual=self.output.skeleton_parts[part].bone_transform;
            let rendered=self.output.world_bones[bone];
            let render_error=(0..4).flat_map(|i|(0..3).map(move |j|(actual[i][j]-rendered[i][j]).abs())).fold(0.0f32,f32::max);
            (name,distance,max_basis,drive,render_error)
        });
        format!("state={} root={:?} forward={:?} velocity={:?} turn={} axes={:?} biped={:?} board_held={} flags={:08x?} pinned={} {} hands={hands:?} ik={ik:?} collision_weight={} collision_mode={:?} intent={:?} danger={danger:?}",
            self.output.state,self.output.root_transform[3],self.output.root_transform[2],
            s.biped_ground.controller.state.motion.velocity_480,s.biped_ground.controller.state.motion.angular_velocity_688,
            self.controls.sampled_camera_axes(),[s.animation_input.extra.biped_world_x,s.animation_input.extra.biped_world_z],s.player_input.physical.off_board.flag_311,
            [s.player_input.processed.flags_2472,s.player_input.processed.flags_2476,s.player_input.processed.flags_2480],
            self.actor_input.pinned,self.test_input_trace(),s.collision_feedback.drive_weight,(s.skeleton_collision.partial_ragdoll,s.skeleton_collision.is_ragdoll),s.biped_ground.controller.state.intent)
    }
    /// Skate 3 gestures in the skater's own channel stack (CharacterGesture 82BAA848), under a
    /// later reach or shove as TU3 stacks them: Up, Down, Left, Right selections, or none with
    /// the Skate 2 gesture set.
    pub(crate) fn set_graph_gestures(&mut self, selections: Option<[u32; 4]>) {
        self.physics.set_gesture_preferences(selections);
    }
    /// The graph gesture as Skate 2's MotionGraph publishes it: a started gesture's id once, then
    /// Invalid (35) while it plays.
    pub(crate) fn graph_gesture_fact(&self) -> (u32, bool) {
        self.skater.animation.motion.gesture_publication.map_or((35, false), |publication| {
            (if publication.started { crate::skate3_gesture_playback::native_gesture_id(publication.gesture) } else { 35 }, true)
        })
    }
    pub(crate) fn prepared_movement(&self)->Option<(&[String],&[i32],&[skate3_core::animation::output::Sqt])> {
        self.prepared_tick.as_ref()?.frame.as_ref()?;
        Some((&self.skater.animation.evaluator.frames.bone_names,&self.skater.animation.evaluator.frames.parents,&self.skater.animation.pose))
    }
    /// The prepared tick's body tree commands, moved out for the Actor's hand streams, and the
    /// leading foot with fakie as the S2 Actor reports it (RIGHT as true).
    pub(crate) fn take_prepared_body_commands(&mut self)->Option<(Vec<skate3_core::animation::playback_tree::PoseCommand>,bool)> {
        self.prepared_tick.as_ref()?.frame.as_ref()?;
        Some((std::mem::take(&mut self.skater.animation.body_commands),!self.skater.animation.foot_forward()))
    }
    pub(crate) fn capture_composed_actor_pose(&mut self,actor:&crate::actor_construction::ActorConstruction)->Result<(),String> {
        if self.prepared_movement().is_none(){return Ok(());}
        let (local,hierarchy)=actor.try_composed_body_pose()?;
        if local.len()!=self.skater.animation.packet.local.len() || hierarchy.len()!=local.len() {
            return Err("Actor composed pose differs from its selected physical player".into());
        }
        self.skater.animation.packet.local=local;
        self.skater.animation.packet.hierarchy=hierarchy;
        Ok(())
    }
    pub(crate) fn advanced_this_tick(&self) -> bool { self.advanced_this_tick }
    /// TU3's shove probe point while the shove behaviour runs it (state flag 86); the host
    /// answers with the pedestrians near it ([`Self::set_shove_pedestrians`]).
    pub(crate) fn shove_query_point(&self) -> Option<[f32; 3]> { self.skater.shove.query_point() }
    pub(crate) fn set_shove_pedestrians(&mut self, pedestrians: Vec<super::shove_probe::ShovePedestrian>) {
        self.skater.shove.set_pedestrians(pedestrians);
    }
    /// The other skaters the probe strikes as type 1 targets.
    pub(crate) fn set_shove_skaters(&mut self, skaters: Vec<super::shove_probe::ShoveSkater>) {
        self.skater.shove.set_skaters(skaters);
    }
    /// A strike on a pedestrian this tick (`ProcessOutput`'s intents, read by 0x8275f0d8).
    pub(crate) fn take_shove_hit(&mut self) -> Option<super::shove_probe::ShoveHit> { self.skater.shove.hit.take() }
    /// This tick's solver contacts with host bodies: the host body, whether the skater's side was
    /// its board, the normal on the host body's side and the contact point.
    pub(crate) fn take_host_contacts(&mut self) -> Vec<(crate::simulation_rigid_body_pool::SimulationRigidBodyObservation, bool, [f32; 3], [f32; 3])> {
        std::mem::take(&mut self.skater.host_contacts).into_iter()
            .filter_map(|contact| Some((self.host_bodies.body(contact.id)?, contact.board, contact.normal, contact.point)))
            .collect()
    }
    pub(crate) fn completed_teleport(&self) -> Option<[f32; 16]> {
        if !self.advanced_this_tick { return None; }
        self.completed_teleport.map(|teleport| teleport.transform).or_else(|| {
            // Automatic source bail recovery has no host Actor generation.
            // Its actual completed external reset still notifies S2's mission
            // and HUD consumers through their ordinary teleport publication.
            (self.awaiting_teleport.is_none() && self.output.teleported && self.output.state != 702).then(||
                flatten(self.skater.player_input.processed.matrix_1536.map(|row| row.map(f32::from_bits))))
        })
    }
    pub(crate) fn acknowledge_teleport(
        &self,
        actor: &crate::actor_construction::ActorConstruction,
    ) -> Result<(), String> {
        if let Some(generation) = self
            .completed_teleport
            .and_then(|teleport| teleport.generation)
        {
            actor
                .try_acknowledge_skate3_teleport(generation)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub(crate) fn sync_teleport(
        &mut self,
        actor: &crate::actor_construction::ActorConstruction,
    ) -> Result<(), String> {
        let (generation, transform, on_board, pending) = actor
            .try_skate3_teleport_publication()
            .map_err(|e| e.to_string())?;
        if (self.teleport_generation.is_none() && pending)
            || self
                .teleport_generation
                .is_some_and(|old| old != generation)
        {
            self.queue_teleport(
                transform
                    .known_lanes()
                    .ok_or("Incomplete host teleport matrix")?,
                on_board,
                generation,
                actor.try_last_set_foot_forward().map_err(|error|error.to_string())?,
            )?;
        }
        self.teleport_generation = Some(generation);
        Ok(())
    }
    /// The Actor's update after the source tick: Actor82592518's reply to this tick's reset
    /// request, then PostUpdate's breadcrumb Update82BFB1E0, with the host's network and
    /// living-world services.
    pub(crate) fn post_update_breadcrumbs(&mut self, host: &mut dyn RespawnHost) -> Result<(), String> {
        super::respawn::post_update(&self.physics, &mut self.skater, Some(host))
    }
    /// The Actor's one breadcrumb manager. A new Actor (SkaterManager::OnCommand case 3, a game
    /// start's spawn) restarts the history at its spawn; taking over an Actor carries on with the
    /// crumbs, orientation and position it recorded before.
    pub(crate) fn sync_breadcrumbs(&mut self, host: &ActorBreadCrumbManager) -> Result<(), String> {
        if self.breadcrumb_generation == Some(host.spawn_generation) {
            return Ok(());
        }
        let candidate = |crumb: &SkaterBreadCrumb| skate3_core::player::respawn::Candidate {
            transform: host_matrix(&crumb.transform),
            stance: u32::from(!crumb.foot_forward_right),
            offboard: crumb.offboard,
            score: 0.,
        };
        let runtime = &mut self.skater.respawn;
        runtime.respawn(skate3_core::player::respawn::Candidate {
            transform: host_matrix(&host.spawn_position),
            stance: self.skater.animation.checkpoint_stance(),
            offboard: false,
            score: 0.,
        });
        runtime.import(&host.ring_oldest_first().iter().map(candidate).collect::<Vec<_>>());
        if self.breadcrumb_generation.is_none() {
            let position = host.last_known_actor_position;
            let [x, y, z] = position.xyz;
            runtime.continue_from(host_matrix(&host.last_known_orientation), [x, y, z, position.w.unwrap_or(0.)]);
        }
        crate::diagnostics::test_log::event("skate3_breadcrumb_spawn", format_args!(
            "generation={} takeover={} spawn={:?} entries={}",
            host.spawn_generation, self.breadcrumb_generation.is_none(),
            host.spawn_position.columns()[3].xyz, runtime.history().entries().len()));
        self.breadcrumb_generation = Some(host.spawn_generation);
        Ok(())
    }
    /// The history as the host Actor's manager shows it to Skate 2's consumers.
    pub(crate) fn mirror_breadcrumbs(&self, host: &mut ActorBreadCrumbManager) {
        let history = self.skater.respawn.history();
        host.mirror(
            history.entries().iter().copied().map(host_crumb),
            SkeletonRootMatrix::known(flatten(history.current_orientation)),
            SkeletonRootVector::known(history.current_position),
        );
    }
    /// GetSuggestedRespawnPosition82BFC828 over the history.
    pub(crate) fn suggested_breadcrumb(&self, host: &mut dyn RespawnHost) -> Result<SkaterBreadCrumb, String> {
        super::respawn::suggested(&self.physics, &self.skater, host).map(host_crumb)
    }
    /// PopSuggestedRespawnPosition82BFC9B0 over the history.
    pub(crate) fn pop_breadcrumb(&mut self, host: &mut dyn RespawnHost) -> Result<SkaterBreadCrumb, String> {
        super::respawn::pop_suggested(&self.physics, &mut self.skater, host).map(host_crumb)
    }
    /// IsPositionOk82BFAFF0 for a session marker.
    pub(crate) fn breadcrumb_position_ok(&self, transform: &SkeletonRootMatrix, host: &mut dyn RespawnHost) -> Result<bool, String> {
        super::respawn::position_ok(&self.physics, &self.skater, &host_matrix(transform), host)
    }
    pub(crate) fn diagnostic_controls(&self) -> (bool, [f32; 4], f32) {
        (self.input_focused,
            std::array::from_fn(|i| f32::from_bits(self.controls.controller.words()[7+i])),
            self.output.effective_root_transform()[2][0].atan2(self.output.effective_root_transform()[2][2]))
    }
    pub(crate) fn diagnostic_transition(&self) -> String {
        let s = &self.skater;
        let motion = s.animation.motion_controller.frame.current
            .and_then(|i|self.graphs.motion.binding.states.get(i)).map(|s|s.name.as_str());
        format!("state={} pinned={} axes={:?} turn={} motion={motion:?} MGturn={:?} landing={:?}",
            self.output.state,self.actor_input.pinned,self.diagnostic_controls().1,
            s.animation_input.fields.turn,s.animation.motion.animation.motion_intents.get("Turn").copied(),
            s.landing_quality)
    }
    pub(crate) fn set_input_focus(&mut self, focused: bool) { self.input_focused = focused; }
    pub(crate) fn capture_actor_input(&mut self, actor: &crate::actor_construction::ActorConstruction) -> Result<(),String> {
        let mut policy=actor.try_skate3_actor_input().map_err(|error|error.to_string())?;
        if !self.input_focused {
            policy.pinned = true;
            policy.pinned_gestures = false;
        }
        if policy!=self.actor_input {
            crate::diagnostics::test_log::event("skate3_actor_policy",format_args!(
                "tick={} pinned={} pinned_gestures={} prevent_manual_respawn={}",
                self.physics.ticks,policy.pinned,policy.pinned_gestures,policy.prevent_manual_respawn));
        }
        let allocation=actor.cached_physical_allocation().map_err(|e|e.to_string())?;
        let host_inputs=allocation.owner().inputs();
        self.host_cvars=Some(host_inputs.cvars.clone());
        self.host_vault=Some(std::sync::Arc::clone(&host_inputs.vault));
        self.actor_input=policy;
        self.controls.host_actor_input=policy;
        // These are semantic S2 Actor policies, not a bitwise alias of S3's
        // unrelated Actor1908 input-filter word.
        self.physics.animation_profile.force_braking=self.actor_input.pinned;
        // TU3 Actor+44 virtual24 (82594AA8): ForceBrake stops a gesture start only while front-end
        // states 27 or 55 are stacked, so a pinned skater (an online countdown) still gestures.
        // The port's front ends take this player's focus instead.
        self.physics.animation_profile.gesture_force_brake_bypass=self.input_focused;
        self.physics.animation_profile.prevent_manual_respawn=self.actor_input.prevent_manual_respawn;
        Ok(())
    }
    pub(crate) fn capture_actor_physics(&mut self, actor: &crate::actor_construction::ActorConstruction) -> Result<(),String> {
        let input=actor.published_animation_physics_input().map_err(|error|error.to_string())?;
        let input=input.lock().map_err(|_|"native Actor physics input lock")?;
        let profile=&mut self.physics.animation_profile;
        let equipment=self.equipment.unwrap_or([input.truck_tightness,input.wheel_hardness]);
        if [profile.truck_tightness,profile.wheel_hardness].map(f32::to_bits)!=equipment.map(f32::to_bits) {
            crate::diagnostics::test_log::event("skate3_equipment",format_args!(
                "tick={} truck_tightness={} wheel_hardness={} skate3={}",
                self.physics.ticks,equipment[0],equipment[1],self.equipment.is_some()));
        }
        if input.flags.living_world_impulse_active {
            crate::diagnostics::test_log::event("skate3_world_impulse",format_args!(
                "tick={} direction={:?} source=skate2_actor",self.physics.ticks,input.living_world_impulse));
        }
        [profile.truck_tightness,profile.wheel_hardness]=equipment;
        profile.prevent_manual_respawn=input.flags.prevent_manual_respawn;
        profile.ignore_respawn_reset_button=u8::from(input.flags.ignore_respawn_reset_button);
        profile.force_braking=input.flags.force_braking || !self.input_focused;
        profile.external_impulse=input.flags.living_world_impulse_active.then_some(input.living_world_impulse.map(f32::to_bits));
        Ok(())
    }
    /// S2 FootForward RIGHT0/LEFT1 matches S3's checkpoint enum; natural
    /// stance uses GOOFY1/REGULAR0. The customization comes from the actual
    /// profile; each marker/breadcrumb independently requests its leading foot.
    pub(crate) fn configure_character_stance(&mut self, regular: bool, foot_forward: u32, entering: bool) -> Result<(),String> {
        if foot_forward > 1 { return Err("Invalid native character leading foot".into()); }
        self.skater.animation.set_natural_stance(u32::from(!regular));
        if entering { self.skater.animation.restore_foot_forward(foot_forward != 0); }
        Ok(())
    }
    /// GetCACSettings82590D20/82590B50: profile refresh does not invent a
    /// posture request; authored Play Begin owns that construction latch.
    pub(crate) fn configure_animation_style(&mut self, settings: skate2_formats::skater_profile::PcSkate3Style) -> Result<(),String> {
        self.skater.animation.set_style_preferences(settings)
    }
    /// The Actor reset 82593998/825939B0 reads the local skater's own PlayersBackEnd trucks and
    /// wheels (Skate 3's CAC record, 0.7 for a new skater; Skate 2's are 0.3 and 0.7).
    pub(crate) fn configure_equipment(&mut self, equipment: Option<[f32; 2]>) {
        self.equipment = equipment.filter(|values| values.iter().all(|value| (0.0..=1.0).contains(value)));
    }
    pub(crate) fn set_camera_type(&mut self, value: u32) { self.camera.set_camera_type(value); }
    pub(crate) fn set_invert_look(&mut self, value: [bool; 2]) { self.camera.set_invert_look(value); }
    pub(crate) fn set_airtime_slow_motion(&mut self, enabled: bool) { self.airtime_slow_motion = enabled; }
    /// Diagnostic only: signed wheel-sphere gap against the actual loaded
    /// collision surface. Queries do not move a body or modify solver state.
    pub(crate) fn test_spawn_wheel_gaps(&self) -> Result<[f32;4],String> {
        use skate3_core::math::Vector3;
        let poses=self.physics.board.part_transforms();
        let mut gaps=[0.;4];
        for i in 0..4 {
            let p=poses[i].translation;
            let hit=self.physics.world.query_thin_line(
                Vector3::new(p.x,p.y+0.5,p.z),Vector3::new(p.x,p.y-0.5,p.z))
                .map_err(str::to_owned)?.ok_or_else(||format!("wheel{i} has no collision ground near {p:?}"))?;
            let q=hit.geometry.position;let n=hit.geometry.normal;
            gaps[i]=(p.x-q.x)*n.x+(p.y-q.y)*n.y+(p.z-q.z)*n.z-self.physics.settings.wheel_radius;
        }
        Ok(gaps)
    }
    #[cfg(test)]
    pub(crate) fn test_ground_height(&self,point:[f32;3])->Result<f32,String> {
        use skate3_core::math::Vector3;
        self.physics.world.query_thin_line(Vector3::new(point[0],point[1]+50.,point[2]),
            Vector3::new(point[0],point[1]-50.,point[2])).map_err(str::to_owned)?
            .map(|hit|hit.geometry.position.y).ok_or_else(||format!("marker fixture has no resident terrain at {point:?}"))
    }
    #[cfg(test)]
    pub(crate) fn test_spawn_stance(&self) -> (u32, i32) {
        (self.skater.animation.checkpoint_stance(), self.skater.animation.natural_stance())
    }
    #[cfg(test)]
    pub(crate) fn test_boneless_trace(&self) -> String {
        let f=&self.skater.boneless;
        let p=&self.skater.player_input.processed;
        format!("toe={} right={} anchor={:?} com={:?} velocity={:?} prepared={:?} flags={:08x}/{:08x} {}",
            f.toe,f.right,f.anchor,p.vectors_544_560_592_608[2].map(f32::from_bits),
            p.vectors_544_560_592_608[3].map(f32::from_bits),p.prepared_jump_704.map(f32::from_bits),
            p.flags_2468,p.flags_2480,self.test_input_trace())
    }
    #[cfg(test)]
    pub(crate) fn test_bail_control_probe(&self)->([f32;2],Option<[f32;2]>) {
        (self.skater.animation_input.extra.wipeout_control,self.controls.sampled_camera_axes())
    }
    #[cfg(test)]
    pub(crate) fn test_plant_control_probe(&self)->([f32;2],[f32;4],Option<[f32;2]>) {
        (self.skater.animation_input.extra.body_adjust,
            self.skater.footplant.test_release_velocity(),self.controls.sampled_camera_axes())
    }
    #[cfg(test)]
    pub(crate) fn test_footplant_trace(&self) -> String {
        let f=&self.skater.footplant;
        let p=&self.skater.player_input.processed;
        format!("enabled={} toe={:?} candidate={} hit={} perform={} active={} requested={} time={} duration={} blend={} flags={:08x}/{:08x}/{:08x} {}",
            f.enabled,f.selected_toe,f.candidate,f.hit,f.perform,f.contact_active,f.requested,
            f.contact_time,f.scalar_596,f.target_blend,p.flags_2476,p.flags_2480,p.flags_2484,self.test_input_trace())
    }
    #[cfg(test)]
    pub(crate) fn test_grab_scene(&self,world:&PhysicalAssemblyWorld)->Result<(usize,usize,usize),String> {
        self.grabs.test_audit(world,&self.physics.world,&self.physics.offboard_grab_scene)
    }
    #[cfg(test)]
    pub(crate) fn test_living_world_impulse(&self) -> (bool,[u32;4]) {
        (self.skater.player_input.processed.flags_2468 & 4 != 0,self.skater.player_input.processed.vector_1520)
    }
    #[cfg(test)]
    pub(crate) fn test_actor_input(&self) -> (bool,bool,Vec<String>,[f32;2]) {
        (self.actor_input.pinned,self.physics.animation_profile.prevent_manual_respawn,
            self.controls.named_intents.keys().cloned().collect(),
            [self.physics.animation_profile.truck_tightness,self.physics.animation_profile.wheel_hardness])
    }
    #[cfg(test)]
    pub(crate) fn test_input_trace(&self) -> String {
        let action = self.skater.animation.action_controller.frame.current
            .and_then(|index|self.graphs.action.binding.states.get(index)).map(|state|state.name.as_str());
        let motion = self.skater.animation.motion_controller.frame.current
            .and_then(|index|self.graphs.motion.binding.states.get(index)).map(|state|state.name.as_str());
        let intents: Vec<_> = self.controls.intents.iter().filter(|intent|intent.value != 0.)
            .map(|intent|(intent.name,intent.value)).collect();
        format!("buttons={:#x} stick={:?} intents={intents:?} AG={action:?} MG={motion:?} capabilities={:#x}",
            self.controls.controller.words()[13], &self.controls.controller.words()[7..11],
            self.skater.player_input.physical.scoring.capabilities_204)
    }
    #[cfg(test)]
    pub(crate) fn camera_shot(&self) -> &str { self.camera.selected_shot() }
    pub(crate) fn output_frame(&self) -> std::sync::Arc<FrameOutput> { std::sync::Arc::clone(&self.output) }
    pub(crate) fn output(&self) -> &FrameOutput {
        &self.output
    }
    #[cfg(test)]
    pub(crate) fn world_body_count(&self) -> usize {
        self.world_body_count
    }
    /// A user-selected physics change returns this Actor to its existing S2
    /// assembly. The matching Actor reset applies these rates once, after its
    /// real body resets and before Ground.Enter samples the deck speed.
    pub(crate) fn return_to_skate2(
        self,
        world: &mut PhysicalAssemblyWorld,
        actor: &crate::actor_construction::ActorConstruction,
        owner: &std::sync::Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), String> {
        let mut rates = Vec::with_capacity(self.board.len() + self.skeleton.len());
        for body in self.board.iter().chain(&self.skeleton) {
            let view = world.backend().try_external_body_snapshot(*body)?;
            rates.push(exit::BodyRate { body: *body, centre: view.center_of_mass,
                velocity: view.velocity, omega: view.omega });
        }
        let transform = crate::skeleton_root_transforms::SkeletonRootMatrix::known(flatten(self.output.effective_root_transform()));
        let on_board = !self.output.offboard;
        let mut pending = owner.skate3_exit.lock().map_err(|_| "native exit lock")?;
        actor.try_publish_teleport(&transform, on_board).map_err(|error|error.to_string())?;
        let generation = actor.try_teleport_publication().map_err(|error|error.to_string())?.0;
        *pending = Some(exit::PlayerExit { generation, rates });
        drop(pending);
        world.backend_mut().try_return_external_player()?;
        world.try_return_native_skater_root(board, skeleton, transform, owner)?;
        self.release(world)
    }
    pub(crate) fn release(self, world: &mut PhysicalAssemblyWorld) -> Result<(), String> {
        world.backend_mut().try_select_external_player(&[])?;
        // Source camera requests have a one-tick lifetime. Let the native
        // countdown expire them without cancelling a story/script rate.
        Ok(())
    }
}

/// Preserve S3's request lifetime through S2's existing timer message API.
pub(crate) fn host_sim_rate_request(request: skate3_core::camera::SimulationRateRequest) -> crate::simulation_sim_rate::SetSimRate {
    let ticks = request.ticks as i32;
    crate::simulation_sim_rate::SetSimRate {
        sim_time: request.timestep,
        ticks_of_slow_motion: if request.timestep == f32::from_bits(0x3c88_8889) { 0 }
            else if ticks > 0 { ticks }
            // S3's fallback stores180 directly; S2's API adds one.
            else { 179 },
    }
}

pub(crate) fn flatten(matrix: [[f32; 4]; 4]) -> [f32; 16] {
    std::array::from_fn(|i| matrix[i / 4][i % 4])
}
fn unflatten(matrix: [f32; 16]) -> [[f32; 4]; 4] {
    std::array::from_fn(|c| std::array::from_fn(|r| matrix[c * 4 + r]))
}

impl Skate3Player {
    pub fn period(&self) -> std::time::Duration {
        self.physics.period()
    }

    /// Replace the current tick's host-owned NPC/car/prop snapshots. A body has
    /// exactly one identity and one reaction recipient; remote-player prediction
    /// and multiplayer clock rules are not enabled by this host boundary.
    pub fn set_moving_bodies(
        &mut self,
        bodies: Vec<skate_dynamics::SolidBody>,
    ) -> Result<(), String> {
        self.set_moving_mesh_bodies(bodies, Vec::new())
    }

    /// `set_moving_bodies`, where each of `meshes` names one of `bodies` whose
    /// authored collision mesh (native triangles at its current placement)
    /// answers the board, skeleton, wheel-line and nearby-triangle queries.
    /// Those bodies stay finite-mass solver proxies and receive reactions.
    pub(crate) fn set_moving_mesh_bodies(
        &mut self,
        bodies: Vec<skate_dynamics::SolidBody>,
        meshes: Vec<crate::simulation_assembly_backend::constraint_session::skate3_world::Skate3MeshBody>,
    ) -> Result<(), String> {
        let mut ids = std::collections::BTreeSet::new();
        for body in &bodies {
            if body.id == 0
                || !ids.insert(body.id)
                || !body.inverse_mass.is_finite()
                || body.inverse_mass < 0.
                || !body.center_of_mass.is_finite()
                || !body.linvel.is_finite()
                || !body.angvel.is_finite()
                || !body.inertia_rotation.is_finite()
                || !body.inverse_inertia.is_finite()
            {
                return Err(
                    "Moving bodies require unique nonzero IDs and finite physical state".into(),
                );
            }
        }
        let mut proxies = super::world_proxies::Proxies::default();
        let meshes: Vec<_> = meshes
            .into_iter()
            .filter_map(|mesh| {
                let body = bodies.iter().find(|body| body.id == mesh.id)?;
                crate::skate3_native::modding::moving_queries::MeshQuery::new(body, mesh.triangles)
            })
            .collect();
        // The same snapshots must answer the source engine's wheel, biped and
        // trajectory queries as well as its solid solver contacts.
        self.physics.set_external_queries(Some(std::sync::Arc::new(
            crate::skate3_native::modding::moving_queries::MovingQueries::new(bodies.clone(), meshes.clone()),
        )));
        for body in bodies {
            let id = body.id;
            proxies.append_solid(body, &self.physics, &self.skater, true);
            if let Some(mesh) = meshes.iter().find(|mesh| mesh.id == id)
                && let Some(&(index, _)) = proxies.solids.last().filter(|(_, solid)| solid.id == id)
            {
                proxies.meshes.push((index, mesh.clone()));
            }
        }
        self.physics.world_proxies = proxies;
        Ok(())
    }

    pub fn set_external_queries(
        &mut self,
        queries: std::sync::Arc<dyn skate3_core::physics::board_world::ExternalQueries>,
    ) {
        self.physics.set_external_queries(Some(queries));
    }

    /// Bind actual authored grab records to their actual collision meshes.
    /// Identities are supplied by the host's object registry, never inferred.
    pub fn set_grab_objects(
        &mut self,
        objects: Vec<skate3_core::player::offboard::grab_scene::Object>,
        mesh_assemblies: Vec<(u32, u32)>,
    ) -> Result<(), String> {
        let bindings = mesh_assemblies
            .into_iter()
            .map(|(mesh, assembly)| super::offboard::grab_scene::MeshAssembly { mesh, assembly })
            .collect();
        let registry =
            super::offboard::grab_scene::Registry::new(&self.physics.world, objects, bindings)?;
        self.physics.offboard_grab_scene = registry;
        Ok(())
    }

    /// Drain once after the tick, then apply each correction to the matching
    /// Skate 2 world body before its next publication.
    pub fn take_body_reactions(&mut self) -> Vec<BodyReaction> {
        std::mem::take(&mut self.physics.world_proxies.dynamics_deltas)
            .into_iter()
            .map(
                |(
                    id,
                    linear_velocity_delta,
                    angular_velocity_delta,
                    position_delta,
                    orientation_delta,
                )| BodyReaction {
                    id,
                    linear_velocity_delta,
                    angular_velocity_delta,
                    position_delta,
                    orientation_delta,
                },
            )
            .collect()
    }

    pub(crate) fn set_profile(&mut self, profile: &str) -> Result<(), String> {
        let _asset_load = crate::diagnostics::test_log::span("skate3_asset_graph_load");
        let difficulty = crate::skate3_native::difficulty::Difficulty::parse(profile)?;
        self.physics.set_difficulty(difficulty);
        Ok(())
    }

    /// Skate 2's tuning on the source solver. Read the original host Vault;
    /// shared settings and the push/jump limits keep their actual provenance.
    /// S3-only settings inherit its authored base collection, not Normal.
    pub(crate) fn set_skate2_profile(
        &mut self,
        vault: &std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>,
    ) -> Result<(), String> {
        use skate2_formats::attrib::string_hash64 as hash;
        let mut data = (*stock_collections()?).clone();
        let mut fields = std::collections::BTreeMap::new();
        let mut field = |target: &str, class: &str, source: u64| -> Result<(), String> {
            let word = vault.instance(hash(class), hash("default"))
                .try_scalar_word(source, 0).map_err(|error|error.to_string())?
                .ok_or_else(||format!("Original Skate 2 {class}.{source:016X} missing"))?;
            if !f32::from_bits(word).is_finite() { return Err(format!("Invalid original Skate 2 {class}.{source:016X}")); }
            fields.insert(target.into(), skate_data::collections::Field {
                type_name: "EA::Reflection::Float".into(), data: format!("{word:08X}"), array: None,
            });
            Ok(())
        };
        // S2 Toolkit_CalcPushAcceleration82b272f8 has one change cap at all speeds.
        for target in ["MaxPushDVStart", "MaxPushDVEnd"] { field(target, "physics_push", 0x76fe_ee54_a3ad_2af5)?; }
        // S2 GroundJumpAttribute minimum/maximum share physical height units.
        for target in ["JumpMinHeight", "Hash_BE3F74F978D777E5"] { field(target, "physics_jump", 0xbb9e_9baf_ba1b_29e6)?; }
        field("JumpMaxHeight", "physics_jump", 0x0b24_54c6_6641_0860)?;
        for (target, class) in [
            ("PumpEffectFactor", "physics_pumping"),
            ("PumpEffectFactorAbsorption", "physics_pumping"),
            ("UnintentionalPumpScalar", "physics_pumping"),
            ("Wipeout_GroundXZAcceleration", "physics_wipeout"),
            ("MaxAutoBodySpinSpeed", "physics_airstates"),
        ] { field(target, class, hash(target))?; }
        crate::diagnostics::test_log::event("difficulty_profile", format_args!("selected=skate2 solver=skate3 source=original_skate2_vault fields={fields:?} base=authored_default"));
        data.override_profile("physics_mode", "test", "default", fields)?;
        self.skater.reload_difficulty(&data)?;
        self.physics.set_difficulty(crate::skate3_native::difficulty::Difficulty::Custom);
        Ok(())
    }

    /// Prepare the complete difficulty table before replacing the active slot.

    pub(crate) fn set_motorized_profile(&mut self, base: &str) -> Result<(), String> {
        let _ = crate::skate3_native::difficulty::Difficulty::parse(base)?;
        let mut data = (*stock_collections()?).clone();
        let target = data
            .field("physics_mode", "motorized", "MotorTopSpeed")?
            .clone();
        // Match Motorized landing protection while inheriting the base handling.
        let landing = data.field("physics_mode", "motorized", "WipeoutCheckForBadLanding")?.clone();
        let enabled = skate_data::collections::Field {
            type_name: "EA::Reflection::Bool".into(),
            data: "01".into(),
            array: None,
        };
        data.override_profile(
            "physics_mode",
            "test",
            base,
            [
                ("MotorEnabled".into(), enabled),
                ("MotorTopSpeed".into(), target),
                ("WipeoutCheckForBadLanding".into(), landing),
            ]
            .into_iter()
            .collect(),
        )?;
        self.skater.reload_difficulty(&data)?;
        self.physics
            .set_difficulty(crate::skate3_native::difficulty::Difficulty::Custom);
        Ok(())
    }

    pub fn replace_scene(&mut self, scene: &Scene) -> Result<(), String> {
        let map = scene.map()?;
        let world = scene.world(self.physics.settings.floor_material)?;
        let grind = crate::skate3_native::grind_world::StaticProvider::new(Some(&map))?;
        let registry = super::offboard::grab_scene::Registry::new(&world, Vec::new(), Vec::new())?;
        self.physics.world.replace_static_geometry(world);
        self.physics.grind_world = std::sync::Arc::new(grind);
        self.skater
            .trajectory
            .bind_grind_world(std::sync::Arc::clone(&self.physics.grind_world));
        self.physics.offboard_grab_scene = registry;
        Ok(())
    }

    pub fn replace_collision_scene(&mut self, scene: &Scene) -> Result<(), String> {
        let world = {
            let _stage = crate::diagnostics::test_log::timed_span("skate3_terrain_world_build");
            scene.world(self.physics.settings.floor_material)?
        };
        self.replace_collision_world(world)
    }

    fn replace_collision_world(&mut self, world: skate3_core::physics::board_world::BoardWorld) -> Result<(),String> {
        let registry = super::offboard::grab_scene::Registry::new(&world, Vec::new(), Vec::new())?;
        self.physics.world.replace_static_geometry(world);
        self.physics.offboard_grab_scene = registry;
        self.skater.offboard_grab.invalidate();
        Ok(())
    }



    pub fn travel(
        &mut self,
        transform: AnimationPartTransform,
        velocity: [f32; 3],
    ) -> Result<(), String> {
        self.skater.travel(transform, Some(velocity))
    }
    pub fn host_teleport(
        &mut self,
        transform: AnimationPartTransform,
        on_board: bool,
    ) -> Result<(), String> {
        // Actor's external-reset packet selects702; its animation graph then
        // restores GivenStance before State61 performs the physical placement.
        // An extra immediate request_teleport_ex resets with the outgoing pose,
        // before that graph, and can turn the new board/leading foot backwards.
        self.interactable_hold = None;
        let mut physical_target = spawn::target(&self.physics.world, transform, on_board)?;
        // Actor targets are effective world facing. Convert once here, after
        // restore_foot_forward, rather than sampling an animation blend packet
        // whose mirror can differ on702's two reset/ground dispatches.
        if on_board && self.skater.animation.stance().1 {
            for axis in [0, 2] { physical_target[axis] = physical_target[axis].map(|v| -v); }
        }
        self.skater.teleport_state.request_manual(physical_target, on_board);
        Ok(())
    }
    pub fn teleport_pending(&self) -> bool {
        self.skater.player_input.pending_teleport().is_some()
            || self.skater.teleport_state.manual_on_board().is_some()
    }

    /// Collect every host sample even when no native fixed tick is due yet.
    pub fn collect_controller(&mut self, raw: Option<RawController>) {
        crate::skate3_native::input::collect_embedded(
            &mut self.input,
            raw.map(|raw| XboxState {
                buttons: raw.buttons,
                triggers: raw.triggers,
                left: raw.left,
                right: raw.right,
            }),
        );
    }



    fn sample_solver_input(&mut self)->Result<(skate3_core::input::gameplay_map::GameplayActions,bool),String> {
        let tick = crate::skate3_native::input::publish_embedded(&mut self.input);
        let mut actions = tick.actions();
        // Same preceding Ground publication as the native input listener.
        self.controls.preferences.automatic_push_enabled=self.skater.player_state.ground_output.as_ref()
            .is_some_and(|g|g.intents_52.selected_mode_below_speed_threshold_58);
        // TU3 Fill 8259A148..A170 reads PhysOut+0x38 (PhysOutAnimation) byte 156, the animation
        // mirror (AnimOutPhysIn+10370), for the automatic push's foot: right when mirrored.
        self.controls.preferences.automatic_push_right=self.output.gameplay.stance[1];
        // A menu is not the native Actor pin. Consume the neutral sample so
        // old stick values cannot survive while the pin suppresses Fill.
        let retained_policy = self.controls.host_actor_input;
        if !self.input_focused { self.controls.host_actor_input.pinned = false; }
        let sampled = self.controls.update_for_physics_with_entry_camera(
            &mut actions, &self.physics, &self.skater, &self.camera, self.entry_camera_basis,
        );
        self.controls.host_actor_input = retained_policy;
        sampled?;
        self.controls.publish_gestures(
            self.physics.difficulty_index(),
            self.skater.player_input.physical.state.state_16,
        );
        self.actor_input.apply(&mut self.controls, self.output.wiping_out);
        Ok((actions,tick.controller_available()))
    }

    pub(crate) fn advance_solver_tick(&mut self) -> Result<FrameOutput, String> {
        let (mut actions,input_available)=self.sample_solver_input()?;
        super::frame::advance(
            &mut self.physics,
            &mut self.skater,
            &mut self.controls,
            &self.graphs,
            &mut actions,
            input_available,
            &mut self.camera,
        )?;
        Ok(export_frame(&self.physics, &self.skater, &self.camera, self.hips_body))
    }


}

/// Skitching's grab manager calls (TU3 Update 0x82D47900..5C), handled by the Skate 2 world in
/// its deferred message pass: contact forces reach a dynamic object, the notifications a vehicle.
fn send_skitch_messages(world: &PhysicalAssemblyWorld, messages: &mut Vec<super::skitching::Message>) -> Result<(), String> {
    use crate::physical_player_grab_spline::manager::{GrabMessage, GrabMessageKind};
    use super::skitching::MessageKind;
    let Some(manager) = world.grab_query_manager().cloned() else {
        messages.clear();
        return Ok(());
    };
    for message in messages.drain(..) {
        let descriptor = crate::physical_player_assembly::remaining_states::GrabDescriptor {
            object_type: message.descriptor.kind as i32,
            object_id: message.descriptor.id as i32,
        };
        let kind = match message.kind {
            MessageKind::ApplyForce { force, position, torque } => GrabMessageKind::ApplyForce {
                force: SkeletonRootVector::known(force),
                position: SkeletonRootVector::known(position),
                torque: SkeletonRootVector::known(torque),
            },
            MessageKind::NotifyGrabbing => GrabMessageKind::NotifyGrabbing,
            MessageKind::NotifyLocalPlayer => GrabMessageKind::NotifyLocalPlayer,
        };
        manager
            .defer_message(GrabMessage { descriptor, kind })
            .map_err(|e| format!("Skitching grab message: {e:?}"))?;
    }
    Ok(())
}

    fn export_frame(physics: &super::Skate3PhysicsState, skater: &super::Skate3ControllerState, camera: &crate::skate3_native::camera::CameraRuntime, hips_body: usize) -> FrameOutput {
        let deck = physics.board.part_transforms()[BodyId::Deck.index()];
        let board_transform = [
            [
                deck.basis.columns[0][0],
                deck.basis.columns[0][1],
                deck.basis.columns[0][2],
                0.,
            ],
            [
                deck.basis.columns[1][0],
                deck.basis.columns[1][1],
                deck.basis.columns[1][2],
                0.,
            ],
            [
                deck.basis.columns[2][0],
                deck.basis.columns[2][1],
                deck.basis.columns[2][2],
                0.,
            ],
            [
                deck.translation.x,
                deck.translation.y,
                deck.translation.z,
                1.,
            ],
        ];
        let velocity = physics.board.bodies()[BodyId::Deck.index()]
            .rates
            .linear_velocity;
        let root = skater.animated_skeleton.roots.animation_to_world;
        let physical = &skater.player_input.physical;
        let skeleton_transforms = skater.skeleton.part_transforms();
        if trace_visual_root() {
            let hips = skater.animation.evaluator.frames.bone_names.iter().position(|name| name.eq_ignore_ascii_case("Hips"));
            let hips_world = hips.and_then(|i| skater.render_pose.get(i)).map(|bone| compose_affine(&root, bone)[3]);
            let reckoning = &physics.riding.reckoning;
            let frames = &physics.riding.reckoning_frames;
            let roots = &skater.animated_skeleton.roots;
            eprintln!(
                "skate3 visual root: state={} board={:?} board_fwd={:?} board_up={:?} vel={:?} predicted={:?} root={:?} root_up={:?} root_fwd={:?} hips={:?} reck_up={:?} heading={:?} sys_up={:?} sys_fwd={:?} flip_up={:?} anim_board={:?}",
                physical.state.state_16, board_transform[3], board_transform[2], board_transform[1],
                [velocity.x, velocity.y, velocity.z], roots.predicted_board_position,
                root[3], root[1], root[2], hips_world,
                [reckoning.up.x, reckoning.up.y, reckoning.up.z], frames.heading,
                frames.system[1], frames.system[2], frames.body_flip[1], roots.animation_to_board[3],
            );
        }
        let score = &skater.animation.motion.score_packet;
        let packet = &skater.animation.packet;
        let contact = &skater.collision_feedback;
        FrameOutput {
            teleported: skater.teleported_this_tick,
            gameplay: crate::skate3_native::world_types::GameplayOutput {
                state_flags: skater.player_state.state_flags,
                interactable_object: (skater.player_input.processed.object_2464,
                    skater.player_input.processed.flags_2480 & (1 << 20) != 0),
                animation_turn: skater.animation_input.fields.turn,
                landing_quality: skater.landing_quality,
                jump_strength: skater.animation_input.extra.jump_strength,
                hips_ground_start: skater.hips_ground_start,
                hips_ground: skater.player_input.player.hips_line_test_1488,
                deck_ground: physics.riding.probes.deck,
                foot_surfaces: [skater.player_input.processed.left_surface_2596, skater.player_input.processed.right_surface_2600],
                deck_tilt: skater.ground.steering.deck_tilt,
                wheel_hardness: skater.player_input.processed.scalar_2764,
                trick: score.trick_names.first.or_else(|| score.grab.map(|value| value.0)),
                // ScoringGrabs publishes a continuous descriptor separately
                // from ScoringTrick. S2 ProcessAnimData82dfc358 admits that
                // descriptor through bit25; otherwise ordinary source grabs
                // animate correctly but never enter the native score holder.
                trick_flags: score.flags | if score.grab.is_some() { 0x0200_0000 } else { 0 },
                trick_tweak: score.grab.or(score.handplant).map_or([0.; 2], |value| value.1),
                wipeout_gesture: skater.animation_input.extra.wipeout_gesture,
                stance: [packet.riding_fakie, packet.mirrored, packet.riding_switch, packet.regular_stance],
                push_style: skater.player_state.push_style,
                board_ground: physics.riding.ground.clone(),
                wheel_lines: physics.riding.wheel_lines,
                bone_contacts: contact.bones,
                bone_velocity_changes: std::array::from_fn(|i| skater.skeleton.record.velocity_changes[i]),
                contact_regions: contact.regions,
                contact_flags: contact.flags,
                specific_contacts: contact.specific,
                wipeout_times: contact.wipeout_times,
                entity_impacts: [contact.maximum_group_8_force, contact.maximum_skater_force, contact.maximum_group_11_force],
                feet: skater.foot_physical.output,
                heavy_landing: skater.wipeout.state.reasons[6].then(|| {
                    let kind = if physical.air.footplant_left_449 != 0 {
                        1
                    } else if physical.air.footplant_right_450 != 0 {
                        2
                    } else {
                        0
                    };
                    (kind, skater.wipeout.state.values[6])
                }),
                skitch: skater.skitch_output,
            },
            tick: physics.ticks,
            physical: *physical,
            centre_of_mass: physical.reckoning.vector_64.map(f32::from_bits),
            damped_centre_of_mass: skater.centre_of_mass_output,
            state: physical.state.state_16,
            category: physical.state.category_12,
            offboard: physical.state.category_12 == 500,
            wiping_out: skater.skeleton_collision.is_ragdoll,
            state_frames: skater.player_state.state_count,
            frames_since_reset: skater.respawn.measurements(),
            board_transform,
            board_flipped: skater.player_input.processed.flags_2468 & 0x0010_0000 != 0,
            board_velocity: [velocity.x, velocity.y, velocity.z],
            board_parts: std::array::from_fn(|i| {
                let part = physics.board.part_transforms()[i];
                let rates = &physics.board.bodies()[i].rates;
                BodyOutput {
                    transform: [
                        [
                            part.basis.columns[0][0],
                            part.basis.columns[0][1],
                            part.basis.columns[0][2],
                            0.,
                        ],
                        [
                            part.basis.columns[1][0],
                            part.basis.columns[1][1],
                            part.basis.columns[1][2],
                            0.,
                        ],
                        [
                            part.basis.columns[2][0],
                            part.basis.columns[2][1],
                            part.basis.columns[2][2],
                            0.,
                        ],
                        [
                            part.translation.x,
                            part.translation.y,
                            part.translation.z,
                            1.,
                        ],
                    ],
                    centre_of_mass: [rates.position.x, rates.position.y, rates.position.z],
                    velocity: [
                        rates.linear_velocity.x,
                        rates.linear_velocity.y,
                        rates.linear_velocity.z,
                    ],
                    omega: [
                        rates.angular_velocity.x,
                        rates.angular_velocity.y,
                        rates.angular_velocity.z,
                    ],
                }
            }),
            skeleton_parts: skater
                .animated_skeleton
                .bone_indices
                .iter()
                .enumerate()
                .map(|(part, &bone)| {
                    let transform = skeleton_transforms[part];
                    let rates = &skater.skeleton.bodies()[part].rates;
                    SkeletonPartOutput {
                        bone_name: skater.animation.evaluator.frames.bone_names[bone].clone(),
                        body: BodyOutput {
                            transform,
                            centre_of_mass: [rates.position.x, rates.position.y, rates.position.z],
                            velocity: [
                                rates.linear_velocity.x,
                                rates.linear_velocity.y,
                                rates.linear_velocity.z,
                            ],
                            omega: [
                                rates.angular_velocity.x,
                                rates.angular_velocity.y,
                                rates.angular_velocity.z,
                            ],
                        },
                        bone_transform: compose_affine(
                            &transform,
                            &skate3_core::physics::skeleton_root::inverse_rigid(
                                &skater.animated_skeleton.physics_frames[part],
                            ),
                        ),
                    }
                })
                .collect(),
            root_velocity: {
                let v = skater.skeleton.bodies()[hips_body]
                    .rates
                    .linear_velocity;
                [v.x, v.y, v.z]
            },
            root_omega: {
                let v = skater.skeleton.bodies()[hips_body]
                    .rates
                    .angular_velocity;
                [v.x, v.y, v.z]
            },
            camera: camera.frame,

            root_transform: root,
            bone_names: skater.animation.evaluator.frames.bone_names.clone(),
            world_bones: skater
                .render_pose
                .iter()
                .map(|bone| compose_affine(&root, bone))
                .collect(),
            pumping: skater.ground.pumping,
        }
    }

#[cfg(test)]
mod style_tests {
    use super::*;
    #[test]
    #[ignore = "requires embedded original Skate 3 animation banks"]
    fn embedded_styles_and_postures_change_authored_motion_without_resetting_stance() {
        use skate3_core::animation::{playback::{PlayAnimation,PlayAnimationInstance,TransitionSettings},playback_tree::{Evaluation,PoseCommand}};
        let root=assets_root().unwrap();
        let data=stock_collections().unwrap();
        let assets=player_assets(root).unwrap();
        let mut animation=crate::skate3_native::skater_animation::SkaterAnimation::from_source(&data,&assets.graphs,b"",std::sync::Arc::clone(&assets.animation)).unwrap();
        let operation=PlayAnimation {
            animation:"S_R_PUSHLSP_HSTR_N_0_CYC1".into(),switch_animation:None,mirror_animation:None,no_board_animation:None,
            playback_speed:1.,apply_posture:true,parameters:vec![],
            transition:TransitionSettings {kind:1,seconds:0.,under:0,matching:0,use_channels_from_weights:false},
        };
        for natural in [0,1] {
            animation.set_natural_stance(natural);
            let mut poses=Vec::new();
            for style in 0..4 {for posture in 0..4 {
                let expected=skate2_formats::skater_profile::PcSkate3Style {style,posture,..Default::default()};
                animation.set_style_preferences(expected).unwrap();
                let context=animation.motion.playback_context.pro_skater;
                // The game's per-tick stance refresh must not clear the choice.
                animation.set_natural_stance(natural);
                assert_eq!(animation.motion.playback_context.pro_skater,context);
                assert_eq!(animation.natural_stance(),natural as i32);
                assert_eq!(animation.motion.animation.posture.profile(),posture);
                let motion=&mut animation.motion;
                PlayAnimationInstance::default().begin(&operation,&mut motion.playback_context,&mut motion.animation).unwrap();
                motion.animation.apply_parameters().unwrap();
                let length=motion.animation.current_length().unwrap();
                motion.animation.advance(length*0.35,1.);
                let commands=motion.animation.evaluate_pose(Evaluation {cull_threshold:0.0001,update_history:false}).unwrap();
                let pose=animation.evaluator.evaluate(&commands).unwrap();
                assert!(pose.iter().flat_map(|v|v.scale.into_iter().chain(v.rotation).chain(v.translation)).all(f32::is_finite));
                if let Some(baseline)=poses.first() {assert!(&pose!=baseline,"style {style}, posture {posture}, stance {natural} matched default");}
                let names:Vec<_>=commands.iter().filter_map(|command|match command {PoseCommand::Clip {name,..}|PoseCommand::Pose {name}=>Some(name.as_str()),_=>None}).collect();
                eprintln!("style133 stance={natural} style={style} posture={posture} authored={names:?}");
                poses.push(pose);
            }}
        }
        let before=animation.motion.playback_context.pro_skater;
        assert!(animation.set_style_preferences(skate2_formats::skater_profile::PcSkate3Style {style:4,..Default::default()}).is_err());
        assert_eq!(animation.motion.playback_context.pro_skater,before);
    }
}

/// SKATE2_TRACE_SKATE3_VISUAL_ROOT: each tick's drawn root and hips against the physical deck
/// and the reckoning frame that places them.
fn trace_visual_root() -> bool {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_VISUAL_ROOT").is_some())
}

/// SKATE2_DIAGNOSTIC_TURN_BAIL_LIMIT=<seconds>: an offline stand-in for an online turn's wipeout limit.
fn diagnostic_turn_bail_limit() -> Option<f32> {
    static LIMIT: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    *LIMIT.get_or_init(|| std::env::var("SKATE2_DIAGNOSTIC_TURN_BAIL_LIMIT").ok().and_then(|v| v.parse().ok()))
}
