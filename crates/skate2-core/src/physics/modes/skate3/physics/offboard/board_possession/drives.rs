//!74FD8 creates both persistent hand drives. AddDrive reverses argument bodies:
//! child/internal A is skeleton part3/7; parent/internal B is deck6.
use super::affine;
use skate3_core::{
    physics::{
        assembly::BodySnapshot,
        drive_frames::{RetailDriveFrame, RetailDriveFrames, retail_quaternion_from_basis},
        drive_parameters::{RetailDriveDynamics, RetailDriveParams, RetailDriveType},
        drive_solver::{RetailDriveBodyState, RetailDriveRows, build_drive_rows},
        rigid_body::pack_world_inverse_inertia,
        skeleton_body::prepare_bone_drive_frames,
    },
    player::offboard::board_possession::State,
};
pub(crate) fn append(
    state: &State,
    deck: BodySnapshot,
    hands: [BodySnapshot; 2],
    deck_reaction: usize,
    hand_reactions: [usize; 2],
    dt: f32,
    rows: &mut Vec<RetailDriveRows>,
) {
    for i in 0..2 {
        if (deck.state_flags | hands[i].state_flags) & 4 == 0 {
            continue;
        }
        let drive = state.hands[i];
        let frame = |v| {
            let v = affine(v);
            RetailDriveFrame {
                orientation: retail_quaternion_from_basis(v.basis),
                translation: v.translation,
            }
        };
        let params = |v: [u32; 4]| RetailDriveParams {
            spring_or_max_velocity: f32::from_bits(v[0]),
            damping: f32::from_bits(v[1]),
            max_strength: f32::from_bits(v[2]),
            drive_type: RetailDriveType::HardDrive,
        };
        rows.push(build_drive_rows(
            body(hands[i], hand_reactions[i]),
            body(deck, deck_reaction),
            //82763BC4..3C44 normalizes both quaternions of every registered
            //drive before Jacobian construction, including possession drives.
            prepare_bone_drive_frames(RetailDriveFrames {
                body_a: frame(drive.child),
                body_b: frame(drive.parent),
            }),
            RetailDriveDynamics {
                linear: params(drive.dynamics[0]),
                angular: params(drive.dynamics[1]),
            },
            dt,
        ));
    }
}
fn body(b: BodySnapshot, reaction_index: usize) -> RetailDriveBodyState {
    let r = b.rates;
    RetailDriveBodyState {
        reaction_index,
        state: b.state_flags,
        orientation: r.orientation,
        basis: r.basis,
        center_of_mass: r.position,
        linear_velocity: r.linear_velocity,
        angular_velocity: r.angular_velocity,
        force_acceleration: r.force_acceleration,
        torque_acceleration: r.torque_acceleration,
        inverse_mass: b.inertia.inverse_mass,
        world_inverse_inertia: pack_world_inverse_inertia(r.world_inverse_inertia),
    }
}
