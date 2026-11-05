use super::*;
#[test]
fn vertical_error_requests_bail_without_planar_restart() {
    let mut state = State::default();
    state.result.velocity_288 = [0., -3., 0., 0.];
    let response = state.collision_response([[0., 0.21, 0., 0.], [0.; 4]], [0., 1., 0., 0.], true);
    assert!(response.request_52);
    assert!(response.restart.is_none());
}
#[test]
fn restart_changes_launch_and_current_sample_without_rewriting_secondary_velocity() {
    let mut state = State::default();
    state.result.velocity_288 = [-3., 1., 0., 0.];
    state.result.position_272 = [2., 3., 4., 0.];
    let response = state.collision_response([[0.02, 0., 0., 0.], [0.; 4]], [0., 1., 0., 0.], true);
    let normal = response.restart.unwrap();
    let mut packet = Packet::initialized(0.);
    packet.secondary_velocity_16 = [5., 6., 7., 0.];
    let result = state.restart_packet(packet, normal, [2., 3., 4., 0.], [0., 1., 0., 0.], -0.8);
    assert_eq!(result.secondary_velocity_16, [5., 6., 7., 0.]);
    for (actual, expected) in result.velocity_0[..3].iter().zip([1., 1., 0.]) {
        assert!((actual - expected).abs() < 1e-5);
    }
    assert!(result.has_board_position_116);
    let retained = state.result.velocity_288;
    state.correct_restarted_sample();
    assert_eq!(state.result.velocity_288, retained);
    assert!((state.result.position_272[0] - (2. + 3. * DT)).abs() < 1e-6);
}
