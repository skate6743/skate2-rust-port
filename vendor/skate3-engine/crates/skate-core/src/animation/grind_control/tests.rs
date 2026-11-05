use super::*;

fn settings() -> FadeSettings {
    FadeSettings {
        response: 1.0,
        input_scale: 1.0,
        acceleration: 0.25,
        maximum_step: 0.5,
    }
}
#[test]
fn fade_skips_first_update_then_accelerates_and_reverses_without_dt_scaling() {
    let mut a = Fade::default();
    assert_eq!(a.begin(-1.0, 1.0, 0.0), 0.0);
    assert_eq!(a.update(10.0, 1.0, settings()), None);
    assert_eq!(a.elapsed, 0.0);
    let mut b = a;
    assert_eq!(a.update(0.01, 1.0, settings()), Some(0.25));
    assert_eq!(b.update(0.5, 1.0, settings()), Some(0.25));
    assert_ne!(a.elapsed, b.elapsed);
    assert_eq!(a.update(0.01, 1.0, settings()), Some(0.75));
    assert_eq!(a.update(0.01, -1.0, settings()), Some(1.0));
    assert_eq!(a.step, 0.25, "reversal must decelerate the retained step");
    assert_eq!(a.update(0.01, -1.0, settings()), Some(1.0));
    assert_eq!(a.update(0.01, -1.0, settings()), Some(0.75));
}
#[test]
fn fade_target_accumulates_and_begin_discards_previous_run() {
    let mut a = Fade::default();
    a.begin(-2.0, 2.0, 9.0);
    assert_eq!((a.value, a.target), (2.0, 2.0));
    a.update(1.0, 0.0, settings());
    a.update(1.0, -0.25, settings());
    assert_eq!(a.target, 1.75);
    a.update(1.0, -0.25, settings());
    assert_eq!(a.target, 1.5);
    a.begin(-1.0, 1.0, -9.0);
    assert_eq!(
        (a.value, a.target, a.step, a.elapsed),
        (-1.0, -1.0, 0.0, 0.0)
    );
    assert!(a.just_began);
}
#[test]
fn crouch_obeys_physical_minimum_and_two_sided_rate_limit() {
    assert_eq!(crouch(1.0, 0.0, 0.75, 0.25, 1.0, 2.0, 0.125), 0.75);
    assert_eq!(crouch(0.75, 0.0, 0.75, 0.25, 1.0, 2.0, 0.125), 0.5);
    assert_eq!(crouch(0.25, 0.0, 0.0, 0.25, 1.0, 2.0, 0.125), 0.5);
    assert_eq!(crouch(1.0, 0.5, 0.0, 0.25, 1.0, 10.0, 1.0), 0.5);
}
#[test]
fn physical_twist_is_mirrored_exactly_once() {
    assert_eq!(mirrored_twist(0.25, false), 0.25);
    assert_eq!(mirrored_twist(0.25, true), std::f32::consts::PI - 0.25);
    assert_eq!(mirrored_twist(-0.25, true), -std::f32::consts::PI + 0.25);
}
#[test]
fn facing_uses_cross_y_and_both_flags_including_zero_boundary() {
    for ground in [false, true] {
        for mirror in [false, true] {
            assert_eq!(
                facing::backwards([1.0, 8.0, 0.0], [0.0, 3.0, -1.0], ground, mirror),
                true ^ ground ^ mirror
            );
            assert_eq!(
                facing::backwards([1.0, 8.0, 0.0], [0.0, 3.0, 1.0], ground, mirror),
                ground ^ mirror
            );
            assert_eq!(
                facing::backwards([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], ground, mirror),
                ground ^ mirror
            );
        }
    }
    // The other cross-product term is not interchangeable with a board Z test.
    assert!(facing::backwards(
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        false,
        false
    ));
}
