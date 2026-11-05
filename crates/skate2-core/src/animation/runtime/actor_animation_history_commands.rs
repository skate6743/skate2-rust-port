//! Inactive movement history on the retained Actor when S3 supplies its pose.
//! Native BlendSpace Eval evaluates unconditionally. A false doEval passed to
//! its AddSQT parent is therefore not a valid way to discard the old movement.
//! Visit that history with ordinary gates, without scheduling a second pose.
use crate::{
    actor_animation_tree::eval::AnimationEvalCommands,
    actor_face_playback::{AnimationPoseRef,FaceCommands,tree_database::clip_fetch::ClipFetchPlan},
};
use std::{marker::PhantomData,sync::{Arc,Mutex}};
pub(super) struct HistoryCommands<E>(PhantomData<fn()->E>);
impl<E> HistoryCommands<E> {pub(super) fn new()->Self {Self(PhantomData)}}
impl<E> AnimationEvalCommands for HistoryCommands<E> {
    type Error=E;
    fn fetch(&mut self,_:ClipFetchPlan)->Result<(),E>{Ok(())}
    fn fetch_pose(&mut self,_:&AnimationPoseRef)->Result<(),E>{Ok(())}
    fn fixed_sqt(&mut self,_:&Arc<Mutex<FaceCommands>>,_:u32)->Option<Result<(),E>>{Some(Ok(()))}
    fn n_blend(&mut self,_:&[f32])->Result<(),E>{Ok(())}
    fn blend(&mut self,_:f32)->Result<(),E>{Ok(())}
    fn channel_blend(&mut self,_:f32,_:bool)->Result<(),E>{Ok(())}
    fn add_sqt(&mut self,_:bool)->Result<(),E>{Ok(())}
    fn mirror(&mut self,_:u32)->Result<(),E>{Ok(())}
}
