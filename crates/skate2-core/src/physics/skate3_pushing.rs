//! PC port: TU3 PhysState_OffBoard_Pushing (vtable `0x82327364`) for the Skate 3 physics. Its
//! Enter (`0x82d442d0`), UpdateSpline (`0x82d444a0`), hand IK targets (`0x82d45d30`,
//! `0x82d46218`, `0x82d463d8`) and MoveObject (`0x82d45318`) are Skate 2's `OffBoardPushing` with
//! the pushing cvars at their defaults, so the host runs Skate 2's over the world's DMO grab
//! splines, whose messages the world applies in Update1. TU3 applies MoveObject's force raw
//! (DynamicObject::ApplyForce `0x82c52b30` -> Body::AddForce); Skate 3's pushing attributes (max
//! force 20, PIDs 20/40) suit Skate 3's prop bodies, so the world's Skate 2 props take Skate 2's
//! (max force 4000, PIDs 10000/4000), the rest of the class being equal in both games.
use std::sync::Arc;

use crate::{
    attribute_runtime::AttributeRuntimeVault,
    reckoning_numeric::ReckoningNumeric,
    skate3_native::graph_host::motion_moving_object::Physical,
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::{
        PhysicalAssemblyWorld,
        remaining_states::{GrabDescriptor, MoveControls, PushingCore},
    },
    physical_player_grab_spline::{
        GrabSpline,
        manager::{GrabMessage, GrabMessageKind},
    },
    physical_player_grind_manager::{math::GrindNum, view::GrindError},
};

/// TU3 Enter reads two more attributes into the state after the push normals.
const WORLD_GRAB_Z: u64 = 0x5e35_db02_be69_7a58;
const WORLD_GRAB_Y: u64 = 0x02f0_3da5_1050_c8b3;
/// The DMO grab spline kind (`GrabSpline` descriptor type 2).
const DMO: u32 = 2;
/// The off-board pushing collection's grab box half dimensions and offset (layout 0 and 32), and
/// the pushing collection's repush delay, which Update reads.
const BOX_DIMENSIONS: u64 = 0xa03e_a2ac_5576_e738;
const BOX_OFFSET: u64 = 0xfe50_188a_8336_fdf9;
const REPUSH_DELAY: u64 = 0xf081_ca3e_c9e5_1bfb;

/// What the Skate 3 physics gives the pushing state each tick.
pub(super) struct Inputs {
    /// The selected physical state is OffBoardPushing.
    pub in_state: bool,
    /// ProcessedPhysIn+2476 bit 22, the world-grab intent Enter tests.
    pub world_grab: bool,
    /// The published first grab record's descriptor (kind, id), when valid.
    pub record: Option<(u32, u32)>,
    /// The left hand, right hand and hips (parts 3, 7 and 23) in world space.
    pub parts: [[f32; 4]; 3],
    /// The effective root transform's X axis.
    pub effective_x: [f32; 4],
    pub controls: MoveControls,
    /// The ground height under the skater, where MoveObject places its force.
    pub ground_y: f32,
    /// The effective root transform, ProcessedPhysIn+2664 (time in state) and OffBoardFeetIK
    /// +0x128 (height above ground): CalculateSkeletonPosition's inputs.
    pub effective: [[f32; 4]; 4],
    pub state_timer: f32,
    pub height_above_ground: f32,
    /// The skeleton's centre of mass (Skeleton+0x3E00).
    pub centre_of_mass: [f32; 4],
    /// ProcessedPhysIn+192 (the effective animation transform, the grab box's frame), +736 (the
    /// collision pose error) and +2484 (bit 26 is timed).
    pub effective_animation: [[f32; 4]; 4],
    pub pose_error: [f32; 4],
    pub flags_2484: u32,
    /// The grab owner's validated splines, when its query results are ready (owner +12836 bit 6).
    pub candidates: Option<Vec<skate3_core::player::offboard::grab_scene::Record>>,
}

/// SetLeft/RightHandIKTargetPosition's targets and blend for the next skeleton update.
#[derive(Clone, Copy, Debug)]
pub(super) struct HandTargets {
    pub left: [f32; 4],
    pub right: [f32; 4],
    pub blend: f32,
}

#[derive(Default)]
pub(super) struct Pushing {
    core: Option<PushingCore>,
    active: bool,
    /// The FillPhysOut record MovingObject reads, from the last update.
    output: Physical,
    /// TU3 Pushing+0x1A0: the follow position, seeded at Enter with the skeleton's centre of mass.
    accumulator: [f32; 4],
    follow: Option<crate::skate3_native::physics::PushingFollow>,
    hands: Option<HandTargets>,
    ticks: u32,
    /// A failure, after which the state moves no objects for the session.
    failed: bool,
}

fn grind(error: GrindError) -> String {
    format!("Skate 3 pushing: {}", error.0)
}

/// `GetGrabSpline` for a DMO spline: the world's live object, refreshed.
fn refresh(world: &PhysicalAssemblyWorld, id: u32, num: &mut GrindNum<'_>) -> Result<Option<GrabSpline>, String> {
    let owner = world.grind_services.living_world();
    let living = owner.lock().map_err(|_| "Skate 3 pushing: LivingWorld lock poisoned")?;
    let Some(dmos) = living.dmo_objects.as_deref() else { return Ok(None) };
    let mut spline = GrabSpline::in_uninitialized_allocation();
    let found = dmos
        .get_grab_spline(
            id,
            &mut spline,
            &mut |body| world.try_living_world_grab_part(body).map_err(|e| GrindError(format!("{e:?}"))),
            num,
        )
        .map_err(grind)?;
    Ok(found.then_some(spline))
}

impl Pushing {
    fn core(&mut self, vault: &Arc<AttributeRuntimeVault>, cvars: &InputCvarReferences) -> Result<&mut PushingCore, String> {
        if self.core.is_none() {
            let mut core = PushingCore::unconstructed();
            core.construct(vault, cvars).map_err(|e| format!("Skate 3 pushing constructor: {e:?}"))?;
            self.core = Some(core);
        }
        Ok(self.core.as_mut().expect("constructed above"))
    }

    /// FillPhysOut 0x82d46fd0's moving-object record: movement and force along the blended hands'
    /// Z and -X, and the crouch (Skate 2's try_fill_pushing_state stores the same).
    pub(super) fn output(&self) -> Physical {
        if self.active { self.output } else { Physical::default() }
    }

    /// The skeleton follow of the last update while grabbing.
    pub(super) fn follow(&self) -> Option<crate::skate3_native::physics::PushingFollow> {
        self.follow.filter(|_| self.active)
    }

    /// The hand IK targets of the last update while grabbing.
    pub(super) fn hand_targets(&self) -> Option<HandTargets> {
        self.hands.filter(|_| self.active)
    }

    /// The state's mGrabObject (Pushing+0x4B0 bit 5), which its FillPhysOut (`0x82d46fd0`)
    /// stores as PhysOut+0x48 byte 0x130.
    pub(super) fn grabbing_object(&self) -> bool {
        self.active && self.core.as_ref().is_some_and(|core| core.grab_object == Some(true))
    }

    /// [`Self::tick`], logging the first failure and moving no objects after it, so pushing
    /// can never stop the skater's own physics.
    pub(super) fn tick_or_disable(
        &mut self,
        world: &PhysicalAssemblyWorld,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &InputCvarReferences,
        inputs: &Inputs,
    ) {
        if self.failed {
            return;
        }
        let was_active = self.active;
        if let Err(error) = self.tick(world, vault, cvars, inputs) {
            eprintln!("{error}");
            self.failed = true;
            self.active = false;
        }
        if self.active != was_active {
            crate::diagnostics::test_log::event(
                "skate3_pushing",
                format_args!("active={} grabbing={:?} record={:?}", self.active, self.core.as_ref().and_then(|c| c.grab_object), inputs.record),
            );
        }
    }

    fn tick(
        &mut self,
        world: &PhysicalAssemblyWorld,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &InputCvarReferences,
        inputs: &Inputs,
    ) -> Result<(), String> {
        let manager = world.grab_query_manager().cloned();
        let mut send = |descriptor: GrabDescriptor, kind: GrabMessageKind| -> Result<(), GrindError> {
            manager.as_ref().map_or(Ok(()), |manager| manager.defer_message(GrabMessage { descriptor, kind }))
        };
        if !inputs.in_state {
            if std::mem::take(&mut self.active)
                && let Some(core) = self.core.as_ref()
                && let Some(descriptor) = core.grab_spline_desc
            {
                // Exit: the grabbed object is frozen again.
                send(descriptor, GrabMessageKind::Freeze).map_err(grind)?;
            }
            return Ok(());
        }
        let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
        let mut sin_cos = crate::reckoning_sin_cos::HostReckoningSinCosNumeric;
        let mut num = GrindNum { n: &mut numeric, s: &mut sin_cos };
        let parts = inputs.parts;
        let mut raw = |part: usize| -> Result<[f32; 4], GrindError> {
            match part {
                3 => Ok(parts[0]),
                7 => Ok(parts[1]),
                23 => Ok(parts[2]),
                _ => Err(GrindError(format!("Skate 3 pushing has no pose part {part}"))),
            }
        };
        let entering = !self.active;
        self.active = true;
        self.follow = None;
        self.hands = None;
        if entering {
            // Enter 0x82d442d0: after Reset, Pushing+0x1A0 = Skeleton+0x3E00.
            self.accumulator = inputs.centre_of_mass;
        }
        let mut accumulator = self.accumulator;
        let trace_tick = self.ticks % 15 == 0;
        let core = self.core(vault, cvars)?;
        if entering {
            // Enter: Reset, then with the world-grab intent and a published DMO grab record the
            // spline, UpdateSpline(first), the hand IK targets and the push normals.
            core.reset(cvars).map_err(|e| format!("Skate 3 pushing reset: {e:?}"))?;
            let Some((DMO, id)) = inputs.record.filter(|_| inputs.world_grab) else { return Ok(()) };
            let Some(spline) = refresh(world, id, &mut num)? else { return Ok(()) };
            core.grab_spline = Some(spline);
            core.try_update_spline_with(true, &mut send, &mut raw, inputs.effective_x, &mut num).map_err(grind)?;
            core.grab_object = Some(true);
            core.try_compute_target_hand_ik(&mut raw, inputs.effective_x, &mut num).map_err(grind)?;
            let hands = core.current_spline_hands_transform.ok_or("Skate 3 pushing: no hands transform")?;
            core.target_push_normal = Some(hands[2].map(|v| -v));
            core.last_push_normal = core.target_push_normal;
            core.target_x_direction = Some(hands[0].map(|v| -v));
            let attrs = core.attribs.as_ref().ok_or("Skate 3 pushing: no attributes")?;
            let read = |key| {
                crate::physical_player_assembly::remaining_states::read_floats::<1>(attrs, key)
                    .map(|v| v[0])
                    .map_err(|e| format!("Skate 3 pushing attribute {key:016x}: {e:?}"))
            };
            let (grab_z, grab_y) = (read(WORLD_GRAB_Z)?, read(WORLD_GRAB_Y)?);
            core.world_grab_z = Some(grab_z);
            core.world_grab_y = Some(grab_y);
            return Ok(());
        }
        // Holding nothing, Pushing::Update still moves the skeleton with no motion or turn
        // toward its follow position (0x82D45218..45260).
        let idle = crate::skate3_native::physics::PushingFollow {
            displacement: [0.0; 4],
            turn: 0.0,
            push: crate::skate3_native::physics::Push { motion: [0.0; 4], turn: 0.0, follow: accumulator },
        };
        let read = |attrs: Option<&crate::attribute_runtime::AttributeRuntimeInstance>, key| {
            let attrs = attrs.ok_or("Skate 3 pushing: no attributes")?;
            crate::physical_player_assembly::remaining_states::read_floats::<4>(attrs, key)
                .map_err(|e| format!("Skate 3 pushing attribute {key:016x}: {e:?}"))
        };
        // Update 0x82D44A28..AA8: a collision pose error (ProcessedPhysIn+736) past 0.2 m is an
        // emergency exit, with the repush delay.
        let error = inputs.pose_error;
        if error[0] * error[0] + error[1] * error[1] + error[2] * error[2] > f32::from_bits(0x3d23_d70b) {
            core.delay_until_repushable = Some(read(core.attribs.as_ref(), REPUSH_DELAY)?[0]);
            core.emergency_exit = Some(true);
        }
        // 0x82D44AAC..AE0: how long ProcessedPhysIn+2484 bit 26 has been set.
        core.time_in_partial_ragdoll = Some(if inputs.flags_2484 & 0x0400_0000 != 0 {
            core.time_in_partial_ragdoll.unwrap_or(0.0) + f32::from_bits(0x3c88_8889)
        } else {
            0.0
        });
        // 0x82D44BF8..C28: the grab box on the effective animation transform, pitch off.
        let bounds = crate::physical_player_grab_spline::geometry::GrabBox::from_player(
            inputs.effective_animation,
            read(core.attribs_off_board.as_ref(), BOX_OFFSET)?,
            read(core.attribs_off_board.as_ref(), BOX_DIMENSIONS)?,
            false,
            &mut num,
        )
        .map_err(grind)?;
        // 0x82D44C3C..D38: the held spline, live from its object, and whether the retained one
        // can still be gripped where the hands are (CanGrabSplineAtPoint).
        let id = core.grab_spline_desc.map_or(0, |descriptor| descriptor.object_id as u32);
        let current = if core.grab_spline.is_some() {
            refresh(world, id, &mut num)?.filter(GrabSpline::has_spline_and_body)
        } else {
            None
        };
        let still_grabbable = match (&current, core.grab_spline.as_ref()) {
            (Some(_), Some(retained)) => core.try_is_spline_grabbable(retained, &bounds, true, &mut num).map_err(grind)?,
            _ => false,
        };
        // 0x82D44DE4..E14: the intent released, the bit-26 time past 0.4 s or an emergency exit
        // lets go; a spline no longer grippable gives way to the best other one in reach.
        let releasing = !inputs.world_grab
            || core.time_in_partial_ragdoll.unwrap_or(0.0) > f32::from_bits(0x3ecc_cccd)
            || core.emergency_exit == Some(true);
        if releasing {
            core.grab_object = Some(false);
        } else if let Some(spline) = current.filter(|_| still_grabbable) {
            core.grab_spline = Some(spline);
            core.try_update_spline_with(false, &mut send, &mut raw, inputs.effective_x, &mut num).map_err(grind)?;
            core.try_compute_target_hand_ik(&mut raw, inputs.effective_x, &mut num).map_err(grind)?;
            core.try_update_target_hand_ik_blend(&mut num).map_err(grind)?;
            core.grab_object = Some(true);
        } else {
            core.time_grabbing_this_spline = Some(0.0);
            // sub_82D4D150 with the held spline excluded, then CanGrabSpline from the reach base.
            let held = core.grab_spline_desc;
            let candidate = match (&inputs.candidates, core.reach_base) {
                (Some(records), Some(reach)) => {
                    let others: Vec<_> = records
                        .iter()
                        .filter(|record| {
                            let key = record.descriptor();
                            held.is_none_or(|held| (held.object_type as u32, held.object_id as u32) != (key.kind, key.id))
                        })
                        .cloned()
                        .collect();
                    skate3_core::player::offboard::grab_scene::best_spline(&others, reach)
                }
                _ => None,
            };
            let candidate = match candidate.filter(|record| record.descriptor().kind == DMO) {
                Some(record) => refresh(world, record.descriptor().id, &mut num)?,
                None => None,
            };
            let accepted = match &candidate {
                Some(spline) => {
                    core.try_is_spline_grabbable(spline, &bounds, false, &mut num).map_err(grind)?
                        // 0x82D44F58..68 tests the OLD retained spline's data and body.
                        && core.grab_spline.as_ref().is_some_and(GrabSpline::has_spline_and_body)
                }
                None => false,
            };
            if let (true, Some(spline)) = (accepted, candidate) {
                let was_grabbing = core.grab_object == Some(true);
                core.grab_spline = Some(spline);
                core.try_update_spline_with(true, &mut send, &mut raw, inputs.effective_x, &mut num).map_err(grind)?;
                core.approximate_skeleton_speed = Some([0.0; 4]);
                core.try_compute_target_hand_ik(&mut raw, inputs.effective_x, &mut num).map_err(grind)?;
                if was_grabbing {
                    core.try_start_target_hand_ik_blend(&mut num).map_err(grind)?;
                }
                core.grab_object = Some(true);
            } else {
                core.grab_object = Some(false);
            }
        }
        if core.grab_object != Some(true) {
            self.follow = Some(idle);
            return Ok(());
        }
        core.try_calculate_skeleton_position(inputs.effective, inputs.state_timer, inputs.height_above_ground, cvars, &mut num)
            .map_err(grind)?;
        let (descriptor, message) = core.try_move_object_with(inputs.controls, inputs.ground_y, cvars, &mut num).map_err(grind)?;
        let output = match (core.blended_spline_hands_transform, core.approximate_skeleton_speed, core.smoothed_force) {
            (Some(hands), Some(speed), Some(force)) => {
                let negative_x = hands[0].map(|v| -v);
                let mut dot = |a, b| num.n.dot3(a, b).map_err(|e| format!("Skate 3 pushing output: {e:?}"));
                Some(Physical {
                    movement: [dot(speed, negative_x)?, dot(speed, hands[2])?],
                    force: [dot(force, negative_x)?, dot(force, hands[2])?],
                    crouch: core.anim_crouch.unwrap_or(0.0),
                })
            }
            _ => None,
        };
        let time = core.time_grabbing_this_spline.unwrap_or(0.0);
        if (time * 60.0) as u32 % 30 == 0 {
            crate::diagnostics::test_log::event(
                "skate3_pushing_move",
                format_args!(
                    "time={time} body={:?} velocity={:?} controls=({}, {}, {}) message={message:?}",
                    core.grab_spline.as_ref().map(|s| s.body_position),
                    core.grab_spline.as_ref().map(|s| s.velocity),
                    inputs.controls.z,
                    inputs.controls.x,
                    inputs.controls.rotation,
                ),
            );
        }
        send(descriptor, message).map_err(grind)?;
        // The object hears it is grabbed, by the local player (Skate 2's Update82d9cff8 order).
        send(descriptor, GrabMessageKind::NotifyGrabbing).map_err(grind)?;
        send(descriptor, GrabMessageKind::NotifyLocalPlayer).map_err(grind)?;
        core.time_grabbing_this_spline = Some(core.time_grabbing_this_spline.unwrap_or(0.0) + f32::from_bits(0x3c88_8889));
        // Update 0x82d4500c..45088: the hand IK blend steps 0.2 toward the targets' validity.
        let old_blend = core.ik_blend_value.unwrap_or(0.0);
        let blend = if core.hand_ik_targets_valid == Some(true) {
            let value = old_blend + 0.2;
            if value - 1.0 >= 0.0 { 1.0 } else { value }
        } else {
            let value = old_blend - 0.2;
            if value >= 0.0 { value } else { 0.0 }
        };
        core.ik_blend_value = Some(blend);
        let hands = core.hand_left_target.zip(core.hand_right_target).map(|(left, right)| HandTargets { left, right, blend });
        // 82d45090..45180: the follow position moves toward the push normal's point kDistComToBoard
        // (0.65) behind the hands, 0.72 above the effective animation transform's position
        // (ProcessedPhysIn+0xF0, the y the vrlimi at 0x82d4514c puts in), by the object's speed
        // this tick within 0.1; Skeleton::UpdateOffBoardPushing then places the COM within 0.3 of
        // its current one plus the skeleton motion. That transform is the skeleton's
        // animation-to-world, read here from the frame's effective root: the processed record's
        // copy is the same each tick but holds zeros on a tick whose input phase reset it.
        let follow = match (
            core.current_spline_hands_transform,
            core.target_push_normal,
            core.approximate_skeleton_speed,
            core.world_position_delta,
        ) {
            (Some(hands), Some(normal), Some(speed), Some(motion)) => {
                let mut target: [f32; 4] = std::array::from_fn(|i| normal[i].mul_add(0.65, hands[3][i]));
                target[1] = inputs.ground_y + f32::from_bits(0x3f38_51ec);
                if trace_tick {
                    crate::diagnostics::test_log::event(
                        "skate3_pushing_follow",
                        format_args!(
                            "anim_root={:?} effective_root_y={} com={:?} hands={:?} target={:?} follow={:?} motion={:?}",
                            inputs.effective_animation[3], inputs.ground_y, inputs.centre_of_mass, hands[3], target,
                            accumulator, motion
                        ),
                    );
                }
                let expected = speed.map(|v| v * f32::from_bits(0x3c88_8889));
                let to_target: [f32; 4] = std::array::from_fn(|i| target[i] - accumulator[i]);
                let step = clamp_within(to_target, expected, 0.1);
                for i in 0..3 {
                    accumulator[i] += step[i];
                }
                let moved: [f32; 4] = std::array::from_fn(|i| inputs.centre_of_mass[i] + motion[i]);
                let target = clamp_within(accumulator, moved, 0.3);
                let turn = core.align_angle.unwrap_or(0.0);
                Some(crate::skate3_native::physics::PushingFollow {
                    displacement: std::array::from_fn(|i| target[i] - inputs.centre_of_mass[i]),
                    turn,
                    // 0x82D45184..451A0: Pushing+0x120, +0x464 and the new +0x1A0.
                    push: crate::skate3_native::physics::Push { motion, turn, follow: accumulator },
                })
            }
            _ => None,
        };
        if let Some(output) = output {
            self.output = output;
        }
        self.ticks = self.ticks.wrapping_add(1);
        if self.ticks % 10 == 0 {
            crate::diagnostics::test_log::event(
                "skate3_pushing_follow",
                format_args!("follow={follow:?} accumulator={accumulator:?} com={:?} blend={blend}", inputs.centre_of_mass),
            );
        }
        self.accumulator = accumulator;
        self.follow = follow;
        self.hands = hands;
        Ok(())
    }
}

/// PhysicsUtility::ClampVectorAWithinRangeOfVectorB (0x82bd41b0): `a` when within `range` of
/// `b`, else the point at `range` from `b` toward `a`.
pub(super) fn clamp_within(a: [f32; 4], b: [f32; 4], range: f32) -> [f32; 4] {
    let d: [f32; 4] = std::array::from_fn(|i| if i < 3 { a[i] - b[i] } else { 0.0 });
    let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if range >= length {
        return a;
    }
    let scale = range / length;
    std::array::from_fn(|i| if i < 3 { d[i].mul_add(scale, b[i]) } else { a[i] })
}
