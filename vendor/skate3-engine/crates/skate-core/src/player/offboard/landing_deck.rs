//! Shared LandingOnDeckManager68 for BipedAir501 and LandingOnDeck503.
//! Original S3 TU3 SHA256:
//! 431b8eba23565affdc10d137df19b06fe286244cefb3e1a13f32693e9600395a.
//! Host submits returned queries and acknowledges success; no world/solver owner.
//! PC arithmetic retains recovered formulas/refinements, not Xenon bit parity.
mod query;
mod trajectory;
mod types;
mod update;
pub use crate::air::trajectory::{QueryRequest, QueryResult, Trajectory};
use crate::player::wipeout_state::math::{add, sub};
use trajectory::{adjust, frames};
pub use types::*;

pub type Vector = [f32; 4];
pub type Frame = [Vector; 4];
pub const STEP: f32 = f32::from_bits(0x3c88_8889);
pub const GRAVITY: Vector = [0., f32::from_bits(0xc11c_cccd), 0., 0.];
const EMPTY: Trajectory = Trajectory {
    position: [0.; 4],
    velocity: [0.; 4],
    acceleration: [0.; 4],
    duration: -1.,
};

///One retained owner. Public fields permit the source-backed503 InitTraj producer
///to initialize this SAME trajectory and force flag, not a parallel state object.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Manager {
    pub trajectory_32: Trajectory,
    pub proposed_96: Trajectory,
    pub elapsed_160: f32,
    pub trajectory_valid_164: bool,
    pub ik_offset_176: Vector,
    pub vector_192: Vector,
    pub moving_contact_208: Vector,
    pub vector_224: Vector,
    pub obstruction_height_240: f32,
    pub time_to_land_244: f32,
    pub proposed_time_248: f32,
    pub completed_queries_252: u32,
    pub can_land_256: bool,
    pub force_257: bool,
    pub blocked_258: bool,
    pub tested_259: bool,
    pub hippy_hurdling_260: bool,
    pub publish_moving_contact_261: bool,
    pub pending_262: bool,
}

impl Default for Manager {
    ///Host allocation seed, NOT a substitute for503 InitTraj or Air assistance.
    fn default() -> Self {
        Self {
            trajectory_32: EMPTY,
            proposed_96: EMPTY,
            elapsed_160: 0.,
            trajectory_valid_164: false,
            ik_offset_176: [0.; 4],
            vector_192: [0.; 4],
            moving_contact_208: [0.; 4],
            vector_224: [0.; 4],
            obstruction_height_240: 0.,
            time_to_land_244: 0.,
            proposed_time_248: 0.,
            completed_queries_252: 0,
            can_land_256: false,
            force_257: false,
            blocked_258: false,
            tested_259: false,
            hippy_hurdling_260: false,
            publish_moving_contact_261: false,
            pending_262: false,
        }
    }
}
impl Manager {
    ///82D78B28: notably does NOT clear proposed96 or outstanding query262.
    pub fn reset(&mut self) {
        self.trajectory_32 = EMPTY;
        self.elapsed_160 = 0.;
        self.trajectory_valid_164 = false;
        self.ik_offset_176 = [0.; 4];
        self.vector_192 = [0.; 4];
        self.moving_contact_208 = [0.; 4];
        self.vector_224 = [0.; 4];
        self.obstruction_height_240 = 0.;
        self.time_to_land_244 = 0.;
        self.proposed_time_248 = 0.;
        self.completed_queries_252 = 0;
        self.can_land_256 = false;
        self.force_257 = false;
        self.blocked_258 = false;
        self.tested_259 = false;
        self.hippy_hurdling_260 = false;
        self.publish_moving_contact_261 = false;
    }

    ///82D78C38: preserve the native correction sign and positive-frame guard.
    pub fn correct_trajectory(&mut self, com: Vector) {
        let error = sub(self.trajectory_32.position_at(self.elapsed_160), com);
        let remaining = frames(self.time_to_land_244);
        if remaining > 0 {
            self.trajectory_32.position = add(self.trajectory_32.position, error);
            adjust(&mut self.trajectory_32, remaining, error.map(|v| -v), 0.5);
        }
    }

    ///82D79420. None means no write to skeleton3482/48, not clear those fields.
    pub fn fill(&self) -> FillOutput {
        FillOutput {
            can_land_316: self.can_land_256 && !self.blocked_258,
            hippy_hurdling_317: self.hippy_hurdling_260 && !self.blocked_258,
            moving_contact: self
                .publish_moving_contact_261
                .then_some(self.moving_contact_208),
        }
    }
}
