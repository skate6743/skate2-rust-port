//! PhysicsSlideGround101: Enter82D3A700, Exit82D3A828 and board82D3A900.
mod forces;
pub use forces::{SlideInput, SlideSettings, SlideSurface, angular_correction, sliding_force};

#[derive(Clone, Copy, Debug)]
pub struct SlideState {
    pub start_speed: f32,
    pub steering_push: f32,
    pub damped_turn: f32,
    pub flag48: bool,
    pub wall_riding: bool,
}
impl SlideState {
    /// Host storage; Enter defines every represented native field before use.
    pub fn new() -> Self {
        Self {
            start_speed: 0.0,
            steering_push: 1.0,
            damped_turn: 0.0,
            flag48: false,
            wall_riding: false,
        }
    }
    pub fn enter(&mut self, speed: f32) {
        self.start_speed = speed;
        self.exit();
    }
    pub fn exit(&mut self) {
        self.flag48 = false;
        self.wall_riding = false;
        self.steering_push = 1.0;
        self.damped_turn = 0.0;
    }
}
