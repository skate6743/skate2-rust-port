//! Offboard selector commit82D6CC50, after scoring82D6D020.
//! Original TU3 disassembly; this is not the on-board selector's selection rule.
use super::{
    DT, Vector,
    sampling::{Selection, SelectorState},
};
use crate::air::trajectory::{Prediction, Trajectory};

///Candidate4032+128*i. The trajectory occupies0..63;100 is contact-position Y.
#[derive(Clone, Copy, Debug)]
pub struct Candidate {
    pub trajectory: Trajectory,
    pub normal_64: Vector,
    pub contact_velocity_80: Vector,
    pub contact_position_96: Vector,
    pub start_frame_112: i32,
    pub landing_frame_116: i32,
    pub valid_120: bool,
    pub special_121: bool,
}
impl Candidate {
    ///82D6BD78 clears every candidate, independently of the launch packet.
    pub fn reset() -> Self {
        Self {
            trajectory: cleared_trajectory(),
            normal_64: [0.; 4],
            contact_velocity_80: [0.; 4],
            contact_position_96: [0.; 4],
            start_frame_112: 0,
            landing_frame_116: 0,
            valid_120: false,
            special_121: false,
        }
    }
}

pub fn cleared_trajectory() -> Trajectory {
    Trajectory {
        position: [0.; 4],
        velocity: [0.; 4],
        acceleration: [0.; 4],
        duration: -1.,
    }
}

///82D6CC88..CCBC uses a strict comparison; equal scores keep the earlier index.
///The native caller always supplies at least one candidate (packet108 >=1).
pub fn selected_index(scores: &[f32]) -> Option<usize> {
    if scores.is_empty() {
        return None;
    }
    let mut best = f32::from_bits(0xccbe_bc20); //Original822F9420, -100000000.
    let mut index = 0;
    for (candidate, &score) in scores.iter().enumerate() {
        if score > best {
            best = score;
            index = candidate;
        }
    }
    Some(index)
}

fn shift(trajectory: &mut Trajectory, time: f32) {
    let position = trajectory.position_at(time);
    let velocity = trajectory.velocity_at(time);
    trajectory.position = position;
    trajectory.velocity = velocity;
}

impl SelectorState {
    ///82D6BD78's sample-visible reset. Packet3904 is NOT reset by that function;
    ///when index8476 is -1, sampling still reads its retained scalar4004.
    pub fn reset_sampling(retained_launch_scalar_100: f32) -> Self {
        Self {
            pending_8492: false,
            restart_allowed_8493: true,
            preinitialized_8494: false,
            fallback_8208: cleared_trajectory(),
            selection: Selection {
                trajectory_8144: cleared_trajectory(),
                normal_6144: [0.; 4],
                velocity_6160: [0.; 4],
                position_6176: [0.; 4],
                valid_6200: false,
                result_present_3888: false,
                landing_frame_8480: 1000,
                scalar_8392: 0.,
                word_8396: 0,
                candidate_scalar_100: retained_launch_scalar_100,
            },
            adjustment_8336: [0.; 4],
            blend_8384: 0.,
            elapsed_8388: 0.,
            frame_8484: 0,
        }
    }

    ///82D6CA58's fallback seed. Call once per actual launch, never per Update.
    ///Candidate construction and query submission follow this stage.
    pub fn seed_fallback(&mut self, position: Vector, velocity: Vector, gravity: Vector) {
        self.fallback_8208 = Trajectory {
            position,
            velocity,
            acceleration: gravity,
            duration: f32::from_bits(0x4000_0000), //Original82060C50=2 seconds.
        };
        self.pending_8492 = true;
    }

    ///Commit the candidate chosen by82D6D020/CC50. Return native byte8497's
    ///set event; a miss does not clear that retained flag or8392/8396.
    pub fn commit(
        &mut self,
        candidate: Candidate,
        prediction: &mut Prediction,
        offset_8272: Vector,
    ) -> bool {
        self.pending_8492 = false;
        let time = candidate.start_frame_112.wrapping_neg() as f32 * DT;
        if prediction.result.valid() {
            prediction.result.contact_frame = prediction
                .result
                .contact_frame
                .wrapping_add(candidate.start_frame_112);
            prediction.result.contact_time -= time;
        }
        //The query trajectory is shifted back but does not receive body offset.
        shift(&mut prediction.request.trajectory, time);
        let mut trajectory = candidate.trajectory;
        shift(&mut trajectory, time);
        trajectory.position = core::array::from_fn(|i| trajectory.position[i] + (-offset_8272[i]));
        let mut duration = self.selection.scalar_8392;
        let mut word = self.selection.word_8396;
        let contact = prediction.result.valid();
        if contact {
            duration = candidate.landing_frame_116 as f32 * DT;
            word = if candidate.special_121 {
                2
            } else {
                (prediction.result.surface >> 7) & 0x1f
            };
        }
        self.selection = Selection {
            trajectory_8144: trajectory,
            normal_6144: candidate.normal_64,
            velocity_6160: candidate.contact_velocity_80,
            position_6176: candidate.contact_position_96,
            valid_6200: candidate.valid_120,
            result_present_3888: true,
            landing_frame_8480: candidate.landing_frame_116,
            scalar_8392: duration,
            word_8396: word,
            candidate_scalar_100: candidate.contact_position_96[1],
        };
        self.preinitialized_8494 = true;
        contact
    }
}
