//! Original82D81610 inactive-query branch and82D2DDD0 result reset. Pending
//! geometry is explicit; this owner never treats an unimplemented query as a miss.
use skate3_core::physics::skeleton_animation_record::{AnimationPartTransform, IDENTITY};
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct PreInputResult {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub frame: AnimationPartTransform,
    pub vector96: [f32; 4],
    pub vector112: [f32; 4],
    pub vector128: [f32; 4],
    pub vector144: [f32; 4],
    pub scalar160: f32,
    pub word164: u32,
    pub scalar168: f32,
    pub scalar172: f32,
    pub word176: u32,
    pub flags180: u32,
}
impl PreInputResult {
    fn reset() -> Self {
        let mut frame = IDENTITY;
        frame[3] = [0.; 4];
        Self {
            position: [0.; 4],
            normal: [0., 1., 0., 0.],
            frame,
            vector96: [0.; 4],
            vector112: [0., 1., 0., 0.],
            vector128: [0.; 4],
            vector144: [0.; 4],
            scalar160: 0.,
            word164: 0,
            scalar168: f32::from_bits(0x5015_02f9),
            scalar172: f32::from_bits(0x5015_02f9),
            word176: 0,
            flags180: 0,
        }
    }
}
pub(crate) struct PreInputManager {
    pub pending_geometry: bool,
    pub result: PreInputResult,
    pub result_counts: [u32; 3],
}
impl PreInputManager {
    pub fn new() -> Self {
        //82D81100 clears pending316; result reset called by ctor82D8108C.
        Self {
            pending_geometry: false,
            result: PreInputResult::reset(),
            result_counts: [0; 3],
        }
    }
    pub fn prepare(&mut self, counter: &mut u32) -> Result<(), String> {
        self.result = PreInputResult::reset();
        self.result_counts = [0; 3];
        *counter = 30;
        if self.pending_geometry {
            return Err(
                "Pending82D811C8 trajectory batch requires its complete82D81610 geometry consumer"
                    .into(),
            );
        }
        Ok(())
    }
}
