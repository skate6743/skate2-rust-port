use super::*;

#[test]
fn ground_reckoning_keeps_retained_dynamic_up_separate_from_wheel_support() {
    let mut packet = ProcessedPhysicsInput::default();
    packet.vectors_464_480_496_512_528[0] = [0.0_f32, 1.0, 0.0, 0.0].map(f32::to_bits);
    packet.vectors_464_480_496_512_528[4] = [0.6_f32, 0.8, 0.0, 0.0].map(f32::to_bits);
    packet.scalar_2652 = 7.0;
    packet.scalar_2612 = -3.0;
    packet.scalar_2616 = 5.0;
    packet.wheel_count_2556 = 2;
    let input = GroundPacketInputs::from_processed(&packet);
    assert_eq!(input.wheel_normal, Vector3::new(0.0, 1.0, 0.0));
    assert_eq!(input.dynamic_up, Vector3::new(0.6, 0.8, 0.0));
    assert_eq!(input.speed, 7.0);
    assert_eq!(input.absolute_speed, 5.0);
    assert_eq!(input.wheel_count, 2);

    //A lost-contact packet must retain its published528, not choose UP or
    //the current wheel normal merely because this frame has no support.
    packet.wheel_count_2556 = 0;
    let lost = GroundPacketInputs::from_processed(&packet);
    assert_eq!(lost.dynamic_up, input.dynamic_up);
    assert_eq!(lost.wheel_count, 0);
}
