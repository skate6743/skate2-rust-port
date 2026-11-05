//! Native SkateboardController drives82D74FD8 /82D751D8 in the shared island.
//! Factory82AE64F8 stores argument5 at drive16 (A), argument4 at20 (B):
//! hand bodies3/7 are A and the deck is B, despite the factory argument order.
use skate3_core::{
    math::{Basis3, Vector3},
    physics::{
        assembly::BodySnapshot,
        board::BodyId,
        board_step::ATTACHED_REACTION_BASE,
        drive_frames::{RetailDriveFrame, RetailDriveFrames, retail_quaternion_from_basis},
        drive_parameters::{RetailDriveDynamics, RetailDriveParams, RetailDriveType},
        drive_solver::{RetailDriveBodyState, RetailDriveRows, build_drive_rows},
        rigid_body::pack_world_inverse_inertia,
        skeleton_body::prepare_bone_drive_frames,
    },
    player::offboard::board_possession::{Frame, State},
};

pub(crate) fn append(
    rows: &mut Vec<RetailDriveRows>,
    state: &State,
    board: &[BodySnapshot; 7],
    skeleton: &[BodySnapshot; 26],
    dt: f32,
) {
    for (hand, part) in [3, 7].into_iter().enumerate() {
        let drive = state.hands[hand];
        let a = skeleton[part];
        let b = board[BodyId::Deck.index()];
        if (a.state_flags | b.state_flags) & 4 == 0 {
            continue;
        }
        let parameters = |words: [u32; 4]| RetailDriveParams {
            spring_or_max_velocity: f32::from_bits(words[0]),
            damping: f32::from_bits(words[1]),
            max_strength: f32::from_bits(words[2]),
            drive_type: match words[3] {
                1 => RetailDriveType::SoftDrive,
                2 => RetailDriveType::HardDrive,
                _ => RetailDriveType::NoDrive,
            },
        };
        rows.push(build_drive_rows(
            body(a, ATTACHED_REACTION_BASE + part),
            body(b, BodyId::Deck.index()),
            prepare_bone_drive_frames(RetailDriveFrames {
                body_a: frame(drive.child),
                body_b: frame(drive.parent),
            }),
            RetailDriveDynamics {
                linear: parameters(drive.dynamics[0]),
                angular: parameters(drive.dynamics[1]),
            },
            dt,
        ));
    }
}
fn frame(f: Frame) -> RetailDriveFrame {
    RetailDriveFrame {
        orientation: retail_quaternion_from_basis(Basis3 {
            columns: std::array::from_fn(|i| [f[i][0], f[i][1], f[i][2]]),
        }),
        translation: Vector3::new(f[3][0], f[3][1], f[3][2]),
    }
}
fn body(b: BodySnapshot, index: usize) -> RetailDriveBodyState {
    let r = b.rates;
    RetailDriveBodyState {
        reaction_index: index,
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

#[cfg(any())]
mod tests {
    use super::*;
    use skate3_core::physics::{
        drive_solver::solve_drive_rows,
        rigid_body::{
            RetailBodyRates, RetailInertiaDynamics, RetailQuaternion, RetailReactionCorrections,
        },
    };
    fn active() -> BodySnapshot {
        let basis = Basis3 {
            columns: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        };
        BodySnapshot {
            state_flags: 4,
            rates: RetailBodyRates {
                orientation: RetailQuaternion::IDENTITY,
                basis,
                world_inverse_inertia: basis,
                position: Vector3::ZERO,
                linear_velocity: Vector3::ZERO,
                angular_velocity: Vector3::ZERO,
                force_acceleration: Vector3::ZERO,
                torque_acceleration: Vector3::ZERO,
                kinetic_energy: 0.,
                cool_down: 0,
            },
            inertia: RetailInertiaDynamics {
                inverse_tensor: Vector3::new(1., 1., 1.),
                inverse_mass: 1.,
                spherical: 0.,
                maximum_linear_velocity: 1000.,
                maximum_angular_velocity: 1000.,
                linear_drag: 0.,
                angular_drag: 0.,
            },
        }
    }
    #[test]
    fn held_hand_and_deck_exchange_reactions_and_release_removes_force() {
        let board = [active(); 7];
        let mut skeleton = [active(); 26];
        skeleton[3].rates.position.x = 1.;
        let mut state = State::default();
        state.selected_hand_424 = 0;
        state.hands[0].dynamics = [[0x4395ffff, 0, 0x468c9fff, 2]; 2];
        let mut rows = Vec::new();
        append(&mut rows, &state, &board, &skeleton, 1. / 60.);
        let mut reactions = [RetailReactionCorrections::default(); 34];
        solve_drive_rows(&mut rows, &mut reactions, 4);
        let hand = reactions[ATTACHED_REACTION_BASE + 3].linear_displacement.x;
        let deck = reactions[BodyId::Deck.index()].linear_displacement.x;
        assert!(hand < 0. && deck > 0., "hand={hand}, deck={deck}");
        assert!((hand + deck).abs() < 1e-6);
        assert_eq!(
            reactions[ATTACHED_REACTION_BASE + 7].linear_displacement,
            Vector3::ZERO
        );
        state.disable_hand();
        rows.clear();
        append(&mut rows, &state, &board, &skeleton, 1. / 60.);
        reactions.fill(RetailReactionCorrections::default());
        solve_drive_rows(&mut rows, &mut reactions, 4);
        assert!(
            reactions
                .iter()
                .all(|r| r.linear_displacement == Vector3::ZERO
                    && r.angular_displacement == Vector3::ZERO)
        );
    }
}
