//! Reuse boundaries on the existing body ACS, shared work and scheduler.
use super::{
    ActorFaceError, AnimationEntryPoint, AnimationExecutionError, AnimationJobCode,
    AnimationJobWork, AnimationQueuedJob, Arc, BindPoseError, BodyAnimationDispatchSource,
    BodyDispatchError, BodyPoseWork, BufferedBodyCommands, DormantAnimationJob, Fetch, Mutex,
    next_job, tree_commands,
};

type Stage = (
    Arc<Mutex<DormantAnimationJob>>,
    Option<Arc<dyn AnimationJobWork>>,
);
use tree_commands::reuse::Segment;
struct SegmentWork {
    body: Arc<BodyPoseWork>,
    segment: Segment,
    final_segment: bool,
}
impl AnimationJobWork for SegmentWork {
    fn run(&self, code: AnimationJobCode) -> Result<(), AnimationExecutionError> {
        let mut state = self
            .body
            .state
            .lock()
            .map_err(|_| AnimationExecutionError::Lock)?;
        let result = (|| {
            let tree = self
                .body
                .tree
                .as_ref()
                .ok_or(BindPoseError::MissingBindPose)?;
            match code {
                AnimationJobCode::SetData => tree.load_external_range(&mut state.tree_sqts,
                    &self.body.database.hierarchy,self.segment.external_sqts.clone())?,
                AnimationJobCode::FetchVbr => {
                    for (slot, fetch) in &tree.fetches[self.segment.fetches.clone()] {
                        let values = match fetch {
                            Fetch::Pose(pose) => pose.constant_vbr_sqt_for(&self.body.database.hierarchy)?,
                            Fetch::Clip(plan) => tree_commands::execute_fetch(plan)?,
                        };
                        state.tree_sqts[usize::from(*slot)] = Some(values);
                    }
                }
                AnimationJobCode::SqtBlend => tree_commands::execute_operation_range(
                    tree,
                    &mut state.tree_sqts,
                    &self.body.database.hierarchy,
                    self.segment.operations.clone(),
                )?,
                AnimationJobCode::ReuseBuffer => tree_commands::reuse::execute(
                    &self.segment.copies,
                    &mut state.tree_sqts,
                    self.body.database.hierarchy.parents.len(),
                )?,
                _ => return Err(BindPoseError::UnsupportedSetup),
            }
            if self.final_segment
                && (code==AnimationJobCode::SqtBlend || (self.segment.operations.is_empty() &&
                    (code==AnimationJobCode::FetchVbr || (self.segment.fetches.is_empty() && code==AnimationJobCode::SetData))))
            {
                state.sqt = state.tree_sqts[usize::from(tree.output()?)].take();
            }
            Ok(())
        })();
        result.map_err(|error| {
            state.failure = Some(Arc::new(error));
            AnimationExecutionError::Work("buffered body reuse segment")
        })
    }
}
impl BufferedBodyCommands {
    #[expect(
        clippy::too_many_lines,
        reason = "retain segment job configuration, submission and publication order"
    )]
    pub(super) fn queue_reused_body<S: BodyAnimationDispatchSource>(
        &mut self,
        work: &Arc<BodyPoseWork>,
        source: &mut S,
    ) -> Result<(), BodyDispatchError<S::Error>> {
        let segments = work
            .tree
            .as_ref()
            .ok_or(BindPoseError::MissingBindPose)?
            .segments();
        let commands = self
            .commands
            .iter()
            .filter_map(|command| match command {
                super::super::bind_pose::BindPoseCommand::ReuseBuffer { segment } => Some(*segment),
                _ => None,
            })
            .collect::<Vec<_>>();
        if commands != (0..segments.len() - 1).collect::<Vec<_>>() {
            return Err(BindPoseError::UnsupportedSetup.into());
        }
        let mut stages: Vec<Stage> = vec![];
        let mut shapes = vec![];
        let mut reuse_job_index = 0;
        for (index, segment) in segments.iter().enumerate() {
            let view = Arc::new(SegmentWork {
                body: Arc::clone(work),
                segment: segment.clone(),
                final_segment: index + 1 == segments.len(),
            });
            let set=!segment.external_sqts.is_empty();
            let fetch = !segment.fetches.is_empty();
            let blend = !segment.operations.is_empty();
            let reuse = !segment.copies.is_empty();
            shapes.push((set,fetch,blend,reuse));
            if set {
                let job=next_job(self.set_pool.as_mut().ok_or(BindPoseError::BufferedOwnerRequired)?)?;
                configure(&job,Some(AnimationJobCode::SetData),b"ACSsetData")?;
                stages.push((job,Some(Arc::clone(&view) as Arc<dyn AnimationJobWork>)));
            }
            if fetch {
                let job = next_job(
                    self.fetch_pool
                        .as_mut()
                        .ok_or(BindPoseError::BufferedOwnerRequired)?,
                )?;
                configure(&job, Some(AnimationJobCode::FetchVbr), b"AnimFetchVBR")?;
                stages.push((job, Some(Arc::clone(&view) as Arc<dyn AnimationJobWork>)));
            }
            if blend {
                let job = next_job(
                    self.blend_pool
                        .as_mut()
                        .ok_or(BindPoseError::BufferedOwnerRequired)?,
                )?;
                configure(&job, Some(AnimationJobCode::SqtBlend), b"ACSsqtBlend")?;
                stages.push((job, Some(Arc::clone(&view) as Arc<dyn AnimationJobWork>)));
            }
            if reuse {
                // Native dispatch assigns reuse code to its retained null boundary job.
                if reuse_job_index == self.body_reuse_jobs.len() {
                    self.body_reuse_jobs.push(self.scheduler.create_job());
                }
                let job = Arc::clone(&self.body_reuse_jobs[reuse_job_index]);
                reuse_job_index += 1;
                configure(&job, Some(AnimationJobCode::ReuseBuffer), b"ACSreuseBuffer")?;
                stages.push((job, Some(view as Arc<dyn AnimationJobWork>)));
            }
        }
        for (pool, code, name) in [
            (
                self.local_pool.as_mut(),
                AnimationJobCode::SqtToLocal,
                b"ACSposeSQTtoLocal".as_slice(),
            ),
            (
                self.global_pool.as_mut(),
                AnimationJobCode::LocalToGlobal,
                b"ACSposeLocaltoGlobal".as_slice(),
            ),
        ] {
            let job = next_job(pool.ok_or(BindPoseError::BufferedOwnerRequired)?)?;
            configure(&job, Some(code), name)?;
            stages.push((job, Some(Arc::clone(work) as Arc<dyn AnimationJobWork>)));
        }
        let root = Arc::clone(
            self.null_job
                .as_ref()
                .ok_or(BindPoseError::BufferedOwnerRequired)?,
        );
        configure(&root, None, b"")?;
        stages.push((root, None));
        stages.reverse();
        let last = stages.len() - 1;
        let jobs = stages
            .into_iter()
            .enumerate()
            .map(|(index, (job, work))| AnimationQueuedJob {
                job,
                work,
                dependencies: if index == last {
                    vec![]
                } else {
                    vec![index + 1]
                },
            })
            .collect();
        self.body_pending.as_mut().expect("retained body").jobs = jobs;
        let manager = source
            .require_job_manager()
            .map_err(BodyDispatchError::Source)?;
        let backend = manager
            .scheduler
            .as_ref()
            .ok_or(AnimationExecutionError::MissingBackend)?;
        backend.submit_body_segments_with_set(
            &self.body_pending.as_ref().expect("retained body").jobs,
            &shapes,
        )?;
        self.body_pending.as_mut().expect("retained body").backend = Some(Arc::clone(backend));
        self.last_submitted.clone_from(&self.commands);
        self.nblend.end_command_set();
        self.in_command_set -= 1;
        self.initiated_commands += 1;
        Ok(())
    }
}
fn configure(
    job: &Arc<Mutex<DormantAnimationJob>>,
    code: Option<AnimationJobCode>,
    name: &[u8],
) -> Result<(), BindPoseError> {
    let mut job = job.lock().map_err(|_| ActorFaceError::Lock)?;
    job.entry_point = AnimationEntryPoint::new(code, Some(name));
    job.entry_point.affinity = 1;
    if matches!(
        code,
        Some(
            AnimationJobCode::SqtBlend
                | AnimationJobCode::SqtToLocal
                | AnimationJobCode::LocalToGlobal
        )
    ) {
        job.parameters[2] = 16;
    }
    Ok(())
}

#[cfg(test)]
#[path = "actor_animation_reuse_body_jobs_tests.rs"]
mod tests;
