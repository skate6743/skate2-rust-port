//! OffboardGroundAnalyzer query ownership (TU3 82D81068/82D811C8).
//!
//! Submitted queries become visible only through the ordered analyzer refresh.
mod analyzer_math;
mod candidate;
mod classification;
mod collection;
mod generation;
mod infill;
mod prefix;
mod probes;
mod profile;
mod publication;
mod samples;
mod sweep;
#[cfg(test)]
mod tests;

use crate::air::trajectory::QueryResult;
pub use prefix::ContactPrefix;
pub use probes::{Batch, Input, LineProbe, ProbeLayout};
pub use samples::{ContactSample, Samples};
pub use sweep::query_sweep;

pub type Vector = [f32; 4];
pub type Frame = [Vector; 4];
pub const UP: Vector = [0., 1., 0., 0.];
pub const ZERO: Vector = [0.; 4];
pub const IDENTITY: Frame = [[1., 0., 0., 0.], UP, [0., 0., 1., 0.], ZERO];

/// Unit face normal shared by the toolkit's scene adapter and query kernels.
pub fn triangle_normal([a, b, c]: [Vector; 3]) -> Vector {
    let normal = cross(sub(b, a), sub(c, a));
    let square = dot(normal, normal);
    let mut inverse = crate::physics::reciprocal_sqrt::estimate(square);
    for _ in 0..2 {
        inverse = (inverse * 0.5).mul_add((-square).mul_add(inverse * inverse, 1.), inverse);
    }
    scale(normal, inverse)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineHit {
    pub position: Vector,
    pub normal: Vector,
    pub fraction: f32,
    pub surface: u16,
    pub mesh_frame: Frame,
    pub geometry: u32,
}

/// Actual batch results, indexed in submission order, not sorted by distance.
#[derive(Clone, Debug)]
pub struct QueryResults {
    pub trajectories: [QueryResult; 3],
    pub lines: Vec<Option<LineHit>>,
    /// Authored static query edges in canonical traversal order, at most forty.
    pub edges: Vec<[Vector; 2]>,
}

/// The scene owns geometry; the toolkit owns query lifetime and retained output.
pub trait Scene {
    type Error;
    fn execute(&self, batch: &Batch) -> Result<QueryResults, Self::Error>;
}

#[derive(Clone, Debug)]
pub struct Collected {
    pub batch: Batch,
    pub results: QueryResults,
    pub prefix: ContactPrefix,
    pub samples: Samples,
}

/// Canonical lifetime of submission316, readiness320, and input0..96.
/// No executor or scene reference is retained across frames.
#[derive(Clone, Debug)]
pub struct Owner {
    layout: ProbeLayout,
    pending: Option<(Batch, QueryResults)>,
    readiness: u32,
    prefix: ContactPrefix,
    candidate: candidate::Candidate,
    history: classification::History,
}
impl Default for Owner {
    fn default() -> Self {
        Self {
            layout: ProbeLayout::stock(),
            pending: None,
            readiness: 0,
            prefix: ContactPrefix::reset(),
            candidate: candidate::Candidate::default(),
            history: classification::History::default(),
        }
    }
}
impl Owner {
    /// Update1_ProcessInput82DB405C..4094 ages then completes the preceding
    /// batch before physical input is processed, regardless of active state.
    pub fn begin_input(&mut self) {
        self.readiness = self.readiness.saturating_sub(1);
        self.refresh();
    }

    /// Ground Reset82D30C20..40 / physical reset82DB93B0..CC.
    /// Complete outstanding work first; only readiness and classification
    /// history are cleared. The original does not erase the retained candidate.
    pub fn reset_history(&mut self) {
        self.refresh();
        self.readiness = 0;
        self.history = classification::History::default();
    }

    pub fn prefix(&self) -> ContactPrefix {
        self.prefix
    }

    /// GroundSync82D32164 supplies the seven vectors after Skeleton Update.
    /// Host execution is synchronous, but visibility remains next-consume only.
    pub fn submit<S: Scene>(
        &mut self,
        input: Input,
        matching_group: i32,
        scene: &S,
    ) -> Result<(), S::Error> {
        let batch = self.layout.prepare(input, matching_group);
        let results = scene.execute(&batch)?;
        self.readiness = 0;
        self.pending = Some((batch, results));
        Ok(())
    }

    /// Consume the prior submission through every analyzer stage. Candidate and
    /// classification history persist; query observations are frame-local.
    pub fn refresh(&mut self) -> Option<Collected> {
        //82D81610 resets the publication even when there is no pending batch.
        self.prefix = ContactPrefix::reset();
        self.readiness = 30;
        let (batch, results) = self.pending.take()?;
        let mut prefix = ContactPrefix::reset();
        prefix.consume_support(batch.input, &results.trajectories, self.candidate.flags);
        let mut samples = collection::collect(&batch, &self.layout, &results);
        infill::insert_obstacles(batch.input, &mut samples);
        infill::correct_normals(batch.input, &mut samples);
        let (obstruction, active_count) = profile::simplify(batch.input, &mut samples.ground);
        prefix.distance_172 = obstruction;
        let profile = profile::build(batch.input, &samples.ground[..active_count]);
        let mut candidates = generation::generate(
            batch.input,
            &profile,
            &prefix,
            obstruction,
            self.candidate.flags,
        );
        publication::publish(
            batch.input,
            &profile,
            &samples,
            &mut candidates,
            obstruction,
            &mut self.candidate,
            &mut self.history,
            &mut prefix,
        );
        self.prefix = prefix;
        Some(Collected {
            batch,
            results,
            prefix,
            samples,
        })
    }

    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn readiness(&self) -> u32 {
        self.readiness
    }
}

pub(super) fn dot(a: Vector, b: Vector) -> f32 {
    crate::physics::native_arithmetic::dot3(a, b)
}
pub(super) fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(super) fn scale(a: Vector, s: f32) -> Vector {
    a.map(|v| v * s)
}
pub(super) fn cross(a: Vector, b: Vector) -> Vector {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        0.,
    ]
}
pub(super) fn length(v: Vector) -> f32 {
    let square = dot(v, v);
    if square == 0. {
        return 0.;
    }
    let mut inverse = crate::physics::reciprocal_sqrt::estimate(square);
    for _ in 0..2 {
        inverse = (inverse * 0.5).mul_add((-square).mul_add(inverse * inverse, 1.), inverse);
    }
    square * inverse
}
