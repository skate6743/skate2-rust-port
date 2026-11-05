use super::offboard_output;
use skate3_core::player::input_phase::OffBoardOutputFields;

// Host wiring regression, not an independently executed Skate 3 parity test.
#[test]
fn offboard_camera_consumes_the_distinct_completed_output_fields() {
    let raw = |v: [f32; 4]| v.map(f32::to_bits);
    let physical = OffBoardOutputFields {
        scalar_92: 2.5,
        scalar_152: 0.75,
        scalar_156: 1.25,
        vector_160: raw([0., 1., 0., 0.]),
        vector_176: raw([1., 2., 3., 1.]),
        vector_192: raw([0., 0., 1., 0.]),
        vector_208: raw([4., 5., 6., 1.]),
        vector_224: raw([1., 0., 0., 0.]),
        vector_240: raw([7., 8., 9., 1.]),
        trajectory_valid_331: 1,
        flag_334: 1,
        flag_308: 1,
        flag_304: 0,
        ..Default::default()
    };
    let camera = offboard_output(&physical);
    assert_eq!(camera.duration_92, 2.5);
    assert_eq!(camera.time_152, 0.75);
    assert_eq!(camera.apex_time_156, 1.25);
    assert_eq!(camera.launch_normal_160, [0., 1., 0., 0.]);
    assert_eq!(camera.launch_position_176, [1., 2., 3., 1.]);
    assert_eq!(camera.landing_normal_192, [0., 0., 1., 0.]);
    assert_eq!(camera.landing_position_208, [4., 5., 6., 1.]);
    assert_eq!(camera.heading_224, [1., 0., 0., 0.]);
    assert_eq!(camera.apex_240, [7., 8., 9., 1.]);
    assert_eq!(camera.use_trajectory_331, 1);
    assert_eq!(camera.dropping_in_334, 1);
    assert_eq!(camera.object_held_304, 1);
}

#[test]
fn reset_offboard_output_does_not_invent_a_valid_trajectory() {
    let camera = offboard_output(&OffBoardOutputFields::default());
    assert_eq!(camera.use_trajectory_331, 0);
    assert_eq!(camera.dropping_in_334, 0);
    assert_eq!(camera.object_held_304, 0);
}
