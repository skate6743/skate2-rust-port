//! BoneDrives::SetDriveFrames82BD6FD0. A common animation-space frame is
//! expressed in the child and parent coordinates before quaternion packing.
use crate::{
    math::{Basis3, Vector3},
    physics::{
        drive_frames::{RetailDriveFrame, RetailDriveFrames, retail_quaternion_from_basis},
        drive_preparation::normalize_drive_frames,
        rigid_body::RetailQuaternion,
        skeleton_animation_record::{AnimationPartTransform, IDENTITY, compose_affine},
    },
};

pub fn bone_drive_frames(
    child_animation: &AnimationPartTransform,
    inverse_child: &AnimationPartTransform,
    inverse_parent: &AnimationPartTransform,
) -> RetailDriveFrames {
    let mut common = IDENTITY;
    common[3] = child_animation[3];
    let child = compose_affine(inverse_child, &common);
    let parent = compose_affine(inverse_parent, &common);
    RetailDriveFrames {
        body_a: pack(child),
        body_b: pack(parent),
    }
}

/// Step_Solver2 normalizes each active pair before building the drive rows.
pub fn prepare_bone_drive_frames(frames: RetailDriveFrames) -> RetailDriveFrames {
    let mut words = [0; 16];
    for (i, frame) in [frames.body_a, frames.body_b].into_iter().enumerate() {
        let q = frame.orientation;
        let p = frame.translation;
        words[i * 8..i * 8 + 8]
            .copy_from_slice(&[q.x, q.y, q.z, q.w, p.x, p.y, p.z, 0.0].map(f32::to_bits));
    }
    normalize_drive_frames(&mut words);
    let frame = |offset: usize| {
        let f = |i: usize| f32::from_bits(words[offset + i]);
        RetailDriveFrame {
            orientation: RetailQuaternion {
                x: f(0),
                y: f(1),
                z: f(2),
                w: f(3),
            },
            translation: Vector3::new(f(4), f(5), f(6)),
        }
    };
    RetailDriveFrames {
        body_a: frame(0),
        body_b: frame(8),
    }
}
fn pack(frame: AnimationPartTransform) -> RetailDriveFrame {
    RetailDriveFrame {
        orientation: retail_quaternion_from_basis(Basis3 {
            columns: std::array::from_fn(|i| [frame[i][0], frame[i][1], frame[i][2]]),
        }),
        translation: Vector3::new(frame[3][0], frame[3][1], frame[3][2]),
    }
}
