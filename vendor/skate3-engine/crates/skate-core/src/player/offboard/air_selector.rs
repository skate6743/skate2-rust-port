//! Persistent offboard selector72: S3 82D6BD78..82D6EA6C.
//! Separate from onboard Air. World work is submitted/staged by the game host;
//! only explicit consume calls publish results. No physical solver ownership.
mod adjustment;
mod candidates;
mod completion;
pub mod ledge;
mod math;
#[cfg(test)]
mod tests;
use super::{air_launch::Packet, biped_air::recovered};
use crate::{
    air::trajectory::{Prediction, QueryRequest, QueryResult, Trajectory},
    point_graph::PointGraph,
};
pub use recovered::selection::Candidate;
pub use recovered::{TrajectoryResult, sampling::SelectorState};
pub type Vector = [f32; 4];
pub type Frame = [Vector; 4];
pub const DT: f32 = f32::from_bits(0x3c888889);
pub const MAX_CANDIDATES: usize = 16;
pub const MESH_REJECT_MASK: u32 = 0x6000;

/// Original stock attributes, not incoming material policy.
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// AB0D9EAEBFC584E9
    pub height: f32,
    /// 620EEA2E27A371A2
    pub sphere_radius: f32,
    /// 1B6E3FF7EC72CDE1
    pub start_index: i32,
}
#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub selection_flags_2948: u32,
    pub matching_group_2952: i32,
    pub up_544: Vector,
    pub forward_224: Vector,
}
#[derive(Clone, Debug)]
pub struct Selector {
    pub sampling: SelectorState,
    pub launch: Packet,
    pub candidates: Vec<Candidate>,
    pub predictions: Vec<Prediction>,
    pub scores: Vec<f32>,
    pub selected_index: Option<usize>,
    ///6080 is NOT the shifted/animation-adjusted sampling trajectory8144.
    pub selected_candidate: Candidate,
    pub offset_8272: Vector,
    pub correction_8304: Vector,
    pub ledge_normal_8320: Vector,
    pub ledge_selected_8496: bool,
    pub just_changed_8497: bool,
    pub requery_pending_8499: bool,
    pub requery_count_8488: i32,
    pub requery_position_8352: Vector,
    pub requery_normal_8368: Vector,
}
impl Default for Selector {
    fn default() -> Self {
        Self::new(Packet::initialized(0.))
    }
}
impl Selector {
    pub fn new(launch: Packet) -> Self {
        //Constructor82D6B870 differs from Reset82D6BD78 at these fields.
        let mut sampling = SelectorState::reset_sampling(launch.scalar_100);
        sampling.restart_allowed_8493 = false;
        sampling.selection.landing_frame_8480 = 0;
        Self {
            sampling,
            launch,
            candidates: Vec::new(),
            predictions: Vec::new(),
            scores: Vec::new(),
            selected_index: None,
            selected_candidate: Candidate::reset(),
            offset_8272: [0.; 4],
            correction_8304: [0.; 4],
            ledge_normal_8320: math::UP,
            ledge_selected_8496: false,
            just_changed_8497: false,
            requery_pending_8499: false,
            requery_count_8488: 0,
            requery_position_8352: [0.; 4],
            requery_normal_8368: [0.; 4],
        }
    }
    ///82D6BD78 retains packet3904. Reset does not invent a new launch.
    pub fn reset(&mut self) {
        *self = Self::new(self.launch);
        self.sampling = SelectorState::reset_sampling(self.launch.scalar_100);
    }
    ///82D2EF20 clears only these two selector flags; retained predictions survive.
    pub fn exit(&mut self) {
        self.sampling.pending_8492 = false;
        self.sampling.preinitialized_8494 = false;
    }
    ///82D6CA58: once per launch. Invalid authored inputs fail, never get clamped
    ///into a different candidate count or padded collision request.
    pub fn begin_launch(
        &mut self,
        launch: Packet,
        gravity: Vector,
        settings: Settings,
    ) -> Result<Vec<QueryRequest>, &'static str> {
        let (candidates, offset, correction) = candidates::prepare(launch, gravity, settings)?;
        self.reset();
        self.launch = launch;
        self.sampling.selection.candidate_scalar_100 = launch.scalar_100;
        self.sampling
            .seed_fallback(launch.position_32, launch.velocity_0, gravity);
        self.offset_8272 = offset;
        self.correction_8304 = correction;
        self.candidates = candidates;
        let requests: Vec<_> = self
            .candidates
            .iter()
            .map(|c| request(c.trajectory, settings.sphere_radius))
            .collect();
        self.predictions = requests
            .iter()
            .map(|&request| Prediction {
                request,
                result: QueryResult::miss(),
            })
            .collect();
        self.scores = vec![0.; requests.len()];
        Ok(requests)
    }
    pub fn sample(
        &mut self,
        frame: i32,
        timestep: f32,
        curve: &PointGraph<8>,
        out: &mut TrajectoryResult,
    ) {
        self.sampling.sample(frame, timestep, curve, out);
    }
    ///82D6E798. Caller supplies current processed suppression flags.
    pub fn begin_requery(
        &mut self,
        flags_2472: u32,
        flags_2488: u32,
        radius: f32,
    ) -> Result<Option<QueryRequest>, &'static str> {
        if flags_2472 & 0x10000000 != 0
            || flags_2488 & 0x400000 != 0
            || !self.sampling.restart_allowed_8493
        {
            return Ok(None);
        }
        let request = request(self.selected_candidate.trajectory, radius);
        validate_request(request)?;
        self.sampling.restart_allowed_8493 = false;
        self.requery_pending_8499 = true;
        //Native requery replaces slot0; it does not replace the winning candidate.
        if let Some(slot) = self.predictions.first_mut() {
            *slot = Prediction {
                request,
                result: QueryResult::miss(),
            };
        }
        Ok(Some(request))
    }
}
pub fn request(mut trajectory: Trajectory, radius: f32) -> QueryRequest {
    //82D6CBxx/E844: f1 radius, f2 .25, f3 1.2, f4 duration2.
    trajectory.duration = 2.;
    QueryRequest {
        trajectory,
        radius,
        start_error: 0.25,
        end_error: 1.2,
    }
}
pub fn validate_request(q: QueryRequest) -> Result<(), &'static str> {
    let t = q.trajectory;
    if [t.duration, q.radius, q.start_error, q.end_error]
        .iter()
        .any(|x| !x.is_finite() || *x <= 0.)
        || [t.position, t.velocity, t.acceleration]
            .iter()
            .any(|v| v[..3].iter().any(|x| !x.is_finite()))
    {
        Err("Invalid native BipedAir trajectory request")
    } else {
        Ok(())
    }
}
