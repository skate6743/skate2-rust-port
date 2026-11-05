//! Original TU3 SkeletonDrives::InitHooks82BEAB70 creates four static targets.
//! These physical constraint endpoints contain no runtime interception code.
use crate::{
    math::{Basis3, Vector3},
    physics::{
        assembly::BodySnapshot,
        board_pose::{PartPose, orthonormalize_rotation, set_part_transform},
        board_runtime::{BoardMotion, initialized_body},
        drive_frames::{RetailDriveFrame, RetailDriveFrames, retail_quaternion_from_basis},
        drive_parameters::{RetailDriveDynamics, RetailDriveParams, RetailDriveType},
        rigid_body::{RetailInertiaDynamics, RetailQuaternion, RetailSimulationStep},
        skeleton_animation_record::{AnimationPartTransform, IDENTITY},
    },
};

pub const TARGET_COUNT: usize = 4;
pub const TARGET_PARTS: [usize; TARGET_COUNT] = [23, 0, 24, 25];
pub(super) const HARD_VELOCITY: f32 = f32::from_bits(0x4415_FFFF);
pub(super) const HARD_STRENGTH: f32 = f32::from_bits(0x470C_9FFF);

pub struct SkeletonTargets {
    pub bodies: [BodySnapshot; TARGET_COUNT],
    pub frames: [RetailDriveFrames; TARGET_COUNT],
    pub dynamics: [RetailDriveDynamics; TARGET_COUNT],
}
impl SkeletonTargets {
    pub fn new(simulation: RetailSimulationStep) -> Self {
        let bodies = std::array::from_fn(|_| {
            let mut part = PartPose {
                transform: words(IDENTITY),
                local_mass_frame: Some(words(IDENTITY)),
                body: Some([0; 44]),
                inertia: None,
            };
            set_part_transform(&mut part, words(IDENTITY));
            // Original82BEAED4 inserts STATIC_BODY(1), giving zero inverse
            // mass/tensor/rates. The centered.1m box's authored100kg and its
            // disabled collision volume never participate in that response.
            initialized_body(&part, STATIC_INERTIA, simulation, BoardMotion::Static)
        });
        let identity = RetailDriveFrame {
            orientation: RetailQuaternion::IDENTITY,
            translation: Vector3::ZERO,
        };
        //82BEAD34..ADC8: X=(0,1,0),Y=(0,0,1),Z=(1,0,0).
        let extra = RetailDriveFrame {
            orientation: retail_quaternion_from_basis(Basis3 {
                columns: [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]],
            }),
            ..identity
        };
        let frames = std::array::from_fn(|i| RetailDriveFrames {
            body_a: if i >= 2 { extra } else { identity },
            body_b: identity,
        });
        let strong = hard(HARD_VELOCITY, HARD_STRENGTH);
        let root = hard(f32::from_bits(0x426F_FFFF), HARD_STRENGTH);
        Self {
            bodies,
            frames,
            dynamics: [
                RetailDriveDynamics {
                    linear: strong,
                    angular: strong,
                },
                RetailDriveDynamics {
                    linear: root,
                    angular: root,
                },
                RetailDriveDynamics {
                    linear: hard(0.0, 0.0),
                    angular: strong,
                },
                RetailDriveDynamics {
                    linear: hard(0.0, 0.0),
                    angular: strong,
                },
            ],
        }
    }
    /// Reset82BD9DB8/9DE0 normalizes these two desired matrices before
    /// PartSetTransform. The other two target parts retain their pose.
    pub fn reset(&mut self, hips: AnimationPartTransform, spawn: AnimationPartTransform) {
        for (index, frame) in [hips, spawn].into_iter().enumerate() {
            self.set_transform(index, floats(orthonormalize_rotation(words(frame))));
        }
    }
    /// Native PartSetTransform preserves velocity and force state while
    /// changing the body transform. Its local mass frame is identity here.
    pub fn set_transform(&mut self, index: usize, frame: AnimationPartTransform) {
        let body = &mut self.bodies[index];
        let mut part = PartPose {
            transform: words(frame),
            local_mass_frame: Some(words(IDENTITY)),
            body: Some([0; 44]),
            inertia: None,
        };
        set_part_transform(&mut part, words(frame));
        let w = part.body.expect("Target has an actual rigid body");
        let f = |i| f32::from_bits(w[i]);
        body.rates.orientation = RetailQuaternion {
            x: f(0),
            y: f(1),
            z: f(2),
            w: f(3),
        };
        body.rates.position = Vector3::new(f(4), f(5), f(6));
        body.rates.basis = Basis3 {
            columns: std::array::from_fn(|i| [f(16 + i * 4), f(17 + i * 4), f(18 + i * 4)]),
        };
    }
    pub fn transform(&self, index: usize) -> AnimationPartTransform {
        let body = self.bodies[index];
        let mut m = IDENTITY;
        for i in 0..3 {
            m[i][..3].copy_from_slice(&body.rates.basis.columns[i]);
        }
        let p = body.rates.position;
        m[3] = [p.x, p.y, p.z, 0.0];
        m
    }
    /// Ground::PredictFutureDeck82D38630 moves target0, not physical hips.
    pub fn apply_future_deck_displacement(&mut self, displacement: Vector3) {
        let mut frame = self.transform(0);
        frame[3][0] += displacement.x;
        frame[3][1] += displacement.y;
        frame[3][2] += displacement.z;
        self.set_transform(0, frame);
    }
}

const STATIC_INERTIA: RetailInertiaDynamics = RetailInertiaDynamics {
    inverse_tensor: Vector3::ZERO,
    inverse_mass: 0.0,
    spherical: 0.0,
    maximum_linear_velocity: 0.0,
    maximum_angular_velocity: 0.0,
    linear_drag: 0.0,
    angular_drag: 0.0,
};
pub(super) fn hard(velocity: f32, strength: f32) -> RetailDriveParams {
    RetailDriveParams {
        spring_or_max_velocity: velocity,
        damping: 0.0,
        max_strength: strength,
        drive_type: RetailDriveType::HardDrive,
    }
}
fn words(frame: AnimationPartTransform) -> [u32; 16] {
    std::array::from_fn(|i| frame[i / 4][i % 4].to_bits())
}
fn floats(frame: [u32; 16]) -> AnimationPartTransform {
    std::array::from_fn(|i| std::array::from_fn(|j| f32::from_bits(frame[i * 4 + j])))
}
