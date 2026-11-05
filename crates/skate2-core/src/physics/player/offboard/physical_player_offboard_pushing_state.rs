//! PhysState_OffBoard_Pushing501 lifecycle on the existing player pair.
//! Entry82da07b8, Exit82d87760, UpdatePostPhysics82d87e38.
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle as Handle, wipeout::WipeoutSkeletonOperation,
    },
    input_cvar_runtime::{
        InputCvarReferences,
        pushing::{PushingBool, PushingFloat},
    },
    living_world_grind_services::grab_provider::GrabBoxQuery,
    physical_player_assembly::remaining_states::{
        OffBoardPushing, OwnerMember, StateKind, StatePayload, read_floats,
    },
    physical_player_grab_spline::manager::GrabMessageKind,
    physical_player_grind_manager::{
        math::{GrindNum, ZERO, fsel, neg},
        view::{GrindError, error, lanes, need},
    },
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric},
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    simulation_assembly_backend::constraint_session::physical_skeleton_controller::SkeletonCollisionTransition,
    skeleton_processed_contact::SkeletonProcessedContact,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};

fn state<T>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    f: impl FnOnce(&mut OffBoardPushing) -> Result<T, GrindError>,
) -> Result<T, GrindError> {
    let mut states = owner.remaining_states().lock().map_err(error)?;
    let value = need(
        states
            .states
            .get_mut(&StateKind::OffBoardPushing)
            .filter(|s| s.published)
            .and_then(|s| s.value.as_mut()),
        "constructed Pushing501",
    )?;
    let StatePayload::OffBoardPushing(s) = value else {
        return Err(error("wrong Pushing payload"));
    };
    f(s)
}

fn processed(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
) -> Result<Arc<Mutex<SkeletonProcessedContact>>, GrindError> {
    Ok(Arc::clone(
        owner.reckoning().lock().map_err(error)?.processed(),
    ))
}

impl PhysicalAssemblyWorld {
    /// Update82d9cff8. Query validation, retained spline geometry, movement
    /// forces and actual Skeleton calls remain one ordered state update.
    pub(crate) fn try_update_pair_offboard_pushing(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        self.validate_pushing_state(owner, board, skeleton)?;
        let p = processed(owner)?;
        let hook = need(
            p.lock().map_err(error)?.process_data_output().hook_to_hips,
            "HookToHips2e0",
        )?;
        state(owner, |s| {
            s.try_consider_emergency_exit(lanes(hook), refs, num)
        })?;
        let flags = p.lock().map_err(error)?.physics_state_fields().flags_8d4;
        if flags.1 & 0x1000_0000 == 0 {
            return Err(error("unknown partial ragdoll8d4 bit28"));
        }
        state(owner, |s| {
            s.time_in_partial_ragdoll = Some(if flags.0 & 0x1000_0000 != 0 {
                need(s.time_in_partial_ragdoll, "Pushing ragdoll time")?
                    + f32::from_bits(0x3c88_8889)
            } else {
                0.0
            });
            Ok(())
        })?;
        self.try_update_pair_offboard_ground_feet(owner, board, skeleton, refs)
            .map_err(error)?;
        self.try_disable_pair_grab_collisions(owner, board, skeleton)?;
        let bounds = self.try_with_offboard_pair(board, skeleton, owner, |normal| -> Result<_, GrindError> {
                let effective = need(p.lock().map_err(error)?.ground_position_fields().effective_animation,
                    "Pushing effective animation")?.columns().map(lanes);
                let mut raw = |part| Ok(lanes(need(normal.anim_part_world_raw_position(part), "Pushing raw animation part")?));
                let bounds = state(owner, |s| s.try_generate_grabbable_box(effective, &mut raw, num))?;
                let holding = refs.try_pushing_bool(PushingBool::kStayInState).map_err(error)?
                    || need(p.lock().map_err(error)?.physics_state_fields().flags_8cc, "processed flags8cc")? & 0x0100_0000 != 0;
                let primary = {
                    let p = p.lock().map_err(error)?;
                    if need(p.physics_state_fields().flags_8d0, "processed flags8d0")? & 0x0100_0000 != 0 {
                        let grab = need(p.grab_spline(false), "primary grab spline")?;
                        grab.has_spline_and_body().then(|| grab.clone())
                    } else { None }
                };
                let still_grabbable = if primary.is_some() {
                    state(owner, |s| s.try_is_spline_grabbable(
                        need(s.grab_spline.as_ref(), "Pushing retained spline")?, &bounds, true, num))?
                } else { false };
                let hook = need(p.lock().map_err(error)?.process_data_output().hook_to_hips, "HookToHips2e0")?;
                state(owner, |s| s.try_calculate_collision_response(lanes(hook), refs, num))?;
                let continue_grabbing = holding && state(owner, |s| {
                    let limit = refs.try_pushing_float(PushingFloat::kOBP_MaxTimePartialRagdoll).map_err(error)?;
                    Ok(!(need(s.time_in_partial_ragdoll, "Pushing ragdoll time")? > limit)
                        && !need(s.emergency_exit, "Pushing emergency exit")?)
                })?;
                if !continue_grabbing {
                    state(owner, |s| { s.grab_object = Some(false); Ok(()) })?;
                } else {
                    let mut grab_guard = owner.grab_spline().lock().map_err(error)?;
                    let grab = need(grab_guard.as_mut(), "PlayerGrabSpline")?;
                    if primary.is_some() && still_grabbable {
                        state(owner, |s| {
                            s.grab_spline = primary;
                            s.try_update_spline(false, grab, &mut raw, effective[0], num)?;
                            s.try_compute_target_hand_ik(&mut raw, effective[0], num)?;
                            s.try_update_target_hand_ik_blend(num)?;
                            s.grab_object = Some(true);
                            Ok(())
                        })?;
                    } else {
                        let (reach, excluded) = state(owner, |s| {
                            s.time_grabbing_this_spline = Some(0.0);
                            Ok((need(s.reach_base, "Pushing reach base")?, need(s.grab_spline_desc, "Pushing descriptor")?))
                        })?;
                        let selected = if grab.query_is_ready() {
                            grab.validated.best_spline(reach, Some((excluded.object_type, excluded.object_id)), num)?
                        } else { None };
                        let accepted = if let Some(candidate) = selected.as_ref() {
                            state(owner, |s| Ok(s.try_is_spline_grabbable(candidate, &bounds, false, num)?
                                // Raw82d9d218/224 checks the OLD retained spline here.
                                && need(s.grab_spline.as_ref(), "Pushing old spline")?.has_spline_and_body()))?
                        } else { false };
                        state(owner, |s| {
                            if accepted {
                                s.grab_spline = selected;
                                s.try_update_spline(true, grab, &mut raw, effective[0], num)?;
                                s.approximate_skeleton_speed = Some(ZERO);
                                s.try_compute_target_hand_ik(&mut raw, effective[0], num)?;
                                if need(s.grab_object, "Pushing old grab flag")? {
                                    s.try_start_target_hand_ik_blend(num)?;
                                }
                                s.grab_object = Some(true);
                            } else { s.grab_object = Some(false); }
                            Ok(())
                        })?;
                    }
                }
                Ok(bounds)
            }).map_err(error)??;
        if state(owner, |s| need(s.grab_object, "Pushing grab flag"))? {
            let height = {
                let remaining = owner.remaining().lock().map_err(error)?;
                let feet = need(remaining.feet_ik.as_ref(), "Pushing FeetIK")?;
                if !feet.binding.matches(owner, &p, skeleton) {
                    return Err(error("foreign Pushing FeetIK"));
                }
                feet.height_above_ground
            };
            let (effective, timer) = {
                let p = p.lock().map_err(error)?;
                (
                    need(
                        p.ground_position_fields().effective_animation,
                        "Pushing effective animation",
                    )?
                    .columns()
                    .map(lanes),
                    need(p.physics_state_fields().state_timer, "Pushing state timer")?,
                )
            };
            state(owner, |s| {
                s.try_calculate_skeleton_position(effective, timer, height, refs, num)
            })?;
            {
                let grab = owner.grab_spline().lock().map_err(error)?;
                let grab = need(grab.as_ref(), "PlayerGrabSpline")?;
                let p = p.lock().map_err(error)?;
                state(owner, |s| s.try_move_object(&p, grab, refs, num))?;
            }
            let (left, right, blend) = state(owner, |s| {
                let old = need(s.ik_blend_value, "Pushing hand IK blend")?;
                let blend = if need(s.hand_ik_targets_valid, "Pushing hand targets valid")? {
                    let value = old + f32::from_bits(0x3e4c_cccd);
                    s.ik_blend_value = Some(value);
                    fsel(value - 1.0, 1.0, value)
                } else {
                    let value = old - f32::from_bits(0x3e4c_cccd);
                    s.ik_blend_value = Some(value);
                    fsel(value, value, 0.0)
                };
                s.ik_blend_value = Some(blend);
                Ok((
                    need(s.hand_left_target, "Pushing left hand")?,
                    need(s.hand_right_target, "Pushing right hand")?,
                    blend,
                ))
            })?;
            self.try_set_pair_grab_hand_targets(
                owner,
                board,
                skeleton,
                left.into(),
                right.into(),
                refs,
                blend,
                num.n,
            )?;
            let (motion, angle) = state(owner, |s| {
                Ok((
                    need(s.world_position_delta, "Pushing motion")?,
                    need(s.align_angle, "Pushing alignment")?,
                ))
            })?;
            self.try_update_pair_offboard_pushing_skeleton(
                owner,
                board,
                skeleton,
                motion.into(),
                angle,
                refs,
                ground,
                num.n,
                num.s,
            )
            .map_err(error)?;
            let descriptor = state(owner, |s| {
                let spline = need(s.grab_spline.as_ref(), "Pushing spline")?;
                Ok(super::remaining_states::GrabDescriptor {
                    object_type: spline.descriptor_type,
                    object_id: spline.descriptor_object,
                })
            })?;
            {
                let mut grab_guard = owner.grab_spline().lock().map_err(error)?;
                let grab = need(grab_guard.as_mut(), "PlayerGrabSpline")?;
                grab.request_spline_data(descriptor.object_type, descriptor.object_id)?;
                grab.defer_message(descriptor, GrabMessageKind::NotifyGrabbing)?;
                if need(
                    p.lock().map_err(error)?.known_flags_8c8(),
                    "processed flags8c8",
                )? & 0x4000_0000
                    == 0
                {
                    grab.defer_message(descriptor, GrabMessageKind::NotifyLocalPlayer)?;
                }
            }
            state(owner, |s| {
                s.time_grabbing_this_spline = Some(
                    need(s.time_grabbing_this_spline, "Pushing spline time")?
                        + f32::from_bits(0x3c88_8889),
                );
                Ok(())
            })?;
        } else {
            self.try_update_pair_offboard_pushing_skeleton(
                owner,
                board,
                skeleton,
                ZERO.into(),
                0.0,
                refs,
                ground,
                num.n,
                num.s,
            )
            .map_err(error)?;
        }
        let query = state(owner, |s| {
            let (angle_limit, horizontal_limit) = s.try_grab_angle_limits()?;
            let end_exclusion = read_floats::<1>(
                need(s.attribs_off_board.as_ref(), "Pushing offboard attributes")?,
                0x2523_6f55_4677_f74f,
            )
            .map_err(error)?[0];
            Ok(GrabBoxQuery {
                reach_start: need(s.reach_base, "Pushing reach base")?,
                bounds,
                end_exclusion,
                angle_limit,
                horizontal_limit,
            })
        })?;
        need(
            owner.grab_spline().lock().map_err(error)?.as_mut(),
            "PlayerGrabSpline",
        )?
        .start_box_query(4, query)
    }

    fn validate_pushing_state(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
    ) -> Result<(), GrindError> {
        if !self
            .try_controller_pair_matches_owner(owner, board, skeleton)
            .map_err(error)?
        {
            return Err(error("foreign Pushing pair"));
        }
        let p = processed(owner)?;
        state(owner, |s| {
            let b = &s.base.physical;
            if !b.board.same_as(board)
                || !b.skeleton.same_as(skeleton)
                || !b.owner.upgrade().is_some_and(|v| Arc::ptr_eq(&v, owner))
                || !Arc::ptr_eq(&s.base.processed, &p)
            {
                return Err(error("foreign Pushing state binding"));
            }
            for (link, member) in [
                (&s.player_grab_spline, OwnerMember::PlayerGrabSpline),
                (&s.off_board_feet_ik, OwnerMember::OffBoardFeetIK),
                (&s.skateboard_controller, OwnerMember::SkateboardController),
            ] {
                let link = need(
                    link.as_ref().and_then(Option::as_ref),
                    "Pushing collaborator",
                )?;
                if link.member != member
                    || !link.owner.upgrade().is_some_and(|v| Arc::ptr_eq(&v, owner))
                {
                    return Err(error("foreign Pushing collaborator"));
                }
            }
            Ok(())
        })
    }

    pub(crate) fn try_enter_pair_offboard_pushing<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), GrindError>
    where
        N::Error: std::fmt::Debug,
    {
        self.validate_pushing_state(owner, board, skeleton)?;
        state(owner, |s| s.reset(refs).map_err(error))?;
        let p = processed(owner)?;
        let last = need(
            p.lock().map_err(error)?.physics_state_fields().last,
            "last state",
        )?;
        let reset = {
            let mut remaining = owner.remaining().lock().map_err(error)?;
            let feet = need(remaining.feet_ik.as_mut(), "Pushing FeetIK")?;
            if !feet.binding.matches(owner, &p, skeleton) {
                return Err(error("foreign Pushing FeetIK"));
            }
            feet.consider_reset_before_disable_ik(last)
        };
        if reset {
            self.try_wipeout_skeleton_operation(
                board,
                skeleton,
                WipeoutSkeletonOperation::DisableIk,
                numeric,
            )
            .map_err(error)?;
        }
        need(
            owner.remaining().lock().map_err(error)?.feet_ik.as_mut(),
            "Pushing FeetIK",
        )?
        .finish_consider_reset(501);
        self.try_request_pair_drop_board(owner, board, skeleton, refs, numeric)
            .map_err(error)?;
        self.try_pair_skeleton_collision_transition(
            board,
            skeleton,
            SkeletonCollisionTransition::Set(4),
        )
        .map_err(error)?;
        let grab = {
            let p = p.lock().map_err(error)?;
            if need(p.physics_state_fields().flags_8d0, "processed flags8d0")? & 0x0100_0000 != 0 {
                let grab = need(p.grab_spline(false), "primary grab spline")?;
                grab.has_spline_and_body().then(|| grab.clone())
            } else {
                None
            }
        };
        if let Some(grab) = grab {
            // Geometry uses the same host VMX estimate/rounding services as
            // the grab provider; controller/IK retain the caller's numeric.
            let mut n = HostReckoningNumeric;
            let mut trig = HostReckoningSinCosNumeric;
            let num = &mut GrindNum {
                n: &mut n,
                s: &mut trig,
            };
            state(owner, |s| {
                s.grab_spline = Some(grab);
                Ok(())
            })?;
            let effective = need(
                p.lock()
                    .map_err(error)?
                    .ground_position_fields()
                    .effective_animation,
                "Pushing effective animation",
            )?;
            let grab_owner = owner.grab_spline().lock().map_err(error)?;
            let grab_owner = need(grab_owner.as_ref(), "PlayerGrabSpline")?;
            self.try_with_offboard_pair(board, skeleton, owner, |normal| -> Result<_, GrindError> {
                    let mut raw = |part| {
                        Ok(lanes(need(
                            normal.anim_part_world_raw_position(part),
                            "Pushing raw animation part",
                        )?))
                    };
                    state(owner, |s| {
                        s.try_update_spline(
                            true,
                            grab_owner,
                            &mut raw,
                            lanes(effective.columns()[0]),
                            num,
                        )?;
                        s.grab_object = Some(true);
                        s.try_compute_target_hand_ik(&mut raw, lanes(effective.columns()[0]), num)?;
                        let hands =
                            need(s.current_spline_hands_transform, "Pushing current hands")?;
                        s.target_push_normal = Some(neg(hands[2]));
                        s.last_push_normal = s.target_push_normal;
                        s.target_x_direction = Some(neg(hands[0]));
                        let (grab_z, grab_y) = {
                            let attrs = need(s.attribs.as_ref(), "Pushing attributes")?;
                            (
                                read_floats::<1>(attrs, 0x5e35_db02_be69_7a58).map_err(error)?[0],
                                read_floats::<1>(attrs, 0x02f0_3da5_1050_c8b3).map_err(error)?[0],
                            )
                        };
                        s.world_grab_z = Some(grab_z);
                        s.world_grab_y = Some(grab_y);
                        Ok(())
                    })
                })
                .map_err(error)??;
        }
        // MoveDMO_Enter82a76160 is gated Juice telemetry (location capture),
        // with no grab/physics writes. The native Juice logger is not ported.
        Ok(())
    }

    pub(crate) fn try_exit_pair_offboard_pushing(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
    ) -> Result<(), GrindError> {
        self.validate_pushing_state(owner, board, skeleton)?;
        self.try_with_offboard_pair(board, skeleton, owner, |normal| -> Result<_, GrindError> {
                normal.disable_proxy_volume();
                Ok(())
            })
            .map_err(error)??;
        let descriptor = state(owner, |s| need(s.grab_spline_desc, "Pushing descriptor"))?;
        need(
            owner.grab_spline().lock().map_err(error)?.as_ref(),
            "PlayerGrabSpline",
        )?
        .defer_message(descriptor, GrabMessageKind::Freeze)?;
        // MoveDMO_Exit82a780a8 only emits the gated Juice DMO_Movement event.
        Ok(())
    }

    pub(crate) fn try_post_pair_offboard_pushing<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), GrindError>
    where
        N::Error: std::fmt::Debug,
    {
        self.validate_pushing_state(owner, board, skeleton)?;
        if refs
            .try_pushing_bool(PushingBool::kEnableOffBoardPushingWipeouts)
            .map_err(error)?
        {
            self.try_check_pair_offboard_wipeout(board, skeleton, owner, numeric)
                .map_err(error)?;
        }
        let p = processed(owner)?;
        let require_wipeout = {
            let remaining = owner.remaining().lock().map_err(error)?;
            let feet = need(remaining.feet_ik.as_ref(), "Pushing FeetIK")?;
            if !feet.binding.matches(owner, &p, skeleton) {
                return Err(error("foreign Pushing FeetIK"));
            }
            feet.request_wipeout // DoesRequireWipeout82dc8998
        };
        if require_wipeout {
            owner
                .wipeout()
                .lock()
                .map_err(error)?
                .try_ground_request(25)
                .map_err(error)?;
        }
        Ok(())
    }
}
