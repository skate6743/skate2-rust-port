//! Real manager-owned constraint frames, with an authored-ground child path.
//!
//! Reuses source `StartFrame` labels, `DriveNormalizer` (0x827b2738), the existing
//! checked pool-backed C/J/D Build producers and Horus (0x82ae2858) followed by
//! one `DynamicUpdate`. Real nonserialized `HookPool` membership now produces
//! exclusion before normalization. This is not complete `Island::Step_Solver2`:
//! thawing, assembly pair partition and live gameplay scheduling remain open.
//! No synthetic contact, arena address, body label or Jacobian supplies inputs.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "managed constraint frame awaits live gameplay scheduling"
    )
)]

#[path = "../player/ground/physical_player_ground_pre_solve.rs"]
pub(crate) mod ground_pre_solve;
#[path = "../player/conditioning/physical_player_post_solve_conditioning.rs"]
pub(crate) mod post_solve_conditioning;

use super::{SimulationAssemblyBackend, SimulationAssemblyBackendInitError};
use crate::{
    dynamic_assembly_session::hook_pool::{
        SimulationHookAssemblyCandidate, SimulationHookAssemblyHandle, SimulationHookAssemblyPool,
        SimulationHookInsertFailure, SimulationHookPoolError, SimulationHookPublicationSnapshot,
    },
    simulation_contact_jacobian::network_preparation::{
        SimulationEmptyContactPreparationError, SimulationManagerContactDomain,
        SimulationManagerContactDomainInitError,
    },
    simulation_drive_jacobian::{
        SimulationDriveIslandBatchPreparationError, try_prepare_active_drive_islands_host,
    },
    simulation_drive_pool::normalization::{
        HostSimulationDriveNormalizationNumeric, SimulationDriveNormalizationError,
    },
    simulation_horus_pipeline::{
        SimulationPreparedHorusSerialFrameError,
        try_solve_prepared_horus_islands_serial_and_integrate_ordinary_finite,
    },
    simulation_island_map::{
        SIMULATION_ISLAND_CAPACITY, persistent::SimulationRigidBodyIslandStateError,
    },
    simulation_joint_pool::{SimulationJointFrames, SimulationJointLimits},
    simulation_joint_preparation::{
        SimulationJointPreparationError, try_prepare_active_joint_islands_host,
    },
    simulation_rigid_body_pool::SimulationRigidBodyDynamicUpdatePrepareError,
};
use std::{convert::Infallible, fmt};

type Backend<M> = SimulationAssemblyBackend<M, SimulationJointFrames, SimulationJointLimits>;

#[path = "simulation_assembly_contact_feedback.rs"]
pub(crate) mod contact_feedback;
#[path = "../player/runtime/physical_player_live_frame_services.rs"]
pub(crate) mod live_frame_services;

#[path = "../player/interaction/physical_player_island.rs"]
pub(crate) mod physical_island;

#[path = "../solver/bodies/simulation_island_culling_table.rs"]
pub(crate) mod island_culling;

#[path = "../player/interaction/physical_player_contact_list.rs"]
pub(crate) mod physical_contact_list;

#[path = "../player/runtime/physical_player_history.rs"]
pub(crate) mod physical_history;

#[path = "../player/skeleton/physical_player_roots.rs"]
pub(crate) mod physical_roots;

#[path = "../player/ground/physical_player_grounded.rs"]
pub(crate) mod physical_grounded;

#[path = "../player/lifecycle/physical_player_current_animation.rs"]
pub(crate) mod physical_current_animation;

#[path = "../board/runtime/skateboard_ground_frame.rs"]
pub(crate) mod skateboard_ground;

#[path = "../player/lifecycle/physical_player_controller_board_target.rs"]
pub(crate) mod controller_board_target;
#[path = "../board/body/skateboard_pool_session.rs"]
pub(crate) mod skateboard_pool;

#[path = "../solver/simulation_dmo_living_world_session.rs"]
pub(crate) mod dmo_living_world;
#[path = "../solver/simulation_dmo_pool_session.rs"]
pub(crate) mod dmo_pool;

#[path = "../solver/simulation_world_line_query.rs"]
pub(crate) mod world_line_query;

#[path = "../player/runtime/physical_player_living_world_respawn.rs"]
pub(crate) mod living_world_respawn;
#[path = "../player/conditioning/physical_player_process_input_controls.rs"]
pub(crate) mod physical_input_controls;
#[path = "../player/skeleton/physical_player_skeleton_line_tests.rs"]
pub(crate) mod physical_skeleton_lines;
#[path = "../player/runtime/physical_player_toolkit.rs"]
pub(crate) mod physical_toolkit;
#[path = "../player/lifecycle/physical_player_update0_collision_status.rs"]
pub(crate) mod physical_update0_collision_status;
#[path = "../player/lifecycle/physical_player_update1_prefix.rs"]
pub(crate) mod physical_update1_prefix;
#[path = "../player/lifecycle/physical_player_update5_prefix.rs"]
pub(crate) mod physical_update5_prefix;
#[path = "../player/prediction/physical_player_world_line.rs"]
pub(crate) mod physical_world_line;

#[path = "../player/board/physical_player_board_fill_contacts.rs"]
pub(crate) mod board_fill_contacts;
#[path = "../player/board/physical_player_board_fill_deck.rs"]
pub(crate) mod board_fill_deck;
#[path = "../player/board/physical_player_board_fill_final.rs"]
pub(crate) mod board_fill_final;
#[path = "../player/board/physical_player_board_fill_forward.rs"]
pub(crate) mod board_fill_forward;
#[path = "../player/board/physical_player_board_fill_ground.rs"]
pub(crate) mod board_fill_ground;
#[path = "../player/board/physical_player_board_fill_prefix.rs"]
pub(crate) mod board_fill_prefix;
#[path = "../player/board/physical_player_board_fill_pumping.rs"]
pub(crate) mod board_fill_pumping;
#[path = "../player/board/physical_player_board_fill_surface.rs"]
pub(crate) mod board_fill_surface;
#[path = "../player/board/physical_player_board_fill_wheels.rs"]
pub(crate) mod board_fill_wheels;
#[path = "../player/board/physical_player_board_fill_wobble.rs"]
pub(crate) mod board_fill_wobble;
#[path = "../player/board/physical_player_board_motion.rs"]
pub(crate) mod board_motion;
#[path = "../player/board/physical_player_board_normal_services.rs"]
pub(crate) mod board_normal_services;
#[path = "../player/board/physical_player_board_post_end.rs"]
pub(crate) mod board_post_end;
#[path = "../player/board/physical_player_board_post_services.rs"]
pub(crate) mod board_post_services;
#[path = "../player/board/physical_player_board_postphysics.rs"]
pub(crate) mod board_postphysics;
#[path = "../player/board/physical_player_board_surface_choice.rs"]
pub(crate) mod board_surface_choice;
#[path = "../player/board/physical_player_board_surface_motion.rs"]
pub(crate) mod board_surface_motion;
#[path = "../player/board/physical_player_board_up_input.rs"]
pub(crate) mod board_up_input;
#[path = "../player/board/physical_player_board_up_post.rs"]
pub(crate) mod board_up_post;
#[path = "../player/offboard/physical_player_offboard_feet_ik_local_targets.rs"]
pub(crate) mod offboard_feet_ik_local_targets;
#[path = "../player/board/physical_player_board_forces.rs"]
pub(crate) mod physical_board_forces;
#[path = "../player/board/physical_player_board_surface.rs"]
pub(crate) mod physical_board_surface;
#[path = "../player/runtime/physical_player_com_session.rs"]
pub(crate) mod physical_com;
#[path = "../player/grind/physical_player_grind_permission_session.rs"]
pub(crate) mod physical_grind_permission;
#[path = "../player/ground/physical_player_ground_position_session.rs"]
pub(crate) mod physical_ground_position;
#[path = "../player/reckoning/physical_player_heading_session.rs"]
pub(crate) mod physical_heading;
#[path = "../player/runtime/physical_player_input_finish.rs"]
pub(crate) mod physical_input_finish;
#[path = "../player/air/physical_player_jump_events.rs"]
pub(crate) mod physical_jump_events;
#[path = "../player/output/physical_player_phys_out_session.rs"]
pub(crate) mod physical_phys_out;
#[path = "../player/runtime/physical_player_prepare_session.rs"]
pub(crate) mod physical_prepare;
#[path = "../player/conditioning/physical_player_process_input_ground.rs"]
pub(crate) mod physical_process_input_ground;
#[path = "../player/conditioning/physical_player_process_input_jump_publication.rs"]
pub(crate) mod physical_process_input_jump_publication;
#[path = "../player/conditioning/physical_player_process_input_post_skeleton.rs"]
pub(crate) mod physical_process_input_post_skeleton;
#[path = "../player/conditioning/physical_player_process_input_selector.rs"]
pub(crate) mod physical_process_input_selector;
#[path = "../player/runtime/physical_player_skateboard_controller_session.rs"]
pub(crate) mod physical_skateboard_controller;
#[path = "../player/skeleton/physical_player_skeleton_anim_attributes.rs"]
pub(crate) mod physical_skeleton_anim_attributes;

#[path = "../player/ground/physical_player_ground_entry_skeleton.rs"]
pub(crate) mod ground_entry_skeleton;
#[path = "../player/runtime/physical_player_motion_force_physics.rs"]
pub(crate) mod motion_force_physics;
#[path = "../player/skeleton/physical_player_skeleton_controller_session.rs"]
pub(crate) mod physical_skeleton_controller;
#[path = "../player/runtime/physical_player_teleport_now_matrix.rs"]
pub(crate) mod physical_teleport_now_matrix;
#[path = "../player/runtime/physical_player_teleport_now_prefix.rs"]
pub(crate) mod physical_teleport_now_prefix;

#[path = "../player/air/physical_player_known_air_state.rs"]
pub(crate) mod known_air_state;
#[path = "../player/lifecycle/physical_player_physics_state_session.rs"]
pub(crate) mod physical_state;
#[path = "../player/board/physical_player_wheel_material.rs"]
pub(crate) mod physical_wheel_material;
#[path = "../player/skeleton/physical_player_skeleton_air_collision.rs"]
pub(crate) mod skeleton_air_collision;
#[path = "../player/lifecycle/physical_player_system_logic_session.rs"]
pub(crate) mod system_logic;

#[path = "../player/board/physical_player_wheel_lines.rs"]
pub(crate) mod physical_wheel_lines;

#[path = "../player/interaction/physical_player_grab_world.rs"]
pub(crate) mod grab_world;
#[path = "../player/grind/physical_player_grind_states.rs"]
pub(crate) mod grind_states;
#[path = "../player/grind/physical_player_grind_world.rs"]
pub(crate) mod grind_world;
#[path = "../player/runtime/physical_player_hilod_tail_session.rs"]
pub(crate) mod hilod_tail_session;
#[path = "../player/grind/physical_player_grind_session.rs"]
pub(crate) mod physical_grind;
#[path = "../player/prediction/physical_player_prediction_owner.rs"]
pub(crate) mod prediction;
#[path = "../player/runtime/physical_player_step_selector.rs"]
pub(crate) mod step_selector;
#[path = "../player/lifecycle/physical_player_update5_outer_locality.rs"]
pub(crate) mod update5_outer_locality;
#[path = "../player/runtime/physical_player_world_services.rs"]
pub(crate) mod world_services;

#[path = "../player/offboard/physical_player_landing_on_deck_session.rs"]
pub(crate) mod landing_on_deck_session;
#[path = "../solver/simulation_pedestrian_visibility.rs"]
mod pedestrian_visibility;

pub(crate) struct SimulationAssemblyConstraintSession<M, C = ()> {
    pub(crate) debug_overlay: crate::debug_overlay::DebugOverlaySlot,
    /// Simulation+0x84/+0x88 slow-motion rate, shared with the camera graph.
    pub(crate) sim_rate:
        std::sync::Arc<std::sync::Mutex<crate::simulation_sim_rate::SimulationSimRate>>,
    pub(crate) grind_services: grind_world::PhysicalGrindServices,
    predictor: prediction::PhysicalPredictionSlot,
    step_selector: step_selector::StepSelectorSlot,
    grab_query_manager: grab_world::GrabQueryManagerSlot,
    world_services: Option<std::sync::Arc<world_services::PhysicalWorldServices>>,
    world_services_attempted: bool,
    feedback_parameters:
        Option<std::sync::Arc<live_frame_services::SimulationManagerFeedbackParameters>>,
    pub(crate) entity_roster:
        Option<std::sync::Arc<crate::simulation_entity_roster::SimulationEntityRoster>>,
    pub(crate) local_skater_identity:
        Option<std::sync::Arc<crate::skater_manager_local_identity::SkaterManagerLocalIdentity>>,
    /// The other online players' skaters as the local skater's scoring collects them, with
    /// their body points (PC port: the original's network game simulated every player's skater
    /// in its SkaterManager, whose roster `CollectNearbyEntitiesForScoring` walks).
    pub(crate) remote_scoring_skaters: std::sync::Mutex<Vec<crate::physical_player_phys_out::conditioners::scoring_external::RemoteScoringSkater>>,
    backend: Backend<M>,
    contacts: SimulationManagerContactDomain,
    hooks: SimulationHookAssemblyPool<M, C>,
    island_flags: u32,
    /// `Island::mCullingTable`, built by the ctor's InitCullingTable 0x827bad10.
    island_culling: skate2_formats::collision::BitTable,
    /// The culling table is the online game's ([`Self::set_online_game_culling`]).
    online_game_culling: bool,
    simulation_max_iterations: Option<u32>,
    world_line_queries: Option<world_line_query::SimulationWorldLineQueries>,
    skaters: Option<crate::dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyPool>,
    pedestrians: Option<crate::simulation_dmo_assembly_part_session::dynamic_pool::character::pedestrian_pool::PedestrianAssemblyPool>,
    dmos: Option<
        crate::simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool,
    >,
    /// Set by the LivingWorld schedule's `BeginDMOUpdate`; the next physics
    /// step's `EndDMOUpdate` consumes it.
    dmo_frame: Option<dmo_living_world::DmoFrameSettings>,
    contact_spies: Option<
        std::sync::Arc<crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame>,
    >,
    contact_feedback_progress: contact_feedback::SimulationContactFeedbackProgress,
    physical_bodies: physical_island::SimulationPhysicalBodyRegistry,
    physical_contacts: physical_contact_list::SimulationPhysicalContactLists,
    skeleton_culling: crate::skeleton_culling::SkeletonCullingInventory,
    #[cfg(test)]
    last_drive_words: Vec<[[u32; 4]; 24]>,
    #[cfg(test)]
    last_contact_materials: Vec<
        crate::simulation_contact_jacobian::network_preparation::PreparedContactMaterialObservation,
    >,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dynamic_assembly_session::DynamicAssemblySimulationBackend,
        simulation_drive_pool::{
            SimulationDriveDynamics, SimulationDriveDynamicsParams, SimulationDriveFrames,
        },
        simulation_rigid_body_pool::SimulationInertia,
        skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    };
    use std::sync::{Arc, RwLock};

    #[test]
    fn manager_static_body_is_required_before_admission() {
        assert!(matches!(
            SimulationAssemblyConstraintSession::<()>::try_new(0, 0, 0, 0),
            Err(SimulationAssemblyConstraintInitError::Manager(
                SimulationManagerContactDomainInitError::StaticBodyUnavailable
            ))
        ));
        assert!(matches!(
            SimulationAssemblyConstraintSession::<()>::try_new(1, 0, 0, u32::MAX),
            Err(SimulationAssemblyConstraintInitError::Manager(
                SimulationManagerContactDomainInitError::NativeContactByteSizeOverflow { .. }
            ))
        ));
    }

    #[test]
    fn empty_managed_frames_retain_one_static_occurrence_and_no_records() {
        let mut world = SimulationAssemblyConstraintSession::<()>::try_new(1, 0, 0, 0).unwrap();
        let original = world.backend().rigid_body_snapshot();
        for _ in 0..2 {
            assert_eq!(
                world.try_step_contactless(1.0 / 60.0, 50).unwrap(),
                SimulationAssemblyConstraintFrameCounts {
                    contacts: 0,
                    joints: 0,
                    drives: 0
                }
            );
            assert_eq!(world.backend().rigid_body_snapshot().slots, original.slots);
            assert!(world.active_rf_is_zero_for_test());
        }
    }

    #[test]
    fn normalization_failure_keeps_prior_drive_change_without_body_integration() {
        let mut world = SimulationAssemblyConstraintSession::<()>::try_new(3, 0, 2, 0).unwrap();
        let backend = world.backend_mut();
        let inertia = Arc::new(SimulationInertia::native_default()).into();
        let parent = backend
            .add_rigid_body(&FRESH_VOLUME_TRANSFORM, &inertia, 4)
            .unwrap();
        let child = backend
            .add_rigid_body(&FRESH_VOLUME_TRANSFORM, &inertia, 4)
            .unwrap();
        let mut frames = Vec::new();
        for _ in 0..2 {
            let frame = Arc::new(RwLock::new(SimulationDriveFrames {
                quaternion_a: [0.0, 0.0, 0.0, 2.0],
                position_a: [0.0; 4],
                quaternion_b: [2.0, 0.0, 0.0, 0.0],
                position_b: [0.0; 4],
            }));
            let params = SimulationDriveDynamicsParams {
                spring: 0.0,
                damping: 0.0,
                strength: 0.0,
                raw_type: 0,
            };
            let dynamics = Arc::new(RwLock::new(SimulationDriveDynamics {
                linear: params,
                angular: params,
            }));
            assert!(
                backend
                    .add_drive(&parent, &child, &frame, &dynamics)
                    .is_some()
            );
            frames.push(frame);
        }
        let poisoned = Arc::clone(&frames[1]);
        let _ = std::panic::catch_unwind(move || {
            let _guard = poisoned.write().unwrap();
            panic!("controlled later-drive failure");
        });
        let before = world.backend().rigid_body_snapshot();
        assert!(matches!(
            world.try_step_contactless(1.0 / 60.0, 50),
            Err(SimulationAssemblyConstraintFrameError::Normalize(
                SimulationDriveNormalizationError::FrameLockPoisoned { .. }
            ))
        ));
        assert_eq!(
            frames[0].read().unwrap().quaternion_a[3].to_bits(),
            1.0_f32.to_bits()
        );
        let after = world.backend().rigid_body_snapshot();
        assert_eq!(after.slots, before.slots);
        assert_eq!(
            after.next_dynamic_update_phase_identity,
            before.next_dynamic_update_phase_identity
        );
        assert!(world.active_rf_is_zero_for_test());
    }
}

#[derive(Debug)]
pub(crate) enum SimulationAssemblyConstraintInitError {
    Backend(SimulationAssemblyBackendInitError),
    Manager(SimulationManagerContactDomainInitError),
    Hook(SimulationHookPoolError),
}
impl fmt::Display for SimulationAssemblyConstraintInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(error) => write!(f, "constraint pools: {error}"),
            Self::Manager(error) => write!(f, "constraint manager: {error:?}"),
            Self::Hook(error) => write!(f, "constraint HookPool: {error}"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum SimulationAssemblyConstraintFrameError {
    Labels(SimulationRigidBodyIslandStateError),
    Hook(SimulationHookPoolError),
    Normalize(SimulationDriveNormalizationError<Infallible>),
    Phase(SimulationRigidBodyDynamicUpdatePrepareError),
    Contact(SimulationEmptyContactPreparationError),
    Joint(SimulationJointPreparationError<Infallible>),
    Drive(SimulationDriveIslandBatchPreparationError<Infallible>),
    Horus(SimulationPreparedHorusSerialFrameError),
}
impl fmt::Display for SimulationAssemblyConstraintFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Labels(error) => write!(f, "constraint labels: {error:?}"),
            Self::Hook(error) => write!(f, "constraint hook labels: {error}"),
            Self::Normalize(error) => write!(f, "constraint normalization: {error:?}"),
            Self::Phase(error) => write!(f, "constraint body phase: {error:?}"),
            Self::Contact(error) => write!(f, "constraint Contact: {error}"),
            Self::Joint(error) => write!(f, "constraint Joint: {error:?}"),
            Self::Drive(error) => write!(f, "constraint Drive: {error:?}"),
            Self::Horus(error) => write!(f, "constraint Horus: {error:?}"),
        }
    }
}

/// Counts observed from actual compiled batches, not requested capacities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SimulationAssemblyConstraintFrameCounts {
    pub(crate) contacts: usize,
    pub(crate) joints: usize,
    pub(crate) drives: usize,
}

impl<M, C> SimulationAssemblyConstraintSession<M, C> {
    /// Construct domain on a pristine pool: persistent labels first, then the
    /// dedicated manager static body. All gameplay bodies must be admitted later.
    pub(crate) fn try_new(
        bodies: usize,
        joints: usize,
        drives: usize,
        maximum_contacts: u32,
    ) -> Result<Self, SimulationAssemblyConstraintInitError> {
        Self::try_new_with_island_flags(bodies, joints, drives, maximum_contacts, 3)
    }

    /// Explicit source world configuration. The historical component constructor
    /// above selects Both3, but does not implement the complete flags3 Tick.
    /// An online game's culling table. The original's online game sets `sGhostSystemEnabled`
    /// (`StateOnlineGame::SetupInitalGameInfo` 0x8275dba4) until `GameModule::BackToSP`, and
    /// `World::UpdateIslandFlags` 0x827ab9c0 then makes the simulation island `FLAG_Simulation`
    /// (`Island::ChangeFlags` 0x827ba5b0), whose table (`InitCullingTable` 0x827b8c10) culls
    /// boards and skeletons against each other (groups 4, 6 and 7): an off-board skater passes
    /// through every board, its own included. The port's netcode leaves the ghost system clear,
    /// so only the table follows the online game; the island keeps its flags.
    pub(crate) fn set_online_game_culling(&mut self, online: bool) {
        if online == self.online_game_culling {
            return;
        }
        self.online_game_culling = online;
        self.island_culling = island_culling::island_culling_table(if online { 1 } else { self.island_flags });
    }

    pub(crate) fn try_new_with_island_flags(
        bodies: usize,
        joints: usize,
        drives: usize,
        maximum_contacts: u32,
        island_flags: u32,
    ) -> Result<Self, SimulationAssemblyConstraintInitError> {
        let mut backend = Backend::try_new(bodies, joints, drives)
            .map_err(SimulationAssemblyConstraintInitError::Backend)?;
        let contacts =
            SimulationManagerContactDomain::try_new(&mut backend.rigid_bodies, maximum_contacts)
                .map_err(SimulationAssemblyConstraintInitError::Manager)?;
        // Both real assembly pools must receive the same world flags.
        let hooks = SimulationHookAssemblyPool::try_new(&contacts, &backend, island_flags)
            .map_err(SimulationAssemblyConstraintInitError::Hook)?;
        Ok(Self {
            debug_overlay: crate::debug_overlay::DebugOverlaySlot::default(),
            sim_rate: std::sync::Arc::new(std::sync::Mutex::new(
                crate::simulation_sim_rate::SimulationSimRate::construct(),
            )),
            local_skater_identity: None,
            entity_roster: None,
            grind_services: grind_world::PhysicalGrindServices::new(),
            predictor: prediction::PhysicalPredictionSlot::default(),
            step_selector: step_selector::StepSelectorSlot::default(),
            grab_query_manager: grab_world::GrabQueryManagerSlot::default(),
            world_services: None,
            world_services_attempted: false,
            feedback_parameters: None,
            backend,
            contacts,
            hooks,
            island_flags,
            island_culling: island_culling::island_culling_table(island_flags),
            online_game_culling: false,
            simulation_max_iterations: None,
            world_line_queries: None,
            skaters: None,
            pedestrians: None,
            dmos: None,
            dmo_frame: None,
            contact_spies: None,
            physical_bodies: physical_island::SimulationPhysicalBodyRegistry::new(),
            physical_contacts: physical_contact_list::SimulationPhysicalContactLists::new(),
            skeleton_culling: crate::skeleton_culling::SkeletonCullingInventory::default(),
            contact_feedback_progress:
                contact_feedback::SimulationContactFeedbackProgress::NotStarted,
            remote_scoring_skaters: std::sync::Mutex::new(Vec::new()),
            #[cfg(test)]
            last_drive_words: Vec::new(),
            #[cfg(test)]
            last_contact_materials: Vec::new(),
        })
    }

    /// Existing opaque backend is the assembly admission/configuration port;
    /// no mutable pool, manager replacement or raw body token is exported.
    pub(crate) fn backend_mut(&mut self) -> &mut Backend<M> {
        &mut self.backend
    }
    pub(crate) fn backend(&self) -> &Backend<M> {
        &self.backend
    }

    /// Retained output from the current successful solve/integration/export.
    /// A later feedback-input refusal keeps that completed prefix observable;
    /// it does not imply that the whole authored ground frame succeeded.
    /// This is not `Island::GetCollisions` or a reconstructed Body contact list.
    pub(crate) fn contact_spies(
        &self,
    ) -> Result<
        Option<&crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame>,
        crate::simulation_contact_jacobian::contact_spy::SimulationContactSpyError,
    > {
        if let Some(spies) = &self.contact_spies {
            spies.try_validate_current(&self.contacts, &self.backend.rigid_bodies)?;
        }
        Ok(self.contact_spies.as_deref())
    }

    pub(crate) fn try_insert_hook(
        &mut self,
        candidate: SimulationHookAssemblyCandidate<M, C>,
    ) -> Result<SimulationHookAssemblyHandle, Box<SimulationHookInsertFailure<M, C>>> {
        self.hooks
            .try_insert(&mut self.backend, &mut self.contacts, candidate)
    }

    pub(crate) fn try_remove_hook(
        &mut self,
        hook: &SimulationHookAssemblyHandle,
    ) -> Result<SimulationHookAssemblyCandidate<M, C>, SimulationHookPoolError> {
        self.hooks.try_remove(&mut self.backend, hook)
    }

    pub(crate) fn try_hook_snapshot(
        &self,
        hook: &SimulationHookAssemblyHandle,
    ) -> Result<SimulationHookPublicationSnapshot, SimulationHookPoolError> {
        self.hooks.try_snapshot(hook)
    }

    pub(crate) fn hook_count(&self) -> usize {
        self.hooks.len()
    }

    /// Explicit contactless/pair-free restricted frame, including real `HookPool`
    /// exclusion. Source labels are not a
    /// substitute for future collision-pair partitioning. Caller supplies actual
    /// frame timestep/current iteration snapshot; no gameplay tuning is baked in.
    /// Earlier StartFrame/normalization effects survive a later host failure.
    pub(crate) fn try_step_contactless(
        &mut self,
        timestep: f32,
        max_iterations: u32,
    ) -> Result<SimulationAssemblyConstraintFrameCounts, SimulationAssemblyConstraintFrameError>
    {
        use SimulationAssemblyConstraintFrameError as Error;
        self.backend
            .rigid_bodies
            .begin_dynamic_update_frame(timestep);
        self.contacts
            .try_start_island_frame_labels(&self.backend.rigid_bodies)
            .map_err(Error::Labels)?;
        self.contacts
            .try_exclude_hook_assemblies(&self.backend.rigid_bodies, &self.hooks)
            .map_err(Error::Hook)?;
        self.backend
            .drives
            .try_normalize_active_drive_frames(&mut HostSimulationDriveNormalizationNumeric)
            .map_err(Error::Normalize)?;
        let map = self
            .contacts
            .try_island_map_snapshot(&self.backend.rigid_bodies)
            .map_err(Error::Labels)?;
        let phase = self
            .backend
            .rigid_bodies
            .try_prepare_dynamic_update_phase()
            .map_err(Error::Phase)?;
        let mut contacts = self
            .contacts
            .try_prepare_empty_contact_islands_host(&phase, &map)
            .map_err(Error::Contact)?;
        let mut joints = try_prepare_active_joint_islands_host(&self.backend.joints, &phase, &map)
            .map_err(Error::Joint)?;
        let mut drives = try_prepare_active_drive_islands_host(&self.backend.drives, &phase, &map)
            .map_err(Error::Drive)?;
        let counts = SimulationAssemblyConstraintFrameCounts {
            contacts: contacts.record_count(),
            joints: joints.record_count(),
            drives: drives.record_count(),
        };
        try_solve_prepared_horus_islands_serial_and_integrate_ordinary_finite(
            &mut contacts,
            &mut joints,
            &mut drives,
            SIMULATION_ISLAND_CAPACITY,
            max_iterations,
            &self.backend.drives,
            phase,
        )
        .map_err(Error::Horus)?;
        #[cfg(test)]
        {
            self.last_drive_words =
                crate::simulation_drive_jacobian::prepared_drive_horus_batch_words_for_test(
                    &drives,
                );
        }
        Ok(counts)
    }

    #[cfg(test)]
    pub(crate) fn drive_words_for_test(&self) -> &[[[u32; 4]; 24]] {
        &self.last_drive_words
    }
    #[cfg(test)]
    pub(crate) fn contact_materials_for_test(&self) -> &[crate::simulation_contact_jacobian::network_preparation::PreparedContactMaterialObservation]{
        &self.last_contact_materials
    }

    #[cfg(test)]
    pub(crate) fn island_labels_for_test(&self) -> Vec<(u32, i8)> {
        let map = self
            .contacts
            .try_island_map_snapshot(&self.backend.rigid_bodies)
            .unwrap();
        let mut labels = Vec::new();
        self.backend
            .rigid_bodies
            .try_for_each_allocated_observation(|body| {
                labels.push((
                    body.identity().id().index(),
                    map.assignment(body).unwrap().1,
                ));
            })
            .unwrap();
        labels
    }

    #[cfg(test)]
    pub(crate) fn active_rf_is_zero_for_test(&self) -> bool {
        let mut zero = true;
        self.backend
            .rigid_bodies
            .try_for_each_active_observation(|body| {
                zero &= self
                    .backend
                    .rigid_bodies
                    .try_with_solver_delta_row(body, |rows| {
                        rows.iter().flatten().all(|value| value.to_bits() == 0)
                    })
                    .expect("same immutable live body authority");
            })
            .expect("valid same-owner active ring");
        zero
    }
}

#[path = "../player/lifecycle/physical_player_update3_prefix.rs"]
pub(crate) mod physical_update3_prefix;

#[path = "../player/ground/physical_player_ground_update_prefix.rs"]
pub(crate) mod physical_ground_update_prefix;

#[path = "../player/ground/physical_player_ground_handplants.rs"]
pub(crate) mod physical_ground_handplants;
#[path = "../player/ground/physical_player_ground_pumping.rs"]
pub(crate) mod physical_ground_pumping;

#[path = "../player/ground/physical_player_ground_control.rs"]
pub(crate) mod ground_control;

#[path = "../player/ground/physical_player_ground_control_complete.rs"]
pub(crate) mod ground_control_complete;

#[path = "../player/lifecycle/physical_player_update5_skeleton_prefix.rs"]
pub(crate) mod update5_skeleton_prefix;

#[path = "../player/ground/physical_player_ground_collision.rs"]
pub(crate) mod ground_collision;
#[path = "../player/ground/physical_player_ground_fake_torque.rs"]
pub(crate) mod ground_fake_torque;
#[path = "../player/ground/physical_player_ground_wallride.rs"]
pub(crate) mod ground_wallride;

#[path = "../player/lifecycle/physical_player_update5_skeleton_parts.rs"]
pub(crate) mod update5_skeleton_parts;

#[path = "../player/runtime/physical_player_push_source.rs"]
pub(crate) mod push_source;

#[path = "../player/board/physical_player_pumping_force_source.rs"]
pub(crate) mod pumping_force_source;

#[path = "../player/ground/physical_player_ground_ordinary_forces.rs"]
pub(crate) mod ground_ordinary_forces;

#[path = "../player/lifecycle/physical_player_update5_state_publication.rs"]
pub(crate) mod update5_state_publication;

#[path = "../player/runtime/physical_player_straighten_source.rs"]
pub(crate) mod straighten_source;

#[path = "../player/reckoning/physical_player_heading_torque_source.rs"]
pub(crate) mod heading_torque_source;

#[path = "../player/air/physical_player_antiflip_source.rs"]
pub(crate) mod antiflip_source;

#[path = "../player/wipeout/physical_player_post_wipeout_pose_world.rs"]
pub(crate) mod skeleton_pose_publication;
#[path = "../player/wipeout/physical_player_post_wipeout_live.rs"]
pub(crate) mod skeleton_post_wipeout;
#[path = "../player/skeleton/physical_player_skeleton_postphysics_live_world.rs"]
pub(crate) mod skeleton_postphysics_live;

#[path = "../player/board/physical_player_manual_effect_source.rs"]
pub(crate) mod manual_effect_source;

#[path = "../player/ground/physical_player_ground_manual_nudge.rs"]
pub(crate) mod ground_manual_nudge;

#[path = "../player/ground/physical_player_ground_antiflip_nudge.rs"]
pub(crate) mod antiflip_nudge;

#[path = "../player/ground/physical_player_ground_queue_torque_prefix.rs"]
pub(crate) mod ground_queue_torque_prefix;

#[path = "../player/lifecycle/physical_player_update5_skeleton_suffix.rs"]
pub(crate) mod update5_skeleton_suffix;

#[path = "../player/ground/physical_player_ground_hang_pinning.rs"]
pub(crate) mod ground_hang_pinning;

#[path = "../player/ground/physical_player_ground_hang_pinning_session.rs"]
pub(crate) mod ground_hang_pinning_session;

#[path = "../player/ground/physical_player_ground_force_prediction.rs"]
pub(crate) mod ground_force_prediction;

#[path = "../player/ground/physical_player_ground_force_prediction_session.rs"]
pub(crate) mod ground_force_prediction_session;

#[path = "../player/ground/physical_player_ground_finish.rs"]
pub(crate) mod ground_finish;

#[path = "../player/ground/physical_player_ground_finish_session.rs"]
pub(crate) mod ground_finish_session;

#[path = "../player/board/physical_player_board_post_outer_live.rs"]
pub(crate) mod board_post_outer_live;

#[path = "../player/lifecycle/physical_player_update5_pumping_session.rs"]
pub(crate) mod update5_pumping;

#[path = "../player/ground/physical_player_ground_state_fill.rs"]
pub(crate) mod ground_state_fill;
#[path = "../player/ground/physical_player_ground_wipeout_runtime.rs"]
pub(crate) mod ground_wipeout;

#[path = "../player/lifecycle/physical_player_controller_update_grip.rs"]
pub(crate) mod controller_update_grip;
#[path = "../player/lifecycle/physical_player_update3_controller_session.rs"]
pub(crate) mod update3_controller;

#[path = "../player/lifecycle/physical_player_update3_prephysics_tail.rs"]
pub(crate) mod update3_prephysics_tail;

#[path = "../player/lifecycle/physical_player_update5_manager_session.rs"]
pub(crate) mod update5_managers;

#[path = "../player/lifecycle/physical_player_update5_plant_session.rs"]
pub(crate) mod update5_plants;

#[path = "../player/lifecycle/physical_player_update5_controller_session.rs"]
pub(crate) mod update5_controller;

#[path = "../player/lifecycle/physical_player_update5_wipeout_session.rs"]
pub(crate) mod update5_wipeout;

#[path = "../player/air/physical_player_air_launch_runtime.rs"]
pub(crate) mod air_launch_runtime;
#[path = "../player/offboard/physical_player_offboard_trajectory_runtime.rs"]
pub(crate) mod offboard_trajectory_runtime;
#[path = "../player/runtime/physical_player_onboard_launch_runtime.rs"]
pub(crate) mod onboard_launch_runtime;

#[path = "../player/air/physical_player_air_update_prefix.rs"]
pub(crate) mod air_update_prefix;

#[path = "../player/air/physical_player_air_jump_velocity.rs"]
pub(crate) mod air_jump_velocity;
#[path = "../player/air/physical_player_air_skateboard_runtime.rs"]
pub(crate) mod air_skateboard_runtime;

#[path = "../player/air/physical_player_air_update_finish.rs"]
pub(crate) mod air_update_finish;
#[path = "../player/board/physical_player_revert_ground.rs"]
pub(crate) mod revert_ground;
#[path = "../player/board/physical_player_slide_ground.rs"]
pub(crate) mod slide_ground;

#[path = "../player/footplant/physical_player_boneless_state.rs"]
pub(crate) mod boneless_state;
#[path = "../player/footplant/physical_player_footplant_air.rs"]
pub(crate) mod footplant_air;
#[path = "../player/footplant/physical_player_footplant_state.rs"]
pub(crate) mod footplant_state;
#[path = "../player/ground/physical_player_ground_animation_update.rs"]
pub(crate) mod ground_animation_update;
#[path = "../player/ground/physical_player_ground_jump_runtime.rs"]
pub(crate) mod ground_jump_runtime;
#[path = "../player/handplant/physical_player_handplant_state.rs"]
pub(crate) mod handplant_state;
#[path = "../player/board/physical_player_skitch_board.rs"]
mod skitch_board;
#[path = "../player/lifecycle/physical_player_skitch_session.rs"]
pub(crate) mod skitch_session;
#[path = "../player/lifecycle/physical_player_follow_path_session.rs"]
pub(crate) mod follow_path_session;

#[path = "../player/air/physical_player_air_com_correction.rs"]
pub(crate) mod air_com_correction;

#[path = "../player/air/physical_player_air_com_launch.rs"]
pub(crate) mod air_com_launch;

#[path = "../player/prediction/physical_player_selector_runtime.rs"]
pub(crate) mod physical_selector_runtime;

#[path = "../player/prediction/physical_player_selector_finish_runtime.rs"]
pub(crate) mod physical_selector_finish_runtime;

#[path = "../player/offboard/physical_player_offboard_location_session.rs"]
pub(crate) mod offboard_location_session;
