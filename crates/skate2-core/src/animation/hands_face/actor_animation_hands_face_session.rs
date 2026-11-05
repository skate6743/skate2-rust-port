//! Host composition of retained hands/face dispatch and original ACS boundaries.
//! Caller owns Actor scheduling and supplies actual LOD/local/clamp capabilities.
use super::{
    ActorConstruction,
    body_session::{ActorBodyAnimationServices, BodySessionError},
};
use crate::{
    actor_animation_construction::{
        hands_face::runtime::{self, HandFaceDispatchSource, HandFaceRuntimeError},
        initialization::{
            ActorAnimationInitializeSource,
            bind_pose::{BindPoseError, sqt_output::OwnedAnimationSqtBuffer},
        },
    },
    actor_face_playback::{FacePlaybackManager, blend_pose_stream::BlendPoseTokenPool},
    animation_job_owners::AnimationJobManager,
};
use std::sync::{Arc, Mutex};

/// Reads occur only where `DispatchHandsFaceAnim825f87b8` asks for them.
pub(crate) trait HandsFaceSelection {
    type Error;
    fn is_local_player(&mut self) -> Result<bool, Self::Error>;
    fn hands_commands_clamp(&mut self) -> Result<i32, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum HandsFaceSourceError<E> {
    Body(BodySessionError),
    Selection(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for HandsFaceSourceError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Body(error) => write!(f, "hands/face body source: {error}"),
            Self::Selection(error) => write!(f, "hands/face selection source: {error:?}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum HandsFaceSessionError<E> {
    Body(BodySessionError),
    Missing(&'static str),
    Lock(&'static str),
    Begin(BindPoseError),
    Gesture(String),
    Runtime(HandFaceRuntimeError<HandsFaceSourceError<E>>),
}
impl<E: std::fmt::Debug> std::fmt::Display for HandsFaceSessionError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Body(error) => write!(f, "hands/face session Body: {error:?}"),
            Self::Missing(error) => write!(f, "hands/face session Missing: {error:?}"),
            Self::Lock(error) => write!(f, "hands/face session Lock: {error:?}"),
            Self::Gesture(error) => write!(f,"gesture streams: {error}"),
            Self::Begin(error) => write!(f, "hands/face session Begin: {error:?}"),
            Self::Runtime(error) => write!(f, "hands/face session Runtime: {error:?}"),
        }
    }
}

/// Original Initialize-time owner references, after each ACS Sync succeeds.
/// A successful call does not promise a new generation at every LOD.
pub(crate) struct HandsFaceSessionOutputs {
    pub(crate) left: OwnedAnimationSqtBuffer,
    pub(crate) right: OwnedAnimationSqtBuffer,
    pub(crate) face: OwnedAnimationSqtBuffer,
}

struct Source<'a, S> {
    body: &'a mut ActorBodyAnimationServices,
    selection: &'a mut S,
    playback: Arc<Mutex<FacePlaybackManager>>,
}
impl<S: HandsFaceSelection> ActorAnimationInitializeSource for Source<'_, S> {
    type Error = HandsFaceSourceError<S::Error>;
    fn read_use_jobs_for_skater_anim_update(&mut self) -> Result<bool, Self::Error> {
        self.body
            .read_use_jobs_for_skater_anim_update()
            .map_err(HandsFaceSourceError::Body)
    }
    fn require_job_manager(&mut self) -> Result<Arc<AnimationJobManager>, Self::Error> {
        self.body
            .require_job_manager()
            .map_err(HandsFaceSourceError::Body)
    }
}
impl<S: HandsFaceSelection> HandFaceDispatchSource for Source<'_, S> {
    fn is_local_player(&mut self) -> Result<bool, Self::Error> {
        self.selection
            .is_local_player()
            .map_err(HandsFaceSourceError::Selection)
    }
    fn hands_commands_clamp(&mut self) -> Result<i32, Self::Error> {
        self.selection
            .hands_commands_clamp()
            .map_err(HandsFaceSourceError::Selection)
    }
    fn animation_playback_manager(
        &mut self,
    ) -> Result<Arc<Mutex<FacePlaybackManager>>, Self::Error> {
        Ok(Arc::clone(&self.playback))
    }
}

impl ActorConstruction {
    /// Selected Actor session, not the global manager's multi-Actor scheduling.
    /// On error prior accepted ACS operations remain committed; no fake rollback.
    pub(crate) fn try_hands_face_session<S: HandsFaceSelection>(
        &self,
        lod: i32,
        body: &mut ActorBodyAnimationServices,
        tokens: &mut BlendPoseTokenPool,
        selection: &mut S,
    ) -> Result<HandsFaceSessionOutputs, HandsFaceSessionError<S::Error>> {
        use HandsFaceSessionError as E;
        let retained = self.checked_body_animation().map_err(E::Body)?;
        let (left, right, face, outputs, playback) = {
            let animation = retained.lock().map_err(|_| E::Lock("retained animation"))?;
            let state = animation
                .initialization()
                .ok_or(E::Missing("animation Initialize"))?;
            let left = state.hands[0].as_ref().ok_or(E::Missing("left hand"))?;
            let right = state.hands[1].as_ref().ok_or(E::Missing("right hand"))?;
            let face = animation.face().ok_or(E::Missing("retained face"))?;
            let left_commands = Arc::clone(
                &left
                    .playback
                    .lock()
                    .map_err(|_| E::Lock("left playback"))?
                    .commands,
            );
            let right_commands = Arc::clone(
                &right
                    .playback
                    .lock()
                    .map_err(|_| E::Lock("right playback"))?
                    .commands,
            );
            (
                left_commands,
                right_commands,
                Arc::clone(face.commands.as_ref().ok_or(E::Missing("face commands"))?),
                HandsFaceSessionOutputs {
                    left: left.local_pose.clone(),
                    right: right.local_pose.clone(),
                    face: face.local_pose.clone(),
                },
                Arc::clone(&face.managers.playback),
            )
        };
        let mut source = Source {
            body,
            selection,
            playback,
        };
        #[cfg(feature="skate3-runtime")]
        if let Some(gesture)=&retained.lock().map_err(|_|E::Lock("gesture hand registration"))?.pc_gesture_playback {
            gesture.register_hand_poses(&left,&right).map_err(E::Gesture)?;
        }
        runtime::begin_command_set(&left).map_err(E::Begin)?;
        runtime::begin_command_set(&right).map_err(E::Begin)?;
        retained
            .lock()
            .map_err(|_| E::Lock("retained animation"))?
            .try_dispatch_hands_face(lod, tokens, &mut source)
            .map_err(E::Runtime)?;
        runtime::end_command_set(&right, &mut source).map_err(E::Runtime)?;
        runtime::end_command_set(&left, &mut source).map_err(E::Runtime)?;
        runtime::sync_command_set(&left, &mut source).map_err(E::Runtime)?;
        runtime::sync_command_set(&right, &mut source).map_err(E::Runtime)?;
        runtime::sync_command_set(&face, &mut source).map_err(E::Runtime)?;
        Ok(outputs)
    }
}
