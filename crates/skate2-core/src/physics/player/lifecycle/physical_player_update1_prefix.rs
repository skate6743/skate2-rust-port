//! Actual `Update1_ProcessInput82e01f98..2174`, through its teleport check.
use crate::physics_state_ids::PHYSICS_STATE_PHYSICS_GROUND;
use super::physical_grounded::live::TeleportGroundSkateboardError;
use super::physical_skeleton_lines::SkeletonLineTestsError;
use super::physical_state::ground_to_air::{GroundDestination, GroundToAirError};
use super::physical_teleport_now_prefix::TeleportNowPrefixError;
use crate::{
    actor_construction::ActorConstruction,
    actor_construction::anim_out_phys_in::AnimOutPhysInOwner,
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::{
        PhysOutAccessError, PhysOutMotionScalar, construct::prior_input::PriorInputReadError,
    },
    reckoning_numeric::ReckoningNumeric,
    skeleton_processed_contact::{
        SkeletonProcessedContact, animation_input::ProcessedAnimationInputError,
    },
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};

/// The source must next execute the AI/input prefix at2174.
/// This is a retained continuation, not a completed physical update.
pub(crate) struct Update1AfterTeleportCheck {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) input: Arc<Mutex<AnimOutPhysInOwner>>,
    pub(crate) processed: Arc<Mutex<SkeletonProcessedContact>>,
    pub(crate) prior_category: i32,
}
#[derive(Debug)]
pub(crate) enum Update1PrefixError<N> {
    Missing(&'static str),
    Poisoned,
    ForeignInput,
    ForeignManager,
    Jumping(
        crate::physical_player_remaining_constructor_owners::jumping_runtime::JumpingError<
            N,
            crate::input_cvar_runtime::InputCvarError,
        >,
    ),
    #[cfg(feature = "skate3-runtime")]
    NativeModeExit(String),
    TeleportNotPublished,
    TeleportPlayerNowPending(crate::skeleton_root_transforms::SkeletonRootMatrix),
    TeleportNowPrefix(TeleportNowPrefixError),
    TeleportGroundSkateboard(TeleportGroundSkateboardError<N>),
    TeleportSetPhysicsState(GroundToAirError<N>),
    Pair(SimulationSkeletonJumpPairError),
    Lines(SkeletonLineTestsError<N>),
    Read(PriorInputReadError),
    Output(PhysOutAccessError),
    Animation(ProcessedAnimationInputError),
}
impl<N: std::fmt::Debug> std::fmt::Display for Update1PrefixError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "physical Update1 missing {field}"),
            Self::Poisoned => f.write_str("physical Update1 owner lock poisoned"),
            Self::ForeignInput => f.write_str("physical Update1 input differs from the bound Actor input"),
            Self::ForeignManager => f.write_str("physical Update1 offboard manager belongs to another owner"),
            Self::Jumping(error) => f.write_str("physical Update1 OffboardJumpingTest.CalculateResults failed for its retained line batch").and_then(|()| write!(f, ": {error:?}")),
            #[cfg(feature = "skate3-runtime")]
            Self::NativeModeExit(error) => write!(f, "native physical ownership return: {error}"),
            Self::TeleportNotPublished => f.write_str("CheckForTeleport assertion: finished teleport lacks Motion.did_teleport"),
            Self::TeleportPlayerNowPending(transform) => write!(f, "CheckForTeleport requires TeleportPlayerNow with the captured transform {transform:?}"),
            Self::TeleportNowPrefix(error) => write!(f, "TeleportPlayerNow prefix: {error:?}"),
            Self::TeleportGroundSkateboard(error) => write!(f, "TeleportPlayerNow UpdateGroundSkateboard: {error:?}"),
            Self::TeleportSetPhysicsState(error) => write!(f, "TeleportPlayerNow SetPhysicsState(Ground): {error}"),
            Self::Pair(error) => write!(f, "physical Update1 pair: {error:?}"),
            Self::Lines(error) => write!(f, "physical Update1 lines: {error}"),
            Self::Read(error) => write!(f, "physical Update1 guarded read: {error:?}"),
            Self::Output(error) => write!(f, "physical Update1 output: {error:?}"),
            Self::Animation(error) => write!(f, "physical Update1 animation: {error:?}"),
        }
    }
}
fn tick_managers<N: ReckoningNumeric>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    queries: Option<&mut crate::simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineQueries>,
    numeric: &mut N,
) -> Result<(), Update1PrefixError<N::Error>> {
    use Update1PrefixError as E;
    let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
    let location = remaining
        .location
        .as_mut()
        .ok_or(E::Missing("Location manager"))?;
    if !location.world.matches(owner) {
        return Err(E::ForeignManager);
    }
    // Tick82dd1fa0 does not clear sync_allowed when the pointer is null.
    if location.line_batch.is_some() {
        location.sync_allowed = true;
    }
    let jumping = remaining
        .jumping
        .as_mut()
        .ok_or(E::Missing("Jumping manager"))?;
    if !jumping.world.matches(owner) {
        return Err(E::ForeignManager);
    }
    // Tick82dd5f98 decrements with native signed wrapping before the signed clamp,
    // then a pending line batch completes through CalculateResults82dd5660.
    if jumping.line_batch.is_some() {
        let queries = queries.ok_or(E::Missing("world line queries"))?;
        if !queries.belongs_to(owner.manager()) {
            return Err(E::ForeignManager);
        }
        jumping
            .tick(queries, &mut owner.inputs().cvars.clone(), numeric)
            .map_err(E::Jumping)?;
    } else {
        jumping.result_valid_counter = jumping.result_valid_counter.wrapping_sub(1).max(0);
    }
    // LandingOnDeckManager.Tick82dc8ed8 has exactly this one store.
    remaining
        .landing
        .as_mut()
        .ok_or(E::Missing("Landing manager"))?
        .can_land = false;
    Ok(())
}
fn set_known_flag(word: &mut (u32, u32), mask: u32, value: bool) {
    word.0 = (word.0 & !mask) | if value { mask } else { 0 };
    word.1 |= mask;
}
impl PhysicalAssemblyWorld {
    /// Runs original entry ticks and line launch before capturing category and Reset.
    /// Failures retain all earlier native writes; this method must not be retried as
    /// though those ticks and launched query batches had not happened.
    #[expect(
        clippy::too_many_lines,
        reason = "consecutive native manager, query, Reset, and masked-store ordering must preserve failure prefixes"
    )]
    pub(crate) fn try_begin_player_update1<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        actor: &ActorConstruction,
        expected_input: &Arc<Mutex<AnimOutPhysInOwner>>,
        ground: &StaticGroundPool,
        numeric: &mut N,
    ) -> Result<Update1AfterTeleportCheck, Update1PrefixError<N::Error>>
    where
        N::Error: std::fmt::Debug,
    {
        use Update1PrefixError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        tick_managers(&owner, self.world_line_queries.as_mut(), numeric)?;
        self.try_start_skeleton_line_tests(board, skeleton, ground, numeric)
            .map_err(E::Lines)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let output = owner.phys_out().lock().map_err(|_| E::Poisoned)?;
        let prior_category = output
            .try_guarded_state_output()
            .map_err(E::Read)?
            .physics_state_category;
        let mut destination = processed.lock().map_err(|_| E::Poisoned)?;
        destination.reset();
        // Source first dereferences the retained input at2014, after Reset.
        let input = owner
            .animation_physics_input
            .lock()
            .map_err(|_| E::Poisoned)?
            .as_ref()
            .cloned()
            .ok_or(E::Missing("bound animation input"))?;
        if !Arc::ptr_eq(&input, expected_input) {
            return Err(E::ForeignInput);
        }
        let animation = input.lock().map_err(|_| E::Poisoned)?;
        let forced_brake = if animation.flags.force_braking {
            let state_count = owner
                .physics_state()
                .lock()
                .map_err(|_| E::Poisoned)?
                .state_count
                .ok_or(E::Missing("state count"))?;
            state_count > 5
                && output
                    .motion_scalar(PhysOutMotionScalar::DeckSpeed)
                    .map_err(E::Output)?
                    .value
                    < f32::from_bits(0x3dcc_cccd)
                && output
                    .try_guarded_state_output()
                    .map_err(E::Read)?
                    .physics_state
                    == PHYSICS_STATE_PHYSICS_GROUND
                && output.try_guarded_wheels_in_contact().map_err(E::Read)? > 2
        } else {
            false
        };
        let flags = &mut destination.physics_state_fields_mut().flags_8cc;
        let old = flags.ok_or(E::Missing("Processed flags8cc"))?;
        *flags = Some((old & !0x0400_0000) | if forced_brake { 0x0400_0000 } else { 0 });
        let (partial, impaled) = output
            .try_guarded_skeleton_collision_flags()
            .map_err(E::Read)?;
        set_known_flag(
            &mut destination.physics_state_fields_mut().flags_8d4,
            0x1000_0000,
            partial,
        );
        set_known_flag(
            &mut destination.physics_state_fields_mut().flags_8d4,
            0x0040_0000,
            impaled,
        );
        let remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
        let valid = remaining
            .jumping
            .as_ref()
            .ok_or(E::Missing("Jumping manager"))?
            .result_valid_counter
            > 0;
        set_known_flag(
            &mut destination.physics_state_fields_mut().flags_8d4,
            0x2000_0000,
            valid,
        );
        destination
            .physics_state_fields_mut()
            .landing_on_deck_adjust = Some(
            remaining
                .landing
                .as_ref()
                .ok_or(E::Missing("Landing manager"))?
                .landing_error,
        );
        drop(remaining);
        destination
            .try_process_animation_input(&animation)
            .map_err(E::Animation)?;
        destination.physics_state_fields_mut().teleport_exit_state = Some(
            output
                .try_guarded_state_output()
                .map_err(E::Read)?
                .teleport_exit_state,
        );
        match check_for_teleport::<N::Error>(&output) {
            Err(E::TeleportPlayerNowPending(transform)) => {
                // Native CheckForTeleport calls TeleportPlayerNow here. Release
                // host guards before its same-pair collision transition; all
                // Processed/AnimOut writes above remain published.
                drop(animation);
                drop(destination);
                drop(output);
                self.try_begin_pair_teleport_player_now(board, skeleton, transform)
                    .map_err(E::TeleportNowPrefix)?;
                self.try_update_pair_teleport_ground_skateboard(
                    &owner, board, skeleton, ground, numeric,
                )
                .map_err(E::TeleportGroundSkateboard)?;
                self.try_finish_teleport_after_ground_skateboard(&owner)
                    .map_err(E::TeleportNowPrefix)?;
                self.try_reset_teleport_conditioners(&owner)
                    .map_err(E::TeleportNowPrefix)?;
                #[cfg(feature = "skate3-runtime")]
                crate::skate3_native::physics::player::exit::apply(self, &owner, actor)
                    .map_err(E::NativeModeExit)?;
                self.try_transition_pair_ground(
                    board,
                    skeleton,
                    actor,
                    GroundDestination::Ground,
                    &owner.inputs().cvars,
                    Some(ground),
                    numeric,
                    &mut |angle| Ok(angle.cos()),
                )
                .map_err(E::TeleportSetPhysicsState)?;
                self.try_finish_teleport_outer_normal(board, skeleton)
                    .map_err(E::TeleportNowPrefix)?;
                self.try_finish_teleport_network_collision(&owner, board, skeleton)
                    .map_err(E::TeleportNowPrefix)?;
                return Ok(Update1AfterTeleportCheck {
                    owner,
                    input,
                    processed,
                    prior_category,
                });
            }
            result => result?,
        }
        drop(animation);
        drop(destination);
        drop(output);
        Ok(Update1AfterTeleportCheck {
            owner,
            input,
            processed,
            prior_category,
        })
    }
}

/// Actual `CheckForTeleport82e01db8` selected branch and assertion order.
pub(crate) fn check_for_teleport<N>(
    output: &crate::physical_player_phys_out::PhysicalPlayerPhysOut,
) -> Result<(), Update1PrefixError<N>> {
    use Update1PrefixError as E;
    // Each source getter samples the latch again.
    if output
        .try_guarded_state_output()
        .map_err(E::Read)?
        .is_teleport_finished
    {
        if !output
            .try_guarded_motion_remainder()
            .map_err(E::Read)?
            .did_teleport
        {
            return Err(E::TeleportNotPublished);
        }
        let transform = output
            .try_guarded_motion_remainder()
            .map_err(E::Read)?
            .teleport_transform;
        return Err(E::TeleportPlayerNowPending(transform));
    }
    Ok(())
}
