//! Trajectory and board-orientation publications inside ProcessData82BD8918.
use super::{
    native_arithmetic::reciprocal_estimate,
    skeleton_animation_record::{AnimationPartTransform, IDENTITY, compose_affine},
};

pub struct SkeletonMotion {
    ///Skeleton12048,12112,12176. Later skeleton modes may set the next frame.
    pub trajectory: AnimationPartTransform,
    pub inverse_trajectory: AnimationPartTransform,
    pub next_trajectory: AnimationPartTransform,
    ///Skeleton16320: current trajectory translation/dt, transformed to world.
    pub velocity_world: [f32; 4],
    ///Skeleton16500, initialized zero by82BD7A6C.
    previous_board_at_y: f32,
}

impl Default for SkeletonMotion {
    fn default() -> Self {
        //82BD7260 seeds the three frames from82139A10/20/30 and zero pos.
        Self {
            trajectory: IDENTITY,
            inverse_trajectory: IDENTITY,
            next_trajectory: IDENTITY,
            velocity_world: [0.; 4],
            previous_board_at_y: 0.,
        }
    }
}

impl SkeletonMotion {
    ///82BE3538 resets only the board-orientation history, not trajectory.
    pub fn reset_board_orientation_history(&mut self) {
        self.previous_board_at_y = 0.0;
    }

    ///82BD89D0..8BA0. Bone0 is the source trajectory, not the animated foot.
    ///Preserve the two reciprocal refinements after the shared estimate model.
    pub fn process_trajectory(
        &mut self,
        trajectory_bone: &AnimationPartTransform,
        animation_to_world: &AnimationPartTransform,
        dt: f32,
    ) {
        self.trajectory = compose_affine(trajectory_bone, &self.next_trajectory);
        self.next_trajectory = self.trajectory;
        let mut reciprocal = reciprocal_estimate(dt);
        for _ in 0..2 {
            let error = (-reciprocal).mul_add(dt, 1.0);
            reciprocal = reciprocal.mul_add(error, reciprocal);
        }
        let velocity = self.trajectory[3].map(|v| v * reciprocal);
        self.velocity_world = std::array::from_fn(|lane| {
            let x = animation_to_world[0][lane] * velocity[0];
            let y = animation_to_world[1][lane].mul_add(velocity[1], x);
            animation_to_world[2][lane].mul_add(velocity[2], y)
        });
        let source = self.trajectory;
        self.inverse_trajectory = std::array::from_fn(|axis| {
            if axis < 3 {
                [source[0][axis], source[1][axis], source[2][axis], 0.]
            } else {
                [0.; 4]
            }
        });
        self.inverse_trajectory[3] = std::array::from_fn(|lane| {
            let z = (0.0 - source[3][2]) * self.inverse_trajectory[2][lane];
            let y = (0.0 - source[3][1]).mul_add(self.inverse_trajectory[1][lane], z);
            (0.0 - source[3][0]).mul_add(self.inverse_trajectory[0][lane], y)
        });
    }

    ///82BD8CB0..8D2C precedes the landing/board-offset adjustment. Only sets
    ///the side-on flag; this stage does not clear an already published flag.
    pub fn publish_unadjusted_board(board: &AnimationPartTransform, flags_2472: &mut u32) {
        if board[0][1].abs() > f32::from_bits(0x3f35_c28f) {
            *flags_2472 |= 0x8000;
        }
    }

    ///82BD8E14..8E50 uses board At.y, not height or translation. Returns the
    ///difference published to Processed2768 and updates its complementary bits.
    pub fn publish_adjusted_board(
        &mut self,
        board: &AnimationPartTransform,
        flags_2468: &mut u32,
    ) -> f32 {
        let at_y = board[2][1];
        let delta = at_y - self.previous_board_at_y;
        self.previous_board_at_y = at_y;
        let positive = at_y > 0.0;
        *flags_2468 =
            (*flags_2468 & !0x1800) | (u32::from(positive) << 12) | (u32::from(!positive) << 11);
        delta
    }
}
