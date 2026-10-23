//! UpdateLocomotionTrajectory82cb7d48 on the retained SkaterAnim allocation.
#![allow(
    clippy::neg_cmp_op_on_partial_ord,
    clippy::too_many_arguments,
    clippy::too_many_lines
)]
use super::{
    ActorAnimationConstruction, AnimationFootControl, AnimationFootData,
    gesture_writes::GestureAnimationWriteError, locomotion_debug::LocomotionDebugError,
    locomotion_params::PhysicsParamsError, locomotion_producer::TrajectoryProducerError,
    locomotion_turn::StepTurnError,
};
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    play_animation::FastString30,
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric, try_normalize_safe,
        wrap_turns_to_radians,
    },
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skeleton_root_transforms::SkeletonRootVector as V,
};
use std::convert::Infallible;
type Params = PhysicsParamsError<
    ReckoningHostNumericError,
    Infallible,
    LocomotionDebugError<ReckoningHostNumericError>,
>;
#[derive(Debug)]
pub(crate) enum LocomotionUpdateError {
    Missing(&'static str),
    UnknownFlags(u32),
    FutureNotReady(usize),
    Index(i32),
    Cvar(InputCvarError),
    Numeric(ReckoningHostNumericError),
    Trajectory(TrajectoryProducerError<Infallible>),
    Params(Params),
    Turn(StepTurnError),
    Overlay(DebugOverlayError),
    Attribute(GestureAnimationWriteError),
    Tree(crate::actor_animation_tree::AnimationTreeError),
}
use LocomotionUpdateError as E;
const ZERO: V = V::known([0.0; 4]);
const UP: V = V::known([0.0, 1.0, 0.0, 0.0]);
fn add(a: V, b: V) -> V {
    a.zip(b, |a, b| a + b)
}
fn sub(a: V, b: V) -> V {
    a.zip(b, |a, b| a - b)
}
fn horizontal(mut v: V) -> V {
    v.xyz[1] = 0.0;
    v
}
fn sqrt_seed(value: f32, mut seed: f32) -> f32 {
    for _ in 0..2 {
        let square = seed * seed;
        let half = seed * 0.5;
        let residual = -value.mul_add(square, -1.0);
        seed = half.mul_add(residual, seed);
    }
    let length = value * seed;
    if value == 0.0 { 0.0 } else { length }
}
pub(super) fn lengths<const N: usize>(
    vectors: [V; N],
    numeric: &mut HostReckoningNumeric,
) -> Result<[f32; N], E> {
    let mut squares = [0.0; N];
    for (i, v) in vectors.iter().enumerate() {
        squares[i] = numeric.dot_vector(*v, *v).map_err(E::Numeric)?;
    }
    let mut seeds = [0.0; N];
    for i in 0..N {
        seeds[i] = numeric
            .reciprocal_sqrt_estimate(squares[i])
            .map_err(E::Numeric)?;
    }
    Ok(std::array::from_fn(|i| sqrt_seed(squares[i], seeds[i])))
}
fn foot(index: i32) -> Result<usize, E> {
    let i = if index < 0 { index + 4 } else { index % 4 };
    usize::try_from(i)
        .ok()
        .filter(|i| *i < 4)
        .ok_or(E::Index(index))
}
impl<K, W> ActorAnimationConstruction<K, W> {
    fn update_flags(&self, mask: u32) -> Result<u32, E> {
        let p = self.player.as_ref().ok_or(E::Missing("player"))?;
        if p.flags_known_mask & mask != mask {
            return Err(E::UnknownFlags(mask));
        }
        Ok(p.flags_bits & mask)
    }
    fn write_update_flags(&mut self, mask: u32, value: u32) -> Result<(), E> {
        let p = self.player.as_mut().ok_or(E::Missing("player"))?;
        p.flags_bits = (p.flags_bits & !mask) | (value & mask);
        p.flags_known_mask |= mask;
        Ok(())
    }
    fn update_attribute(&mut self, name: &str, value: f32, stage: i32) -> Result<(), E> {
        self.try_add_gesture_attribute(
            FastString30::encode(name).expect("source literal"),
            value,
            false,
            stage,
        )
        .map_err(E::Attribute)
    }
    fn retain_step(
        &mut self,
        index: usize,
        current_state: i32,
        cvars: &InputCvarReferences,
        numeric: &mut HostReckoningNumeric,
    ) -> Result<AnimationFootData, E> {
        let result = self
            .skater
            .as_ref()
            .ok_or(E::Missing("SkaterAnim"))?
            .step_results[index]
            .result;
        let candidate = AnimationFootData {
            position: result.position,
            normal: result.normal,
            surface: result.surface,
        };
        self.before_sprint
            .as_mut()
            .ok_or(E::Missing("locomotion fields"))?
            .step_failure_counts[index] = 0;
        let valid_mask = if index == 0 { 0x100 } else { 0x40 };
        let previous_mask = if index == 0 { 0x80 } else { 0x20 };
        let selected = if self.update_flags(valid_mask)? != 0 {
            let last = self.skater.as_ref().unwrap().last_steps[index];
            let query = self
                .before_face
                .as_ref()
                .ok_or(E::Missing("locomotion suffix"))?
                .queried_feet[index];
            let [between, new_distance, old_distance] = lengths(
                [
                    sub(candidate.position, last.position),
                    sub(candidate.position, query),
                    sub(last.position, query),
                ],
                numeric,
            )?;
            let previous_distance = if self.update_flags(previous_mask)? != 0 {
                lengths(
                    [sub(
                        self.skater.as_ref().unwrap().last_steps[index + 2].position,
                        candidate.position,
                    )],
                    numeric,
                )?[0]
            } else {
                f32::MAX
            };
            self.skater.as_mut().unwrap().last_steps[index + 2] = last;
            let threshold = cvars
                .try_loco_update_oscillation_threshold()
                .map_err(E::Cvar)?;
            let multiplier = if index == 0 {
                f32::from_bits(0x4019999a)
            } else {
                2.5
            };
            let keep = (previous_distance < threshold && between > previous_distance * multiplier)
                || (old_distance < new_distance
                    && old_distance < threshold
                    && new_distance < threshold)
                || (new_distance > threshold && current_state == 2);
            let selected = if keep {
                last
            } else {
                self.skater.as_mut().unwrap().last_steps[index] = candidate;
                candidate
            };
            self.write_update_flags(previous_mask, previous_mask)?;
            selected
        } else {
            self.skater.as_mut().unwrap().last_steps[index] = candidate;
            candidate
        };
        self.write_update_flags(valid_mask, valid_mask)?;
        Ok(selected)
    }
    fn fallback_step(&self, index: usize) -> Result<AnimationFootData, E> {
        let mask = if index == 0 { 0x100 } else { 0x40 };
        if self.update_flags(mask)? != 0 {
            Ok(self
                .skater
                .as_ref()
                .ok_or(E::Missing("SkaterAnim"))?
                .last_steps[index])
        } else {
            Ok(AnimationFootData {
                position: self
                    .before_face
                    .as_ref()
                    .ok_or(E::Missing("locomotion suffix"))?
                    .queried_feet[index],
                normal: UP,
                surface: 0,
            })
        }
    }
    /// First future readiness is unconditional. The second is queried only if first valid.
    fn update_selected_steps(
        &mut self,
        cvars: &InputCvarReferences,
        numeric: &mut HostReckoningNumeric,
    ) -> Result<([AnimationFootData; 2], bool), E> {
        let skater = self.skater.as_ref().ok_or(E::Missing("SkaterAnim"))?;
        if !skater.step_results[0].ready {
            return Err(E::FutureNotReady(0));
        }
        let current = skater.trajectory.current_state;
        let first_valid = skater.step_results[0].result.valid;
        let first = if first_valid {
            self.retain_step(0, current, cvars, numeric)?
        } else {
            let first = self.fallback_step(0)?;
            let fields = self
                .before_sprint
                .as_mut()
                .ok_or(E::Missing("locomotion fields"))?;
            fields.step_failure_counts[0] = fields.step_failure_counts[0].wrapping_add(1);
            first
        };
        let second_valid = if first_valid {
            let enable = cvars.try_loco_update_future_assert().map_err(E::Cvar)?;
            let second = &self.skater.as_ref().unwrap().step_results[1];
            if enable && !second.ready {
                return Err(E::FutureNotReady(1));
            }
            second.result.valid
        } else {
            false
        };
        let second = if second_valid {
            self.retain_step(1, current, cvars, numeric)?
        } else {
            let second = self.fallback_step(1)?;
            let fields = self
                .before_sprint
                .as_mut()
                .ok_or(E::Missing("locomotion fields"))?;
            fields.step_failure_counts[1] = fields.step_failure_counts[1].wrapping_add(1);
            second
        };
        Ok(([first, second], second_valid))
    }
}

#[derive(Clone, Copy)]
struct StrideMetrics {
    previous_y: f32,
    next_y: f32,
    previous_length: f32,
    next_length: f32,
    vertical_speed: f32,
    forward_speed: f32,
    previous_normal: f32,
    next_normal: f32,
    direction: V,
}
fn stride_metrics(
    current: AnimationFootControl,
    next: AnimationFootControl,
    velocity: V,
    numeric: &mut HostReckoningNumeric,
) -> Result<StrideMetrics, E> {
    let previous_y = current.stride.xyz[1];
    let next_y = next.stride.xyz[1];
    let [previous_length, next_length] = lengths([current.stride, next.stride], numeric)?;
    let direction =
        try_normalize_safe(horizontal(current.stride), ZERO, numeric).map_err(E::Numeric)?;
    let forward_speed = numeric
        .dot_vector(direction, velocity)
        .map_err(E::Numeric)?;
    let next_direction =
        try_normalize_safe(horizontal(next.stride), ZERO, numeric).map_err(E::Numeric)?;
    let previous_normal = numeric
        .dot_vector(direction, current.data.normal)
        .map_err(E::Numeric)?;
    let next_normal = numeric
        .dot_vector(next_direction, next.data.normal)
        .map_err(E::Numeric)?;
    Ok(StrideMetrics {
        previous_y,
        next_y,
        previous_length,
        next_length,
        vertical_speed: velocity.xyz[1],
        forward_speed,
        previous_normal,
        next_normal,
        direction,
    })
}
impl<K, W> ActorAnimationConstruction<K, W> {
    pub(crate) fn try_update_locomotion_trajectory(
        &mut self,
        dt: f32,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
        numeric: &mut HostReckoningNumeric,
        sin_cos: &mut HostReckoningSinCosNumeric,
    ) -> Result<(), E> {
        let time = self
            .skater
            .as_ref()
            .ok_or(E::Missing("SkaterAnim"))?
            .trajectory
            .try_time_to_next_foot()
            .map_err(E::Trajectory)?;
        self.before_face
            .as_mut()
            .ok_or(E::Missing("locomotion suffix"))?
            .time_to_foot_change = time;
        let ([next_step, after_step], valid) = self.update_selected_steps(cvars, numeric)?;
        if !valid {
            let suffix = self.before_face.as_mut().unwrap();
            let desired = suffix.desired_speed;
            suffix.desired_speed = if -desired >= 0.0 { desired } else { 0.0 };
            let failures = self.before_sprint.as_ref().unwrap().step_failure_counts[0];
            if failures
                > cvars
                    .try_create_locomotion_failure_frame_count()
                    .map_err(E::Cvar)?
            {
                self.write_update_flags(!0xfffbe7ff, 0)?;
                self.skater
                    .as_mut()
                    .unwrap()
                    .trajectory
                    .try_add_time(dt)
                    .map_err(E::Trajectory)?;
                return Ok(());
            }
        }
        let trajectory = &self.skater.as_ref().unwrap().trajectory;
        let initial_state = trajectory.current_state;
        let initial_foot = foot(trajectory.physical_foot_index)?;
        let state_index = usize::try_from(initial_state)
            .ok()
            .filter(|v| *v < 3)
            .ok_or(E::Index(initial_state))?;
        let original_action = trajectory.physical_feet[initial_foot].states[state_index].action;
        if original_action != 1 && original_action != 3 {
            let next_delta = horizontal(sub(after_step.position, next_step.position));
            let previous_delta = horizontal(sub(
                next_step.position,
                trajectory.physical_feet[initial_foot].data.position,
            ));
            let threshold = cvars
                .try_capture_loco_update_max_decel_step_ratio()
                .map_err(E::Cvar)?;
            let next_squared = numeric
                .dot_vector(next_delta, next_delta)
                .map_err(E::Numeric)?;
            let threshold = threshold.try_value().map_err(E::Cvar)?;
            let previous_squared = numeric
                .dot_vector(previous_delta, previous_delta)
                .map_err(E::Numeric)?;
            let next_seed = numeric
                .reciprocal_sqrt_estimate(next_squared)
                .map_err(E::Numeric)?;
            let previous_seed = numeric
                .reciprocal_sqrt_estimate(previous_squared)
                .map_err(E::Numeric)?;
            let next_length = sqrt_seed(next_squared, next_seed);
            let previous_length = sqrt_seed(previous_squared, previous_seed);
            if next_length / previous_length < threshold {
                let suffix = self.before_face.as_mut().unwrap();
                let desired = suffix.desired_speed;
                suffix.desired_speed = if -desired >= 0.0 { desired } else { 0.0 };
            }
        }
        let inputs = self.skater.as_ref().unwrap().step_inputs;
        let origin = add(inputs[0].reference, inputs[0].reference_offset);
        let angle = crate::physical_player_phys_out::construct::skeleton_fill_suffix::plane_angle(
            sub(next_step.position, origin),
            inputs[0].parameter_vector,
            UP,
            numeric,
        )
        .map_err(E::Numeric)?;
        let next_angle = wrap_turns_to_radians(angle * f32::from_bits(0x3e22f983));
        let angle = crate::physical_player_phys_out::construct::skeleton_fill_suffix::plane_angle(
            sub(after_step.position, origin),
            add(inputs[1].parameter_vector, inputs[0].parameter_vector),
            UP,
            numeric,
        )
        .map_err(E::Numeric)?;
        let after_angle = wrap_turns_to_radians(angle * f32::from_bits(0x3e22f983));
        self.skater
            .as_mut()
            .unwrap()
            .trajectory
            .try_update_physics_data(
                next_step,
                after_step,
                cvars,
                numeric,
                sin_cos,
                |trajectory, index, numeric| {
                    trajectory.try_debug_draw_locomotion(index, true, cvars, overlay, numeric)
                },
            )
            .map_err(E::Params)?;
        let mut adjustment = self
            .try_calculate_ob_step_turn(next_angle)
            .map_err(E::Turn)?;
        let second = self
            .try_calculate_ob_step_turn(after_angle)
            .map_err(E::Turn)?;
        if cvars.try_loco_update_general_debug().map_err(E::Cvar)? {
            overlay.require().map_err(E::Overlay)?;
        }
        // Raw BGT keeps first only for ordered abs(first)>0, not unordered.
        if !(adjustment.abs() > 0.0) {
            adjustment = second;
        }
        let sign = cvars
            .try_loco_update_correction_turn_sign()
            .map_err(E::Cvar)?;
        let suffix = self.before_face.as_mut().unwrap();
        suffix.step_turn_adjustment = if (suffix.turn * adjustment) * sign < 0.0 {
            0.0
        } else {
            adjustment * sign
        };
        let mut ratio = self
            .skater
            .as_ref()
            .unwrap()
            .trajectory
            .try_current_time_ratio()
            .map_err(E::Trajectory)?;
        if self.update_flags(0x200)? != 0 {
            ratio = if ratio - 1.0 >= 0.0 { ratio } else { 1.0 };
        }
        self.try_set_locomotion_base_speed(ratio)?;
        self.skater
            .as_mut()
            .unwrap()
            .trajectory
            .try_add_time(dt)
            .map_err(E::Trajectory)?;
        let suffix = self.before_face.as_mut().unwrap();
        suffix.applied_extra_y_rotation = suffix.extra_y_rotation;
        self.update_locomotion_stages(
            initial_state,
            initial_foot,
            original_action,
            cvars,
            overlay,
            numeric,
        )?;
        self.before_sprint.as_mut().unwrap().last_start_state = initial_state;
        Ok(())
    }
    fn update_stride_attributes(&mut self, stage: i32, values: [f32; 4]) -> Result<(), E> {
        let [previous_y, next_y, previous_length, next_length] = values;
        let previous_slope = if previous_length > 0.001 {
            previous_y / previous_length
        } else {
            0.0
        };
        let next_slope = if next_length > 0.001 {
            next_y / next_length
        } else {
            0.0
        };
        for (name, value) in [
            ("YPrevStride", previous_y),
            ("ZPrevStride", previous_length),
            ("YNextStride", next_y),
            ("ZNextStride", next_length),
            ("NextSlope", next_slope),
            ("PrevSlope", previous_slope),
        ] {
            self.update_attribute(name, value, stage)?;
        }
        Ok(())
    }
}
impl<K, W> ActorAnimationConstruction<K, W> {
    /// `GetBaseAnim()->SetSpeed`, also used by StandingJumpDescent82ca6328.
    pub(crate) fn try_set_base_animation_speed(&mut self, speed: f32) -> Result<(), E> {
        self.try_set_locomotion_base_speed(speed)
    }
    fn try_set_locomotion_base_speed(&mut self, speed: f32) -> Result<(), E> {
        let root = self
            .base
            .as_ref()
            .ok_or(E::Missing("Animatable"))?
            .body_root
            .clone()
            .ok_or(E::Missing("GetBaseAnim"))?;
        self.set_locomotion_reference_speed(root, speed)
    }
    // Transition::SetSpeed82d6db50 forwards TO only, including sequences.
    fn set_locomotion_reference_speed(
        &mut self,
        root: super::blend_anim::BodyAnimationReference,
        speed: f32,
    ) -> Result<(), E> {
        use super::blend_anim::BodyAnimationReference as R;
        match root {
            R::Tree(tree) => self
                .initialization
                .as_mut()
                .ok_or(E::Missing("Initialize"))?
                .tree_pools
                .get_mut(tree.pool_index)
                .ok_or(E::Missing("tree pool"))?
                .runtime
                .set_speed(&tree.node, speed)
                .map_err(E::Tree),
            R::TransitionBlend(index) => {
                let target = self
                    .base
                    .as_ref()
                    .ok_or(E::Missing("Animatable"))?
                    .transition_store
                    .blends
                    .get(index)
                    .and_then(|b| b.runtime.as_ref())
                    .ok_or(E::Missing("blend transition"))?
                    .to
                    .clone();
                self.set_locomotion_reference_speed(target, speed)
            }
            R::TransitionSequence(index) => {
                let target = self
                    .base
                    .as_ref()
                    .ok_or(E::Missing("Animatable"))?
                    .transition_store
                    .sequence_runtime
                    .get(index)
                    .and_then(|b| b.as_ref())
                    .ok_or(E::Missing("sequence transition"))?
                    .to
                    .clone();
                self.set_locomotion_reference_speed(target, speed)
            }
        }
    }
    fn update_locomotion_stages(
        &mut self,
        initial_state: i32,
        initial_foot: usize,
        original_action: i32,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
        numeric: &mut HostReckoningNumeric,
    ) -> Result<(), E> {
        let feet = &self.skater.as_ref().unwrap().trajectory.physical_feet;
        let previous = (initial_foot + 3) % 4;
        let mut current = initial_foot;
        let mut next = (previous + 2) % 4;
        let velocity = if feet[previous].initialized {
            feet[previous].states[2].vectors[3]
        } else {
            feet[current].states[0].vectors[1]
        };
        let mut metrics = stride_metrics(feet[current], feet[next], velocity, numeric)?;
        if feet[current].stride_initialized && !matches!(original_action, 0 | 1 | 9) {
            let speed = numeric
                .dot_vector(
                    feet[current].states[initial_state as usize].vectors[1],
                    metrics.direction,
                )
                .map_err(E::Numeric)?;
            let fields = self.before_sprint.as_mut().unwrap();
            if speed < fields.speed {
                fields.speed = speed;
            }
        }
        let previous_flags = self.update_flags(0x202)?;
        self.write_update_flags(0x200, 0)?;
        self.write_update_flags(2, 0)?;
        let last_start = self.before_sprint.as_ref().unwrap().last_start_state;
        let mut speed = self.before_sprint.as_ref().unwrap().speed;
        let mut stage = self.before_face.as_ref().unwrap().current_stage;
        let mut state = initial_state;
        let mut transitions = if initial_state == 0 { -1 } else { 0 };
        let mut action = -1;
        let mut projected_action = -1;
        let mut retained_stride = [0.0; 4];
        for offset in 0..12 {
            let feet = &self.skater.as_ref().unwrap().trajectory.physical_feet;
            if transitions < 2 && feet[current].initialized && feet[next].initialized {
                action = feet[current].states[state as usize].action;
                if (offset > 0 && initial_state < 2) || (offset > 1 && initial_state == 2) {
                    self.update_stride_attributes(
                        stage,
                        [
                            metrics.previous_y,
                            metrics.next_y,
                            metrics.previous_length,
                            metrics.next_length,
                        ],
                    )?;
                    self.update_attribute("YSpeed", metrics.vertical_speed, stage)?;
                    self.update_attribute("ZSpeed", metrics.forward_speed, stage)?;
                    let old = action;
                    if action == 1 {
                        projected_action = 1;
                    } else {
                        action = super::locomotion_action::try_calc_locomotion_action(
                            &mut projected_action,
                            state,
                            metrics.previous_y,
                            metrics.previous_length,
                            metrics.next_y,
                            metrics.next_length,
                            // Raw caller8f60/64 crosses geometric prev/next normals.
                            metrics.next_normal,
                            metrics.previous_normal,
                            metrics.vertical_speed,
                            metrics.forward_speed,
                            cvars,
                        )
                        .map_err(E::Cvar)?;
                        self.skater
                            .as_mut()
                            .unwrap()
                            .trajectory
                            .try_set_action(action, state, (offset - 1) as u32, 1)
                            .map_err(E::Trajectory)?;
                    }
                    if matches!(old, 12 | 13 | 14 | 15 | 16) && action != 3 {
                        action = old;
                        self.skater
                            .as_mut()
                            .unwrap()
                            .trajectory
                            .try_set_action(action, state, (offset - 1) as u32, 1)
                            .map_err(E::Trajectory)?;
                    }
                } else {
                    projected_action = if action == 6 && state != 2 { 4 } else { -1 };
                }
                retained_stride = [
                    metrics.previous_y,
                    metrics.next_y,
                    metrics.previous_length,
                    metrics.next_length,
                ];
            } else {
                self.update_stride_attributes(stage, retained_stride)?;
            }
            if state != 2 && state != last_start {
                let mut desired = self.before_face.as_ref().unwrap().desired_speed;
                if self.update_flags(0x10)? != 0 && matches!(action, 0 | 9) {
                    if desired > self.before_sprint.as_ref().unwrap().speed {
                        let maximum = cvars.try_loco_update_max_accel().map_err(E::Cvar)?;
                        if !(desired - speed < maximum) {
                            desired = maximum + speed;
                        }
                    } else if desired <= 0.0 {
                        // 1.02 82a9a0ec: from zero down, where 1.00 needed below -0.1.
                        desired =
                            speed - cvars.try_loco_update_max_decel_stop().map_err(E::Cvar)?;
                    } else {
                        let maximum = cvars.try_loco_update_max_decel().map_err(E::Cvar)?;
                        if !(speed - desired < maximum) {
                            desired = speed - maximum;
                        }
                    }
                }
                let lower = if -desired >= 0.0 { 0.0 } else { desired };
                speed = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
                if offset == 0 {
                    self.before_sprint.as_mut().unwrap().speed = speed;
                }
            }
            self.update_attribute("NormalizedSpeed", speed, stage)?;
            let name = match action {
                0 => {
                    let fields = self.before_sprint.as_ref().unwrap();
                    if fields.second_flags_known_mask & 0x80000000 == 0 {
                        return Err(E::UnknownFlags(0x80000000));
                    }
                    if fields.second_flags_bits & 0x80000000 == 0 {
                        "RunNB"
                    } else {
                        "Run"
                    }
                }
                1 | 3 => {
                    self.write_update_flags(0x200, previous_flags)?;
                    if action == 1 { "Jump" } else { "AutoJump" }
                }
                4 => "Land",
                5 => "Land1FT",
                6 | 14 => "LandOut",
                7 => "StairsDN",
                8 => "StairsUP",
                9 => "InclineDN",
                10 => "InclineUP",
                12 => "Dismount",
                13 => {
                    self.write_update_flags(2, previous_flags)?;
                    if previous_flags & 2 != 0 {
                        "Land"
                    } else {
                        "Land1FT"
                    }
                }
                15 => "RunoutFront",
                16 => "RunoutBack",
                _ => "",
            };
            if cvars.try_loco_update_general_debug().map_err(E::Cvar)? {
                overlay.require().map_err(E::Overlay)?;
            }
            if action != -1 {
                self.update_attribute(name, 0.0, stage)?;
            }
            stage = (stage + 1) % 12;
            match state {
                0 => {
                    state = 1;
                    transitions += 1;
                }
                1 => state = 2,
                2 => {
                    let feet = &self.skater.as_ref().unwrap().trajectory.physical_feet;
                    let velocity = feet[current].states[2].vectors[3];
                    current = next;
                    next = (next + 1) % 4;
                    state = 0;
                    metrics = stride_metrics(feet[current], feet[next], velocity, numeric)?;
                }
                _ => return Err(E::Index(state)),
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_animation_construction::{
        ActorAnimationCvars, ActorAnimationInputs, ActorAnimationInterface, ActorAnimationRegistry,
        AnimationLifetime,
    };
    use crate::input_cvar_runtime::InputCvarManager;
    use crate::motion_graph_entity_authority::{ActorGraphAuthority, MotionGraphEntityAuthority};
    use std::sync::Arc;
    struct Registry;
    impl ActorAnimationRegistry<u32> for Registry {
        type Error = Infallible;
        fn register_animation_interface(
            &mut self,
            _: &MotionGraphEntityAuthority<u32>,
            _: ActorAnimationInterface,
            _: &Arc<()>,
        ) -> Result<(), Infallible> {
            Ok(())
        }
        fn actor_player_index(
            &mut self,
            _: &MotionGraphEntityAuthority<u32>,
        ) -> Result<u32, Infallible> {
            Ok(0)
        }
    }
    struct Sprint;
    impl ActorAnimationCvars for Sprint {
        type Error = Infallible;
        fn use_sprint_button(&mut self) -> Result<u8, Infallible> {
            Ok(0)
        }
    }
    fn owner() -> ActorAnimationConstruction<u32, ()> {
        let authority = ActorGraphAuthority::new(0);
        let mut owner = ActorAnimationConstruction::new(
            &authority,
            ActorAnimationInputs {
                physics_world: Arc::new(()),
                life: AnimationLifetime::Transient,
                use_low_blend_cull_threshold: false,
                face_name: None,
            },
        );
        owner.try_mark_sim_allocation().unwrap();
        owner.try_construct_prefix(&mut Registry).unwrap();
        let vault = crate::attribute_runtime::tests::graph(
            crate::actor_animation_construction::ACTOR_ANIMATION_OFFBOARD_CLASS,
            Vec::new(),
        );
        owner
            .try_construct_vault_suffix(Arc::new(vault), &mut Sprint)
            .unwrap();
        owner
    }
    #[test]
    fn pending_first_future_retains_time_publication_without_sampling_cvars() {
        let mut owner = owner();
        let trajectory = &mut owner.skater.as_mut().unwrap().trajectory;
        trajectory.current_state = 0;
        trajectory.time = 1.0;
        for (state, time) in trajectory.physical_feet[0]
            .states
            .iter_mut()
            .zip([2.0, 3.0, 4.0])
        {
            state.time = time;
        }
        let refs = InputCvarReferences::new_static_initialized();
        assert!(matches!(
            owner.try_update_locomotion_trajectory(
                1.0 / 60.0,
                &refs,
                &DebugOverlaySlot::default(),
                &mut HostReckoningNumeric,
                &mut HostReckoningSinCosNumeric
            ),
            Err(E::FutureNotReady(0))
        ));
        assert_eq!(owner.before_face.as_ref().unwrap().time_to_foot_change, 8.0);
    }
    #[test]
    fn invalid_first_skips_unready_second_and_preserves_fallback_fields() {
        let mut owner = owner();
        owner.skater.as_mut().unwrap().step_results[0].ready = true;
        owner.before_face.as_mut().unwrap().queried_feet = [
            V {
                xyz: [1.0, 2.0, 3.0],
                w: None,
            },
            V {
                xyz: [4.0, 5.0, 6.0],
                w: None,
            },
        ];
        let (steps, valid) = owner
            .update_selected_steps(
                &InputCvarReferences::new_static_initialized(),
                &mut HostReckoningNumeric,
            )
            .unwrap();
        assert!(!valid);
        assert_eq!(steps[0].position.w, None);
        assert_eq!(steps[1].position.xyz, [4.0, 5.0, 6.0]);
        assert_eq!(steps[1].normal, UP);
        assert_eq!(
            owner.before_sprint.as_ref().unwrap().step_failure_counts,
            [1, 1]
        );
    }
    #[test]
    fn second_future_failure_keeps_first_step_and_validity_publication() {
        let mut owner = owner();
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        let first = &mut owner.skater.as_mut().unwrap().step_results[0];
        first.ready = true;
        first.result.valid = true;
        first.result.position = V {
            xyz: [1.0, 2.0, 3.0],
            w: None,
        };
        first.result.normal = UP;
        first.result.surface = 77;
        assert!(matches!(
            owner.update_selected_steps(&refs, &mut HostReckoningNumeric),
            Err(E::FutureNotReady(1))
        ));
        assert_eq!(owner.skater.as_ref().unwrap().last_steps[0].surface, 77);
        assert_eq!(owner.update_flags(0x100).unwrap(), 0x100);
        assert_eq!(
            owner.before_sprint.as_ref().unwrap().step_failure_counts[1],
            0
        );
        manager
            .try_deserialize_startup(Some(b"[Config]\nkEnableLocomotionFutureAssert=0\n"))
            .unwrap();
        assert!(
            !owner
                .update_selected_steps(&refs, &mut HostReckoningNumeric)
                .unwrap()
                .1
        );
    }
    #[test]
    fn stage_loop_publishes_all_twelve_distinct_sequence_ids() {
        let mut owner = owner();
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        owner.before_face.as_mut().unwrap().current_stage = 10;
        owner.before_face.as_mut().unwrap().desired_speed = 0.25;
        owner
            .update_locomotion_stages(
                0,
                0,
                -1,
                &refs,
                &DebugOverlaySlot::default(),
                &mut HostReckoningNumeric,
            )
            .unwrap();
        let attrs = &owner.base.as_ref().unwrap().settable_attributes;
        assert_eq!(attrs.len, 84);
        let stages = attrs.slots[..attrs.len]
            .iter()
            .filter_map(|row| {
                let row = row.as_ref().unwrap();
                (row.name == FastString30::encode("NormalizedSpeed").unwrap())
                    .then_some(row.sequence_id)
            })
            .collect::<Vec<_>>();
        assert_eq!(stages, vec![10, 11, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(owner.before_face.as_ref().unwrap().current_stage, 10);
    }
}
