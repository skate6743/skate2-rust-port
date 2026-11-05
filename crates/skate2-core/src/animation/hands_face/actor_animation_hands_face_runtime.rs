//! Actor825f87b8 source dispatch order; manager owns outer hand command sets.
use super::commands::StreamCommands;
use super::{
    ActorAnimationConstruction, AnimationPointerState, HandFaceDispatchError, emit_hand_face_stream,
};
use crate::{
    actor_animation_construction::initialization::{
        ActorAnimationInitializeSource, bind_pose::BindPoseError,
    },
    actor_face_playback::{
        ActorFaceError, FaceCommands, FacePlaybackManager,
        blend_pose_stream::{BlendPoseBuffer, BlendPoseError, BlendPoseTokenPool},
    },
    actor_graph_construction::ActorGraphIntents,
};
use std::sync::{Arc, Mutex};

pub(crate) trait HandFaceDispatchSource: ActorAnimationInitializeSource {
    fn is_local_player(&mut self) -> Result<bool, Self::Error>;
    fn hands_commands_clamp(&mut self) -> Result<i32, Self::Error>;
    fn animation_playback_manager(
        &mut self,
    ) -> Result<Arc<Mutex<FacePlaybackManager>>, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum HandFaceRuntimeError<E> {
    Source(E),
    Animation(ActorFaceError),
    BindPose(BindPoseError),
    Execution(crate::animation_job_owners::execution::AnimationExecutionError),
    Evaluation(Arc<BindPoseError>),
    Stream(HandFaceDispatchError<BindPoseError>),
    Tokens(BlendPoseError),
    Missing(&'static str),
    Gesture(String),
}
impl<E> From<ActorFaceError> for HandFaceRuntimeError<E> {
    fn from(e: ActorFaceError) -> Self {
        Self::Animation(e)
    }
}
impl<E> From<BindPoseError> for HandFaceRuntimeError<E> {
    fn from(e: BindPoseError) -> Self {
        Self::BindPose(e)
    }
}
impl<E> From<crate::animation_job_owners::execution::AnimationExecutionError>
    for HandFaceRuntimeError<E>
{
    fn from(error: crate::animation_job_owners::execution::AnimationExecutionError) -> Self {
        Self::Execution(error)
    }
}
impl<E> From<BlendPoseError> for HandFaceRuntimeError<E> {
    fn from(e: BlendPoseError) -> Self {
        Self::Tokens(e)
    }
}

#[derive(Clone, Copy)]
enum Target {
    Face,
    Left,
    Right,
}
impl<K, W> ActorAnimationConstruction<K, W> {
    /// PC port: the strongest blend of this Actor's own gesture channels (the Skate 2 set's).
    #[cfg(feature="skate3-runtime")]
    fn skate2_gesture_weight(&self)->f32 {
        let Some(channels)=self.initialization().and_then(|init|init.channels.as_ref()) else {return 0.};
        [b"GestureBoth".as_slice(),b"GestureLeft",b"GestureRight"].iter()
            .filter_map(|name|channels.dictionary.get(&ActorGraphIntents::native_name(name)))
            .filter_map(|&index|channels.channels.get(index).and_then(|channel|channel.blend_coefficient))
            .fold(0.,f32::max).clamp(0.,1.)
    }

    /// Manager begins left/right command sets before this actual Actor call and
    /// submits them afterwards. Face set is begun/ended by this function itself.
    pub(crate) fn try_dispatch_hands_face<S: HandFaceDispatchSource>(
        &mut self,
        lod: i32,
        pool: &mut BlendPoseTokenPool,
        source: &mut S,
    ) -> Result<(), HandFaceRuntimeError<S::Error>> {
        let face_commands = self.face_command_owner()?;
        if let Some(commands) = &face_commands {
            begin_command_set(commands)?;
        }
        pool.reset();
        let mut buffer = pool.buffer(0)?;
        let mut clamp = 10000;
        let player = self
            .player
            .as_mut()
            .ok_or(HandFaceRuntimeError::Missing("SkaterAnim player flags"))?;
        player.flags_bits &= 0xfc7f_ffff;
        player.flags_known_mask |= 0x0380_0000;
        let local = source
            .is_local_player()
            .map_err(HandFaceRuntimeError::Source)?;
        if lod < 6 && (local || lod < 3) {
            self.try_eval_hand_face_tokens(&mut buffer, pool)?;
            #[cfg(feature="skate3-runtime")]
            if let Some(gesture)=&self.pc_gesture_playback {
                if let Some(commands)=&self.mode_body_commands {
                    // A Skate 2 set gesture plays on this Actor's own gesture channels, whose
                    // finger cues are already in the buffer: the Skate 3 body's cues cover only
                    // what the gesture does not.
                    let weight=1.-self.skate2_gesture_weight();
                    gesture.apply_body_hand_streams(commands,weight,&mut buffer,pool).map_err(HandFaceRuntimeError::Gesture)?;
                }
                gesture.apply_hand_face_streams(&mut buffer,pool).map_err(HandFaceRuntimeError::Gesture)?;
                static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
                if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_HANDS").is_some()) {
                    let mut streams = Vec::new();
                    for name in [b"LeftHand".as_slice(), b"RightHand"] {
                        let key = crate::actor_graph_construction::ActorGraphIntents::native_name(name);
                        if let Some(stream) = buffer.stream_index(&key) {
                            let poses: Vec<u32> = pool.tokens(&buffer, stream).map(|tokens| tokens.iter()
                                .filter(|token| **token >> 30 == 0).map(|token| (token >> 23) & 0x7f).collect())
                                .unwrap_or_default();
                            streams.push(format!("{}={poses:?}", String::from_utf8_lossy(name)));
                        }
                    }
                    eprintln!("skate3 hand streams: s3_body={} s2_gesture_weight={:.3} {}", self.mode_body_commands.is_some(), self.skate2_gesture_weight(), streams.join(" "));
                }
            }
            if face_commands.is_some() && ((local && lod < 4) || lod == 0) {
                self.emit_selected_stream(
                    Target::Face,
                    &buffer,
                    pool,
                    b"Face",
                    10000,
                    false,
                    source,
                )?;
                self.mark_stream_initialized(0x0080_0000)?;
            }
            if (local && lod == 5) || (!local && lod == 2) {
                clamp = source
                    .hands_commands_clamp()
                    .map_err(HandFaceRuntimeError::Source)?;
            }
            self.emit_selected_stream(
                Target::Left,
                &buffer,
                pool,
                b"LeftHand",
                clamp,
                true,
                source,
            )?;
            self.mark_stream_initialized(0x0200_0000)?;
            self.emit_selected_stream(
                Target::Right,
                &buffer,
                pool,
                b"RightHand",
                clamp,
                true,
                source,
            )?;
            self.mark_stream_initialized(0x0100_0000)?;
        }
        let flags = self
            .player
            .as_ref()
            .ok_or(HandFaceRuntimeError::Missing("SkaterAnim player flags"))?
            .flags_bits;
        if flags & 0x0200_0000 == 0 {
            self.emit_selected_stream(
                Target::Left,
                &buffer,
                pool,
                b"Skateboard_Trucks",
                clamp,
                true,
                source,
            )?;
            self.mark_stream_initialized(0x0200_0000)?;
        }
        let flags = self
            .player
            .as_ref()
            .ok_or(HandFaceRuntimeError::Missing("SkaterAnim player flags"))?
            .flags_bits;
        if flags & 0x0100_0000 == 0 {
            self.emit_selected_stream(
                Target::Right,
                &buffer,
                pool,
                b"Skateboard_Trucks",
                clamp,
                true,
                source,
            )?;
            self.mark_stream_initialized(0x0100_0000)?;
        }
        pool.reset();
        if let Some(commands) = face_commands {
            end_command_set(&commands, source)?;
        }
        Ok(())
    }
    fn mark_stream_initialized<E>(&mut self, mask: u32) -> Result<(), HandFaceRuntimeError<E>> {
        let player = self
            .player
            .as_mut()
            .ok_or(HandFaceRuntimeError::Missing("SkaterAnim player flags"))?;
        player.flags_bits |= mask;
        player.flags_known_mask |= mask;
        Ok(())
    }
    fn face_command_owner<E>(
        &self,
    ) -> Result<Option<Arc<Mutex<FaceCommands>>>, HandFaceRuntimeError<E>> {
        let skater = self
            .skater
            .as_ref()
            .ok_or(HandFaceRuntimeError::Missing("SkaterAnim"))?;
        if skater.command_manager == AnimationPointerState::Null {
            return Ok(None);
        }
        let face = self
            .face
            .as_ref()
            .ok_or(HandFaceRuntimeError::Missing("face command manager"))?;
        let commands = face
            .managers
            .command
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .entries
            .get(&(face.playback_name, 0))
            .cloned()
            .ok_or(HandFaceRuntimeError::Missing("named face ACS"))?;
        Ok(Some(commands))
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "native stream target, token owners and independent setup arguments"
    )]
    fn emit_selected_stream<S: HandFaceDispatchSource>(
        &self,
        target: Target,
        buffer: &BlendPoseBuffer,
        pool: &BlendPoseTokenPool,
        part: &[u8],
        clamp: i32,
        add_bind_pose: bool,
        source: &mut S,
    ) -> Result<(), HandFaceRuntimeError<S::Error>> {
        let suffix = self
            .before_sprint
            .as_ref()
            .ok_or(HandFaceRuntimeError::Missing("SkaterAnim Clip pointers"))?;
        let (name, output, expected, clip) = match target {
            Target::Face => {
                let face = self
                    .face
                    .as_ref()
                    .ok_or(HandFaceRuntimeError::Missing("face"))?;
                (
                    face.playback_name,
                    face.local_pose.clone(),
                    Arc::clone(
                        face.playback
                            .as_ref()
                            .ok_or(HandFaceRuntimeError::Missing("face playback"))?,
                    ),
                    suffix.face_clip,
                )
            }
            Target::Left | Target::Right => {
                let index = usize::from(matches!(target, Target::Right));
                let hand = self
                    .initialization
                    .as_ref()
                    .and_then(|state| state.hands[index].as_ref())
                    .ok_or(HandFaceRuntimeError::Missing("initialized hand"))?;
                let name = if index == 0 {
                    b"SkaterLeftHand".as_slice()
                } else {
                    b"SkaterRightHand".as_slice()
                };
                (
                    ActorGraphIntents::native_name(name),
                    hand.local_pose.clone(),
                    Arc::clone(&hand.playback),
                    if index == 0 {
                        suffix.left_hand_clip
                    } else {
                        suffix.right_hand_clip
                    },
                )
            }
        };
        let manager = source
            .animation_playback_manager()
            .map_err(HandFaceRuntimeError::Source)?;
        let playback = manager
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .entries
            .get(&(name, 0))
            .cloned()
            .ok_or(HandFaceRuntimeError::Missing("named stream playback"))?;
        if !Arc::ptr_eq(&playback, &expected) {
            return Err(HandFaceRuntimeError::Missing(
                "same initialized stream playback",
            ));
        }
        let playback = playback.lock().map_err(|_| ActorFaceError::Lock)?;
        let mut commands = StreamCommands::new(Arc::clone(&playback.commands), output, clip)?;
        emit_hand_face_stream(
            buffer,
            pool,
            &ActorGraphIntents::native_name(part),
            &playback,
            clamp,
            add_bind_pose,
            &mut commands,
        )
        .map_err(HandFaceRuntimeError::Stream)
    }
}

pub(crate) fn begin_command_set(commands: &Arc<Mutex<FaceCommands>>) -> Result<(), BindPoseError> {
    let commands = commands.lock().map_err(|_| ActorFaceError::Lock)?;
    let mut owner = commands
        .buffered
        .as_ref()
        .ok_or(BindPoseError::BufferedOwnerRequired)?
        .lock()
        .map_err(|_| ActorFaceError::Lock)?;
    if owner.in_command_set != 0 || owner.initiated_commands != 0 {
        return Err(BindPoseError::AlreadyInCommands);
    }
    owner.in_command_set += 1;
    Ok(())
}
pub(crate) fn end_command_set<S: ActorAnimationInitializeSource>(
    commands: &Arc<Mutex<FaceCommands>>,
    source: &mut S,
) -> Result<(), HandFaceRuntimeError<S::Error>> {
    // Fixed-only extraction runs during scheduling and reads this same owner.
    let owner = commands
        .lock()
        .map_err(|_| ActorFaceError::Lock)?
        .buffered
        .clone()
        .ok_or(BindPoseError::BufferedOwnerRequired)?;
    owner
        .lock()
        .map_err(|_| ActorFaceError::Lock)?
        .submit_sqt_programs(source)
}

pub(crate) fn sync_command_set<S: ActorAnimationInitializeSource>(
    commands: &Arc<Mutex<FaceCommands>>,
    source: &mut S,
) -> Result<(), HandFaceRuntimeError<S::Error>> {
    // Do not hold the fixed-frame owner while worker jobs resolve its frames.
    let owner = commands
        .lock()
        .map_err(|_| ActorFaceError::Lock)?
        .buffered
        .clone()
        .ok_or(BindPoseError::BufferedOwnerRequired)?;
    owner
        .lock()
        .map_err(|_| ActorFaceError::Lock)?
        .sync_sqt_programs(source)
}
