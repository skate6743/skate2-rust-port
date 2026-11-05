//! TU3 82D2DAB8 result storage and 82D30300 OffBoard publication.
use super::Vector;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrajectoryResult {
    pub position_272: Vector,
    pub velocity_288: Vector,
    pub normal_304: Vector,
    pub contact_velocity_320: Vector,
    pub contact_position_336: Vector,
    pub adjustment_352: Vector,
    pub apex_368: Vector,
    pub time_remaining_384: f32,
    pub duration_388: f32,
    pub scalar_392: f32,
    pub apex_time_396: f32,
    pub frame_400: i32,
    pub valid_404: bool,
    pub word_408: u32,
}
impl TrajectoryResult {
    ///82D2DAB8 retains result+80/+96/+124. Do not zero the whole packet.
    pub fn reset(&mut self) {
        self.position_272 = [0.0; 4];
        self.velocity_288 = [0.0; 4];
        self.normal_304 = [0.0; 4];
        self.contact_velocity_320 = [0.0; 4];
        self.contact_position_336 = [0.0; 4];
        self.time_remaining_384 = 0.0;
        self.duration_388 = 0.0;
        self.scalar_392 = 0.0;
        self.frame_400 = 0;
        self.valid_404 = false;
        self.word_408 = 0;
    }
}
///Every offset is relative to PhysOut's OffBoard child, never Air.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OffBoardOutput {
    pub scalar_32: f32,
    pub vector_64: Vector,
    pub scalar_92: f32,
    pub vector_96: Vector,
    pub word_144: u32,
    pub scalar_148: f32,
    pub scalar_152: f32,
    pub scalar_156: f32,
    pub vector_160: Vector,
    pub vector_176: Vector,
    pub vector_192: Vector,
    pub vector_208: Vector,
    pub vector_224: Vector,
    pub vector_240: Vector,
    pub flag_320: bool,
    pub flag_328: bool,
    pub flag_331: bool,
}
