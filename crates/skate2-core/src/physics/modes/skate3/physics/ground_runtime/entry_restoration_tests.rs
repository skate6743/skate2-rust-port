//! Independent field-ownership regression, not a stock gameplay/parity fixture.
use super::restore_standard_board_fields;
use skate3_core::{
    math::Vector3,
    physics::{
        board_runtime::{BoardMotion, BoardRuntime},
        drive_frames::RetailAffineTransform,
        rigid_body::{
            RetailBodyMassProperties, RetailInertiaDynamics, RetailLocalMassFrame,
            RetailSimulationStep,
        },
    },
};

#[test]
fn ground_reentry_restores_collision_group_without_reactivating_bodies() {
    let mass = RetailBodyMassProperties {
        local_mass_frame: RetailLocalMassFrame::IDENTITY,
        dynamics: RetailInertiaDynamics {
            inverse_tensor: Vector3::new(1.0, 2.0, 3.0),
            inverse_mass: 1.0,
            spherical: 0.0,
            maximum_linear_velocity: 100.0,
            maximum_angular_velocity: 100.0,
            linear_drag: 0.25,
            angular_drag: 0.5,
        },
    };
    let mut board = BoardRuntime::new(
        [mass; 7],
        [RetailAffineTransform::IDENTITY; 7],
        RetailAffineTransform::IDENTITY,
        RetailSimulationStep::fixed_60_hz(30, 0.001, Vector3::ZERO),
        BoardMotion::Active,
    );
    board.set_collision_group(7);
    for (i, body) in board.bodies_mut().iter_mut().enumerate() {
        body.state_flags = [4, 12, 2, 10, 1, 9, 4][i];
        body.rates.cool_down = 10 + i as u32;
        body.rates.linear_velocity = Vector3::new(i as f32, 3.0, 2.0);
        body.rates.angular_velocity = Vector3::new(1.0, i as f32, 2.0);
    }
    let before = *board.bodies();
    let target_before = board.hook().body;
    let mut flags = 0xd5;
    let supplied_stock_drag = 2.75;

    restore_standard_board_fields(&mut board, &mut flags, supplied_stock_drag);

    assert_eq!(board.collision_group(), 4);
    assert_eq!(flags, 0x55);
    for (i, (body, old)) in board.bodies().iter().zip(before).enumerate() {
        assert_eq!(body.state_flags, old.state_flags);
        assert_eq!(body.rates, old.rates);
        let mut expected_inertia = old.inertia;
        if i == 6 {
            expected_inertia.angular_drag = supplied_stock_drag;
        }
        assert_eq!(body.inertia, expected_inertia);
    }
    assert_eq!(board.hook().body.state_flags, target_before.state_flags);
    assert_eq!(board.hook().body.rates, target_before.rates);
    assert_eq!(board.hook().body.inertia, target_before.inertia);
}
