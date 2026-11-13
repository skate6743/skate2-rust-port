//! One retained authored SkaterPool/static-ground collision component frame.
//! Real board and skeleton membership supplies the full publication range.
//! Flags3 adjacent board/skeleton pairs and DMO pairs share the native island
//! partition. Collision workers execute in a documented serial host order.

use super::{
    HostSimulationDriveNormalizationNumeric, SIMULATION_ISLAND_CAPACITY,
    SimulationAssemblyConstraintFrameCounts, SimulationAssemblyConstraintFrameError,
    SimulationAssemblyConstraintSession,
    contact_feedback::{
        SimulationContactFeedbackError, SimulationContactFeedbackInput,
        SimulationContactFeedbackProgress,
    },
    prediction::runtime::PredictionRuntimeError,
    try_prepare_active_drive_islands_host, try_prepare_active_joint_islands_host,
};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterPoolError,
    serialised_assembly_bbox::HostSerialisedAssemblyBBoxNumeric,
    simulation_contact_jacobian::{
        ground_preparation::{
            GroundAssemblyPublication, SimulationGroundRunError, SimulationGroundRunSettings,
            cpu_job::{
                SimulationAuthoredGroundJobInput, SimulationStaticGroundJobError,
                SimulationStaticGroundJobReport, try_run_authored_ground_cpu_job,
                try_run_serialized_ground_cpu_job,
            },
            dmo_dmo::{
                AssemblyPairSide, SimulationAssemblyPairJobInput, SimulationDmoDmoJobInput,
                try_run_dmo_dmo_cpu_jobs, try_run_dmo_vehicle_cpu_jobs,
                try_run_skater_skater_cpu_jobs,
                PedestrianPairPipeline, try_run_pedestrian_pair_cpu_jobs,
            },
            skater_dmo::{
                MeshCollisionTarget, SimulationSkaterDmoJobError, SimulationSkaterDmoJobInput,
                try_run_skater_dmo_cpu_jobs, try_run_skater_mesh_cpu_jobs,
            },
        },
        network_preparation::SimulationGroundContactPreparationError,
    },
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoPoolUpdateError,
    simulation_island_map::assembly_pairs::{
        SimulationAssemblyPairFrame, SimulationCollisionAssembly,
    },
    skateboard_hook::SkateboardHookDefinition,
    static_ground_pool::StaticGroundPool,
};
use skate2_formats::collision::{AssemblyPartMaterial, BitTable};
use std::{convert::Infallible, fmt};

/// Explicit scope: component probes do not imply a constructed gameplay world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkateboardGroundFrameSchedule {
    IsolatedCollisionComponent,
    /// Only the no-visual, flags3 serial subset. First `SimUpdate1` harvest only;
    /// StepSelector/locomotion and the `SimUpdate2` harvest remain separate stages.
    SinglePlayerPhysicsFlags3,
}

/// The pairs `Step_Collision1`'s skater-skater pipeline collides this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkaterSkaterPairs {
    /// `kRunSkaterSkaterCollisionPipeline` is off, or the frame is an isolated probe.
    Off,
    /// Every pair in the skater pool.
    All,
    /// Each skater's board and Skeleton, the pool's adjacent members, with each other only:
    /// Party Play's skaters pass through each other as an online game's remote skaters do, a
    /// port change.
    Own,
}

/// Explicit settings for a serial source CPU job, not a cvar cache.
#[derive(Clone, Copy)]
pub(crate) struct SkateboardGroundFrameInput<'a> {
    pub(crate) schedule: SkateboardGroundFrameSchedule,
    pub(crate) ground_pool: &'a StaticGroundPool,
    pub(crate) ground_group: u32,
    pub(crate) pipeline_culling: Option<&'a BitTable>,
    pub(crate) allow_multiple_writes: bool,
    /// Current `Simulation.m_SpyFlag` snapshot, not a host tuning default.
    pub(crate) spy_flags: u32,
}

#[derive(Debug)]
pub(crate) enum SkateboardGroundFrameError<E = std::convert::Infallible> {
    Frame(SimulationAssemblyConstraintFrameError),
    UnsupportedPredictorSchedule { island_flags: u32 },
    MissingPredictor,
    MissingWorldQueries,
    OddSkaterAssemblyCount { count: usize },
    Prediction(PredictionRuntimeError<Infallible>),
    StepSelector(super::step_selector::StepSelectorError),
    Locomotion(Box<crate::actor_construction::locomotion_mid_session::ActorMidLocomotionError>),
    Pool(SimulationSkaterPoolError),
    Ground(SimulationStaticGroundJobError),
    Contact(SimulationGroundContactPreparationError),
    Spy(crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyError),
    Feedback(SimulationContactFeedbackError<E>),
    ContactList(super::physical_contact_list::SimulationPhysicalContactListError),
    DmoUpdate(SimulationDmoPoolUpdateError<Infallible>),
    DmoGround(SimulationStaticGroundJobError),
    DmoDmo(SimulationGroundRunError),
    SkaterDmo(SimulationSkaterDmoJobError),
    SkaterSkater(SimulationGroundRunError),
    VehicleUpdate(SimulationDmoPoolUpdateError<Infallible>),
    PedestrianUpdate(crate::simulation_dmo_assembly_part_session::dynamic_pool::character::CharacterPoolError),
    DmoVehicle(SimulationGroundRunError),
    PedestrianGround(SimulationStaticGroundJobError),
    PedestrianPair(SimulationGroundRunError),
    SkaterVehicle(SimulationSkaterDmoJobError),
    ThawBody(crate::simulation_rigid_body_pool::SimulationRigidBodyPoolError),
}

impl<E: fmt::Debug> fmt::Display for SkateboardGroundFrameError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frame(error) => write!(f, "board ground frame: {error}"),
            Self::UnsupportedPredictorSchedule { island_flags } => write!(
                f,
                "first predictor harvest requires flags3, got {island_flags}"
            ),
            Self::MissingPredictor => {
                f.write_str("first predictor harvest requires the constructed World predictor")
            }
            Self::MissingWorldQueries => {
                f.write_str("first predictor harvest requires the constructed World query owner")
            }
            Self::OddSkaterAssemblyCount { count } => write!(
                f,
                "flags3 Collision1 requires an even SkaterPool member count, got {count}"
            ),
            Self::Prediction(error) => write!(f, "first predictor harvest: {error}"),
            Self::StepSelector(error) => write!(f, "first step-selector harvest: {error}"),
            Self::Locomotion(error) => write!(f, "mid-physics locomotion: {error:?}"),
            Self::Pool(error) => write!(f, "board ground pool: {error}"),
            Self::Ground(error) => write!(f, "board ground job: {error}"),
            Self::Contact(error) => write!(f, "board ground contacts: {error}"),
            Self::Spy(error) => write!(f, "board ground contact spies: {error}"),
            Self::Feedback(error) => write!(f, "board ground {error}"),
            Self::ContactList(error) => write!(f, "board ground contact list: {error}"),
            Self::DmoUpdate(error) => write!(f, "DMO pool EndUpdate: {error}"),
            Self::DmoGround(error) => write!(f, "DMO ground job: {error}"),
            Self::DmoDmo(error) => write!(f, "DMO assembly collision job: {error}"),
            Self::SkaterDmo(error) => write!(f, "{error}"),
            Self::SkaterSkater(error) => write!(f, "skater-skater collision: {error}"),
            Self::VehicleUpdate(error) => write!(f, "vehicle pool EndUpdate: {error}"),
            Self::PedestrianUpdate(error) => write!(f, "pedestrian pool EndUpdate: {error:?}"),
            Self::PedestrianGround(error) => write!(f, "pedestrian ground collision: {error}"),
            Self::PedestrianPair(error) => write!(f, "pedestrian assembly collision: {error}"),
            Self::DmoVehicle(error) => write!(f, "DMO vehicle collision: {error}"),
            Self::SkaterVehicle(error) => write!(f, "skater vehicle collision: {error}"),
            Self::ThawBody(error) => write!(f, "DMO thaw body: {error}"),
        }
    }
}

impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    /// Existing isolated component probes have no selected Actor. Production
    /// must use the callback-bearing entry below; no live no-op is supplied.
    #[cfg(test)]
    pub(crate) fn try_step_authored_ground<I: SimulationContactFeedbackInput>(
        &mut self,
        input: SkateboardGroundFrameInput<'_>,
        timestep: f32,
        max_iterations: u32,
        feedback_input: &mut I,
    ) -> Result<
        (
            SimulationAssemblyConstraintFrameCounts,
            SimulationStaticGroundJobReport,
        ),
        SkateboardGroundFrameError<I::Error>,
    > {
        self.try_step_authored_ground_with_locomotion(
            input,
            timestep,
            max_iterations,
            feedback_input,
            SkaterSkaterPairs::Off, // Isolated collision probes do not run the skater-pair pipeline.
            |_| Ok(()),
        )
    }

    /// Refresh is inside the owner, immediately before the exclusive collision
    /// phase. No callback can relocate a body after bounds publication. This
    /// nonparallel pool refresh preserves all-updates/all-bounds/publication.
    /// The selected frame supplies the live skater-pair pipeline switch.
    #[expect(
        clippy::too_many_lines,
        reason = "one existing serial frame with an explicit first-harvest seam and retained failure prefixes"
    )]
    pub(crate) fn try_step_authored_ground_with_locomotion<I: SimulationContactFeedbackInput>(
        &mut self,
        input: SkateboardGroundFrameInput<'_>,
        timestep: f32,
        max_iterations: u32,
        feedback_input: &mut I,
        skater_skater: SkaterSkaterPairs,
        locomotion: impl FnOnce(
            &mut Self,
        ) -> Result<
            (),
            crate::actor_construction::locomotion_mid_session::ActorMidLocomotionError,
        >,
    ) -> Result<
        (
            SimulationAssemblyConstraintFrameCounts,
            SimulationStaticGroundJobReport,
        ),
        SkateboardGroundFrameError<I::Error>,
    > {
        use SimulationAssemblyConstraintFrameError as Frame;
        use SkateboardGroundFrameError as Error;
        let profile_prepare = crate::diagnostics::frame_profile::span(14);
        // Checked host prerequisite, before any frame-side effects.
        self.skater_pool().map_err(Error::Pool)?;
        let harvest = input.schedule == SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3;
        if harvest && self.island_flags != 3 {
            return Err(Error::UnsupportedPredictorSchedule {
                island_flags: self.island_flags,
            });
        }
        // Host stale-output invalidation; a failed later stage must not expose
        // the preceding frame as this frame's successful solved-spy result.
        self.contact_spies = None;
        let dmo_frame = self.dmo_frame.take();
        self.physical_contacts.begin_frame();
        self.contact_feedback_progress = SimulationContactFeedbackProgress::NotStarted;
        self.backend
            .rigid_bodies
            .begin_dynamic_update_frame(timestep);
        self.contacts
            .try_start_island_frame_labels(&self.backend.rigid_bodies)
            .map_err(|e| Error::Frame(Frame::Labels(e)))?;
        // SimUpdate1's EndDMOUpdate 0x8289f6ac -> Island::Step_UpdatePrePhysics2
        // 0x827b28f0: the DMO pool's serialized EndUpdate, run once the body
        // pool has this frame's identity (no body moves before the solver).
        // After a full mesh collection it publishes the retained records.
        if dmo_frame.is_some()
            && let Some(dmos) = self.dmos.as_mut()
        {
            dmos.try_end_update(&self.backend, &mut HostSerialisedAssemblyBBoxNumeric)
                .map_err(Error::DmoUpdate)?;
        }
        // Native PrePhysics2 ends the pedestrian pool before the vehicle pool
        // (827b298c/29a0). The host refresh shares the established immutable
        // publication preparation seam; collision-job admission follows below.
        if dmo_frame.is_some() && let Some(pedestrians) = self.pedestrians.as_mut() {
            pedestrians.try_refresh(&self.backend).map_err(Error::PedestrianUpdate)?;
        }
        // Native PrePhysics2 launches DMO jobs before vehicle EndUpdate. This
        // serial host executor prepares both immutable publications before it
        // takes the exclusive body-pool phase. No body moves between these
        // publications and their jobs; contact/job admission order stays below.
        if dmo_frame.is_some() {
            let queries = self
                .world_line_queries
                .as_mut()
                .ok_or(Error::MissingWorldQueries)?;
            queries
                .vehicle
                .try_end_update(&self.backend, &mut HostSerialisedAssemblyBBoxNumeric)
                .map_err(Error::VehicleUpdate)?;
        }
        if harvest {
            self.try_refresh_skaters().map_err(Error::Pool)?;
            // SimUpdate1 8289f8f0, after World.Update1 and before World.Update2.
            // Flags3 Collision2 is a no-op. The isolated synchronous ground job
            // below completes in the later Collision3/solver portion. Bounds
            // remain protected: harvest receives no bodies or whole-world callback.
            let predictor = self
                .predictor
                .value
                .as_ref()
                .ok_or(Error::MissingPredictor)?;
            let queries = self
                .world_line_queries
                .as_mut()
                .ok_or(Error::MissingWorldQueries)?;
            predictor
                .try_update_queries::<Infallible>(queries, &self.debug_overlay)
                .map_err(Error::Prediction)?;
            // SimUpdate1 8289f908, between Predictor and UpdateLocomotion.
            self.try_update_step_selector()
                .map_err(Error::StepSelector)?;
            // SimUpdate1 8289f934: result0 has completed; now produce result1.
            locomotion(self).map_err(|error| Error::Locomotion(Box::new(error)))?;
        }
        // Native Solver2: hook exclusion and drive normalization follow harvest.
        self.contacts
            .try_exclude_hook_assemblies(&self.backend.rigid_bodies, &self.hooks)
            .map_err(|e| Error::Frame(Frame::Hook(e)))?;
        self.backend
            .drives
            .try_normalize_active_drive_frames(&mut HostSimulationDriveNormalizationNumeric)
            .map_err(|e| Error::Frame(Frame::Normalize(e)))?;
        if !harvest {
            // Isolated component callers preserve their original operation order.
            self.try_refresh_skaters().map_err(Error::Pool)?;
        }
        let dmo_image = self
            .backend
            .source_routes
            .as_ref()
            .map(|routes| routes.image());
        let phase = self
            .backend
            .rigid_bodies
            .try_prepare_dynamic_update_phase()
            .map_err(|e| Error::Frame(Frame::Phase(e)))?;
        // Step_UpdatePrePhysics2 0x827b2904..2954: the DMO ground pipeline
        // (`applyObjectFixes`, multiple contact writes) over the serialized
        // publications in pool order. Its jobs start at EndDMOUpdate, before
        // Collision1's skater jobs reserve from the same Contact array.
        let mut dmo_records = Vec::new();
        drop(profile_prepare);
        if let (Some(settings), Some(dmos), Some(image)) =
            (dmo_frame, self.dmos.as_ref(), dmo_image)
            && settings.run_ground
        {
            let _profile = crate::diagnostics::frame_profile::span(15);
            dmo_records = try_run_serialized_ground_cpu_job(
                dmos.publications(),
                image,
                &SimulationAuthoredGroundJobInput {
                    ground_pool: input.ground_pool,
                    phase: &phase,
                    maximum_contacts: self.contacts.maximum(),
                    allow_multiple_writes: true,
                    run_settings: SimulationGroundRunSettings {
                        ground: Some(self.contacts.static_body_observation()),
                        ground_group: input.ground_group,
                        pipeline_culling: Some(&self.island_culling),
                        is_object: true,
                        draw_contacts: settings.draw_ground_contacts,
                    },
                },
            )
            .map_err(Error::DmoGround)?
            .into_records();
        }
        let mut pair_frame = SimulationAssemblyPairFrame::default();
        let mut dmo_body_records = Vec::new();
        // EndDMOUpdate starts this pipeline directly after DMO-ground. Native
        // workers race to reserve; the CPU port keeps this serial job order.
        if let (Some(settings), Some(dmos), Some(image)) =
            (dmo_frame, self.dmos.as_ref(), dmo_image)
            && settings.run_dmo_dmo
        {
            let _profile = crate::diagnostics::frame_profile::span(16);
            let targets: Vec<_> = dmos.publications().collect();
            let output = try_run_dmo_dmo_cpu_jobs(
                &targets,
                &SimulationDmoDmoJobInput {
                    phase: &phase,
                    image,
                    pipeline_culling: Some(&self.island_culling),
                    maximum_contacts: self
                        .contacts
                        .maximum()
                        .saturating_sub(u32::try_from(dmo_records.len()).unwrap_or(u32::MAX)),
                    remaining_pairs: pair_frame.remaining_pairs(),
                    remaining_thaws: pair_frame.remaining_thaws(),
                },
            )
            .map_err(Error::DmoDmo)?;
            dmo_body_records = output.records;
            for [a, b] in output.pairs {
                pair_frame.append_pair(
                    SimulationCollisionAssembly::dmo(targets[a]),
                    SimulationCollisionAssembly::dmo(targets[b]),
                );
            }
            for index in output.thaw {
                pair_frame.append_thaw(SimulationCollisionAssembly::dmo(targets[index]));
            }
        }
        // Step_UpdatePrePhysics2: pedestrian ground, DMO-pedestrian,
        // DMO-vehicle, pedestrian-vehicle, pedestrian-pedestrian, in that order.
        // All refreshes completed before borrowing the exclusive body phase.
        let pedestrian_targets: Vec<_> = self.pedestrians.as_ref().into_iter()
            .flat_map(|pool| pool.publications()).collect();
        let pedestrian_publications: Vec<_> = pedestrian_targets.iter()
            .map(|p| GroundAssemblyPublication::Authored((*p).into())).collect();
        if let Some(settings) = dmo_frame
            && settings.run_pedestrian_ground
        {
            // InitWorld: five assemblies per ground job, multiple writes,
            // default applyObjectFixes=false. Keep source ordinals in pool space.
            for (job_index, job_targets) in pedestrian_targets.chunks(5).enumerate() {
                let mut job = try_run_authored_ground_cpu_job(job_targets.iter().copied(),
                    &SimulationAuthoredGroundJobInput {
                        ground_pool: input.ground_pool,
                        phase: &phase,
                        maximum_contacts: self.contacts.maximum().saturating_sub(
                            u32::try_from(dmo_records.len() + dmo_body_records.len()).unwrap_or(u32::MAX)),
                        allow_multiple_writes: true,
                        run_settings: SimulationGroundRunSettings {
                            ground: Some(self.contacts.static_body_observation()),
                            ground_group: input.ground_group,
                            pipeline_culling: Some(&self.island_culling),
                            is_object: false,
                            draw_contacts: settings.draw_pedestrian_ground_contacts,
                        },
                    }).map_err(Error::PedestrianGround)?;
                job.offset_assembly_ordinals(job_index * 5);
                let contacts = job.into_records();
                // This vector retains all post-DMO-ground PrePhysics2 output,
                // including pedestrian ground, in worker submission order.
                dmo_body_records.extend(contacts);
            }
        }
        if let (Some(settings), Some(dmos), Some(image)) =
            (dmo_frame, self.dmos.as_ref(), dmo_image)
            && settings.run_dmo_pedestrian
        {
            let targets: Vec<_> = dmos.publications().collect();
            let publications: Vec<_> = targets.iter()
                .map(|assembly| GroundAssemblyPublication::Serialized { assembly, image }).collect();
            let output = try_run_pedestrian_pair_cpu_jobs(&publications, &pedestrian_publications,
                &SimulationAssemblyPairJobInput {
                    phase: &phase,
                    pipeline_culling: Some(&self.island_culling),
                    maximum_contacts: self.contacts.maximum().saturating_sub(
                        u32::try_from(dmo_records.len() + dmo_body_records.len()).unwrap_or(u32::MAX)),
                    remaining_pairs: pair_frame.remaining_pairs(),
                    remaining_thaws: pair_frame.remaining_thaws(),
                }, PedestrianPairPipeline::DmoPedestrian).map_err(Error::PedestrianPair)?;
            dmo_body_records.extend(output.records);
            for [a, b] in output.pairs {
                pair_frame.append_pair(SimulationCollisionAssembly::dmo(targets[a]),
                    SimulationCollisionAssembly::skater(pedestrian_targets[b].into()));
            }
            for (side, ordinal) in output.thaw {
                pair_frame.append_thaw(match side {
                    AssemblyPairSide::A => SimulationCollisionAssembly::dmo(targets[ordinal]),
                    AssemblyPairSide::B => SimulationCollisionAssembly::skater(pedestrian_targets[ordinal].into()),
                });
            }
        }
        let vehicle_targets: Vec<_> = self
            .world_line_queries
            .as_ref()
            .into_iter()
            .flat_map(|queries| queries.vehicle.character_publications())
            .collect();
        if let (Some(settings), Some(dmos), Some(image)) =
            (dmo_frame, self.dmos.as_ref(), dmo_image)
            && settings.run_dmo_vehicle
            && !vehicle_targets.is_empty()
        {
            let _profile = crate::diagnostics::frame_profile::span(17);
            let targets: Vec<_> = dmos.publications().collect();
            let output = try_run_dmo_vehicle_cpu_jobs(
                &targets,
                &vehicle_targets,
                image,
                &SimulationAssemblyPairJobInput {
                    phase: &phase,
                    pipeline_culling: Some(&self.island_culling),
                    maximum_contacts: self.contacts.maximum().saturating_sub(
                        u32::try_from(dmo_records.len() + dmo_body_records.len())
                            .unwrap_or(u32::MAX),
                    ),
                    remaining_pairs: pair_frame.remaining_pairs(),
                    remaining_thaws: pair_frame.remaining_thaws(),
                },
            )
            .map_err(Error::DmoVehicle)?;
            dmo_body_records.extend(output.records);
            for [a, b] in output.pairs {
                pair_frame.append_pair(
                    SimulationCollisionAssembly::dmo(targets[a]),
                    SimulationCollisionAssembly::skater(vehicle_targets[b].into()),
                );
            }
            for (side, ordinal) in output.thaw {
                pair_frame.append_thaw(match side {
                    AssemblyPairSide::A => SimulationCollisionAssembly::dmo(targets[ordinal]),
                    AssemblyPairSide::B => {
                        SimulationCollisionAssembly::skater(vehicle_targets[ordinal].into())
                    }
                });
            }
        }
        if let Some(settings) = dmo_frame {
            let vehicle_publications: Vec<_> = vehicle_targets.iter()
                .map(|p| GroundAssemblyPublication::Authored((*p).into())).collect();
            for (enabled, pipeline, other_publications, other_targets) in [
                (settings.run_pedestrian_vehicle, PedestrianPairPipeline::PedestrianVehicle,
                    &vehicle_publications, &vehicle_targets),
                (settings.run_pedestrian_pedestrian, PedestrianPairPipeline::PedestrianPedestrian,
                    &pedestrian_publications, &pedestrian_targets),
            ] {
                if !enabled { continue; }
                let output = try_run_pedestrian_pair_cpu_jobs(&pedestrian_publications, other_publications,
                    &SimulationAssemblyPairJobInput {
                        phase: &phase,
                        pipeline_culling: Some(&self.island_culling),
                        maximum_contacts: self.contacts.maximum().saturating_sub(
                            u32::try_from(dmo_records.len() + dmo_body_records.len()).unwrap_or(u32::MAX)),
                        remaining_pairs: pair_frame.remaining_pairs(),
                        remaining_thaws: pair_frame.remaining_thaws(),
                    }, pipeline).map_err(Error::PedestrianPair)?;
                dmo_body_records.extend(output.records);
                for [a, b] in output.pairs {
                    pair_frame.append_pair(SimulationCollisionAssembly::skater(pedestrian_targets[a].into()),
                        SimulationCollisionAssembly::skater(other_targets[b].into()));
                }
                for (side, ordinal) in output.thaw {
                    let target = match side {
                        AssemblyPairSide::A => pedestrian_targets[ordinal],
                        AssemblyPairSide::B => other_targets[ordinal],
                    };
                    pair_frame.append_thaw(SimulationCollisionAssembly::skater(target.into()));
                }
            }
        }
        let reserved =
            u32::try_from(dmo_records.len() + dmo_body_records.len()).unwrap_or(u32::MAX);
        let skater_targets: Vec<
            crate::authored_collision_publication::AuthoredCollisionPublication<'_>,
        > = self
            .skaters
            .as_ref()
            .expect("pool checked before frame")
            .publications()
            // A Skate 3 physics player's board and Skeleton are integrated by its own solver.
            .filter(|publication| !publication.rigid_bodies().iter().any(|body| phase.is_externally_driven(*body)))
            .map(Into::into)
            .collect();
        if input.schedule == SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3 {
            // Step_Collision1 827b386c..3998: the dense pool has adjacent
            // board/Skeleton members. This is unconditional pair admission,
            // independent of AABB, culling, body state and ground-run settings.
            if skater_targets.len() % 2 != 0 {
                return Err(Error::OddSkaterAssemblyCount {
                    count: skater_targets.len(),
                });
            }
            for pair in skater_targets.chunks_exact(2) {
                pair_frame.append_pair(
                    SimulationCollisionAssembly::skater(pair[0]),
                    SimulationCollisionAssembly::skater(pair[1]),
                );
            }
        }
        let profile_ground = crate::diagnostics::frame_profile::span(18);
        let job = try_run_authored_ground_cpu_job(
            skater_targets.iter().copied(),
            &SimulationAuthoredGroundJobInput {
                ground_pool: input.ground_pool,
                phase: &phase,
                maximum_contacts: self.contacts.maximum().saturating_sub(reserved),
                allow_multiple_writes: input.allow_multiple_writes,
                run_settings: SimulationGroundRunSettings {
                    ground: Some(self.contacts.static_body_observation()),
                    ground_group: input.ground_group,
                    // Island ctor InitCullingTable 0x827bad10 gives every flags3
                    // pipeline the Island table; isolated probes keep their input.
                    pipeline_culling: match input.schedule {
                        SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3 => {
                            Some(input.pipeline_culling.unwrap_or(&self.island_culling))
                        }
                        SkateboardGroundFrameSchedule::IsolatedCollisionComponent => {
                            input.pipeline_culling
                        }
                    },
                    is_object: false,
                    draw_contacts: false,
                },
            },
        )
        .map_err(Error::Ground)?;
        drop(profile_ground);
        let (mut records, report) = job.into_parts();
        if std::env::var_os("SKATE2_TRACE_GROUND_RECORDS").is_some() {
            eprintln!("ground-records count={}", records.len());
            for (index, record) in records.iter().take(64).enumerate() {
                let (assembly, volume, point, body_a, body_b) = record.diagnostic_binding();
                eprintln!(
                    "ground-record[{index}] assembly={assembly} volume={volume} point={point} body-a={body_a:?} body-b={body_b:?}"
                );
            }
        }
        // Step_Collision1: the skater DMO pipeline after the skater ground
        // pipeline, one job per skater assembly over the DMO pool. Each job's
        // `GlobalBuffer::Store` 0x827cb5c8 keeps only what still fits.
        let mut body_records = Vec::new();
        if let (Some(settings), Some(dmos)) = (dmo_frame, self.dmos.as_ref())
            && settings.run_skater_dmo
            && dmos.len() != 0
        {
            let _profile = crate::diagnostics::frame_profile::span(19);
            let targets: Vec<_> = dmos.publications().collect();
            let output = try_run_skater_dmo_cpu_jobs(
                skater_targets.iter().copied(),
                &targets,
                &SimulationSkaterDmoJobInput {
                    phase: &phase,
                    pipeline_culling: match input.schedule {
                        SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3 => {
                            Some(input.pipeline_culling.unwrap_or(&self.island_culling))
                        }
                        SkateboardGroundFrameSchedule::IsolatedCollisionComponent => {
                            input.pipeline_culling
                        }
                    },
                    remaining_pairs: pair_frame.remaining_pairs(),
                    remaining_thaws: pair_frame.remaining_thaws(),
                },
            )
            .map_err(Error::SkaterDmo)?;
            body_records = output.records;
            let remaining = (self.contacts.maximum() as usize)
                .saturating_sub(dmo_records.len() + dmo_body_records.len() + records.len());
            body_records.truncate(remaining);
            for [a, b] in output.pairs {
                pair_frame.append_pair(
                    SimulationCollisionAssembly::skater(skater_targets[a]),
                    SimulationCollisionAssembly::dmo(targets[b]),
                );
            }
            for ordinal in output.thaw {
                pair_frame.append_thaw(SimulationCollisionAssembly::dmo(targets[ordinal]));
            }
        }
        // Step_Collision1 827b3a20..3a3c: flags3 runs the same-pool skater
        // worker after skater-DMO and before skater-pedestrian. Include every
        // published skater (local, ambient AI, challenge AI) and self pairs;
        // the original assembly/volume culling tables decide eligibility.
        // This switch is independent of LivingWorld/DMO initialization.
        if harvest && skater_skater != SkaterSkaterPairs::Off {
            let publications: Vec<_> = skater_targets
                .iter()
                .copied()
                .map(GroundAssemblyPublication::Authored)
                .collect();
            // Own pairs run each skater's adjacent board and Skeleton as a pool of their own.
            let members = if skater_skater == SkaterSkaterPairs::Own { 2 } else { publications.len().max(1) };
            for (index, pool) in publications.chunks(members).enumerate() {
                let start = index * members;
                let output = try_run_skater_skater_cpu_jobs(
                    pool,
                    &SimulationAssemblyPairJobInput {
                        phase: &phase,
                        pipeline_culling: Some(input.pipeline_culling.unwrap_or(&self.island_culling)),
                        maximum_contacts: self.contacts.maximum().saturating_sub(
                            u32::try_from(
                                dmo_records.len() + dmo_body_records.len() + records.len() + body_records.len(),
                            ).unwrap_or(u32::MAX),
                        ),
                        remaining_pairs: pair_frame.remaining_pairs(),
                        remaining_thaws: pair_frame.remaining_thaws(),
                    },
                ).map_err(Error::SkaterSkater)?;
                body_records.extend(output.records);
                for [a, b] in output.pairs {
                    pair_frame.append_pair(
                        SimulationCollisionAssembly::skater(skater_targets[start + a]),
                        SimulationCollisionAssembly::skater(skater_targets[start + b]),
                    );
                }
                for (_, ordinal) in output.thaw {
                    pair_frame.append_thaw(SimulationCollisionAssembly::skater(skater_targets[start + ordinal]));
                }
            }
        }
        // Collision1: skater-pedestrian is the ordinary pair worker, two
        // skater assemblies per job. It precedes the custom skater-vehicle job.
        if let Some(settings) = dmo_frame
            && settings.run_skater_pedestrian
        {
            let publications: Vec<_> = skater_targets.iter().copied()
                .map(GroundAssemblyPublication::Authored).collect();
            let output = try_run_pedestrian_pair_cpu_jobs(&publications, &pedestrian_publications,
                &SimulationAssemblyPairJobInput {
                    phase: &phase,
                    pipeline_culling: match input.schedule {
                        SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3 =>
                            Some(input.pipeline_culling.unwrap_or(&self.island_culling)),
                        SkateboardGroundFrameSchedule::IsolatedCollisionComponent => input.pipeline_culling,
                    },
                    maximum_contacts: self.contacts.maximum().saturating_sub(
                        u32::try_from(dmo_records.len() + dmo_body_records.len() + records.len() + body_records.len())
                            .unwrap_or(u32::MAX)),
                    remaining_pairs: pair_frame.remaining_pairs(),
                    remaining_thaws: pair_frame.remaining_thaws(),
                }, PedestrianPairPipeline::SkaterPedestrian).map_err(Error::PedestrianPair)?;
            body_records.extend(output.records);
            for [a, b] in output.pairs {
                pair_frame.append_pair(SimulationCollisionAssembly::skater(skater_targets[a]),
                    SimulationCollisionAssembly::skater(pedestrian_targets[b].into()));
            }
            for (side, ordinal) in output.thaw {
                pair_frame.append_thaw(match side {
                    AssemblyPairSide::A => SimulationCollisionAssembly::skater(skater_targets[ordinal]),
                    AssemblyPairSide::B => SimulationCollisionAssembly::skater(pedestrian_targets[ordinal].into()),
                });
            }
        }
        // Step_Collision1's final pipeline uses the very same custom mesh job
        // as skater-DMO; the vehicle publication retains its real mesh arena.
        if let Some(settings) = dmo_frame
            && settings.run_skater_vehicle
            && !vehicle_targets.is_empty()
        {
            let _profile = crate::diagnostics::frame_profile::span(20);
            let targets: Vec<_> = vehicle_targets
                .iter()
                .map(|publication| MeshCollisionTarget::Character(*publication))
                .collect();
            let mut output = try_run_skater_mesh_cpu_jobs(
                skater_targets.iter().copied(),
                &targets,
                &SimulationSkaterDmoJobInput {
                    phase: &phase,
                    pipeline_culling: match input.schedule {
                        SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3 => {
                            Some(input.pipeline_culling.unwrap_or(&self.island_culling))
                        }
                        SkateboardGroundFrameSchedule::IsolatedCollisionComponent => {
                            input.pipeline_culling
                        }
                    },
                    remaining_pairs: pair_frame.remaining_pairs(),
                    remaining_thaws: pair_frame.remaining_thaws(),
                },
            )
            .map_err(Error::SkaterVehicle)?;
            output
                .records
                .truncate((self.contacts.maximum() as usize).saturating_sub(
                    dmo_records.len() + dmo_body_records.len() + records.len() + body_records.len(),
                ));
            body_records.extend(output.records);
            for [a, b] in output.pairs {
                pair_frame.append_pair(
                    SimulationCollisionAssembly::skater(skater_targets[a]),
                    SimulationCollisionAssembly::skater(vehicle_targets[b].into()),
                );
            }
            for ordinal in output.thaw {
                pair_frame.append_thaw(SimulationCollisionAssembly::skater(
                    vehicle_targets[ordinal].into(),
                ));
            }
        }
        drop(phase);
        let profile_partition = crate::diagnostics::frame_profile::span(21);
        if !pair_frame.thaw().is_empty()
            && feedback_input
                .current_enable_freezing()
                .map_err(|e| Error::Feedback(SimulationContactFeedbackError::Input(e)))?
                != 0
        {
            // `Assembly::Activate` 0x82ae09b8 in thaw-list and Part order: a
            // Frozen body goes Active, then every Part body's cooldown clears.
            // The Contacts keep the rows the jobs built before the thaw.
            for assembly in pair_frame.thaw() {
                for &body in assembly.bodies() {
                    // Duplicates are native. Read current state on each visit,
                    // rather than reusing a frozen flag captured before the first.
                    if self
                        .backend
                        .rigid_bodies
                        .try_raw_state(body)
                        .map_err(Error::ThawBody)?
                        & 7
                        == 2
                    {
                        self.backend
                            .rigid_bodies
                            .try_activate_frozen_body(body)
                            .map_err(Error::ThawBody)?;
                    }
                    self.backend
                        .rigid_bodies
                        .try_clear_body_cooldown(body)
                        .map_err(Error::ThawBody)?;
                }
            }
        }
        // Step_Solver2 827b3698 follows thaw. Frozen body labels can be -1
        // from their birth frame, so snapshotting before this loses contacts.
        if input.schedule == SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3 {
            self.contacts
                .try_partition_assembly_pairs(&self.backend.rigid_bodies, &pair_frame)
                .map_err(|e| Error::Frame(Frame::Labels(e)))?;
        }
        let map = self
            .contacts
            .try_island_map_snapshot(&self.backend.rigid_bodies)
            .map_err(|e| Error::Frame(Frame::Labels(e)))?;
        let phase = self
            .backend
            .rigid_bodies
            .try_prepare_dynamic_update_phase()
            .map_err(|e| Error::Frame(Frame::Phase(e)))?;
        for record in dmo_records
            .iter_mut()
            .chain(dmo_body_records.iter_mut())
            .chain(records.iter_mut())
            .chain(body_records.iter_mut())
        {
            record.rebind_phase(&phase);
        }
        drop(profile_partition);
        let profile_contacts = crate::diagnostics::frame_profile::span(22);
        let mut contacts = self
            .contacts
            .try_prepare_assembly_contact_islands_host(
                &dmo_records,
                &dmo_body_records,
                &records,
                &body_records,
                &phase,
                &map,
            )
            .map_err(Error::Contact)?;
        drop(profile_contacts);
        let profile_joints = crate::diagnostics::frame_profile::span(23);
        let mut joints = try_prepare_active_joint_islands_host(&self.backend.joints, &phase, &map)
            .map_err(|e| Error::Frame(Frame::Joint(e)))?;
        let mut drives = try_prepare_active_drive_islands_host(&self.backend.drives, &phase, &map)
            .map_err(|e| Error::Frame(Frame::Drive(e)))?;
        let counts = SimulationAssemblyConstraintFrameCounts {
            contacts: contacts.record_count(),
            joints: joints.record_count(),
            drives: drives.record_count(),
        };
        drop(profile_joints);
        let profile_iterations = crate::diagnostics::frame_profile::span(24);
        let completed =
            crate::simulation_horus_pipeline::try_complete_prepared_horus_islands_serial(
                &mut contacts,
                &mut joints,
                &mut drives,
                SIMULATION_ISLAND_CAPACITY,
                max_iterations,
                &self.backend.drives,
                phase,
            )
            .map_err(|e| Error::Frame(Frame::Horus(e)))?;
        drop(profile_iterations);
        let _profile_feedback = crate::diagnostics::frame_profile::span(25);
        #[cfg(test)]
        {
            self.last_contact_materials = contacts.material_observations_for_test();
            self.last_drive_words =
                crate::simulation_drive_jacobian::prepared_drive_horus_batch_words_for_test(
                    &drives,
                );
        }
        self.contact_spies = Some(std::sync::Arc::new(
            contacts
                .try_export_contact_spies(completed, input.spy_flags)
                .map_err(Error::Spy)?,
        ));
        self.try_prepare_contact_feedback(feedback_input)
            .map_err(Error::Feedback)?;
        self.try_create_physical_contact_lists()
            .map_err(Error::ContactList)?;
        Ok((counts, report))
    }
}
