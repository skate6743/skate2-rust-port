//! PostWipeoutCheck82BD83E0's error correction before wobble and physical IK.
use crate::physics::{
    native_arithmetic::{dot3, vector_min},
    skeleton_body::SkeletonBody,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct CorrectionState {
    ///Skeleton16224, initialized zero at82BD7918.
    pub board_prediction_error: [f32; 4],
    ///Skeleton16388, initialized false at82BD7998 and set by Ground::Update.
    pub pending: bool,
}
impl CorrectionState {
    ///UpdatePostPhysics82BD8194..81B0, before collision and error feedback:
    ///actual board GetPartTransform translation minus predicted16112.
    pub fn observe_board(&mut self, actual: [f32; 4], predicted: [f32; 4]) {
        self.board_prediction_error = core::array::from_fn(|i| actual[i] - predicted[i]);
    }
    pub fn apply(
        &mut self,
        body: &mut SkeletonBody,
        ground_normal: [f32; 4],
        wipeout: bool,
        flags_2468: u32,
        flags_2476: u32,
    ) {
        apply(
            body,
            &mut self.pending,
            self.board_prediction_error,
            ground_normal,
            wipeout,
            flags_2468,
            flags_2476,
        );
    }
}

///Skeleton16224 is the already-produced board prediction error, and16388 says the
///record adjustment remains pending. This function does not synthesize errors.
pub fn apply(
    body: &mut SkeletonBody,
    pending: &mut bool,
    error: [f32; 4],
    ground_normal: [f32; 4],
    wipeout: bool,
    flags_2468: u32,
    flags_2476: u32,
) {
    if wipeout && flags_2476 & (1 << 30) != 0 {
        //82BE2AB0 moves live parts0..23 only when correction length²<1.
        if 1.0 > dot3(error, error) {
            for part in 0..24 {
                let mut frame = body.record.pose[part];
                for lane in 0..4 {
                    frame[3][lane] += error[lane];
                }
                body.set_part_transform(part, frame);
            }
        }
        *pending = false;
    } else if !wipeout && flags_2468 & (1 << 18) == 0 && *pending {
        let distance = dot3(error, ground_normal);
        let allowed = vector_min(0.0, distance);
        let offset: [f32; 4] = core::array::from_fn(|i| {
            let tangent = error[i] - ground_normal[i] * distance;
            ground_normal[i].mul_add(allowed, tangent)
        });
        //82BEC2C0 starts at record part1 and translates through23. It leaves
        //the observed board, solver states, COM and velocity history alone.
        for part in 1..24 {
            for lane in 0..4 {
                body.record.pose[part][3][lane] += offset[lane];
            }
        }
        *pending = false;
    }
}
