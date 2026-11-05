//! Canonical Biped owner and original ground dispatcher82D7C818.
//! Constructor82D7AFD8/reset82D7B1C0; S3 SHA256
//!431b8eba23565affdc10d137df19b06fe286244cefb3e1a13f32693e9600395a.
mod output;
mod placement;
mod state;
mod step;
pub(super) use output::build_frame;
pub use placement::PlacementInput;
#[cfg(test)]
mod tests;
use super::{movement_intent, movement_velocity};
use crate::point_graph::PointGraph;
pub use output::GroundResult;
pub use state::{ClipMetric, State};
pub type Vector = [f32; 4];
pub type Frame = [Vector; 4];

/// Validated stock physics_biped curves, supplied by the data boundary.
pub struct Settings {
    pub movement_intent: movement_intent::Settings,
    pub movement_velocity: movement_velocity::Settings,
    pub slide_vs_slope: PointGraph<8>,
    pub slide_vs_speed: PointGraph<8>,
}

/// Actual448-byte ground-job content as typed values. Ground-state geometry/
/// processed controls and delayed scene query consumption happen before this job.
/// No default implementation: a caller must supply its actual observations.
#[derive(Clone, Copy, Debug)]
pub struct GroundJob {
    pub contact_position: Vector,             //0
    pub contact_normal: Vector,               //16
    pub support_frame: Frame,                 //32..80
    pub target_position: Vector,              //96
    pub target_normal: Vector,                //112
    pub edge_position: Vector,                //128
    pub edge_normal: Vector,                  //144
    pub flags: u32,                           //176
    pub support_id: u32,                      //180
    pub collision_displacements: [Vector; 2], //192/208
    pub animation_motion: Vector,             //224
    pub animation_velocity: Vector,           //240
    pub desired_direction: Vector,            //256
    pub animation_position: Vector,           //272
    pub requested_duration: f32,              //288
    pub mirrored: bool,                       //292
    pub requested_phase: f32,                 //296
    pub override_duration: f32,               //300
    pub animation_directed: bool,             //304
    pub movement: f32,                        //308
    pub steering: f32,                        //312
    pub sprint_pressed: bool,                 //316
    pub suppress_lean: bool,                  //317
    pub suppress_minimum: bool,               //318
    pub target_frame_present: bool,           //352
    pub edge_active: bool,                    //353
    pub target_frame: Frame,                  //368..416
    pub ignore_obstacle: bool,                //432
}

pub struct Controller {
    pub state: State,
    pub settings: Settings,
}
impl Controller {
    /// Metrics are actual OffBoard walk/run/sprint AnimTransZ query results.
    /// None means the original query returned absent, whose native speed is0.
    pub fn new(settings: Settings, metrics: [Option<ClipMetric>; 3]) -> Self {
        Self {
            state: State::new(metrics),
            settings,
        }
    }
    /// Original Reset preserves cadence and positional-correction vectors.
    /// Physical Enter placement is a separate lifecycle operation.
    pub fn reset(&mut self) {
        self.state.reset();
    }
    pub fn place(&mut self, input: PlacementInput) {
        self.state.place(input);
    }
    pub fn step_ground(&mut self, job: &GroundJob) -> GroundResult {
        step_ground(&mut self.state, &self.settings, job)
    }
}

/// Run the recovered ground producer against an externally owned state.  The
/// game coordinator owns the state lifetime; this keeps the numerical producer
/// reusable without constructing a second controller every tick.
pub fn step_ground(state: &mut State, settings: &Settings, job: &GroundJob) -> GroundResult {
    step::update(state, settings, job);
    output::export(state)
}
