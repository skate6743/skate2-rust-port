//! Owned host execution for the recovered SetData worker.
//!
//! 828D7E08/828D7FD8 define batch boundaries and dependency collection; preparing
//! a job is not submitting or completing it (828C88D8). This synchronous host
//! adapter accepts only SetData jobs and already-materialized input buffers.
//! It is not a replacement for the full ACS scheduler or its other engines.
use super::{
    buffers::{BufferError, PoseBuffers},
    set_data::{self, SetDataCommand},
};
use std::num::NonZeroU16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JobId {
    owner: usize,
    index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueError {
    CommandCapacityExceeded,
    DependencyCapacityExceeded,
    /// Other engines must finish and supply their owned buffers before this
    /// SetData-only adapter is constructed; unresolved handles are rejected.
    UnavailableDependency,
}

struct Job {
    bone_count: u16,
    commands: Vec<SetDataCommand>,
    dependencies: Vec<JobId>,
}
impl Job {
    fn new() -> Self {
        Self {
            bone_count: 0,
            commands: Vec::new(),
            dependencies: Vec::new(),
        }
    }
}

pub struct SetDataQueue {
    owner: usize,
    buffers: PoseBuffers,
    max_work_units: NonZeroU16,
    prepared: Vec<Job>,
    current: Job,
    command_count: usize,
}
impl SetDataQueue {
    pub fn new(buffers: PoseBuffers, max_work_units: NonZeroU16) -> Self {
        Self {
            owner: super::buffers::new_owner(),
            buffers,
            max_work_units,
            prepared: Vec::new(),
            current: Job::new(),
            command_count: 0,
        }
    }

    /// A returned job identity establishes neither execution nor output readiness.
    /// None corresponds to a null dependency; current-job references are removed.
    pub fn enqueue(
        &mut self,
        bone_count: u16,
        command: SetDataCommand,
        dependencies: &[Option<JobId>],
    ) -> Result<JobId, QueueError> {
        if self.command_count == usize::from(u16::MAX) {
            return Err(QueueError::CommandCapacityExceeded);
        }
        let full = self.current.commands.len() == usize::from(self.max_work_units.get());
        let id = JobId {
            owner: self.owner,
            index: self.prepared.len() + usize::from(full),
        };
        let mut collected = if full {
            Vec::new()
        } else {
            self.current.dependencies.clone()
        };
        for dependency in dependencies.iter().flatten().copied() {
            if dependency == id {
                continue;
            }
            if dependency.owner != self.owner || dependency.index >= id.index {
                return Err(QueueError::UnavailableDependency);
            }
            if !collected.contains(&dependency) {
                if collected.len() == usize::from(u8::MAX) {
                    return Err(QueueError::DependencyCapacityExceeded);
                }
                collected.push(dependency);
            }
        }
        if full {
            self.flush();
        }
        // The native job header is rewritten on EVERY enqueue. Its last count
        // applies to external copies for all records in that job.
        self.current.bone_count = bone_count;
        self.current.commands.push(command);
        self.current.dependencies = collected;
        self.command_count += 1;
        Ok(id)
    }

    /// Finalize pending metadata; never executes a worker or exposes its output.
    pub fn flush(&mut self) -> bool {
        if self.current.commands.is_empty() {
            return false;
        }
        self.prepared
            .push(std::mem::replace(&mut self.current, Job::new()));
        true
    }

    pub fn submit(mut self) -> PendingSetData {
        self.flush();
        PendingSetData {
            owner: self.owner,
            buffers: self.buffers,
            jobs: self.prepared,
        }
    }

    /// Safe because no workers have been started by this host adapter.
    pub fn cancel(self) -> PoseBuffers {
        self.buffers
    }
}

pub struct PendingSetData {
    owner: usize,
    buffers: PoseBuffers,
    jobs: Vec<Job>,
}
impl PendingSetData {
    /// Execute complete workers in preparation order. All accepted dependencies
    /// point to earlier jobs, so their writes precede dependent reads. No native
    /// concurrency ordering beyond those dependencies is asserted.
    pub fn complete(mut self) -> Result<CompletedSetData, ExecutionFailure> {
        for (index, job) in self.jobs.iter().enumerate() {
            if let Err(cause) = set_data::execute(&mut self.buffers, job.bone_count, &job.commands)
            {
                return Err(ExecutionFailure {
                    buffers: self.buffers,
                    job: JobId {
                        owner: self.owner,
                        index,
                    },
                    cause,
                });
            }
        }
        Ok(CompletedSetData {
            buffers: self.buffers,
        })
    }
}

pub struct ExecutionFailure {
    /// Diagnostic/recovery storage only; this is not a completed pose certificate.
    pub buffers: PoseBuffers,
    pub job: JobId,
    pub cause: BufferError,
}

pub struct CompletedSetData {
    buffers: PoseBuffers,
}
impl CompletedSetData {
    pub fn buffers(&self) -> &PoseBuffers {
        &self.buffers
    }
    /// Reuse can begin only after completion releases ownership.
    pub fn into_buffers(self) -> PoseBuffers {
        self.buffers
    }
}
