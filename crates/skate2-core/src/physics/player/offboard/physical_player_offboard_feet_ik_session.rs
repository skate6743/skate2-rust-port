//! Live first two calls of `OffBoardFeetIK::UpdateGround` on the retained pair.
use super::offboard_bad_ground_correction::LiveBadGroundCorrectionError;
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    collision_line_numeric::HostCollisionLineNumeric,
    physical_player_remaining_constructor_owners::jumping_runtime::{
        JumpingError, JumpingInput, JumpingTuning, JumpingWorld,
    },
    skeleton_processed_contact::SkeletonProcessedContact,
};
use crate::{
    dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedSteeringError,
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
        SimulationSkeletonOffboardTrajectoryResetError,
    },
    dynamic_assembly_session::skeleton_normal::part_transform_query::SkeletonPartTransformQueryError,
    dynamic_assembly_session::skeleton_normal::postphysics_preparation::transform_point_stage,
    input_cvar_runtime::{InputCvarError, InputCvarReferences, reckoning::ReckoningCvarAdapter},
    physical_player_assembly::remaining_states::{
        OwnerMember, StateConstructorError, StateKind, StatePayload, read_floats,
    },
    physical_player_reckoning_owner::{
        PhysicalPlayerReckoningPhase, PhysicalPlayerReckoningUpdateError,
    },
    physical_player_remaining_constructor_owners::feet_ik_prefix::{
        CalculateIkTargetsError, FeetDownPrefixError, FeetIkUpdateStateError,
        OffboardFeetIkPrefixError, ProcessedFootFlagsError, try_query_feet_down_part_positions,
    },
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric,
        length_from_squared_and_estimate, normalize_from_squared_and_estimate,
    },
    reckoning_sin_cos::{HostReckoningSinCosNumeric, try_sin_cos},
    simulation_assembly_backend::constraint_session::ground_control_complete::SteeringInput,
    simulation_assembly_backend::constraint_session::offboard_feet_ik_local_targets::OffboardLocalFootTargetsError,
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_affine_interpolation::try_normalize_rotation,
    skeleton_body_attributes::{ROOT_HALF_LENGTH, ROOT_RADIUS},
    skeleton_processed_contact::grounded_runtime::GroundedProcessed,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};

#[path = "physical_player_offboard_pushing_skeleton.rs"]
mod pushing_skeleton;

#[derive(Debug)]
pub(crate) enum OffboardPostOrientationStep {
    CheckForGrabbableObject,
    UpdateJumpingPreperation,
}

#[derive(Debug)]
pub(crate) enum OffboardGroundFeetIkError {
    Pool(SimulationSkaterPoolError),
    Poisoned,
    MissingFeetIk,
    AirHeight(crate::physical_player_remaining_constructor_owners::feet_ik_prefix::OffboardAirHeightError),
    GroundFinish(
        crate::physical_player_remaining_constructor_owners::feet_ik_ground_finish::OffboardGroundFinishError,
    ),
    MissingAirAnimationInput,
    MissingAirDeckTransform,
    PendingAirSkeletonDebug,
    ForeignBinding,
    Init(OffboardFeetIkPrefixError),
    Condition(InputCvarError),
    Update(OffboardFeetIkPrefixError),
    FeetDown(FeetDownPrefixError),
    PartTransform {
        part: usize,
        error: SkeletonPartTransformQueryError,
    },
    SurfaceHeight(InputCvarError),
    ProcessedFlags(ProcessedFootFlagsError),
    UpdateState(FeetIkUpdateStateError),
    Targets(CalculateIkTargetsError),
    ApplyIkEnable(InputCvarError),
    ApplyIkBlend(InputCvarError),
    SetLocalTargets(OffboardLocalFootTargetsError),
    SetNormals(OffboardLocalFootTargetsError),
    BadGround(LiveBadGroundCorrectionError),
    Steering(SimulationConstructedSteeringError<&'static str>),
    MissingProcessedFlags8d0,
    MissingOffboardState,
    MissingProcessedOffboardTurn,
    MissingProcessedStateTimer,
    MissingAutoTurn,
    MissingStartingComVel,
    MissingOffboardReorientFlag,
    MissingSkeletonConstruction,
    MissingSkeletonOuter,
    MissingLocomotionCom,
    AnimationRootNaN,
    TrajectoryRootNaN,
    ComposedRootNaN,
    MissingPhysicsSkeletonAttributes,
    SkateRootAttribute(StateConstructorError),
    BlendCvar(InputCvarError),
    BlendNumeric(ReckoningHostNumericError),
    SkeletonPair(OffboardLocalFootTargetsError),
    MissingOffboardLocationTest,
    ForeignOffboardLocationTest,
    UnknownProcessedFlags8d4,
    Location(crate::simulation_assembly_backend::constraint_session::offboard_location_session::LocationSessionError),
    MissingOffboardNextStep,
    SetSkateRoot(OffboardLocalFootTargetsError),
    RootSinCos(std::convert::Infallible),
    Reorient(OffboardLocalFootTargetsError),
    SetComposedRoot(OffboardLocalFootTargetsError),
    NormalizeRoot(ReckoningHostNumericError),
    SetNormalizedRoot(OffboardLocalFootTargetsError),
    SetCorrectedRootTranslation(OffboardLocalFootTargetsError),
    ResetTrajectory(SimulationSkeletonOffboardTrajectoryResetError),
    HookNumeric(ReckoningHostNumericError),
    HookCvar(InputCvarError),
    GroundCvar(InputCvarError),
    GroundTranslationNaN,
    PostTranslationRootNaN,
    CallerRootNaN,
    SetGroundAdjustedRootTranslation(OffboardLocalFootTargetsError),
    SetWorldToAnimationInverse(OffboardLocalFootTargetsError),
    MissingOffboardVolumeRoot,
    ComposedVolumeRootNaN,
    PublishOffboardOrientation(
        crate::dynamic_assembly_session::skeleton_normal::general_owner::live_input::GroundedInputError,
    ),
    ApplyBoardAnimation(
        crate::dynamic_assembly_session::skeleton_normal::general_owner::known_air::KnownAirSkeletonError<
            ReckoningHostNumericError, std::convert::Infallible, ()>,
    ),
    GeneralUpdate(
        crate::simulation_assembly_backend::constraint_session::physical_grounded::live::OffboardGeneralWorldError,
    ),
    NextTrajectory(OffboardLocalFootTargetsError),
    GrindHeading(OffboardLocalFootTargetsError),
    UpdateGrindStates(
        PhysicalPlayerReckoningUpdateError<
            std::convert::Infallible,
            InputCvarError,
            ReckoningHostNumericError,
        >,
    ),
    MissingRepushDelay,
    MissingProcessedFlags8cc,
    MissingPlayerGrabSplineLink,
    ForeignPlayerGrabSplineLink,
    MissingPlayerGrabSpline,
    GrabInvalidate(crate::physical_player_grab_spline::invalidate::GrabInvalidateError),
    MissingProcessedCom,
    HookTranslationNaN,
    HookRootNaN,
    MissingJumpEffectiveAnimation,
    MissingOffboardJumpInput,
    JumpTuning(InputCvarError),
    JumpNumeric(ReckoningHostNumericError),
    MissingJumpQueries,
    ForeignJumpQueries,
    MissingJumpingTestLink,
    ForeignJumpingTestLink,
    MissingJumpingTest,
    ForeignJumpingTest,
    JumpQuery(JumpingError<ReckoningHostNumericError, InputCvarError>),
    GrabCheck(super::offboard_locomotion_state::LocomotionStateError),
    Interactable(crate::physical_player_grind_manager::view::GrindError),
}
impl std::fmt::Display for OffboardGroundFeetIkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OffBoardFeetIK UpdateGround: {self:?}")
    }
}

pub(crate) struct OffboardUpdateComplete {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) reckoning: PhysicalPlayerReckoningPhase,
}

impl PhysicalAssemblyWorld {
    /// Shared ApplyIK82dcd8e0 dispatch; both ground and locomotion callers
    /// can reach retained state 3. It must bypass the local-foot branch.
    fn try_apply_pair_offboard_feet_ik(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &InputCvarReferences,
    ) -> Result<(), OffboardGroundFeetIkError> {
        use OffboardGroundFeetIkError as E;
        if !cvars
            .try_enable_off_board_feet_ik()
            .map_err(E::ApplyIkEnable)?
        {
            return Ok(());
        }
        let state = {
            let remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
            let feet = remaining.feet_ik.as_ref().ok_or(E::MissingFeetIk)?;
            feet.state
        };
        if state == 3 {
            return self
                .try_set_pair_offboard_locomotion_foot_targets(board, skeleton, owner)
                .map_err(E::SetLocalTargets);
        }
        let blend = if state == 4 {
            1.0
        } else {
            cvars.try_ob_ik_blend_val().map_err(E::ApplyIkBlend)?
        };
        self.try_set_pair_offboard_ground_foot_targets(board, skeleton, owner, blend, cvars)
            .map_err(E::SetLocalTargets)?;
        self.try_set_pair_offboard_foot_normals(board, skeleton, owner, cvars)
            .map_err(E::SetNormals)
    }

    /// Source 0x82dd1e14..1e20. `InitUpdate` reads the prior Processed foot
    /// surfaces and current raw animated toe transforms, then the condition
    /// reads its linked threshold. `Update` copies the last good position
    /// before feet-down normal clamps and raw local animation reads. The first
    /// four physical/animated `GetPartTransform` calls before the next
    /// surface-validity and projection branches and Processed foot-fix bits.
    /// FeetIK `UpdateState` follows those flags. The reached state-2 target
    /// branch then writes left/right full vectors before ApplyIK's cvar gate.
    pub(crate) fn try_update_pair_offboard_ground_feet(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &InputCvarReferences,
    ) -> Result<(), OffboardGroundFeetIkError> {
        use OffboardGroundFeetIkError as E;
        let normal = self.skeleton(skeleton).map_err(E::Pool)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
        let feet = remaining.feet_ik.as_mut().ok_or(E::MissingFeetIk)?;
        if !feet.binding.matches(owner, &processed, skeleton) {
            return Err(E::ForeignBinding);
        }
        feet.try_init_ground_update(normal).map_err(E::Init)?;
        feet.try_condition_ground_line_results(cvars)
            .map_err(E::Condition)?;
        feet.try_begin_ground_update().map_err(E::Update)?;
        let raw_positions = feet
            .try_begin_calculate_feet_down_state(normal, cvars)
            .map_err(E::FeetDown)?;
        let positions = try_query_feet_down_part_positions(normal, raw_positions)
            .map_err(|(part, error)| E::PartTransform { part, error })?;
        feet.try_resolve_surface_feet_down(positions, cvars)
            .map_err(E::SurfaceHeight)?;
        feet.try_override_processed_foot_flags()
            .map_err(E::ProcessedFlags)?;
        feet.try_update_ground_state(normal)
            .map_err(E::UpdateState)?;
        feet.try_calculate_targets(cvars, &self.debug_overlay)
            .map_err(E::Targets)?;
        drop(remaining);
        self.try_apply_pair_offboard_feet_ik(owner, board, skeleton, cvars)?;
        // UpdateGround 0x82dd1e28..1e34: CalculateGroundCorrectionAndWipeout,
        // then CalculateHeightAboveGround stored to FeetIK+0x110.
        let trajectory = {
            let outer = self
                .skeleton_construction(skeleton)
                .map_err(E::Pool)?
                .ok_or(E::MissingSkeletonConstruction)?
                .constructor
                .outer
                .as_ref()
                .ok_or(E::MissingSkeletonOuter)?;
            let t = &outer.trajectory_transform;
            [t[12], t[13], t[14]]
        };
        {
            let normal = self.skeleton(skeleton).map_err(E::Pool)?;
            let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
            remaining
                .feet_ik
                .as_mut()
                .ok_or(E::MissingFeetIk)?
                .try_finish_ground_update(normal, trajectory, cvars)
                .map_err(E::GroundFinish)?;
        }
        Ok(())
    }

    /// OffBoard500 caller suffix. Pushing501 shares only UpdateGround above;
    /// it must not execute this stationary-state steering/root update.
    pub(crate) fn try_begin_pair_offboard_ground_feet_ik(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &InputCvarReferences,
        ground: &StaticGroundPool,
        numeric: &mut HostReckoningNumeric,
        sin_cos: &mut HostReckoningSinCosNumeric,
    ) -> Result<OffboardUpdateComplete, OffboardGroundFeetIkError> {
        use OffboardGroundFeetIkError as E;
        self.try_update_pair_offboard_ground_feet(owner, board, skeleton, cvars)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        // OffBoard.Update 0x82da03cc..03d8 loads FeetIK+0x110 into f30
        // after UpdateGround and before CalculateBadGroundCorrection.
        let height_above_ground = owner
            .remaining()
            .lock()
            .map_err(|_| E::Poisoned)?
            .feet_ik
            .as_ref()
            .ok_or(E::MissingFeetIk)?
            .height_above_ground;
        let correction = self
            .try_calculate_pair_offboard_bad_ground_correction(owner, board, skeleton, cvars)
            .map_err(E::BadGround)?;
        // OffBoard.Update 0x82da03e0..03f0 supplies exact positive zero.
        // The constructed board owns the already-reviewed steering leaf.
        self.try_update_constructed_board_steering(board, 0.0, &mut SteeringInput(&processed))
            .map_err(E::Steering)?;
        // 0x82da03f4..0410: Processed+8d0 bit 21 conditionally selects
        // fsel(-height, height, +0). Preserve the signed input lane.
        let flags = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .flags_8d0
            .ok_or(E::MissingProcessedFlags8d0)?;
        let height_above_ground = select_offboard_update_height(height_above_ground, flags);
        self.try_update_pair_offboard_location_viability(owner, ground, cvars, numeric)
            .map_err(E::Location)?;
        // OffBoard.Update 0x82da046c reads the animation-owned +0x984 turn.
        let offboard_turn = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .animation_attribute_fields()
            .offboard_turn
            .ok_or(E::MissingProcessedOffboardTurn)?;
        let selected_turn = if offboard_turn.abs() < f32::from_bits(0x3c23_d70a) {
            let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
            let Some(StatePayload::OffBoard(offboard)) = states
                .states
                .get(&StateKind::OffBoard)
                .filter(|state| state.published)
                .and_then(|state| state.value.as_ref())
            else {
                return Err(E::MissingOffboardState);
            };
            offboard.auto_turn.ok_or(E::MissingAutoTurn)? + offboard_turn
        } else {
            offboard_turn
        };
        // 0x82da04cc precedes the final turn product and blend cvar reads.
        {
            let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
            let Some(StatePayload::OffBoard(offboard)) = states
                .states
                .get_mut(&StateKind::OffBoard)
                .filter(|state| state.published)
                .and_then(|state| state.value.as_mut())
            else {
                return Err(E::MissingOffboardState);
            };
            offboard.grab_object = Some(false);
        }
        // 1.02 82b6b64c..6654: a constant 500 deg/s in one product per 60 Hz frame
        // replaces the state attribute's 135 deg/s.
        let turn = selected_turn * f32::from_bits(0x3e14_ef4d);
        if std::env::var_os("SKATE2_TRACE_OFFBOARD_TURN").is_some() {
            eprintln!(
                "offboard turn: attr={offboard_turn:?} selected={selected_turn:?} radians={turn:?} heading={:?}",
                self.skeleton(skeleton)
                    .map_err(E::Pool)?
                    .root_transforms()
                    .animation_to_world()
                    .columns()[2]
            );
        }
        let frames = cvars
            .try_frames_to_blend_out_starting_com_vel()
            .map_err(E::BlendCvar)?;
        let timer = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .state_timer
            .ok_or(E::MissingProcessedStateTimer)?;
        let do_blend = cvars.try_do_com_vel_blend_out().map_err(E::BlendCvar)?;
        let duration = (frames as f32) * f32::from_bits(0x3c88_8889);
        let com_vel_blend = if do_blend && timer < duration {
            let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
            let Some(StatePayload::OffBoard(offboard)) = states
                .states
                .get(&StateKind::OffBoard)
                .filter(|state| state.published)
                .and_then(|state| state.value.as_ref())
            else {
                return Err(E::MissingOffboardState);
            };
            let velocity = offboard.starting_com_vel.ok_or(E::MissingStartingComVel)?;
            let mut numeric = HostReckoningNumeric;
            let estimate = numeric
                .reciprocal_estimate(duration)
                .map_err(E::BlendNumeric)?;
            let reciprocal = refine_drive_reciprocal(duration, estimate);
            let mut adjust = velocity.map(|lane| (lane * (duration - timer)) * reciprocal);
            // 0x82da05bc overwrites the Y lane with positive zero after the
            // full vector product; X, Z and the typed W lane survive.
            adjust.xyz[1] = 0.0;
            adjust
        } else {
            crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4])
        };
        // 0x82da05c8..05e4 checks byte+0x74 first, then +0x73 only when
        // needed. These are the two retained OffBoard entry predicates.
        let reorient_upright = {
            let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
            let Some(StatePayload::OffBoard(offboard)) = states
                .states
                .get(&StateKind::OffBoard)
                .filter(|state| state.published)
                .and_then(|state| state.value.as_ref())
            else {
                return Err(E::MissingOffboardState);
            };
            let too_fast = offboard
                .is_moving_to_fast_entering_offboard
                .ok_or(E::MissingOffboardReorientFlag)?;
            if too_fast {
                false
            } else {
                !offboard
                    .is_not_upright_entering_offboard
                    .ok_or(E::MissingOffboardReorientFlag)?
            }
        };
        self.try_begin_pair_offboard_skeleton_update(board, skeleton, owner)
            .map_err(E::SkeletonPair)?;
        self.try_set_pair_offboard_skate_root(owner, board, skeleton, &processed, cvars)?;
        self.try_update_pair_generic_offboard_roots(
            owner,
            board,
            skeleton,
            &processed,
            cvars,
            correction,
            height_above_ground,
            turn,
            com_vel_blend,
            reorient_upright,
            None,
        )?;
        // UpdateRootTransformsOffBoard returns here. Its caller
        // Skeleton::UpdateOffBoard repeats the all-column XYZ assertion at
        // 0x82b5c45c..5c5e8 before reading animation volume row zero.
        if self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world()
            .has_nan_xyz()
        {
            return Err(E::CallerRootNaN);
        }
        // UpdateOffBoard 0x82b5c5ec..678 composes the current AnimToWorld
        // with the existing mapped animation volume row zero. It checks all
        // four XYZ columns before writing Processed orientation and bit 19.
        let mapped = self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .offboard_volume_root_row()
            .ok_or(E::MissingOffboardVolumeRoot)?;
        let root = *self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world();
        let orientation = root.compose(&mapped);
        if orientation.has_nan_xyz() {
            return Err(E::ComposedVolumeRootNaN);
        }
        let mut processed_guard = processed.lock().map_err(|_| E::Poisoned)?;
        GroundedProcessed::new(&mut processed_guard)
            .publish_offboard_orientation(orientation)
            .map_err(E::PublishOffboardOrientation)?;
        // UpdateOffBoard 0x82b5c814..824 tests Processed+8d0 bit 0x40000.
        // The clear branch skips ApplyBoardAnimation and enters GeneralUpdate.
        let flags_8d0 = processed_guard
            .physics_state_fields()
            .flags_8d0
            .ok_or(E::MissingProcessedFlags8d0)?;
        drop(processed_guard);
        if flags_8d0 & 0x0004_0000 != 0 {
            self.try_apply_pair_offboard_board_animation(
                owner, board, skeleton, &processed, numeric, sin_cos,
            )
            .map_err(E::ApplyBoardAnimation)?;
        }
        let completed_reckoning;
        let next = {
            self.try_update_pair_offboard_general(
                owner, board, skeleton, &processed, ground, cvars, numeric, sin_cos,
            )
            .map_err(E::GeneralUpdate)?;
            self.try_finish_pair_offboard_next_trajectory(board, skeleton, owner)
                .map_err(E::NextTrajectory)?;
            let heading = self
                .try_pair_offboard_reckoning_heading(board, skeleton, owner)
                .map_err(E::GrindHeading)?;
            let phase = owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_update_grind_live(
                    &processed,
                    SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
                    heading,
                    0.5,
                    true,
                    &mut ReckoningCvarAdapter(cvars),
                    numeric,
                )
                .map_err(E::UpdateGrindStates)?;
            completed_reckoning = phase;
            // OffBoard.Update 0x82da05fc..0658 rereads Processed+8d0.
            // Both set bits skip the grab gate and join jumping preparation.
            let mut next = OffboardPostOrientationStep::UpdateJumpingPreperation;
            let suffix_flags = processed
                .lock()
                .map_err(|_| E::Poisoned)?
                .physics_state_fields()
                .flags_8d0
                .ok_or(E::MissingProcessedFlags8d0)?;
            if suffix_flags & (0x1 | 0x0010_0000) == 0 {
                let delay = processed
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .pushing_delay_until_repushable()
                    .ok_or(E::MissingRepushDelay)?;
                // Native fcmpu/bgt skips only ordered > +0; NaN enters.
                if !(delay > 0.0) {
                    let flags_8cc = processed
                        .lock()
                        .map_err(|_| E::Poisoned)?
                        .physics_state_fields()
                        .flags_8cc
                        .ok_or(E::MissingProcessedFlags8cc)?;
                    // Native82da0628 reads BYTE+8cc on big-endian PPC.
                    if flags_8cc & 0x0100_0000 != 0 {
                        next = OffboardPostOrientationStep::CheckForGrabbableObject;
                    } else {
                        let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
                        let Some(StatePayload::OffBoard(offboard)) = states
                            .states
                            .get(&StateKind::OffBoard)
                            .filter(|state| state.published)
                            .and_then(|state| state.value.as_ref())
                        else {
                            return Err(E::MissingOffboardState);
                        };
                        let link = offboard
                            .player_grab_spline
                            .as_ref()
                            .and_then(Option::as_ref)
                            .ok_or(E::MissingPlayerGrabSplineLink)?;
                        if link.member != OwnerMember::PlayerGrabSpline
                            || !link
                                .owner
                                .upgrade()
                                .is_some_and(|linked| Arc::ptr_eq(&linked, owner))
                        {
                            return Err(E::ForeignPlayerGrabSplineLink);
                        }
                        drop(states);
                        let mut grab = owner.grab_spline().lock().map_err(|_| E::Poisoned)?;
                        let grab = grab.as_mut().ok_or(E::MissingPlayerGrabSpline)?;
                        grab.try_invalidate_query(self.world_line_owners().0)
                            .map_err(E::GrabInvalidate)?;
                        grab.interactable.lock().map_err(|_| E::Poisoned)?.ready = false;
                    }
                }
            }
            next
        };
        if matches!(next, OffboardPostOrientationStep::CheckForGrabbableObject) {
            let grabbing = super::offboard_locomotion_state::check_offboard_grab(
                owner,
                StateKind::OffBoard,
                cvars,
                &self.debug_overlay,
                &mut crate::physical_player_grind_manager::math::GrindNum {
                    n: numeric,
                    s: sin_cos,
                },
            )
            .map_err(E::GrabCheck)?;
            if !grabbing {
                // 82d9c620 fallback uses the original effective transform,
                // not the translated grabbable box or skeleton root.
                let transform = processed
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .ground_position_fields()
                    .effective_animation
                    .ok_or(E::MissingJumpEffectiveAnimation)?;
                let (manager, output) = {
                    let grab = owner.grab_spline().lock().map_err(|_| E::Poisoned)?;
                    let grab = grab.as_ref().ok_or(E::MissingPlayerGrabSpline)?;
                    (
                        grab.manager.clone().ok_or(E::MissingPlayerGrabSpline)?,
                        Arc::clone(&grab.interactable),
                    )
                };
                manager
                    .start_interactable_query(
                        &output,
                        transform
                            .columns()
                            .map(crate::physical_player_grind_manager::view::lanes),
                        self,
                        ground,
                        cvars,
                    )
                    .map_err(E::Interactable)?;
            }
        }
        // UpdateJumpingPreperation82da0658 follows either grab branch.
        let flags = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .flags_8d0
            .ok_or(E::MissingProcessedFlags8d0)?;
        if flags & 0x2 != 0 {
            self.try_start_pair_offboard_jumping_test(owner, &processed, cvars, ground)?;
        }
        Ok(OffboardUpdateComplete {
            owner: Arc::clone(owner),
            reckoning: completed_reckoning,
        })
    }
}

impl PhysicalAssemblyWorld {
    /// UpdateJumpingPreperation82d86d20..6f38: JumpingTestInput from the effective
    /// animation transform (+c0 X, +e0 Z, +f0 translation) and the animation
    /// OB_JumpX/OB_JumpZ lanes (+a14/+a18), then StartQuery on the linked test.
    fn try_start_pair_offboard_jumping_test(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        cvars: &InputCvarReferences,
        ground: &StaticGroundPool,
    ) -> Result<(), OffboardGroundFeetIkError> {
        use OffboardGroundFeetIkError as E;
        let mut tuning = cvars.clone();
        let mut numeric = HostCollisionLineNumeric;
        let (side_axis, forward_axis, position, jump_x, jump_z) = {
            let p = processed.lock().map_err(|_| E::Poisoned)?;
            let effective = p
                .ground_position_fields()
                .effective_animation
                .ok_or(E::MissingJumpEffectiveAnimation)?;
            let attributes = p.animation_attribute_fields();
            (
                effective.columns()[0],
                effective.columns()[2],
                effective.columns()[3],
                attributes
                    .offboard_jump_x
                    .ok_or(E::MissingOffboardJumpInput)?,
                attributes
                    .offboard_jump_z
                    .ok_or(E::MissingOffboardJumpInput)?,
            )
        };
        let maximum = tuning.max_jump_distance().map_err(E::JumpTuning)?;
        // Raw 82d86e14..e30: (Z * jumpZ - X * jumpX) * kOB_MaxJumpDistance.
        let side = side_axis.map(|v| v * jump_x);
        let forward = forward_axis.map(|v| v * jump_z);
        let jump = forward.zip(side, |f, s| f - s).map(|v| v * maximum);
        let squared = numeric.dot_vector(jump, jump).map_err(E::JumpNumeric)?;
        let estimate = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(E::JumpNumeric)?;
        let length = length_from_squared_and_estimate(squared, estimate);
        // fcmpu/ble at 82d86e80: only an ordered length above 0.1 normalizes.
        let input = if length > f32::from_bits(0x3dcc_cccd) {
            let minimum = tuning.min_jump_distance().map_err(E::JumpTuning)?;
            let estimate = numeric
                .reciprocal_sqrt_estimate(squared)
                .map_err(E::JumpNumeric)?;
            JumpingInput {
                ground: position,
                direction: normalize_from_squared_and_estimate(jump, squared, estimate),
                distance: if length < minimum { minimum } else { length },
            }
        } else {
            JumpingInput {
                ground: position,
                direction: forward_axis,
                distance: tuning.min_jump_distance().map_err(E::JumpTuning)?,
            }
        };
        {
            let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
            let Some(StatePayload::OffBoard(offboard)) = states
                .states
                .get(&StateKind::OffBoard)
                .filter(|state| state.published)
                .and_then(|state| state.value.as_ref())
            else {
                return Err(E::MissingOffboardState);
            };
            let link = offboard
                .offboard_jumping_test
                .as_ref()
                .and_then(Option::as_ref)
                .ok_or(E::MissingJumpingTestLink)?;
            if link.member != OwnerMember::OffboardJumpingTest
                || !link
                    .owner
                    .upgrade()
                    .is_some_and(|linked| Arc::ptr_eq(&linked, owner))
            {
                return Err(E::ForeignJumpingTestLink);
            }
        }
        let (queries, dmos) = self.world_line_owners();
        let queries = queries.ok_or(E::MissingJumpQueries)?;
        if !queries.belongs_to(owner.manager()) {
            return Err(E::ForeignJumpQueries);
        }
        let dmos = dmos.ok_or(E::MissingJumpQueries)?;
        let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
        let jumping = remaining.jumping.as_mut().ok_or(E::MissingJumpingTest)?;
        if !jumping.world.matches(owner) {
            return Err(E::ForeignJumpingTest);
        }
        jumping
            .start_query(
                input,
                &mut JumpingWorld {
                    queries,
                    ground,
                    dmos,
                },
                &mut tuning,
                &mut numeric,
            )
            .map_err(E::JumpQuery)
    }
}

fn select_offboard_update_height(height: f32, flags_8d0: u32) -> f32 {
    if flags_8d0 & 0x0020_0000 != 0 {
        if -height >= 0.0 { height } else { 0.0 }
    } else {
        height
    }
}

fn fsel_offboard(test: f32, nonnegative: f32, negative_or_nan: f32) -> f32 {
    if test >= 0.0 {
        nonnegative
    } else {
        negative_or_nan
    }
}

impl PhysicalAssemblyWorld {
    /// Skeleton.SetOffBoardSkateRootTransform82b4d198, shared by Ground/Air.
    pub(crate) fn try_set_pair_offboard_skate_root(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        processed: &Arc<
            std::sync::Mutex<crate::skeleton_processed_contact::SkeletonProcessedContact>,
        >,
        cvars: &InputCvarReferences,
    ) -> Result<(), OffboardGroundFeetIkError> {
        use OffboardGroundFeetIkError as E;
        // SetOffBoardSkateRootTransform 0x82b4d1d0..d21c first samples the
        // physical COM and animation-to-world Y, then its physics_skeleton
        // attribute. Missing attribute uses the native four-byte zero area.
        let normal = self.skeleton(skeleton).map_err(E::Pool)?;
        let animation_y = normal.root_transforms().animation_to_world().columns()[3].xyz[1];
        // Skeleton+29d0 = SkeletonState1940 + PhysRecord610 + COMa80.
        let mut translation = normal.physics_history().record().vector_typed(
            crate::skeleton_physics_history::lane_knowledge::RecordVectorField::CentreOfMass,
        );
        let construction = self
            .skeleton_construction(skeleton)
            .map_err(E::Pool)?
            .ok_or(E::MissingSkeletonConstruction)?;
        let attributes = construction
            .constructor
            .outer_attributes
            .as_ref()
            .and_then(|outer| outer.physics_skeleton.as_ref())
            .ok_or(E::MissingPhysicsSkeletonAttributes)?;
        let root_y =
            read_floats::<1>(attributes, 0x31bd_1d86_8668_4e85).map_err(E::SkateRootAttribute)?[0];
        translation = offboard_skate_root_position(translation, animation_y, root_y);
        let flags = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .flags_8d0
            .ok_or(E::MissingProcessedFlags8d0)?;
        if flags & 0x8 != 0 {
            let half_length =
                read_floats::<1>(attributes, ROOT_HALF_LENGTH).map_err(E::SkateRootAttribute)?[0];
            let radius =
                read_floats::<1>(attributes, ROOT_RADIUS).map_err(E::SkateRootAttribute)?[0];
            let next_y = processed
                .lock()
                .map_err(|_| E::Poisoned)?
                .prior_output()
                .offboard_next_step_position()
                .ok_or(E::MissingOffboardNextStep)?
                .xyz[1];
            let next_plus_radius = next_y + radius;
            let maximum = cvars.try_max_proxy_y_dist().map_err(E::GroundCvar)?;
            translation.xyz[1] =
                offboard_proxy_height(translation.xyz[1], next_plus_radius, half_length, maximum);
        }
        let normal = self.skeleton(skeleton).map_err(E::Pool)?;
        let columns = normal.root_transforms().animation_to_world().columns();
        let skate_root =
            SkeletonRootMatrix::from_columns([columns[2], columns[0], columns[1], translation]);
        self.try_set_pair_offboard_skate_root_transform(board, skeleton, owner, skate_root)
            .map_err(E::SetSkateRoot)?;
        Ok(())
    }
}

///82b4d1e0/214/21c: preserve physical COM X/Z/W, replace Y only.
fn offboard_skate_root_position(
    mut physical_com: SkeletonRootVector,
    animation_y: f32,
    root_offset: f32,
) -> SkeletonRootVector {
    physical_com.xyz[1] = root_offset + animation_y;
    physical_com
}

///82b4d2bc..2e0: two ordered additions and two literal fsel gates.
fn offboard_proxy_height(base: f32, next_plus_radius: f32, half: f32, maximum: f32) -> f32 {
    let upper = maximum + base;
    let desired = next_plus_radius + half;
    let lower = fsel_offboard(desired - base, desired, base);
    fsel_offboard(upper - lower, lower, upper)
}

impl PhysicalAssemblyWorld {
    /// OffBoardFeetIK.UpdateAir82dd1e50. Kept separate from Skeleton caller so
    /// the enclosing state can preserve its intervening trajectory operations.
    pub(crate) fn try_update_pair_offboard_air_feet(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &InputCvarReferences,
    ) -> Result<(), OffboardGroundFeetIkError> {
        self.try_update_pair_offboard_air_or_locomotion_feet(owner, board, skeleton, cvars, true)
    }
    pub(crate) fn try_update_pair_offboard_locomotion_feet(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &InputCvarReferences,
    ) -> Result<(), OffboardGroundFeetIkError> {
        self.try_update_pair_offboard_air_or_locomotion_feet(owner, board, skeleton, cvars, false)
    }
    fn try_update_pair_offboard_air_or_locomotion_feet(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &InputCvarReferences,
        air: bool,
    ) -> Result<(), OffboardGroundFeetIkError> {
        use OffboardGroundFeetIkError as E;
        let normal = self.skeleton(skeleton).map_err(E::Pool)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
        let feet = remaining.feet_ik.as_mut().ok_or(E::MissingFeetIk)?;
        if !feet.binding.matches(owner, &processed, skeleton) {
            return Err(E::ForeignBinding);
        }
        feet.begin_air_update();
        feet.try_init_ground_update(normal).map_err(E::Init)?;
        feet.try_begin_ground_update().map_err(E::Update)?;
        let raw_positions = feet
            .try_begin_calculate_feet_down_state(normal, cvars)
            .map_err(E::FeetDown)?;
        let positions = try_query_feet_down_part_positions(normal, raw_positions)
            .map_err(|(part, error)| E::PartTransform { part, error })?;
        feet.try_resolve_surface_feet_down(positions, cvars)
            .map_err(E::SurfaceHeight)?;
        feet.try_override_processed_foot_flags()
            .map_err(E::ProcessedFlags)?;
        feet.try_update_ground_state(normal)
            .map_err(E::UpdateState)?;
        feet.try_calculate_targets(cvars, &self.debug_overlay)
            .map_err(E::Targets)?;
        drop(remaining);
        self.try_apply_pair_offboard_feet_ik(owner, board, skeleton, cvars)?;
        if !air {
            return Ok(());
        }
        let normal = self.skeleton(skeleton).map_err(E::Pool)?;
        let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
        remaining
            .feet_ik
            .as_mut()
            .ok_or(E::MissingFeetIk)?
            .try_finish_air_height(normal, cvars)
            .map_err(E::AirHeight)
    }

    #[expect(clippy::too_many_arguments)]
    fn try_update_pair_generic_offboard_roots(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        processed: &Arc<
            std::sync::Mutex<crate::skeleton_processed_contact::SkeletonProcessedContact>,
        >,
        cvars: &InputCvarReferences,
        correction: SkeletonRootVector,
        height_above_ground: f32,
        turn: f32,
        com_vel_blend: SkeletonRootVector,
        reorient_upright: bool,
        supplied_trajectory: Option<SkeletonRootMatrix>,
    ) -> Result<OffboardRootTrace, OffboardGroundFeetIkError> {
        use OffboardGroundFeetIkError as E;
        // UpdateRootTransformsOffBoard 0x82b4492c snapshots P before the
        // source SinCos call and optional upright reorientation. Its unusual
        // yaw W lanes are shared with the grounded root update.
        let prior_translation = self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world()
            .columns()[3];
        let (sin, cos) =
            try_sin_cos(turn, &mut HostReckoningSinCosNumeric).map_err(E::RootSinCos)?;
        let yaw = SkeletonRootMatrix::heading_yaw(sin, cos);
        if reorient_upright {
            self.try_reorient_pair_offboard_skeleton(board, skeleton, owner)
                .map_err(E::Reorient)?;
        }
        // Source 0x82b449b0..44b3c checks all four AnimToWorld columns'
        // XYZ lanes before it first dereferences the constructor-owned
        // current trajectory at 0x82b44b40. Neither assertion tests W.
        if self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world()
            .has_nan_xyz()
        {
            return Err(E::AnimationRootNaN);
        }
        // Walking supplies the retained animation trajectory; Pushing supplies
        // a stack matrix containing WorldToAnim * requested world displacement
        // (82b5df64..dffc). Both enter the same native root routine82b448e8.
        let trajectory = match supplied_trajectory {
            Some(trajectory) => trajectory,
            None => {
                let construction = self
                    .skeleton_construction(skeleton)
                    .map_err(E::Pool)?
                    .ok_or(E::MissingSkeletonConstruction)?;
                SkeletonRootMatrix::known(
                    construction
                        .constructor
                        .outer
                        .as_ref()
                        .ok_or(E::MissingSkeletonOuter)?
                        .trajectory_transform,
                )
            }
        };
        if trajectory.has_nan_xyz() {
            return Err(E::TrajectoryRootNaN);
        }
        // Source 0x82b44cc8..44db0 first composes the constructor-owned
        // trajectory with yaw, then the current (possibly reoriented)
        // animation-to-world with that intermediate. 0x82b44db8..c4 writes
        // every column back to the same retained Skeleton root owner.
        let intermediate = trajectory.compose(&yaw);
        let current = *self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world();
        let composed = current.compose(&intermediate);
        self.try_set_pair_offboard_animation_to_world(board, skeleton, owner, composed)
            .map_err(E::SetComposedRoot)?;
        // Source 0x82b44dc8..44f48 repeats the XYZ-only four-column NaN
        // assertion over the just-written root. Its next call is the shared
        // reverse-axis OrthoNormalizeRotation, which writes all four returned
        // columns to the same root at 0x82b44f5c..8c.
        if self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world()
            .has_nan_xyz()
        {
            return Err(E::ComposedRootNaN);
        }
        let stored = *self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world();
        let normalized =
            try_normalize_rotation(&stored, &mut HostReckoningNumeric).map_err(E::NormalizeRoot)?;
        self.try_set_pair_offboard_animation_to_world(board, skeleton, owner, normalized)
            .map_err(E::SetNormalizedRoot)?;
        // 0x82b44f94..44ff0 samples the retained HookToHips, computes its
        // XYZ length even at zero, captures the distinct Physics_Skeleton
        // cvar target before the current-minus-prior full-vector delta, and
        // reads the cvar value between the first seed square and half-seed.
        let hook = self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .post_physics_records()
            .hook_to_hips();
        let hook_lanes = [hook.xyz[0], hook.xyz[1], hook.xyz[2], hook.w.unwrap_or(0.0)];
        let squared = HostReckoningNumeric
            .dot3(hook_lanes, hook_lanes)
            .map_err(E::HookNumeric)?;
        let seed = HostReckoningNumeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(E::HookNumeric)?;
        let max_target = cvars.try_capture_max_skeleton_disp().map_err(E::HookCvar)?;
        let exactly_zero = squared == 0.0;
        let current_translation = self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world()
            .columns()[3];
        let delta = current_translation.zip(prior_translation, |current, prior| current - prior);
        let square_0 = seed * seed;
        let max_displacement = max_target.try_value().map_err(E::HookCvar)?;
        // This native branch orders each half-seed before its fused residual,
        // unlike the shared refiner's ordinary-value-equivalent ordering.
        let half_0 = seed * 0.5;
        let residual_0 = (-squared).mul_add(square_0, 1.0);
        let refined_0 = half_0.mul_add(residual_0, seed);
        let square_1 = refined_0 * refined_0;
        let half_1 = refined_0 * 0.5;
        let residual_1 = (-squared).mul_add(square_1, 1.0);
        let refined_1 = half_1.mul_add(residual_1, refined_0);
        let length = squared * refined_1;
        let length = if exactly_zero { 0.0 } else { length };
        if length > max_displacement {
            if let Some(translation) =
                try_offboard_hook_translation(hook, delta, prior_translation, length, || {
                    cvars.try_collision_push_fraction()
                })?
            {
                self.try_set_pair_offboard_animation_translation(
                    board,
                    skeleton,
                    owner,
                    translation,
                )
                .map_err(E::SetCorrectedRootTranslation)?;
                // Native checks every column after committing translation.
                if self
                    .skeleton(skeleton)
                    .map_err(E::Pool)?
                    .root_transforms()
                    .animation_to_world()
                    .has_nan_xyz()
                {
                    return Err(E::HookRootNaN);
                }
            }
        }
        // 0x82b452f4..45330 reads kUpdateTime_55, multiplies all four
        // lanes of the retained second Update argument, then tests its XYZ
        // dot3 against Math::VECFLOAT_SMALL with a strict ordered compare.
        let step = com_vel_blend.map(|lane| lane * f32::from_bits(0x3c88_8889));
        let step_lanes = [step.xyz[0], step.xyz[1], step.xyz[2], step.w.unwrap_or(0.0)];
        let step_squared = HostReckoningNumeric
            .dot3(step_lanes, step_lanes)
            .map_err(E::BlendNumeric)?;
        let (step_length, projection, corrected) = if step_squared > f32::from_bits(0x3780_0000) {
            // 0x82b45344..4541c uses the current root, actual SkeletonState
            // animation COM, and retained Processed COM. The public VMADD
            // stage is shared with the existing point-transform kernel.
            let seed = HostReckoningNumeric
                .reciprocal_sqrt_estimate(step_squared)
                .map_err(E::BlendNumeric)?;
            let exactly_zero = step_squared == 0.0;
            let root = *self
                .skeleton(skeleton)
                .map_err(E::Pool)?
                .root_transforms()
                .animation_to_world();
            let animation_com = self
                .skeleton(skeleton)
                .map_err(E::Pool)?
                .current_animation()
                .record()
                .centre_of_mass();
            let square_0 = seed * seed;
            let half_0 = seed * 0.5;
            let processed_com = processed
                .lock()
                .map_err(|_| E::Poisoned)?
                .prior_output()
                .centre_of_mass()
                .ok_or(E::MissingProcessedCom)?
                .value;
            let columns = root.columns();
            let first = transform_point_stage(columns[3], columns[0], animation_com[0]);
            let residual_0 = (-step_squared).mul_add(square_0, 1.0);
            let second = transform_point_stage(first, columns[1], animation_com[1]);
            let refined_0 = half_0.mul_add(residual_0, seed);
            let transformed_com = transform_point_stage(second, columns[2], animation_com[2]);
            let square_1 = refined_0 * refined_0;
            let half_1 = refined_0 * 0.5;
            let com_displacement = transformed_com.zip(processed_com, |world, prior| world - prior);
            let residual_1 = (-step_squared).mul_add(square_1, 1.0);
            let refined_1 = half_1.mul_add(residual_1, refined_0);
            let step_length = if exactly_zero {
                0.0
            } else {
                step_squared * refined_1
            };
            let reciprocal_seed = HostReckoningNumeric
                .reciprocal_estimate(step_length)
                .map_err(E::BlendNumeric)?;
            let reciprocal = refine_drive_reciprocal(step_length, reciprocal_seed);
            let unit_step = step.map(|lane| lane * reciprocal);
            let unit_lanes = [
                unit_step.xyz[0],
                unit_step.xyz[1],
                unit_step.xyz[2],
                unit_step.w.unwrap_or(0.0),
            ];
            let displacement_lanes = [
                com_displacement.xyz[0],
                com_displacement.xyz[1],
                com_displacement.xyz[2],
                com_displacement.w.unwrap_or(0.0),
            ];
            let projection = HostReckoningNumeric
                .dot3(unit_lanes, displacement_lanes)
                .map_err(E::BlendNumeric)?;
            let corrected = step_length > projection;
            if corrected {
                let remaining = step_length - projection;
                let prior = root.columns()[3];
                let translation =
                    unit_step.zip(prior, |unit, prior| unit.mul_add(remaining, prior));
                self.try_set_pair_offboard_animation_translation(
                    board,
                    skeleton,
                    owner,
                    translation,
                )
                .map_err(E::SetCorrectedRootTranslation)?;
            }
            (step_length, projection, corrected)
        } else {
            (0.0, 0.0, false)
        };
        // Both COM-step paths join at 0x82b45420. The source writes only
        // current trajectory and pose index zero (global then local).
        self.try_reset_offboard_trajectory_and_root_poses(skeleton, owner)
            .map_err(E::ResetTrajectory)?;
        // 0x82b454c0/454c4 captures two distinct OffBoard cvar targets
        // before reloading the first Update vector and current root P.
        let max_target = cvars
            .try_capture_ob_ground_max_delta()
            .map_err(E::GroundCvar)?;
        let blend_target = cvars
            .try_capture_ob_ground_prop_blend()
            .map_err(E::GroundCvar)?;
        let adjustment = correction;
        let current_translation = self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world()
            .columns()[3];
        let max_delta = max_target.try_value().map_err(E::GroundCvar)?;
        let neg_max = -max_delta;
        let prop_blend = blend_target.try_value().map_err(E::GroundCvar)?;
        let scaled = prop_blend * height_above_ground;
        let neg_scaled = -scaled;
        let lower_test = neg_max - neg_scaled;
        let lower = fsel_offboard(lower_test, neg_max, neg_scaled);
        let upper_test = max_delta - lower;
        let ground_vertical = fsel_offboard(upper_test, lower, max_delta);
        let scratch = crate::skeleton_root_transforms::SkeletonRootVector::known([
            0.0,
            ground_vertical,
            0.0,
            0.0,
        ]);
        let adjusted = scratch.zip(adjustment, |scratch, adjustment| scratch + adjustment);
        let translation = adjusted.zip(current_translation, |adjusted, current| adjusted + current);
        if translation.xyz.iter().any(|lane| lane.is_nan()) {
            return Err(E::GroundTranslationNaN);
        }
        self.try_set_pair_offboard_animation_translation(board, skeleton, owner, translation)
            .map_err(E::SetGroundAdjustedRootTranslation)?;
        // 0x82b4557c..456fc checks every AnimToWorld column's XYZ after
        // the P store. W is not an assertion input.
        if self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world()
            .has_nan_xyz()
        {
            return Err(E::PostTranslationRootNaN);
        }
        let root = *self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world();
        let inverse = root.inverse();
        self.try_set_pair_offboard_world_to_animation_inverse(board, skeleton, owner, inverse)
            .map_err(E::SetWorldToAnimationInverse)?;
        Ok(OffboardRootTrace {
            prior_translation,
            normalized,
            length,
            step,
            step_squared,
            step_length,
            projection,
            corrected,
            ground_vertical,
            translation,
        })
    }

    /// Root child82b441b0 used when Processed8cc bit0x40 selects locomotion.
    /// comPosAnim and turnoffset in its native signature are unused.
    pub(crate) fn try_update_pair_offboard_locomotion_roots(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        numeric: &mut HostReckoningNumeric,
    ) -> Result<(), OffboardGroundFeetIkError> {
        use OffboardGroundFeetIkError as E;
        self.try_reorient_pair_offboard_skeleton(board, skeleton, owner)
            .map_err(E::Reorient)?;
        let trajectory = {
            let construction = self
                .skeleton_construction(skeleton)
                .map_err(E::Pool)?
                .ok_or(E::MissingSkeletonConstruction)?;
            let outer = construction
                .constructor
                .outer
                .as_ref()
                .ok_or(E::MissingSkeletonOuter)?;
            SkeletonRootMatrix::known(outer.trajectory_transform)
        };
        let current = *self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world();
        let composed = current.compose(&trajectory);
        self.try_set_pair_offboard_animation_to_world(board, skeleton, owner, composed)
            .map_err(E::SetComposedRoot)?;
        if composed.has_nan_xyz() {
            return Err(E::ComposedRootNaN);
        }
        let normalized = try_normalize_rotation(&composed, numeric).map_err(E::NormalizeRoot)?;
        self.try_set_pair_offboard_animation_to_world(board, skeleton, owner, normalized)
            .map_err(E::SetNormalizedRoot)?;
        if normalized.has_nan_xyz() {
            return Err(E::PostTranslationRootNaN);
        }
        let centre = self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .current_animation()
            .record()
            .centre_of_mass();
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let target = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .offboard_locomotion_com()
            .ok_or(E::MissingLocomotionCom)?;
        let position = offboard_locomotion_com_translation(&normalized, centre, target);
        if position.xyz.iter().any(|v| v.is_nan()) {
            return Err(E::GroundTranslationNaN);
        }
        self.try_set_pair_offboard_animation_translation(board, skeleton, owner, position)
            .map_err(E::SetCorrectedRootTranslation)?;
        let updated = *self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world();
        if updated.has_nan_xyz() {
            return Err(E::PostTranslationRootNaN);
        }
        self.try_reset_offboard_trajectory_and_root_poses(skeleton, owner)
            .map_err(E::ResetTrajectory)?;
        self.try_set_pair_offboard_world_to_animation_inverse(
            board,
            skeleton,
            owner,
            updated.inverse(),
        )
        .map_err(E::SetWorldToAnimationInverse)
    }

    /// Skeleton.UpdateOffBoardLocomotion82b5c920, retaining both source root routes.
    pub(crate) fn try_update_pair_offboard_locomotion_skeleton(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        reorient: bool,
        cvars: &InputCvarReferences,
        ground: &StaticGroundPool,
        numeric: &mut HostReckoningNumeric,
        sin_cos: &mut HostReckoningSinCosNumeric,
    ) -> Result<PhysicalPlayerReckoningPhase, OffboardGroundFeetIkError> {
        use OffboardGroundFeetIkError as E;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        self.try_begin_pair_offboard_skeleton_update(board, skeleton, owner)
            .map_err(E::SkeletonPair)?;
        self.try_set_pair_offboard_skate_root(owner, board, skeleton, &processed, cvars)?;
        let flags = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .flags_8cc
            .ok_or(E::MissingProcessedFlags8cc)?;
        if flags & 0x40 != 0 {
            self.try_update_pair_offboard_locomotion_roots(owner, board, skeleton, numeric)?;
        } else {
            let mut velocity = processed
                .lock()
                .map_err(|_| E::Poisoned)?
                .prior_output()
                .centre_of_mass_velocity()
                .ok_or(E::MissingProcessedCom)?
                .value;
            velocity.xyz[1] = 0.0;
            let scalar = cvars
                .try_skeleton_speed_scalar_on_enter_locomotion()
                .map_err(E::GroundCvar)?;
            let velocity = velocity.map(|v| v * scalar);
            self.try_update_pair_generic_offboard_roots(
                owner,
                board,
                skeleton,
                &processed,
                cvars,
                SkeletonRootVector::known([0.0; 4]),
                0.0,
                0.0,
                velocity,
                reorient,
                None,
            )?;
        }
        let root = *self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world();
        if root.has_nan_xyz() {
            return Err(E::CallerRootNaN);
        }
        let mapped = self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .offboard_volume_root_row()
            .ok_or(E::MissingOffboardVolumeRoot)?;
        let orientation = root.compose(&mapped);
        if orientation.has_nan_xyz() {
            return Err(E::ComposedVolumeRootNaN);
        }
        let flags = {
            let mut p = processed.lock().map_err(|_| E::Poisoned)?;
            GroundedProcessed::new(&mut p)
                .publish_offboard_orientation(orientation)
                .map_err(E::PublishOffboardOrientation)?;
            p.physics_state_fields()
                .flags_8d0
                .ok_or(E::MissingProcessedFlags8d0)?
        };
        if flags & 0x40000 != 0 {
            self.try_apply_pair_offboard_board_animation(
                owner, board, skeleton, &processed, numeric, sin_cos,
            )
            .map_err(E::ApplyBoardAnimation)?;
        }
        self.try_update_pair_offboard_general(
            owner, board, skeleton, &processed, ground, cvars, numeric, sin_cos,
        )
        .map_err(E::GeneralUpdate)?;
        // Locomotion reads Skeleton+2da0; Air instead uses the deck heading.
        let heading = self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world()
            .columns()[2];
        let phase = owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_update_grind_live(
                &processed,
                SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
                heading,
                0.5,
                true,
                &mut ReckoningCvarAdapter(cvars),
                numeric,
            )
            .map_err(E::UpdateGrindStates)?;
        self.try_finish_pair_offboard_next_trajectory(board, skeleton, owner)
            .map_err(E::NextTrajectory)?;
        Ok(phase)
    }

    /// Skeleton.UpdateOffBoardAir82b5cee8 on the existing attached pair.
    #[expect(clippy::too_many_arguments)]
    pub(crate) fn try_update_pair_offboard_air_skeleton(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        world_com: SkeletonRootVector,
        turn: f32,
        reorient: bool,
        cvars: &InputCvarReferences,
        ground: &StaticGroundPool,
        numeric: &mut HostReckoningNumeric,
        sin_cos: &mut HostReckoningSinCosNumeric,
    ) -> Result<PhysicalPlayerReckoningPhase, OffboardGroundFeetIkError> {
        use OffboardGroundFeetIkError as E;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        self.try_begin_pair_offboard_skeleton_update(board, skeleton, owner)
            .map_err(E::SkeletonPair)?;
        self.try_set_pair_offboard_skate_root(owner, board, skeleton, &processed, cvars)?;
        // Native SinCos occurs before current trajectory/global/local pose resets.
        let (sin, cos) = try_sin_cos(turn, sin_cos).map_err(E::RootSinCos)?;
        self.try_reset_offboard_trajectory_and_root_poses(skeleton, owner)
            .map_err(E::ResetTrajectory)?;
        let (revert, frames) = {
            let construction = self
                .skeleton_construction(skeleton)
                .map_err(E::Pool)?
                .ok_or(E::MissingSkeletonConstruction)?;
            let binding = construction
                .constructor
                .bindings
                .as_ref()
                .ok_or(E::MissingAirAnimationInput)?;
            let input = binding
                .animation_input
                .as_ref()
                .ok_or(E::MissingAirAnimationInput)?;
            let input = input.lock().map_err(|_| E::Poisoned)?;
            (
                input.locomotion_flags & 0x1000_0000 != 0,
                input.air_dismount_revert_frames,
            )
        };
        let yaw = if revert {
            let flags = processed
                .lock()
                .map_err(|_| E::Poisoned)?
                .physics_state_fields()
                .flags_8cc
                .ok_or(E::MissingProcessedFlags8cc)?;
            let angle = offboard_air_revert_angle(frames, flags & 8 != 0);
            let (sin, cos) = try_sin_cos(angle, sin_cos).map_err(E::RootSinCos)?;
            SkeletonRootMatrix::heading_yaw(sin, cos)
        } else {
            SkeletonRootMatrix::heading_yaw(sin, cos)
        };
        let original = *self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world();
        let composed = original.compose(&yaw);
        let composed = if revert {
            try_normalize_rotation(&composed, numeric).map_err(E::NormalizeRoot)?
        } else {
            composed
        };
        self.try_set_pair_offboard_animation_to_world(board, skeleton, owner, composed)
            .map_err(E::SetComposedRoot)?;
        if reorient {
            self.try_reorient_pair_offboard_skeleton(board, skeleton, owner)
                .map_err(E::Reorient)?;
        }
        let normal = self.skeleton(skeleton).map_err(E::Pool)?;
        let centre = normal.current_animation().record().centre_of_mass();
        let mut transform = *normal.root_transforms().animation_to_world();
        let translation = offboard_air_com_translation(&transform, centre, world_com);
        transform.set_column(3, translation);
        self.try_set_pair_offboard_animation_to_world(board, skeleton, owner, transform)
            .map_err(E::SetComposedRoot)?;
        let normalized = try_normalize_rotation(&transform, numeric).map_err(E::NormalizeRoot)?;
        self.try_set_pair_offboard_animation_to_world(board, skeleton, owner, normalized)
            .map_err(E::SetNormalizedRoot)?;
        self.try_set_pair_offboard_world_to_animation_inverse(
            board,
            skeleton,
            owner,
            normalized.inverse(),
        )
        .map_err(E::SetWorldToAnimationInverse)?;
        if cvars
            .try_draw_offboard_air_skeleton()
            .map_err(E::GroundCvar)?
        {
            return Err(E::PendingAirSkeletonDebug);
        }
        if normalized.has_nan_xyz() {
            return Err(E::CallerRootNaN);
        }
        let mapped = self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .offboard_volume_root_row()
            .ok_or(E::MissingOffboardVolumeRoot)?;
        let orientation = normalized.compose(&mapped);
        if orientation.has_nan_xyz() {
            return Err(E::ComposedVolumeRootNaN);
        }
        let flags = {
            let mut p = processed.lock().map_err(|_| E::Poisoned)?;
            GroundedProcessed::new(&mut p)
                .publish_offboard_orientation(orientation)
                .map_err(E::PublishOffboardOrientation)?;
            p.physics_state_fields()
                .flags_8d0
                .ok_or(E::MissingProcessedFlags8d0)?
        };
        if flags & 0x40000 != 0 {
            self.try_apply_pair_offboard_board_animation(
                owner, board, skeleton, &processed, numeric, sin_cos,
            )
            .map_err(E::ApplyBoardAnimation)?;
        }
        self.try_update_pair_offboard_general(
            owner, board, skeleton, &processed, ground, cvars, numeric, sin_cos,
        )
        .map_err(E::GeneralUpdate)?;
        let heading = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .deck_transform()
            .ok_or(E::MissingAirDeckTransform)?
            .columns()[2];
        let phase = owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_update_grind_live(
                &processed,
                SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
                heading,
                0.5,
                true,
                &mut ReckoningCvarAdapter(cvars),
                numeric,
            )
            .map_err(E::UpdateGrindStates)?;
        self.try_finish_pair_offboard_next_trajectory(board, skeleton, owner)
            .map_err(E::NextTrajectory)?;
        Ok(phase)
    }
}

fn offboard_air_revert_angle(frames: u32, mirrored: bool) -> f32 {
    let angle = f32::from_bits(0x4049_0fdb) / frames as f32;
    if mirrored { angle } else { -angle }
}
fn offboard_air_com_translation(
    transform: &SkeletonRootMatrix,
    centre: [f32; 4],
    target: SkeletonRootVector,
) -> SkeletonRootVector {
    let axes = transform.columns();
    let first = axes[0].map(|v| v * centre[0]);
    let second = axes[1].zip(first, |v, old| v.mul_add(centre[1], old));
    let rotated = axes[2].zip(second, |v, old| v.mul_add(centre[2], old));
    target.zip(rotated, |a, b| a - b)
}

#[cfg(test)]
mod height_tests {
    use super::{fsel_offboard, select_offboard_update_height};

    #[test]
    fn mounting_bit_selects_nonpositive_height_before_viability() {
        assert_eq!(select_offboard_update_height(1.5, 0), 1.5);
        assert_eq!(select_offboard_update_height(1.5, 0x0020_0000), 0.0);
        assert_eq!(select_offboard_update_height(-1.5, 0x0020_0000), -1.5);
    }

    #[test]
    fn ground_delta_fsel_preserves_negative_zero_and_nan_fallback() {
        assert_eq!(
            fsel_offboard(-0.0, -0.0, 1.0).to_bits(),
            (-0.0_f32).to_bits()
        );
        assert_eq!(fsel_offboard(f32::NAN, -1.0, 1.0), 1.0);
    }
}

#[cfg(test)]
mod offboard_proxy_tests {
    use super::*;
    #[test]
    fn proxy_uses_physical_com_xz_but_animation_y_and_preserves_unknown_w() {
        let physical = SkeletonRootVector {
            xyz: [11.0, 22.0, 33.0],
            w: None,
        };
        let result = offboard_skate_root_position(physical, 44.0, 0.5);
        assert_eq!(result.xyz, [11.0, 44.5, 33.0]);
        assert_eq!(result.w, None);
    }
    #[test]
    fn next_step_proxy_clamps_with_native_unordered_fsel() {
        assert_eq!(offboard_proxy_height(2.0, 1.0, 0.5, 0.3), 2.0);
        assert_eq!(offboard_proxy_height(2.0, 2.0, 0.2, 0.3), 2.2);
        assert_eq!(offboard_proxy_height(2.0, 3.0, 0.2, 0.3), 2.3);
        assert_eq!(offboard_proxy_height(2.0, f32::NAN, 0.2, 0.3), 2.0);
        assert!(offboard_proxy_height(2.0, 2.0, 0.2, f32::NAN).is_nan());
    }
}

#[cfg(test)]
mod offboard_air_root_tests {
    use super::*;
    #[test]
    fn revert_preserves_source_sign_and_zero_division() {
        assert_eq!(
            offboard_air_revert_angle(2, true),
            f32::from_bits(0x4049_0fdb) / 2.0
        );
        assert_eq!(
            offboard_air_revert_angle(2, false),
            -offboard_air_revert_angle(2, true)
        );
        assert_eq!(offboard_air_revert_angle(0, true), f32::INFINITY);
        assert_eq!(offboard_air_revert_angle(0, false), f32::NEG_INFINITY);
    }
    #[test]
    fn com_placement_ignores_old_translation_and_keeps_unknown_lanes() {
        let mut transform =
            SkeletonRootMatrix::known(crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM);
        transform.set_column(3, SkeletonRootVector::known([100.0; 4]));
        let target = SkeletonRootVector {
            xyz: [10.0, 20.0, 30.0],
            w: None,
        };
        let actual = offboard_air_com_translation(&transform, [1.0, 2.0, 3.0, 99.0], target);
        assert_eq!(actual.xyz, [9.0, 18.0, 27.0]);
        assert_eq!(actual.w, None);
    }
}

///82b445d8..600 deliberately retains both translated terms; cancellation is
/// source-visible floating-point arithmetic, not simplified target-rotation.
fn offboard_locomotion_com_translation(
    matrix: &SkeletonRootMatrix,
    centre: [f32; 4],
    target: SkeletonRootVector,
) -> SkeletonRootVector {
    let axes = matrix.columns();
    let point = axes[0].zip(axes[3], |axis, p| axis.mul_add(centre[0], p));
    let point = axes[1].zip(point, |axis, p| axis.mul_add(centre[1], p));
    let point = axes[2].zip(point, |axis, p| axis.mul_add(centre[2], p));
    axes[3]
        .zip(point, |p, world| p - world)
        .zip(target, |delta, com| delta + com)
}

#[cfg(test)]
mod locomotion_root_tests {
    use super::*;
    #[test]
    fn locomotion_com_preserves_native_translation_cancellation() {
        let mut m = SkeletonRootMatrix::known(crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM);
        m.set_column(3, SkeletonRootVector::known([1.0e20, 0.0, 0.0, 0.0]));
        let p = offboard_locomotion_com_translation(
            &m,
            [1.0, 0.0, 0.0, 0.0],
            SkeletonRootVector::known([10.0, 2.0, 3.0, 0.0]),
        );
        assert_eq!(p.xyz, [10.0, 2.0, 3.0]); // simplifying COM minus rotated centre would give9
    }
    #[test]
    fn locomotion_com_keeps_unknown_translation_w() {
        let mut m = SkeletonRootMatrix::known(crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM);
        m.set_column(
            3,
            SkeletonRootVector {
                xyz: [4.0, 5.0, 6.0],
                w: None,
            },
        );
        let p = offboard_locomotion_com_translation(
            &m,
            [1.0, 2.0, 3.0, 0.0],
            SkeletonRootVector::known([10.0, 20.0, 30.0, 0.0]),
        );
        assert_eq!(p.xyz, [9.0, 18.0, 27.0]);
        assert_eq!(p.w, None);
    }
}

struct OffboardRootTrace {
    prior_translation: SkeletonRootVector,
    normalized: SkeletonRootMatrix,
    length: f32,
    step: SkeletonRootVector,
    step_squared: f32,
    step_length: f32,
    projection: f32,
    corrected: bool,
    ground_vertical: f32,
    translation: SkeletonRootVector,
}

/// UpdateRootTransformsOffBoard 0x82b45010..45168, after length > max displacement.
/// The cvar is sampled only after both numeric operations and the projection gate.
fn try_offboard_hook_translation(
    hook: SkeletonRootVector,
    delta: SkeletonRootVector,
    prior: SkeletonRootVector,
    length: f32,
    fraction: impl FnOnce() -> Result<f32, InputCvarError>,
) -> Result<Option<SkeletonRootVector>, OffboardGroundFeetIkError> {
    use OffboardGroundFeetIkError as E;
    let seed = HostReckoningNumeric
        .reciprocal_estimate(length)
        .map_err(E::HookNumeric)?;
    // vnmsubfp negates the completed FMA, including its zero sign.
    let residual = -seed.mul_add(length, -1.0);
    let refined = seed.mul_add(residual, seed);
    let residual = -refined.mul_add(length, -1.0);
    let reciprocal = refined.mul_add(residual, refined);
    let unit = hook.map(|lane| reciprocal * lane);
    let projection = HostReckoningNumeric
        .dot_vector(unit, delta)
        .map_err(E::HookNumeric)?;
    if !(length > projection) {
        return Ok(None);
    }
    // Two separate native ordered comparisons choose these operands.
    let backward = if 0.0 > projection { -projection } else { 0.0 };
    let remaining = if 0.0 > projection {
        length
    } else {
        length - projection
    };
    let fraction = fraction().map_err(E::HookCvar)?;
    let distance = remaining.mul_add(fraction, backward);
    let movement = unit.zip(delta, |unit, delta| unit.mul_add(distance, delta));
    let translation = prior.zip(movement, |prior, movement| prior + movement);
    if translation.xyz.iter().any(|lane| lane.is_nan()) {
        return Err(E::HookTranslationNaN);
    }
    Ok(Some(translation))
}

#[cfg(test)]
mod hook_correction_tests {
    use super::*;
    fn vector(x: f32, y: f32, z: f32) -> SkeletonRootVector {
        SkeletonRootVector {
            xyz: [x, y, z],
            w: None,
        }
    }
    #[test]
    fn forward_and_backward_motion_use_distinct_push_operands() {
        let hook = vector(0.0, 2.0, 0.0);
        let prior = vector(4.0, 5.0, 6.0);
        let forward =
            try_offboard_hook_translation(hook, vector(3.0, 1.0, 0.0), prior, 2.0, || Ok(0.25))
                .unwrap()
                .unwrap();
        assert_eq!(forward.xyz, [7.0, 6.25, 6.0]);
        assert_eq!(forward.w, None);
        let backward =
            try_offboard_hook_translation(hook, vector(3.0, -1.0, 0.0), prior, 2.0, || Ok(0.25))
                .unwrap()
                .unwrap();
        assert_eq!(backward.xyz, [7.0, 5.5, 6.0]);
        assert_eq!(backward.w, None);
    }
    #[test]
    fn sufficient_or_unordered_projection_skips_live_fraction() {
        for y in [2.0, 3.0, f32::NAN] {
            let result = try_offboard_hook_translation(
                vector(0.0, 2.0, 0.0),
                vector(0.0, y, 0.0),
                vector(0.0, 0.0, 0.0),
                2.0,
                || panic!("fraction sampled after rejected projection"),
            );
            assert!(result.unwrap().is_none());
        }
    }
    #[test]
    fn nan_fraction_fails_before_root_publication() {
        assert!(matches!(
            try_offboard_hook_translation(
                vector(0.0, 2.0, 0.0),
                vector(0.0, 0.0, 0.0),
                vector(0.0, 0.0, 0.0),
                2.0,
                || Ok(f32::NAN)
            ),
            Err(OffboardGroundFeetIkError::HookTranslationNaN)
        ));
    }
}
