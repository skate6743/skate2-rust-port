//! Original offboard selector GetTrajectoryResultAtFrame82D6E3F8.
//! This selector is distinct from the on-board TrajectorySelector82D67848.
use super::{DT, TrajectoryResult, Vector};
use crate::{air::trajectory::Trajectory, point_graph::PointGraph};

///Retained data produced by offboard selection82D6CC50 and adjustment82D6DE08.
///No Default: completing a scene query must supply these actual observations.
#[derive(Clone, Copy, Debug)]
pub struct Selection {
    pub trajectory_8144: Trajectory,
    pub normal_6144: Vector,
    pub velocity_6160: Vector,
    pub position_6176: Vector,
    pub valid_6200: bool,
    pub result_present_3888: bool,
    pub landing_frame_8480: i32,
    pub scalar_8392: f32,
    pub word_8396: u32,
    ///Selected candidate+100, or launch packet+100 when index8476 is -1.
    pub candidate_scalar_100: f32,
}

#[derive(Clone, Debug)]
pub struct SelectorState {
    pub pending_8492: bool,
    pub restart_allowed_8493: bool,
    pub preinitialized_8494: bool,
    pub fallback_8208: Trajectory,
    pub selection: Selection,
    pub adjustment_8336: Vector,
    pub blend_8384: f32,
    pub elapsed_8388: f32,
    pub frame_8484: i32,
}
impl SelectorState {
    ///The game constructs this from the actual launch/selection producer.
    pub fn sample(
        &mut self,
        frame: i32,
        timestep: f32,
        blend: &PointGraph<8>,
        out: &mut TrajectoryResult,
    ) {
        self.frame_8484 = frame;
        let time = frame as f32 * DT;
        let s = self.selection;
        if self.pending_8492 {
            out.position_272 = self.fallback_8208.position_at(time);
            out.velocity_288 = self.fallback_8208.velocity_at(time);
            out.valid_404 = false;
            out.contact_position_336 = [0.; 4];
            out.normal_304 = [0., 1., 0., 0.];
            out.contact_velocity_320 = [0.; 4];
        } else {
            self.elapsed_8388 += timestep;
            self.blend_8384 = blend.evaluate(self.elapsed_8388);
            let first = s.trajectory_8144.position_at(time);
            let second = self.fallback_8208.position_at(time);
            let weight = self.blend_8384;
            out.position_272 =
                core::array::from_fn(|i| second[i].mul_add(1. - weight, first[i] * weight));
            //The velocity is the selected trajectory velocity, NOT the
            //derivative of the position blend and NOT the launch velocity.
            out.velocity_288 = s.trajectory_8144.velocity_at(time);
            out.valid_404 = s.valid_6200;
            out.contact_position_336 = s.position_6176;
            out.normal_304 = s.normal_6144;
            out.contact_velocity_320 = s.velocity_6160;
        }
        out.time_remaining_384 = if s.result_present_3888 {
            s.landing_frame_8480.wrapping_sub(frame) as f32 * DT
        } else {
            0.
        };
        out.duration_388 = s.scalar_8392;
        out.frame_400 = frame;
        out.adjustment_352 = self.adjustment_8336;
        let (apex, time) = s.trajectory_8144.highest_position();
        out.apex_368 = apex;
        out.apex_time_396 = time;
        out.word_408 = s.word_8396;
        out.scalar_392 = s.candidate_scalar_100;
    }
}
