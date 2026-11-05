//! Persistent animation-space board16016 and the Biped reckoning callback.
use super::{ReckoningUpdate, Transform};
use crate::skate3_native::physics::{air_reckoning::AirReckoning, riding_outputs::RidingOutputs};
use skate3_core::{
    physics::skeleton_animation_record::IDENTITY,
    player::{input_phase::ProcessedPhysicsInput, offboard::ground_reckoning},
    point_graph::PointGraph,
};
use skate_data::collections::Collections;

pub(crate) struct State {
    //82BDE158..17C copies mapped12624 here only when2484bit0 is clear.
    //This must never alias physical_board12496 or be reset on each update.
    pub(super) retained_board: Transform,
    ground_normal_smoothing: [f32; 4],
    tilt_vs_rotation: PointGraph<8>,
    tilt_vs_slope: PointGraph<8>,
}

impl State {
    pub(crate) fn load(data: &Collections) -> Result<Self, String> {
        let graph = |field| -> Result<PointGraph<8>, String> {
            let words = data
                .words::<16>("physics_reckoning", "default", field)?
                .map(f32::from_bits);
            Ok(PointGraph {
                x: words[..8].try_into().unwrap(),
                y: words[8..].try_into().unwrap(),
            })
        };
        Ok(Self {
            //Skeleton ctor82BD7760..7884 initializes16016 to identity.
            retained_board: IDENTITY,
            ground_normal_smoothing: data
                .words::<4>("physics_reckoning", "default", "GroundNormalSmoothing")?
                .map(f32::from_bits),
            tilt_vs_rotation: graph("TiltVsRotGround")?,
            tilt_vs_slope: graph("TiltVsSlopeGround")?,
        })
    }

    ///Apply after GeneralUpdate and trajectory clear, replacing the riding job.
    ///82BDE2CC explicitly passes r7=1: consume physical body spin2812.
    pub(crate) fn finish_reckoning(
        &self,
        update: ReckoningUpdate,
        riding: &mut RidingOutputs,
        air: &mut AirReckoning,
        processed: &ProcessedPhysicsInput,
        physical_body_spin: f32,
    ) -> ground_reckoning::Output {
        let previous = riding.reckoning.up;
        ground_reckoning::update(
            &mut riding.reckoning,
            &mut riding.reckoning_frames,
            &mut riding.body_spin,
            &mut air.state,
            ground_reckoning::Settings {
                ground_normal_smoothing: self.ground_normal_smoothing,
                tilt_vs_rotation: &self.tilt_vs_rotation,
                tilt_vs_slope: &self.tilt_vs_slope,
            },
            ground_reckoning::Input {
                previous_up: [previous.x, previous.y, previous.z, 0.0],
                requested_up: update.up,
                requested_forward: update.forward,
                blend: update.blend,
                reverse_stance: processed.flags_2468 & 0x100000 != 0,
                enable_body_spin_input: true,
                physical_body_spin_2812: physical_body_spin,
            },
        )
    }
}
