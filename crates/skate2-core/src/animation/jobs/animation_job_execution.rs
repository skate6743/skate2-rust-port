//! Selected buffered bind-pose jobs on a retained local scheduler.
//! Sources: manager `Init826e3490`, `AddTree829aec18`, barriers829ae000,
//! `RunSingleJob829b00b0`, `JobInstance::Run829aff10`, `Sync82a30088`.
use super::{AnimationJobCode, AnimationJobManager, DormantAnimationJob};
use std::{
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
};

#[path = "animation_job_affinity.rs"]
pub(crate) mod affinity;
#[cfg(test)]
#[path = "animation_job_affinity_tests.rs"]
mod affinity_tests;

#[cfg(test)]
#[path = "animation_job_execution_tests.rs"]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AnimationExecutionError {
    AlreadyInitialized,
    MissingBackend,
    Lock,
    ThreadSpawn,
    UnsupportedGraph,
    UnsupportedEntryPoint,
    UnsupportedCategory(u32),
    SubmissionStillRetained,
    Work(&'static str),
    WorkerPanicked,
    InvalidHandle,
}

/// Typed owner of the actual parameters consumed by a local entry point.
pub(crate) trait AnimationJobWork: Send + Sync {
    fn run(&self, code: AnimationJobCode) -> Result<(), AnimationExecutionError>;
}

pub(crate) struct AnimationQueuedJob {
    pub(crate) job: Arc<Mutex<DormantAnimationJob>>,
    /// Indices in the retained breadth-first submission list.
    pub(crate) dependencies: Vec<usize>,
    pub(crate) work: Option<Arc<dyn AnimationJobWork>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InstancePhase {
    Barrier,
    Ready,
    Running,
    Done,
    Failed,
}
struct Instance {
    submission: u64,
    affinity: u32,
    code: Option<AnimationJobCode>,
    work: Option<Arc<dyn AnimationJobWork>>,
    barriers: usize,
    dependents: Vec<usize>,
    phase: InstancePhase,
}
struct Queue {
    slots: Vec<Option<Instance>>,
    previous_index: usize,
    next_submission: u64,
    retained: Vec<usize>,
    failure: Option<AnimationExecutionError>,
    stopping: bool,
}
struct Shared {
    queue: Mutex<Queue>,
    changed: Condvar,
}

/// Host worker threads retain the native LOCAL/ANY configuration. Processor
/// values are source hints, not host CPU indices. All admitted entries have
/// source priority128; per-job affinity and submission order remain retained.
pub(crate) struct AnimationSchedulerBackend {
    shared: Arc<Shared>,
    workers: Vec<JoinHandle<()>>,
    pub(crate) source_processors: [u32; 3],
    pub(crate) worker_affinity: u32,
    pub(crate) calling_thread_may_run_deferred: bool,
    pub(crate) function_queue_size: usize,
    pub(crate) job_queue_size: usize,
}

impl AnimationJobManager {
    /// Actual constructor remains dormant; this is the separate manager Init.
    pub(crate) fn try_initialize_local_backend(&mut self) -> Result<(), AnimationExecutionError> {
        if self.scheduler.is_some() {
            return Err(AnimationExecutionError::AlreadyInitialized);
        }
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue {
                slots: (0..512).map(|_| None).collect(),
                previous_index: 0,
                next_submission: 0,
                retained: Vec::new(),
                failure: None,
                stopping: false,
            }),
            changed: Condvar::new(),
        });
        let mut backend = AnimationSchedulerBackend {
            shared,
            workers: Vec::new(),
            source_processors: [1, 3, 5],
            worker_affinity: 63,
            calling_thread_may_run_deferred: true,
            function_queue_size: 512,
            job_queue_size: 128,
        };
        for _ in backend.source_processors {
            let shared = Arc::clone(&backend.shared);
            let worker_affinity = backend.worker_affinity;
            let worker = thread::Builder::new()
                .name("Job Manager - Job Thread".into())
                .spawn(move || {
                    let _host_thread = crate::host_thread::WorkerThreadScope::enter();
                    worker_loop(&shared, worker_affinity);
                })
                .map_err(|_| AnimationExecutionError::ThreadSpawn)?;
            backend.workers.push(worker);
        }
        self.scheduler = Some(Arc::new(backend));
        Ok(())
    }
}

impl Drop for AnimationSchedulerBackend {
    fn drop(&mut self) {
        if let Ok(mut queue) = self.shared.queue.lock() {
            queue.stopping = true;
        }
        self.shared.changed.notify_all();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

impl AnimationSchedulerBackend {
    pub(crate) fn submit_blend_space(
        self: &Arc<Self>,
        jobs: &[AnimationQueuedJob],
    ) -> Result<(), AnimationExecutionError> {
        self.submit_selected_chain(jobs, &[Some(AnimationJobCode::BlendSpace)])
    }
    /// Selected reachable graph only: null root depends on `ACSpose`, which
    /// depends on `FetchVBR`. Every instance/start barrier is installed before
    /// dependency events and before any start barrier is released.
    pub(crate) fn submit_bind_pose(
        self: &Arc<Self>,
        jobs: &[AnimationQueuedJob],
    ) -> Result<(), AnimationExecutionError> {
        let expected = [
            None,
            Some(AnimationJobCode::Pose),
            Some(AnimationJobCode::FetchVbr),
        ];
        self.submit_selected_chain(jobs, &expected)
    }

    pub(crate) fn submit_body_pose(
        self: &Arc<Self>,
        jobs: &[AnimationQueuedJob],
    ) -> Result<(), AnimationExecutionError> {
        self.submit_selected_chain(
            jobs,
            &[
                None,
                Some(AnimationJobCode::LocalToGlobal),
                Some(AnimationJobCode::SqtToLocal),
                Some(AnimationJobCode::FetchVbr),
            ],
        )
    }

    pub(crate) fn submit_body_tree(
        self: &Arc<Self>,
        jobs: &[AnimationQueuedJob],
        has_blend: bool,
    ) -> Result<(), AnimationExecutionError> {
        self.submit_body_tree_with_set(jobs,has_blend,false)
    }

    pub(crate) fn submit_body_tree_with_set(self:&Arc<Self>,jobs:&[AnimationQueuedJob],has_blend:bool,has_set:bool)
        ->Result<(),AnimationExecutionError>{
        let mut expected=vec![None,Some(AnimationJobCode::LocalToGlobal),Some(AnimationJobCode::SqtToLocal)];
        if has_blend {expected.push(Some(AnimationJobCode::SqtBlend));}
        expected.push(Some(AnimationJobCode::FetchVbr));
        if has_set {expected.push(Some(AnimationJobCode::SetData));}
        self.submit_selected_chain(jobs,&expected)
    }

    /// Existing body jobs separated by native bounded-buffer reuse barriers.
    pub(crate) fn submit_body_segments(
        self: &Arc<Self>,
        jobs: &[AnimationQueuedJob],
        segments: &[(bool, bool, bool)],
    ) -> Result<(), AnimationExecutionError> {
        let with_set=segments.iter().map(|&(fetch,blend,reuse)|(false,fetch,blend,reuse)).collect::<Vec<_>>();
        self.submit_body_segments_with_set(jobs,&with_set)
    }
    pub(crate) fn submit_body_segments_with_set(self:&Arc<Self>,jobs:&[AnimationQueuedJob],segments:&[(bool,bool,bool,bool)])
        ->Result<(),AnimationExecutionError>{
        if segments.len() < 2 || segments.len() > 8 {
            return Err(AnimationExecutionError::UnsupportedGraph);
        }
        let mut expected = vec![];
        for &(set,fetch,blend,reuse) in segments {
            if set {expected.push(Some(AnimationJobCode::SetData));}
            if fetch {
                expected.push(Some(AnimationJobCode::FetchVbr));
            }
            if blend {
                expected.push(Some(AnimationJobCode::SqtBlend));
            }
            if reuse {
                expected.push(Some(AnimationJobCode::ReuseBuffer));
            }
        }
        expected.extend([
            Some(AnimationJobCode::SqtToLocal),
            Some(AnimationJobCode::LocalToGlobal),
            None,
        ]);
        expected.reverse();
        self.submit_selected_chain(jobs, &expected)
    }

    /// Selected RAW hand/face dependencies. Extraction belongs to the producing
    /// final-segment fetch/blend; reuse is a barrier on the same bounded buffer.
    pub(crate) fn submit_sqt_segments(
        self: &Arc<Self>,
        jobs: &[AnimationQueuedJob],
        segments: &[(bool, bool, bool)],
    ) -> Result<(), AnimationExecutionError> {
        if segments.len() < 2 || segments.len() > 8 {
            return Err(AnimationExecutionError::UnsupportedGraph);
        }
        let mut expected = vec![];
        for &(fetch, blend, reuse) in segments {
            if fetch {
                expected.push(Some(AnimationJobCode::FetchRaw));
            }
            if blend {
                expected.push(Some(AnimationJobCode::SqtBlend));
            }
            if reuse {
                expected.push(Some(AnimationJobCode::ReuseBuffer));
            }
        }
        expected.push(None);
        expected.reverse();
        self.submit_selected_chain(jobs, &expected)
    }

    /// Selected RAW hand/face dependencies. Extraction belongs to the producing
    /// fetch/blend job; a fixed-only or empty set has only its null root.
    pub(crate) fn submit_sqt_streams(
        self: &Arc<Self>,
        jobs: &[AnimationQueuedJob],
        has_blend: bool,
        has_fetch: bool,
    ) -> Result<(), AnimationExecutionError> {
        self.submit_sqt_streams_with_set(jobs, has_blend, has_fetch, false)
    }
    pub(crate) fn submit_sqt_streams_with_set(
        self: &Arc<Self>,
        jobs: &[AnimationQueuedJob],
        has_blend: bool,
        has_fetch: bool,
        has_set: bool,
    ) -> Result<(), AnimationExecutionError> {
        self.submit_sqt_matrix_streams(jobs, has_blend, has_fetch, has_set, false)
    }
    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "independent selected native command-stage presence flags"
    )]
    pub(crate) fn submit_sqt_matrix_streams(
        self: &Arc<Self>,
        jobs: &[AnimationQueuedJob],
        has_blend: bool,
        has_fetch: bool,
        has_set: bool,
        has_matrix_pose: bool,
    ) -> Result<(), AnimationExecutionError> {
        self.submit_sqt_matrix_streams_with_fetch(jobs, has_blend, has_fetch, has_set,
            has_matrix_pose, AnimationJobCode::FetchRaw)
    }

    pub(crate) fn submit_sqt_matrix_streams_with_fetch(
        self: &Arc<Self>, jobs: &[AnimationQueuedJob], has_blend: bool,
        has_fetch: bool, has_set: bool, has_matrix_pose: bool, fetch_code: AnimationJobCode,
    ) -> Result<(), AnimationExecutionError> {
        if !matches!(fetch_code, AnimationJobCode::FetchRaw | AnimationJobCode::FetchVbr) {
            return Err(AnimationExecutionError::UnsupportedEntryPoint);
        }
        if has_set && has_fetch {
            return Err(AnimationExecutionError::UnsupportedGraph);
        }
        let mut expected = vec![None];
        if has_matrix_pose {
            expected.push(Some(AnimationJobCode::PoseMatMult));
        }
        if has_blend {
            expected.push(Some(AnimationJobCode::SqtBlend));
        }
        if has_fetch {
            expected.push(Some(fetch_code));
        }
        if has_set {
            expected.push(Some(AnimationJobCode::SetData));
        }
        self.submit_selected_chain(jobs, &expected)
    }

    fn validate_selected_chain(
        jobs: &[AnimationQueuedJob],
        expected: &[Option<AnimationJobCode>],
    ) -> Result<(), AnimationExecutionError> {
        if jobs.len() != expected.len() {
            return Err(AnimationExecutionError::UnsupportedGraph);
        }
        for (index, job) in jobs.iter().enumerate() {
            let valid_dependencies = if index + 1 == jobs.len() {
                job.dependencies.is_empty()
            } else {
                job.dependencies == [index + 1]
            };
            if !valid_dependencies
                || jobs[..index]
                    .iter()
                    .any(|previous| Arc::ptr_eq(&previous.job, &job.job))
            {
                return Err(AnimationExecutionError::UnsupportedGraph);
            }
        }
        Ok(())
    }

    fn validate_entry_points(
        owners: &[std::sync::MutexGuard<'_, DormantAnimationJob>],
        jobs: &[AnimationQueuedJob],
        expected: &[Option<AnimationJobCode>],
    ) -> Result<(), AnimationExecutionError> {
        for ((owner, expected), queued) in owners.iter().zip(expected.iter().copied()).zip(jobs) {
            let entry = &owner.entry_point;
            if entry.code != expected
                || entry.environment != 0
                || !(matches!(entry.affinity, 1 | 4)
                    || (expected == Some(AnimationJobCode::BlendSpace) && entry.affinity == 63))
                || entry.priority != 128
                || entry.allow_sleep
                || entry.break_on_entry
                || queued.work.is_some() != expected.is_some()
            {
                return Err(AnimationExecutionError::UnsupportedEntryPoint);
            }
        }
        Ok(())
    }

    fn submit_selected_chain(
        self: &Arc<Self>,
        jobs: &[AnimationQueuedJob],
        expected: &[Option<AnimationJobCode>],
    ) -> Result<(), AnimationExecutionError> {
        Self::validate_selected_chain(jobs, expected)?;
        self.submit_validated_graph(jobs, expected)
    }

    /// ACS Schedule82a32800 publishes engine jobs with deduplicated producer
    /// dependencies. Callers retain native per-engine batches and pool bounds.
    pub(crate) fn submit_sqt_job_graph(self: &Arc<Self>, jobs: &[AnimationQueuedJob])
        -> Result<(), AnimationExecutionError> {
        if jobs.is_empty() { return Err(AnimationExecutionError::UnsupportedGraph); }
        let mut expected = Vec::with_capacity(jobs.len());
        let mut referenced = vec![false; jobs.len()];
        referenced[0] = true;
        for (index, job) in jobs.iter().enumerate() {
            let code = job.job.lock().map_err(|_| AnimationExecutionError::Lock)?.entry_point.code;
            if (index == 0 && code.is_some()) || (index != 0 && !matches!(code,
                None | Some(AnimationJobCode::FetchVbr | AnimationJobCode::FetchRaw | AnimationJobCode::SqtBlend | AnimationJobCode::PoseMatMult | AnimationJobCode::SetData | AnimationJobCode::ReuseBuffer)))
                || jobs[..index].iter().any(|other| Arc::ptr_eq(&other.job, &job.job)) {
                return Err(AnimationExecutionError::UnsupportedGraph);
            }
            for (position, &dependency) in job.dependencies.iter().enumerate() {
                if dependency <= index || dependency >= jobs.len()
                    || job.dependencies[..position].contains(&dependency) {
                    return Err(AnimationExecutionError::UnsupportedGraph);
                }
                referenced[dependency] = true;
            }
            expected.push(code);
        }
        if referenced.contains(&false) { return Err(AnimationExecutionError::UnsupportedGraph); }
        self.submit_validated_graph(jobs, &expected)
    }

    fn submit_validated_graph(self: &Arc<Self>, jobs: &[AnimationQueuedJob],
        expected: &[Option<AnimationJobCode>]) -> Result<(), AnimationExecutionError> {
        let mut owners = jobs
            .iter()
            .map(|job| job.job.lock().map_err(|_| AnimationExecutionError::Lock))
            .collect::<Result<Vec<_>, _>>()?;
        Self::validate_entry_points(&owners, jobs, expected)?;
        let mut queue = self
            .shared
            .queue
            .lock()
            .map_err(|_| AnimationExecutionError::Lock)?;
        // Multiple ACS owners share this manager. Reject re-submission of a
        // live handle, not the presence of unrelated ACS submissions.
        for owner in &owners {
            if owner
                .handle
                .backend
                .as_ref()
                .is_some_and(|backend| Arc::ptr_eq(self, backend))
                && queue
                    .slots
                    .get(usize::from(owner.handle.index))
                    .and_then(Option::as_ref)
                    .is_some_and(|slot| slot.submission == owner.handle.submission_id)
            {
                return Err(AnimationExecutionError::SubmissionStillRetained);
            }
        }
        if jobs.len() > queue.slots.len()
            || (1..=jobs.len()).any(|offset| {
                queue.slots[(queue.previous_index + offset) % queue.slots.len()].is_some()
            })
        {
            return Err(AnimationExecutionError::SubmissionStillRetained);
        }
        let mut retained = Vec::with_capacity(jobs.len());
        for (owner, queued) in owners.iter_mut().zip(jobs) {
            let index = (queue.previous_index + 1) % queue.slots.len();
            if queue.slots[index].is_some() {
                return Err(AnimationExecutionError::SubmissionStillRetained);
            }
            queue.next_submission += 1;
            let submission = queue.next_submission;
            queue.slots[index] = Some(Instance {
                submission,
                affinity: owner.entry_point.affinity,
                code: owner.entry_point.code,
                work: queued.work.clone(),
                barriers: 1,
                dependents: Vec::new(),
                phase: InstancePhase::Barrier,
            });
            queue.previous_index = index;
            queue.retained.push(index);
            retained.push(index);
            owner.seen = true;
            owner.handle.submission_id = submission;
            owner.handle.backend = Some(Arc::clone(self));
            owner.handle.index =
                u16::try_from(index).map_err(|_| AnimationExecutionError::InvalidHandle)?;
            // AddBarrier returns EVENT_TYPE_WRITE. Its host decrementer and
            // ready-queue destination are the retained instance above; native
            // pointer words are never fabricated in the scalar payload array.
            owner.start_event.kind = 1;
        }
        for (ordinal, queued) in jobs.iter().enumerate() {
            owners[ordinal].dependency_count = queued.dependencies.len();
            owners[ordinal].has_job_dependency = !queued.dependencies.is_empty();
            owners[ordinal].dependent_count = 0;
        }
        for (ordinal, queued) in jobs.iter().enumerate() {
            for (dependency_index, dependency) in queued.dependencies.iter().enumerate() {
                let dependency_owner = if dependency_index < 10 {
                    &mut owners[ordinal].dependencies[dependency_index]
                } else {
                    let extension = (dependency_index - 10) / 10;
                    if extension == owners[ordinal].dependency_extensions.len() {
                        owners[ordinal].dependency_extensions.push(Vec::with_capacity(10));
                    }
                    owners[ordinal].dependency_extensions[extension].push(super::AnimationJobDependency {
                        job: None, handle: super::AnimationJobHandle::new(), trigger: None,
                    });
                    owners[ordinal].dependency_extensions[extension].last_mut().expect("retained dependency")
                };
                dependency_owner.job = Some(Arc::clone(&jobs[*dependency].job));
                dependency_owner.trigger = Some(1); // EVENT_WHEN_JOB_END.
                dependency_owner.handle.submission_id = 0x1_0000_0000;
                let dependent_index = owners[*dependency].dependent_count;
                if dependent_index < 6 {
                    owners[*dependency].dependents[dependent_index] = Some(Arc::downgrade(&queued.job));
                } else {
                    let extension = (dependent_index - 6) / 6;
                    if extension == owners[*dependency].dependent_extensions.len() {
                        owners[*dependency].dependent_extensions.push(Vec::with_capacity(6));
                    }
                    owners[*dependency].dependent_extensions[extension].push(Arc::downgrade(&queued.job));
                }
                owners[*dependency].dependent_count += 1;
                let index = retained[ordinal];
                queue.slots[index]
                    .as_mut()
                    .ok_or(AnimationExecutionError::InvalidHandle)?
                    .barriers += 1;
                queue.slots[retained[*dependency]]
                    .as_mut()
                    .ok_or(AnimationExecutionError::InvalidHandle)?
                    .dependents
                    .push(index);
            }
        }
        for (ordinal, owner) in owners.iter_mut().enumerate() {
            release_barrier(&mut queue, retained[ordinal])?;
            owner.seen = false;
        }
        drop(queue);
        self.shared.changed.notify_all();
        Ok(())
    }

    /// Existing category0 callers retain their original local queue mask.
    pub(crate) fn wait(
        &self,
        root: &Arc<Mutex<DormantAnimationJob>>,
    ) -> Result<(), AnimationExecutionError> {
        self.wait_in_category(root, 0)
    }

    /// `WaitForJob824d53a8` uses the actual caller category, not a category
    /// inferred from whichever entry point happens to be ready on the queue.
    pub(crate) fn wait_in_category(
        &self,
        root: &Arc<Mutex<DormantAnimationJob>>,
        category: u32,
    ) -> Result<(), AnimationExecutionError> {
        let caller_affinity = affinity::selected_category_affinity(category)?;
        let (index, submission) = {
            let root = root.lock().map_err(|_| AnimationExecutionError::Lock)?;
            (usize::from(root.handle.index), root.handle.submission_id)
        };
        loop {
            let mut queue = self
                .shared
                .queue
                .lock()
                .map_err(|_| AnimationExecutionError::Lock)?;
            if let Some(error) = &queue.failure {
                return Err(error.clone());
            }
            let instance = queue
                .slots
                .get(index)
                .and_then(Option::as_ref)
                .ok_or(AnimationExecutionError::InvalidHandle)?;
            if instance.submission != submission {
                return Err(AnimationExecutionError::InvalidHandle);
            }
            if instance.phase == InstancePhase::Done {
                return Ok(());
            }
            if let Some(work) = take_ready(&mut queue, caller_affinity) {
                drop(queue);
                execute(&self.shared, work);
            } else {
                drop(
                    self.shared
                        .changed
                        .wait(queue)
                        .map_err(|_| AnimationExecutionError::Lock)?,
                );
            }
        }
    }

    /// Sync82a30088 waits this ACS's null root and reinitializes this ACS only.
    /// Retiring that completed submission must leave peer ACS jobs observable.
    pub(crate) fn release_selected(
        &self,
        jobs: &[AnimationQueuedJob],
    ) -> Result<(), AnimationExecutionError> {
        let mut selected = Vec::with_capacity(jobs.len());
        for queued in jobs {
            let job = queued
                .job
                .lock()
                .map_err(|_| AnimationExecutionError::Lock)?;
            let backend = job
                .handle
                .backend
                .as_ref()
                .ok_or(AnimationExecutionError::InvalidHandle)?;
            if !std::ptr::eq(self, Arc::as_ptr(backend)) {
                return Err(AnimationExecutionError::InvalidHandle);
            }
            let index = usize::from(job.handle.index);
            if selected.iter().any(|&(previous, _)| previous == index) {
                return Err(AnimationExecutionError::InvalidHandle);
            }
            selected.push((index, job.handle.submission_id));
        }
        let mut queue = self
            .shared
            .queue
            .lock()
            .map_err(|_| AnimationExecutionError::Lock)?;
        for &(index, submission) in &selected {
            let slot = queue
                .slots
                .get(index)
                .and_then(Option::as_ref)
                .ok_or(AnimationExecutionError::InvalidHandle)?;
            if slot.submission != submission || !queue.retained.contains(&index) {
                return Err(AnimationExecutionError::InvalidHandle);
            }
            if slot.phase != InstancePhase::Done {
                return Err(AnimationExecutionError::SubmissionStillRetained);
            }
            if slot
                .dependents
                .iter()
                .any(|dependent| !selected.iter().any(|&(index, _)| index == *dependent))
            {
                return Err(AnimationExecutionError::UnsupportedGraph);
            }
        }
        for &(index, _) in &selected {
            queue.slots[index] = None;
        }
        queue
            .retained
            .retain(|index| !selected.iter().any(|&(selected, _)| selected == *index));
        Ok(())
    }

    pub(crate) fn release_completed(&self) -> Result<(), AnimationExecutionError> {
        let mut queue = self
            .shared
            .queue
            .lock()
            .map_err(|_| AnimationExecutionError::Lock)?;
        if queue.retained.iter().any(|index| {
            queue.slots[*index]
                .as_ref()
                .is_none_or(|slot| slot.phase != InstancePhase::Done)
        }) {
            return Err(AnimationExecutionError::SubmissionStillRetained);
        }
        for index in std::mem::take(&mut queue.retained) {
            queue.slots[index] = None;
        }
        queue.failure = None;
        Ok(())
    }
}

fn release_barrier(queue: &mut Queue, index: usize) -> Result<(), AnimationExecutionError> {
    let instance = queue.slots[index]
        .as_mut()
        .ok_or(AnimationExecutionError::InvalidHandle)?;
    instance.barriers = instance
        .barriers
        .checked_sub(1)
        .ok_or(AnimationExecutionError::InvalidHandle)?;
    if instance.barriers == 0 {
        instance.phase = InstancePhase::Ready;
    }
    Ok(())
}
struct ReadyWork {
    index: usize,
    code: Option<AnimationJobCode>,
    work: Option<Arc<dyn AnimationJobWork>>,
}
fn take_ready(queue: &mut Queue, caller_affinity: u32) -> Option<ReadyWork> {
    if queue.failure.is_some() {
        return None;
    }
    let index = queue
        .retained
        .iter()
        .copied()
        .filter(|index| {
            queue.slots[*index].as_ref().is_some_and(|instance| {
                instance.phase == InstancePhase::Ready && instance.affinity & caller_affinity != 0
            })
        })
        .min_by_key(|index| {
            let instance = queue.slots[*index]
                .as_ref()
                .expect("retained ready instance");
            affinity::ready_order(instance.affinity, caller_affinity, instance.submission)
        })?;
    let instance = queue.slots[index].as_mut()?;
    instance.phase = InstancePhase::Running;
    Some(ReadyWork {
        index,
        code: instance.code,
        work: instance.work.clone(),
    })
}
fn execute(shared: &Shared, work: ReadyWork) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        match (work.code, work.work) {
            (Some(code), Some(owner)) => owner.run(code),
            (None, None) => Ok(()),
            _ => Err(AnimationExecutionError::UnsupportedEntryPoint),
        }
    }))
    .unwrap_or(Err(AnimationExecutionError::WorkerPanicked));
    if let Ok(mut queue) = shared.queue.lock() {
        if let Some(instance) = queue.slots[work.index].as_mut() {
            match result {
                Ok(()) => {
                    instance.phase = InstancePhase::Done;
                    let dependents = instance.dependents.clone();
                    for index in dependents {
                        if let Err(error) = release_barrier(&mut queue, index) {
                            queue.failure = Some(error);
                            break;
                        }
                    }
                }
                Err(error) => {
                    instance.phase = InstancePhase::Failed;
                    queue.failure = Some(error);
                }
            }
        }
    }
    shared.changed.notify_all();
}
fn worker_loop(shared: &Shared, worker_affinity: u32) {
    loop {
        let Ok(mut queue) = shared.queue.lock() else {
            return;
        };
        let work = loop {
            if queue.stopping {
                return;
            }
            if let Some(work) = take_ready(&mut queue, worker_affinity) {
                break work;
            }
            let Ok(next) = shared.changed.wait(queue) else {
                return;
            };
            queue = next;
        };
        drop(queue);
        execute(shared, work);
    }
}
