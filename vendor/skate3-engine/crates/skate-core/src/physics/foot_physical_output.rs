//! Full original SkeletonIK::FillPhysOut_FeetDeckInteraction82BF22A0.
//! Physical toe positions and derivatives are observed in the physical board's
//! frame. They are distinct from animation targets and contact-query validity.
use super::{
    native_arithmetic::reciprocal_estimate,
    skeleton_animation_record::AnimationPartTransform as Transform,
    skeleton_body::SkeletonPhysicalRecord,
};

#[derive(Clone, Copy, Debug)]
pub struct FootPhysicalSettings {
    pub deck_half_width: f32,
    pub deck_total_half_length: f32,
    /// Original physics_skeletonik layout64 FootOnDeckPadding.
    pub padding: [f32; 4],
}
#[derive(Clone, Debug, Default)]
pub struct FootPhysicalState {
    /// IK3136/3152. Reset82BEDB5C/60 writes both vectors to zero.
    pub previous_local_toes: [[f32; 4]; 2],
}
#[derive(Clone, Copy, Debug, Default)]
pub struct FootPhysicalOutput {
    /// PhysOutSkeleton192/208: change in local physical toe positions / dt.
    pub local_velocity: [[f32; 4]; 2],
    /// PhysOutSkeleton224/240: physical record velocities15/19, copied intact.
    pub world_velocity: [[f32; 4]; 2],
    /// PhysOutSkeleton600/601. Strict XYZ containment, with authored padding.
    pub within_deck_box: [bool; 2],
}
impl FootPhysicalState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn update(
        &mut self,
        record: &SkeletonPhysicalRecord,
        dt: f32,
        settings: FootPhysicalSettings,
    ) -> FootPhysicalOutput {
        let inverse = inverse_physical_board(&record.pose[0]);
        let positions = [15usize, 19].map(|part| transform_point(&inverse, record.pose[part][3]));
        let mut inverse_dt = reciprocal_estimate(dt);
        for _ in 0..2 {
            inverse_dt = inverse_dt.mul_add((-inverse_dt).mul_add(dt, 1.0), inverse_dt);
        }
        let local_velocity = core::array::from_fn(|foot| {
            core::array::from_fn(|lane| {
                (positions[foot][lane] - self.previous_local_toes[foot][lane]) * inverse_dt
            })
        });
        self.previous_local_toes = positions;
        //82165A10 is zero: only authored padding supplies the box's Y extent.
        let dimensions = [
            settings.deck_half_width,
            0.0,
            settings.deck_total_half_length,
            0.0,
        ];
        let bounds: [f32; 4] =
            core::array::from_fn(|lane| dimensions[lane] + settings.padding[lane]);
        FootPhysicalOutput {
            local_velocity,
            world_velocity: [record.velocities[15], record.velocities[19]],
            within_deck_box: positions.map(|p| {
                p[0].abs() < bounds[0] && p[1].abs() < bounds[1] && p[2].abs() < bounds[2]
            }),
        }
    }
}

fn inverse_physical_board(board: &Transform) -> Transform {
    let mut inverse: Transform = [[0.0; 4]; 4];
    for axis in 0..3 {
        for lane in 0..3 {
            inverse[axis][lane] = board[lane][axis];
        }
    }
    //82BF2398..241C builds -P.z * inverseZ, then Y and X in source order.
    for lane in 0..4 {
        let z = -board[3][2] * inverse[2][lane];
        let yz = (-board[3][1]).mul_add(inverse[1][lane], z);
        inverse[3][lane] = (-board[3][0]).mul_add(inverse[0][lane], yz);
    }
    inverse
}
fn transform_point(frame: &Transform, p: [f32; 4]) -> [f32; 4] {
    core::array::from_fn(|lane| {
        let x = frame[0][lane].mul_add(p[0], frame[3][lane]);
        let xy = frame[1][lane].mul_add(p[1], x);
        frame[2][lane].mul_add(p[2], xy)
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moving_board_frame_separates_world_rates_from_relative_foot_motion() {
        let mut record = SkeletonPhysicalRecord::default();
        record.pose[0][3] = [10.0, 2.0, 3.0, 0.0];
        record.pose[15][3] = [10.2, 2.0, 3.3, 0.0];
        record.pose[19][3] = [9.8, 2.0, 2.7, 0.0];
        record.velocities[15] = [4.0, 0.0, 0.0, 0.0];
        record.velocities[19] = [5.0, 0.0, 0.0, 0.0];
        let settings = FootPhysicalSettings {
            deck_half_width: 0.3,
            deck_total_half_length: 0.5,
            padding: [0.0, 0.1, 0.0, 0.0],
        };
        let mut state = FootPhysicalState::default();
        let initial = state.update(&record, 0.25, settings);
        assert_eq!(initial.within_deck_box, [true, true]);
        assert_eq!(
            initial.world_velocity,
            [record.velocities[15], record.velocities[19]]
        );
        for part in [0, 15, 19] {
            record.pose[part][3][0] += 1.0;
        }
        let moved = state.update(&record, 0.25, settings);
        assert!(
            moved
                .local_velocity
                .iter()
                .flatten()
                .all(|value| value.abs() < 0.00001)
        );
        let mut boundaries = SkeletonPhysicalRecord::default();
        boundaries.pose[15][3] = [0.3, 0.0, 0.0, 0.0];
        boundaries.pose[19][3] = [0.0, 0.1, 0.0, 0.0];
        assert_eq!(
            state.update(&boundaries, 0.25, settings).within_deck_box,
            [false, false]
        );
    }
}
