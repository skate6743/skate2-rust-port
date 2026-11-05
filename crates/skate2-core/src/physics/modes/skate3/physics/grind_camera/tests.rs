use super::*;
fn output() -> CameraGrindOutput {
    CameraGrindOutput {
        direction_0: [3.0; 4],
        camera_target_96: [7.0; 4],
        grinding_316: 1,
    }
}
fn contact(x: f32, family: u32) -> Contact {
    Contact {
        point: [x, 0.0, 0.0, 0.0],
        direction: [1.0, 0.0, 0.0, 0.0],
        endpoints: [[0.0; 4], [20.0, 0.0, 0.0, 0.0]],
        family,
    }
}
#[test]
fn ordinary_motion_is_not_smoothed_but_family_discontinuity_is() {
    let mut camera = GrindCamera::default();
    let mut out = output();
    camera.update(Some(contact(1.0, 0)), &mut out);
    camera.update(Some(contact(2.0, 0)), &mut out);
    assert_eq!(out.camera_target_96[0], 2.0);
    camera.update(Some(contact(10.0, 1)), &mut out);
    assert!((out.camera_target_96[0] - 3.35).abs() < 0.00001);
    assert_eq!(out.grinding_316, 1);
}
#[test]
fn primitive_change_uses_xyz_midpoint_and_inactive_does_not_erase_output() {
    let mut camera = GrindCamera::default();
    let mut out = output();
    camera.update(Some(contact(2.0, 0)), &mut out);
    let mut next = contact(10.0, 0);
    next.endpoints[1][0] = 22.0;
    camera.update(Some(next), &mut out);
    assert!((out.camera_target_96[0] - 2.4).abs() < 0.00001);
    let retained = out.camera_target_96;
    camera.update(None, &mut out);
    assert_eq!(out.camera_target_96, retained);
    camera.update(Some(contact(25.0, 0)), &mut out);
    assert_eq!(out.camera_target_96[0], 25.0);
}
