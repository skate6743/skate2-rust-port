//! Actual retained-tree `EvalBlendPose` and `SetUpBlendPoseStream825f8398`.
//! Unregistered draft: shared ACS/SQT allocation amendments are coordinated first.
use super::channel_runtime::ChannelBaseReference;
use super::{ActorAnimationConstruction, AnimationPointerState};
use crate::{
    actor_animation_tree::eval::AnimationEvalCommands,
    actor_face_playback::{
        AnimationPoseRef, FacePlaybackData,
        blend_pose_stream::{
            BlendPoseBuffer, BlendPoseError, BlendPoseTokenPool, decode_coefficient,
        },
    },
    actor_graph_construction::ActorGraphIntents,
    play_animation::FastString36,
};

#[path = "actor_animation_hand_face_commands.rs"]
pub(crate) mod commands;
#[path = "actor_animation_hands_face_runtime.rs"]
pub(crate) mod runtime;
#[path = "../pose/actor_animation_sqt_dispatch_jobs.rs"]
pub(crate) mod sqt_jobs;
#[cfg(test)]
#[path = "actor_animation_hands_face_dispatch_tests.rs"]
mod tests;

#[derive(Debug)]
pub(crate) enum HandFaceDispatchError<E> {
    Tokens(BlendPoseError),
    Command(E),
    Missing(&'static str),
    Animation(crate::actor_face_playback::ActorFaceError),
    TokenAssertion,
}
impl<E> From<BlendPoseError> for HandFaceDispatchError<E> {
    fn from(value: BlendPoseError) -> Self {
        Self::Tokens(value)
    }
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// Animatable82d6bb68 uses the current `ChannelAnimatable` root when present;
    /// a null root reaches `Pose::EvalBlendPose82587b98`'s source assertion.
    pub(crate) fn try_eval_hand_face_tokens(
        &self,
        buffer: &mut BlendPoseBuffer,
        pool: &mut BlendPoseTokenPool,
    ) -> Result<(), BlendPoseError> {
        let initialized = self
            .initialization
            .as_ref()
            .ok_or(BlendPoseError::Missing("Initialize"))?;
        let channels = initialized
            .channels
            .as_ref()
            .ok_or(BlendPoseError::Missing("channels"))?;
        let reference = if let Some(root) = channels.root {
            ChannelBaseReference::Channel(root)
        } else {
            let base = self
                .base
                .as_ref()
                .ok_or(BlendPoseError::Missing("Animatable"))?;
            if base.base_animation == AnimationPointerState::Null {
                return Err(BlendPoseError::PoseNodeAssertion);
            }
            ChannelBaseReference::Body(
                base.body_root
                    .clone()
                    .ok_or(BlendPoseError::Missing("body root"))?,
            )
        };
        self.eval_blend_reference(&reference, buffer, pool)
    }

    /// `ChannelBlend82d79bf0`: base first; positive coefficient evaluates the same
    /// retained channel tree into the other token pool and merges its streams.
    fn eval_blend_reference(
        &self,
        reference: &ChannelBaseReference,
        buffer: &mut BlendPoseBuffer,
        pool: &mut BlendPoseTokenPool,
    ) -> Result<(), BlendPoseError> {
        let initialized = self
            .initialization
            .as_ref()
            .ok_or(BlendPoseError::Missing("Initialize"))?;
        match reference {
            ChannelBaseReference::Empty => Err(BlendPoseError::Missing("channel base animation")),
            ChannelBaseReference::Body(root) => self.eval_body_blend_pose(root, buffer, pool),
            ChannelBaseReference::Channel(index) => {
                let channels = initialized
                    .channels
                    .as_ref()
                    .ok_or(BlendPoseError::Missing("channels"))?;
                let channel = channels
                    .channels
                    .get(*index)
                    .ok_or(BlendPoseError::Missing("channel"))?;
                let runtime = channel
                    .runtime
                    .as_ref()
                    .ok_or(BlendPoseError::Missing("channel runtime"))?;
                self.eval_blend_reference(&runtime.base, buffer, pool)?;
                let coefficient = channel
                    .blend_coefficient
                    .ok_or(BlendPoseError::Missing("channel blend coefficient"))?;
                if coefficient > 0.0 {
                    let mut temporary = pool.buffer(1 - buffer.pool_index)?;
                    super::channel_runtime::transition::eval_blend_pose(
                        channel,
                        &channels.trees,
                        &mut temporary,
                        pool,
                    )?;
                    pool.blend(buffer, &temporary, coefficient)?;
                }
                Ok(())
            }
        }
    }
}

/// These are the additional native ACS calls, not an alternate pose evaluator.
/// The concrete adapter must append to the existing `TreeCommands` owner and use
/// its existing SQT job execution. Fixed IDs address SAME `FaceCommands` frames.
pub(crate) trait HandFaceStreamCommands: AnimationEvalCommands {
    fn begin_stream(&mut self) -> Result<(), Self::Error>;
    fn pose_frame(&mut self, pose: u32) -> Result<(), Self::Error>;
    fn extract_sqt(&mut self) -> Result<(), Self::Error>;
    fn end_stream(&mut self) -> Result<(), Self::Error>;
}

/// SetUpBlendPoseStream825f846c..87a8. `playback` is the actual manager lookup;
/// source command Begin precedes stream lookup and authored fallback pose lookup.
pub(crate) fn emit_hand_face_stream<C: HandFaceStreamCommands>(
    buffer: &BlendPoseBuffer,
    pool: &BlendPoseTokenPool,
    part: &FastString36,
    playback: &FacePlaybackData,
    clamp: i32,
    add_bind_pose: bool,
    commands: &mut C,
) -> Result<(), HandFaceDispatchError<C::Error>> {
    commands
        .begin_stream()
        .map_err(HandFaceDispatchError::Command)?;
    let stream = buffer.stream_index(part);
    let mut poses = 0_i32;
    if let Some(index) = stream {
        let tokens = pool.tokens(buffer, index)?;
        poses = emit_tokens(tokens, clamp, commands).inspect_err(|_| {
            if std::env::var_os("SKATE2_TRACE_HAND_TOKENS").is_some() {
                let words: Vec<String> = tokens.iter().map(|t| format!("{t:08x}")).collect();
                eprintln!("hand stream tokens rejected: {}", words.join(" "));
            }
        })?;
    }
    if add_bind_pose {
        let pose = required_pose(playback, b"rig_TPose")?;
        commands
            .fetch_pose(&pose)
            .map_err(HandFaceDispatchError::Command)?;
        if poses != 0 {
            commands
                .add_sqt(true)
                .map_err(HandFaceDispatchError::Command)?;
        }
    } else if stream.is_none() || poses == 0 {
        let pose = required_pose(playback, b"IdentityPose")?;
        commands
            .fetch_pose(&pose)
            .map_err(HandFaceDispatchError::Command)?;
    }
    commands
        .extract_sqt()
        .map_err(HandFaceDispatchError::Command)?;
    commands
        .end_stream()
        .map_err(HandFaceDispatchError::Command)
}

fn required_pose<E>(
    playback: &FacePlaybackData,
    name: &[u8],
) -> Result<AnimationPoseRef, HandFaceDispatchError<E>> {
    playback
        .get_pose(ActorGraphIntents::native_name(name))
        .map_err(HandFaceDispatchError::Animation)?
        .ok_or(HandFaceDispatchError::Missing(
            "authored stream fallback pose",
        ))
}

fn emit_tokens<C: HandFaceStreamCommands>(
    tokens: &[u32],
    clamp: i32,
    commands: &mut C,
) -> Result<i32, HandFaceDispatchError<C::Error>> {
    let mut coefficients = [0.0; 10];
    let mut coefficient_count = 0;
    let mut stack_depth = 0_u32;
    let mut poses = 0_i32;
    for &token in tokens {
        match token >> 30 {
            0 => {
                commands
                    .pose_frame((token >> 23) & 0x7f)
                    .map_err(HandFaceDispatchError::Command)?;
                stack_depth = stack_depth.wrapping_add(1);
                poses = poses.wrapping_add(1);
            }
            1 => {
                *coefficients
                    .get_mut(coefficient_count)
                    .ok_or(HandFaceDispatchError::TokenAssertion)? = decode_coefficient(token);
                coefficient_count += 1;
            }
            _ => {
                let dimensions = ((token >> 20) & 7) as usize;
                if stack_depth > 1 && coefficient_count == 1 {
                    if dimensions != 2 {
                        return Err(HandFaceDispatchError::TokenAssertion);
                    }
                    commands
                        .blend(coefficients[0])
                        .map_err(HandFaceDispatchError::Command)?;
                    stack_depth -= 1;
                } else if coefficient_count > 1 {
                    if coefficient_count != dimensions {
                        return Err(HandFaceDispatchError::TokenAssertion);
                    }
                    commands
                        .n_blend(&coefficients[..coefficient_count])
                        .map_err(HandFaceDispatchError::Command)?;
                    stack_depth = stack_depth
                        .wrapping_sub(
                            u32::try_from(coefficient_count)
                                .map_err(|_| HandFaceDispatchError::TokenAssertion)?,
                        )
                        .wrapping_add(1);
                } else {
                    return Err(HandFaceDispatchError::TokenAssertion);
                }
                coefficient_count = 0;
                // Clamp is tested only after a blend reduces the stack. It is
                // strict GT; it is not a maximum number of poses or tokens.
                if poses > clamp && stack_depth < 2 {
                    break;
                }
            }
        }
    }
    Ok(poses)
}
