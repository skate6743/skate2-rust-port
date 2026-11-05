use super::*;
fn surface() -> SurfaceInput {
    SurfaceInput {
        flags: 1,
        normal: UP,
        origin: [0.0; 4],
        edge_point: [0.0, 0.0, 1.0, 0.0],
        edge_normal: UP,
    }
}
fn spring() -> SpringInput {
    SpringInput {
        velocity: [0.0; 4],
        up: UP,
        right: [1.0, 0.0, 0.0, 0.0],
        forward: [0.0, 0.0, 1.0, 0.0],
        spring_right: [1.0, 0.0, 0.0, 0.0],
        spring_forward: [0.0, 0.0, 1.0, 0.0],
        right_delta: 0.0,
        forward_delta: 0.0,
        suppress_lean: false,
    }
}
#[test]
fn missing_contact_preserves_both_normals() {
    let mut state = State::default();
    let before = state.clone();
    let mut input = surface();
    input.flags = 0;
    input.normal = [1.0, 0.0, 0.0, 7.0];
    state.update_surface(&input);
    assert_eq!(state, before);
}
#[test]
fn large_normal_jump_updates_source_but_rejects_surface() {
    let mut state = State::default();
    let mut input = surface();
    input.normal = [1.0, 0.0, 0.0, 2.0];
    state.update_surface(&input);
    assert_eq!(state.source_normal, input.normal);
    assert_eq!(state.surface_normal, UP);
}
#[test]
fn accepted_normal_preserves_w_through_normalization() {
    let mut state = State::default();
    let mut input = surface();
    input.normal = [0.0, 1.0, 0.0, 2.0];
    state.update_surface(&input);
    assert_eq!(state.surface_normal, [0.0, 1.0, 0.0, 1.0]);
}
#[test]
fn edge_degeneracy_and_angle_antiparallel_return_original() {
    let mut state = State::default();
    let mut input = surface();
    input.flags = 5;
    input.edge_point = input.origin;
    state.update_surface(&input);
    assert_eq!(state.surface_normal, UP);
    let target = [0.0, -1.0, 0.0, 3.0];
    assert_eq!(clamp_angle(target, UP, 0.5), target);
}
#[test]
fn lean_is_separate_from_spring_and_suppression_decays_it() {
    let mut state = State::default();
    let mut input = spring();
    input.right_delta = 1.0;
    state.update_spring(&input);
    assert_eq!(state.spring_normal, UP);
    assert_eq!(state.spring_delta, [0.0; 4]);
    assert!((state.lean[0] - 0.035).abs() < 0.000001);
    assert!(state.final_up[0] > 0.0);
    let prior = state.lean;
    input.suppress_lean = true;
    state.update_spring(&input);
    assert_eq!(state.lean, scale(prior, f32::from_bits(0x3f73_3333)));
}
#[test]
fn angle_limit_rotates_reference_toward_target_and_retains_target_magnitude() {
    let limited = clamp_angle([2.0, 0.0, 0.0, 0.0], UP, f32::from_bits(0x3f49_0fdb));
    assert!((length(limited) - 2.0).abs() < 0.00001);
    assert!((limited[0] - limited[1]).abs() < 0.00001);
    assert!(limited[0] > 0.0);
}
