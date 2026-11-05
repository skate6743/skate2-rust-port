//! Complete eleven-child conditioner constructor82e316f0, called by `HiLOD` at
//!82e03f58 after `PhysOut` and before `SurfacePhysics`. This owner retains partial
//! allocations separately from the native published pointers.
#![expect(
    dead_code,
    reason = "complete retained constructor fields await original conditioner Update consumers"
)]
use super::{
    PhysOutAccessError, PhysOutComponent, PhysicalPlayerPhysOut, SkeletonRootVector, ZERO,
};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
};
use std::sync::Arc;
#[path = "physical_player_phys_out_conditioners_air_collector.rs"]
mod air_collector;
#[path = "physical_player_phys_out_conditioners_animation.rs"]
pub(crate) mod animation;
#[path = "../../conditioning/physical_player_conditioner_animation_update.rs"]
pub(crate) mod animation_update;
#[path = "../../conditioning/physical_player_conditioner_audio_update.rs"]
pub(crate) mod audio_update;
#[path = "physical_player_phys_out_conditioners_collector_base.rs"]
mod collector_base;
#[path = "physical_player_phys_out_conditioners_collectors.rs"]
mod collectors;
#[path = "../../lifecycle/physical_player_physics_damped_com.rs"]
pub(crate) mod damped_com;
#[path = "../../lifecycle/physical_player_physics_foot_side.rs"]
pub(crate) mod foot_side;
#[path = "physical_player_phys_out_conditioners_grinds.rs"]
mod grinds;
#[path = "../../conditioning/physical_player_conditioner_grinds_update.rs"]
pub(crate) mod grinds_update;
#[path = "../../lifecycle/physical_player_physics_landing_quality.rs"]
pub(crate) mod landing_quality;
#[path = "physical_player_phys_out_conditioner_physics_update.rs"]
pub(crate) mod physics_update;
#[path = "physical_player_phys_out_conditioners_scorable.rs"]
mod scorable;
#[path = "physical_player_phys_out_conditioners_score_holder.rs"]
mod score_holder;
#[path = "physical_player_phys_out_conditioners_score_module.rs"]
mod score_module;
#[path = "scoring/attributes.rs"]
pub(crate) mod scoring_attributes;
#[path = "scoring/meters.rs"]
mod scoring_meters;
#[path = "scoring/database.rs"]
pub(crate) mod scoring_database;
#[path = "scoring/catalog.rs"]
mod scoring_catalog;
#[cfg(feature = "skate3-runtime")]
#[path = "scoring/skate3_catalog.rs"]
mod skate3_catalog;
#[cfg(feature = "skate3-runtime")]
pub(super) const SCORABLE_CAPACITY: usize = skate3_catalog::CAPACITY;
#[cfg(not(feature = "skate3-runtime"))]
pub(super) const SCORABLE_CAPACITY: usize = scoring_catalog::SCORABLES.len();
#[cfg(feature = "skate3-runtime")]
pub(crate) fn skate3_trick_strings() -> Result<Vec<(u32, Vec<u16>)>, String> { skate3_catalog::localized_tricks() }
/// Cross-game IDs are resolved by the shared authored FastString name.
/// Skate 3 numeric IDs must never index Skate 2's different catalog.
#[cfg(feature = "skate3-runtime")]
pub(crate) fn scorable_from_animation_name(name: crate::play_animation::FastString36) -> i32 {
    static IDS: std::sync::OnceLock<std::collections::BTreeMap<crate::play_animation::FastString36, i32>> = std::sync::OnceLock::new();
    IDS.get_or_init(|| scoring_catalog::SCORABLES.into_iter().map(|info|
        (crate::play_animation::FastString36::encode(info.name).expect("authored scorable name"), info.id)).collect())
        .get(&name).copied().or_else(||skate3_catalog::animation_id(&name)).unwrap_or(-1)
}
/// Shared original GetScorableIDCategory lookup for non-scoring consumers.
pub(crate) fn scorable_id_category(id: i32) -> Option<i32> {
    scoring_catalog::info(id).map(|entry| entry.category)
}
/// `Sk8::Score::GetTrickType` (`0x82dedfc8`) of a valid scorable.
pub(crate) fn scorable_id_trick_type(id: i32) -> Option<i32> {
    scoring_catalog::info(id).map(|entry| entry.trick_type)
}
/// `Sk8::Score::GetScorableIDConsumed` (`0x82dee0a0`) of a valid scorable.
pub(crate) fn scorable_id_consumed(id: i32) -> Option<i32> {
    scoring_catalog::info(id).map(|entry| entry.consumed)
}
/// `gScorableInfo[id].mBaseScorable`, which `Sk8::Score::GetBaseScorableID` (`0x82dee128`) checks.
pub(crate) fn scorable_id_base(id: i32) -> Option<i32> {
    scoring_catalog::info(id).map(|entry| entry.base)
}
/// `Sk8::Score::GetScorableIDString` of a valid scorable.
pub(crate) fn scorable_id_string(id: i32) -> Option<&'static str> {
    scoring_catalog::info(id).map(|entry| entry.name)
}
/// The audio fields of a Skate 3-only scorable (past Skate 2's catalog), which exist only in the
/// linked Skate 3 data. Records without a retained lookup (replays, remote skaters) use this.
#[cfg(feature = "skate3-runtime")]
pub fn skate3_only_scorable_audio(id: i32) -> Option<(i32, i32, bool)> {
    if id < scoring_catalog::SCORABLES.len() as i32 {
        return None;
    }
    skate3_catalog::audio(id)
}
#[path = "scoring/scorable.rs"]
mod scoring_scorable;
#[path = "scoring/sequence.rs"]
mod scoring_sequence;
#[path = "scoring/holder.rs"]
mod scoring_holder;
#[path = "scoring/timed.rs"]
mod scoring_timed;
#[path = "scoring/analysis.rs"]
mod scoring_analysis;
#[path = "scoring/ground.rs"]
mod scoring_ground;
#[path = "scoring/grind.rs"]
mod scoring_grind;
#[path = "scoring/handplant.rs"]
mod scoring_handplant;
#[path = "scoring/rotation.rs"]
mod scoring_rotation;
#[path = "scoring/offboard.rs"]
mod scoring_offboard;
#[path = "scoring/air_tricks.rs"]
mod scoring_air_tricks;
#[path = "scoring/air_metrics.rs"]
mod scoring_air_metrics;
#[path = "scoring/air_gaps.rs"]
mod scoring_air_gaps;
#[path = "scoring/context.rs"]
mod scoring_context;
#[path = "scoring/air_output.rs"]
mod scoring_air_output;
#[path = "scoring/air_update.rs"]
mod scoring_air_update;
#[path = "scoring/air_lifecycle.rs"]
mod scoring_air_lifecycle;
#[path = "scoring/module_finish.rs"]
mod scoring_module_finish;
#[path = "scoring/module_selection.rs"]
mod scoring_module_selection;
#[path = "scoring/module_input.rs"]
mod scoring_module_input;
#[path = "scoring/module_output.rs"]
mod scoring_module_output;
#[path = "scoring/module_dispatch.rs"]
mod scoring_module_dispatch;
#[path = "scoring/module_update.rs"]
mod scoring_module_update;
#[path = "scoring/frame.rs"]
mod scoring_frame;
#[path = "scoring/conditioner.rs"]
pub(crate) mod scoring_update;
#[path = "scoring/hud_frame.rs"]
mod scoring_hud_frame;
#[path = "scoring/hud_stats.rs"]
mod scoring_hud_stats;
#[path = "scoring/wipeout_entities.rs"]
mod scoring_wipeout_entities;
#[path = "scoring/wipeout_bones.rs"]
mod scoring_wipeout_bones;
#[path = "scoring/wipeout_character.rs"]
mod scoring_wipeout_character;
#[path = "scoring/wipeout_tweaks.rs"]
mod scoring_wipeout_tweaks;
#[path = "scoring/wipeout_output.rs"]
mod scoring_wipeout_output;
#[path = "scoring/wipeout_lifecycle.rs"]
mod scoring_wipeout_lifecycle;
#[cfg(feature = "skate3-runtime")]
#[path = "scoring/wipeout_skate3.rs"]
mod scoring_wipeout_skate3;
#[path = "scoring/external.rs"]
pub(crate) mod scoring_external;
#[path = "scoring/entity_types.rs"]
mod scoring_entity_types;
#[path = "scoring/context_geometry.rs"]
mod scoring_context_geometry;
#[path = "scoring/context_collect.rs"]
mod scoring_context_collect;
#[path = "scoring/metrics.rs"]
mod scoring_metrics;
#[path = "scoring/thresholds.rs"]
mod scoring_thresholds;
#[path = "physical_player_phys_out_conditioners_sequence.rs"]
mod sequence;
#[path = "physical_player_phys_out_conditioners_simple.rs"]
mod simple;
#[path = "../../lifecycle/physical_player_physics_stairs.rs"]
pub(crate) mod stairs;
#[cfg(test)]
#[path = "physical_player_phys_out_conditioners_tests.rs"]
mod tests;
#[path = "physical_player_phys_out_conditioners_wipeout_score.rs"]
mod wipeout_score;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AllocationSite {
    Intents,
    Grinds,
    MissionSystem,
    Audio,
    Camera,
    Physics,
    Animation,
    Scoring,
    Stats,
    PathRecording,
    GameCam,
    AnimationGraph1,
    AnimationGraph2,
    ScoreModule,
    GlobalMetrics,
    ScoreHolder,
    Sequence,
    AirCollector,
    GrindCollector,
    GroundCollector,
    HandplantCollector,
    OffboardCollector,
    OtherCollector,
    LineTimer,
    MomentumMeter,
    WipeoutScorer,
    PedestrianScorer,
    VehicleScorer,
    DmoScorer,
}
impl AllocationSite {
    /// Source requested byte count; all calls use flags1. These are allocation
    /// categories, not an excuse to zero `PhysicsScoring` or `PhysicsDebug` memory.
    pub(crate) const fn request(self) -> (usize, &'static str) {
        use AllocationSite::{
            AirCollector, Animation, AnimationGraph1, AnimationGraph2, Audio, Camera, DmoScorer,
            GameCam, GlobalMetrics, GrindCollector, Grinds, GroundCollector, HandplantCollector,
            Intents, LineTimer, MissionSystem, MomentumMeter, OffboardCollector, OtherCollector,
            PathRecording, PedestrianScorer, Physics, ScoreHolder, ScoreModule, Scoring, Sequence,
            Stats, VehicleScorer, WipeoutScorer,
        };
        match self {
            Intents | MissionSystem | Camera | Stats => (8, "Sim"),
            Grinds => (0x1200, "Sim"),
            Audio => (0x300, "Sim"),
            Physics | Animation => (0x110, "Sim"),
            Scoring => (0x14, "Sim"),
            PathRecording => (0x40, "Sim"),
            GameCam => (0x80, "Sim"),
            AnimationGraph1 | AnimationGraph2 => (1, "PhysicsDebug"),
            ScoreModule => (0x90, "PhysicsScoring"),
            GlobalMetrics => (0x250, "PhysicsScoring"),
            ScoreHolder => (0x2460, "Sim"),
            Sequence => (0xa40, "Sim"),
            AirCollector => (0xa30, "Sim"),
            GrindCollector => (0x210, "PhysicsScoring"),
            GroundCollector => (0x5f0, "PhysicsScoring"),
            HandplantCollector => (0x1c0, "PhysicsScoring"),
            OffboardCollector => (0x270, "PhysicsScoring"),
            OtherCollector => (0x74, "PhysicsScoring"),
            LineTimer => (0x20, "PhysicsScoring"),
            MomentumMeter => (0x24, "PhysicsScoring"),
            WipeoutScorer => (0x860, "Sim"),
            PedestrianScorer | VehicleScorer => (0x84, "PhysicsScoring"),
            DmoScorer => (0x88, "PhysicsScoring"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum ConstructionError<E> {
    AlreadyStarted,
    ForeignOutput,
    Allocation(E),
    Access(PhysOutAccessError),
    Missing(AllocationSite),
    MissingOutput(PhysOutComponent),
    Cvar(InputCvarError),
    Attribute(AttributeRuntimeError),
    Air(air_collector::AirResetError),
    Dmo(wipeout_score::DmoScorerError),
    Lock,
}
#[derive(Debug)]
pub(crate) enum ResetAllError {
    ForeignOutput,
    Missing(AllocationSite),
    Cvar(InputCvarError),
    Score(ConstructionError<std::convert::Infallible>),
}
struct Retained<T> {
    value: Option<T>,
    published: bool,
}
impl<T> Retained<T> {
    fn new() -> Self {
        Self {
            value: None,
            published: false,
        }
    }
    fn published_value(&self) -> Option<&T> {
        if self.published {
            self.value.as_ref()
        } else {
            None
        }
    }
    fn published_value_mut(&mut self) -> Option<&mut T> {
        if self.published {
            self.value.as_mut()
        } else {
            None
        }
    }
}
struct Bound<T> {
    output: Arc<()>,
    payload: T,
}
struct Scoring {
    audio_score: i32,
    update_tick: i32,
    database: Option<scoring_database::TrickDatabase>,
    module: Retained<Box<score_module::ScoreModule>>,
}
#[expect(
    clippy::struct_excessive_bools,
    reason = "four original path recording flags"
)]
struct PathRecording {
    // All three source pointers are explicitly NULL at construction. No live
    // path/node/trajectory is manufactured in this constructor-only owner.
    high_detail_path: Option<Arc<()>>,
    last_node_output: Option<Arc<()>>,
    last_frames_trajectory: Option<Arc<()>>,
    last_frames_trajectory_offset: SkeletonRootVector,
    frame_count_since_last_node_output: u32,
    is_recording: bool,
    has_a_node_been_output: bool,
    valid_trajectory_last_frame: bool,
    external_reset_received_during_recording: bool,
}
pub(crate) struct Conditioners {
    output: Arc<()>,
    started: bool,
    complete: bool,
    intents: Retained<Box<Bound<()>>>,
    grinds: Retained<Box<Bound<grinds::Grinds>>>,
    mission: Retained<Box<Bound<()>>>,
    audio: Retained<Box<Bound<simple::Audio>>>,
    camera: Retained<Box<Bound<()>>>,
    physics: Retained<Box<Bound<simple::Physics>>>,
    animation: Retained<Box<Bound<animation::Animation>>>,
    animation_attributes: Option<animation::AnimationAttributes>,
    audio_attributes: Option<audio_update::AudioAttributes>,
    scoring: Retained<Box<Bound<Scoring>>>,
    stats: Retained<Box<Bound<()>>>,
    path: Retained<Box<Bound<PathRecording>>>,
    game_cam: Retained<Box<Bound<simple::GameCam>>>,
}
impl Conditioners {
    pub(crate) fn before_constructor(output: &PhysicalPlayerPhysOut) -> Self {
        Self {
            output: output.occurrence().clone(),
            started: false,
            complete: false,
            intents: Retained::new(),
            grinds: Retained::new(),
            mission: Retained::new(),
            audio: Retained::new(),
            camera: Retained::new(),
            physics: Retained::new(),
            animation: Retained::new(),
            animation_attributes: None,
            audio_attributes: None,
            scoring: Retained::new(),
            stats: Retained::new(),
            path: Retained::new(),
            game_cam: Retained::new(),
        }
    }
    pub(crate) const fn is_complete(&self) -> bool {
        self.complete
    }
    pub(crate) fn matches_output(&self, output: &PhysicalPlayerPhysOut) -> bool {
        Arc::ptr_eq(&self.output, output.occurrence())
    }
    /// PhysOutConditioners::ResetAll at TeleportPlayerNow 0x82e00988.
    /// Resolve each retained virtual child only when native reaches it.
    pub(crate) fn try_reset_all(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        cvars: &InputCvarReferences,
    ) -> Result<(), ResetAllError> {
        use AllocationSite as A;
        use ResetAllError as E;
        if !self.matches_output(output) {
            return Err(E::ForeignOutput);
        }
        self.intents
            .published_value()
            .ok_or(E::Missing(A::Intents))?;
        let grinds = &mut self
            .grinds
            .published_value_mut()
            .ok_or(E::Missing(A::Grinds))?
            .payload;
        grinds.reset_prefix();
        let minimum = cvars.try_min_approach_non_grind_frames().map_err(E::Cvar)?;
        grinds.finish_constructor(minimum);
        self.mission
            .published_value()
            .ok_or(E::Missing(A::MissionSystem))?;
        self.audio
            .published_value_mut()
            .ok_or(E::Missing(A::Audio))?
            .payload
            .reset();
        self.camera.published_value().ok_or(E::Missing(A::Camera))?;
        self.physics
            .published_value_mut()
            .ok_or(E::Missing(A::Physics))?
            .payload = simple::Physics::reset_values();
        self.animation
            .published_value_mut()
            .ok_or(E::Missing(A::Animation))?
            .payload
            .reset();
        let scoring = &mut self
            .scoring
            .published_value_mut()
            .ok_or(E::Missing(A::Scoring))?
            .payload;
        scoring
            .module
            .published_value_mut()
            .ok_or(E::Missing(A::ScoreModule))?
            .reset(output)
            .map_err(E::Score)?;
        self.stats.published_value().ok_or(E::Missing(A::Stats))?;
        self.path
            .published_value()
            .ok_or(E::Missing(A::PathRecording))?;
        self.game_cam
            .published_value_mut()
            .ok_or(E::Missing(A::GameCam))?
            .payload
            .reset();
        Ok(())
    }
    fn bound<T>(&self, payload: T) -> Box<Bound<T>> {
        Box::new(Bound {
            output: self.output.clone(),
            payload,
        })
    }
    /// Each true admission makes an actual host allocation. False preserves the
    /// native NULL branch and continues until a real dependent access fails.
    #[expect(
        clippy::too_many_lines,
        reason = "preserve source order of eleven child constructors and their publication/failure boundaries"
    )]
    pub(crate) fn try_construct<E>(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &InputCvarReferences,
        allocate: &mut impl FnMut(AllocationSite) -> Result<bool, E>,
    ) -> Result<(), ConstructionError<E>> {
        if self.started {
            return Err(ConstructionError::AlreadyStarted);
        }
        if !self.matches_output(output) {
            return Err(ConstructionError::ForeignOutput);
        }
        self.started = true;
        if allocate(AllocationSite::Intents).map_err(ConstructionError::Allocation)? {
            self.intents.value = Some(self.bound(()));
        }
        self.intents.published = true;
        if allocate(AllocationSite::Grinds).map_err(ConstructionError::Allocation)? {
            self.grinds.value = Some(self.bound(grinds::Grinds::before_cvar()));
            let current = cvars
                .try_min_approach_non_grind_frames()
                .map_err(ConstructionError::Cvar)?;
            self.grinds
                .value
                .as_mut()
                .expect("retained Grinds")
                .payload
                .finish_constructor(current);
        }
        self.grinds.published = true;
        if allocate(AllocationSite::MissionSystem).map_err(ConstructionError::Allocation)? {
            self.mission.value = Some(self.bound(()));
        }
        self.mission.published = true;
        if allocate(AllocationSite::Audio).map_err(ConstructionError::Allocation)? {
            self.audio.value = Some(self.bound(simple::Audio::before_output_read()));
            output
                .check_current_guard()
                .map_err(ConstructionError::Access)?;
            let wiping = output
                .concrete
                .state
                .as_ref()
                .ok_or(ConstructionError::MissingOutput(PhysOutComponent::State))?
                .is_wiping_out;
            let audio = &mut self.audio.value.as_mut().expect("retained Audio").payload;
            audio.is_wiping_out = wiping;
            audio.reset();
            self.audio_attributes = Some(audio_update::AudioAttributes::bind(vault));
        }
        self.audio.published = true;
        if allocate(AllocationSite::Camera).map_err(ConstructionError::Allocation)? {
            self.camera.value = Some(self.bound(()));
        }
        self.camera.published = true;
        if allocate(AllocationSite::Physics).map_err(ConstructionError::Allocation)? {
            self.physics.value = Some(self.bound(simple::Physics::reset_values()));
        }
        self.physics.published = true;
        if allocate(AllocationSite::Animation).map_err(ConstructionError::Allocation)? {
            self.animation.value = Some(self.bound(animation::Animation::before_filters()));
            self.animation_attributes = Some(animation::AnimationAttributes::bind(vault));
            let attributes = self
                .animation_attributes
                .as_ref()
                .expect("retained global animation provider");
            let animation = &mut self
                .animation
                .value
                .as_mut()
                .expect("retained Animation")
                .payload;
            animation.direction_filter = Some(simple::ScalarPid::new(
                attributes
                    .control(33)
                    .map_err(ConstructionError::Attribute)?,
            ));
            animation.lean_filter = Some(simple::ScalarPid::new(
                attributes
                    .control(32)
                    .map_err(ConstructionError::Attribute)?,
            ));
            animation.slow_lean_filter = Some(simple::ScalarPid::new(
                attributes
                    .control(31)
                    .map_err(ConstructionError::Attribute)?,
            ));
            if allocate(AllocationSite::AnimationGraph1).map_err(ConstructionError::Allocation)? {
                animation.debug_graphs[0] = Some(Arc::new(()));
            }
            if allocate(AllocationSite::AnimationGraph2).map_err(ConstructionError::Allocation)? {
                animation.debug_graphs[1] = Some(Arc::new(()));
            }
            animation.reset();
        }
        self.animation.published = true;
        if allocate(AllocationSite::Scoring).map_err(ConstructionError::Allocation)? {
            self.scoring.value = Some(self.bound(Scoring {
                database: None,
                audio_score: 0,
                update_tick: 0,
                module: Retained::new(),
            }));
            let scoring = &mut self
                .scoring
                .value
                .as_mut()
                .expect("retained Scoring")
                .payload;
            if allocate(AllocationSite::ScoreModule).map_err(ConstructionError::Allocation)? {
                scoring.module.value = Some(Box::new(score_module::ScoreModule::before_children(
                    self.output.clone(),
                    vault,
                )));
                scoring
                    .module
                    .value
                    .as_mut()
                    .expect("retained ScoreModule")
                    .construct(output, vault, allocate)?;
            }
            scoring.module.published = true;
            // Scoring's constructor performs a second Reset after ScoreModule's
            // own constructor Reset. A NULL module faults here, not at allocation.
            scoring
                .module
                .published_value_mut()
                .ok_or(ConstructionError::Missing(AllocationSite::ScoreModule))?
                .reset(output)?;
        }
        self.scoring.published = true;
        if allocate(AllocationSite::Stats).map_err(ConstructionError::Allocation)? {
            self.stats.value = Some(self.bound(()));
        }
        self.stats.published = true;
        if allocate(AllocationSite::PathRecording).map_err(ConstructionError::Allocation)? {
            self.path.value = Some(self.bound(PathRecording {
                high_detail_path: None,
                last_node_output: None,
                last_frames_trajectory: None,
                last_frames_trajectory_offset: ZERO,
                frame_count_since_last_node_output: 0,
                is_recording: false,
                has_a_node_been_output: false,
                valid_trajectory_last_frame: false,
                external_reset_received_during_recording: false,
            }));
        }
        self.path.published = true;
        if allocate(AllocationSite::GameCam).map_err(ConstructionError::Allocation)? {
            self.game_cam.value = Some(self.bound(simple::GameCam::new()));
        }
        self.game_cam.published = true;
        self.complete = true;
        Ok(())
    }
}
