//! Retained Andale node Eval calls. Commands are accepted before node stores.
use super::{AnimationNodeHandle, AnimationNodeKind, AnimationTreeError, AnimationTreePool};
use crate::{
    actor_face_playback::tree_database::clip_fetch::{ClipFetchError, ClipFetchPlan},
    actor_face_playback::{AnimationPoseRef, FaceCommands},
};
use std::sync::{Arc, Mutex};

pub(crate) struct AnimationEvalParams {
    pub(crate) commands: Arc<Mutex<FaceCommands>>,
    pub(crate) update_old_time: bool,
    pub(crate) blend_frames: u32,
    pub(crate) blend_cull_threshold: f32,
    pub(crate) blend_space_cvars: Option<crate::input_cvar_runtime::InputCvarReferences>,
}

/// Each method is one original ACS call, including its buffer acceptance.
pub(crate) trait AnimationEvalCommands {
    type Error;
    fn fetch(&mut self, plan: ClipFetchPlan) -> Result<(), Self::Error>;
    fn fetch_pose(&mut self, pose: &AnimationPoseRef) -> Result<(), Self::Error>;
    /// Optional command capability; absent implementations are rejected by
    /// the evaluator before publishing a fixed-SQT node as evaluated.
    fn fixed_sqt(
        &mut self,
        _owner: &Arc<Mutex<FaceCommands>>,
        _id: u32,
    ) -> Option<Result<(), Self::Error>> {
        None
    }
    fn external_sqt(&mut self, _source: &crate::actor_animation_construction::initialization::bind_pose::sqt_output::OwnedAnimationSqtBuffer)
        -> Option<Result<(),Self::Error>> {None}
    fn n_blend(&mut self, coefficients: &[f32]) -> Result<(), Self::Error>;
    fn blend(&mut self, coefficient: f32) -> Result<(), Self::Error>;
    fn channel_blend(
        &mut self,
        coefficient: f32,
        channel_from_first: bool,
    ) -> Result<(), Self::Error>;
    fn add_sqt(&mut self, motion_a: bool) -> Result<(), Self::Error>;
    fn mirror(&mut self, trajectory_use: u32) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub(crate) enum AnimationEvalError<E> {
    Tree(AnimationTreeError),
    Fetch(ClipFetchError),
    Command(E),
    MissingOwner(&'static str),
    InvalidChildResult,
    InvalidCoefficient,
    Cvar(crate::input_cvar_runtime::InputCvarError),
}
impl<E> From<AnimationTreeError> for AnimationEvalError<E> {
    fn from(error: AnimationTreeError) -> Self {
        Self::Tree(error)
    }
}
impl<E: std::fmt::Debug> std::fmt::Display for AnimationEvalError<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tree(error) => write!(formatter, "tree Eval: {error}"),
            Self::Fetch(error) => write!(formatter, "tree Eval fetch: {error}"),
            Self::Cvar(error) => write!(formatter, "tree Eval cvar: {error:?}"),
            Self::Command(error) => write!(formatter, "tree Eval command: {error:?}"),
            Self::MissingOwner(owner) => write!(formatter, "tree Eval requires {owner}"),
            other => write!(formatter, "tree Eval: {other:?}"),
        }
    }
}

impl AnimationTreePool {
    /// `Clip82d6ef78`, `PhaseBlend82d6e9d8`, `Pose825852d0`,
    /// `AddSQT82588308`, `Mirror82c92818`.
    #[expect(
        clippy::too_many_lines,
        reason = "keep node visitation, command acceptance and evaluation stores in source order"
    )]
    pub(crate) fn eval<C: AnimationEvalCommands>(
        &mut self,
        handle: &AnimationNodeHandle,
        params: &AnimationEvalParams,
        do_eval: bool,
        commands: &mut C,
    ) -> Result<bool, AnimationEvalError<C::Error>> {
        use AnimationEvalError as E;
        // Copy only the references/scalars needed across recursive mutation.
        enum Visit {
            Clip(bool),
            Pose(AnimationPoseRef),
            FixedSqt(u32),
            Phase(AnimationNodeHandle, AnimationNodeHandle, f32),
            Add(AnimationNodeHandle, AnimationNodeHandle, bool),
            Mirror(AnimationNodeHandle, Option<u32>),
        }
        let visit = match &self.node(handle)?.kind {
            AnimationNodeKind::Sequence(_) => {
                return self.eval_sequence(handle, params, do_eval, commands);
            }
            AnimationNodeKind::Selector(_) => {
                return self.eval_selector(handle, params, do_eval, commands);
            }
            AnimationNodeKind::BlendSpace(node) => {
                if node.trimmed {
                    return Ok(false);
                }
                let count = node.active_count()?;
                if crate::diagnostics::lean_trace::enabled() {
                    crate::diagnostics::lean_trace::record(
                        "blend_query",
                        Arc::as_ptr(&handle.pool) as usize,
                        handle.index as u64,
                        &node.attributes,
                    );
                    crate::diagnostics::lean_trace::record(
                        "blend_volume",
                        Arc::as_ptr(&handle.pool) as usize,
                        handle.index as u64,
                        &[node.volume_set as f32, node.volume as f32, node.time],
                    );
                }
                for index in 0..count {
                    let child = self.blend_space(handle)?.active_child(index)?;
                    self.eval(&child, params, true, commands)?;
                }
                let coefficients = self
                    .blend_space(handle)?
                    .parameters
                    .lock()
                    .map_err(|_| AnimationTreeError::Lock)?
                    .coefficients
                    .clone();
                crate::diagnostics::lean_trace::record(
                    "blend_weights",
                    Arc::as_ptr(&handle.pool) as usize,
                    handle.index as u64,
                    &coefficients[..count],
                );
                commands
                    .n_blend(&coefficients[..count])
                    .map_err(E::Command)?;
                let cvars = params
                    .blend_space_cvars
                    .as_ref()
                    .ok_or(E::MissingOwner("BlendSpace Eval cvars"))?;
                if cvars.try_blend_space_print().map_err(E::Cvar)? {
                    return Err(AnimationTreeError::PendingBlendSpaceOperation(
                        "Eval debug overlay",
                    )
                    .into());
                }
                // Global debugLine increment remains the existing explicit debug-state debt.
                return Ok(true);
            }
            AnimationNodeKind::Clip(clip) => {
                clip.header.as_ref().ok_or(E::MissingOwner("Clip header"))?;
                if crate::diagnostics::lean_trace::enabled() {
                    crate::diagnostics::lean_trace::record(
                        "clip",
                        Arc::as_ptr(&handle.pool) as usize,
                        handle.index as u64,
                        &[
                            clip.header.as_ref().map_or(0, |h| h.record_index) as f32,
                            clip.time,
                            clip.old_time,
                            clip.length,
                            clip.speed,
                            clip.loops as f32,
                            f32::from(u8::from(do_eval)),
                            f32::from(u8::from(clip.trimmed)),
                        ],
                    );
                }
                Visit::Clip(clip.trimmed)
            }
            AnimationNodeKind::Pose { header } => {
                Visit::Pose(header.clone().ok_or(E::MissingOwner("Pose header"))?)
            }
            AnimationNodeKind::FixedSqt { id } => Visit::FixedSqt(*id),
            AnimationNodeKind::PhaseBlend(phase) => {
                let first = phase
                    .child1
                    .clone()
                    .ok_or(E::MissingOwner("PhaseBlend first child"))?;
                let second = phase
                    .child2
                    .clone()
                    .ok_or(E::MissingOwner("PhaseBlend second child"))?;
                let coefficient = phase_coefficient(
                    phase.blend,
                    params.blend_cull_threshold,
                    phase
                        .children
                        .first()
                        .is_some_and(|(_, node)| *node == first),
                    phase
                        .children
                        .last()
                        .is_some_and(|(_, node)| *node == second),
                );
                Visit::Phase(first, second, coefficient)
            }
            AnimationNodeKind::AddSqt { a, b, motion } => {
                if motion == a {
                    Visit::Add(a.clone(), b.clone(), true)
                } else {
                    Visit::Add(b.clone(), a.clone(), false)
                }
            }
            AnimationNodeKind::Mirror {
                child,
                trajectory_use,
                ..
            } => Visit::Mirror(
                child.clone().ok_or(E::MissingOwner("Mirror child"))?,
                *trajectory_use,
            ),
        };
        let evaluated = match visit {
            Visit::Clip(trimmed) => {
                let evaluated = do_eval && !trimmed;
                if evaluated {
                    let plan = self
                        .prepare_clip_fetch(handle, &params.commands, params.blend_frames)
                        .map_err(E::Fetch)?;
                    commands.fetch(plan).map_err(E::Command)?;
                }
                if params.update_old_time {
                    let AnimationNodeKind::Clip(clip) = &mut self.node_mut(handle)?.kind else {
                        unreachable!()
                    };
                    clip.old_time = clip.time;
                    clip.loops = 0;
                }
                evaluated
            }
            Visit::Pose(pose) => {
                if do_eval {
                    commands.fetch_pose(&pose).map_err(E::Command)?;
                }
                do_eval
            }
            Visit::FixedSqt(id) => {
                if do_eval {
                    commands
                        .fixed_sqt(&params.commands, id)
                        .ok_or(E::MissingOwner("SetSQTById command capability"))?
                        .map_err(E::Command)?;
                }
                do_eval
            }
            Visit::Phase(first, second, coefficient) => {
                crate::diagnostics::lean_trace::record(
                    "phase_blend",
                    Arc::as_ptr(&handle.pool) as usize,
                    handle.index as u64,
                    &[
                        coefficient,
                        first.index as f32,
                        second.index as f32,
                        f32::from(u8::from(do_eval)),
                    ],
                );
                let AnimationNodeKind::PhaseBlend(phase) = &mut self.node_mut(handle)?.kind else {
                    unreachable!()
                };
                phase.cull_threshold = params.blend_cull_threshold;
                // Ordered BLT/BGT: unordered coefficients gate both children off.
                let first_gate = do_eval && coefficient < 1.0;
                let first_result = self.eval(&first, params, first_gate, commands)?;
                if first_result && !first_gate {
                    return Err(E::InvalidChildResult);
                }
                let second_gate = do_eval && coefficient > 0.0;
                let second_result = self.eval(&second, params, second_gate, commands)?;
                if second_result && !second_gate {
                    return Err(E::InvalidChildResult);
                }
                if first_result && second_result {
                    commands.blend(coefficient).map_err(E::Command)?;
                }
                first_result || second_result
            }
            Visit::Add(motion, other, motion_a) => {
                let result = self.eval(&motion, params, do_eval, commands)?;
                if result {
                    if !do_eval {
                        return Err(E::InvalidChildResult);
                    }
                    if !self.eval(&other, params, do_eval, commands)? {
                        return Err(E::InvalidChildResult);
                    }
                    commands.add_sqt(motion_a).map_err(E::Command)?;
                }
                result
            }
            Visit::Mirror(child, trajectory_use) => {
                crate::diagnostics::lean_trace::record(
                    "mirror",
                    Arc::as_ptr(&handle.pool) as usize,
                    handle.index as u64,
                    &[child.index as f32, f32::from(u8::from(do_eval))],
                );
                let result = self.eval(&child, params, do_eval, commands)?;
                if do_eval && result {
                    commands
                        .mirror(trajectory_use.ok_or(E::MissingOwner("Mirror trajectory use"))?)
                        .map_err(E::Command)?;
                }
                // Unlike all other nodes here, Mirror has no mWasEvaluated store.
                return Ok(result);
            }
        };
        self.node_mut(handle)?.evaluated = evaluated;
        Ok(evaluated)
    }
}

fn select(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}

/// Raw `82d6ea34..ead4`: preserve scalar FSEL and fused smoothing order.
fn phase_coefficient(blend: f32, threshold: f32, first_endpoint: bool, last_endpoint: bool) -> f32 {
    let lower = select(threshold - blend, threshold, blend);
    let upper = 1.0 - threshold;
    let denominator = -(threshold.mul_add(2.0, -1.0));
    let capped = select(upper - lower, lower, upper);
    let normalized = (capped - threshold) / denominator;
    let squared = normalized * normalized;
    if normalized > 0.5 {
        if last_endpoint {
            let four = normalized * 4.0;
            (8.0 - four).mul_add(squared, -four) + 1.0
        } else {
            blend
        }
    } else if first_endpoint {
        (-(normalized.mul_add(4.0, -4.0))) * squared
    } else {
        blend
    }
}

#[cfg(test)]
#[path = "actor_animation_tree_eval_tests.rs"]
mod tests;
