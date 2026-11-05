use super::*;

fn input() -> PushInput {
    PushInput {
        flags_2468: 0x0200_0000,
        flags_2472: 0,
        target_speed: 10.0,
        current_speed: 0.0,
        absolute_body_speed: 0.0,
        scale: 0.5,
        delta_seconds: 0.125,
        direction: Vector3::new(0.0, 0.0, 1.0),
    }
}
fn limits() -> PushLimits {
    PushLimits {
        maximum_pushable_speed: 8.5,
        low_speed_change: 2.0,
        high_speed_change: 1.0,
    }
}

#[test]
fn inactive_and_suppressed_push_clear_both_output_vectors() {
    let mut i = input();
    i.flags_2468 = 0;
    i.flags_2472 = 0x200;
    assert_eq!(
        calculate_acceleration(i, limits()),
        PushAcceleration::default()
    );
    i.flags_2468 = 0x0200_0000;
    assert_eq!(
        calculate_acceleration(i, limits()),
        PushAcceleration {
            suppressed: true,
            ..Default::default()
        }
    );
}

#[test]
fn tu3_blends_the_two_speed_limits_before_time_conversion() {
    assert_eq!(calculate_acceleration(input(), limits()).vector.z, 8.0);
    let i = PushInput {
        current_speed: 4.25,
        absolute_body_speed: 4.25,
        ..input()
    };
    assert_eq!(calculate_acceleration(i, limits()).vector.z, 6.0);
}

#[test]
fn speed_cap_is_based_on_absolute_speed_and_never_brakes() {
    let i = PushInput {
        current_speed: 8.25,
        absolute_body_speed: 8.25,
        ..input()
    };
    assert_eq!(calculate_acceleration(i, limits()).vector.z, 1.0);
    let i = PushInput {
        absolute_body_speed: 9.0,
        ..i
    };
    assert_eq!(calculate_acceleration(i, limits()).vector, Vector3::ZERO);
    let i = PushInput {
        target_speed: -1.0,
        ..input()
    };
    assert_eq!(calculate_acceleration(i, limits()).vector, Vector3::ZERO);
}

#[test]
fn native_force_direction_keeps_vertical_component_and_zero_application_point() {
    let i = PushInput {
        direction: Vector3::new(0.0, 0.6, 0.8),
        ..input()
    };
    let result = calculate_acceleration(i, limits());
    assert_eq!(result.vector, Vector3::new(0.0, 4.8, 6.4));
    assert_eq!(result.local_point, Vector3::ZERO);
}
