//! BipedAir501 state from original TU3 IDA, build SHA256
//!431b8eba23565affdc10d137df19b06fe286244cefb3e1a13f32693e9600395a.
//! Native result offsets identify fields;288 is velocity,304 is a normal.
#[path = "height.rs"]
pub mod height;
#[path = "post.rs"]
pub mod post;
#[path = "result.rs"]
pub mod result;
#[path = "sampling.rs"]
pub mod sampling;
#[path = "selection.rs"]
pub mod selection;
pub use result::{OffBoardOutput, TrajectoryResult};
#[path = "collision.rs"]
pub mod collision;
#[path = "feet.rs"]
pub mod feet;
use super::super::air_launch::math;
#[path = "orientation.rs"]
mod orientation;
use super::super::{air_launch, cadence::BipedCadence};
use height::{dot, select, sub};
pub type Vector = [f32; 4];
pub type Frame = [Vector; 4];
pub const DT: f32 = f32::from_bits(0x3c888889);
const IDENTITY: Frame = [
    [1., 0., 0., 0.],
    [0., 1., 0., 0.],
    [0., 0., 1., 0.],
    [0.; 4],
];

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub frame_80: Frame,
    pub frame_144: Frame,
    pub frame_208: Frame,
    pub result: TrajectoryResult,
    pub body_target_416: Vector,
    pub height_432: f32,
    pub body_offset_436: f32,
    pub blend_440: f32,
    pub time_remaining_444: f32,
    pub duration_448: f32,
    pub frame_452: i32,
    pub local_contact_464: Vector,
    pub initial_up_480: Vector,
    pub adjustment_496: Vector,
    pub vector_512: Vector,
    pub restart_normal_528: Vector,
    pub flags_544_550: [bool; 7],
    ///Host publication of the shared Biped cadence; never reset on Air entry.
    pub cadence: BipedCadence,
    pub active: bool,
}
impl Default for State {
    fn default() -> Self {
        let mut state = Self {
            frame_80: IDENTITY,
            frame_144: IDENTITY,
            frame_208: IDENTITY,
            result: TrajectoryResult::default(),
            body_target_416: [0.; 4],
            height_432: 0.,
            body_offset_436: 0.,
            blend_440: 0.,
            time_remaining_444: 0.,
            duration_448: -1.,
            frame_452: 0,
            local_contact_464: [0.; 4],
            initial_up_480: [0.; 4],
            adjustment_496: [0.; 4],
            vector_512: [0.; 4],
            restart_normal_528: [0.; 4],
            flags_544_550: [false; 7],
            cadence: BipedCadence::default(),
            active: false,
        };
        state.reset();
        state
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EnterInput {
    pub animation_frame: Frame,
    pub body_position_15872: Vector,
    pub body_position_15936: Vector,
    pub position_592: Vector,
    pub up_544: Vector,
    pub flags_2484: u32,
}

impl State {
    ///82D2EDD8, including selective result reset and byte548=true.
    pub fn reset(&mut self) {
        self.flags_544_550 = [false, false, false, false, true, false, false];
        self.height_432 = 0.;
        self.body_offset_436 = 0.;
        self.blend_440 = 0.;
        self.time_remaining_444 = 0.;
        self.duration_448 = -1.;
        self.frame_452 = 0;
        self.body_target_416 = [0.; 4];
        self.result.reset();
        self.local_contact_464 = [0.; 4];
        self.adjustment_496 = [0.; 4];
        self.vector_512 = [0., 0., 1., 0.];
        self.restart_normal_528 = [0., 0., 1., 0.];
    }

    ///State-local82D2E5C0. Caller performs manager/feet reset and conditional
    ///launch, then Biped.place between these two explicit entry stages.
    pub fn begin_enter(&mut self, input: EnterInput) {
        self.reset();
        self.begin_enter_after_reset(input);
    }
    ///Host places the shared landing reset between Air reset and initialization.
    pub fn begin_enter_after_reset(&mut self, input: EnterInput) {
        self.frame_80 = input.animation_frame;
        self.flags_544_550[6] = input.flags_2484 & 0x4000 != 0;
        self.frame_144 = IDENTITY;
        self.frame_208 = input.animation_frame;
        self.initial_up_480 = input.animation_frame[1];
        self.active = true;
    }
    pub fn finish_enter(&mut self, input: EnterInput) {
        self.body_target_416 = input.body_position_15872;
        self.height_432 = dot(
            sub(input.body_position_15872, input.position_592),
            input.up_544,
        );
        self.body_offset_436 = dot(
            sub(input.body_position_15936, input.body_position_15872),
            input.up_544,
        );
    }
    ///Only Enter's !selector8494 branch and contact restart call this producer.
    pub fn launch_packet(
        &self,
        input: &air_launch::Processed,
        biped: &super::super::controller::State,
        turn: &crate::point_graph::PointGraph<8>,
        settings: air_launch::Settings,
    ) -> Result<air_launch::Packet, &'static str> {
        //82D7BA78 overwrites104 unconditionally, so the initializer's
        //unwritten104 cannot escape the producer. No retained gameplay value.
        let mut packet = air_launch::Packet::initialized(0.);
        air_launch::produce(&mut packet, biped, turn, settings, input, false)?;
        Ok(packet)
    }
    ///82D2EF20 clears the shared selector, not the landing manager or result.
    pub fn exit(&mut self, selector: &mut sampling::SelectorState) {
        selector.pending_8492 = false;
        selector.preinitialized_8494 = false;
        self.active = false;
    }
    pub fn begin_update(&mut self) -> i32 {
        self.frame_452 = self.frame_452.wrapping_add(1);
        self.frame_452
    }
    ///82D2F170..434, after frame sampling and first-valid orientation setup.
    pub fn update_times(&mut self) {
        let duration = self.result.duration_388;
        self.duration_448 = select(duration - DT, duration, DT);
        if self.result.valid_404 {
            self.time_remaining_444 = self.result.time_remaining_384;
            let blend = 1. - self.time_remaining_444 / self.duration_448;
            let lower = select(-blend, 0., blend);
            self.blend_440 = select(1. - lower, lower, 1.);
        } else {
            self.blend_440 = 0.;
            self.time_remaining_444 = 10.;
            self.duration_448 = 10.;
        }
    }
    ///82D2F564..650. Receives the SAME Biped cadence used while walking.
    pub fn update_cadence(
        &mut self,
        biped: &mut super::super::controller::State,
        requested_phase: f32,
    ) {
        if self.result.valid_404 {
            biped.correction_target_592 = self.result.contact_position_336;
            let phase = &mut biped.cadence.phase;
            if requested_phase >= 0. && self.time_remaining_444 > 0. {
                phase.forward_target = true;
                let delta = requested_phase - phase.phase;
                phase.target = requested_phase;
                phase.duration = Some(self.time_remaining_444);
                phase.rate = if delta < 0. {
                    (delta + 1.) / self.time_remaining_444
                } else {
                    delta / self.time_remaining_444
                };
            }
            phase.advance();
            let frame = biped.motion.frame_0;
            let delta = sub(self.result.contact_position_336, frame[3]);
            self.local_contact_464 = [
                dot(frame[0], delta),
                dot(frame[1], delta),
                dot(frame[2], delta),
                0.,
            ];
        } else {
            self.local_contact_464 = [0., 0., 1., 0.];
        }
        self.cadence = biped.cadence;
    }
    pub fn output(&self) -> OffBoardOutput {
        OffBoardOutput {
            scalar_32: self.time_remaining_444,
            vector_64: self.result.velocity_288,
            scalar_92: self.duration_448,
            vector_96: self.local_contact_464,
            word_144: self.result.word_408,
            scalar_148: self.result.scalar_392,
            scalar_152: self.frame_452 as f32 * DT,
            scalar_156: self.result.apex_time_396,
            vector_160: self.initial_up_480,
            vector_176: self.frame_80[3],
            vector_192: self.result.normal_304,
            vector_208: self.result.contact_position_336,
            vector_224: self.frame_144[2],
            vector_240: self.result.apex_368,
            flag_320: self.time_remaining_444 <= DT,
            flag_328: self.flags_544_550[4],
            flag_331: true,
        }
    }
}
