//! Regressions for the paired source boundary, not original-game fidelity proof.
use super::*;
use crate::physics::board::BODY_COUNT;

fn settings() -> DynamicNormalSettings {
    DynamicNormalSettings {
        speed_damping: 0.0,
        up_vector_damping: 1.0,
        maximum_delta: 10.0,
        speed_scale: 1.0,
        maximum_delta_vs_speed: PointGraph {
            x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
            y: [1.0; 8],
        },
    }
}

#[test]
fn retained_normal_consumes_sampled_acceleration() {
    let mut contacts = BoardGroundState::default();
    contacts.wheel_contact_count = 1;
    contacts.parts[0].in_contact = true;
    contacts.sample_accelerations([Vector3::new(1.0, 0.0, 0.0); BODY_COUNT], 1.0);
    let mut filter = BoardDynamicNormal::new();
    filter.update(&contacts, ZERO, 0.0, &settings());
    assert!(filter.normal.x > 0.9999);
    assert!(filter.normal.y.abs() < 0.0001 && filter.normal.z.abs() < 0.0001);
    let once = filter.normal;
    // There is no live-body/timestep input or second history to re-sample.
    filter.update(&contacts, ZERO, 0.0, &settings());
    assert!((filter.normal.x - once.x).abs() < 0.0001);
}

#[test]
fn body_history_reset_does_not_reset_retained_normal_or_delta() {
    let mut contacts = BoardGroundState::default();
    contacts.sample_accelerations([Vector3::new(5.0, 0.0, 0.0); BODY_COUNT], 1.0);
    let mut filter = BoardDynamicNormal::new();
    filter.normal = Vector3::new(0.6, 0.8, 0.0);
    filter.delta = Vector3::new(0.1, 0.2, 0.3);
    filter.update(&contacts, ZERO, 0.0, &settings());
    let retained = (filter.normal, filter.delta);
    contacts = BoardGroundState::default();
    filter.update(&contacts, ZERO, 0.0, &settings());
    assert_eq!((filter.normal, filter.delta), retained);
    contacts.wheel_contact_count = 1;
    contacts.parts[0].in_contact = true;
    contacts.sample_accelerations([Vector3::new(2.0, 0.0, 0.0); BODY_COUNT], 1.0);
    filter.update(&contacts, ZERO, 0.0, &settings());
    // Reusing the old velocity5 would produce acceleration-3 instead of+2.
    assert!(filter.normal.x > 0.9999);
    assert!(filter.normal.y.abs() < 0.0001 && filter.normal.z.abs() < 0.0001);
}

#[test]
fn fourth_wheel_contact_does_not_enter_native_three_wheel_sum() {
    let mut contacts = BoardGroundState::default();
    contacts.wheel_contact_count = 1;
    contacts.parts[3].in_contact = true;
    contacts.accelerations[3] = Vector3::new(9.0, 0.0, 0.0);
    let mut filter = BoardDynamicNormal::new();
    filter.update(&contacts, ZERO, 0.0, &settings());
    assert!(filter.normal.x.abs() < 0.0001);
}
