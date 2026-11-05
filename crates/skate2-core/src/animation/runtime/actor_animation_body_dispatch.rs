//! `DispatchBodyAnim825f1040` / `EvalCurrAnim82d6e658` and the later body ACS Sync.
//! A source-null current tree selects the bind node; retained nonnull trees
//! produce fetch/SQT commands. Dependency-cycle batch splits remain checked.
use super::bind_pose::{
    BindPoseError, BufferedBodyCommands, BufferedEntryAllocation, BufferedJobPool,
    PoseOutputAllocation,
};
use super::{
    ActorAnimationConstruction, ActorAnimationInitializeSource, AnimationInitializeStage,
    AnimationPointerState,
};
use crate::{
    actor_face_playback::{ActorFaceError, AnimationPoseRef, FaceCommandManager},
    actor_graph_construction::ActorGraphIntents,
    animation::{AnimationMatrix, AnimationSkeleton, LocalPose},
    animation_job_owners::{
        AnimationEntryPoint, AnimationJobCode, DormantAnimationJob,
        execution::{
            AnimationExecutionError, AnimationJobWork, AnimationQueuedJob,
            AnimationSchedulerBackend,
        },
    },
};
use skate2_formats::andale::{AndaleDatabase, AndaleHierarchy, AndaleSqt};
use std::sync::{Arc, Mutex};

#[path = "../trees/actor_animation_tree_eval_commands.rs"]
pub(crate) mod tree_commands;
use tree_commands::{Fetch, TreeCommands};
#[path = "actor_animation_reuse_body_jobs.rs"]
mod reuse_jobs;
#[cfg(feature="skate3-runtime")]
#[path="actor_animation_history_commands.rs"]
mod movement_history;

pub(crate) trait BodyAnimationDispatchSource: ActorAnimationInitializeSource {
    fn body_command_manager(&mut self) -> Result<Arc<Mutex<FaceCommandManager>>, Self::Error>;
    fn blend_cull_threshold(&mut self, low: bool) -> Result<f32, Self::Error>;
    fn blend_space_cvars(&self) -> Option<crate::input_cvar_runtime::InputCvarReferences> {
        None
    }
}
#[derive(Debug, PartialEq)]
pub(crate) enum BodyDispatchError<E> {
    Source(E),
    BindPose(BindPoseError),
    CommandOwnerMismatch,
    MissingCommandSystem,
    Evaluation(Arc<BindPoseError>),
    TreeEvaluation(String),
}
impl<E> From<BindPoseError> for BodyDispatchError<E> {
    fn from(error: BindPoseError) -> Self {
        Self::BindPose(error)
    }
}
impl<E> From<ActorFaceError> for BodyDispatchError<E> {
    fn from(error: ActorFaceError) -> Self {
        Self::BindPose(error.into())
    }
}
impl<E> From<AnimationExecutionError> for BodyDispatchError<E> {
    fn from(error: AnimationExecutionError) -> Self {
        Self::BindPose(error.into())
    }
}

pub(crate) struct PendingBodyDispatch {
    work: Arc<BodyPoseWork>,
    backend: Option<Arc<AnimationSchedulerBackend>>,
    jobs: Vec<AnimationQueuedJob>,
}
struct BodyPoseWork {
    pose: Option<AnimationPoseRef>,
    tree: Option<TreeCommands>,
    /// The hierarchy owner's database; `hierarchy` reads its skeleton.
    database: Arc<AndaleDatabase>,
    local: Arc<Mutex<PoseOutputAllocation>>,
    global: Arc<Mutex<PoseOutputAllocation>>,
    state: Mutex<BodyPoseWorkState>,
}
struct BodyPoseWorkState {
    sqt: Option<Vec<AndaleSqt>>,
    tree_sqts: Vec<Option<Vec<AndaleSqt>>>,
    /// The same dynamic MATRIX0 changes from local to global in place.
    matrix: Option<Vec<AnimationMatrix>>,
    failure: Option<Arc<BindPoseError>>,
}
impl AnimationJobWork for BodyPoseWork {
    fn run(&self, code: AnimationJobCode) -> Result<(), AnimationExecutionError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AnimationExecutionError::Lock)?;
        let result = match code {
            AnimationJobCode::SetData => (|| {
                let tree=self.tree.as_ref().ok_or(BindPoseError::MissingBindPose)?;
                tree.load_external_range(&mut state.tree_sqts,&self.database.hierarchy,0..tree.external_sqts.len())
            })(),
            AnimationJobCode::FetchVbr => (|| {
                if let Some(tree) = &self.tree {
                    for (slot, fetch) in &tree.fetches {
                        let pose = match fetch {
                            Fetch::Pose(pose) => pose
                                .constant_vbr_sqt_for(&self.database.hierarchy)
                                .map_err(BindPoseError::Animation)?,
                            Fetch::Clip(plan) => tree_commands::execute_fetch(plan)?,
                        };
                        state.tree_sqts[usize::from(*slot)] = Some(pose);
                    }
                    if tree.operations.is_empty() {
                        state.sqt = state.tree_sqts[usize::from(tree.output()?)].take();
                    }
                    Ok(())
                } else {
                    let pose = self.pose.as_ref().ok_or(BindPoseError::MissingBindPose)?;
                    state.sqt = Some(pose.constant_vbr_sqt_for(&self.database.hierarchy)?);
                    Ok(())
                }
            })(),
            AnimationJobCode::SqtBlend => (|| {
                let tree = self.tree.as_ref().ok_or(BindPoseError::MissingBindPose)?;
                tree_commands::execute_operations(tree, &mut state.tree_sqts, &self.database.hierarchy)?;
                state.sqt = state.tree_sqts[usize::from(tree.output()?)].take();
                Ok(())
            })(),
            AnimationJobCode::SqtToLocal => {
                let input = state
                    .sqt
                    .as_deref()
                    .map(LocalPose::from_andale)
                    .ok_or(BindPoseError::MissingBindPose);
                input.and_then(|input| {
                    let skeleton = AnimationSkeleton::from_andale(&self.database.hierarchy)
                        .map_err(|_| BindPoseError::InvalidHierarchy)?;
                    let matrices = skeleton
                        .local_matrices(&input)
                        .map_err(|_| BindPoseError::InvalidHierarchy)?;
                    copy_extra(&self.local, &matrices)?;
                    state.matrix = Some(matrices);
                    Ok(())
                })
            }
            AnimationJobCode::LocalToGlobal => AnimationSkeleton::from_andale(&self.database.hierarchy)
                .map_err(|_| BindPoseError::InvalidHierarchy)
                .and_then(|mut skeleton| {
                    skeleton.trajectory_bone = Some(0);
                    let input = state
                        .matrix
                        .as_mut()
                        .ok_or(BindPoseError::MissingBindPose)?;
                    let matrices = skeleton
                        .global_matrices_from_local(input)
                        .map_err(|_| BindPoseError::InvalidHierarchy)?;
                    input.clone_from_slice(&matrices);
                    copy_extra(&self.global, input)
                }),
            _ => Err(BindPoseError::UnsupportedSetup),
        };
        result.map_err(|error| {
            state.failure = Some(Arc::new(error));
            AnimationExecutionError::Work("buffered body pose")
        })
    }
}
fn copy_extra(
    output: &Arc<Mutex<PoseOutputAllocation>>,
    values: &[AnimationMatrix],
) -> Result<(), BindPoseError> {
    let mut output = output.lock().map_err(|_| ActorFaceError::Lock)?;
    if output.values.len() != values.len() {
        return Err(BindPoseError::InvalidHierarchy);
    }
    for (destination, value) in output.values.iter_mut().zip(values) {
        *destination = Some(*value);
    }
    Ok(())
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// Actual selected `DispatchBodyAnim` path; it queues work and does not Sync.
    #[expect(
        clippy::too_many_lines,
        reason = "keep named ACS resolution, Begin, live threshold, retained Eval and End failure prefixes ordered"
    )]
    pub(crate) fn try_dispatch_body_animation<S: BodyAnimationDispatchSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), BodyDispatchError<S::Error>> {
        let initialized = self
            .initialization
            .as_ref()
            .ok_or(BindPoseError::InitializationRequired)?;
        if initialized.stage != AnimationInitializeStage::BodyReturned {
            return Err(BindPoseError::InitializationRequired.into());
        }
        let player = self
            .player
            .as_ref()
            .ok_or(BindPoseError::InitializationRequired)?;
        let manager = source
            .body_command_manager()
            .map_err(BodyDispatchError::Source)?;
        let named = manager
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .entries
            .get(&(
                ActorGraphIntents::native_name(b"Skater"),
                i32::from_ne_bytes(player.player_index.to_ne_bytes()),
            ))
            .cloned()
            .ok_or(BodyDispatchError::MissingCommandSystem)?;
        let playback = initialized
            .playback
            .as_ref()
            .ok_or(BindPoseError::BufferedOwnerRequired)?
            .lock()
            .map_err(|_| ActorFaceError::Lock)?;
        if !Arc::ptr_eq(&named, &playback.commands) {
            return Err(BodyDispatchError::CommandOwnerMismatch);
        }
        drop(playback);
        let buffered = named
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .buffered
            .clone()
            .ok_or(BindPoseError::BufferedOwnerRequired)?;
        let mut buffered = buffered.lock().map_err(|_| ActorFaceError::Lock)?;
        if buffered.in_command_set != 0
            || buffered.in_commands != 0
            || buffered.initiated_commands != 0
        {
            return Err(BindPoseError::AlreadyInCommands.into());
        }
        #[cfg(feature = "skate3-runtime")]
        if self.mode_body_pose.is_some() || self.pc_gesture_playback.is_some() {
            // One movement base plus the three native gesture hand channels.
            buffered.max_set_replace_sqts=buffered.max_set_replace_sqts.max(4);
        }
        buffered.in_command_set += 1;
        buffered.last_body_cull_threshold = Some(
            source
                .blend_cull_threshold(player.flags_bits & 0x0800_0000 != 0)
                .map_err(BodyDispatchError::Source)?,
        );
        buffered.in_commands += 1;
        buffered.nblend.begin_commands();
        let base = self
            .base
            .as_mut()
            .ok_or(BindPoseError::InitializationRequired)?;
        let root = self
            .channel
            .as_ref()
            .ok_or(BindPoseError::InitializationRequired)?
            .root_channel;
        if root != AnimationPointerState::Null || base.base_animation != AnimationPointerState::Null
        {
            use crate::actor_animation_construction::channel_runtime::ChannelBaseReference as Reference;
            use crate::actor_animation_tree::eval::AnimationEvalParams;
            let reference =
                if let Some(index) = initialized
                    .channels
                    .as_ref()
                    .and_then(|channels| channels.root)
                {
                    Reference::Channel(index)
                } else {
                    Reference::Body(base.body_root.clone().ok_or_else(|| {
                        BodyDispatchError::TreeEvaluation("missing body root".into())
                    })?)
                };
            let local = initialized.local_matrices.shared_storage()?;
            let global = initialized.global_matrices.shared_storage()?;
            let pose = initialized
                .bind_pose
                .clone()
                .ok_or(BindPoseError::MissingBindPose)?;
            let params = AnimationEvalParams {
                commands: Arc::clone(&named),
                blend_space_cvars: source.blend_space_cvars(),
                update_old_time: true,
                blend_frames: 2,
                blend_cull_threshold: buffered
                    .last_body_cull_threshold
                    .expect("sampled threshold"),
            };
            drop(buffered);
            let owner = named
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .buffered
                .clone()
                .ok_or(BindPoseError::BufferedOwnerRequired)?;
            let mut tree = TreeCommands::new(Arc::clone(&owner));
            tree.enable_reuse();
            self.eval_reference(&reference, &params, true, &mut tree)
                .map_err(|error| BodyDispatchError::TreeEvaluation(error.to_string()))?;
            return owner
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .queue_body_pose(&pose, Some(tree), local, global, &mut false, source);
        }
        let pose = initialized
            .bind_pose
            .as_ref()
            .ok_or(BindPoseError::MissingBindPose)?;
        let local = initialized.local_matrices.shared_storage()?;
        let global = initialized.global_matrices.shared_storage()?;
        buffered.queue_body_pose(
            pose,
            None,
            local,
            global,
            &mut base.bind_pose_evaluated,
            source,
        )
    }

    /// Selected `SkaterManager::UpdatePhysics` Sync boundary before `SetUpPhysics`.
    pub(crate) fn try_sync_body_animation<S: BodyAnimationDispatchSource>(
        &self,
        source: &mut S,
    ) -> Result<(), BodyDispatchError<S::Error>> {
        let player = self
            .player
            .as_ref()
            .ok_or(BindPoseError::InitializationRequired)?;
        let manager = source
            .body_command_manager()
            .map_err(BodyDispatchError::Source)?;
        let named = manager
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .entries
            .get(&(
                ActorGraphIntents::native_name(b"Skater"),
                i32::from_ne_bytes(player.player_index.to_ne_bytes()),
            ))
            .cloned()
            .ok_or(BodyDispatchError::MissingCommandSystem)?;
        self.require_body_command_owner(&named)?;
        let buffered = named
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .buffered
            .clone()
            .ok_or(BindPoseError::BufferedOwnerRequired)?;
        buffered
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .sync_body_pose(source)
    }

    fn require_body_command_owner<E>(
        &self,
        named: &Arc<Mutex<crate::actor_face_playback::FaceCommands>>,
    ) -> Result<(), BodyDispatchError<E>> {
        let playback = self
            .initialization
            .as_ref()
            .and_then(|state| state.playback.as_ref())
            .ok_or(BindPoseError::BufferedOwnerRequired)?
            .lock()
            .map_err(|_| ActorFaceError::Lock)?;
        if !Arc::ptr_eq(named, &playback.commands) {
            return Err(BodyDispatchError::CommandOwnerMismatch);
        }
        Ok(())
    }
}

impl BufferedBodyCommands {
    #[expect(
        clippy::too_many_lines,
        reason = "Retain native command emission, lazy pool construction, flush and submission failure order"
    )]
    fn queue_body_pose<S: BodyAnimationDispatchSource>(
        &mut self,
        pose: &AnimationPoseRef,
        tree: Option<TreeCommands>,
        local: Arc<Mutex<PoseOutputAllocation>>,
        global: Arc<Mutex<PoseOutputAllocation>>,
        evaluated: &mut bool,
        source: &mut S,
    ) -> Result<(), BodyDispatchError<S::Error>> {
        use super::bind_pose::BindPoseCommand as Command;
        let guid = pose.guid()?;
        let output = if let Some(tree) = &tree {
            tree.output()?
        } else {
            self.commands.push(Command::FetchPose { guid, sqt: 0 });
            *evaluated = true;
            0
        };
        self.commands.push(Command::SqtToLocal {
            sqt: output,
            matrix: 0,
        });
        self.commands.push(Command::ExtractMatrix { matrix: 0 });
        self.commands.push(Command::LocalToGlobal { matrix: 0 });
        self.commands.push(Command::ExtractMatrix { matrix: 0 });
        self.in_commands -= 1;
        let pose = if tree.is_none() {
            Some(
                self.database_manager
                    .lock()
                    .map_err(|_| ActorFaceError::Lock)?
                    .pose_by_guid(guid)?,
            )
        } else {
            None
        };
        let database = Arc::clone(
            self.hierarchy_owner
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .parsed
                .as_ref()
                .ok_or(ActorFaceError::MissingRecord)?,
        );
        let hierarchy = &database.hierarchy;
        if hierarchy.bone_names.is_empty()
            || hierarchy.parts.len() != self.max_extracts_per_fetch
            || hierarchy.parts.len() >= 256
        {
            return Err(BindPoseError::UnsupportedSetup.into());
        }
        local.lock().map_err(|_| ActorFaceError::Lock)?.synchronized = false;
        global
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .synchronized = false;
        let tree_slots = tree.as_ref().map_or(0, TreeCommands::dynamic_count);
        let has_set=tree.as_ref().is_some_and(|tree|!tree.external_sqts.is_empty());
        let has_blend = tree
            .as_ref()
            .is_some_and(|tree| !tree.operations.is_empty());
        if let Some(tree) = &tree {
            let mut total_extracts = 0;
            for segment in tree.segments() {
                let extracts: usize = tree.fetches[segment.fetches]
                    .iter()
                    .map(|(_, fetch)| match fetch {
                        Fetch::Pose(_) => self.max_extracts_per_fetch,
                        Fetch::Clip(plan) => plan.extract_count(),
                    })
                    .sum();
                total_extracts += extracts;
                if extracts >= self.max_extracts_per_job
                    || segment.operations.len()
                        > self
                            .blend_pool
                            .as_ref()
                            .ok_or(BindPoseError::BufferedOwnerRequired)?
                            .max_work_units
                {
                    return Err(BindPoseError::UnsupportedSetup.into());
                }
            }
            if total_extracts > self.extract_capacity {
                return Err(BindPoseError::UnsupportedSetup.into());
            }
        }
        let work = Arc::new(BodyPoseWork {
            pose,
            tree,
            database,
            local,
            global,
            state: Mutex::new(BodyPoseWorkState {
                sqt: None,
                tree_sqts: vec![None; tree_slots],
                matrix: None,
                failure: None,
            }),
        });
        self.body_pending = Some(PendingBodyDispatch {
            work: Arc::clone(&work),
            backend: None,
            jobs: Vec::new(),
        });
        if self.local_pool.is_none() {
            self.load_body_entry(source, AnimationJobCode::SqtToLocal, b"ACSposeSQTtoLocal")?;
            self.local_pool = Some(BufferedJobPool::new(
                &self.scheduler,
                14,
                19,
                self.max_pose_operations / 6 + 1,
            ));
        }
        if self.global_pool.is_none() {
            self.load_body_entry(
                source,
                AnimationJobCode::LocalToGlobal,
                b"ACSposeLocaltoGlobal",
            )?;
            self.global_pool = Some(BufferedJobPool::new(
                &self.scheduler,
                14,
                19,
                self.max_pose_operations / 6 + 1,
            ));
        }
        if work.tree.as_ref().is_some_and(TreeCommands::has_reuse) {
            return self.queue_reused_body(&work, source);
        }
        let root = Arc::clone(
            self.null_job
                .as_ref()
                .ok_or(BindPoseError::BufferedOwnerRequired)?,
        );
        let fetch = next_job(
            self.fetch_pool
                .as_mut()
                .ok_or(BindPoseError::BufferedOwnerRequired)?,
        )?;
        let local = next_job(self.local_pool.as_mut().expect("local pool"))?;
        let global = next_job(self.global_pool.as_mut().expect("global pool"))?;
        let blend = if has_blend {
            Some(next_job(
                self.blend_pool
                    .as_mut()
                    .ok_or(BindPoseError::BufferedOwnerRequired)?,
            )?)
        } else {
            None
        };
        let set=if has_set {
            Some(next_job(self.set_pool.as_mut().ok_or(BindPoseError::BufferedOwnerRequired)?)?)
        }else{None};
        let mut configured = vec![
            (
                &fetch,
                Some(AnimationJobCode::FetchVbr),
                b"AnimFetchVBR".as_slice(),
            ),
            (
                &local,
                Some(AnimationJobCode::SqtToLocal),
                b"ACSposeSQTtoLocal".as_slice(),
            ),
            (
                &global,
                Some(AnimationJobCode::LocalToGlobal),
                b"ACSposeLocaltoGlobal".as_slice(),
            ),
            (&root, None, b"".as_slice()),
        ];
        if let Some(blend) = &blend {
            configured.push((
                blend,
                Some(AnimationJobCode::SqtBlend),
                b"ACSsqtBlend".as_slice(),
            ));
        }
        if let Some(set)=&set {configured.push((set,Some(AnimationJobCode::SetData),b"ACSsetData".as_slice()));}
        for (job, code, name) in configured {
            let mut job = job.lock().map_err(|_| ActorFaceError::Lock)?;
            job.entry_point = AnimationEntryPoint::new(code, Some(name));
            job.entry_point.affinity = 1;
            if matches!(
                code,
                Some(
                    AnimationJobCode::SqtToLocal
                        | AnimationJobCode::LocalToGlobal
                        | AnimationJobCode::SqtBlend
                )
            ) {
                job.parameters[2] = 16;
            }
        }
        let mut chain = vec![root, global, local];
        if let Some(blend) = blend {
            chain.push(blend);
        }
        chain.push(fetch);
        if let Some(set)=set {chain.push(set);}
        let last = chain.len() - 1;
        let jobs = chain
            .into_iter()
            .enumerate()
            .map(|(index, job)| AnimationQueuedJob {
                job,
                dependencies: if index == last {
                    Vec::new()
                } else {
                    vec![index + 1]
                },
                work: (index != 0).then(|| Arc::clone(&work) as Arc<dyn AnimationJobWork>),
            })
            .collect::<Vec<_>>();
        let manager = source
            .require_job_manager()
            .map_err(BodyDispatchError::Source)?;
        let backend = manager
            .scheduler
            .as_ref()
            .ok_or(AnimationExecutionError::MissingBackend)?;
        self.body_pending.as_mut().expect("retained body").jobs = jobs;
        backend.submit_body_tree_with_set(
            &self.body_pending.as_ref().expect("retained body").jobs,
            has_blend,has_set,
        )?;
        self.body_pending.as_mut().expect("retained body").backend = Some(Arc::clone(backend));
        self.last_submitted.clone_from(&self.commands);
        self.nblend.end_command_set();
        self.in_command_set -= 1;
        self.initiated_commands += 1;
        Ok(())
    }
    fn load_body_entry<S: BodyAnimationDispatchSource>(
        &mut self,
        source: &mut S,
        code: AnimationJobCode,
        name: &[u8],
    ) -> Result<(), BodyDispatchError<S::Error>> {
        self.entries.push(BufferedEntryAllocation {
            manager: None,
            entry: None,
        });
        let entry = self.entries.last_mut().expect("retained allocation");
        entry.manager = Some(
            source
                .require_job_manager()
                .map_err(BodyDispatchError::Source)?,
        );
        entry.entry = Some(AnimationEntryPoint::new(Some(code), Some(name)));
        Ok(())
    }
    fn sync_body_pose<S: BodyAnimationDispatchSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), BodyDispatchError<S::Error>> {
        if self.initiated_commands == 0 {
            return Ok(());
        }
        let pending = self
            .body_pending
            .as_ref()
            .ok_or(BindPoseError::BufferedOwnerRequired)?;
        let backend = pending
            .backend
            .as_ref()
            .ok_or(AnimationExecutionError::MissingBackend)?;
        let manager = source
            .require_job_manager()
            .map_err(BodyDispatchError::Source)?;
        if !manager
            .scheduler
            .as_ref()
            .is_some_and(|actual| Arc::ptr_eq(actual, backend))
        {
            return Err(AnimationExecutionError::MissingBackend.into());
        }
        if let Err(error) = backend.wait(&pending.jobs[0].job) {
            if let Some(error) = pending
                .work
                .state
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .failure
                .clone()
            {
                return Err(BodyDispatchError::Evaluation(error));
            }
            return Err(error.into());
        }
        backend.release_selected(&pending.jobs)?;
        for job in &pending.jobs {
            job.job
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .clear_after_sync();
        }
        self.fetch_pool.as_mut().expect("fetch pool").current_job = 0;
        if let Some(set)=&mut self.set_pool {set.current_job=0;}
        if let Some(blend) = &mut self.blend_pool {
            blend.current_job = 0;
        }
        self.local_pool.as_mut().expect("local pool").current_job = 0;
        self.global_pool.as_mut().expect("global pool").current_job = 0;
        self.commands.clear();
        self.nblend.synchronized();
        self.initiated_commands -= 1;
        self.synchronized_sets += 1;
        pending
            .work
            .local
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .synchronized = true;
        pending
            .work
            .global
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .synchronized = true;
        self.body_pending = None;
        Ok(())
    }
}
fn next_job(pool: &mut BufferedJobPool) -> Result<Arc<Mutex<DormantAnimationJob>>, BindPoseError> {
    let job = pool
        .jobs
        .get(pool.current_job)
        .cloned()
        .ok_or(BindPoseError::UnsupportedSetup)?;
    pool.current_job += 1;
    Ok(job)
}

#[cfg(test)]
#[path = "actor_animation_body_dispatch_tests.rs"]
mod tests;

use crate::actor_animation_tree::eval::{
    AnimationEvalCommands, AnimationEvalError, AnimationEvalParams,
};
impl<K, W> crate::actor_animation_construction::ActorAnimationConstruction<K, W> {
    /// `ChannelBlend::Eval82d6f178`; the base evaluates before the channel even
    /// when its coefficient is zero. A culled Clip still commits its old time.
    pub(crate) fn eval_reference<C: AnimationEvalCommands>(
        &mut self,
        reference: &crate::actor_animation_construction::channel_runtime::ChannelBaseReference,
        params: &AnimationEvalParams,
        do_eval: bool,
        commands: &mut C,
    ) -> Result<bool, AnimationEvalError<C::Error>> {
        use crate::actor_animation_construction::channel_runtime::ChannelBaseReference as R;
        use AnimationEvalError as E;
        match reference {
            R::Empty => Err(E::MissingOwner("ChannelBlend base animation")),
            R::Body(root) => {
                #[cfg(feature = "skate3-runtime")]
                let result=if let Some(pose)=self.mode_body_pose.clone() {
                    let mut history=movement_history::HistoryCommands::<C::Error>::new();
                    self.eval_body_reference(root,params,true,&mut history)?;
                    if do_eval {
                        commands.external_sqt(&pose).ok_or(E::MissingOwner("Actor movement SetData"))?.map_err(E::Command)?;
                        true
                    }else{false}
                }else{self.eval_body_reference(root,params,do_eval,commands)?};
                #[cfg(not(feature = "skate3-runtime"))]
                let result=self.eval_body_reference(root,params,do_eval,commands)?;
                #[cfg(feature = "skate3-runtime")]
                if result && do_eval && let Some(runtime)=self.pc_gesture_playback.as_mut() {
                    let poses=runtime.evaluate(params.blend_cull_threshold)
                        .map_err(|_|E::MissingOwner("Skate 3 gesture evaluation"))?;
                    for channel in poses {
                        let source=crate::actor_animation_construction::initialization::bind_pose::sqt_output::OwnedAnimationSqtBuffer::allocate_unknown(channel.pose.len());
                        let values=channel.pose.iter().map(|sqt|skate2_formats::andale::AndaleSqt {
                            scale:sqt.scale,rotation:sqt.rotation,translation:sqt.translation,
                        }).collect::<Vec<_>>();
                        source.extract(&values).and_then(|()|source.complete_sync())
                            .map_err(|_|E::MissingOwner("Skate 3 gesture SQT publication"))?;
                        commands.external_sqt(&source).ok_or(E::MissingOwner("Actor gesture SetData"))?.map_err(E::Command)?;
                        commands.channel_blend(channel.influence,channel.weights_from_channel).map_err(E::Command)?;
                    }
                }
                Ok(result)
            },
            R::Channel(index) => {
                let channels = self
                    .initialization
                    .as_ref()
                    .and_then(|state| state.channels.as_ref())
                    .ok_or(E::MissingOwner("channels"))?;
                let channel = channels
                    .channels
                    .get(*index)
                    .ok_or(E::MissingOwner("channel"))?;
                let coefficient = channel
                    .blend_coefficient
                    .ok_or(E::MissingOwner("channel coefficient"))?;
                if coefficient > 1.0 || coefficient < 0.0 {
                    return Err(E::InvalidCoefficient);
                }
                #[cfg(feature = "skate3-runtime")]
                let allow_channel=self.mode_body_pose.is_none() ||
                    [b"GestureBoth".as_slice(),b"GestureLeft",b"GestureRight"].iter().any(|name|
                        channels.dictionary.get(&ActorGraphIntents::native_name(name))==Some(index));
                #[cfg(not(feature = "skate3-runtime"))]
                let allow_channel=true;
                let runtime = channel
                    .runtime
                    .as_ref()
                    .ok_or(E::MissingOwner("channel runtime"))?;
                let base = runtime.base.clone();

                let first = self.eval_reference(&base, params, do_eval, commands)?;
                let channels = self
                    .initialization
                    .as_mut()
                    .and_then(|state| state.channels.as_mut())
                    .ok_or(E::MissingOwner("channels"))?;
                #[cfg(feature="skate3-runtime")]
                if !allow_channel {
                    let mut history=movement_history::HistoryCommands::<C::Error>::new();
                    super::super::channel_runtime::transition::eval(
                        channels.channels.get_mut(*index).ok_or(E::MissingOwner("channel"))?,
                        &mut channels.trees,params,true,&mut history)?;
                    return Ok(first);
                }
                let second = super::super::channel_runtime::transition::eval(
                    channels
                        .channels
                        .get_mut(*index)
                        .ok_or(E::MissingOwner("channel"))?,
                    &mut channels.trees,
                    params,
                    do_eval && coefficient > 0.0 && allow_channel,
                    commands,
                )?;
                let second=second&&allow_channel;
                if do_eval && coefficient > 0.0 && first && second {
                    commands
                        .channel_blend(coefficient, false)
                        .map_err(E::Command)?;
                }
                Ok(first || second)
            }
        }
    }
}
