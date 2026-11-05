//! Wipeout300 Fill82D3EEE8 writes the shared completed physical packet.
use super::super::Skate3ControllerState;

pub(super) fn publish(skater: &mut Skate3ControllerState) {
    let output = super::super::wipeout_states::fill(skater);
    let physical = &mut skater.player_input.physical;
    let skeleton = &mut physical.skeleton;
    skeleton.over_599 = u8::from(output.over_599);
    skeleton.scalar_544 = output.scalar_544;
    skeleton.no_support_time_548 = output.no_support_time_548;
    skeleton.response_strength_580 = output.response_strength_580;
    skeleton.extra_weight_584 = output.extra_weight_584;
    skeleton.time_until_teleport_576 = output.time_until_teleport_576;
    skeleton.teleport_pending_604 = u8::from(output.teleport_pending_604);
    skeleton.hips_right_angle_496 = output.hips_right_angle_496;
    skeleton.hips_up_angle_500 = output.hips_up_angle_500;
    if let Some(value) = output.response_change_588 {
        skeleton.response_change_588 = value;
        skeleton.response_changed_605 = 1;
    }
    physical.animation.collision_time_144 = output.collision_time_144;
    physical.animation.profile_148 = output.profile_148;
    physical.collision.flag_3479 = u8::from(output.material_ten_3479);
    physical.collision.flag_3480 = u8::from(output.material_eleven_3480);
    physical.collision.flag_3483 = u8::from(output.imminent_surface_twelve_3483);
    physical.collision.predicted_position_64 = output.predicted_position_64.map(f32::to_bits);
    if let Some(velocity) = output.retained_air_velocity_160 {
        physical.air.vector_160 = velocity.map(f32::to_bits);
        physical.air.use_air_reckoning_452 = 1;
    }
    physical.state.surface_height_32 = output.surface_height_32;
    let flags = &mut skater.player_state.state_flags;
    for (offset, value) in [
        (72, output.can_leave_72),
        (81, output.special_surface_81),
        (82, output.below_surface_82),
        (83, output.surface_height_valid_83),
    ] {
        flags[offset - 52] = value;
    }
    // These source branches only store true, preserving earlier publication.
    if output.teleport_countdown_68 {
        flags[68 - 52] = true;
    }
    if output.request_teleport_69 {
        flags[69 - 52] = true;
        physical.state.flag_69 = 1;
    }
}
