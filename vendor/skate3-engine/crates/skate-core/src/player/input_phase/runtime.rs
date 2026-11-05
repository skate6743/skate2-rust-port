use super::publication::{
    effective_animation_transform, publish_animation_packet, publish_collision_prefix,
    publish_external_physics, publish_force_braking, publish_line_tests, publish_physical_outputs,
    publish_player_flags, publish_state_prefix, replace_bool, replace_byte, select_surface,
    transfer_bit,
};
use super::requests::{GroundHistoryRequest, PrepareJumpRequest};
use super::types::{
    AnimationInputPacket, PhysicalPlayerInput, PlayerInputState, ProcessedPhysicsInput, RawVector,
};

const FIXED_FRAME_STEP: f32 = f32::from_bits(0x3c88_8889);

/// Mandatory boundaries called by TU3 `PhysicalPlayerHiLOD::Input`.
///
/// Inlined motion calculations execute directly in this module. Services own
/// only separate physical subsystems and host data access.
pub trait InputPhaseServices {
    type Error;

    fn update_pre_input_manager_82d81610(
        &mut self,
        player: &mut PlayerInputState,
        physical: &mut PhysicalPlayerInput,
    ) -> Result<(), Self::Error>;
    fn reset_processed_input_82bf9ef0(
        &mut self,
        output: &mut ProcessedPhysicsInput,
    ) -> Result<(), Self::Error>;
    fn actor_query_slot_56(&mut self) -> Result<u32, Self::Error>;
    fn actor_query_slot_44(&mut self) -> Result<u32, Self::Error>;
    fn reset_player_probe_82d7a330(
        &mut self,
        player: &mut PlayerInputState,
        physical: &mut PhysicalPlayerInput,
    ) -> Result<(), Self::Error>;
    fn check_teleport_82db88c8(
        &mut self,
        player: &mut PlayerInputState,
        physical: &mut PhysicalPlayerInput,
        output: &mut ProcessedPhysicsInput,
    ) -> Result<(), Self::Error>;
    fn actor_input_available_slot_4(&mut self) -> Result<bool, Self::Error>;
    fn transition_action_825903c8(&mut self) -> Result<f32, Self::Error>;
    fn calculate_ground_position_82c02840(
        &mut self,
        physical: &PhysicalPlayerInput,
    ) -> Result<RawVector, Self::Error>;
    fn prepare_board_toolkit_82c013f0(
        &mut self,
        player: &mut PlayerInputState,
        physical: &mut PhysicalPlayerInput,
        output: &mut ProcessedPhysicsInput,
    ) -> Result<(), Self::Error>;
    fn process_skeleton_82bd8918(
        &mut self,
        packet: &AnimationInputPacket<'_>,
        physical: &mut PhysicalPlayerInput,
        output: &mut ProcessedPhysicsInput,
    ) -> Result<(), Self::Error>;
    fn update_grind_manager_82d8a828(
        &mut self,
        player: &mut PlayerInputState,
        physical: &mut PhysicalPlayerInput,
        output: &mut ProcessedPhysicsInput,
    ) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputPhaseError<E> {
    Service(E),
    /// Native dereferences a null +2548 pointer when this packet value is >4.
    InvalidStateVariant(u32),
}

/// Pre-reset observations retained by82DB4048 across CheckTeleport.
#[derive(Debug)]
pub struct InputContinuation {
    captured_state: u32,
    captured_category: u32,
}

/// Complete recovered TU3 PhysicalPlayerHiLOD::Input82DB4048.
/// The split also lets a host release subsystem borrows at CheckTeleport.
pub fn process_input<S: InputPhaseServices>(
    player: &mut PlayerInputState,
    physical: &mut PhysicalPlayerInput,
    packet: &AnimationInputPacket<'_>,
    output: &mut ProcessedPhysicsInput,
    services: &mut S,
) -> Result<(), InputPhaseError<S::Error>> {
    let continuation = start_input(player, physical, packet, output, services)?;
    finish_input(continuation, player, physical, packet, output, services)
}

///82DB4048 through CheckTeleport82DB42DC. Complete the physical reset
///continuation before calling finish_input; captured fields must survive it.
pub fn start_input<S: InputPhaseServices>(
    player: &mut PlayerInputState,
    physical: &mut PhysicalPlayerInput,
    packet: &AnimationInputPacket<'_>,
    output: &mut ProcessedPhysicsInput,
    services: &mut S,
) -> Result<InputContinuation, InputPhaseError<S::Error>> {
    let captured_state = physical.state.state_16;
    player.manager_1856_counter_320 = native_nonnegative_decrement(player.manager_1856_counter_320);
    services
        .update_pre_input_manager_82d81610(player, physical)
        .map_err(InputPhaseError::Service)?;
    player.manager_1852_flag_256 = 0;
    let captured_category = physical.state.category_12;
    services
        .reset_processed_input_82bf9ef0(output)
        .map_err(InputPhaseError::Service)?;

    publish_player_flags(player, output);
    output.actor_query_2948 = services
        .actor_query_slot_56()
        .map_err(InputPhaseError::Service)?;
    output.actor_query_2952 = services
        .actor_query_slot_44()
        .map_err(InputPhaseError::Service)?;
    publish_force_braking(player, physical, packet, output);
    output.probe_1792 = player.probe;
    services
        .reset_player_probe_82d7a330(player, physical)
        .map_err(InputPhaseError::Service)?;
    publish_collision_prefix(player, physical, output);
    publish_animation_packet(packet, output);

    output.state_identifier_2496 = physical.state.identifier_8;
    services
        .check_teleport_82db88c8(player, physical, output)
        .map_err(InputPhaseError::Service)?;
    Ok(InputContinuation {
        captured_state,
        captured_category,
    })
}

///82DB42F0 through82DB5570, after any reset/normal-setup continuation.
pub fn finish_input<S: InputPhaseServices>(
    continuation: InputContinuation,
    player: &mut PlayerInputState,
    physical: &mut PhysicalPlayerInput,
    packet: &AnimationInputPacket<'_>,
    output: &mut ProcessedPhysicsInput,
    services: &mut S,
) -> Result<(), InputPhaseError<S::Error>> {
    let InputContinuation {
        captured_state,
        captured_category,
    } = continuation;
    transfer_bit(&mut output.flags_2472, 10, player.flags_1296, 29);
    publish_transition(packet, output, services)?;
    update_signed_ground_time(player, physical, output);
    update_time_on_ground(player, captured_category, output);

    select_surface(player, packet, physical.surface_default_mode, output)
        .map_err(InputPhaseError::InvalidStateVariant)?;
    replace_byte(&mut output.flags_2476, 2, packet.flag_10370);
    publish_external_physics(player, packet, output);
    publish_state_prefix(player, physical, packet, captured_state, output);
    publish_physical_outputs(physical, packet, output);
    update_timers(player, physical, output);
    update_ground_position(player, physical, captured_category, output, services)?;

    output.effective_anim_transform_192 =
        effective_animation_transform(physical.skeleton.anim_to_world_11920, output.flags_2476);
    update_ground_history(player, captured_category, output);
    publish_ground_history(player, output);
    services
        .prepare_board_toolkit_82c013f0(player, physical, output)
        .map_err(InputPhaseError::Service)?;
    publish_line_tests(player, output);
    services
        .process_skeleton_82bd8918(packet, physical, output)
        .map_err(InputPhaseError::Service)?;
    publish_post_skeleton(player, physical, output);
    services
        .update_grind_manager_82d8a828(player, physical, output)
        .map_err(InputPhaseError::Service)
}

fn publish_transition<S: InputPhaseServices>(
    packet: &AnimationInputPacket<'_>,
    output: &mut ProcessedPhysicsInput,
    services: &mut S,
) -> Result<(), InputPhaseError<S::Error>> {
    if !services
        .actor_input_available_slot_4()
        .map_err(InputPhaseError::Service)?
    {
        return Ok(());
    }
    output.transition_2636 = if packet.suppress_transition_10376 != 0 {
        0.0
    } else {
        let value = services
            .transition_action_825903c8()
            .map_err(InputPhaseError::Service)?;
        let lower = xenon_fsel(-1.0 - value, -1.0, value);
        xenon_fsel(1.0 - lower, lower, 1.0)
    };
    Ok(())
}

fn update_signed_ground_time(
    player: &mut PlayerInputState,
    physical: &PhysicalPlayerInput,
    output: &mut ProcessedPhysicsInput,
) {
    let previous = player.signed_ground_time_1356;
    player.signed_ground_time_1356 = if physical.state.signed_ground_step_84 != 0 {
        xenon_fsel(previous, previous, 0.0) + FIXED_FRAME_STEP
    } else {
        xenon_fsel(previous, 0.0, previous) - FIXED_FRAME_STEP
    };
    output.signed_ground_time_2756 = player.signed_ground_time_1356;
}

fn update_time_on_ground(
    player: &mut PlayerInputState,
    captured_category: u32,
    output: &mut ProcessedPhysicsInput,
) {
    if captured_category == 100 {
        output.flags_2472 |= 1 << 13;
        player.grounded_frames_1300 = player.grounded_frames_1300.wrapping_add(1);
        player.time_on_ground_1352 += output.timestep_2604;
        replace_bool(
            &mut player.flags_1296,
            31,
            (player.grounded_frames_1300 as i32) > 6,
        );
    } else {
        player.time_on_ground_1352 = 0.0;
        player.grounded_frames_1300 = 0;
    }
    output.time_on_ground_2752 = player.time_on_ground_1352;
}

fn update_timers(
    player: &mut PlayerInputState,
    physical: &PhysicalPlayerInput,
    output: &mut ProcessedPhysicsInput,
) {
    if physical.state.state_16 == 104 {
        player.skitch_timer_1376 = 2.0;
        player.skitch_value_1372 = physical.state.skitch_value_40;
    }
    if player.skitch_timer_1376 > 0.0 {
        player.skitch_timer_1376 -= output.timestep_2604;
        output.skitch_value_2592 = player.skitch_value_1372;
    }
    if physical.ground.scalar_292 > 0.0 {
        player.ground_timer_1380 = physical.ground.scalar_292;
    }
    if player.ground_timer_1380 > 0.0 {
        player.ground_timer_1380 -= output.timestep_2604;
        output.ground_timer_2848 = player.ground_timer_1380;
        output.skitch_value_2592 = player.skitch_value_1372;
    }
    if physical.ground.scalar_296 > 0.0 {
        player.secondary_ground_timer_1384 = physical.ground.scalar_296;
    }
    if player.secondary_ground_timer_1384 > 0.0 {
        player.secondary_ground_timer_1384 -= output.timestep_2604;
        output.secondary_ground_timer_2852 = player.secondary_ground_timer_1384;
    }
}

fn update_ground_position<S: InputPhaseServices>(
    player: &mut PlayerInputState,
    physical: &PhysicalPlayerInput,
    captured_category: u32,
    output: &mut ProcessedPhysicsInput,
    services: &mut S,
) -> Result<(), InputPhaseError<S::Error>> {
    if captured_category == 500 {
        player.current_ground_position_1216 = physical.skeleton.anim_to_world_11920[3];
        player.time_off_board_1368 += FIXED_FRAME_STEP;
    } else {
        player.current_ground_position_1216 = services
            .calculate_ground_position_82c02840(physical)
            .map_err(InputPhaseError::Service)?;
        player.time_off_board_1368 = 0.0;
    }
    output.time_off_board_2836 = player.time_off_board_1368;
    Ok(())
}

fn update_ground_history(
    player: &mut PlayerInputState,
    captured_category: u32,
    output: &ProcessedPhysicsInput,
) {
    if captured_category != 200 && captured_category != 600 && player.ground_history_frames_1304 > 0
    {
        let previous = player.previous_ground_position_1200;
        player.previous_ground_position_1200 = player.current_ground_position_1216;
        if output.frames_since_teleport_2584 > 1 {
            let result = super::motion_math::ground_history(GroundHistoryRequest {
                previous_position: previous,
                current_position: player.current_ground_position_1216,
                previous_filtered_delta: player.damped_ground_delta_1248,
                timestep_bits: output.timestep_2604.to_bits(),
            });
            player.ground_delta_1232 = result.delta;
            player.damped_ground_delta_1248 = result.filtered_delta;
        }
    }
}

fn publish_ground_history(player: &PlayerInputState, output: &mut ProcessedPhysicsInput) {
    output.vectors_464_480_496_512_528[1] = player.previous_ground_position_1200;
    output.vectors_464_480_496_512_528[2] = player.current_ground_position_1216;
    output.vectors_464_480_496_512_528[3] = player.damped_ground_delta_1248;
}

fn publish_post_skeleton(
    player: &mut PlayerInputState,
    physical: &PhysicalPlayerInput,
    output: &mut ProcessedPhysicsInput,
) {
    if output.flags_2472 & (1 << 21) != 0 {
        replace_byte(&mut output.flags_2472, 21, physical.state.flag_66);
    }
    let skitch_or_ground_window = player.skitch_timer_1376 > 0.0
        || (output.flags_2476 & (1 << 22) != 0 && physical.ground.scalar_276 >= 0.0);
    replace_bool(&mut output.flags_2476, 3, skitch_or_ground_window);
    player.time_since_last_input_1348 = if output.flags_2472 & (1 << 23) != 0 {
        player.time_since_last_input_1348 + output.timestep_2604
    } else {
        0.0
    };
    output.time_since_last_input_2748 = player.time_since_last_input_1348;

    if output.flags_2468 & (1 << 21) != 0 {
        player.prepared_jump_velocity_1184 = output.vectors_400_416[1];
    } else {
        player.prepared_jump_velocity_1184 =
            super::motion_math::prepared_jump_velocity(PrepareJumpRequest {
                skateboard_vector: output.vectors_400_416[1],
                previous_velocity: player.prepared_jump_velocity_1184,
            });
    }

    player.spin_same_direction_frames_1324 =
        if output.spin_input_2672 * player.previous_spin_input_1360 <= 0.0 {
            0
        } else {
            player.spin_same_direction_frames_1324.wrapping_add(1)
        };
    player.previous_spin_input_1360 = output.spin_input_2672;
    output.spin_same_direction_frames_2580 = player.spin_same_direction_frames_1324;
    output.prepared_jump_704 = player.prepared_jump_velocity_1184;
    output.crouch_delta_2780 = output.crouch_2776 - player.previous_crouch_1364;
    player.previous_crouch_1364 = output.crouch_2776;
}

fn native_nonnegative_decrement(value: u32) -> u32 {
    let decremented = value.wrapping_sub(1);
    if decremented.overflowing_add(0x8000_0000).1 {
        0
    } else {
        decremented
    }
}

fn xenon_fsel(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= -0.0 { nonnegative } else { negative }
}
